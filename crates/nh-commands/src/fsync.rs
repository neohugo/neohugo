//! Module `fsync`.
//!
//! NEW: spf13/fsync@v0.10.1 Syncer (copy-if-different, chmod, mtimes)
//!
//! Owner: Wave B task T25 (commands-cli).

//!
//! Port of `github.com/spf13/fsync` v0.10.1 `Syncer` as used by `copyStaticTo`:
//! recursive copy-if-different (size, then content), directory creation, optional
//! chmod/mtime sync, optional delete of extraneous destination files (`cleanDestinationDir`),
//! `DeleteFilter` / `ChmodFilter` hooks. Files are compared/copied through the afero ports.
//!
//! Go reports errors by panicking inside `sync` and recovering in `syncRecover`; the port
//! returns them. Go runs `syncstats` in a `defer`, so it also runs when `sync` panics (and its
//! own panic then replaces the first one); the port runs it on success only: an error ends
//! the sync either way and the error text is the first one.

use std::io::{Read, Write};
use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::{Error, is_not_exist};
use nh_hugofs::afero::Fs;
use nh_hugofs::fileinfo::FileMetaInfo;

/// Go: `fsync.ErrFileOverDir`.
pub const ERR_FILE_OVER_DIR: &str = "fsync: trying to overwrite a non-empty directory with a file";

/// Go: `fsync.FileInfo` (`Name()`, `IsDir()`).
pub struct FileInfo<'a> {
    pub name: &'a str,
    pub is_dir: bool,
}

/// Go: `fsync.Syncer`.
pub struct Syncer {
    pub delete: bool,
    pub no_times: bool,
    pub no_chmod: bool,
    pub src_fs: Arc<dyn Fs>,
    pub dest_fs: Arc<dyn Fs>,
    /// Go: `ChmodFilter func(dst, src os.FileInfo) bool` (Hugo: skip dirs / preserve dst mode).
    pub chmod_filter: Option<fn(dst_mode: u32, src_mode: u32, is_dir: bool) -> bool>,
    /// Go: `DeleteFilter func(f fsync.FileInfo) bool`.
    pub delete_filter: Option<fn(path: &str, is_dir: bool) -> bool>,
}

impl Syncer {
    /// Go: `fsync.NewSyncer()` with the given filesystems (Go defaults both to the OS fs and
    /// the delete filter to "never skip").
    // Go: fsync.go:NewSyncer
    pub fn new(src_fs: Arc<dyn Fs>, dest_fs: Arc<dyn Fs>) -> Syncer {
        Syncer {
            delete: false,
            no_times: false,
            no_chmod: false,
            src_fs,
            dest_fs,
            chmod_filter: None,
            delete_filter: Some(|_, _| false),
        }
    }

    /// Go: `(*Syncer).Sync(dst, src string) error`.
    // Go: fsync.go:(*Syncer).Sync
    pub fn sync(&self, dst: &str, src: &str) -> Result<()> {
        // make sure src exists
        self.src_fs.stat(src)?;
        // return error instead of replacing a non-empty directory with a file
        if self.check_dir(dst, src)? {
            return Err(Error::new(ERR_FILE_OVER_DIR));
        }
        self.sync_inner(dst, src)
    }

    // Go: fsync.go:(*Syncer).sync
    fn sync_inner(&self, dst: &str, src: &str) -> Result<()> {
        self.sync_content(dst, src)?;
        // sync permissions and modification times after handling content
        self.syncstats(dst, src)
    }

    fn sync_content(&self, dst: &str, src: &str) -> Result<()> {
        // read files info
        let dstat = match self.dest_fs.stat(dst) {
            Ok(fi) => Some(fi),
            Err(e) if is_not_exist(&e) => None,
            Err(e) => return Err(e),
        };
        let sstat = match self.src_fs.stat(src) {
            Ok(fi) => fi,
            // src was deleted before we could copy it
            Err(e) if is_not_exist(&e) => return Ok(()),
            Err(e) => return Err(e),
        };

        if !sstat.is_dir() {
            // src is a file
            // delete dst if its a directory
            if let Some(d) = &dstat
                && d.is_dir()
            {
                self.dest_fs.remove_all(dst)?;
            }
            if !self.equal(dst, src, dstat.as_ref(), Some(&sstat))? {
                // perform copy
                let mut df = self.dest_fs.create(dst)?;
                let mut sf = match self.src_fs.open(src) {
                    Ok(f) => f,
                    Err(e) if is_not_exist(&e) => {
                        let _ = df.close();
                        return Ok(());
                    }
                    Err(e) => {
                        let _ = df.close();
                        return Err(e);
                    }
                };
                let res = copy(&mut *sf, &mut *df);
                let _ = sf.close();
                let _ = df.close();
                match res {
                    Ok(()) => {}
                    Err(e) if is_not_exist(&e) => return Ok(()),
                    Err(e) => return Err(e),
                }
            }
            return Ok(());
        }

        // src is a directory
        // make dst if necessary
        match &dstat {
            None => {
                // dst does not exist; create directory
                self.dest_fs.mkdir_all(dst, 0o755)?; // permissions will be synced later
            }
            Some(d) if !d.is_dir() => {
                // dst is a file; remove and create directory
                self.dest_fs.remove(dst)?;
                self.dest_fs.mkdir_all(dst, 0o755)?; // permissions will be synced later
            }
            _ => {}
        }

        // make a map of filenames for quick lookup; used in deletion
        let mut m: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let entries = match with_dir_entry(&*self.src_fs, src) {
            Ok(e) => e,
            Err(e) if is_not_exist(&e) => return Ok(()),
            Err(e) => return Err(e),
        };
        for fi in &entries {
            let dst2 = go_path::filepath::join(&[dst, fi.name()]);
            let src2 = go_path::filepath::join(&[src, fi.name()]);
            self.sync_inner(&dst2, &src2)?;
            m.insert(fi.name().to_string());
        }

        // delete files from dst that does not exist in src
        if self.delete {
            let entries = with_dir_entry(&*self.dest_fs, dst)?;
            for fi in &entries {
                let skip = match self.delete_filter {
                    Some(f) => f(fi.name(), fi.is_dir()),
                    None => false,
                };
                if !m.contains(fi.name()) && !skip {
                    self.dest_fs
                        .remove_all(&go_path::filepath::join(&[dst, fi.name()]))?;
                }
            }
        }
        Ok(())
    }

    /// Go: `syncstats` — makes sure dst has the same permissions and modification time as src.
    // Go: fsync.go:(*Syncer).syncstats
    fn syncstats(&self, dst: &str, src: &str) -> Result<()> {
        // get file infos; return if not exist and fail if error
        let dstat = self.dest_fs.stat(dst);
        let sstat = self.src_fs.stat(src);
        if matches!(&dstat, Err(e) if is_not_exist(e))
            || matches!(&sstat, Err(e) if is_not_exist(e))
        {
            return Ok(());
        }
        let dstat = dstat?;
        let sstat = sstat?;

        // update dst's permission bits
        let mut no_chmod = self.no_chmod;
        if !no_chmod && let Some(f) = self.chmod_filter {
            no_chmod = f(dstat.mode(), sstat.mode(), sstat.is_dir());
        }
        if !no_chmod && perm(dstat.mode()) != perm(sstat.mode()) {
            self.dest_fs.chmod(dst, perm(sstat.mode()))?;
        }

        // update dst's modification time
        if !self.no_times && !dstat.mod_time().equal(sstat.mod_time()) {
            self.dest_fs
                .chtimes(dst, sstat.mod_time(), sstat.mod_time())?;
        }
        Ok(())
    }

    /// Go: `equal` — true if both dst and src files are equal (size, then 1000-byte chunks).
    // Go: fsync.go:(*Syncer).equal
    fn equal(
        &self,
        dst: &str,
        src: &str,
        dstat: Option<&FileMetaInfo>,
        sstat: Option<&FileMetaInfo>,
    ) -> Result<bool> {
        let (Some(dstat), Some(sstat)) = (dstat, sstat) else {
            return Ok(false);
        };
        // check sizes
        if dstat.size() != sstat.size() {
            return Ok(false);
        }
        // both have the same size, check the contents
        let mut f1 = self.dest_fs.open(dst)?;
        let mut f2 = match self.src_fs.open(src) {
            Ok(f) => f,
            Err(e) => {
                let _ = f1.close();
                return Err(e);
            }
        };
        let mut buf1 = [0u8; 1000];
        let mut buf2 = [0u8; 1000];
        let res = (|| -> Result<bool> {
            loop {
                // read from both
                let n1 = f1
                    .read(&mut buf1)
                    .map_err(nh_hugofs::oserror::from_io_plain)?;
                let n2 = f2
                    .read(&mut buf2)
                    .map_err(nh_hugofs::oserror::from_io_plain)?;
                // compare read bytes
                if buf1[..n1] != buf2[..n2] {
                    return Ok(false);
                }
                // end of both files
                if n1 == 0 && n2 == 0 {
                    return Ok(true);
                }
            }
        })();
        let _ = f1.close();
        let _ = f2.close();
        res
    }

    /// Go: `checkDir` — true if dst is a non-empty directory and src is a file.
    // Go: fsync.go:(*Syncer).checkDir
    fn check_dir(&self, dst: &str, src: &str) -> Result<bool> {
        // read file info
        let dstat = match self.dest_fs.stat(dst) {
            Ok(fi) => fi,
            Err(e) if is_not_exist(&e) => return Ok(false),
            Err(e) => return Err(e),
        };
        let sstat = self.src_fs.stat(src)?;

        // return false is dst is not a directory or src is a directory
        if !dstat.is_dir() || sstat.is_dir() {
            return Ok(false);
        }

        // dst is a directory and src is a file
        // check if dst is non-empty
        let entries = with_dir_entry(&*self.dest_fs, dst)?;
        Ok(!entries.is_empty())
    }
}

/// Go `os.FileMode.Perm()`.
fn perm(mode: u32) -> u32 {
    mode & 0o777
}

/// Go: `io.Copy(df, sf)`.
fn copy(src: &mut dyn Read, dst: &mut dyn Write) -> Result<()> {
    let mut buf = vec![0u8; 32 * 1024];
    loop {
        let n = src
            .read(&mut buf)
            .map_err(nh_hugofs::oserror::from_io_plain)?;
        if n == 0 {
            return Ok(());
        }
        dst.write_all(&buf[..n])
            .map_err(nh_hugofs::oserror::from_io_plain)?;
    }
}

/// Go: `withDirEntry(fs, path, fn)` — the entries of the directory (`ReadDir(-1)`), in the
/// file system's order.
// Go: fsync.go:withDirEntry
fn with_dir_entry(fs: &dyn Fs, path: &str) -> Result<Vec<FileMetaInfo>> {
    let mut f = fs.open(path)?;
    let r = f.read_dir(-1);
    let _ = f.close();
    r
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (github.com/spf13/fsync v0.10.1 fsync.go)
// OK NewSyncer, (*Syncer).Sync, syncRecover (errors are returned), sync, syncstats, equal,
//    checkDir, withDirEntry, check
//    Sync(dst, src) / SyncTo / (*Syncer).SyncTo: not used by neohugo
// ---------------------------------------------------------------------------
