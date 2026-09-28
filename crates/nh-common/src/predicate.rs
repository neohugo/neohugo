//! Port of `common/predicate/predicate.go`.
//!
//! Owner: Wave B task T01 (common-values).

use std::sync::Arc;

/// Go: `predicate.P[T]` — a predicate function that tests whether a value of type T satisfies
/// some condition. A nil Go predicate is `None` where Go allows one (the receiver of `And`/`Or`).
pub type P<T> = Arc<dyn Fn(&T) -> bool + Send + Sync>;

/// Wraps a closure as a predicate.
pub fn new<T: 'static>(f: impl Fn(&T) -> bool + Send + Sync + 'static) -> P<T> {
    Arc::new(f)
}

// Go: common/predicate/predicate.go:And
/// And returns a predicate that is a short-circuiting logical AND of `p` and the given predicates:
/// `ps` are evaluated first, then `p` (a nil `p` is true).
pub fn and<T: 'static>(p: Option<P<T>>, ps: Vec<P<T>>) -> P<T> {
    Arc::new(move |v| {
        for pp in &ps {
            if !pp(v) {
                return false;
            }
        }
        match &p {
            None => true,
            Some(p) => p(v),
        }
    })
}

// Go: common/predicate/predicate.go:Or
/// Or returns a predicate that is a short-circuiting logical OR of `p` and the given predicates:
/// `ps` are evaluated first, then `p` (a nil `p` is false).
pub fn or<T: 'static>(p: Option<P<T>>, ps: Vec<P<T>>) -> P<T> {
    Arc::new(move |v| {
        for pp in &ps {
            if pp(v) {
                return true;
            }
        }
        match &p {
            None => false,
            Some(p) => p(v),
        }
    })
}

// Go: common/predicate/predicate.go:Negate
/// Negate returns a predicate that is a logical negation of this predicate.
pub fn negate<T: 'static>(p: P<T>) -> P<T> {
    Arc::new(move |v| !p(v))
}

// Go: common/predicate/predicate.go:Filter
/// Filter keeps only the elements of s that satisfy p, in place (Go returns the shortened slice
/// that shares s's array).
pub fn filter<T>(p: &P<T>, s: &mut Vec<T>) {
    s.retain(|v| p(v));
}

// Go: common/predicate/predicate.go:FilterCopy
/// FilterCopy returns a new slice holding only the elements of s that satisfy p.
pub fn filter_copy<T: Clone>(p: &P<T>, s: &[T]) -> Vec<T> {
    s.iter().filter(|v| p(v)).cloned().collect()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/predicate/predicate.go (78 lines; 3/5 funcs executed)
//   types: P[T
// OK L20-32: (p P[T]) And(ps ...P[T]) P[T]
// OK L35-47: (p P[T]) Or(ps ...P[T]) P[T]
// OK L50-54: (p P[T]) Negate() P[T]
// OK L58-67: (p P[T]) Filter(s []T) []T
// OK L70-78: (p P[T]) FilterCopy(s []T) []T
// ---------------------------------------------------------------------------
