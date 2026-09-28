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

/// Go: `processingStatsTitleVal`.
struct ProcessingStatsTitleVal {
    name: &'static str,
    val: u64,
}

impl ProcessingStats {
    // Go: helpers/processing_stats.go:toVals
    fn to_vals(&self) -> Vec<ProcessingStatsTitleVal> {
        let v = |name, c: &AtomicU64| ProcessingStatsTitleVal {
            name,
            val: c.load(Ordering::Relaxed),
        };
        vec![
            v("Pages", &self.pages),
            v("Paginator pages", &self.paginator_pages),
            v("Non-page files", &self.files),
            v("Static files", &self.static_),
            v("Processed images", &self.processed_images),
            v("Aliases", &self.aliases),
            v("Cleaned", &self.cleaned),
        ]
    }

    /// Go: `NewProcessingStats(name)`.
    // Go: helpers/processing_stats.go:NewProcessingStats
    pub fn new(name: &str) -> ProcessingStats {
        ProcessingStats {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Go: `Incr(counter)`.
    // Go: helpers/processing_stats.go:Incr
    pub fn incr(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Go: `Add(counter, amount)` — `atomic.AddUint64(counter, uint64(amount))` (a negative
    /// amount wraps like Go).
    // Go: helpers/processing_stats.go:Add
    pub fn add(counter: &AtomicU64, amount: i64) {
        counter.fetch_add(amount as u64, Ordering::Relaxed);
    }
}

/// Go: `ProcessingStatsTable(w, stats...)` — the summary table printed after a build
/// (olekukonko/tablewriter v1.0.8, see [`crate::tablewriter`]).
// Go: helpers/processing_stats.go:ProcessingStatsTable
pub fn processing_stats_table(w: &mut dyn std::io::Write, stats: &[&ProcessingStats]) {
    let mut names = vec![String::new(); stats.len() + 1];

    let mut data: Vec<Vec<String>> = Vec::new();

    for (i, stat) in stats.iter().enumerate() {
        names[i + 1] = stat.name.clone();

        let title_vals = stat.to_vals();

        if i == 0 {
            data = vec![Vec::new(); title_vals.len()];
        }

        for (j, tv) in title_vals.iter().enumerate() {
            // strconv.Itoa(int(tv.val)): a uint64 above MaxInt64 wraps negative.
            let v = go_strconv::itoa(tv.val as i64);
            if i == 0 {
                data[j] = vec![tv.name.to_string(), v];
            } else {
                data[j].push(v);
            }
        }
    }

    crate::tablewriter::render_processing_stats(w, &names, &data);
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/processing_stats.go (116 lines; 4/5 funcs executed)
//   types: ProcessingStats, processingStatsTitleVal
// OK L44-54: (s *ProcessingStats) toVals() []processingStatsTitleVal
// OK L57-59: NewProcessingStats(name string) *ProcessingStats
// OK L62-64: (s *ProcessingStats) Incr(counter *uint64)
// OK L67-69: (s *ProcessingStats) Add(counter *uint64, amount int)
// OK L72-116: ProcessingStatsTable(w io.Writer, stats ...*ProcessingStats)
// ---------------------------------------------------------------------------
