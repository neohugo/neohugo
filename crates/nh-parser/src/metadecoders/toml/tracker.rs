//! Port of `github.com/pelletier/go-toml/v2@v2.2.4/internal/tracker/seen.go`.
//!
//! The Go tracker for inline tables comes from a `sync.Pool`; every use starts with `reset()`,
//! so a fresh tracker is equivalent.

use super::parser::{Kind, Node, node_children, node_key, node_value};

/// Go: `keyKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyKind {
    Invalid,
    Value,
    Table,
    ArrayTable,
}

impl KeyKind {
    // Go: internal/tracker/seen.go:String
    fn string(self) -> &'static str {
        match self {
            KeyKind::Invalid => "invalid",
            KeyKind::Value => "value",
            KeyKind::Table => "table",
            KeyKind::ArrayTable => "array table",
        }
    }
}

/// Go: `entry`. Use -1 to indicate no child or no sibling.
#[derive(Clone, Debug)]
struct Entry {
    child: isize,
    next: isize,

    name: Option<Vec<u8>>,
    kind: KeyKind,
    explicit: bool,
    kv: bool,
}

impl Default for Entry {
    fn default() -> Self {
        Entry {
            child: 0,
            next: 0,
            name: None,
            kind: KeyKind::Invalid,
            explicit: false,
            kv: false,
        }
    }
}

/// Go: `SeenTracker` — tracks which keys have been seen with which TOML type to flag
/// duplicates and mismatches according to the spec.
#[derive(Default)]
pub(crate) struct SeenTracker {
    entries: Vec<Entry>,
    current_idx: isize,
    /// Go's `s.entries == nil`.
    initialized: bool,
}

/// Errors of the tracker are plain `fmt.Errorf` errors (message bytes).
pub(crate) type TrackerResult<T> = Result<T, Vec<u8>>;

fn errorf(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

impl SeenTracker {
    // Go: internal/tracker/seen.go:reset
    fn reset(&mut self) {
        // Always contains a root element at index 0.
        self.current_idx = 0;
        if self.entries.is_empty() {
            self.entries = vec![Entry::default()];
        } else {
            self.entries.truncate(1);
        }
        self.entries[0].child = -1;
        self.entries[0].next = -1;
        self.initialized = true;
    }

    /// Find the index of the child of parentIdx with key k. Returns -1 if
    /// it does not exist.
    // Go: internal/tracker/seen.go:find
    fn find(&self, parent_idx: isize, k: &[u8]) -> isize {
        let mut i = self.entries[parent_idx as usize].child;
        while i >= 0 {
            if self.entries[i as usize].name.as_deref().unwrap_or(&[]) == k {
                return i;
            }
            i = self.entries[i as usize].next;
        }
        -1
    }

    /// Remove all descendants of node at position idx.
    // Go: internal/tracker/seen.go:clear
    fn clear(&mut self, idx: isize) {
        if idx as usize >= self.entries.len() {
            return;
        }

        let mut i = self.entries[idx as usize].child;
        while i >= 0 {
            let next = self.entries[i as usize].next;
            let n = self.entries[0].next;
            self.entries[0].next = i;
            self.entries[i as usize].next = n;
            self.entries[i as usize].name = None;
            self.clear(i);
            i = next;
        }

        self.entries[idx as usize].child = -1;
    }

    // Go: internal/tracker/seen.go:create
    fn create(
        &mut self,
        parent_idx: isize,
        name: &[u8],
        kind: KeyKind,
        explicit: bool,
        kv: bool,
    ) -> isize {
        let e = Entry {
            child: -1,
            next: self.entries[parent_idx as usize].child,

            name: Some(name.to_vec()),
            kind,
            explicit,
            kv,
        };
        let idx;
        if self.entries[0].next >= 0 {
            idx = self.entries[0].next;
            self.entries[0].next = self.entries[idx as usize].next;
            self.entries[idx as usize] = e;
        } else {
            idx = self.entries.len() as isize;
            self.entries.push(e);
        }

        self.entries[parent_idx as usize].child = idx;

        idx
    }

    // Go: internal/tracker/seen.go:setExplicitFlag
    fn set_explicit_flag(&mut self, parent_idx: isize) {
        let mut i = self.entries[parent_idx as usize].child;
        while i >= 0 {
            if self.entries[i as usize].kv {
                self.entries[i as usize].explicit = true;
                self.entries[i as usize].kv = false;
            }
            self.set_explicit_flag(i);
            i = self.entries[i as usize].next;
        }
    }

    /// CheckExpression takes a top-level node and checks that it does not contain
    /// keys that have been seen in previous calls, and validates that types are
    /// consistent. It returns true if it is the first time this node's key is seen.
    /// Useful to clear array tables on first use.
    // Go: internal/tracker/seen.go:CheckExpression
    pub(crate) fn check_expression(
        &mut self,
        nodes: &[Node<'_>],
        node: usize,
    ) -> TrackerResult<bool> {
        if !self.initialized {
            self.reset();
        }
        match nodes[node].kind {
            Kind::KeyValue => self.check_key_value(nodes, node),
            Kind::Table => self.check_table(nodes, node),
            Kind::ArrayTable => self.check_array_table(nodes, node),
            k => panic!("this should not be a top level node type: {}", k.string()),
        }
    }

    // Go: internal/tracker/seen.go:checkTable
    fn check_table(&mut self, nodes: &[Node<'_>], node: usize) -> TrackerResult<bool> {
        if self.current_idx >= 0 {
            self.set_explicit_flag(self.current_idx);
        }

        let mut it = node_key(nodes, node);

        let mut parent_idx: isize = 0;

        // This code is duplicated in checkArrayTable. This is because factoring
        // it in a function requires to copy the iterator, or allocate it to the
        // heap, which is not cheap.
        while it.next(nodes) {
            if it.is_last(nodes) {
                break;
            }

            let k = &nodes[it.node()].data;

            let mut idx = self.find(parent_idx, k);

            if idx < 0 {
                idx = self.create(parent_idx, k, KeyKind::Table, false, false);
            } else {
                let entry = &self.entries[idx as usize];
                if entry.kind == KeyKind::Value {
                    return Err(errorf(&[
                        b"toml: expected ",
                        k,
                        b" to be a table, not a ",
                        entry.kind.string().as_bytes(),
                    ]));
                }
            }
            parent_idx = idx;
        }

        let k = &nodes[it.node()].data;
        let mut idx = self.find(parent_idx, k);

        let mut first = false;
        if idx >= 0 {
            let kind = self.entries[idx as usize].kind;
            if kind != KeyKind::Table {
                return Err(errorf(&[
                    b"toml: key ",
                    k,
                    b" should be a table, not a ",
                    kind.string().as_bytes(),
                ]));
            }
            if self.entries[idx as usize].explicit {
                return Err(errorf(&[b"toml: table ", k, b" already exists"]));
            }
            self.entries[idx as usize].explicit = true;
        } else {
            idx = self.create(parent_idx, k, KeyKind::Table, true, false);
            first = true;
        }

        self.current_idx = idx;

        Ok(first)
    }

    // Go: internal/tracker/seen.go:checkArrayTable
    fn check_array_table(&mut self, nodes: &[Node<'_>], node: usize) -> TrackerResult<bool> {
        if self.current_idx >= 0 {
            self.set_explicit_flag(self.current_idx);
        }

        let mut it = node_key(nodes, node);

        let mut parent_idx: isize = 0;

        while it.next(nodes) {
            if it.is_last(nodes) {
                break;
            }

            let k = &nodes[it.node()].data;

            let mut idx = self.find(parent_idx, k);

            if idx < 0 {
                idx = self.create(parent_idx, k, KeyKind::Table, false, false);
            } else {
                let entry = &self.entries[idx as usize];
                if entry.kind == KeyKind::Value {
                    return Err(errorf(&[
                        b"toml: expected ",
                        k,
                        b" to be a table, not a ",
                        entry.kind.string().as_bytes(),
                    ]));
                }
            }

            parent_idx = idx;
        }

        let k = &nodes[it.node()].data;
        let mut idx = self.find(parent_idx, k);

        let first_time = idx < 0;
        if first_time {
            idx = self.create(parent_idx, k, KeyKind::ArrayTable, true, false);
        } else {
            let kind = self.entries[idx as usize].kind;
            if kind != KeyKind::ArrayTable {
                // Go passes the arguments in the wrong order (`kind, string(k)`).
                return Err(errorf(&[
                    b"toml: key ",
                    kind.string().as_bytes(),
                    b" already exists as a ",
                    k,
                    b",  but should be an array table",
                ]));
            }
            self.clear(idx);
        }

        self.current_idx = idx;

        Ok(first_time)
    }

    // Go: internal/tracker/seen.go:checkKeyValue
    fn check_key_value(&mut self, nodes: &[Node<'_>], node: usize) -> TrackerResult<bool> {
        let mut parent_idx = self.current_idx;
        let mut it = node_key(nodes, node);

        while it.next(nodes) {
            let k = &nodes[it.node()].data;

            let mut idx = self.find(parent_idx, k);

            if idx < 0 {
                idx = self.create(parent_idx, k, KeyKind::Table, false, true);
            } else {
                let entry = &self.entries[idx as usize];
                if it.is_last(nodes) {
                    return Err(errorf(&[b"toml: key ", k, b" is already defined"]));
                } else if entry.kind != KeyKind::Table {
                    return Err(errorf(&[
                        b"toml: expected ",
                        k,
                        b" to be a table, not a ",
                        entry.kind.string().as_bytes(),
                    ]));
                } else if entry.explicit {
                    return Err(errorf(&[
                        b"toml: cannot redefine table ",
                        k,
                        b" that has already been explicitly defined",
                    ]));
                }
            }

            parent_idx = idx;
        }

        self.entries[parent_idx as usize].kind = KeyKind::Value;

        let value = node_value(nodes, node);

        match nodes[value].kind {
            Kind::InlineTable => check_inline_table(nodes, value),
            Kind::Array => check_array(nodes, value),
            _ => Ok(false),
        }
    }
}

// Go: internal/tracker/seen.go:checkArray
fn check_array(nodes: &[Node<'_>], node: usize) -> TrackerResult<bool> {
    let mut first = false;
    let mut it = node_children(nodes, node);
    while it.next(nodes) {
        let n = it.node();
        match nodes[n].kind {
            Kind::InlineTable => {
                first = check_inline_table(nodes, n)?;
            }
            Kind::Array => {
                first = check_array(nodes, n)?;
            }
            _ => {}
        }
    }
    Ok(first)
}

// Go: internal/tracker/seen.go:checkInlineTable
fn check_inline_table(nodes: &[Node<'_>], node: usize) -> TrackerResult<bool> {
    // Go: s = pool.Get().(*SeenTracker); s.reset()
    let mut s = SeenTracker::default();
    s.reset();

    let mut first = false;
    let mut it = node_children(nodes, node);
    while it.next(nodes) {
        let n = it.node();
        first = s.check_key_value(nodes, n)?;
    }

    // As inline tables are self-contained, the tracker does not
    // need to retain the details of what they contain. The
    // keyValue element that creates the inline table is kept to
    // mark the presence of the inline table and prevent
    // redefinition of its keys: check* functions cannot walk into
    // a value.
    Ok(first)
}
