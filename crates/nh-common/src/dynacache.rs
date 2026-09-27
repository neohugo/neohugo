//! Port of `cache/dynacache/dynacache.go`.
//!
//! SIMPLIFIED: get-or-create partitions, no eviction
//!
//! Owner: Wave B task T01 (common-values).


//! SIMPLIFIED `cache/dynacache`: named partitions of get-or-create caches, no eviction, no
//! identity-based invalidation. **First writer wins** (Go semantics that leak into output:
//! `resources.Concat` keyed by target path, `ExecuteAsTemplate` keyed by target path, image cache
//! keyed by target path, resource `Get` keyed by path). Never evict: Go's eviction under memory
//! pressure is a nondeterminism we do not emulate.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

use crate::herrors::Result;

/// Go: `dynacache.Partition[K, V]`.
pub struct Partition<K, V> {
    pub name: String,
    m: Mutex<HashMap<K, V>>,
}

impl<K: Eq + Hash + Clone, V: Clone> Partition<K, V> {
    pub fn new(name: impl Into<String>) -> Self {
        Partition { name: name.into(), m: Mutex::new(HashMap::new()) }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.m.lock().unwrap().get(key).cloned()
    }

    /// Go: `Partition.GetOrCreate(key, create)`. The lock is NOT held while `create` runs (create may
    /// recursively use the same partition with other keys, as Go allows); if two creators race,
    /// the first inserted value wins and is returned to both.
    // Go: cache/dynacache/dynacache.go:GetOrCreate
    pub fn get_or_create(&self, key: K, create: impl FnOnce(&K) -> Result<V>) -> Result<V> {
        if let Some(v) = self.get(&key) {
            return Ok(v);
        }
        let v = create(&key)?;
        let mut m = self.m.lock().unwrap();
        Ok(m.entry(key).or_insert(v).clone())
    }

    pub fn set(&self, key: K, value: V) {
        self.m.lock().unwrap().insert(key, value);
    }

    pub fn len(&self) -> usize {
        self.m.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Go: `dynacache.CleanKey(key)` — `path.Clean(strings.ToLower(filepath.ToSlash(key)))`, leading "/" trimmed.
// Go: cache/dynacache/dynacache.go:CleanKey
pub fn clean_key(key: &str) -> String {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: cache/dynacache/dynacache.go (647 lines; 16/28 funcs executed)
//   types: KeyIdentity, Options, OptionsPartition, Cache, keysProvider, Partition[K, PartitionManager, ClearWhen,
//          stats
// EX L47-97: New(opts Options) *Cache
// EX L117-119: (o OptionsPartition) WeightFraction() float64
// EX L121-123: (o OptionsPartition) CalculateMaxSize(maxSizePerPartition int) int
//    L143-145: (c *Cache) DrainEvictedIdentities() []KeyIdentity
//    L148-150: (c *Cache) DrainEvictedIdentitiesMatching(predicate func(KeyIdentity) bool) []KeyIdentity
//    L153-176: (c *Cache) ClearMatching(predicatePartition func(k string, p PartitionManager) bool, predicateValue func(k, v any) bool)
//    L180-209: (c *Cache) ClearOnRebuild(predicate func(k, v any) bool, changeset ...identity.Identity)
//    L216-232: (c *Cache) Keys(predicate func(s string) bool) []string
// EX L234-244: calculateMaxSizePerPartition(maxItemsTotal, totalWeightQuantity, numPartitions int) int
// EX L247-251: (c *Cache) Stop()
// EX L253-307: (c *Cache) adjustCurrentMaxSize()
// EX L309-330: (c *Cache) start() func()
// EX L335-383: GetOrCreatePartition[K comparable, V any](c *Cache, name string, opts OptionsPartition) *Partition[K, V]
// EX L398-408: (p *Partition[K, V]) GetOrCreate(key K, create func(key K) (V, error)) (V, error)
// EX L410-413: (p *Partition[K, V]) doGetOrCreate(key K, create func(key K) (V, error)) (V, error)
// EX L415-425: (p *Partition[K, V]) GetOrCreateWitTimeout(key K, duration time.Duration, create func(key K) (V, error)) (V, error)
// EX L429-465: (p *Partition[K, V]) doGetOrCreateWitTimeout(key K, duration time.Duration, create func(key K) (V, error)) (V, error)
//    L467-481: (p *Partition[K, V]) clearMatching(predicate func(k, v any) bool)
//    L483-546: (p *Partition[K, V]) clearOnRebuild(predicate func(k, v any) bool, changeset ...identity.Identity)
//    L548-555: (p *Partition[K, V]) Keys() []K
//    L557-572: (p *Partition[K, V]) clearStale()
// EX L575-586: (p *Partition[K, V]) adjustMaxSize(newMaxSize int) int
//    L588-590: (p *Partition[K, V]) getMaxSize() int
// EX L592-594: (p *Partition[K, V]) getOptions() OptionsPartition
//    L596-600: (p *Partition[K, V]) Clear()
//    L602-604: (p *Partition[K, V]) Get(ctx context.Context, key K) (V, bool)
// EX L632-641: (s *stats) adjustCurrentMaxSize() bool
// EX L645-647: CleanKey(s string) string
// ---------------------------------------------------------------------------
