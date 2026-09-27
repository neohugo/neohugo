//! The `hash.Hash64` implementations used with hashstructure.

/// Go's `hash.Hash64` subset hashstructure uses.
pub trait Hasher64 {
    fn reset(&mut self);
    fn write(&mut self, b: &[u8]);
    fn sum64(&self) -> u64;
}

/// github.com/cespare/xxhash/v2 `xxhash.New()` (XXH64, seed 0) — the hasher
/// neohugo's `common/hashing` passes to hashstructure.
#[derive(Clone)]
pub struct XxHash64(xxhash_rust::xxh64::Xxh64);

impl XxHash64 {
    pub fn new() -> XxHash64 {
        XxHash64(xxhash_rust::xxh64::Xxh64::new(0))
    }
}

impl Default for XxHash64 {
    fn default() -> Self {
        XxHash64::new()
    }
}

impl Hasher64 for XxHash64 {
    fn reset(&mut self) {
        self.0.reset(0);
    }
    fn write(&mut self, b: &[u8]) {
        self.0.update(b);
    }
    fn sum64(&self) -> u64 {
        self.0.digest()
    }
}

/// Go's `hash/fnv.New64()` (FNV-1, 64 bit): hashstructure's default hasher.
#[derive(Clone)]
pub struct Fnv64(u64);

const OFFSET64: u64 = 14695981039346656037;
const PRIME64: u64 = 1099511628211;

impl Fnv64 {
    pub fn new() -> Fnv64 {
        Fnv64(OFFSET64)
    }
}

impl Default for Fnv64 {
    fn default() -> Self {
        Fnv64::new()
    }
}

impl Hasher64 for Fnv64 {
    // Go: hash/fnv/fnv.go:(*sum64).Reset
    fn reset(&mut self) {
        self.0 = OFFSET64;
    }
    // Go: hash/fnv/fnv.go:(*sum64).Write
    fn write(&mut self, data: &[u8]) {
        let mut hash = self.0;
        for &c in data {
            hash = hash.wrapping_mul(PRIME64);
            hash ^= c as u64;
        }
        self.0 = hash;
    }
    fn sum64(&self) -> u64 {
        self.0
    }
}
