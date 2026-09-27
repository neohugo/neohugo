//! Module `overlayfs`.
//!
//! NEW: bep/overlayfs subset (first-wins Stat/Open, merged ReadDir with DirsMerger)
//!
//! Owner: Wave B task T05 (hugofs-vfs).


//! `github.com/bep/overlayfs` subset: an ordered stack of filesystems. `Stat`/`Open` of a file
//! return the first fs that has it; directories are merged across all fss with a
//! [`crate::dirsmerger::DirsMerger`].

use std::sync::Arc;

use crate::afero::Fs;
use crate::dirsmerger::DirsMerger;

/// Go: `overlayfs.Options`.
#[derive(Clone)]
pub struct Options {
    pub fss: Vec<Arc<dyn Fs>>,
    pub first_writable: bool,
    pub dirs_merger: Option<DirsMerger>,
}

/// Go: `overlayfs.OverlayFs`.
#[derive(Clone)]
pub struct OverlayFs {
    pub fss: Vec<Arc<dyn Fs>>,
    pub first_writable: bool,
    pub merge_dirs: DirsMerger,
}

impl OverlayFs {
    /// Go: `overlayfs.New(opts)`.
    pub fn new(opts: Options) -> OverlayFs {
        todo!()
    }

    /// Go: `Append(fss...)` — returns a new overlay with the fss added.
    pub fn append(&self, fss: Vec<Arc<dyn Fs>>) -> OverlayFs {
        todo!()
    }

    pub fn num_filesystems(&self) -> usize {
        self.fss.len()
    }
}
