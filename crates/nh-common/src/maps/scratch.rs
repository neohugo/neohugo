//! Port of `common/maps/scratch.go`.
//!
//! Owner: Wave B task T01 (common-values).


//! `*maps.Scratch` (`.Scratch`, `.Store`, `newScratch`). One Scratch per page, shared by all output
//! formats and pager renders of that page (Go: `hugolib/page__common.go`).

use std::collections::BTreeMap;
use std::sync::Mutex;

use go_value::{GoString, Object, Value};

use crate::herrors::Result;
use crate::object::{args, GoResult, HostCtx};

/// Go: `maps.StoreProvider`.
pub trait StoreProvider {
    fn store(&self) -> std::sync::Arc<Scratch>;
}

/// Go: `maps.Scratch`.
#[derive(Default)]
pub struct Scratch {
    values: Mutex<BTreeMap<GoString, Value>>,
}

impl Scratch {
    // Go: common/maps/scratch.go:NewScratch
    pub fn new() -> Self {
        Self::default()
    }

    /// Go: `Scratch.Add`: missing key -> set; existing slice -> `collections.Append`; otherwise
    /// `math.DoArithmetic(existing, addend, '+')` (int+int -> int64, string+string concat).
    /// Returns "" (printed by templates).
    // Go: common/maps/scratch.go:Add
    pub fn add(&self, ctx: HostCtx<'_>, key: &GoString, addend: Value) -> Result<GoString> {
        todo!()
    }

    // Go: common/maps/scratch.go:Set
    pub fn set(&self, key: &GoString, value: Value) -> GoString {
        self.values.lock().unwrap().insert(key.clone(), value);
        GoString::empty()
    }

    // Go: common/maps/scratch.go:Delete
    pub fn delete(&self, key: &GoString) -> GoString {
        self.values.lock().unwrap().remove(key);
        GoString::empty()
    }

    /// Missing key -> `Value::Invalid` (Go returns nil any).
    // Go: common/maps/scratch.go:Get
    pub fn get(&self, key: &GoString) -> Value {
        self.values.lock().unwrap().get(key).cloned().unwrap_or(Value::Invalid)
    }

    /// Go: `Scratch.Values()` (`map[string]any`).
    pub fn values(&self) -> Value {
        todo!()
    }

    /// Go: `Scratch.SetInMap(key, mapKey, value)` — creates a `map[string]any` if missing.
    // Go: common/maps/scratch.go:SetInMap
    pub fn set_in_map(&self, key: &GoString, map_key: &GoString, value: Value) -> GoString {
        todo!()
    }

    // Go: common/maps/scratch.go:DeleteInMap
    pub fn delete_in_map(&self, key: &GoString, map_key: &GoString) -> GoString {
        todo!()
    }

    // Go: common/maps/scratch.go:GetSortedMapValues
    pub fn get_sorted_map_values(&self, key: &GoString) -> Value {
        todo!()
    }
}

crate::go_methods!(Scratch {
    "Add" => |s, ctx, a| Ok(Value::String(s.add(ctx, &args::string(a, 0)?, args::get(a, 1)?)?)),
    "Set" => |s, _ctx, a| Ok(Value::String(s.set(&args::string(a, 0)?, args::get(a, 1)?))),
    "Delete" => |s, _ctx, a| Ok(Value::String(s.delete(&args::string(a, 0)?))),
    "Get" => |s, _ctx, a| Ok(s.get(&args::string(a, 0)?)),
    "Values" => |s, _ctx, _a| Ok(s.values()),
    "SetInMap" => |s, _ctx, a| Ok(Value::String(s.set_in_map(&args::string(a, 0)?, &args::string(a, 1)?, args::get(a, 2)?))),
    "DeleteInMap" => |s, _ctx, a| Ok(Value::String(s.delete_in_map(&args::string(a, 0)?, &args::string(a, 1)?))),
    "GetSortedMapValues" => |s, _ctx, a| Ok(s.get_sorted_map_values(&args::string(a, 0)?)),
});

impl Object for Scratch {
    crate::object_basics!("*maps.Scratch");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/scratch.go (160 lines; 6/9 funcs executed)
//   types: StoreProvider, Scratch
// EX L41-69: (c *Scratch) Add(key string, newAddend any) (string, error)
// EX L73-78: (c *Scratch) Set(key string, value any) string
// EX L81-86: (c *Scratch) Delete(key string) string
// EX L89-95: (c *Scratch) Get(key string) any
//    L100-104: (c *Scratch) Values() map[string]any
// EX L108-118: (c *Scratch) SetInMap(key string, mapKey string, value any) string
//    L121-129: (c *Scratch) DeleteInMap(key string, mapKey string) string
//    L132-155: (c *Scratch) GetSortedMapValues(key string) any
// EX L158-160: NewScratch() *Scratch
// ---------------------------------------------------------------------------
