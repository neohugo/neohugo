//! Port of `config/security/whitelist.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::any::Any;
use std::borrow::Cow;

use go_value::{HostCtx, Kind, Object, Value};
use nh_common::{Error, Result};

use crate::decode_struct;
use crate::goregexp::Regexp;

const ACCEPT_NONE_KEYWORD: &str = "none";

/// Go: `security.Whitelist` — regexp patterns (RE2), or `none`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Whitelist {
    accept_none: bool,
    patterns: Vec<Regexp>,
    /// Store this for debugging/error reporting (`None` = Go's nil slice).
    patterns_strings: Option<Vec<String>>,
}

decode_struct!(Whitelist, "security.Whitelist", |_s| Vec::new());

/// `security.Whitelist` as a Go value (the result of the security decode hook, assigned by
/// mapstructure as it is).
impl Object for Whitelist {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("security.Whitelist")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "Accept" | "String")
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "String" => Some(Ok(Value::string(self.string()))),
            "Accept" => Some(match args {
                [Value::String(s)] => Ok(Value::Bool(self.accept(&s.to_str_lossy()))),
                _ => Err(go_value::Error::new("Accept: expected one string argument")),
            }),
            _ => None,
        }
    }
    fn go_string(&self) -> Option<go_value::GoString> {
        Some(go_value::GoString::from(self.string()))
    }
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(self.marshal_json_bytes().map_err(Into::into))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Whitelist {
    /// Go: `NewWhitelist(patterns...)`: no patterns (or only blank ones, or the keyword `none`
    /// before any other) accept nothing.
    // Go: config/security/whitelist.go:NewWhitelist
    pub fn new(patterns: &[&str]) -> Result<Whitelist> {
        if patterns.is_empty() {
            return Ok(Whitelist {
                accept_none: true,
                ..Default::default()
            });
        }

        let mut accept_some = false;
        let mut patterns_strings: Vec<String> = Vec::new();

        for p in patterns {
            if *p == ACCEPT_NONE_KEYWORD {
                accept_some = false;
                break;
            }

            let ps = go_unicode::strings::trim_space_str(p);
            if !ps.is_empty() {
                accept_some = true;
                patterns_strings.push(ps.to_string());
            }
        }

        if !accept_some {
            return Ok(Whitelist {
                accept_none: true,
                ..Default::default()
            });
        }

        let mut patternsr = Vec::new();

        for p in patterns {
            let p = go_unicode::strings::trim_space_str(p);
            if p.is_empty() {
                continue;
            }
            let re = Regexp::compile(p).map_err(|e| {
                Error::new(format!(
                    "failed to compile whitelist pattern {}: {}",
                    go_strconv::quote(p),
                    e
                ))
            })?;
            patternsr.push(re);
        }

        Ok(Whitelist {
            accept_none: false,
            patterns: patternsr,
            patterns_strings: Some(patterns_strings),
        })
    }

    /// Go: `MustNewWhitelist(patterns...)` (panics on error).
    // Go: config/security/whitelist.go:MustNewWhitelist
    pub fn must_new(patterns: &[&str]) -> Whitelist {
        match Whitelist::new(patterns) {
            Ok(w) => w,
            Err(e) => panic!("{e}"),
        }
    }

    // Go: config/security/whitelist.go:Accept
    pub fn accept(&self, name: &str) -> bool {
        if self.accept_none {
            return false;
        }

        self.patterns.iter().any(|p| p.match_string(name))
    }

    /// Whether this whitelist accepts nothing (Go's `acceptNone`).
    pub fn accept_none(&self) -> bool {
        self.accept_none
    }

    /// The trimmed source patterns (Go's `patternsStrings`; `None` = nil).
    pub fn patterns_strings(&self) -> Option<&[String]> {
        self.patterns_strings.as_deref()
    }

    /// Go: `MarshalJSON`: `"none"` or the patterns (`null` for none).
    // Go: config/security/whitelist.go:MarshalJSON
    pub fn marshal_json_bytes(&self) -> Result<Vec<u8>> {
        let v = if self.accept_none {
            Value::string(ACCEPT_NONE_KEYWORD)
        } else {
            match &self.patterns_strings {
                None => Value::TypedNil(std::sync::Arc::from("[]string")),
                Some(p) => Value::string_list(p.iter().map(String::as_str)),
            }
        };
        go_json::marshal(&v).map_err(|e| Error::new(e.to_string()))
    }

    /// Go: `String()` = `fmt.Sprint(patternsStrings)`.
    // Go: config/security/whitelist.go:String
    pub fn string(&self) -> String {
        match &self.patterns_strings {
            None => "[]".to_string(),
            Some(p) => format!("[{}]", p.join(" ")),
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/security/whitelist.go (116 lines; 3/5 funcs executed)
//   types: Whitelist
// OK L37-43: (w Whitelist) MarshalJSON() ([]byte, error)
// OK L48-89: NewWhitelist(patterns ...string) (Whitelist, error)
// OK L92-98: MustNewWhitelist(patterns ...string) Whitelist
// OK L101-112: (w Whitelist) Accept(name string) bool
// OK L114-116: (w Whitelist) String() string
// ---------------------------------------------------------------------------
