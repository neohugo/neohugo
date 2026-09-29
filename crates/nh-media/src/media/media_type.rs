//! Port of `media/mediaType.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

//! Go `media.Type` (a value type: compared by value, printed via `String()` = `Type`).

use std::borrow::Cow;

use go_value::{GoString, Kind, Object, Value};
use nh_common::object::args;
use nh_common::{Error, Result};
use nh_config::decode::FieldRef;
use nh_config::{decode_struct, struct_object};

use super::builtin::builtin;

/// Go: `media.DefaultDelimiter`.
pub const DEFAULT_DELIMITER: &str = ".";

/// Go: `media.SuffixInfo`: information about a Media Type's suffix.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SuffixInfo {
    /// Suffix is the suffix without the delimiter, e.g. "xml".
    pub suffix: String,
    /// FullSuffix is the suffix with the delimiter, e.g. ".xml".
    pub full_suffix: String,
}

decode_struct!(SuffixInfo, "media.SuffixInfo", |s| vec![
    FieldRef::new("Suffix", &mut s.suffix),
    FieldRef::new("FullSuffix", &mut s.full_suffix),
]);

struct_object!(SuffixInfo, "media.SuffixInfo", |s| [
    ("Suffix", Value::string(s.suffix.as_str())),
    ("FullSuffix", Value::string(s.full_suffix.as_str())),
]);

/// Go: `media.Type`: a MIME type (top-level type name / subtype name + suffix), e.g.
/// application/svg+xml.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MediaType {
    /// The full MIME type string, e.g. `application/rss+xml` (Go field `Type`, also `String()`).
    pub typ: String,
    /// The top-level type name, e.g. "application".
    pub main_type: String,
    /// The subtype name, e.g. "rss".
    pub sub_type: String,
    /// The delimiter before the suffix, e.g. ".".
    pub delimiter: String,
    /// FirstSuffix holds the first suffix defined for this MediaType.
    pub first_suffix: SuffixInfo,
    /// This is the optional suffix after the "+" in the MIME type, e.g. "xml" in
    /// "application/rss+xml".
    pub(crate) mime_suffix: String,
    /// Comma-separated suffixes, e.g. "xml,rss" (Go `SuffixesCSV`).
    pub suffixes_csv: String,
}

decode_struct!(MediaType, "media.Type", |s| vec![
    FieldRef::new("Type", &mut s.typ),
    FieldRef::new("MainType", &mut s.main_type),
    FieldRef::new("SubType", &mut s.sub_type),
    FieldRef::new("Delimiter", &mut s.delimiter),
    FieldRef::new("FirstSuffix", &mut s.first_suffix),
    FieldRef::new("mimeSuffix", &mut s.mime_suffix).unexported(),
    FieldRef::new("SuffixesCSV", &mut s.suffixes_csv),
]);

impl MediaType {
    /// Go: `media.FromString("application/rss+xml")`: a Type from a string on the form
    /// MainType/SubType and an optional suffix, e.g. "text/html" or "text/html+html".
    // Go: media/mediaType.go:FromString
    pub fn from_string(t: &str) -> Result<MediaType> {
        let t = go_unicode::strings::to_lower_str(t).into_owned();
        let parts: Vec<&str> = t.split('/').collect();
        if parts.len() != 2 {
            return Err(Error::new(format!(
                "cannot parse {} as a media type",
                go_strconv::quote(&t)
            )));
        }
        let main_type = parts[0];
        let sub_parts: Vec<&str> = parts[1].split('+').collect();

        let sub_type = sub_parts[0].split(';').next().unwrap_or("");

        let suffix = if sub_parts.len() > 1 {
            sub_parts[1]
        } else {
            ""
        };

        let typ = if !suffix.is_empty() {
            format!("{main_type}/{sub_type}+{suffix}")
        } else {
            format!("{main_type}/{sub_type}")
        };

        Ok(MediaType {
            typ,
            main_type: main_type.to_string(),
            sub_type: sub_type.to_string(),
            mime_suffix: suffix.to_string(),
            ..Default::default()
        })
    }

    /// Go: `media.FromStringAndExt(t, ext...)`: a Type from a MIME string and the given
    /// extensions.
    // Go: media/mediaType.go:FromStringAndExt
    pub fn from_string_and_ext(t: &str, ext: &[&str]) -> Result<MediaType> {
        let mut tp = MediaType::from_string(t)?;
        let ext: Vec<&str> = ext
            .iter()
            .map(|e| e.strip_prefix('.').unwrap_or(e))
            .collect();
        tp.suffixes_csv = ext.join(",");
        tp.delimiter = DEFAULT_DELIMITER.to_string();
        tp.init();
        Ok(tp)
    }

    /// For internal use.
    // Go: media/mediaType.go:String
    pub fn string(&self) -> &str {
        &self.typ
    }

    /// Suffixes returns all valid file suffixes for this type (Go's nil for none).
    // Go: media/mediaType.go:Suffixes
    pub fn suffixes(&self) -> Vec<String> {
        if self.suffixes_csv.is_empty() {
            Vec::new()
        } else {
            self.suffixes_csv.split(',').map(str::to_string).collect()
        }
    }

    /// Go: `IsText()` — main type `text`, or sub type in {javascript, json, rss, xml, svg, toml, yml, yaml}.
    // Go: media/mediaType.go:IsText
    pub fn is_text(&self) -> bool {
        if self.main_type == "text" {
            return true;
        }
        matches!(
            self.sub_type.as_str(),
            "javascript" | "json" | "rss" | "xml" | "svg" | "toml" | "yml" | "yaml"
        )
    }

    // Go: media/mediaType.go:IsHTML
    pub fn is_html(&self) -> bool {
        self.sub_type == builtin().html_type.sub_type
    }

    // Go: media/mediaType.go:IsMarkdown
    pub fn is_markdown(&self) -> bool {
        self.sub_type == builtin().markdown_type.sub_type
    }

    // Go: media/mediaType.go:HasSuffix
    pub fn has_suffix(&self, suffix: &str) -> bool {
        format!(",{},", self.suffixes_csv).contains(&format!(",{suffix},"))
    }

    /// IsZero reports whether this Type represents a zero value.
    // Go: media/mediaType.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.sub_type.is_empty()
    }

    /// The MIME suffix after the "+" (Go's unexported `mimeSuffix`).
    pub fn mime_suffix(&self) -> &str {
        &self.mime_suffix
    }

    /// Go: `InitMediaType` / `init()` — sets the first suffix.
    // Go: media/mediaType.go:init
    pub fn init(&mut self) {
        self.first_suffix.full_suffix = String::new();
        self.first_suffix.suffix = String::new();
        let suffixes = self.suffixes();
        if !suffixes.is_empty() {
            self.first_suffix.suffix = suffixes[0].clone();
            self.first_suffix.full_suffix =
                format!("{}{}", self.delimiter, self.first_suffix.suffix);
        }
    }

    /// Go: `MarshalJSON`: the fields of the type plus `type`, `string` and `suffixes`.
    // Go: media/mediaType.go:MarshalJSON
    pub fn marshal_json_bytes(&self) -> Result<Vec<u8>> {
        let s = |v: &str| -> Result<Vec<u8>> {
            go_json::marshal(&Value::string(v)).map_err(|e| Error::new(e.to_string()))
        };
        let suffixes: Vec<&str> = self.suffixes_csv.split(',').collect();
        let suffixes = go_json::marshal(&Value::string_list(suffixes))
            .map_err(|e| Error::new(e.to_string()))?;
        let mut b = Vec::new();
        b.extend_from_slice(b"{\"mainType\":");
        b.extend(s(&self.main_type)?);
        b.extend_from_slice(b",\"subType\":");
        b.extend(s(&self.sub_type)?);
        b.extend_from_slice(b",\"delimiter\":");
        b.extend(s(&self.delimiter)?);
        b.extend_from_slice(b",\"type\":");
        b.extend(s(&self.typ)?);
        b.extend_from_slice(b",\"string\":");
        b.extend(s(self.string())?);
        b.extend_from_slice(b",\"suffixes\":");
        b.extend(suffixes);
        b.push(b'}');
        Ok(b)
    }

    /// Template value.
    pub fn to_value(&self) -> Value {
        Value::object(self.clone())
    }
}

/// Go: `InitMediaType(m)`.
// Go: media/mediaType.go:InitMediaType
pub fn init_media_type(m: &mut MediaType) {
    m.init();
}

// Go: media/mediaType.go:newMediaType
pub fn new_media_type(main: &str, sub: &str, suffixes: &[&str]) -> MediaType {
    let mut t = MediaType {
        main_type: main.to_string(),
        sub_type: sub.to_string(),
        suffixes_csv: suffixes.join(","),
        delimiter: DEFAULT_DELIMITER.to_string(),
        ..Default::default()
    };
    t.init();
    t
}

// Go: media/mediaType.go:newMediaTypeWithMimeSuffix
pub fn new_media_type_with_mime_suffix(
    main: &str,
    sub: &str,
    mime_suffix: &str,
    suffixes: &[&str],
) -> MediaType {
    let mut mt = new_media_type(main, sub, suffixes);
    mt.mime_suffix = mime_suffix.to_string();
    mt.init();
    mt
}

/// Go: `media.Types` — sorted by `Type` string (`sort.Sort`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Types(pub Vec<MediaType>);

impl Types {
    /// Go: `sort.Sort(types)` (Less: by `Type`).
    // Go: media/mediaType.go:Less
    pub fn sort(&mut self) {
        go_sort::sort_by(&mut self.0, |a, b| a.typ < b.typ);
    }

    /// GetBestMatch returns the best match for the given media type string: an exact match,
    /// else by sub type, else by suffix.
    // Go: media/mediaType.go:GetBestMatch
    pub fn get_best_match(&self, s: &str) -> Option<MediaType> {
        // First try an exact match.
        if let Some(mt) = self.get_by_type(s) {
            return Some(mt);
        }

        // Try main type.
        if let Some(mt) = self.get_by_sub_type(s) {
            return Some(mt);
        }

        // Try extension.
        if let Some((mt, _)) = self.get_first_by_suffix(s) {
            return Some(mt);
        }

        None
    }

    /// GetByType returns a media type for tp.
    pub fn get_by_type(&self, tp: &str) -> Option<MediaType> {
        found(self.get_by_type_found(tp))
    }

    /// [`Types::get_by_type`] with Go's results: when the main/sub type lookup is ambiguous
    /// the first match is returned with `false`.
    // Go: media/mediaType.go:GetByType
    pub fn get_by_type_found(&self, tp: &str) -> (MediaType, bool) {
        for tt in &self.0 {
            if go_unicode::strings::equal_fold_str(&tt.typ, tp) {
                return (tt.clone(), true);
            }
        }

        if !tp.contains('+') {
            // Try with the main and sub type
            let parts: Vec<&str> = tp.split('/').collect();
            if parts.len() == 2 {
                return self.get_by_main_sub_type_found(parts[0], parts[1]);
            }
        }

        (MediaType::default(), false)
    }

    // Go: media/mediaType.go:normalizeSuffix
    fn normalize_suffix(&self, s: &str) -> String {
        go_unicode::strings::to_lower_str(s.strip_prefix('.').unwrap_or(s)).into_owned()
    }

    /// BySuffix will return all media types matching a suffix.
    // Go: media/mediaType.go:BySuffix
    pub fn by_suffix(&self, suffix: &str) -> Vec<MediaType> {
        let suffix = self.normalize_suffix(suffix);
        self.0
            .iter()
            .filter(|tt| tt.has_suffix(&suffix))
            .cloned()
            .collect()
    }

    /// GetFirstBySuffix will return the first type matching the given suffix. The suffix `xml`
    /// resolves first to `application/rss+xml` (types are sorted by string).
    // Go: media/mediaType.go:GetFirstBySuffix
    pub fn get_first_by_suffix(&self, suffix: &str) -> Option<(MediaType, SuffixInfo)> {
        let suffix = self.normalize_suffix(suffix);
        for tt in &self.0 {
            if tt.has_suffix(&suffix) {
                return Some((
                    tt.clone(),
                    SuffixInfo {
                        full_suffix: format!("{}{}", tt.delimiter, suffix),
                        suffix,
                    },
                ));
            }
        }
        None
    }

    /// GetBySuffix gets a media type given as suffix, e.g. "html". It will return `None` if no
    /// format could be found, or if the suffix given is ambiguous. The lookup is case
    /// insensitive.
    pub fn get_by_suffix(&self, suffix: &str) -> Option<(MediaType, SuffixInfo)> {
        let (tp, si, ok) = self.get_by_suffix_found(suffix);
        ok.then_some((tp, si))
    }

    /// [`Types::get_by_suffix`] with Go's results: when the suffix is ambiguous the first
    /// match is returned with `false`.
    // Go: media/mediaType.go:GetBySuffix
    pub fn get_by_suffix_found(&self, suffix: &str) -> (MediaType, SuffixInfo, bool) {
        let suffix = self.normalize_suffix(suffix);
        let mut tp = MediaType::default();
        let mut si = SuffixInfo::default();
        let mut found = false;
        for tt in &self.0 {
            if tt.has_suffix(&suffix) {
                if found {
                    // ambiguous
                    return (tp, si, false);
                }
                tp = tt.clone();
                si = SuffixInfo {
                    full_suffix: format!("{}{}", tt.delimiter, suffix),
                    suffix: suffix.clone(),
                };
                found = true;
            }
        }
        (tp, si, found)
    }

    // Go: media/mediaType.go:IsTextSuffix
    pub fn is_text_suffix(&self, suffix: &str) -> bool {
        let suffix = self.normalize_suffix(suffix);
        for tt in &self.0 {
            if tt.has_suffix(&suffix) {
                return tt.is_text();
            }
        }
        false
    }

    /// GetByMainSubType gets a media type given a main and a sub type e.g. "text" and "plain".
    /// It will return `None` if no format could be found, or if the combination given is
    /// ambiguous. The lookup is case insensitive.
    pub fn get_by_main_sub_type(&self, main: &str, sub: &str) -> Option<MediaType> {
        found(self.get_by_main_sub_type_found(main, sub))
    }

    /// [`Types::get_by_main_sub_type`] with Go's results (ambiguous: the first match, `false`).
    // Go: media/mediaType.go:GetByMainSubType
    pub fn get_by_main_sub_type_found(&self, main: &str, sub: &str) -> (MediaType, bool) {
        let mut tp = MediaType::default();
        let mut found = false;
        for tt in &self.0 {
            if go_unicode::strings::equal_fold_str(main, &tt.main_type)
                && go_unicode::strings::equal_fold_str(sub, &tt.sub_type)
            {
                if found {
                    // ambiguous
                    return (tp, false);
                }
                tp = tt.clone();
                found = true;
            }
        }
        (tp, found)
    }

    /// GetBySubType gets a media type given a sub type e.g. "plain".
    pub fn get_by_sub_type(&self, sub: &str) -> Option<MediaType> {
        found(self.get_by_sub_type_found(sub))
    }

    /// [`Types::get_by_sub_type`] with Go's results (ambiguous: the first match, `false`).
    // Go: media/mediaType.go:GetBySubType
    pub fn get_by_sub_type_found(&self, sub: &str) -> (MediaType, bool) {
        let mut tp = MediaType::default();
        let mut found = false;
        for tt in &self.0 {
            if go_unicode::strings::equal_fold_str(sub, &tt.sub_type) {
                if found {
                    // ambiguous
                    return (tp, false);
                }
                tp = tt.clone();
                found = true;
            }
        }
        (tp, found)
    }
}

fn found(r: (MediaType, bool)) -> Option<MediaType> {
    r.1.then_some(r.0)
}

/// Go: `media.FromContent(types, extensionHints, content)`: the Type from
/// `http.DetectContentType`; for text/plain or application/xml the extension hints decide a
/// more specific text type. A zero Type when nothing fits.
// Go: media/mediaType.go:FromContent
pub fn from_content(types: &Types, extension_hints: &[String], content: &[u8]) -> MediaType {
    let t = super::sniff::detect_content_type(content)
        .split(';')
        .next()
        .unwrap_or("");
    if t == "application/octet-stream" {
        return MediaType::default();
    }

    let (mut m, mut found) = types.get_by_type_found(t);
    if !found && t == "text/xml" {
        // This is how it's configured in Hugo by default.
        (m, found) = types.get_by_type_found("application/xml");
    }

    if !found {
        return MediaType::default();
    }

    // (Go reuses `found`: with no extension hints it is still true here and `mm` is the zero
    // Type, so the result is the zero Type unless m is zero.)
    let mut mm = MediaType::default();
    for extension in extension_hints {
        let extension = extension.strip_prefix('.').unwrap_or(extension);
        match types.get_first_by_suffix(extension) {
            Some((t, _)) => {
                mm = t;
                found = true;
                break;
            }
            None => {
                mm = MediaType::default();
                found = false;
            }
        }
    }

    if found {
        if m == mm {
            return m;
        }

        if m.is_text() && mm.is_text() {
            // http.DetectContentType isn't brilliant when it comes to common text formats, so
            // we need to do better. If it's detected to be a text format and the extension
            // reports it to be a text format, then we use that.
            return mm;
        }

        // E.g. an image with a *.js extension.
        return MediaType::default();
    }

    m
}

nh_common::go_methods!(MediaType {
    // Go: media/mediaType.go:String
    "String" => |m, _c, a| {
        args::exactly(a, 0, "String")?;
        Ok(Value::string(m.typ.as_str()))
    },
    // Go: media/mediaType.go:Suffixes
    "Suffixes" => |m, _c, a| {
        args::exactly(a, 0, "Suffixes")?;
        Ok(if m.suffixes_csv.is_empty() {
            Value::TypedNil(std::sync::Arc::from("[]string"))
        } else {
            nh_common::object::string_slice(&m.suffixes())
        })
    },
    // Go: media/mediaType.go:IsZero
    "IsZero" => |m, _c, a| {
        args::exactly(a, 0, "IsZero")?;
        Ok(Value::Bool(m.is_zero()))
    },
    // Go: media/mediaType.go:IsText
    "IsText" => |m, _c, a| {
        args::exactly(a, 0, "IsText")?;
        Ok(Value::Bool(m.is_text()))
    },
    // Go: media/mediaType.go:IsHTML
    "IsHTML" => |m, _c, a| {
        args::exactly(a, 0, "IsHTML")?;
        Ok(Value::Bool(m.is_html()))
    },
    // Go: media/mediaType.go:IsMarkdown
    "IsMarkdown" => |m, _c, a| {
        args::exactly(a, 0, "IsMarkdown")?;
        Ok(Value::Bool(m.is_markdown()))
    },
    // Go: media/mediaType.go:HasSuffix
    "HasSuffix" => |m, _c, a| {
        args::exactly(a, 1, "HasSuffix")?;
        let suffix = args::string(a, 0)?;
        let mut hay = b",".to_vec();
        hay.extend_from_slice(m.suffixes_csv.as_bytes());
        hay.push(b',');
        let mut needle = b",".to_vec();
        needle.extend_from_slice(suffix.as_bytes());
        needle.push(b',');
        Ok(Value::Bool(
            hay.windows(needle.len()).any(|w| w == needle.as_slice()),
        ))
    },
    // Go: media/mediaType.go:MarshalJSON
    "MarshalJSON" => |m, _c, a| {
        args::exactly(a, 0, "MarshalJSON")?;
        let b = m.marshal_json_bytes()?;
        Ok(Value::list(
            go_value::SliceType::Uint8,
            b.into_iter().map(|c| Value::Uint(u64::from(c), go_value::UintKind::Uint8)).collect(),
        ))
    },
});

impl Object for MediaType {
    nh_common::object_basics!("media.Type");

    fn kind(&self) -> Kind {
        Kind::Struct
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Type" => Some(Value::string(self.typ.as_str())),
            "MainType" => Some(Value::string(self.main_type.as_str())),
            "SubType" => Some(Value::string(self.sub_type.as_str())),
            "Delimiter" => Some(Value::string(self.delimiter.as_str())),
            "FirstSuffix" => Some(Value::object(self.first_suffix.clone())),
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

    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Type"), Value::string(self.typ.as_str())),
            (
                Cow::Borrowed("MainType"),
                Value::string(self.main_type.as_str()),
            ),
            (
                Cow::Borrowed("SubType"),
                Value::string(self.sub_type.as_str()),
            ),
            (
                Cow::Borrowed("Delimiter"),
                Value::string(self.delimiter.as_str()),
            ),
            (
                Cow::Borrowed("FirstSuffix"),
                Value::object(self.first_suffix.clone()),
            ),
            (
                Cow::Borrowed("SuffixesCSV"),
                Value::string(self.suffixes_csv.as_str()),
            ),
        ])
    }

    /// Go: `media.Type.MarshalJSON` (mainType, subType, delimiter, type, string, suffixes).
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(self.marshal_json_bytes().map_err(Into::into))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: media/mediaType.go (401 lines; 23/27 funcs executed)
//   types: Type, SuffixInfo, Types
// OK L72-118: FromContent(types Types, extensionHints []string, content []byte) Type
// OK L121-133: FromStringAndExt(t string, ext ...string) (Type, error)
// OK L137-162: FromString(t string) (Type, error)
// OK L165-167: (m Type) String() string
// OK L170-176: (m Type) Suffixes() []string
// OK L182-191: (m Type) IsText() bool
// OK L194-196: (m Type) IsHTML() bool
// OK L199-201: (m Type) IsMarkdown() bool
// OK L203-205: InitMediaType(m *Type)
// OK L207-214: (m *Type) init()
// OK L216-220: newMediaType(main, sub string, suffixes []string) Type
// OK L222-227: newMediaTypeWithMimeSuffix(main, sub, mimeSuffix string, suffixes []string) Type
// OK L233-233: (t Types) Len() int
// OK L234-234: (t Types) Swap(i, j int)
// OK L235-235: (t Types) Less(i, j int) bool
// OK L238-255: (t Types) GetBestMatch(s string) (Type, bool)
// OK L258-274: (t Types) GetByType(tp string) (Type, bool)
// OK L276-278: (t Types) normalizeSuffix(s string) string
// OK L281-290: (t Types) BySuffix(suffix string) []Type
// OK L293-304: (t Types) GetFirstBySuffix(suffix string) (Type, SuffixInfo, bool)
// OK L310-328: (t Types) GetBySuffix(suffix string) (tp Type, si SuffixInfo, found bool)
// OK L330-338: (t Types) IsTextSuffix(suffix string) bool
// OK L340-342: (m Type) HasSuffix(suffix string) bool
// OK L348-362: (t Types) GetByMainSubType(mainType, subType string) (tp Type, found bool)
// OK L365-378: (t Types) GetBySubType(subType string) (tp Type, found bool)
// OK L382-384: (m Type) IsZero() bool
// OK L388-401: (m Type) MarshalJSON() ([]byte, error)
// ---------------------------------------------------------------------------
