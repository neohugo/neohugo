//! Port of `parser/pageparser/item.go`, `parser/pageparser/itemtype_string.go`.
//!
//! Owner: Wave B task T03 (parser-langs).

use std::borrow::Cow;

use go_value::Value;

/// Go: `pageparser.ItemType` (the `iota` order is kept; `as i64` gives Go's value).
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

const ITEM_TYPE_NAMES: [&str; 20] = [
    "tError",
    "tEOF",
    "TypeLeadSummaryDivider",
    "TypeFrontMatterYAML",
    "TypeFrontMatterTOML",
    "TypeFrontMatterJSON",
    "TypeFrontMatterORG",
    "TypeIgnore",
    "tLeftDelimScNoMarkup",
    "tRightDelimScNoMarkup",
    "tLeftDelimScWithMarkup",
    "tRightDelimScWithMarkup",
    "tScClose",
    "tScName",
    "tScNameInline",
    "tScParam",
    "tScParamVal",
    "tIndentation",
    "tText",
    "tKeywordMarker",
];

impl ItemType {
    /// Go's integer value of the constant.
    pub fn as_int(self) -> i64 {
        self as i64
    }

    // Go: parser/pageparser/itemtype_string.go:String
    /// Go: `ItemType.String()` (stringer output).
    pub fn string(self) -> &'static str {
        ITEM_TYPE_NAMES[self as usize]
    }
}

/// Go: `pageparser.lowHigh`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LowHigh {
    pub low: usize,
    pub high: usize,
}

/// Go: `pageparser.Item`: a token referencing a byte range (or several segments) of the source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub typ: ItemType,
    /// Set for `ItemType::Error` (Go's `Err.Error()`).
    pub err: Option<String>,
    /// Go's `Err.Error()` byte for byte (it can quote invalid UTF-8 from the input).
    pub err_bytes: Option<Vec<u8>>,
    pub(crate) low: usize,
    pub(crate) high: usize,
    /// The uncommon case (escaped quotes removed): several segments (Go `segments []lowHigh`).
    pub(crate) segments: Vec<(usize, usize)>,
    /// Used for validation.
    pub(crate) first_byte: u8,
    pub(crate) is_string: bool,
}

/// Go: `pageparser.Items`.
pub type Items = Vec<Item>;

impl Item {
    pub(crate) fn new(typ: ItemType, low: usize, high: usize) -> Item {
        Item {
            typ,
            err: None,
            err_bytes: None,
            low,
            high,
            segments: Vec::new(),
            first_byte: 0,
            is_string: false,
        }
    }

    /// Go's unexported `low` field (e.g. `input[item.low:]` in `ParseFrontMatterAndContent` and
    /// the content parser of hugolib).
    pub fn low(&self) -> usize {
        self.low
    }

    /// Go's unexported `high` field.
    pub fn high(&self) -> usize {
        self.high
    }

    /// Go's unexported `segments` field.
    pub fn segments(&self) -> &[(usize, usize)] {
        &self.segments
    }

    /// Go's unexported `isString` field.
    pub fn is_string(&self) -> bool {
        self.is_string
    }

    /// Go's unexported `firstByte` field.
    pub fn first_byte(&self) -> u8 {
        self.first_byte
    }

    // Go: parser/pageparser/item.go:Pos
    pub fn pos(&self) -> usize {
        if !self.segments.is_empty() {
            return self.segments[0].0;
        }
        self.low
    }

    // Go: parser/pageparser/item.go:Val
    pub fn val<'a>(&self, source: &'a [u8]) -> Cow<'a, [u8]> {
        if self.segments.is_empty() {
            return Cow::Borrowed(&source[self.low..self.high]);
        }

        if self.segments.len() == 1 {
            return Cow::Borrowed(&source[self.segments[0].0..self.segments[0].1]);
        }

        let mut b = Vec::new();
        for s in &self.segments {
            b.extend_from_slice(&source[s.0..s.1]);
        }
        Cow::Owned(b)
    }

    /// Go: `ValStr` returns a Go `string`; this returns its bytes (Go strings may hold invalid
    /// UTF-8). See [`Item::val_str`] for a lossy `String`.
    // Go: parser/pageparser/item.go:ValStr
    pub fn val_bytes(&self, source: &[u8]) -> Vec<u8> {
        self.val(source).into_owned()
    }

    /// Go: `ValStr` (invalid UTF-8 is replaced; use [`Item::val_bytes`] in parity paths).
    // Go: parser/pageparser/item.go:ValStr
    pub fn val_str(&self, source: &[u8]) -> String {
        String::from_utf8_lossy(&self.val(source)).into_owned()
    }

    /// Go: `ValTyped` — shortcode param values: bool/int/float literals typed, else string.
    // Go: parser/pageparser/item.go:ValTyped
    pub fn val_typed(&self, source: &[u8]) -> Value {
        let str = self.val_bytes(source);
        if self.is_string {
            // A quoted value that is a string even if it looks like a number etc.
            return Value::string(str);
        }

        if bool_re(&str) {
            return Value::Bool(str == b"true");
        }

        if int_re(&str) {
            return match go_strconv::atoi(&str) {
                Ok(num) => Value::int(num),
                Err(_) => Value::string(str),
            };
        }

        if float_re(&str) {
            return match go_strconv::parse_float(&str, 64) {
                Ok(num) => Value::float64(num),
                Err(_) => Value::string(str),
            };
        }

        Value::string(str)
    }

    // Go: parser/pageparser/item.go:IsText
    pub fn is_text(&self) -> bool {
        self.typ == ItemType::Text || self.is_indentation()
    }

    // Go: parser/pageparser/item.go:IsIndentation
    pub fn is_indentation(&self) -> bool {
        self.typ == ItemType::Indentation
    }

    // Go: parser/pageparser/item.go:IsNonWhitespace
    pub fn is_non_whitespace(&self, source: &[u8]) -> bool {
        !go_unicode::bytes::trim_space(&self.val(source)).is_empty()
    }

    // Go: parser/pageparser/item.go:IsShortcodeName
    pub fn is_shortcode_name(&self) -> bool {
        self.typ == ItemType::ScName
    }

    // Go: parser/pageparser/item.go:IsInlineShortcodeName
    pub fn is_inline_shortcode_name(&self) -> bool {
        self.typ == ItemType::ScNameInline
    }

    // Go: parser/pageparser/item.go:IsLeftShortcodeDelim
    pub fn is_left_shortcode_delim(&self) -> bool {
        matches!(
            self.typ,
            ItemType::LeftDelimScWithMarkup | ItemType::LeftDelimScNoMarkup
        )
    }

    // Go: parser/pageparser/item.go:IsRightShortcodeDelim
    pub fn is_right_shortcode_delim(&self) -> bool {
        matches!(
            self.typ,
            ItemType::RightDelimScWithMarkup | ItemType::RightDelimScNoMarkup
        )
    }

    // Go: parser/pageparser/item.go:IsShortcodeClose
    pub fn is_shortcode_close(&self) -> bool {
        self.typ == ItemType::ScClose
    }

    // Go: parser/pageparser/item.go:IsShortcodeParam
    pub fn is_shortcode_param(&self) -> bool {
        self.typ == ItemType::ScParam
    }

    // Go: parser/pageparser/item.go:IsShortcodeParamVal
    pub fn is_shortcode_param_val(&self) -> bool {
        self.typ == ItemType::ScParamVal
    }

    // Go: parser/pageparser/item.go:IsShortcodeMarkupDelimiter
    pub fn is_shortcode_markup_delimiter(&self) -> bool {
        matches!(
            self.typ,
            ItemType::LeftDelimScWithMarkup | ItemType::RightDelimScWithMarkup
        )
    }

    // Go: parser/pageparser/item.go:IsFrontMatter
    pub fn is_front_matter(&self) -> bool {
        self.typ >= ItemType::FrontMatterYaml && self.typ <= ItemType::FrontMatterOrg
    }

    // Go: parser/pageparser/item.go:IsDone
    pub fn is_done(&self) -> bool {
        self.is_error() || self.is_eof()
    }

    // Go: parser/pageparser/item.go:IsEOF
    pub fn is_eof(&self) -> bool {
        self.typ == ItemType::Eof
    }

    // Go: parser/pageparser/item.go:IsError
    pub fn is_error(&self) -> bool {
        self.typ == ItemType::Error
    }

    // Go: parser/pageparser/item.go:ToString
    /// Go: `ToString` (debugging; the bytes of Go's string).
    pub fn to_string_bytes(&self, source: &[u8]) -> Vec<u8> {
        let val = self.val(source);
        let mut out = Vec::new();
        if self.is_eof() {
            out.extend_from_slice(b"EOF");
        } else if self.is_error() {
            out.extend_from_slice(&val);
        } else if self.is_indentation() {
            out.extend_from_slice(self.typ.string().as_bytes());
            out.extend_from_slice(b":[");
            out.extend_from_slice(&visualize_spaces(&val));
            out.push(b']');
        } else if self.typ > ItemType::KeywordMarker {
            out.push(b'<');
            out.extend_from_slice(&val);
            out.push(b'>');
        } else if val.len() > 50 {
            // fmt.Sprintf("%v:%.20q...", i.Type, val)
            out.extend_from_slice(self.typ.string().as_bytes());
            out.push(b':');
            out.extend_from_slice(go_strconv::quote(truncate_runes(&val, 20)).as_bytes());
            out.extend_from_slice(b"...");
        } else {
            out.extend_from_slice(self.typ.string().as_bytes());
            out.extend_from_slice(b":[");
            out.extend_from_slice(&val);
            out.push(b']');
        }
        out
    }
}

/// Go: fmt's `truncateString` (precision counts runes; an invalid byte is one rune).
fn truncate_runes(s: &[u8], prec: usize) -> &[u8] {
    let mut n = prec as isize;
    let mut i = 0;
    while i < s.len() {
        n -= 1;
        if n < 0 {
            return &s[..i];
        }
        let (_, w) = go_unicode::utf8::decode_rune(&s[i..]);
        i += w;
    }
    s
}

// Go: github.com/yuin/goldmark@v1.7.12/util/util.go:VisualizeSpaces
fn visualize_spaces(bs: &[u8]) -> Vec<u8> {
    let r = |b: &[u8], old: &[u8], new: &[u8]| go_unicode::bytes::replace_all(b, old, new);
    let bs = r(bs, b" ", b"[SPACE]");
    let bs = r(&bs, b"\t", b"[TAB]");
    let bs = r(&bs, b"\n", b"[NEWLINE]\n");
    let bs = r(&bs, b"\r", b"[CR]");
    let bs = r(&bs, b"\x0b", b"[VTAB]");
    let bs = r(&bs, b"\x00", b"[NUL]");
    r(&bs, "\u{fffd}".as_bytes(), b"[U+FFFD]")
}

/// Go: `boolRe = ^(true|false)$`.
fn bool_re(s: &[u8]) -> bool {
    s == b"true" || s == b"false"
}

/// Go: `intRe = ^[-+]?\d+$` (`\d` is ASCII in RE2).
fn int_re(s: &[u8]) -> bool {
    let s = match s.first() {
        Some(b'-' | b'+') => &s[1..],
        _ => s,
    };
    !s.is_empty() && s.iter().all(u8::is_ascii_digit)
}

/// Go: `floatRe = ^[-+]?\d*\.\d+$`.
fn float_re(s: &[u8]) -> bool {
    let s = match s.first() {
        Some(b'-' | b'+') => &s[1..],
        _ => s,
    };
    let int_len = s.iter().take_while(|b| b.is_ascii_digit()).count();
    let rest = &s[int_len..];
    rest.len() >= 2 && rest[0] == b'.' && rest[1..].iter().all(u8::is_ascii_digit)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/item.go (221 lines; 18/20 funcs executed)
//   types: lowHigh, Item, Items, ItemType
// OK L49-54: (i Item) Pos() int
// OK L56-70: (i Item) Val(source []byte) []byte
// OK L72-74: (i Item) ValStr(source []byte) string
// OK L76-104: (i Item) ValTyped(source []byte) any
// OK L106-108: (i Item) IsText() bool
// OK L110-112: (i Item) IsIndentation() bool
// OK L114-116: (i Item) IsNonWhitespace(source []byte) bool
// OK L118-120: (i Item) IsShortcodeName() bool
// OK L122-124: (i Item) IsInlineShortcodeName() bool
// OK L126-128: (i Item) IsLeftShortcodeDelim() bool
// OK L130-132: (i Item) IsRightShortcodeDelim() bool
// OK L134-136: (i Item) IsShortcodeClose() bool
// OK L138-140: (i Item) IsShortcodeParam() bool
// OK L142-144: (i Item) IsShortcodeParamVal() bool
// OK L146-148: (i Item) IsShortcodeMarkupDelimiter() bool
// OK L150-152: (i Item) IsFrontMatter() bool
// OK L154-156: (i Item) IsDone() bool
// OK L158-160: (i Item) IsEOF() bool
// OK L162-164: (i Item) IsError() bool
// OK L166-182: (i Item) ToString(source []byte) string
// Source: parser/pageparser/itemtype_string.go (42 lines; 0/2 funcs executed)
// OK L7-31: _()   (compile-time check; the enum order is the Go iota order)
// OK L37-42: (i ItemType) String() string
// ---------------------------------------------------------------------------
