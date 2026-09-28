//! Port of `golang.org/x/net@v0.41.0/html/atom` (`atom.go`; the table is generated into
//! `atom_table.rs`).
//!
//! Go looks names up in a perfect hash table; the table holds exactly the atoms below, so a
//! binary search over them by name gives the same answers.

pub use super::atom_table::*;

/// Go: `atom.Atom` — an integer code for a string (0 = not an atom). The high 24 bits are the
/// offset in `atomText`, the low 8 bits the length.
pub type Atom = u32;

/// Go: `Atom.String()`.
// Go: html/atom/atom.go:String
pub fn string(a: Atom) -> &'static [u8] {
    let start = (a >> 8) as usize;
    let n = (a & 0xff) as usize;
    if start + n > ATOM_TEXT.len() {
        return b"";
    }
    &ATOM_TEXT[start..start + n]
}

/// Go: `atom.Lookup(s)` — the atom whose name is s, or 0.
// Go: html/atom/atom.go:Lookup
pub fn lookup(s: &[u8]) -> Atom {
    match BY_NAME.binary_search_by(|(k, _)| (*k).cmp(s)) {
        Ok(i) => BY_NAME[i].1,
        Err(_) => 0,
    }
}

/// Go: `atom.String(s)` — the atom's string if s is an atom, else s itself.
// Go: html/atom/atom.go:String
pub fn string_of(s: &[u8]) -> Vec<u8> {
    let a = lookup(s);
    if a != 0 {
        return string(a).to_vec();
    }
    s.to_vec()
}
