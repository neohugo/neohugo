//! Port of `resources/resource_spec.go`.
//!
//! Owner: Wave B task T14 (resources-core).

//! Go `resources.Spec` — the resource factory + caches, one per site (they share `SpecCommon`:
//! the resource cache, file caches and post-build assets are GLOBAL across languages).

use std::sync::{Arc, OnceLock};

use nh_common::Result;
use nh_common::dynacache::Cache as MemCache;
use nh_common::herrors::Error;
use nh_common::loggers::Logger;
use nh_config::common_config::BuildConfig;
use nh_config::config_provider::config_section;
use nh_config::hexec::Exec;
use nh_helpers::cache::filecache::filecache::Caches;
use nh_helpers::pathspec::PathSpec;
use nh_images::image::ImageProcessor;
use nh_media::media::media_type::Types as MediaTypes;
use nh_media::output::output_format::Formats;
use nh_page::permalinks::PermalinkExpander;
use nh_resource::resourcetypes::Resource;

use crate::image::ImageResource;
use crate::image_cache::ImageCache;
use crate::postpub::postpub::PostPublishResource;
use crate::resource::{AtomicStaler, GenericResource, ResourceHash, ResourceSourceDescriptor};
use crate::resource_cache::ResourceCache;
use crate::transform::{BaseResource, ResourceAdapter};

/// Go: `resources.PostBuildAssets`.
#[derive(Default)]
pub struct PostBuildAssets {
    /// TransformationKey -> placeholder resource (`resources.PostProcess`).
    pub post_process_resources:
        std::sync::Mutex<std::collections::BTreeMap<String, Arc<PostPublishResource>>>,
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

/// Go's `herrors.ErrorSender` (hugolib collects the errors of lazy transformations and fails
/// the build).
pub type ErrorSender = Arc<dyn Fn(&Error) + Send + Sync>;

/// Go: `resources.Spec`.
pub struct Spec {
    pub path_spec: Arc<PathSpec>,
    pub permalinks: PermalinkExpander,
    pub image_cache: Arc<ImageCache>,
    pub imaging: Arc<ImageProcessor>,
    pub exec_helper: Arc<Exec>,
    pub common: Arc<SpecCommon>,
    pub logger: Logger,
    /// Go `ErrorSender` (nil in Go tests: errors are then logged).
    pub error_sender: OnceLock<ErrorSender>,
}

impl Spec {
    /// Go: `resources.NewSpec(s, common, fileCaches, memCache, incr, logger, errorHandler, execHelper, ...)`.
    /// `incr` is the build's `BuildState` (`None` -> `IncrementByOne`, as in Go); it is only
    /// used when `common` is `None` (first site), later sites share `common` and its `incr`.
    /// `mem_cache` is the build's dynacache (Go `memCache`), shared by all sites: the resource
    /// and image partitions come from it.
    // Go: resources/resource_spec.go:NewSpec
    pub fn new(
        ps: Arc<PathSpec>,
        common: Option<Arc<SpecCommon>>,
        file_caches: Caches,
        mem_cache: &MemCache,
        incr: Option<Arc<dyn nh_common::identity::Incrementer>>,
        exec: Arc<Exec>,
        logger: Option<Logger>,
    ) -> Result<Arc<Spec>> {
        let conf = ps
            .cfg
            .get_config()
            .downcast::<nh_allconfig::allconfig::Config>()
            .map_err(|_| Error::new("resources.NewSpec: the config is not an *allconfig.Config"))?;
        let img_config = conf
            .imaging
            .clone()
            .ok_or_else(|| Error::new("resources.NewSpec: the imaging config is not decoded"))?;

        let imaging = ImageProcessor::new(img_config)?;

        let incr = incr.unwrap_or_else(|| Arc::new(nh_common::identity::IncrementByOne::default()));

        let logger = logger.unwrap_or_else(Logger::new_default);

        let urlize_ps = ps.clone();
        let permalinks = PermalinkExpander::new(
            Arc::new(move |s: &str| urlize_ps.urlize(s)),
            &conf.permalinks,
        )?;

        let common = match common {
            Some(c) => c,
            None => Arc::new(SpecCommon {
                incr,
                resource_cache: ResourceCache::new_with(file_caches.get("assets"), mem_cache),
                file_caches: file_caches.clone(),
                post_build_assets: PostBuildAssets::default(),
            }),
        };

        let image_cache = Arc::new(ImageCache::new_with(
            file_caches.get("images"),
            mem_cache,
            ps.clone(),
        ));

        // (Go assigns `rs.ResourceCache = newResourceCache(rs, memCache)` to the shared
        // SpecCommon on every NewSpec; its partitions come from the shared memCache, so the
        // first one is kept.)
        Ok(Arc::new(Spec {
            path_spec: ps,
            permalinks,
            image_cache,
            imaging,
            exec_helper: exec,
            common,
            logger,
            error_sender: OnceLock::new(),
        }))
    }

    /// Go: `NewResource(rd)` — genericResource or imageResource (by media type), wrapped in a
    /// resourceAdapter. Sets `includeHashInKey` for images (cold-cache Key rule).
    // Go: resources/resource_spec.go:NewResource
    pub fn new_resource(
        self: &Arc<Self>,
        mut rd: ResourceSourceDescriptor,
    ) -> Result<Arc<dyn Resource>> {
        Ok(self.new_resource_adapter(&mut rd)?)
    }

    /// [`Spec::new_resource`] with the concrete adapter type.
    pub fn new_resource_adapter(
        self: &Arc<Self>,
        rd: &mut ResourceSourceDescriptor,
    ) -> Result<Arc<ResourceAdapter>> {
        rd.init(self)?;
        let rd = rd.clone();

        let (dir, name) = go_path::path::split(&rd.target_path);
        let mut dir = nh_common::paths::path::to_slash_preserve_leading(dir);
        if dir == "/" {
            dir = String::new();
        }
        let rp = nh_resource::internal::resourcepaths::ResourcePaths {
            file: name.to_string(),
            dir,
            base_dir_target: rd.base_path_target_path.clone(),
            base_dir_link: rd.base_path_rel_permalink.clone(),
            target_base_paths: rd.target_base_paths.clone(),
        };

        let media_type = rd.media_type.clone().unwrap_or_default();
        let mut is_image = media_type.main_type == "image";
        let mut img_format = None;
        if is_image {
            img_format = nh_images::config::image_format_from_media_sub_type(&media_type.sub_type);
            is_image = img_format.is_some();
        }

        let gr = GenericResource {
            staler: Arc::new(AtomicStaler::default()),
            h: Arc::new(ResourceHash::default()),
            published: OnceLock::new(),
            key: OnceLock::new(),
            include_hash_in_key: is_image,
            source_filename_is_hash: false,
            paths: rp,
            spec: self.clone(),
            params: crate::resource_metadata::SharedParams::new(
                rd.params
                    .clone()
                    .unwrap_or_else(|| go_value::Map::new(go_value::MapType::Params)),
            ),
            name: rd.name_original.clone(),
            title: rd.title.clone(),
            sd: rd.clone(),
        };

        if let Some(img_format) = img_format {
            let gr = Arc::new(gr);
            let ir = ImageResource::new_root(
                nh_images::image::Image::new(
                    img_format,
                    self.imaging.clone(),
                    None,
                    Some(gr.clone() as Arc<dyn nh_images::image::Spec>),
                ),
                gr,
            );
            return Ok(ResourceAdapter::new(
                self.clone(),
                rd.lazy_publish,
                BaseResource::Image(Arc::new(ir)),
            ));
        }

        Ok(ResourceAdapter::new(
            self.clone(),
            rd.lazy_publish,
            BaseResource::Generic(Arc::new(gr)),
        ))
    }

    /// Go: `NewResourceWrapperFromResourceConfig(rc)` — the resource of a content adapter's
    /// resource config, with its metadata.
    // Go: resources/resource_spec.go:NewResourceWrapperFromResourceConfig
    pub fn new_resource_wrapper_from_resource_config(
        &self,
        rc: &mut nh_page::pagemeta::page_frontmatter::ResourceConfig,
    ) -> Result<Arc<dyn Resource>> {
        let content = rc.content.value.clone().unwrap_or(go_value::Value::Invalid);
        match nh_resource::resourcetypes::resource_from_value_any(&content) {
            Some(r) => Ok(
                crate::resource_metadata::clone_with_metadata_from_resource_config_if_needed(rc, r),
            ),
            None => {
                let path = rc
                    .path_info
                    .as_ref()
                    .map(|p| p.path().to_string())
                    .unwrap_or_default();
                Err(Error::new(format!(
                    "failed to create resource for path {}, expected a resource.Resource, got {}",
                    go_strconv::quote(path.as_bytes()),
                    match &content {
                        go_value::Value::Invalid => "<nil>".into(),
                        v => v.go_type_name(),
                    }
                )))
            }
        }
    }

    // Go: resources/resource_spec.go:MediaTypes
    pub fn media_types(&self) -> MediaTypes {
        (*config_section::<MediaTypes>(self.path_spec.cfg.as_ref(), "mediaTypes")).clone()
    }

    // Go: resources/resource_spec.go:OutputFormats
    pub fn output_formats(&self) -> Formats {
        (*config_section::<Formats>(self.path_spec.cfg.as_ref(), "outputFormats")).clone()
    }

    // Go: resources/resource_spec.go:BuildConfig
    pub fn build_config(&self) -> BuildConfig {
        (*config_section::<BuildConfig>(self.path_spec.cfg.as_ref(), "build")).clone()
    }

    // Go: resources/resource_spec.go:String
    pub fn string(&self) -> String {
        "spec".to_string()
    }

    /// Go `spec.PublishFs` (promoted from the embedded `*helpers.PathSpec` -> `*filesystems.BaseFs`).
    pub fn publish_fs(&self) -> Arc<dyn nh_hugofs::afero::Fs> {
        self.path_spec.base_fs.publish_fs.clone()
    }

    /// Go `spec.MultihostTargetBasePaths` (promoted from `*paths.Paths`).
    pub fn multihost_target_base_paths(&self) -> Vec<String> {
        self.path_spec.multihost_target_base_paths.clone()
    }

    /// Go `spec.Lang()` (promoted from `*paths.Paths`).
    pub fn lang(&self) -> String {
        self.path_spec.lang()
    }

    /// Go `spec.ResourceCache` (promoted from the embedded `*SpecCommon`).
    pub fn resource_cache(&self) -> &ResourceCache {
        &self.common.resource_cache
    }

    /// Go `spec.PostProcess(r)` (resources/post_publish.go).
    pub fn post_process(&self, r: Arc<dyn Resource>) -> Result<Arc<PostPublishResource>> {
        crate::post_publish::post_process(self, r)
    }

    /// Sends a lazy transformation error (Go `ErrorSender.SendError`), or logs it when no
    /// sender is set (Go: `Logger.Errorf("Transformation failed: %s", err)`).
    pub(crate) fn send_error(&self, err: &Error) {
        match self.error_sender.get() {
            Some(s) => s(err),
            None => self.logger.errorf(format!("Transformation failed: {err}")),
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_spec.go (231 lines; 5/7 funcs executed)
//   types: Spec, SpecCommon, PostBuildAssets
// OK L48-116: NewSpec( s *helpers.PathSpec, common *SpecCommon, fileCaches filecache.Caches, memCache *dynacache.Cache, incr identity.Incrementer, logger loggers...
// OK L155-163: (r *Spec) NewResourceWrapperFromResourceConfig(rc *pagemeta.ResourceConfig) (resource.Resource, error)
// OK L166-215: (r *Spec) NewResource(rd ResourceSourceDescriptor) (resource.Resource, error)
// OK L217-219: (r *Spec) MediaTypes() media.Types
// OK L221-223: (r *Spec) OutputFormats() output.Formats
// OK L225-227: (r *Spec) BuildConfig() config.BuildConfig
// OK L229-231: (s *Spec) String() string
// ---------------------------------------------------------------------------
