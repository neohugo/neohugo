//! Port of `hugofs/glob.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

use std::sync::Arc;

use nh_common::Result;
use nh_common::glob::glob as hglob;
use nh_common::herrors::Error;

use crate::afero::Fs;
use crate::fileinfo::FileMetaInfo;
use crate::walk::{Walkway, WalkwayConfig, skip_dir};

/// Go: `hugofs.Glob(fs, pattern, handle)` — walks the fs and passes all matches to the handle
/// func; the handle func can return true to signal a stop (`resources.Match`,
/// `.Resources.Match`).
// Go: hugofs/glob.go:Glob
pub fn glob(
    fs: Arc<dyn Fs>,
    pattern: &str,
    handle: &mut dyn FnMut(&FileMetaInfo) -> Result<bool>,
) -> Result<()> {
    let pattern = hglob::normalize_path_no_lower(pattern);
    if pattern.is_empty() {
        return Ok(());
    }
    let mut root = hglob::resolve_root_dir(&pattern);
    if !root.starts_with('/') {
        root = format!("/{root}");
    }
    let pattern = go_unicode::strings::to_lower_str(&pattern).into_owned();

    let g = hglob::get_glob(&pattern)?;

    let has_super_asterisk = pattern.contains("**");
    let levels = pattern.matches('/').count();

    // Signals that we're done.
    const DONE: &str = "done";
    let mut done = false;

    let mut wfn = |p: &str, info: &FileMetaInfo| -> Result<()> {
        let p = hglob::normalize_path(p);
        if info.is_dir() {
            if !has_super_asterisk {
                // Avoid walking to the bottom if we can avoid it.
                if !p.is_empty() && p.matches('/').count() >= levels {
                    return Err(skip_dir());
                }
            }
            return Ok(());
        }

        if g.matches(&p) {
            let d = handle(info)?;
            if d {
                done = true;
                return Err(Error::new(DONE));
            }
        }

        Ok(())
    };

    let mut cfg = WalkwayConfig::new(fs, &mut wfn);
    cfg.root = root;
    cfg.fail_on_not_exist = true;
    let mut w = Walkway::new(cfg);

    let err = w.walk();
    drop(w);

    match err {
        Err(e) if !(done && e.message() == DONE) => Err(e),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/glob.go (90 lines; 0/1 funcs executed)
// OK L28-90: Glob(fs afero.Fs, pattern string, handle func(fi FileMetaInfo) (bool, error)) error
// ---------------------------------------------------------------------------
