//! A port of `gopkg.in/yaml.v2` v2.4.0 **decoding** (the version neohugo's
//! `parser/metadecoders` uses for front matter, config, data files and
//! i18n), plus neohugo's YAML post-processing (`stringifyMapKeys`) and the
//! conversion to [`go_value::Value`], and of its **encoding** (`yaml.Marshal`,
//! [`encode::marshal`], for `parser.InterfaceToConfig`).
//!
//! The whole libyaml-derived pipeline is ported from the Go sources
//! (reader, scanner, parser, node builder, decoder, resolver), so scanner
//! and parser errors, line numbers, YAML 1.1 scalar resolution and edge
//! cases match yaml.v2 exactly. See `PORTING.md`.
//!
//! Entry points:
//! - [`unmarshal`]: `yaml.Unmarshal(data, &v)` with `v interface{}`.
//! - [`unmarshal_str_map`]: `yaml.Unmarshal(data, &m)` with
//!   `m := make(map[string]interface{})`.
//! - [`Decoder`]: `yaml.NewDecoder` (multi-document streams).
//! - [`metadecoders`]: neohugo `metadecoders.Default.UnmarshalToMap` /
//!   `Unmarshal` for the YAML format, producing `go_value` values.

#![allow(clippy::too_many_arguments)] // faithful port of libyaml signatures
#![allow(clippy::collapsible_else_if, clippy::collapsible_if)] // keep Go control flow
#![allow(clippy::nonminimal_bool)] // keep Go boolean expressions verbatim

mod decode;
pub mod encode;
mod gostd;
pub mod metadecoders;
mod parserc;
mod readerc;
mod resolve;
mod scannerc;
mod yamlh;

use std::collections::{BTreeMap, HashMap};
use std::fmt;

pub use resolve::GO_NAN_BITS;

// ---------------------------------------------------------------------------
// Errors

/// The kind of a decoding error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// A fatal error (Go: `yaml: ...` raised through `fail`/`failf`):
    /// reader, scanner and parser errors, bad tags, unknown anchors, ...
    Fatal,
    /// Go's `*yaml.TypeError`: values that could not be stored into the
    /// target type. The individual messages are in [`Error::type_errors`].
    Type(Vec<Vec<u8>>),
}

/// A yaml.v2 decoding error. [`Error::message_bytes`] is exactly Go's
/// `err.Error()` (it can contain non-UTF-8 bytes copied from the input);
/// `Display` renders it lossily.
#[derive(Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    msg: Vec<u8>,
}

impl Error {
    pub(crate) fn fatal(msg: impl Into<Vec<u8>>) -> Error {
        Error {
            kind: ErrorKind::Fatal,
            msg: msg.into(),
        }
    }

    // Go: yaml.go:(*TypeError).Error
    pub(crate) fn type_error(errors: Vec<Vec<u8>>) -> Error {
        let mut msg = b"yaml: unmarshal errors:\n  ".to_vec();
        msg.extend_from_slice(&errors.join(&b"\n  "[..]));
        Error {
            kind: ErrorKind::Type(errors),
            msg,
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// The messages of a TypeError (empty for fatal errors).
    pub fn type_errors(&self) -> &[Vec<u8>] {
        match &self.kind {
            ErrorKind::Type(v) => v,
            ErrorKind::Fatal => &[],
        }
    }

    /// Go's `err.Error()`, byte for byte.
    pub fn message_bytes(&self) -> &[u8] {
        &self.msg
    }

    /// Go's `err.Error()`, with invalid UTF-8 replaced by U+FFFD.
    pub fn message(&self) -> String {
        String::from_utf8_lossy(&self.msg).into_owned()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.msg))
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "yaml::Error({:?})", String::from_utf8_lossy(&self.msg))
    }
}

impl std::error::Error for Error {}

// ---------------------------------------------------------------------------
// Values

/// A value as yaml.v2 decodes it into `interface{}`.
///
/// yaml.v2 never produces `int64` on 64-bit platforms (every integer that
/// fits is an `int`), `uint64` only above `MaxInt64`, and keeps timestamps
/// as strings when the target is `interface{}`.
#[derive(Clone, Debug, Default)]
pub enum Yaml {
    /// `nil`.
    #[default]
    Nil,
    Bool(bool),
    /// Go `int`.
    Int(i64),
    /// Go `uint64`.
    Uint64(u64),
    /// Go `float64`. `.nan` has Go's `math.NaN()` bits ([`GO_NAN_BITS`]).
    Float64(f64),
    /// Go `string` (bytes; `!!binary` values may not be UTF-8).
    String(Vec<u8>),
    /// `[]interface{}`.
    Seq(Vec<Yaml>),
    /// `map[interface{}]interface{}`.
    Map(IfaceMap),
}

impl Yaml {
    pub fn is_nil(&self) -> bool {
        matches!(self, Yaml::Nil)
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Yaml::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[Yaml]> {
        match self {
            Yaml::Seq(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&IfaceMap> {
        match self {
            Yaml::Map(m) => Some(m),
            _ => None,
        }
    }

    fn is_leaf(&self) -> bool {
        match self {
            Yaml::Seq(v) => v.is_empty(),
            Yaml::Map(m) => m.entries.is_empty(),
            _ => true,
        }
    }
}

/// Values can nest ~20000 levels deep (yaml.v2's depth limits); drop them
/// iteratively so that dropping never overflows the stack.
impl Drop for Yaml {
    fn drop(&mut self) {
        let mut stack: Vec<Yaml> = Vec::new();
        match self {
            Yaml::Seq(v) => {
                if v.iter().all(Yaml::is_leaf) {
                    return;
                }
                stack.append(v);
            }
            Yaml::Map(m) => {
                if m.entries.iter().all(|(k, v)| k.is_leaf() && v.is_leaf()) {
                    return;
                }
                m.index.clear();
                for (k, v) in m.entries.drain(..) {
                    stack.push(k);
                    stack.push(v);
                }
            }
            _ => return,
        }
        while let Some(mut y) = stack.pop() {
            match &mut y {
                Yaml::Seq(v) => stack.append(v),
                Yaml::Map(m) => {
                    m.index.clear();
                    for (k, v) in m.entries.drain(..) {
                        stack.push(k);
                        stack.push(v);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Hashable form of an `interface{}` map key, following Go's `==` on
/// interface values (dynamic type and value; `+0 == -0`; NaN != NaN).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum KeyRepr {
    Nil,
    Bool(bool),
    Int(i64),
    Uint64(u64),
    Float(u64),
    Str(Vec<u8>),
}

fn key_repr(k: &Yaml) -> Option<KeyRepr> {
    Some(match k {
        Yaml::Nil => KeyRepr::Nil,
        Yaml::Bool(b) => KeyRepr::Bool(*b),
        Yaml::Int(i) => KeyRepr::Int(*i),
        Yaml::Uint64(u) => KeyRepr::Uint64(*u),
        Yaml::Float64(f) => {
            if f.is_nan() {
                return None; // NaN keys never compare equal.
            }
            if *f == 0.0 {
                KeyRepr::Float(0)
            } else {
                KeyRepr::Float(f.to_bits())
            }
        }
        Yaml::String(s) => KeyRepr::Str(s.clone()),
        Yaml::Seq(_) | Yaml::Map(_) => return None,
    })
}

/// Go's `map[interface{}]interface{}` as decoded by yaml.v2.
///
/// Entries are kept in first-insertion (document) order; Go maps are
/// unordered, so consumers that care about order must sort, as Go code
/// does. Setting an existing key replaces both the stored key and the
/// value (Go updates interface keys on assignment).
#[derive(Clone, Debug, Default)]
pub struct IfaceMap {
    entries: Vec<(Yaml, Yaml)>,
    /// Key index, built once the map has more than `INDEX_MIN` entries.
    index: HashMap<KeyRepr, usize>,
}

const INDEX_MIN: usize = 8;

impl IfaceMap {
    pub fn new() -> IfaceMap {
        IfaceMap::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn find(&self, r: &KeyRepr) -> Option<usize> {
        if self.entries.len() > INDEX_MIN {
            return self.index.get(r).copied();
        }
        self.entries
            .iter()
            .position(|(k, _)| key_repr(k).as_ref() == Some(r))
    }

    /// Go: `m[k] = v`.
    pub fn insert(&mut self, k: Yaml, v: Yaml) {
        let r = key_repr(&k);
        if let Some(r) = &r
            && let Some(i) = self.find(r)
        {
            self.entries[i] = (k, v);
            return;
        }
        // A new entry (keys without a repr — NaN — are always new: NaN != NaN).
        self.entries.push((k, v));
        let n = self.entries.len();
        if n == INDEX_MIN + 1 {
            // Build the index for all entries. This must also happen when
            // the entry that crosses the threshold has no repr (a NaN key),
            // otherwise `find` would consult an empty index.
            self.index.clear();
            for (i, (k, _)) in self.entries.iter().enumerate() {
                if let Some(r) = key_repr(k) {
                    self.index.insert(r, i);
                }
            }
        } else if n > INDEX_MIN + 1
            && let Some(r) = r
        {
            self.index.insert(r, n - 1);
        }
    }

    /// Go: `v, ok := m[k]`.
    pub fn get(&self, k: &Yaml) -> Option<&Yaml> {
        let r = key_repr(k)?;
        self.find(&r).map(|i| &self.entries[i].1)
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }

    /// Entries in first-insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&Yaml, &Yaml)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

/// Go's `map[string]interface{}` (the top-level target of
/// [`unmarshal_str_map`]). Keys are bytes, iterated in sorted order.
#[derive(Clone, Debug, Default)]
pub struct StrMap {
    entries: BTreeMap<Vec<u8>, Yaml>,
}

impl StrMap {
    pub fn new() -> StrMap {
        StrMap::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn insert(&mut self, k: Vec<u8>, v: Yaml) {
        self.entries.insert(k, v);
    }

    pub fn get(&self, k: &[u8]) -> Option<&Yaml> {
        self.entries.get(k)
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &Yaml)> {
        self.entries.iter()
    }

    pub(crate) fn into_entries(self) -> BTreeMap<Vec<u8>, Yaml> {
        self.entries
    }
}

/// Ordering used for `%#v` map keys (Go's internal/fmtsort).
///
/// Keys of the same dynamic type compare by value. Keys of different types
/// compare by the address of their type descriptors, which depends on the
/// linker's layout; the rank below is the order observed in go1.27.1
/// darwin/arm64 binaries (nil, string, uint64, int, float64, bool).
pub(crate) fn fmtsort_compare(a: &Yaml, b: &Yaml) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    fn rank(y: &Yaml) -> u8 {
        match y {
            Yaml::Nil => 0,
            Yaml::String(_) => 1,
            Yaml::Uint64(_) => 2,
            Yaml::Int(_) => 3,
            Yaml::Float64(_) => 4,
            Yaml::Bool(_) => 5,
            Yaml::Seq(_) => 6,
            Yaml::Map(_) => 7,
        }
    }
    match (a, b) {
        (Yaml::Bool(x), Yaml::Bool(y)) => x.cmp(y),
        (Yaml::Int(x), Yaml::Int(y)) => x.cmp(y),
        (Yaml::Uint64(x), Yaml::Uint64(y)) => x.cmp(y),
        (Yaml::Float64(x), Yaml::Float64(y)) => {
            if x < y {
                Ordering::Less
            } else if x > y {
                Ordering::Greater
            } else if x.is_nan() && !y.is_nan() {
                Ordering::Less
            } else if !x.is_nan() && y.is_nan() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
        (Yaml::String(x), Yaml::String(y)) => x.cmp(y),
        _ => rank(a).cmp(&rank(b)),
    }
}

// ---------------------------------------------------------------------------
// Unmarshal

/// Nesting depth above which decoding runs on a dedicated thread with a
/// large stack (the decoder is recursive, like Go's).
const DEEP_NESTING: usize = 200;

/// Run `f` on the current thread, or on a big-stack thread for deeply
/// nested documents (Go's goroutine stacks grow to 1 GB). `depth` is
/// [`decode::NodeParser::decode_depth`]: the recursion depth including
/// alias expansion.
pub(crate) fn with_stack<T: Send>(depth: usize, f: impl FnOnce() -> T + Send) -> T {
    if depth <= DEEP_NESTING {
        return f();
    }
    // Go's maximum goroutine stack is 1 GB; alias chains can make the
    // bound large, so cap the reservation there.
    let size = depth
        .saturating_mul(64 * 1024)
        .saturating_add(64 * 1024 * 1024)
        .min(1 << 30);
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(size)
            .spawn_scoped(scope, f)
            .expect("spawn yaml decoder thread")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

/// Go: `yaml.Unmarshal(data, &v)` with `var v interface{}`.
///
/// Decodes the first document only. An empty stream gives `Yaml::Nil`.
/// A `*yaml.TypeError` cannot occur for this target.
pub fn unmarshal(data: &[u8]) -> Result<Yaml, Error> {
    unmarshal_with(data, false)
}

/// Go: `yaml.UnmarshalStrict(data, &v)` (`strict == true`) or
/// `yaml.Unmarshal` into `interface{}`.
pub fn unmarshal_with(data: &[u8], strict: bool) -> Result<Yaml, Error> {
    let mut p = decode::NodeParser::new(data);
    let node = p.parse()?;
    let Some(node) = node else {
        return Ok(Yaml::Nil);
    };
    let depth = p.decode_depth(node);
    let nodes = &p.nodes;
    with_stack(depth, || {
        let mut d = decode::Decoder::new(nodes, strict);
        let mut v = Yaml::Nil;
        d.unmarshal(node, &mut decode::Out::Iface(&mut v))?;
        if !d.terrors.is_empty() {
            return Err(Error::type_error(std::mem::take(&mut d.terrors)));
        }
        Ok(v)
    })
}

/// Go: `m := make(map[string]interface{}); err := yaml.Unmarshal(data, &m)`.
///
/// Returns the map (`None` when the document is `null`, which sets Go's map
/// to nil). Nested maps are `map[interface{}]interface{}` ([`Yaml::Map`]).
/// Values that cannot be stored (e.g. a sequence as a key) produce a
/// TypeError, as in Go.
pub fn unmarshal_str_map(data: &[u8]) -> Result<Option<StrMap>, Error> {
    let mut p = decode::NodeParser::new(data);
    let node = p.parse()?;
    let Some(node) = node else {
        return Ok(Some(StrMap::new()));
    };
    let depth = p.decode_depth(node);
    let nodes = &p.nodes;
    with_stack(depth, || {
        let mut d = decode::Decoder::new(nodes, false);
        let mut m = Some(StrMap::new());
        d.unmarshal(node, &mut decode::Out::TopMap(&mut m))?;
        if !d.terrors.is_empty() {
            return Err(Error::type_error(std::mem::take(&mut d.terrors)));
        }
        Ok(m)
    })
}

/// Go: `yaml.Decoder` — reads a stream of documents.
pub struct Decoder {
    parser: decode::NodeParser,
    strict: bool,
}

impl Decoder {
    /// Go: `yaml.NewDecoder(bytes.NewReader(data))`.
    pub fn new(data: &[u8]) -> Decoder {
        Decoder {
            parser: decode::NodeParser::new_from_reader(data),
            strict: false,
        }
    }

    /// Go: `(*Decoder).SetStrict`.
    pub fn set_strict(&mut self, strict: bool) {
        self.strict = strict;
    }

    /// Go: `(*Decoder).Decode(&v)` with `var v interface{}`. Returns `None`
    /// at the end of the stream (Go: `io.EOF`).
    pub fn decode(&mut self) -> Option<Result<Yaml, Error>> {
        self.parser.nodes.clear();
        let node = match self.parser.parse() {
            Ok(Some(n)) => n,
            Ok(None) => return None,
            Err(e) => return Some(Err(e)),
        };
        let depth = self.parser.decode_depth(node);
        let nodes = &self.parser.nodes;
        let strict = self.strict;
        Some(with_stack(depth, || {
            let mut d = decode::Decoder::new(nodes, strict);
            let mut v = Yaml::Nil;
            d.unmarshal(node, &mut decode::Out::Iface(&mut v))?;
            if !d.terrors.is_empty() {
                return Err(Error::type_error(std::mem::take(&mut d.terrors)));
            }
            Ok(v)
        }))
    }
}

// ---------------------------------------------------------------------------
// Test support: a canonical, typed dump shared with the Go oracle
// (tools/go-oracle/go-yaml).

/// Escape bytes for dumps: printable ASCII except `"` and `\` as is,
/// everything else as `\xHH`.
#[doc(hidden)]
pub fn dump_escape(s: &[u8]) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for &c in s {
        if (0x20..0x7F).contains(&c) && c != b'"' && c != b'\\' {
            out.push(c as char);
        } else {
            out.push_str(&format!("\\x{c:02x}"));
        }
    }
    out
}

/// Canonical typed dump of a decoded value (see the Go oracle).
#[doc(hidden)]
pub fn dump(v: &Yaml) -> String {
    match v {
        Yaml::Nil => "nil".to_string(),
        Yaml::Bool(b) => format!("bool:{b}"),
        Yaml::Int(i) => format!("int:{i}"),
        Yaml::Uint64(u) => format!("uint64:{u}"),
        Yaml::Float64(f) => format!("float64:{:016x}", f.to_bits()),
        Yaml::String(s) => format!("str:\"{}\"", dump_escape(s)),
        Yaml::Seq(items) => {
            let parts: Vec<String> = items.iter().map(dump).collect();
            format!("[{}]", parts.join(","))
        }
        Yaml::Map(m) => {
            let mut parts: Vec<(String, String)> =
                m.iter().map(|(k, v)| (dump(k), dump(v))).collect();
            parts.sort();
            let parts: Vec<String> = parts.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
            format!("imap{{{}}}", parts.join(","))
        }
    }
}

/// Canonical typed dump of a `map[string]interface{}` result.
#[doc(hidden)]
pub fn dump_str_map(m: &Option<StrMap>) -> String {
    match m {
        None => "smap(nil)".to_string(),
        Some(m) => {
            let parts: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("str:\"{}\"={}", dump_escape(k), dump(v)))
                .collect();
            format!("smap{{{}}}", parts.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Yaml {
        Yaml::String(x.as_bytes().to_vec())
    }

    /// Regression: a NaN key crossing the index threshold (9th entry) used
    /// to leave the key index empty, so later duplicates of the first 8
    /// keys were appended instead of replacing (Go: `m[k] = v` replaces).
    #[test]
    fn iface_map_index_after_nan_key() {
        let mut m = IfaceMap::new();
        for k in ["a", "b", "c", "d", "e", "f", "g", "h"] {
            m.insert(s(k), Yaml::Int(1));
        }
        m.insert(Yaml::Float64(f64::NAN), Yaml::Int(9));
        m.insert(s("a"), Yaml::Int(10));
        m.insert(Yaml::Float64(0.0), Yaml::Int(1));
        m.insert(Yaml::Float64(-0.0), Yaml::Int(2));
        assert_eq!(m.len(), 10);
        assert!(matches!(m.get(&s("a")), Some(Yaml::Int(10))));
        // Go updates the stored key: -0.0 replaced 0.0.
        let z: Vec<_> = m
            .iter()
            .filter(|(k, _)| matches!(k, Yaml::Float64(f) if *f == 0.0))
            .collect();
        assert_eq!(z.len(), 1);
        assert!(matches!(z[0].0, Yaml::Float64(f) if f.is_sign_negative()));
        assert!(matches!(z[0].1, Yaml::Int(2)));
    }
}
