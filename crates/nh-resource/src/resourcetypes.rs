//! Port of `resources/resource/resourcetypes.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `resources/resource/resourcetypes.go`: the `resource.Resource` interface family.
//!
//! Rust shape: one object-safe trait [`Resource`] (the union of the Go interfaces that are asserted
//! on resources) implemented by every concrete resource type (nh-resources: genericResource,
//! resourceAdapter, imageResource, PostPublishResource; nh-hugolib: pages, since `page.Page`
//! embeds `resource.Resource`). Optional Go interfaces (`ContentProvider`, `Source`,
//! `NameNormalizedProvider`, ...) are `Option`-returning methods.
//!
//! Template values: non-page resources are `Value::Object(Arc<ResourceRef>)`; pages are
//! `nh_page::PageRef` objects. Each concrete type builds its own value with [`Resource::to_value`].

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, Object, Value};
use nh_common::hugio::ReadSeekCloser;
use nh_common::object::GoResult;
use nh_common::Result;
use nh_langs::language::Language;
use nh_media::media::media_type::MediaType;

/// Go: `resource.Resource` (+ `Identifier`, `ContentProvider`, `Source`, `NameNormalizedProvider`,
/// `ReadSeekCloserResource`, `LanguageProvider`, `TranslationKeyProvider`, `Staler`).
pub trait Resource: Any + Send + Sync {
    /// Go: `ResourceType()` — e.g. "image", "page", "text", "application".
    fn resource_type(&self) -> String;
    fn media_type(&self) -> MediaType;
    /// Go: `Permalink()` — MAY publish the resource (lazy publish-once).
    fn permalink(&self) -> String;
    /// Go: `RelPermalink()` — MAY publish the resource.
    fn rel_permalink(&self) -> String;
    /// Go: `Data()`.
    fn data(&self) -> Value;
    /// Go: `Name()` (original case for bundle resources).
    fn name(&self) -> String;
    fn title(&self) -> String;
    fn params(&self) -> Arc<Map>;
    /// Go: `resource.Identifier.Key()` (see nh-resources `genericResource.Key`, cold-cache rule).
    fn key(&self) -> String;

    /// Go: `NameNormalizedProvider.NameNormalized()`.
    fn name_normalized(&self) -> Option<String> {
        None
    }
    /// Go: `ContentProvider.Content(ctx)`.
    fn content(&self, ctx: HostCtx<'_>) -> Option<Result<Value>> {
        None
    }
    /// Go: `hugio.ReadSeekCloserProvider.ReadSeekCloser()`.
    fn read_seek_closer(&self) -> Option<Result<Box<dyn ReadSeekCloser>>> {
        None
    }
    /// Go: `resource.Source.Publish()`.
    fn publish(&self) -> Option<Result<()>> {
        None
    }
    /// Go: `LanguageProvider.Language()`.
    fn language(&self) -> Option<Arc<Language>> {
        None
    }
    /// Go: `TranslationKeyProvider.TranslationKey()`.
    fn translation_key(&self) -> Option<String> {
        None
    }

    // ---- template API (replaces reflection) ----
    /// Go type string, e.g. `*resources.resourceAdapter`.
    fn tpl_type_name(&self) -> Cow<'_, str>;
    fn tpl_has_method(&self, name: &str) -> bool;
    fn tpl_call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>>;
    /// `fmt.Stringer` (Go resources print via `String()` = `Name()`; `path.Ext` relies on it).
    fn tpl_go_string(&self) -> Option<GoString> {
        None
    }
    fn tpl_is_zero(&self) -> Option<bool> {
        None
    }

    /// The template value for this resource (its own wrapper type).
    fn to_value(self: Arc<Self>) -> Value;
    /// Owned downcast support (`Arc<dyn Resource>` -> concrete).
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn as_any(&self) -> &dyn Any;
}

/// Object wrapper for non-page resources.
#[derive(Clone)]
pub struct ResourceRef(pub Arc<dyn Resource>);

impl Object for ResourceRef {
    fn type_name(&self) -> Cow<'_, str> {
        self.0.tpl_type_name()
    }
    fn has_method(&self, name: &str) -> bool {
        self.0.tpl_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        self.0.tpl_call_method(ctx, name, args)
    }
    fn go_string(&self) -> Option<GoString> {
        self.0.tpl_go_string()
    }
    fn is_zero(&self) -> Option<bool> {
        self.0.tpl_is_zero()
    }
    fn hash_key(&self) -> Option<GoString> {
        Some(GoString::from(self.0.key()))
    }
    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as *const () as usize
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go type `resource.Resources` as a Rust list.
pub type Resources = Vec<Arc<dyn Resource>>;

/// Go: `resource.Resources` as a template value (`SliceType::Named("resource.Resources")`).
pub fn resources_to_value(rs: &Resources) -> Value {
    Value::list(
        go_value::SliceType::Named(Arc::from("resource.Resources")),
        rs.iter().map(|r| r.clone().to_value()).collect(),
    )
}

/// A non-page resource from a template value (pages: `nh_page::page::resource_from_value`).
pub fn resource_ref_from_value(v: &Value) -> Option<Arc<dyn Resource>> {
    v.downcast::<ResourceRef>().map(|r| r.0.clone())
}

/// Go: `resource.ResourceError`.
#[derive(Clone, Debug)]
pub struct ResourceError {
    pub msg: String,
    pub data: Option<Value>,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource/resourcetypes.go (306 lines; 2/9 funcs executed)
//   types: Cloner, OriginProvider, resourceError, ResourceError, Resource, ResourceWithoutMeta, ResourceTypeProvider,
//          ResourceTypesProvider, MediaTypeProvider, ResourceLinksProvider, ResourceMetaProvider,
//          WithResourceMetaProvider, ResourceNameTitleProvider, NameNormalizedProvider, ResourceParamsProvider,
//          ResourceDataProvider, ResourcesLanguageMerger, Identifier, TransientIdentifier, WeightProvider,
//          Weight0Provider, ContentResource, ContentProvider, ReadSeekCloserResource, LengthProvider,
//          LanguageProvider, TranslationKeyProvider, Staler, StaleMarker, StaleInfo, UnmarshableResource,
//          resourceTypesHolder
//    L45-53: NewResourceError(err error, data any) ResourceError
//    L61-63: (e *resourceError) Data() any
// EX L247-252: StaleVersion(os any) uint32
//    L255-263: StaleVersionSum(oss ...any) uint32
//    L266-275: MarkStale(os ...any)
//    L288-290: (r resourceTypesHolder) MediaType() media.Type
//    L292-294: (r resourceTypesHolder) ResourceType() string
// EX L296-298: NewResourceTypesProvider(mediaType media.Type, resourceType string) ResourceTypesProvider
//    L301-306: NameNormalizedOrName(r Resource) string
// ---------------------------------------------------------------------------
