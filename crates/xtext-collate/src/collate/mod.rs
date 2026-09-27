//! Port of golang.org/x/text@v0.26.0/collate (collate.go, index.go,
//! option.go, sort.go and the generated tables.go).

pub(crate) mod index;
pub mod option;
pub mod sort;
pub(crate) mod tables;
#[cfg(test)]
mod tests;

use std::cmp::Ordering;

use crate::colltab::collelem::{
    Elem, IDENTITY, IGNORE, MAX_QUATERNARY, QUATERNARY, SECONDARY, TERTIARY, make_quaternary,
};
use crate::colltab::iter::Iter;
use crate::colltab::match_lang;
use crate::colltab::numeric::new_numeric_weighter;
use crate::colltab::weighter::Weighter;
use crate::language::{self, Tag};
use index::get_table;
use option::{AlternateHandling, CollOption, Options, new_options};
use sort::Sorter;
pub use tables::{cldr_version, unicode_version};

/// Collator provides functionality for comparing strings for a given
/// collation order.
///
/// Like Go's `*collate.Collator` it is not safe for concurrent use: every
/// method takes `&mut self` because it reuses internal iterators/buffers.
pub struct Collator {
    options: Options,
    sorter: Sorter,
    // Reusable element buffers of Go's `_iter [2]iter` (`wa [512]Elem`).
    bufs: [Vec<Elem>; 2],
}

// Go: collate/collate.go:Supported
/// Returns the list of languages for which collating differs from its parent.
pub fn supported() -> Vec<Tag> {
    tags().to_vec()
}

/// `tags`: the parsed `availableLocales` (`language.Raw.MustParse`).
pub(crate) fn tags() -> &'static [Tag] {
    use std::sync::OnceLock;
    static TAGS: OnceLock<Vec<Tag>> = OnceLock::new();
    TAGS.get_or_init(|| {
        tables::tables()
            .available_locales
            .split(',')
            .map(|s| language::RAW.must_parse(s))
            .collect()
    })
}

impl Collator {
    /// `collate.New(language.Make(tag))`: a collator for a BCP 47 string,
    /// ignoring parse errors like Go's `language.Make`.
    pub fn new(tag: &str) -> Collator {
        Collator::from_tag(&language::make(tag), &[])
    }

    /// `collate.New(language.Make(tag), o...)`.
    pub fn with_options(tag: &str, o: &[CollOption]) -> Collator {
        Collator::from_tag(&language::make(tag), o)
    }

    // Go: collate/collate.go:New
    /// Returns a new Collator initialized for the given locale.
    pub fn from_tag(t: &Tag, o: &[CollOption]) -> Collator {
        let index = match_lang(t, tags());
        let mut c = new_collator(Box::new(get_table(tables::tables().locales[index])));

        // Set options from the user-supplied tag.
        c.options.set_from_tag(t);

        // Set the user-supplied options.
        c.options.set_options(o);

        c.init();
        c
    }

    /// The collator neohugo's `langs.NewLanguage` creates for a site
    /// language key: `collate.New(tag)` if `language.Parse(lang)` succeeds,
    /// otherwise `collate.New(language.English)`.
    pub fn for_hugo_language(lang: &str) -> Collator {
        match language::parse(lang) {
            Ok(tag) => Collator::from_tag(&tag, &[]),
            Err(_) => Collator::from_tag(&language::english(), &[]),
        }
    }

    // Go: collate/collate.go:NewFromTable
    /// Returns a new Collator for the given Weighter.
    pub fn new_from_table(w: Box<dyn Weighter>, o: &[CollOption]) -> Collator {
        let mut c = new_collator(w);
        c.options.set_options(o);
        c.init();
        c
    }

    /// The locale index into `collate.Supported()` / availableLocales that
    /// `colltab.MatchLang` selects for `t` (exposed for tests and callers
    /// that want to share tables).
    pub fn match_index(t: &Tag) -> usize {
        match_lang(t, tags())
    }

    // Go: collate/collate.go:Collator.init
    fn init(&mut self) {
        if self.options.numeric {
            let t = std::mem::replace(&mut self.options.t, Box::new(NullWeighter));
            self.options.t = Box::new(new_numeric_weighter(t));
        }
    }

    // Go: collate/collate.go:Collator.Compare
    /// Returns an integer comparing the two byte slices. The result will be 0
    /// if a==b, -1 if a < b, and +1 if a > b.
    pub fn compare(&mut self, a: &[u8], b: &[u8]) -> i32 {
        // TODO: skip identical prefixes once we have a fast way to detect if a
        // rune is part of a contraction.
        let [b0, b1] = std::mem::take(&mut self.bufs);
        let w: &dyn Weighter = &*self.options.t;
        let mut ia = CIter::new(w, b0);
        let mut ib = CIter::new(w, b1);
        ia.it.set_input(a);
        ib.it.set_input(b);
        let mut res = compare_iters(&self.options, &mut ia, &mut ib);
        if res == 0 && !self.options.ignore[IDENTITY] {
            res = match a.cmp(b) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
        }
        self.bufs = [ia.it.into_elems(), ib.it.into_elems()];
        res
    }

    // Go: collate/collate.go:Collator.CompareString
    /// Returns an integer comparing the two strings. The result will be 0 if
    /// a==b, -1 if a < b, and +1 if a > b.
    pub fn compare_string(&mut self, a: &str, b: &str) -> i32 {
        self.compare(a.as_bytes(), b.as_bytes())
    }

    // Go: collate/collate.go:Collator.Key
    /// Returns the collation key for `s`, appended to `buf`. The returned
    /// slice points into `buf` and remains valid until the next call to
    /// `buf.reset()`.
    pub fn key<'b>(&mut self, buf: &'b mut Buffer, s: &[u8]) -> &'b [u8] {
        // See https://www.unicode.org/reports/tr10/#Main_Algorithm for more details.
        let mut w = self.get_col_elems(s);
        let kn = buf.key.len();
        self.key_elems(buf, &mut w);
        self.bufs[0] = w;
        &buf.key[kn..]
    }

    // Go: collate/collate.go:Collator.KeyFromString
    pub fn key_from_string<'b>(&mut self, buf: &'b mut Buffer, s: &str) -> &'b [u8] {
        self.key(buf, s.as_bytes())
    }

    /// Convenience: the collation key of `s` as a new vector.
    pub fn key_vec(&mut self, s: &[u8]) -> Vec<u8> {
        // No 4 KiB preallocation: sort/sort_strings call this per element.
        let mut buf = Buffer::default();
        self.key(&mut buf, s);
        buf.key
    }

    // Go: collate/collate.go:Collator.key
    fn key_elems(&self, buf: &mut Buffer, w: &mut [Elem]) {
        process_weights(self.options.alternate, self.options.t.top(), w);
        self.key_from_elems(buf, w);
    }

    // Go: collate/collate.go:Collator.getColElems / getColElemsString
    fn get_col_elems(&mut self, s: &[u8]) -> Vec<Elem> {
        let b0 = std::mem::take(&mut self.bufs[0]);
        let mut i = Iter::new(&*self.options.t, b0);
        i.set_input(s);
        while i.next() {}
        i.into_elems()
    }

    // Go: collate/collate.go:Collator.keyFromElems
    /// Converts the weights ws to a compact sequence of bytes. The result
    /// will be appended to the byte buffer in buf.
    fn key_from_elems(&self, buf: &mut Buffer, ws: &[Elem]) {
        let o = &self.options;
        let key = &mut buf.key;
        for v in ws {
            let w = v.primary();
            if w > 0 {
                append_primary(key, w);
            }
        }
        if !o.ignore[SECONDARY] {
            key.extend_from_slice(&[0, 0]);
            // TODO: we can use one 0 if we can guarantee that all non-zero
            // weights are > 0xFF.
            if !o.backwards {
                for v in ws {
                    let w = v.secondary();
                    if w > 0 {
                        key.extend_from_slice(&[(w >> 8) as u8, w as u8]);
                    }
                }
            } else {
                for v in ws.iter().rev() {
                    let w = v.secondary();
                    if w > 0 {
                        key.extend_from_slice(&[(w >> 8) as u8, w as u8]);
                    }
                }
            }
        } else if o.case_level {
            key.extend_from_slice(&[0, 0]);
        }
        if !o.ignore[TERTIARY] || o.case_level {
            key.extend_from_slice(&[0, 0]);
            for v in ws {
                let w = v.tertiary();
                if w > 0 {
                    key.push(w);
                }
            }
            // Derive the quaternary weights from the options and other
            // levels. Note that we represent MaxQuaternary as 0xFF. The first
            // byte of the representation of a primary weight is always
            // smaller than 0xFF, so using this single byte value will compare
            // correctly.
            if !o.ignore[QUATERNARY] && o.alternate >= AlternateHandling::Shifted {
                if o.alternate == AlternateHandling::ShiftTrimmed {
                    let mut last_non_ffff = key.len();
                    key.push(0);
                    for v in ws {
                        let w = v.quaternary();
                        if w == MAX_QUATERNARY {
                            key.push(0xFF);
                        } else if w > 0 {
                            append_primary(key, w);
                            last_non_ffff = key.len();
                        }
                    }
                    key.truncate(last_non_ffff);
                } else {
                    key.push(0);
                    for v in ws {
                        let w = v.quaternary();
                        if w == MAX_QUATERNARY {
                            key.push(0xFF);
                        } else if w > 0 {
                            append_primary(key, w);
                        }
                    }
                }
            }
        }
    }

    // Go: collate/sort.go:Collator.Sort
    /// Sorts the strings represented by x using the rules of c (Go
    /// `sort.Sort`, pdqsort on the keys).
    pub fn sort<L: Lister + ?Sized>(&mut self, x: &mut L) {
        let n = x.len();
        let mut sorter = std::mem::take(&mut self.sorter);
        sorter.init(n);
        for i in 0..n {
            sorter.keys[i] = self.key_vec(x.bytes(i));
        }
        sorter.sort(x);
        self.sorter = sorter;
    }

    // Go: collate/sort.go:Collator.SortStrings
    /// Sorts the strings in x using the rules of c.
    pub fn sort_strings<S: AsRef<str>>(&mut self, x: &mut [S]) {
        let mut sorter = std::mem::take(&mut self.sorter);
        sorter.init(x.len());
        for (i, s) in x.iter().enumerate() {
            sorter.keys[i] = self.key_vec(s.as_ref().as_bytes());
        }
        sorter.sort(&mut SliceLister(x));
        self.sorter = sorter;
    }
}

/// A Lister can be sorted by Collator's Sort method.
pub trait Lister {
    fn len(&self) -> usize;
    fn swap(&mut self, i: usize, j: usize);
    /// Returns the bytes of the text at index i.
    fn bytes(&self, i: usize) -> &[u8];
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

struct SliceLister<'a, S>(&'a mut [S]);

impl<S: AsRef<str>> Lister for SliceLister<'_, S> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j)
    }
    fn bytes(&self, i: usize) -> &[u8] {
        self.0[i].as_ref().as_bytes()
    }
}

/// Placeholder used while moving the weighter into a numeric wrapper.
struct NullWeighter;

impl Weighter for NullWeighter {
    fn append_next(&self, _buf: &mut Vec<Elem>, _s: &[u8]) -> usize {
        0
    }
    fn top(&self) -> u32 {
        0
    }
}

// Go: collate/option.go:newCollator
fn new_collator(t: Box<dyn Weighter>) -> Collator {
    Collator {
        options: new_options(t),
        sorter: Sorter::default(),
        bufs: [Vec::with_capacity(512), Vec::with_capacity(512)],
    }
}

/// Buffer holds keys generated by Key and KeyString.
#[derive(Default, Clone, Debug)]
pub struct Buffer {
    pub(crate) key: Vec<u8>,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            key: Vec::with_capacity(4096),
        }
    }

    // Go: collate/collate.go:Buffer.Reset
    /// Clears the buffer from previous results generated by Key and KeyString.
    pub fn reset(&mut self) {
        self.key.clear();
    }

    /// All keys appended since the last reset.
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }
}

/// Go's `iter` (collate.go): a colltab.Iter plus the level cursor `pce`.
struct CIter<'a> {
    it: Iter<'a>,
    pce: usize,
}

impl<'a> CIter<'a> {
    // Go: collate/collate.go:iter.init
    fn new(w: &'a dyn Weighter, buf: Vec<Elem>) -> CIter<'a> {
        CIter {
            it: Iter::new(w, buf),
            pce: 0,
        }
    }

    // Go: collate/collate.go:iter.nextPrimary
    fn next_primary(&mut self) -> i32 {
        loop {
            while self.pce < self.it.n {
                let v = self.it.elems[self.pce].primary();
                if v != 0 {
                    self.pce += 1;
                    return v;
                }
                self.pce += 1;
            }
            if !self.it.next() {
                return 0;
            }
        }
    }

    // Go: collate/collate.go:iter.nextSecondary
    fn next_secondary(&mut self) -> i32 {
        while self.pce < self.it.elems.len() {
            let v = self.it.elems[self.pce].secondary();
            if v != 0 {
                self.pce += 1;
                return v;
            }
            self.pce += 1;
        }
        0
    }

    // Go: collate/collate.go:iter.prevSecondary
    fn prev_secondary(&mut self) -> i32 {
        while self.pce < self.it.elems.len() {
            let v = self.it.elems[self.it.elems.len() - self.pce - 1].secondary();
            if v != 0 {
                self.pce += 1;
                return v;
            }
            self.pce += 1;
        }
        0
    }

    // Go: collate/collate.go:iter.nextTertiary
    fn next_tertiary(&mut self) -> i32 {
        while self.pce < self.it.elems.len() {
            let v = self.it.elems[self.pce].tertiary();
            if v != 0 {
                self.pce += 1;
                return v as i32;
            }
            self.pce += 1;
        }
        0
    }

    // Go: collate/collate.go:iter.nextQuaternary
    fn next_quaternary(&mut self) -> i32 {
        while self.pce < self.it.elems.len() {
            let v = self.it.elems[self.pce].quaternary();
            if v != 0 {
                self.pce += 1;
                return v;
            }
            self.pce += 1;
        }
        0
    }
}

// Go: collate/collate.go:compareLevel
fn compare_level<'a>(f: fn(&mut CIter<'a>) -> i32, a: &mut CIter<'a>, b: &mut CIter<'a>) -> i32 {
    a.pce = 0;
    b.pce = 0;
    loop {
        let va = f(a);
        let vb = f(b);
        if va != vb {
            if va < vb {
                return -1;
            }
            return 1;
        } else if va == 0 {
            break;
        }
    }
    0
}

// Go: collate/collate.go:Collator.compare
fn compare_iters<'a>(o: &Options, ia: &mut CIter<'a>, ib: &mut CIter<'a>) -> i32 {
    // Process primary level
    if o.alternate != AlternateHandling::Shifted {
        // TODO: implement script reordering
        let res = compare_level(CIter::next_primary, ia, ib);
        if res != 0 {
            return res;
        }
    } else {
        // TODO: handle shifted
    }
    if !o.ignore[SECONDARY] {
        let f: fn(&mut CIter<'a>) -> i32 = if o.backwards {
            CIter::prev_secondary
        } else {
            CIter::next_secondary
        };
        let res = compare_level(f, ia, ib);
        if res != 0 {
            return res;
        }
    }
    // TODO: special case handling (Danish?)
    if !o.ignore[TERTIARY] || o.case_level {
        let res = compare_level(CIter::next_tertiary, ia, ib);
        if res != 0 {
            return res;
        }
        if !o.ignore[QUATERNARY] {
            let res = compare_level(CIter::next_quaternary, ia, ib);
            if res != 0 {
                return res;
            }
        }
    }
    0
}

// Go: collate/collate.go:appendPrimary
fn append_primary(key: &mut Vec<u8>, p: i32) {
    // Convert to variable length encoding; supports up to 23 bits.
    if p <= 0x7FFF {
        key.extend_from_slice(&[(p >> 8) as u8, p as u8]);
    } else {
        key.extend_from_slice(&[((p >> 16) as u8) | 0x80, (p >> 8) as u8, p as u8]);
    }
}

// Go: collate/collate.go:processWeights
fn process_weights(vw: AlternateHandling, top: u32, wa: &mut [Elem]) {
    let mut ignore = false;
    let vtop = top as i32;
    match vw {
        AlternateHandling::Shifted | AlternateHandling::ShiftTrimmed => {
            for w in wa.iter_mut() {
                let p = w.primary();
                if p <= vtop && p != 0 {
                    *w = make_quaternary(p);
                    ignore = true;
                } else if p == 0 {
                    if ignore {
                        *w = IGNORE;
                    }
                } else {
                    ignore = false;
                }
            }
        }
        AlternateHandling::Blanked => {
            for w in wa.iter_mut() {
                let p = w.primary();
                if p <= vtop && (ignore || p != 0) {
                    *w = IGNORE;
                    ignore = true;
                } else {
                    ignore = false;
                }
            }
        }
        AlternateHandling::NonIgnorable => {}
    }
}
