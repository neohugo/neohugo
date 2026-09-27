//! Port of `parser/pageparser/item.go`, `parser/pageparser/itemtype_string.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


use go_value::Value;

/// Go: `pageparser.ItemType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemType {
    Error,
    Eof,
    // page items
    /// `<!--more-->`, `# more`
    LeadSummaryDivider,
    FrontMatterYaml,
    FrontMatterToml,
    FrontMatterJson,
    FrontMatterOrg,
    /// The BOM and possibly others.
    Ignore,
    // shortcode items
    LeftDelimScNoMarkup,
    RightDelimScNoMarkup,
    LeftDelimScWithMarkup,
    RightDelimScWithMarkup,
    ScClose,
    ScName,
    ScNameInline,
    ScParam,
    ScParamVal,
    Indentation,
    /// plain text
    Text,
    /// preserved for later - keywords come after this
    KeywordMarker,
}

/// Go: `pageparser.Item`: a token referencing a byte range (or several segments) of the source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub typ: ItemType,
    /// Set for `ItemType::Error`.
    pub err: Option<String>,
    pub(crate) low: usize,
    pub(crate) high: usize,
    pub(crate) segments: Vec<(usize, usize)>,
    pub(crate) first_byte: u8,
    pub(crate) is_string: bool,
}

/// Go: `pageparser.Items`.
pub type Items = Vec<Item>;

impl Item {
    // Go: parser/pageparser/item.go:Pos
    pub fn pos(&self) -> usize {
        todo!()
    }

    // Go: parser/pageparser/item.go:Val
    pub fn val<'a>(&self, source: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
        todo!()
    }

    // Go: parser/pageparser/item.go:ValStr
    pub fn val_str(&self, source: &[u8]) -> String {
        todo!()
    }

    /// Go: `ValTyped` — shortcode param values: bool/int/float literals typed, else string.
    // Go: parser/pageparser/item.go:ValTyped
    pub fn val_typed(&self, source: &[u8]) -> Value {
        todo!()
    }

    pub fn is_text(&self) -> bool { self.typ == ItemType::Text }
    pub fn is_indentation(&self) -> bool { self.typ == ItemType::Indentation }
    pub fn is_non_whitespace(&self, source: &[u8]) -> bool { todo!() }
    pub fn is_shortcode_name(&self) -> bool { self.typ == ItemType::ScName }
    pub fn is_inline_shortcode_name(&self) -> bool { self.typ == ItemType::ScNameInline }
    pub fn is_left_shortcode_delim(&self) -> bool {
        matches!(self.typ, ItemType::LeftDelimScWithMarkup | ItemType::LeftDelimScNoMarkup)
    }
    pub fn is_right_shortcode_delim(&self) -> bool {
        matches!(self.typ, ItemType::RightDelimScWithMarkup | ItemType::RightDelimScNoMarkup)
    }
    pub fn is_shortcode_close(&self) -> bool { self.typ == ItemType::ScClose }
    pub fn is_shortcode_param(&self) -> bool { self.typ == ItemType::ScParam }
    pub fn is_shortcode_param_val(&self) -> bool { self.typ == ItemType::ScParamVal }
    pub fn is_shortcode_markup_delimiter(&self) -> bool {
        matches!(self.typ, ItemType::LeftDelimScWithMarkup | ItemType::RightDelimScWithMarkup)
    }
    pub fn is_front_matter(&self) -> bool {
        matches!(self.typ, ItemType::FrontMatterYaml | ItemType::FrontMatterToml | ItemType::FrontMatterJson | ItemType::FrontMatterOrg)
    }
    pub fn is_done(&self) -> bool { matches!(self.typ, ItemType::Error | ItemType::Eof) }
    pub fn is_eof(&self) -> bool { self.typ == ItemType::Eof }
    pub fn is_error(&self) -> bool { self.typ == ItemType::Error }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/item.go (221 lines; 18/20 funcs executed)
//   types: lowHigh, Item, Items, ItemType
// EX L49-54: (i Item) Pos() int
// EX L56-70: (i Item) Val(source []byte) []byte
// EX L72-74: (i Item) ValStr(source []byte) string
// EX L76-104: (i Item) ValTyped(source []byte) any
// EX L106-108: (i Item) IsText() bool
// EX L110-112: (i Item) IsIndentation() bool
//    L114-116: (i Item) IsNonWhitespace(source []byte) bool
// EX L118-120: (i Item) IsShortcodeName() bool
// EX L122-124: (i Item) IsInlineShortcodeName() bool
// EX L126-128: (i Item) IsLeftShortcodeDelim() bool
// EX L130-132: (i Item) IsRightShortcodeDelim() bool
// EX L134-136: (i Item) IsShortcodeClose() bool
// EX L138-140: (i Item) IsShortcodeParam() bool
// EX L142-144: (i Item) IsShortcodeParamVal() bool
// EX L146-148: (i Item) IsShortcodeMarkupDelimiter() bool
// EX L150-152: (i Item) IsFrontMatter() bool
// EX L154-156: (i Item) IsDone() bool
// EX L158-160: (i Item) IsEOF() bool
// EX L162-164: (i Item) IsError() bool
//    L166-182: (i Item) ToString(source []byte) string
// Source: parser/pageparser/itemtype_string.go (42 lines; 0/2 funcs executed)
//    L7-31: _()
//    L37-42: (i ItemType) String() string
// ---------------------------------------------------------------------------
