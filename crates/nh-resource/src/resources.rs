//! Port of `resources/resource/resources.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
//! Go `resource.Resources` methods (`Get`, `GetMatch`, `Match`, `ByType`, ...). `Get` compares
//! with `strings.EqualFold` on `Name()` first, then `NameNormalized()`; `./` prefix = relative.
//!
//! Go distinguishes a nil `Resources` from an empty one only in `Get` (a nil receiver returns nil
//! before converting the name) and in results (`Match`/`ByType` return a nil slice when nothing
//! matches). The functions here take the receiver as `Option<&[..]>` where the difference is
//! visible (`None` = Go's nil) and return `Option<Resources>` (`None` = Go's nil slice).
//! The Go functions panic on a bad argument (a name that cast cannot convert, a bad glob); the
//! `try_*` forms return that panic value as an error and the plain forms panic like Go.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use go_unicode::strings;
use go_value::{GoString, HostCtx, SliceType, Value};
use nh_common::Result;
use nh_common::cast::caste;
use nh_common::herrors::Error;
use nh_common::object::{GoResult, NamedMethods};
use nh_common::paths::path as paths;

use crate::resourcetypes::{
    RESOURCE_TYPE, RESOURCES_TYPE, Resource, Resources, resource_from_value_any, resources_to_value,
};

fn cast_to_string(v: &Value) -> Result<GoString> {
    caste::to_string_e(v)
}

fn lossy(s: &GoString) -> String {
    // Resource names and the glob/path helpers are UTF-8 (nh-common deviation 21); a name that
    // is not valid UTF-8 can never equal one, and the glob matcher sees the same bytes.
    String::from_utf8_lossy(s.as_bytes()).into_owned()
}

/// Go: `Resources.Mount(base, target)` — a `ResourceGetter` resolving names below `target`
/// against the resources' names below `base`.
// Go: resources/resource/resources.go:Mount
pub fn mount(r: &[Arc<dyn Resource>], base: &str, target: &str) -> MountedResources {
    MountedResources {
        r: r.to_vec(),
        base: base.to_string(),
        target: target.to_string(),
    }
}

/// The `resourceGetterFunc` returned by [`mount`].
#[derive(Clone)]
pub struct MountedResources {
    r: Resources,
    base: String,
    target: String,
}

impl ResourceGetter for MountedResources {
    // Go: resources/resource/resources.go:Mount (the returned func)
    fn try_get(&self, namev: &Value) -> Result<Option<Arc<dyn Resource>>> {
        let mut name1 = lossy(&cast_to_string(namev)?);

        let is_target_abs = self.target.starts_with('/');

        if !self.target.is_empty() {
            name1 = name1
                .strip_prefix(self.target.as_str())
                .unwrap_or(&name1)
                .to_string();
            if !is_target_abs {
                name1 = paths::trim_leading(&name1);
            }
        }

        if !self.base.is_empty() && is_target_abs {
            name1 = go_path::path::join(&[self.base.as_str(), name1.as_str()]);
        }

        for res in &self.r {
            let mut name2 = res.name();

            if !self.base.is_empty() && !is_target_abs {
                name2 =
                    paths::trim_leading(name2.strip_prefix(self.base.as_str()).unwrap_or(&name2));
            }

            if strings::equal_fold(name1.as_bytes(), name2.as_bytes()) {
                return Ok(Some(res.clone()));
            }
        }

        Ok(None)
    }
}

/// Go: `Resources.ByType(typ)`; `None` is Go's nil slice (nothing matched).
// Go: resources/resource/resources.go:ByType
pub fn try_by_type(r: &[Arc<dyn Resource>], typ: &Value) -> Result<Option<Resources>> {
    let tpstr = cast_to_string(typ)?;
    let mut filtered: Option<Resources> = None;

    for resource in r {
        if resource.resource_type().as_bytes() == tpstr.as_bytes() {
            filtered.get_or_insert_with(Vec::new).push(resource.clone());
        }
    }
    Ok(filtered)
}

/// [`try_by_type`], panicking on a bad argument like Go (an empty result for Go's nil).
// Go: resources/resource/resources.go:ByType
pub fn by_type(r: &Resources, typ: &Value) -> Resources {
    match try_by_type(r, typ) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => panic!("{}", e.message()),
    }
}

/// Go: `Resources.Get(name)`; `r` = `None` is Go's nil receiver.
// Go: resources/resource/resources.go:Get
pub fn try_get(r: Option<&[Arc<dyn Resource>]>, name: &Value) -> Result<Option<Arc<dyn Resource>>> {
    let Some(r) = r else {
        return Ok(None);
    };
    let namestr = lossy(&cast_to_string(name)?);

    let is_dot_current = namestr.starts_with("./");
    let namestr = if is_dot_current {
        namestr.strip_prefix("./").unwrap_or(&namestr).to_string()
    } else {
        paths::add_leading_slash(&namestr)
    };

    let check = |name: &str| -> bool {
        let name = if !is_dot_current {
            paths::add_leading_slash(name)
        } else {
            name.to_string()
        };
        strings::equal_fold(namestr.as_bytes(), name.as_bytes())
    };

    // First check the Name.
    // Note that this can be modified by the user in the front matter,
    // also, it does not contain any language code.
    for resource in r {
        if check(&resource.name()) {
            return Ok(Some(resource.clone()));
        }
    }

    // Finally, check the normalized name.
    for resource in r {
        if let Some(nn) = resource.name_normalized()
            && check(&nn)
        {
            return Ok(Some(resource.clone()));
        }
    }

    Ok(None)
}

/// [`try_get`] on a non-nil receiver, panicking on a bad argument like Go.
// Go: resources/resource/resources.go:Get
pub fn get(r: &Resources, name: &Value) -> Option<Arc<dyn Resource>> {
    match try_get(Some(r), name) {
        Ok(v) => v,
        Err(e) => panic!("{}", e.message()),
    }
}

fn glob_for(pattern: &Value) -> Result<nh_common::glob::glob::Glob> {
    let patternstr = lossy(&cast_to_string(pattern)?);
    nh_common::glob::glob::get_glob(&paths::add_leading_slash(&patternstr))
}

/// Go: `Resources.GetMatch(pattern)`.
// Go: resources/resource/resources.go:GetMatch
pub fn try_get_match(
    r: &[Arc<dyn Resource>],
    pattern: &Value,
) -> Result<Option<Arc<dyn Resource>>> {
    let g = glob_for(pattern)?;

    for resource in r {
        if g.matches(&paths::add_leading_slash(&resource.name())) {
            return Ok(Some(resource.clone()));
        }
    }

    // Finally, check the normalized name.
    for resource in r {
        if let Some(nn) = resource.name_normalized()
            && g.matches(&paths::add_leading_slash(&nn))
        {
            return Ok(Some(resource.clone()));
        }
    }

    Ok(None)
}

/// [`try_get_match`], panicking on a bad argument like Go.
// Go: resources/resource/resources.go:GetMatch
pub fn get_match(r: &Resources, pattern: &Value) -> Option<Arc<dyn Resource>> {
    match try_get_match(r, pattern) {
        Ok(v) => v,
        Err(e) => panic!("{}", e.message()),
    }
}

/// Go: `Resources.Match(pattern)`; `None` is Go's nil slice.
// Go: resources/resource/resources.go:Match
pub fn try_match(r: &[Arc<dyn Resource>], pattern: &Value) -> Result<Option<Resources>> {
    let g = glob_for(pattern)?;

    let mut matches: Option<Resources> = None;
    for resource in r {
        if g.matches(&paths::add_leading_slash(&resource.name())) {
            matches.get_or_insert_with(Vec::new).push(resource.clone());
        }
    }
    if matches.is_none() {
        // 	Fall back to the normalized name.
        for resource in r {
            if let Some(nn) = resource.name_normalized()
                && g.matches(&paths::add_leading_slash(&nn))
            {
                matches.get_or_insert_with(Vec::new).push(resource.clone());
            }
        }
    }
    Ok(matches)
}

/// [`try_match`], panicking on a bad argument like Go (an empty result for Go's nil).
// Go: resources/resource/resources.go:Match
pub fn match_(r: &Resources, pattern: &Value) -> Resources {
    match try_match(r, pattern) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => panic!("{}", e.message()),
    }
}

/// Go: `Resources.MergeByLanguage(r2)` — adds the resources of `r2` whose translation key is
/// not in `r` (resources without a translation key are never added from `r2`).
// Go: resources/resource/resources.go:MergeByLanguage
pub fn merge_by_language(r: &[Arc<dyn Resource>], r2: &[Arc<dyn Resource>]) -> Resources {
    let mut result: Resources = r.to_vec();
    let mut m: BTreeMap<String, bool> = BTreeMap::new();
    for rr in r {
        if let Some(k) = rr.translation_key() {
            m.insert(k, true);
        }
    }

    for rr in r2 {
        if let Some(k) = rr.translation_key()
            && !m.contains_key(&k)
        {
            result.push(rr.clone());
        }
    }
    result
}

/// Go: `Resources.MergeByLanguageInterface(in)` — `in` must be a `resource.Resources`.
// Go: resources/resource/resources.go:MergeByLanguageInterface
pub fn merge_by_language_interface(r: &[Arc<dyn Resource>], input: &Value) -> Result<Value> {
    let r2 = match input {
        Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == RESOURCES_TYPE) => {
            values_to_resources(&l.items)
        }
        Value::TypedNil(t) if &**t == RESOURCES_TYPE => Some(Vec::new()),
        _ => None,
    };
    match r2 {
        Some(r2) => Ok(resources_to_value(&merge_by_language(r, &r2))),
        None => Err(Error::new(format!(
            "{} cannot be merged by language",
            go_type_name(input)
        ))),
    }
}

fn go_type_name(v: &Value) -> String {
    match v {
        Value::Invalid => "<nil>".to_string(),
        _ => v.go_type_name().into_owned(),
    }
}

/// The elements of a `resource.Resources` template value as resources (every element must be
/// a resource; pages through the converter registered by nh-page).
pub fn values_to_resources(items: &[Value]) -> Option<Resources> {
    items.iter().map(resource_from_value_any).collect()
}

/// A `resource.Resources` template value (`List` or `TypedNil`) as `(is_nil, resources)`.
pub fn resources_from_value(v: &Value) -> Option<(bool, Resources)> {
    match v {
        Value::List(l) => values_to_resources(&l.items).map(|r| (false, r)),
        Value::TypedNil(_) => Some((true, Vec::new())),
        _ => None,
    }
}

fn nil_resource() -> Value {
    Value::TypedNil(Arc::from(RESOURCE_TYPE))
}

fn resource_value(r: Option<Arc<dyn Resource>>) -> Value {
    match r {
        Some(r) => r.to_value(),
        None => nil_resource(),
    }
}

fn resources_value(r: Option<Resources>) -> Value {
    match r {
        Some(r) => resources_to_value(&r),
        None => Value::TypedNil(Arc::from(RESOURCES_TYPE)),
    }
}

/// Methods of the named slice type `resource.Resources`: `Get`, `GetMatch`, `Match`, `ByType`,
/// `Mount`, `MergeByLanguage`, `MergeByLanguageInterface`.
pub fn resources_has_method(name: &str) -> bool {
    matches!(
        name,
        "Get"
            | "GetMatch"
            | "Match"
            | "ByType"
            | "Mount"
            | "MergeByLanguage"
            | "MergeByLanguageInterface"
    )
}

/// Dispatch for [`nh_common::object::NamedTypeRegistry`]. Elements are converted with
/// [`resource_from_value_any`] (nh-page registers the converter that knows pages).
pub fn resources_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    args: &[Value],
) -> Option<GoResult<Value>> {
    if !resources_has_method(name) {
        return None;
    }
    let Some((is_nil, r)) = resources_from_value(recv) else {
        return Some(Err(go_value::Error::new(format!(
            "{} is not a resource.Resources",
            go_type_name(recv)
        ))));
    };
    use nh_common::object::args;
    let res: GoResult<Value> = (|| match name {
        "Get" => {
            args::exactly(args, 1, name)?;
            let rr = if is_nil { None } else { Some(&r[..]) };
            Ok(resource_value(try_get(rr, &args[0])?))
        }
        "GetMatch" => {
            args::exactly(args, 1, name)?;
            Ok(resource_value(try_get_match(&r, &args[0])?))
        }
        "Match" => {
            args::exactly(args, 1, name)?;
            Ok(resources_value(try_match(&r, &args[0])?))
        }
        "ByType" => {
            args::exactly(args, 1, name)?;
            Ok(resources_value(try_by_type(&r, &args[0])?))
        }
        "Mount" => {
            args::exactly(args, 2, name)?;
            let base = args::string(args, 0)?;
            let target = args::string(args, 1)?;
            Ok(Value::object(ResourceGetterObject(Arc::new(mount(
                &r,
                &String::from_utf8_lossy(base.as_bytes()),
                &String::from_utf8_lossy(target.as_bytes()),
            )))))
        }
        "MergeByLanguage" => {
            args::exactly(args, 1, name)?;
            let Some((_, r2)) = resources_from_value(&args[0]) else {
                return Err(go_value::Error::new(format!(
                    "wrong type for value; expected resource.Resources; got {}",
                    go_type_name(&args[0])
                )));
            };
            Ok(resources_to_value(&merge_by_language(&r, &r2)))
        }
        "MergeByLanguageInterface" => {
            // Two results (any, error): callable from templates.
            args::exactly(args, 1, name)?;
            Ok(merge_by_language_interface(&r, &args[0])?)
        }
        _ => unreachable!(),
    })();
    Some(res)
}

pub const RESOURCES_METHODS: NamedMethods = NamedMethods {
    has_method: resources_has_method,
    call: resources_call_method,
};

/// Go: `resource.ResourceGetter`.
pub trait ResourceGetter: Send + Sync {
    /// Go: `Get(name)`; the error is Go's panic value (a name cast cannot convert).
    fn try_get(&self, name: &Value) -> Result<Option<Arc<dyn Resource>>>;
}

impl ResourceGetter for Resources {
    fn try_get(&self, name: &Value) -> Result<Option<Arc<dyn Resource>>> {
        try_get(Some(&self[..]), name)
    }
}

/// A `ResourceGetter` as a template value (Go `resource.resourceGetterFunc` from `Mount`).
#[derive(Clone)]
pub struct ResourceGetterObject(pub Arc<dyn ResourceGetter>);

impl go_value::Object for ResourceGetterObject {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("resource.resourceGetterFunc")
    }
    fn has_method(&self, name: &str) -> bool {
        name == "Get"
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        if name != "Get" {
            return None;
        }
        Some((|| {
            nh_common::object::args::exactly(args, 1, name)?;
            Ok(resource_value(self.0.try_get(&args[0])?))
        })())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `multiResourceGetter` — the first getter that finds the name wins.
#[derive(Clone, Default)]
pub struct MultiResourceGetter(pub Vec<Arc<dyn ResourceGetter>>);

impl ResourceGetter for MultiResourceGetter {
    // Go: resources/resource/resources.go:Get
    fn try_get(&self, name: &Value) -> Result<Option<Arc<dyn Resource>>> {
        for g in &self.0 {
            if let Some(res) = g.try_get(name)? {
                return Ok(Some(res));
            }
        }
        Ok(None)
    }
}

/// Go: `cachedResourceGetter` (`NewCachedResourceGetter`): results are cached by the name's
/// string form (first computed value wins, computed without the lock; HUGO_LAYER.md §4.8).
pub struct CachedResourceGetter {
    cache: Mutex<BTreeMap<GoString, Option<Arc<dyn Resource>>>>,
    /// Insertion order of the cache keys (Go's `maps.Cache.ForEeach` order).
    order: Mutex<Vec<GoString>>,
    delegate: MultiResourceGetter,
}

/// Go: `resource.NewCachedResourceGetter(os...)` — the objects that are (or unwrap to)
/// resource getters, merged with the first match winning.
// Go: resources/resource/resources.go:NewCachedResourceGetter
pub fn new_cached_resource_getter(os: &[Value]) -> CachedResourceGetter {
    let mut getters = MultiResourceGetter::default();
    for o in os {
        if let Some(g) = unwrap_resource_getter(o) {
            getters.0.push(g);
        }
    }

    CachedResourceGetter {
        cache: Mutex::new(BTreeMap::new()),
        order: Mutex::new(Vec::new()),
        delegate: getters,
    }
}

impl ResourceGetter for CachedResourceGetter {
    // Go: resources/resource/resources.go:Get
    fn try_get(&self, name: &Value) -> Result<Option<Arc<dyn Resource>>> {
        let namestr = cast_to_string(name)?;
        if let Some(v) = self
            .cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&namestr)
        {
            return Ok(v.clone());
        }
        // Go: `v := c.delegate.Get(name)` inside GetOrCreate (a panic propagates).
        let v = self.delegate.try_get(name)?;
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = cache.get(&namestr) {
            return Ok(existing.clone());
        }
        cache.insert(namestr.clone(), v.clone());
        self.order
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(namestr);
        Ok(v)
    }
}

impl CachedResourceGetter {
    /// Go: `IsProbablySameResourceGetter(other)` — every cached name resolves to the same
    /// resource (pointer identity) in `other`.
    // Go: resources/resource/resources.go:IsProbablySameResourceGetter
    pub fn is_probably_same_resource_getter(&self, other: &dyn ResourceGetter) -> bool {
        let order = self.order.lock().unwrap_or_else(|e| e.into_inner()).clone();
        for k in order {
            let v = self
                .cache
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&k)
                .cloned()
                .flatten();
            let o = other.try_get(&Value::String(k)).ok().flatten();
            let same = match (&v, &o) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    std::ptr::eq(Arc::as_ptr(a) as *const (), Arc::as_ptr(b) as *const ())
                }
                _ => false,
            };
            if !same {
                return false;
            }
        }
        true
    }
}

struct ObjectGetter(Arc<dyn go_value::Object>);

impl ResourceGetter for ObjectGetter {
    fn try_get(&self, name: &Value) -> Result<Option<Arc<dyn Resource>>> {
        match self.0.call_method(&(), "Get", std::slice::from_ref(name)) {
            Some(Ok(v)) => Ok(resource_from_value_any(&v)),
            Some(Err(e)) => Err(Error::new(e.message())),
            None => Ok(None),
        }
    }
}

/// Go: `unwrapResourceGetter(v)` over template values: a getter object (a `Get` method), a
/// `ResourcesProvider` (a `Resources` method), a `resource.Resources`, or a slice of those.
// Go: resources/resource/resources.go:unwrapResourceGetter
pub fn unwrap_resource_getter(v: &Value) -> Option<Arc<dyn ResourceGetter>> {
    match v {
        Value::Invalid => None,
        Value::Object(o) if o.as_any().downcast_ref::<ResourceGetterObject>().is_some() => o
            .as_any()
            .downcast_ref::<ResourceGetterObject>()
            .map(|g| g.0.clone()),
        Value::Object(o) if o.has_method("Get") => Some(Arc::new(ObjectGetter(o.clone()))),
        Value::Object(o) if o.has_method("Resources") => {
            match o.call_method(&(), "Resources", &[]) {
                Some(Ok(rv)) => resources_from_value(&rv).map(|(_, r)| {
                    let g: Arc<dyn ResourceGetter> = Arc::new(r);
                    g
                }),
                _ => None,
            }
        }
        Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == RESOURCES_TYPE) => {
            values_to_resources(&l.items).map(|r| {
                let g: Arc<dyn ResourceGetter> = Arc::new(r);
                g
            })
        }
        _ => {
            let vvv = nh_common::hreflect::to_slice_any(v)?;
            let mut getters = MultiResourceGetter::default();
            for vv in &vvv {
                if let Some(g) = unwrap_resource_getter(vv) {
                    getters.0.push(g);
                }
            }
            if getters.0.is_empty() {
                None
            } else {
                Some(Arc::new(getters))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource/resources.go (408 lines; 1/13 funcs executed)
//   types: Resources, ResourcesProvider, ResourcesConverter, translatedResource, Source, ResourceGetter,
//          IsProbablySameResourceGetter, StaleInfoResourceGetter, resourceGetterFunc, ResourceFinder,
//          multiResourceGetter, cachedResourceGetter
// OK L39-74: (r Resources) Mount(base, target string) ResourceGetter
// OK L89-102: (r Resources) ByType(typ any) Resources
// OK L106-148: (r Resources) Get(name any) Resource
// OK L152-179: (r Resources) GetMatch(pattern any) Resource
// OK L190-218: (r Resources) Match(pattern any) Resources
// OK L225-242: (r Resources) MergeByLanguage(r2 Resources) Resources
// OK L247-253: (r Resources) MergeByLanguageInterface(in any) (any, error)
// OK L281-283: (f resourceGetterFunc) Get(name any) Resource
// OK L324-336: NewCachedResourceGetter(os ...any) *cachedResourceGetter
// OK L340-347: (m multiResourceGetter) Get(name any) Resource
// OK L359-369: (c *cachedResourceGetter) Get(name any) Resource
// OK L371-382: (c *cachedResourceGetter) IsProbablySameResourceGetter(other ResourceGetter) bool
// OK L384-408: unwrapResourceGetter(v any) (ResourceGetter, bool)
// ---------------------------------------------------------------------------
