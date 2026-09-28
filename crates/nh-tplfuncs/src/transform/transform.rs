//! Port of `tpl/transform/transform.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! Stubs (explicit `neohugo-rs: ... is not supported` errors; HUGO_LAYER.md §1 rule 5):
//! `Highlight`, `HighlightCodeBlock` and `CanHighlight` (Chroma is not ported), `PortableText`
//! (goportabletext), `ToMath` (KaTeX over WASM). `Emojify` goes through nh-helpers' `Emojify`,
//! which is a stub too (the kyokomi emoji table is not ported).

use std::sync::Arc;

use go_value::{GoString, HostCtx, Object, Value};
use nh_common::cast::caste;
use nh_common::dynacache::{ClearWhen, OptionsPartition, Partition, get_or_create_partition};
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

// Parity notes: `markdownify` = home page RenderString (inline) + TrimShortHTML; `unmarshal` of a resource/string (JSON -> float64 numbers, map[string]interface {}); `htmlEscape`/`plainify` ...

/// Go: `transform.Namespace` (template value `*transform.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    /// Go: `cacheUnmarshal` (`*resources.StaleValue[any]`; nothing goes stale in a one-shot
    /// build, so the value is stored as it is).
    pub(crate) cache_unmarshal: Arc<Partition<String, Value>>,
}

pub(crate) fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

pub(crate) fn unsupported(what: &str) -> go_value::Error {
    gerr(format!("neohugo-rs: {what} is not supported"))
}

impl Namespace {
    /// New returns a new instance of the transform-namespaced template functions.
    // Go: tpl/transform/transform.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let cache_unmarshal = get_or_create_partition::<String, Value>(
            &d.mem_cache,
            "/tmpl/transform/unmarshal",
            OptionsPartition {
                weight: 30,
                clear_when: ClearWhen::OnChange,
            },
        );
        Namespace { d, cache_unmarshal }
    }

    /// CanHighlight returns whether the given code language is supported by the Chroma
    /// highlighter. STUB: Chroma is not ported.
    // Go: tpl/transform/transform.go:CanHighlight
    pub fn can_highlight(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "CanHighlight")?;
        args::string(a, 0)?;
        Err(unsupported("transform.CanHighlight (chroma lexers)"))
    }

    /// Emojify returns a copy of s with all emoji codes replaced with actual emojis.
    // Go: tpl/transform/transform.go:Emojify
    pub fn emojify(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Emojify")?;
        let ss = caste::to_string_e(&a[0])?;
        Ok(Value::html(nh_helpers::emoji::emojify(ss.as_bytes())?))
    }

    /// Highlight returns a copy of s as an HTML string with syntax highlighting applied.
    /// STUB: Chroma is not ported.
    // Go: tpl/transform/transform.go:Highlight
    pub fn highlight(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "Highlight")?;
        caste::to_string_e(&a[0])?;
        args::string(a, 1)?;
        Err(unsupported("transform.Highlight (chroma)"))
    }

    /// HighlightCodeBlock highlights a code block on the form received in the codeblock render
    /// hooks. STUB: Chroma is not ported.
    // Go: tpl/transform/transform.go:HighlightCodeBlock
    pub fn highlight_code_block(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "HighlightCodeBlock")?;
        Err(unsupported("transform.HighlightCodeBlock (chroma)"))
    }

    /// HTMLEscape returns a copy of s with reserved HTML characters escaped.
    // Go: tpl/transform/transform.go:HTMLEscape
    pub fn html_escape(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "HTMLEscape")?;
        let ss = caste::to_string_e(&a[0])?;
        Ok(Value::string(go_html::escape_string_bytes(ss.as_bytes())))
    }

    /// HTMLUnescape returns a copy of s with HTML escape requences converted to plain text.
    // Go: tpl/transform/transform.go:HTMLUnescape
    pub fn html_unescape(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "HTMLUnescape")?;
        let ss = caste::to_string_e(&a[0])?;
        Ok(Value::string(go_html::unescape_string_bytes(ss.as_bytes())))
    }

    /// Markdownify renders s from Markdown to HTML.
    // Go: tpl/transform/transform.go:Markdownify
    pub fn markdownify(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Markdownify")?;
        let home = self.d.site.get().and_then(|s| s.0.home());
        let Some(home) = home else {
            // Go: panic("home must not be nil").
            return Err(gerr("home must not be nil"));
        };
        let ss = home.0.render_string(ctx, std::slice::from_ref(&a[0]))?;
        let ss = caste::to_string_e(&ss)?;

        // Strip if this is a short inline type of text.
        let bb = self
            .d
            .content_spec()
            .trim_short_html(ss.as_bytes(), "markdown");

        Ok(nh_helpers::content::bytes_to_html(&bb))
    }

    /// Plainify returns a copy of s with all HTML tags removed.
    // Go: tpl/transform/transform.go:Plainify
    pub fn plainify(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Plainify")?;
        let ss = caste::to_string_e(&a[0])?;
        Ok(Value::html(nh_tpl::template::strip_html(ss.as_bytes())))
    }

    /// PortableText converts the portable text in v to Markdown. STUB: goportabletext is not
    /// ported.
    // Go: tpl/transform/transform.go:PortableText
    pub fn portable_text(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "PortableText")?;
        Err(unsupported("transform.PortableText (goportabletext)"))
    }

    /// For internal use.
    // Go: tpl/transform/transform.go:Reset
    pub fn reset(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "Reset")?;
        self.cache_unmarshal.clear();
        // Go: a method without results; a template call of it is rejected by text/template.
        Err(gerr("can't call method/function \"Reset\" with 0 results"))
    }

    /// ToMath converts a LaTeX string to math in the given format, default MathML. STUB: KaTeX
    /// (WASM) is not ported.
    // Go: tpl/transform/transform.go:ToMath
    pub fn to_math(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.is_empty() {
            return Err(gerr("must provide at least one argument"));
        }
        caste::to_string_e(&a[0])?;
        Err(unsupported("transform.ToMath (KaTeX)"))
    }

    /// XMLEscape returns the given string, removing disallowed characters then escaping the
    /// result to its XML equivalent.
    // Go: tpl/transform/transform.go:XMLEscape
    pub fn xml_escape(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "XMLEscape")?;
        let ss = caste::to_string_e(&a[0])?;

        // https://www.w3.org/TR/xml/#NT-Char
        let cleaned = go_unicode::strings::map(
            |r| {
                if r == 0x9
                    || r == 0xA
                    || r == 0xD
                    || (0x20..=0xD7FF).contains(&r)
                    || (0xE000..=0xFFFD).contains(&r)
                    || (0x10000..=0x10FFFF).contains(&r)
                {
                    return r;
                }
                -1
            },
            ss.as_bytes(),
        );

        Ok(Value::String(GoString::from(xml_escape_text(&cleaned))))
    }
}

/// Go: `encoding/xml.EscapeText` (`escapeText(w, s, true)`).
// Go: encoding/xml/xml.go:escapeText
fn xml_escape_text(s: &[u8]) -> Vec<u8> {
    fn in_character_range(r: i32) -> bool {
        r == 0x09
            || r == 0x0A
            || r == 0x0D
            || (0x20..=0xD7FF).contains(&r)
            || (0xE000..=0xFFFD).contains(&r)
            || (0x10000..=0x10FFFF).contains(&r)
    }
    let mut out = Vec::with_capacity(s.len());
    let mut last = 0;
    let mut i = 0;
    while i < s.len() {
        let (r, width) = go_unicode::utf8::decode_rune(&s[i..]);
        i += width;
        let esc: &[u8] = match r {
            0x22 => b"&#34;",
            0x27 => b"&#39;",
            0x26 => b"&amp;",
            0x3C => b"&lt;",
            0x3E => b"&gt;",
            0x09 => b"&#x9;",
            0x0A => b"&#xA;",
            0x0D => b"&#xD;",
            _ => {
                if !in_character_range(r) || (r == 0xFFFD && width == 1) {
                    "\u{FFFD}".as_bytes()
                } else {
                    continue;
                }
            }
        };
        out.extend_from_slice(&s[last..i - width]);
        out.extend_from_slice(esc);
        last = i;
    }
    out.extend_from_slice(&s[last..]);
    out
}

nh_common::go_methods!(Namespace {
    "CanHighlight" => |n, ctx, a| n.can_highlight(ctx, a),
    "Emojify" => |n, ctx, a| n.emojify(ctx, a),
    "Highlight" => |n, ctx, a| n.highlight(ctx, a),
    "HighlightCodeBlock" => |n, ctx, a| n.highlight_code_block(ctx, a),
    "HTMLEscape" => |n, ctx, a| n.html_escape(ctx, a),
    "HTMLUnescape" => |n, ctx, a| n.html_unescape(ctx, a),
    "Markdownify" => |n, ctx, a| n.markdownify(ctx, a),
    "Plainify" => |n, ctx, a| n.plainify(ctx, a),
    "PortableText" => |n, ctx, a| n.portable_text(ctx, a),
    "Remarshal" => |n, ctx, a| n.remarshal(ctx, a),
    "Reset" => |n, ctx, a| n.reset(ctx, a),
    "ToMath" => |n, ctx, a| n.to_math(ctx, a),
    "Unmarshal" => |n, ctx, a| n.unmarshal(ctx, a),
    "XMLEscape" => |n, ctx, a| n.xml_escape(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*transform.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/transform/transform.go (326 lines; 2/13 funcs executed)
//   types: Namespace
// OK L51-69: New(deps *deps.Deps) *Namespace
// OK L83-90: (ns *Namespace) Emojify(s any) (template.HTML, error) (helpers.Emojify is an nh-helpers STUB)
// STUB L94-111: (ns *Namespace) Highlight(s any, lang string, opts ...any) (template.HTML, error)
// STUB L114-123: (ns *Namespace) HighlightCodeBlock(ctx hooks.CodeblockContext, opts ...any) (highlight.HighlightResult, error)
// STUB L126-128: (ns *Namespace) CanHighlight(language string) bool
// OK L131-138: (ns *Namespace) HTMLEscape(s any) (string, error)
// OK L142-149: (ns *Namespace) HTMLUnescape(s any) (string, error)
// OK L153-177: (ns *Namespace) XMLEscape(s any) (string, error)
// OK L180-194: (ns *Namespace) Markdownify(ctx context.Context, s any) (template.HTML, error)
// OK L197-204: (ns *Namespace) Plainify(s any) (template.HTML, error)
// STUB L208-219: (ns *Namespace) PortableText(v any) (string, error)
// STUB L223-321: (ns *Namespace) ToMath(ctx context.Context, args ...any) (template.HTML, error)
// OK L324-326: (ns *Namespace) Reset()
// ---------------------------------------------------------------------------
