//! Views and render state (REWRITE_PLAN.md §2.5): the render scope carried as `__nh`, the
//! [`ContentRenderer`] callback, the pre-serialised page and site values of the [`ViewCache`],
//! and the pagination recorder.
//!
//! **State: T38 walking skeleton.** Frozen here: [`RenderScope`] (with [`Phase`],
//! [`HookVariant`], [`Stage`], [`SCOPE_KEY`]) and the [`ContentRenderer`] trait. The views and
//! the cache are a subset built from the throwaway [`interim`] model; T33 replaces both.

#![forbid(unsafe_code)]

mod cache;
mod content;
pub mod interim;
mod pagination;
mod scope;
pub mod views;

pub use cache::{Contents, ViewCache, ViewGeneration};
pub use content::{
    ContentError, ContentRenderer, ExpandedSource, RenderStringOptions, RenderedContent,
};
pub use pagination::{PaginationRecorder, Recorded};
pub use scope::{HookVariant, MAX_DEPTH, Phase, RenderScope, SCOPE_KEY, Stage};
