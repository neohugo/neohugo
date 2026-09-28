//! Module `goi18n::template`.
//!
//! PORT internal/template.go (text/template, no funcs, lazy parse)
//!
//! Owner: Wave B task T17 (i18n).

use std::sync::{Arc, OnceLock};

use go_value::{HostCtx, Kind, Value};
use gotemplate::text::ExecHelper;
use nh_common::Result;

/// Go: go-i18n `internal/template.go` — Go text/template with no funcs, parsed lazily; fast path
/// returns the source when it has no left delimiter. Printing follows text/template (`<no value>`
/// for nil, Go `%v` floats).
#[derive(Clone)]
pub struct Template {
    pub src: String,
    pub left_delim: String,
    pub right_delim: String,
    /// Go `parseOnce` + `parsedTemplate` + `parseError` (shared by clones, like Go's pointer).
    parsed: Arc<OnceLock<std::result::Result<gotemplate::text::Template, String>>>,
}

impl std::fmt::Debug for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Template")
            .field("src", &self.src)
            .field("left_delim", &self.left_delim)
            .field("right_delim", &self.right_delim)
            .finish()
    }
}

impl Template {
    pub fn new(src: &str, left_delim: &str, right_delim: &str) -> Template {
        Template {
            src: src.to_string(),
            left_delim: left_delim.to_string(),
            right_delim: right_delim.to_string(),
            parsed: Arc::new(OnceLock::new()),
        }
    }

    /// Go: `Template.Execute(nil, data)` without a host context.
    pub fn execute(&self, data: &Value) -> Result<String> {
        self.execute_ctx(&(), data)
            .map_err(nh_common::herrors::Error::new)
    }

    /// Go: `Template.Execute(funcs, data)` with `funcs == nil` (Hugo never passes funcs). The
    /// host context is handed to the methods the template calls (Go gets it from the
    /// `page.PageWithContext` wrapper Hugo passes as data). The error is Go's text.
    // Go: go-i18n internal/template.go:Execute
    pub fn execute_ctx(
        &self,
        ctx: HostCtx<'_>,
        data: &Value,
    ) -> std::result::Result<String, String> {
        let left_delim = if self.left_delim.is_empty() {
            "{{"
        } else {
            self.left_delim.as_str()
        };
        if !self.src.contains(left_delim) {
            // Fast path to avoid parsing a template that has no actions.
            return Ok(self.src.clone());
        }

        // If funcs is nil, then we only need to parse this template once.
        let gt = self.parsed.get_or_init(|| {
            let t = gotemplate::text::Template::new("");
            t.delims(&self.left_delim, &self.right_delim);
            t.parse(self.src.as_bytes()).map_err(|e| e.message())
        });
        let gt = match gt {
            Ok(gt) => gt,
            Err(e) => return Err(e.clone()),
        };
        let mut buf: Vec<u8> = Vec::new();
        if let Err(e) = gt.execute_with_helper(ctx, &StdHelper, &mut buf, data) {
            return Err(e.message());
        }
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
}

/// The exec helper of a plain Go `text/template` (go-i18n uses the standard library, not
/// Hugo's fork): no functions beyond the builtins, exact map keys, Go's reflection method
/// sets, and Go's `isTrue` (`template.IsTrue`) for `if`/`with`/`and`/`or`/`not` instead of
/// Hugo's `hreflect.IsTruthfulValue`.
pub(crate) struct StdHelper;

impl ExecHelper for StdHelper {
    fn is_true(&self, v: &Value) -> bool {
        is_true(v)
    }
}

/// Go: `text/template.isTrue(val)` (the truth; the `ok == false` case is complex numbers,
/// which the value model does not have).
// Go: text/template/exec.go:isTrue
pub(crate) fn is_true(v: &Value) -> bool {
    match v {
        // Something like var x interface{}, never set. It's a form of nil.
        Value::Invalid => false,
        Value::TypedNil(_) => false,
        Value::Bool(b) => *b,
        Value::Int(i, _) => *i != 0,
        Value::Uint(u, _) => *u != 0,
        Value::Float(f, _) => *f != 0.0,
        Value::String(s) | Value::Safe(_, s) => !s.is_empty(),
        // Struct values are always true.
        Value::Time(_) => true,
        Value::List(l) => !l.items.is_empty(),
        Value::Map(m) => !m.entries.is_empty(),
        Value::Object(o) => {
            if let Some(u) = o.underlying() {
                return is_true(&u);
            }
            match o.kind() {
                Kind::Map => !o.map_keys().is_empty(),
                Kind::Slice => o.list().is_some_and(|l| !l.is_empty()),
                // Non-nil pointers, funcs and interfaces; struct values.
                Kind::Ptr | Kind::Struct | Kind::Func | Kind::Interface => true,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (go-i18n internal/template.go)
// OK Template.Execute (funcs == nil path; Hugo never passes funcs)
// ---------------------------------------------------------------------------
