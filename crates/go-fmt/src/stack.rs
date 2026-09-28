//! Stack depth guard for `printValue`'s recursion (a copy of go-json's
//! `stack` module).
//!
//! Go goroutine stacks grow on demand (up to 1 GB), so Go's `printValue`
//! prints values nested 100000 levels deep. Rust threads have fixed stacks
//! (2 MiB for spawned threads by default), and each nesting level costs
//! several frames here. [`guard`] runs the recursion on the current thread
//! until it has used about [`LIMIT`] bytes of stack since the outermost
//! guarded call on this thread, then continues it on a fresh scoped thread
//! with a [`THREAD_STACK`]-byte stack. Results and panics are passed back
//! unchanged, so this cannot affect output.

use std::cell::Cell;

/// Stack bytes a thread may use below the first guarded frame before the
/// recursion moves to a new thread.
const LIMIT: usize = 256 << 10;
/// Stack size of the helper threads.
const THREAD_STACK: usize = 8 << 20;

thread_local! {
    /// Approximate stack address of the outermost guarded frame on this
    /// thread (0: none active).
    static BASE: Cell<usize> = const { Cell::new(0) };
}

/// Restores `BASE` when the outermost guarded call returns or unwinds.
struct Reset;

impl Drop for Reset {
    fn drop(&mut self) {
        BASE.with(|b| b.set(0));
    }
}

#[inline(never)]
fn stack_addr() -> usize {
    let marker = 0u8;
    std::hint::black_box(&marker) as *const u8 as usize
}

/// Runs `f` on this thread, or on a new thread if this thread's guarded
/// recursion is already deep.
pub(crate) fn guard<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    let here = stack_addr();
    let base = BASE.with(|b| b.get());
    if base == 0 {
        BASE.with(|b| b.set(here));
        let _reset = Reset;
        return f();
    }
    if base.abs_diff(here) < LIMIT {
        return f();
    }
    std::thread::scope(|s| {
        let h = std::thread::Builder::new()
            .stack_size(THREAD_STACK)
            .spawn_scoped(s, f)
            .expect("spawn stack helper thread");
        match h.join() {
            Ok(v) => v,
            Err(p) => std::panic::resume_unwind(p),
        }
    })
}
