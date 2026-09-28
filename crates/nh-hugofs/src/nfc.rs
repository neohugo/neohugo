//! Go `golang.org/x/text/unicode/norm` `NFC.String` (NEW; x/text v0.26.0, Unicode 15.0.0).
//!
//! Owner: Wave B task T05 (hugofs-vfs).
//!
//! Hugo NFC-normalizes file names on darwin only (`hugofs/fileinfo.go:normalizeFilename`,
//! `component_fs.go:applyMeta`); the callers are behind `cfg(target_os = "macos")`, this function
//! is compiled and tested everywhere.
//!
//! The data (`nfc15.txt`) is extracted from Go's `norm` package by
//! `tools/go-oracle/nh-hugofs/nfc`: canonical combining classes, full canonical decompositions
//! and the 941 primary composites (x/text's `recompMap`). The algorithm is Unicode's (UAX #15):
//! decompose with x/text's stream-safe rule (a CGJ, U+034F, after 30 non-starters), order marks
//! canonically, recompose. `unicode-normalization` is not used because it carries a newer
//! Unicode version than Go's tables (PORTING.md).

use std::collections::HashMap;
use std::sync::OnceLock;

const DATA: &str = include_str!("nfc15.txt");

const HANGUL_S_BASE: u32 = 0xAC00;
const HANGUL_L_BASE: u32 = 0x1100;
const HANGUL_V_BASE: u32 = 0x1161;
const HANGUL_T_BASE: u32 = 0x11A7;
const HANGUL_L_COUNT: u32 = 19;
const HANGUL_V_COUNT: u32 = 21;
const HANGUL_T_COUNT: u32 = 28;
const HANGUL_N_COUNT: u32 = HANGUL_V_COUNT * HANGUL_T_COUNT;
const HANGUL_S_COUNT: u32 = HANGUL_L_COUNT * HANGUL_N_COUNT;

/// x/text `maxNonStarters`.
const MAX_NON_STARTERS: usize = 30;
/// U+034F COMBINING GRAPHEME JOINER.
const CGJ: u32 = 0x034F;

struct Tables {
    ccc: HashMap<u32, u8>,
    decomp: HashMap<u32, Vec<u32>>,
    comp: HashMap<(u32, u32), u32>,
    /// Runes that are the second element of a primary composite (x/text `combinesBackward`).
    combines_backward: std::collections::HashSet<u32>,
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = Tables {
            ccc: HashMap::new(),
            decomp: HashMap::new(),
            comp: HashMap::new(),
            combines_backward: Default::default(),
        };
        let hex = |s: &str| u32::from_str_radix(s, 16).expect("nfc15.txt: hex");
        for line in DATA.lines() {
            let mut f = line.split(' ');
            match f.next() {
                Some("C") => {
                    let r = hex(f.next().unwrap());
                    let c: u8 = f.next().unwrap().parse().expect("nfc15.txt: ccc");
                    t.ccc.insert(r, c);
                }
                Some("D") => {
                    let r = hex(f.next().unwrap());
                    t.decomp.insert(r, f.map(hex).collect());
                }
                Some("P") => {
                    let a = hex(f.next().unwrap());
                    let b = hex(f.next().unwrap());
                    let c = hex(f.next().unwrap());
                    t.comp.insert((a, b), c);
                    t.combines_backward.insert(b);
                }
                _ => {}
            }
        }
        // Hangul vowels and trailing consonants combine backward (algorithmic composition).
        for r in HANGUL_V_BASE..HANGUL_V_BASE + HANGUL_V_COUNT {
            t.combines_backward.insert(r);
        }
        for r in HANGUL_T_BASE + 1..HANGUL_T_BASE + HANGUL_T_COUNT {
            t.combines_backward.insert(r);
        }
        t
    })
}

fn ccc(r: u32) -> u8 {
    tables().ccc.get(&r).copied().unwrap_or(0)
}

/// Whether x/text counts the rune as a non-starter for the stream-safe rule.
fn is_non_starter(r: u32) -> bool {
    ccc(r) != 0 || tables().combines_backward.contains(&r)
}

fn decompose(r: u32, out: &mut Vec<u32>) {
    if (HANGUL_S_BASE..HANGUL_S_BASE + HANGUL_S_COUNT).contains(&r) {
        let s = r - HANGUL_S_BASE;
        out.push(HANGUL_L_BASE + s / HANGUL_N_COUNT);
        out.push(HANGUL_V_BASE + (s % HANGUL_N_COUNT) / HANGUL_T_COUNT);
        let t = s % HANGUL_T_COUNT;
        if t != 0 {
            out.push(HANGUL_T_BASE + t);
        }
        return;
    }
    match tables().decomp.get(&r) {
        Some(d) => out.extend_from_slice(d),
        None => out.push(r),
    }
}

fn compose_pair(a: u32, b: u32) -> Option<u32> {
    // Hangul LV and LV + T.
    if (HANGUL_L_BASE..HANGUL_L_BASE + HANGUL_L_COUNT).contains(&a)
        && (HANGUL_V_BASE..HANGUL_V_BASE + HANGUL_V_COUNT).contains(&b)
    {
        return Some(
            HANGUL_S_BASE
                + ((a - HANGUL_L_BASE) * HANGUL_V_COUNT + (b - HANGUL_V_BASE)) * HANGUL_T_COUNT,
        );
    }
    if (HANGUL_S_BASE..HANGUL_S_BASE + HANGUL_S_COUNT).contains(&a)
        && (a - HANGUL_S_BASE).is_multiple_of(HANGUL_T_COUNT)
        && (HANGUL_T_BASE + 1..HANGUL_T_BASE + HANGUL_T_COUNT).contains(&b)
    {
        return Some(a + (b - HANGUL_T_BASE));
    }
    tables().comp.get(&(a, b)).copied()
}

/// Go: `norm.NFC.String(s)`.
pub fn nfc_string(s: &str) -> String {
    // Decompose, inserting a CGJ where the stream-safe rule overflows.
    let mut d: Vec<u32> = Vec::with_capacity(s.len());
    let mut tmp = Vec::new();
    let mut ss = 0usize;
    for ch in s.chars() {
        tmp.clear();
        decompose(ch as u32, &mut tmp);
        let n_lead = tmp.iter().take_while(|&&r| is_non_starter(r)).count();
        let n_trail = if n_lead == tmp.len() {
            n_lead
        } else {
            tmp.iter().rev().take_while(|&&r| is_non_starter(r)).count()
        };
        // Go: streamSafe.next
        ss += n_lead;
        if ss > MAX_NON_STARTERS {
            d.push(CGJ);
            ss = n_lead;
        }
        if n_lead == 0 {
            ss = n_trail;
        }
        d.extend_from_slice(&tmp);
    }

    // Canonical ordering: a stable sort of every run of non-starters by combining class.
    let mut i = 0;
    while i < d.len() {
        if ccc(d[i]) == 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < d.len() && ccc(d[i]) != 0 {
            i += 1;
        }
        d[start..i].sort_by_key(|&r| ccc(r));
    }

    // Canonical composition.
    let mut out: Vec<u32> = Vec::with_capacity(d.len());
    let mut starter: Option<usize> = None;
    let mut last_class: i32 = -1;
    for &c in &d {
        let cc = ccc(c) as i32;
        if let Some(sp) = starter {
            // Not blocked: the last kept character is the starter (last_class 0 right after it)
            // or has a lower combining class.
            let blocked = !(last_class < cc || (last_class == 0 && out.len() == sp + 1));
            if !blocked && let Some(comp) = compose_pair(out[sp], c) {
                out[sp] = comp;
                continue;
            }
        }
        if cc == 0 {
            starter = Some(out.len());
        }
        last_class = cc;
        out.push(c);
    }

    out.into_iter()
        .map(|r| char::from_u32(r).expect("valid scalar value"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::nfc_string;

    #[test]
    fn nfc_basics() {
        assert_eq!(nfc_string("cafe\u{301}.md"), "café.md");
        assert_eq!(nfc_string("café.md"), "café.md");
        assert_eq!(nfc_string("\u{1100}\u{1161}\u{11a8}"), "\u{ac01}");
        assert_eq!(nfc_string("e\u{327}\u{306}"), "\u{1e1d}");
        assert_eq!(nfc_string("บทความ/ไทย.md"), "บทความ/ไทย.md");
        assert_eq!(nfc_string(""), "");
    }
}
