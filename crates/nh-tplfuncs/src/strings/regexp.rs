//! Port of `tpl/strings/regexp.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! The regexps are Go's (`nh_common::goregexp`, compiled through Go's cache
//! `hstrings.GetOrCompileRegexp`) and run over the template's byte strings.

use std::sync::Arc;

use go_value::{GoString, HostCtx, SliceType, Value};
use nh_common::hstrings::get_or_compile_regexp;
use nh_common::object::{GoResult, args};

use super::strings::{Namespace, string_slice, sv, to_int_e, to_string_e};

impl Namespace {
    /// FindRE returns a list of strings that match the regular expression. By default all
    /// matches will be included. The number of matches can be limited with an optional third
    /// parameter.
    // Go: tpl/strings/regexp.go:FindRE
    pub fn find_re(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "FindRE")?;
        let expr = args::string(a, 0)?;
        let re = get_or_compile_regexp(expr.as_bytes())?;

        let conv = to_string_e(&a[1])?;

        let lim = if a.len() == 2 {
            -1
        } else {
            to_int_e(&a[2])? as isize
        };

        let found = re.find_all(conv.as_bytes(), lim);
        // Go: FindAllString returns nil when there is no match.
        if found.is_empty() {
            return Ok(string_slice(None));
        }
        Ok(string_slice(Some(
            found.into_iter().map(|s| s.to_vec()).collect(),
        )))
    }

    /// FindRESubmatch returns a slice of all successive matches of the regular expression in
    /// content. Each element is a slice of strings holding the text of the leftmost match of the
    /// regular expression and the matches, if any, of its subexpressions.
    ///
    /// By default all matches will be included. The number of matches can be limited with the
    /// optional limit parameter. A return value of nil indicates no match.
    // Go: tpl/strings/regexp.go:FindRESubmatch
    pub fn find_re_submatch(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "FindRESubmatch")?;
        let expr = args::string(a, 0)?;
        let re = get_or_compile_regexp(expr.as_bytes())?;

        let conv = to_string_e(&a[1])?;
        let mut n: isize = -1;
        if a.len() > 2 {
            n = to_int_e(&a[2])? as isize;
        }

        let found = re.find_all_submatch(conv.as_bytes(), n);
        if found.is_empty() {
            return Ok(Value::TypedNil(Arc::from("[][]string")));
        }
        let items = found
            .into_iter()
            .map(|m| {
                Value::list(
                    SliceType::String,
                    m.into_iter()
                        .map(|g| Value::String(GoString::from(g.unwrap_or(b"").to_vec())))
                        .collect(),
                )
            })
            .collect();
        Ok(Value::list(
            SliceType::Named(Arc::from("[][]string")),
            items,
        ))
    }

    /// ReplaceRE returns a copy of s, replacing all matches of the regular expression pattern
    /// with the replacement text repl. The number of replacements can be limited with an
    /// optional fourth parameter.
    // Go: tpl/strings/regexp.go:ReplaceRE
    pub fn replace_re(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 3, "ReplaceRE")?;
        let sp = to_string_e(&a[0])?;
        let sr = to_string_e(&a[1])?;
        let ss = to_string_e(&a[2])?;

        let mut nn: i64 = -1;
        if a.len() > 3 {
            nn = to_int_e(&a[3])?;
        }

        let re = get_or_compile_regexp(sp.as_bytes())?;

        let out = re.replace_all_func(ss.as_bytes(), |s| {
            if nn == 0 {
                return s.to_vec();
            }

            nn -= 1;
            re.replace_all(s, sr.as_bytes())
        });
        Ok(sv(out))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/strings/regexp.go (115 lines; 0/3 funcs executed)
// OK L23-44: (ns *Namespace) FindRE(expr string, content any, limit ...any) ([]string, error)
// OK L54-73: (ns *Namespace) FindRESubmatch(expr string, content any, limit ...any) ([][]string, error)
// OK L78-115: (ns *Namespace) ReplaceRE(pattern, repl, s any, n ...any) (_ string, err error)
// ---------------------------------------------------------------------------
