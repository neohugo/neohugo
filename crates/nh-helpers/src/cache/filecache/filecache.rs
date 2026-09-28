//! Port of `cache/filecache/filecache.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).
//!
//! Go `cache/filecache`. Cold-cache rule (HUGO_LAYER.md §9): the `images` and `assets` caches are
//! never READ in the Rust port (the golden is a cold build and the Go read path changes image
//! names); `getresource` IS read (YouTube responses live there).

use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use go_path::filepath;
use go_time::Duration;
use nh_common::Result;
use nh_common::herrors::{Error, ErrorKind};
use nh_hugofs::afero::{File, Fs};

use crate::cache::httpcache::transport::HttpCache;
use crate::path::{FILE_PATH_SEPARATOR, open_file_for_writing};

/// Go: `filecache.FilecacheRootDirname`.
pub const FILECACHE_ROOT_DIRNAME: &str = "filecache";

/// Go: `filecache.ErrFatal` — can be used to signal an unrecoverable error.
pub fn err_fatal() -> Error {
    Error::with_kind(ErrorKind::Fatal, "fatal filecache error")
}

/// Go: `filecache.ItemInfo`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemInfo {
    /// This is the file's name relative to the cache's filesystem.
    pub name: String,
}

/// The `read` callback of [`Cache::read_or_create`].
pub type ReadFunc<'a> = &'a mut dyn FnMut(&ItemInfo, &mut dyn File) -> Result<()>;
/// The `create` callback of [`Cache::read_or_create`] (it owns and closes the file).
pub type CreateFunc<'a> = &'a mut dyn FnMut(&ItemInfo, Box<dyn File>) -> Result<()>;

/// Go: `lockTracker` (`BurntSushi/locker` + the ids seen): a named mutex per id.
#[derive(Default)]
struct LockTracker {
    seen: Mutex<HashSet<String>>,
    locked: Mutex<HashSet<String>>,
    cond: Condvar,
}

impl LockTracker {
    /// Go: `Lock(id)` — tracks the ids in use (for garbage collection after a build).
    // Go: cache/filecache/filecache.go:(*lockTracker).Lock
    fn lock(&self, id: &str) {
        {
            let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
            if !seen.contains(id) {
                seen.insert(id.to_string());
            }
        }
        let mut locked = self.locked.lock().unwrap_or_else(|e| e.into_inner());
        while locked.contains(id) {
            locked = self.cond.wait(locked).unwrap_or_else(|e| e.into_inner());
        }
        locked.insert(id.to_string());
    }

    /// Go: `locker.Unlock(id)`.
    fn unlock(&self, id: &str) {
        let mut locked = self.locked.lock().unwrap_or_else(|e| e.into_inner());
        locked.remove(id);
        self.cond.notify_all();
    }
}

/// Releases a named lock when dropped (Go: `defer c.nlocker.Unlock(id)`, or calling the func
/// `NamedLock` returns).
pub struct NamedLockGuard {
    l: Arc<LockTracker>,
    id: String,
}

impl Drop for NamedLockGuard {
    fn drop(&mut self) {
        self.l.unlock(&self.id);
    }
}

/// Go: `filecache.Cache` — caches a set of files in a directory (an afero filesystem).
pub struct Cache {
    pub fs: Arc<dyn Fs>,
    /// Max age for items in this cache. Negative duration means forever, 0 is effectively
    /// turning this cache off (Go `time.Duration`).
    pub max_age: Duration,
    /// When set, we just remove this entire root directory on expiration.
    pub prune_all_root_dir: String,
    nlocker: Arc<LockTracker>,
    init_once: OnceLock<Option<Error>>,
}

/// Go: `lockedFile` — a file with a lock that is released on Close (here: on drop).
pub struct LockedFile {
    file: Box<dyn File>,
    nlocker: Arc<LockTracker>,
    id: String,
}

impl LockedFile {
    /// Go: `(*lockedFile).Close()`.
    // Go: cache/filecache/filecache.go:(*lockedFile).Close
    pub fn close(mut self) -> Result<()> {
        self.file.close()
    }
}

impl Write for LockedFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl Drop for LockedFile {
    fn drop(&mut self) {
        let _ = self.file.close();
        self.nlocker.unlock(&self.id);
    }
}

impl Cache {
    /// Go: `NewCache(fs, maxAge, pruneAllRootDir)`.
    // Go: cache/filecache/filecache.go:NewCache
    pub fn new(fs: Arc<dyn Fs>, max_age: Duration, prune_all_root_dir: &str) -> Arc<Cache> {
        Arc::new(Cache {
            fs,
            max_age,
            prune_all_root_dir: prune_all_root_dir.to_string(),
            nlocker: Arc::new(LockTracker::default()),
            init_once: OnceLock::new(),
        })
    }

    /// The ids locked so far (Go `lockTracker.seen`; used for garbage collection).
    pub fn seen_ids(&self) -> Vec<String> {
        let seen = self.nlocker.seen.lock().unwrap_or_else(|e| e.into_inner());
        let mut v: Vec<String> = seen.iter().cloned().collect();
        v.sort();
        v
    }

    // Go: cache/filecache/filecache.go:init
    fn init(&self) -> Result<()> {
        let err = self.init_once.get_or_init(|| {
            // Create the base dir if it does not exist.
            match self.fs.mkdir_all("", 0o777) {
                Err(e) if !nh_common::herrors::is_exist(&e) => Some(e),
                _ => None,
            }
        });
        match err {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    fn named_lock_guard(&self, id: &str) -> NamedLockGuard {
        self.nlocker.lock(id);
        NamedLockGuard {
            l: self.nlocker.clone(),
            id: id.to_string(),
        }
    }

    /// Go: `WriteCloser(id)` — a transactional writer into the cache; the lock is held until
    /// it is closed (dropped).
    // Go: cache/filecache/filecache.go:WriteCloser
    pub fn write_closer(&self, id: &str) -> Result<(ItemInfo, LockedFile)> {
        self.init()?;

        let id = clean_id(id);
        self.nlocker.lock(&id);

        let info = ItemInfo { name: id.clone() };

        match open_file_for_writing(self.fs.as_ref(), &id) {
            Ok(f) => Ok((
                info,
                LockedFile {
                    file: f,
                    nlocker: self.nlocker.clone(),
                    id,
                },
            )),
            Err(e) => {
                self.nlocker.unlock(&id);
                Err(e)
            }
        }
    }

    /// Writes `data` as the item `id` (Go: `WriteCloser` + write + close).
    // Go: cache/filecache/filecache.go:WriteCloser
    pub fn write(&self, id: &str, data: &[u8]) -> Result<ItemInfo> {
        let (info, mut w) = self.write_closer(id)?;
        w.write_all(data).map_err(|e| Error::new(e.to_string()))?;
        w.close()?;
        Ok(info)
    }

    /// Go: `ReadOrCreate(id, read, create)` — if found, the file is passed to `read`; else a new
    /// file is created and passed to `create`. A read error other than `ErrFatal` is treated as
    /// a missing item (recovering from file corruption).
    // Go: cache/filecache/filecache.go:ReadOrCreate
    pub fn read_or_create(
        &self,
        id: &str,
        read: ReadFunc<'_>,
        create: CreateFunc<'_>,
    ) -> Result<ItemInfo> {
        self.init()?;

        let id = clean_id(id);

        let _g = self.named_lock_guard(&id);

        let info = ItemInfo { name: id.clone() };
        // TODO: checking error
        if let Ok(Some(mut r)) = self.get_or_remove(&id) {
            let err = read(&info, &mut *r);
            let _ = r.close();
            match err {
                Ok(()) => return Ok(info),
                Err(e) if e.kind() == ErrorKind::Fatal => return Err(e),
                Err(_) => {}
            }
        }

        let f = open_file_for_writing(self.fs.as_ref(), &id)?;

        create(&info, f)?;

        Ok(info)
    }

    /// Go: `NamedLock(id)` — the lock is released when the returned guard is dropped.
    // Go: cache/filecache/filecache.go:NamedLock
    pub fn named_lock(&self, id: &str) -> NamedLockGuard {
        let id = clean_id(id);
        self.named_lock_guard(&id)
    }

    /// Go: `GetOrCreate(id, create)` — a reader of the item, or of `create`'s content (cached
    /// unless `maxAge == 0`). Protected by a named lock on `id`.
    // Go: cache/filecache/filecache.go:GetOrCreate
    pub fn get_or_create(
        &self,
        id: &str,
        create: impl FnOnce() -> Result<Box<dyn Read + Send>>,
    ) -> Result<(ItemInfo, Box<dyn Read + Send>)> {
        self.init()?;
        let id = clean_id(id);

        let _g = self.named_lock_guard(&id);

        let info = ItemInfo { name: id.clone() };
        // TODO: checking error
        if let Ok(Some(r)) = self.get_or_remove(&id) {
            return Ok((info, Box::new(FileReader(r))));
        }

        let mut r = create()?;

        if self.max_age.0 == 0 {
            // No caching.
            return Ok((info, r));
        }

        // Go: c.writeReader(id, io.TeeReader(r, &buff)) and a reader of buff.
        let b = read_all(&mut *r)?;
        self.write_reader(&id, &b)?;
        Ok((info, Box::new(std::io::Cursor::new(b))))
    }

    // Go: cache/filecache/filecache.go:writeReader
    fn write_reader(&self, id: &str, b: &[u8]) -> Result<()> {
        let dir = filepath::dir(id);
        if !dir.is_empty() {
            let _ = self.fs.mkdir_all(&dir, 0o777);
        }
        let mut f = self.fs.create(id)?;

        let _ = f.write_all(b);
        let _ = f.close();

        Ok(())
    }

    /// Go: `GetOrCreateBytes(id, create)`.
    // Go: cache/filecache/filecache.go:GetOrCreateBytes
    pub fn get_or_create_bytes(
        &self,
        id: &str,
        create: impl FnOnce() -> Result<Vec<u8>>,
    ) -> Result<(ItemInfo, Vec<u8>)> {
        self.init()?;
        let id = clean_id(id);

        let _g = self.named_lock_guard(&id);

        let info = ItemInfo { name: id.clone() };
        // TODO: checking error
        if let Ok(Some(mut r)) = self.get_or_remove(&id) {
            let b = self.read_file(&id, &mut *r);
            let _ = r.close();
            return Ok((info, b?));
        }

        let b = create()?;

        if self.max_age.0 == 0 {
            return Ok((info, b));
        }

        self.write_reader(&id, &b)?;

        Ok((info, b))
    }

    /// Go: `GetBytes(id)` — `None` (Go nil) if missing/expired.
    // Go: cache/filecache/filecache.go:GetBytes
    pub fn get_bytes(&self, id: &str) -> Result<(ItemInfo, Option<Vec<u8>>)> {
        self.init()?;
        let id = clean_id(id);

        let _g = self.named_lock_guard(&id);

        let info = ItemInfo { name: id.clone() };
        // TODO: checking error
        if let Ok(Some(mut r)) = self.get_or_remove(&id) {
            let b = self.read_file(&id, &mut *r);
            let _ = r.close();
            return Ok((info, Some(b?)));
        }

        Ok((info, None))
    }

    /// Go: `Get(id)` — the open file, `None` (Go nil) if missing/expired.
    // Go: cache/filecache/filecache.go:Get
    pub fn get(&self, id: &str) -> Result<(ItemInfo, Option<Box<dyn File>>)> {
        self.init()?;
        let id = clean_id(id);

        let _g = self.named_lock_guard(&id);

        let info = ItemInfo { name: id.clone() };

        // checking error
        let r = self.get_or_remove(&id).unwrap_or(None);

        Ok((info, r))
    }

    /// Go: `getOrRemove(id)` — the file, or nothing if it's expired (then it is removed).
    // Go: cache/filecache/filecache.go:getOrRemove
    fn get_or_remove(&self, id: &str) -> Result<Option<Box<dyn File>>> {
        if self.max_age.0 == 0 {
            // No caching.
            return Ok(None);
        }

        let removed = self.remove_if_expired(id)?;
        if removed {
            return Ok(None);
        }

        Ok(Some(self.fs.open(id)?))
    }

    // Go: cache/filecache/filecache.go:getBytesAndRemoveIfExpired
    fn get_bytes_and_remove_if_expired(&self, id: &str) -> (Option<Vec<u8>>, bool) {
        if self.max_age.0 == 0 {
            // No caching.
            return (None, false);
        }

        let Ok(mut f) = self.fs.open(id) else {
            return (None, false);
        };

        let b = self.read_file(id, &mut *f);
        let _ = f.close();
        let Ok(b) = b else {
            return (None, false);
        };

        match self.remove_if_expired(id) {
            Ok(removed) => (Some(b), removed),
            Err(_) => (None, false),
        }
    }

    // Go: cache/filecache/filecache.go:removeIfExpired
    fn remove_if_expired(&self, id: &str) -> Result<bool> {
        if self.max_age.0 <= 0 {
            return Ok(false);
        }

        let fi = self.fs.stat(id)?;

        if self.is_expired(fi.mod_time()) {
            let _ = self.fs.remove(id);
            return Ok(true);
        }

        Ok(false)
    }

    // Go: cache/filecache/filecache.go:isExpired
    fn is_expired(&self, mod_time: &go_value::Time) -> bool {
        if self.max_age.0 < 0 {
            return false;
        }

        // Note the use of time.Since here.
        // We cannot use Hugo's global Clock for this.
        self.max_age.0 == 0 || go_time::since(mod_time) > self.max_age
    }

    /// Go: `GetString(id)` (for testing).
    // Go: cache/filecache/filecache.go:GetString
    pub fn get_string(&self, id: &str) -> Vec<u8> {
        let id = clean_id(id);

        let _g = self.named_lock_guard(&id);

        let Ok(mut f) = self.fs.open(&id) else {
            return Vec::new();
        };
        let b = self.read_file(&id, &mut *f).unwrap_or_default();
        let _ = f.close();
        b
    }

    /// Go: `AsHTTPCache()` (gohugoio/httpcache.Cache: Get/Set/Delete of raw response dumps).
    /// None of the methods are protected by named locks (as in Go).
    // Go: cache/filecache/filecache.go:AsHTTPCache
    pub fn as_http_cache(self: &Arc<Self>) -> Arc<dyn HttpCache> {
        Arc::new(HttpCacheImpl { c: self.clone() })
    }
}

impl Cache {
    /// Go `io.ReadAll(f)` of a cache file. Go's `*os.File` reports a read error as
    /// `read <real path>: <errno>`; nh-hugofs's `OsFile` returns the bare OS error, so it is
    /// wrapped here with the real path of the item (see PORTING.md, "Requests").
    fn read_file(&self, id: &str, f: &mut dyn File) -> Result<Vec<u8>> {
        let mut b = Vec::new();
        match f.read_to_end(&mut b) {
            Ok(_) => Ok(b),
            Err(e) if e.raw_os_error().is_some() => Err(nh_hugofs::oserror::from_io(
                "read",
                &self.real_path(id, f),
                &e,
            )),
            Err(e) => Err(nh_hugofs::oserror::from_io_plain(e)),
        }
    }

    /// The OS path of the item `id` (the cache fs is a `BasePathFs`, possibly wrapped).
    fn real_path(&self, id: &str, f: &dyn File) -> String {
        let mut fs: &dyn Fs = self.fs.as_ref();
        if let Some(w) = fs
            .as_any()
            .downcast_ref::<nh_hugofs::fs::FilesystemsWrapper>()
        {
            fs = w.container().as_ref();
        }
        match fs.as_any().downcast_ref::<nh_hugofs::afero::BasePathFs>() {
            Some(bp) => bp.real_path(id).unwrap_or_else(|(n, _)| n),
            None => f.name(),
        }
    }
}

/// Go: `filecache.Caches` — a named set of caches (getjson, getcsv, images, assets, modules,
/// getresource, misc).
#[derive(Clone, Default)]
pub struct Caches(pub BTreeMap<String, Arc<Cache>>);

impl Caches {
    /// Go: `Caches.Get(name)` — a named cache, `None` (Go nil) if none found.
    // Go: cache/filecache/filecache.go:(Caches).Get
    pub fn get(&self, name: &str) -> Option<Arc<Cache>> {
        self.0
            .get(go_unicode::strings::to_lower_str(name).as_ref())
            .cloned()
    }
}

/// Go: `filecache.NewCaches(p)` — the file caches from the `caches` config section.
// Go: cache/filecache/filecache.go:NewCaches
pub fn new_caches(p: &crate::pathspec::PathSpec) -> Result<Caches> {
    let dcfg = nh_config::config_provider::config_section::<super::filecache_config::Configs>(
        p.cfg.as_ref(),
        "caches",
    );
    let fs = p.fs.source.clone();

    let mut m = BTreeMap::new();
    for (k, v) in dcfg.iter() {
        let cfs = if v.is_resource_dir {
            p.base_fs.resources_cache.clone()
        } else {
            fs.clone()
        };

        let base_dir = &v.dir_compiled;

        let bfs = nh_hugofs::fs::new_base_path_fs(cfs, base_dir);

        let mut prune_all_root_dir = "";
        if k == super::filecache_config::CACHE_KEY_MODULES {
            prune_all_root_dir = "pkg";
        }

        m.insert(k.clone(), Cache::new(bfs, v.max_age, prune_all_root_dir));
    }

    Ok(Caches(m))
}

/// Go: `cleanID(name)` = `strings.TrimPrefix(filepath.Clean(name), "/")`.
// Go: cache/filecache/filecache.go:cleanID
pub fn clean_id(name: &str) -> String {
    let c = filepath::clean(name);
    c.strip_prefix(FILE_PATH_SEPARATOR)
        .unwrap_or(&c)
        .to_string()
}

/// Go: `httpCache`.
struct HttpCacheImpl {
    c: Arc<Cache>,
}

impl HttpCache for HttpCacheImpl {
    /// Go: `(*httpCache).Get(id)` — `(bytes, !removed)`: a missing item is `(nil, true)`, an
    /// expired one `(bytes, false)` (and it is removed).
    // Go: cache/filecache/filecache.go:(*httpCache).Get
    fn get(&self, id: &str) -> (Option<Vec<u8>>, bool) {
        let id = clean_id(id);
        let (b, removed) = self.c.get_bytes_and_remove_if_expired(&id);
        (b, !removed)
    }

    /// Go: `(*httpCache).Set(id, resp)` (panics on a write error, like Go).
    // Go: cache/filecache/filecache.go:(*httpCache).Set
    fn set(&self, id: &str, resp: &[u8]) {
        if self.c.max_age.0 == 0 {
            return;
        }

        let id = clean_id(id);

        if let Err(e) = self.c.write_reader(&id, resp) {
            panic!("{e}");
        }
    }

    /// Go: `(*httpCache).Delete(key)` (the key is not cleaned, as in Go).
    // Go: cache/filecache/filecache.go:(*httpCache).Delete
    fn delete(&self, key: &str) {
        let _ = self.c.fs.remove(key);
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: cache/filecache/filecache.go (499 lines; 14/24 funcs executed)
//   types: Cache, lockTracker, ItemInfo, lockedFile, Caches, httpCache
// OK L70-82: (l *lockTracker) Lock(id string)
// OK L91-98: NewCache(fs afero.Fs, maxAge time.Duration, pruneAllRootDir string) *Cache
// OK L106-109: (l *lockedFile) Close() error
// OK L111-119: (c *Cache) init() error
// OK L123-143: (c *Cache) WriteCloser(id string) (ItemInfo, io.WriteCloser, error)
// OK L149-184: (c *Cache) ReadOrCreate(id string, read func(info ItemInfo, r io.ReadSeeker) error, create func(info ItemInfo, w io.WriteCloser) error, ) (info Ite...
// OK L187-193: (c *Cache) NamedLock(id string) func()
// OK L198-232: (c *Cache) GetOrCreate(id string, create func() (io.ReadCloser, error)) (ItemInfo, io.ReadCloser, error)
// OK L234-248: (c *Cache) writeReader(id string, r io.Reader) error
// OK L251-288: (c *Cache) GetOrCreateBytes(id string, create func() ([]byte, error)) (ItemInfo, []byte, error)
// OK L291-310: (c *Cache) GetBytes(id string) (ItemInfo, []byte, error)
// OK L313-328: (c *Cache) Get(id string) (ItemInfo, io.ReadCloser, error)
// OK L332-348: (c *Cache) getOrRemove(id string) (hugio.ReadSeekCloser, error)
// OK L350-373: (c *Cache) getBytesAndRemoveIfExpired(id string) ([]byte, bool)
// OK L375-391: (c *Cache) removeIfExpired(id string) (bool, error)
// OK L393-401: (c *Cache) isExpired(modTime time.Time) bool
// OK L404-418: (c *Cache) GetString(id string) string
// OK L424-426: (f Caches) Get(name string) *Cache
// OK L430-461: NewCaches(p *helpers.PathSpec) (Caches, error)
// OK L463-465: cleanID(name string) string
// OK L470-472: (c *Cache) AsHTTPCache() httpcache.Cache
// OK L478-483: (h *httpCache) Get(id string) (resp []byte, ok bool)
// OK L485-495: (h *httpCache) Set(id string, resp []byte)
// OK L497-499: (h *httpCache) Delete(key string)
// ---------------------------------------------------------------------------

/// An open cache file as a reader (closed when dropped).
struct FileReader(Box<dyn File>);

impl Read for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Drop for FileReader {
    fn drop(&mut self) {
        let _ = self.0.close();
    }
}

/// Go `io.ReadAll` of a reader made by a `create` func.
fn read_all(r: &mut dyn Read) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    r.read_to_end(&mut b)
        .map_err(nh_hugofs::oserror::from_io_plain)?;
    Ok(b)
}
