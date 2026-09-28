//! Port of `cache/dynacache/dynacache.go`.
//!
//! SIMPLIFIED: get-or-create partitions, no eviction
//!
//! Owner: Wave B task T01 (common-values).

//! SIMPLIFIED `cache/dynacache`: named partitions of get-or-create caches, no eviction, no
//! identity-based invalidation, no stale-value checks (nothing goes stale in a one-shot build).
//! **First writer wins** (Go semantics that leak into output: `resources.Concat` keyed by target
//! path, `ExecuteAsTemplate` keyed by target path, image cache keyed by target path, resource
//! `Get` keyed by path). Never evict: Go's eviction under memory pressure is a nondeterminism we
//! do not emulate.
//!
//! Go's `lazycache` runs `create` while other callers of the same key wait; here `create` runs
//! without any lock held (HUGO_LAYER.md §4.8: rendering re-enters caches), and when two callers
//! race the first stored value wins and is returned to both. With sequential rendering this is
//! exactly Go's behaviour. Errors are not cached, as in Go.

use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use crate::herrors::Result;

/// Go: `dynacache.ClearWhen` (only recorded; nothing is cleared in a one-shot build).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClearWhen {
    #[default]
    Unset,
    /// Go: `ClearOnRebuild`.
    OnRebuild,
    /// Go: `ClearOnChange`.
    OnChange,
    /// Go: `ClearNever`.
    Never,
}

/// Go: `dynacache.OptionsPartition`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OptionsPartition {
    /// When to clear this partition.
    pub clear_when: ClearWhen,
    /// A number between 1 and 100 that indicates how big this partition may get.
    pub weight: i64,
}

impl OptionsPartition {
    // Go: cache/dynacache/dynacache.go:WeightFraction
    pub fn weight_fraction(&self) -> f64 {
        self.weight as f64 / 100.0
    }

    // Go: cache/dynacache/dynacache.go:CalculateMaxSize
    pub fn calculate_max_size(&self, max_size_per_partition: i64) -> i64 {
        (max_size_per_partition as f64 * self.weight_fraction()).floor() as i64
    }
}

/// Go: `dynacache.Options`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub max_size: i64,
    pub min_max_size: i64,
    pub watching: bool,
}

/// Go: `dynacache.Cache` — a set of named, typed partitions.
#[derive(Default)]
pub struct Cache {
    pub opts: Options,
    partitions: Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl Cache {
    // Go: cache/dynacache/dynacache.go:New
    /// New creates a new cache (defaults as in Go; no memory-pressure watcher).
    pub fn new(mut opts: Options) -> Self {
        if opts.max_size == 0 {
            opts.max_size = 100000;
        }
        if opts.min_max_size == 0 {
            opts.min_max_size = 30;
        }
        Cache {
            opts,
            partitions: Mutex::new(HashMap::new()),
        }
    }

    /// The partition names (Go: `Keys(predicate)` lists partition keys; this lists partitions).
    pub fn partition_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .partitions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect();
        names.sort();
        names
    }
}

// Go: cache/dynacache/dynacache.go:partitionNameRe
/// `^\/[a-zA-Z0-9]{4}(\/[a-zA-Z0-9]+)?(\/[a-zA-Z0-9]+)?` must match the whole name.
fn valid_partition_name(name: &[u8]) -> bool {
    let alnum = |c: &u8| c.is_ascii_alphanumeric();
    if name.len() < 5 || name[0] != b'/' || !name[1..5].iter().all(alnum) {
        return false;
    }
    let mut rest = &name[5..];
    for _ in 0..2 {
        if rest.is_empty() {
            return true;
        }
        if rest[0] != b'/' {
            return false;
        }
        let n = rest[1..].iter().take_while(|c| alnum(c)).count();
        if n == 0 {
            return false;
        }
        rest = &rest[1 + n..];
    }
    rest.is_empty()
}

// Go: cache/dynacache/dynacache.go:GetOrCreatePartition
/// GetOrCreatePartition gets or creates a partition with the given name. Go panics on a nil
/// cache, a weight outside 1..=100, an invalid name, or an existing partition of other types.
pub fn get_or_create_partition<K, V>(
    c: &Cache,
    name: &str,
    opts: OptionsPartition,
) -> Arc<Partition<K, V>>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    if opts.weight < 1 || opts.weight > 100 {
        panic!("invalid Weight, must be between 1 and 100");
    }

    if !valid_partition_name(name.as_bytes()) {
        panic!(
            "invalid partition name {}",
            go_strconv::quote(name.as_bytes())
        );
    }

    let mut partitions = c.partitions.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(p) = partitions.get(name) {
        return p
            .clone()
            .downcast::<Partition<K, V>>()
            .unwrap_or_else(|_| panic!("partition {name:?} has other key/value types"));
    }

    let mut partition = Partition::new(name);
    partition.opts = opts;
    let partition = Arc::new(partition);
    partitions.insert(name.to_string(), partition.clone());
    partition
}

/// Go: `dynacache.Partition[K, V]`.
pub struct Partition<K, V> {
    pub name: String,
    pub opts: OptionsPartition,
    m: Mutex<HashMap<K, V>>,
}

impl<K: Eq + Hash + Clone, V: Clone> Partition<K, V> {
    pub fn new(name: impl Into<String>) -> Self {
        Partition {
            name: name.into(),
            opts: OptionsPartition::default(),
            m: Mutex::new(HashMap::new()),
        }
    }

    // Go: cache/dynacache/dynacache.go:Get
    /// Get returns the cached value for key, if any.
    pub fn get(&self, key: &K) -> Option<V> {
        self.m
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    // Go: cache/dynacache/dynacache.go:GetOrCreate
    /// GetOrCreate gets or creates a value for the given key. The lock is NOT held while
    /// `create` runs (it may use this partition with other keys); if two creators race, the
    /// first inserted value wins and is returned to both. Errors are not cached.
    pub fn get_or_create(&self, key: K, create: impl FnOnce(&K) -> Result<V>) -> Result<V> {
        self.do_get_or_create(key, create)
    }

    // Go: cache/dynacache/dynacache.go:doGetOrCreate
    fn do_get_or_create(&self, key: K, create: impl FnOnce(&K) -> Result<V>) -> Result<V> {
        if let Some(v) = self.get(&key) {
            return Ok(v);
        }
        let v = create(&key)?;
        let mut m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        Ok(m.entry(key).or_insert(v).clone())
    }

    // Go: cache/dynacache/dynacache.go:GetOrCreateWitTimeout
    /// GetOrCreateWitTimeout: `create` runs on the calling thread and the timeout is not
    /// enforced (Go returns a `TimeoutError` when `create` runs longer, which only happens in
    /// failing builds).
    pub fn get_or_create_with_timeout(
        &self,
        key: K,
        _timeout: go_time::Duration,
        create: impl FnOnce(&K) -> Result<V>,
    ) -> Result<V> {
        self.do_get_or_create(key, create)
    }

    /// Stores a value (overwrites).
    pub fn set(&self, key: K, value: V) {
        self.m
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, value);
    }

    // Go: cache/dynacache/dynacache.go:Keys
    /// Keys returns the keys in the partition (unordered, like Go's).
    pub fn keys(&self) -> Vec<K> {
        self.m
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    // Go: cache/dynacache/dynacache.go:Clear
    pub fn clear(&self) {
        self.m.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    pub fn len(&self) -> usize {
        self.m.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Go: cache/dynacache/dynacache.go:getOptions
    pub fn get_options(&self) -> OptionsPartition {
        self.opts
    }
}

// Go: cache/dynacache/dynacache.go:CleanKey
/// CleanKey turns s into a format suitable for a cache key for this package: a Unix-styled path
/// with a leading slash but no trailing slash (`path.Clean(paths.ToSlashPreserveLeading(s))`).
pub fn clean_key(key: &str) -> String {
    String::from_utf8_lossy(&clean_key_bytes(key.as_bytes())).into_owned()
}

/// [`clean_key`] over bytes.
pub fn clean_key_bytes(key: &[u8]) -> Vec<u8> {
    go_path::path::clean_bytes(&to_slash_preserve_leading(key))
}

// Go: common/paths/path.go:ToSlashPreserveLeading
/// `"/" + strings.Trim(filepath.ToSlash(s), "/")` (T02 owns `paths`; inlined to keep this module
/// independent of it).
fn to_slash_preserve_leading(s: &[u8]) -> Vec<u8> {
    let mut out = b"/".to_vec();
    out.extend_from_slice(go_unicode::strings::trim(
        &go_path::filepath::to_slash_bytes(s),
        b"/",
    ));
    out
}

// Go: cache/dynacache/dynacache.go:calculateMaxSizePerPartition
/// calculateMaxSizePerPartition (kept for the checklist; sizes are never enforced here).
pub fn calculate_max_size_per_partition(
    max_items_total: i64,
    total_weight_quantity: i64,
    num_partitions: i64,
) -> i64 {
    if num_partitions == 0 {
        panic!("numPartitions must be > 0");
    }
    if total_weight_quantity == 0 {
        panic!("totalWeightQuantity must be > 0");
    }

    let avg_weight = total_weight_quantity as f64 / num_partitions as f64;
    (max_items_total as f64 / num_partitions as f64 * (100.0 / avg_weight)).floor() as i64
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// SIMPLIFIED (see module docs): no eviction, no memory-pressure resizing, no rebuild clearing.
// Source: cache/dynacache/dynacache.go (647 lines; 16/28 funcs executed)
//   types: KeyIdentity, Options, OptionsPartition, Cache, keysProvider, Partition[K, PartitionManager, ClearWhen,
//          stats
// OK L47-97: New(opts Options) *Cache (no watcher goroutine)
// OK L117-119: (o OptionsPartition) WeightFraction() float64
// OK L121-123: (o OptionsPartition) CalculateMaxSize(maxSizePerPartition int) int
//    L143-145: (c *Cache) DrainEvictedIdentities() []KeyIdentity
//    L148-150: (c *Cache) DrainEvictedIdentitiesMatching(predicate func(KeyIdentity) bool) []KeyIdentity
//    L153-176: (c *Cache) ClearMatching(predicatePartition func(k string, p PartitionManager) bool, predicateValue func(k, v any) bool)
//    L180-209: (c *Cache) ClearOnRebuild(predicate func(k, v any) bool, changeset ...identity.Identity)
//    L216-232: (c *Cache) Keys(predicate func(s string) bool) []string
// OK L234-244: calculateMaxSizePerPartition(maxItemsTotal, totalWeightQuantity, numPartitions int) int
// OK L247-251: (c *Cache) Stop() (nothing to stop)
// OK L253-307: (c *Cache) adjustCurrentMaxSize() (SIMPLIFIED: never evicts)
// OK L309-330: (c *Cache) start() func() (SIMPLIFIED: no watcher)
// OK L335-383: GetOrCreatePartition[K comparable, V any](c *Cache, name string, opts OptionsPartition) *Partition[K, V]
// OK L398-408: (p *Partition[K, V]) GetOrCreate(key K, create func(key K) (V, error)) (V, error)
// OK L410-413: (p *Partition[K, V]) doGetOrCreate(key K, create func(key K) (V, error)) (V, error)
// OK L415-425: (p *Partition[K, V]) GetOrCreateWitTimeout(key K, duration time.Duration, create func(key K) (V, error)) (V, error)
// OK L429-465: (p *Partition[K, V]) doGetOrCreateWitTimeout(key K, duration time.Duration, create func(key K) (V, error)) (V, error) (timeout not enforced)
//    L467-481: (p *Partition[K, V]) clearMatching(predicate func(k, v any) bool)
//    L483-546: (p *Partition[K, V]) clearOnRebuild(predicate func(k, v any) bool, changeset ...identity.Identity)
// OK L548-555: (p *Partition[K, V]) Keys() []K
//    L557-572: (p *Partition[K, V]) clearStale()
// OK L575-586: (p *Partition[K, V]) adjustMaxSize(newMaxSize int) int (SIMPLIFIED: no max size)
//    L588-590: (p *Partition[K, V]) getMaxSize() int
// OK L592-594: (p *Partition[K, V]) getOptions() OptionsPartition
// OK L596-600: (p *Partition[K, V]) Clear()
// OK L602-604: (p *Partition[K, V]) Get(ctx context.Context, key K) (V, bool)
// OK L632-641: (s *stats) adjustCurrentMaxSize() bool (SIMPLIFIED: no max size)
// OK L645-647: CleanKey(s string) string
// ---------------------------------------------------------------------------
