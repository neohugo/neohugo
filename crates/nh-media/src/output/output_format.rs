//! Port of `output/outputFormat.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


//! Go `output.Format` (+ built-in formats and `Formats` sorting: weight>0 first ascending, then name).

use go_value::{GoString, Object, Value};
use nh_common::Result;

use crate::media::media_type::MediaType;

/// Go: `output.Format`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputFormat {
    /// Lower-cased name, e.g. "html", "rss", "404", "sitemapindex".
    pub name: String,
    pub media_type: MediaType,
    /// Sub path, e.g. "amp".
    pub path: String,
    /// Base file name without extension, e.g. "index".
    pub base_name: String,
    /// e.g. "alternate", "canonical".
    pub rel: String,
    /// e.g. "webcal://".
    pub protocol: String,
    /// Selects text/template (no escaping) instead of html/template.
    pub is_plain_text: bool,
    pub is_html: bool,
    pub no_ugly: bool,
    pub ugly: bool,
    pub not_alternative: bool,
    pub root: bool,
    pub permalinkable: bool,
    pub weight: i64,
}

impl OutputFormat {
    // Go: output/outputFormat.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.name.is_empty()
    }

    // Go: output/outputFormat.go:BaseFilename
    pub fn base_filename(&self) -> String {
        todo!()
    }
}

/// Go: `output.Formats` (sorted by `Formats.Less`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Formats(pub Vec<OutputFormat>);

impl Formats {
    /// Go: `sort.Sort(formats)` with `Less`: weight > 0 first (ascending), then by name.
    // Go: output/outputFormat.go:Less
    pub fn sort(&mut self) {
        todo!("unique names -> any sort is fine; use go-sort to be safe")
    }
    // Go: output/outputFormat.go:GetByName (case-insensitive)
    pub fn get_by_name(&self, name: &str) -> Option<OutputFormat> { todo!() }
    // Go: output/outputFormat.go:GetBySuffix
    pub fn get_by_suffix(&self, suffix: &str) -> Option<OutputFormat> { todo!() }
    // Go: output/outputFormat.go:GetByNames
    pub fn get_by_names(&self, names: &[&str]) -> Result<Formats> { todo!() }
    // Go: output/outputFormat.go:FromFilename
    pub fn from_filename(&self, filename: &str) -> Option<OutputFormat> { todo!() }
}

/// Built-in output formats (Go: `output.AMPFormat`, `CalendarFormat`, ... `HTTPStatus404HTMLFormat`).
/// See specs/architecture-core.md §5.6 for the table.
pub struct BuiltinFormats {
    pub amp: OutputFormat,
    pub calendar: OutputFormat,
    pub css: OutputFormat,
    pub csv: OutputFormat,
    pub html: OutputFormat,
    pub alias_html: OutputFormat,
    pub markdown: OutputFormat,
    pub json: OutputFormat,
    pub web_app_manifest: OutputFormat,
    pub robots_txt: OutputFormat,
    pub rss: OutputFormat,
    pub sitemap: OutputFormat,
    pub sitemap_index: OutputFormat,
    pub gotmpl: OutputFormat,
    pub http_status_404_html: OutputFormat,
}

/// Go: the package-level format vars.
pub fn builtin_formats() -> &'static BuiltinFormats {
    todo!()
}

/// Go: `output.DefaultFormats` (sorted).
pub fn default_formats() -> Formats {
    todo!()
}

/// Template API of `output.Format` (reached via `page.OutputFormat`'s embedded `Format` field and
/// printed as a struct with `%v`): exported fields only.
impl Object for OutputFormat {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("output.Format")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "IsZero" | "BaseFilename" | "MarshalJSON")
    }
    fn call_method(&self, _ctx: go_value::HostCtx<'_>, name: &str, _args: &[Value]) -> Option<go_value::Result<Value>> {
        match name {
            "IsZero" => Some(Ok(Value::Bool(self.is_zero()))),
            "BaseFilename" => Some(Ok(Value::string(self.base_filename()))),
            _ => None,
        }
    }
    fn field(&self, name: &str) -> Option<Value> {
        Some(match name {
            "Name" => Value::string(self.name.as_str()),
            "MediaType" => self.media_type.to_value(),
            "Path" => Value::string(self.path.as_str()),
            "BaseName" => Value::string(self.base_name.as_str()),
            "Rel" => Value::string(self.rel.as_str()),
            "Protocol" => Value::string(self.protocol.as_str()),
            "IsPlainText" => Value::Bool(self.is_plain_text),
            "IsHTML" => Value::Bool(self.is_html),
            "NoUgly" => Value::Bool(self.no_ugly),
            "Ugly" => Value::Bool(self.ugly),
            "NotAlternative" => Value::Bool(self.not_alternative),
            "Root" => Value::Bool(self.root),
            "Permalinkable" => Value::Bool(self.permalinkable),
            "Weight" => Value::int(self.weight),
            _ => return None,
        })
    }
    fn is_zero(&self) -> Option<bool> {
        Some(self.is_zero())
    }
    fn struct_fields(&self) -> Option<Vec<(std::borrow::Cow<'_, str>, Value)>> {
        todo!("exported fields in declaration order (fmt %v / json)")
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: output/outputFormat.go (364 lines; 6/11 funcs executed)
//   types: Format, Formats
// EX L242-244: init()
// EX L250-250: (formats Formats) Len() int
// EX L251-251: (formats Formats) Swap(i, j int)
// EX L252-263: (formats Formats) Less(i, j int) bool
//    L269-284: (formats Formats) GetBySuffix(suffix string) (f Format, found bool)
// EX L287-296: (formats Formats) GetByName(name string) (f Format, found bool)
//    L299-310: (formats Formats) GetByNames(names ...string) (Formats, error)
//    L313-340: (formats Formats) FromFilename(filename string) (f Format, found bool)
//    L344-346: (f Format) BaseFilename() string
// EX L349-351: (f Format) IsZero() bool
//    L355-364: (f Format) MarshalJSON() ([]byte, error)
// ---------------------------------------------------------------------------
