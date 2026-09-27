//! Port of `resources/postpub/postpub.go`.
//!
//! Owner: Wave B task T14 (resources-core).


//! Go `resources/postpub`: placeholder resource. Field accessors return `prefix + Field + "__e="`
//! (e.g. `__h_pp_l1_1_Content__e=`); after rendering + hugo_stats.json, nh-hugolib's postProcess
//! replaces the placeholders in recorded files with `GetFieldString(field)` (which runs the lazy
//! chain: toCSS | postCSS | minify | fingerprint).

use std::sync::Arc;

use nh_common::Result;
use nh_resource::resourcetypes::Resource;

pub const POST_PROCESS_PREFIX: &str = "__h_pp_l1";
pub const POST_PROCESS_SUFFIX: &str = "__e=";

/// Go: `postpub.PostPublishedResource` / `PostPublishResource`.
pub struct PostPublishResource {
    /// `__h_pp_l1_<id>_`.
    pub prefix: String,
    pub delegate: Arc<dyn Resource>,
}

impl PostPublishResource {
    // Go: resources/postpub/postpub.go:NewPostPublishResource
    pub fn new(id: u64, r: Arc<dyn Resource>) -> Arc<PostPublishResource> {
        Arc::new(PostPublishResource { prefix: format!("{POST_PROCESS_PREFIX}_{id}_"), delegate: r })
    }

    /// Go: `field(name)` — the placeholder text.
    pub fn field(&self, name: &str) -> String {
        format!("{}{}{}", self.prefix, name, POST_PROCESS_SUFFIX)
    }

    /// Go: `GetFieldString(pattern)` — resolves a placeholder (`Content`, `RelPermalink`, `Permalink`,
    /// `Name`, `Title`, `ResourceType`, `MediaType.*`, `Data.Integrity` ...).
    // Go: resources/postpub/postpub.go:GetFieldString
    pub fn get_field_string(&self, pattern: &str) -> Option<Result<String>> {
        todo!()
    }
}

// Template API `*postpub.PostPublishResource`: Content, RelPermalink, Permalink, Name, Title,
// ResourceType, MediaType (struct of placeholders), Data (map of placeholders), Params (panics in Go).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/postpub/postpub.go (182 lines; 4/15 funcs executed)
//   types: PostPublishedResource, PostPublishResource
// EX L51-56: NewPostPublishResource(id int, r resource.Resource) PostPublishedResource
// EX L64-66: (r *PostPublishResource) field(name string) string
//    L68-70: (r *PostPublishResource) Permalink() string
//    L72-74: (r *PostPublishResource) RelPermalink() string
//    L76-78: (r *PostPublishResource) Origin() resource.Resource
// EX L80-117: (r *PostPublishResource) GetFieldString(pattern string) (string, bool)
//    L119-145: (r *PostPublishResource) fieldToString(receiver any, path string) string
//    L147-153: (r *PostPublishResource) Data() any
//    L155-158: (r *PostPublishResource) MediaType() map[string]any
//    L160-162: (r *PostPublishResource) ResourceType() string
//    L164-166: (r *PostPublishResource) Name() string
//    L168-170: (r *PostPublishResource) Title() string
//    L172-174: (r *PostPublishResource) Params() maps.Params
// EX L176-178: (r *PostPublishResource) Content(context.Context) (any, error)
//    L180-182: (r *PostPublishResource) fieldNotSupported(name string) string
// ---------------------------------------------------------------------------
