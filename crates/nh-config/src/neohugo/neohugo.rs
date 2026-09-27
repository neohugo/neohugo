//! Port of `common/neohugo/neohugo.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Object, SafeKind, Value};
use nh_common::maps::scratch::Scratch;

/// Go: `neohugo.ConfigProvider`.
pub trait HugoInfoConfig: Send + Sync {
    fn environment(&self) -> String;
    fn running(&self) -> bool;
    fn working_dir(&self) -> String;
    fn is_multihost(&self) -> bool;
    fn is_multilingual(&self) -> bool;
}

/// Go: `neohugo.HugoInfo` (template func `hugo`).
#[derive(Clone)]
pub struct HugoInfo {
    pub commit_hash: String,
    pub build_date: String,
    /// "production" for builds.
    pub environment: String,
    pub go_version: String,
    pub conf: Arc<dyn HugoInfoConfig>,
    pub store: Arc<Scratch>,
}

impl HugoInfo {
    // Go: common/neohugo/neohugo.go:NewInfo
    pub fn new(conf: Arc<dyn HugoInfoConfig>) -> HugoInfo {
        todo!()
    }

    /// Go: `HugoInfo.Generator()` — `<meta name="generator" content="Hugo 0.149.0-DEV">` (template.HTML).
    // Go: common/neohugo/neohugo.go:Generator
    pub fn generator(&self) -> Value {
        todo!()
    }

    // Go: common/neohugo/neohugo.go:IsProduction
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

nh_common::go_methods!(HugoInfo {
    "Version" => |h, _c, _a| todo!(),
    "Generator" => |h, _c, _a| Ok(h.generator()),
    "IsDevelopment" => |h, _c, _a| Ok(Value::Bool(h.environment == "development")),
    "IsProduction" => |h, _c, _a| Ok(Value::Bool(h.is_production())),
    "IsServer" => |h, _c, _a| Ok(Value::Bool(h.conf.running())),
    "WorkingDir" => |h, _c, _a| Ok(Value::string(h.conf.working_dir())),
    "Store" => |h, _c, _a| Ok(Value::Object(h.store.clone())),
    "IsMultihost" => |h, _c, _a| Ok(Value::Bool(h.conf.is_multihost())),
    "IsMultilingual" => |h, _c, _a| Ok(Value::Bool(h.conf.is_multilingual())),
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
            _ => None,
        }
    }
}

/// Go: `neohugo.GetExecEnviron(workDir, cfg, fs)`: `NODE_PATH`, `PWD`, `HUGO_ENVIRONMENT`,
/// `HUGO_ENV`, `HUGO_PUBLISHDIR` (= filepath.Join(workDir, publishDir) — quirk kept), plus
/// `HUGO_FILE_<NAME>` for each file mounted at `assets/_jsconfig`.
/// Deviation: Go walks the assets fs itself; the Rust caller passes the `_jsconfig` files
/// (name, real filename) so this crate does not depend on nh-hugofs.
// Go: common/neohugo/neohugo.go:GetExecEnviron
pub fn get_exec_environ(work_dir: &str, environment: &str, publish_dir: &str, js_config_files: &[(String, String)]) -> Vec<String> {
    todo!()
}

/// Go: `neohugo.IsRunningAsTest()` etc. are not needed.
pub fn deprecate(item: &str, alternative: &str, version: &str) {
    // Go logs a deprecation warning (once per item). Not part of output.
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/neohugo/neohugo.go (460 lines; 6/30 funcs executed)
//   types: HugoInfo, contextKey, Context, ConfigProvider, buildInfo, Dependency
//    L83-85: (i HugoInfo) Version() VersionString
//    L88-90: (i HugoInfo) Generator() template.HTML
//    L93-95: (i HugoInfo) IsDevelopment() bool
//    L98-100: (i HugoInfo) IsProduction() bool
//    L103-105: (i HugoInfo) IsServer() bool
//    L108-110: (i HugoInfo) WorkingDir() string
//    L113-115: (i HugoInfo) Deps() []*Dependency
//    L117-119: (i HugoInfo) Store() *maps.Scratch
//    L122-125: (i HugoInfo) IsMultiHost() bool
//    L128-130: (i HugoInfo) IsMultihost() bool
//    L133-135: (i HugoInfo) IsMultilingual() bool
//    L147-149: (c Context) MarkupScope(ctx context.Context) string
//    L152-154: SetMarkupScope(ctx context.Context, s string) context.Context
// EX L157-159: GetMarkupScope(ctx context.Context) string
// EX L171-197: NewInfo(conf ConfigProvider, deps []*Dependency) HugoInfo
// EX L201-230: GetExecEnviron(workDir string, cfg config.AllProvider, fs afero.Fs) []string
// EX L249-277: getBuildInfo() *buildInfo
//    L279-281: formatDep(path, version string) string
//    L285-302: GetDependencyList() []string
//    L305-326: GetDependencyListNonGo() []string
// EX L329-336: IsRunningAsTest() bool
//    L362-368: dartSassVersion() godartsass.DartSassVersion
// EX L374-389: init()
//    L398-401: IsDartSassGeV2() bool
//    L409-412: Deprecate(item, alternative string, version string)
//    L415-418: DeprecateWithLogger(item, alternative string, version string, log logg.Logger)
//    L421-424: DeprecateLevelMin(item, alternative string, version string, minLevel logg.Level)
//    L427-429: deprecateLevel(item, alternative, version string, level logg.Level)
//    L432-441: deprecateLevelWithLogger(item, alternative, version string, level logg.Level, log logg.Logger)
//    L446-460: deprecationLogLevelFromVersion(ver string) logg.Level
// ---------------------------------------------------------------------------
