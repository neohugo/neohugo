//! Port of golang.org/x/text/internal/colltab/contract.go.
//!
//! Go keeps two verbatim copies of the scanner (`[]byte` and `string`); the
//! port has one over `&[u8]`.

use crate::goutf8::rune_start;

/// One node of a contraction trie (`struct{ L, H, N, I uint8 }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CtEntry {
    pub l: u8,
    pub h: u8,
    pub n: u8,
    pub i: u8,
}

/// `colltab.ContractTrieSet`.
pub type ContractTrieSet = [CtEntry];

/// `ctScanner`: `states` is represented as an offset into the full set.
pub(crate) struct CtScanner<'a> {
    set: &'a ContractTrieSet,
    states: usize,
    pub(crate) s: &'a [u8],
    n: usize,
    index: usize,
    pub(crate) pindex: usize,
    pub(crate) done: bool,
}

const FINAL: u8 = 0;
const NO_INDEX: u8 = 0xFF;

// Go: internal/colltab/contract.go:ContractTrieSet.scanner
pub(crate) fn scanner<'a>(
    t: &'a ContractTrieSet,
    index: usize,
    n: usize,
    b: &'a [u8],
) -> CtScanner<'a> {
    CtScanner {
        set: t,
        states: index,
        s: b,
        n,
        index: 0,
        pindex: 0,
        done: false,
    }
}

impl CtScanner<'_> {
    // Go: internal/colltab/contract.go:ctScanner.result
    pub(crate) fn result(&self) -> (usize, usize) {
        (self.index, self.pindex)
    }

    // Go: internal/colltab/contract.go:ctScanner.scan
    /// Matches the longest suffix at the current location in the input and
    /// returns the number of bytes consumed.
    pub(crate) fn scan(&mut self, mut p: usize) -> usize {
        let mut pr = p; // the p at the rune start
        let str = self.s;
        let (mut states, mut n) = (self.states, self.n);
        let mut i = 0;
        while i < n && p < str.len() {
            let e = self.set[states + i];
            let c = str[p];
            if c >= e.l {
                if e.l == c {
                    p += 1;
                    if e.i != NO_INDEX {
                        self.index = e.i as usize;
                        self.pindex = p;
                    }
                    if e.n != FINAL {
                        // i, states, n = 0, states[int(e.H)+n:], int(e.N)
                        states += e.h as usize + n;
                        n = e.n as usize;
                        i = 0;
                        if p >= str.len() || rune_start(str[p]) {
                            self.states = states;
                            self.n = n;
                            pr = p;
                        }
                    } else {
                        self.done = true;
                        return p;
                    }
                    continue;
                } else if e.n == FINAL && c <= e.h {
                    p += 1;
                    self.done = true;
                    self.index = (c - e.l) as usize + e.i as usize;
                    self.pindex = p;
                    return p;
                }
            }
            i += 1;
        }
        pr
    }
}
