//! Port of `common/maps/ordered.go`.
//!
//! Owner: Wave B task T01 (common-values).

use std::collections::HashMap;
use std::hash::Hash;

/// Go: `maps.Ordered[K,T]` — a map that can be iterated in the order of insertion. Insertion
/// order is not affected if a key is re-inserted. Not thread safe.
#[derive(Clone, Debug)]
pub struct Ordered<K, T> {
    /// The keys in the order they were added.
    keys: Vec<K>,
    /// The values.
    values: HashMap<K, T>,
}

impl<K: Eq + Hash + Clone, T: Clone> Default for Ordered<K, T> {
    fn default() -> Self {
        Ordered {
            keys: Vec::new(),
            values: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash + Clone, T: Clone> Ordered<K, T> {
    // Go: common/maps/ordered.go:NewOrdered
    /// NewOrdered creates a new Ordered map.
    pub fn new() -> Self {
        Self::default()
    }

    // Go: common/maps/ordered.go:Set
    /// Set sets the value for the given key (a re-inserted key keeps its position).
    pub fn set(&mut self, key: K, value: T) {
        // Check if key already exists.
        if !self.values.contains_key(&key) {
            self.keys.push(key.clone());
        }
        self.values.insert(key, value);
    }

    // Go: common/maps/ordered.go:Get
    /// Get gets the value for the given key.
    pub fn get(&self, key: &K) -> Option<&T> {
        self.values.get(key)
    }

    // Go: common/maps/ordered.go:Has
    /// Has returns whether the given key exists in the map.
    pub fn has(&self, key: &K) -> bool {
        self.values.contains_key(key)
    }

    // Go: common/maps/ordered.go:Delete
    /// Delete deletes the value for the given key.
    pub fn delete(&mut self, key: &K) {
        self.values.remove(key);
        if let Some(i) = self.keys.iter().position(|k| k == key) {
            self.keys.remove(i);
        }
    }

    // Go: common/maps/ordered.go:Clone
    /// Clone creates a shallow copy of the map.
    pub fn clone_ordered(&self) -> Self {
        let mut clone = Self::new();
        for k in &self.keys {
            if let Some(v) = self.values.get(k) {
                clone.set(k.clone(), v.clone());
            }
        }
        clone
    }

    // Go: common/maps/ordered.go:Keys
    /// Keys returns the keys in the order they were added.
    pub fn keys(&self) -> &[K] {
        &self.keys
    }

    // Go: common/maps/ordered.go:Values
    /// Values returns the values in the order they were added.
    pub fn values(&self) -> Vec<T> {
        self.keys
            .iter()
            .filter_map(|k| self.values.get(k).cloned())
            .collect()
    }

    // Go: common/maps/ordered.go:Len
    /// Len returns the number of items in the map.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    // Go: common/maps/ordered.go:Range
    /// Range calls f sequentially for each key and value present in the map, in insertion
    /// order. If f returns false, range stops the iteration.
    pub fn range(&self, mut f: impl FnMut(&K, &T) -> bool) {
        for k in &self.keys {
            if let Some(v) = self.values.get(k)
                && !f(k, v)
            {
                break;
            }
        }
    }

    // Go: common/maps/ordered.go:Hash
    /// Hash calculates a hash from the values: Go hashes the `map[K]T` with hashstructure
    /// (an unordered map hash); `to_hash` gives each entry's (key, value) as hashstructure sees
    /// them.
    pub fn hash(
        &self,
        to_hash: impl Fn(&K, &T) -> (go_hashstructure::HashValue, go_hashstructure::HashValue),
    ) -> crate::herrors::Result<u64> {
        let entries = self
            .keys
            .iter()
            .filter_map(|k| self.values.get(k).map(|v| to_hash(k, v)))
            .collect();
        crate::hashing::hash_values(&[go_hashstructure::HashValue::Map(
            go_hashstructure::GoMap::new(entries),
        )])
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/ordered.go (144 lines; 2/11 funcs executed)
//   types: Ordered[K
// OK L34-36: NewOrdered[K comparable, T any]() *Ordered[K, T]
// OK L40-49: (m *Ordered[K, T]) Set(key K, value T)
// OK L52-59: (m *Ordered[K, T]) Get(key K) (T, bool)
// OK L62-68: (m *Ordered[K, T]) Has(key K) bool
// OK L71-82: (m *Ordered[K, T]) Delete(key K)
// OK L85-94: (m *Ordered[K, T]) Clone() *Ordered[K, T]
// OK L97-102: (m *Ordered[K, T]) Keys() []K
// OK L105-114: (m *Ordered[K, T]) Values() []T
// OK L117-122: (m *Ordered[K, T]) Len() int
// OK L127-136: (m *Ordered[K, T]) Range(f func(key K, value T) bool)
// OK L139-144: (m *Ordered[K, T]) Hash() (uint64, error)
// ---------------------------------------------------------------------------
