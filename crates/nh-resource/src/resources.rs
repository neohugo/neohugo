//! Port of `resources/resource/resources.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `resource.Resources` methods (`Get`, `GetMatch`, `Match`, `ByType`, ...). `Get` compares
//! with `strings.EqualFold` on `Name()` first, then `NameNormalized()`; `./` prefix = relative.

use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_common::object::{GoResult, NamedMethods};

use crate::resourcetypes::{Resource, Resources};

// Go: resources/resource/resources.go:Get
pub fn get(r: &Resources, name: &Value) -> Option<Arc<dyn Resource>> {
    todo!()
}

// Go: resources/resource/resources.go:GetMatch
pub fn get_match(r: &Resources, pattern: &Value) -> Option<Arc<dyn Resource>> {
    todo!()
}

// Go: resources/resource/resources.go:Match
pub fn match_(r: &Resources, pattern: &Value) -> Resources {
    todo!()
}

// Go: resources/resource/resources.go:ByType
pub fn by_type(r: &Resources, typ: &Value) -> Resources {
    todo!()
}

/// Methods of the named slice type `resource.Resources`: `Get`, `GetMatch`, `Match`, `ByType`,
/// `Mount` (unused), `ToResources`, `MergeByLanguage`, `MergeByLanguageInterface`.
pub fn resources_has_method(name: &str) -> bool {
    matches!(name, "Get" | "GetMatch" | "Match" | "ByType" | "MergeByLanguage" | "MergeByLanguageInterface")
}

/// Dispatch for [`nh_common::object::NamedTypeRegistry`]. Needs Value -> Resource conversion for
/// every element (pages included): nh-hugolib registers it with a converter that knows pages.
pub fn resources_call_method(ctx: HostCtx<'_>, recv: &Value, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource/resources.go (408 lines; 1/13 funcs executed)
//   types: Resources, ResourcesProvider, ResourcesConverter, translatedResource, Source, ResourceGetter,
//          IsProbablySameResourceGetter, StaleInfoResourceGetter, resourceGetterFunc, ResourceFinder,
//          multiResourceGetter, cachedResourceGetter
//    L39-74: (r Resources) Mount(base, target string) ResourceGetter
//    L89-102: (r Resources) ByType(typ any) Resources
// EX L106-148: (r Resources) Get(name any) Resource
//    L152-179: (r Resources) GetMatch(pattern any) Resource
//    L190-218: (r Resources) Match(pattern any) Resources
//    L225-242: (r Resources) MergeByLanguage(r2 Resources) Resources
//    L247-253: (r Resources) MergeByLanguageInterface(in any) (any, error)
//    L281-283: (f resourceGetterFunc) Get(name any) Resource
//    L324-336: NewCachedResourceGetter(os ...any) *cachedResourceGetter
//    L340-347: (m multiResourceGetter) Get(name any) Resource
//    L359-369: (c *cachedResourceGetter) Get(name any) Resource
//    L371-382: (c *cachedResourceGetter) IsProbablySameResourceGetter(other ResourceGetter) bool
//    L384-408: unwrapResourceGetter(v any) (ResourceGetter, bool)
// ---------------------------------------------------------------------------
