//! Module `overlayfs`.
//!
//! NEW: bep/overlayfs subset (first-wins Stat/Open, merged ReadDir with DirsMerger)
//!
//! Owner: Wave B task T05 (hugofs-vfs).

//! `github.com/bep/overlayfs@v0.10.0` (the whole package): an ordered stack of filesystems.
//! `Stat`/`Open` of a file return the first fs that has it; directories are merged across all
//! fss with a [`crate::dirsmerger::DirsMerger`].

use std::any::Any;
use std::sync::Arc;

use go_value::Time;
use nh_common::Result;

use crate::afero::{File, Fs, flags, no_regular_file_ops};
use crate::dirsmerger::DirsMerger;
use crate::fileinfo::{FileMetaInfo, OpenFunc};
use crate::oserror;

/// Go: `overlayfs.Options`.
#[derive(Clone, Default)]
pub struct Options {
    /// The filesystems to overlay ordered in priority from left to right.
    pub fss: Vec<Arc<dyn Fs>>,
    /// The OverlayFs is by default read-only, but you can nominate the first filesystem to be
    /// writable.
    pub first_writable: bool,
    /// The DirsMerger is used to merge the contents of two directories. If not provided, the
    /// default merger is used.
    pub dirs_merger: Option<DirsMerger>,
}

/// Go: `overlayfs.OverlayFs`.
#[derive(Clone)]
pub struct OverlayFs {
    pub fss: Vec<Arc<dyn Fs>>,
    pub first_writable: bool,
    pub merge_dirs: DirsMerger,
}

// Go: overlayfs.go:defaultDirMerger
/// Go: `defaultDirMerger` — entries of `bofi` whose name is not in `lofi` are appended.
pub fn default_dir_merger() -> DirsMerger {
    Arc::new(|mut lofi: Vec<FileMetaInfo>, bofi: Vec<FileMetaInfo>| {
        for b in bofi {
            let found = lofi.iter().any(|l| b.name() == l.name());
            if !found {
                lofi.push(b);
            }
        }
        lofi
    })
}

impl OverlayFs {
    /// Go: `overlayfs.New(opts)`.
    // Go: overlayfs.go:New
    pub fn new(opts: Options) -> OverlayFs {
        OverlayFs {
            fss: opts.fss,
            merge_dirs: opts.dirs_merger.unwrap_or_else(default_dir_merger),
            first_writable: opts.first_writable,
        }
    }

    /// Go: `Append(fss...)` — returns a new overlay with the fss added.
    // Go: overlayfs.go:Append
    pub fn append(&self, fss: Vec<Arc<dyn Fs>>) -> OverlayFs {
        let mut ofs = self.clone();
        ofs.fss.extend(fss);
        ofs
    }

    /// Go: `WithDirsMerger(d)` — a shallow copy with another merger.
    // Go: overlayfs.go:WithDirsMerger
    pub fn with_dirs_merger(&self, d: DirsMerger) -> OverlayFs {
        let mut ofs = self.clone();
        ofs.merge_dirs = d;
        ofs
    }

    /// Go: `Filesystem(i)` — `None` if out of range.
    // Go: overlayfs.go:Filesystem
    pub fn filesystem(&self, i: usize) -> Option<Arc<dyn Fs>> {
        self.fss.get(i).cloned()
    }

    // Go: overlayfs.go:NumFilesystems
    pub fn num_filesystems(&self) -> usize {
        self.fss.len()
    }

    // Go: overlayfs.go:collectDirs
    fn collect_dirs(&self, name: &str, with_fs: &mut dyn FnMut(Arc<dyn Fs>)) -> Result<()> {
        for fs in &self.fss {
            self.collect_dirs_recursive(fs.clone(), name, with_fs)?;
        }
        Ok(())
    }

    // Go: overlayfs.go:collectDirsRecursive
    fn collect_dirs_recursive(
        &self,
        fs: Arc<dyn Fs>,
        name: &str,
        with_fs: &mut dyn FnMut(Arc<dyn Fs>),
    ) -> Result<()> {
        if let Ok(fi) = fs.stat(name)
            && fi.is_dir()
        {
            with_fs(fs.clone());
        }
        if let Some(children) = fs.filesystem_iterator() {
            for c in children {
                self.collect_dirs_recursive(c, name, with_fs)?;
            }
        }
        Ok(())
    }

    // Go: overlayfs.go:stat (lstatIfPossible = false: Stat, LstatIfPossible and Open all pass
    // false in v0.10.0)
    fn stat_fs(&self, name: &str) -> Result<(Arc<dyn Fs>, FileMetaInfo)> {
        for fs in &self.fss {
            match self.stat_recursive(fs.clone(), name) {
                Ok(v) => return Ok(v),
                Err(e) if !e.is_not_exist() => return Err(e),
                Err(_) => {}
            }
        }
        Err(oserror::err_not_exist())
    }

    // Go: overlayfs.go:statRecursive
    fn stat_recursive(&self, fs: Arc<dyn Fs>, name: &str) -> Result<(Arc<dyn Fs>, FileMetaInfo)> {
        match fs.stat(name) {
            Ok(fi) => return Ok((fs, fi)),
            Err(e) if !e.is_not_exist() => return Err(e),
            Err(_) => {}
        }
        if let Some(children) = fs.filesystem_iterator() {
            for c in children {
                match self.stat_recursive(c, name) {
                    Ok(v) => return Ok(v),
                    Err(e) if !e.is_not_exist() => return Err(e),
                    Err(_) => {}
                }
            }
        }
        Err(oserror::err_not_exist())
    }

    // Go: overlayfs.go:writeFs
    fn write_fs(&self) -> Result<&Arc<dyn Fs>> {
        if !self.first_writable {
            return Err(oserror::err_permission());
        }
        self.fss.first().ok_or_else(|| {
            nh_common::herrors::Error::new("overlayfs: there are no filesystems to write to")
        })
    }
}

impl Fs for OverlayFs {
    // Go: overlayfs.go:Name
    fn name(&self) -> &str {
        "overlayfs"
    }
    // Go: writeops.go:Create
    fn create(&self, name: &str) -> Result<Box<dyn File>> {
        self.write_fs()?.create(name)
    }
    // Go: writeops.go:Mkdir
    fn mkdir(&self, name: &str, perm: u32) -> Result<()> {
        self.write_fs()?.mkdir(name, perm)
    }
    // Go: writeops.go:MkdirAll
    fn mkdir_all(&self, path: &str, perm: u32) -> Result<()> {
        self.write_fs()?.mkdir_all(path, perm)
    }
    // Go: readops.go:Open
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        let (fs, fi) = self.stat_fs(name)?;

        if fi.is_dir() {
            let mut dir_fss: Vec<Arc<dyn Fs>> = Vec::new();
            self.collect_dirs(name, &mut |fs| dir_fss.push(fs))?;

            if dir_fss.is_empty() {
                // They mave been deleted.
                return Err(oserror::err_not_exist());
            }

            if dir_fss.len() == 1 {
                // Optimize for the common case.
                return dir_fss[0].open(name);
            }

            return Ok(Box::new(Dir {
                name: name.to_string(),
                fss: dir_fss,
                dir_openers: Vec::new(),
                info: None,
                merge: self.merge_dirs.clone(),
                err: None,
                offset: 0,
                fis: Vec::new(),
            }));
        }

        fs.open(name)
    }
    // Go: writeops.go:OpenFile
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
        if flag
            & (flags::O_WRONLY | flags::O_RDWR | flags::O_APPEND | flags::O_CREATE | flags::O_TRUNC)
            != 0
        {
            return self.write_fs()?.open_file(name, flag, perm);
        }
        self.open(name)
    }
    // Go: writeops.go:Remove
    fn remove(&self, name: &str) -> Result<()> {
        self.write_fs()?.remove(name)
    }
    // Go: writeops.go:RemoveAll
    fn remove_all(&self, path: &str) -> Result<()> {
        self.write_fs()?.remove_all(path)
    }
    // Go: writeops.go:Rename
    fn rename(&self, old: &str, new: &str) -> Result<()> {
        self.write_fs()?.rename(old, new)
    }
    // Go: readops.go:Stat
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        self.stat_fs(name).map(|(_, fi)| fi)
    }
    // Go: writeops.go:Chmod
    fn chmod(&self, name: &str, mode: u32) -> Result<()> {
        self.write_fs()?.chmod(name, mode)
    }
    // Go: writeops.go:Chown
    fn chown(&self, name: &str, uid: i32, gid: i32) -> Result<()> {
        self.write_fs()?.chown(name, uid, gid)
    }
    // Go: writeops.go:Chtimes
    fn chtimes(&self, name: &str, atime: &Time, mtime: &Time) -> Result<()> {
        self.write_fs()?.chtimes(name, atime, mtime)
    }
    fn filesystem_iterator(&self) -> Option<Vec<Arc<dyn Fs>>> {
        Some(self.fss.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `OpenDir`'s `info` argument.
pub type DirInfoFunc = Arc<dyn Fn() -> Result<FileMetaInfo> + Send + Sync>;

/// Go: `overlayfs.OpenDir(merge, info, dirOpeners...)` — a directory merged from the given
/// openers (`merge` `None`: the default merger).
// Go: overlayfs.go:OpenDir
pub fn open_dir(
    merge: Option<DirsMerger>,
    info: DirInfoFunc,
    dir_openers: Vec<OpenFunc>,
) -> Result<Dir> {
    if dir_openers.is_empty() {
        return Err(nh_common::herrors::Error::new(
            "overlayfs: dirOpeners must not be empty",
        ));
    }
    Ok(Dir {
        name: String::new(),
        fss: Vec::new(),
        dir_openers,
        info: Some(info),
        merge: merge.unwrap_or_else(default_dir_merger),
        err: None,
        offset: 0,
        fis: Vec::new(),
    })
}

/// Go: `overlayfs.Dir` — a list of directories that are merged in `ReadDir`.
pub struct Dir {
    // It's either a named directory in a slice of filesystems or a slice of directories.
    name: String,
    fss: Vec<Arc<dyn Fs>>,
    // Set if fss is not set.
    dir_openers: Vec<OpenFunc>,
    info: Option<DirInfoFunc>,
    merge: DirsMerger,
    err: Option<nh_common::herrors::Error>,
    offset: usize,
    fis: Vec<FileMetaInfo>,
}

no_regular_file_ops!(Dir);

impl Dir {
    // Go: overlayfs.go:isClosed
    fn is_closed(&self) -> bool {
        self.fss.is_empty() && self.dir_openers.is_empty()
    }

    fn read_dir_one(&mut self, mut f: Box<dyn File>) -> Result<()> {
        let r = f.read_dir(-1);
        let _ = f.close();
        let dir_entries = r?;
        let fis = std::mem::take(&mut self.fis);
        self.fis = (self.merge)(fis, dir_entries);
        Ok(())
    }
}

impl File for Dir {
    // Go: overlayfs.go:Name
    fn name(&self) -> String {
        self.name.clone()
    }

    // Go: overlayfs.go:Stat
    fn stat(&self) -> Result<FileMetaInfo> {
        if self.is_closed() {
            return Err(nh_common::herrors::Error::new(oserror::ERR_CLOSED));
        }
        if let Some(info) = &self.info {
            return info();
        }
        self.fss[0].stat(&self.name)
    }

    // Go: overlayfs.go:ReadDir
    fn read_dir(&mut self, n: i32) -> Result<Vec<FileMetaInfo>> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        if self.is_closed() {
            return Err(nh_common::herrors::Error::new(oserror::ERR_CLOSED));
        }

        if self.offset == 0 {
            let fss = self.fss.clone();
            for fs in fss {
                let f = fs.open(&self.name)?;
                self.read_dir_one(f)?;
            }
            let openers = self.dir_openers.clone();
            for open in openers {
                let f = open()?;
                self.read_dir_one(f)?;
            }
        }

        let fis = &self.fis[self.offset.min(self.fis.len())..];

        if n <= 0 {
            self.err = Some(oserror::eof());
            if self.offset > 0 && fis.is_empty() {
                return Err(oserror::eof());
            }
            return Ok(fis.to_vec());
        }

        if fis.is_empty() {
            self.err = Some(oserror::eof());
            return Err(oserror::eof());
        }

        let mut n = n as usize;
        if n > self.fis.len() {
            n = self.fis.len();
        }
        // Go slices `fis[:n]` within the capacity when n > len(fis) (stale entries); the port
        // stops at the end (PORTING.md, deviation 6).
        let m = n.min(fis.len());
        let out = fis[..m].to_vec();
        self.offset += n;
        Ok(out)
    }

    // Go: overlayfs.go:Readdirnames
    fn readdirnames(&mut self, n: i32) -> Result<Vec<String>> {
        if self.is_closed() {
            return Err(nh_common::herrors::Error::new(oserror::ERR_CLOSED));
        }
        Ok(self
            .read_dir(n)?
            .iter()
            .map(|fi| fi.name().to_string())
            .collect())
    }

    // Go: overlayfs.go:Close (releaseDir)
    fn close(&mut self) -> Result<()> {
        self.fss.clear();
        self.fis.clear();
        self.dir_openers.clear();
        self.info = None;
        self.offset = 0;
        self.name.clear();
        self.err = None;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (github.com/bep/overlayfs@v0.10.0; not generated)
// OK overlayfs.go: New, Append, WithDirsMerger, Filesystem, NumFilesystems, Name, collectDirs,
//    collectDirsRecursive, stat, statRecursive, writeFs, defaultDirMerger, OpenDir,
//    Dir.{Readdir (= ReadDir), ReadDir, Readdirnames, Stat, Close, Name, isClosed}
//    (the sync.Pool of Dirs is not ported; Read/Write/Seek on a Dir fail instead of panicking)
// OK readops.go: Stat, LstatIfPossible (= Stat), Open
// OK writeops.go: Chmod, Chown, Chtimes, Mkdir, MkdirAll, OpenFile, Remove, RemoveAll, Rename,
//    Create
// ---------------------------------------------------------------------------
