//! Port of `commands/hugobuilder.go`.
//!
//! Owner: Wave B task T25 (commands-cli).

//!
//! Only the non-watch path is in scope: `build` -> `fullBuild` -> `copyStatic` + `buildSites`.
//! Go runs the two concurrently in an errgroup unless `cleanDestinationDir` is set (then
//! copyStatic first). The port always runs copyStatic FIRST, then buildSites: the only observable
//! difference would be a static file and a rendered file sharing a target path, where rendered
//! output wins in the usual Go interleaving; the golden comparison (task I3) confirms no such
//! collision exists for seeksnack.

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::{Error, is_not_exist};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_hugofs::afero::Fs as AferoFs;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_hugofs::filesystems::basefs::SourceFilesystem;
use nh_hugolib::hugo_sites::HugoSites;
use nh_hugolib::hugo_sites_build::BuildCfg;

use crate::commandeer::{CommonConfig, ConfigKey, RootCommand};
use crate::fsync::Syncer;

/// Go: `commands.hugoBuilder`.
pub struct HugoBuilder {
    pub conf: Mutex<Option<Arc<CommonConfig>>>,
    pub err_state: HugoBuilderErrState,
}

/// Go: `commands.hugoBuilderErrState`.
#[derive(Default)]
pub struct HugoBuilderErrState {
    pub paused: bool,
    pub build_err: Mutex<Option<String>>,
    pub was_err: bool,
}

impl HugoBuilderErrState {
    // Go: commands/hugobuilder.go:(*hugoBuilderErrState).setBuildErr
    pub fn set_build_err(&self, err: Option<String>) {
        *self.build_err.lock().unwrap_or_else(|e| e.into_inner()) = err;
    }
}

/// The static copy options of `copyStaticTo` (Go: from `conf.configs.Base`).
#[derive(Clone, Copy, Debug, Default)]
pub struct StaticSyncOptions {
    pub no_times: bool,
    pub no_chmod: bool,
    /// `cleanDestinationDir` (the syncer's `Delete`).
    pub clean_destination_dir: bool,
}

impl Default for HugoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HugoBuilder {
    /// Go: `newHugoBuilder(r, nil)`.
    pub fn new() -> Self {
        HugoBuilder {
            conf: Mutex::new(None),
            err_state: HugoBuilderErrState::default(),
        }
    }

    /// Go: `(*hugoBuilder).withConfE`.
    // Go: commands/hugobuilder.go:(*hugoBuilder).withConfE
    fn with_conf(&self) -> Result<Arc<CommonConfig>> {
        self.conf
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| Error::new("config not set"))
    }

    /// Go: `(*hugoBuilder).build`.
    // Go: commands/hugobuilder.go:(*hugoBuilder).build
    pub fn build(&self, root: &RootCommand) -> Result<()> {
        self.init_profiling(root)?;

        self.full_build(root, false)?;

        if !root.quiet {
            root.print("\n");
            let h = self.built(root)?;
            let mut buf = Vec::new();
            h.print_processing_stats(&mut buf);
            {
                let mut w = root.opts.stdout.lock().unwrap_or_else(|e| e.into_inner());
                let _ = w.write_all(&buf);
            }
            root.print("\n");
        }
        Ok(())
    }

    /// Go: `(*hugoBuilder).initProfiling` — the profiling flags are not supported.
    // Go: commands/hugobuilder.go:(*hugoBuilder).initProfiling
    fn init_profiling(&self, root: &RootCommand) -> Result<()> {
        for (flag, v) in [
            ("--profile-cpu", &root.cpuprofile),
            ("--profile-mutex", &root.mutexprofile),
            ("--trace", &root.traceprofile),
            ("--profile-mem", &root.memprofile),
        ] {
            if !v.is_empty() {
                return Err(Error::new(format!("neohugo-rs: {flag} is not supported")));
            }
        }
        if root.printm {
            return Err(Error::new(
                "neohugo-rs: --printMemoryUsage is not supported",
            ));
        }
        Ok(())
    }

    /// Go: `(*hugoBuilder).fullBuild`.
    // Go: commands/hugobuilder.go:(*hugoBuilder).fullBuild
    pub fn full_build(&self, root: &RootCommand, no_build_lock: bool) -> Result<()> {
        root.logger.println("Start building sites … ");
        root.logger
            .println(nh_config::neohugo::version::build_version_string());
        root.logger.println("");

        let conf = self.with_conf()?;

        // Go: c.hugo() — both goroutines ask for the (lazily created) HugoSites.
        let h = root
            .new_hugo_sites(&conf)
            .map_err(|e| Error::with_kind(e.kind(), format!("error copying static files: {e}")))?;

        // Go: copyStaticFunc (run first; see the module docs).
        let lang_count = self
            .copy_static(root, &h)
            .map_err(|e| Error::with_kind(e.kind(), format!("error copying static files: {e}")))?;

        // Go: buildSitesFunc.
        let h = self
            .build_sites(h, no_build_lock)
            .map_err(|e| Error::with_kind(e.kind(), format!("error building site: {e}")))?;
        root.hugo_sites
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((
                ConfigKey {
                    counter: root.config_version_id.load(Ordering::SeqCst),
                    ignore_modules_does_not_exists: false,
                },
                h.clone(),
            ));

        for s in &h.sites {
            let lang = s.language().lang.clone();
            s.deps.path_spec().processing_stats.static_.store(
                lang_count.get(&lang).copied().unwrap_or(0),
                Ordering::Relaxed,
            );
        }

        if root.gc {
            // Go: h.GC() removes unused file cache entries.
            return Err(Error::new("neohugo-rs: --gc is not supported"));
        }
        Ok(())
    }

    /// Go: `(*hugoBuilder).buildSites`.
    // Go: commands/hugobuilder.go:(*hugoBuilder).buildSites
    pub fn build_sites(&self, h: HugoSites, no_build_lock: bool) -> Result<Arc<HugoSites>> {
        let res = nh_hugolib::hugo_sites_build::build(
            h,
            BuildCfg {
                no_build_lock,
                ..Default::default()
            },
        );
        self.err_state
            .set_build_err(res.as_ref().err().map(|e| e.to_string()));
        res
    }

    /// Go: `(*hugoBuilder).copyStatic` — per publish dir (language) file counts.
    // Go: commands/hugobuilder.go:(*hugoBuilder).copyStatic
    pub fn copy_static(&self, root: &RootCommand, h: &HugoSites) -> Result<BTreeMap<String, u64>> {
        let (m, err) = self.do_with_publish_dirs(root, h);
        match err {
            None => Ok(m),
            Some(e) if is_not_exist(&e) => Ok(m),
            Some(e) => Err(e),
        }
    }

    /// Go: `(*hugoBuilder).doWithPublishDirs(c.copyStaticTo)`.
    // Go: commands/hugobuilder.go:(*hugoBuilder).doWithPublishDirs
    fn do_with_publish_dirs(
        &self,
        root: &RootCommand,
        h: &HugoSites,
    ) -> (BTreeMap<String, u64>, Option<Error>) {
        let mut lang_count = BTreeMap::new();
        let conf = match self.with_conf() {
            Ok(c) => c,
            Err(e) => return (lang_count, Some(e)),
        };
        let static_filesystems = &h.deps.path_spec().base_fs.source_filesystems.static_;

        if static_filesystems.is_empty() {
            root.logger.infof("No static directories found to sync");
            return (lang_count, None);
        }

        for (lang, fs) in static_filesystems {
            let cnt = match self.copy_static_to(root, &conf, fs) {
                Ok(c) => c,
                Err(e) => return (lang_count, Some(e)),
            };
            if lang.is_empty() {
                // Not multihost
                for l in conf.configs.languages.iter() {
                    lang_count.insert(l.lang.clone(), cnt);
                }
            } else {
                lang_count.insert(lang.clone(), cnt);
            }
        }
        (lang_count, None)
    }

    /// Go: `(*hugoBuilder).copyStaticTo` — `fsync.Syncer{NoTimes, NoChmod, ChmodFilter, SrcFs, DestFs}`,
    /// `Delete` = `cleanDestinationDir`; syncs `sourceFs.PublishFolder`.
    // Go: commands/hugobuilder.go:(*hugoBuilder).copyStaticTo
    pub fn copy_static_to(
        &self,
        root: &RootCommand,
        conf: &CommonConfig,
        source_fs: &SourceFilesystem,
    ) -> Result<u64> {
        let base = &conf.configs.base.root;
        let opts = StaticSyncOptions {
            no_times: base.no_times,
            no_chmod: base.no_chmod,
            clean_destination_dir: base.clean_destination_dir,
        };
        if opts.clean_destination_dir {
            root.logger.infof(
                "static: removing all files from destination that don't exist in static dirs",
            );
        }
        let start = std::time::Instant::now();
        let n = copy_static_to_fs(source_fs, conf.fs.publish_dir_static.clone(), opts)?;
        root.logger.time_trackf(
            nh_common::loggers::Level::Info,
            start,
            format!(
                "static: syncing static files to {}",
                publish_dir_of(source_fs)
            ),
        );
        Ok(n)
    }

    /// Go: `(*hugoBuilder).hugo` — the built sites (the port creates and builds them in
    /// `fullBuild`; see `RootCommand::new_hugo_sites`).
    // Go: commands/hugobuilder.go:(*hugoBuilder).hugo
    pub fn hugo(&self, root: &RootCommand) -> Result<Arc<HugoSites>> {
        self.built(root)
    }

    fn built(&self, root: &RootCommand) -> Result<Arc<HugoSites>> {
        root.hugo_sites
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .last()
            .map(|(_, h)| h.clone())
            .ok_or_else(|| Error::new("neohugo-rs: the sites are not built"))
    }

    /// Go: `(*hugoBuilder).postBuild`.
    // Go: commands/hugobuilder.go:(*hugoBuilder).postBuild
    pub fn post_build(&self, root: &RootCommand, what: &str, start: std::time::Instant) {
        root.time_track(start, what);
    }

    /// Go: `(*hugoBuilder).loadConfig`.
    // Go: commands/hugobuilder.go:(*hugoBuilder).loadConfig
    pub fn load_config(&self, root: &mut RootCommand, running: bool) -> Result<()> {
        let cfg = DefaultConfigProvider::new();
        cfg.set("renderToMemory", Value::Bool(root.render_to_memory));
        let watch = root.build_watch;
        if root.environment.is_empty() {
            // We need to set the environment as early as possible because we need it to load the
            // correct config. Check if the user has set it in env.
            let env = root.getenv("HUGO_ENVIRONMENT");
            if !env.is_empty() {
                root.environment = env;
            } else {
                let env = root.getenv("HUGO_ENV");
                if !env.is_empty() {
                    root.environment = env;
                } else {
                    root.environment =
                        nh_config::neohugo::neohugo::ENVIRONMENT_PRODUCTION.to_string();
                }
            }
        }
        cfg.set("environment", Value::string(root.environment.as_str()));

        let mut internal = Map::new(MapType::Params);
        internal
            .entries
            .insert("running".into(), Value::Bool(running));
        internal.entries.insert("watch".into(), Value::Bool(watch));
        internal
            .entries
            .insert("verbose".into(), Value::Bool(root.is_verbose()));
        internal
            .entries
            .insert("fastRenderMode".into(), Value::Bool(false));
        cfg.set("internal", Value::Map(Arc::new(internal)));

        let cfg: Arc<dyn Provider> = Arc::new(cfg);
        let conf = root.config_from_provider(
            ConfigKey {
                counter: root.config_version_id.load(Ordering::SeqCst),
                ignore_modules_does_not_exists: false,
            },
            crate::helpers::flags_to_cfg(&root.flags, cfg),
        )?;

        if conf.configs.loading_info.config_files.is_empty() {
            return Err(Error::new(
                "Unable to locate config file or config directory. Perhaps you need to create a new site.\nRun `hugo help new` for details.",
            ));
        }

        *self.conf.lock().unwrap_or_else(|e| e.into_inner()) = Some(conf);
        Ok(())
    }
}

/// Go: `publishDir := helpers.FilePathSeparator` joined with `sourceFs.PublishFolder`.
fn publish_dir_of(source_fs: &SourceFilesystem) -> String {
    let mut publish_dir = "/".to_string();
    if !source_fs.publish_folder.is_empty() {
        publish_dir = go_path::filepath::join(&[publish_dir.as_str(), &source_fs.publish_folder]);
    }
    publish_dir
}

/// The body of Go's `copyStaticTo` without the config lookup: syncs `source_fs` (from its root)
/// into `dest_fs` at the publish folder and returns the number of files (Go: the counting
/// stat fs counts two `Stat` calls per source file).
// Go: commands/hugobuilder.go:(*hugoBuilder).copyStaticTo
pub fn copy_static_to_fs(
    source_fs: &SourceFilesystem,
    dest_fs: Arc<dyn AferoFs>,
    opts: StaticSyncOptions,
) -> Result<u64> {
    let publish_dir = publish_dir_of(source_fs);

    let fs = Arc::new(CountingStatFs {
        fs: source_fs.fs.clone(),
        stat_counter: AtomicU64::new(0),
    });

    let mut syncer = Syncer::new(fs.clone(), dest_fs);
    syncer.no_times = opts.no_times;
    syncer.no_chmod = opts.no_chmod;
    syncer.chmod_filter = Some(chmod_filter);
    // Now that we are using a unionFs for the static directories
    // We can effectively clean the publishDir on initial sync
    syncer.delete = opts.clean_destination_dir;

    if syncer.delete {
        syncer.delete_filter = Some(|name, is_dir| is_dir && name.starts_with('.'));
    }

    // because we are using a baseFs (to get the union right).
    // set sync src to root
    syncer.sync(&publish_dir, "/")?;

    // Sync runs Stat 2 times for every source file.
    Ok(fs.stat_counter.load(Ordering::SeqCst) / 2)
}

/// Go: `commands.chmodFilter` (server.go) — never sync the permissions of directories: Hugo
/// publishes from several sources with overlapping directory structures (e.g. 0555 module cache
/// directories).
// Go: commands/server.go:chmodFilter
fn chmod_filter(_dst_mode: u32, _src_mode: u32, src_is_dir: bool) -> bool {
    src_is_dir
}

/// Go: `commands.countingStatFs` (server.go) — counts the `Stat` calls of regular files.
struct CountingStatFs {
    fs: Arc<dyn AferoFs>,
    stat_counter: AtomicU64,
}

impl AferoFs for CountingStatFs {
    fn name(&self) -> &str {
        self.fs.name()
    }
    fn embedded(&self) -> Option<&dyn AferoFs> {
        Some(&*self.fs)
    }
    // Go: commands/server.go:(*countingStatFs).Stat
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        let f = self.fs.stat(name)?;
        if !f.is_dir() {
            self.stat_counter.fetch_add(1, Ordering::SeqCst);
        }
        Ok(f)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/hugobuilder.go (1157 lines; 17/33 funcs executed)
//   types: hugoBuilder, hugoBuilderErrState
// OK L80-87: (c *hugoBuilder) withConfE(fn func(conf *commonConfig) error) error
// OK L89-93: (c *hugoBuilder) withConf(fn func(conf *commonConfig))
//    L102-106: (e *hugoBuilderErrState) setPaused(p bool) (server only)
//    L108-112: (e *hugoBuilderErrState) isPaused() bool (server only)
// OK L114-118: (e *hugoBuilderErrState) setBuildErr(err error)
//    L120-124: (e *hugoBuilderErrState) buildErr() error (server only)
//    L126-130: (e *hugoBuilderErrState) setWasErr(w bool) (server only)
//    L132-136: (e *hugoBuilderErrState) wasErr() bool (server only)
//    L139-146: (c *hugoBuilder) getDirList() ([]string, error) (watch only)
// OK L148-164: (c *hugoBuilder) initCPUProfile() (func(), error) (unsupported error when set)
// OK L166-180: (c *hugoBuilder) initMemProfile() (unsupported error when set)
//    L182-207: (c *hugoBuilder) initMemTicker() func() (unsupported error when set)
// OK L209-225: (c *hugoBuilder) initMutexProfile() (func(), error) (unsupported error when set)
// OK L227-266: (c *hugoBuilder) initProfiling() (func(), error)
// OK L268-286: (c *hugoBuilder) initTraceProfile() (func(), error) (unsupported error when set)
//    L289-383: (c *hugoBuilder) newWatcher(pollIntervalStr string, dirList ...string) (*watcher.Batcher, error) (watch only)
// OK L385-413: (c *hugoBuilder) build() error
// OK L415-427: (c *hugoBuilder) buildSites(noBuildLock bool) (err error)
// OK L429-435: (c *hugoBuilder) copyStatic() (map[string]uint64, error)
// OK L437-482: (c *hugoBuilder) copyStaticTo(sourceFs *filesystems.SourceFilesystem) (uint64, error)
// OK L484-516: (c *hugoBuilder) doWithPublishDirs(f func(sourceFs *filesystems.SourceFilesystem) (uint64, error)) (map[string]uint64, error)
// OK L518-589: (c *hugoBuilder) fullBuild(noBuildLock bool) error (sequential; --gc unsupported)
//    L591-643: (c *hugoBuilder) fullRebuild(changeType string) (server only)
//    L645-648: (c *hugoBuilder) handleBuildErr(err error, msg string) (server only)
//    L650-993: (c *hugoBuilder) handleEvents(watcher *watcher.Batcher, staticSyncer *staticSyncer, evs []fsnotify.Event, configSet map[string]bool, ) (server only)
// OK L995-1000: (c *hugoBuilder) postBuild(what string, start time.Time)
// OK L1002-1019: (c *hugoBuilder) hugo() (*hugolib.HugoSites, error)
//    L1021-1027: (c *hugoBuilder) hugoTry() *hugolib.HugoSites (server only)
// OK L1029-1076: (c *hugoBuilder) loadConfig(cd *simplecobra.Commandeer, running bool) error
//    L1080-1090: (c *hugoBuilder) printChangeDetected(typ string) (server only)
//    L1092-1109: (c *hugoBuilder) rebuildSites(events []fsnotify.Event) (err error) (server only)
//    L1111-1126: (c *hugoBuilder) rebuildSitesForChanges(ids []identity.Identity) (err error) (server only)
//    L1128-1157: (c *hugoBuilder) reloadConfig() error (server only)
// ---------------------------------------------------------------------------
