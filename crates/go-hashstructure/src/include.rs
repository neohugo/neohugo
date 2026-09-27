//! Go: github.com/gohugoio/hashstructure@v0.5.0 include.go

use crate::Error;
use crate::value::HashValue;

/// Go: `Includable` — implemented by a struct to decide per field whether
/// it is included in the hash. `v` is the field's value (Go passes the
/// `reflect.Value` of the field, after any `fmt.Stringer` substitution).
pub trait Includable: Send + Sync {
    fn hash_include(&self, field: &str, v: &HashValue) -> Result<bool, Error>;
}

/// Go: `IncludableMap` — implemented by a struct (for its map-typed
/// fields, `field` is the field name) or by a map type (`field` is "") to
/// decide per entry whether it is included.
pub trait IncludableMap: Send + Sync {
    fn hash_include_map(&self, field: &str, k: &HashValue, v: &HashValue) -> Result<bool, Error>;
}

/// Go: `Hashable` — overrides the hash of the whole struct.
pub trait Hashable: Send + Sync {
    fn hash(&self) -> Result<u64, Error>;
}
