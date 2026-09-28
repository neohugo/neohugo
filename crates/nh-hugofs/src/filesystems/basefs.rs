//! Port of `hugolib/filesystems/basefs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

//! Go `hugolib/filesystems.BaseFs`: assembles the component filesystems from module mounts
//! (RootMappingFs per module, overlays, ComponentFs views). The `assets` view resolves
//! `/vendor/...` through the `node_modules -> assets/vendor` mount; `_jsconfig` files are mounted
//! under `assets/_jsconfig`.
//!
//! The modules come from `Paths::all_modules()` (the `allModules` config section): a
//! `modules::module::Modules` list whose `Module` values carry what `modules.Module` gives
//! basefs.go (`Path`, `Dir`, `Mounts`, `Owner`, `Watch`). T09 builds them in a real build; tests
//! build them from the Go oracle's recording (PORTING.md, "The modules seam").

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use nh_common::Result;
use nh_common::files::{
    COMPONENT_FOLDER_ARCHETYPES, COMPONENT_FOLDER_ASSETS, COMPONENT_FOLDER_CONTENT,
    COMPONENT_FOLDER_DATA, COMPONENT_FOLDER_I18N, COMPONENT_FOLDER_LAYOUTS,
    COMPONENT_FOLDER_STATIC, COMPONENT_FOLDERS, FOLDER_JS_CONFIG, FOLDER_RESOURCES,
};
use nh_common::glob::filename_filter::FilenameFilter;
use nh_common::herrors::Error;
use nh_common::loggers::Logger;
use nh_common::paths::path as hpaths;

use crate::afero::Fs;
use crate::component_fs::{ComponentFs, ComponentFsOptions};
use crate::decorators::new_base_file_decorator;
use crate::dirsmerger::{append_dirs_merger, language_dirs_merger};
use crate::fileinfo::{FileMeta, FileMetaInfo};
use crate::fs::{new_base_path_fs, new_read_only_fs, no_op_fs, walk_filesystems};
use crate::modules::config::Mount;
use crate::modules::module::Module;
use crate::overlayfs::{Options, OverlayFs};
use crate::paths::Paths;
use crate::rootmapping_fs::{ComponentPath, RootMapping, RootMappingFs};
use crate::walk::{Walkway, WalkwayConfig, skip_dir};

/// Used to control concurrency between multiple Hugo instances, e.g. a running server and
/// building new content with 'hugo new'. It's placed in the project root.
const LOCK_FILE_BUILD: &str = ".hugo_build.lock";

const FILE_PATH_SEPARATOR: &str = "/";

/// Go: `filesystems.Lockable`.
pub trait Lockable: Send + Sync {
    /// Go: `Lock() (unlock func(), err error)`.
    fn lock(&self) -> Result<Box<dyn FnOnce() + Send>>;
}

/// Go: `fakeLockfileMutex` (a process-local mutex; `noBuildLock`).
#[derive(Default)]
struct FakeLockfileMutex {
    mu: Arc<Mutex<bool>>,
    cv: Arc<std::sync::Condvar>,
}

impl Lockable for FakeLockfileMutex {
    // Go: hugolib/filesystems/basefs.go:(f *fakeLockfileMutex) Lock
    fn lock(&self) -> Result<Box<dyn FnOnce() + Send>> {
        let mut locked = self.mu.lock().unwrap_or_else(|e| e.into_inner());
        while *locked {
            locked = self.cv.wait(locked).unwrap_or_else(|e| e.into_inner());
        }
        *locked = true;
        let (mu, cv) = (self.mu.clone(), self.cv.clone());
        Ok(Box::new(move || {
            *mu.lock().unwrap_or_else(|e| e.into_inner()) = false;
            cv.notify_one();
        }))
    }
}

/// Go: `lockedfile.MutexAt(path)` (github.com/rogpeppe/go-internal): an exclusive `flock` on a
/// file opened with `O_RDWR|O_CREATE` and mode 0666.
struct LockedFileMutex {
    path: String,
}

impl Lockable for LockedFileMutex {
    fn lock(&self) -> Result<Box<dyn FnOnce() + Send>> {
        use std::os::unix::fs::OpenOptionsExt;
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o666)
            .open(&self.path)
            .map_err(|e| crate::oserror::from_io("open", &self.path, &e))?;
        f.lock()
            .map_err(|e| crate::oserror::from_io("lock", &self.path, &e))?;
        Ok(Box::new(move || {
            let _ = f.unlock();
            drop(f);
        }))
    }
}

/// Go: `filesystems.SourceFilesystem` — the filesystem for a given source type in Hugo (data,
/// i18n, layouts, static) and additional metadata to be able to use that filesystem in server
/// mode.
#[derive(Clone)]
pub struct SourceFilesystem {
    /// Name matches one of the top level folders (i.e. "content", "assets").
    pub name: String,
    /// The component view (a ComponentFs over the overlay).
    pub fs: Arc<dyn Fs>,
    /// The source filesystem (usually the OS filesystem).
    pub source_fs: Arc<dyn Fs>,
    /// When syncing a source folder to the target (e.g. /public), this may be set to publish into a subfolder.
    pub publish_folder: String,
}

impl SourceFilesystem {
    /// Go: `MakePathRelative(filename, checkExists)` — real filename -> path relative to the component.
    // Go: hugolib/filesystems/basefs.go:MakePathRelative
    pub fn make_path_relative(&self, filename: &str, check_exists: bool) -> Option<String> {
        let cps = match self.reverse_lookup(filename, check_exists) {
            Ok(cps) => cps,
            Err(e) => panic!("{e}"),
        };
        if cps.is_empty() {
            return None;
        }

        Some(go_path::filepath::from_slash(&cps[0].path).to_string())
    }

    /// Go: `ReverseLookup(filename, checkExists)` — the component paths for the given filename.
    // Go: hugolib/filesystems/basefs.go:ReverseLookup
    pub fn reverse_lookup(&self, filename: &str, check_exists: bool) -> Result<Vec<ComponentPath>> {
        let mut cps = Vec::new();
        walk_filesystems(&self.fs, &mut |fs| {
            if let Some(rfs) = fs.reverse_lookup_provider()
                && let Ok(mut c) = rfs.reverse_lookup_component(&self.name, filename)
            {
                if check_exists {
                    c.retain(|cp| {
                        self.fs
                            .stat(go_path::filepath::from_slash(&cp.path))
                            .is_ok()
                    });
                }
                cps.extend(c);
            }
            false
        });
        Ok(cps)
    }

    // Go: hugolib/filesystems/basefs.go:mounts
    fn mounts(&self) -> Vec<FileMetaInfo> {
        let mut m = Vec::new();
        walk_filesystems(&self.fs, &mut |fs| {
            if let Some(rfs) = fs.as_any().downcast_ref::<RootMappingFs>()
                && let Ok(mounts) = rfs.mounts(&self.name)
            {
                m.extend(mounts);
            }
            false
        });

        // Filter out any mounts not belonging to this filesystem.
        m.retain(|mm| mm.meta().component == self.name);

        m
    }

    /// Go: `RealFilename(rel)`.
    // Go: hugolib/filesystems/basefs.go:RealFilename
    pub fn real_filename(&self, rel: &str) -> String {
        match self.fs.stat(rel) {
            Err(_) => rel.to_string(),
            Ok(fi) => fi.meta().filename.clone(),
        }
    }

    /// Go: `RealDirs(from)` — for each mount dir: join(mountRealDir, from) if it exists
    /// (libsass include paths).
    // Go: hugolib/filesystems/basefs.go:RealDirs
    pub fn real_dirs(&self, from: &str) -> Vec<String> {
        let mut dirnames = Vec::new();
        for m in self.mounts() {
            if !m.is_dir() {
                continue;
            }
            let dirname = go_path::filepath::join(&[m.meta().filename.as_str(), from]);
            if self.source_fs.stat(&dirname).is_ok() {
                dirnames.push(dirname);
            }
        }
        dirnames
    }

    /// Go: `Contains(filename)` — whether the given filename is a member of the current
    /// filesystem.
    // Go: hugolib/filesystems/basefs.go:Contains
    pub fn contains(&self, filename: &str) -> bool {
        for dir in self.mounts() {
            if !dir.is_dir() {
                continue;
            }
            if filename.starts_with(dir.meta().filename.as_str()) {
                return true;
            }
        }
        false
    }
}

/// Go: `filesystems.SourceFilesystems`.
#[derive(Clone)]
pub struct SourceFilesystems {
    pub content: Arc<SourceFilesystem>,
    pub data: Arc<SourceFilesystem>,
    pub i18n: Arc<SourceFilesystem>,
    pub layouts: Arc<SourceFilesystem>,
    pub archetypes: Arc<SourceFilesystem>,
    pub assets: Arc<SourceFilesystem>,
    pub assets_with_duplicates_preserved: Arc<SourceFilesystem>,
    pub root_fss: Vec<Arc<RootMappingFs>>,
    /// Writable filesystem on top the project's resources directory.
    pub resources_cache: Arc<dyn Fs>,
    /// The work folder (may be a composite of project and theme components).
    pub work: Arc<dyn Fs>,
    /// When in multihost we have one static filesystem per language (key "" otherwise).
    pub static_: BTreeMap<String, Arc<SourceFilesystem>>,
}

impl SourceFilesystems {
    /// Go: `StaticFs(lang)` — the static filesystem for the given language.
    // Go: hugolib/filesystems/basefs.go:StaticFs
    pub fn static_fs(&self, lang: &str) -> Arc<dyn Fs> {
        if let Some(fs) = self.static_.get(lang) {
            return fs.fs.clone();
        }
        if let Some(fs) = self.static_.get("") {
            return fs.fs.clone();
        }
        no_op_fs()
    }

    /// Go: `StatResource(lang, filename)` — looks for a resource in static, assets and content
    /// (in that order); returns the result of the first fs that has it or fails with another
    /// error than not-exist, and that fs.
    // Go: hugolib/filesystems/basefs.go:StatResource
    pub fn stat_resource(&self, lang: &str, filename: &str) -> (Result<FileMetaInfo>, Arc<dyn Fs>) {
        let mut last = None;
        for fs in [
            self.static_fs(lang),
            self.assets.fs.clone(),
            self.content.fs.clone(),
        ] {
            let r = fs.stat(filename);
            match &r {
                Ok(_) => return (r, fs),
                Err(e) if !e.is_not_exist() => return (r, fs),
                Err(_) => last = Some((r, fs)),
            }
        }
        // Not found.
        last.expect("three filesystems")
    }

    /// Go: `IsStatic(filename)`.
    // Go: hugolib/filesystems/basefs.go:IsStatic
    pub fn is_static(&self, filename: &str) -> bool {
        self.static_.values().any(|fs| fs.contains(filename))
    }

    // Go: hugolib/filesystems/basefs.go:IsContent
    pub fn is_content(&self, filename: &str) -> bool {
        self.content.contains(filename)
    }

    /// Go: `ResolvePaths(filename)` (Go panics on a lookup error).
    // Go: hugolib/filesystems/basefs.go:ResolvePaths
    pub fn resolve_paths(&self, filename: &str) -> Vec<ComponentPath> {
        let mut cpss = Vec::new();
        for rfs in &self.root_fss {
            match rfs.reverse_lookup(filename) {
                Ok(cps) => cpss.extend(cps),
                Err(e) => panic!("{e}"),
            }
        }
        cpss
    }

    /// Go: `MakeStaticPathRelative(filename)` — "" if not in a static filesystem (Go iterates the
    /// static map in random order; byte order here).
    // Go: hugolib/filesystems/basefs.go:MakeStaticPathRelative
    pub fn make_static_path_relative(&self, filename: &str) -> String {
        for static_fs in self.static_.values() {
            if let Some(rel) = static_fs.make_path_relative(filename, true)
                && !rel.is_empty()
            {
                return rel;
            }
        }
        String::new()
    }
}

/// Go: `filesystems.BaseFs`.
#[derive(Clone)]
pub struct BaseFs {
    pub source_filesystems: Arc<SourceFilesystems>,
    /// The source filesystem (needs absolute filenames).
    pub source_fs: Arc<dyn Fs>,
    /// The project source.
    pub project_source_fs: Arc<dyn Fs>,
    /// The filesystem used to publish the rendered site (below the publish dir).
    pub publish_fs: Arc<dyn Fs>,
    /// The filesystem used for static files.
    pub publish_fs_static: Arc<dyn Fs>,
    /// A read-only filesystem starting from the project workDir.
    pub work_dir: Arc<dyn Fs>,
    pub(crate) the_big_fs: Arc<FilesystemsCollector>,
    pub(crate) working_dir: String,
    /// Locks: `<project>/.hugo_build.lock`.
    build_mu: Arc<dyn Lockable>,
}

impl std::ops::Deref for BaseFs {
    type Target = SourceFilesystems;
    /// Go embeds `*SourceFilesystems` in `BaseFs`.
    fn deref(&self) -> &SourceFilesystems {
        &self.source_filesystems
    }
}

/// Go: `filesystemsCollector`.
pub struct FilesystemsCollector {
    /// Source for project folders.
    pub(crate) source_project: Arc<dyn Fs>,
    /// Source for modules/themes.
    pub(crate) source_modules: Arc<dyn Fs>,
    pub(crate) overlay_mounts: OverlayFs,
    pub(crate) overlay_mounts_content: OverlayFs,
    pub(crate) overlay_mounts_static: OverlayFs,
    /// Go: never set when there are mounts (nil).
    pub(crate) overlay_mounts_full: Option<OverlayFs>,
    pub(crate) overlay_full: OverlayFs,
    pub(crate) overlay_resources: OverlayFs,
    pub(crate) root_fss: Vec<Arc<RootMappingFs>>,
    /// Set if in multihost mode.
    pub(crate) static_per_language: Option<BTreeMap<String, OverlayFs>>,
}

impl FilesystemsCollector {
    // Go: hugolib/filesystems/basefs.go:addRootFs
    fn add_root_fs(&mut self, rfs: Arc<RootMappingFs>) {
        self.root_fss.push(rfs);
    }
}

/// Go: `mountsDescriptor`.
struct MountsDescriptor {
    module: Arc<Module>,
    dir: String,
    is_main_project: bool,
    /// zero based starting from the project.
    ordinal: i64,
}

impl BaseFs {
    /// Go: `filesystems.NewBase(p, logger)`.
    // Go: hugolib/filesystems/basefs.go:NewBase
    pub fn new(p: &Paths) -> Result<Arc<BaseFs>> {
        Self::new_with_base(p, None, None)
    }

    /// Go: `filesystems.NewBase(p, logger, WithBaseFs(base))` — `base` reuses the potentially
    /// expensive parts that remain the same across sites/languages.
    // Go: hugolib/filesystems/basefs.go:NewBase
    pub fn new_with_base(
        p: &Paths,
        logger: Option<Logger>,
        base: Option<&BaseFs>,
    ) -> Result<Arc<BaseFs>> {
        let fs = &p.fs;
        let logger = logger.unwrap_or_else(Logger::new_default);

        let publish_fs = new_base_file_decorator(fs.publish_dir.clone(), Vec::new());
        let project_source_fs = new_base_file_decorator(
            new_base_path_fs(fs.source.clone(), &p.cfg.base_config().working_dir),
            Vec::new(),
        );
        let source_fs = new_base_file_decorator(fs.source.clone(), Vec::new());
        let publish_fs_static = fs.publish_dir_static.clone();

        let build_mu: Arc<dyn Lockable> = if p.cfg.no_build_lock() {
            Arc::new(FakeLockfileMutex::default())
        } else {
            Arc::new(LockedFileMutex {
                path: go_path::filepath::join(&[
                    p.cfg.base_config().working_dir.as_str(),
                    LOCK_FILE_BUILD,
                ]),
            })
        };

        let working_dir = p.cfg.base_config().working_dir.clone();

        // Go: WithBaseFs(b)
        if let Some(b) = base {
            return Ok(Arc::new(BaseFs {
                source_filesystems: b.source_filesystems.clone(),
                source_fs,
                project_source_fs,
                publish_fs,
                publish_fs_static,
                work_dir: fs.working_dir_read_only.clone(),
                the_big_fs: b.the_big_fs.clone(),
                working_dir,
                build_mu,
            }));
        }

        let mut builder = SourceFilesystemsBuilder::new(p, logger);
        let source_filesystems = builder.build().map_err(|e| e.wrap("build filesystems"))?;

        Ok(Arc::new(BaseFs {
            source_filesystems: Arc::new(source_filesystems),
            source_fs,
            project_source_fs,
            publish_fs,
            publish_fs_static,
            work_dir: fs.working_dir_read_only.clone(),
            the_big_fs: builder.the_big_fs.expect("built"),
            working_dir,
            build_mu,
        }))
    }

    /// Go: `LockBuild()` — `.hugo_build.lock` file lock in the working dir (skipped with noBuildLock).
    // Go: hugolib/filesystems/basefs.go:LockBuild
    pub fn lock_build(&self) -> Result<Box<dyn FnOnce() + Send>> {
        self.build_mu.lock()
    }

    /// Go: `WatchFilenames()` — the directories (and single files) to watch.
    // Go: hugolib/filesystems/basefs.go:WatchFilenames
    pub fn watch_filenames(&self) -> Vec<String> {
        let mut filenames = Vec::new();
        let source_fs = self.source_fs.clone();

        for rfs in &self.root_fss {
            for component in COMPONENT_FOLDERS {
                let fis = match rfs.mounts(component) {
                    Ok(fis) => fis,
                    Err(_) => continue,
                };

                for fim in fis {
                    let meta = fim.meta();
                    if !meta.watch {
                        continue;
                    }

                    if !fim.is_dir() {
                        filenames.push(meta.filename.clone());
                        continue;
                    }

                    let mut wfn = |_path: &str, fi: &FileMetaInfo| -> Result<()> {
                        if !fi.is_dir() {
                            return Ok(());
                        }
                        if fi.name() == ".git"
                            || fi.name() == "node_modules"
                            || fi.name() == "bower_components"
                        {
                            return Err(skip_dir());
                        }
                        filenames.push(fi.meta().filename.clone());
                        Ok(())
                    };
                    let mut cfg = WalkwayConfig::new(source_fs.clone(), &mut wfn);
                    cfg.root = meta.filename.clone();
                    let mut w = Walkway::new(cfg);

                    let _ = w.walk();
                }
            }
        }

        filenames
    }

    // Go: hugolib/filesystems/basefs.go:mountsForComponent
    fn mounts_for_component(&self, component: &str) -> Vec<FileMetaInfo> {
        let mut result = Vec::new();
        for rfs in &self.root_fss {
            if let Ok(dirs) = rfs.mounts(component) {
                result.extend(dirs);
            }
        }
        result
    }

    /// Go: `AbsProjectContentDir(filename)` — tries to construct a filename below the most
    /// relevant content directory: `(relative, absolute)`.
    // Go: hugolib/filesystems/basefs.go:AbsProjectContentDir
    pub fn abs_project_content_dir(&self, filename: &str) -> Result<(String, String)> {
        let is_abs = go_path::filepath::is_abs(filename);
        for fi in self.mounts_for_component(COMPONENT_FOLDER_CONTENT) {
            if !fi.is_dir() {
                continue;
            }
            let meta = fi.meta();
            if !meta.is_project {
                continue;
            }

            if is_abs {
                if filename.starts_with(meta.filename.as_str()) {
                    let prefix = format!("{}{FILE_PATH_SEPARATOR}", meta.filename);
                    return Ok((
                        filename
                            .strip_prefix(prefix.as_str())
                            .unwrap_or(filename)
                            .to_string(),
                        filename.to_string(),
                    ));
                }
            } else {
                let content_dir = meta
                    .filename
                    .strip_prefix(meta.base_dir.as_str())
                    .unwrap_or(&meta.filename);
                let content_dir = format!(
                    "{}{FILE_PATH_SEPARATOR}",
                    content_dir
                        .strip_prefix(FILE_PATH_SEPARATOR)
                        .unwrap_or(content_dir)
                );

                if let Some(rel_filename) = filename.strip_prefix(content_dir.as_str()) {
                    let abs_filename =
                        go_path::filepath::join(&[meta.filename.as_str(), rel_filename]);
                    return Ok((rel_filename.to_string(), abs_filename));
                }
            }
        }

        if !is_abs {
            // A filename on the form "posts/mypage.md", put it inside
            // the first content folder, usually <workDir>/content.
            // Pick the first project dir (which is probably the most important one).
            for dir in self.content.mounts() {
                if !dir.is_dir() {
                    continue;
                }
                let meta = dir.meta();
                if meta.is_project {
                    return Ok((
                        filename.to_string(),
                        go_path::filepath::join(&[meta.filename.as_str(), filename]),
                    ));
                }
            }
        }

        Err(Error::new(format!(
            "could not determine content directory for {}",
            go_strconv::quote(filename)
        )))
    }

    /// Go: `ResolveJSConfigFile(name)` — `assets/_jsconfig/<name>` real filename, else working dir.
    // Go: hugolib/filesystems/basefs.go:ResolveJSConfigFile
    pub fn resolve_js_config_file(&self, name: &str) -> String {
        // First look in assets/_jsconfig
        if let Ok(fi) = self
            .assets
            .fs
            .stat(&go_path::filepath::join(&[FOLDER_JS_CONFIG, name]))
        {
            return fi.meta().filename.clone();
        }
        // Fall back to the work dir.
        if let Ok(fi) = self.work.stat(name) {
            return fi.meta().filename.clone();
        }

        String::new()
    }
}

/// Go: `sourceFilesystemsBuilder`.
struct SourceFilesystemsBuilder<'a> {
    #[allow(dead_code)] // Go keeps the logger for the builder; nothing logs yet.
    logger: Logger,
    p: &'a Paths,
    source_fs: Arc<dyn Fs>,
    the_big_fs: Option<Arc<FilesystemsCollector>>,
}

impl<'a> SourceFilesystemsBuilder<'a> {
    // Go: hugolib/filesystems/basefs.go:newSourceFilesystemsBuilder
    fn new(p: &'a Paths, logger: Logger) -> Self {
        let source_fs = new_base_file_decorator(p.fs.source.clone(), Vec::new());
        SourceFilesystemsBuilder {
            p,
            logger,
            source_fs,
            the_big_fs: None,
        }
    }

    // Go: hugolib/filesystems/basefs.go:newSourceFilesystem
    fn new_source_filesystem(&self, name: &str, fs: Arc<dyn Fs>) -> SourceFilesystem {
        SourceFilesystem {
            name: name.to_string(),
            fs,
            source_fs: self.source_fs.clone(),
            publish_folder: String::new(),
        }
    }

    // Go: hugolib/filesystems/basefs.go:Build
    fn build(&mut self) -> Result<SourceFilesystems> {
        if self.the_big_fs.is_none() {
            let the_big_fs = self
                .create_main_overlay_fs(self.p)
                .map_err(|e| e.wrap("create main fs"))?;

            self.the_big_fs = Some(Arc::new(the_big_fs));
        }
        let big = self.the_big_fs.clone().expect("set above");

        let cfg = self.p.cfg.clone();
        let create_view = |component_id: &str, overlay_fs: &OverlayFs| -> SourceFilesystem {
            let fs = ComponentFs::new(ComponentFsOptions {
                fs: Arc::new(overlay_fs.clone()),
                component: component_id.to_string(),
                default_content_language: cfg.default_content_language(),
                path_parser: cfg.path_parser(),
            });

            self.new_source_filesystem(component_id, fs)
        };

        let archetypes = create_view(COMPONENT_FOLDER_ARCHETYPES, &big.overlay_mounts);
        let layouts = create_view(COMPONENT_FOLDER_LAYOUTS, &big.overlay_mounts);
        let assets = create_view(COMPONENT_FOLDER_ASSETS, &big.overlay_mounts);
        let resources_cache: Arc<dyn Fs> = Arc::new(big.overlay_resources.clone());
        let root_fss = big.root_fss.clone();

        // data and i18n  needs a different merge strategy.
        let overlay_mounts_preserve_dupes =
            big.overlay_mounts.with_dirs_merger(append_dirs_merger());
        let data = create_view(COMPONENT_FOLDER_DATA, &overlay_mounts_preserve_dupes);
        let i18n = create_view(COMPONENT_FOLDER_I18N, &overlay_mounts_preserve_dupes);
        let assets_with_duplicates_preserved =
            create_view(COMPONENT_FOLDER_ASSETS, &overlay_mounts_preserve_dupes);

        let content_fs = ComponentFs::new(ComponentFsOptions {
            fs: Arc::new(big.overlay_mounts_content.clone()),
            component: COMPONENT_FOLDER_CONTENT.to_string(),
            default_content_language: cfg.default_content_language(),
            path_parser: cfg.path_parser(),
        });

        let content = self.new_source_filesystem(COMPONENT_FOLDER_CONTENT, content_fs);
        let work = new_read_only_fs(Arc::new(big.overlay_full.clone()));

        // Create static filesystem(s)
        let mut ms = BTreeMap::new();

        if let Some(static_per_language) = &big.static_per_language {
            // Multihost mode
            for (k, v) in static_per_language {
                let mut sfs =
                    self.new_source_filesystem(COMPONENT_FOLDER_STATIC, Arc::new(v.clone()));
                sfs.publish_folder = k.clone();
                ms.insert(k.clone(), Arc::new(sfs));
            }
        } else {
            let bfs = new_base_path_fs(
                Arc::new(big.overlay_mounts_static.clone()),
                COMPONENT_FOLDER_STATIC,
            );
            ms.insert(
                String::new(),
                Arc::new(self.new_source_filesystem(COMPONENT_FOLDER_STATIC, bfs)),
            );
        }

        Ok(SourceFilesystems {
            content: Arc::new(content),
            data: Arc::new(data),
            i18n: Arc::new(i18n),
            layouts: Arc::new(layouts),
            archetypes: Arc::new(archetypes),
            assets: Arc::new(assets),
            assets_with_duplicates_preserved: Arc::new(assets_with_duplicates_preserved),
            root_fss,
            resources_cache,
            work,
            static_: ms,
        })
    }

    // Go: hugolib/filesystems/basefs.go:createMainOverlayFs
    fn create_main_overlay_fs(&self, p: &Paths) -> Result<FilesystemsCollector> {
        let mut static_fs_map: Option<BTreeMap<String, OverlayFs>> = None;
        if self.p.cfg.is_multihost() {
            let languages = self.p.cfg.languages();
            let mut m = BTreeMap::new();
            for l in languages {
                m.insert(l.lang.clone(), OverlayFs::new(Options::default()));
            }
            static_fs_map = Some(m);
        }

        let mut collector = FilesystemsCollector {
            source_project: self.source_fs.clone(),
            source_modules: self.source_fs.clone(),
            static_per_language: static_fs_map,

            overlay_mounts: OverlayFs::new(Options::default()),
            overlay_mounts_content: OverlayFs::new(Options {
                dirs_merger: Some(language_dirs_merger()),
                ..Default::default()
            }),
            overlay_mounts_static: OverlayFs::new(Options {
                dirs_merger: Some(language_dirs_merger()),
                ..Default::default()
            }),
            overlay_mounts_full: None,
            overlay_full: OverlayFs::new(Options::default()),
            overlay_resources: OverlayFs::new(Options {
                first_writable: true,
                ..Default::default()
            }),
            root_fss: Vec::new(),
        };

        let mods = p.all_modules();

        let mut mounts = Vec::with_capacity(mods.len());

        for (i, m) in mods.iter().enumerate() {
            let dir = m.dir.clone();

            let is_main_project = m.owner.is_none();
            mounts.push(MountsDescriptor {
                module: m.clone(),
                dir,
                is_main_project,
                ordinal: i as i64,
            });
        }

        self.create_overlay_fs(&mut collector, &mounts)?;

        Ok(collector)
    }

    // Go: hugolib/filesystems/basefs.go:isContentMount
    fn is_content_mount(&self, mnt: &Mount) -> bool {
        mnt.target.starts_with(COMPONENT_FOLDER_CONTENT)
    }

    // Go: hugolib/filesystems/basefs.go:isStaticMount
    fn is_static_mount(&self, mnt: &Mount) -> bool {
        mnt.target.starts_with(COMPONENT_FOLDER_STATIC)
    }

    // Go: hugolib/filesystems/basefs.go:createOverlayFs
    fn create_overlay_fs(
        &self,
        collector: &mut FilesystemsCollector,
        mounts: &[MountsDescriptor],
    ) -> Result<()> {
        if mounts.is_empty() {
            let append_nop_if_empty = |ofs: &OverlayFs| -> OverlayFs {
                if ofs.num_filesystems() > 0 {
                    return ofs.clone();
                }
                ofs.append(vec![no_op_fs()])
            };
            collector.overlay_mounts = append_nop_if_empty(&collector.overlay_mounts);
            collector.overlay_mounts_content =
                append_nop_if_empty(&collector.overlay_mounts_content);
            collector.overlay_mounts_static = append_nop_if_empty(&collector.overlay_mounts_static);
            // Go: appendNopIfEmpty(nil) calls NumFilesystems on a nil *OverlayFs (0) and appends
            // to a copy of the zero OverlayFs.
            collector.overlay_mounts_full = Some(append_nop_if_empty(
                collector
                    .overlay_mounts_full
                    .as_ref()
                    .unwrap_or(&OverlayFs {
                        fss: Vec::new(),
                        first_writable: false,
                        merge_dirs: crate::overlayfs::default_dir_merger(),
                    }),
            ));
            collector.overlay_full = append_nop_if_empty(&collector.overlay_full);
            collector.overlay_resources = append_nop_if_empty(&collector.overlay_resources);

            return Ok(());
        }

        for md in mounts {
            let mut from_to: Vec<RootMapping> = Vec::new();
            let mut from_to_content: Vec<RootMapping> = Vec::new();
            let mut from_to_static: Vec<RootMapping> = Vec::new();

            let abs_pathify = |path: &str| -> (String, String) {
                if go_path::filepath::is_abs(path) {
                    return (String::new(), path.to_string());
                }
                (md.dir.clone(), hpaths::abs_pathify(&md.dir, path))
            };

            let md_mounts = &md.module.mounts;
            for (i, mount) in md_mounts.iter().enumerate() {
                // Add more weight to early mounts.
                // When two mounts contain the same filename,
                // the first entry wins.
                let mount_weight = (10 + md.ordinal) * (md_mounts.len() as i64 - i as i64);

                let inclusion_filter =
                    FilenameFilter::new(&mount.include_files, &mount.exclude_files)?;

                let (base, filename) = abs_pathify(&mount.source);

                let mut rm = RootMapping {
                    from: mount.target.clone(),
                    to: filename,
                    to_base: base,
                    module: md.module.path.clone(),
                    module_ordinal: md.ordinal,
                    is_project: md.is_main_project,
                    meta: Arc::new(FileMeta {
                        watch: !mount.disable_watch && md.module.watch,
                        weight: mount_weight,
                        inclusion_filter,
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                let is_content_mount = self.is_content_mount(mount);

                let mut lang = mount.lang.clone();
                if lang.is_empty() && is_content_mount {
                    lang = self.p.cfg.default_content_language();
                }

                Arc::make_mut(&mut rm.meta).lang = lang;

                if is_content_mount {
                    from_to_content.push(rm);
                } else if self.is_static_mount(mount) {
                    from_to_static.push(rm);
                } else {
                    from_to.push(rm);
                }
            }

            let mod_base = if md.is_main_project {
                collector.source_project.clone()
            } else {
                collector.source_modules.clone()
            };

            let source_static = mod_base.clone();

            let rmfs = RootMappingFs::new(mod_base.clone(), from_to)?;
            let rmfs_content = RootMappingFs::new(mod_base.clone(), from_to_content)?;
            let rmfs_static = RootMappingFs::new(source_static, from_to_static)?;

            // We need to keep the list of directories for watching.
            collector.add_root_fs(rmfs.clone());
            collector.add_root_fs(rmfs_content.clone());
            collector.add_root_fs(rmfs_static.clone());

            if let Some(static_per_language) = collector.static_per_language.as_mut() {
                for l in self.p.cfg.languages() {
                    let lang = l.lang.clone();

                    let lfs = rmfs_static.filter(&|rm: &RootMapping| {
                        let rlang = &rm.meta.lang;
                        rlang.is_empty() || *rlang == lang
                    });
                    let bfs = new_base_path_fs(lfs, COMPONENT_FOLDER_STATIC);
                    let cur = static_per_language
                        .get(&lang)
                        .cloned()
                        .unwrap_or_else(|| OverlayFs::new(Options::default()));
                    static_per_language.insert(lang, cur.append(vec![bfs]));
                }
            }

            let get_resources_dir = || -> String {
                if md.is_main_project {
                    return self.p.abs_resources_dir.clone();
                }
                let (_, filename) = abs_pathify(FOLDER_RESOURCES);
                filename
            };

            collector.overlay_mounts = collector.overlay_mounts.append(vec![rmfs]);
            collector.overlay_mounts_content =
                collector.overlay_mounts_content.append(vec![rmfs_content]);
            collector.overlay_mounts_static =
                collector.overlay_mounts_static.append(vec![rmfs_static]);
            collector.overlay_full = collector
                .overlay_full
                .append(vec![new_base_path_fs(mod_base.clone(), &md.dir)]);
            collector.overlay_resources = collector
                .overlay_resources
                .append(vec![new_base_path_fs(mod_base, &get_resources_dir())]);
        }

        Ok(())
    }
}

/// Go: `printFs(fs, path, w)` — debugging helper (not ported).
// Go: hugolib/filesystems/basefs.go:printFs
#[allow(dead_code)]
fn print_fs(_fs: &dyn Fs, _path: &str) {}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/filesystems/basefs.go (852 lines; 16/29 funcs executed)
//   types: BaseFs, Lockable, fakeLockfileMutex, SourceFilesystems, SourceFilesystem, sourceFilesystemsBuilder,
//          filesystemsCollector, mountsDescriptor
// OK L95-98: (f *fakeLockfileMutex) Lock() (func(), error)
// OK L101-103: (b *BaseFs) LockBuild() (unlock func(), err error)
// OK L105-150: (b *BaseFs) WatchFilenames() []string
// OK L152-161: (b *BaseFs) mountsForComponent(component string) []hugofs.FileMetaInfo
// OK L165-208: (b *BaseFs) AbsProjectContentDir(filename string) (string, string, error)
// OK L212-225: (b *BaseFs) ResolveJSConfigFile(name string) string
// OK L279-289: (s SourceFilesystems) StaticFs(lang string) afero.Fs
// OK L297-307: (s SourceFilesystems) StatResource(lang, filename string) (fi os.FileInfo, fs afero.Fs, err error)
// OK L311-318: (s SourceFilesystems) IsStatic(filename string) bool
// OK L321-323: (s SourceFilesystems) IsContent(filename string) bool
// OK L326-336: (s *SourceFilesystems) ResolvePaths(filename string) []hugofs.ComponentPath
// OK L340-348: (s SourceFilesystems) MakeStaticPathRelative(filename string) string
// OK L351-361: (d *SourceFilesystem) MakePathRelative(filename string, checkExists bool) (string, bool)
// OK L364-385: (d *SourceFilesystem) ReverseLookup(filename string, checkExists bool) ([]hugofs.ComponentPath, error)
// OK L387-411: (d *SourceFilesystem) mounts() []hugofs.FileMetaInfo
// OK L413-423: (d *SourceFilesystem) RealFilename(rel string) string
// OK L426-436: (d *SourceFilesystem) Contains(filename string) bool
// OK L440-452: (d *SourceFilesystem) RealDirs(from string) []string
// OK L456-462: WithBaseFs(b *BaseFs) func(*BaseFs) error (BaseFs::new_with_base)
// OK L465-513: NewBase(p *paths.Paths, logger loggers.Logger, options ...func(*BaseFs) error) (*BaseFs, error)
// OK L523-531: newSourceFilesystemsBuilder(p *paths.Paths, logger loggers.Logger, b *BaseFs) *sourceFilesystemsBuilder
// OK L533-539: (b *sourceFilesystemsBuilder) newSourceFilesystem(name string, fs afero.Fs) *SourceFilesystem
// OK L541-609: (b *sourceFilesystemsBuilder) Build() (*SourceFilesystems, error)
// OK L611-654: (b *sourceFilesystemsBuilder) createMainOverlayFs(p *paths.Paths) (*filesystemsCollector, error)
// OK L656-658: (b *sourceFilesystemsBuilder) isContentMount(mnt modules.Mount) bool
// OK L660-662: (b *sourceFilesystemsBuilder) isStaticMount(mnt modules.Mount) bool
// OK L664-802: (b *sourceFilesystemsBuilder) createOverlayFs( collector *filesystemsCollector, mounts []mountsDescriptor, ) error
// OK L805-824: printFs(fs afero.Fs, path string, w io.Writer) (debug only; empty)
// OK L843-845: (c *filesystemsCollector) addRootFs(rfs *hugofs.RootMappingFs)
// ---------------------------------------------------------------------------
