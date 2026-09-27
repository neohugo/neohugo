//! Port of `parser/metadecoders/format.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


/// Go: `metadecoders.Format`.
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
}

/// Go: `metadecoders.FormatFromString("yml")` etc. (by extension or name, case-insensitive).
// Go: parser/metadecoders/format.go:FormatFromString
pub fn format_from_string(s: &str) -> Format {
    todo!()
}

/// Go: `FormatFromStrings(ss...)` — first known.
pub fn format_from_strings(ss: &[&str]) -> Format {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/metadecoders/format.go (114 lines; 2/4 funcs executed)
//   types: Format
// EX L35-42: FormatFromStrings(ss ...string) Format
// EX L46-68: FormatFromString(formatStr string) Format
//    L73-101: (d Decoder) FormatFromContentString(data string) Format
//    L103-114: isLowerIndexThan(first int, others ...int) bool
// ---------------------------------------------------------------------------
