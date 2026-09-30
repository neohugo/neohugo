//! The pagination recorder (REWRITE_PLAN.md §3.3): the first `paginator()` / `paginate()` call
//! per (page, format) records the pagination; wave 2 renders pagers 2..N from the records.
//!
//! **Skeleton (T38).** Records the first call only; the conflict check (a different list or
//! size, an error naming both template positions) and nav's `Pagination` type are T35's and
//! T24's.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use neohugo_base::{FormatId, PageId};

/// A recorded pagination: the paged list and the pager size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    pub items: Arc<[PageId]>,
    pub size: usize,
}

impl Recorded {
    /// The number of pagers (at least 1, also for an empty list).
    #[must_use]
    pub fn total_pages(&self) -> u32 {
        let n = self.items.len().div_ceil(self.size.max(1)).max(1);
        u32::try_from(n).unwrap_or(u32::MAX)
    }

    /// The items of pager `number` (1-based).
    #[must_use]
    pub fn page(&self, number: u32) -> &[PageId] {
        let size = self.size.max(1);
        let start = usize::try_from(number.saturating_sub(1))
            .unwrap_or(usize::MAX)
            .saturating_mul(size)
            .min(self.items.len());
        let end = start.saturating_add(size).min(self.items.len());
        &self.items[start..end]
    }
}

/// The paginations recorded during wave 1, per (page, format).
#[derive(Debug, Default)]
pub struct PaginationRecorder {
    recorded: Mutex<BTreeMap<(PageId, FormatId), Arc<Recorded>>>,
}

impl PaginationRecorder {
    /// The recorded pagination of (`page`, `format`), recording `make()` on the first call.
    pub fn record(
        &self,
        page: PageId,
        format: FormatId,
        make: impl FnOnce() -> Recorded,
    ) -> Arc<Recorded> {
        let mut m = self.recorded.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(m.entry((page, format)).or_insert_with(|| Arc::new(make())))
    }

    /// Every recorded pagination, sorted by (page, format).
    #[must_use]
    pub fn recorded(&self) -> Vec<((PageId, FormatId), Arc<Recorded>)> {
        let m = self.recorded.lock().unwrap_or_else(PoisonError::into_inner);
        m.iter().map(|(k, v)| (*k, Arc::clone(v))).collect()
    }
}
