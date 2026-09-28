//! Port of `hugolib/codeowners.go`.
//!
//! STUB
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go reads a `CODEOWNERS` file (`hairyhenderson/go-codeowners`) when `enableGitInfo` is set;
//! like Git info it is out of scope: `.CodeOwners` is Go's nil `[]string` and parsing a file is
//! the explicit unsupported error.

use nh_common::Result;
use nh_common::herrors::Error;

/// The locations Go looks in, in order (`findCodeOwnersFile`).
pub const CODEOWNERS_DIRS: [&str; 4] = [".", "docs", ".github", ".gitlab"];

/// Go: `codeownerInfo` (never constructed).
pub struct CodeownerInfo;

impl CodeownerInfo {
    /// Go: `forPage(p)` — nil.
    // Go: hugolib/codeowners.go:forPage
    pub fn for_page(&self, _filename: &str) -> Option<Vec<String>> {
        None
    }
}

/// Go: `findCodeOwnersFile(dir)` — the first `CODEOWNERS` below `dir` in [`CODEOWNERS_DIRS`].
// Go: hugolib/codeowners.go:findCodeOwnersFile
pub fn find_code_owners_file(dir: &str) -> Result<Option<String>> {
    for p in CODEOWNERS_DIRS {
        let f = go_path::path::join(&[dir, p, "CODEOWNERS"]);
        match std::fs::metadata(&f) {
            Ok(_) => return Ok(Some(f)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(Error::new(e.to_string())),
        }
    }
    Ok(None)
}

/// Go: `newCodeOwners(workingDir)` — no file is `nil, nil`; parsing one is not supported.
// Go: hugolib/codeowners.go:newCodeOwners
pub fn new_code_owners(working_dir: &str) -> Result<Option<CodeownerInfo>> {
    match find_code_owners_file(working_dir)? {
        None => Ok(None),
        Some(_) => Err(Error::new("neohugo-rs: CODEOWNERS is not supported")),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/codeowners.go (67 lines; 0/3 funcs executed)
//   types: codeownerInfo
// OK L29-45: findCodeOwnersFile(dir string) (io.Reader, error)
// OK L51-53: (c *codeownerInfo) forPage(p page.Page) []string  [STUB: nil]
// OK L55-67: newCodeOwners(workingDir string) (*codeownerInfo, error)  [STUB: explicit error when a file exists]
// ---------------------------------------------------------------------------
