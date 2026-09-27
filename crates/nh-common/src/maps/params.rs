//! Port of `common/maps/params.go`.
//!
//! Owner: Wave B task T01 (common-values).


//! `maps.Params` is represented as a [`go_value::Map`] with [`MapType::Params`]: keys are stored
//! lower-cased (Go `strings.ToLower`, simple case mapping — use go-unicode, never
//! `str::to_lowercase`), nested maps are Params too, slices are NOT descended into.
//! Template lookups on Params are case-insensitive ([`params_get`]); lookups on every other map
//! type are exact.

use go_value::{GoString, Map, MapType, Value};

use crate::herrors::Result;
use crate::object::{GoResult, HostCtx, NamedMethods};

/// Go: `maps.MergeStrategyKey`.
pub const MERGE_STRATEGY_KEY: &str = "_merge";

/// Go: `maps.ParamsMergeStrategy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamsMergeStrategy {
    /// "none"
    None,
    /// "shallow"
    Shallow,
    /// "deep"
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

/// Go: `maps.Params.GetNested(indices ...string)` — lower-cases each index.
// Go: common/maps/params.go:GetNested
pub fn get_nested(p: &Map, indices: &[&[u8]]) -> Value {
    todo!("port common/maps/params.go:GetNested + getNested")
}

/// Go: `maps.GetNestedParam(keyStr, separator, candidates...)`.
// Go: common/maps/params.go:GetNestedParam
pub fn get_nested_param(key: &str, separator: &str, candidates: &[&Map]) -> Result<Value> {
    todo!()
}

/// Go: `maps.Params.IsZero()` — empty, or only the `_merge` key.
// Go: common/maps/params.go:IsZero
pub fn params_is_zero(p: &Map) -> bool {
    p.is_empty() || (p.len() == 1 && p.get(MERGE_STRATEGY_KEY.as_bytes()).is_some())
}

/// Template map lookup on Params (Go: `tplimpl.templateExecHelper.GetMapValue`): the key is
/// lower-cased; a present key holding nil yields `Value::Invalid` (same as missing).
pub fn params_get(p: &Map, key: &[u8]) -> Value {
    todo!("lower-case key with go-unicode ToLower; nil -> Invalid")
}

/// Go: `maps.PrepareParams` — lower-cases keys recursively and converts nested maps
/// (`map[string]any`, `map[any]any`, `map[string]string`) to Params. Does not descend into slices.
// Go: common/maps/params.go:PrepareParams
pub fn prepare_params(p: &mut Map) {
    todo!()
}

/// Go: `maps.ToParamsAndPrepare(in any) (Params, error)`.
// Go: common/maps/maps.go:ToParamsAndPrepare
pub fn to_params_and_prepare(v: &Value) -> Result<Map> {
    todo!()
}

/// Go: `maps.MergeParams(dst, src)` — adds missing keys (dst wins), recursively per strategy.
// Go: common/maps/params.go:MergeParams
pub fn merge_params(dst: &mut Map, src: &Map) {
    todo!()
}

/// Go: `maps.MergeParamsWithStrategy`.
// Go: common/maps/params.go:MergeParamsWithStrategy
pub fn merge_params_with_strategy(strategy: &str, dst: &mut Map, src: &Map) {
    todo!()
}

/// Go: `maps.SetParams(dst, src)` — overwrites.
// Go: common/maps/params.go:SetParams
pub fn set_params(dst: &mut Map, src: &Map) {
    todo!()
}

/// Go: `Params.GetMergeStrategy`.
// Go: common/maps/params.go:GetMergeStrategy
pub fn get_merge_strategy(p: &Map) -> (ParamsMergeStrategy, bool) {
    todo!()
}

/// Go: `Params.SetMergeStrategy`.
pub fn set_merge_strategy(p: &mut Map, s: ParamsMergeStrategy) {
    todo!()
}

/// Go: `Params.DeleteMergeStrategy`.
pub fn delete_merge_strategy(p: &mut Map) -> bool {
    todo!()
}

/// Go: `maps.CleanConfigStringMap` (drops `_merge` keys recursively, keeps key case as stored).
// Go: common/maps/params.go:CleanConfigStringMap
pub fn clean_config_string_map(m: &Map) -> Map {
    todo!()
}

/// Go: `maps.CleanConfigStringMapString`.
pub fn clean_config_string_map_string(m: &Map) -> Map {
    todo!()
}

/// Methods declared on `maps.Params` (they shadow keys of the same exact name in templates):
/// `GetNested`, `IsZero`, `GetMergeStrategy`, `DeleteMergeStrategy`, `SetMergeStrategy`, `Set`,
/// `SetDefaultMergeStrategy`, `Merge`.
pub fn params_has_method(name: &str) -> bool {
    matches!(
        name,
        "GetNested" | "IsZero" | "GetMergeStrategy" | "DeleteMergeStrategy" | "SetMergeStrategy" | "SetDefaultMergeStrategy"
    )
}

/// Dispatch of `maps.Params` methods for the [`crate::object::NamedTypeRegistry`].
pub fn params_call_method(ctx: HostCtx<'_>, recv: &Value, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
    todo!()
}

/// Registry entry for `maps.Params`.
pub const PARAMS_METHODS: NamedMethods = NamedMethods { has_method: params_has_method, call: params_call_method };

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/params.go (384 lines; 13/17 funcs executed)
//   types: Params, KeyParams, ParamsMergeStrategy
// EX L35-38: (p Params) GetNested(indices ...string) any
// EX L42-60: SetParams(dst, src Params)
// EX L63-77: (p Params) IsZero() bool
//    L81-83: MergeParamsWithStrategy(strategy string, dst, src Params)
// EX L87-90: MergeParams(dst, src Params)
// EX L92-122: (p Params) merge(ps ParamsMergeStrategy, pp Params)
// EX L125-132: (p Params) GetMergeStrategy() (ParamsMergeStrategy, bool)
// EX L135-141: (p Params) DeleteMergeStrategy() bool
// EX L144-151: (p Params) SetMergeStrategy(s ParamsMergeStrategy)
// EX L153-180: getNested(m map[string]any, indices []string) (any, string, map[string]any)
// EX L186-204: GetNestedParam(keyStr, separator string, candidates ...Params) (any, error)
//    L206-231: GetNestedParamFn(keyStr, separator string, lookupFn func(key string) any) (any, string, map[string]any, error)
// EX L249-264: CleanConfigStringMapString(m map[string]string) map[string]string
// EX L268-293: CleanConfigStringMap(m map[string]any) map[string]any
//    L295-303: toMergeStrategy(v any) ParamsMergeStrategy
// EX L310-345: PrepareParams(m Params)
//    L348-384: PrepareParamsClone(m Params) Params
// ---------------------------------------------------------------------------
