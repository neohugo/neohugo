//! Port of `common/predicate/predicate.go`.
//!
//! Owner: Wave B task T01 (common-values).


/// Go: `predicate.P[T]` — a composable boolean predicate.
pub type P<T> = std::sync::Arc<dyn Fn(&T) -> bool + Send + Sync>;

/// Go: `P.And`.
pub fn and<T: 'static>(a: P<T>, b: P<T>) -> P<T> {
    std::sync::Arc::new(move |v| a(v) && b(v))
}

/// Go: `P.Or`.
pub fn or<T: 'static>(a: P<T>, b: P<T>) -> P<T> {
    std::sync::Arc::new(move |v| a(v) || b(v))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/predicate/predicate.go (78 lines; 3/5 funcs executed)
//   types: P[T
// EX L20-32: (p P[T]) And(ps ...P[T]) P[T]
// EX L35-47: (p P[T]) Or(ps ...P[T]) P[T]
//    L50-54: (p P[T]) Negate() P[T]
// EX L58-67: (p P[T]) Filter(s []T) []T
//    L70-78: (p P[T]) FilterCopy(s []T) []T
// ---------------------------------------------------------------------------
