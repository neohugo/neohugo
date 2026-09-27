//! Port of `common/types/types.go`, `common/types/closer.go`, `common/types/evictingqueue.go`.
//!
//! Owner: Wave B task T01 (common-values).


use go_value::{GoString, Value};

/// Go: `types.LowHigh[S]` — a sub-slice range `[low, high)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LowHigh {
    pub low: usize,
    pub high: usize,
}

impl LowHigh {
    pub fn is_zero(&self) -> bool {
        self.low == 0 && self.high == 0
    }

    pub fn value<'a>(&self, source: &'a [u8]) -> &'a [u8] {
        &source[self.low..self.high]
    }
}

/// Go: `types.KeyValue`.
#[derive(Clone, Debug)]
pub struct KeyValue {
    pub key: Value,
    pub value: Value,
}

/// Go: `types.KeyValues` (key + many values; used by related `.RelatedTo`).
#[derive(Clone, Debug)]
pub struct KeyValues {
    pub key: Value,
    pub values: Vec<Value>,
}

/// Go: `types.KeyValueStr`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyValueStr {
    pub key: String,
    pub value: String,
}

/// Go: `types.Zeroer` — see `go_value::Object::is_zero`.
pub trait Zeroer {
    fn is_zero(&self) -> bool;
}

/// Go: `types.EvictingQueue` (only used for rebuild bookkeeping; kept for API shape).
pub struct EvictingQueue<T> {
    pub size: usize,
    pub vals: Vec<T>,
}

/// Go: `types.Closers` — `Close()` all in order.
#[derive(Default)]
pub struct Closers {
    pub closers: std::sync::Mutex<Vec<Box<dyn FnOnce() -> crate::herrors::Result<()> + Send>>>,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/types/types.go (145 lines; 2/8 funcs executed)
//   types: RLocker, Locker, RWLocker, KeyValue, KeyValueStr, KeyValues, Zeroer, DevMarker, Unwrapper, LowHigh[S,
//          PrintableValueProvider
//    L60-62: (k KeyValues) KeyString() string
//    L64-66: (k KeyValues) String() string
//    L70-76: NewKeyValuesStrings(key string, values ...string) KeyValues
// EX L85-97: IsNil(v any) bool
//    L113-118: Unwrapv(v any) any
// EX L126-128: (l LowHigh[S]) IsZero() bool
//    L130-132: (l LowHigh[S]) Value(source S) S
//    L138-140: NewBool(b bool) *bool
// Source: common/types/closer.go (54 lines; 1/3 funcs executed)
//   types: Closer, CloserFunc, CloseAdder, Closers
//    L25-27: (f CloserFunc) Close() error
//    L38-42: (cs *Closers) Add(c Closer)
// EX L44-54: (cs *Closers) Close() error
// Source: common/types/evictingqueue.go (114 lines; 0/7 funcs executed)
//   types: EvictingQueue[T
//    L34-36: NewEvictingQueue[T comparable](size int) *EvictingQueue[T]
//    L39-56: (q *EvictingQueue[T]) Add(v T) *EvictingQueue[T]
//    L58-65: (q *EvictingQueue[T]) Len() int
//    L68-75: (q *EvictingQueue[T]) Contains(v T) bool
//    L78-88: (q *EvictingQueue[T]) Peek() T
//    L91-103: (q *EvictingQueue[T]) PeekAll() []T
//    L106-114: (q *EvictingQueue[T]) PeekAllSet() map[T]bool
// ---------------------------------------------------------------------------
