//! Port of `common/maps/ordered.go`.
//!
//! Owner: Wave B task T01 (common-values).


use std::collections::HashMap;
use std::hash::Hash;

/// Go: `maps.Ordered[K,T]` — a map that remembers insertion order.
#[derive(Clone, Debug)]
pub struct Ordered<K, T> {
    keys: Vec<K>,
    values: HashMap<K, T>,
}

impl<K: Eq + Hash + Clone, T: Clone> Default for Ordered<K, T> {
    fn default() -> Self {
        Ordered { keys: Vec::new(), values: HashMap::new() }
    }
}

impl<K: Eq + Hash + Clone, T: Clone> Ordered<K, T> {
    // Go: common/maps/ordered.go:NewOrdered
    pub fn new() -> Self {
        Self::default()
    }

    // Go: common/maps/ordered.go:Set
    pub fn set(&mut self, key: K, value: T) {
        if !self.values.contains_key(&key) {
            self.keys.push(key.clone());
        }
        self.values.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&T> {
        self.values.get(key)
    }

    pub fn keys(&self) -> &[K] {
        &self.keys
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Go: `Range(f)` in insertion order; stops when `f` returns false.
    // Go: common/maps/ordered.go:Range
    pub fn range(&self, mut f: impl FnMut(&K, &T) -> bool) {
        for k in &self.keys {
            if let Some(v) = self.values.get(k) {
                if !f(k, v) {
                    break;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/ordered.go (144 lines; 2/11 funcs executed)
//   types: Ordered[K
// EX L34-36: NewOrdered[K comparable, T any]() *Ordered[K, T]
//    L40-49: (m *Ordered[K, T]) Set(key K, value T)
//    L52-59: (m *Ordered[K, T]) Get(key K) (T, bool)
//    L62-68: (m *Ordered[K, T]) Has(key K) bool
//    L71-82: (m *Ordered[K, T]) Delete(key K)
//    L85-94: (m *Ordered[K, T]) Clone() *Ordered[K, T]
//    L97-102: (m *Ordered[K, T]) Keys() []K
//    L105-114: (m *Ordered[K, T]) Values() []T
//    L117-122: (m *Ordered[K, T]) Len() int
// EX L127-136: (m *Ordered[K, T]) Range(f func(key K, value T) bool)
//    L139-144: (m *Ordered[K, T]) Hash() (uint64, error)
// ---------------------------------------------------------------------------
