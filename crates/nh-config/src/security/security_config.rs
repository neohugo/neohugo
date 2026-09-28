//! Port of `config/security/securityConfig.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::sync::OnceLock;

use go_value::Value;
use nh_common::{Error, Result};

use super::whitelist::Whitelist;
use crate::config_provider::Provider;
use crate::decode::{Decode, Decoder, DecoderConfig, FieldRef};
use crate::decode_struct;

const SECURITY_CONFIG_KEY: &str = "security";

/// Go: `security.Config`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    /// Restricts access to os.Exec....
    pub exec: Exec,
    /// Restricts access to certain template funcs.
    pub funcs: Funcs,
    /// Restricts access to resources.GetRemote, getJSON, getCSV.
    pub http: Http,
    /// Allow inline shortcodes
    pub enable_inline_shortcodes: bool,
}

decode_struct!(Config, "security.Config", |s| vec![
    FieldRef::new("Exec", &mut s.exec),
    FieldRef::new("Funcs", &mut s.funcs),
    FieldRef::new("HTTP", &mut s.http),
    FieldRef::new("EnableInlineShortcodes", &mut s.enable_inline_shortcodes),
]);

/// Go: `security.Exec` — allowed binaries (`^(dart-)?sass(-embedded)?$`, `^go$`, `^git$`, `^npx$`,
/// `^postcss$`, `^tailwindcss$`) and the OS env whitelist passed to them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Exec {
    pub allow: Whitelist,
    pub os_env: Whitelist,
}

decode_struct!(Exec, "security.Exec", |s| vec![
    FieldRef::new("Allow", &mut s.allow),
    FieldRef::new("OsEnv", &mut s.os_env),
]);

/// Go: `security.Funcs`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Funcs {
    /// OS env keys allowed to query in os.Getenv.
    pub getenv: Whitelist,
}

decode_struct!(Funcs, "security.Funcs", |s| vec![FieldRef::new(
    "Getenv",
    &mut s.getenv
)]);

/// Go: `security.HTTP` (GetRemote URL/method/media-type policy).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Http {
    /// URLs to allow in remote HTTP (resources.Get, getJSON, getCSV).
    pub urls: Whitelist,
    /// HTTP methods to allow.
    pub methods: Whitelist,
    /// Media types where the Content-Type in the response is used instead of resolving from
    /// the file content.
    pub media_types: Whitelist,
}

decode_struct!(Http, "security.HTTP", |s| vec![
    FieldRef::new("URLs", &mut s.urls),
    FieldRef::new("Methods", &mut s.methods),
    FieldRef::new("MediaTypes", &mut s.media_types),
]);

/// Go: `security.AccessDeniedError`.
// Go: config/security/securityConfig.go:(*AccessDeniedError).Error
fn access_denied_error(name: &str, path: &str, policies: &str) -> Error {
    Error::new(format!(
        "access denied: {} is not whitelisted in policy {}; the current security configuration is:\n\n{}\n\n",
        go_strconv::quote(name),
        go_strconv::quote(path),
        policies
    ))
}

/// Go: `security.IsAccessDenied(err)`.
// Go: config/security/securityConfig.go:IsAccessDenied
pub fn is_access_denied(err: &Error) -> bool {
    err.message().starts_with("access denied: ")
}

impl Config {
    /// Go: `ToTOML()`: the policies as TOML (`[security]` root, indented tables), as go-toml v2
    /// encodes `ToSecurityMap()`.
    // Go: config/security/securityConfig.go:ToTOML
    pub fn to_toml(&self) -> String {
        let mut b = String::new();
        b.push_str("[security]\n");
        b.push_str(&format!(
            "  enableInlineShortcodes = {}\n",
            self.enable_inline_shortcodes
        ));
        let tables: [(&str, Vec<(&str, &Whitelist)>); 3] = [
            (
                "exec",
                vec![("allow", &self.exec.allow), ("osEnv", &self.exec.os_env)],
            ),
            ("funcs", vec![("getenv", &self.funcs.getenv)]),
            (
                "http",
                vec![
                    ("mediaTypes", &self.http.media_types),
                    ("methods", &self.http.methods),
                    ("urls", &self.http.urls),
                ],
            ),
        ];
        for (table, entries) in tables {
            b.push_str(&format!("\n  [security.{table}]\n"));
            for (key, w) in entries {
                if w.accept_none() {
                    b.push_str(&format!("    {key} = 'none'\n"));
                } else if let Some(p) = w.patterns_strings() {
                    let items: Vec<String> = p.iter().map(|s| toml_string(s)).collect();
                    b.push_str(&format!("    {key} = [{}]\n", items.join(", ")));
                }
                // A nil pattern list is JSON null, which the TOML encoder omits.
            }
        }
        go_unicode::strings::trim_space_str(&b).to_string()
    }

    /// Go: `ToSecurityMap()`: the config through JSON (json tags) and back, under a `security`
    /// root key.
    // Go: config/security/securityConfig.go:ToSecurityMap
    pub fn to_security_map(&self) -> go_value::Map {
        use go_value::{Map, MapType};
        let wl = |w: &Whitelist| -> Value {
            if w.accept_none() {
                return Value::string("none");
            }
            match w.patterns_strings() {
                None => Value::Invalid,
                Some(p) => Value::any_list(p.iter().map(|s| Value::string(s.as_str())).collect()),
            }
        };
        let table = |entries: Vec<(&str, Value)>| -> Value {
            let mut m = Map::new(MapType::StringAny);
            for (k, v) in entries {
                m.insert(k, v);
            }
            Value::map(m)
        };
        let mut m = Map::new(MapType::StringAny);
        m.insert(
            "exec",
            table(vec![
                ("allow", wl(&self.exec.allow)),
                ("osEnv", wl(&self.exec.os_env)),
            ]),
        );
        m.insert("funcs", table(vec![("getenv", wl(&self.funcs.getenv))]));
        m.insert(
            "http",
            table(vec![
                ("urls", wl(&self.http.urls)),
                ("methods", wl(&self.http.methods)),
                ("mediaTypes", wl(&self.http.media_types)),
            ]),
        );
        m.insert(
            "enableInlineShortcodes",
            Value::Bool(self.enable_inline_shortcodes),
        );
        let mut sec = Map::new(MapType::StringAny);
        sec.insert("security", Value::map(m));
        sec
    }

    // Go: config/security/securityConfig.go:CheckAllowedExec
    pub fn check_allowed_exec(&self, name: &str) -> Result<()> {
        if !self.exec.allow.accept(name) {
            return Err(access_denied_error(
                name,
                "security.exec.allow",
                &self.to_toml(),
            ));
        }
        Ok(())
    }

    // Go: config/security/securityConfig.go:CheckAllowedGetEnv
    pub fn check_allowed_get_env(&self, name: &str) -> Result<()> {
        if !self.funcs.getenv.accept(name) {
            return Err(access_denied_error(
                name,
                "security.funcs.getenv",
                &self.to_toml(),
            ));
        }
        Ok(())
    }

    // Go: config/security/securityConfig.go:CheckAllowedHTTPURL
    pub fn check_allowed_http_url(&self, url: &str) -> Result<()> {
        if !self.http.urls.accept(url) {
            return Err(access_denied_error(
                url,
                "security.http.urls",
                &self.to_toml(),
            ));
        }
        Ok(())
    }

    // Go: config/security/securityConfig.go:CheckAllowedHTTPMethod
    pub fn check_allowed_http_method(&self, method: &str) -> Result<()> {
        if !self.http.methods.accept(method) {
            return Err(access_denied_error(
                method,
                "security.http.method",
                &self.to_toml(),
            ));
        }
        Ok(())
    }
}

/// go-toml v2's string encoding: a literal string unless it contains `'`, CR, LF or another
/// invalid ASCII control character, else a basic string.
fn toml_string(v: &str) -> String {
    let needs_quoting = v
        .bytes()
        .any(|b| b == b'\'' || b == b'\r' || b == b'\n' || invalid_ascii(b));
    if !needs_quoting {
        return format!("'{v}'");
    }
    let mut out = String::from("\"");
    for c in v.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// go-toml's `characters.InvalidAscii`: control characters other than tab.
fn invalid_ascii(b: u8) -> bool {
    (b <= 0x08) || (0x0a..=0x1f).contains(&b) || b == 0x7f
}

/// Go: `security.DefaultConfig`.
pub fn default_config() -> Config {
    static C: OnceLock<Config> = OnceLock::new();
    C.get_or_init(|| Config {
        exec: Exec {
            allow: Whitelist::must_new(&[
                "^(dart-)?sass(-embedded)?$", // sass, dart-sass, dart-sass-embedded.
                "^go$",                       // for Go Modules
                "^git$",                      // For Git info
                "^npx$",                      // used by all Node tools (Babel, PostCSS).
                "^postcss$",
                "^tailwindcss$",
            ]),
            // These have been tested to work with Hugo's external programs on Windows, Linux
            // and MacOS.
            os_env: Whitelist::must_new(&[
                r"(?i)^((HTTPS?|NO)_PROXY|PATH(EXT)?|APPDATA|TE?MP|TERM|GO\w+|(XDG_CONFIG_)?HOME|USERPROFILE|SSH_AUTH_SOCK|DISPLAY|LANG|SYSTEMDRIVE)$",
            ]),
        },
        funcs: Funcs {
            getenv: Whitelist::must_new(&["^HUGO_", "^CI$"]),
        },
        http: Http {
            urls: Whitelist::must_new(&[".*"]),
            methods: Whitelist::must_new(&["(?i)GET|POST"]),
            media_types: Whitelist::default(),
        },
        enable_inline_shortcodes: false,
    })
    .clone()
}

/// Go: `stringSliceToWhitelistHook()`: a value decoded into a `Whitelist` becomes
/// `NewWhitelist(types.ToStringSlicePreserveString(data)...)`.
// Go: config/security/securityConfig.go:stringSliceToWhitelistHook
fn string_slice_to_whitelist_hook(
    data: &Value,
    target: &dyn Decode,
) -> std::result::Result<Value, String> {
    if !target.as_any().is::<Whitelist>() {
        return Ok(data.clone());
    }

    let wl: Vec<String> = nh_common::types::convert::to_string_slice_preserve_string(data)
        .iter()
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    let refs: Vec<&str> = wl.iter().map(String::as_str).collect();

    Whitelist::new(&refs)
        .map(Value::object)
        .map_err(|e| e.to_string())
}

/// Go: `security.DecodeConfig(cfg)`.
// Go: config/security/securityConfig.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<Config> {
    let (sc, err) = decode_config_partial(cfg);
    match err {
        Some(e) => Err(e),
        None => Ok(sc),
    }
}

/// [`decode_config`] returning the (partially decoded) config along with the error, like Go.
pub fn decode_config_partial(cfg: &dyn Provider) -> (Config, Option<Error>) {
    let mut sc = default_config();
    if cfg.is_set(SECURITY_CONFIG_KEY) {
        let m = cfg.get_string_map(SECURITY_CONFIG_KEY);
        let hook = string_slice_to_whitelist_hook;
        let dec = Decoder::new(DecoderConfig {
            weakly_typed_input: true,
            decode_hook: Some(&hook),
            ..Default::default()
        });

        if let Err(e) = dec.decode_input(&Value::map(m), &mut sc) {
            return (sc, Some(Error::from(e)));
        }
    }

    if !sc.enable_inline_shortcodes {
        // Legacy
        sc.enable_inline_shortcodes = cfg.get_bool("enableInlineShortcodes");
    }

    (sc, None)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/security/securityConfig.go (230 lines; 4/10 funcs executed)
//   types: Config, Exec, Funcs, HTTP, AccessDeniedError
// OK L99-109: (c Config) ToTOML() string
// OK L111-120: (c Config) CheckAllowedExec(name string) error
// OK L122-131: (c Config) CheckAllowedGetEnv(name string) error
// OK L133-142: (c Config) CheckAllowedHTTPURL(url string) error
// OK L144-153: (c Config) CheckAllowedHTTPMethod(method string) error
// OK L156-168: (c Config) ToSecurityMap() map[string]any
// OK L171-197: DecodeConfig(cfg config.Provider) (Config, error)
// OK L199-213: stringSliceToWhitelistHook() mapstructure.DecodeHookFuncType
// OK L222-224: (e *AccessDeniedError) Error() string
// OK L227-230: IsAccessDenied(err error) bool
// ---------------------------------------------------------------------------
