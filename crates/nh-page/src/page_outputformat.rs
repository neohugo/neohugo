//! Port of `resources/page/page_outputformat.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{HostCtx, Object, SliceType, Value};
use nh_common::object::{GoResult, NamedMethods};
use nh_media::output::output_format::{OutputFormat as Format, default_formats};

/// Go type string of the `page.OutputFormats` named slice.
pub const OUTPUT_FORMATS_TYPE: &str = "page.OutputFormats";

/// Go: `page.OutputFormat` (template: fields `Rel`, `Format`; methods `Name`, `MediaType`,
/// `Permalink`, `RelPermalink`). `Format` is a named field, not an embedded one, so the
/// `output.Format` fields are reached as `.Format.X`.
#[derive(Clone, Debug)]
pub struct OutputFormat {
    /// `canonical` for the page's main format, else the format's Rel.
    pub rel: String,
    pub format: Format,
    pub(crate) rel_permalink: String,
    pub(crate) permalink: String,
}

impl OutputFormat {
    /// Go: `page.NewOutputFormat(relPermalink, permalink, isCanonical, f)`.
    // Go: resources/page/page_outputformat.go:NewOutputFormat
    pub fn new(
        rel_permalink: &str,
        permalink: &str,
        is_canonical: bool,
        f: Format,
    ) -> OutputFormat {
        let mut is_user_configured = true;
        for d in default_formats().0.iter() {
            if go_unicode::strings::equal_fold(d.name.as_bytes(), f.name.as_bytes()) {
                is_user_configured = false;
            }
        }
        let mut rel = f.rel.clone();
        // If the output format is the canonical format for the content, we want
        // to specify this in the "rel" attribute of an HTML "link" element.
        // However, for custom output formats, we don't want to surprise users by
        // overwriting "rel"
        if is_canonical && !is_user_configured {
            rel = "canonical".to_string();
        }
        OutputFormat {
            rel,
            format: f,
            rel_permalink: rel_permalink.to_string(),
            permalink: permalink.to_string(),
        }
    }

    /// Go: `Name()` — this OutputFormat's name, i.e. HTML, AMP, JSON etc.
    // Go: resources/page/page_outputformat.go:Name
    pub fn name(&self) -> &str {
        &self.format.name
    }

    /// Go: `MediaType()`.
    // Go: resources/page/page_outputformat.go:MediaType
    pub fn media_type(&self) -> &nh_media::media::media_type::MediaType {
        &self.format.media_type
    }

    /// Go: `Permalink()`.
    // Go: resources/page/page_outputformat.go:Permalink
    pub fn permalink(&self) -> &str {
        &self.permalink
    }

    /// Go: `RelPermalink()`.
    // Go: resources/page/page_outputformat.go:RelPermalink
    pub fn rel_permalink(&self) -> &str {
        &self.rel_permalink
    }

    fn struct_fields_impl(&self) -> Vec<(Cow<'_, str>, Value)> {
        vec![
            (Cow::Borrowed("Rel"), Value::string(self.rel.as_str())),
            (Cow::Borrowed("Format"), Value::object(self.format.clone())),
            (
                Cow::Borrowed("relPermalink"),
                Value::string(self.rel_permalink.as_str()),
            ),
            (
                Cow::Borrowed("permalink"),
                Value::string(self.permalink.as_str()),
            ),
        ]
    }

    fn field_impl(&self, name: &str) -> Option<Value> {
        match name {
            "Rel" => Some(Value::string(self.rel.as_str())),
            "Format" => Some(Value::object(self.format.clone())),
            _ => None,
        }
    }
}

nh_common::go_methods!(OutputFormat {
    "Name" => |o, _c, a| {
        nh_common::object::args::exactly(a, 0, "Name")?;
        Ok(Value::string(o.format.name.as_str()))
    },
    "MediaType" => |o, _c, a| {
        nh_common::object::args::exactly(a, 0, "MediaType")?;
        Ok(o.format.media_type.to_value())
    },
    "Permalink" => |o, _c, a| {
        nh_common::object::args::exactly(a, 0, "Permalink")?;
        Ok(Value::string(o.permalink.as_str()))
    },
    "RelPermalink" => |o, _c, a| {
        nh_common::object::args::exactly(a, 0, "RelPermalink")?;
        Ok(Value::string(o.rel_permalink.as_str()))
    },
});

/// The value `page.OutputFormat` (elements of `page.OutputFormats`).
impl Object for OutputFormat {
    nh_common::object_basics!("page.OutputFormat");

    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.field_impl(name)
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(self.struct_fields_impl())
    }
}

/// `*page.OutputFormat` (the result of `OutputFormats.Get`): the value's method set and fields.
#[derive(Clone, Debug)]
pub struct OutputFormatPtr(pub OutputFormat);

impl Object for OutputFormatPtr {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*page.OutputFormat")
    }
    fn has_method(&self, name: &str) -> bool {
        OutputFormat::go_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        self.0.go_call_method(ctx, name, args)
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.0.field_impl(name)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `page.OutputFormats`.
pub type OutputFormats = Vec<OutputFormat>;

/// `page.OutputFormats` template value (`SliceType::Named("page.OutputFormats")`); Go's nil
/// slice (a page with no links) is `output_formats_nil_value`.
pub fn output_formats_to_value(ofs: &OutputFormats) -> Value {
    Value::list(
        SliceType::Named(Arc::from(OUTPUT_FORMATS_TYPE)),
        ofs.iter().map(|o| Value::object(o.clone())).collect(),
    )
}

/// A nil `page.OutputFormats`.
pub fn output_formats_nil_value() -> Value {
    Value::TypedNil(Arc::from(OUTPUT_FORMATS_TYPE))
}

/// Go: `OutputFormats.Get(name)` — case-insensitive; `None` -> nil.
// Go: resources/page/page_outputformat.go:Get
pub fn output_formats_get<'a>(ofs: &'a OutputFormats, name: &str) -> Option<&'a OutputFormat> {
    ofs.iter()
        .find(|f| go_unicode::strings::equal_fold(f.format.name.as_bytes(), name.as_bytes()))
}

pub fn output_formats_has_method(name: &str) -> bool {
    name == "Get"
}

/// Dispatch of the `page.OutputFormats` methods (`Get`).
pub fn output_formats_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    args: &[Value],
) -> Option<GoResult<Value>> {
    if name != "Get" {
        return None;
    }
    let items: Vec<OutputFormat> = match recv {
        Value::List(l) => l
            .items
            .iter()
            .filter_map(|v| v.downcast::<OutputFormat>().cloned())
            .collect(),
        Value::TypedNil(_) => Vec::new(),
        _ => return None,
    };
    Some((|| {
        nh_common::object::args::exactly(args, 1, name)?;
        let n = nh_common::object::args::string(args, 0)?;
        let n = String::from_utf8_lossy(n.as_bytes());
        Ok(match output_formats_get(&items, &n) {
            Some(o) => Value::object(OutputFormatPtr(o.clone())),
            None => Value::TypedNil(Arc::from("*page.OutputFormat")),
        })
    })())
}

pub const OUTPUT_FORMATS_METHODS: NamedMethods = NamedMethods {
    has_method: output_formats_has_method,
    call: output_formats_call_method,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_outputformat.go (95 lines; 5/6 funcs executed)
//   types: OutputFormats, OutputFormat
// OK L49-51: (o OutputFormat) Name() string
// OK L54-56: (o OutputFormat) MediaType() media.Type
// OK L59-61: (o OutputFormat) Permalink() string
// OK L64-66: (o OutputFormat) RelPermalink() string
// OK L68-84: NewOutputFormat(relPermalink, permalink string, isCanonical bool, f output.Format) OutputFormat
// OK L88-95: (o OutputFormats) Get(name string) *OutputFormat
// ---------------------------------------------------------------------------
