//! Port of `resources/resource_spec.go`.
//!
//! Owner: Wave B task T14 (resources-core).


//! Go `resources.Spec` — the resource factory + caches, one per site (they share `SpecCommon`:
//! the resource cache, file caches and post-build assets are GLOBAL across languages).

use std::sync::Arc;

use nh_common::Result;
use nh_config::common_config::BuildConfig;
use nh_config::hexec::Exec;
use nh_helpers::cache::filecache::filecache::Caches;
use nh_helpers::pathspec::PathSpec;
use nh_images::image::ImageProcessor;
use nh_media::media::media_type::Types as MediaTypes;
use nh_media::output::output_format::Formats;
use nh_page::permalinks::PermalinkExpander;
use nh_resource::resourcetypes::Resource;

use crate::image_cache::ImageCache;
use crate::postpub::postpub::PostPublishResource;
use crate::resource::ResourceSourceDescriptor;
use crate::resource_cache::ResourceCache;

/// Go: `resources.PostBuildAssets`.
#[derive(Default)]
pub struct PostBuildAssets {
    /// TransformationKey -> placeholder resource (`resources.PostProcess`).
    pub post_process_resources: std::sync::Mutex<std::collections::BTreeMap<String, Arc<PostPublishResource>>>,
    pub js_config_builder: crate::jsconfig::Builder,
}

/// Go: `resources.SpecCommon` (shared by all sites).
pub struct SpecCommon {
    /// Go `identity.Incrementer` — the build's `deps.BuildState` (shared by all sites; the ONLY
    /// PostProcess id counter). Never a second counter here.
    pub incr: Arc<dyn nh_common::identity::Incrementer>,
    pub resource_cache: ResourceCache,
    pub file_caches: Caches,
    pub post_build_assets: PostBuildAssets,
}

/// Go: `resources.Spec`.
pub struct Spec {
    pub path_spec: Arc<PathSpec>,
    pub permalinks: PermalinkExpander,
    pub image_cache: Arc<ImageCache>,
    pub imaging: Arc<ImageProcessor>,
    pub exec_helper: Arc<Exec>,
    pub common: Arc<SpecCommon>,
}

impl Spec {
    /// Go: `resources.NewSpec(s, common, fileCaches, memCache, incr, logger, errorHandler, execHelper, ...)`.
    /// `incr` is the build's `BuildState` (`None` -> `IncrementByOne`, as in Go); it is only
    /// used when `common` is `None` (first site), later sites share `common` and its `incr`.
    // Go: resources/resource_spec.go:NewSpec
    pub fn new(
        ps: Arc<PathSpec>,
        common: Option<Arc<SpecCommon>>,
        file_caches: Caches,
        incr: Option<Arc<dyn nh_common::identity::Incrementer>>,
        exec: Arc<Exec>,
    ) -> Result<Arc<Spec>> {
        todo!()
    }

    /// Go: `NewResource(rd)` — genericResource or imageResource (by media type), wrapped in a
    /// resourceAdapter. Sets `includeHashInKey` for images (cold-cache Key rule).
    // Go: resources/resource_spec.go:NewResource
    pub fn new_resource(self: &Arc<Self>, rd: ResourceSourceDescriptor) -> Result<Arc<dyn Resource>> {
        todo!()
    }

    // Go: resources/resource_spec.go:NewResourceWrapperFromResourceConfig
    pub fn new_resource_wrapper_from_resource_config(&self, rc: &nh_page::pagemeta::page_frontmatter::ResourceConfig) -> Result<Arc<dyn Resource>> {
        todo!()
    }

    // Go: resources/resource_spec.go:MediaTypes
    pub fn media_types(&self) -> MediaTypes {
        todo!()
    }

    // Go: resources/resource_spec.go:OutputFormats
    pub fn output_formats(&self) -> Formats {
        todo!()
    }

    // Go: resources/resource_spec.go:BuildConfig
    pub fn build_config(&self) -> BuildConfig {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_spec.go (231 lines; 5/7 funcs executed)
//   types: Spec, SpecCommon, PostBuildAssets
// EX L48-116: NewSpec( s *helpers.PathSpec, common *SpecCommon, fileCaches filecache.Caches, memCache *dynacache.Cache, incr identity.Incrementer, logger loggers...
//    L155-163: (r *Spec) NewResourceWrapperFromResourceConfig(rc *pagemeta.ResourceConfig) (resource.Resource, error)
// EX L166-215: (r *Spec) NewResource(rd ResourceSourceDescriptor) (resource.Resource, error)
// EX L217-219: (r *Spec) MediaTypes() media.Types
// EX L221-223: (r *Spec) OutputFormats() output.Formats
// EX L225-227: (r *Spec) BuildConfig() config.BuildConfig
//    L229-231: (s *Spec) String() string
// ---------------------------------------------------------------------------
