//! Port of `hugolib/doctree/support.go`.
//!
//! Owner: Wave B task T27 (doctree).

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use nh_common::{Error, Result};

use crate::dimensions::Dimension;
use crate::simpletree::SimpleTree;

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
    /// Go: `&doctree.Event[T]{Source: source, Path: path, Name: name}`.
    pub fn new(name: impl Into<String>, path: impl Into<String>, source: T) -> Self {
        Event {
            name: name.into(),
            path: path.into(),
            source,
            stop_propagation: false,
        }
    }

    /// Go: `StopPropagation()` — the handlers registered before the current one (further up the
    /// tree) do not see this event.
    // Go: hugolib/doctree/support.go:StopPropagation
    pub fn stop_propagation(&mut self) {
        self.stop_propagation = true;
    }

    /// Whether [`Event::stop_propagation`] was called.
    pub fn is_propagation_stopped(&self) -> bool {
        self.stop_propagation
    }
}

/// An event listener (Go `func(*Event[T])`).
pub type EventHandler<T> = Box<dyn FnMut(&mut Event<T>)>;

/// A post-walk hook (Go `func() error`).
pub type PostHook = Box<dyn FnMut() -> Result<()>>;

/// The event queue of a [`WalkContext`]. Go's handlers close over the context and call
/// `ctx.SendEvent` while `HandleEvents` runs; a Rust handler captures a clone of this sender
/// instead (from [`WalkContext::event_sender`]). The queue is locked only to push or pop one
/// event, never while a handler runs.
pub struct EventSender<T>(Arc<Mutex<VecDeque<Event<T>>>>);

impl<T> Clone for EventSender<T> {
    fn clone(&self) -> Self {
        EventSender(self.0.clone())
    }
}

impl<T> Default for EventSender<T> {
    fn default() -> Self {
        EventSender(Arc::new(Mutex::new(VecDeque::new())))
    }
}

impl<T> EventSender<T> {
    fn queue(&self) -> MutexGuard<'_, VecDeque<Event<T>>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Go: `ctx.SendEvent(event)` — appends the event to the queue.
    pub fn send(&self, event: Event<T>) {
        self.queue().push_back(event);
    }

    fn pop(&self) -> Option<Event<T>> {
        self.queue().pop_front()
    }

    /// The number of queued events.
    pub fn len(&self) -> usize {
        self.queue().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue().is_empty()
    }
}

/// Go: `doctree.WalkContext[T]` — passed along the walks of a tree: `data` passes values down
/// (cascades), events pass values up (dates), post hooks run after the walks.
///
/// `D` is the value type of the data tree (Go: `any`).
pub struct WalkContext<T, D = go_value::Value> {
    /// Go `Data()` — a tree shared by the walks of this context.
    pub data: SimpleTree<D>,
    /// Go `eventHandlers`: per event name, the listeners in registration order, each with the
    /// path prefix (the listener's path + "/") the event path must have.
    pub(crate) event_handlers: BTreeMap<String, Vec<(String, EventHandler<T>)>>,
    pub(crate) events: EventSender<T>,
    pub(crate) hooks_post: Vec<PostHook>,
}

impl<T, D> Default for WalkContext<T, D> {
    fn default() -> Self {
        WalkContext {
            data: SimpleTree::default(),
            event_handlers: BTreeMap::new(),
            events: EventSender::default(),
            hooks_post: Vec::new(),
        }
    }
}

impl<T> WalkContext<T> {
    /// Go: `&doctree.WalkContext[T]{}` (data values are `go_value::Value`; use
    /// `WalkContext::<T, D>::default()` for another data type).
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T, D> WalkContext<T, D> {
    /// Go: `AddEventListener(event, path, handler)` — `handler` sees the `event` events whose path
    /// starts with `path` + "/" (a listener on the home page, path "", sees every event but the
    /// home page's own). Note that the handler may not add listeners.
    // Go: hugolib/doctree/support.go:AddEventListener
    pub fn add_event_listener(&mut self, event: &str, path: &str, handler: EventHandler<T>) {
        // We want to match all above the path, so we need to exclude any similar named siblings.
        let mut path = path.to_string();
        if !path.ends_with('/') {
            path.push('/');
        }
        self.event_handlers
            .entry(event.to_string())
            .or_default()
            .push((path, handler));
    }

    /// Go: `AddPostHook(handler)` — run after the tree has been walked, by
    /// [`WalkContext::handle_events_and_hooks`].
    // Go: hugolib/doctree/support.go:AddPostHook
    pub fn add_post_hook(&mut self, handler: PostHook) {
        self.hooks_post.push(handler);
    }

    /// Go: `Data()`.
    // Go: hugolib/doctree/support.go:Data
    pub fn data(&mut self) -> &mut SimpleTree<D> {
        &mut self.data
    }

    /// Go: `SendEvent(event)` — sends an event up the tree.
    // Go: hugolib/doctree/support.go:SendEvent
    pub fn send_event(&mut self, event: Event<T>) {
        self.events.send(event);
    }

    /// A handle for sending events from an event handler (Go's handlers call `ctx.SendEvent`).
    pub fn event_sender(&self) -> EventSender<T> {
        self.events.clone()
    }

    /// Go: `HandleEvents()` — dispatches the queued events (including those sent by the handlers
    /// themselves) to their listeners, the last registered first, until one stops propagation.
    // Go: hugolib/doctree/support.go:HandleEvents
    pub fn handle_events(&mut self) -> Result<()> {
        while let Some(mut event) = self.events.pop() {
            // Loop the event handlers in reverse order so that events created by the handlers
            // themselves will be picked up further up the tree.
            if let Some(handlers) = self.event_handlers.get_mut(&event.name) {
                for (path, handler) in handlers.iter_mut().rev() {
                    // Propagate events up the tree only.
                    if event.path.starts_with(path.as_str()) {
                        handler(&mut event);
                    }
                    if event.stop_propagation {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// Go: `HandleEventsAndHooks()`.
    // Go: hugolib/doctree/support.go:HandleEventsAndHooks
    pub fn handle_events_and_hooks(&mut self) -> Result<()> {
        self.handle_events()?;
        for hook in self.hooks_post.iter_mut() {
            hook()?;
        }
        Ok(())
    }
}

/// Go: `doctree.ValidateKey(key)` (keys start with "/" except "", no trailing "/").
// Go: hugolib/doctree/support.go:ValidateKey
pub fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        // Root node.
        return Ok(());
    }

    if key.len() < 2 {
        return Err(Error::new(format!(
            "too short key: {}",
            go_strconv::quote(key)
        )));
    }

    if !key.starts_with('/') {
        return Err(Error::new(format!(
            "key must start with '/': {}",
            go_strconv::quote(key)
        )));
    }

    if key.ends_with('/') {
        return Err(Error::new(format!(
            "key must not end with '/': {}",
            go_strconv::quote(key)
        )));
    }

    Ok(())
}

/// Go: `cleanKey(key)` — the home page's logical path "/" is stored as "", so that a prefix
/// search for "/" returns the home page's descendants.
// Go: hugolib/doctree/support.go:cleanKey
pub(crate) fn clean_key(key: &str) -> &str {
    if key == "/" {
        return "";
    }
    key
}

/// Go: `mustValidateKey(key)` — panics like Go on an invalid key. Tree keys are built by Hugo
/// (`paths.Path.Base()`), so an invalid key is a programming error, not an input error.
// Go: hugolib/doctree/support.go:mustValidateKey
pub(crate) fn must_validate_key(key: &str) -> &str {
    if let Err(err) = validate_key(key) {
        panic!("{err}");
    }
    key
}

/// Go: `doctree.WalkableTree[T]`.
pub trait WalkableTree<T> {
    /// Go: `WalkPrefixRaw(prefix, walker)` — `walker` returns true to stop.
    fn walk_prefix_raw(&self, prefix: &str, walker: &mut dyn FnMut(&str, &T) -> bool);
}

/// Go: `doctree.WalkableTrees[T]`.
pub struct WalkableTrees<'a, T>(pub Vec<&'a dyn WalkableTree<T>>);

impl<T> WalkableTrees<'_, T> {
    /// Go: `WalkPrefixRaw` over every tree in turn (stopping one tree's walk does not stop the
    /// next one's, as in Go).
    // Go: hugolib/doctree/support.go:WalkPrefixRaw
    pub fn walk_prefix_raw(&self, prefix: &str, walker: &mut dyn FnMut(&str, &T) -> bool) {
        for tree in &self.0 {
            tree.walk_prefix_raw(prefix, walker);
        }
    }
}

/// Go: `doctree.MutableTree`. `dims` is the shape of the tree (Go: the tree's own dimension).
pub trait MutableTree {
    fn delete_raw(&mut self, dims: Dimension, key: &str);
    fn delete_all(&mut self, key: &str);
    fn delete_prefix(&mut self, dims: Dimension, prefix: &str) -> usize;
    fn delete_prefix_all(&mut self, prefix: &str) -> usize;
    /// Go: `Lock(writable) (commit func())`. Rust borrows do the locking; the commit is a no-op.
    fn lock(&self, writable: bool) -> Box<dyn FnOnce()>;
    /// Used for troubleshooting only.
    fn can_lock(&self) -> bool;
}

/// Go: `doctree.MutableTrees`.
pub struct MutableTrees<'a>(pub Vec<&'a mut dyn MutableTree>);

impl MutableTrees<'_> {
    // Go: hugolib/doctree/support.go:DeleteRaw
    pub fn delete_raw(&mut self, dims: Dimension, key: &str) {
        for tree in self.0.iter_mut() {
            tree.delete_raw(dims, key);
        }
    }

    // Go: hugolib/doctree/support.go:DeleteAll
    pub fn delete_all(&mut self, key: &str) {
        for tree in self.0.iter_mut() {
            tree.delete_all(key);
        }
    }

    // Go: hugolib/doctree/support.go:DeletePrefix
    pub fn delete_prefix(&mut self, dims: Dimension, prefix: &str) -> usize {
        let mut count = 0;
        for tree in self.0.iter_mut() {
            count += tree.delete_prefix(dims, prefix);
        }
        count
    }

    // Go: hugolib/doctree/support.go:DeletePrefixAll
    pub fn delete_prefix_all(&mut self, prefix: &str) -> usize {
        let mut count = 0;
        for tree in self.0.iter_mut() {
            count += tree.delete_prefix_all(prefix);
        }
        count
    }

    // Go: hugolib/doctree/support.go:Lock
    pub fn lock(&self, writable: bool) -> impl FnOnce() + use<> {
        let commits: Vec<Box<dyn FnOnce()>> = self.0.iter().map(|t| t.lock(writable)).collect();
        move || {
            for commit in commits {
                commit();
            }
        }
    }

    // Go: hugolib/doctree/support.go:CanLock
    pub fn can_lock(&self) -> bool {
        self.0.iter().all(|t| t.can_lock())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/support.go (247 lines; 8/17 funcs executed)
//   types: Event[T, LockType, MutableTree, WalkableTree[T, WalkableTrees[T, MutableTrees, WalkContext[T,
//          eventHandlers[T
// OK L32-53: (ctx *WalkContext[T]) AddEventListener(event, path string, handler func(*Event[T]))
// OK L57-59: (ctx *WalkContext[T]) AddPostHook(handler func() error)
// OK L61-66: (ctx *WalkContext[T]) Data() *SimpleThreadSafeTree[any]
// OK L69-71: (ctx *WalkContext[T]) SendEvent(event *Event[T])
// OK L74-76: (e *Event[T]) StopPropagation()
// OK L79-98: ValidateKey(key string) error
// OK L129-133: (t WalkableTrees[T]) WalkPrefixRaw(prefix string, walker func(key string, value T) bool)
// OK L139-143: (t MutableTrees) DeleteRaw(key string)
// OK L145-149: (t MutableTrees) DeleteAll(key string)
// OK L151-157: (t MutableTrees) DeletePrefix(prefix string) int
// OK L159-165: (t MutableTrees) DeletePrefixAll(prefix string) int
// OK L167-177: (t MutableTrees) Lock(writable bool) (commit func())
// OK L179-186: (t MutableTrees) CanLock() bool
// OK L200-209: cleanKey(key string) string
// OK L211-227: (ctx *WalkContext[T]) HandleEvents() error
// OK L229-240: (ctx *WalkContext[T]) HandleEventsAndHooks() error
// OK L242-247: mustValidateKey(key string) string
// ---------------------------------------------------------------------------
