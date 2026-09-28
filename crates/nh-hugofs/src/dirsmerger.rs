//! Port of `hugofs/dirsmerger.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

use std::sync::Arc;

use crate::fileinfo::FileMetaInfo;

/// Go: `overlayfs.DirsMerger func(lofi, bofi []fs.DirEntry) []fs.DirEntry`.
pub type DirsMerger =
    Arc<dyn Fn(Vec<FileMetaInfo>, Vec<FileMetaInfo>) -> Vec<FileMetaInfo> + Send + Sync>;

/// Go: `hugofs.LanguageDirsMerger` — dirs merge by name; files merge by (name, lang).
// Go: hugofs/dirsmerger.go:LanguageDirsMerger
pub fn language_dirs_merger() -> DirsMerger {
    Arc::new(|mut lofi: Vec<FileMetaInfo>, bofi: Vec<FileMetaInfo>| {
        for fi1 in bofi {
            let mut found = false;
            for fi2 in &lofi {
                if fi1.name() == fi2.name() && fi1.meta().lang == fi2.meta().lang {
                    found = true;
                    break;
                }
            }
            if !found {
                lofi.push(fi1);
            }
        }
        lofi
    })
}

/// Go: `hugofs.AppendDirsMerger` — keeps duplicates (data, i18n).
// Go: hugofs/dirsmerger.go:AppendDirsMerger
pub fn append_dirs_merger() -> DirsMerger {
    Arc::new(|mut lofi: Vec<FileMetaInfo>, bofi: Vec<FileMetaInfo>| {
        for fi1 in bofi {
            let mut found = false;
            // Remove duplicate directories.
            if fi1.is_dir() {
                for fi2 in &lofi {
                    if fi2.is_dir() && fi2.name() == fi1.name() {
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                lofi.push(fi1);
            }
        }
        lofi
    })
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/dirsmerger.go (65 lines; 0/0 funcs executed)
// OK L24-43: LanguageDirsMerger (var)
// OK L48-65: AppendDirsMerger (var)
// ---------------------------------------------------------------------------
