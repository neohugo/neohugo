//! Port of `parser/metadecoders/format.go`.
//!
//! Owner: Wave B task T03 (parser-langs).

use super::decoder::Decoder;

/// Go: `metadecoders.Format` (a string type; `Unknown` is Go's `""`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    /// Unknown / empty.
    Unknown,
    Org,
    Json,
    Toml,
    Yaml,
    Csv,
    Xml,
}

impl Format {
    /// Go's string value of the format.
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Unknown => "",
            Format::Org => "org",
            Format::Json => "json",
            Format::Toml => "toml",
            Format::Yaml => "yaml",
            Format::Csv => "csv",
            Format::Xml => "xml",
        }
    }

    /// Go's `f == ""`.
    pub fn is_empty(self) -> bool {
        self == Format::Unknown
    }
}

/// FormatFromStrings returns the first non-empty Format from the given strings.
// Go: parser/metadecoders/format.go:FormatFromStrings
pub fn format_from_strings(ss: &[&str]) -> Format {
    for s in ss {
        let f = format_from_string(s);
        if f != Format::Unknown {
            return f;
        }
    }
    Format::Unknown
}

/// FormatFromString turns formatStr, typically a file extension without any ".",
/// into a Format. It returns an empty string for unknown formats.
// Go: parser/metadecoders/format.go:FormatFromString
pub fn format_from_string(format_str: &str) -> Format {
    let lower =
        String::from_utf8(go_unicode::strings::to_lower(format_str.as_bytes()).into_owned())
            .expect("strings.ToLower keeps valid UTF-8 valid");
    let mut format_str: &str = &lower;
    if format_str.contains('.') {
        // Assume a filename
        let ext = go_path::filepath::ext(format_str);
        format_str = ext.strip_prefix('.').unwrap_or(ext);
    }
    match format_str.as_bytes() {
        b"yaml" | b"yml" => Format::Yaml,
        b"json" => Format::Json,
        b"toml" => Format::Toml,
        b"org" => Format::Org,
        b"csv" => Format::Csv,
        b"xml" => Format::Xml,
        _ => Format::Unknown,
    }
}

impl Decoder {
    /// FormatFromContentString tries to detect the format (JSON, YAML, TOML or XML)
    /// in the given string.
    /// It return an empty string if no format could be detected.
    // Go: parser/metadecoders/format.go:FormatFromContentString
    pub fn format_from_content_string(&self, data: &[u8]) -> Format {
        let csv_idx = go_unicode::strings::index_rune(data, self.delimiter as i32);
        let json_idx = go_unicode::strings::index(data, b"{");
        let yaml_idx = go_unicode::strings::index(data, b":");
        let xml_idx = go_unicode::strings::index(data, b"<");
        let toml_idx = go_unicode::strings::index(data, b"=");

        if is_lower_index_than(csv_idx, &[json_idx, yaml_idx, xml_idx, toml_idx]) {
            return Format::Csv;
        }

        if is_lower_index_than(json_idx, &[yaml_idx, xml_idx, toml_idx]) {
            return Format::Json;
        }

        if is_lower_index_than(yaml_idx, &[xml_idx, toml_idx]) {
            return Format::Yaml;
        }

        if is_lower_index_than(xml_idx, &[toml_idx]) {
            return Format::Xml;
        }

        if toml_idx != -1 {
            return Format::Toml;
        }

        Format::Unknown
    }
}

// Go: parser/metadecoders/format.go:isLowerIndexThan
fn is_lower_index_than(first: isize, others: &[isize]) -> bool {
    if first == -1 {
        return false;
    }
    for &other in others {
        if other != -1 && other < first {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/metadecoders/format.go (114 lines; 2/4 funcs executed)
//   types: Format
// OK L35-42: FormatFromStrings(ss ...string) Format
// OK L46-68: FormatFromString(formatStr string) Format
// OK L73-101: (d Decoder) FormatFromContentString(data string) Format
// OK L103-114: isLowerIndexThan(first int, others ...int) bool
// ---------------------------------------------------------------------------
