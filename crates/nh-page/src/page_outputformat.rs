//! Port of `resources/page/page_outputformat.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{HostCtx, Object, SliceType, Value};
use nh_media::output::output_format::OutputFormat as Format;

/// Go: `page.OutputFormat` (template: `.Rel`, `.Name`, `.MediaType`, `.Permalink`, `.RelPermalink`,
/// embedded `Format` fields).
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
    pub fn new(rel_permalink: &str, permalink: &str, is_canonical: bool, f: Format) -> OutputFormat {
        todo!()
    }

    pub fn name(&self) -> &str {
        &self.format.name
    }

    pub fn permalink(&self) -> &str {
        &self.permalink
    }

    pub fn rel_permalink(&self) -> &str {
        &self.rel_permalink
    }
}

nh_common::go_methods!(OutputFormat {
    "Name" => |o, _c, _a| Ok(Value::string(o.format.name.as_str())),
    "MediaType" => |o, _c, _a| Ok(o.format.media_type.to_value()),
    "Permalink" => |o, _c, _a| Ok(Value::string(o.permalink.as_str())),
    "RelPermalink" => |o, _c, _a| Ok(Value::string(o.rel_permalink.as_str())),
});

impl Object for OutputFormat {
    nh_common::object_basics!("*page.OutputFormat");

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Rel" => Some(Value::string(self.rel.as_str())),
            "Format" => Some(Value::object(self.format.clone())),
            // Promoted fields of the embedded output.Format.
            other => self.format.field(other),
        }
    }
}

/// Go: `page.OutputFormats`.
pub type OutputFormats = Vec<OutputFormat>;

/// `page.OutputFormats` template value (`SliceType::Named("page.OutputFormats")`).
pub fn output_formats_to_value(ofs: &OutputFormats) -> Value {
    Value::list(
        SliceType::Named(Arc::from("page.OutputFormats")),
        ofs.iter().map(|o| Value::object(o.clone())).collect(),
    )
}

/// Go: `OutputFormats.Get(name)` — case-insensitive; `None` -> nil.
// Go: resources/page/page_outputformat.go:Get
pub fn output_formats_get<'a>(ofs: &'a OutputFormats, name: &str) -> Option<&'a OutputFormat> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_outputformat.go (95 lines; 5/6 funcs executed)
//   types: OutputFormats, OutputFormat
//    L49-51: (o OutputFormat) Name() string
// EX L54-56: (o OutputFormat) MediaType() media.Type
// EX L59-61: (o OutputFormat) Permalink() string
// EX L64-66: (o OutputFormat) RelPermalink() string
// EX L68-84: NewOutputFormat(relPermalink, permalink string, isCanonical bool, f output.Format) OutputFormat
// EX L88-95: (o OutputFormats) Get(name string) *OutputFormat
// ---------------------------------------------------------------------------
