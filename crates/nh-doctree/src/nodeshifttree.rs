//! Port of `hugolib/doctree/nodeshifttree.go`.
//!
//! Owner: Wave B task T27 (doctree).
//!
//! Go `doctree.NodeShiftTree[T]`: a radix tree (armon/go-radix, ported in `radix.rs`) keyed by
//! path, whose values carry a language dimension resolved by a [`Shifter`]. Walks visit keys in
//! byte order (parent before children).
//!
//! Porting notes:
//! * Go's `Shape(d, v)` returns a copy of the tree header that shares the radix tree. The Rust
//!   tree is one value and every dimension-dependent method takes the shape (`dims`) explicitly;
//!   [`NodeShiftTree::shape`] computes it.
//! * `LongestPrefix` is CHARACTER-level (not segment-aware) and retries with `path.Dir(s)` when the
//!   found node does not exist in the current dimension or fails the predicate.
//! * Go walks that insert into or delete from the walked tree (`assembleTerms`,
//!   `addMissingRootSections`, the term `Delete` in `applyAggregatesToTaxonomiesAndTerms`) use
//!   [`NodeShiftTree::walk_mut`]: its handle gets the tree back through [`WalkState::tree_mut`],
//!   and the walk continues over the modified tree exactly as Go's `recursiveWalk` does.
//! * Go's `sync.RWMutex` is replaced by Rust borrows: `&mut self` for writers, `&self` for
//!   readers. `Lock`/`LockType` are kept for the API and do nothing. No lock is ever held while a
//!   handle or predicate runs (HUGO_LAYER.md §4.8).

use std::sync::Arc;

use nh_common::{Error, Result};

use crate::dimensions::{Dimension, DimensionFlag};
use crate::radix::{self, Cursor, NIL_DEREF, Step};
use crate::support::{MutableTree, WalkContext, WalkableTree, clean_key, must_validate_key};

/// Go: `doctree.Shifter[T]` — how a node value holding several language variants is shifted to /
/// merged into one dimension.
pub trait Shifter<T>: Send + Sync {
    /// Calls `f` for each variant of `n` in dimension `d` (stop when `f` returns true).
    fn for_each_in_dimension(&self, n: &T, d: usize, f: &mut dyn FnMut(&T) -> bool);
    /// Inserts `new` into `old` (same key). Returns (merged, replaced-old, updated).
    fn insert(&self, old: T, new: T) -> (T, Option<T>, bool);
    /// Inserts `new` into the dimension slot of `old`.
    fn insert_into(&self, old: T, new: T, dimension: Dimension) -> (T, Option<T>, bool);
    /// Deletes the dimension slot. Returns (deleted, was_deleted, is_empty). The trees do not call
    /// this directly: they call [`Shifter::delete_in_place`], whose default forwards here.
    fn delete(&self, v: T, dimension: Dimension) -> (Option<T>, bool, bool);
    /// Go's `Delete(v, dimension)` exactly: deletes the dimension slot of `v` IN PLACE (the tree
    /// keeps `v` unless `is_empty`). Returns (deleted, was_deleted, is_empty).
    ///
    /// The trees call this method. Shifters whose values hold several dimension slots (Hugo's
    /// `contentNodeIs`, `resourceSources`) must implement it: the default forwards to
    /// [`Shifter::delete`], which cannot give back the remaining slots, and panics on such a
    /// partial delete.
    fn delete_in_place(&self, v: &mut T, dimension: Dimension) -> (Option<T>, bool, bool)
    where
        T: Clone,
    {
        let (deleted, was_deleted, is_empty) = self.delete(v.clone(), dimension);
        assert!(
            is_empty || !was_deleted,
            "Shifter::delete_in_place must be implemented by a shifter whose values hold several dimensions"
        );
        (deleted, was_deleted, is_empty)
    }
    /// Resolves `v` for `dimension`; `exact=false` allows falling back to another language
    /// (resources). Returns (value, found, exact-flag).
    fn shift(&self, v: &T, dimension: Dimension, exact: bool) -> (Option<T>, bool, DimensionFlag);
}

/// Go: `doctree.NodeShiftTree[T]`.
pub struct NodeShiftTree<T> {
    pub(crate) tree: radix::Tree<T>,
    pub(crate) dims: Dimension,
    pub(crate) shifter: Arc<dyn Shifter<T>>,
}

impl<T: Clone> NodeShiftTree<T> {
    /// Go: `New(Config[T]{Shifter: shifter})` (Go panics on a nil shifter; the type prevents it).
    // Go: hugolib/doctree/nodeshifttree.go:New
    pub fn new(shifter: Arc<dyn Shifter<T>>) -> Self {
        NodeShiftTree {
            tree: radix::Tree::new(),
            dims: [0],
            shifter,
        }
    }

    /// The tree's own shape (Go: the unshaped tree's `dims`, `[0]`).
    pub fn dims(&self) -> Dimension {
        self.dims
    }

    /// Go: `Shape(d, v)` — a view of the same tree for another dimension value (language index).
    /// Rust: returns the shape to pass to the dimension-dependent methods.
    // Go: hugolib/doctree/nodeshifttree.go:Shape
    pub fn shape(&self, d: usize, v: usize) -> Dimension {
        let mut dims = self.dims;
        dims[d] = v;
        dims
    }

    /// Go: `Increment(d)` on a tree shaped `dims` — the value of dimension `d` plus one.
    // Go: hugolib/doctree/nodeshifttree.go:Increment
    pub fn increment(&self, dims: Dimension, d: usize) -> Dimension {
        let mut dims = dims;
        dims[d] += 1;
        dims
    }

    /// Go: `String()` of a tree shaped `dims` (`Root{[0]}`).
    // Go: hugolib/doctree/nodeshifttree.go:String
    pub fn string(&self, dims: Dimension) -> String {
        format!("Root{{[{}]}}", dims[0])
    }

    /// Go: `Get(s)` — the value shifted to `dims` (exact).
    // Go: hugolib/doctree/nodeshifttree.go:Get
    pub fn get(&self, dims: Dimension, s: &str) -> Option<T> {
        self.get_internal(dims, s).0
    }

    // Go: hugolib/doctree/nodeshifttree.go:Has
    pub fn has(&self, dims: Dimension, s: &str) -> bool {
        self.get_internal(dims, s).1
    }

    // Go: hugolib/doctree/nodeshifttree.go:get
    fn get_internal(&self, dims: Dimension, s: &str) -> (Option<T>, bool) {
        let s = clean_key(s);
        let Some(v) = self.tree.get(s) else {
            return (None, false);
        };
        let (t, ok, _) = self.shift(dims, v, true);
        (t, ok)
    }

    // Go: hugolib/doctree/nodeshifttree.go:shift
    fn shift(&self, dims: Dimension, t: &T, exact: bool) -> (Option<T>, bool, DimensionFlag) {
        self.shifter.shift(t, dims, exact)
    }

    /// Go: `GetRaw(s)` — the multi-dimension node (no key cleaning).
    // Go: hugolib/doctree/nodeshifttree.go:GetRaw
    pub fn get_raw(&self, s: &str) -> Option<&T> {
        self.tree.get(s)
    }

    /// Go: `InsertIntoValuesDimension(s, v)` — inserts `v` at `s` into the dimension the value
    /// carries (the shifter merges it with an existing node). Returns the stored node, the
    /// replaced value and whether an existing record was updated. `dims` is not used (as in
    /// Go). Panics like Go on an invalid key (see [`crate::support::validate_key`]).
    // Go: hugolib/doctree/nodeshifttree.go:InsertIntoValuesDimension
    pub fn insert_into_values_dimension(
        &mut self,
        dims: Dimension,
        s: &str,
        v: T,
    ) -> (T, Option<T>, bool) {
        let _ = dims;
        let s = must_validate_key(clean_key(s));
        let (v, existing, updated) = match self.tree.get(s) {
            Some(vv) => self.shifter.insert(vv.clone(), v),
            None => (v, None, false),
        };
        self.tree.insert(s, v.clone());
        (v, existing, updated)
    }

    /// Go: `InsertIntoValuesDimensionWithLock(s, v)` (Rust: `&mut self` is the lock).
    // Go: hugolib/doctree/nodeshifttree.go:InsertIntoValuesDimensionWithLock
    pub fn insert_into_values_dimension_with_lock(
        &mut self,
        dims: Dimension,
        s: &str,
        v: T,
    ) -> (T, Option<T>, bool) {
        self.insert_into_values_dimension(dims, s, v)
    }

    /// Go: `InsertIntoCurrentDimension(s, v)` — inserts `v` at `s` into the dimension `dims`.
    // Go: hugolib/doctree/nodeshifttree.go:InsertIntoCurrentDimension
    pub fn insert_into_current_dimension(
        &mut self,
        dims: Dimension,
        s: &str,
        v: T,
    ) -> (T, Option<T>, bool) {
        let s = must_validate_key(clean_key(s));
        let (v, existing, updated) = match self.tree.get(s) {
            Some(vv) => self.shifter.insert_into(vv.clone(), v, dims),
            None => (v, None, false),
        };
        self.tree.insert(s, v.clone());
        (v, existing, updated)
    }

    /// Go: `InsertRaw` / `InsertRawWithLock` — stores `v` as is; returns the previous value.
    pub fn insert_raw(&mut self, s: &str, v: T) -> Option<T> {
        self.tree.insert(s, v).0
    }

    /// Go: `InsertRawWithLock(s, v)` — returns the previous value and whether `s` existed.
    // Go: hugolib/doctree/nodeshifttree.go:InsertRawWithLock
    pub fn insert_raw_with_lock(&mut self, s: &str, v: T) -> (Option<T>, bool) {
        self.tree.insert(s, v)
    }

    /// Go: `LongestPrefix(s, exact, predicate)` — the longest key that is a byte prefix of `s`
    /// whose node exists in dimension `dims` (only exactly there if `exact`) and matches the
    /// predicate; when the found node does not qualify, retries with `path.Dir(s)` until `s` is
    /// `""` or `"/"`. Go returns `("", zero)` when nothing qualifies.
    ///
    /// Rust: for a relative `s` Go's retry loop reaches `"."` and never ends; the port returns
    /// `None` there.
    // Go: hugolib/doctree/nodeshifttree.go:LongestPrefix
    pub fn longest_prefix(
        &self,
        dims: Dimension,
        s: &str,
        exact: bool,
        predicate: Option<&dyn Fn(&T) -> bool>,
    ) -> Option<(String, T)> {
        let mut s = s.to_string();
        loop {
            if let Some((longest_prefix, v)) = self.tree.longest_prefix(&s)
                && let (Some(t), true, _) = self.shift(dims, v, exact)
                && predicate.is_none_or(|p| p(&t))
            {
                return Some((longest_prefix.to_string(), t));
            }

            if s.is_empty() || s == "/" {
                return None;
            }

            // Walk up to find a node in the correct dimension.
            let dir = go_path::path::dir(&s);
            if dir == s {
                return None;
            }
            s = dir;
        }
    }

    /// Go: `LongestPrefixAll(s)` — the longest key that is a byte prefix of `s`, ignoring
    /// dimensions.
    // Go: hugolib/doctree/nodeshifttree.go:LongestPrefixAll
    pub fn longest_prefix_all(&self, s: &str) -> Option<String> {
        self.tree.longest_prefix(s).map(|(k, _)| k.to_string())
    }

    /// Go: `ForEeachInDimension(s, d, f)`.
    // Go: hugolib/doctree/nodeshifttree.go:ForEeachInDimension
    pub fn for_each_in_dimension(&self, s: &str, d: usize, f: &mut dyn FnMut(&T) -> bool) {
        let s = clean_key(s);
        let Some(v) = self.tree.get(s) else {
            return;
        };
        self.shifter.for_each_in_dimension(v, d, f);
    }

    /// Go: `Delete(key)` for a dimension (no key cleaning, as in Go). Returns the deleted value.
    // Go: hugolib/doctree/nodeshifttree.go:Delete
    pub fn delete(&mut self, dims: Dimension, key: &str) -> Option<T> {
        self.delete_with_status(dims, key).0
    }

    /// Go: `Delete(key) (T, bool)` — the deleted value and whether the shifter deleted something.
    /// The node leaves the tree only when no dimension is left.
    // Go: hugolib/doctree/nodeshifttree.go:delete
    pub fn delete_with_status(&mut self, dims: Dimension, key: &str) -> (Option<T>, bool) {
        let mut deleted = None;
        let mut was_deleted = false;
        if let Some(v) = self.tree.get_mut(key) {
            let is_empty;
            (deleted, was_deleted, is_empty) = self.shifter.delete_in_place(v, dims);
            if is_empty {
                self.tree.delete(key);
            }
        }
        (deleted, was_deleted)
    }

    /// Go: `DeleteRaw(key)`.
    // Go: hugolib/doctree/nodeshifttree.go:DeleteRaw
    pub fn delete_raw(&mut self, dims: Dimension, key: &str) {
        self.delete_with_status(dims, key);
    }

    /// Go: `DeleteAll(key)` — removes every node under the prefix `key` in all dimensions, deleting
    /// while walking (so Go's walk-under-mutation rules decide which keys are reached).
    /// `resource.MarkStale` (rebuilds) is not ported.
    // Go: hugolib/doctree/nodeshifttree.go:DeleteAll
    pub fn delete_all(&mut self, key: &str) {
        self.delete_prefix_all(key);
    }

    /// Go: `DeletePrefix(prefix)` — deletes dimension `dims` of every node under `prefix`; returns
    /// how many deletes succeeded.
    // Go: hugolib/doctree/nodeshifttree.go:DeletePrefix
    pub fn delete_prefix(&mut self, dims: Dimension, prefix: &str) -> usize {
        let keys = self.keys_with_prefix(prefix);
        let mut count = 0;
        for key in keys {
            if self.delete_with_status(dims, &key).1 {
                count += 1;
            }
        }
        count
    }

    /// Go: `DeletePrefixAll(prefix)` — like [`NodeShiftTree::delete_all`], returning how many
    /// nodes were removed.
    // Go: hugolib/doctree/nodeshifttree.go:DeletePrefixAll
    pub fn delete_prefix_all(&mut self, prefix: &str) -> usize {
        let mut count = 0;
        let mut c = Cursor::walk_prefix(&self.tree, prefix);
        loop {
            match c.next(&self.tree) {
                Step::Leaf(l) => {
                    let key = self.tree.leaf(l).0.to_string();
                    if self.tree.delete(&key).1 {
                        count += 1;
                    }
                }
                Step::Done => return count,
                // Deleting the visited key never shares an edge array with a node the walk
                // stands on (see radix.rs), so Go's nil dereference cannot happen here.
                Step::NilDeref => unreachable!("{NIL_DEREF}"),
            }
        }
    }

    /// Go: `Len()` — the number of nodes (all dimensions).
    // Go: hugolib/doctree/nodeshifttree.go:Len
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.len() == 0
    }

    /// Go: `Lock(writable) (commit func())`. Rust borrows do the locking (`&mut self` to write);
    /// the returned commit does nothing.
    // Go: hugolib/doctree/nodeshifttree.go:Lock
    pub fn lock(&self, writable: bool) -> impl FnOnce() + use<T> {
        let _ = writable;
        || {}
    }

    /// Go: `CanLock()` (troubleshooting only): always true in Rust.
    // Go: hugolib/doctree/nodeshifttree.go:CanLock
    pub fn can_lock(&self) -> bool {
        true
    }

    /// Go: `WalkPrefixRaw(prefix, walker)` — the raw multi-dimension nodes under `prefix` in byte
    /// order; `walker` returns true to stop.
    // Go: hugolib/doctree/nodeshifttree.go:WalkPrefixRaw
    pub fn walk_prefix_raw(&self, prefix: &str, walker: &mut dyn FnMut(&str, &T) -> bool) {
        for (k, v) in radix::Iter::new(&self.tree, Cursor::walk_prefix(&self.tree, prefix)) {
            if walker(k, v) {
                break;
            }
        }
    }

    /// Keys under `prefix` in walk order (byte order), for snapshot iteration.
    pub fn keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        radix::Iter::new(&self.tree, Cursor::walk_prefix(&self.tree, prefix))
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Go: `(&NodeShiftTreeWalker[T]{Tree: tree.Shape(cfg.dims), Prefix, NoShift, Exact,
    /// Handle}).Walk(ctx)` for a handle that only reads the tree. The handle gets the walk state
    /// (Go: the walker it closes over: `w.SkipPrefix`, `w.Tree`), the key, the value shifted to
    /// `cfg.dims` and how exactly it matched; it returns true to stop the walk.
    pub fn walk<'t, F>(&'t self, cfg: &WalkConfig, mut handle: F) -> Result<()>
    where
        F: FnMut(&mut WalkState<&'t NodeShiftTree<T>>, &str, &T, DimensionFlag) -> Result<bool>,
    {
        let mut w = WalkState::new(self, cfg.dims);
        walk_core(&mut w, cfg, &mut handle)
    }

    /// Like [`NodeShiftTree::walk`], for a handle that may insert into or delete from the tree
    /// (through [`WalkState::tree_mut`]) while it is walked, as Go's walks under
    /// `LockTypeWrite` do. The walk then continues over the modified tree exactly as Go's.
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use nh_doctree::dimensions::{Dimension, DimensionFlag};
    /// # use nh_doctree::nodeshifttree::{NodeShiftTree, Shifter, WalkConfig};
    /// # struct Echo;
    /// # impl Shifter<String> for Echo {
    /// #     fn for_each_in_dimension(&self, n: &String, _: usize, f: &mut dyn FnMut(&String) -> bool) { f(n); }
    /// #     fn insert(&self, old: String, new: String) -> (String, Option<String>, bool) { (new, Some(old), true) }
    /// #     fn insert_into(&self, old: String, new: String, _: Dimension) -> (String, Option<String>, bool) { (new, Some(old), true) }
    /// #     fn delete(&self, v: String, _: Dimension) -> (Option<String>, bool, bool) { (Some(v), true, true) }
    /// #     fn shift(&self, v: &String, _: Dimension, _: bool) -> (Option<String>, bool, DimensionFlag) { (Some(v.clone()), true, DimensionFlag::LANGUAGE) }
    /// # }
    /// # fn main() -> nh_common::Result<()> {
    /// let mut tree = NodeShiftTree::new(Arc::new(Echo));
    /// tree.insert_into_values_dimension([0], "/posts/p1", "p1".to_string());
    /// tree.insert_into_values_dimension([0], "/tags", "tags".to_string());
    ///
    /// // Go's assembleTerms: a term page created while walking the pages.
    /// let mut visited = Vec::new();
    /// tree.walk_mut(&WalkConfig::default(), |w, key, _value, _flag| {
    ///     visited.push(key.to_string());
    ///     if key == "/posts/p1" {
    ///         let dims = w.dims();
    ///         w.tree_mut().insert_into_values_dimension(dims, "/tags/go", "go".to_string());
    ///     }
    ///     Ok(false) // true stops the walk
    /// })?;
    /// // The new key lands in a subtree the walk has not passed yet, so Go visits it too.
    /// assert_eq!(visited, ["/posts/p1", "/tags", "/tags/go"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn walk_mut<'t, F>(&'t mut self, cfg: &WalkConfig, mut handle: F) -> Result<()>
    where
        F: FnMut(&mut WalkState<&'t mut NodeShiftTree<T>>, &str, &T, DimensionFlag) -> Result<bool>,
    {
        let dims = cfg.dims;
        let mut w = WalkState::new(self, dims);
        walk_core(&mut w, cfg, &mut handle)
    }
}

impl<T: Clone> WalkableTree<T> for NodeShiftTree<T> {
    fn walk_prefix_raw(&self, prefix: &str, walker: &mut dyn FnMut(&str, &T) -> bool) {
        NodeShiftTree::walk_prefix_raw(self, prefix, walker)
    }
}

impl<T: Clone> MutableTree for NodeShiftTree<T> {
    fn delete_raw(&mut self, dims: Dimension, key: &str) {
        NodeShiftTree::delete_raw(self, dims, key)
    }

    fn delete_all(&mut self, key: &str) {
        NodeShiftTree::delete_all(self, key)
    }

    fn delete_prefix(&mut self, dims: Dimension, prefix: &str) -> usize {
        NodeShiftTree::delete_prefix(self, dims, prefix)
    }

    fn delete_prefix_all(&mut self, prefix: &str) -> usize {
        NodeShiftTree::delete_prefix_all(self, prefix)
    }

    fn lock(&self, _writable: bool) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn can_lock(&self) -> bool {
        true
    }
}

/// Go: `doctree.LockType`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockType {
    #[default]
    None,
    Read,
    Write,
}

/// Go: the configuration fields of `NodeShiftTreeWalker[T]` (Go's unused `WalkConfig[T]` has the
/// same fields plus a callback).
#[derive(Clone, Debug, Default)]
pub struct WalkConfig {
    /// The shape of the walked tree (Go: the walker's `Tree` is a `Shape`d tree).
    pub dims: Dimension,
    /// Optional prefix filter.
    pub prefix: String,
    /// Go: enable read or write locking if needed. Rust borrows do this; kept for the record.
    pub lock_type: LockType,
    /// When set, no dimension shifting will be performed.
    pub no_shift: bool,
    /// Don't fall back to alternative dimensions (e.g. language).
    pub exact: bool,
}

impl WalkConfig {
    /// Go: `Extend()` — the same configuration for a nested walker (Go also copies the handle and
    /// the `WalkContext`; in Rust both are passed to each walk). Local state (skipped prefixes)
    /// always starts empty.
    // Go: hugolib/doctree/nodeshifttree.go:Extend
    pub fn extend(&self) -> WalkConfig {
        self.clone()
    }
}

/// The running walk, handed to the handle of [`NodeShiftTree::walk`] (`R = &NodeShiftTree<T>`)
/// and [`NodeShiftTree::walk_mut`] (`R = &mut NodeShiftTree<T>`). Go's handles reach the same
/// things through the walker they close over.
pub struct WalkState<R> {
    tree: R,
    dims: Dimension,
    skip_prefixes: Vec<String>,
}

impl<R> WalkState<R> {
    fn new(tree: R, dims: Dimension) -> Self {
        WalkState {
            tree,
            dims,
            skip_prefixes: Vec::new(),
        }
    }

    /// Go: `SkipPrefix(prefix)` — keys starting with `prefix` are not handled for the rest of the
    /// walk.
    // Go: hugolib/doctree/nodeshifttree.go:SkipPrefix
    pub fn skip_prefix(&mut self, prefix: &str) {
        self.skip_prefixes.push(prefix.to_string());
    }

    /// Go: `ShouldSkip(s)`.
    // Go: hugolib/doctree/nodeshifttree.go:ShouldSkip
    pub fn should_skip(&self, s: &str) -> bool {
        should_skip(&self.skip_prefixes, s)
    }

    /// The shape of the walked tree.
    pub fn dims(&self) -> Dimension {
        self.dims
    }
}

impl<'t, T> WalkState<&'t NodeShiftTree<T>> {
    /// Go: `w.Tree`.
    pub fn tree(&self) -> &'t NodeShiftTree<T> {
        self.tree
    }
}

impl<T> WalkState<&mut NodeShiftTree<T>> {
    /// Go: `w.Tree` (read).
    pub fn tree(&self) -> &NodeShiftTree<T> {
        self.tree
    }

    /// Go: `w.Tree` (insert/delete during the walk).
    pub fn tree_mut(&mut self) -> &mut NodeShiftTree<T> {
        self.tree
    }
}

fn should_skip(skip_prefixes: &[String], s: &str) -> bool {
    skip_prefixes.iter().any(|p| s.starts_with(p.as_str()))
}

/// Read access to the walked tree for both walk flavours.
trait TreeRef<T> {
    fn tree_ref(&self) -> &NodeShiftTree<T>;
}

impl<T> TreeRef<T> for &NodeShiftTree<T> {
    fn tree_ref(&self) -> &NodeShiftTree<T> {
        self
    }
}

impl<T> TreeRef<T> for &mut NodeShiftTree<T> {
    fn tree_ref(&self) -> &NodeShiftTree<T> {
        self
    }
}

/// The handle of a walk (Go `Handle func(s string, v T, exact DimensionFlag) (bool, error)`).
type Handle<'h, R, T> = dyn FnMut(&mut WalkState<R>, &str, &T, DimensionFlag) -> Result<bool> + 'h;

// Go: hugolib/doctree/nodeshifttree.go:Walk
fn walk_core<T: Clone, R: TreeRef<T>>(
    w: &mut WalkState<R>,
    cfg: &WalkConfig,
    handle: &mut Handle<'_, R, T>,
) -> Result<()> {
    // Go: resetLocalState.
    w.skip_prefixes.clear();

    let mut cursor = if cfg.prefix.is_empty() {
        Cursor::walk()
    } else {
        Cursor::walk_prefix(&w.tree.tree_ref().tree, &cfg.prefix)
    };

    loop {
        let (key, t, exact) = {
            let tree = w.tree.tree_ref();
            let l = match cursor.next(&tree.tree) {
                Step::Leaf(l) => l,
                Step::Done => return Ok(()),
                Step::NilDeref => return Err(Error::new(NIL_DEREF)),
            };
            let (key, v) = tree.tree.leaf(l);
            if should_skip(&w.skip_prefixes, key) {
                continue;
            }
            let (t, ok, exact) = to_t(tree, cfg, v);
            if !ok {
                continue;
            }
            // A shifter that reports a match returns a value (Hugo's always does).
            let Some(t) = t else {
                continue;
            };
            (key.to_string(), t, exact)
        };

        if handle(w, &key, &t, exact)? {
            return Ok(());
        }
    }
}

// Go: hugolib/doctree/nodeshifttree.go:toT
fn to_t<T: Clone>(
    tree: &NodeShiftTree<T>,
    cfg: &WalkConfig,
    v: &T,
) -> (Option<T>, bool, DimensionFlag) {
    if cfg.no_shift {
        (Some(v.clone()), true, DimensionFlag(0))
    } else {
        tree.shift(cfg.dims, v, cfg.exact)
    }
}

/// Go: `doctree.NodeShiftTreeWalker[T]`, for a read-only walk whose handle needs neither
/// `SkipPrefix` nor the tree (use [`NodeShiftTree::walk`] / [`NodeShiftTree::walk_mut`] for
/// those).
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
    /// Go: `WalkContext` — carried for the handle's owner; the walk does not use it.
    pub walk_context: Option<&'a mut WalkContext<T>>,
    pub(crate) skip_prefixes: Vec<String>,
}

impl<'a, T: Clone> NodeShiftTreeWalker<'a, T> {
    pub fn new(
        tree: &'a NodeShiftTree<T>,
        dims: Dimension,
        handle: &'a mut dyn FnMut(&str, &T, DimensionFlag) -> Result<bool>,
    ) -> Self {
        NodeShiftTreeWalker {
            tree,
            dims,
            handle,
            prefix: String::new(),
            lock_type: LockType::None,
            no_shift: false,
            exact: false,
            walk_context: None,
            skip_prefixes: Vec::new(),
        }
    }

    /// The walker's configuration.
    pub fn config(&self) -> WalkConfig {
        WalkConfig {
            dims: self.dims,
            prefix: self.prefix.clone(),
            lock_type: self.lock_type,
            no_shift: self.no_shift,
            exact: self.exact,
        }
    }

    /// Go: `Extend()` (see [`WalkConfig::extend`]).
    pub fn extend(&self) -> WalkConfig {
        self.config().extend()
    }

    // Go: hugolib/doctree/nodeshifttree.go:SkipPrefix
    pub fn skip_prefix(&mut self, prefix: &str) {
        self.skip_prefixes.push(prefix.to_string());
    }

    // Go: hugolib/doctree/nodeshifttree.go:ShouldSkip
    pub fn should_skip(&self, s: &str) -> bool {
        should_skip(&self.skip_prefixes, s)
    }

    /// Go: `Walk(ctx)`. Like Go, the walk starts by clearing the skipped prefixes.
    // Go: hugolib/doctree/nodeshifttree.go:Walk
    pub fn walk(&mut self) -> Result<()> {
        // Go: resetLocalState.
        self.skip_prefixes.clear();
        let cfg = self.config();
        let handle = &mut *self.handle;
        let mut w = WalkState::new(self.tree, cfg.dims);
        walk_core(&mut w, &cfg, &mut |_, k: &str, v: &T, f| handle(k, v, f))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/nodeshifttree.go (452 lines; 19/33 funcs executed)
//   types: (group), NodeShiftTree[T, WalkFunc[T, NodeShiftTreeWalker[T, WalkConfig[T
// OK L74-84: New[T any](cfg Config[T]) *NodeShiftTree[T]
// OK L86-88: (r *NodeShiftTree[T]) Delete(key string) (T, bool)
// OK L90-92: (r *NodeShiftTree[T]) DeleteRaw(key string)
// OK L94-102: (r *NodeShiftTree[T]) DeleteAll(key string)  (MarkStale: rebuild only, not ported)
// OK L104-117: (r *NodeShiftTree[T]) DeletePrefix(prefix string) int
// OK L119-130: (r *NodeShiftTree[T]) delete(key string) (T, bool)
// OK L132-144: (t *NodeShiftTree[T]) DeletePrefixAll(prefix string) int  (MarkStale: not ported)
// OK L147-149: (t *NodeShiftTree[T]) Increment(d int) *NodeShiftTree[T]
// OK L151-162: (r *NodeShiftTree[T]) InsertIntoCurrentDimension(s string, v T) (T, T, bool)
// OK L167-178: (r *NodeShiftTree[T]) InsertIntoValuesDimension(s string, v T) (T, T, bool)
// OK L180-184: (r *NodeShiftTree[T]) InsertRawWithLock(s string, v any) (any, bool)
// OK L187-191: (r *NodeShiftTree[T]) InsertIntoValuesDimensionWithLock(s string, v T) (T, T, bool)
// OK L193-195: (t *NodeShiftTree[T]) Len() int
// OK L197-203: (t *NodeShiftTree[T]) CanLock() bool  (Rust: borrows; always true)
// OK L207-220: (t *NodeShiftTree[T]) Lock(writable bool) (commit func())  (Rust: borrows; no-op)
// OK L224-243: (r *NodeShiftTree[T]) LongestPrefix(s string, exact bool, predicate func(v T) bool) (string, T)
// OK L246-249: (r *NodeShiftTree[T]) LongestPrefixAll(s string) (string, bool)
// OK L251-258: (r *NodeShiftTree[T]) GetRaw(s string) (T, bool)
// OK L260-265: (r *NodeShiftTree[T]) WalkPrefixRaw(prefix string, walker func(key string, value T) bool)
// OK L268-272: (t *NodeShiftTree[T]) Shape(d, v int) *NodeShiftTree[T]  (Rust: returns the Dimension)
// OK L274-276: (t *NodeShiftTree[T]) String() string
// OK L278-281: (r *NodeShiftTree[T]) Get(s string) T
// OK L283-290: (r *NodeShiftTree[T]) ForEeachInDimension(s string, d int, f func(T) bool)
// OK L332-335: (r NodeShiftTreeWalker[T]) Extend() *NodeShiftTreeWalker[T]  (WalkConfig::extend)
// OK L338-340: (r *NodeShiftTreeWalker[T]) SkipPrefix(prefix ...string)
// OK L343-350: (r *NodeShiftTreeWalker[T]) ShouldSkip(s string) bool
// OK L352-395: (r *NodeShiftTreeWalker[T]) Walk(ctx context.Context) error  (walk_core)
// OK L397-399: (r *NodeShiftTreeWalker[T]) resetLocalState()
// OK L401-409: (r *NodeShiftTreeWalker[T]) toT(tree *NodeShiftTree[T], v any) (t T, ok bool, exact DimensionFlag)
// OK L411-414: (r *NodeShiftTree[T]) Has(s string) bool
// OK L416-418: (t NodeShiftTree[T]) clone() *NodeShiftTree[T]  (Rust: shapes are values)
// OK L420-422: (r *NodeShiftTree[T]) shift(t T, exact bool) (T, bool, DimensionFlag)
// OK L424-433: (r *NodeShiftTree[T]) get(s string) (T, bool)
// ---------------------------------------------------------------------------
