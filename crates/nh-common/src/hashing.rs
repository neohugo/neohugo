//! Port of `common/hashing/hashing.go`.
//!
//! Owner: Wave B task T01 (common-values).

//! Go `common/hashing` over the Wave A `go-hashstructure` crate (gohugoio/hashstructure v0.5.0,
//! XXH64 seed 0), which ports the hashing itself. Where hashes reach output bytes: `_hu_` image
//! names, image filter keys, the imaging-config SourceHash, the GetRemote file-cache key,
//! resource transformation keys.
//!
//! Go panics when a value cannot be hashed (`HashString`, `HashStringHex`, `HashUint64`); the
//! plain functions here panic too, the `try_*` variants return the error.

use std::io::Read;
use std::sync::Once;

use go_hashstructure::HashValue;
use go_value::Value;

pub use go_hashstructure::hashing::ReaderKind;

use crate::herrors::{Error, Result};

/// Registers how nh-common's named basic types look to hashstructure (Go hashes a named string
/// type as a string). Called by every entry point of this module; idempotent.
pub fn register_hash_types() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        go_hashstructure::register_object::<crate::types::hstring::Html>(|h| {
            HashValue::String(h.0.clone())
        });
        go_hashstructure::register_object::<crate::maps::params::ParamsMergeStrategy>(|s| {
            HashValue::String(s.as_str().into())
        });
        go_hashstructure::register_object::<crate::types::css::QuotedString>(|s| {
            HashValue::String(s.0.clone())
        });
        go_hashstructure::register_object::<crate::types::css::UnquotedString>(|s| {
            HashValue::String(s.0.clone())
        });
    });
}

/// A named basic type's underlying value (Go hashes `time.Duration` as an int64, a named string
/// type as a string), for objects that are not `Key()` providers.
fn underlying_basic(o: &std::sync::Arc<dyn go_value::Object>) -> Option<Value> {
    if o.hash_key().is_some() {
        return None;
    }
    match o.underlying()? {
        u @ (Value::Bool(_)
        | Value::Int(..)
        | Value::Uint(..)
        | Value::Float(..)
        | Value::String(_)) => Some(u),
        _ => None,
    }
}

/// Replaces named basic objects by their underlying values, recursively through slices and maps
/// (hashstructure reflects on the kind, which for a named basic type is its underlying kind).
/// `None` when nothing changes.
fn rewrite_named(v: &Value) -> Option<Value> {
    match v {
        Value::Object(o) => underlying_basic(o),
        Value::List(l) => {
            let items: Vec<Option<Value>> = l.items.iter().map(rewrite_named).collect();
            if items.iter().all(Option::is_none) {
                return None;
            }
            let items = items
                .into_iter()
                .zip(l.items.iter())
                .map(|(n, o)| n.unwrap_or_else(|| o.clone()))
                .collect();
            Some(Value::list(l.ty.clone(), items))
        }
        Value::Map(m) => {
            let changed: Vec<(go_value::GoString, Value)> = m
                .entries
                .iter()
                .filter_map(|(k, v)| rewrite_named(v).map(|n| (k.clone(), n)))
                .collect();
            if changed.is_empty() {
                return None;
            }
            let mut m2 = (**m).clone();
            for (k, n) in changed {
                m2.entries.insert(k, n);
            }
            Some(Value::map(m2))
        }
        _ => None,
    }
}

fn values(vs: &[Value]) -> Vec<HashValue> {
    vs.iter()
        .map(|v| HashValue::Value(rewrite_named(v).unwrap_or_else(|| v.clone())))
        .collect()
}

fn hash_error(e: go_hashstructure::Error) -> Error {
    Error::new(e.to_string())
}

// Go: common/hashing/hashing.go:XXHashFromReader
/// XXHashFromReader calculates the xxHash for the given reader: (hash, size). The size depends on
/// how Go's `io.Copy` reads the reader (see [`ReaderKind`]); the hash does not.
pub fn xxhash_from_reader(r: &mut dyn Read, kind: ReaderKind) -> Result<(u64, i64)> {
    Ok(go_hashstructure::hashing::xxhash_from_reader(r, kind)?)
}

// Go: common/hashing/hashing.go:XxHashFromReaderHexEncoded
/// XxHashFromReaderHexEncoded calculates the xxHash for the given reader and returns the hash as
/// a hex encoded string (16 digits).
pub fn xxhash_from_reader_hex_encoded(r: &mut dyn Read) -> Result<String> {
    Ok(go_hashstructure::hashing::xxhash_from_reader_hex_encoded(
        r,
    )?)
}

// Go: common/hashing/hashing.go:XXHashFromString
/// XXHashFromString calculates the xxHash for the given string.
pub fn xxhash_from_string(s: &[u8]) -> u64 {
    go_hashstructure::hashing::xxhash_from_string(s)
}

// Go: common/hashing/hashing.go:XxHashFromStringHexEncoded
/// XxHashFromStringHexEncoded calculates the xxHash for the given string and returns the hash as
/// a hex encoded string (16 digits, big endian).
pub fn xxhash_from_string_hex_encoded(s: &[u8]) -> String {
    go_hashstructure::hashing::xxhash_from_string_hex_encoded(s)
}

// Go: common/hashing/hashing.go:MD5FromStringHexEncoded
/// MD5FromStringHexEncoded returns the MD5 hash of the given string.
pub fn md5_from_string_hex_encoded(s: &[u8]) -> String {
    go_hashstructure::hashing::md5_from_string_hex_encoded(s)
}

// Go: common/hashing/hashing.go:HashString
/// HashString returns a hash from the given elements as a decimal string. Panics if the hash
/// cannot be calculated (as Go does); see [`try_hash_string`].
pub fn hash_string(vs: &[Value]) -> String {
    hash_uint64(vs).to_string()
}

/// [`hash_string`] returning the error instead of panicking.
pub fn try_hash_string(vs: &[Value]) -> Result<String> {
    Ok(try_hash_uint64(vs)?.to_string())
}

// Go: common/hashing/hashing.go:HashStringHex
/// HashStringHex returns a hash from the given elements as a lower-case hex string without zero
/// padding (`strconv.FormatUint(hash, 16)`). Panics like Go; see [`try_hash_string_hex`].
pub fn hash_string_hex(vs: &[Value]) -> String {
    format!("{:x}", hash_uint64(vs))
}

/// [`hash_string_hex`] returning the error instead of panicking.
pub fn try_hash_string_hex(vs: &[Value]) -> Result<String> {
    Ok(format!("{:x}", try_hash_uint64(vs)?))
}

// Go: common/hashing/hashing.go:HashUint64
/// HashUint64 returns a hash from the given elements: one element is hashed as itself, several
/// as a `[]interface {}`; each element is first passed through `toHashable` (a `Key() string`
/// provider, `Object::hash_key`, hashes as its key). Panics like Go; see [`try_hash_uint64`].
pub fn hash_uint64(vs: &[Value]) -> u64 {
    match try_hash_uint64(vs) {
        Ok(h) => h,
        Err(e) => panic!("{}", e.message()),
    }
}

/// [`hash_uint64`] returning the error instead of panicking.
pub fn try_hash_uint64(vs: &[Value]) -> Result<u64> {
    register_hash_types();
    go_hashstructure::hashing::hash_uint64(&values(vs)).map_err(hash_error)
}

/// [`hash_uint64`] over explicit hashstructure values (Go structs with tags, `Hashable`s).
pub fn hash_uint64_values(vs: &[HashValue]) -> Result<u64> {
    register_hash_types();
    go_hashstructure::hashing::hash_uint64(vs).map_err(hash_error)
}

// Go: common/hashing/hashing.go:getHashOpts
/// The pooled `HashOptions{Hasher: xxhash.New()}` (a fresh one per call here).
pub fn get_hash_opts() -> go_hashstructure::HashOptions {
    go_hashstructure::hashing::hugo_hash_options()
}

// Go: common/hashing/hashing.go:putHashOpts
/// Go resets the pooled hasher; nothing to do without a pool.
pub fn put_hash_opts(_opts: go_hashstructure::HashOptions) {}

// Go: common/hashing/hashing.go:Hash
/// Hash returns a hash from vs (one value: that value; several: the `[]interface {}` of them),
/// without `toHashable`.
pub fn hash(vs: &[Value]) -> Result<u64> {
    register_hash_types();
    go_hashstructure::hashing::hash(&values(vs)).map_err(hash_error)
}

/// [`hash`] over explicit hashstructure values.
pub fn hash_values(vs: &[HashValue]) -> Result<u64> {
    register_hash_types();
    go_hashstructure::hashing::hash(vs).map_err(hash_error)
}

// Go: common/hashing/hashing.go:toHashable
/// For structs, hashstructure.Hash only works on the exported fields, so known identity types
/// are rewritten: a `Key() string` provider becomes its key. (`identity.IdentityProvider` values
/// become their identity in Go; identity is a stub, and hosts that need it implement
/// `Object::hash_key`.)
pub fn to_hashable(v: &Value) -> HashValue {
    go_hashstructure::hashing::to_hashable(&HashValue::Value(v.clone()))
}

// Go: common/hashing/hashing.go:xxhashReadFrom.ReadFrom
// Go: common/hashing/hashing.go:getXxHashReadFrom
// Go: common/hashing/hashing.go:putXxHashReadFrom
// (The 48 KiB read loop is go_hashstructure::hashing::xxhash_from_reader; no pool is needed.)

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hashing/hashing.go (194 lines; 12/15 funcs executed)
//   types: keyer, xxhashReadFrom
// OK L30-39: XXHashFromReader(r io.Reader) (uint64, int64, error)
// OK L43-52: XxHashFromReaderHexEncoded(r io.Reader) (string, error)
// OK L55-59: XXHashFromString(s string) (uint64, error)
// OK L63-68: XxHashFromStringHexEncoded(f string) string
// OK L71-75: MD5FromStringHexEncoded(f string) string
// OK L81-84: HashString(vs ...any) string
// OK L88-91: HashStringHex(vs ...any) string
// OK L101-103: getHashOpts() *hashstructure.HashOptions
// OK L105-108: putHashOpts(opts *hashstructure.HashOptions)
// OK L114-131: HashUint64(vs ...any) uint64
// OK L134-142: Hash(vs ...any) (uint64, error)
// OK L150-159: toHashable(v any) any
// OK L166-179: (x *xxhashReadFrom) ReadFrom(r io.Reader) (int64, error) (go-hashstructure)
// OK L187-189: getXxHashReadFrom() *xxhashReadFrom
// OK L191-194: putXxHashReadFrom(h *xxhashReadFrom)
// ---------------------------------------------------------------------------
