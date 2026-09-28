//! Port of `common/maps/maps.go`.
//!
//! Owner: Wave B task T01 (common-values).

use std::sync::Arc;

use go_value::{GoString, List, Map, MapType, SliceType, Value};

use crate::herrors::{Error, Result};

// Go: common/maps/maps.go:ToStringMapE
/// ToStringMapE converts in to `map[string]interface {}`: Params as they are, a
/// `map[string]string` value by value, everything else through `cast.ToStringMapE`. The result is
/// typed `map[string]interface {}` (Go's static result type).
pub fn to_string_map_e(v: &Value) -> Result<Map> {
    match v {
        Value::Map(m) if m.ty == MapType::Params => {
            Ok(Map::with_entries(MapType::StringAny, m.entries.clone()))
        }
        Value::Map(m) if m.ty == MapType::StringString => {
            Ok(Map::with_entries(MapType::StringAny, m.entries.clone()))
        }
        // A nil Params is returned as it is (a nil map, empty here); a nil map[string]string
        // gives a new empty map.
        Value::TypedNil(t) if matches!(&**t, "maps.Params" | "map[string]string") => {
            Ok(Map::new(MapType::StringAny))
        }
        _ => crate::cast::caste::to_string_map_e(v),
    }
}

// Go: common/maps/maps.go:ToStringMap
/// ToStringMap converts in to `map[string]interface {}` (empty on error).
pub fn to_string_map(v: &Value) -> Map {
    to_string_map_e(v).unwrap_or_else(|_| Map::new(MapType::StringAny))
}

// Go: common/maps/maps.go:ToStringMapStringE
/// ToStringMapStringE converts in to `map[string]string`.
pub fn to_string_map_string_e(v: &Value) -> Result<Map> {
    let m = to_string_map_e(v)?;
    crate::cast::caste::to_string_map_string_e(&Value::map(m))
}

// Go: common/maps/maps.go:ToStringMapString
/// ToStringMapString converts in to `map[string]string` (empty on error).
pub fn to_string_map_string(v: &Value) -> Map {
    to_string_map_string_e(v).unwrap_or_else(|_| Map::new(MapType::StringString))
}

// Go: common/maps/maps.go:ToStringMapBool
/// ToStringMapBool converts in to `map[string]bool`.
pub fn to_string_map_bool(v: &Value) -> Map {
    let m = to_string_map(v);
    crate::cast::caste::to_string_map_bool(&Value::map(m))
}

// Go: common/maps/maps.go:ToSliceStringMap
/// ToSliceStringMap converts in to `[]map[string]interface {}`: such a slice as it is, Params as
/// a one-element slice, the `map[string]interface {}` elements of a `[]interface {}`.
pub fn to_slice_string_map(v: &Value) -> Result<Vec<Map>> {
    match v {
        Value::List(l) if l.ty == SliceType::MapStringAny => {
            Ok(l.items.iter().filter_map(|e| e.as_map().cloned()).collect())
        }
        Value::TypedNil(t) if &**t == "[]map[string]interface {}" => Ok(Vec::new()),
        Value::Map(m) if m.ty == MapType::Params => Ok(vec![Map::with_entries(
            MapType::StringAny,
            m.entries.clone(),
        )]),
        // Go: `[]map[string]any{v}` with v a nil Params (a nil map, empty here).
        Value::TypedNil(t) if &**t == "maps.Params" => Ok(vec![Map::new(MapType::StringAny)]),
        Value::List(l) if l.ty == SliceType::Any => {
            let mut s = Vec::new();
            for entry in &l.items {
                if let Value::Map(vv) = entry
                    && vv.ty == MapType::StringAny
                {
                    s.push((**vv).clone());
                }
            }
            Ok(s)
        }
        _ => Err(Error::new(format!(
            "unable to cast {} of type {} to []map[string]interface{{}}",
            String::from_utf8_lossy(&go_fmt::sprintf("%#v", std::slice::from_ref(v))),
            v.go_type_name()
        ))),
    }
}

// Go: common/maps/maps.go:LookupEqualFold
/// LookupEqualFold finds key in m with case insensitive equality checks; returns (value, the
/// map's key). The exact key wins; otherwise Go returns a random one of several fold-equal keys,
/// here the first in byte order.
pub fn lookup_equal_fold<'a>(m: &'a Map, key: &[u8]) -> Option<(&'a Value, &'a GoString)> {
    if let Some((k, v)) = m.entries.get_key_value(key) {
        return Some((v, k));
    }
    for (k, v) in &m.entries {
        if go_unicode::strings::equal_fold(k, key) {
            return Some((v, k));
        }
    }
    None
}

// Go: common/maps/maps.go:MergeShallow
/// MergeShallow merges src into dst, but only if the key does not already exist in dst. The keys
/// are compared case insensitively. (src is visited in byte order; Go's order is random, which
/// matters only when src has several keys that differ only in case and none is in dst.)
pub fn merge_shallow(dst: &mut Map, src: &Map) {
    for (k, v) in &src.entries {
        let found = dst
            .entries
            .keys()
            .any(|dk| go_unicode::strings::equal_fold(dk, k));
        if !found {
            dst.entries.insert(k.clone(), v.clone());
        }
    }
}

/// Go: `maps.keyRename`.
#[derive(Clone, Debug)]
struct KeyRename {
    pattern: glob_lite::Glob,
    new_key: GoString,
}

/// Go: `maps.KeyRenamer` — renames keys in a map by glob patterns over `/`-joined, lower-cased
/// key paths (config key aliases such as `menu` -> `menus`).
#[derive(Clone, Debug, Default)]
pub struct KeyRenamer {
    renames: Vec<KeyRename>,
}

impl KeyRenamer {
    // Go: common/maps/maps.go:NewKeyRenamer
    /// NewKeyRenamer creates a new KeyRenamer given a list of pattern and new key value pairs.
    pub fn new(pattern_keys: &[&str]) -> Result<Self> {
        let mut renames = Vec::new();
        let mut i = 0;
        while i < pattern_keys.len() {
            let lower = go_unicode::strings::to_lower(pattern_keys[i].as_bytes()).into_owned();
            let g = glob_lite::Glob::compile(&lower, b'/')?;
            // Go indexes patternKeys[i+1] (panics on an odd count).
            let new_key = pattern_keys[i + 1];
            renames.push(KeyRename {
                pattern: g,
                new_key: GoString::from(new_key),
            });
            i += 2;
        }
        Ok(KeyRenamer { renames })
    }

    // Go: common/maps/maps.go:getNewKey
    fn get_new_key(&self, key_path: &[u8]) -> Option<&GoString> {
        self.renames
            .iter()
            .find(|m| m.pattern.matches(key_path))
            .map(|m| &m.new_key)
    }

    // Go: common/maps/maps.go:Rename
    /// Rename renames the keys in the given map according to the patterns in the current
    /// KeyRenamer.
    pub fn rename(&self, m: &mut Map) {
        self.rename_path(b"", m);
    }

    // Go: common/maps/maps.go:keyPath
    fn key_path(k1: &[u8], k2: &[u8]) -> Vec<u8> {
        let k1 = go_unicode::strings::to_lower(k1);
        let k2 = go_unicode::strings::to_lower(k2);
        if k1.is_empty() {
            return k2.into_owned();
        }
        let mut p = k1.into_owned();
        p.push(b'/');
        p.extend_from_slice(&k2);
        p
    }

    // Go: common/maps/maps.go:renamePath
    /// Keys are visited in byte order (Go ranges over the map while renaming; a renamed key
    /// overwrites an existing key of the new name).
    fn rename_path(&self, parent_key_path: &[u8], m: &mut Map) {
        let keys: Vec<GoString> = m.entries.keys().cloned().collect();
        for k in keys {
            let key_path = Self::key_path(parent_key_path, &k);
            if let Some(Value::Map(vv)) = m.entries.get_mut(&k) {
                // Go: `case map[string]any` (renamed in place; other map types are not descended
                // into, and Go's `map[any]any` case renames a converted copy).
                if vv.ty == MapType::StringAny {
                    self.rename_path(&key_path, Arc::make_mut(vv));
                }
            }

            if let Some(new_key) = self.get_new_key(&key_path) {
                let new_key = new_key.clone();
                if let Some(v) = m.entries.remove(&k) {
                    m.entries.insert(new_key, v);
                }
            }
        }
    }
}

// Go: common/maps/maps.go:ConvertFloat64WithNoDecimalsToInt
/// ConvertFloat64WithNoDecimalsToInt converts float64 values with no decimals to int64
/// recursively (in `map[string]interface {}` values and `[]interface {}` elements). The check
/// `v == float64(int64(v))` uses arm64's saturating conversion (the golden platform), so 2^63
/// becomes `math.MaxInt64` and NaN stays a float.
pub fn convert_float64_with_no_decimals_to_int(m: &mut Map) {
    fn no_decimals(f: f64) -> Option<i64> {
        let i = f as i64; // saturating, NaN -> 0 (arm64)
        if f == i as f64 { Some(i) } else { None }
    }
    for v in m.entries.values_mut() {
        match v {
            Value::Float(f, go_value::FloatKind::F64) => {
                if let Some(i) = no_decimals(*f) {
                    *v = Value::int64(i);
                }
            }
            Value::Map(vv) if vv.ty == MapType::StringAny => {
                convert_float64_with_no_decimals_to_int(Arc::make_mut(vv));
            }
            Value::List(vv) if vv.ty == SliceType::Any => {
                let l: &mut List = Arc::make_mut(vv);
                for vvv in l.items.iter_mut() {
                    match vvv {
                        Value::Float(f, go_value::FloatKind::F64) => {
                            if let Some(i) = no_decimals(*f) {
                                *vvv = Value::int64(i);
                            }
                        }
                        Value::Map(vvvv) if vvvv.ty == MapType::StringAny => {
                            convert_float64_with_no_decimals_to_int(Arc::make_mut(vvvv));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

/// The subset of `github.com/gobwas/glob` v0.2.3 that [`KeyRenamer`] needs: `*` (any run of
/// non-separator characters), `**` (any run), `?` (one non-separator character), `[...]` /
/// `[!...]` lists and ranges, `{a,b}` alternatives (nested), `\` escapes; the whole string must
/// match. T02's `glob` module ports gobwas/glob in full; this private copy keeps `maps` free of
/// that dependency.
mod glob_lite {
    use crate::herrors::{Error, Result};

    #[derive(Clone, Debug)]
    enum Node {
        Text(Vec<char>),
        Any,
        Super,
        Single,
        List { items: Vec<(char, char)>, not: bool },
        Alt(Vec<Vec<Node>>),
    }

    #[derive(Clone, Debug)]
    pub struct Glob {
        nodes: Vec<Node>,
        sep: char,
    }

    struct Parser {
        r: Vec<char>,
        pos: usize,
    }

    fn eof() -> Error {
        Error::new("unexpected end of input")
    }

    fn to_chars(s: &[u8]) -> Vec<char> {
        go_unicode::utf8::to_runes(s)
            .iter()
            .map(|&c| char::from_u32(c as u32).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }

    impl Parser {
        fn peek(&self) -> Option<char> {
            self.r.get(self.pos).copied()
        }

        fn next(&mut self) -> Option<char> {
            let c = self.peek();
            if c.is_some() {
                self.pos += 1;
            }
            c
        }

        // Go: github.com/gobwas/glob syntax/lexer/lexer.go:fetchText
        /// Text with backslash escapes, up to (not including) a breaker.
        fn text(&mut self, breakers: &[char]) -> Vec<char> {
            let mut text = Vec::new();
            let mut escaped = false;
            while let Some(c) = self.peek() {
                if !escaped {
                    if c == '\\' {
                        escaped = true;
                        self.pos += 1;
                        continue;
                    }
                    if breakers.contains(&c) {
                        break;
                    }
                }
                escaped = false;
                text.push(c);
                self.pos += 1;
            }
            text
        }

        // Go: github.com/gobwas/glob syntax/lexer/lexer.go:fetchItem
        /// A sequence until EOF, or (inside terms) until `,`/`}`.
        fn seq(&mut self, in_terms: bool) -> Result<Vec<Node>> {
            let mut nodes = Vec::new();
            while let Some(c) = self.peek() {
                match c {
                    ',' | '}' if in_terms => break,
                    '{' => {
                        self.pos += 1;
                        let mut alts = vec![self.seq(true)?];
                        loop {
                            match self.next() {
                                Some(',') => alts.push(self.seq(true)?),
                                Some('}') => break,
                                _ => return Err(eof()),
                            }
                        }
                        nodes.push(Node::Alt(alts));
                    }
                    '*' => {
                        self.pos += 1;
                        if self.peek() == Some('*') {
                            self.pos += 1;
                            nodes.push(Node::Super);
                        } else {
                            nodes.push(Node::Any);
                        }
                    }
                    '?' => {
                        self.pos += 1;
                        nodes.push(Node::Single);
                    }
                    '[' => {
                        self.pos += 1;
                        nodes.push(self.list()?);
                    }
                    _ => {
                        let text = if in_terms {
                            self.text(&['?', '*', '[', '{', '}', ','])
                        } else {
                            self.text(&['?', '*', '[', '{'])
                        };
                        if !text.is_empty() {
                            nodes.push(Node::Text(text));
                        }
                    }
                }
            }
            Ok(nodes)
        }

        // Go: github.com/gobwas/glob syntax/lexer/lexer.go:fetchRange
        /// `[`, an optional `!`, one `lo-hi` range or a list of characters, then `]`.
        fn list(&mut self) -> Result<Node> {
            let mut not = false;
            if self.peek() == Some('!') {
                not = true;
                self.pos += 1;
            }
            let Some(c) = self.peek() else {
                return Err(eof());
            };
            let items = if self.r.get(self.pos + 1) == Some(&'-') {
                self.pos += 2;
                let Some(hi) = self.next() else {
                    return Err(eof());
                };
                vec![(c, hi)]
            } else {
                self.text(&[']']).into_iter().map(|c| (c, c)).collect()
            };
            match self.next() {
                Some(']') => Ok(Node::List { items, not }),
                Some(_) => Err(Error::new("expected close range character")),
                None => Err(eof()),
            }
        }
    }

    impl Glob {
        // Go: github.com/gobwas/glob glob.go:Compile
        pub fn compile(pattern: &[u8], sep: u8) -> Result<Glob> {
            if go_unicode::utf8::to_runes(pattern).contains(&go_unicode::utf8::RUNE_ERROR) {
                return Err(Error::new("could not read rune"));
            }
            let mut p = Parser {
                r: to_chars(pattern),
                pos: 0,
            };
            let nodes = p.seq(false)?;
            if p.pos != p.r.len() {
                return Err(eof());
            }
            Ok(Glob {
                nodes,
                sep: sep as char,
            })
        }

        // Go: github.com/gobwas/glob (compiled matcher).Match
        pub fn matches(&self, s: &[u8]) -> bool {
            match_nodes(&self.nodes, &to_chars(s), self.sep)
        }
    }

    fn match_nodes(nodes: &[Node], s: &[char], sep: char) -> bool {
        let Some((first, rest)) = nodes.split_first() else {
            return s.is_empty();
        };
        match first {
            Node::Text(t) => s.starts_with(t) && match_nodes(rest, &s[t.len()..], sep),
            Node::Single => !s.is_empty() && s[0] != sep && match_nodes(rest, &s[1..], sep),
            Node::List { items, not } => {
                !s.is_empty()
                    && items.iter().any(|&(lo, hi)| lo <= s[0] && s[0] <= hi) != *not
                    && match_nodes(rest, &s[1..], sep)
            }
            Node::Any => {
                for i in 0..=s.len() {
                    if match_nodes(rest, &s[i..], sep) {
                        return true;
                    }
                    if i < s.len() && s[i] == sep {
                        return false;
                    }
                }
                false
            }
            Node::Super => (0..=s.len()).any(|i| match_nodes(rest, &s[i..], sep)),
            Node::Alt(alts) => alts.iter().any(|alt| {
                let mut seq = alt.clone();
                seq.extend_from_slice(rest);
                match_nodes(&seq, s, sep)
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/maps.go (236 lines; 12/16 funcs executed)
//   types: keyRename, KeyRenamer
// OK L27-41: ToStringMapE(in any) (map[string]any, error)
// OK L46-56: ToParamsAndPrepare(in any) (Params, error) (params.rs)
// OK L59-65: MustToParamsAndPrepare(in any) Params (params.rs)
// OK L68-71: ToStringMap(in any) map[string]any
// OK L74-80: ToStringMapStringE(in any) (map[string]string, error)
// OK L83-86: ToStringMapString(in any) map[string]string
// OK L89-92: ToStringMapBool(in any) map[string]bool
// OK L95-112: ToSliceStringMap(in any) ([]map[string]any, error)
// OK L115-126: LookupEqualFold[T any | string](m map[string]T, key string) (T, string, bool)
// OK L130-143: MergeShallow(dst, src map[string]any)
// OK L157-168: NewKeyRenamer(patternKeys ...string) (KeyRenamer, error)
// OK L170-178: (r KeyRenamer) getNewKey(keyPath string) string
// OK L182-184: (r KeyRenamer) Rename(m map[string]any)
// OK L186-192: (KeyRenamer) keyPath(k1, k2 string) string
// OK L194-211: (r KeyRenamer) renamePath(parentKeyPath string, m map[string]any)
// OK L214-236: ConvertFloat64WithNoDecimalsToInt(m map[string]any)
// ---------------------------------------------------------------------------
