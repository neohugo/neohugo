//! Port of `common/maps/cache.go`.
//!
//! Owner: Wave B task T01 (common-values).

//! Go `maps.Cache[K,V]`: a concurrent map with get-or-create where the **first writer wins**.
//! Go runs `create` under the write lock; here it runs without the lock (HUGO_LAYER.md §4.8) and
//! the value is stored only if the key is still absent. With sequential rendering this
//! reproduces Go's single-worker behaviour exactly.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Mutex, MutexGuard};

use crate::herrors::Result;

/// Go: `maps.Cache[K, T]` — a simple thread safe cache backed by a map.
pub struct Cache<K, V> {
    m: Mutex<CacheInner<K, V>>,
}

struct CacheInner<K, V> {
    m: HashMap<K, V>,
    has_been_initialized: bool,
}

impl<K: Eq + Hash + Clone, V: Clone> Default for Cache<K, V> {
    fn default() -> Self {
        Cache {
            m: Mutex::new(CacheInner {
                m: HashMap::new(),
                has_been_initialized: false,
            }),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    // Go: common/maps/cache.go:NewCache
    /// NewCache creates a new Cache.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, CacheInner<K, V>> {
        self.m.lock().unwrap_or_else(|e| e.into_inner())
    }

    // Go: common/maps/cache.go:Get
    pub fn get(&self, key: &K) -> Option<V> {
        self.lock().m.get(key).cloned()
    }

    // Go: common/maps/cache.go:GetOrCreate
    /// GetOrCreate gets the value for the given key if it exists, or creates it if not. Errors
    /// are not cached. `create` runs without the lock; the first stored value wins.
    pub fn get_or_create(&self, key: K, create: impl FnOnce() -> Result<V>) -> Result<V> {
        if let Some(v) = self.get(&key) {
            return Ok(v);
        }
        let v = create()?;
        let mut m = self.lock();
        Ok(m.m.entry(key).or_insert(v).clone())
    }

    // Go: common/maps/cache.go:Contains
    /// Contains returns whether the given key exists in the cache.
    pub fn contains(&self, key: &K) -> bool {
        self.lock().m.contains_key(key)
    }

    // Go: common/maps/cache.go:InitAndGet
    /// InitAndGet initializes the cache (once, until Reset/Drain) and returns the value for the
    /// given key (`None` = Go's zero value). `init` fills the cache through the map it is given.
    pub fn init_and_get(
        &self,
        key: &K,
        init: impl FnOnce(&mut HashMap<K, V>) -> Result<()>,
    ) -> Result<Option<V>> {
        let mut c = self.lock();
        if !c.has_been_initialized {
            // Go runs init under the write lock too (it receives the unlocked get/set).
            init(&mut c.m)?;
            c.has_been_initialized = true;
        }
        Ok(c.m.get(key).cloned())
    }

    // Go: common/maps/cache.go:Set
    /// Set sets the given key to the given value.
    pub fn set(&self, key: K, value: V) {
        self.lock().m.insert(key, value);
    }

    // Go: common/maps/cache.go:SetIfAbsent
    /// SetIfAbsent sets the given key to the given value if the key does not already exist.
    pub fn set_if_absent(&self, key: K, value: V) {
        self.lock().m.entry(key).or_insert(value);
    }

    // Go: common/maps/cache.go:ForEeach
    /// ForEeach calls f for each key/value pair (in unspecified order, like Go's map range)
    /// until it returns false. `f` runs on a snapshot, without the lock.
    pub fn for_each(&self, mut f: impl FnMut(&K, &V) -> bool) {
        let snapshot: Vec<(K, V)> = self
            .lock()
            .m
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (k, v) in &snapshot {
            if !f(k, v) {
                return;
            }
        }
    }

    // Go: common/maps/cache.go:Drain
    /// Drain removes and returns all entries (and resets the init state).
    pub fn drain(&self) -> HashMap<K, V> {
        let mut c = self.lock();
        c.has_been_initialized = false;
        std::mem::take(&mut c.m)
    }

    // Go: common/maps/cache.go:Len
    pub fn len(&self) -> usize {
        self.lock().m.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Go: common/maps/cache.go:Reset
    pub fn reset(&self) {
        let mut c = self.lock();
        c.m.clear();
        c.has_been_initialized = false;
    }
}

/// Go: `maps.SliceCache[T]` — a simple thread safe cache of slices.
pub struct SliceCache<T> {
    m: Mutex<HashMap<String, Vec<T>>>,
}

impl<T: Clone> Default for SliceCache<T> {
    fn default() -> Self {
        SliceCache {
            m: Mutex::new(HashMap::new()),
        }
    }
}

impl<T: Clone> SliceCache<T> {
    // Go: common/maps/cache.go:NewSliceCache
    pub fn new() -> Self {
        Self::default()
    }

    // Go: common/maps/cache.go:(*SliceCache).Get
    pub fn get(&self, key: &str) -> Option<Vec<T>> {
        self.m
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    // Go: common/maps/cache.go:Append
    pub fn append(&self, key: &str, values: &[T]) {
        self.m
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(key.to_string())
            .or_default()
            .extend_from_slice(values);
    }

    // Go: common/maps/cache.go:(*SliceCache).Reset
    pub fn reset(&self) {
        self.m.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/maps/cache.go (195 lines; 10/17 funcs executed)
//   types: Cache[K, SliceCache[T
// OK L28-30: NewCache[K comparable, T any]() *Cache[K, T]
// OK L34-43: (c *Cache[K, T]) Get(key K) (T, bool)
// OK L45-48: (c *Cache[K, T]) get(key K) (T, bool)
// OK L51-70: (c *Cache[K, T]) GetOrCreate(key K, create func() (T, error)) (T, error)
// OK L73-78: (c *Cache[K, T]) Contains(key K) bool
// OK L82-110: (c *Cache[K, T]) InitAndGet(key K, init func(get func(key K) (T, bool), set func(key K, value T)) error) (T, error)
// OK L113-117: (c *Cache[K, T]) Set(key K, value T)
// OK L120-128: (c *Cache[K, T]) SetIfAbsent(key K, value T)
// OK L130-132: (c *Cache[K, T]) set(key K, value T)
// OK L136-144: (c *Cache[K, T]) ForEeach(f func(K, T) bool)
// OK L146-153: (c *Cache[K, T]) Drain() map[K]T
// OK L155-159: (c *Cache[K, T]) Len() int
// OK L161-166: (c *Cache[K, T]) Reset()
// OK L174-176: NewSliceCache[T any]() *SliceCache[T]
// OK L178-183: (c *SliceCache[T]) Get(key string) ([]T, bool)
// OK L185-189: (c *SliceCache[T]) Append(key string, values ...T)
// OK L191-195: (c *SliceCache[T]) Reset()
// ---------------------------------------------------------------------------
