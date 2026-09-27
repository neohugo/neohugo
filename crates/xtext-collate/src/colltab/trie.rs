//! Port of golang.org/x/text/internal/colltab/trie.go.
//!
//! Go keeps two verbatim copies of `lookup` (`[]byte` and `string`); the port
//! has one over `&[u8]`.

use super::collelem::Elem;

const TX: u8 = 0x80;
const T2: u8 = 0xC0;
const T3: u8 = 0xE0;
const T4: u8 = 0xF0;
const T5: u8 = 0xF8;

/// `colltab.Trie`. `index0`/`values0` are the per-locale offset views of
/// `index`/`values` (Go slices `mainLookup[blockSize*lookupOffset:]`).
#[derive(Clone, Copy)]
pub struct Trie {
    pub index0: &'static [u16],
    pub values0: &'static [u32],
    pub index: &'static [u16],
    pub values: &'static [u32],
}

impl Trie {
    // Go: internal/colltab/trie.go:Trie.lookupValue
    fn lookup_value(&self, n: u16, b: u8) -> Elem {
        Elem(self.values[((n as usize) << 6) + b as usize])
    }

    // Go: internal/colltab/trie.go:Trie.lookup
    /// Returns the trie value for the first UTF-8 encoding in `s` and its
    /// width. The size is 0 if `s` does not hold enough bytes to complete the
    /// encoding. `s` must be non-empty.
    pub(crate) fn lookup(&self, s: &[u8]) -> (Elem, usize) {
        let c0 = s[0];
        if c0 < TX {
            return (Elem(self.values0[c0 as usize]), 1);
        } else if c0 < T2 {
            return (Elem(0), 1);
        } else if c0 < T3 {
            if s.len() < 2 {
                return (Elem(0), 0);
            }
            let i = self.index0[c0 as usize];
            let c1 = s[1];
            if c1 < TX || T2 <= c1 {
                return (Elem(0), 1);
            }
            return (self.lookup_value(i, c1), 2);
        } else if c0 < T4 {
            if s.len() < 3 {
                return (Elem(0), 0);
            }
            let mut i = self.index0[c0 as usize];
            let c1 = s[1];
            if c1 < TX || T2 <= c1 {
                return (Elem(0), 1);
            }
            let o = ((i as usize) << 6) + c1 as usize;
            i = self.index[o];
            let c2 = s[2];
            if c2 < TX || T2 <= c2 {
                return (Elem(0), 2);
            }
            return (self.lookup_value(i, c2), 3);
        } else if c0 < T5 {
            if s.len() < 4 {
                return (Elem(0), 0);
            }
            let mut i = self.index0[c0 as usize];
            let c1 = s[1];
            if c1 < TX || T2 <= c1 {
                return (Elem(0), 1);
            }
            let mut o = ((i as usize) << 6) + c1 as usize;
            i = self.index[o];
            let c2 = s[2];
            if c2 < TX || T2 <= c2 {
                return (Elem(0), 2);
            }
            o = ((i as usize) << 6) + c2 as usize;
            i = self.index[o];
            let c3 = s[3];
            if c3 < TX || T2 <= c3 {
                return (Elem(0), 3);
            }
            return (self.lookup_value(i, c3), 4);
        }
        // Illegal rune
        (Elem(0), 1)
    }
}
