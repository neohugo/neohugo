//! Port of `hugolib/doctree/simpletree.go`.
//!
//! Owner: Wave B task T27 (doctree).
//!
//! `SimpleTree[T]` (template store trees, `WalkContext.Data()`) and `SimpleThreadSafeTree[T]`
//! (the per-language trees of `TreeShiftTree`) are the same Rust type: Go's read/write locks are
//! Rust borrows (`&self` / `&mut self`), so the `lockType` arguments of the thread-safe variant
//! disappear. The walks cannot modify the tree they walk (the callback gets no `&mut` to it),
//! which is also how Hugo uses these trees.

use std::fmt;

use nh_common::Result;

use crate::radix::{self, Cursor};

/// Go: `doctree.SimpleTree[T]` — a radix tree (armon/go-radix) that holds `T`. Walks visit keys in
/// byte order.
#[derive(Clone)]
pub struct SimpleTree<T> {
    pub(crate) tree: radix::Tree<T>,
}

/// Go: `doctree.SimpleThreadSafeTree[T]` (Rust borrows replace its `sync.RWMutex`).
pub type SimpleThreadSafeTree<T> = SimpleTree<T>;

impl<T: fmt::Debug> fmt::Debug for SimpleTree<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tree.fmt(f)
    }
}

impl<T> Default for SimpleTree<T> {
    fn default() -> Self {
        SimpleTree {
            tree: radix::Tree::new(),
        }
    }
}

/// Go's callback adapter: `b, err = f(s, v); if err != nil { return true }; return b`.
fn walk_cursor<T>(
    tree: &radix::Tree<T>,
    cursor: Cursor,
    f: &mut dyn FnMut(&str, &T) -> Result<bool>,
) -> Result<()> {
    for (k, v) in radix::Iter::new(tree, cursor) {
        if f(k, v)? {
            break;
        }
    }
    Ok(())
}

impl<T> SimpleTree<T> {
    // Go: hugolib/doctree/simpletree.go:NewSimpleTree
    pub fn new() -> Self {
        Self::default()
    }

    /// Go: `NewSimpleThreadSafeTree[T]()`.
    // Go: hugolib/doctree/simpletree.go:NewSimpleThreadSafeTree
    pub fn new_thread_safe() -> Self {
        Self::default()
    }

    /// Go: `Get(s)` (Go returns the zero `T` when `s` is absent).
    // Go: hugolib/doctree/simpletree.go:Get
    pub fn get(&self, s: &str) -> Option<&T> {
        self.tree.get(s)
    }

    /// Mutable access to the value at `s` (Go mutates the maps it stores through their pointer).
    pub fn get_mut(&mut self, s: &str) -> Option<&mut T> {
        self.tree.get_mut(s)
    }

    /// Go: `Insert(s, v)`. Rust: returns the value `s` held before (Go returns `v`).
    // Go: hugolib/doctree/simpletree.go:Insert
    pub fn insert(&mut self, s: &str, v: T) -> Option<T> {
        self.tree.insert(s, v).0
    }

    /// Go: `LongestPrefix(s)`: the longest key that is a byte prefix of `s` (character level, not
    /// path-segment aware: `/ab` is a prefix of `/abc`). Go returns `("", zero)` when none is.
    // Go: hugolib/doctree/simpletree.go:LongestPrefix
    pub fn longest_prefix(&self, s: &str) -> Option<(&str, &T)> {
        self.tree.longest_prefix(s)
    }

    /// Go: `Walk(f)` — every key in byte order; `f` returns true to stop; an error stops the walk
    /// and is returned.
    // Go: hugolib/doctree/simpletree.go:Walk
    pub fn walk(&self, f: &mut dyn FnMut(&str, &T) -> Result<bool>) -> Result<()> {
        walk_cursor(&self.tree, Cursor::walk(), f)
    }

    /// Go: `WalkPrefix(s, f)` — every key that starts with `s` (bytes), in byte order; f returns
    /// terminate.
    // Go: hugolib/doctree/simpletree.go:WalkPrefix
    pub fn walk_prefix(&self, s: &str, f: &mut dyn FnMut(&str, &T) -> Result<bool>) -> Result<()> {
        walk_cursor(&self.tree, Cursor::walk_prefix(&self.tree, s), f)
    }

    /// Go: `WalkPath(s, f)` — every key that is a prefix of `s`, root first (template lookup walks
    /// from `""` down to the query path).
    // Go: hugolib/doctree/simpletree.go:WalkPath
    pub fn walk_path(&self, s: &str, f: &mut dyn FnMut(&str, &T) -> Result<bool>) -> Result<()> {
        let mut err = None;
        self.tree.walk_path(s, &mut |k, v| match f(k, v) {
            Ok(b) => b,
            Err(e) => {
                err = Some(e);
                true
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Go: `All()` — every key and value in byte order.
    // Go: hugolib/doctree/simpletree.go:All
    pub fn all(&self) -> impl Iterator<Item = (&String, &T)> {
        radix::Iter::new(&self.tree, Cursor::walk())
    }

    /// Go: `tree.Len()` of the underlying radix tree.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.len() == 0
    }

    /// Go: `tree.DeletePrefix(prefix)` of the underlying radix tree; returns how many keys were
    /// deleted.
    pub fn delete_prefix(&mut self, prefix: &str) -> usize {
        self.tree.delete_prefix(prefix)
    }
}

impl<T: Clone> SimpleTree<T> {
    /// Go: `tree.Delete(s)` of the underlying radix tree (used by `TreeShiftTree`).
    pub fn delete(&mut self, s: &str) -> Option<T> {
        self.tree.delete(s).0
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/simpletree.go (244 lines; 13/19 funcs executed)
//   types: Tree[T, TreeThreadSafe[T, TreeCommon[T, SimpleTree[T, SimpleThreadSafeTree[T
// OK L45-47: NewSimpleTree[T any]() *SimpleTree[T]
// OK L56-61: (tree *SimpleTree[T]) Get(s string) T
// OK L63-68: (tree *SimpleTree[T]) LongestPrefix(s string) (string, T)
// OK L70-73: (tree *SimpleTree[T]) Insert(s string, v T) T
// OK L75-86: (tree *SimpleTree[T]) Walk(f func(s string, v T) (bool, error)) error
// OK L88-100: (tree *SimpleTree[T]) WalkPrefix(s string, f func(s string, v T) (bool, error)) error
// OK L102-113: (tree *SimpleTree[T]) WalkPath(s string, f func(s string, v T) (bool, error)) error
// OK L115-121: (tree *SimpleTree[T]) All() iter.Seq2[string, T]
// OK L124-126: NewSimpleThreadSafeTree[T any]() *SimpleThreadSafeTree[T]
// OK L138-144: (tree *SimpleThreadSafeTree[T]) readLock() func()  (Rust: `&self`)
// OK L146-152: (tree *SimpleThreadSafeTree[T]) writeLock() func()  (Rust: `&mut self`)
// OK L154-162: (tree *SimpleThreadSafeTree[T]) Get(s string) T
// OK L164-172: (tree *SimpleThreadSafeTree[T]) LongestPrefix(s string) (string, T)
// OK L174-180: (tree *SimpleThreadSafeTree[T]) Insert(s string, v T) T
// OK L182-194: (tree *SimpleThreadSafeTree[T]) Lock(lockType LockType) func()  (Rust: borrows)
// OK L196-200: (tree SimpleThreadSafeTree[T]) LockTree(lockType LockType) (TreeThreadSafe[T], func())  (Rust: borrows)
// OK L202-216: (tree *SimpleThreadSafeTree[T]) WalkPrefix(lockType LockType, s string, f func(s string, v T) (bool, error)) error
// OK L218-232: (tree *SimpleThreadSafeTree[T]) WalkPath(lockType LockType, s string, f func(s string, v T) (bool, error)) error
// OK L234-242: (tree *SimpleThreadSafeTree[T]) All(lockType LockType) iter.Seq2[string, T]
// ---------------------------------------------------------------------------
