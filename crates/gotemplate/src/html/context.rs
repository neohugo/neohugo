//! Go: tpl/internal/go_templates/htmltemplate/context.go (+ the generated
//! `state_string.go`, `delim_string.go`, `urlpart_string.go`,
//! `jsctx_string.go`, `element_string.go`, `attr_string.go`).
//!
//! go1.24 fork: `eq` ignores `jsBraceDepth` (and `n`), and there is no
//! `clone`: copying a context shares the `jsBraceDepth` backing array, which
//! [`IntSlice`] reproduces.

use std::fmt;
use std::sync::{Arc, Mutex};

use crate::parse::Node;

use super::error::Error;

// ---------------------------------------------------------------------------
// state

/// Go: `state` — a high-level HTML parser state.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash, PartialOrd, Ord)]
pub(crate) enum State {
    /// Go: `stateText` — parsed character data.
    #[default]
    Text,
    /// Go: `stateTag` — before an HTML attribute or the end of a tag.
    Tag,
    /// Go: `stateAttrName` — inside an attribute name.
    AttrName,
    /// Go: `stateAfterName` — after an attr name, before any equals sign.
    AfterName,
    /// Go: `stateBeforeValue` — after the equals sign, before the value.
    BeforeValue,
    /// Go: `stateHTMLCmt` — inside an `<!-- HTML comment -->`.
    HtmlCmt,
    /// Go: `stateRCDATA` — inside an RCDATA element (`<textarea>`, `<title>`).
    Rcdata,
    /// Go: `stateAttr` — inside an HTML attribute whose content is text.
    Attr,
    /// Go: `stateURL` — inside an HTML attribute whose content is a URL.
    Url,
    /// Go: `stateSrcset` — inside an HTML srcset attribute.
    Srcset,
    /// Go: `stateJS` — inside an event handler or script element.
    Js,
    /// Go: `stateJSDqStr` — inside a JavaScript double quoted string.
    JsDqStr,
    /// Go: `stateJSSqStr` — inside a JavaScript single quoted string.
    JsSqStr,
    /// Go: `stateJSTmplLit` — inside a JavaScript back quoted string.
    JsTmplLit,
    /// Go: `stateJSRegexp` — inside a JavaScript regexp literal.
    JsRegexp,
    /// Go: `stateJSBlockCmt` — inside a JavaScript `/* block comment */`.
    JsBlockCmt,
    /// Go: `stateJSLineCmt` — inside a JavaScript `// line comment`.
    JsLineCmt,
    /// Go: `stateJSHTMLOpenCmt` — inside a JavaScript `<!--` HTML-like comment.
    JsHtmlOpenCmt,
    /// Go: `stateJSHTMLCloseCmt` — inside a JavaScript `-->` HTML-like comment.
    JsHtmlCloseCmt,
    /// Go: `stateCSS` — inside a `<style>` element or style attribute.
    Css,
    /// Go: `stateCSSDqStr` — inside a CSS double quoted string.
    CssDqStr,
    /// Go: `stateCSSSqStr` — inside a CSS single quoted string.
    CssSqStr,
    /// Go: `stateCSSDqURL` — inside a CSS double quoted `url("...")`.
    CssDqUrl,
    /// Go: `stateCSSSqURL` — inside a CSS single quoted `url('...')`.
    CssSqUrl,
    /// Go: `stateCSSURL` — inside a CSS unquoted `url(...)`.
    CssUrl,
    /// Go: `stateCSSBlockCmt` — inside a CSS `/* block comment */`.
    CssBlockCmt,
    /// Go: `stateCSSLineCmt` — inside a CSS `// line comment`.
    CssLineCmt,
    /// Go: `stateError` — an infectious error state.
    Error,
    /// Go: `stateDead` — unreachable code after a `{{break}}`/`{{continue}}`.
    Dead,
}

impl State {
    /// All states in Go's numeric order (`State::ALL[i] as u8 == i`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: [State; 29] = [
        State::Text,
        State::Tag,
        State::AttrName,
        State::AfterName,
        State::BeforeValue,
        State::HtmlCmt,
        State::Rcdata,
        State::Attr,
        State::Url,
        State::Srcset,
        State::Js,
        State::JsDqStr,
        State::JsSqStr,
        State::JsTmplLit,
        State::JsRegexp,
        State::JsBlockCmt,
        State::JsLineCmt,
        State::JsHtmlOpenCmt,
        State::JsHtmlCloseCmt,
        State::Css,
        State::CssDqStr,
        State::CssSqStr,
        State::CssDqUrl,
        State::CssSqUrl,
        State::CssUrl,
        State::CssBlockCmt,
        State::CssLineCmt,
        State::Error,
        State::Dead,
    ];

    // Go: state_string.go:(state).String
    /// The Go stringer name (`stateRCDATA`, ...).
    pub(crate) fn string(self) -> &'static str {
        match self {
            State::Text => "stateText",
            State::Tag => "stateTag",
            State::AttrName => "stateAttrName",
            State::AfterName => "stateAfterName",
            State::BeforeValue => "stateBeforeValue",
            State::HtmlCmt => "stateHTMLCmt",
            State::Rcdata => "stateRCDATA",
            State::Attr => "stateAttr",
            State::Url => "stateURL",
            State::Srcset => "stateSrcset",
            State::Js => "stateJS",
            State::JsDqStr => "stateJSDqStr",
            State::JsSqStr => "stateJSSqStr",
            State::JsTmplLit => "stateJSTmplLit",
            State::JsRegexp => "stateJSRegexp",
            State::JsBlockCmt => "stateJSBlockCmt",
            State::JsLineCmt => "stateJSLineCmt",
            State::JsHtmlOpenCmt => "stateJSHTMLOpenCmt",
            State::JsHtmlCloseCmt => "stateJSHTMLCloseCmt",
            State::Css => "stateCSS",
            State::CssDqStr => "stateCSSDqStr",
            State::CssSqStr => "stateCSSSqStr",
            State::CssDqUrl => "stateCSSDqURL",
            State::CssSqUrl => "stateCSSSqURL",
            State::CssUrl => "stateCSSURL",
            State::CssBlockCmt => "stateCSSBlockCmt",
            State::CssLineCmt => "stateCSSLineCmt",
            State::Error => "stateError",
            State::Dead => "stateDead",
        }
    }

    /// The state with Go's numeric value `v`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn from_u8(v: u8) -> Option<State> {
        State::ALL.get(v as usize).copied()
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.string())
    }
}

// Go: context.go:isComment
/// isComment is true for any state that contains content meant for template
/// authors & maintainers, not for end-users or machines.
pub(crate) fn is_comment(s: State) -> bool {
    matches!(
        s,
        State::HtmlCmt
            | State::JsBlockCmt
            | State::JsLineCmt
            | State::JsHtmlOpenCmt
            | State::JsHtmlCloseCmt
            | State::CssBlockCmt
            | State::CssLineCmt
    )
}

// Go: context.go:isInTag
/// isInTag return whether s occurs solely inside an HTML tag.
pub(crate) fn is_in_tag(s: State) -> bool {
    matches!(
        s,
        State::Tag | State::AttrName | State::AfterName | State::BeforeValue | State::Attr
    )
}

// Go: context.go:isInScriptLiteral
/// isInScriptLiteral returns true if s is one of the literal states within a
/// `<script>` tag, and as such occurrences of `<!--`, `<script`, and
/// `</script` need to be treated specially.
pub(crate) fn is_in_script_literal(s: State) -> bool {
    // Ignore the comment states (stateJSBlockCmt, stateJSLineCmt,
    // stateJSHTMLOpenCmt, stateJSHTMLCloseCmt) because their content is already
    // omitted from the output.
    matches!(
        s,
        State::JsDqStr | State::JsSqStr | State::JsTmplLit | State::JsRegexp
    )
}

// ---------------------------------------------------------------------------
// delim, urlPart, jsCtx, element, attr

/// Go: `delim` — the delimiter that will end the current HTML attribute.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash, PartialOrd, Ord)]
pub(crate) enum Delim {
    /// Go: `delimNone` — outside any attribute.
    #[default]
    None,
    /// Go: `delimDoubleQuote`.
    DoubleQuote,
    /// Go: `delimSingleQuote`.
    SingleQuote,
    /// Go: `delimSpaceOrTagEnd` — a space or `>` closes the attribute.
    SpaceOrTagEnd,
}

impl Delim {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: [Delim; 4] = [
        Delim::None,
        Delim::DoubleQuote,
        Delim::SingleQuote,
        Delim::SpaceOrTagEnd,
    ];

    // Go: delim_string.go:(delim).String
    pub(crate) fn string(self) -> &'static str {
        match self {
            Delim::None => "delimNone",
            Delim::DoubleQuote => "delimDoubleQuote",
            Delim::SingleQuote => "delimSingleQuote",
            Delim::SpaceOrTagEnd => "delimSpaceOrTagEnd",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn from_u8(v: u8) -> Option<Delim> {
        Delim::ALL.get(v as usize).copied()
    }
}

impl fmt::Display for Delim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.string())
    }
}

/// Go: `urlPart` — a part of an RFC 3986 hierarchical URL.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash, PartialOrd, Ord)]
pub(crate) enum UrlPart {
    /// Go: `urlPartNone`.
    #[default]
    None,
    /// Go: `urlPartPreQuery` — scheme, authority, or path.
    PreQuery,
    /// Go: `urlPartQueryOrFrag` — query or fragment.
    QueryOrFrag,
    /// Go: `urlPartUnknown` — joined contexts before and after the query.
    Unknown,
}

impl UrlPart {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: [UrlPart; 4] = [
        UrlPart::None,
        UrlPart::PreQuery,
        UrlPart::QueryOrFrag,
        UrlPart::Unknown,
    ];

    // Go: urlpart_string.go:(urlPart).String
    pub(crate) fn string(self) -> &'static str {
        match self {
            UrlPart::None => "urlPartNone",
            UrlPart::PreQuery => "urlPartPreQuery",
            UrlPart::QueryOrFrag => "urlPartQueryOrFrag",
            UrlPart::Unknown => "urlPartUnknown",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn from_u8(v: u8) -> Option<UrlPart> {
        UrlPart::ALL.get(v as usize).copied()
    }
}

impl fmt::Display for UrlPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.string())
    }
}

/// Go: `jsCtx` — whether a `/` starts a regular expression or a division.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash, PartialOrd, Ord)]
pub(crate) enum JsCtx {
    /// Go: `jsCtxRegexp`.
    #[default]
    Regexp,
    /// Go: `jsCtxDivOp`.
    DivOp,
    /// Go: `jsCtxUnknown`.
    Unknown,
}

impl JsCtx {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: [JsCtx; 3] = [JsCtx::Regexp, JsCtx::DivOp, JsCtx::Unknown];

    // Go: jsctx_string.go:(jsCtx).String
    pub(crate) fn string(self) -> &'static str {
        match self {
            JsCtx::Regexp => "jsCtxRegexp",
            JsCtx::DivOp => "jsCtxDivOp",
            JsCtx::Unknown => "jsCtxUnknown",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn from_u8(v: u8) -> Option<JsCtx> {
        JsCtx::ALL.get(v as usize).copied()
    }
}

impl fmt::Display for JsCtx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.string())
    }
}

/// Go: `element` — the HTML element when inside a start tag or special body.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash, PartialOrd, Ord)]
pub(crate) enum Element {
    /// Go: `elementNone`.
    #[default]
    None,
    /// Go: `elementScript`.
    Script,
    /// Go: `elementStyle`.
    Style,
    /// Go: `elementTextarea`.
    Textarea,
    /// Go: `elementTitle`.
    Title,
}

impl Element {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: [Element; 5] = [
        Element::None,
        Element::Script,
        Element::Style,
        Element::Textarea,
        Element::Title,
    ];

    // Go: element_string.go:(element).String
    pub(crate) fn string(self) -> &'static str {
        match self {
            Element::None => "elementNone",
            Element::Script => "elementScript",
            Element::Style => "elementStyle",
            Element::Textarea => "elementTextarea",
            Element::Title => "elementTitle",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn from_u8(v: u8) -> Option<Element> {
        Element::ALL.get(v as usize).copied()
    }
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.string())
    }
}

/// Go: `attr` — the current HTML attribute when inside the attribute.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash, PartialOrd, Ord)]
pub(crate) enum Attr {
    /// Go: `attrNone` — a normal attribute or no attribute.
    #[default]
    None,
    /// Go: `attrScript` — an event handler attribute.
    Script,
    /// Go: `attrScriptType` — the type attribute in a script element.
    ScriptType,
    /// Go: `attrStyle` — the style attribute whose value is CSS.
    Style,
    /// Go: `attrURL` — an attribute whose value is a URL.
    Url,
    /// Go: `attrSrcset` — a srcset attribute.
    Srcset,
}

impl Attr {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: [Attr; 6] = [
        Attr::None,
        Attr::Script,
        Attr::ScriptType,
        Attr::Style,
        Attr::Url,
        Attr::Srcset,
    ];

    // Go: attr_string.go:(attr).String
    pub(crate) fn string(self) -> &'static str {
        match self {
            Attr::None => "attrNone",
            Attr::Script => "attrScript",
            Attr::ScriptType => "attrScriptType",
            Attr::Style => "attrStyle",
            Attr::Url => "attrURL",
            Attr::Srcset => "attrSrcset",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn from_u8(v: u8) -> Option<Attr> {
        Attr::ALL.get(v as usize).copied()
    }
}

impl fmt::Display for Attr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.string())
    }
}

// ---------------------------------------------------------------------------
// IntSlice: Go `[]int` with slice aliasing

/// Go: a `[]int` value, with Go's slice semantics.
///
/// `context.jsBraceDepth` is a Go slice, and contexts are copied by value, so
/// every copy shares the backing array: `c.jsBraceDepth[len-1]++` in `tJS`
/// is seen by all copies of the context whose length covers that element,
/// and `append` writes in place while there is spare capacity. The go1.24
/// fork neither clones nor compares `jsBraceDepth`, so this aliasing is
/// observable in the escaper (e.g. both branches of an `{{if}}` inside a
/// `${...}` interpolation update the same counters). `Clone` of an
/// `IntSlice` is therefore a Go slice copy (same array), and
/// [`IntSlice::append`] grows the array exactly like Go's `growslice` for
/// 8-byte elements, so capacities (and thus which appends alias) match.
#[derive(Clone, Default)]
pub(crate) struct IntSlice {
    /// The backing array; its length is the slice capacity. `None` is a nil
    /// slice.
    arr: Option<Arc<Mutex<Vec<i64>>>>,
    len: usize,
}

impl IntSlice {
    /// A slice with these elements and `cap == len` (Go: `[]int{...}`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn from_vec(v: Vec<i64>) -> IntSlice {
        let len = v.len();
        IntSlice {
            arr: Some(Arc::new(Mutex::new(v))),
            len,
        }
    }

    /// Go: `len(s)`.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Go: `len(s) == 0`.
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Go: `cap(s)`.
    pub(crate) fn cap(&self) -> usize {
        match &self.arr {
            Some(a) => a.lock().unwrap_or_else(|e| e.into_inner()).len(),
            None => 0,
        }
    }

    /// Go: `s[i]`. Panics (like Go) when `i >= len(s)`.
    pub(crate) fn get(&self, i: usize) -> i64 {
        assert!(
            i < self.len,
            "index out of range [{i}] with length {}",
            self.len
        );
        let a = self.arr.as_ref().expect("non-empty slice has an array");
        a.lock().unwrap_or_else(|e| e.into_inner())[i]
    }

    /// Go: `s[i] = v` (visible through every slice sharing the array).
    /// Panics (like Go) when `i >= len(s)`.
    pub(crate) fn set(&self, i: usize, v: i64) {
        assert!(
            i < self.len,
            "index out of range [{i}] with length {}",
            self.len
        );
        let a = self.arr.as_ref().expect("non-empty slice has an array");
        a.lock().unwrap_or_else(|e| e.into_inner())[i] = v;
    }

    /// Go: `s[:n]` (shares the array). Panics (like Go) when `n > cap(s)`.
    pub(crate) fn reslice(&self, n: usize) -> IntSlice {
        assert!(n <= self.cap(), "slice bounds out of range [:{n}]");
        IntSlice {
            arr: self.arr.clone(),
            len: n,
        }
    }

    /// Go: `append(s, v)`: in place when `len(s) < cap(s)` (shared with the
    /// other slices of the array), else into a new array grown like
    /// `runtime.growslice`.
    pub(crate) fn append(&self, v: i64) -> IntSlice {
        let cap = self.cap();
        if self.len < cap {
            let a = self.arr.as_ref().expect("cap > 0 has an array");
            a.lock().unwrap_or_else(|e| e.into_inner())[self.len] = v;
            return IntSlice {
                arr: self.arr.clone(),
                len: self.len + 1,
            };
        }
        let new_len = self.len + 1;
        let new_cap = grow_cap_8(new_len, cap);
        let mut arr = vec![0i64; new_cap];
        if let Some(a) = &self.arr {
            let old = a.lock().unwrap_or_else(|e| e.into_inner());
            arr[..self.len].copy_from_slice(&old[..self.len]);
        }
        arr[self.len] = v;
        IntSlice {
            arr: Some(Arc::new(Mutex::new(arr))),
            len: new_len,
        }
    }

    /// The elements `s[:len(s)]`.
    pub(crate) fn to_vec(&self) -> Vec<i64> {
        match &self.arr {
            Some(a) => a.lock().unwrap_or_else(|e| e.into_inner())[..self.len].to_vec(),
            None => Vec::new(),
        }
    }

    /// Whether two slices share one backing array (Go: same pointer).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn same_array(&self, other: &IntSlice) -> bool {
        match (&self.arr, &other.arr) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl fmt::Debug for IntSlice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_vec())
    }
}

// Go: runtime/sizeclasses.go (internal/runtime/gc, go1.27.1): SizeClassToSize
const SIZE_CLASS_TO_SIZE: [usize; 68] = [
    0, 8, 16, 24, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240, 256, 288, 320,
    352, 384, 416, 448, 480, 512, 576, 640, 704, 768, 896, 1024, 1152, 1280, 1408, 1536, 1792,
    2048, 2304, 2688, 3072, 3200, 3456, 4096, 4864, 5376, 6144, 6528, 6784, 6912, 8192, 9472, 9728,
    10240, 10880, 12288, 13568, 14336, 16384, 18432, 19072, 20480, 21760, 24576, 27264, 28672,
    32768,
];

// Go: runtime/slice.go:nextslicecap
fn next_slice_cap(new_len: usize, old_cap: usize) -> usize {
    let mut newcap = old_cap as isize;
    let doublecap = newcap + newcap;
    if new_len as isize > doublecap {
        return new_len;
    }
    const THRESHOLD: isize = 256;
    if (old_cap as isize) < THRESHOLD {
        return doublecap as usize;
    }
    loop {
        // Transition from growing 2x for small slices
        // to growing 1.25x for large slices.
        newcap += (newcap + 3 * THRESHOLD) >> 2;
        if newcap as usize >= new_len {
            break;
        }
    }
    if newcap <= 0 {
        return new_len;
    }
    newcap as usize
}

// Go: runtime/msize.go:roundupsize (noscan: []int has no pointers)
fn roundupsize_noscan(size: usize) -> usize {
    const MAX_SMALL_SIZE: usize = 32768;
    const MALLOC_HEADER_SIZE: usize = 8;
    const PAGE_SIZE: usize = 8192;
    if size <= MAX_SMALL_SIZE - MALLOC_HEADER_SIZE {
        // The smallest size class that fits (what the size_to_class tables
        // index).
        for &c in &SIZE_CLASS_TO_SIZE[1..] {
            if c >= size {
                return c;
            }
        }
        return size;
    }
    let req = size.wrapping_add(PAGE_SIZE - 1);
    if req < size {
        return size;
    }
    req & !(PAGE_SIZE - 1)
}

// Go: runtime/slice.go:growslice (the capacity computation for et.Size_ == 8)
fn grow_cap_8(new_len: usize, old_cap: usize) -> usize {
    let newcap = next_slice_cap(new_len, old_cap);
    roundupsize_noscan(newcap * 8) / 8
}

// ---------------------------------------------------------------------------
// context

/// Go: `context` — the state an HTML parser must be in when it reaches the
/// portion of HTML produced by evaluating a particular template node.
///
/// The zero value (`Context::default()`) is the start context for a
/// template that produces an HTML fragment.
#[derive(Clone, Default, Debug)]
pub(crate) struct Context {
    pub(crate) state: State,
    pub(crate) delim: Delim,
    pub(crate) url_part: UrlPart,
    pub(crate) js_ctx: JsCtx,
    /// jsBraceDepth contains the current depth, for each JS template literal
    /// string interpolation expression, of braces we've seen. This is used to
    /// determine if the next } will close a JS template literal string
    /// interpolation expression or not. (A Go slice: copies share it.)
    pub(crate) js_brace_depth: IntSlice,
    pub(crate) attr: Attr,
    pub(crate) element: Element,
    /// For range break/continue.
    pub(crate) n: Option<Node>,
    /// Go: `*Error`, compared by pointer.
    pub(crate) err: Option<Arc<Error>>,
}

impl Context {
    /// A context in state `s` with every other field zero.
    pub(crate) fn with_state(state: State) -> Context {
        Context {
            state,
            ..Context::default()
        }
    }

    /// `context{state: stateError, err: err}`.
    pub(crate) fn error(err: Error) -> Context {
        Context {
            state: State::Error,
            err: Some(Arc::new(err)),
            ..Context::default()
        }
    }

    // Go: context.go:(context).String
    pub(crate) fn string(&self) -> String {
        // fmt.Sprintf("{%v %v %v %v %v %v %v}", ..., err) where a nil error
        // prints "<nil>" and an *Error prints its Error().
        let err = match &self.err {
            Some(e) => e.to_string(),
            None => "<nil>".to_string(),
        };
        format!(
            "{{{} {} {} {} {} {} {}}}",
            self.state.string(),
            self.delim.string(),
            self.url_part.string(),
            self.js_ctx.string(),
            self.attr.string(),
            self.element.string(),
            err
        )
    }

    // Go: context.go:(context).eq
    /// eq reports whether two contexts are equal (go1.24: `jsBraceDepth` and
    /// `n` are not compared; `err` is compared by pointer).
    pub(crate) fn eq(&self, d: &Context) -> bool {
        self.state == d.state
            && self.delim == d.delim
            && self.url_part == d.url_part
            && self.js_ctx == d.js_ctx
            && self.attr == d.attr
            && self.element == d.element
            && match (&self.err, &d.err) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
    }

    // Go: context.go:(context).mangle
    /// mangle produces an identifier that includes a suffix that distinguishes
    /// it from template names mangled with different contexts.
    pub(crate) fn mangle(&self, template_name: &str) -> String {
        // The mangled name for the default context is the input templateName.
        if self.state == State::Text {
            return template_name.to_string();
        }
        let mut s = format!("{}$htmltemplate_{}", template_name, self.state.string());
        if self.delim != Delim::None {
            s.push('_');
            s.push_str(self.delim.string());
        }
        if self.url_part != UrlPart::None {
            s.push('_');
            s.push_str(self.url_part.string());
        }
        if self.js_ctx != JsCtx::Regexp {
            s.push('_');
            s.push_str(self.js_ctx.string());
        }
        if self.attr != Attr::None {
            s.push('_');
            s.push_str(self.attr.string());
        }
        if self.element != Element::None {
            s.push('_');
            s.push_str(self.element.string());
        }
        s
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stringer_names_match_go_tables() {
        // Go: state_string.go _state_name / _state_index.
        let name = "stateTextstateTagstateAttrNamestateAfterNamestateBeforeValuestateHTMLCmtstateRCDATAstateAttrstateURLstateSrcsetstateJSstateJSDqStrstateJSSqStrstateJSTmplLitstateJSRegexpstateJSBlockCmtstateJSLineCmtstateJSHTMLOpenCmtstateJSHTMLCloseCmtstateCSSstateCSSDqStrstateCSSSqStrstateCSSDqURLstateCSSSqURLstateCSSURLstateCSSBlockCmtstateCSSLineCmtstateErrorstateDead";
        let index = [
            0, 9, 17, 30, 44, 60, 72, 83, 92, 100, 111, 118, 130, 142, 156, 169, 184, 198, 216,
            235, 243, 256, 269, 282, 295, 306, 322, 337, 347, 356,
        ];
        for (i, s) in State::ALL.iter().enumerate() {
            assert_eq!(s.string(), &name[index[i]..index[i + 1]]);
            assert_eq!(*s as usize, i);
            assert_eq!(State::from_u8(i as u8), Some(*s));
        }
        let name = "delimNonedelimDoubleQuotedelimSingleQuotedelimSpaceOrTagEnd";
        let index = [0, 9, 25, 41, 59];
        for (i, s) in Delim::ALL.iter().enumerate() {
            assert_eq!(s.string(), &name[index[i]..index[i + 1]]);
            assert_eq!(*s as usize, i);
        }
        let name = "urlPartNoneurlPartPreQueryurlPartQueryOrFragurlPartUnknown";
        let index = [0, 11, 26, 44, 58];
        for (i, s) in UrlPart::ALL.iter().enumerate() {
            assert_eq!(s.string(), &name[index[i]..index[i + 1]]);
            assert_eq!(*s as usize, i);
        }
        let name = "jsCtxRegexpjsCtxDivOpjsCtxUnknown";
        let index = [0, 11, 21, 33];
        for (i, s) in JsCtx::ALL.iter().enumerate() {
            assert_eq!(s.string(), &name[index[i]..index[i + 1]]);
            assert_eq!(*s as usize, i);
        }
        let name = "elementNoneelementScriptelementStyleelementTextareaelementTitle";
        let index = [0, 11, 24, 36, 51, 63];
        for (i, s) in Element::ALL.iter().enumerate() {
            assert_eq!(s.string(), &name[index[i]..index[i + 1]]);
            assert_eq!(*s as usize, i);
        }
        let name = "attrNoneattrScriptattrScriptTypeattrStyleattrURLattrSrcset";
        let index = [0, 8, 18, 32, 41, 48, 58];
        for (i, s) in Attr::ALL.iter().enumerate() {
            assert_eq!(s.string(), &name[index[i]..index[i + 1]]);
            assert_eq!(*s as usize, i);
        }
    }

    #[test]
    fn zero_context_string_and_mangle() {
        let c = Context::default();
        assert_eq!(
            c.string(),
            "{stateText delimNone urlPartNone jsCtxRegexp attrNone elementNone <nil>}"
        );
        assert_eq!(c.mangle("t"), "t");
        let c = Context {
            state: State::Rcdata,
            element: Element::Title,
            ..Context::default()
        };
        assert_eq!(
            c.mangle("title"),
            "title$htmltemplate_stateRCDATA_elementTitle"
        );
        let c = Context {
            state: State::Url,
            delim: Delim::DoubleQuote,
            url_part: UrlPart::PreQuery,
            js_ctx: JsCtx::DivOp,
            attr: Attr::Url,
            element: Element::Script,
            ..Context::default()
        };
        assert_eq!(
            c.mangle("x"),
            "x$htmltemplate_stateURL_delimDoubleQuote_urlPartPreQuery_jsCtxDivOp_attrURL_elementScript"
        );
    }

    #[test]
    fn eq_ignores_brace_depth_and_compares_err_by_pointer() {
        let a = Context {
            js_brace_depth: IntSlice::from_vec(vec![1]),
            ..Context::default()
        };
        let b = Context::default();
        assert!(a.eq(&b));
        let e1 = Context::error(super::super::error::Error::new(
            super::super::error::ErrorCode::ErrBadHTML,
            "x".to_string(),
        ));
        let e2 = Context::error(super::super::error::Error::new(
            super::super::error::ErrorCode::ErrBadHTML,
            "x".to_string(),
        ));
        assert!(!e1.eq(&e2));
        assert!(e1.eq(&e1.clone()));
    }

    #[test]
    fn context_is_send_sync() {
        fn check<T: Send + Sync + Clone + Default>() {}
        check::<Context>();
    }

    #[test]
    fn int_slice_go_semantics() {
        // append to nil: caps 1, 2, 4, 8, ... (8-byte elements).
        let mut s = IntSlice::default();
        let mut caps = Vec::new();
        for i in 0..20 {
            s = s.append(i);
            caps.push(s.cap());
        }
        assert_eq!(
            caps,
            [
                1, 2, 4, 4, 8, 8, 8, 8, 16, 16, 16, 16, 16, 16, 16, 16, 32, 32, 32, 32
            ]
        );
        // Aliasing: a copy sees in-place writes; append within capacity
        // overwrites what a longer copy sees.
        let a = IntSlice::from_vec(vec![0]).append(5); // cap 2, len 2
        let b = a.reslice(1);
        a.set(0, 7);
        assert_eq!(b.to_vec(), [7]);
        let c = b.append(9);
        assert!(c.same_array(&a));
        assert_eq!(a.to_vec(), [7, 9]);
        // Beyond the 256 threshold.
        assert_eq!(grow_cap_8(257, 256), 512);
        assert_eq!(grow_cap_8(513, 512), 848);
        assert_eq!(grow_cap_8(4097, 4096), 6144);
    }
}
