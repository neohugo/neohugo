//! Port of `common/maps/params.go`.
//!
//! Owner: Wave B task T01 (common-values).

//! `maps.Params` is represented as a [`go_value::Map`] with [`MapType::Params`]: keys are stored
//! lower-cased (Go `strings.ToLower`, via go-unicode), nested maps are Params too, slices are NOT
//! descended into. Template lookups on Params are case-insensitive ([`params_get`]); lookups on
//! every other map type are exact.
//!
//! Go maps are reference types: `PrepareParams`, `SetParams` and `MergeParams` mutate nested
//! maps in place, and a nested map inserted into two parents is shared. The value model's maps
//! are `Arc<Map>` values; nested maps are updated copy-on-write (`Arc::make_mut`), so a mutation
//! is never visible through another parent that shares the same nested map (see PORTING.md).

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Kind, Map, MapType, Object, Value};

use crate::herrors::Result;
use crate::object::{GoResult, NamedMethods, args, bad_results_error};

/// Go: `maps.MergeStrategyKey`.
pub const MERGE_STRATEGY_KEY: &str = "_merge";

/// Go: `maps.ParamsMergeStrategy` (a named string type) for the three valid values. As a
/// template value (stored under `_merge` by [`prepare_params`]) it is an object of type
/// `maps.ParamsMergeStrategy` whose `underlying` value is the string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamsMergeStrategy {
    /// "none": do not merge.
    None,
    /// "shallow": only add new keys.
    Shallow,
    /// "deep": add new keys, merge existing.
    Deep,
}

impl ParamsMergeStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            ParamsMergeStrategy::None => "none",
            ParamsMergeStrategy::Shallow => "shallow",
            ParamsMergeStrategy::Deep => "deep",
        }
    }

    /// The strategy named `s`, if it is one of the three valid values.
    pub fn from_str_opt(s: &[u8]) -> Option<Self> {
        match s {
            b"none" => Some(ParamsMergeStrategy::None),
            b"shallow" => Some(ParamsMergeStrategy::Shallow),
            b"deep" => Some(ParamsMergeStrategy::Deep),
            _ => None,
        }
    }

    /// The template value (`maps.ParamsMergeStrategy` object).
    pub fn value(self) -> Value {
        Value::object(self)
    }
}

impl Object for ParamsMergeStrategy {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("maps.ParamsMergeStrategy")
    }
    fn kind(&self) -> Kind {
        // A named string type: `underlying` gives the string kind.
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<GoResult<Value>> {
        None
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::string(self.as_str()))
    }
    fn marshal_json(&self) -> Option<GoResult<Vec<u8>>> {
        // encoding/json encodes a string kind as a JSON string; the three values are plain ASCII.
        Some(Ok(format!("\"{}\"", self.as_str()).into_bytes()))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `maps.KeyParams`.
#[derive(Clone, Debug)]
pub struct KeyParams {
    pub key: GoString,
    pub params: Map,
}

/// An empty `maps.Params`.
pub fn new_params() -> Map {
    Map::new(MapType::Params)
}

/// Go `strings.ToLower` over bytes.
fn to_lower(s: &[u8]) -> GoString {
    GoString::from(go_unicode::strings::to_lower(s).into_owned())
}

/// Whether `v` is Go's nil `any` (an untyped nil; a nil interface stored in a map is one).
fn is_nil_any(v: &Value) -> bool {
    crate::hreflect::type_of(v).is_none()
}

/// A `Params` or `map[string]interface {}` value (the two `getNested` descends into): the shared
/// map, or `None` for a nil map of those types (lookups in a nil Go map find nothing).
fn as_string_any_map(v: &Value) -> Option<Option<&Arc<Map>>> {
    match v {
        Value::Map(m) if matches!(m.ty, MapType::Params | MapType::StringAny) => Some(Some(m)),
        Value::TypedNil(t) if matches!(&**t, "maps.Params" | "map[string]interface {}") => {
            Some(None)
        }
        _ => None,
    }
}

// Go: common/maps/params.go:GetNested
/// GetNested does a lower case and nested search in this map. It returns `Invalid` (nil) if
/// none found.
pub fn get_nested(p: &Map, indices: &[&[u8]]) -> Value {
    get_nested_in(Some(p), indices).0
}

// Go: common/maps/params.go:getNested
/// Returns (value, key, owner): the owner is the map that holds (or would hold) the last key,
/// `None` for no owner or a nil owner map (Go returns a nil map in both cases).
fn get_nested_in<'a>(m: Option<&'a Map>, indices: &[&[u8]]) -> (Value, GoString, Option<&'a Map>) {
    if indices.is_empty() {
        return (Value::Invalid, GoString::empty(), None);
    }

    let first = indices[0];
    let Some(v) = m.and_then(|m| m.get(&to_lower(first))) else {
        if indices.len() == 1 {
            return (Value::Invalid, GoString::from(first), m);
        }
        return (Value::Invalid, GoString::empty(), None);
    };

    if indices.len() == 1 {
        return (v.clone(), GoString::from(first), m);
    }

    match as_string_any_map(v) {
        Some(m2) => get_nested_in(m2.map(|a| &**a), &indices[1..]),
        None => (Value::Invalid, GoString::empty(), None),
    }
}

/// [`get_nested_in`] over a shared map, returning the owner as the shared nested map.
fn get_nested_arc(m: Option<&Arc<Map>>, indices: &[&[u8]]) -> (Value, GoString, Option<Arc<Map>>) {
    if indices.is_empty() {
        return (Value::Invalid, GoString::empty(), None);
    }

    let first = indices[0];
    let Some(v) = m.and_then(|m| m.get(&to_lower(first))) else {
        if indices.len() == 1 {
            return (Value::Invalid, GoString::from(first), m.cloned());
        }
        return (Value::Invalid, GoString::empty(), None);
    };

    if indices.len() == 1 {
        return (v.clone(), GoString::from(first), m.cloned());
    }

    match as_string_any_map(v) {
        Some(m2) => get_nested_arc(m2, &indices[1..]),
        None => (Value::Invalid, GoString::empty(), None),
    }
}

// Go: common/maps/params.go:GetNestedParam
/// GetNestedParam gets the first match of the keyStr in the candidates given. It first tries the
/// exact (lower-cased) key and then the nested value, splitting on `separator`, e.g.
/// "mymap.name". It assumes that all the maps given have lower cased keys.
pub fn get_nested_param(key: &[u8], separator: &[u8], candidates: &[&Map]) -> Result<Value> {
    let key = to_lower(key);

    // Try exact match first
    for m in candidates {
        if let Some(v) = m.get(&key) {
            return Ok(v.clone());
        }
    }

    let key_segments = go_unicode::strings::split(&key, separator);
    for m in candidates {
        let v = get_nested(m, &key_segments);
        if !is_nil_any(&v) {
            return Ok(v);
        }
    }

    Ok(Value::Invalid)
}

// Go: common/maps/params.go:GetNestedParamFn
/// GetNestedParamFn looks up the first segment of `key` with `lookup` and descends into
/// `Params`/`map[string]interface {}` values for the others. Returns (value, last key, owner map
/// of the last key); the owner is `None` for a one-segment key.
pub fn get_nested_param_fn(
    key: &[u8],
    separator: &[u8],
    lookup: impl Fn(&[u8]) -> Value,
) -> Result<(Value, GoString, Option<Arc<Map>>)> {
    let key_segments = go_unicode::strings::split(key, separator);
    if key_segments.is_empty() {
        return Ok((Value::Invalid, GoString::empty(), None));
    }

    let first = lookup(key_segments[0]);
    if is_nil_any(&first) {
        return Ok((Value::Invalid, GoString::empty(), None));
    }

    if key_segments.len() == 1 {
        return Ok((first, GoString::from(key_segments[0]), None));
    }

    if let Some(m) = as_string_any_map(&first) {
        return Ok(get_nested_arc(m, &key_segments[1..]));
    }

    Ok((Value::Invalid, GoString::empty(), None))
}

// Go: common/maps/params.go:IsZero
/// IsZero returns true if p is considered empty: no keys, or only the `_merge` key.
pub fn params_is_zero(p: &Map) -> bool {
    if p.is_empty() {
        return true;
    }

    if p.len() > 1 {
        return false;
    }

    p.entries
        .keys()
        .next()
        .is_some_and(|k| k.as_bytes() == MERGE_STRATEGY_KEY.as_bytes())
}

/// Template map lookup on Params (Go: `tplimpl.templateExecHelper.GetMapValue`): the key is
/// lower-cased; a present key holding nil yields `Value::Invalid`, the same as a missing key.
pub fn params_get(p: &Map, key: &[u8]) -> Value {
    match p.get(&to_lower(key)) {
        Some(v) if !is_nil_any(v) => v.clone(),
        _ => Value::Invalid,
    }
}

// Go: common/maps/params.go:toMergeStrategy
fn to_merge_strategy(v: &Value) -> ParamsMergeStrategy {
    let s = crate::cast::caste::to_string(v);
    ParamsMergeStrategy::from_str_opt(&s).unwrap_or(ParamsMergeStrategy::Deep)
}

/// The Params conversion of a nested map value, if PrepareParams retypes it (Go's
/// `map[interface{}]interface{}` case cannot occur in the value model).
fn retype_nested(v: &Value, prepare: fn(&mut Map)) -> Option<Value> {
    match v {
        Value::Map(m) if m.ty == MapType::StringAny => {
            // Go: `var p Params = v.(map[string]any)` (the same map, mutated in place).
            let mut p = (**m).clone();
            p.ty = MapType::Params;
            prepare(&mut p);
            Some(Value::map(p))
        }
        Value::Map(m) if m.ty == MapType::StringString => {
            let mut p = Map::with_entries(MapType::Params, m.entries.clone());
            prepare(&mut p);
            Some(Value::map(p))
        }
        // A nil map[string]any becomes a nil Params; a nil map[string]string an empty Params.
        Value::TypedNil(t) if &**t == "map[string]interface {}" => {
            Some(Value::TypedNil(Arc::from("maps.Params")))
        }
        Value::TypedNil(t) if &**t == "map[string]string" => Some(Value::map(new_params())),
        _ => None,
    }
}

// Go: common/maps/params.go:PrepareParams
/// PrepareParams
/// * makes all the keys in the given map lower cased and will do so recursively;
/// * converts any nested `map[string]interface {}` / `map[string]string` to Params (already
///   typed Params values are left as they are, and not descended into);
/// * converts any `_merge` value to a [`ParamsMergeStrategy`].
///
/// Go ranges over the map while deleting and inserting; keys are visited here in byte order
/// with their current values, which gives Go's result whenever it is deterministic (when several
/// keys differ only in case, Go keeps a random one of the non-lower-case ones; here the last in
/// byte order).
pub fn prepare_params(m: &mut Map) {
    let keys: Vec<GoString> = m.entries.keys().cloned().collect();
    for k in keys {
        let Some(v) = m.entries.get(&k).cloned() else {
            continue;
        };
        let l_key = to_lower(&k);
        let (v, retyped) = if l_key.as_bytes() == MERGE_STRATEGY_KEY.as_bytes() {
            (to_merge_strategy(&v).value(), true)
        } else {
            match retype_nested(&v, prepare_params) {
                Some(p) => (p, true),
                None => (v, false),
            }
        };

        if retyped || k != l_key {
            m.entries.remove(&k);
            m.entries.insert(l_key, v);
        }
    }
}

// Go: common/maps/params.go:PrepareParamsClone
/// PrepareParamsClone is like PrepareParams, but it does not modify the input.
pub fn prepare_params_clone(m: &Map) -> Map {
    let mut m2 = new_params();
    for (k, v) in &m.entries {
        let l_key = to_lower(k);
        let (v, retyped) = if l_key.as_bytes() == MERGE_STRATEGY_KEY.as_bytes() {
            (to_merge_strategy(v).value(), true)
        } else {
            match v {
                Value::Map(mm) if mm.ty == MapType::StringAny => {
                    let mut p = (**mm).clone();
                    p.ty = MapType::Params;
                    (Value::map(prepare_params_clone(&p)), true)
                }
                // Go: `PrepareParamsClone(nil)` returns an empty (non-nil) Params.
                Value::TypedNil(t) if &**t == "map[string]interface {}" => {
                    (Value::map(new_params()), true)
                }
                _ => match retype_nested(v, prepare_params) {
                    Some(p) => (p, true),
                    None => (v.clone(), false),
                },
            }
        };

        if retyped || *k != l_key {
            m2.entries.insert(l_key, v);
        } else {
            m2.entries.insert(k.clone(), v);
        }
    }
    m2
}

// Go: common/maps/maps.go:ToParamsAndPrepare
/// ToParamsAndPrepare converts in to Params and prepares it for use. If in is nil, an empty map
/// is returned. See PrepareParams.
pub fn to_params_and_prepare(v: &Value) -> Result<Map> {
    if crate::types::types::is_nil(v) {
        return Ok(new_params());
    }
    let mut m = super::maps::to_string_map_e(v)?;
    m.ty = MapType::Params;
    prepare_params(&mut m);
    Ok(m)
}

// Go: common/maps/maps.go:MustToParamsAndPrepare
/// MustToParamsAndPrepare calls ToParamsAndPrepare and panics if it fails.
pub fn must_to_params_and_prepare(v: &Value) -> Map {
    match to_params_and_prepare(v) {
        Ok(p) => p,
        Err(e) => panic!(
            "cannot convert {} to maps.Params: {}",
            v.go_type_name(),
            e.message()
        ),
    }
}

/// A nested Params value for in-place updates (Go mutates the shared nested map).
fn nested_params_mut(v: &mut Value) -> Option<&mut Map> {
    match v {
        Value::Map(m) if m.ty == MapType::Params => Some(Arc::make_mut(m)),
        _ => None,
    }
}

fn is_params(v: &Value) -> bool {
    matches!(v, Value::Map(m) if m.ty == MapType::Params)
}

// Go: common/maps/params.go:SetParams
/// SetParams overwrites values in dst with values in src for common or new keys. This is done
/// recursively (where both sides are Params).
pub fn set_params(dst: &mut Map, src: &Map) {
    for (k, v) in &src.entries {
        match dst.entries.get_mut(k) {
            None => {
                dst.entries.insert(k.clone(), v.clone());
            }
            Some(vv) => {
                if is_params(vv)
                    && let Value::Map(pv) = v
                    && pv.ty == MapType::Params
                {
                    if let Some(vvv) = nested_params_mut(vv) {
                        set_params(vvv, pv);
                    }
                    continue;
                }
                *vv = v.clone();
            }
        }
    }
}

// Go: common/maps/params.go:MergeParamsWithStrategy
/// MergeParamsWithStrategy transfers values from src to dst for new keys using the merge
/// strategy given ("" = the strategy encoded in dst, or shallow). This is done recursively.
pub fn merge_params_with_strategy(strategy: &str, dst: &mut Map, src: &Map) {
    merge(dst, strategy, src);
}

// Go: common/maps/params.go:MergeParams
/// MergeParams transfers values from src to dst for new keys using the merge encoded in dst.
/// This is done recursively.
pub fn merge_params(dst: &mut Map, src: &Map) {
    let (ms, _) = get_merge_strategy(dst);
    merge(dst, ms.as_str(), src);
}

// Go: common/maps/params.go:merge
fn merge(p: &mut Map, ps: &str, pp: &Map) {
    let (ns, found) = get_merge_strategy(p);

    let mut ms: &str = ns.as_str();
    if !found && !ps.is_empty() {
        ms = ps;
    }

    let mut no_update = ms == ParamsMergeStrategy::None.as_str();
    no_update = no_update || (!ps.is_empty() && ps == ParamsMergeStrategy::Shallow.as_str());

    // `ms` borrows from `ps` or is static; copy it before `p` is mutated.
    let ms = ms.to_string();
    for (k, v) in &pp.entries {
        if k.as_bytes() == MERGE_STRATEGY_KEY.as_bytes() {
            continue;
        }
        match p.entries.get_mut(k) {
            Some(vv) => {
                // Key matches, if both sides are Params, we try to merge.
                if let Value::Map(pv) = v
                    && pv.ty == MapType::Params
                    && let Some(vvv) = nested_params_mut(vv)
                {
                    merge(vvv, &ms, pv);
                }
            }
            None => {
                if !no_update {
                    p.entries.insert(k.clone(), v.clone());
                }
            }
        }
    }
}

// Go: common/maps/params.go:GetMergeStrategy
/// GetMergeStrategy returns the strategy stored under `_merge` (a [`ParamsMergeStrategy`]
/// value), or (shallow, false).
pub fn get_merge_strategy(p: &Map) -> (ParamsMergeStrategy, bool) {
    if let Some(v) = p.get(MERGE_STRATEGY_KEY.as_bytes())
        && let Some(s) = v.downcast::<ParamsMergeStrategy>()
    {
        return (*s, true);
    }
    (ParamsMergeStrategy::Shallow, false)
}

// Go: common/maps/params.go:SetMergeStrategy
/// SetMergeStrategy stores the strategy under `_merge`.
pub fn set_merge_strategy(p: &mut Map, s: ParamsMergeStrategy) {
    p.insert(MERGE_STRATEGY_KEY, s.value());
}

// Go: common/maps/params.go:DeleteMergeStrategy
/// DeleteMergeStrategy removes `_merge`; returns whether it was there.
pub fn delete_merge_strategy(p: &mut Map) -> bool {
    p.entries.remove(MERGE_STRATEGY_KEY.as_bytes()).is_some()
}

// Go: common/maps/params.go:CleanConfigStringMapString
/// CleanConfigStringMapString removes any processing instructions (`_merge`) from m; m is never
/// modified.
pub fn clean_config_string_map_string(m: &Map) -> Map {
    if m.is_empty() {
        return m.clone();
    }
    if m.get(MERGE_STRATEGY_KEY.as_bytes()).is_none() {
        return m.clone();
    }
    // Create a new map and copy all the keys except the merge strategy key.
    let mut m2 = Map::new(m.ty.clone());
    for (k, v) in &m.entries {
        if k.as_bytes() != MERGE_STRATEGY_KEY.as_bytes() {
            m2.entries.insert(k.clone(), v.clone());
        }
    }
    m2
}

// Go: common/maps/params.go:CleanConfigStringMap
/// CleanConfigStringMap is the same as CleanConfigStringMapString but for
/// `map[string]interface {}` (nested maps of a map that has `_merge` are cleaned recursively,
/// keeping their types). The result keeps m's map type (Go's static result type is
/// `map[string]interface {}`; callers convert).
pub fn clean_config_string_map(m: &Map) -> Map {
    if m.is_empty() {
        return m.clone();
    }
    if m.get(MERGE_STRATEGY_KEY.as_bytes()).is_none() {
        return m.clone();
    }
    // Create a new map and copy all the keys except the merge strategy key.
    let mut m2 = Map::new(m.ty.clone());
    for (k, v) in &m.entries {
        if k.as_bytes() != MERGE_STRATEGY_KEY.as_bytes() {
            m2.entries.insert(k.clone(), v.clone());
        }
        match v {
            Value::Map(v2) if matches!(v2.ty, MapType::StringAny | MapType::Params) => {
                m2.entries
                    .insert(k.clone(), Value::map(clean_config_string_map(v2)));
            }
            Value::Map(v2) if v2.ty == MapType::StringString => {
                m2.entries
                    .insert(k.clone(), Value::map(clean_config_string_map_string(v2)));
            }
            _ => {}
        }
    }
    m2
}

/// Methods declared on `maps.Params` (they shadow keys of the same exact name in templates):
/// `GetNested`, `IsZero`, `GetMergeStrategy`, `DeleteMergeStrategy`, `SetMergeStrategy`.
pub fn params_has_method(name: &str) -> bool {
    matches!(
        name,
        "GetNested" | "IsZero" | "GetMergeStrategy" | "DeleteMergeStrategy" | "SetMergeStrategy"
    )
}

/// Dispatch of `maps.Params` methods for the [`crate::object::NamedTypeRegistry`]. The receiver
/// is a `Value::Map` of type `maps.Params` (or a nil `maps.Params`).
///
/// `GetMergeStrategy` (two results, the second not an `error`) and `SetMergeStrategy` (no
/// result) cannot be called from a template (Go's `evalCall`). `DeleteMergeStrategy` reports
/// whether `_merge` is present but cannot delete it from the shared, immutable map value (Go
/// deletes it from the live map).
pub fn params_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    let empty = new_params();
    let p: &Map = match recv {
        Value::Map(m) if m.ty == MapType::Params => m,
        Value::TypedNil(t) if &**t == "maps.Params" => &empty,
        _ => return None,
    };
    Some(match name {
        "GetNested" => (|| {
            let indices = args::strings(a, 0)?;
            let idx: Vec<&[u8]> = indices.iter().map(|s| s.as_bytes()).collect();
            Ok(get_nested(p, &idx))
        })(),
        "IsZero" => args::exactly(a, 0, name).map(|_| Value::Bool(params_is_zero(p))),
        "GetMergeStrategy" => Err(bad_results_error(name, 2)),
        "SetMergeStrategy" => Err(bad_results_error(name, 0)),
        "DeleteMergeStrategy" => args::exactly(a, 0, name)
            .map(|_| Value::Bool(p.get(MERGE_STRATEGY_KEY.as_bytes()).is_some())),
        _ => return None,
    })
}

/// Registry entry for `maps.Params`.
pub const PARAMS_METHODS: NamedMethods = NamedMethods {
    has_method: params_has_method,
    call: params_call_method,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/params.go (384 lines; 13/17 funcs executed)
//   types: Params, KeyParams, ParamsMergeStrategy
// OK L35-38: (p Params) GetNested(indices ...string) any
// OK L42-60: SetParams(dst, src Params)
// OK L63-77: (p Params) IsZero() bool
// OK L81-83: MergeParamsWithStrategy(strategy string, dst, src Params)
// OK L87-90: MergeParams(dst, src Params)
// OK L92-122: (p Params) merge(ps ParamsMergeStrategy, pp Params)
// OK L125-132: (p Params) GetMergeStrategy() (ParamsMergeStrategy, bool)
// OK L135-141: (p Params) DeleteMergeStrategy() bool
// OK L144-151: (p Params) SetMergeStrategy(s ParamsMergeStrategy) (the enum has only valid values; Go panics on others)
// OK L153-180: getNested(m map[string]any, indices []string) (any, string, map[string]any)
// OK L186-204: GetNestedParam(keyStr, separator string, candidates ...Params) (any, error)
// OK L206-231: GetNestedParamFn(keyStr, separator string, lookupFn func(key string) any) (any, string, map[string]any, error)
// OK L249-264: CleanConfigStringMapString(m map[string]string) map[string]string
// OK L268-293: CleanConfigStringMap(m map[string]any) map[string]any
// OK L295-303: toMergeStrategy(v any) ParamsMergeStrategy
// OK L310-345: PrepareParams(m Params)
// OK L348-384: PrepareParamsClone(m Params) Params
// ---------------------------------------------------------------------------
