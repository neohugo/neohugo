//! Port of `resources/resource.go`.
//!
//! Owner: Wave B task T14 (resources-core).


//! Go `resources/resource.go`: `genericResource` (source bytes, target paths, publish-once, Key),
//! `ResourceSourceDescriptor`, `Copy`.
//!
//! COLD-CACHE RULE (HUGO_LAYER.md §9): `Key()` = RelPermalink minus base path, plus
//! `"_" + decimal(xxhash64(ORIGINAL source bytes))` when `include_hash_in_key && !source_filename_is_hash`.
//! The Rust port never sets `source_filename_is_hash` (it never reads the images file cache), so
//! processed images always carry the hash suffix — exactly the golden (cold) behaviour. Clones share
//! the root's hash (`Arc<ResourceHash>`).

use std::sync::{Arc, OnceLock};

use go_value::{Map, Value};
use nh_common::hugio::OpenReadSeekCloser;
use nh_common::paths::pathparser::Path;
use nh_common::Result;
use nh_media::media::media_type::MediaType;
use nh_resource::internal::resourcepaths::ResourcePaths;

use crate::resource_spec::Spec;

/// Go: `resources.ResourceSourceDescriptor`.
#[derive(Clone, Default)]
pub struct ResourceSourceDescriptor {
    /// The source content.
    pub open_read_seek_closer: Option<OpenReadSeekCloser>,
    /// The canonical source path.
    pub path: Option<Arc<Path>>,
    /// The normalized name of the resource.
    pub name_normalized: String,
    /// The name of the resource as it was read from the source.
    pub name_original: String,
    /// Any base paths prepended to the target path (multihost).
    pub target_base_paths: Vec<String>,
    /// The target path relative to the publish dir (defaults to the path).
    pub target_path: String,
    pub base_path_rel_permalink: String,
    pub base_path_target_path: String,
    /// The source filename or path (for error messages / cache keys).
    pub source_filename_or_path: String,
    pub title: String,
    pub data: Option<Map>,
    pub params: Option<Map>,
    /// Delay publishing until either Permalink or RelPermalink is called. Maybe never.
    pub lazy_publish: bool,
    /// Set if a specific media type is wanted (else from the target path extension).
    pub media_type: Option<MediaType>,
}

impl ResourceSourceDescriptor {
    // Go: resources/resource.go:init
    pub(crate) fn init(&mut self, spec: &Spec) -> Result<()> {
        todo!()
    }
}

/// Go: `resourceHash` (xxhash64 of the original source + size), shared by clones.
#[derive(Default)]
pub struct ResourceHash {
    pub(crate) value: OnceLock<(u64, i64)>,
}

/// Go: `genericResource`.
pub struct GenericResource {
    pub(crate) spec: Arc<Spec>,
    pub(crate) sd: ResourceSourceDescriptor,
    pub(crate) paths: ResourcePaths,
    pub(crate) include_hash_in_key: bool,
    /// Always false in the Rust port (cold semantics).
    pub(crate) source_filename_is_hash: bool,
    pub(crate) h: Arc<ResourceHash>,
    pub(crate) title: String,
    pub(crate) name: String,
    pub(crate) params: Arc<Map>,
    pub(crate) media_type: MediaType,
    pub(crate) key: OnceLock<String>,
    /// Go `publishInit *lazy.OnceMore`.
    pub(crate) published: OnceLock<Result<()>>,
}

impl GenericResource {
    /// Go: `Key()` (see the module doc for the cold-cache rule).
    // Go: resources/resource.go:Key
    pub fn key(&self) -> String {
        todo!()
    }

    /// Go: `hash()` — xxhash64 of the source (lazily, once, shared by clones).
    // Go: resources/resource.go:hash
    pub(crate) fn hash(&self) -> u64 {
        todo!()
    }

    /// Go: `Publish()` — copy the source to every target filename (publish fs), once.
    // Go: resources/resource.go:Publish
    pub fn publish(&self) -> Result<()> {
        todo!()
    }

    /// Go: `RelPermalink()` = basePath + PathEscape(TargetLink()).
    // Go: resources/resource.go:RelPermalink
    pub fn rel_permalink(&self) -> String {
        todo!()
    }

    /// Go: `Permalink()` = BaseURL.WithPathNoTrailingSlash + PathEscape(TargetPath()).
    // Go: resources/resource.go:Permalink
    pub fn permalink(&self) -> String {
        todo!()
    }

    // Go: resources/resource.go:Content
    pub fn content(&self) -> Result<Value> {
        todo!()
    }

    /// Go: `clone()` (copies the struct; the hash pointer is shared).
    // Go: resources/resource.go:clone
    pub(crate) fn clone_resource(&self) -> GenericResource {
        todo!()
    }
}

/// Go: `resources.Copy(r, targetPath)`.
// Go: resources/resource.go:Copy
pub fn copy(r: &Arc<dyn nh_resource::resourcetypes::Resource>, target_path: &str) -> Result<Arc<dyn nh_resource::resourcetypes::Resource>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource.go (733 lines; 32/53 funcs executed)
//   types: ResourceSourceDescriptor, ResourceTransformer, Transformer, transformerNotAvailable, resourceCopier,
//          baseResourceResource, baseResourceInternal, specProvider, baseResource, commonResource, fileInfo,
//          hashProvider, StaleValue[V, AtomicStaler, GenericResourceTestInfo, genericResource, targetPather,
//          isPublishedProvider, resourceHash, targetPathProvider, sourcePathProvider
// EX L109-192: (fd *ResourceSourceDescriptor) init(r *Spec) error
//    L204-208: NewFeatureNotAvailableTransformer(key string, elements ...any) ResourceTransformation
//    L214-216: (t transformerNotAvailable) Transform(ctx *ResourceTransformationCtx) error
//    L218-220: (t transformerNotAvailable) Key() internal.ResourceTransformationKey
//    L228-230: Copy(r resource.Resource, targetPath string) resource.Resource
// EX L278-297: (commonResource) Slice(in any) (any, error)
// EX L322-324: (s *StaleValue[V]) StaleVersion() uint32
//    L330-332: (s *AtomicStaler) MarkStale()
// EX L334-336: (s *AtomicStaler) StaleVersion() uint32
//    L344-357: GetTestInfoForResource(r resource.Resource) GenericResourceTestInfo
//    L383-385: (l *genericResource) IdentifierBase() string
//    L387-389: (l *genericResource) GetIdentityGroup() identity.Identity
// EX L391-393: (l *genericResource) GetDependencyManager() identity.Manager
// EX L395-397: (l *genericResource) ReadSeekCloser() (hugio.ReadSeekCloser, error)
// EX L399-401: (l *genericResource) Clone() resource.Resource
//    L403-406: (l *genericResource) size() int64
// EX L408-413: (l *genericResource) hash() uint64
// EX L415-417: (l *genericResource) setOpenSource(openSource hugio.OpenReadSeekCloser)
//    L419-421: (l *genericResource) setSourceFilenameIsHash(b bool)
// EX L423-425: (l *genericResource) setTargetPath(d internal.ResourcePaths)
//    L427-431: (l *genericResource) cloneTo(targetPath string) resource.Resource
// EX L433-441: (l *genericResource) Content(context.Context) (any, error)
//    L443-445: (l *genericResource) Data() any
// EX L447-466: (l *genericResource) Key() string
//    L468-470: (l *genericResource) TransientKey() string
//    L472-474: (l *genericResource) targetPath() string
//    L476-481: (l *genericResource) sourcePath() string
// EX L483-485: (l *genericResource) MediaType() media.Type
// EX L487-489: (l *genericResource) setMediaType(mediaType media.Type)
// EX L491-493: (l *genericResource) Name() string
// EX L495-497: (l *genericResource) NameNormalized() string
//    L499-501: (l *genericResource) Params() maps.Params
// EX L503-540: (l *genericResource) Publish() error
// EX L542-544: (l *genericResource) isPublished() bool
// EX L546-548: (l *genericResource) RelPermalink() string
// EX L550-552: (l *genericResource) Permalink() string
// EX L554-556: (l *genericResource) ResourceType() string
//    L558-560: (l *genericResource) String() string
// EX L563-565: (l *genericResource) TargetPath() string
// EX L567-569: (l *genericResource) Title() string
// EX L571-573: (l *genericResource) getSpec() *Spec
// EX L575-577: (l *genericResource) getResourcePaths() internal.ResourcePaths
//    L579-590: (r *genericResource) tryTransformedFileCache(key string, u *transformationUpdate) io.ReadCloser
// EX L592-604: (r *genericResource) mergeData(in map[string]any)
// EX L606-636: (rc *genericResource) cloneWithUpdates(u *transformationUpdate) (baseResource, error)
// EX L638-642: (l genericResource) clone() *genericResource
// EX L644-647: (r *genericResource) openPublishFileForWriting(relTargetPath string) (io.WriteCloser, error)
// EX L663-684: (r *resourceHash) init(l hugio.ReadSeekCloserProvider) error
// EX L686-688: hashImage(r io.ReadSeeker) (uint64, int64, error)
//    L691-693: InternalResourceTargetPath(r resource.Resource) string
//    L697-704: InternalResourceSourcePath(r resource.Resource) string
//    L709-714: InternalResourceSourcePathBestEffort(r resource.Resource) string
// EX L717-719: IsPublished(r resource.Resource) bool
// ---------------------------------------------------------------------------
