//! Port of `hugolib/hugo_sites.go` (`.Site.Data`: `Data`, `loadData`, `handleDataFile`, `readData`).
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Split from `hugo_sites.go` (construction is T20's): `h.Data()` runs `loadData` once (Go
//! `h.init.data`), which walks the data component (BaseFs.Data, walk order), decodes each file
//! with the metadecoders (JSON/YAML/TOML/CSV; Go types preserved: int vs float64 vs string,
//! `[]interface {}`, `map[string]interface {}`), and nests the results by directory with Go's
//! merge rules for key collisions (`handleDataFile`). The comments partial reads this structure.
//! Oracle: tools/go-oracle/nh-hugolib/data (T23).

use std::sync::Arc;

use go_value::Map;
use nh_common::Result;

use crate::hugo_sites::HugoSites;

impl HugoSites {
    /// Go: `h.Data()` (lazy `loadData`; the result is cached in `self.data`).
    // Go: hugolib/hugo_sites.go:Data
    pub fn data(&self) -> Arc<Map> {
        todo!()
    }

    /// Go: `loadData()`.
    // Go: hugolib/hugo_sites.go:loadData
    pub fn load_data(&self) -> Result<Arc<Map>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites.go (data part; the rest is in hugo_sites.rs)
// EX L190-196: (h *HugoSites) Data() map[string]any
// EX L498-521: (h *HugoSites) loadData() error
// EX L523-599: (h *HugoSites) handleDataFile(r *source.File) error
//    L601-604: (h *HugoSites) errWithFileContext(err error, f *source.File) error
// EX L606-616: (h *HugoSites) readData(f *source.File) (any, error)
// ---------------------------------------------------------------------------
