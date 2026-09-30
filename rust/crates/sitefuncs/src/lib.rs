//! Site-bound Tera functions as small handle structs (`get_page`, `ref`, `i18n`, assets, images,
//! `paginate`, `partial`, …).
//!
//! **State: signature stub (T33).** [`Handles`] and [`register`] are frozen here
//! (REWRITE_PLAN.md §2.6) so that the render session (T34) builds against them; T35 implements
//! the functions. Each function struct clones only the `Arc`s it needs.

#![forbid(unsafe_code)]

use std::sync::{Arc, OnceLock, Weak};

use dashmap::DashMap;
use neohugo_base::FrameId;
use neohugo_base::diag::Diagnostics;
use neohugo_images::ImageQueue;
use neohugo_locale::Translations;
use neohugo_nav::{Menus, RelatedIndex};
use neohugo_resources::ResourceStore;
use neohugo_site::Model;
use neohugo_view::{ContentRenderer, DeferredRegistry, PageStores, PaginationRecorder, ViewCache};

/// A cached `partial_cached` result: the rendered text, or the partial's `return_value`.
#[derive(Clone, Debug, PartialEq)]
pub enum PartialResult {
    Text(String),
    Value(tera::Value),
}

/// Everything site functions need; each function struct clones only its own `Arc`s.
pub struct Handles {
    pub model: Arc<Model>,
    pub views: Arc<ViewCache>,
    pub store: Arc<ResourceStore>,
    pub images: Arc<ImageQueue>,
    pub stores: Arc<PageStores>,
    pub pagination: Arc<PaginationRecorder>,
    pub deferred: Arc<DeferredRegistry>,
    pub menus: Arc<Menus>,
    pub related: Arc<RelatedIndex>,
    pub i18n: Arc<Translations>,
    pub diagnostics: Arc<Diagnostics>,
    /// Set by the session once it exists (`Arc::downgrade`); content-dependent functions
    /// upgrade it per call.
    pub renderer: Arc<OnceLock<Weak<dyn ContentRenderer>>>,
    /// `return_value` frames of `partial()` calls.
    pub frames: Arc<DashMap<FrameId, tera::Value>>,
    /// `partial_cached` results by (name, variant key).
    pub partial_cache: Arc<DashMap<(String, String), PartialResult>>,
}

/// Registers every site-bound function, filter and test of `neohugo_funcs::spec::FUNCS` on
/// `t`. **Stub:** registers nothing yet (T35).
pub fn register(t: &mut tera::Tera, h: &Handles) {
    let _ = (t, h);
}
