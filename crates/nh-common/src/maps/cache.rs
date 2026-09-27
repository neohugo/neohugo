//! Port of `common/maps/cache.go`.
//!
//! Owner: Wave B task T01 (common-values).


//! Go `maps.Cache[K,V]`: a concurrent map with get-or-create where the **first writer wins**.
//! With sequential rendering this reproduces Go's single-worker behaviour exactly.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

use crate::herrors::Result;

pub struct Cache<K, V> {
    m: Mutex<HashMap<K, V>>,
}

impl<K: Eq + Hash + Clone, V: Clone> Default for Cache<K, V> {
    fn default() -> Self {
        Cache { m: Mutex::new(HashMap::new()) }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    // Go: common/maps/cache.go:NewCache
    pub fn new() -> Self {
        Self::default()
    }

    // Go: common/maps/cache.go:Get
    pub fn get(&self, key: &K) -> Option<V> {
        self.m.lock().unwrap().get(key).cloned()
    }

    /// Go: `GetOrCreate`: returns the cached value, or runs `create` once and caches its result.
    /// Errors are not cached. Must not be re-entered for the same key from inside `create`.
    // Go: common/maps/cache.go:GetOrCreate
    pub fn get_or_create(&self, key: K, create: impl FnOnce() -> Result<V>) -> Result<V> {
        if let Some(v) = self.get(&key) {
            return Ok(v);
        }
        let v = create()?;
        let mut m = self.m.lock().unwrap();
        Ok(m.entry(key).or_insert(v).clone())
    }

    // Go: common/maps/cache.go:Set
    pub fn set(&self, key: K, value: V) {
        self.m.lock().unwrap().insert(key, value);
    }

    pub fn len(&self) -> usize {
        self.m.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn reset(&self) {
        self.m.lock().unwrap().clear();
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/cache.go (195 lines; 10/17 funcs executed)
//   types: Cache[K, SliceCache[T
// EX L28-30: NewCache[K comparable, T any]() *Cache[K, T]
// EX L34-43: (c *Cache[K, T]) Get(key K) (T, bool)
// EX L45-48: (c *Cache[K, T]) get(key K) (T, bool)
// EX L51-70: (c *Cache[K, T]) GetOrCreate(key K, create func() (T, error)) (T, error)
//    L73-78: (c *Cache[K, T]) Contains(key K) bool
//    L82-110: (c *Cache[K, T]) InitAndGet(key K, init func(get func(key K) (T, bool), set func(key K, value T)) error) (T, error)
// EX L113-117: (c *Cache[K, T]) Set(key K, value T)
//    L120-128: (c *Cache[K, T]) SetIfAbsent(key K, value T)
// EX L130-132: (c *Cache[K, T]) set(key K, value T)
//    L136-144: (c *Cache[K, T]) ForEeach(f func(K, T) bool)
//    L146-153: (c *Cache[K, T]) Drain() map[K]T
// EX L155-159: (c *Cache[K, T]) Len() int
// EX L161-166: (c *Cache[K, T]) Reset()
// EX L174-176: NewSliceCache[T any]() *SliceCache[T]
//    L178-183: (c *SliceCache[T]) Get(key string) ([]T, bool)
//    L185-189: (c *SliceCache[T]) Append(key string, values ...T)
// EX L191-195: (c *SliceCache[T]) Reset()
// ---------------------------------------------------------------------------
