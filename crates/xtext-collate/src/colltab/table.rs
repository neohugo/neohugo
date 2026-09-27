//! Port of golang.org/x/text/internal/colltab/table.go.
//!
//! Go's `source` abstraction (either `string` or `[]byte`) collapses to
//! `&[u8]`: both variants run the same code (`matchContractionString` is a
//! verbatim copy of `matchContraction`).

use super::collelem::{
    CeType, Elem, MAX_TERTIARY, implicit_primary, make_implicit_ce, split_contract_index,
    split_decompose, split_expand_index,
};
use super::contract::{ContractTrieSet, scanner};
use super::trie::Trie;
use super::weighter::Weighter;
use crate::goutf8::{RUNE_SELF, decode_rune};
use crate::norm::{Form, MAX_SEGMENT_SIZE, nfd_hangul_syllable};

/// `colltab.Table`: all collation data for a given collation ordering.
#[derive(Clone, Copy)]
pub struct Table {
    /// main trie
    pub index: Trie,
    /// expansion info
    pub expand_elem: &'static [u32],
    /// contraction info
    pub contract_tries: &'static ContractTrieSet,
    pub contract_elem: &'static [u32],
    pub max_contract_len: usize,
    pub variable_top: u32,
}

impl Weighter for Table {
    // Go: internal/colltab/table.go:Table.AppendNext / AppendNextString
    fn append_next(&self, w: &mut Vec<Elem>, s: &[u8]) -> usize {
        self.append_next_src(w, s)
    }

    // Go: internal/colltab/table.go:Table.Top
    fn top(&self) -> u32 {
        self.variable_top
    }
}

impl Table {
    // Go: internal/colltab/table.go:Table.appendNext
    /// Appends the weights corresponding to the next rune or contraction in
    /// `src`. If a contraction is matched to a discontinuous sequence of
    /// runes, the weights for the interstitial runes are appended as well.
    /// Returns the number of bytes consumed from `src`.
    fn append_next_src(&self, w: &mut Vec<Elem>, src: &[u8]) -> usize {
        let (mut ce, mut sz) = self.index.lookup(src);
        let tp = ce.ctype();
        if tp == CeType::Normal {
            if ce.0 == 0 {
                let (r, _) = decode_rune(src);
                const HANGUL_SIZE: usize = 3;
                const FIRST_HANGUL: i32 = 0xAC00;
                const LAST_HANGUL: i32 = 0xD7A3;
                if (FIRST_HANGUL..=LAST_HANGUL).contains(&r) {
                    // TODO: performance can be considerably improved here.
                    let n = sz;
                    let mut buf = [0u8; 16]; // Used for decomposing Hangul.
                    let bl = nfd_hangul_syllable(&src[..HANGUL_SIZE], &mut buf);
                    let mut b = &buf[..bl];
                    while !b.is_empty() {
                        let (ce2, sz2) = self.index.lookup(b);
                        w.push(ce2);
                        // Go: b = b[sz:]; jamo are complete 3-byte runes so sz2 > 0.
                        b = &b[sz2.max(1).min(b.len())..];
                    }
                    return n;
                }
                ce = make_implicit_ce(implicit_primary(r));
            }
            w.push(ce);
        } else if tp == CeType::ExpansionIndex {
            self.append_expansion(w, ce);
        } else if tp == CeType::ContractionIndex {
            let tail = &src[sz..];
            let n = self.match_contraction(w, ce, tail);
            sz += n;
        } else if tp == CeType::Decompose {
            // Decompose using NFKD and replace tertiary weights.
            let (t1, t2) = split_decompose(ce);
            let mut i = w.len();
            let mut nfkd = Form::Nfkd.properties(src).decomposition();
            while !nfkd.is_empty() {
                let p = self.append_next_src(w, nfkd);
                // Go loops forever if p == 0; decompositions are valid UTF-8
                // so this cannot happen with the x/text tables.
                nfkd = &nfkd[p.max(1).min(nfkd.len())..];
            }
            // Go indexes w[i] unconditionally (panics if nothing was appended,
            // which the tables never cause).
            if i < w.len() {
                w[i] = w[i].update_tertiary(t1);
                i += 1;
                if i < w.len() {
                    w[i] = w[i].update_tertiary(t2);
                    i += 1;
                    while i < w.len() {
                        w[i] = w[i].update_tertiary(MAX_TERTIARY);
                        i += 1;
                    }
                }
            }
        }
        sz
    }

    // Go: internal/colltab/table.go:Table.appendExpansion
    fn append_expansion(&self, w: &mut Vec<Elem>, ce: Elem) {
        let mut i = split_expand_index(ce);
        let n = self.expand_elem[i] as usize;
        i += 1;
        for &ce in &self.expand_elem[i..i + n] {
            w.push(Elem(ce));
        }
    }

    // Go: internal/colltab/table.go:Table.matchContraction / matchContractionString
    fn match_contraction(&self, w: &mut Vec<Elem>, ce: Elem, suffix: &[u8]) -> usize {
        let (index, n, offset) = split_contract_index(ce);

        let mut scan = scanner(self.contract_tries, index, n, suffix);
        let mut buf = [0u8; MAX_SEGMENT_SIZE];
        let mut bufp = 0usize;
        let mut p = scan.scan(0);

        if !scan.done && p < suffix.len() && suffix[p] >= RUNE_SELF {
            // By now we should have filtered most cases.
            let mut p0 = p;
            let mut bufn = 0usize;
            let mut rune = Form::Nfd.properties(&suffix[p..]);
            p += rune.size();
            if rune.lead_ccc() != 0 {
                let mut prev_cc = rune.trail_ccc();
                // A gap may only occur in the last normalization segment.
                // This also ensures that len(scan.s) < norm.MaxSegmentSize.
                let end = Form::Nfd.first_boundary(&suffix[p..]);
                if end != -1 {
                    scan.s = &suffix[..p + end as usize];
                }
                while p < suffix.len() && !scan.done && suffix[p] >= RUNE_SELF {
                    rune = Form::Nfd.properties(&suffix[p..]);
                    let ccc = rune.lead_ccc();
                    if ccc == 0 || prev_cc >= ccc {
                        break;
                    }
                    prev_cc = rune.trail_ccc();
                    let pp = scan.scan(p);
                    if pp != p {
                        // Copy the interstitial runes for later processing.
                        // Go: bufn += copy(buf[bufn:], suffix[p0:p])
                        let src = &suffix[p0..p];
                        let k = src.len().min(MAX_SEGMENT_SIZE - bufn);
                        buf[bufn..bufn + k].copy_from_slice(&src[..k]);
                        bufn += k;
                        if scan.pindex == pp {
                            bufp = bufn;
                        }
                        p = pp;
                        p0 = pp;
                    } else {
                        p += rune.size();
                    }
                }
            }
        }
        // Append weights for the matched contraction, which may be an expansion.
        let (i, n) = scan.result();
        let ce = Elem(self.contract_elem[i + offset]);
        if ce.ctype() == CeType::Normal {
            w.push(ce);
        } else {
            self.append_expansion(w, ce);
        }
        // Append weights for the runes in the segment not part of the contraction.
        let mut b = &buf[..bufp];
        while !b.is_empty() {
            let p = self.append_next_src(w, b);
            // Go loops forever on p == 0 (a rune cut by the 128-byte buffer);
            // unreachable for stream-safe segments.
            b = &b[p.max(1).min(b.len())..];
        }
        n
    }
}
