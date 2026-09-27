//! Go slice semantics (shared backing arrays, `append` growth) for the one
//! place goldmark's output depends on them: the parser context's opened
//! blocks. `parseBlocks` keeps a copy of the `[]Block` header while
//! `openBlocks`/`closeBlocks` append to and shift the same backing array;
//! goldmark relies on seeing those writes (parser.go line 1105, "lastNode is
//! a paragraph and was transformed by the paragraph transformers").

use std::sync::{Arc, Mutex};

#[rustfmt::skip]
#[path = "goslice_tables.rs"]
mod tables;

use tables::{SIZE_CLASS_TO_SIZE, SIZE_TO_SIZE_CLASS8, SIZE_TO_SIZE_CLASS128};

const MAX_SMALL_SIZE: usize = 32768;
const SMALL_SIZE_DIV: usize = 8;
const SMALL_SIZE_MAX: usize = 1024;
const LARGE_SIZE_DIV: usize = 128;
const PAGE_SIZE: usize = 8192;
const MALLOC_HEADER_SIZE: usize = 8;
const MIN_SIZE_FOR_MALLOC_HEADER: usize = 8 * 64;

fn div_round_up(n: usize, a: usize) -> usize {
    n.div_ceil(a)
}

// Go: runtime/msize.go:roundupsize (go1.27.1)
fn roundupsize(size: usize, noscan: bool) -> usize {
    let mut req_size = size;
    if req_size <= MAX_SMALL_SIZE - MALLOC_HEADER_SIZE {
        // Small object.
        if !noscan && req_size > MIN_SIZE_FOR_MALLOC_HEADER {
            req_size += MALLOC_HEADER_SIZE;
        }
        if req_size <= SMALL_SIZE_MAX - 8 {
            return SIZE_CLASS_TO_SIZE
                [SIZE_TO_SIZE_CLASS8[div_round_up(req_size, SMALL_SIZE_DIV)] as usize]
                as usize
                - (req_size - size);
        }
        return SIZE_CLASS_TO_SIZE[SIZE_TO_SIZE_CLASS128
            [div_round_up(req_size - SMALL_SIZE_MAX, LARGE_SIZE_DIV)]
            as usize] as usize
            - (req_size - size);
    }
    // Large object. Align reqSize up to the next page.
    req_size += PAGE_SIZE - 1;
    req_size & !(PAGE_SIZE - 1)
}

// Go: runtime/slice.go:nextslicecap
fn nextslicecap(new_len: usize, old_cap: usize) -> usize {
    let mut newcap = old_cap;
    let doublecap = newcap + newcap;
    if new_len > doublecap {
        return new_len;
    }
    const THRESHOLD: usize = 256;
    if old_cap < THRESHOLD {
        return doublecap;
    }
    loop {
        newcap += (newcap + 3 * THRESHOLD) >> 2;
        if newcap >= new_len {
            break;
        }
    }
    newcap
}

/// The capacity Go's `growslice` gives a slice of `elem_size`-byte elements
/// (a power of two) holding pointers, grown from `old_cap` to at least
/// `new_len`.
pub(crate) fn grow_cap(new_len: usize, old_cap: usize, elem_size: usize) -> usize {
    debug_assert!(elem_size.is_power_of_two());
    let newcap = nextslicecap(new_len, old_cap);
    let capmem = roundupsize(newcap * elem_size, false);
    capmem / elem_size
}

/// A Go slice header over a shared backing array.
pub struct GoSlice<T: Clone> {
    arr: Option<Arc<Mutex<Vec<Option<T>>>>>,
    off: usize,
    len: usize,
    cap: usize,
    elem_size: usize,
}

impl<T: Clone> Clone for GoSlice<T> {
    fn clone(&self) -> Self {
        GoSlice {
            arr: self.arr.clone(),
            off: self.off,
            len: self.len,
            cap: self.cap,
            elem_size: self.elem_size,
        }
    }
}

impl<T: Clone> GoSlice<T> {
    /// A nil slice of elements whose Go size is `elem_size` bytes.
    pub fn nil(elem_size: usize) -> Self {
        GoSlice {
            arr: None,
            off: 0,
            len: 0,
            cap: 0,
            elem_size,
        }
    }

    /// len(s)
    pub fn len(&self) -> usize {
        self.len
    }

    /// len(s) == 0
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// s[i]
    pub fn get(&self, i: usize) -> T {
        assert!(
            i < self.len,
            "index out of range [{i}] with length {}",
            self.len
        );
        let arr = self.arr.as_ref().unwrap().lock().unwrap();
        arr[self.off + i].clone().expect("unset slice element")
    }

    /// s[i] = v
    pub fn set(&self, i: usize, v: T) {
        assert!(
            i < self.len,
            "index out of range [{i}] with length {}",
            self.len
        );
        let mut arr = self.arr.as_ref().unwrap().lock().unwrap();
        arr[self.off + i] = Some(v);
    }

    /// s[lo:hi]
    pub fn slice(&self, lo: usize, hi: usize) -> Self {
        assert!(
            lo <= hi && hi <= self.cap,
            "slice bounds out of range [{lo}:{hi}] with capacity {}",
            self.cap
        );
        GoSlice {
            arr: self.arr.clone(),
            off: self.off + lo,
            len: hi - lo,
            cap: self.cap - lo,
            elem_size: self.elem_size,
        }
    }

    /// The elements, copied out.
    pub fn to_vec(&self) -> Vec<T> {
        (0..self.len).map(|i| self.get(i)).collect()
    }

    /// append(s, vs...)
    pub fn append_all(&self, vs: &[T]) -> Self {
        let new_len = self.len + vs.len();
        if new_len <= self.cap {
            let s = GoSlice {
                arr: self.arr.clone(),
                off: self.off,
                len: new_len,
                cap: self.cap,
                elem_size: self.elem_size,
            };
            if !vs.is_empty() {
                let mut arr = s.arr.as_ref().unwrap().lock().unwrap();
                for (k, v) in vs.iter().enumerate() {
                    arr[s.off + self.len + k] = Some(v.clone());
                }
            }
            return s;
        }
        let newcap = grow_cap(new_len, self.cap, self.elem_size);
        let mut arr: Vec<Option<T>> = Vec::with_capacity(newcap);
        for i in 0..self.len {
            arr.push(Some(self.get(i)));
        }
        for v in vs {
            arr.push(Some(v.clone()));
        }
        arr.resize(newcap, None);
        GoSlice {
            arr: Some(Arc::new(Mutex::new(arr))),
            off: 0,
            len: new_len,
            cap: newcap,
            elem_size: self.elem_size,
        }
    }

    /// append(s, v)
    pub fn append(&self, v: T) -> Self {
        self.append_all(std::slice::from_ref(&v))
    }

    /// append(s, t...) where t may alias s's backing array: the source
    /// elements are read before writing (Go's memmove).
    pub fn append_slice(&self, t: &GoSlice<T>) -> Self {
        let vs = t.to_vec();
        self.append_all(&vs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_matches_go_for_32_byte_elements() {
        // append one at a time from nil: Go 1.27 caps for a 32-byte element
        // type with pointers (checked with a Go program).
        let mut cap = 0;
        let mut caps = vec![];
        for len in 1..=300 {
            if len > cap {
                cap = grow_cap(len, cap, 32);
                caps.push(cap);
            }
        }
        assert_eq!(caps, vec![1, 2, 4, 8, 16, 35, 71, 151, 303]);
    }
}
