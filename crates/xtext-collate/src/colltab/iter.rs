//! Port of golang.org/x/text/internal/colltab/iter.go.

use super::collelem::Elem;
use super::weighter::Weighter;

const MAX_COMBINING_CHARACTERS: usize = 30;

/// An Iter incrementally converts chunks of the input text to collation
/// elements, while ensuring that the collation elements are in normalized
/// order (that is, they are in the order as if the input text were
/// normalized first).
pub struct Iter<'a> {
    pub weighter: &'a dyn Weighter,
    pub elems: Vec<Elem>,
    /// N is the number of elements in Elems that will not be reordered on
    /// subsequent iterations, N <= len(Elems).
    pub n: usize,

    bytes: &'a [u8],
    // end position in text corresponding to N.
    p_end: usize,
    // pEnd <= pNext.
    p_next: usize,
}

impl<'a> Iter<'a> {
    /// Creates an iterator; `elems` is a (cleared) reusable buffer.
    pub fn new(weighter: &'a dyn Weighter, mut elems: Vec<Elem>) -> Iter<'a> {
        elems.clear();
        Iter {
            weighter,
            elems,
            n: 0,
            bytes: &[],
            p_end: 0,
            p_next: 0,
        }
    }

    /// Returns the element buffer for reuse.
    pub fn into_elems(self) -> Vec<Elem> {
        self.elems
    }

    // Go: internal/colltab/iter.go:Iter.Reset
    /// Sets the position in the current input text to p and discards any
    /// results obtained so far.
    pub fn reset(&mut self, p: usize) {
        self.elems.clear();
        self.n = 0;
        self.p_end = p;
        self.p_next = p;
    }

    // Go: internal/colltab/iter.go:Iter.Len
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    // Go: internal/colltab/iter.go:Iter.Discard
    /// Removes the collation elements up to N.
    pub fn discard(&mut self) {
        self.elems.drain(..self.n);
        self.n = 0;
    }

    // Go: internal/colltab/iter.go:Iter.End
    pub fn end(&self) -> usize {
        self.p_end
    }

    // Go: internal/colltab/iter.go:Iter.SetInput / SetInputString
    pub fn set_input(&mut self, s: &'a [u8]) {
        self.bytes = s;
        self.reset(0);
    }

    // Go: internal/colltab/iter.go:Iter.done
    fn done(&self) -> bool {
        self.p_next >= self.bytes.len()
    }

    // Go: internal/colltab/iter.go:Iter.appendNext
    fn append_next(&mut self) -> bool {
        if self.done() {
            return false;
        }
        let mut sz = self
            .weighter
            .append_next(&mut self.elems, &self.bytes[self.p_next..]);
        if sz == 0 {
            sz = 1;
        }
        self.p_next += sz;
        true
    }

    // Go: internal/colltab/iter.go:Iter.Next
    /// Appends Elems to the internal array. On each iteration, it will either
    /// add starters or modifiers.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        if self.n == self.elems.len() && !self.append_next() {
            return false;
        }

        // Check if the current segment starts with a starter.
        let mut prev_ccc = self.elems[self.elems.len() - 1].ccc();
        if prev_ccc == 0 {
            self.n = self.elems.len();
            self.p_end = self.p_next;
            return true;
        } else if self.elems[self.n].ccc() == 0 {
            // set i.N to only cover part of i.Elems for which prevCCC == 0 and
            // use rest for the next call to next.
            self.n += 1;
            while self.n < self.elems.len() && self.elems[self.n].ccc() == 0 {
                self.n += 1;
            }
            self.p_end = self.p_next;
            return true;
        }

        // The current (partial) segment starts with modifiers. We need to
        // collect all successive modifiers to ensure that they are normalized.
        loop {
            let p = self.elems.len();
            self.p_end = self.p_next;
            if !self.append_next() {
                break;
            }
            let ccc = self.elems[p].ccc();
            if ccc == 0 || self.elems.len() - self.n > MAX_COMBINING_CHARACTERS {
                // Leave the starter for the next iteration. This ensures that
                // we do not return sequences of collation elements that cross
                // two segments.
                self.n = p;
                return true;
            } else if ccc < prev_ccc {
                self.do_norm(p, ccc); // should be rare, never occurs for NFD and FCC.
            } else {
                prev_ccc = ccc;
            }
        }

        let done = self.elems.len() != self.n;
        self.n = self.elems.len();
        done
    }

    // Go: internal/colltab/iter.go:Iter.nextNoNorm
    #[allow(dead_code)]
    fn next_no_norm(&mut self) -> bool {
        if self.done() {
            return false;
        }
        self.append_next();
        self.n = self.elems.len();
        true
    }

    #[cfg(test)]
    pub(crate) fn do_norm_for_test(&mut self, p: usize, ccc: u8) {
        self.do_norm(p, ccc)
    }

    // Go: internal/colltab/iter.go:Iter.doNorm
    /// Reorders the collation elements in i.Elems.
    fn do_norm(&mut self, p: usize, ccc: u8) {
        let n = self.elems.len();
        let k = p;
        let mut p = p - 1;
        while p > self.n && ccc < self.elems[p - 1].ccc() {
            p -= 1;
        }
        // i.Elems = append(i.Elems, i.Elems[p:k]...); copy(i.Elems[p:], i.Elems[k:]);
        // i.Elems = i.Elems[:n]  ==  rotate [p:n] left by k-p.
        self.elems[p..n].rotate_left(k - p);
    }
}
