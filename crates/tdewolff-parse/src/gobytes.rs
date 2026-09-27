//! Emulation of Go's `[]byte` slice semantics.
//!
//! The tdewolff libraries hand out sub-slices of their input buffer and then
//! mutate bytes through them (lexers lowercase tag names in place, the
//! minifier rewrites numbers in place, `append` writes into spare capacity,
//! ...).  Later reads through *other* slices of the same backing array see
//! those writes.  [`GoBytes`] reproduces that exactly: it is a
//! `(array, offset, len, cap)` header over a shared, fixed-size backing array
//! of `Cell<u8>`, so every alias observes every write, and `append` follows
//! Go's `growslice` capacity rules (including malloc size-class rounding for
//! go1.27.1 on 64-bit platforms) so that "append in place vs. reallocate"
//! decisions are the same as in Go.
//!
//! `GoBytes` is deliberately `!Send`/`!Sync` (it uses `Rc`): a Go minify call
//! runs on one goroutine, and the port keeps each call on one thread.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
enum Backing {
    /// A nil slice.
    Nil,
    /// A read-only Go package-level byte slice (e.g. `var wsBytes = []byte(" ")`).
    /// Its capacity equals its length, so `append` always reallocates; writing
    /// into it panics (Go would silently mutate the global).
    Static(&'static [u8]),
    /// A heap array shared by all slices that alias it.
    Heap(Rc<[Cell<u8>]>),
}

/// A Go `[]byte`: a window `[off, off+len)` with capacity `cap` over a shared
/// backing array.
#[derive(Clone)]
pub struct GoBytes {
    back: Backing,
    off: usize,
    len: usize,
    cap: usize,
}

impl Default for GoBytes {
    fn default() -> Self {
        GoBytes::nil()
    }
}

fn alloc(cap: usize) -> Rc<[Cell<u8>]> {
    (0..cap).map(|_| Cell::new(0u8)).collect()
}

fn alloc_from(data: &[u8], cap: usize) -> Rc<[Cell<u8>]> {
    debug_assert!(data.len() <= cap);
    data.iter()
        .copied()
        .chain(std::iter::repeat_n(0u8, cap - data.len()))
        .map(Cell::new)
        .collect()
}

// Go: runtime/slice.go:nextslicecap (go1.27.1)
fn next_slice_cap(new_len: usize, old_cap: usize) -> usize {
    let mut newcap = old_cap as isize;
    let doublecap = newcap + newcap;
    let new_len_i = new_len as isize;
    if new_len_i > doublecap {
        return new_len;
    }
    const THRESHOLD: isize = 256;
    if (old_cap as isize) < THRESHOLD {
        return doublecap as usize;
    }
    loop {
        newcap += (newcap + 3 * THRESHOLD) >> 2;
        if (newcap as usize) >= new_len {
            break;
        }
    }
    if newcap <= 0 {
        return new_len;
    }
    newcap as usize
}

// Go: internal/runtime/gc/sizeclasses.go (go1.27.1)
const SIZE_CLASS_TO_SIZE: [u16; 68] = [
    0, 8, 16, 24, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240, 256, 288, 320,
    352, 384, 416, 448, 480, 512, 576, 640, 704, 768, 896, 1024, 1152, 1280, 1408, 1536, 1792,
    2048, 2304, 2688, 3072, 3200, 3456, 4096, 4864, 5376, 6144, 6528, 6784, 6912, 8192, 9472, 9728,
    10240, 10880, 12288, 13568, 14336, 16384, 18432, 19072, 20480, 21760, 24576, 27264, 28672,
    32768,
];
const SIZE_TO_SIZE_CLASS8: [u8; 129] = [
    0, 1, 2, 3, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 15,
    16, 16, 17, 17, 18, 18, 19, 19, 19, 19, 20, 20, 20, 20, 21, 21, 21, 21, 22, 22, 22, 22, 23, 23,
    23, 23, 24, 24, 24, 24, 25, 25, 25, 25, 26, 26, 26, 26, 27, 27, 27, 27, 27, 27, 27, 27, 28, 28,
    28, 28, 28, 28, 28, 28, 29, 29, 29, 29, 29, 29, 29, 29, 30, 30, 30, 30, 30, 30, 30, 30, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
    32, 32, 32, 32, 32, 32,
];
const SIZE_TO_SIZE_CLASS128: [u8; 249] = [
    32, 33, 34, 35, 36, 37, 37, 38, 38, 39, 39, 40, 40, 40, 41, 41, 41, 42, 43, 43, 44, 44, 44, 44,
    44, 45, 45, 45, 45, 45, 45, 46, 46, 46, 46, 47, 47, 47, 47, 47, 47, 48, 48, 48, 49, 49, 50, 51,
    51, 51, 51, 51, 51, 51, 51, 51, 51, 52, 52, 52, 52, 52, 52, 52, 52, 52, 52, 53, 53, 54, 54, 54,
    54, 55, 55, 55, 55, 55, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 58, 58, 58, 58, 58, 58, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59,
    59, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 61, 61, 61, 61, 61, 62, 62,
    62, 62, 62, 62, 62, 62, 62, 62, 62, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 64, 64, 64, 64, 64,
    64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 65, 65, 65, 65, 65, 65, 65,
    65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 66, 66, 66, 66, 66, 66, 66, 66, 66, 66,
    66, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67,
    67, 67, 67, 67, 67, 67, 67, 67, 67,
];

// Go: runtime/msize.go:roundupsize for noscan objects (go1.27.1, 64-bit)
fn round_up_size(size: usize) -> usize {
    const MAX_SMALL_SIZE: usize = 32768;
    const MALLOC_HEADER_SIZE: usize = 8;
    const SMALL_SIZE_DIV: usize = 8;
    const SMALL_SIZE_MAX: usize = 1024;
    const LARGE_SIZE_DIV: usize = 128;
    const PAGE_SIZE: usize = 8192;
    // Go: runtime/stubs.go:divRoundUp (uintptr arithmetic, wraps)
    fn div_round_up(n: usize, a: usize) -> usize {
        n.wrapping_add(a - 1) / a
    }
    let req = size;
    if req <= MAX_SMALL_SIZE - MALLOC_HEADER_SIZE {
        if req <= SMALL_SIZE_MAX - 8 {
            return SIZE_CLASS_TO_SIZE
                [SIZE_TO_SIZE_CLASS8[div_round_up(req, SMALL_SIZE_DIV)] as usize]
                as usize;
        }
        // for 1016 < req <= 1024 the subtraction wraps and the index is 0, as in Go
        return SIZE_CLASS_TO_SIZE[SIZE_TO_SIZE_CLASS128
            [div_round_up(req.wrapping_sub(SMALL_SIZE_MAX), LARGE_SIZE_DIV)]
            as usize] as usize;
    }
    let r = req.wrapping_add(PAGE_SIZE - 1);
    if r < size {
        return size;
    }
    r & !(PAGE_SIZE - 1)
}

/// The capacity Go's `growslice` gives a `[]byte` that must hold `new_len`
/// bytes and previously had capacity `old_cap`.
pub fn grow_cap(new_len: usize, old_cap: usize) -> usize {
    round_up_size(next_slice_cap(new_len, old_cap))
}

/// The capacity of `[]byte(s)` for a heap-allocated conversion of a string of
/// length `n` (`runtime.rawbyteslice`). Conversions that do not escape may use
/// a 32-byte stack buffer instead; callers that care must decide themselves.
pub fn string_to_bytes_cap(n: usize) -> usize {
    round_up_size(n)
}

impl GoBytes {
    /// A nil slice.
    pub const fn nil() -> GoBytes {
        GoBytes {
            back: Backing::Nil,
            off: 0,
            len: 0,
            cap: 0,
        }
    }

    /// A Go package-level `[]byte("...")` global: read-only, `cap == len`.
    pub const fn from_static(s: &'static [u8]) -> GoBytes {
        GoBytes {
            back: Backing::Static(s),
            off: 0,
            len: s.len(),
            cap: s.len(),
        }
    }

    /// A fresh heap copy of `s` with `cap == len` (like `parse.Copy`).
    pub fn from_slice(s: &[u8]) -> GoBytes {
        GoBytes::from_slice_cap(s, s.len())
    }

    /// A fresh heap copy of `s` with the given capacity (`cap >= s.len()`).
    pub fn from_slice_cap(s: &[u8], cap: usize) -> GoBytes {
        assert!(cap >= s.len());
        GoBytes {
            back: Backing::Heap(alloc_from(s, cap)),
            off: 0,
            len: s.len(),
            cap,
        }
    }

    /// Takes ownership of `v`, `cap == len`.
    pub fn from_vec(v: Vec<u8>) -> GoBytes {
        GoBytes::from_slice(&v)
    }

    /// Go `make([]byte, len, cap)` (zeroed).
    pub fn make(len: usize, cap: usize) -> GoBytes {
        assert!(len <= cap, "makeslice: cap out of range");
        GoBytes {
            back: Backing::Heap(alloc(cap)),
            off: 0,
            len,
            cap,
        }
    }

    /// Go `[]byte{}` (non-nil, empty, cap 0).
    pub fn empty() -> GoBytes {
        GoBytes::make(0, 0)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn cap(&self) -> usize {
        self.cap
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `s == nil`
    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self.back, Backing::Nil)
    }

    #[inline]
    fn raw_get(&self, abs: usize) -> u8 {
        match &self.back {
            Backing::Heap(a) => a[abs].get(),
            Backing::Static(s) => s[abs],
            Backing::Nil => panic!("index out of range on nil slice"),
        }
    }

    #[inline]
    fn raw_set(&self, abs: usize, v: u8) {
        match &self.back {
            Backing::Heap(a) => a[abs].set(v),
            Backing::Static(_) => panic!("write into a read-only Go global []byte"),
            Backing::Nil => panic!("index out of range on nil slice"),
        }
    }

    /// `s[i]`
    #[inline]
    pub fn at(&self, i: usize) -> u8 {
        assert!(
            i < self.len,
            "index out of range [{}] with length {}",
            i,
            self.len
        );
        self.raw_get(self.off + i)
    }

    /// `s[i] = v`
    #[inline]
    pub fn set(&self, i: usize, v: u8) {
        assert!(
            i < self.len,
            "index out of range [{}] with length {}",
            i,
            self.len
        );
        self.raw_set(self.off + i, v)
    }

    /// `s[lo:hi]` (`hi` may extend up to `cap`).
    pub fn slice(&self, lo: usize, hi: usize) -> GoBytes {
        assert!(
            lo <= hi && hi <= self.cap,
            "slice bounds out of range [{}:{}] with capacity {}",
            lo,
            hi,
            self.cap
        );
        if self.is_nil() {
            return GoBytes::nil();
        }
        GoBytes {
            back: self.back.clone(),
            off: self.off + lo,
            len: hi - lo,
            cap: self.cap - lo,
        }
    }

    /// `s[lo:hi:max]`
    pub fn slice3(&self, lo: usize, hi: usize, max: usize) -> GoBytes {
        assert!(
            lo <= hi && hi <= max && max <= self.cap,
            "slice bounds out of range [{}:{}:{}] with capacity {}",
            lo,
            hi,
            max,
            self.cap
        );
        if self.is_nil() {
            return GoBytes::nil();
        }
        GoBytes {
            back: self.back.clone(),
            off: self.off + lo,
            len: hi - lo,
            cap: max - lo,
        }
    }

    /// `s[lo:]`
    pub fn slice_from(&self, lo: usize) -> GoBytes {
        self.slice(lo, self.len)
    }

    /// `s[:hi]`
    pub fn slice_to(&self, hi: usize) -> GoBytes {
        self.slice(0, hi)
    }

    /// Whether both slices share the same backing array.
    pub fn same_array(&self, other: &GoBytes) -> bool {
        match (&self.back, &other.back) {
            (Backing::Heap(a), Backing::Heap(b)) => Rc::ptr_eq(a, b),
            (Backing::Static(a), Backing::Static(b)) => std::ptr::eq(a.as_ptr(), b.as_ptr()),
            _ => false,
        }
    }

    /// `append(s, src...)` with Go's growth rules. `src` must not alias `self`
    /// (use [`GoBytes::append_bytes`] for Go slices, which may alias).
    pub fn append(&self, src: &[u8]) -> GoBytes {
        let n = src.len();
        if n == 0 {
            return self.clone();
        }
        let new_len = self.len + n;
        if new_len <= self.cap {
            for (k, &c) in src.iter().enumerate() {
                self.raw_set(self.off + self.len + k, c);
            }
            return GoBytes {
                back: self.back.clone(),
                off: self.off,
                len: new_len,
                cap: self.cap,
            };
        }
        let newcap = grow_cap(new_len, self.cap);
        let arr = alloc(newcap);
        for k in 0..self.len {
            arr[k].set(self.raw_get(self.off + k));
        }
        for (k, &c) in src.iter().enumerate() {
            arr[self.len + k].set(c);
        }
        GoBytes {
            back: Backing::Heap(arr),
            off: 0,
            len: new_len,
            cap: newcap,
        }
    }

    /// `append(s, src...)` where `src` is a Go slice that may alias `s`
    /// (memmove semantics: `src` is read completely before writing).
    pub fn append_bytes(&self, src: &GoBytes) -> GoBytes {
        let tmp = src.to_vec();
        self.append(&tmp)
    }

    /// `append(s, c)`
    pub fn append_byte(&self, c: u8) -> GoBytes {
        self.append(&[c])
    }

    /// Go `copy(dst, src)` with memmove semantics; returns the count copied.
    pub fn copy_from(&self, src: &GoBytes) -> usize {
        let n = self.len.min(src.len);
        if n == 0 {
            return 0;
        }
        if self.same_array(src) {
            if self.off == src.off {
                return n;
            }
            let tmp: Vec<u8> = (0..n).map(|k| src.raw_get(src.off + k)).collect();
            for (k, c) in tmp.into_iter().enumerate() {
                self.raw_set(self.off + k, c);
            }
        } else {
            for k in 0..n {
                self.raw_set(self.off + k, src.raw_get(src.off + k));
            }
        }
        n
    }

    /// Go `copy(dst, src)` from a Rust slice.
    pub fn copy_from_slice(&self, src: &[u8]) -> usize {
        let n = self.len.min(src.len());
        for (k, &c) in src.iter().take(n).enumerate() {
            self.raw_set(self.off + k, c);
        }
        n
    }

    /// A copy of the bytes (`string(s)` / `[]byte` contents).
    pub fn to_vec(&self) -> Vec<u8> {
        (0..self.len).map(|k| self.raw_get(self.off + k)).collect()
    }

    /// Appends the bytes to `out` (e.g. `w.Write(s)`).
    pub fn write_to(&self, out: &mut Vec<u8>) {
        out.reserve(self.len);
        for k in 0..self.len {
            out.push(self.raw_get(self.off + k));
        }
    }

    /// Iterates over the bytes (reading each at iteration time).
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..self.len).map(move |k| self.raw_get(self.off + k))
    }

    /// `bytes.Equal(s, other)` (nil and empty compare equal).
    pub fn equal(&self, other: &[u8]) -> bool {
        self.len == other.len() && other.iter().enumerate().all(|(k, &c)| self.at(k) == c)
    }

    /// `bytes.HasPrefix(s, prefix)`
    pub fn has_prefix(&self, prefix: &[u8]) -> bool {
        self.len >= prefix.len() && prefix.iter().enumerate().all(|(k, &c)| self.at(k) == c)
    }

    /// `bytes.HasSuffix(s, suffix)`
    pub fn has_suffix(&self, suffix: &[u8]) -> bool {
        let n = suffix.len();
        self.len >= n
            && suffix
                .iter()
                .enumerate()
                .all(|(k, &c)| self.at(self.len - n + k) == c)
    }

    /// `bytes.IndexByte(s, c)`
    pub fn index_byte(&self, c: u8) -> isize {
        for k in 0..self.len {
            if self.raw_get(self.off + k) == c {
                return k as isize;
            }
        }
        -1
    }

    /// `bytes.LastIndexByte(s, c)`
    pub fn last_index_byte(&self, c: u8) -> isize {
        for k in (0..self.len).rev() {
            if self.raw_get(self.off + k) == c {
                return k as isize;
            }
        }
        -1
    }

    /// `bytes.Index(s, sep)` (empty `sep` matches at 0).
    pub fn index(&self, sep: &[u8]) -> isize {
        let n = sep.len();
        if n == 0 {
            return 0;
        }
        if n > self.len {
            return -1;
        }
        'outer: for k in 0..=self.len - n {
            for (j, &c) in sep.iter().enumerate() {
                if self.raw_get(self.off + k + j) != c {
                    continue 'outer;
                }
            }
            return k as isize;
        }
        -1
    }

    /// `bytes.Contains(s, sub)`
    pub fn contains(&self, sub: &[u8]) -> bool {
        self.index(sub) != -1
    }

    /// `bytes.IndexByte(s, c) != -1`
    pub fn contains_byte(&self, c: u8) -> bool {
        self.index_byte(c) != -1
    }

    /// `bytes.Compare(s, other)`
    pub fn compare(&self, other: &[u8]) -> std::cmp::Ordering {
        self.iter().cmp(other.iter().copied())
    }

    /// Lossy UTF-8 rendering for diagnostics only (never for output bytes).
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.to_vec()).into_owned()
    }
}

impl PartialEq for GoBytes {
    /// `bytes.Equal`
    fn eq(&self, other: &GoBytes) -> bool {
        self.len == other.len && (0..self.len).all(|k| self.at(k) == other.at(k))
    }
}

impl Eq for GoBytes {}

impl PartialEq<[u8]> for GoBytes {
    fn eq(&self, other: &[u8]) -> bool {
        self.equal(other)
    }
}

impl PartialEq<&[u8]> for GoBytes {
    fn eq(&self, other: &&[u8]) -> bool {
        self.equal(other)
    }
}

impl<const N: usize> PartialEq<&[u8; N]> for GoBytes {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.equal(&other[..])
    }
}

impl PartialEq<Vec<u8>> for GoBytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.equal(other)
    }
}

impl fmt::Debug for GoBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            return write!(f, "nil");
        }
        write!(f, "b\"")?;
        for c in self.iter() {
            for e in std::ascii::escape_default(c) {
                write!(f, "{}", e as char)?;
            }
        }
        write!(f, "\"")
    }
}

impl From<&[u8]> for GoBytes {
    fn from(s: &[u8]) -> GoBytes {
        GoBytes::from_slice(s)
    }
}

impl From<Vec<u8>> for GoBytes {
    fn from(v: Vec<u8>) -> GoBytes {
        GoBytes::from_vec(v)
    }
}

impl From<&str> for GoBytes {
    fn from(s: &str) -> GoBytes {
        GoBytes::from_slice(s.as_bytes())
    }
}

/// Read-only byte access shared by `[u8]`, `Vec<u8>` and [`GoBytes`], so that
/// pure scanning functions (`Number`, `ParseFloat`, `ToHash`, ...) accept
/// either without copying.
pub trait ByteView {
    fn len(&self) -> usize;
    fn at(&self, i: usize) -> u8;
    fn is_empty(&self) -> bool {
        ByteView::len(self) == 0
    }
}

impl ByteView for [u8] {
    #[inline]
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }
    #[inline]
    fn at(&self, i: usize) -> u8 {
        self[i]
    }
}

impl<const N: usize> ByteView for [u8; N] {
    #[inline]
    fn len(&self) -> usize {
        N
    }
    #[inline]
    fn at(&self, i: usize) -> u8 {
        self[i]
    }
}

impl ByteView for Vec<u8> {
    #[inline]
    fn len(&self) -> usize {
        Vec::len(self)
    }
    #[inline]
    fn at(&self, i: usize) -> u8 {
        self[i]
    }
}

impl ByteView for str {
    #[inline]
    fn len(&self) -> usize {
        str::len(self)
    }
    #[inline]
    fn at(&self, i: usize) -> u8 {
        self.as_bytes()[i]
    }
}

impl ByteView for GoBytes {
    #[inline]
    fn len(&self) -> usize {
        self.len
    }
    #[inline]
    fn at(&self, i: usize) -> u8 {
        GoBytes::at(self, i)
    }
}

impl<B: ByteView + ?Sized> ByteView for &B {
    #[inline]
    fn len(&self) -> usize {
        (**self).len()
    }
    #[inline]
    fn at(&self, i: usize) -> u8 {
        (**self).at(i)
    }
}

/// `b[off:]` as a [`ByteView`] without copying.
pub struct SubView<'a, B: ByteView + ?Sized> {
    b: &'a B,
    off: usize,
}

impl<'a, B: ByteView + ?Sized> SubView<'a, B> {
    pub fn new(b: &'a B, off: usize) -> Self {
        assert!(off <= b.len(), "slice bounds out of range");
        SubView { b, off }
    }
}

impl<B: ByteView + ?Sized> ByteView for SubView<'_, B> {
    #[inline]
    fn len(&self) -> usize {
        self.b.len() - self.off
    }
    #[inline]
    fn at(&self, i: usize) -> u8 {
        self.b.at(self.off + i)
    }
}

/// Copies any [`ByteView`] into a `Vec<u8>`.
pub fn view_to_vec<B: ByteView + ?Sized>(b: &B) -> Vec<u8> {
    (0..b.len()).map(|i| b.at(i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_growth_matches_go() {
        // Go: append([]byte(nil), 'a') has cap 8; appending up to 9 gives 16...
        let s = GoBytes::nil().append(b"a");
        assert_eq!(s.cap(), 8);
        let s = GoBytes::nil().append(&[0u8; 9]);
        assert_eq!(s.cap(), 16);
        let s = GoBytes::make(0, 5).append(&[0u8; 6]);
        assert_eq!(s.cap(), 16);
        let s = GoBytes::make(300, 300).append(b"x");
        // nextslicecap: 300 + (300+768)/4 = 567 -> roundupsize 576
        assert_eq!(s.cap(), 576);
        assert_eq!(grow_cap(40000, 0), 40960);
        // 1017..=1024 take the wrapping path in Go's roundupsize
        assert_eq!(grow_cap(1020, 0), 1024);
        assert_eq!(grow_cap(1016, 0), 1024);
        assert_eq!(grow_cap(1025, 0), 1152);
    }

    #[test]
    fn search_helpers() {
        let s = GoBytes::from_slice(b"abcabc");
        assert_eq!(s.index(b"ca"), 2);
        assert_eq!(s.index(b""), 0);
        assert_eq!(s.index(b"x"), -1);
        assert_eq!(s.index(b"abcabcd"), -1);
        assert_eq!(s.last_index_byte(b'a'), 3);
        assert!(s.contains(b"bca"));
        assert!(s.has_prefix(b"abc") && s.has_suffix(b"cabc"));
        assert_eq!(s.compare(b"abd"), std::cmp::Ordering::Less);
        assert!(GoBytes::nil() == GoBytes::empty());
        assert!(GoBytes::nil().is_nil() && !GoBytes::empty().is_nil());
        assert!(GoBytes::nil().slice(0, 0).is_nil());
    }

    #[test]
    fn aliasing() {
        let a = GoBytes::from_slice_cap(b"abc", 8);
        let b = a.slice(1, 3);
        b.set(0, b'X');
        assert_eq!(a.to_vec(), b"aXc");
        let c = a.slice_to(1).append(b"YZ");
        assert!(c.same_array(&a));
        assert_eq!(a.to_vec(), b"aYZ");
        let d = a.slice3(0, 3, 3).append(b"!");
        assert!(!d.same_array(&a));
    }
}
