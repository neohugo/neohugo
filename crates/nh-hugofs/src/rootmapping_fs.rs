//! Port of `hugofs/rootmapping_fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


//! Go `hugofs.RootMappingFs`: a virtual fs built from module mounts (`From` = component target
//! path, `To` = real source dir). Earlier mounts get higher `Weight` and win.

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::Result;

use crate::afero::Fs;
use crate::fileinfo::{FileMeta, FileMetaInfo};

/// Go: `hugofs.RootMapping`.
#[derive(Clone, Default)]
pub struct RootMapping {
    /// The virtual mount point, e.g. `content`, `assets/vendor`.
    pub from: String,
    /// `from` without its component prefix.
    pub from_base: String,
    /// Real source dir (absolute), e.g. `<site>/node_modules`.
    pub to: String,
    /// The base of `to`, used for `ReverseLookup`.
    pub to_base: String,
    pub module: String,
    pub module_ordinal: i64,
    pub is_project: bool,
    pub meta: Arc<FileMeta>,
}

/// Go: `hugofs.ComponentPath`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComponentPath {
    pub component: String,
    pub path: String,
    pub lang: String,
    pub watch: bool,
}

/// Go: `hugofs.RootMappingFs`. Go keys two radix trees; a `BTreeMap` walk has the same order.
pub struct RootMappingFs {
    pub id: String,
    pub fs: Arc<dyn Fs>,
    pub(crate) root_map_to_real: BTreeMap<String, Vec<RootMapping>>,
    pub(crate) real_map_to_root: BTreeMap<String, Vec<RootMapping>>,
}

impl RootMappingFs {
    /// Go: `hugofs.NewRootMappingFs(fs, rms...)`.
    // Go: hugofs/rootmapping_fs.go:NewRootMappingFs
    pub fn new(fs: Arc<dyn Fs>, rms: Vec<RootMapping>) -> Result<Arc<RootMappingFs>> {
        todo!()
    }

    /// Go: `Mounts(base)` — the mounts at a virtual path.
    // Go: hugofs/rootmapping_fs.go:Mounts
    pub fn mounts(&self, base: &str) -> Result<Vec<FileMetaInfo>> {
        todo!()
    }

    /// Go: `ReverseLookupComponent(component, filename)` — real filename -> component paths
    /// (e.g. `<site>/node_modules/bootstrap/scss` -> `vendor/bootstrap/scss` in assets).
    // Go: hugofs/rootmapping_fs.go:ReverseLookupComponent
    pub fn reverse_lookup_component(&self, component: &str, filename: &str) -> Result<Vec<ComponentPath>> {
        todo!()
    }

    /// Go: `ReverseLookup(filename)`.
    pub fn reverse_lookup(&self, filename: &str) -> Result<Vec<ComponentPath>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/rootmapping_fs.go (854 lines; 26/35 funcs executed)
//   types: RootMapping, keyRootMappings, RootMappingFs, ComponentPath, ReverseLookupProvder, rootMappingDir
// EX L44-168: NewRootMappingFs(fs afero.Fs, rms ...RootMapping) (*RootMappingFs, error)
//    L170-185: newRootMappingFsFromFromTo( baseDir string, fs afero.Fs, fromTo ...string, ) (*RootMappingFs, error)
// EX L208-211: (rm *RootMapping) clean()
// EX L213-218: (r RootMapping) filename(name string) string
// EX L220-225: (r RootMapping) trimFrom(name string) string
// EX L241-270: (fs *RootMappingFs) Mounts(base string) ([]FileMetaInfo, error)
//    L272-274: (fs *RootMappingFs) Key() string
// EX L276-278: (fs *RootMappingFs) UnwrapFilesystem() afero.Fs
//    L281-300: (fs RootMappingFs) Filter(f func(m RootMapping) bool) *RootMappingFs
// EX L303-310: (fs *RootMappingFs) Open(name string) (afero.File, error)
// EX L315-321: (fs *RootMappingFs) Stat(name string) (os.FileInfo, error)
//    L330-332: (c ComponentPath) ComponentPathJoined() string
//    L340-342: (fs *RootMappingFs) ReverseLookup(filename string) ([]ComponentPath, error)
// EX L344-386: (fs *RootMappingFs) ReverseLookupComponent(component, filename string) ([]ComponentPath, error)
// EX L388-396: (fs *RootMappingFs) hasPrefix(prefix string) bool
// EX L398-405: (fs *RootMappingFs) getRoot(key string) []RootMapping
// EX L407-440: (fs *RootMappingFs) getRoots(key string) (string, []RootMapping)
// EX L442-449: (fs *RootMappingFs) getRootsReverse(key string) (string, []RootMapping)
// EX L451-459: (fs *RootMappingFs) getRootsWithPrefix(prefix string) []RootMapping
// EX L461-474: (fs *RootMappingFs) getAncestors(prefix string) []keyRootMappings
// EX L476-525: (fs *RootMappingFs) newUnionFile(fis ...FileMetaInfo) (afero.File, error)
// EX L527-533: (fs *RootMappingFs) cleanName(name string) string
// EX L535-654: (rfs *RootMappingFs) collectDirEntries(prefix string) ([]iofs.DirEntry, error)
// EX L656-676: (fs *RootMappingFs) doStat(name string) ([]FileMetaInfo, error)
// EX L678-720: (fs *RootMappingFs) doDoStat(name string) ([]FileMetaInfo, error)
// EX L722-769: (fs *RootMappingFs) statRoot(root RootMapping, filename string) (FileMetaInfo, error)
// EX L771-773: (fs *RootMappingFs) virtualDirOpener(name string) func() (afero.File, error)
// EX L775-783: (fs *RootMappingFs) realDirOpener(name string, meta *FileMeta) func() (afero.File, error)
// EX L795-800: (f *rootMappingDir) Close() error
//    L802-804: (f *rootMappingDir) Name() string
// EX L806-825: (f *rootMappingDir) ReadDir(count int) ([]iofs.DirEntry, error)
// EX L830-832: (f *rootMappingDir) Stat() (iofs.FileInfo, error)
//    L834-836: (f *rootMappingDir) Readdir(count int) ([]os.FileInfo, error)
//    L840-846: (f *rootMappingDir) Readdirnames(count int) ([]string, error)
//    L848-854: dirEntriesToNames(fis []iofs.DirEntry) []string
// ---------------------------------------------------------------------------
