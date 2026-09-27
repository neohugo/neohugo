//! Port of `hugolib/doctree/treeshifttree.go`.
//!
//! Owner: Wave B task T27 (doctree).


use nh_common::Result;

use crate::simpletree::SimpleTree;

/// Go: `doctree.TreeShiftTree[T]` — one SimpleTree per dimension value (language), used for
/// `treeTaxonomyEntries`.
pub struct TreeShiftTree<T> {
    pub(crate) trees: Vec<SimpleTree<T>>,
}

impl<T: Clone> TreeShiftTree<T> {
    // Go: hugolib/doctree/treeshifttree.go:NewTreeShiftTree
    pub fn new(length: usize) -> Self {
        TreeShiftTree { trees: (0..length).map(|_| SimpleTree::new()).collect() }
    }

    pub fn tree(&self, lang_index: usize) -> &SimpleTree<T> {
        &self.trees[lang_index]
    }

    pub fn tree_mut(&mut self, lang_index: usize) -> &mut SimpleTree<T> {
        &mut self.trees[lang_index]
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/treeshifttree.go (124 lines; 4/14 funcs executed)
//   types: TreeShiftTree[T
// EX L34-43: NewTreeShiftTree[T comparable](d, length int) *TreeShiftTree[T]
// EX L45-54: (t TreeShiftTree[T]) Shape(d, v int) *TreeShiftTree[T]
//    L56-58: (t *TreeShiftTree[T]) Get(s string) T
//    L60-69: (t *TreeShiftTree[T]) DeleteAllFunc(s string, f func(s string, v T) bool)
//    L71-73: (t *TreeShiftTree[T]) LongestPrefix(s string) (string, T)
// EX L75-77: (t *TreeShiftTree[T]) Insert(s string, v T) T
//    L79-81: (t *TreeShiftTree[T]) Lock(lockType LockType) func()
// EX L83-85: (t *TreeShiftTree[T]) WalkPrefix(lockType LockType, s string, f func(s string, v T) (bool, error)) error
//    L87-94: (t *TreeShiftTree[T]) WalkPrefixRaw(lockType LockType, s string, f func(s string, v T) (bool, error)) error
//    L96-98: (t *TreeShiftTree[T]) WalkPath(lockType LockType, s string, f func(s string, v T) (bool, error)) error
//    L100-102: (t *TreeShiftTree[T]) All(lockType LockType) iter.Seq2[string, T]
//    L104-110: (t *TreeShiftTree[T]) LenRaw() int
//    L112-116: (t *TreeShiftTree[T]) Delete(key string)
//    L118-124: (t *TreeShiftTree[T]) DeletePrefix(prefix string) int
// ---------------------------------------------------------------------------
