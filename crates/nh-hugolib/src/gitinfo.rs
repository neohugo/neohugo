//! Port of `hugolib/gitinfo.go`.
//!
//! STUB (enableGitInfo=false)
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `gitInfo` maps content files to their last Git commit (`bep/gitmap` running `git log`).
//! Git info is out of scope (seeksnack does not set `enableGitInfo`): construction fails with
//! the explicit unsupported error (T20's `HugoSites::git_info_for_page` reports it when the
//! config enables it), and `.GitInfo` is Go's nil `*source.GitInfo` (tplapi/page_methods.rs).

use nh_common::Result;
use nh_common::herrors::Error;

/// Go: `gitInfo` (never constructed: see the module docs).
pub struct GitInfo {
    pub content_dir: String,
}

impl GitInfo {
    /// Go: `forPage(p)` — no Git data (nil).
    // Go: hugolib/gitinfo.go:forPage
    pub fn for_page(&self, _filename: &str) -> Option<()> {
        None
    }
}

/// Go: `newGitInfo(d)` — the explicit unsupported error.
// Go: hugolib/gitinfo.go:newGitInfo
pub fn new_git_info() -> Result<GitInfo> {
    Err(Error::new("neohugo-rs: enableGitInfo is not supported"))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/gitinfo.go (66 lines; 0/2 funcs executed)
//   types: gitInfo
// OK L33-41: (g *gitInfo) forPage(p page.Page) *source.GitInfo  [STUB: nil]
// OK L43-66: newGitInfo(d *deps.Deps) (*gitInfo, error)  [STUB: explicit unsupported error]
// ---------------------------------------------------------------------------
