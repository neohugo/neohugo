//! Port of `resources/resource_transformers/tocss/sass/helpers.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

use go_value::{Map, Value};
use nh_common::goregexp::Regexp;
use nh_common::types::css::{QuotedString, UnquotedString};
use std::sync::LazyLock;

pub const HUGO_VARS_NAMESPACE: &str = "hugo:vars";
/// Transpiler implementation can be controlled from the client by setting the 'transpiler'
/// option. Default is currently 'libsass', but that may change.
pub const TRANSPILER_DART: &str = "dartsass";
pub const TRANSPILER_LIB_SASS: &str = "libsass";

/// Go: `CreateVarsStyleSheet(transpiler, vars)` — `vars == None` is Go's nil map (`""`).
// Go: resources/resource_transformers/tocss/sass/helpers.go:CreateVarsStyleSheet
pub fn create_vars_style_sheet(transpiler: &str, vars: Option<&Map>) -> String {
    let Some(vars) = vars else {
        return String::new();
    };

    let mut vars_slice: Vec<Vec<u8>> = Vec::new();
    for (k, v) in vars.entries.iter() {
        let k = k.as_bytes();
        let prefix: &[u8] = if !k.starts_with(b"$") { b"$" } else { b"" };
        let name = [prefix, k].concat();
        let name = Value::string(go_value::GoString::from(name));

        if is_quoted_string(v) {
            // Marked by the user as a string that needs to be quoted.
            vars_slice.push(go_fmt::sprintf("%s: %q;", &[name, v.clone()]));
        } else if is_typed_css_value(v) {
            // E.g. 24px, 1.5rem, 10%, hsl(0, 0%, 100%), calc(24px + 36px), #fff, #ffffff.
            vars_slice.push(go_fmt::sprintf("%s: %v;", &[name, v.clone()]));
        } else if transpiler == TRANSPILER_DART {
            // unquote will preserve quotes around URLs etc. if needed.
            vars_slice.push(go_fmt::sprintf(
                "%s: string.unquote(%q);",
                &[name, v.clone()],
            ));
        } else {
            vars_slice.push(go_fmt::sprintf("%s: unquote(%q);", &[name, v.clone()]));
        }
    }
    // sort.Strings (byte order); the map order does not matter after it.
    vars_slice.sort();

    let joined = vars_slice.join(&b"\n"[..]);
    let s = if transpiler == TRANSPILER_DART {
        [b"@use \"sass:string\";\n".as_slice(), &joined].concat()
    } else {
        joined
    };
    String::from_utf8(s).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

fn is_quoted_string(v: &Value) -> bool {
    v.as_object()
        .is_some_and(|o| o.as_any().downcast_ref::<QuotedString>().is_some())
}

static IS_CSS_COLOR: LazyLock<Regexp> =
    LazyLock::new(|| Regexp::must_compile(r"^#[0-9a-fA-F]{3,6}$"));
static IS_CSS_FUNC: LazyLock<Regexp> = LazyLock::new(|| Regexp::must_compile(r"^([a-zA-Z-]+)\("));
static IS_CSS_UNIT: LazyLock<Regexp> =
    LazyLock::new(|| Regexp::must_compile(r"^([0-9]+)(\.[0-9]+)?([a-zA-Z-%]+)$"));

/// isTypedCSSValue returns true if the given string is a CSS value that we should preserve the
/// type of, as in: Not wrap it in quotes.
// Go: resources/resource_transformers/tocss/sass/helpers.go:isTypedCSSValue
fn is_typed_css_value(v: &Value) -> bool {
    match v {
        Value::Int(..) | Value::Uint(..) | Value::Float(..) => true,
        Value::Object(o) if o.as_any().downcast_ref::<UnquotedString>().is_some() => true,
        Value::String(s) => {
            let s = s.as_bytes();
            IS_CSS_COLOR.is_match(s) || IS_CSS_FUNC.is_match(s) || IS_CSS_UNIT.is_match(s)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/tocss/sass/helpers.go (102 lines; 1/2 funcs executed)
// OK L34-74: CreateVarsStyleSheet(transpiler string, vars map[string]any) string
// OK L84-102: isTypedCSSValue(v any) bool
// ---------------------------------------------------------------------------
