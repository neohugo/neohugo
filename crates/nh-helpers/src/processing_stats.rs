//! Port of `helpers/processing_stats.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


use std::sync::atomic::{AtomicU64, Ordering};

/// Go: `helpers.ProcessingStats` (the build summary table; stdout only, not parity).
#[derive(Default, Debug)]
pub struct ProcessingStats {
    pub name: String,
    pub pages: AtomicU64,
    pub paginator_pages: AtomicU64,
    pub static_: AtomicU64,
    pub processed_images: AtomicU64,
    pub files: AtomicU64,
    pub aliases: AtomicU64,
    pub cleaned: AtomicU64,
}

impl ProcessingStats {
    pub fn incr(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/processing_stats.go (116 lines; 4/5 funcs executed)
//   types: ProcessingStats, processingStatsTitleVal
// EX L44-54: (s *ProcessingStats) toVals() []processingStatsTitleVal
// EX L57-59: NewProcessingStats(name string) *ProcessingStats
// EX L62-64: (s *ProcessingStats) Incr(counter *uint64)
//    L67-69: (s *ProcessingStats) Add(counter *uint64, amount int)
// EX L72-116: ProcessingStatsTable(w io.Writer, stats ...*ProcessingStats)
// ---------------------------------------------------------------------------
