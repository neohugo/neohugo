//! Port of `resources/resource/resourcetypes.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
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
use std::sync::{Arc, OnceLock, RwLock};

use go_value::{GoString, HostCtx, Map, Object, Value};
use nh_common::Result;
use nh_common::hugio::ReadSeekCloser;
use nh_common::object::GoResult;
use nh_langs::language::Language;
use nh_media::media::media_type::MediaType;

/// The Go type string of the `resource.Resource` interface.
pub const RESOURCE_TYPE: &str = "resource.Resource";
/// The Go type string of the `resource.Resources` named slice.
pub const RESOURCES_TYPE: &str = "resource.Resources";

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
    fn content(&self, _ctx: HostCtx<'_>) -> Option<Result<Value>> {
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
    /// Go: `StaleInfo.StaleVersion()`; `None` when the resource does not implement `StaleInfo`.
    fn stale_version(&self) -> Option<u32> {
        None
    }
    /// Go: `StaleMarker.MarkStale()`; false when the resource does not implement `StaleMarker`.
    fn mark_stale(&self) -> bool {
        false
    }

    // ---- template API (replaces reflection) ----
    /// Go type string, e.g. `*resources.resourceAdapter`.
    fn tpl_type_name(&self) -> Cow<'_, str>;
    fn tpl_has_method(&self, name: &str) -> bool;
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>>;
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
        go_value::SliceType::Named(Arc::from(RESOURCES_TYPE)),
        rs.iter().map(|r| r.clone().to_value()).collect(),
    )
}

/// A non-page resource from a template value (pages: `nh_page::page::resource_from_value`).
pub fn resource_ref_from_value(v: &Value) -> Option<Arc<dyn Resource>> {
    v.downcast::<ResourceRef>().map(|r| r.0.clone())
}

/// The function that turns ANY template value holding a `resource.Resource` (pages included)
/// into a resource. nh-page registers `nh_page::page::resource_from_value` (it knows `PageRef`)
/// in `nh_page::init`; until then only [`ResourceRef`]s are recognised.
pub type ValueToResource = fn(&Value) -> Option<Arc<dyn Resource>>;

fn value_to_resource_slot() -> &'static RwLock<ValueToResource> {
    static SLOT: OnceLock<RwLock<ValueToResource>> = OnceLock::new();
    SLOT.get_or_init(|| RwLock::new(resource_ref_from_value))
}

/// Installs the converter used by [`resource_from_value_any`].
pub fn register_value_to_resource(f: ValueToResource) {
    *value_to_resource_slot()
        .write()
        .unwrap_or_else(|e| e.into_inner()) = f;
}

/// Any resource (page or not) from a template value, through the registered converter.
pub fn resource_from_value_any(v: &Value) -> Option<Arc<dyn Resource>> {
    let f = *value_to_resource_slot()
        .read()
        .unwrap_or_else(|e| e.into_inner());
    f(v)
}

/// Go type strings of the concrete types that implement `resource.Resource` (non-page resources
/// and pages), for `hreflect::register_interface`.
pub fn implements_resource(type_name: &str) -> bool {
    matches!(
        type_name,
        "resource.Resource"
            | "page.Page"
            | "*resources.genericResource"
            | "*resources.resourceAdapter"
            | "*resources.imageResource"
            | "*postpub.PostPublishResource"
            | "*page.nopPage"
            | "*hugolib.pageState"
            | "hugolib.pageWithWeight0"
            | "*hugolib.pageWithOrdinal"
            | "*hugolib.pageForShortcode"
            | "*hugolib.pageForRenderHooks"
    )
}

/// Registers `resource.Resource` with `hreflect::register_interface` and `resource.Resources`
/// as a named slice of it (idempotent; `nh_page::init` calls it too).
pub fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        nh_common::hreflect::register_interface(RESOURCE_TYPE, implements_resource);
        nh_common::hreflect::register_named_elem(RESOURCES_TYPE, RESOURCE_TYPE);
    });
}

/// Go: `resource.ResourceError` (the `*resourceError` created by `NewResourceError`).
#[derive(Clone, Debug)]
pub struct ResourceError {
    pub msg: String,
    pub data: Option<Value>,
}

impl ResourceError {
    /// Go: `NewResourceError(err, data)` — a nil `data` becomes an empty `map[string]any`.
    // Go: resources/resource/resourcetypes.go:NewResourceError
    pub fn new(err: &nh_common::herrors::Error, data: Option<Value>) -> ResourceError {
        let data = match data {
            None | Some(Value::Invalid) => Value::map(Map::new(go_value::MapType::StringAny)),
            Some(d) => d,
        };
        ResourceError {
            msg: err.message().to_string(),
            data: Some(data),
        }
    }

    /// Go: `(*resourceError).Data()`.
    // Go: resources/resource/resourcetypes.go:Data
    pub fn data(&self) -> Value {
        self.data.clone().unwrap_or(Value::Invalid)
    }
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

/// Go: `resource.StaleVersion(os)` — 0 when `r` is not a `StaleInfo`.
// Go: resources/resource/resourcetypes.go:StaleVersion
pub fn stale_version(r: &dyn Resource) -> u32 {
    r.stale_version().unwrap_or(0)
}

/// Go: `resource.StaleVersionSum(oss...)` (uint32 arithmetic wraps like Go).
// Go: resources/resource/resourcetypes.go:StaleVersionSum
pub fn stale_version_sum(oss: &[&dyn Resource]) -> u32 {
    let mut version: u32 = 0;
    for o in oss {
        if let Some(v) = o.stale_version()
            && v > 0
        {
            version = version.wrapping_add(v);
        }
    }
    version
}

/// Go: `resource.MarkStale(os...)` (nil entries, `None`, are skipped).
// Go: resources/resource/resourcetypes.go:MarkStale
pub fn mark_stale(os: &[Option<&dyn Resource>]) {
    for o in os.iter().flatten() {
        o.mark_stale();
    }
}

/// Go: `resource.resourceTypesHolder` (returned by `NewResourceTypesProvider`).
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceTypesHolder {
    media_type: MediaType,
    resource_type: String,
}

impl ResourceTypesHolder {
    // Go: resources/resource/resourcetypes.go:MediaType
    pub fn media_type(&self) -> &MediaType {
        &self.media_type
    }

    // Go: resources/resource/resourcetypes.go:ResourceType
    pub fn resource_type(&self) -> &str {
        &self.resource_type
    }
}

/// Go: `resource.NewResourceTypesProvider(mediaType, resourceType)`.
// Go: resources/resource/resourcetypes.go:NewResourceTypesProvider
pub fn new_resource_types_provider(
    media_type: MediaType,
    resource_type: &str,
) -> ResourceTypesHolder {
    ResourceTypesHolder {
        media_type,
        resource_type: resource_type.to_string(),
    }
}

/// Go: `resource.NameNormalizedOrName(r)`.
// Go: resources/resource/resourcetypes.go:NameNormalizedOrName
pub fn name_normalized_or_name(r: &dyn Resource) -> String {
    match r.name_normalized() {
        Some(n) => n,
        None => r.name(),
    }
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
// OK L45-53: NewResourceError(err error, data any) ResourceError
// OK L61-63: (e *resourceError) Data() any
// OK L247-252: StaleVersion(os any) uint32
// OK L255-263: StaleVersionSum(oss ...any) uint32
// OK L266-275: MarkStale(os ...any)
// OK L288-290: (r resourceTypesHolder) MediaType() media.Type
// OK L292-294: (r resourceTypesHolder) ResourceType() string
// OK L296-298: NewResourceTypesProvider(mediaType media.Type, resourceType string) ResourceTypesProvider
// OK L301-306: NameNormalizedOrName(r Resource) string
// ---------------------------------------------------------------------------
