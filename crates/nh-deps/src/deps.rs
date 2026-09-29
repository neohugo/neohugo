//! Port of `deps/deps.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture), crate lead of nh-deps.

//! Go `deps.Deps`: the per-site dependency container — the Rust "context struct" handed to every
//! component that Go gives a `*deps.Deps` (template namespaces, i18n, content spec...).
//!
//! Construction order (Go `NewHugoSites` -> `firstSiteDeps.Init()` -> `Clone(site, conf)` per site
//! -> `Compile(prototype)` = i18n `NewResource`/`CloneResource`): hexec, publish fs wrapped with
//! the HasBytes receiver, PathSpec (first site creates BaseFs, others share it), ContentSpec,
//! SourceSpec, file caches, resources Spec (SpecCommon shared; its PostProcess incrementer IS
//! this build's `BuildState`). Owned by T20 because construction is part of capture.
//!
//! The service fields are `Option`s, like Go's nil-able pointers: `init`/`clone_for` set all of
//! them; [`Deps::for_tests`] sets none and tests add what they need with the `with_*` builders.
//! The accessors panic on a missing service (an invariant in a real build, a test setup error
//! otherwise). Fields that need the frozen `HugoSites` (the `page.Site`) are `OnceLock`s set by
//! nh-hugolib once it is frozen; the template store and the translate func are set during
//! construction (`NewHugoSites`), like Go.
//!
//! Go's `Compile(prototype)` (i18n `NewResource`/`CloneResource`) is called by nh-hugolib
//! directly on `nh_i18n::translation_provider::TranslationProvider`: nh-i18n depends on this
//! crate, so the provider cannot be a field here (Go `Deps.TranslationProvider`); `HugoSites`
//! owns it (`nh_hugolib::hugo_sites::compile_deps`).

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use go_value::{HostCtx, Value};
use nh_common::Result;
use nh_common::dynacache::{Cache as MemCache, Options as MemCacheOptions};
use nh_common::herrors::Error;
use nh_common::identity::Incrementer;
use nh_common::loggers::{Level, Logger};
use nh_config::config_provider::{AllProvider, config_section};
use nh_config::hexec::Exec;
use nh_config::security::security_config::Config as SecurityConfig;
use nh_helpers::content::ContentSpec;
use nh_helpers::pathspec::PathSpec;
use nh_helpers::source::source_spec::SourceSpec;
use nh_hugofs::fs::Fs;
use nh_media::media::config::ContentTypes;
use nh_media::media::media_type::Types as MediaTypes;
use nh_page::site::SiteRef;
use nh_resources::resource_spec::Spec as ResourceSpec;
use nh_tplimpl::templatestore::TemplateStore;

/// Go: `Deps.Translate func(ctx, translationID, templateData) string`.
///
/// Deviation: the Rust func returns a `Result`. Go's translate func panics for a few inputs
/// (`i18n "x" ""`, an untyped nil `Count`, a nil pointer as data) and text/template turns the
/// panic into the error of the `i18n`/`T` call; the `Err` carries Go's panic message.
pub type TranslateFunc = Arc<dyn Fn(HostCtx<'_>, &str, &Value) -> Result<String> + Send + Sync>;

/// Go: `deps.Deps`.
pub struct Deps {
    pub log: Logger,
    /// The per-language config (`config.AllProvider`, concretely `allconfig.ConfigLanguage`).
    pub conf: Arc<dyn AllProvider>,
    pub exec_helper: Option<Arc<Exec>>,
    pub fs: Option<Fs>,
    pub path_spec: Option<Arc<PathSpec>>,
    pub content_spec: Option<Arc<ContentSpec>>,
    pub source_spec: Option<Arc<SourceSpec>>,
    pub resource_spec: Option<Arc<ResourceSpec>>,
    /// Go `MemCache`: the build's dynacache, shared by all sites.
    pub mem_cache: Arc<MemCache>,
    /// Set by the i18n translation provider (Go `TranslationProvider.NewResource`).
    pub translate: OnceLock<TranslateFunc>,
    /// Go `Deps.Site` (a `page.Site`); set once HugoSites is frozen.
    pub site: Arc<OnceLock<SiteRef>>,
    /// Go `Deps.TemplateStore` (this site's view).
    pub template_store: OnceLock<TemplateStore>,
    /// Shared by all sites of the build.
    pub build_state: Arc<BuildState>,
    pub counters: Arc<Counters>,
    /// Go `BuildStartListeners` (shared by all sites: Go's `Clone` copies the pointer).
    pub build_start_listeners: Arc<Listeners<()>>,
    /// Go `BuildEndListeners` (shared by all sites).
    pub build_end_listeners: Arc<Listeners<()>>,
    /// Go `*globalErrHandler` (embedded; shared by all sites).
    pub global_err_handler: Arc<GlobalErrHandler>,
    is_closed: AtomicBool,
}

impl Deps {
    /// Go: `Deps.Init()` for the first site (`firstSiteDeps.Init()` in `NewHugoSites`): the
    /// publish fs wrapped with the HasBytes receiver, then the PathSpec (which creates the
    /// BaseFs), ContentSpec, SourceSpec, the file caches and the resources Spec (a new
    /// `SpecCommon` whose PostProcess incrementer is `cfg.build_state`).
    // Go: deps/deps.go:Init
    pub fn init(cfg: DepsCfg) -> Result<Arc<Deps>> {
        let DepsCfg {
            fs: mut dfs,
            conf,
            log,
            build_state,
            mem_cache,
            counters,
        } = cfg;
        let global_err_handler = Arc::new(GlobalErrHandler::new(log.clone()));

        let exec_helper = new_exec_helper(&*conf);

        // PathSpec == nil: wrap the publish dir, then create the PathSpec (and its BaseFs).
        let bs = build_state.clone();
        let has_bytes_receiver_func = Arc::new(move |name: &str, m: &[u8]| {
            if m == nh_resources::postpub::postpub::POST_PROCESS_PREFIX.as_bytes() {
                bs.add_filename_with_post_prefix(name);
            } else if m == nh_tpl::template::HUGO_DEFERRED_TEMPLATE_PREFIX.as_bytes() {
                bs.add_filename_with_deferred_prefix(name);
            }
        });
        // Skip binary files.
        let media_types = config_section::<MediaTypes>(&*conf, "mediaTypes");
        let has_bytes_should_check = Arc::new(move |name: &str| {
            let ext = go_path::filepath::ext(name);
            media_types.is_text_suffix(ext.strip_prefix('.').unwrap_or(ext))
        });
        dfs.publish_dir = nh_hugofs::hasbytes_fs::new_has_bytes_receiver(
            dfs.publish_dir.clone(),
            has_bytes_should_check,
            has_bytes_receiver_func,
            vec![
                nh_tpl::template::HUGO_DEFERRED_TEMPLATE_PREFIX
                    .as_bytes()
                    .to_vec(),
                nh_resources::postpub::postpub::POST_PROCESS_PREFIX
                    .as_bytes()
                    .to_vec(),
            ],
        );
        let path_spec = PathSpec::new(dfs.clone(), conf.clone())?;

        let mut d = Deps {
            log,
            conf,
            exec_helper: Some(exec_helper),
            fs: Some(dfs),
            path_spec: Some(path_spec),
            content_spec: None,
            source_spec: None,
            resource_spec: None,
            mem_cache,
            translate: OnceLock::new(),
            site: Arc::new(OnceLock::new()),
            template_store: OnceLock::new(),
            build_state,
            counters,
            build_start_listeners: Arc::new(Listeners::default()),
            build_end_listeners: Arc::new(Listeners::default()),
            global_err_handler,
            is_closed: AtomicBool::new(false),
        };
        d.init_rest()?;
        Ok(Arc::new(d))
    }

    /// Go: `Deps.Clone(site, conf)` + its `Init()` — another site's deps. The copy shares the fs
    /// (with the wrapped publish dir), the BaseFs (`NewPathSpecWithBaseBaseFsProvided`), the
    /// SourceSpec (Go's `Init` keeps the copied non-nil one: the FIRST site's), the memory
    /// cache, the build state, the counters, the listeners, the error handler and the resources
    /// `SpecCommon`; it gets a new exec helper (Go sets it to nil before `Init`), ContentSpec
    /// (likewise), file caches and resources Spec for `conf`.
    // Go: deps/deps.go:Clone
    pub fn clone_for(&self, conf: Arc<dyn AllProvider>) -> Result<Arc<Deps>> {
        let fs = self.fs().clone();
        let base_fs = self.path_spec().base_fs.clone();
        let exec_helper = new_exec_helper(&*conf);
        // PathSpec != nil: a new PathSpec for conf over the first site's BaseFs.
        let path_spec = PathSpec::new_with_base_fs(fs.clone(), conf.clone(), Some(base_fs))?;
        let mut d = Deps {
            log: self.log.clone(),
            conf,
            exec_helper: Some(exec_helper),
            fs: Some(fs),
            path_spec: Some(path_spec),
            content_spec: None,
            source_spec: self.source_spec.clone(),
            resource_spec: self.resource_spec.clone(),
            mem_cache: self.mem_cache.clone(),
            translate: OnceLock::new(),
            site: Arc::new(OnceLock::new()),
            template_store: OnceLock::new(),
            build_state: self.build_state.clone(),
            counters: self.counters.clone(),
            build_start_listeners: self.build_start_listeners.clone(),
            build_end_listeners: self.build_end_listeners.clone(),
            global_err_handler: self.global_err_handler.clone(),
            is_closed: AtomicBool::new(false),
        };
        d.init_rest()?;
        Ok(Arc::new(d))
    }

    /// The part of Go's `Init()` after the PathSpec: ContentSpec (when nil), SourceSpec (when
    /// nil), the file caches and a new resources Spec (sharing the `SpecCommon` of the one
    /// already set, if any).
    // Go: deps/deps.go:Init
    fn init_rest(&mut self) -> Result<()> {
        let path_spec = self.path_spec().clone();
        if self.content_spec.is_none() {
            let content_types = config_section::<ContentTypes>(&*self.conf, "contentTypes");
            let content_spec = ContentSpec::new_with_content_types(
                self.conf.clone(),
                self.exec_helper().clone(),
                Some(Arc::new(self.log.clone())),
                (*content_types).clone(),
            )?;
            self.content_spec = Some(content_spec);
        }

        if self.source_spec.is_none() {
            self.source_spec = Some(SourceSpec::new(
                path_spec.clone(),
                None,
                self.fs().source.clone(),
            ));
        }

        let common = self.resource_spec.as_ref().map(|rs| rs.common.clone());

        let file_caches =
            nh_helpers::cache::filecache::filecache::new_caches(&path_spec).map_err(|err| {
                Error::new(format!(
                    "failed to create file caches from configuration: {}",
                    err.message()
                ))
            })?;

        let incr: Arc<dyn Incrementer> = self.build_state.clone();
        let resource_spec = ResourceSpec::new(
            path_spec,
            common,
            file_caches,
            &self.mem_cache,
            Some(incr),
            self.exec_helper().clone(),
            Some(self.log.clone()),
        )
        .map_err(|err| Error::new(format!("failed to create resource spec: {}", err.message())))?;
        // Go passes `d` (its embedded `*globalErrHandler`) as the resources error handler.
        let geh = self.global_err_handler.clone();
        let _ = resource_spec
            .error_sender
            .set(Arc::new(move |err: &Error| geh.send_error(err.clone())));
        self.resource_spec = Some(resource_spec);

        Ok(())
    }

    /// A `Deps` with only `conf` (and a warn-level logger, fresh `BuildState`/`Counters`): the
    /// test constructor for tasks that must not wait for the full construction path (T15, T17,
    /// T18, T19; HUGO_LAYER.md §11.1). Mirrors Go tests that build `&deps.Deps{Cfg:.., Log:..}`
    /// with nil services. Add services with the `with_*` builders before wrapping in `Arc`.
    pub fn for_tests(conf: Arc<dyn AllProvider>) -> Deps {
        let log = Logger::new(Level::Warn, BTreeSet::new());
        Deps {
            global_err_handler: Arc::new(GlobalErrHandler::new(log.clone())),
            log,
            conf,
            exec_helper: None,
            fs: None,
            path_spec: None,
            content_spec: None,
            source_spec: None,
            resource_spec: None,
            mem_cache: Arc::new(MemCache::new(MemCacheOptions::default())),
            translate: OnceLock::new(),
            site: Arc::new(OnceLock::new()),
            template_store: OnceLock::new(),
            build_state: Arc::new(BuildState::default()),
            counters: Arc::new(Counters::default()),
            build_start_listeners: Arc::new(Listeners::default()),
            build_end_listeners: Arc::new(Listeners::default()),
            is_closed: AtomicBool::new(false),
        }
    }

    pub fn with_log(mut self, log: Logger) -> Deps {
        self.global_err_handler = Arc::new(GlobalErrHandler::new(log.clone()));
        self.log = log;
        self
    }
    pub fn with_exec_helper(mut self, e: Arc<Exec>) -> Deps {
        self.exec_helper = Some(e);
        self
    }
    pub fn with_fs(mut self, fs: Fs) -> Deps {
        self.fs = Some(fs);
        self
    }
    pub fn with_path_spec(mut self, ps: Arc<PathSpec>) -> Deps {
        self.path_spec = Some(ps);
        self
    }
    pub fn with_content_spec(mut self, cs: Arc<ContentSpec>) -> Deps {
        self.content_spec = Some(cs);
        self
    }
    pub fn with_source_spec(mut self, ss: Arc<SourceSpec>) -> Deps {
        self.source_spec = Some(ss);
        self
    }
    pub fn with_resource_spec(mut self, rs: Arc<ResourceSpec>) -> Deps {
        self.resource_spec = Some(rs);
        self
    }

    pub fn exec_helper(&self) -> &Arc<Exec> {
        self.exec_helper.as_ref().expect("Deps.exec_helper not set")
    }
    pub fn fs(&self) -> &Fs {
        self.fs.as_ref().expect("Deps.fs not set")
    }
    pub fn path_spec(&self) -> &Arc<PathSpec> {
        self.path_spec.as_ref().expect("Deps.path_spec not set")
    }
    pub fn content_spec(&self) -> &Arc<ContentSpec> {
        self.content_spec
            .as_ref()
            .expect("Deps.content_spec not set")
    }
    pub fn source_spec(&self) -> &Arc<SourceSpec> {
        self.source_spec.as_ref().expect("Deps.source_spec not set")
    }
    pub fn resource_spec(&self) -> &Arc<ResourceSpec> {
        self.resource_spec
            .as_ref()
            .expect("Deps.resource_spec not set")
    }

    /// Go: `Deps.GetTemplateStore()`.
    // Go: deps/deps.go:GetTemplateStore
    pub fn get_template_store(&self) -> &TemplateStore {
        self.template_store.get().expect("template store not set")
    }

    /// The site (panics before HugoSites is frozen — no template runs before that).
    pub fn site(&self) -> &SiteRef {
        self.site.get().expect("site not set")
    }

    /// Go: `d.Translate(ctx, id, data)`. Without a translate func (Go: a nil func, only in
    /// tests) the result is empty.
    pub fn translate(&self, ctx: HostCtx<'_>, id: &str, data: &Value) -> Result<String> {
        match self.translate.get() {
            Some(f) => f(ctx, id, data),
            None => Ok(String::new()),
        }
    }

    /// Go: `d.SendError(err)` (the embedded `*globalErrHandler`).
    pub fn send_error(&self, err: Error) {
        self.global_err_handler.send_error(err);
    }

    /// Go: `Deps.Close()` — once; stops the memory cache and runs the build closers. Neither
    /// has work in the port (the dynacache has no background goroutine; no build closer is
    /// registered on the ported paths).
    // Go: deps/deps.go:Close
    pub fn close(&self) -> Result<()> {
        if self.is_closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        Ok(())
    }
}

/// Go: `hexec.New(d.Conf.GetConfigSection("security").(security.Config), d.Conf.WorkingDir(),
/// d.Log)`.
fn new_exec_helper(conf: &dyn AllProvider) -> Arc<Exec> {
    let sc = config_section::<SecurityConfig>(conf, "security");
    Exec::new((*sc).clone(), &conf.working_dir())
}

/// Go: `deps.DepsCfg` + the fields of the first site's `Deps` literal in `NewHugoSites` (Fs,
/// Log, Conf, BuildState, Counters, MemCache): the inputs of [`Deps::init`].
pub struct DepsCfg {
    pub fs: Fs,
    pub conf: Arc<dyn AllProvider>,
    pub log: Logger,
    pub build_state: Arc<BuildState>,
    /// Go `MemCache` (`dynacache.New(...)` in `NewHugoSites`).
    pub mem_cache: Arc<MemCache>,
    pub counters: Arc<Counters>,
}

/// Go: `deps.BuildState` (one per build, shared by all sites).
#[derive(Default)]
pub struct BuildState {
    /// Go `counter`: the PostProcess placeholder ids (via `Incrementer`, stored in the shared
    /// `resources.SpecCommon`). The only such counter in the port.
    pub(crate) counter: AtomicU64,
    /// Files in /public that contain a post-processing placeholder (`__h_pp_l1`).
    pub(crate) filenames_with_post_prefix: Mutex<BTreeSet<String>>,
    /// Go `DeferredExecutions`: the deferred executions of the rendering stage in progress
    /// (replaced by a fresh set when a stage stops).
    pub(crate) deferred_executions: Mutex<Arc<DeferredExecutions>>,
    /// Go `DeferredExecutionsGroupedByRenderingContext` (a map in Go, iterated in random order
    /// by `renderDeferred`; the port keeps the stages in render order, one of Go's orders).
    pub(crate) deferred_executions_grouped: Mutex<Vec<(RenderingContext, Arc<DeferredExecutions>)>>,
}

impl BuildState {
    // Go: deps/deps.go:AddFilenameWithPostPrefix
    pub fn add_filename_with_post_prefix(&self, filename: &str) {
        self.filenames_with_post_prefix
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(filename.to_string());
    }

    /// Go: `GetFilenamesWithPostPrefix()` (sorted).
    // Go: deps/deps.go:GetFilenamesWithPostPrefix
    pub fn get_filenames_with_post_prefix(&self) -> Vec<String> {
        self.filenames_with_post_prefix
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    /// Go: `DeferredExecutions.FilenamesWithPostPrefix.Set(name, true)` (the HasBytes receiver
    /// callback for `__hdeferred/`): recorded in the current stage's executions.
    pub fn add_filename_with_deferred_prefix(&self, filename: &str) {
        self.deferred_executions()
            .filenames_with_post_prefix
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(filename.to_string());
    }

    /// The files with a deferred-template placeholder of the current stage (sorted).
    pub fn get_filenames_with_deferred_prefix(&self) -> Vec<String> {
        self.deferred_executions().filenames()
    }

    /// Go: `BuildState.DeferredExecutions` (the current stage's).
    pub fn deferred_executions(&self) -> Arc<DeferredExecutions> {
        self.deferred_executions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Go: `StartStageRender(stage)` (a no-op).
    // Go: deps/deps.go:StartStageRender
    pub fn start_stage_render(&self, _stage: RenderingContext) {}

    /// Go: `StopStageRender(stage)`: the stage's executions are grouped under the stage (a map
    /// assignment in Go: the same stage replaces its earlier group) and a fresh set starts.
    // Go: deps/deps.go:StopStageRender
    pub fn stop_stage_render(&self, stage: RenderingContext) {
        let de = std::mem::take(
            &mut *self
                .deferred_executions
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        );
        let mut grouped = self
            .deferred_executions_grouped
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match grouped.iter_mut().find(|(rc, _)| *rc == stage) {
            Some(g) => g.1 = de,
            None => grouped.push((stage, de)),
        }
    }

    /// Go: `DeferredExecutionsGroupedByRenderingContext` (in render order).
    pub fn deferred_executions_grouped(&self) -> Vec<(RenderingContext, Arc<DeferredExecutions>)> {
        self.deferred_executions_grouped
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

/// Go: `tpl.RenderingContext{Site, SiteOutIdx}` (the site by index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderingContext {
    pub site_idx: usize,
    pub site_out_idx: usize,
}

/// Go: `deps.DeferredExecutions`.
#[derive(Default)]
pub struct DeferredExecutions {
    /// Go `FilenamesWithPostPrefix`: the files in /public that contain a deferred placeholder.
    pub filenames_with_post_prefix: Mutex<BTreeSet<String>>,
    /// Go `Executions`: placeholder id -> deferred execution.
    pub executions: Mutex<std::collections::HashMap<String, Arc<DeferredExecution>>>,
}

impl DeferredExecutions {
    /// The recorded filenames (sorted; Go's `ForEeach` over a map is unordered, and each file is
    /// handled on its own).
    pub fn filenames(&self) -> Vec<String> {
        self.filenames_with_post_prefix
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    /// Go: `Executions.GetOrCreate(id, create)` (the first creator wins).
    pub fn get_or_create(
        &self,
        id: &str,
        create: impl FnOnce() -> DeferredExecution,
    ) -> Arc<DeferredExecution> {
        self.executions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(create()))
            .clone()
    }

    /// Go: `Executions.Get(id)`.
    pub fn get(&self, id: &str) -> Option<Arc<DeferredExecution>> {
        self.executions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned()
    }
}

/// Go: `tpl.DeferredExecution`: the template and data of a deferred execution; `result` is
/// `Some` once executed (Go's `Executed` + `Result`, under `Mu`).
pub struct DeferredExecution {
    pub template_path: String,
    pub ctx: nh_tpl::template::TplContext,
    pub data: Value,
    pub result: Mutex<Option<Vec<u8>>>,
}

impl Incrementer for BuildState {
    /// Go: `Incr()` — PostProcess placeholder ids start at 1.
    // Go: deps/deps.go:Incr
    fn incr(&self) -> i64 {
        (self.counter.fetch_add(1, Ordering::SeqCst) + 1) as i64
    }
}

/// Go: `deps.Counters`.
#[derive(Default)]
pub struct Counters {
    pub math_counter: AtomicU64,
}

/// Go: `globalErrHandler` — collects the "hard to get to" build errors (e.g. from lazy resource
/// transformations) while a build runs, and logs them otherwise.
pub struct GlobalErrHandler {
    logger: Logger,
    /// Go `buildErrors` (a channel whose reader in `HugoSites.Build` keeps the first 50 errors);
    /// `None` when no collector runs.
    build_errors: Mutex<Option<Vec<Error>>>,
}

/// Go's error reader in `HugoSites.Build` stops after this many errors.
const MAX_COLLECTED_ERRORS: usize = 50;

impl GlobalErrHandler {
    pub fn new(logger: Logger) -> GlobalErrHandler {
        GlobalErrHandler {
            logger,
            build_errors: Mutex::new(None),
        }
    }

    /// SendError sends the error on a channel to be handled later. This can be used in
    /// situations where returning and aborting the current operation isn't practical.
    // Go: deps/deps.go:SendError
    pub fn send_error(&self, err: Error) {
        let mut guard = self.build_errors.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(errs) = guard.as_mut() {
            if errs.len() < MAX_COLLECTED_ERRORS {
                errs.push(err);
            }
            return;
        }
        drop(guard);
        self.logger.errorf(err.to_string());
    }

    /// Go: `StartErrorCollector()`; the errors are read with
    /// [`GlobalErrHandler::stop_error_collector`].
    // Go: deps/deps.go:StartErrorCollector
    pub fn start_error_collector(&self) {
        *self.build_errors.lock().unwrap_or_else(|e| e.into_inner()) = Some(Vec::new());
    }

    /// Go: `StopErrorCollector()` + draining the channel: the collected errors in send order.
    // Go: deps/deps.go:StopErrorCollector
    pub fn stop_error_collector(&self) -> Vec<Error> {
        self.build_errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .unwrap_or_default()
    }
}

/// A listener func: called with the notified values; returning true removes it.
pub type ListenerFunc<T> = Box<dyn FnMut(&[T]) -> bool + Send>;

/// Go: `deps.Listeners[T]` — an event listener.
pub struct Listeners<T> {
    /// A list of funcs to be notified about an event. If the return value is true, the
    /// listener will be removed.
    listeners: Mutex<Vec<ListenerFunc<T>>>,
}

impl<T> Default for Listeners<T> {
    fn default() -> Self {
        Listeners {
            listeners: Mutex::new(Vec::new()),
        }
    }
}

impl<T> Listeners<T> {
    /// Add adds a function to a Listeners instance.
    // Go: deps/deps.go:Add
    pub fn add(&self, f: ListenerFunc<T>) {
        self.listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(f);
    }

    /// Notify executes all listener functions (Go holds the lock while they run).
    // Go: deps/deps.go:Notify
    pub fn notify(&self, vs: &[T]) {
        let mut guard = self.listeners.lock().unwrap_or_else(|e| e.into_inner());
        let listeners = std::mem::take(&mut *guard);
        for mut notify in listeners {
            if !notify(vs) {
                guard.push(notify);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: deps/deps.go (498 lines; 13/17 funcs executed)
//   types: Deps, globalErrHandler, Listeners[T, ResourceProvider, DepsCfg, BuildState, Counters, DeferredExecutions
// OK L116-127: (d Deps) Clone(s page.Site, conf config.AllProvider) (*Deps, error)
// OK L129-131: (d *Deps) GetTemplateStore() *tplimpl.TemplateStore
// OK L133-262: (d *Deps) Init() error
// OK L265-280: (d *Deps) Compile(prototype *Deps) error  [nh-hugolib hugo_sites.rs compile_deps]
//    L283-297: (d Deps) MkdirTemp(pattern string) (string, error)
// OK L311-321: (e *globalErrHandler) SendError(err error)
// OK L323-327: (e *globalErrHandler) StartErrorCollector() chan error
// OK L329-334: (e *globalErrHandler) StopErrorCollector()
// OK L346-353: (b *Listeners[T]) Add(f func(...T) bool)
// OK L356-366: (b *Listeners[T]) Notify(vs ...T)
// OK L374-387: (d *Deps) Close() error
// OK L460-461: (b *BuildState) StartStageRender(stage tpl.RenderingContext)
// OK L464-470: (b *BuildState) StopStageRender(stage tpl.RenderingContext)
//    L472-474: (b *BuildState) SignalRebuild(ids ...identity.Identity)
// OK L476-483: (b *BuildState) AddFilenameWithPostPrefix(filename string)
// OK L485-494: (b *BuildState) GetFilenamesWithPostPrefix() []string
// OK L496-498: (b *BuildState) Incr() int
// ---------------------------------------------------------------------------
