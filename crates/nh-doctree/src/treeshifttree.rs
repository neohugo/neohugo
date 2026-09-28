//! Port of `hugolib/doctree/treeshifttree.go`.
//!
//! Owner: Wave B task T27 (doctree).
//!
//! Go's `Shape(d, v)` returns a copy that shares the per-dimension trees and selects tree `v`.
//! In Rust the selection is an argument (`v`, the dimension value, e.g. the language index) of
//! every method that Go runs on the selected tree; [`TreeShiftTree::shape`] validates it.

use nh_common::Result;

use crate::dimensions::DIMENSION_LANGUAGE;
use crate::simpletree::SimpleTree;

/// Go: `doctree.TreeShiftTree[T]` — one SimpleTree per dimension value (language), used for
/// `treeTaxonomyEntries`.
pub struct TreeShiftTree<T> {
    /// The dimension this tree is shiftable in.
    pub(crate) d: usize,
    /// One tree per value of the dimension.
    pub(crate) trees: Vec<SimpleTree<T>>,
}

impl<T> TreeShiftTree<T> {
    /// Go: `NewTreeShiftTree[T](DimensionLanguage.Index(), length)`.
    pub fn new(length: usize) -> Self {
        Self::with_dimension(DIMENSION_LANGUAGE, length)
    }

    /// Go: `NewTreeShiftTree[T](d, length)` — panics like Go when `length` is 0.
    // Go: hugolib/doctree/treeshifttree.go:NewTreeShiftTree
    pub fn with_dimension(d: usize, length: usize) -> Self {
        if length == 0 {
            panic!("length must be > 0");
        }
        TreeShiftTree {
            d,
            trees: (0..length).map(|_| SimpleTree::new_thread_safe()).collect(),
        }
    }

    /// Go: `Shape(d, v)` — the tree for value `v` of dimension `d`. Rust: returns `v`, the value
    /// to pass to the other methods. Panics like Go on a dimension mismatch or an out-of-range
    /// value.
    // Go: hugolib/doctree/treeshifttree.go:Shape
    pub fn shape(&self, d: usize, v: usize) -> usize {
        if d != self.d {
            panic!("dimension mismatch");
        }
        if v >= self.trees.len() {
            panic!("value out of range");
        }
        v
    }

    pub fn tree(&self, lang_index: usize) -> &SimpleTree<T> {
        &self.trees[lang_index]
    }

    pub fn tree_mut(&mut self, lang_index: usize) -> &mut SimpleTree<T> {
        &mut self.trees[lang_index]
    }

    /// Go: `Get(s)` on the tree shaped `v`.
    // Go: hugolib/doctree/treeshifttree.go:Get
    pub fn get(&self, v: usize, s: &str) -> Option<&T> {
        self.trees[v].get(s)
    }

    /// Go: `LongestPrefix(s)` on the tree shaped `v`.
    // Go: hugolib/doctree/treeshifttree.go:LongestPrefix
    pub fn longest_prefix(&self, v: usize, s: &str) -> Option<(&str, &T)> {
        self.trees[v].longest_prefix(s)
    }

    /// Go: `Insert(s, val)` on the tree shaped `v`. Rust: returns the previous value (Go returns
    /// `val`).
    // Go: hugolib/doctree/treeshifttree.go:Insert
    pub fn insert(&mut self, v: usize, s: &str, val: T) -> Option<T> {
        self.trees[v].insert(s, val)
    }

    /// Go: `Lock(lockType)`: Rust borrows do the locking; the returned unlock does nothing.
    // Go: hugolib/doctree/treeshifttree.go:Lock
    pub fn lock(&self, v: usize) -> impl FnOnce() + use<T> {
        let _ = &self.trees[v];
        || {}
    }

    /// Go: `WalkPrefix(lockType, s, f)` on the tree shaped `v`.
    // Go: hugolib/doctree/treeshifttree.go:WalkPrefix
    pub fn walk_prefix(
        &self,
        v: usize,
        s: &str,
        f: &mut dyn FnMut(&str, &T) -> Result<bool>,
    ) -> Result<()> {
        self.trees[v].walk_prefix(s, f)
    }

    /// Go: `WalkPrefixRaw(lockType, s, f)` — `WalkPrefix` on every dimension's tree in turn
    /// (stopping one tree's walk does not stop the next; an error does).
    // Go: hugolib/doctree/treeshifttree.go:WalkPrefixRaw
    pub fn walk_prefix_raw(
        &self,
        s: &str,
        f: &mut dyn FnMut(&str, &T) -> Result<bool>,
    ) -> Result<()> {
        for tt in &self.trees {
            tt.walk_prefix(s, f)?;
        }
        Ok(())
    }

    /// Go: `WalkPath(lockType, s, f)` on the tree shaped `v`.
    // Go: hugolib/doctree/treeshifttree.go:WalkPath
    pub fn walk_path(
        &self,
        v: usize,
        s: &str,
        f: &mut dyn FnMut(&str, &T) -> Result<bool>,
    ) -> Result<()> {
        self.trees[v].walk_path(s, f)
    }

    /// Go: `All(lockType)` on the tree shaped `v`.
    // Go: hugolib/doctree/treeshifttree.go:All
    pub fn all(&self, v: usize) -> impl Iterator<Item = (&String, &T)> {
        self.trees[v].all()
    }

    /// Go: `LenRaw()` — the number of keys in all dimensions.
    // Go: hugolib/doctree/treeshifttree.go:LenRaw
    pub fn len_raw(&self) -> usize {
        self.trees.iter().map(|t| t.len()).sum()
    }

    /// Go: `DeletePrefix(prefix)` — in every dimension; returns how many keys were deleted.
    // Go: hugolib/doctree/treeshifttree.go:DeletePrefix
    pub fn delete_prefix(&mut self, prefix: &str) -> usize {
        let mut count = 0;
        for tt in self.trees.iter_mut() {
            count += tt.delete_prefix(prefix);
        }
        count
    }
}

impl<T: Clone> TreeShiftTree<T> {
    /// Go: `DeleteAllFunc(s, f)` — in every dimension's tree holding `s`, deletes it when `f`
    /// returns true. (Go skips a stored zero value; Rust has none.)
    // Go: hugolib/doctree/treeshifttree.go:DeleteAllFunc
    pub fn delete_all_func(&mut self, s: &str, f: &mut dyn FnMut(&str, &T) -> bool) {
        for tt in self.trees.iter_mut() {
            if let Some(v) = tt.get(s)
                && f(s, v)
            {
                // Delete.
                tt.delete(s);
            }
        }
    }

    /// Go: `Delete(key)` — in every dimension.
    // Go: hugolib/doctree/treeshifttree.go:Delete
    pub fn delete(&mut self, key: &str) {
        for tt in self.trees.iter_mut() {
            tt.delete(key);
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/treeshifttree.go (124 lines; 4/14 funcs executed)
//   types: TreeShiftTree[T
// OK L34-43: NewTreeShiftTree[T comparable](d, length int) *TreeShiftTree[T]
// OK L45-54: (t TreeShiftTree[T]) Shape(d, v int) *TreeShiftTree[T]  (Rust: returns v)
// OK L56-58: (t *TreeShiftTree[T]) Get(s string) T
// OK L60-69: (t *TreeShiftTree[T]) DeleteAllFunc(s string, f func(s string, v T) bool)
// OK L71-73: (t *TreeShiftTree[T]) LongestPrefix(s string) (string, T)
// OK L75-77: (t *TreeShiftTree[T]) Insert(s string, v T) T
// OK L79-81: (t *TreeShiftTree[T]) Lock(lockType LockType) func()  (Rust: borrows; no-op)
// OK L83-85: (t *TreeShiftTree[T]) WalkPrefix(lockType LockType, s string, f func(s string, v T) (bool, error)) error
// OK L87-94: (t *TreeShiftTree[T]) WalkPrefixRaw(lockType LockType, s string, f func(s string, v T) (bool, error)) error
// OK L96-98: (t *TreeShiftTree[T]) WalkPath(lockType LockType, s string, f func(s string, v T) (bool, error)) error
// OK L100-102: (t *TreeShiftTree[T]) All(lockType LockType) iter.Seq2[string, T]
// OK L104-110: (t *TreeShiftTree[T]) LenRaw() int
// OK L112-116: (t *TreeShiftTree[T]) Delete(key string)
// OK L118-124: (t *TreeShiftTree[T]) DeletePrefix(prefix string) int
// ---------------------------------------------------------------------------
