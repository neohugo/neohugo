//! Port of `hugolib/doctree/simpletree.go`.
//!
//! Owner: Wave B task T27 (doctree).


use std::collections::BTreeMap;

use nh_common::Result;

/// Go: `doctree.SimpleTree[T]` (template store trees). BTreeMap walk order == radix order.
#[derive(Clone, Debug)]
pub struct SimpleTree<T> {
    pub(crate) tree: BTreeMap<String, T>,
}

impl<T: Clone> Default for SimpleTree<T> {
    fn default() -> Self {
        SimpleTree { tree: BTreeMap::new() }
    }
}

impl<T: Clone> SimpleTree<T> {
    // Go: hugolib/doctree/simpletree.go:NewSimpleTree
    pub fn new() -> Self {
        Self::default()
    }

    // Go: hugolib/doctree/simpletree.go:Get
    pub fn get(&self, s: &str) -> Option<&T> {
        self.tree.get(s)
    }

    pub fn get_mut(&mut self, s: &str) -> Option<&mut T> {
        self.tree.get_mut(s)
    }

    // Go: hugolib/doctree/simpletree.go:Insert
    pub fn insert(&mut self, s: &str, v: T) -> Option<T> {
        self.tree.insert(s.to_string(), v)
    }

    /// Go: `LongestPrefix(s)` (character-level).
    // Go: hugolib/doctree/simpletree.go:LongestPrefix
    pub fn longest_prefix(&self, s: &str) -> Option<(&str, &T)> {
        todo!()
    }

    /// Go: `WalkPrefix(s, f)` — f returns terminate.
    // Go: hugolib/doctree/simpletree.go:WalkPrefix
    pub fn walk_prefix(&self, s: &str, f: &mut dyn FnMut(&str, &T) -> Result<bool>) -> Result<()> {
        todo!()
    }

    /// Go: `WalkPath(s, f)` — every key that is a prefix of `s`, root first (template lookup walks
    /// from `""` down to the query path).
    // Go: hugolib/doctree/simpletree.go:WalkPath
    pub fn walk_path(&self, s: &str, f: &mut dyn FnMut(&str, &T) -> Result<bool>) -> Result<()> {
        todo!()
    }

    // Go: hugolib/doctree/simpletree.go:All
    pub fn all(&self) -> impl Iterator<Item = (&String, &T)> {
        self.tree.iter()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/simpletree.go (244 lines; 13/19 funcs executed)
//   types: Tree[T, TreeThreadSafe[T, TreeCommon[T, SimpleTree[T, SimpleThreadSafeTree[T
// EX L45-47: NewSimpleTree[T any]() *SimpleTree[T]
// EX L56-61: (tree *SimpleTree[T]) Get(s string) T
//    L63-68: (tree *SimpleTree[T]) LongestPrefix(s string) (string, T)
// EX L70-73: (tree *SimpleTree[T]) Insert(s string, v T) T
// EX L75-86: (tree *SimpleTree[T]) Walk(f func(s string, v T) (bool, error)) error
//    L88-100: (tree *SimpleTree[T]) WalkPrefix(s string, f func(s string, v T) (bool, error)) error
// EX L102-113: (tree *SimpleTree[T]) WalkPath(s string, f func(s string, v T) (bool, error)) error
// EX L115-121: (tree *SimpleTree[T]) All() iter.Seq2[string, T]
// EX L124-126: NewSimpleThreadSafeTree[T any]() *SimpleThreadSafeTree[T]
// EX L138-144: (tree *SimpleThreadSafeTree[T]) readLock() func()
// EX L146-152: (tree *SimpleThreadSafeTree[T]) writeLock() func()
//    L154-162: (tree *SimpleThreadSafeTree[T]) Get(s string) T
// EX L164-172: (tree *SimpleThreadSafeTree[T]) LongestPrefix(s string) (string, T)
// EX L174-180: (tree *SimpleThreadSafeTree[T]) Insert(s string, v T) T
// EX L182-194: (tree *SimpleThreadSafeTree[T]) Lock(lockType LockType) func()
//    L196-200: (tree SimpleThreadSafeTree[T]) LockTree(lockType LockType) (TreeThreadSafe[T], func())
// EX L202-216: (tree *SimpleThreadSafeTree[T]) WalkPrefix(lockType LockType, s string, f func(s string, v T) (bool, error)) error
//    L218-232: (tree *SimpleThreadSafeTree[T]) WalkPath(lockType LockType, s string, f func(s string, v T) (bool, error)) error
//    L234-242: (tree *SimpleThreadSafeTree[T]) All(lockType LockType) iter.Seq2[string, T]
// ---------------------------------------------------------------------------
