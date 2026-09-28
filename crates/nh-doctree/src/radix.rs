//! Port of `github.com/armon/go-radix` v1.0.1-0.20221118154546-54df44f2176c (`radix.go`), the
//! radix tree behind every doctree tree. `// Go: radix.go:<Func>` markers refer to that file.
//!
//! Owner: Wave B task T27 (doctree).
//!
//! Why a port and not a `BTreeMap`: with no mutation, go-radix walks its keys in byte order,
//! which a `BTreeMap<String, _>` reproduces (the `order` oracle checks this on every key set).
//! But Hugo inserts into and deletes from `treePages` *while walking it* (`assembleTerms`,
//! `addMissingRootSections`, `applyAggregatesToTaxonomiesAndTerms`, `DeleteAll`), and
//! `recursiveWalk`'s result then depends on the node structure: a key inserted after the current
//! one is visited only if it lands in a subtree the walk has not passed, deleting the current key
//! can make the walk visit a sibling a second time, and so on. Neither a key snapshot nor a live
//! `BTreeMap` cursor reproduces that, so the tree itself is ported, node for node.
//!
//! The port keeps Go's pointer semantics through arenas: nodes, leaves and edge arrays are
//! indices into vectors and are never freed (a node Go detaches keeps its state for a walk that
//! still stands on it). `[]edge` is emulated as a Go slice header over a shared backing array
//! with Go's capacity growth (`mergeChild` shares the child's array like Go), so that walks over
//! a tree mutated in any way visit exactly what Go visits. See [`Cursor`] for the walk.

use std::fmt;

pub(crate) type NodeId = usize;
pub(crate) type LeafId = usize;

/// The root node (Go `t.root`).
pub(crate) const ROOT: NodeId = 0;
/// A nil `*node` (the node of a zeroed `edge{}`).
const NIL: NodeId = usize::MAX;
/// A nil backing array (a nil `[]edge`).
const NIL_ARR: usize = usize::MAX;

/// Go: `edge`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Edge {
    label: u8,
    node: NodeId,
}

/// Go's zero `edge{}` (nil node).
const ZERO_EDGE: Edge = Edge {
    label: 0,
    node: NIL,
};

/// A Go `[]edge` slice header: `arr` indexes [`Tree::arrays`] (each backing array is `cap` long).
#[derive(Clone, Copy, Debug)]
struct Edges {
    arr: usize,
    len: usize,
    cap: usize,
}

impl Edges {
    const NIL: Edges = Edges {
        arr: NIL_ARR,
        len: 0,
        cap: 0,
    };
}

/// Go: `node`.
#[derive(Clone, Debug)]
struct Node {
    /// Go `leaf *leafNode`.
    leaf: Option<LeafId>,
    /// Go `prefix string` (a node prefix may end inside a UTF-8 sequence).
    prefix: Vec<u8>,
    /// Go `edges edges`.
    edges: Edges,
}

/// Go: `leafNode`.
#[derive(Clone, Debug)]
struct Leaf<T> {
    key: String,
    val: T,
}

/// Go: `radix.Tree`.
#[derive(Clone)]
pub(crate) struct Tree<T> {
    nodes: Vec<Node>,
    leaves: Vec<Leaf<T>>,
    arrays: Vec<Vec<Edge>>,
    size: usize,
}

impl<T: fmt::Debug> fmt::Debug for Tree<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut m = f.debug_map();
        let mut c = Cursor::walk();
        while let Step::Leaf(l) = c.next(self) {
            m.entry(&self.leaves[l].key, &self.leaves[l].val);
        }
        m.finish()
    }
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Go's `append` capacity for a `[]edge` (16-byte elements) of capacity `old` that is full,
/// as measured on go1.27.1 (identical on amd64 and arm64): `growslice` with Go's size classes
/// and the malloc header of pointer-carrying objects above 512 bytes.
fn grow_cap(old: usize) -> usize {
    match old {
        0 => 2,
        2 => 4,
        4 => 8,
        8 => 16,
        16 => 32,
        32 => 71,
        71 => 143,
        143 => 303,
        // A node has at most 256 edges (one per byte), so 303 is never full.
        _ => old * 2,
    }
}

/// Go: `longestPrefix(k1, k2 string) int` — the length of the shared prefix of two strings.
// Go: radix.go:longestPrefix
fn longest_prefix(k1: &[u8], k2: &[u8]) -> usize {
    let max = k1.len().min(k2.len());
    let mut i = 0;
    while i < max {
        if k1[i] != k2[i] {
            break;
        }
        i += 1;
    }
    i
}

impl<T> Tree<T> {
    // Go: radix.go:New
    pub(crate) fn new() -> Self {
        Tree {
            nodes: vec![Node {
                leaf: None,
                prefix: Vec::new(),
                edges: Edges::NIL,
            }],
            leaves: Vec::new(),
            arrays: Vec::new(),
            size: 0,
        }
    }

    // Go: radix.go:Len
    pub(crate) fn len(&self) -> usize {
        self.size
    }

    fn new_node(&mut self, leaf: Option<LeafId>, prefix: &[u8]) -> NodeId {
        self.nodes.push(Node {
            leaf,
            prefix: prefix.to_vec(),
            edges: Edges::NIL,
        });
        self.nodes.len() - 1
    }

    fn new_leaf(&mut self, key: &str, val: T) -> LeafId {
        self.leaves.push(Leaf {
            key: key.to_string(),
            val,
        });
        self.leaves.len() - 1
    }

    fn edges_len(&self, n: NodeId) -> usize {
        self.nodes[n].edges.len
    }

    fn edge(&self, n: NodeId, i: usize) -> Edge {
        let e = self.nodes[n].edges;
        self.arrays[e.arr][i]
    }

    /// `sort.Search(len(n.edges), func(i) bool { return n.edges[i].label >= label })`.
    fn search(&self, n: NodeId, label: u8) -> usize {
        let (mut i, mut j) = (0usize, self.edges_len(n));
        while i < j {
            let h = (i + j) >> 1;
            if self.edge(n, h).label < label {
                i = h + 1;
            } else {
                j = h;
            }
        }
        i
    }

    // Go: radix.go:(*node).addEdge
    fn add_edge(&mut self, n: NodeId, e: Edge) {
        let num = self.edges_len(n);
        let idx = self.search(n, e.label);

        // n.edges = append(n.edges, edge{})
        let s = self.nodes[n].edges;
        let s = if s.len < s.cap {
            self.arrays[s.arr][s.len] = ZERO_EDGE;
            Edges {
                arr: s.arr,
                len: s.len + 1,
                cap: s.cap,
            }
        } else {
            let cap = grow_cap(s.cap);
            let mut a = vec![ZERO_EDGE; cap];
            if s.len > 0 {
                a[..s.len].copy_from_slice(&self.arrays[s.arr][..s.len]);
            }
            self.arrays.push(a);
            Edges {
                arr: self.arrays.len() - 1,
                len: s.len + 1,
                cap,
            }
        };
        self.nodes[n].edges = s;

        // copy(n.edges[idx+1:], n.edges[idx:]); n.edges[idx] = e
        let a = &mut self.arrays[s.arr];
        a.copy_within(idx..num, idx + 1);
        a[idx] = e;
    }

    // Go: radix.go:(*node).updateEdge
    fn update_edge(&mut self, n: NodeId, label: u8, node: NodeId) {
        let num = self.edges_len(n);
        let idx = self.search(n, label);
        if idx < num && self.edge(n, idx).label == label {
            let s = self.nodes[n].edges;
            self.arrays[s.arr][idx].node = node;
            return;
        }
        // Invariant (Insert only replaces the edge it just followed).
        panic!("replacing missing edge");
    }

    // Go: radix.go:(*node).getEdge
    fn get_edge(&self, n: NodeId, label: u8) -> Option<NodeId> {
        let num = self.edges_len(n);
        let idx = self.search(n, label);
        if idx < num {
            let e = self.edge(n, idx);
            if e.label == label && e.node != NIL {
                return Some(e.node);
            }
        }
        None
    }

    // Go: radix.go:(*node).delEdge
    fn del_edge(&mut self, n: NodeId, label: u8) {
        let num = self.edges_len(n);
        let idx = self.search(n, label);
        if idx < num && self.edge(n, idx).label == label {
            let s = self.nodes[n].edges;
            let a = &mut self.arrays[s.arr];
            // copy(n.edges[idx:], n.edges[idx+1:])
            a.copy_within(idx + 1..num, idx);
            // n.edges[len(n.edges)-1] = edge{}
            a[num - 1] = ZERO_EDGE;
            // n.edges = n.edges[:len(n.edges)-1]
            self.nodes[n].edges.len -= 1;
        }
    }

    // Go: radix.go:(*node).mergeChild
    fn merge_child(&mut self, n: NodeId) {
        let child = self.edge(n, 0).node;
        let child_prefix = self.nodes[child].prefix.clone();
        self.nodes[n].prefix.extend_from_slice(&child_prefix);
        self.nodes[n].leaf = self.nodes[child].leaf;
        // Shares the child's backing array, as Go's slice assignment does.
        self.nodes[n].edges = self.nodes[child].edges;
    }

    // Go: radix.go:Insert
    /// Adds or updates `s`; returns the previous value and whether `s` existed.
    pub(crate) fn insert(&mut self, s: &str, v: T) -> (Option<T>, bool) {
        let mut n = ROOT;
        let mut search: &[u8] = s.as_bytes();
        loop {
            // Handle key exhaustion.
            if search.is_empty() {
                if let Some(l) = self.nodes[n].leaf {
                    let old = std::mem::replace(&mut self.leaves[l].val, v);
                    return (Some(old), true);
                }
                let l = self.new_leaf(s, v);
                self.nodes[n].leaf = Some(l);
                self.size += 1;
                return (None, false);
            }

            // Look for the edge.
            let parent = n;
            let Some(next) = self.get_edge(n, search[0]) else {
                // No edge, create one.
                let l = self.new_leaf(s, v);
                let nn = self.new_node(Some(l), search);
                self.add_edge(
                    parent,
                    Edge {
                        label: search[0],
                        node: nn,
                    },
                );
                self.size += 1;
                return (None, false);
            };
            n = next;

            // Determine longest prefix of the search key on match.
            let common = longest_prefix(search, &self.nodes[n].prefix);
            if common == self.nodes[n].prefix.len() {
                search = &search[common..];
                continue;
            }

            // Split the node.
            self.size += 1;
            let child = self.new_node(None, &search[..common]);
            self.update_edge(parent, search[0], child);

            // Restore the existing node.
            let label = self.nodes[n].prefix[common];
            self.add_edge(child, Edge { label, node: n });
            self.nodes[n].prefix.drain(..common);

            // Create a new leaf node.
            let l = self.new_leaf(s, v);

            // If the new key is a subset, add to this node.
            search = &search[common..];
            if search.is_empty() {
                self.nodes[child].leaf = Some(l);
                return (None, false);
            }

            // Create a new edge for the node.
            let nn = self.new_node(Some(l), search);
            self.add_edge(
                child,
                Edge {
                    label: search[0],
                    node: nn,
                },
            );
            return (None, false);
        }
    }

    /// Go `Get`, returning the leaf.
    fn get_leaf(&self, s: &str) -> Option<LeafId> {
        let mut n = ROOT;
        let mut search: &[u8] = s.as_bytes();
        loop {
            // Check for key exhaustion.
            if search.is_empty() {
                return self.nodes[n].leaf;
            }

            // Look for an edge.
            n = self.get_edge(n, search[0])?;

            // Consume the search prefix.
            let p = &self.nodes[n].prefix;
            if search.starts_with(p) {
                search = &search[p.len()..];
            } else {
                return None;
            }
        }
    }

    // Go: radix.go:Get
    pub(crate) fn get(&self, s: &str) -> Option<&T> {
        self.get_leaf(s).map(|l| &self.leaves[l].val)
    }

    pub(crate) fn get_mut(&mut self, s: &str) -> Option<&mut T> {
        self.get_leaf(s).map(|l| &mut self.leaves[l].val)
    }

    // Go: radix.go:LongestPrefix
    /// The longest key that is a byte prefix of `s`, with its value.
    pub(crate) fn longest_prefix(&self, s: &str) -> Option<(&str, &T)> {
        let mut last: Option<LeafId> = None;
        let mut n = ROOT;
        let mut search: &[u8] = s.as_bytes();
        loop {
            // Look for a leaf node.
            if let Some(l) = self.nodes[n].leaf {
                last = Some(l);
            }

            // Check for key exhaustion.
            if search.is_empty() {
                break;
            }

            // Look for an edge.
            let Some(next) = self.get_edge(n, search[0]) else {
                break;
            };
            n = next;

            // Consume the search prefix.
            let p = &self.nodes[n].prefix;
            if search.starts_with(p) {
                search = &search[p.len()..];
            } else {
                break;
            }
        }
        last.map(|l| (self.leaves[l].key.as_str(), &self.leaves[l].val))
    }

    // Go: radix.go:WalkPath
    /// Visits every key that is a byte prefix of `path`, shortest first; `f` returns true to stop.
    pub(crate) fn walk_path(&self, path: &str, f: &mut dyn FnMut(&str, &T) -> bool) {
        let mut n = ROOT;
        let mut search: &[u8] = path.as_bytes();
        loop {
            // Visit the leaf values if any.
            if let Some(l) = self.nodes[n].leaf
                && f(&self.leaves[l].key, &self.leaves[l].val)
            {
                return;
            }

            // Check for key exhaustion.
            if search.is_empty() {
                return;
            }

            // Look for an edge.
            let Some(next) = self.get_edge(n, search[0]) else {
                return;
            };
            n = next;

            // Consume the search prefix.
            let p = &self.nodes[n].prefix;
            if search.starts_with(p) {
                search = &search[p.len()..];
            } else {
                return;
            }
        }
    }

    /// The key and value of a leaf yielded by a [`Cursor`].
    pub(crate) fn leaf(&self, l: LeafId) -> (&str, &T) {
        (&self.leaves[l].key, &self.leaves[l].val)
    }

    // Go: radix.go:DeletePrefix
    /// Deletes the subtree under `s`; returns how many keys were deleted. Like Go, the emptied
    /// node stays in the tree.
    pub(crate) fn delete_prefix(&mut self, s: &str) -> usize {
        // Go: radix.go:deletePrefix (the recursion unrolled).
        let mut parent: Option<NodeId> = None;
        let mut n = ROOT;
        let mut prefix: &[u8] = s.as_bytes();
        loop {
            // Check for key exhaustion.
            if prefix.is_empty() {
                // Remove the leaf node.
                let mut sub_tree_size = 0;
                let mut c = Cursor::at(n);
                loop {
                    match c.next(self) {
                        Step::Leaf(_) => sub_tree_size += 1,
                        Step::Done => break,
                        // Go dereferences a nil node here only on a corrupted tree.
                        Step::NilDeref => panic!("{NIL_DEREF}"),
                    }
                }
                if self.nodes[n].leaf.is_some() {
                    self.nodes[n].leaf = None;
                }
                self.nodes[n].edges = Edges::NIL; // deletes the entire subtree

                // Check if we should merge the parent's other child.
                if let Some(p) = parent
                    && p != ROOT
                    && self.edges_len(p) == 1
                    && self.nodes[p].leaf.is_none()
                {
                    self.merge_child(p);
                }
                self.size -= sub_tree_size;
                return sub_tree_size;
            }

            // Look for an edge.
            let label = prefix[0];
            let Some(child) = self.get_edge(n, label) else {
                return 0;
            };
            let cp = &self.nodes[child].prefix;
            if !cp.starts_with(prefix) && !prefix.starts_with(cp) {
                return 0;
            }

            // Consume the search prefix.
            if cp.len() > prefix.len() {
                prefix = &prefix[prefix.len()..];
            } else {
                prefix = &prefix[cp.len()..];
            }
            parent = Some(n);
            n = child;
        }
    }
}

impl<T: Clone> Tree<T> {
    // Go: radix.go:Delete
    /// Deletes `s`; returns its value and whether it existed. The leaf keeps its value (a node a
    /// walk still stands on may share it, as in Go), so the value is cloned.
    pub(crate) fn delete(&mut self, s: &str) -> (Option<T>, bool) {
        let mut parent: Option<NodeId> = None;
        let mut label = 0u8;
        let mut n = ROOT;
        let mut search: &[u8] = s.as_bytes();
        loop {
            // Check for key exhaustion.
            if search.is_empty() {
                if self.nodes[n].leaf.is_none() {
                    return (None, false);
                }
                break;
            }

            // Look for an edge.
            parent = Some(n);
            label = search[0];
            let Some(next) = self.get_edge(n, label) else {
                return (None, false);
            };
            n = next;

            // Consume the search prefix.
            let p = &self.nodes[n].prefix;
            if search.starts_with(p) {
                search = &search[p.len()..];
            } else {
                return (None, false);
            }
        }

        // Delete the leaf.
        let leaf = self.nodes[n].leaf.take().expect("leaf checked above");
        self.size -= 1;

        // Check if we should delete this node from the parent.
        if let Some(p) = parent
            && self.edges_len(n) == 0
        {
            self.del_edge(p, label);
        }

        // Check if we should merge this node.
        if n != ROOT && self.edges_len(n) == 1 {
            self.merge_child(n);
        }

        // Check if we should merge the parent's other child.
        if let Some(p) = parent
            && p != ROOT
            && self.edges_len(p) == 1
            && self.nodes[p].leaf.is_none()
        {
            self.merge_child(p);
        }

        (Some(self.leaves[leaf].val.clone()), true)
    }
}

/// Go's runtime error for a nil `*node` dereference in `recursiveWalk` (reachable only through
/// a walk over a node that a `mergeChild` shared with a node that was modified afterwards).
pub(crate) const NIL_DEREF: &str =
    "runtime error: invalid memory address or nil pointer dereference";

/// One step of a [`Cursor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Visit this leaf (Go calls `fn(leaf.key, leaf.val)`).
    Leaf(LeafId),
    /// The walk is over.
    Done,
    /// Go would panic with [`NIL_DEREF`].
    NilDeref,
}

#[derive(Clone, Copy, Debug)]
enum FrameState {
    /// The node's leaf was just visited; `k := len(n.edges)` is still to be read.
    NeedK,
    /// The walk of `n.edges[i]` just returned.
    AfterChild,
}

/// A `recursiveWalk(n, fn)` call in progress.
#[derive(Clone, Copy, Debug)]
struct Frame {
    node: NodeId,
    i: usize,
    k: usize,
    state: FrameState,
}

/// Go's `recursiveWalk` as a resumable state machine, so that the caller can mutate the tree
/// between two visits exactly where Go's `fn` runs:
///
/// ```go
/// func recursiveWalk(n *node, fn WalkFn) bool {
///     if n.leaf != nil && fn(n.leaf.key, n.leaf.val) { return true }
///     i := 0
///     k := len(n.edges)
///     for i < k {
///         e := n.edges[i]
///         if recursiveWalk(e.node, fn) { return true }
///         if len(n.edges) == 0 { return recursiveWalk(n, fn) }
///         if len(n.edges) >= k { i++ }
///         k = len(n.edges)
///     }
///     return false
/// }
/// ```
///
/// Stopping the walk (Go's `fn` returning true) is the caller not calling [`Cursor::next`]
/// again. Every step reads the current tree, so insertions, deletions and node merges done
/// between two steps affect the rest of the walk as they do in Go.
#[derive(Clone, Debug)]
pub(crate) struct Cursor {
    stack: Vec<Frame>,
    /// The node whose `recursiveWalk` starts next.
    pending: Option<NodeId>,
}

impl Cursor {
    /// A walk of the subtree at `n`.
    fn at(n: NodeId) -> Cursor {
        Cursor {
            stack: Vec::new(),
            pending: Some(n),
        }
    }

    fn empty() -> Cursor {
        Cursor {
            stack: Vec::new(),
            pending: None,
        }
    }

    // Go: radix.go:Walk
    /// Go `Walk`: `recursiveWalk(t.root, fn)`.
    pub(crate) fn walk() -> Cursor {
        Cursor::at(ROOT)
    }

    // Go: radix.go:WalkPrefix
    /// Go `WalkPrefix`: descends to the node under which every key starts with `prefix` (before
    /// any `fn` call), then walks it.
    pub(crate) fn walk_prefix<T>(t: &Tree<T>, prefix: &str) -> Cursor {
        let mut n = ROOT;
        let mut search: &[u8] = prefix.as_bytes();
        loop {
            // Check for key exhaustion.
            if search.is_empty() {
                return Cursor::at(n);
            }

            // Look for an edge.
            let Some(next) = t.get_edge(n, search[0]) else {
                return Cursor::empty();
            };
            n = next;

            // Consume the search prefix.
            let p = &t.nodes[n].prefix;
            if search.starts_with(p) {
                search = &search[p.len()..];
                continue;
            }
            if p.starts_with(search) {
                // Child may be under our search prefix.
                return Cursor::at(n);
            }
            return Cursor::empty();
        }
    }

    // Go: radix.go:recursiveWalk
    /// Advances to the next leaf Go would visit.
    pub(crate) fn next<T>(&mut self, t: &Tree<T>) -> Step {
        loop {
            if let Some(n) = self.pending.take() {
                if n == NIL {
                    self.stack.clear();
                    return Step::NilDeref;
                }
                self.stack.push(Frame {
                    node: n,
                    i: 0,
                    k: 0,
                    state: FrameState::NeedK,
                });
                // Visit the leaf values if any.
                if let Some(l) = t.nodes[n].leaf {
                    return Step::Leaf(l);
                }
                continue;
            }

            let Some(f) = self.stack.last_mut() else {
                return Step::Done;
            };
            match f.state {
                FrameState::NeedK => {
                    f.k = t.edges_len(f.node);
                }
                FrameState::AfterChild => {
                    let len = t.edges_len(f.node);
                    // If there are no more edges, mergeChild happened: walk n once more.
                    if len == 0 {
                        let n = f.node;
                        self.stack.pop();
                        self.pending = Some(n);
                        continue;
                    }
                    // Fewer edges than before: the current index points to a new edge.
                    if len >= f.k {
                        f.i += 1;
                    }
                    f.k = len;
                }
            }
            if f.i < f.k {
                let e = t.edge(f.node, f.i);
                f.state = FrameState::AfterChild;
                self.pending = Some(e.node);
            } else {
                self.stack.pop();
            }
        }
    }
}

/// Go `Tree.Walk` over a tree that is not modified during the walk, as an iterator.
pub(crate) struct Iter<'t, T> {
    tree: &'t Tree<T>,
    cursor: Cursor,
}

impl<'t, T> Iter<'t, T> {
    pub(crate) fn new(tree: &'t Tree<T>, cursor: Cursor) -> Self {
        Iter { tree, cursor }
    }
}

impl<'t, T> Iterator for Iter<'t, T> {
    type Item = (&'t String, &'t T);

    fn next(&mut self) -> Option<Self::Item> {
        match self.cursor.next(self.tree) {
            Step::Leaf(l) => {
                let leaf = &self.tree.leaves[l];
                Some((&leaf.key, &leaf.val))
            }
            // A tree that is not modified during the walk has no nil edges within any length.
            Step::Done | Step::NilDeref => None,
        }
    }
}

#[cfg(test)]
mod tests {
    //! armon/go-radix's `radix_test.go`, ported.

    use super::*;

    fn keys<T>(t: &Tree<T>, c: Cursor) -> Vec<String> {
        Iter::new(t, c).map(|(k, _)| k.clone()).collect()
    }

    // Go: radix_test.go:TestRadix (1000 deterministic pseudo-UUIDs instead of crypto/rand).
    #[test]
    fn test_radix() {
        let mut state = 0x9E3779B97F4A7C15u64;
        let mut next = || {
            state = state.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };
        let mut inp = std::collections::BTreeMap::new();
        for i in 0..1000 {
            let (a, b) = (next(), next());
            let key = format!(
                "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                a >> 32,
                (a >> 16) & 0xffff,
                a & 0xffff,
                b >> 48,
                b & 0xffff_ffff_ffff
            );
            inp.insert(key, i);
        }
        let mut r = Tree::new();
        for (k, v) in &inp {
            r.insert(k, *v);
        }
        assert_eq!(r.len(), inp.len());
        // Walk order is byte order.
        assert_eq!(
            keys(&r, Cursor::walk()),
            inp.keys().cloned().collect::<Vec<_>>()
        );
        for (k, v) in &inp {
            assert_eq!(r.get(k), Some(v), "missing key: {k}");
        }
        for (k, v) in &inp {
            assert_eq!(r.delete(k), (Some(*v), true), "missing key: {k}");
        }
        assert_eq!(r.len(), 0);
    }

    // Go: radix_test.go:TestRoot
    #[test]
    fn test_root() {
        let mut r = Tree::new();
        assert!(!r.delete("").1);
        assert!(!r.insert("", true).1);
        assert_eq!(r.get(""), Some(&true));
        assert_eq!(r.delete(""), (Some(true), true));
    }

    // Go: radix_test.go:TestDelete
    #[test]
    fn test_delete() {
        let mut r = Tree::new();
        let s = ["", "A", "AB"];
        for ss in s {
            r.insert(ss, true);
        }
        for ss in s {
            assert!(r.delete(ss).1, "bad {ss:?}");
        }
    }

    // Go: radix_test.go:TestDeletePrefix
    #[test]
    fn test_delete_prefix() {
        let inp = ["", "A", "AB", "ABC", "R", "S"];
        let cases: [(&str, &[&str], usize); 5] = [
            ("A", &["", "R", "S"], 3),
            ("ABC", &["", "A", "AB", "R", "S"], 1),
            ("", &[], 6),
            ("S", &["", "A", "AB", "ABC", "R"], 1),
            ("SS", &["", "A", "AB", "ABC", "R", "S"], 0),
        ];
        for (prefix, out, num_deleted) in cases {
            let mut r = Tree::new();
            for ss in inp {
                r.insert(ss, true);
            }
            assert_eq!(r.delete_prefix(prefix), num_deleted);
            assert_eq!(keys(&r, Cursor::walk()), out);
        }
    }

    // Go: radix_test.go:TestLongestPrefix
    #[test]
    fn test_longest_prefix() {
        let mut r = Tree::new();
        let ks = ["", "foo", "foobar", "foobarbaz", "foobarbazzip", "foozip"];
        for k in ks {
            r.insert(k, ());
        }
        assert_eq!(r.len(), ks.len());
        let cases = [
            ("a", ""),
            ("abc", ""),
            ("fo", ""),
            ("foo", "foo"),
            ("foob", "foo"),
            ("foobar", "foobar"),
            ("foobarba", "foobar"),
            ("foobarbaz", "foobarbaz"),
            ("foobarbazzi", "foobarbaz"),
            ("foobarbazzip", "foobarbazzip"),
            ("foozi", "foo"),
            ("foozip", "foozip"),
            ("foozipzap", "foozip"),
        ];
        for (inp, out) in cases {
            let (m, _) = r.longest_prefix(inp).expect("no match");
            assert_eq!(m, out, "{inp}");
        }
    }

    // Go: radix_test.go:TestWalkPrefix
    #[test]
    fn test_walk_prefix() {
        let mut r = Tree::new();
        let ks = [
            "foobar",
            "foo/bar/baz",
            "foo/baz/bar",
            "foo/zip/zap",
            "zipzap",
        ];
        for k in ks {
            r.insert(k, ());
        }
        assert_eq!(r.len(), ks.len());
        let cases: [(&str, &[&str]); 10] = [
            (
                "f",
                &["foobar", "foo/bar/baz", "foo/baz/bar", "foo/zip/zap"],
            ),
            (
                "foo",
                &["foobar", "foo/bar/baz", "foo/baz/bar", "foo/zip/zap"],
            ),
            ("foob", &["foobar"]),
            ("foo/", &["foo/bar/baz", "foo/baz/bar", "foo/zip/zap"]),
            ("foo/b", &["foo/bar/baz", "foo/baz/bar"]),
            ("foo/ba", &["foo/bar/baz", "foo/baz/bar"]),
            ("foo/bar", &["foo/bar/baz"]),
            ("foo/bar/baz", &["foo/bar/baz"]),
            ("foo/bar/bazoo", &[]),
            ("z", &["zipzap"]),
        ];
        for (inp, out) in cases {
            let mut got = keys(&r, Cursor::walk_prefix(&r, inp));
            got.sort();
            let mut want: Vec<String> = out.iter().map(|s| s.to_string()).collect();
            want.sort();
            assert_eq!(got, want, "{inp}");
        }
    }

    // Go: radix_test.go:TestWalkPath
    #[test]
    fn test_walk_path() {
        let mut r = Tree::new();
        let ks = [
            "foo",
            "foo/bar",
            "foo/bar/baz",
            "foo/baz/bar",
            "foo/zip/zap",
            "zipzap",
        ];
        for k in ks {
            r.insert(k, ());
        }
        assert_eq!(r.len(), ks.len());
        let cases: [(&str, &[&str]); 8] = [
            ("f", &[]),
            ("foo", &["foo"]),
            ("foo/", &["foo"]),
            ("foo/ba", &["foo"]),
            ("foo/bar", &["foo", "foo/bar"]),
            ("foo/bar/baz", &["foo", "foo/bar", "foo/bar/baz"]),
            ("foo/bar/bazoo", &["foo", "foo/bar", "foo/bar/baz"]),
            ("z", &[]),
        ];
        for (inp, out) in cases {
            let mut got = Vec::new();
            r.walk_path(inp, &mut |k, _| {
                got.push(k.to_string());
                false
            });
            assert_eq!(got, out, "{inp}");
        }
    }

    // Go: radix_test.go:TestWalkDelete
    #[test]
    fn test_walk_delete() {
        let mut r = Tree::new();
        for k in [
            "init0/0", "init0/1", "init0/2", "init0/3", "init1/0", "init1/1", "init1/2", "init1/3",
            "init2",
        ] {
            r.insert(k, ());
        }
        let delete_all = |r: &mut Tree<()>, mut c: Cursor| {
            while let Step::Leaf(l) = c.next(r) {
                let k = r.leaf(l).0.to_string();
                r.delete(&k);
            }
        };
        let c = Cursor::walk_prefix(&r, "init1");
        delete_all(&mut r, c);
        for s in ["init0/0", "init0/1", "init0/2", "init0/3", "init2"] {
            assert!(r.get(s).is_some(), "expecting to still find {s:?}");
        }
        assert_eq!(r.len(), 5);
        delete_all(&mut r, Cursor::walk());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn grow_cap_matches_go() {
        // go1.27.1: appending one edge at a time to a nil []edge.
        let mut cap = 0;
        let mut seen = Vec::new();
        for len in 0..256 {
            if len == cap {
                cap = grow_cap(cap);
                seen.push(cap);
            }
        }
        assert_eq!(seen, [2, 4, 8, 16, 32, 71, 143, 303]);
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (github.com/armon/go-radix v1.0.1-0.20221118154546-54df44f2176c, radix.go;
// third-party code under hugolib/doctree, all of it on the executed path of the functions used)
// OK L38-40: (n *node) isLeaf() bool  (`leaf.is_some()`)
// OK L42-51: (n *node) addEdge(e edge)  (with Go's append growth and shared backing arrays)
// OK L53-63: (n *node) updateEdge(label byte, node *node)
// OK L65-74: (n *node) getEdge(label byte) *node
// OK L76-86: (n *node) delEdge(label byte)
// NP L90-104: edges Len/Less/Swap/Sort  (sort.Interface; unused by the tree)
// OK L116-118: New() *Tree
// NP L122-128: NewFromMap(m map[string]interface{}) *Tree  (unused by doctree)
// OK L131-133: (t *Tree) Len() int
// OK L137-149: longestPrefix(k1, k2 string) int
// OK L153-239: (t *Tree) Insert(s string, v interface{}) (interface{}, bool)
// OK L243-296: (t *Tree) Delete(s string) (interface{}, bool)
// OK L301-303: (t *Tree) DeletePrefix(s string) int
// OK L306-343: (t *Tree) deletePrefix(parent, n *node, prefix string) int
// OK L345-351: (n *node) mergeChild()
// OK L355-381: (t *Tree) Get(s string) (interface{}, bool)
// OK L385-417: (t *Tree) LongestPrefix(s string) (string, interface{}, bool)
// NP L420-433: (t *Tree) Minimum() (string, interface{}, bool)  (unused by doctree)
// NP L436-449: (t *Tree) Maximum() (string, interface{}, bool)  (unused by doctree)
// OK L452-454: (t *Tree) Walk(fn WalkFn)  (Cursor::walk)
// OK L457-484: (t *Tree) WalkPrefix(prefix string, fn WalkFn)  (Cursor::walk_prefix)
// OK L490-517: (t *Tree) WalkPath(path string, fn WalkFn)
// OK L521-551: recursiveWalk(n *node, fn WalkFn) bool  (Cursor::next)
// NP L554-561: (t *Tree) ToMap() map[string]interface{}  (unused by doctree)
// ---------------------------------------------------------------------------
