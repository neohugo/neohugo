//! Port of [github.com/gohugoio/hashstructure] v0.5.0 (the version in
//! neohugo's go.mod) and of neohugo's `common/hashing`.
//!
//! hashstructure hashes Go values through reflection. Here the reflected
//! value is a [`HashValue`]: either a value of the shared model
//! ([`go_value::Value`], converted on the fly) or an explicit stand-in for a
//! Go struct ([`GoStruct`]: type name, ordered fields with tags, and the
//! `Hashable`/`Includable`/`IncludableMap`/`fmt.Stringer` method sets).
//!
//! neohugo always hashes with xxhash (`hashing.HashString`,
//! `hashing.HashStringHex`, `hashing.HashUint64`) — see [`hashing`].
//!
//! [github.com/gohugoio/hashstructure]: https://github.com/gohugoio/hashstructure

mod errors;
mod hasher;
pub mod hashing;
mod hashstructure;
mod include;
mod structtag;
mod value;

pub use errors::Error;
pub use hasher::{Fnv64, Hasher64, XxHash64};
pub use hashstructure::{HashOptions, hash};
pub use include::{Hashable, Includable, IncludableMap};
pub use value::{
    GoField, GoMap, GoStruct, HashValue, Receiver, UnsupportedKind, bare_type_name, from_value,
    is_zero, natural_stringer, register_object,
};
