//! Port of `common/hashing/hashing.go`.
//!
//! Owner: Wave B task T01 (common-values).


//! Go `common/hashing` over the Wave A `go-hashstructure` crate (gohugoio/hashstructure v0.5.0,
//! XXH64 seed 0). Where hashes reach output bytes: `_hu_` image names, image filter keys, the
//! imaging-config SourceHash, the GetRemote file-cache key, resource transformation keys.

use std::io::Read;

use go_value::Value;

use crate::herrors::Result;

/// Go: `hashing.HashString(vs ...any)` — decimal string of [`hash_uint64`].
// Go: common/hashing/hashing.go:HashString
pub fn hash_string(vs: &[Value]) -> String {
    hash_uint64(vs).to_string()
}

/// Go: `hashing.HashStringHex(vs ...any)` — lowercase hex, NOT zero padded.
// Go: common/hashing/hashing.go:HashStringHex
pub fn hash_string_hex(vs: &[Value]) -> String {
    format!("{:x}", hash_uint64(vs))
}

/// Go: `hashing.HashUint64(vs ...any)`: one arg -> `toHashable(v)`; several -> `[]any{toHashable(v)...}`;
/// then `hashstructure.Hash(o, {Hasher: xxhash.New()})`. `toHashable` maps `Key() string` providers
/// (`Object::hash_key`) to their key.
// Go: common/hashing/hashing.go:HashUint64
pub fn hash_uint64(vs: &[Value]) -> u64 {
    todo!("go_hashstructure::hash(...) (Wave A)")
}

/// Go: `hashing.XXHashFromReader(r) (hash uint64, size int64, err)`.
// Go: common/hashing/hashing.go:XXHashFromReader
pub fn xxhash_from_reader(r: &mut dyn Read) -> Result<(u64, i64)> {
    todo!("xxhash-rust xxh64 streaming, seed 0")
}

/// Go: `hashing.XxHashFromStringHexEncoded`.
pub fn xxhash_from_string_hex_encoded(s: &[u8]) -> String {
    todo!()
}

/// Go: `hashing.MD5FromStringHexEncoded(f string) string`.
// Go: common/hashing/hashing.go:MD5FromStringHexEncoded
pub fn md5_from_string_hex_encoded(s: &[u8]) -> String {
    todo!("md-5 crate")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hashing/hashing.go (194 lines; 12/15 funcs executed)
//   types: keyer, xxhashReadFrom
// EX L30-39: XXHashFromReader(r io.Reader) (uint64, int64, error)
//    L43-52: XxHashFromReaderHexEncoded(r io.Reader) (string, error)
//    L55-59: XXHashFromString(s string) (uint64, error)
//    L63-68: XxHashFromStringHexEncoded(f string) string
// EX L71-75: MD5FromStringHexEncoded(f string) string
// EX L81-84: HashString(vs ...any) string
// EX L88-91: HashStringHex(vs ...any) string
// EX L101-103: getHashOpts() *hashstructure.HashOptions
// EX L105-108: putHashOpts(opts *hashstructure.HashOptions)
// EX L114-131: HashUint64(vs ...any) uint64
// EX L134-142: Hash(vs ...any) (uint64, error)
// EX L150-159: toHashable(v any) any
// EX L166-179: (x *xxhashReadFrom) ReadFrom(r io.Reader) (int64, error)
// EX L187-189: getXxHashReadFrom() *xxhashReadFrom
// EX L191-194: putXxHashReadFrom(h *xxhashReadFrom)
// ---------------------------------------------------------------------------
