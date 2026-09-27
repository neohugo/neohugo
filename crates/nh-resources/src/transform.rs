//! Port of `resources/transform.go`.
//!
//! Owner: Wave B task T14 (resources-core).


//! Go `resources/transform.go`: `resourceAdapter` — every resource returned to templates. Holds
//! the target (generic or image resource) plus a list of queued transformations; the chain runs
//! lazily once (`init(publish, setContent)`), with two ping-pong buffers, and writes the result to
//! the publish dir only when a link method is called (`.Content` never publishes).

use std::sync::{Arc, Mutex, OnceLock};

use go_value::{HostCtx, Map, Value};
use nh_common::Result;
use nh_media::media::media_type::MediaType;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_tpl::template::TplContext;

use crate::image::ImageResource;
use crate::resource::GenericResource;
use crate::resource_spec::Spec;

/// Go: `resources.ResourceTransformation`.
pub trait ResourceTransformation: Send + Sync {
    /// The key: `Name` + elements hashed (`internal.ResourceTransformationKey`).
    fn key(&self) -> ResourceTransformationKey;
    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()>;
}

/// Go: `resources.ResourceTransformationCtx`.
pub struct ResourceTransformationCtx<'a> {
    /// The context the transformation was queued with (ExecuteAsTemplate needs it).
    pub ctx: &'a TplContext,
    /// The content to transform.
    pub from: &'a [u8],
    /// The target of the content transformation. If nothing is written, `from` passes through.
    pub to: &'a mut Vec<u8>,
    /// The relative source path to the resource being transformed.
    pub source_path: String,
    /// The relative target path to the resource being transformed.
    pub in_path: String,
    /// The relative target path to the transformed resource ("" = unchanged).
    pub out_path: String,
    pub in_media_type: MediaType,
    pub out_media_type: MediaType,
    /// Data data can be set on the transformed Resource (e.g. `Integrity`).
    pub data: Map,
    /// Opens a publish file (source maps).
    pub open_resource_publisher: Option<&'a dyn Fn(&str) -> Result<Box<dyn std::io::Write>>>,
}

impl ResourceTransformationCtx<'_> {
    /// Go: `AddOutPathIdentifier(".min")` — `dir + base + id + ext` from InPath.
    // Go: resources/transform.go:AddOutPathIdentifier
    pub fn add_out_path_identifier(&mut self, identifier: &str) {
        todo!()
    }

    /// Go: `ReplaceOutPathExtension(".css")`.
    // Go: resources/transform.go:ReplaceOutPathExtension
    pub fn replace_out_path_extension(&mut self, new_ext: &str) {
        todo!()
    }
}

/// Go: `baseResource` — the transformable target.
#[derive(Clone)]
pub enum BaseResource {
    Generic(Arc<GenericResource>),
    Image(Arc<ImageResource>),
}

/// Go: `resourceAdapterInner` (shared between adapter copies with the same transformations).
pub struct ResourceAdapterInner {
    pub ctx: TplContext,
    pub target: BaseResource,
    pub spec: Arc<Spec>,
    /// Go `publishOnce`.
    pub(crate) publish_once: OnceLock<Result<()>>,
}

/// Go: `resourceAdapter` (`*resources.resourceAdapter` in templates).
pub struct ResourceAdapter {
    pub(crate) inner: Arc<Mutex<Arc<ResourceAdapterInner>>>,
    pub(crate) transformations: Vec<Arc<dyn ResourceTransformation>>,
    pub(crate) transformations_init: OnceLock<Result<()>>,
    /// Go `metaProvider` (resource metadata from front matter `resources`).
    pub(crate) meta: Option<Arc<crate::resource_metadata::MetaResource>>,
}

impl ResourceAdapter {
    /// Go: `newResourceAdapter(spec, lazyPublish, target)`.
    // Go: resources/transform.go:newResourceAdapter
    pub fn new(spec: Arc<Spec>, lazy_publish: bool, target: BaseResource) -> Arc<ResourceAdapter> {
        todo!()
    }

    /// Go: `TransformWithContext(ctx, t...)` — a COPY with the transformation appended.
    // Go: resources/transform.go:TransformWithContext
    pub fn transform_with_context(&self, ctx: &TplContext, t: Vec<Arc<dyn ResourceTransformation>>) -> Result<Arc<ResourceAdapter>> {
        todo!()
    }

    /// Go: `TransformationKey()` = cleanKey(target.Key()) + "_" + md5hex(concat("_" + t.Key().Value())).
    // Go: resources/transform.go:TransformationKey
    pub fn transformation_key(&self) -> String {
        todo!()
    }

    /// Go: `init(publish, setContent)` -> `getOrTransform` -> `transform(key, publish, setContent)`.
    // Go: resources/transform.go:init
    pub(crate) fn init(&self, publish: bool, set_content: bool) -> Result<()> {
        todo!()
    }

    // Go: resources/transform.go:Content
    pub fn content(&self, ctx: HostCtx<'_>) -> Result<Value> {
        todo!()
    }

    // Go: resources/transform.go:Permalink
    pub fn permalink(&self) -> String {
        todo!()
    }

    // Go: resources/transform.go:RelPermalink
    pub fn rel_permalink(&self) -> String {
        todo!()
    }

    // Go: resources/transform.go:Key
    pub fn key(&self) -> String {
        todo!()
    }

    // Go: resources/transform.go:Resize (image ops delegate to the image target)
    pub fn resize(&self, spec: &str) -> Result<Arc<ResourceAdapter>> {
        todo!()
    }

    // Go: resources/transform.go:Filter
    pub fn filter(&self, filters: &[Value]) -> Result<Arc<ResourceAdapter>> {
        todo!()
    }

    // Go: resources/transform.go:Width
    pub fn width(&self) -> i64 {
        todo!()
    }

    // Go: resources/transform.go:Height
    pub fn height(&self) -> i64 {
        todo!()
    }
}

// Template API: `*resources.resourceAdapter` — Content, Data, Permalink, RelPermalink, Name,
// Title, Params, MediaType, ResourceType, Key, Publish, Resize, Fill, Fit, Crop, Filter, Process,
// Width, Height, Exif, Colors, String (= Name), and `Slice` (Go `commonResource.Slice`,
// resource.go:278-297, promoted into every resource): `slice $r1 $r2 ...` calls
// `collections.Slice`, which asks the first element for `Slice` and gets a `resource.Resources`
// (`SliceType::Named("resource.Resources")`); `resources.Concat` accepts that and rejects a plain
// `[]interface {}`. Implemented in Wave B as
// `impl nh_resource::resourcetypes::Resource for ResourceAdapter` + a `go_methods!` table.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/transform.go (771 lines; 35/56 funcs executed)
//   types: ResourceTransformation, ResourceTransformationCtx, publishOnce, resourceAdapter, resourceAdapterInner,
//          resourceTransformations, transformableResource, transformationUpdate, transformedResourceMetadata
// EX L77-93: newResourceAdapter(spec *Spec, lazyPublish bool, target transformableResource) *resourceAdapter
// EX L143-145: (ctx *ResourceTransformationCtx) AddOutPathIdentifier(identifier string)
//    L149-158: (ctx *ResourceTransformationCtx) PublishSourceMap(content string) error
// EX L162-166: (ctx *ResourceTransformationCtx) ReplaceOutPathExtension(newExt string)
// EX L168-172: (ctx *ResourceTransformationCtx) addPathIdentifier(inPath, identifier string) string
// EX L188-194: (r *resourceAdapter) Content(ctx context.Context) (any, error)
//    L196-198: (r *resourceAdapter) GetIdentity() identity.Identity
//    L200-203: (r *resourceAdapter) Data() any
//    L205-215: (r *resourceAdapter) ForEeachIdentityByName(name string, f func(identity.Identity) bool)
//    L217-219: (r *resourceAdapter) GetIdentityGroup() identity.Identity
//    L221-223: (r *resourceAdapter) GetDependencyManager() identity.Manager
//    L225-238: (r resourceAdapter) cloneTo(targetPath string) resource.Resource
//    L240-242: (r *resourceAdapter) Process(spec string) (images.ImageResource, error)
//    L244-246: (r *resourceAdapter) Crop(spec string) (images.ImageResource, error)
//    L248-250: (r *resourceAdapter) Fill(spec string) (images.ImageResource, error)
//    L252-254: (r *resourceAdapter) Fit(spec string) (images.ImageResource, error)
// EX L256-258: (r *resourceAdapter) Filter(filters ...any) (images.ImageResource, error)
// EX L260-262: (r *resourceAdapter) Resize(spec string) (images.ImageResource, error)
// EX L264-266: (r *resourceAdapter) Height() int
//    L268-270: (r *resourceAdapter) Exif() *exif.ExifInfo
//    L272-274: (r *resourceAdapter) Colors() ([]images.Color, error)
// EX L276-279: (r *resourceAdapter) Key() string
//    L281-283: (r *resourceAdapter) TransientKey() string
//    L285-288: (r *resourceAdapter) targetPath() string
//    L290-296: (r *resourceAdapter) sourcePath() string
// EX L298-301: (r *resourceAdapter) MediaType() media.Type
// EX L303-306: (r *resourceAdapter) Name() string
// EX L308-311: (r *resourceAdapter) NameNormalized() string
//    L313-316: (r *resourceAdapter) Params() maps.Params
// EX L318-321: (r *resourceAdapter) Permalink() string
// EX L323-327: (r *resourceAdapter) Publish() error
// EX L329-332: (r *resourceAdapter) isPublished() bool
// EX L334-337: (r *resourceAdapter) ReadSeekCloser() (hugio.ReadSeekCloser, error)
// EX L339-342: (r *resourceAdapter) RelPermalink() string
// EX L344-347: (r *resourceAdapter) ResourceType() string
// EX L349-351: (r *resourceAdapter) String() string
// EX L353-356: (r *resourceAdapter) Title() string
// EX L358-360: (r resourceAdapter) Transform(t ...ResourceTransformation) (ResourceTransformer, error)
// EX L362-376: (r resourceAdapter) TransformWithContext(ctx context.Context, t ...ResourceTransformation) (ResourceTransformer, error)
// EX L378-380: (r *resourceAdapter) Width() int
// EX L382-384: (r *resourceAdapter) DecodeImage() (image.Image, error)
//    L386-389: (r resourceAdapter) WithResourceMeta(mp resource.ResourceMetaProvider) resource.Resource
// EX L391-401: (r *resourceAdapter) getImageOps() images.ImageResourceOps
// EX L403-415: (r *resourceAdapter) publish()
// EX L417-423: (r *resourceAdapter) TransformationKey() string
// EX L425-436: (r *resourceAdapter) getOrTransform(publish, setContent bool) error
// EX L438-637: (r *resourceAdapter) transform(key string, publish, setContent bool) (*resourceAdapterInner, error)
// EX L639-641: (r *resourceAdapter) init(publish, setContent bool)
// EX L643-669: (r *resourceAdapter) initTransform(publish, setContent bool)
//    L685-687: (r *resourceAdapterInner) GetIdentityGroup() identity.Identity
// EX L689-692: (r *resourceAdapterInner) StaleVersion() uint32
//    L702-709: (r *resourceTransformations) hasTransformationPermalinkHash() bool
//    L732-734: (u *transformationUpdate) isContentChanged() bool
// EX L736-742: (u *transformationUpdate) toTransformedResourceMetadata() transformedResourceMetadata
// EX L744-749: (u *transformationUpdate) updateFromCtx(ctx *ResourceTransformationCtx)
// EX L759-771: contentReadSeekerCloser(r resource.Resource) (hugio.ReadSeekCloser, error)
// ---------------------------------------------------------------------------
