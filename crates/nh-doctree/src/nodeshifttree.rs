//! Port of `hugolib/doctree/nodeshifttree.go`.
//!
//! Owner: Wave B task T27 (doctree).


//! Go `doctree.NodeShiftTree[T]`: a radix tree (armon/go-radix) keyed by path, whose values carry a
//! language dimension resolved by a [`Shifter`]. A `BTreeMap<String, T>` gives the same
//! byte-lexicographic walk order (parent before children).
//!
//! Porting hazards:
//! * `LongestPrefix` is CHARACTER-level (not segment-aware) and retries with `path.Dir(s)` when the
//!   found node does not exist in the current dimension or fails the predicate.
//! * Some Go walks insert/delete nodes while walking (under the write lock). Iterate over a
//!   snapshot of keys and re-`get` each key at visit time; verify the result against the Go
//!   model dump (work/content-model/model_full.json) for every such walk.

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::Result;

use crate::dimensions::{Dimension, DimensionFlag};
use crate::support::WalkContext;

/// Go: `doctree.Shifter[T]` — how a node value holding several language variants is shifted to /
/// merged into one dimension.
pub trait Shifter<T>: Send + Sync {
    /// Calls `f` for each variant of `n` in dimension `d` (stop when `f` returns true).
    fn for_each_in_dimension(&self, n: &T, d: usize, f: &mut dyn FnMut(&T) -> bool);
    /// Inserts `new` into `old` (same key). Returns (merged, replaced-old, updated).
    fn insert(&self, old: T, new: T) -> (T, Option<T>, bool);
    /// Inserts `new` into the dimension slot of `old`.
    fn insert_into(&self, old: T, new: T, dimension: Dimension) -> (T, Option<T>, bool);
    /// Deletes the dimension slot. Returns (deleted, was_deleted, is_empty).
    fn delete(&self, v: T, dimension: Dimension) -> (Option<T>, bool, bool);
    /// Resolves `v` for `dimension`; `exact=false` allows falling back to another language
    /// (resources). Returns (value, found, exact-flag).
    fn shift(&self, v: &T, dimension: Dimension, exact: bool) -> (Option<T>, bool, DimensionFlag);
}

/// Go: `doctree.NodeShiftTree[T]`.
pub struct NodeShiftTree<T> {
    pub(crate) tree: BTreeMap<String, T>,
    pub(crate) dims: Dimension,
    pub(crate) shifter: Arc<dyn Shifter<T>>,
}

impl<T: Clone> NodeShiftTree<T> {
    // Go: hugolib/doctree/nodeshifttree.go:New
    pub fn new(shifter: Arc<dyn Shifter<T>>) -> Self {
        NodeShiftTree { tree: BTreeMap::new(), dims: [0], shifter }
    }

    /// Go: `Shape(d, v)` — a view of the same tree for another dimension value (language index).
    /// Rust: the view shares the data; model it as `(tree, dims)` pairs or pass the dimension
    /// explicitly to every call (preferred).
    // Go: hugolib/doctree/nodeshifttree.go:Shape
    pub fn shape(&self, d: usize, v: usize) -> Dimension {
        let mut dims = self.dims;
        dims[d] = v;
        dims
    }

    /// Go: `Get(s)` — the value shifted to `dims` (exact).
    // Go: hugolib/doctree/nodeshifttree.go:Get
    pub fn get(&self, dims: Dimension, s: &str) -> Option<T> {
        todo!()
    }

    // Go: hugolib/doctree/nodeshifttree.go:Has
    pub fn has(&self, dims: Dimension, s: &str) -> bool {
        todo!()
    }

    /// Raw access to the multi-dimension node.
    pub fn get_raw(&self, s: &str) -> Option<&T> {
        self.tree.get(s)
    }

    /// Go: `InsertIntoValuesDimension(s, v)`.
    // Go: hugolib/doctree/nodeshifttree.go:InsertIntoValuesDimension
    pub fn insert_into_values_dimension(&mut self, dims: Dimension, s: &str, v: T) -> (T, Option<T>, bool) {
        todo!()
    }

    /// Go: `InsertRaw` / `InsertRawWithLock`.
    pub fn insert_raw(&mut self, s: &str, v: T) -> Option<T> {
        self.tree.insert(s.to_string(), v)
    }

    /// Go: `LongestPrefix(s, exact, predicate)`.
    // Go: hugolib/doctree/nodeshifttree.go:LongestPrefix
    pub fn longest_prefix(&self, dims: Dimension, s: &str, exact: bool, predicate: Option<&dyn Fn(&T) -> bool>) -> Option<(String, T)> {
        todo!()
    }

    /// Go: `LongestPrefixAll(s)` — ignoring dimensions.
    // Go: hugolib/doctree/nodeshifttree.go:LongestPrefixAll
    pub fn longest_prefix_all(&self, s: &str) -> Option<String> {
        todo!()
    }

    /// Go: `ForEeachInDimension(s, d, f)`.
    // Go: hugolib/doctree/nodeshifttree.go:ForEeachInDimension
    pub fn for_each_in_dimension(&self, s: &str, d: usize, f: &mut dyn FnMut(&T) -> bool) {
        todo!()
    }

    /// Go: `Delete(key)` for a dimension.
    pub fn delete(&mut self, dims: Dimension, key: &str) -> Option<T> {
        todo!()
    }

    /// Keys under `prefix` in walk order (byte order), for snapshot iteration.
    pub fn keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        self.tree.range(prefix.to_string()..).take_while(|(k, _)| k.starts_with(prefix)).map(|(k, _)| k.clone()).collect()
    }
}

/// Go: `doctree.LockType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockType {
    None,
    Read,
    Write,
}

/// Go: `doctree.NodeShiftTreeWalker[T]`.
pub struct NodeShiftTreeWalker<'a, T> {
    pub tree: &'a NodeShiftTree<T>,
    pub dims: Dimension,
    /// Handle(key, value, exact-flag) -> terminate.
    pub handle: &'a mut dyn FnMut(&str, &T, DimensionFlag) -> Result<bool>,
    pub prefix: String,
    pub lock_type: LockType,
    /// Don't shift to the current dimension (visit raw multi-dimension nodes).
    pub no_shift: bool,
    /// When shifting, require an exact dimension match.
    pub exact: bool,
    pub walk_context: Option<&'a mut WalkContext<T>>,
    pub(crate) skip_prefixes: Vec<String>,
}

impl<'a, T: Clone> NodeShiftTreeWalker<'a, T> {
    // Go: hugolib/doctree/nodeshifttree.go:SkipPrefix
    pub fn skip_prefix(&mut self, prefix: &str) {
        self.skip_prefixes.push(prefix.to_string());
    }

    // Go: hugolib/doctree/nodeshifttree.go:ShouldSkip
    pub fn should_skip(&self, s: &str) -> bool {
        self.skip_prefixes.iter().any(|p| s.starts_with(p.as_str()))
    }

    // Go: hugolib/doctree/nodeshifttree.go:Walk
    pub fn walk(&mut self) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/nodeshifttree.go (452 lines; 19/33 funcs executed)
//   types: (group), NodeShiftTree[T, WalkFunc[T, NodeShiftTreeWalker[T, WalkConfig[T
// EX L74-84: New[T any](cfg Config[T]) *NodeShiftTree[T]
//    L86-88: (r *NodeShiftTree[T]) Delete(key string) (T, bool)
//    L90-92: (r *NodeShiftTree[T]) DeleteRaw(key string)
//    L94-102: (r *NodeShiftTree[T]) DeleteAll(key string)
//    L104-117: (r *NodeShiftTree[T]) DeletePrefix(prefix string) int
//    L119-130: (r *NodeShiftTree[T]) delete(key string) (T, bool)
//    L132-144: (t *NodeShiftTree[T]) DeletePrefixAll(prefix string) int
//    L147-149: (t *NodeShiftTree[T]) Increment(d int) *NodeShiftTree[T]
//    L151-162: (r *NodeShiftTree[T]) InsertIntoCurrentDimension(s string, v T) (T, T, bool)
// EX L167-178: (r *NodeShiftTree[T]) InsertIntoValuesDimension(s string, v T) (T, T, bool)
//    L180-184: (r *NodeShiftTree[T]) InsertRawWithLock(s string, v any) (any, bool)
// EX L187-191: (r *NodeShiftTree[T]) InsertIntoValuesDimensionWithLock(s string, v T) (T, T, bool)
//    L193-195: (t *NodeShiftTree[T]) Len() int
//    L197-203: (t *NodeShiftTree[T]) CanLock() bool
// EX L207-220: (t *NodeShiftTree[T]) Lock(writable bool) (commit func())
// EX L224-243: (r *NodeShiftTree[T]) LongestPrefix(s string, exact bool, predicate func(v T) bool) (string, T)
// EX L246-249: (r *NodeShiftTree[T]) LongestPrefixAll(s string) (string, bool)
//    L251-258: (r *NodeShiftTree[T]) GetRaw(s string) (T, bool)
//    L260-265: (r *NodeShiftTree[T]) WalkPrefixRaw(prefix string, walker func(key string, value T) bool)
// EX L268-272: (t *NodeShiftTree[T]) Shape(d, v int) *NodeShiftTree[T]
//    L274-276: (t *NodeShiftTree[T]) String() string
// EX L278-281: (r *NodeShiftTree[T]) Get(s string) T
// EX L283-290: (r *NodeShiftTree[T]) ForEeachInDimension(s string, d int, f func(T) bool)
// EX L332-335: (r NodeShiftTreeWalker[T]) Extend() *NodeShiftTreeWalker[T]
// EX L338-340: (r *NodeShiftTreeWalker[T]) SkipPrefix(prefix ...string)
// EX L343-350: (r *NodeShiftTreeWalker[T]) ShouldSkip(s string) bool
// EX L352-395: (r *NodeShiftTreeWalker[T]) Walk(ctx context.Context) error
// EX L397-399: (r *NodeShiftTreeWalker[T]) resetLocalState()
// EX L401-409: (r *NodeShiftTreeWalker[T]) toT(tree *NodeShiftTree[T], v any) (t T, ok bool, exact DimensionFlag)
// EX L411-414: (r *NodeShiftTree[T]) Has(s string) bool
// EX L416-418: (t NodeShiftTree[T]) clone() *NodeShiftTree[T]
// EX L420-422: (r *NodeShiftTree[T]) shift(t T, exact bool) (T, bool, DimensionFlag)
// EX L424-433: (r *NodeShiftTree[T]) get(s string) (T, bool)
// ---------------------------------------------------------------------------
