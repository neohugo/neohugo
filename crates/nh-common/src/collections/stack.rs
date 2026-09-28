//! Port of `common/collections/stack.go`.
//!
//! Owner: Wave B task T01 (common-values).

use std::sync::RwLock;

/// Go: `collections.Stack[T]` — a simple LIFO stack, safe for concurrent use.
#[derive(Default, Debug)]
pub struct Stack<T> {
    items: RwLock<Vec<T>>,
}

impl<T: Clone> Stack<T> {
    // Go: common/collections/stack.go:NewStack
    /// NewStack returns a new Stack.
    pub fn new() -> Self {
        Stack {
            items: RwLock::new(Vec::new()),
        }
    }

    // Go: common/collections/stack.go:Push
    /// Push adds a new item to the stack.
    pub fn push(&self, item: T) {
        self.items
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(item);
    }

    // Go: common/collections/stack.go:Pop
    /// Pop removes the top item from the stack and returns it (`None` = Go's `zero, false`).
    pub fn pop(&self) -> Option<T> {
        self.items.write().unwrap_or_else(|e| e.into_inner()).pop()
    }

    // Go: common/collections/stack.go:Peek
    /// Peek returns the top item on the stack.
    pub fn peek(&self) -> Option<T> {
        self.items
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .last()
            .cloned()
    }

    // Go: common/collections/stack.go:Len
    /// Len returns the number of items on the stack.
    pub fn len(&self) -> usize {
        self.items.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Go: common/collections/stack.go:Drain
    /// Drain removes all items from the stack and returns them (bottom first).
    pub fn drain(&self) -> Vec<T> {
        std::mem::take(&mut *self.items.write().unwrap_or_else(|e| e.into_inner()))
    }

    // Go: common/collections/stack.go:DrainMatching
    /// DrainMatching removes all items matching the predicate and returns them, top first.
    pub fn drain_matching(&self, mut predicate: impl FnMut(&T) -> bool) -> Vec<T> {
        let mut items = self.items.write().unwrap_or_else(|e| e.into_inner());
        let mut out = Vec::new();
        let mut i = items.len();
        while i > 0 {
            i -= 1;
            if predicate(&items[i]) {
                out.push(items.remove(i));
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/collections/stack.go (82 lines; 1/7 funcs executed)
//   types: Stack[T
// OK L27-29: NewStack[T any]() *Stack[T]
// OK L31-35: (s *Stack[T]) Push(item T)
// OK L37-46: (s *Stack[T]) Pop() (T, bool)
// OK L48-55: (s *Stack[T]) Peek() (T, bool)
// OK L57-61: (s *Stack[T]) Len() int
// OK L63-69: (s *Stack[T]) Drain() []T
// OK L71-82: (s *Stack[T]) DrainMatching(predicate func(T) bool) []T
// ---------------------------------------------------------------------------
