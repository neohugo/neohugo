//! Port of `resources/page/page_markup.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `resources/page/page_markup.go`: summary extraction (`.Summary` is computed eagerly by
//! page__content.go but unused by seeksnack) and the `page.Summary` value type.

use std::any::Any;
use std::borrow::Cow;

use go_value::{GoString, HostCtx, Object, SafeKind, Value};
use nh_common::types::types::LowHigh;
use nh_media::media::media_type::MediaType;

pub const SUMMARY_TYPE_AUTO: &str = "auto";
pub const SUMMARY_TYPE_MANUAL: &str = "manual";
pub const SUMMARY_TYPE_FRONT_MATTER: &str = "frontmatter";

/// Go: `page.Summary` (fields `Text` (template.HTML), `Type`, `Truncated`; `IsZero`;
/// `PrintableValue()` = Text).
#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub text: GoString,
    pub type_: String,
    pub truncated: bool,
}

impl Object for Summary {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("page.Summary")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "IsZero" | "PrintableValue")
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, _args: &[Value]) -> Option<go_value::Result<Value>> {
        match name {
            "IsZero" => Some(Ok(Value::Bool(self.text.is_empty()))),
            "PrintableValue" => Some(Ok(Value::Safe(SafeKind::Html, self.text.clone()))),
            _ => None,
        }
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Text" => Some(Value::Safe(SafeKind::Html, self.text.clone())),
            "Type" => Some(Value::string(self.type_.as_str())),
            "Truncated" => Some(Value::Bool(self.truncated)),
            _ => None,
        }
    }
    fn is_zero(&self) -> Option<bool> {
        Some(self.text.is_empty())
    }
    fn printable_value(&self) -> Option<Value> {
        Some(Value::Safe(SafeKind::Html, self.text.clone()))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.HtmlSummary`.
#[derive(Clone, Debug, Default)]
pub struct HtmlSummary {
    pub source: String,
    pub summary_low_high: LowHigh,
    pub summary_end_tag: LowHigh,
    pub wrapper_start: LowHigh,
    pub wrapper_end: LowHigh,
    pub divider: LowHigh,
}

impl HtmlSummary {
    // Go: resources/page/page_markup.go:Summary
    pub fn summary(&self) -> String { todo!() }
    // Go: resources/page/page_markup.go:ContentWithoutSummary
    pub fn content_without_summary(&self) -> String { todo!() }
    // Go: resources/page/page_markup.go:Truncated
    pub fn truncated(&self) -> bool { todo!() }
}

/// Go: `page.ExtractSummaryFromHTML(mt, input, numWords, isCJK)`.
// Go: resources/page/page_markup.go:ExtractSummaryFromHTML
pub fn extract_summary_from_html(mt: &MediaType, input: &str, num_words: i64, is_cjk: bool) -> HtmlSummary {
    todo!()
}

/// Go: `ExtractSummaryFromHTMLWithDivider`.
pub fn extract_summary_from_html_with_divider(mt: &MediaType, input: &str, divider: &str) -> HtmlSummary {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_markup.go (362 lines; 11/15 funcs executed)
//   types: Content, Markup, Summary, HtmlSummary, tagReStartEnd
// EX L63-65: (s Summary) IsZero() bool
//    L67-69: (s Summary) PrintableValue() any
// EX L82-87: (s HtmlSummary) wrap(ss string) string
// EX L89-95: (s HtmlSummary) wrapLeft(ss string) string
// EX L97-99: (s HtmlSummary) Value(l types.LowHigh[string]) string
// EX L101-103: (s HtmlSummary) trimSpace(ss string) string
//    L105-112: (s HtmlSummary) Content() string
// EX L114-126: (s HtmlSummary) Summary() string
// EX L128-139: (s HtmlSummary) ContentWithoutSummary() string
// EX L141-143: (s HtmlSummary) Truncated() bool
// EX L145-162: (s *HtmlSummary) resolveParagraphTagAndSetWrapper(mt media.Type) tagReStartEnd
// EX L170-172: isProbablyHTMLToken(s string) bool
// EX L175-254: ExtractSummaryFromHTML(mt media.Type, input string, numWords int, isCJK bool) (result HtmlSummary)
//    L258-280: ExtractSummaryFromHTMLWithDivider(mt media.Type, input, divider string) (result HtmlSummary)
//    L304-362: expandSummaryDivider(s string, re tagReStartEnd, divider types.LowHigh[string]) (types.LowHigh[string], types.LowHigh[string])
// ---------------------------------------------------------------------------
