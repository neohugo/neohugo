//! Port of `resources/postpub/postpub.go`.
//!
//! Owner: Wave B task T14 (resources-core).

//! Go `resources/postpub`: placeholder resource. Field accessors return `prefix + Field + "__e="`
//! (e.g. `__h_pp_l1_1_Content__e=`); after rendering + hugo_stats.json, nh-hugolib's postProcess
//! replaces the placeholders in recorded files with `GetFieldString(field)` (which runs the lazy
//! chain: toCSS | postCSS | minify | fingerprint).
//!
//! `*postpub.PostPublishResource` does not implement `resource.Resource` in Go (its `MediaType`
//! returns a map), so the port gives it its own template object ([`PostPublishRef`]).

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, MapType, Object, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_media::media::media_type::MediaType;
use nh_resource::resourcetypes::Resource;

use super::fields::{
    MEDIA_TYPE_STRUCT, insert_field_placeholders, struct_to_map_with_placeholders,
};

pub const POST_PROCESS_PREFIX: &str = "__h_pp_l1";
/// The suffix has an '=' in it to prevent the minifier to remove any enclosing quoutes around
/// the attribute values. See issue #8884.
pub const POST_PROCESS_SUFFIX: &str = "__e=";

/// Go: `postpub.PostPublishedResource` / `PostPublishResource`.
pub struct PostPublishResource {
    /// `__h_pp_l1_<id>_`.
    pub prefix: String,
    pub delegate: Arc<dyn Resource>,
}

impl PostPublishResource {
    // Go: resources/postpub/postpub.go:NewPostPublishResource
    pub fn new(id: i64, r: Arc<dyn Resource>) -> Arc<PostPublishResource> {
        Arc::new(PostPublishResource {
            prefix: format!("{POST_PROCESS_PREFIX}_{id}_"),
            delegate: r,
        })
    }

    /// Go: `field(name)` — the placeholder text.
    // Go: resources/postpub/postpub.go:field
    pub fn field(&self, name: &str) -> String {
        format!("{}{}{}", self.prefix, name, POST_PROCESS_SUFFIX)
    }

    // Go: resources/postpub/postpub.go:Permalink
    pub fn permalink(&self) -> String {
        self.field("Permalink")
    }

    // Go: resources/postpub/postpub.go:RelPermalink
    pub fn rel_permalink(&self) -> String {
        self.field("RelPermalink")
    }

    // Go: resources/postpub/postpub.go:Origin
    pub fn origin(&self) -> Arc<dyn Resource> {
        self.delegate.clone()
    }

    /// Go: `GetFieldString(pattern)` — resolves a placeholder (`Content`, `RelPermalink`,
    /// `Permalink`, `Name`, `Title`, `ResourceType`, `MediaType.*`, `Data.Integrity`).
    /// `None` = not a field of this resource (Go's `"", false`). Go panics on an unknown field
    /// accessor and on a pattern without the suffix; the port returns those as errors. As in
    /// Go, a failing `Content` gives `""`.
    // Go: resources/postpub/postpub.go:GetFieldString
    pub fn get_field_string(&self, pattern: &str) -> Option<Result<String>> {
        let prefix_idx = pattern.find(&self.prefix)?;

        let start = prefix_idx + self.prefix.len();
        let end = match pattern.find(POST_PROCESS_SUFFIX) {
            Some(e) if e >= start => e,
            Some(e) => {
                return Some(Err(Error::new(format!(
                    "runtime error: slice bounds out of range [{start}:{e}]"
                ))));
            }
            None => {
                return Some(Err(Error::new(format!(
                    "runtime error: slice bounds out of range [:-1] (start {start})"
                ))));
            }
        };
        let field_accessor = &pattern[start..end];

        let d = &self.delegate;
        Some(Ok(match field_accessor {
            "RelPermalink" => d.rel_permalink(),
            "Permalink" => d.permalink(),
            "Name" => d.name(),
            "Title" => d.title(),
            "ResourceType" => d.resource_type(),
            "Content" => {
                let ctx = nh_tpl::template::TplContext::default();
                match d.content(ctx.as_host()) {
                    Some(Ok(content)) => String::from_utf8_lossy(
                        nh_common::cast::caste::to_string(&content).as_bytes(),
                    )
                    .into_owned(),
                    Some(Err(_)) => String::new(),
                    None => {
                        return Some(Err(Error::new(format!(
                            "interface conversion: {} is not resource.ContentProvider: missing method Content",
                            d.tpl_type_name()
                        ))));
                    }
                }
            }
            f if f.starts_with("MediaType") => self.field_to_string(&d.media_type(), f),
            "Data.Integrity" => {
                let data = d.data();
                match &data {
                    Value::Map(m) => m
                        .get(b"Integrity")
                        .map(|v| {
                            String::from_utf8_lossy(nh_common::cast::caste::to_string(v).as_bytes())
                                .into_owned()
                        })
                        .unwrap_or_default(),
                    Value::TypedNil(_) => String::new(),
                    other => {
                        return Some(Err(Error::new(format!(
                            "interface conversion: interface {{}} is {}, not map[string]interface {{}}",
                            other.go_type_name()
                        ))));
                    }
                }
            }
            other => {
                return Some(Err(Error::new(format!(
                    "unknown field accessor {}",
                    go_strconv::quote(other.as_bytes())
                ))));
            }
        }))
    }

    /// Go: `fieldToString(receiver, path)` over `d.MediaType()` (a `media.Type` struct: a field
    /// by name, else a method without arguments, through `cast.ToString`).
    // Go: resources/postpub/postpub.go:fieldToString
    fn field_to_string(&self, receiver: &MediaType, path: &str) -> String {
        let fieldname = path.split('.').nth(1).unwrap_or("");
        let b = |v: bool| if v { "true" } else { "false" }.to_string();
        match fieldname {
            "Type" => receiver.typ.clone(),
            "MainType" => receiver.main_type.clone(),
            "SubType" => receiver.sub_type.clone(),
            "Delimiter" => receiver.delimiter.clone(),
            "SuffixesCSV" => receiver.suffixes_csv.clone(),
            // A SuffixInfo struct: cast.ToString fails and returns "".
            "FirstSuffix" => String::new(),
            "String" => receiver.string().to_string(),
            "IsText" => b(receiver.is_text()),
            "IsHTML" => b(receiver.is_html()),
            "IsMarkdown" => b(receiver.is_markdown()),
            "IsZero" => b(receiver.is_zero()),
            "MarshalJSON" => receiver
                .marshal_json_bytes()
                .map(|v| String::from_utf8_lossy(&v).into_owned())
                .unwrap_or_default(),
            // A []string: cast.ToString fails and returns "".
            "Suffixes" => String::new(),
            _ => String::new(),
        }
    }

    // Go: resources/postpub/postpub.go:Data
    pub fn data(&self) -> Value {
        let mut m = Map::new(MapType::StringAny);
        m.entries
            .insert(GoString::from("Integrity"), Value::string(""));
        insert_field_placeholders("Data", &mut m, &|s| self.field(s));
        Value::map(m)
    }

    // Go: resources/postpub/postpub.go:MediaType
    pub fn media_type(&self) -> Value {
        let m =
            struct_to_map_with_placeholders("MediaType", &MEDIA_TYPE_STRUCT, &|s| self.field(s));
        Value::map(m)
    }

    // Go: resources/postpub/postpub.go:ResourceType
    pub fn resource_type(&self) -> String {
        self.field("ResourceType")
    }

    // Go: resources/postpub/postpub.go:Name
    pub fn name(&self) -> String {
        self.field("Name")
    }

    // Go: resources/postpub/postpub.go:Title
    pub fn title(&self) -> String {
        self.field("Title")
    }

    /// Go panics with this message.
    // Go: resources/postpub/postpub.go:Params
    pub fn params(&self) -> Result<Value> {
        Err(Error::new(self.field_not_supported("Params")))
    }

    // Go: resources/postpub/postpub.go:Content
    pub fn content(&self) -> Result<Value> {
        Ok(Value::string(self.field("Content")))
    }

    // Go: resources/postpub/postpub.go:fieldNotSupported
    fn field_not_supported(&self, name: &str) -> String {
        format!("method .{name} is currently not supported in post-publish transformations.")
    }

    /// The template value (`*postpub.PostPublishResource`).
    pub fn to_value(self: &Arc<Self>) -> Value {
        Value::object(PostPublishRef(self.clone()))
    }
}

/// `*postpub.PostPublishResource` as a template object.
#[derive(Clone)]
pub struct PostPublishRef(pub Arc<PostPublishResource>);

fn gerr(e: Error) -> go_value::Error {
    go_value::Error::new(e.message().to_string())
}

nh_common::go_methods!(PostPublishRef {
    "Content" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Content")?;
        s.0.content().map_err(gerr)
    },
    "Data" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Data")?;
        Ok(s.0.data())
    },
    "GetFieldString" => |_s, _ctx, _a| Err(go_value::Error::new(
        "can't call method/function \"GetFieldString\" with 2 results",
    )),
    "MediaType" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "MediaType")?;
        Ok(s.0.media_type())
    },
    "Name" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Name")?;
        Ok(Value::string(s.0.name()))
    },
    "Origin" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Origin")?;
        Ok(s.0.origin().to_value())
    },
    "Params" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Params")?;
        s.0.params().map_err(gerr)
    },
    "Permalink" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Permalink")?;
        Ok(Value::string(s.0.permalink()))
    },
    "RelPermalink" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "RelPermalink")?;
        Ok(Value::string(s.0.rel_permalink()))
    },
    "ResourceType" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "ResourceType")?;
        Ok(Value::string(s.0.resource_type()))
    },
    "Title" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Title")?;
        Ok(Value::string(s.0.title()))
    },
});

impl Object for PostPublishRef {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*postpub.PostPublishResource")
    }
    fn has_method(&self, name: &str) -> bool {
        Self::go_has_method(name)
    }
    fn call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        self.go_call_method(ctx, name, args)
    }
    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// The post-publish resource behind a template value.
pub fn post_publish_from_value(v: &Value) -> Option<Arc<PostPublishResource>> {
    v.downcast::<PostPublishRef>().map(|r| r.0.clone())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/postpub/postpub.go (182 lines; 4/15 funcs executed)
//   types: PostPublishedResource, PostPublishResource
// OK L51-56: NewPostPublishResource(id int, r resource.Resource) PostPublishedResource
// OK L64-66: (r *PostPublishResource) field(name string) string
// OK L68-70: (r *PostPublishResource) Permalink() string
// OK L72-74: (r *PostPublishResource) RelPermalink() string
// OK L76-78: (r *PostPublishResource) Origin() resource.Resource
// OK L80-117: (r *PostPublishResource) GetFieldString(pattern string) (string, bool)
// OK L119-145: (r *PostPublishResource) fieldToString(receiver any, path string) string
// OK L147-153: (r *PostPublishResource) Data() any
// OK L155-158: (r *PostPublishResource) MediaType() map[string]any
// OK L160-162: (r *PostPublishResource) ResourceType() string
// OK L164-166: (r *PostPublishResource) Name() string
// OK L168-170: (r *PostPublishResource) Title() string
// OK L172-174: (r *PostPublishResource) Params() maps.Params
// OK L176-178: (r *PostPublishResource) Content(context.Context) (any, error)
// OK L180-182: (r *PostPublishResource) fieldNotSupported(name string) string
// ---------------------------------------------------------------------------
