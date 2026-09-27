//! Port of `resources/page/pagemeta/pagemeta.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


use nh_common::Result;

pub const NEVER: &str = "never";
pub const ALWAYS: &str = "always";
pub const LIST_LOCALLY: &str = "local";
pub const LINK: &str = "link";

/// Go: `pagemeta.BuildConfig` (front matter `build`; defaults list=always, render=always,
/// publishResources=true).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildConfig {
    pub list: String,
    pub render: String,
    pub publish_resources: bool,
    pub(crate) set: bool,
}

impl Default for BuildConfig {
    /// Go: `pagemeta.DefaultBuildConfig`.
    fn default() -> Self {
        BuildConfig { list: ALWAYS.into(), render: ALWAYS.into(), publish_resources: true, set: false }
    }
}

impl BuildConfig {
    // Go: resources/page/pagemeta/pagemeta.go:IsZero
    pub fn is_zero(&self) -> bool {
        !self.set
    }
}

/// Go: `pagemeta.DecodeBuildConfig(m)`.
// Go: resources/page/pagemeta/pagemeta.go:DecodeBuildConfig
pub fn decode_build_config(m: &go_value::Value) -> Result<BuildConfig> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pagemeta/pagemeta.go (103 lines; 2/3 funcs executed)
//   types: BuildConfig
//    L60-65: (b *BuildConfig) Disable()
// EX L67-69: (b BuildConfig) IsZero() bool
// EX L71-103: DecodeBuildConfig(m any) (BuildConfig, error)
// ---------------------------------------------------------------------------
