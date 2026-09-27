//! The subset of golang.org/x/text@v0.26.0/unicode/norm used by colltab,
//! with the Unicode 15.0.0 tables of `tables15.0.0.go` (the file selected by
//! the `go1.21` build tag) extracted into `data/norm.bin`.
//!
//! Ported: the form tries (`nfcTrie`/`nfkcTrie` lookup + sparse blocks),
//! `compInfo`, the `Properties` accessors, `Form.FirstBoundary`,
//! `streamSafe.next`/`isMax` and Hangul decomposition (the only input colltab
//! ever passes to `NFD.Append`).

use std::sync::OnceLock;

use crate::blob::Blob;
use crate::goutf8::{encode_rune, rune_start};

/// `norm.MaxSegmentSize` (= maxByteBufferSize = utf8.UTFMax * (maxNonStarters + 2)).
pub const MAX_SEGMENT_SIZE: usize = 128;

const MAX_NON_STARTERS: u8 = 30;

const QC_INFO_MASK: u8 = 0x3F;
const HEADER_LEN_MASK: u8 = 0x3F;
const HEADER_FLAGS_MASK: u8 = 0xC0;

/// A normalization form.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

struct Trie {
    values: Vec<u16>,
    index: Vec<u16>,
    sparse_offset: Vec<u16>,
    // (value, lo, hi)
    sparse_values: Vec<(u16, u8, u8)>,
    cutoff: u32,
}

struct Tables {
    ccc: &'static [u8],
    decomps: &'static [u8],
    first_ccc: u16,
    first_leading_ccc: u16,
    first_ccc_zero_except: u16,
    first_starter_with_n_lead: u16,
    nfc: Trie,
    nfkc: Trie,
    version: &'static str,
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let b = Blob::parse(include_bytes!("../data/norm.bin"));
        let consts = b.u32s("consts");
        let cut = b.u32s("cutoffs");
        let sparse = |name: &str| {
            b.u32s(name)
                .into_iter()
                .map(|v| (v as u16, (v >> 16) as u8, (v >> 24) as u8))
                .collect::<Vec<_>>()
        };
        Tables {
            ccc: b.bytes("ccc"),
            decomps: b.bytes("decomps"),
            // consts: firstMulti, firstCCC, endMulti, firstLeadingCCC,
            // firstCCCZeroExcept, firstStarterWithNLead, lastDecomp, maxDecomp
            first_ccc: consts[1] as u16,
            first_leading_ccc: consts[3] as u16,
            first_ccc_zero_except: consts[4] as u16,
            first_starter_with_n_lead: consts[5] as u16,
            nfc: Trie {
                values: b.u16s("nfcValues"),
                index: b.bytes("nfcIndex").iter().map(|&x| x as u16).collect(),
                sparse_offset: b.u16s("nfcSparseOffset"),
                sparse_values: sparse("nfcSparseValues"),
                cutoff: cut[0],
            },
            nfkc: Trie {
                values: b.u16s("nfkcValues"),
                index: b.u16s("nfkcIndex"),
                sparse_offset: b.u16s("nfkcSparseOffset"),
                sparse_values: sparse("nfkcSparseValues"),
                cutoff: cut[1],
            },
            version: b.str("Version"),
        }
    })
}

/// The Unicode version of the norm tables (x/text `norm.Version`).
pub fn version() -> &'static str {
    tables().version
}

impl Trie {
    // Go: unicode/norm/trie.go:sparseBlocks.lookup
    fn sparse_lookup(&self, n: u32, b: u8) -> u16 {
        let offset = self.sparse_offset[n as usize];
        let header = self.sparse_values[offset as usize];
        let mut lo = offset + 1;
        let mut hi = lo + header.1 as u16;
        while lo < hi {
            let m = lo + (hi - lo) / 2;
            let r = self.sparse_values[m as usize];
            if r.1 <= b && b <= r.2 {
                return r.0.wrapping_add(((b - r.1) as u16).wrapping_mul(header.0));
            }
            if b < r.1 {
                hi = m;
            } else {
                lo = m + 1;
            }
        }
        0
    }

    // Go: unicode/norm/tables15.0.0.go:nfcTrie.lookupValue / nfkcTrie.lookupValue
    fn lookup_value(&self, n: u32, b: u8) -> u16 {
        if n < self.cutoff {
            self.values[((n << 6) + b as u32) as usize]
        } else {
            self.sparse_lookup(n - self.cutoff, b)
        }
    }

    // Go: unicode/norm/tables15.0.0.go:nfcTrie.lookup (identical for nfkcTrie)
    fn lookup(&self, s: &[u8]) -> (u16, usize) {
        let c0 = s[0];
        match c0 {
            0x00..=0x7F => (self.values[c0 as usize], 1),
            0x80..=0xC1 => (0, 1),
            0xC2..=0xDF => {
                if s.len() < 2 {
                    return (0, 0);
                }
                let i = self.index[c0 as usize];
                let c1 = s[1];
                if !(0x80..0xC0).contains(&c1) {
                    return (0, 1);
                }
                (self.lookup_value(i as u32, c1), 2)
            }
            0xE0..=0xEF => {
                if s.len() < 3 {
                    return (0, 0);
                }
                let mut i = self.index[c0 as usize];
                let c1 = s[1];
                if !(0x80..0xC0).contains(&c1) {
                    return (0, 1);
                }
                let o = ((i as u32) << 6) + c1 as u32;
                i = self.index[o as usize];
                let c2 = s[2];
                if !(0x80..0xC0).contains(&c2) {
                    return (0, 2);
                }
                (self.lookup_value(i as u32, c2), 3)
            }
            0xF0..=0xF7 => {
                if s.len() < 4 {
                    return (0, 0);
                }
                let mut i = self.index[c0 as usize];
                let c1 = s[1];
                if !(0x80..0xC0).contains(&c1) {
                    return (0, 1);
                }
                let mut o = ((i as u32) << 6) + c1 as u32;
                i = self.index[o as usize];
                let c2 = s[2];
                if !(0x80..0xC0).contains(&c2) {
                    return (0, 2);
                }
                o = ((i as u32) << 6) + c2 as u32;
                i = self.index[o as usize];
                let c3 = s[3];
                if !(0x80..0xC0).contains(&c3) {
                    return (0, 3);
                }
                (self.lookup_value(i as u32, c3), 4)
            }
            _ => (0, 1),
        }
    }
}

/// Normalization properties of a rune (Go `norm.Properties`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Properties {
    size: u8,
    ccc: u8,
    tccc: u8,
    n_lead: u8,
    flags: u8,
    index: u16,
}

impl Properties {
    // Go: unicode/norm/forminfo.go:BoundaryBefore
    pub fn boundary_before(&self) -> bool {
        self.ccc == 0 && !self.combines_backward()
    }

    // Go: unicode/norm/forminfo.go:BoundaryAfter
    pub fn boundary_after(&self) -> bool {
        self.is_inert()
    }

    fn combines_backward(&self) -> bool {
        self.flags & 0x8 != 0
    }

    fn is_inert(&self) -> bool {
        self.flags & QC_INFO_MASK == 0 && self.ccc == 0
    }

    fn n_leading_non_starters(&self) -> u8 {
        self.n_lead
    }

    fn n_trailing_non_starters(&self) -> u8 {
        self.flags & 0x03
    }

    // Go: unicode/norm/forminfo.go:Decomposition
    pub fn decomposition(&self) -> &'static [u8] {
        if self.index == 0 {
            return &[];
        }
        let d = tables().decomps;
        let i = self.index as usize;
        let n = (d[i] & HEADER_LEN_MASK) as usize;
        &d[i + 1..i + 1 + n]
    }

    // Go: unicode/norm/forminfo.go:Size
    pub fn size(&self) -> usize {
        self.size as usize
    }

    // Go: unicode/norm/forminfo.go:CCC
    pub fn ccc(&self) -> u8 {
        if self.index >= tables().first_ccc_zero_except {
            return 0;
        }
        tables().ccc[self.ccc as usize]
    }

    // Go: unicode/norm/forminfo.go:LeadCCC
    pub fn lead_ccc(&self) -> u8 {
        tables().ccc[self.ccc as usize]
    }

    // Go: unicode/norm/forminfo.go:TrailCCC
    pub fn trail_ccc(&self) -> u8 {
        tables().ccc[self.tccc as usize]
    }
}

// Go: unicode/norm/forminfo.go:compInfo
fn comp_info(v: u16, sz: usize) -> Properties {
    let t = tables();
    if v == 0 {
        return Properties {
            size: sz as u8,
            ..Default::default()
        };
    } else if v >= 0x8000 {
        let mut p = Properties {
            size: sz as u8,
            ccc: v as u8,
            tccc: v as u8,
            flags: (v >> 8) as u8,
            ..Default::default()
        };
        if p.ccc > 0 || p.combines_backward() {
            p.n_lead = p.flags & 0x3;
        }
        return p;
    }
    // has decomposition
    let h = t.decomps[v as usize];
    let f = ((h & HEADER_FLAGS_MASK) >> 2) | 0x4;
    let mut p = Properties {
        size: sz as u8,
        flags: f,
        index: v,
        ..Default::default()
    };
    let mut v = v;
    if v >= t.first_ccc {
        v += (h & HEADER_LEN_MASK) as u16 + 1;
        let c = t.decomps[v as usize];
        p.tccc = c >> 2;
        p.flags |= c & 0x3;
        if v >= t.first_leading_ccc {
            p.n_lead = c & 0x3;
            if v >= t.first_starter_with_n_lead {
                // We were tricked. Remove the decomposition.
                p.flags &= 0x03;
                p.index = 0;
                return p;
            }
            p.ccc = t.decomps[v as usize + 1];
        }
    }
    p
}

impl Form {
    fn trie(self) -> &'static Trie {
        match self {
            Form::Nfc | Form::Nfd => &tables().nfc,
            Form::Nfkc | Form::Nfkd => &tables().nfkc,
        }
    }

    // Go: unicode/norm/forminfo.go:Form.Properties
    /// Properties of the first rune in `s` (`s` must be non-empty).
    pub fn properties(self, s: &[u8]) -> Properties {
        let (v, sz) = self.trie().lookup(s);
        comp_info(v, sz)
    }

    // Go: unicode/norm/normalize.go:Form.FirstBoundary / firstBoundary
    /// Position of the first segment boundary in `b`, or -1.
    pub fn first_boundary(self, b: &[u8]) -> isize {
        let nsrc = b.len();
        let mut i = skip_continuation_bytes(b, 0);
        if i >= nsrc {
            return -1;
        }
        let mut ss = StreamSafe(0);
        loop {
            let info = self.properties(&b[i..]);
            if info.size == 0 {
                return -1;
            }
            if ss.next(info) != SsState::Success {
                return i as isize;
            }
            i += info.size as usize;
            if i >= nsrc {
                if !info.boundary_after() && !ss.is_max() {
                    return -1;
                }
                return nsrc as isize;
            }
        }
    }
}

// Go: unicode/norm/input.go:skipContinuationBytes
fn skip_continuation_bytes(b: &[u8], mut p: usize) -> usize {
    while p < b.len() && !rune_start(b[p]) {
        p += 1;
    }
    p
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SsState {
    Success,
    Starter,
    Overflow,
}

struct StreamSafe(u8);

impl StreamSafe {
    // Go: unicode/norm/composition.go:streamSafe.next
    fn next(&mut self, p: Properties) -> SsState {
        debug_assert!(self.0 <= MAX_NON_STARTERS, "streamSafe was not reset");
        let n = p.n_leading_non_starters();
        self.0 = self.0.wrapping_add(n);
        if self.0 > MAX_NON_STARTERS {
            self.0 = 0;
            return SsState::Overflow;
        }
        if n == 0 {
            self.0 = p.n_trailing_non_starters();
            return SsState::Starter;
        }
        SsState::Success
    }

    // Go: unicode/norm/composition.go:streamSafe.isMax
    fn is_max(&self) -> bool {
        self.0 == MAX_NON_STARTERS
    }
}

const HANGUL_BASE: i32 = 0xAC00;
const HANGUL_END: i32 = HANGUL_BASE + JAMO_LVT_COUNT;
const JAMO_L_BASE: i32 = 0x1100;
const JAMO_V_BASE: i32 = 0x1161;
const JAMO_T_BASE: i32 = 0x11A7;
const JAMO_T_COUNT: i32 = 28;
const JAMO_V_COUNT: i32 = 21;
const JAMO_LVT_COUNT: i32 = 19 * 21 * 28;

// Go: unicode/norm/composition.go:decomposeHangul
fn decompose_hangul(buf: &mut [u8], r: i32) -> usize {
    const JAMO_UTF8_LEN: usize = 3;
    let mut r = r - HANGUL_BASE;
    let x = r % JAMO_T_COUNT;
    r /= JAMO_T_COUNT;
    encode_rune(buf, JAMO_L_BASE + r / JAMO_V_COUNT);
    encode_rune(&mut buf[JAMO_UTF8_LEN..], JAMO_V_BASE + r % JAMO_V_COUNT);
    if x != 0 {
        encode_rune(&mut buf[2 * JAMO_UTF8_LEN..], JAMO_T_BASE + x);
        return 3 * JAMO_UTF8_LEN;
    }
    2 * JAMO_UTF8_LEN
}

/// `norm.NFD.Append(nil, s...)` restricted to what colltab uses: `s` is the
/// UTF-8 encoding of exactly one Hangul syllable (U+AC00..U+D7A3). The NFD of
/// a lone precomposed syllable is its algorithmic L V (T) decomposition.
pub(crate) fn nfd_hangul_syllable(s: &[u8], out: &mut [u8; 16]) -> usize {
    let (r, sz) = crate::goutf8::decode_rune(s);
    if sz == s.len() && (HANGUL_BASE..HANGUL_END).contains(&r) {
        return decompose_hangul(out, r);
    }
    // Not reachable from colltab; keep the bytes unchanged.
    let n = s.len().min(out.len());
    out[..n].copy_from_slice(&s[..n]);
    n
}
