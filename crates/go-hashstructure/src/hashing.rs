//! neohugo `common/hashing` (common/hashing/hashing.go).

use std::io::{self, Read};

use md5::{Digest, Md5};

use crate::Error;
use crate::hasher::{Hasher64, XxHash64};
use crate::hashstructure::{HashOptions, hash as hs_hash};
use crate::value::HashValue;

/// How the Go `io.Reader` passed to `XXHashFromReader` behaves in
/// `io.Copy(h, r)`, which decides the returned size (the hash is the same).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderKind {
    /// The reader implements `io.WriterTo` with a real implementation
    /// (`strings.Reader`, `bytes.Reader`, `bytes.Buffer`): the size is the
    /// total number of bytes.
    WriterTo,
    /// Any other reader, including `*os.File` (whose `WriteTo` falls back to
    /// `genericWriteTo` → the hasher's `ReadFrom`): the size is the byte
    /// count of the *last* `Read` call, i.e. 0 for readers that report EOF
    /// with an empty read. neohugo's resource hashing gets 0 here.
    ReadFrom,
}

// Go: hashing.go:XXHashFromReader
//
// XXHashFromReader calculates the xxHash for the given reader.
pub fn xxhash_from_reader(r: &mut dyn Read, kind: ReaderKind) -> io::Result<(u64, i64)> {
    let mut h = XxHash64::new();
    // Go: xxhashReadFrom uses a 48 KiB buffer.
    let mut buf = vec![0u8; 48 * 1024];
    let mut total: i64 = 0;
    let mut last: i64;
    loop {
        let n = match r.read(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if n > 0 {
            h.write(&buf[..n]);
        }
        total += n as i64;
        last = n as i64;
        if n == 0 {
            break;
        }
    }
    let size = match kind {
        ReaderKind::WriterTo => total,
        ReaderKind::ReadFrom => last,
    };
    Ok((h.sum64(), size))
}

// Go: hashing.go:XxHashFromReaderHexEncoded
//
// XxHashFromReaderHexEncoded calculates the xxHash for the given reader
// and returns the hash as a hex encoded string (16 digits, big endian).
pub fn xxhash_from_reader_hex_encoded(r: &mut dyn Read) -> io::Result<String> {
    let (h, _) = xxhash_from_reader(r, ReaderKind::WriterTo)?;
    Ok(format!("{h:016x}"))
}

// Go: hashing.go:XXHashFromString
pub fn xxhash_from_string(s: impl AsRef<[u8]>) -> u64 {
    xxhash_rust::xxh64::xxh64(s.as_ref(), 0)
}

// Go: hashing.go:XxHashFromStringHexEncoded
pub fn xxhash_from_string_hex_encoded(s: impl AsRef<[u8]>) -> String {
    format!("{:016x}", xxhash_from_string(s))
}

// Go: hashing.go:MD5FromStringHexEncoded
pub fn md5_from_string_hex_encoded(s: impl AsRef<[u8]>) -> String {
    let d = Md5::digest(s.as_ref());
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// neohugo's hashing options: `hashstructure.HashOptions{Hasher: xxhash.New()}`.
pub fn hugo_hash_options() -> HashOptions {
    HashOptions {
        hasher: Some(Box::new(XxHash64::new())),
        ..Default::default()
    }
}

// Go: hashing.go:Hash
//
// Hash returns a hash from vs (one value: that value; several: the
// `[]interface{}` of them).
pub fn hash(vs: &[HashValue]) -> Result<u64, Error> {
    let mut opts = hugo_hash_options();
    if vs.len() == 1 {
        return hs_hash(&vs[0], Some(&mut opts));
    }
    let v = HashValue::any_slice(vs.to_vec());
    hs_hash(&v, Some(&mut opts))
}

// Go: hashing.go:HashUint64
//
// HashUint64 returns a hash from the given elements. Go panics when the
// hash cannot be calculated; this returns the error instead.
pub fn hash_uint64(vs: &[HashValue]) -> Result<u64, Error> {
    let o = if vs.len() == 1 {
        to_hashable(&vs[0])
    } else {
        HashValue::any_slice(vs.iter().map(to_hashable).collect())
    };
    hash(std::slice::from_ref(&o))
}

// Go: hashing.go:HashString
//
// HashString returns a hash from the given elements, as a decimal string.
pub fn hash_string(vs: &[HashValue]) -> Result<String, Error> {
    Ok(hash_uint64(vs)?.to_string())
}

// Go: hashing.go:HashStringHex
//
// HashStringHex returns a hash from the given elements as a (lower-case,
// unpadded) hex string.
pub fn hash_string_hex(vs: &[HashValue]) -> Result<String, Error> {
    Ok(format!("{:x}", hash_uint64(vs)?))
}

// Go: hashing.go:toHashable
//
// For structs, hashstructure.Hash only works on the exported fields,
// so rewrite the input slice for known identity types: a `Key() string`
// provider is replaced by its key. (`identity.IdentityProvider` values
// must be replaced by their `GetIdentity()` by the caller: the shared
// value model has no such hook.)
pub fn to_hashable(v: &HashValue) -> HashValue {
    match v {
        HashValue::Value(go_value::Value::Object(o)) => match o.hash_key() {
            Some(k) => HashValue::String(k),
            None => v.clone(),
        },
        HashValue::Struct(s) => match &s.key {
            Some(k) => HashValue::String(k.clone()),
            None => v.clone(),
        },
        HashValue::Ptr(Some(inner), _) => match inner.as_ref() {
            HashValue::Struct(s) if s.key.is_some() => {
                HashValue::String(s.key.clone().unwrap_or_default())
            }
            _ => v.clone(),
        },
        _ => v.clone(),
    }
}
