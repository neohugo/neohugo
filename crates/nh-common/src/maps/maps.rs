//! Port of `common/maps/maps.go`.
//!
//! Owner: Wave B task T01 (common-values).


use go_value::{GoString, Map, Value};

use crate::herrors::Result;

/// Go: `maps.ToStringMapE(in any) (map[string]any, error)`.
// Go: common/maps/maps.go:ToStringMapE
pub fn to_string_map_e(v: &Value) -> Result<Map> {
    todo!()
}

/// Go: `maps.ToStringMapStringE`.
// Go: common/maps/maps.go:ToStringMapStringE
pub fn to_string_map_string_e(v: &Value) -> Result<Map> {
    todo!()
}

/// Go: `maps.ToSliceStringMap` (`[]map[string]any`).
// Go: common/maps/maps.go:ToSliceStringMap
pub fn to_slice_string_map(v: &Value) -> Result<Vec<Map>> {
    todo!()
}

/// Go: `maps.LookupEqualFold` — case-insensitive key lookup; returns (value, actual key).
// Go: common/maps/maps.go:LookupEqualFold
pub fn lookup_equal_fold<'a>(m: &'a Map, key: &[u8]) -> Option<(&'a Value, &'a GoString)> {
    todo!()
}

/// Go: `maps.MergeShallow(dst, src)` — copies keys missing in dst.
// Go: common/maps/maps.go:MergeShallow
pub fn merge_shallow(dst: &mut Map, src: &Map) {
    todo!()
}

/// Go: `maps.KeyRenamer` (config key aliases such as `menu` -> `menus`, glob patterns on key paths).
pub struct KeyRenamer {
    /// (glob pattern over "/"-joined lower-cased key paths, new key)
    pub renames: Vec<(String, String)>,
}

impl KeyRenamer {
    // Go: common/maps/maps.go:NewKeyRenamer
    pub fn new(pattern_keys: &[&str]) -> Result<Self> {
        todo!()
    }

    // Go: common/maps/maps.go:Rename
    pub fn rename(&self, m: &mut Map) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/maps.go (236 lines; 12/16 funcs executed)
//   types: keyRename, KeyRenamer
// EX L27-41: ToStringMapE(in any) (map[string]any, error)
// EX L46-56: ToParamsAndPrepare(in any) (Params, error)
//    L59-65: MustToParamsAndPrepare(in any) Params
// EX L68-71: ToStringMap(in any) map[string]any
// EX L74-80: ToStringMapStringE(in any) (map[string]string, error)
// EX L83-86: ToStringMapString(in any) map[string]string
//    L89-92: ToStringMapBool(in any) map[string]bool
//    L95-112: ToSliceStringMap(in any) ([]map[string]any, error)
// EX L115-126: LookupEqualFold[T any | string](m map[string]T, key string) (T, string, bool)
// EX L130-143: MergeShallow(dst, src map[string]any)
// EX L157-168: NewKeyRenamer(patternKeys ...string) (KeyRenamer, error)
// EX L170-178: (r KeyRenamer) getNewKey(keyPath string) string
// EX L182-184: (r KeyRenamer) Rename(m map[string]any)
// EX L186-192: (KeyRenamer) keyPath(k1, k2 string) string
// EX L194-211: (r KeyRenamer) renamePath(parentKeyPath string, m map[string]any)
//    L214-236: ConvertFloat64WithNoDecimalsToInt(m map[string]any)
// ---------------------------------------------------------------------------
