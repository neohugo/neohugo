//! Port of `common/maps/scratch.go`.
//!
//! Owner: Wave B task T01 (common-values).

//! `*maps.Scratch` (`.Scratch`, `.Store`, `newScratch`). One Scratch per page, shared by all output
//! formats and pager renders of that page (Go: `hugolib/page__common.go`).
//!
//! Values are stored as template values; a nil `any` is `Value::Invalid` (a present key). The
//! lock is only held to read or write the map, never while `Add` computes (§4.8). Go's
//! `SetInMap`/`DeleteInMap` mutate the stored `map[string]interface {}` in place, which a
//! template that got the map earlier (`$m := .Scratch.Get "k"`) would observe; here the stored
//! map is updated copy-on-write, so earlier copies do not change (see PORTING.md).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use go_value::{GoString, Map, MapType, Object, Value};

use crate::herrors::{Error, Result};
use crate::object::{HostCtx, args};

/// Go: `maps.StoreProvider`.
pub trait StoreProvider {
    /// Store returns a Scratch that can be used to store temporary state.
    fn store(&self) -> std::sync::Arc<Scratch>;
}

/// Go: `maps.Scratch` — a writable context used for stateful build operations.
#[derive(Default)]
pub struct Scratch {
    values: Mutex<BTreeMap<GoString, Value>>,
}

/// Go's runtime panic for `v.(map[string]any)` on another dynamic type.
fn interface_conversion_error(v: &Value) -> Error {
    let t = crate::hreflect::type_of(v).map_or_else(|| "nil".to_string(), |t| t.into_owned());
    Error::new(format!(
        "interface conversion: interface {{}} is {t}, not map[string]interface {{}}"
    ))
}

impl Scratch {
    // Go: common/maps/scratch.go:NewScratch
    /// NewScratch returns a new instance of Scratch.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<GoString, Value>> {
        self.values.lock().unwrap_or_else(|e| e.into_inner())
    }

    // Go: common/maps/scratch.go:Add
    /// Add will, for single values, add (using the + operator) the addend to the existing addend
    /// (if found): `math.DoArithmetic(existing, addend, '+')` (int+int -> int64, string+string
    /// concatenates). If the existing value is a slice, the new value(s) are appended
    /// (`collections.Append`). A missing key is set to the addend. Returns "" (printed by
    /// templates).
    pub fn add(&self, ctx: HostCtx<'_>, key: &GoString, new_addend: Value) -> Result<GoString> {
        let existing_addend = self.lock().get(key).cloned();
        let new_val = match existing_addend {
            Some(existing_addend) => {
                // reflect.TypeOf(existingAddend).Kind(): Go dereferences a nil reflect.Type for
                // a nil existing value (a runtime panic, reported as an error by text/template).
                let kind = crate::hreflect::kind_of(&existing_addend);
                if crate::hreflect::type_of(&existing_addend).is_none() {
                    return Err(Error::new(
                        "runtime error: invalid memory address or nil pointer dereference",
                    ));
                }
                if kind == crate::hreflect::ReflectKind::Slice {
                    crate::collections::append::append(
                        ctx,
                        &existing_addend,
                        std::slice::from_ref(&new_addend),
                    )?
                } else {
                    crate::math::do_arithmetic(&existing_addend, &new_addend, '+')?
                }
            }
            None => new_addend,
        };
        self.lock().insert(key.clone(), new_val);
        // have to return something to make it work with the Go templates
        Ok(GoString::empty())
    }

    // Go: common/maps/scratch.go:Set
    /// Set stores a value with the given key. This value can later be retrieved with Get.
    pub fn set(&self, key: &GoString, value: Value) -> GoString {
        self.lock().insert(key.clone(), value);
        GoString::empty()
    }

    // Go: common/maps/scratch.go:Delete
    /// Delete deletes the given key.
    pub fn delete(&self, key: &GoString) -> GoString {
        self.lock().remove(key);
        GoString::empty()
    }

    // Go: common/maps/scratch.go:Get
    /// Get returns a value previously set by Add or Set; a missing key is `Value::Invalid` (Go
    /// returns a nil any).
    pub fn get(&self, key: &GoString) -> Value {
        self.lock().get(key).cloned().unwrap_or(Value::Invalid)
    }

    // Go: common/maps/scratch.go:Values
    /// Values returns the backing map, as a `map[string]interface {}` value (a snapshot; Go
    /// returns the live map).
    pub fn values(&self) -> Value {
        let m = self.lock().clone();
        Value::map(Map::with_entries(MapType::StringAny, m))
    }

    // Go: common/maps/scratch.go:SetInMap
    /// SetInMap stores a value in the `map[string]interface {}` stored under key (created when
    /// the key is missing). Go panics (an error in templates) if the key holds anything else.
    pub fn set_in_map(&self, key: &GoString, map_key: &GoString, value: Value) -> Result<GoString> {
        let mut values = self.lock();
        let entry = values
            .entry(key.clone())
            .or_insert_with(|| Value::map(Map::new(MapType::StringAny)));
        match entry {
            Value::Map(m) if m.ty == MapType::StringAny => {
                Arc::make_mut(m).insert(map_key.clone(), value);
            }
            Value::TypedNil(t) if &**t == "map[string]interface {}" => {
                return Err(Error::new("assignment to entry in nil map"));
            }
            other => return Err(interface_conversion_error(other)),
        }
        Ok(GoString::empty())
    }

    // Go: common/maps/scratch.go:DeleteInMap
    /// DeleteInMap deletes a value from the map stored under key.
    pub fn delete_in_map(&self, key: &GoString, map_key: &GoString) -> Result<GoString> {
        let mut values = self.lock();
        if let Some(entry) = values.get_mut(key) {
            match entry {
                Value::Map(m) if m.ty == MapType::StringAny => {
                    Arc::make_mut(m).entries.remove(map_key);
                }
                // Deleting from a nil map is a no-op in Go.
                Value::TypedNil(t) if &**t == "map[string]interface {}" => {}
                other => return Err(interface_conversion_error(other)),
            }
        }
        Ok(GoString::empty())
    }

    // Go: common/maps/scratch.go:GetSortedMapValues
    /// GetSortedMapValues returns the values of the map stored under key (filled with SetInMap),
    /// sorted by map key, as a `[]interface {}`; nil (`Invalid`) when the key is missing or nil.
    pub fn get_sorted_map_values(&self, key: &GoString) -> Result<Value> {
        let v = self.lock().get(key).cloned().unwrap_or(Value::Invalid);
        if crate::hreflect::type_of(&v).is_none() {
            return Ok(Value::Invalid);
        }
        let unsorted_map = match &v {
            Value::Map(m) if m.ty == MapType::StringAny => m.clone(),
            Value::TypedNil(t) if &**t == "map[string]interface {}" => {
                Arc::new(Map::new(MapType::StringAny))
            }
            other => return Err(interface_conversion_error(other)),
        };
        // sort.Strings(keys): byte order, which is the map's iteration order.
        let sorted_array: Vec<Value> = unsorted_map.entries.values().cloned().collect();
        Ok(Value::any_list(sorted_array))
    }
}

crate::go_methods!(Scratch {
    "Add" => |s, ctx, a| {
        args::exactly(a, 2, "Add")?;
        Ok(Value::String(s.add(ctx, &args::string(a, 0)?, args::get(a, 1)?)?))
    },
    "Set" => |s, _ctx, a| {
        args::exactly(a, 2, "Set")?;
        Ok(Value::String(s.set(&args::string(a, 0)?, args::get(a, 1)?)))
    },
    "Delete" => |s, _ctx, a| {
        args::exactly(a, 1, "Delete")?;
        Ok(Value::String(s.delete(&args::string(a, 0)?)))
    },
    "Get" => |s, _ctx, a| {
        args::exactly(a, 1, "Get")?;
        Ok(s.get(&args::string(a, 0)?))
    },
    "Values" => |s, _ctx, a| {
        args::exactly(a, 0, "Values")?;
        Ok(s.values())
    },
    "SetInMap" => |s, _ctx, a| {
        args::exactly(a, 3, "SetInMap")?;
        Ok(Value::String(s.set_in_map(&args::string(a, 0)?, &args::string(a, 1)?, args::get(a, 2)?)?))
    },
    "DeleteInMap" => |s, _ctx, a| {
        args::exactly(a, 2, "DeleteInMap")?;
        Ok(Value::String(s.delete_in_map(&args::string(a, 0)?, &args::string(a, 1)?)?))
    },
    "GetSortedMapValues" => |s, _ctx, a| {
        args::exactly(a, 1, "GetSortedMapValues")?;
        Ok(s.get_sorted_map_values(&args::string(a, 0)?)?)
    },
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
// OK L41-69: (c *Scratch) Add(key string, newAddend any) (string, error)
// OK L73-78: (c *Scratch) Set(key string, value any) string
// OK L81-86: (c *Scratch) Delete(key string) string
// OK L89-95: (c *Scratch) Get(key string) any
// OK L100-104: (c *Scratch) Values() map[string]any
// OK L108-118: (c *Scratch) SetInMap(key string, mapKey string, value any) string
// OK L121-129: (c *Scratch) DeleteInMap(key string, mapKey string) string
// OK L132-155: (c *Scratch) GetSortedMapValues(key string) any
// OK L158-160: NewScratch() *Scratch
// ---------------------------------------------------------------------------
