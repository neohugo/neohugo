//! Port of `common/collections/stack.go`.
//!
//! Owner: Wave B task T01 (common-values).


/// Go: `collections.Stack[T]`.
#[derive(Default, Debug, Clone)]
pub struct Stack<T> {
    items: Vec<T>,
}

impl<T: Clone> Stack<T> {
    // Go: common/collections/stack.go:NewStack
    pub fn new() -> Self {
        Stack { items: Vec::new() }
    }
    pub fn push(&mut self, v: T) {
        self.items.push(v);
    }
    pub fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }
    pub fn peek(&self) -> Option<&T> {
        self.items.last()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/collections/stack.go (82 lines; 1/7 funcs executed)
//   types: Stack[T
// EX L27-29: NewStack[T any]() *Stack[T]
//    L31-35: (s *Stack[T]) Push(item T)
//    L37-46: (s *Stack[T]) Pop() (T, bool)
//    L48-55: (s *Stack[T]) Peek() (T, bool)
//    L57-61: (s *Stack[T]) Len() int
//    L63-69: (s *Stack[T]) Drain() []T
//    L71-82: (s *Stack[T]) DrainMatching(predicate func(T) bool) []T
// ---------------------------------------------------------------------------
