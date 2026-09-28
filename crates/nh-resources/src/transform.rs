//! Port of `resources/transform.go`.
//!
//! Owner: Wave B task T14 (resources-core).

//! Go `resources/transform.go`: `resourceAdapter` — every resource returned to templates. Holds
//! the target (generic or image resource) plus a list of queued transformations; the chain runs
//! lazily once (`init(publish, setContent)`), with two ping-pong buffers, and writes the result to
//! the publish dir only when a link method is called (`.Content` never publishes).
//!
//! Sharing follows Go's pointers: the adapter struct is copied by `TransformWithContext`,
//! `cloneTo` and `WithResourceMeta`; a copy shares the `*resourceTransformations` (the
//! `sync.Once` and its error) and, until one of them re-points its own field, the
//! `*resourceAdapterInner` whose `target` a first transformation replaces in place.

use std::io::{Read, SeekFrom, Write};
use std::sync::{Arc, Mutex, OnceLock};

use go_value::{GoString, HostCtx, Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::hugio::{MultiWriteCloser, ReadSeekCloser};
use nh_media::media::media_type::MediaType;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resource::resourcetypes::Resource;
use nh_tpl::template::TplContext;

use crate::image::ImageResource;
use crate::resource::{AtomicStaler, GenericResource};
use crate::resource_metadata::MetaResource;
use crate::resource_spec::Spec;

/// These are transformations that need special support in Hugo that may not be available when
/// building the theme/site so we write the transformation result to disk and reuse if needed
/// for these.
// Go: resources/transform.go:transformationsToCacheOnDisk
fn transformation_may_be_cached_on_disk(name: &str) -> bool {
    matches!(name, "postcss" | "tocss" | "tocss-dart")
}

/// Go: `resources.ResourceTransformation`.
pub trait ResourceTransformation: Send + Sync {
    /// The key: `Name` + elements hashed (`internal.ResourceTransformationKey`).
    fn key(&self) -> ResourceTransformationKey;
    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()>;
}

/// Go's `*bytes.Buffer` as the transformation chain uses it: reads consume from the front,
/// `Len()` is the unread part, `Reset()` empties it.
#[derive(Default)]
pub struct GoBuffer {
    buf: Vec<u8>,
    off: usize,
}

impl GoBuffer {
    /// Go `Len()`.
    pub fn len(&self) -> usize {
        self.buf.len() - self.off
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Go `Reset()`.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.off = 0;
    }

    /// Go `Bytes()` (the unread part).
    pub fn bytes(&self) -> &[u8] {
        &self.buf[self.off..]
    }
}

impl Read for GoBuffer {
    fn read(&mut self, p: &mut [u8]) -> std::io::Result<usize> {
        let n = p.len().min(self.len());
        p[..n].copy_from_slice(&self.buf[self.off..self.off + n]);
        self.off += n;
        if self.off == self.buf.len() {
            // Go: Buffer is empty, reset to recover space.
            self.reset();
        }
        Ok(n)
    }
}

enum FromInner<'a> {
    /// The source's `hugio.ReadSeekCloser` (an `io.ReadSeeker`).
    Seeker(&'a mut dyn ReadSeekCloser),
    /// One of the chain's `*bytes.Buffer`s (not an `io.ReadSeeker`).
    Buffer(&'a mut GoBuffer),
}

/// Go `ResourceTransformationCtx.From` (an `io.Reader`): the source reader for the first
/// transformation (and after transformations that wrote nothing), else the previous
/// transformation's output buffer.
pub struct TransformFrom<'a> {
    inner: FromInner<'a>,
}

impl TransformFrom<'_> {
    /// Go `ctx.From.(io.ReadSeeker)` (integrity's `fingerprint` hashes such a source without
    /// writing `To`, and seeks back).
    pub fn as_read_seeker(&mut self) -> Option<&mut dyn ReadSeekCloser> {
        match &mut self.inner {
            FromInner::Seeker(s) => Some(&mut **s),
            FromInner::Buffer(_) => None,
        }
    }

    /// Whether `From` is an `io.ReadSeeker`.
    pub fn is_read_seeker(&self) -> bool {
        matches!(self.inner, FromInner::Seeker(_))
    }

    /// Reads everything that is left (Go `io.ReadAll(ctx.From)`).
    pub fn read_all(&mut self) -> Result<Vec<u8>> {
        let mut v = Vec::new();
        self.read_to_end(&mut v)?;
        Ok(v)
    }
}

impl Read for TransformFrom<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.inner {
            FromInner::Seeker(s) => s.read(buf),
            FromInner::Buffer(b) => b.read(buf),
        }
    }
}

/// Opens a publish file for a relative target path (Go `OpenResourcePublisher`).
pub type ResourcePublisher<'a> = &'a dyn Fn(&str) -> Result<MultiWriteCloser>;

/// Go: `resources.ResourceTransformationCtx`.
pub struct ResourceTransformationCtx<'a> {
    /// The context the transformation was queued with (ExecuteAsTemplate needs it).
    pub ctx: &'a TplContext,
    /// The content to transform.
    pub from: TransformFrom<'a>,
    /// The target of the content transformation. The current implementation requires that
    /// `from` is written to `to` even if no transformation is performed; if nothing is written,
    /// the chain passes `from` on.
    pub to: &'a mut Vec<u8>,
    /// The relative source path to the resource being transformed (Unix slashes).
    pub source_path: String,
    /// The relative target path to the resource being transformed (Unix slashes).
    pub in_path: String,
    /// The relative target path to the transformed resource ("" = unchanged).
    pub out_path: String,
    pub in_media_type: MediaType,
    pub out_media_type: MediaType,
    /// Data data can be set on the transformed Resource (e.g. `Integrity`). Must be simple
    /// types (Go serialises it to JSON).
    pub data: Map,
    /// Opens a publish file (source maps).
    pub open_resource_publisher: Option<ResourcePublisher<'a>>,
}

impl ResourceTransformationCtx<'_> {
    /// Go: `AddOutPathIdentifier(".min")` — `dir + base + id + ext` from InPath.
    // Go: resources/transform.go:AddOutPathIdentifier
    pub fn add_out_path_identifier(&mut self, identifier: &str) {
        self.out_path = self.add_path_identifier(&self.in_path, identifier);
    }

    /// PublishSourceMap writes the content to the target folder of the main resource with the
    /// ".map" extension added.
    // Go: resources/transform.go:PublishSourceMap
    pub fn publish_source_map(&self, content: &str) -> Result<()> {
        let target = format!("{}.map", self.out_path);
        let open = self.open_resource_publisher.ok_or_else(|| {
            Error::new("runtime error: invalid memory address or nil pointer dereference")
        })?;
        let mut f = open(&target)?;
        let r = f.write_all(content.as_bytes());
        let c = f.close();
        r?;
        c?;
        Ok(())
    }

    /// Go: `ReplaceOutPathExtension(".css")`.
    // Go: resources/transform.go:ReplaceOutPathExtension
    pub fn replace_out_path_extension(&mut self, new_ext: &str) {
        let (dir, file) = go_path::path::split(&self.in_path);
        let (base, _) = nh_common::paths::path::path_and_ext(file);
        self.out_path = go_path::path::join(&[dir, &format!("{base}{new_ext}")]).to_string();
    }

    // Go: resources/transform.go:addPathIdentifier
    fn add_path_identifier(&self, in_path: &str, identifier: &str) -> String {
        let (dir, file) = go_path::path::split(in_path);
        let (base, ext) = nh_common::paths::path::path_and_ext(file);
        go_path::path::join(&[dir, &format!("{base}{identifier}{ext}")]).to_string()
    }
}

/// Go: `baseResource` — the transformable target.
#[derive(Clone)]
pub enum BaseResource {
    Generic(Arc<GenericResource>),
    Image(Arc<ImageResource>),
}

impl BaseResource {
    /// The `genericResource` behind the target (an image's `baseResource`).
    pub fn generic(&self) -> &Arc<GenericResource> {
        match self {
            BaseResource::Generic(g) => g,
            BaseResource::Image(i) => &i.base,
        }
    }

    pub fn key(&self) -> String {
        self.generic().key()
    }
    pub fn media_type(&self) -> MediaType {
        self.generic().media_type()
    }
    pub fn name(&self) -> String {
        self.generic().name()
    }
    pub fn title(&self) -> String {
        self.generic().title()
    }
    pub fn params(&self) -> Arc<Map> {
        self.generic().params()
    }
    pub fn data(&self) -> Value {
        self.generic().data()
    }
    pub fn resource_type(&self) -> String {
        self.generic().resource_type()
    }
    pub fn rel_permalink(&self) -> String {
        self.generic().rel_permalink()
    }
    pub fn permalink(&self) -> String {
        self.generic().permalink()
    }
    pub fn target_path(&self) -> String {
        self.generic().target_path()
    }
    pub fn publish(&self) -> Result<()> {
        self.generic().publish()
    }
    pub fn is_published(&self) -> bool {
        self.generic().is_published()
    }
    pub fn read_seek_closer(&self) -> Result<Box<dyn ReadSeekCloser>> {
        self.generic().read_seek_closer()
    }
    pub fn content(&self) -> Result<Value> {
        self.generic().content()
    }
    pub fn name_normalized(&self) -> String {
        self.generic().name_normalized()
    }
    pub(crate) fn source_path(&self) -> String {
        self.generic().source_path()
    }
    pub(crate) fn stale_version(&self) -> u32 {
        self.generic().stale_version()
    }

    /// Go: `cloneTo(targetPath)` of the target.
    pub(crate) fn clone_to(&self, target_path: &str) -> BaseResource {
        match self {
            BaseResource::Generic(g) => BaseResource::Generic(Arc::new(g.clone_to(target_path))),
            BaseResource::Image(i) => BaseResource::Image(Arc::new(i.clone_to(target_path))),
        }
    }

    /// Go: `cloneWithUpdates(u)` of the target.
    pub(crate) fn clone_with_updates(&self, u: &TransformationUpdate) -> Result<BaseResource> {
        match self {
            BaseResource::Generic(g) => {
                Ok(BaseResource::Generic(Arc::new(g.clone_with_updates(u)?)))
            }
            BaseResource::Image(i) => Ok(BaseResource::Image(Arc::new(i.clone_with_updates(u)?))),
        }
    }
}

/// Go: `publishOnce` (`publisherInit sync.Once` + `publisherErr`).
#[derive(Default)]
pub struct PublishOnce {
    pub(crate) publisher: OnceLock<Option<Error>>,
}

/// Go: `resourceAdapterInner` (shared between adapter copies until one of them re-points).
pub struct ResourceAdapterInner {
    /// The context that started this transformation.
    pub ctx: TplContext,
    /// Go `target` (replaced in place by the first transformation of this inner).
    pub(crate) target: Mutex<BaseResource>,
    pub spec: Arc<Spec>,
    /// Go `resource.Staler` (a pointer shared by the adapter copies).
    pub(crate) staler: Arc<AtomicStaler>,
    /// Go `*publishOnce`: handles publishing (to /public) if needed; `None` is Go's nil.
    pub(crate) publish_once: Mutex<Option<Arc<PublishOnce>>>,
}

impl ResourceAdapterInner {
    pub fn target(&self) -> BaseResource {
        self.target
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn publish_once(&self) -> Option<Arc<PublishOnce>> {
        self.publish_once
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    // Go: resources/transform.go:(*resourceAdapterInner).StaleVersion
    pub fn stale_version(&self) -> u32 {
        // Both of these are incremented on change.
        self.staler
            .stale_version()
            .wrapping_add(self.target().stale_version())
    }
}

/// Go: `resourceTransformations`.
#[derive(Default)]
pub struct ResourceTransformations {
    /// Go `transformationsInit` + `transformationsErr`.
    pub(crate) init: OnceLock<Option<Error>>,
    pub(crate) transformations: Vec<Arc<dyn ResourceTransformation>>,
}

impl ResourceTransformations {
    /// hasTransformationPermalinkHash reports whether any of the transformations in the chain
    /// creates a permalink that's based on the content, e.g. fingerprint.
    // Go: resources/transform.go:hasTransformationPermalinkHash
    pub fn has_transformation_permalink_hash(&self) -> bool {
        self.transformations
            .iter()
            .any(|t| nh_common::constants::is_resource_transformation_permalink_hash(&t.key().name))
    }

    fn err(&self) -> Option<Error> {
        self.init.get().cloned().flatten()
    }
}

/// Go `resourceAdapter.metaProvider` (`resource.ResourceMetaProvider`): the target at creation
/// time, or the metadata of `WithResourceMeta`.
#[derive(Clone)]
pub(crate) enum MetaProvider {
    Target(BaseResource),
    Meta(Arc<MetaResource>),
}

impl MetaProvider {
    fn name(&self) -> String {
        match self {
            MetaProvider::Target(t) => t.name(),
            MetaProvider::Meta(m) => m.name(),
        }
    }
    fn title(&self) -> String {
        match self {
            MetaProvider::Target(t) => t.title(),
            MetaProvider::Meta(m) => m.title(),
        }
    }
    fn params(&self) -> Option<Arc<Map>> {
        match self {
            MetaProvider::Target(t) => Some(t.params()),
            MetaProvider::Meta(m) => m.params(),
        }
    }
}

/// Go: `resourceAdapter` (`*resources.resourceAdapter` in templates).
pub struct ResourceAdapter {
    /// Go's `*resourceAdapterInner` field of this adapter struct.
    pub(crate) inner: Mutex<Arc<ResourceAdapterInner>>,
    /// Go's `*resourceTransformations` field.
    pub(crate) transformations: Arc<ResourceTransformations>,
    /// Go `metaProvider`.
    pub(crate) meta: MetaProvider,
}

/// Go: `transformationUpdate`.
pub struct TransformationUpdate {
    pub content: Option<Vec<u8>>,
    pub source_filename: Option<String>,
    pub source_fs: Option<Arc<dyn nh_hugofs::afero::Fs>>,
    pub target_path: String,
    pub media_type: MediaType,
    pub data: Option<Map>,
}

impl TransformationUpdate {
    // Go: resources/transform.go:isContentChanged
    pub fn is_content_changed(&self) -> bool {
        self.content.is_some() || self.source_filename.is_some()
    }

    // Go: resources/transform.go:toTransformedResourceMetadata
    pub fn to_transformed_resource_metadata(&self) -> TransformedResourceMetadata {
        TransformedResourceMetadata {
            media_type_v: self.media_type.typ.clone(),
            target: self.target_path.clone(),
            meta_data: self.data.clone(),
        }
    }

    /// Go: `updateFromCtx(ctx)` (the context's fields are passed: the port's context borrows
    /// the chain's buffers and lives only for one transformation).
    // Go: resources/transform.go:updateFromCtx
    fn update_from_ctx(
        &mut self,
        out_path: &str,
        in_path: &str,
        out_media_type: &MediaType,
        data: &Map,
    ) {
        self.target_path = out_path.to_string();
        self.media_type = out_media_type.clone();
        self.data = Some(data.clone());
        self.target_path = in_path.to_string();
    }
}

/// Go: `transformedResourceMetadata` (persisted to `resources/_gen/assets/<key>.json`).
#[derive(Clone, Debug)]
pub struct TransformedResourceMetadata {
    pub target: String,
    pub media_type_v: String,
    pub meta_data: Option<Map>,
}

impl TransformedResourceMetadata {
    /// Go `json.Marshal(meta)`: `{"Target":…,"MediaType":…,"Data":…}` (struct field order).
    pub fn marshal_json(&self) -> Result<Vec<u8>> {
        let j = |v: &Value| go_json::marshal(v).map_err(|e| Error::new(e.to_string()));
        let mut b = b"{\"Target\":".to_vec();
        b.extend(j(&Value::string(self.target.as_str()))?);
        b.extend_from_slice(b",\"MediaType\":");
        b.extend(j(&Value::string(self.media_type_v.as_str()))?);
        b.extend_from_slice(b",\"Data\":");
        match &self.meta_data {
            Some(m) => b.extend(j(&Value::map(m.clone()))?),
            None => b.extend_from_slice(b"null"),
        }
        b.push(b'}');
        Ok(b)
    }
}

impl ResourceAdapter {
    /// Go: `newResourceAdapter(spec, lazyPublish, target)`.
    // Go: resources/transform.go:newResourceAdapter
    pub fn new(spec: Arc<Spec>, lazy_publish: bool, target: BaseResource) -> Arc<ResourceAdapter> {
        let po = if lazy_publish {
            Some(Arc::new(PublishOnce::default()))
        } else {
            None
        };
        Arc::new(ResourceAdapter {
            transformations: Arc::new(ResourceTransformations::default()),
            meta: MetaProvider::Target(target.clone()),
            inner: Mutex::new(Arc::new(ResourceAdapterInner {
                ctx: TplContext::default(),
                spec,
                publish_once: Mutex::new(po),
                target: Mutex::new(target),
                staler: Arc::new(AtomicStaler::default()),
            })),
        })
    }

    /// This adapter's current `*resourceAdapterInner`.
    pub fn inner(&self) -> Arc<ResourceAdapterInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set_inner(&self, inner: Arc<ResourceAdapterInner>) {
        *self.inner.lock().unwrap_or_else(|e| e.into_inner()) = inner;
    }

    /// Go `r.target` (without running the transformations).
    pub fn target(&self) -> BaseResource {
        self.inner().target()
    }

    fn spec(&self) -> Arc<Spec> {
        self.inner().spec.clone()
    }

    /// The adapter's transformations error (Go `transformationsErr`).
    pub fn transformations_err(&self) -> Option<Error> {
        self.transformations.err()
    }

    /// The queued transformations (Go `r.transformations`).
    pub fn transformations(&self) -> &[Arc<dyn ResourceTransformation>] {
        &self.transformations.transformations
    }

    /// A copy of this adapter struct (Go's value copy: same inner and transformations).
    fn copy_struct(&self) -> ResourceAdapter {
        ResourceAdapter {
            inner: Mutex::new(self.inner()),
            transformations: self.transformations.clone(),
            meta: self.meta.clone(),
        }
    }

    // Go: resources/transform.go:(*resourceAdapter).Content
    pub fn content(&self, _ctx: HostCtx<'_>) -> Result<Value> {
        self.init(false, true);
        if let Some(e) = self.transformations_err() {
            return Err(e);
        }
        self.target().content()
    }

    // Go: resources/transform.go:(*resourceAdapter).Data
    pub fn data(&self) -> Value {
        self.init(false, false);
        self.target().data()
    }

    /// Go: `cloneTo(targetPath)` (the copy shares the transformations).
    // Go: resources/transform.go:(resourceAdapter).cloneTo
    pub fn clone_to(&self, target_path: &str) -> Arc<dyn Resource> {
        let old = self.inner();
        let new_target = old.target().clone_to(target_path);
        let po = if old.publish_once().is_some() {
            Some(Arc::new(PublishOnce::default()))
        } else {
            None
        };
        let new_inner = ResourceAdapterInner {
            ctx: old.ctx.clone(),
            spec: old.spec.clone(),
            staler: old.staler.clone(),
            target: Mutex::new(new_target),
            publish_once: Mutex::new(po),
        };
        let r = self.copy_struct();
        r.set_inner(Arc::new(new_inner));
        Arc::new(r)
    }

    // Go: resources/transform.go:(*resourceAdapter).Process
    pub fn process(&self, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.get_image_ops()?.process(spec)
    }

    // Go: resources/transform.go:(*resourceAdapter).Crop
    pub fn crop(&self, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.get_image_ops()?.crop(spec)
    }

    // Go: resources/transform.go:(*resourceAdapter).Fill
    pub fn fill(&self, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.get_image_ops()?.fill(spec)
    }

    // Go: resources/transform.go:(*resourceAdapter).Fit
    pub fn fit(&self, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.get_image_ops()?.fit(spec)
    }

    // Go: resources/transform.go:(*resourceAdapter).Filter
    pub fn filter(&self, filters: &[Value]) -> Result<Arc<ResourceAdapter>> {
        self.get_image_ops()?.filter(filters)
    }

    /// Go: `Resize(spec)` (image ops delegate to the image target).
    // Go: resources/transform.go:(*resourceAdapter).Resize
    pub fn resize(&self, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.get_image_ops()?.resize(spec)
    }

    /// Go: `Height()` (Go panics for non-image resources; the port returns the message).
    // Go: resources/transform.go:(*resourceAdapter).Height
    pub fn height(&self) -> Result<i64> {
        Ok(self.get_image_ops()?.image.height())
    }

    // Go: resources/transform.go:(*resourceAdapter).Width
    pub fn width(&self) -> Result<i64> {
        Ok(self.get_image_ops()?.image.width())
    }

    // Go: resources/transform.go:(*resourceAdapter).Exif
    pub fn exif(&self) -> Result<Option<Arc<nh_images::exif::ExifInfo>>> {
        self.get_image_ops()?.exif()
    }

    // Go: resources/transform.go:(*resourceAdapter).Colors
    pub fn colors(&self) -> Result<Vec<nh_images::color::Color>> {
        self.get_image_ops()?.colors()
    }

    // Go: resources/transform.go:(*resourceAdapter).DecodeImage
    pub fn decode_image(&self) -> Result<nh_images::image::GoImage> {
        self.get_image_ops()?.decode_image()
    }

    // Go: resources/transform.go:(*resourceAdapter).Key
    pub fn key(&self) -> String {
        self.init(false, false);
        self.target().key()
    }

    // Go: resources/transform.go:(*resourceAdapter).TransientKey
    pub fn transient_key(&self) -> String {
        self.key()
    }

    // Go: resources/transform.go:(*resourceAdapter).targetPath
    pub(crate) fn target_path_internal(&self) -> String {
        self.init(false, false);
        self.target().generic().target_path_internal()
    }

    // Go: resources/transform.go:(*resourceAdapter).sourcePath
    pub(crate) fn source_path(&self) -> String {
        self.init(false, false);
        self.target().source_path()
    }

    // Go: resources/transform.go:(*resourceAdapter).MediaType
    pub fn media_type(&self) -> MediaType {
        self.init(false, false);
        self.target().media_type()
    }

    // Go: resources/transform.go:(*resourceAdapter).Name
    pub fn name(&self) -> String {
        self.init(false, false);
        self.meta.name()
    }

    // Go: resources/transform.go:(*resourceAdapter).NameNormalized
    pub fn name_normalized(&self) -> String {
        self.init(false, false);
        self.target().name_normalized()
    }

    /// Go: `Params()` (`None` is a nil `maps.Params`, only possible from resource metadata).
    // Go: resources/transform.go:(*resourceAdapter).Params
    pub fn params(&self) -> Option<Arc<Map>> {
        self.init(false, false);
        self.meta.params()
    }

    /// Go `Params()` as the shared map itself (front matter metadata starts from it and
    /// writes into it).
    pub(crate) fn params_shared(&self) -> Option<crate::resource_metadata::SharedParams> {
        self.init(false, false);
        match &self.meta {
            MetaProvider::Target(t) => Some(t.generic().params.clone()),
            MetaProvider::Meta(m) => m.params.clone(),
        }
    }

    // Go: resources/transform.go:(*resourceAdapter).Permalink
    pub fn permalink(&self) -> String {
        self.init(true, false);
        self.target().permalink()
    }

    // Go: resources/transform.go:(*resourceAdapter).Publish
    pub fn publish(&self) -> Result<()> {
        self.init(false, false);
        self.target().publish()
    }

    // Go: resources/transform.go:(*resourceAdapter).isPublished
    pub(crate) fn is_published(&self) -> bool {
        self.init(false, false);
        self.target().is_published()
    }

    // Go: resources/transform.go:(*resourceAdapter).ReadSeekCloser
    pub fn read_seek_closer(&self) -> Result<Box<dyn ReadSeekCloser>> {
        self.init(false, false);
        self.target().read_seek_closer()
    }

    // Go: resources/transform.go:(*resourceAdapter).RelPermalink
    pub fn rel_permalink(&self) -> String {
        self.init(true, false);
        self.target().rel_permalink()
    }

    // Go: resources/transform.go:(*resourceAdapter).ResourceType
    pub fn resource_type(&self) -> String {
        self.init(false, false);
        self.target().resource_type()
    }

    // Go: resources/transform.go:(*resourceAdapter).String
    pub fn string(&self) -> String {
        self.name()
    }

    // Go: resources/transform.go:(*resourceAdapter).Title
    pub fn title(&self) -> String {
        self.init(false, false);
        self.meta.title()
    }

    // Go: resources/transform.go:(resourceAdapter).Transform
    pub fn transform(
        &self,
        t: Vec<Arc<dyn ResourceTransformation>>,
    ) -> Result<Arc<ResourceAdapter>> {
        self.transform_with_context(&TplContext::default(), t)
    }

    /// Go: `TransformWithContext(ctx, t...)` — a COPY with the transformation appended.
    // Go: resources/transform.go:(resourceAdapter).TransformWithContext
    pub fn transform_with_context(
        &self,
        ctx: &TplContext,
        t: Vec<Arc<dyn ResourceTransformation>>,
    ) -> Result<Arc<ResourceAdapter>> {
        let mut transformations = self.transformations.transformations.clone();
        transformations.extend(t);

        let old = self.inner();
        let inner = ResourceAdapterInner {
            ctx: ctx.clone(),
            spec: old.spec.clone(),
            staler: old.staler.clone(),
            publish_once: Mutex::new(Some(Arc::new(PublishOnce::default()))),
            target: Mutex::new(old.target()),
        };

        Ok(Arc::new(ResourceAdapter {
            inner: Mutex::new(Arc::new(inner)),
            transformations: Arc::new(ResourceTransformations {
                init: OnceLock::new(),
                transformations,
            }),
            meta: self.meta.clone(),
        }))
    }

    /// Go: `WithResourceMeta(mp)` (a copy with the metadata; shares the transformations).
    // Go: resources/transform.go:(resourceAdapter).WithResourceMeta
    pub fn with_resource_meta(&self, mp: Arc<MetaResource>) -> Arc<ResourceAdapter> {
        let mut r = self.copy_struct();
        r.meta = MetaProvider::Meta(mp);
        Arc::new(r)
    }

    /// Go: `getImageOps()` — the image target (Go panics for other resources; the port returns
    /// the panic message).
    // Go: resources/transform.go:(*resourceAdapter).getImageOps
    pub(crate) fn get_image_ops(&self) -> Result<Arc<ImageResource>> {
        let BaseResource::Image(img) = self.target() else {
            if self.media_type().sub_type == "svg" {
                return Err(Error::new(
                    "this method is only available for raster images. To determine if an image is SVG, you can do {{ if eq .MediaType.SubType \"svg\" }}{{ end }}",
                ));
            }
            return Err(Error::new(
                "this method is only available for image resources",
            ));
        };
        self.init(false, false);
        // Go returns the target read before init.
        Ok(img)
    }

    // Go: resources/transform.go:(*resourceAdapter).publish
    fn publish_lazy(&self) {
        let inner = self.inner();
        let Some(po) = inner.publish_once() else {
            return;
        };

        po.publisher.get_or_init(|| {
            let err = inner.target().publish().err();
            if let Some(e) = &err {
                inner
                    .spec
                    .logger
                    .errorf(format!("Failed to publish Resource: {e}"));
            }
            err
        });
    }

    /// Go: `TransformationKey()` = cleanKey(target.Key()) + "_" + md5hex(concat("_" + t.Key().Value())).
    // Go: resources/transform.go:(*resourceAdapter).TransformationKey
    pub fn transformation_key(&self) -> String {
        let mut key = String::new();
        for tr in &self.transformations.transformations {
            key.push('_');
            key.push_str(&tr.key().value());
        }
        format!(
            "{}_{}",
            crate::resource_cache::ResourceCache::clean_key(&self.target().key()),
            nh_common::hashing::md5_from_string_hex_encoded(key.as_bytes())
        )
    }

    // Go: resources/transform.go:(*resourceAdapter).getOrTransform
    fn get_or_transform(&self, publish: bool, set_content: bool) -> Result<()> {
        let key = self.transformation_key();
        let spec = self.spec();
        let res = spec
            .resource_cache()
            .cache_resource_transformation
            .get_or_create(key.clone(), |_| {
                self.do_transform(&key, publish, set_content)
            })?;

        self.set_inner(res);
        Ok(())
    }

    // Go: resources/transform.go:(*resourceAdapter).transform
    fn do_transform(
        &self,
        key: &str,
        publish: bool,
        set_content: bool,
    ) -> Result<Arc<ResourceAdapterInner>> {
        let inner = self.inner();
        let spec = inner.spec.clone();
        let target = inner.target();

        let mut b1 = GoBuffer::default();
        let mut b2 = GoBuffer::default();

        let target_generic = target.generic().clone();
        let open_publisher = move |rel: &str| target_generic.open_publish_file_for_writing(rel);

        let mut data = Map::new(MapType::StringAny);
        let mut in_media_type = target.media_type();
        let mut out_media_type = target.media_type();

        let mut updates = TransformationUpdate {
            content: None,
            source_filename: None,
            source_fs: None,
            target_path: String::new(),
            media_type: MediaType::default(),
            data: None,
        };

        let mut contentrc = content_read_seeker_closer(&target)?;

        let mut in_path = target.target_path();
        let source_path = in_path.strip_prefix('/').unwrap_or(&in_path).to_string();
        let mut out_path = String::new();

        let mut counter = 0;
        let mut write_to_file_cache = false;

        // Which reader is `From` and which buffer is `To`.
        #[derive(Clone, Copy, PartialEq)]
        enum Sel {
            Source,
            B1,
            B2,
        }
        let mut from_sel = Sel::Source;
        let mut to_sel = Sel::B1;

        let mut transformed_contentr: Option<Vec<u8>> = None;
        let mut from_file_cache = false;

        let bcfg = spec.build_config();

        for (i, tr) in self.transformations.transformations.iter().enumerate() {
            let tr_key = tr.key();
            if i != 0 {
                in_media_type = out_media_type.clone();
            }

            let may_be_cached_on_disk = transformation_may_be_cached_on_disk(&tr_key.name);
            if !write_to_file_cache {
                write_to_file_cache = may_be_cached_on_disk;
            }

            if i > 0 {
                let to_len = match to_sel {
                    Sel::B1 => b1.len(),
                    Sel::B2 => b2.len(),
                    Sel::Source => 0,
                };
                let has_writes = to_len > 0;
                if has_writes {
                    counter += 1;
                    // Switch the buffers
                    if counter % 2 == 0 {
                        from_sel = Sel::B2;
                        b1.reset();
                        to_sel = Sel::B1;
                    } else {
                        from_sel = Sel::B1;
                        b2.reset();
                        to_sel = Sel::B2;
                    }
                }
            }

            let new_err = |err: Error, in_path: &str, in_media_type: &MediaType| -> Error {
                let msg = format!(
                    "{}: failed to transform {} ({})",
                    String::from_utf8_lossy(&go_unicode::strings::to_upper(tr_key.name.as_bytes())),
                    go_strconv::quote(in_path.as_bytes()),
                    in_media_type.typ
                );

                if err.is_feature_not_available() {
                    let err_msg = match String::from_utf8_lossy(&go_unicode::strings::to_lower(
                        tr_key.name.as_bytes(),
                    ))
                    .as_ref()
                    {
                        "postcss" => {
                            // This transformation is not available in this
                            // Most likely because PostCSS is not installed.
                            ". You need to install PostCSS. See https://gohugo.io/functions/css/postcss/"
                        }
                        "tailwindcss" => {
                            ". You need to install TailwindCSS CLI. See https://gohugo.io/functions/css/tailwindcss/"
                        }
                        "tocss-dart" => {
                            ". You need to install Dart Sass, see https://gohugo.io//functions/css/sass/#dart-sass"
                        }
                        "babel" => {
                            ". You need to install Babel, see https://gohugo.io/functions/js/babel/"
                        }
                        _ => "",
                    };
                    return err.wrap(format!("{msg}{err_msg}"));
                }

                err.wrap(msg)
            };

            let mut try_file_cache = false;
            let mut err: Option<Error> = None;
            if may_be_cached_on_disk && bcfg.use_resource_cache(None) {
                try_file_cache = true;
            } else {
                // (Once From is a buffer it never goes back to the source; To is always the
                // other buffer.)
                let (from, to): (FromInner<'_>, &mut Vec<u8>) = match (from_sel, to_sel) {
                    (Sel::Source, Sel::B1) => (FromInner::Seeker(contentrc.as_mut()), &mut b1.buf),
                    (Sel::Source, Sel::B2) => (FromInner::Seeker(contentrc.as_mut()), &mut b2.buf),
                    (Sel::B1, Sel::B2) => (FromInner::Buffer(&mut b1), &mut b2.buf),
                    (Sel::B2, Sel::B1) => (FromInner::Buffer(&mut b2), &mut b1.buf),
                    _ => unreachable!("From and To are different buffers"),
                };
                let mut tctx = ResourceTransformationCtx {
                    ctx: &inner.ctx,
                    from: TransformFrom { inner: from },
                    to,
                    source_path: source_path.clone(),
                    in_path: in_path.clone(),
                    out_path: out_path.clone(),
                    in_media_type: in_media_type.clone(),
                    out_media_type: out_media_type.clone(),
                    data: std::mem::replace(&mut data, Map::new(MapType::StringAny)),
                    open_resource_publisher: Some(&open_publisher),
                };
                let res = tr.transform(&mut tctx);
                in_path = tctx.in_path;
                out_path = tctx.out_path;
                in_media_type = tctx.in_media_type;
                out_media_type = tctx.out_media_type;
                data = tctx.data;

                if let Err(e) = res {
                    if !e.is_feature_not_available() {
                        return Err(new_err(e, &in_path, &in_media_type));
                    }
                    err = Some(e);
                }

                if may_be_cached_on_disk {
                    try_file_cache = bcfg.use_resource_cache(err.as_ref());
                }
                if let Some(e) = &err
                    && !try_file_cache
                {
                    return Err(new_err(e.clone(), &in_path, &in_media_type));
                }
            }

            if try_file_cache {
                let f = target
                    .generic()
                    .try_transformed_file_cache(key, &mut updates);
                let Some(mut f) = f else {
                    if let Some(e) = err {
                        return Err(new_err(e, &in_path, &in_media_type));
                    }
                    return Err(new_err(
                        Error::new(format!(
                            "resource {} not found in file cache",
                            go_strconv::quote(key.as_bytes())
                        )),
                        &in_path,
                        &in_media_type,
                    ));
                };
                let mut v = Vec::new();
                f.read_to_end(&mut v)?;
                transformed_contentr = Some(v);
                from_file_cache = true;
                // (updates.sourceFs = cache.fileCache.Fs: never reached, nothing is found.)

                // The reader above is all we need.
                break;
            }

            if !out_path.is_empty() {
                in_path = std::mem::take(&mut out_path);
            }
        }

        if !from_file_cache {
            updates.update_from_ctx(&out_path, &in_path, &out_media_type, &data);
        }

        let mut publishwriters: Vec<nh_common::hugio::BoxWriter> = Vec::new();

        if publish {
            let publicw = target
                .generic()
                .open_publish_file_for_writing(&updates.target_path)?;
            publishwriters.push(Box::new(publicw));
        }

        if transformed_contentr.is_none() {
            if write_to_file_cache {
                // Go also writes the result to the file cache (`resources/_gen/assets`): the
                // meta JSON and the content, and re-points the source to the cache file. The
                // port neither writes nor reads it (HUGO_LAYER.md §4.5): the content is kept in
                // memory instead (below), the same bytes.
                let _ = spec
                    .resource_cache()
                    .write_meta(key, &updates.to_transformed_resource_metadata())?;
            }

            // Any transformations reading from From must also write to To. This means that
            // if the target buffer is empty, we can just reuse the original reader.
            let to = match to_sel {
                Sel::B1 => &b1,
                Sel::B2 => &b2,
                Sel::Source => unreachable!("To is always a buffer"),
            };
            if !to.is_empty() {
                transformed_contentr = Some(to.bytes().to_vec());
            } else {
                let mut v = Vec::new();
                contentrc.read_to_end(&mut v)?;
                transformed_contentr = Some(v);
            }
        }
        let transformed = transformed_contentr.unwrap_or_default();

        // Also write it to memory. (Go skips this when the result went to the file cache and
        // the content was not asked for, and later reads the cache file; the port never reads
        // `resources/_gen`, so it always keeps the content.)
        let _ = set_content;

        let mut publishw = nh_common::hugio::new_multi_write_closer(publishwriters);
        publishw.write_all(&transformed)?;
        publishw.close()?;

        updates.content = Some(transformed);

        let new_target = target.clone_with_updates(&updates)?;
        *inner.target.lock().unwrap_or_else(|e| e.into_inner()) = new_target;

        Ok(inner)
    }

    // Go: resources/transform.go:(*resourceAdapter).init
    pub(crate) fn init(&self, publish: bool, set_content: bool) {
        self.init_transform(publish, set_content);
    }

    // Go: resources/transform.go:(*resourceAdapter).initTransform
    fn init_transform(&self, publish: bool, set_content: bool) {
        self.transformations.init.get_or_init(|| {
            if self.transformations.transformations.is_empty() {
                // Nothing to do.
                return None;
            }

            if publish {
                // The transformation will write the content directly to
                // the destination.
                *self
                    .inner()
                    .publish_once
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
            }

            let err = self.get_or_transform(publish, set_content).err();
            if let Some(e) = &err {
                self.spec().send_error(e);
            }
            err
        });

        if publish && self.inner().publish_once().is_some() {
            self.publish_lazy();
        }
    }

    /// Go `ForEeachIdentityByName` / `GetIdentity` / `GetIdentityGroup` /
    /// `GetDependencyManager`: the identity system is not ported (change detection is only
    /// needed by the server).
    fn identity_not_supported(name: &str) -> go_value::Error {
        go_value::Error::new(format!(
            "neohugo-rs: resourceAdapter.{name} (identity tracking) is not supported"
        ))
    }
}

/// contentReadSeekerCloser returns a ReadSeekerCloser if possible for a given Resource.
// Go: resources/transform.go:contentReadSeekerCloser
fn content_read_seeker_closer(r: &BaseResource) -> Result<Box<dyn ReadSeekCloser>> {
    r.read_seek_closer()
}

// ---------------------------------------------------------------------------
// Resource trait + template API.

fn to_go_err(e: Error) -> go_value::Error {
    go_value::Error::new(e.message().to_string())
}

fn params_value(p: Option<Arc<Map>>) -> Value {
    match p {
        Some(m) => Value::Map(m),
        None => Value::TypedNil(Arc::from("maps.Params")),
    }
}

fn adapter_value(r: Result<Arc<ResourceAdapter>>) -> go_value::Result<Value> {
    r.map(|a| a.to_value()).map_err(to_go_err)
}

nh_common::go_methods!(ResourceAdapter {
    "Colors" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Colors")?;
        let colors = s.colors().map_err(to_go_err)?;
        Ok(Value::list(
            go_value::SliceType::Named(Arc::from("[]images.Color")),
            colors.into_iter().map(Value::object).collect(),
        ))
    },
    "Content" => |s, ctx, a| {
        nh_common::object::args::exactly(a, 0, "Content")?;
        s.content(ctx).map_err(to_go_err)
    },
    "Crop" => |s, _ctx, a| adapter_value(s.crop(&nh_common::object::args::string(a, 0)?.to_str_lossy())),
    "Data" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Data")?;
        Ok(s.data())
    },
    "DecodeImage" => |_s, _ctx, _a| Err(go_value::Error::new(
        "neohugo-rs: resourceAdapter.DecodeImage from a template is not supported",
    )),
    "Exif" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Exif")?;
        match s.exif().map_err(to_go_err)? {
            Some(x) => Ok(Value::object(crate::image::ExifInfoObject(x))),
            None => Ok(Value::TypedNil(Arc::from("*exif.ExifInfo"))),
        }
    },
    "Fill" => |s, _ctx, a| adapter_value(s.fill(&nh_common::object::args::string(a, 0)?.to_str_lossy())),
    "Filter" => |s, _ctx, a| adapter_value(s.filter(a)),
    "Fit" => |s, _ctx, a| adapter_value(s.fit(&nh_common::object::args::string(a, 0)?.to_str_lossy())),
    "ForEeachIdentityByName" => |_s, _ctx, _a| Err(ResourceAdapter::identity_not_supported("ForEeachIdentityByName")),
    "GetDependencyManager" => |_s, _ctx, _a| Err(ResourceAdapter::identity_not_supported("GetDependencyManager")),
    "GetIdentity" => |_s, _ctx, _a| Err(ResourceAdapter::identity_not_supported("GetIdentity")),
    "GetIdentityGroup" => |_s, _ctx, _a| Err(ResourceAdapter::identity_not_supported("GetIdentityGroup")),
    "Height" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Height")?;
        s.height().map(Value::int).map_err(to_go_err)
    },
    "Key" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Key")?;
        Ok(Value::string(s.key()))
    },
    "MarkStale" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "MarkStale")?;
        s.inner().staler.mark_stale();
        Ok(Value::Invalid)
    },
    "MediaType" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "MediaType")?;
        Ok(s.media_type().to_value())
    },
    "Name" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Name")?;
        Ok(Value::string(s.name()))
    },
    "NameNormalized" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "NameNormalized")?;
        Ok(Value::string(s.name_normalized()))
    },
    "Params" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Params")?;
        Ok(params_value(s.params()))
    },
    "Permalink" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Permalink")?;
        Ok(Value::string(s.permalink()))
    },
    "Process" => |s, _ctx, a| adapter_value(s.process(&nh_common::object::args::string(a, 0)?.to_str_lossy())),
    "Publish" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Publish")?;
        s.publish().map_err(to_go_err)?;
        Ok(Value::TypedNil(Arc::from("error")))
    },
    "ReadSeekCloser" => |_s, _ctx, _a| Err(go_value::Error::new(
        "neohugo-rs: resourceAdapter.ReadSeekCloser from a template is not supported",
    )),
    "RelPermalink" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "RelPermalink")?;
        Ok(Value::string(s.rel_permalink()))
    },
    "Resize" => |s, _ctx, a| adapter_value(s.resize(&nh_common::object::args::string(a, 0)?.to_str_lossy())),
    "ResourceType" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "ResourceType")?;
        Ok(Value::string(s.resource_type()))
    },
    "Slice" => |_s, _ctx, a| {
        nh_common::object::args::exactly(a, 1, "Slice")?;
        crate::resource::common_resource_slice(&a[0]).map_err(to_go_err)
    },
    "StaleVersion" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "StaleVersion")?;
        Ok(Value::Uint(s.inner().stale_version() as u64, go_value::UintKind::Uint32))
    },
    "String" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "String")?;
        Ok(Value::string(s.string()))
    },
    "Title" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Title")?;
        Ok(Value::string(s.title()))
    },
    "Transform" => |_s, _ctx, _a| Err(go_value::Error::new(
        "neohugo-rs: resourceAdapter.Transform from a template is not supported",
    )),
    "TransformWithContext" => |_s, _ctx, _a| Err(go_value::Error::new(
        "neohugo-rs: resourceAdapter.TransformWithContext from a template is not supported",
    )),
    "TransformationKey" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "TransformationKey")?;
        Ok(Value::string(s.transformation_key()))
    },
    "TransientKey" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "TransientKey")?;
        Ok(Value::string(s.transient_key()))
    },
    "Width" => |s, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Width")?;
        s.width().map(Value::int).map_err(to_go_err)
    },
    "WithResourceMeta" => |_s, _ctx, _a| Err(go_value::Error::new(
        "neohugo-rs: resourceAdapter.WithResourceMeta from a template is not supported",
    )),
});

impl Resource for ResourceAdapter {
    fn resource_type(&self) -> String {
        ResourceAdapter::resource_type(self)
    }
    fn media_type(&self) -> MediaType {
        ResourceAdapter::media_type(self)
    }
    fn permalink(&self) -> String {
        ResourceAdapter::permalink(self)
    }
    fn rel_permalink(&self) -> String {
        ResourceAdapter::rel_permalink(self)
    }
    fn data(&self) -> Value {
        ResourceAdapter::data(self)
    }
    fn name(&self) -> String {
        ResourceAdapter::name(self)
    }
    fn title(&self) -> String {
        ResourceAdapter::title(self)
    }
    fn params(&self) -> Arc<Map> {
        ResourceAdapter::params(self).unwrap_or_else(|| Arc::new(Map::new(MapType::Params)))
    }
    fn key(&self) -> String {
        ResourceAdapter::key(self)
    }
    fn name_normalized(&self) -> Option<String> {
        Some(ResourceAdapter::name_normalized(self))
    }
    fn content(&self, ctx: HostCtx<'_>) -> Option<Result<Value>> {
        Some(ResourceAdapter::content(self, ctx))
    }
    fn read_seek_closer(&self) -> Option<Result<Box<dyn ReadSeekCloser>>> {
        Some(ResourceAdapter::read_seek_closer(self))
    }
    fn publish(&self) -> Option<Result<()>> {
        Some(ResourceAdapter::publish(self))
    }
    fn stale_version(&self) -> Option<u32> {
        Some(self.inner().stale_version())
    }
    fn mark_stale(&self) -> bool {
        self.inner().staler.mark_stale();
        true
    }
    fn tpl_type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("*resources.resourceAdapter")
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        ResourceAdapter::go_has_method(name)
    }
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<nh_common::object::GoResult<Value>> {
        self.go_call_method(ctx, name, args)
    }
    fn tpl_go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.string()))
    }
    fn to_value(self: Arc<Self>) -> Value {
        Value::object(nh_resource::resourcetypes::ResourceRef(self))
    }
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
        self
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ResourceAdapter {
    /// The template value (`*resources.resourceAdapter`).
    pub fn to_value(self: Arc<Self>) -> Value {
        Resource::to_value(self)
    }
}

/// The overlay/mask source of `images.Overlay`/`images.Mask` (Go `images.ImageSource`):
/// `DecodeImage` decodes the ENCODED bytes of the image (for a processed watermark, its encoded
/// PNG), never in-memory pixels (images.md §9.10).
impl nh_images::image::ImageSource for ResourceAdapter {
    fn decode_image(&self) -> Result<nh_images::image::GoImage> {
        ResourceAdapter::decode_image(self)
    }
    fn key(&self) -> String {
        ResourceAdapter::key(self)
    }
}

/// The `images.ImageSource` of a template value holding an image resource (for `images.Overlay`
/// and `images.Mask`; T19). `None` when the value is not a `*resources.resourceAdapter`.
pub fn image_source_from_value(v: &Value) -> Option<Arc<dyn nh_images::image::ImageSource>> {
    let r = nh_resource::resourcetypes::resource_ref_from_value(v)?;
    let a = r.as_any_arc().downcast::<ResourceAdapter>().ok()?;
    Some(a)
}

/// The adapter behind a `resource.Resource` (for the other resource crates).
pub fn resource_adapter(r: &Arc<dyn Resource>) -> Option<Arc<ResourceAdapter>> {
    r.clone().as_any_arc().downcast::<ResourceAdapter>().ok()
}

/// Seek helper for transformations that need the source's `io.ReadSeeker` (integrity).
pub fn seek_start(r: &mut dyn ReadSeekCloser) -> Result<()> {
    r.seek(SeekFrom::Start(0))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/transform.go (771 lines; 35/56 funcs executed)
//   types: ResourceTransformation, ResourceTransformationCtx, publishOnce, resourceAdapter, resourceAdapterInner,
//          resourceTransformations, transformableResource, transformationUpdate, transformedResourceMetadata
// OK L77-93: newResourceAdapter(spec *Spec, lazyPublish bool, target transformableResource) *resourceAdapter
// OK L143-145: (ctx *ResourceTransformationCtx) AddOutPathIdentifier(identifier string)
// OK L149-158: (ctx *ResourceTransformationCtx) PublishSourceMap(content string) error
// OK L162-166: (ctx *ResourceTransformationCtx) ReplaceOutPathExtension(newExt string)
// OK L168-172: (ctx *ResourceTransformationCtx) addPathIdentifier(inPath, identifier string) string
// OK L188-194: (r *resourceAdapter) Content(ctx context.Context) (any, error)
// OK L196-198: (r *resourceAdapter) GetIdentity() identity.Identity (identity not ported: explicit error)
// OK L200-203: (r *resourceAdapter) Data() any
// OK L205-215: (r *resourceAdapter) ForEeachIdentityByName(name string, f func(identity.Identity) bool) (identity not ported: explicit error)
// OK L217-219: (r *resourceAdapter) GetIdentityGroup() identity.Identity (identity not ported: explicit error)
// OK L221-223: (r *resourceAdapter) GetDependencyManager() identity.Manager (identity not ported: explicit error)
// OK L225-238: (r resourceAdapter) cloneTo(targetPath string) resource.Resource
// OK L240-242: (r *resourceAdapter) Process(spec string) (images.ImageResource, error)
// OK L244-246: (r *resourceAdapter) Crop(spec string) (images.ImageResource, error)
// OK L248-250: (r *resourceAdapter) Fill(spec string) (images.ImageResource, error)
// OK L252-254: (r *resourceAdapter) Fit(spec string) (images.ImageResource, error)
// OK L256-258: (r *resourceAdapter) Filter(filters ...any) (images.ImageResource, error)
// OK L260-262: (r *resourceAdapter) Resize(spec string) (images.ImageResource, error)
// OK L264-266: (r *resourceAdapter) Height() int
// OK L268-270: (r *resourceAdapter) Exif() *exif.ExifInfo
// OK L272-274: (r *resourceAdapter) Colors() ([]images.Color, error) (color-extractor not ported: explicit error)
// OK L276-279: (r *resourceAdapter) Key() string
// OK L281-283: (r *resourceAdapter) TransientKey() string
// OK L285-288: (r *resourceAdapter) targetPath() string
// OK L290-296: (r *resourceAdapter) sourcePath() string
// OK L298-301: (r *resourceAdapter) MediaType() media.Type
// OK L303-306: (r *resourceAdapter) Name() string
// OK L308-311: (r *resourceAdapter) NameNormalized() string
// OK L313-316: (r *resourceAdapter) Params() maps.Params
// OK L318-321: (r *resourceAdapter) Permalink() string
// OK L323-327: (r *resourceAdapter) Publish() error
// OK L329-332: (r *resourceAdapter) isPublished() bool
// OK L334-337: (r *resourceAdapter) ReadSeekCloser() (hugio.ReadSeekCloser, error)
// OK L339-342: (r *resourceAdapter) RelPermalink() string
// OK L344-347: (r *resourceAdapter) ResourceType() string
// OK L349-351: (r *resourceAdapter) String() string
// OK L353-356: (r *resourceAdapter) Title() string
// OK L358-360: (r resourceAdapter) Transform(t ...ResourceTransformation) (ResourceTransformer, error)
// OK L362-376: (r resourceAdapter) TransformWithContext(ctx context.Context, t ...ResourceTransformation) (ResourceTransformer, error)
// OK L378-380: (r *resourceAdapter) Width() int
// OK L382-384: (r *resourceAdapter) DecodeImage() (image.Image, error)
// OK L386-389: (r resourceAdapter) WithResourceMeta(mp resource.ResourceMetaProvider) resource.Resource
// OK L391-401: (r *resourceAdapter) getImageOps() images.ImageResourceOps
// OK L403-415: (r *resourceAdapter) publish()
// OK L417-423: (r *resourceAdapter) TransformationKey() string
// OK L425-436: (r *resourceAdapter) getOrTransform(publish, setContent bool) error
// OK L438-637: (r *resourceAdapter) transform(key string, publish, setContent bool) (*resourceAdapterInner, error)
// OK L639-641: (r *resourceAdapter) init(publish, setContent bool)
// OK L643-669: (r *resourceAdapter) initTransform(publish, setContent bool)
// OK L685-687: (r *resourceAdapterInner) GetIdentityGroup() identity.Identity (identity not ported)
// OK L689-692: (r *resourceAdapterInner) StaleVersion() uint32
// OK L702-709: (r *resourceTransformations) hasTransformationPermalinkHash() bool
// OK L732-734: (u *transformationUpdate) isContentChanged() bool
// OK L736-742: (u *transformationUpdate) toTransformedResourceMetadata() transformedResourceMetadata
// OK L744-749: (u *transformationUpdate) updateFromCtx(ctx *ResourceTransformationCtx)
// OK L759-771: contentReadSeekerCloser(r resource.Resource) (hugio.ReadSeekCloser, error)
// ---------------------------------------------------------------------------
