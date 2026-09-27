//! Port of `config/security/securityConfig.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use go_value::Map;
use nh_common::Result;

use super::whitelist::Whitelist;
use crate::config_provider::Provider;

/// Go: `security.Config`.
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub exec: Exec,
    pub funcs: Funcs,
    pub http: Http,
    pub enable_inline_shortcodes: bool,
}

/// Go: `security.Exec` — allowed binaries (`^(dart-)?sass(-embedded)?$`, `^go$`, `^git$`, `^npx$`,
/// `^postcss$`, `^tailwindcss$`, `^babel$`) and the OS env whitelist passed to them.
#[derive(Clone, Debug, Default)]
pub struct Exec {
    pub allow: Whitelist,
    pub os_env: Whitelist,
}

/// Go: `security.Funcs`.
#[derive(Clone, Debug, Default)]
pub struct Funcs {
    pub getenv: Whitelist,
}

/// Go: `security.HTTP` (GetRemote URL/method/media-type policy).
#[derive(Clone, Debug, Default)]
pub struct Http {
    pub urls: Whitelist,
    pub methods: Whitelist,
    pub media_types: Whitelist,
}

impl Config {
    // Go: config/security/securityConfig.go:CheckAllowedExec
    pub fn check_allowed_exec(&self, name: &str) -> Result<()> { todo!() }
    // Go: config/security/securityConfig.go:CheckAllowedGetEnv
    pub fn check_allowed_get_env(&self, name: &str) -> Result<()> { todo!() }
    // Go: config/security/securityConfig.go:CheckAllowedHTTPURL
    pub fn check_allowed_http_url(&self, url: &str) -> Result<()> { todo!() }
    // Go: config/security/securityConfig.go:CheckAllowedHTTPMethod
    pub fn check_allowed_http_method(&self, method: &str) -> Result<()> { todo!() }
}

/// Go: `security.DefaultConfig`.
pub fn default_config() -> Config {
    todo!()
}

/// Go: `security.DecodeConfig(cfg)`.
// Go: config/security/securityConfig.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<Config> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/security/securityConfig.go (230 lines; 4/10 funcs executed)
//   types: Config, Exec, Funcs, HTTP, AccessDeniedError
//    L99-109: (c Config) ToTOML() string
// EX L111-120: (c Config) CheckAllowedExec(name string) error
//    L122-131: (c Config) CheckAllowedGetEnv(name string) error
// EX L133-142: (c Config) CheckAllowedHTTPURL(url string) error
// EX L144-153: (c Config) CheckAllowedHTTPMethod(method string) error
//    L156-168: (c Config) ToSecurityMap() map[string]any
// EX L171-197: DecodeConfig(cfg config.Provider) (Config, error)
//    L199-213: stringSliceToWhitelistHook() mapstructure.DecodeHookFuncType
//    L222-224: (e *AccessDeniedError) Error() string
//    L227-230: IsAccessDenied(err error) bool
// ---------------------------------------------------------------------------
