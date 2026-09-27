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
//! otherwise). Fields that need the frozen `HugoSites` (the `page.Site`, the template store, the
//! translate func) are `OnceLock`s set by nh-hugolib before rendering.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use go_value::{HostCtx, Value};
use nh_common::identity::Incrementer;
use nh_common::loggers::{Level, Logger};
use nh_common::Result;
use nh_config::config_provider::AllProvider;
use nh_config::hexec::Exec;
use nh_helpers::content::ContentSpec;
use nh_helpers::pathspec::PathSpec;
use nh_helpers::source::source_spec::SourceSpec;
use nh_hugofs::fs::Fs;
use nh_page::site::SiteRef;
use nh_resources::resource_spec::Spec as ResourceSpec;
use nh_tplimpl::templatestore::TemplateStore;

/// Go: `Deps.Translate func(ctx, translationID, templateData) string`.
pub type TranslateFunc = Arc<dyn Fn(HostCtx<'_>, &str, &Value) -> String + Send + Sync>;

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
    /// Set by the i18n translation provider (Go `TranslationProvider.NewResource`).
    pub translate: OnceLock<TranslateFunc>,
    /// Go `Deps.Site` (a `page.Site`); set once HugoSites is frozen.
    pub site: Arc<OnceLock<SiteRef>>,
    /// Go `Deps.TemplateStore` (this site's view).
    pub template_store: OnceLock<TemplateStore>,
    /// Shared by all sites of the build.
    pub build_state: Arc<BuildState>,
    pub counters: Arc<Counters>,
}

impl Deps {
    /// Go: `Deps.Init()` (first site).
    // Go: deps/deps.go:Init
    pub fn init(cfg: DepsCfg) -> Result<Arc<Deps>> {
        todo!()
    }

    /// Go: `Deps.Clone(site, conf)` (other sites share BaseFs, SpecCommon, BuildState, MemCache).
    // Go: deps/deps.go:Clone
    pub fn clone_for(&self, conf: Arc<dyn AllProvider>) -> Result<Arc<Deps>> {
        todo!()
    }

    /// A `Deps` with only `conf` (and a warn-level logger, fresh `BuildState`/`Counters`): the
    /// test constructor for tasks that must not wait for the full construction path (T15, T17,
    /// T18, T19; HUGO_LAYER.md §11.1). Mirrors Go tests that build `&deps.Deps{Cfg:.., Log:..}`
    /// with nil services. Add services with the `with_*` builders before wrapping in `Arc`.
    pub fn for_tests(conf: Arc<dyn AllProvider>) -> Deps {
        Deps {
            log: Logger::new(Level::Warn, BTreeSet::new()),
            conf,
            exec_helper: None,
            fs: None,
            path_spec: None,
            content_spec: None,
            source_spec: None,
            resource_spec: None,
            translate: OnceLock::new(),
            site: Arc::new(OnceLock::new()),
            template_store: OnceLock::new(),
            build_state: Arc::new(BuildState::default()),
            counters: Arc::new(Counters::default()),
        }
    }

    pub fn with_log(mut self, log: Logger) -> Deps {
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
        self.content_spec.as_ref().expect("Deps.content_spec not set")
    }
    pub fn source_spec(&self) -> &Arc<SourceSpec> {
        self.source_spec.as_ref().expect("Deps.source_spec not set")
    }
    pub fn resource_spec(&self) -> &Arc<ResourceSpec> {
        self.resource_spec.as_ref().expect("Deps.resource_spec not set")
    }

    /// Go: `Deps.GetTemplateStore()`.
    pub fn get_template_store(&self) -> &TemplateStore {
        self.template_store.get().expect("template store not set")
    }

    /// The site (panics before HugoSites is frozen — no template runs before that).
    pub fn site(&self) -> &SiteRef {
        self.site.get().expect("site not set")
    }

    /// Go: `d.Translate(ctx, id, data)`.
    pub fn translate(&self, ctx: HostCtx<'_>, id: &str, data: &Value) -> String {
        match self.translate.get() {
            Some(f) => f(ctx, id, data),
            None => String::new(),
        }
    }
}

/// Go: `deps.DepsCfg`.
pub struct DepsCfg {
    pub fs: Fs,
    pub conf: Arc<dyn AllProvider>,
    pub log: Logger,
    pub build_state: Arc<BuildState>,
}

/// Go: `deps.BuildState` (one per build, shared by all sites).
#[derive(Default)]
pub struct BuildState {
    /// Go `counter`: the PostProcess placeholder ids (via `Incrementer`, stored in the shared
    /// `resources.SpecCommon`). The only such counter in the port.
    pub(crate) counter: AtomicU64,
    /// Files in /public that contain a post-processing placeholder (`__h_pp_l1`).
    pub(crate) filenames_with_post_prefix: Mutex<BTreeSet<String>>,
    /// Files with deferred-template placeholders (`__hdeferred/`); unused by seeksnack.
    pub(crate) filenames_with_deferred_prefix: Mutex<BTreeSet<String>>,
}

impl BuildState {
    // Go: deps/deps.go:AddFilenameWithPostPrefix
    pub fn add_filename_with_post_prefix(&self, filename: &str) {
        self.filenames_with_post_prefix.lock().unwrap().insert(filename.to_string());
    }

    /// Go: `GetFilenamesWithPostPrefix()` (sorted).
    // Go: deps/deps.go:GetFilenamesWithPostPrefix
    pub fn get_filenames_with_post_prefix(&self) -> Vec<String> {
        self.filenames_with_post_prefix.lock().unwrap().iter().cloned().collect()
    }
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

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: deps/deps.go (498 lines; 13/17 funcs executed)
//   types: Deps, globalErrHandler, Listeners[T, ResourceProvider, DepsCfg, BuildState, Counters, DeferredExecutions
// EX L116-127: (d Deps) Clone(s page.Site, conf config.AllProvider) (*Deps, error)
// EX L129-131: (d *Deps) GetTemplateStore() *tplimpl.TemplateStore
// EX L133-262: (d *Deps) Init() error
// EX L265-280: (d *Deps) Compile(prototype *Deps) error
//    L283-297: (d Deps) MkdirTemp(pattern string) (string, error)
//    L311-321: (e *globalErrHandler) SendError(err error)
// EX L323-327: (e *globalErrHandler) StartErrorCollector() chan error
// EX L329-334: (e *globalErrHandler) StopErrorCollector()
// EX L346-353: (b *Listeners[T]) Add(f func(...T) bool)
// EX L356-366: (b *Listeners[T]) Notify(vs ...T)
// EX L374-387: (d *Deps) Close() error
//    L460-461: (b *BuildState) StartStageRender(stage tpl.RenderingContext)
// EX L464-470: (b *BuildState) StopStageRender(stage tpl.RenderingContext)
//    L472-474: (b *BuildState) SignalRebuild(ids ...identity.Identity)
// EX L476-483: (b *BuildState) AddFilenameWithPostPrefix(filename string)
// EX L485-494: (b *BuildState) GetFilenamesWithPostPrefix() []string
// EX L496-498: (b *BuildState) Incr() int
// ---------------------------------------------------------------------------
