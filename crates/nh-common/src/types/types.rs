//! Port of `common/types/types.go`, `common/types/closer.go`, `common/types/evictingqueue.go`.
//!
//! Owner: Wave B task T01 (common-values).

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Mutex;

use go_value::{GoString, Value};

/// Go: `types.LowHigh[S]` — a sub-slice range `[low, high)` of a string or byte slice.
/// (Go's `Low` is an `int`; neohugo never stores a negative one.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LowHigh {
    pub low: usize,
    pub high: usize,
}

impl LowHigh {
    // Go: common/types/types.go:IsZero
    /// Go: `l.Low < 0 || (l.Low == 0 && l.High == 0)`.
    pub fn is_zero(&self) -> bool {
        self.low == 0 && self.high == 0
    }

    // Go: common/types/types.go:Value
    pub fn value<'a>(&self, source: &'a [u8]) -> &'a [u8] {
        &source[self.low..self.high]
    }
}

/// Go: `types.KeyValue` (an `interface{}` tuple).
#[derive(Clone, Debug)]
pub struct KeyValue {
    pub key: Value,
    pub value: Value,
}

/// Go: `types.KeyValues` — a key and a slice of values (used by related `.RelatedTo`).
#[derive(Clone, Debug)]
pub struct KeyValues {
    pub key: Value,
    pub values: Vec<Value>,
}

impl KeyValues {
    // Go: common/types/types.go:KeyString
    /// KeyString returns the key as a string, an empty string if conversion fails.
    pub fn key_string(&self) -> GoString {
        crate::cast::caste::to_string(&self.key)
    }

    // Go: common/types/types.go:String
    /// Go: `fmt.Sprintf("%v: %v", k.Key, k.Values)` (`Values` is a `[]interface {}`).
    pub fn string(&self) -> Vec<u8> {
        go_fmt::sprintf(
            "%v: %v",
            &[self.key.clone(), Value::any_list(self.values.clone())],
        )
    }
}

// Go: common/types/types.go:NewKeyValuesStrings
/// NewKeyValuesStrings takes a given key and slice of values and returns a new KeyValues.
pub fn new_key_values_strings(key: &str, values: &[&str]) -> KeyValues {
    KeyValues {
        key: Value::string(key),
        values: values.iter().map(|v| Value::string(*v)).collect(),
    }
}

/// Go: `types.KeyValueStr` (a string tuple).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyValueStr {
    pub key: String,
    pub value: String,
}

/// Go: `types.Zeroer` — for template values see `go_value::Object::is_zero`.
pub trait Zeroer {
    fn is_zero(&self) -> bool;
}

// Go: common/types/types.go:IsNil
/// IsNil reports whether v is nil: an untyped nil, or a nil chan/func/interface/map/pointer/slice
/// (every `TypedNil`).
pub fn is_nil(v: &Value) -> bool {
    matches!(v, Value::Invalid | Value::TypedNil(_))
}

// Go: common/types/types.go:Unwrapv
/// Unwrapv returns the underlying value of v if it implements `Unwrapper` (an object with the
/// `Unwrapv` method), otherwise v.
pub fn unwrapv(v: &Value) -> Value {
    if let Value::Object(o) = v
        && o.has_method("Unwrapv")
        && let Some(Ok(u)) = o.call_method(&(), "Unwrapv", &[])
    {
        return u;
    }
    v.clone()
}

// Go: common/types/types.go:NewBool
/// NewBool returns a pointer to b (a boxed bool in Rust).
pub fn new_bool(b: bool) -> Box<bool> {
    Box::new(b)
}

/// Go: `types.EvictingQueue[T]` — a queue that evicts from the head when full, ordered LIFO by
/// `peek_all`, without duplicates.
pub struct EvictingQueue<T> {
    inner: Mutex<EvictingQueueInner<T>>,
}

struct EvictingQueueInner<T> {
    size: usize,
    vals: Vec<T>,
    set: HashSet<T>,
}

impl<T: Eq + Hash + Clone + Default> EvictingQueue<T> {
    // Go: common/types/evictingqueue.go:NewEvictingQueue
    /// NewEvictingQueue creates a new queue with the given size.
    pub fn new(size: usize) -> Self {
        EvictingQueue {
            inner: Mutex::new(EvictingQueueInner {
                size,
                vals: Vec::new(),
                set: HashSet::new(),
            }),
        }
    }

    // Go: common/types/evictingqueue.go:Add
    /// Add adds a new value to the tail of the queue if it's not already there.
    pub fn add(&self, v: T) -> &Self {
        let mut q = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if q.set.contains(&v) {
            return self;
        }
        if q.set.len() == q.size {
            // Full
            let first = q.vals.remove(0);
            q.set.remove(&first);
        }
        q.set.insert(v.clone());
        q.vals.push(v);
        self
    }

    // Go: common/types/evictingqueue.go:Len
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .vals
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Go: common/types/evictingqueue.go:Contains
    /// Contains returns whether the queue contains v.
    pub fn contains(&self, v: &T) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set
            .contains(v)
    }

    // Go: common/types/evictingqueue.go:Peek
    /// Peek looks at the last element added to the queue (the zero value when empty).
    pub fn peek(&self) -> T {
        let q = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        q.vals.last().cloned().unwrap_or_default()
    }

    // Go: common/types/evictingqueue.go:PeekAll
    /// PeekAll looks at all the elements in the queue, with the newest first.
    pub fn peek_all(&self) -> Vec<T> {
        let mut vals = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .vals
            .clone();
        vals.reverse();
        vals
    }

    // Go: common/types/evictingqueue.go:PeekAllSet
    /// PeekAllSet returns PeekAll as a set.
    pub fn peek_all_set(&self) -> HashSet<T> {
        self.peek_all().into_iter().collect()
    }
}

/// Go: `types.Closer` as a boxed `Close()` function.
pub type Closer = Box<dyn FnOnce() -> crate::herrors::Result<()> + Send>;

/// Go: `types.Closers` — `Close()` all in order.
#[derive(Default)]
pub struct Closers {
    pub closers: Mutex<Vec<Closer>>,
}

impl Closers {
    // Go: common/types/closer.go:Add
    pub fn add(&self, c: Closer) {
        self.closers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(c);
    }

    // Go: common/types/closer.go:Close
    /// Calls every closer in order (errors are ignored, as in Go) and empties the list.
    pub fn close(&self) -> crate::herrors::Result<()> {
        let cs: Vec<Closer> =
            std::mem::take(&mut *self.closers.lock().unwrap_or_else(|e| e.into_inner()));
        for c in cs {
            let _ = c();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/types/types.go (145 lines; 2/8 funcs executed)
//   types: RLocker, Locker, RWLocker, KeyValue, KeyValueStr, KeyValues, Zeroer, DevMarker, Unwrapper, LowHigh[S,
//          PrintableValueProvider
// OK L60-62: (k KeyValues) KeyString() string
// OK L64-66: (k KeyValues) String() string
// OK L70-76: NewKeyValuesStrings(key string, values ...string) KeyValues
// OK L85-97: IsNil(v any) bool
// OK L113-118: Unwrapv(v any) any
// OK L126-128: (l LowHigh[S]) IsZero() bool
// OK L130-132: (l LowHigh[S]) Value(source S) S
// OK L138-140: NewBool(b bool) *bool
// Source: common/types/closer.go (54 lines; 1/3 funcs executed)
//   types: Closer, CloserFunc, CloseAdder, Closers
// OK L25-27: (f CloserFunc) Close() error (a boxed closure)
// OK L38-42: (cs *Closers) Add(c Closer)
// OK L44-54: (cs *Closers) Close() error
// Source: common/types/evictingqueue.go (114 lines; 0/7 funcs executed)
//   types: EvictingQueue[T
// OK L34-36: NewEvictingQueue[T comparable](size int) *EvictingQueue[T]
// OK L39-56: (q *EvictingQueue[T]) Add(v T) *EvictingQueue[T]
// OK L58-65: (q *EvictingQueue[T]) Len() int
// OK L68-75: (q *EvictingQueue[T]) Contains(v T) bool
// OK L78-88: (q *EvictingQueue[T]) Peek() T
// OK L91-103: (q *EvictingQueue[T]) PeekAll() []T
// OK L106-114: (q *EvictingQueue[T]) PeekAllSet() map[T]bool
// ---------------------------------------------------------------------------
