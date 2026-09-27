//! Port of `hugolib/doctree/support.go`.
//!
//! Owner: Wave B task T27 (doctree).


use std::collections::BTreeMap;

use nh_common::Result;

/// Go: `doctree.Event[T]` (e.g. "dates" events propagated to ancestor branches during
/// `applyAggregates`; queued and handled after the walk).
#[derive(Clone, Debug)]
pub struct Event<T> {
    pub name: String,
    pub path: String,
    pub source: T,
    pub(crate) stop_propagation: bool,
}

impl<T> Event<T> {
    pub fn stop_propagation(&mut self) {
        self.stop_propagation = true;
    }
}

/// Go: `doctree.WalkContext[T]`.
pub struct WalkContext<T> {
    /// Go `Data()` — a scratch tree shared by the walk.
    pub data: BTreeMap<String, go_value::Value>,
    pub(crate) event_handlers: BTreeMap<String, Vec<(String, Box<dyn FnMut(&mut Event<T>) + Send>)>>,
    pub(crate) events: Vec<Event<T>>,
    pub(crate) hooks_post: Vec<Box<dyn FnOnce() -> Result<()> + Send>>,
}

impl<T: Clone> WalkContext<T> {
    pub fn new() -> Self {
        WalkContext { data: BTreeMap::new(), event_handlers: BTreeMap::new(), events: Vec::new(), hooks_post: Vec::new() }
    }

    /// Go: `AddEventListener(event, path, handler)`.
    // Go: hugolib/doctree/support.go:AddEventListener
    pub fn add_event_listener(&mut self, event: &str, path: &str, handler: Box<dyn FnMut(&mut Event<T>) + Send>) {
        todo!()
    }

    // Go: hugolib/doctree/support.go:SendEvent
    pub fn send_event(&mut self, event: Event<T>) {
        self.events.push(event);
    }

    /// Go: `HandleEvents()` — events dispatched to listeners registered on ancestor paths
    /// (handler order: Go's registration order; see support.go).
    // Go: hugolib/doctree/support.go:HandleEvents
    pub fn handle_events(&mut self) -> Result<()> {
        todo!()
    }

    // Go: hugolib/doctree/support.go:HandleEventsAndHooks
    pub fn handle_events_and_hooks(&mut self) -> Result<()> {
        todo!()
    }
}

/// Go: `doctree.ValidateKey(key)` (keys start with "/" except "", no trailing "/").
// Go: hugolib/doctree/support.go:ValidateKey
pub fn validate_key(key: &str) -> Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/support.go (247 lines; 8/17 funcs executed)
//   types: Event[T, LockType, MutableTree, WalkableTree[T, WalkableTrees[T, MutableTrees, WalkContext[T,
//          eventHandlers[T
// EX L32-53: (ctx *WalkContext[T]) AddEventListener(event, path string, handler func(*Event[T]))
//    L57-59: (ctx *WalkContext[T]) AddPostHook(handler func() error)
// EX L61-66: (ctx *WalkContext[T]) Data() *SimpleThreadSafeTree[any]
// EX L69-71: (ctx *WalkContext[T]) SendEvent(event *Event[T])
//    L74-76: (e *Event[T]) StopPropagation()
// EX L79-98: ValidateKey(key string) error
//    L129-133: (t WalkableTrees[T]) WalkPrefixRaw(prefix string, walker func(key string, value T) bool)
//    L139-143: (t MutableTrees) DeleteRaw(key string)
//    L145-149: (t MutableTrees) DeleteAll(key string)
//    L151-157: (t MutableTrees) DeletePrefix(prefix string) int
//    L159-165: (t MutableTrees) DeletePrefixAll(prefix string) int
//    L167-177: (t MutableTrees) Lock(writable bool) (commit func())
//    L179-186: (t MutableTrees) CanLock() bool
// EX L200-209: cleanKey(key string) string
// EX L211-227: (ctx *WalkContext[T]) HandleEvents() error
// EX L229-240: (ctx *WalkContext[T]) HandleEventsAndHooks() error
// EX L242-247: mustValidateKey(key string) string
// ---------------------------------------------------------------------------
