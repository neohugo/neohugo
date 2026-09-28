//! Port of `github.com/gobwas/glob@v0.2.3` `match/*.go` and `util/strings/strings.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).
//!
//! Matching is over Go string bytes: Go ranges over runes (an invalid byte is `U+FFFD` of width
//! 1, whose `utf8.RuneLen` is 3) and slices by byte offsets that are sometimes rune counts
//! (`BTree`, `Row`). Those upstream quirks are reproduced, including two panics: `Row` slicing
//! past the end of its input (a zero-length matcher at the end, e.g. `a{,}` on "a") and
//! `LastIndexAnyRunes` with a non-ASCII separator. The segment pools of `match/segments.go` only
//! recycle buffers and do not change results.

use go_unicode::{strings, utf8};

use super::lexer::{Rune, index_rune};

const LEN_ONE: isize = 1;
const LEN_ZERO: isize = 0;
const LEN_NO: isize = -1;

/// Go: `match.Matcher` (the value types of package `match`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Matcher {
    Any {
        separators: Vec<Rune>,
    },
    AnyOf {
        matchers: Vec<Matcher>,
    },
    BTree(Box<BTree>),
    Contains {
        needle: Vec<u8>,
        not: bool,
    },
    EveryOf {
        matchers: Vec<Matcher>,
    },
    List {
        list: Vec<Rune>,
        not: bool,
    },
    Max {
        limit: isize,
    },
    Min {
        limit: isize,
    },
    Nothing,
    Prefix {
        prefix: Vec<u8>,
    },
    PrefixAny {
        prefix: Vec<u8>,
        separators: Vec<Rune>,
    },
    PrefixSuffix {
        prefix: Vec<u8>,
        suffix: Vec<u8>,
    },
    Range {
        lo: Rune,
        hi: Rune,
        not: bool,
    },
    Row {
        matchers: Vec<Matcher>,
        runes_length: isize,
        segments: Vec<isize>,
    },
    Single {
        separators: Vec<Rune>,
    },
    Suffix {
        suffix: Vec<u8>,
    },
    SuffixAny {
        suffix: Vec<u8>,
        separators: Vec<Rune>,
    },
    Super,
    Text {
        str: Vec<u8>,
        runes_length: isize,
        bytes_length: isize,
        segments: Vec<isize>,
    },
}

/// Go: `match.BTree`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BTree {
    pub value: Matcher,
    pub left: Option<Matcher>,
    pub right: Option<Matcher>,
    pub value_length_runes: isize,
    pub left_length_runes: isize,
    pub right_length_runes: isize,
    pub length_runes: isize,
}

// Go: github.com/gobwas/glob match/any.go:NewAny
pub fn new_any(s: &[Rune]) -> Matcher {
    Matcher::Any {
        separators: s.to_vec(),
    }
}

// Go: github.com/gobwas/glob match/any_of.go:NewAnyOf
pub fn new_any_of(m: Vec<Matcher>) -> Matcher {
    Matcher::AnyOf { matchers: m }
}

// Go: github.com/gobwas/glob match/contains.go:NewContains
pub fn new_contains(needle: &[u8], not: bool) -> Matcher {
    Matcher::Contains {
        needle: needle.to_vec(),
        not,
    }
}

// Go: github.com/gobwas/glob match/every_of.go:NewEveryOf
pub fn new_every_of(m: Vec<Matcher>) -> Matcher {
    Matcher::EveryOf { matchers: m }
}

// Go: github.com/gobwas/glob match/list.go:NewList
pub fn new_list(list: Vec<Rune>, not: bool) -> Matcher {
    Matcher::List { list, not }
}

// Go: github.com/gobwas/glob match/max.go:NewMax
pub fn new_max(l: isize) -> Matcher {
    Matcher::Max { limit: l }
}

// Go: github.com/gobwas/glob match/min.go:NewMin
pub fn new_min(l: isize) -> Matcher {
    Matcher::Min { limit: l }
}

// Go: github.com/gobwas/glob match/nothing.go:NewNothing
pub fn new_nothing() -> Matcher {
    Matcher::Nothing
}

// Go: github.com/gobwas/glob match/prefix.go:NewPrefix
pub fn new_prefix(p: &[u8]) -> Matcher {
    Matcher::Prefix { prefix: p.to_vec() }
}

// Go: github.com/gobwas/glob match/prefix_any.go:NewPrefixAny
pub fn new_prefix_any(s: &[u8], sep: &[Rune]) -> Matcher {
    Matcher::PrefixAny {
        prefix: s.to_vec(),
        separators: sep.to_vec(),
    }
}

// Go: github.com/gobwas/glob match/prefix_suffix.go:NewPrefixSuffix
pub fn new_prefix_suffix(p: &[u8], s: &[u8]) -> Matcher {
    Matcher::PrefixSuffix {
        prefix: p.to_vec(),
        suffix: s.to_vec(),
    }
}

// Go: github.com/gobwas/glob match/range.go:NewRange
pub fn new_range(lo: Rune, hi: Rune, not: bool) -> Matcher {
    Matcher::Range { lo, hi, not }
}

// Go: github.com/gobwas/glob match/row.go:NewRow
pub fn new_row(len: isize, m: Vec<Matcher>) -> Matcher {
    Matcher::Row {
        matchers: m,
        runes_length: len,
        segments: vec![len],
    }
}

// Go: github.com/gobwas/glob match/single.go:NewSingle
pub fn new_single(s: &[Rune]) -> Matcher {
    Matcher::Single {
        separators: s.to_vec(),
    }
}

// Go: github.com/gobwas/glob match/suffix.go:NewSuffix
pub fn new_suffix(s: &[u8]) -> Matcher {
    Matcher::Suffix { suffix: s.to_vec() }
}

// Go: github.com/gobwas/glob match/suffix_any.go:NewSuffixAny
pub fn new_suffix_any(s: &[u8], sep: &[Rune]) -> Matcher {
    Matcher::SuffixAny {
        suffix: s.to_vec(),
        separators: sep.to_vec(),
    }
}

// Go: github.com/gobwas/glob match/super.go:NewSuper
pub fn new_super() -> Matcher {
    Matcher::Super
}

// Go: github.com/gobwas/glob match/text.go:NewText
pub fn new_text(s: &[u8]) -> Matcher {
    Matcher::Text {
        str: s.to_vec(),
        runes_length: utf8::rune_count_in_string(s) as isize,
        bytes_length: s.len() as isize,
        segments: vec![s.len() as isize],
    }
}

// Go: github.com/gobwas/glob match/btree.go:NewBTree
pub fn new_btree(value: Matcher, left: Option<Matcher>, right: Option<Matcher>) -> Matcher {
    let mut tree = BTree {
        value_length_runes: 0,
        left_length_runes: 0,
        right_length_runes: 0,
        length_runes: 0,
        value,
        left,
        right,
    };

    let mut len_ok = true;
    tree.value_length_runes = tree.value.len();
    if tree.value_length_runes == -1 {
        len_ok = false;
    }

    if let Some(l) = &tree.left {
        tree.left_length_runes = l.len();
        if tree.left_length_runes == -1 {
            len_ok = false;
        }
    }

    if let Some(r) = &tree.right {
        tree.right_length_runes = r.len();
        if tree.right_length_runes == -1 {
            len_ok = false;
        }
    }

    if len_ok {
        tree.length_runes =
            tree.left_length_runes + tree.value_length_runes + tree.right_length_runes;
    } else {
        tree.length_runes = -1;
    }

    Matcher::BTree(Box::new(tree))
}

/// Go `utf8.RuneLen(r)` for a rune from ranging over a string (1..=4; `U+FFFD` is 3).
fn rune_len(r: Rune) -> isize {
    utf8::rune_len(r)
}

/// Go `for i := range s` (the rune start offsets).
fn rune_starts(s: &[u8]) -> impl Iterator<Item = (usize, Rune)> + '_ {
    utf8::runes(s)
}

fn index(s: &[u8], sub: &[u8]) -> isize {
    strings::index(s, sub)
}

// Go: github.com/gobwas/glob util/strings/strings.go:IndexAnyRunes
pub(crate) fn index_any_runes(s: &[u8], rs: &[Rune]) -> isize {
    for &r in rs {
        let i = strings::index_rune(s, r);
        if i != -1 {
            return i;
        }
    }

    -1
}

// Go: github.com/gobwas/glob util/strings/strings.go:LastIndexAnyRunes
pub(crate) fn last_index_any_runes(s: &[u8], rs: &[Rune]) -> isize {
    for &r in rs {
        let mut i: isize = -1;
        if (0..utf8::RUNE_SELF).contains(&r) {
            i = strings::last_index_byte(s, r as u8);
        } else {
            let mut sub = s;
            while !sub.is_empty() {
                // Go searches `s`, not `sub`.
                let j = strings::index_rune(s, r);
                if j == -1 {
                    break;
                }
                i = j;
                let from = (i + 1) as usize;
                if from > sub.len() {
                    panic!(
                        "runtime error: slice bounds out of range [{}:{}]",
                        from,
                        sub.len()
                    );
                }
                sub = &sub[from..];
            }
        }
        if i != -1 {
            return i;
        }
    }
    -1
}

// Go: github.com/gobwas/glob match/match.go:appendMerge
/// Merges and sorts given already SORTED and UNIQUE segments.
fn append_merge(target: Vec<isize>, sub: &[isize]) -> Vec<isize> {
    let (lt, ls) = (target.len(), sub.len());
    let mut out: Vec<isize> = Vec::with_capacity(lt + ls);

    let (mut x, mut y) = (0, 0);
    while x < lt || y < ls {
        if x >= lt {
            out.extend_from_slice(&sub[y..]);
            break;
        }

        if y >= ls {
            out.extend_from_slice(&target[x..]);
            break;
        }

        let x_value = target[x];
        let y_value = sub[y];

        if x_value == y_value {
            out.push(x_value);
            x += 1;
            y += 1;
        } else if x_value < y_value {
            out.push(x_value);
            x += 1;
        } else {
            out.push(y_value);
            y += 1;
        }
    }

    out
}

fn string_of_runes(rs: &[Rune]) -> String {
    String::from_utf8_lossy(&utf8::from_runes(rs)).into_owned()
}

fn string_of_bytes(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

// Go: github.com/gobwas/glob match/match.go:(Matchers).String
fn matchers_string(ms: &[Matcher]) -> String {
    ms.iter().map(|m| m.string()).collect::<Vec<_>>().join(",")
}

impl Matcher {
    /// Go: `Matcher.Match(s)`.
    pub fn is_match(&self, s: &[u8]) -> bool {
        match self {
            // Go: github.com/gobwas/glob match/any.go:(Any).Match
            Matcher::Any { separators } => index_any_runes(s, separators) == -1,

            // Go: github.com/gobwas/glob match/any_of.go:(AnyOf).Match
            Matcher::AnyOf { matchers } => matchers.iter().any(|m| m.is_match(s)),

            // Go: github.com/gobwas/glob match/btree.go:(BTree).Match
            Matcher::BTree(t) => t.is_match(s),

            // Go: github.com/gobwas/glob match/contains.go:(Contains).Match
            Matcher::Contains { needle, not } => strings::contains(s, needle) != *not,

            // Go: github.com/gobwas/glob match/every_of.go:(EveryOf).Match
            Matcher::EveryOf { matchers } => matchers.iter().all(|m| m.is_match(s)),

            // Go: github.com/gobwas/glob match/list.go:(List).Match
            Matcher::List { list, not } => {
                let (r, w) = utf8::decode_rune_in_string(s);
                if s.len() > w {
                    return false;
                }

                let in_list = index_rune(list, r) != -1;
                // Go: inList == !self.Not
                in_list != *not
            }

            // Go: github.com/gobwas/glob match/max.go:(Max).Match
            Matcher::Max { limit } => {
                let mut l = 0;
                for _ in rune_starts(s) {
                    l += 1;
                    if l > *limit {
                        return false;
                    }
                }
                true
            }

            // Go: github.com/gobwas/glob match/min.go:(Min).Match
            Matcher::Min { limit } => {
                let mut l = 0;
                for _ in rune_starts(s) {
                    l += 1;
                    if l >= *limit {
                        return true;
                    }
                }
                false
            }

            // Go: github.com/gobwas/glob match/nothing.go:(Nothing).Match
            Matcher::Nothing => s.is_empty(),

            // Go: github.com/gobwas/glob match/prefix.go:(Prefix).Match
            Matcher::Prefix { prefix } => s.starts_with(prefix),

            // Go: github.com/gobwas/glob match/prefix_any.go:(PrefixAny).Match
            Matcher::PrefixAny { prefix, separators } => {
                if !s.starts_with(prefix) {
                    return false;
                }
                index_any_runes(&s[prefix.len()..], separators) == -1
            }

            // Go: github.com/gobwas/glob match/prefix_suffix.go:(PrefixSuffix).Match
            Matcher::PrefixSuffix { prefix, suffix } => {
                s.starts_with(prefix) && s.ends_with(suffix)
            }

            // Go: github.com/gobwas/glob match/range.go:(Range).Match
            Matcher::Range { lo, hi, not } => {
                let (r, w) = utf8::decode_rune_in_string(s);
                if s.len() > w {
                    return false;
                }

                let in_range = r >= *lo && r <= *hi;

                // Go: inRange == !self.Not
                in_range != *not
            }

            // Go: github.com/gobwas/glob match/row.go:(Row).Match
            Matcher::Row {
                matchers,
                runes_length,
                ..
            } => row_len_ok(*runes_length, s) && row_match_all(matchers, s),

            // Go: github.com/gobwas/glob match/single.go:(Single).Match
            Matcher::Single { separators } => {
                let (r, w) = utf8::decode_rune_in_string(s);
                if s.len() > w {
                    return false;
                }

                index_rune(separators, r) == -1
            }

            // Go: github.com/gobwas/glob match/suffix.go:(Suffix).Match
            Matcher::Suffix { suffix } => s.ends_with(suffix),

            // Go: github.com/gobwas/glob match/suffix_any.go:(SuffixAny).Match
            Matcher::SuffixAny { suffix, separators } => {
                if !s.ends_with(suffix) {
                    return false;
                }
                index_any_runes(&s[..s.len() - suffix.len()], separators) == -1
            }

            // Go: github.com/gobwas/glob match/super.go:(Super).Match
            Matcher::Super => true,

            // Go: github.com/gobwas/glob match/text.go:(Text).Match
            Matcher::Text { str, .. } => str.as_slice() == s,
        }
    }

    /// Go: `Matcher.Index(s)` — the index of the first match and the lengths of the matches
    /// there (`-1, nil` if none).
    pub fn index(&self, s: &[u8]) -> (isize, Vec<isize>) {
        match self {
            // Go: github.com/gobwas/glob match/any.go:(Any).Index
            Matcher::Any { separators } => {
                let found = index_any_runes(s, separators);
                let s = match found {
                    -1 => s,
                    0 => return (0, vec![0]),
                    _ => &s[..found as usize],
                };

                let mut segments: Vec<isize> = rune_starts(s).map(|(i, _)| i as isize).collect();
                segments.push(s.len() as isize);

                (0, segments)
            }

            // Go: github.com/gobwas/glob match/any_of.go:(AnyOf).Index
            Matcher::AnyOf { matchers } => {
                let mut index: isize = -1;

                let mut segments: Vec<isize> = Vec::new();
                for m in matchers {
                    let (idx, seg) = m.index(s);
                    if idx == -1 {
                        continue;
                    }

                    if index == -1 || idx < index {
                        index = idx;
                        segments = seg;
                        continue;
                    }

                    if idx > index {
                        continue;
                    }

                    // here idx == index
                    segments = append_merge(segments, &seg);
                }

                if index == -1 {
                    return (-1, Vec::new());
                }

                (index, segments)
            }

            // Go: github.com/gobwas/glob match/btree.go:(BTree).Index
            Matcher::BTree(_) => (-1, Vec::new()),

            // Go: github.com/gobwas/glob match/contains.go:(Contains).Index
            Matcher::Contains { needle, not } => {
                let mut offset: isize = 0;

                let idx = index(s, needle);

                let mut s = s;
                if !*not {
                    if idx == -1 {
                        return (-1, Vec::new());
                    }

                    offset = idx + needle.len() as isize;
                    if s.len() as isize <= offset {
                        return (0, vec![offset]);
                    }
                    s = &s[offset as usize..];
                } else if idx != -1 {
                    s = &s[..idx as usize];
                }

                let mut segments: Vec<isize> =
                    rune_starts(s).map(|(i, _)| offset + i as isize).collect();
                segments.push(offset + s.len() as isize);

                (0, segments)
            }

            // Go: github.com/gobwas/glob match/every_of.go:(EveryOf).Index
            Matcher::EveryOf { matchers } => {
                let mut index: isize = 0;
                let mut offset: isize = 0;

                let mut next: Vec<isize> = Vec::new();
                let mut current: Vec<isize> = Vec::new();

                let mut sub = s;
                for (i, m) in matchers.iter().enumerate() {
                    let (idx, seg) = m.index(sub);
                    if idx == -1 {
                        return (-1, Vec::new());
                    }

                    if i == 0 {
                        current.extend_from_slice(&seg);
                    } else {
                        // clear the next
                        next.clear();

                        let delta = index - (idx + offset);
                        for &ex in &current {
                            for &n in &seg {
                                if ex + delta == n {
                                    next.push(n);
                                }
                            }
                        }

                        if next.is_empty() {
                            return (-1, Vec::new());
                        }

                        current.clear();
                        current.extend_from_slice(&next);
                    }

                    index = idx + offset;
                    sub = &s[index as usize..];
                    offset += idx;
                }

                (index, current)
            }

            // Go: github.com/gobwas/glob match/list.go:(List).Index
            Matcher::List { list, not } => {
                for (i, r) in rune_starts(s) {
                    if *not == (index_rune(list, r) == -1) {
                        return (i as isize, vec![rune_len(r)]);
                    }
                }

                (-1, Vec::new())
            }

            // Go: github.com/gobwas/glob match/max.go:(Max).Index
            Matcher::Max { limit } => {
                let mut segments = vec![0];
                let mut count = 0;
                for (i, r) in rune_starts(s) {
                    count += 1;
                    if count > *limit {
                        break;
                    }
                    segments.push(i as isize + rune_len(r));
                }

                (0, segments)
            }

            // Go: github.com/gobwas/glob match/min.go:(Min).Index
            Matcher::Min { limit } => {
                let mut count = 0;

                let c = s.len() as isize - *limit + 1;
                if c <= 0 {
                    return (-1, Vec::new());
                }

                let mut segments = Vec::new();
                for (i, r) in rune_starts(s) {
                    count += 1;
                    if count >= *limit {
                        segments.push(i as isize + rune_len(r));
                    }
                }

                if segments.is_empty() {
                    return (-1, Vec::new());
                }

                (0, segments)
            }

            // Go: github.com/gobwas/glob match/nothing.go:(Nothing).Index
            Matcher::Nothing => (0, vec![0]),

            // Go: github.com/gobwas/glob match/prefix.go:(Prefix).Index
            Matcher::Prefix { prefix } => {
                let idx = index(s, prefix);
                if idx == -1 {
                    return (-1, Vec::new());
                }

                let length = prefix.len() as isize;
                let sub: &[u8] = if s.len() as isize > idx + length {
                    &s[(idx + length) as usize..]
                } else {
                    b""
                };

                let mut segments = vec![length];
                for (i, r) in rune_starts(sub) {
                    segments.push(length + i as isize + rune_len(r));
                }

                (idx, segments)
            }

            // Go: github.com/gobwas/glob match/prefix_any.go:(PrefixAny).Index
            Matcher::PrefixAny { prefix, separators } => {
                let idx = index(s, prefix);
                if idx == -1 {
                    return (-1, Vec::new());
                }

                let n = prefix.len() as isize;
                let mut sub = &s[(idx + n) as usize..];
                let i = index_any_runes(sub, separators);
                if i > -1 {
                    sub = &sub[..i as usize];
                }

                let mut seg = vec![n];
                for (i, r) in rune_starts(sub) {
                    seg.push(n + i as isize + rune_len(r));
                }

                (idx, seg)
            }

            // Go: github.com/gobwas/glob match/prefix_suffix.go:(PrefixSuffix).Index
            Matcher::PrefixSuffix { prefix, suffix } => {
                let prefix_idx = index(s, prefix);
                if prefix_idx == -1 {
                    return (-1, Vec::new());
                }

                let suffix_len = suffix.len() as isize;
                if suffix_len <= 0 {
                    return (prefix_idx, vec![s.len() as isize - prefix_idx]);
                }

                if (s.len() as isize - prefix_idx) <= 0 {
                    return (-1, Vec::new());
                }

                let mut segments = Vec::new();
                let mut sub = &s[prefix_idx as usize..];
                loop {
                    let suffix_idx = strings::last_index(sub, suffix);
                    if suffix_idx == -1 {
                        break;
                    }

                    segments.push(suffix_idx + suffix_len);
                    sub = &sub[..suffix_idx as usize];
                }

                if segments.is_empty() {
                    return (-1, Vec::new());
                }

                segments.reverse();

                (prefix_idx, segments)
            }

            // Go: github.com/gobwas/glob match/range.go:(Range).Index
            Matcher::Range { lo, hi, not } => {
                for (i, r) in rune_starts(s) {
                    if *not != (r >= *lo && r <= *hi) {
                        return (i as isize, vec![rune_len(r)]);
                    }
                }

                (-1, Vec::new())
            }

            // Go: github.com/gobwas/glob match/row.go:(Row).Index
            Matcher::Row {
                matchers,
                runes_length,
                segments,
            } => {
                for (i, _) in rune_starts(s) {
                    if ((s.len() - i) as isize) < *runes_length {
                        break;
                    }
                    if row_match_all(matchers, &s[i..]) {
                        return (i as isize, segments.clone());
                    }
                }
                (-1, Vec::new())
            }

            // Go: github.com/gobwas/glob match/single.go:(Single).Index
            Matcher::Single { separators } => {
                for (i, r) in rune_starts(s) {
                    if index_rune(separators, r) == -1 {
                        return (i as isize, vec![rune_len(r)]);
                    }
                }

                (-1, Vec::new())
            }

            // Go: github.com/gobwas/glob match/suffix.go:(Suffix).Index
            Matcher::Suffix { suffix } => {
                let idx = index(s, suffix);
                if idx == -1 {
                    return (-1, Vec::new());
                }

                (0, vec![idx + suffix.len() as isize])
            }

            // Go: github.com/gobwas/glob match/suffix_any.go:(SuffixAny).Index
            Matcher::SuffixAny { suffix, separators } => {
                let idx = index(s, suffix);
                if idx == -1 {
                    return (-1, Vec::new());
                }

                let i = last_index_any_runes(&s[..idx as usize], separators) + 1;

                (i, vec![idx + suffix.len() as isize - i])
            }

            // Go: github.com/gobwas/glob match/super.go:(Super).Index
            Matcher::Super => {
                let mut segments: Vec<isize> = rune_starts(s).map(|(i, _)| i as isize).collect();
                segments.push(s.len() as isize);

                (0, segments)
            }

            // Go: github.com/gobwas/glob match/text.go:(Text).Index
            Matcher::Text { str, segments, .. } => {
                let index = index(s, str);
                if index == -1 {
                    return (-1, Vec::new());
                }

                (index, segments.clone())
            }
        }
    }

    /// Go: `Matcher.Len()` — the length in runes of every match, or -1 (not a collection size,
    /// so there is no `is_empty`).
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> isize {
        match self {
            Matcher::Any { .. } => LEN_NO,

            // Go: github.com/gobwas/glob match/any_of.go:(AnyOf).Len
            Matcher::AnyOf { matchers } => {
                let mut l: isize = -1;
                for m in matchers {
                    let ml = m.len();
                    if l == -1 {
                        l = ml;
                        continue;
                    } else if ml == -1 || l != ml {
                        return -1;
                    }
                }
                l
            }

            Matcher::BTree(t) => t.length_runes,
            Matcher::Contains { .. } => LEN_NO,

            // Go: github.com/gobwas/glob match/every_of.go:(EveryOf).Len
            // (upstream bug kept: `l` starts at 0, so any matcher makes it -1)
            Matcher::EveryOf { matchers } => {
                let mut l: isize = 0;
                for m in matchers {
                    let ml = m.len();
                    if l > 0 {
                        l += ml;
                    } else {
                        return -1;
                    }
                }
                l
            }

            Matcher::List { .. } => LEN_ONE,
            Matcher::Max { .. } => LEN_NO,
            Matcher::Min { .. } => LEN_NO,
            Matcher::Nothing => LEN_ZERO,
            Matcher::Prefix { .. } => LEN_NO,
            Matcher::PrefixAny { .. } => LEN_NO,
            Matcher::PrefixSuffix { .. } => LEN_NO,
            Matcher::Range { .. } => LEN_ONE,
            Matcher::Row { runes_length, .. } => *runes_length,
            Matcher::Single { .. } => LEN_ONE,
            Matcher::Suffix { .. } => LEN_NO,
            Matcher::SuffixAny { .. } => LEN_NO,
            Matcher::Super => LEN_NO,
            Matcher::Text { runes_length, .. } => *runes_length,
        }
    }

    /// Go: `Matcher.String()` (the upstream debug form, used to compare compiled trees).
    pub fn string(&self) -> String {
        match self {
            Matcher::Any { separators } => format!("<any:![{}]>", string_of_runes(separators)),
            Matcher::AnyOf { matchers } => format!("<any_of:[{}]>", matchers_string(matchers)),
            Matcher::BTree(t) => {
                let n = "<nil>";
                let l = t.left.as_ref().map_or(n.to_string(), |m| m.string());
                let r = t.right.as_ref().map_or(n.to_string(), |m| m.string());
                format!("<btree:[{}<-{}->{}]>", l, t.value.string(), r)
            }
            Matcher::Contains { needle, not } => format!(
                "<contains:{}[{}]>",
                if *not { "!" } else { "" },
                string_of_bytes(needle)
            ),
            Matcher::EveryOf { matchers } => {
                format!("<every_of:[{}]>", matchers_string(matchers))
            }
            Matcher::List { list, not } => format!(
                "<list:{}[{}]>",
                if *not { "!" } else { "" },
                string_of_runes(list)
            ),
            Matcher::Max { limit } => format!("<max:{limit}>"),
            Matcher::Min { limit } => format!("<min:{limit}>"),
            Matcher::Nothing => "<nothing>".to_string(),
            Matcher::Prefix { prefix } => format!("<prefix:{}>", string_of_bytes(prefix)),
            Matcher::PrefixAny { prefix, separators } => format!(
                "<prefix_any:{}![{}]>",
                string_of_bytes(prefix),
                string_of_runes(separators)
            ),
            Matcher::PrefixSuffix { prefix, suffix } => format!(
                "<prefix_suffix:[{},{}]>",
                string_of_bytes(prefix),
                string_of_bytes(suffix)
            ),
            Matcher::Range { lo, hi, not } => format!(
                "<range:{}[{},{}]>",
                if *not { "!" } else { "" },
                string_of_runes(&[*lo]),
                string_of_runes(&[*hi])
            ),
            Matcher::Row {
                matchers,
                runes_length,
                ..
            } => format!("<row_{}:[{}]>", runes_length, matchers_string(matchers)),
            Matcher::Single { separators } => {
                format!("<single:![{}]>", string_of_runes(separators))
            }
            Matcher::Suffix { suffix } => format!("<suffix:{}>", string_of_bytes(suffix)),
            Matcher::SuffixAny { suffix, separators } => format!(
                "<suffix_any:![{}]{}>",
                string_of_runes(separators),
                string_of_bytes(suffix)
            ),
            Matcher::Super => "<super>".to_string(),
            Matcher::Text { str, .. } => format!("<text:`{}`>", string_of_bytes(str)),
        }
    }
}

impl BTree {
    // Go: github.com/gobwas/glob match/btree.go:(BTree).Match
    fn is_match(&self, s: &[u8]) -> bool {
        let input_len = s.len() as isize;

        // self.Length, self.RLen and self.LLen are values meaning the length of runes for each part
        // here we manipulating byte length for better optimizations
        // but these checks still works, cause minLen of 1-rune string is 1 byte.
        if self.length_runes != -1 && self.length_runes > input_len {
            return false;
        }

        // try to cut unnecessary parts
        // by knowledge of length of right and left part
        let mut offset: isize = 0;
        if self.left_length_runes >= 0 {
            offset = self.left_length_runes;
        }
        let limit = if self.right_length_runes >= 0 {
            input_len - self.right_length_runes
        } else {
            input_len
        };

        while offset < limit {
            // search for matching part in substring
            let (index, segments) = self.value.index(&s[offset as usize..limit as usize]);
            if index == -1 {
                return false;
            }

            let l = &s[..(offset + index) as usize];
            let left = match &self.left {
                Some(m) => m.is_match(l),
                None => l.is_empty(),
            };

            if left {
                for &length in segments.iter().rev() {
                    // if there is no string for the right branch
                    let r: &[u8] = if input_len <= offset + index + length {
                        b""
                    } else {
                        &s[(offset + index + length) as usize..]
                    };

                    let right = match &self.right {
                        Some(m) => m.is_match(r),
                        None => r.is_empty(),
                    };

                    if right {
                        return true;
                    }
                }
            }

            let (_, step) = utf8::decode_rune_in_string(&s[(offset + index) as usize..]);
            offset += index + step as isize;
        }

        false
    }
}

// Go: github.com/gobwas/glob match/row.go:(Row).matchAll
fn row_match_all(matchers: &[Matcher], s: &[u8]) -> bool {
    let mut idx: usize = 0;
    for m in matchers {
        let length = m.len();

        let mut next: usize = 0;
        let mut i: isize = 0;
        for (n, _) in rune_starts(&s[idx..]) {
            next = n;
            i += 1;
            if i == length {
                break;
            }
        }

        if i < length {
            return false;
        }
        let end = idx + next + 1;
        if end > s.len() {
            panic!(
                "runtime error: slice bounds out of range [:{}] with length {}",
                end,
                s.len()
            );
        }
        if !m.is_match(&s[idx..end]) {
            return false;
        }

        idx += next + 1;
    }

    true
}

// Go: github.com/gobwas/glob match/row.go:(Row).lenOk
fn row_len_ok(runes_length: isize, s: &[u8]) -> bool {
    let mut i: isize = 0;
    for _ in rune_starts(s) {
        i += 1;
        if i > runes_length {
            return false;
        }
    }
    runes_length == i
}
