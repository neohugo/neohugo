//! Port of `resources/page/pagemeta/pagemeta.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).

use nh_common::Result;
use nh_config::decode::FieldRef;

pub const NEVER: &str = "never";
pub const ALWAYS: &str = "always";
pub const LIST_LOCALLY: &str = "local";
pub const LINK: &str = "link";

/// Go: `pagemeta.BuildConfig` (front matter `build`). `Default` is Go's zero value
/// (`BuildConfig{}`: empty strings, `IsZero`); Go's `DefaultBuildConfig` (list=always,
/// render=always, publishResources=true, set) is [`BuildConfig::default_build_config`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BuildConfig {
    /// Whether to add it to any of the page collections. Valid values: never, always, local.
    pub list: String,
    /// Whether to render it. Valid values: never, always, link.
    pub render: String,
    /// Whether to publish its resources.
    pub publish_resources: bool,
    pub(crate) set: bool,
}

nh_config::decode_struct!(BuildConfig, "pagemeta.BuildConfig", |s| vec![
    FieldRef::new("List", &mut s.list),
    FieldRef::new("Render", &mut s.render),
    FieldRef::new("PublishResources", &mut s.publish_resources),
    FieldRef::new("set", &mut s.set).unexported(),
]);

impl BuildConfig {
    /// Go: `pagemeta.DefaultBuildConfig`.
    pub fn default_build_config() -> BuildConfig {
        BuildConfig {
            list: ALWAYS.into(),
            render: ALWAYS.into(),
            publish_resources: true,
            set: true,
        }
    }

    /// Disable sets all options to their off value.
    // Go: resources/page/pagemeta/pagemeta.go:Disable
    pub fn disable(&mut self) {
        self.list = NEVER.into();
        self.render = NEVER.into();
        self.publish_resources = false;
        self.set = true;
    }

    // Go: resources/page/pagemeta/pagemeta.go:IsZero
    pub fn is_zero(&self) -> bool {
        !self.set
    }
}

/// Go: `pagemeta.DecodeBuildConfig(m)` — weakly decoded over `DefaultBuildConfig`; the legacy
/// bool forms ("0"/"1") and unknown values are normalised. Go returns the config together with a
/// decode error; the port returns only the error.
// Go: resources/page/pagemeta/pagemeta.go:DecodeBuildConfig
pub fn decode_build_config(m: &go_value::Value) -> Result<BuildConfig> {
    let mut b = BuildConfig::default_build_config();
    if m.is_invalid() {
        return Ok(b);
    }

    let err = nh_config::decode::weak_decode_into(m, &mut b).err();

    // In 0.67.1 we changed the list attribute from a bool to a string (enum).
    // Bool values will become 0 or 1.
    match b.list.as_str() {
        "0" => b.list = NEVER.into(),
        "1" => b.list = ALWAYS.into(),
        ALWAYS | NEVER | LIST_LOCALLY => {}
        _ => b.list = ALWAYS.into(),
    }

    // In 0.76.0 we changed the Render from bool to a string.
    match b.render.as_str() {
        "0" => b.render = NEVER.into(),
        "1" => b.render = ALWAYS.into(),
        ALWAYS | NEVER | LINK => {}
        _ => b.render = ALWAYS.into(),
    }

    match err {
        Some(e) => Err(e),
        None => Ok(b),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pagemeta/pagemeta.go (103 lines; 2/3 funcs executed)
//   types: BuildConfig
// OK L60-65: (b *BuildConfig) Disable()
// OK L67-69: (b BuildConfig) IsZero() bool
// OK L71-103: DecodeBuildConfig(m any) (BuildConfig, error)
// ---------------------------------------------------------------------------
