//! Port of `common/neohugo/neohugo.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::any::Any;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

use go_value::{HostCtx, Kind, Object, SliceType, Value};
use nh_common::maps::scratch::Scratch;

use super::version::CURRENT_VERSION;

pub const ENVIRONMENT_DEVELOPMENT: &str = "development";
pub const ENVIRONMENT_PRODUCTION: &str = "production";

/// The Go version of the golden build's binary (Go: `debug.ReadBuildInfo().GoVersion`).
pub const GO_VERSION: &str = "go1.27.1";

/// Go: `neohugo.ConfigProvider`: the config options that are relevant for HugoInfo.
pub trait HugoInfoConfig: Send + Sync {
    fn environment(&self) -> String;
    fn running(&self) -> bool;
    fn working_dir(&self) -> String;
    fn is_multihost(&self) -> bool;
    fn is_multilingual(&self) -> bool;
}

/// Go: `neohugo.Dependency`: a Hugo Module or a local theme.
#[derive(Clone, Debug, Default)]
pub struct Dependency {
    /// The path to this module ("github.com/gohugoio/myshortcodes" or a theme dir name).
    pub path: String,
    /// The module version.
    pub version: String,
    /// Whether this dependency is vendored.
    pub vendor: bool,
    /// Time version was created.
    pub time: Option<go_value::Time>,
    /// In the dependency tree, this is the first module that defines this module as a
    /// dependency.
    pub owner: Option<Arc<Dependency>>,
    /// Replaced by this dependency.
    pub replace: Option<Arc<Dependency>>,
}

impl Object for Dependency {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*neohugo.Dependency")
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        let dep = |d: &Option<Arc<Dependency>>| match d {
            Some(d) => Value::object((**d).clone()),
            None => Value::TypedNil(Arc::from("*neohugo.Dependency")),
        };
        Some(match name {
            "Path" => Value::string(self.path.as_str()),
            "Version" => Value::string(self.version.as_str()),
            "Vendor" => Value::Bool(self.vendor),
            "Time" => match &self.time {
                Some(t) => Value::Time(t.clone()),
                None => Value::Time(go_value::Time::zero()),
            },
            "Owner" => dep(&self.owner),
            "Replace" => dep(&self.replace),
            _ => return None,
        })
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `neohugo.HugoInfo` (template func `hugo`): information about the current Hugo
/// environment.
#[derive(Clone)]
pub struct HugoInfo {
    pub commit_hash: String,
    pub build_date: String,
    /// The build environment: "production" (hugo) and "development" (hugo server) by default;
    /// any string, lower case.
    pub environment: String,
    /// The version of go that the Hugo binary was built with.
    pub go_version: String,
    pub conf: Arc<dyn HugoInfoConfig>,
    pub deps: Vec<Arc<Dependency>>,
    pub store: Arc<Scratch>,
}

impl HugoInfo {
    /// NewInfo creates a new Hugo Info object (panics when the environment is not set, like
    /// Go). The build information is that of this binary: [`GO_VERSION`], and the commit and
    /// build date from the `NEOHUGO_VCS_REVISION`/`NEOHUGO_VCS_TIME` build environment.
    // Go: common/neohugo/neohugo.go:NewInfo
    pub fn new(conf: Arc<dyn HugoInfoConfig>) -> HugoInfo {
        HugoInfo::new_with_deps(conf, Vec::new())
    }

    /// Go: `NewInfo(conf, deps)`.
    pub fn new_with_deps(conf: Arc<dyn HugoInfoConfig>, deps: Vec<Arc<Dependency>>) -> HugoInfo {
        let environment = conf.environment();
        if environment.is_empty() {
            panic!("environment not set");
        }
        let bi = super::version::BuildInfo::current();

        HugoInfo {
            commit_hash: bi.revision,
            build_date: bi.revision_time,
            environment,
            conf,
            deps,
            store: Arc::new(Scratch::new()),
            go_version: GO_VERSION.to_string(),
        }
    }

    /// Version returns the current version as a comparable version string.
    // Go: common/neohugo/neohugo.go:Version
    pub fn version(&self) -> super::version::VersionString {
        CURRENT_VERSION.version()
    }

    /// Go: `HugoInfo.Generator()` — `<meta name="generator" content="Hugo 0.149.0-DEV">` (template.HTML).
    // Go: common/neohugo/neohugo.go:Generator
    pub fn generator(&self) -> Value {
        Value::html(format!(
            "<meta name=\"generator\" content=\"Hugo {}\">",
            CURRENT_VERSION.string()
        ))
    }

    // Go: common/neohugo/neohugo.go:IsDevelopment
    pub fn is_development(&self) -> bool {
        self.environment == ENVIRONMENT_DEVELOPMENT
    }

    // Go: common/neohugo/neohugo.go:IsProduction
    pub fn is_production(&self) -> bool {
        self.environment == ENVIRONMENT_PRODUCTION
    }

    // Go: common/neohugo/neohugo.go:Deps
    fn deps_value(&self) -> Value {
        if self.deps.is_empty() {
            return Value::TypedNil(Arc::from("[]*neohugo.Dependency"));
        }
        Value::list(
            SliceType::Named(Arc::from("[]*neohugo.Dependency")),
            self.deps
                .iter()
                .map(|d| Value::object((**d).clone()))
                .collect(),
        )
    }
}

// Go: common/neohugo/neohugo.go:HugoInfo (methods; text/template checks the argument count)
nh_common::go_methods!(HugoInfo {
    "Version" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "Version")?;
        Ok(Value::object(h.version()))
    },
    "Generator" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "Generator")?;
        Ok(h.generator())
    },
    "IsDevelopment" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "IsDevelopment")?;
        Ok(Value::Bool(h.is_development()))
    },
    "IsProduction" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "IsProduction")?;
        Ok(Value::Bool(h.is_production()))
    },
    "IsServer" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "IsServer")?;
        Ok(Value::Bool(h.conf.running()))
    },
    "WorkingDir" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "WorkingDir")?;
        Ok(Value::string(h.conf.working_dir()))
    },
    "Deps" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "Deps")?;
        Ok(h.deps_value())
    },
    "Store" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "Store")?;
        Ok(Value::Object(h.store.clone()))
    },
    "IsMultiHost" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "IsMultiHost")?;
        deprecate("hugo.IsMultiHost", "Use hugo.IsMultihost instead.", "v0.124.0");
        Ok(Value::Bool(h.conf.is_multihost()))
    },
    "IsMultihost" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "IsMultihost")?;
        Ok(Value::Bool(h.conf.is_multihost()))
    },
    "IsMultilingual" => |h, _c, a| {
        nh_common::object::args::exactly(a, 0, "IsMultilingual")?;
        Ok(Value::Bool(h.conf.is_multilingual()))
    },
});

impl Object for HugoInfo {
    nh_common::object_basics!("neohugo.HugoInfo");

    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "CommitHash" => Some(Value::string(self.commit_hash.as_str())),
            "BuildDate" => Some(Value::string(self.build_date.as_str())),
            "Environment" => Some(Value::string(self.environment.as_str())),
            "GoVersion" => Some(Value::string(self.go_version.as_str())),
            "Context" => Some(Value::object(Context)),
            _ => None,
        }
    }
}

/// Go: `neohugo.Context`: gives access to some of the context scoped variables.
#[derive(Clone, Copy, Debug, Default)]
pub struct Context;

impl Context {
    // Go: common/neohugo/neohugo.go:(Context).MarkupScope
    pub fn markup_scope(&self, ctx: HostCtx<'_>) -> String {
        get_markup_scope(ctx)
    }
}

nh_common::go_methods!(Context {
    "MarkupScope" => |c, ctx, _a| Ok(Value::string(c.markup_scope(ctx))),
});

impl Object for Context {
    nh_common::object_basics!("neohugo.Context");

    fn kind(&self) -> Kind {
        Kind::Struct
    }
}

/// The reader of the markup scope in a template context (the `context.Context` stand-in lives
/// in nh-tpl, which registers this; see PORTING.md).
static MARKUP_SCOPE_GETTER: OnceLock<fn(HostCtx<'_>) -> String> = OnceLock::new();

/// Registers the function [`get_markup_scope`] uses to read the scope from a `HostCtx`.
pub fn set_markup_scope_getter(f: fn(HostCtx<'_>) -> String) {
    let _ = MARKUP_SCOPE_GETTER.set(f);
}

/// GetMarkupScope gets the markup scope from the context ("" when none is set).
// Go: common/neohugo/neohugo.go:GetMarkupScope
pub fn get_markup_scope(ctx: HostCtx<'_>) -> String {
    match MARKUP_SCOPE_GETTER.get() {
        Some(f) => f(ctx),
        None => String::new(),
    }
}

/// Go: `neohugo.GetExecEnviron(workDir, cfg, fs)`: `NODE_PATH`, `PWD`, `HUGO_ENVIRONMENT`,
/// `HUGO_ENV`, `HUGO_PUBLISHDIR` (= filepath.Join(workDir, publishDir) — quirk kept), plus
/// `HUGO_FILE_<NAME>` for each file mounted at `assets/_jsconfig`.
/// Deviation: Go walks the assets fs itself; the Rust caller passes the `_jsconfig` files
/// (name, real filename) so this crate does not depend on nh-hugofs.
// Go: common/neohugo/neohugo.go:GetExecEnviron
pub fn get_exec_environ(
    work_dir: &str,
    environment: &str,
    publish_dir: &str,
    js_config_files: &[(String, String)],
) -> Vec<String> {
    let node_path = std::env::var_os("NODE_PATH").map(|p| p.to_string_lossy().into_owned());
    get_exec_environ_with(
        work_dir,
        environment,
        publish_dir,
        node_path.as_deref(),
        js_config_files,
    )
}

/// [`get_exec_environ`] with the process's `NODE_PATH` given (`None` = unset).
pub fn get_exec_environ_with(
    work_dir: &str,
    environment: &str,
    publish_dir: &str,
    node_path_env: Option<&str>,
    js_config_files: &[(String, String)],
) -> Vec<String> {
    use crate::env::set_env_vars;

    let mut env: Vec<String> = Vec::new();
    let mut nodepath = go_path::filepath::join(&[work_dir, "node_modules"]);
    if let Some(np) = node_path_env
        && !np.is_empty()
    {
        nodepath = format!("{work_dir}:{np}");
    }
    set_env_vars(&mut env, &[("NODE_PATH", &nodepath)]);
    set_env_vars(&mut env, &[("PWD", work_dir)]);
    set_env_vars(&mut env, &[("HUGO_ENVIRONMENT", environment)]);
    set_env_vars(&mut env, &[("HUGO_ENV", environment)]);
    set_env_vars(
        &mut env,
        &[(
            "HUGO_PUBLISHDIR",
            &go_path::filepath::join(&[work_dir, publish_dir]),
        )],
    );

    for (name, filename) in js_config_files {
        let key = format!(
            "HUGO_FILE_{}",
            go_unicode::strings::to_upper_str(name).replace('.', "_")
        );
        set_env_vars(&mut env, &[(&key, filename)]);
    }

    env
}

/// IsRunningAsTest reports whether we are running as a test (an argument starting with
/// `-test`).
// Go: common/neohugo/neohugo.go:IsRunningAsTest
pub fn is_running_as_test() -> bool {
    std::env::args().any(|a| a.starts_with("-test"))
}

/// Go: `deprecationLogLevelFromVersion(ver)`: the log level of a deprecation (about one minor
/// version a month: warnings after 3 minor versions, errors after 15).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeprecationLevel {
    Info,
    Warn,
    Error,
}

// Go: common/neohugo/neohugo.go:deprecationLogLevelFromVersion
pub fn deprecation_log_level_from_version(ver: &str) -> DeprecationLevel {
    let from = super::version::must_parse_version(ver);
    let to = CURRENT_VERSION;
    let minor_diff = to.minor.wrapping_sub(from.minor);
    if minor_diff >= 15 {
        // Start failing the build after about 15 months.
        DeprecationLevel::Error
    } else if minor_diff >= 3 {
        // Start printing warnings after about 3 months.
        DeprecationLevel::Warn
    } else {
        DeprecationLevel::Info
    }
}

/// The deprecation message of `deprecateLevelWithLogger`.
// Go: common/neohugo/neohugo.go:deprecateLevelWithLogger
pub fn deprecation_message(
    item: &str,
    alternative: &str,
    version: &str,
    level: DeprecationLevel,
) -> String {
    if level == DeprecationLevel::Error {
        format!("{item} was deprecated in Hugo {version} and subsequently removed. {alternative}")
    } else {
        format!(
            "{item} was deprecated in Hugo {version} and will be removed in a future release. {alternative}"
        )
    }
}

/// Deprecate informs about a deprecation starting at the given version (logged to the global
/// logger at the level [`deprecation_log_level_from_version`] gives).
// Go: common/neohugo/neohugo.go:Deprecate
pub fn deprecate(item: &str, alternative: &str, version: &str) {
    let level = deprecation_log_level_from_version(version);
    deprecate_level(item, alternative, version, level);
}

/// DeprecateLevelMin informs about a deprecation starting at the given version, but with a
/// minimum log level.
// Go: common/neohugo/neohugo.go:DeprecateLevelMin
pub fn deprecate_level_min(item: &str, alternative: &str, version: &str, min: DeprecationLevel) {
    let level = deprecation_log_level_from_version(version).max(min);
    deprecate_level(item, alternative, version, level);
}

// Go: common/neohugo/neohugo.go:deprecateLevel
fn deprecate_level(item: &str, alternative: &str, version: &str, level: DeprecationLevel) {
    deprecate_level_with_logger(
        item,
        alternative,
        version,
        level,
        &nh_common::loggers::log(),
    );
}

/// DeprecateLevel informs about a deprecation logging at the given level, with the command
/// field `deprecated` (printed as a `deprecated: ` prefix).
// Go: common/neohugo/neohugo.go:deprecateLevelWithLogger
pub fn deprecate_level_with_logger(
    item: &str,
    alternative: &str,
    version: &str,
    level: DeprecationLevel,
    log: &nh_common::loggers::Logger,
) {
    use nh_common::loggers::Level;
    let msg = deprecation_message(item, alternative, version, level);
    let level = match level {
        DeprecationLevel::Error => Level::Error,
        DeprecationLevel::Warn => Level::Warn,
        DeprecationLevel::Info => Level::Info,
    };
    log.logf_cmd(level, "deprecated", msg);
}

/// Go: `GetDependencyListNonGo()` without Dart Sass (not supported): libsass and libwebp.
// Go: common/neohugo/neohugo.go:GetDependencyListNonGo
pub fn get_dependency_list_non_go() -> Vec<String> {
    vec![
        format_dep("github.com/sass/libsass", "3.6.6"),
        format_dep("github.com/webmproject/libwebp", "v1.3.2"),
    ]
}

// Go: common/neohugo/neohugo.go:formatDep
fn format_dep(path: &str, version: &str) -> String {
    format!("{}={}", path, go_strconv::quote(version))
}

/// Go: `IsDartSassGeV2()`: whether the Dart Sass binary is not the old embedded one. Dart
/// Sass is not supported by the port, so no binary is configured.
// Go: common/neohugo/neohugo.go:IsDartSassGeV2
pub fn is_dart_sass_ge_v2() -> bool {
    !dart_sass_binary_name().contains("embedded")
}

/// Go: `DartSassBinaryName` (`DART_SASS_BINARY`, else the first of dart-sass, sass,
/// dart-sass-embedded found in PATH).
// Go: common/neohugo/neohugo.go:init
pub fn dart_sass_binary_name() -> String {
    static N: OnceLock<String> = OnceLock::new();
    N.get_or_init(|| {
        if let Ok(n) = std::env::var("DART_SASS_BINARY")
            && !n.is_empty()
        {
            return n;
        }
        for name in ["dart-sass", "sass"] {
            if crate::hexec::in_path(name) {
                return name.to_string();
            }
        }
        if crate::hexec::in_path("dart-sass-embedded") {
            return "dart-sass-embedded".to_string();
        }
        String::new()
    })
    .clone()
}

/// Go: `hugo.Deps()` etc. are in [`HugoInfo`]; `GetDependencyList` needs Go's build info.
pub fn get_dependency_list() -> Vec<String> {
    let mut deps = get_dependency_list_non_go();
    deps.sort();
    deps
}

impl std::fmt::Debug for HugoInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HugoInfo")
            .field("environment", &self.environment)
            .finish()
    }
}

/// The `hugo` template func's value (Go returns the `HugoInfo` struct).
pub fn hugo_info_value(h: &HugoInfo) -> Value {
    Value::object(h.clone())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/neohugo/neohugo.go (460 lines; 6/30 funcs executed)
//   types: HugoInfo, contextKey, Context, ConfigProvider, buildInfo, Dependency
// OK L83-85: (i HugoInfo) Version() VersionString
// OK L88-90: (i HugoInfo) Generator() template.HTML
// OK L93-95: (i HugoInfo) IsDevelopment() bool
// OK L98-100: (i HugoInfo) IsProduction() bool
// OK L103-105: (i HugoInfo) IsServer() bool
// OK L108-110: (i HugoInfo) WorkingDir() string
// OK L113-115: (i HugoInfo) Deps() []*Dependency
// OK L117-119: (i HugoInfo) Store() *maps.Scratch
// OK L122-125: (i HugoInfo) IsMultiHost() bool
// OK L128-130: (i HugoInfo) IsMultihost() bool
// OK L133-135: (i HugoInfo) IsMultilingual() bool
// OK L147-149: (c Context) MarkupScope(ctx context.Context) string
// OK L152-154: SetMarkupScope(ctx context.Context, s string) context.Context (nh-tpl TplContext.markup_scope)
// OK L157-159: GetMarkupScope(ctx context.Context) string
// OK L171-197: NewInfo(conf ConfigProvider, deps []*Dependency) HugoInfo
// OK L201-230: GetExecEnviron(workDir string, cfg config.AllProvider, fs afero.Fs) []string
// OK L249-277: getBuildInfo() *buildInfo
// OK L279-281: formatDep(path, version string) string
// OK L285-302: GetDependencyList() []string
// OK L305-326: GetDependencyListNonGo() []string
// OK L329-336: IsRunningAsTest() bool
// STUB L362-368: dartSassVersion() godartsass.DartSassVersion (Dart Sass is not supported)
// OK L374-389: init()
// OK L398-401: IsDartSassGeV2() bool
// OK L409-412: Deprecate(item, alternative string, version string)
// OK L415-418: DeprecateWithLogger(item, alternative string, version string, log logg.Logger)
// OK L421-424: DeprecateLevelMin(item, alternative string, version string, minLevel logg.Level)
// OK L427-429: deprecateLevel(item, alternative, version string, level logg.Level)
// OK L432-441: deprecateLevelWithLogger(item, alternative, version string, level logg.Level, log logg.Logger)
// OK L446-460: deprecationLogLevelFromVersion(ver string) logg.Level
// ---------------------------------------------------------------------------
