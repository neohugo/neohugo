//! Port of `media/mediaType.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


//! Go `media.Type` (a value type: compared by value, printed via `String()` = `Type`).

use std::any::Any;
use std::borrow::Cow;

use go_value::{GoString, HostCtx, Object, Value};
use nh_common::Result;

/// Go: `media.DefaultDelimiter`.
pub const DEFAULT_DELIMITER: &str = ".";

/// Go: `media.SuffixInfo`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SuffixInfo {
    pub suffix: String,
    pub full_suffix: String,
}

/// Go: `media.Type`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MediaType {
    /// e.g. `application/rss+xml` (Go field `Type`, also `String()`).
    pub typ: String,
    pub main_type: String,
    pub sub_type: String,
    pub delimiter: String,
    pub first_suffix: SuffixInfo,
    pub(crate) mime_suffix: String,
    /// Comma-separated suffixes, e.g. "xml,rss" (Go `SuffixesCSV`).
    pub suffixes_csv: String,
}

impl MediaType {
    /// Go: `media.FromString("application/rss+xml")` — splits `main/sub+suffix`, drops params.
    // Go: media/mediaType.go:FromString
    pub fn from_string(t: &str) -> Result<MediaType> {
        todo!()
    }

    /// Go: `media.FromStringAndExt(t, ext...)`.
    // Go: media/mediaType.go:FromStringAndExt
    pub fn from_string_and_ext(t: &str, ext: &[&str]) -> Result<MediaType> {
        todo!()
    }

    // Go: media/mediaType.go:String
    pub fn string(&self) -> &str {
        &self.typ
    }

    // Go: media/mediaType.go:Suffixes
    pub fn suffixes(&self) -> Vec<String> {
        if self.suffixes_csv.is_empty() { Vec::new() } else { self.suffixes_csv.split(',').map(str::to_string).collect() }
    }

    /// Go: `IsText()` — main type `text`, or sub type in {javascript, json, rss, xml, svg, toml, yml, yaml}.
    // Go: media/mediaType.go:IsText
    pub fn is_text(&self) -> bool {
        todo!()
    }

    // Go: media/mediaType.go:IsHTML
    pub fn is_html(&self) -> bool {
        todo!()
    }

    // Go: media/mediaType.go:IsMarkdown
    pub fn is_markdown(&self) -> bool {
        todo!()
    }

    // Go: media/mediaType.go:HasSuffix
    pub fn has_suffix(&self, suffix: &str) -> bool {
        todo!()
    }

    // Go: media/mediaType.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.sub_type.is_empty()
    }

    /// Go: `InitMediaType` / `init()` — computes `typ`, first suffix.
    // Go: media/mediaType.go:init
    pub fn init(&mut self) {
        todo!()
    }
}

/// Go: `media.Types` — sorted by `Type` string (`sort.Sort`, unique keys).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Types(pub Vec<MediaType>);

impl Types {
    // Go: media/mediaType.go:GetByType
    pub fn get_by_type(&self, tp: &str) -> Option<MediaType> { todo!() }
    // Go: media/mediaType.go:GetBestMatch
    pub fn get_best_match(&self, s: &str) -> Option<MediaType> { todo!() }
    // Go: media/mediaType.go:BySuffix
    pub fn by_suffix(&self, suffix: &str) -> Vec<MediaType> { todo!() }
    /// The suffix `xml` resolves first to `application/rss+xml` (types are sorted by string).
    // Go: media/mediaType.go:GetFirstBySuffix
    pub fn get_first_by_suffix(&self, suffix: &str) -> Option<(MediaType, SuffixInfo)> { todo!() }
    // Go: media/mediaType.go:IsTextSuffix
    pub fn is_text_suffix(&self, suffix: &str) -> bool { todo!() }
    // Go: media/mediaType.go:GetByMainSubType
    pub fn get_by_main_sub_type(&self, main: &str, sub: &str) -> Option<MediaType> { todo!() }
    // Go: media/mediaType.go:GetBySubType
    pub fn get_by_sub_type(&self, sub: &str) -> Option<MediaType> { todo!() }
}

/// Go: `media.FromContent(types, extensionHints, content)` (GetRemote/`http.DetectContentType` fallback).
// Go: media/mediaType.go:FromContent
pub fn from_content(types: &Types, extension_hints: &[String], content: &[u8]) -> MediaType {
    todo!()
}

/// Template API of `media.Type`: fields `Type`, `MainType`, `SubType`, `Delimiter`,
/// `FirstSuffix`(struct), `SuffixesCSV`; methods `String`, `Suffixes`, `IsZero`, `IsText`, `IsHTML`...
/// Printed via `String()` (e.g. `printf "%q" .MediaType` -> `"application/rss+xml"`).
impl MediaType {
    pub fn to_value(&self) -> Value {
        Value::object(self.clone())
    }
}

nh_common::go_methods!(MediaType {
    "String" => |m, _c, _a| Ok(Value::string(m.typ.as_str())),
    "Suffixes" => |m, _c, _a| Ok(nh_common::object::string_slice(&m.suffixes())),
    "IsZero" => |m, _c, _a| Ok(Value::Bool(m.is_zero())),
    "IsText" => |m, _c, _a| Ok(Value::Bool(m.is_text())),
    "IsHTML" => |m, _c, _a| Ok(Value::Bool(m.is_html())),
    "IsMarkdown" => |m, _c, _a| Ok(Value::Bool(m.is_markdown())),
});

impl Object for MediaType {
    nh_common::object_basics!("media.Type");

    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Type" => Some(Value::string(self.typ.as_str())),
            "MainType" => Some(Value::string(self.main_type.as_str())),
            "SubType" => Some(Value::string(self.sub_type.as_str())),
            "Delimiter" => Some(Value::string(self.delimiter.as_str())),
            "SuffixesCSV" => Some(Value::string(self.suffixes_csv.as_str())),
            _ => None,
        }
    }

    fn is_zero(&self) -> Option<bool> {
        Some(self.is_zero())
    }

    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.typ.as_str()))
    }

    /// Go: `media.Type.MarshalJSON` (type, string, mainType, subType, delimiter, firstSuffix, mimeSuffix, suffixes).
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: media/mediaType.go (401 lines; 23/27 funcs executed)
//   types: Type, SuffixInfo, Types
// EX L72-118: FromContent(types Types, extensionHints []string, content []byte) Type
// EX L121-133: FromStringAndExt(t string, ext ...string) (Type, error)
// EX L137-162: FromString(t string) (Type, error)
// EX L165-167: (m Type) String() string
// EX L170-176: (m Type) Suffixes() []string
// EX L182-191: (m Type) IsText() bool
// EX L194-196: (m Type) IsHTML() bool
// EX L199-201: (m Type) IsMarkdown() bool
// EX L203-205: InitMediaType(m *Type)
// EX L207-214: (m *Type) init()
//    L216-220: newMediaType(main, sub string, suffixes []string) Type
//    L222-227: newMediaTypeWithMimeSuffix(main, sub, mimeSuffix string, suffixes []string) Type
// EX L233-233: (t Types) Len() int
// EX L234-234: (t Types) Swap(i, j int)
// EX L235-235: (t Types) Less(i, j int) bool
// EX L238-255: (t Types) GetBestMatch(s string) (Type, bool)
// EX L258-274: (t Types) GetByType(tp string) (Type, bool)
// EX L276-278: (t Types) normalizeSuffix(s string) string
// EX L281-290: (t Types) BySuffix(suffix string) []Type
// EX L293-304: (t Types) GetFirstBySuffix(suffix string) (Type, SuffixInfo, bool)
//    L310-328: (t Types) GetBySuffix(suffix string) (tp Type, si SuffixInfo, found bool)
// EX L330-338: (t Types) IsTextSuffix(suffix string) bool
// EX L340-342: (m Type) HasSuffix(suffix string) bool
// EX L348-362: (t Types) GetByMainSubType(mainType, subType string) (tp Type, found bool)
// EX L365-378: (t Types) GetBySubType(subType string) (tp Type, found bool)
// EX L382-384: (m Type) IsZero() bool
//    L388-401: (m Type) MarshalJSON() ([]byte, error)
// ---------------------------------------------------------------------------
