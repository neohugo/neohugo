//! The deferred image queue and its file cache (`[caches.images]`).
//!
//! [`ImageQueue::enqueue`] plans an operation from metadata only and returns its final name
//! and size at once, so templates can print `.Width` and `.RelPermalink` without decoding
//! pixels. The pixels are produced later, in parallel and outside any render, by
//! [`ImageQueue::process`] (build phase E6), or on demand by [`ImageQueue::encoded`].

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, SystemTime};

use neohugo_base::paths::OutputPath;
use neohugo_base::{ImageOpId, Sink};
use neohugo_config::global::{FileCache, MaxAge};
use rayon::prelude::*;
use xxhash_rust::xxh3::xxh3_64;

use crate::codec;
use crate::error::ImageError;
use crate::exif;
use crate::filter::{ImageFilter, ImageInput};
use crate::format::ImageFormat;
use crate::pixels;
use crate::plan::{InputInfo, InputRef, Plan};
use crate::settings::Imaging;
use crate::spec::ImageSpec;

/// The result of [`ImageQueue::enqueue`]: known before any pixel is processed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Enqueued {
    /// Identifies the operation *and* its name: the same content and operation under two
    /// source names (identical bundle images) are two operations that share their pixels.
    pub id: ImageOpId,
    /// `<source stem>_hu_<16 hex digits>.<ext>`, as in Go: the stem and extension are the
    /// source's own and the digits hash the source content and the planned operation.
    pub file_name: String,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

/// Where processed images are kept between builds.
#[derive(Clone, Debug)]
pub struct ImageCache {
    pub dir: PathBuf,
    pub max_age: MaxAge,
}

impl ImageCache {
    /// The `images` cache of `[caches]`.
    #[must_use]
    pub fn from_config(cache: &FileCache) -> Self {
        Self {
            dir: cache.path.clone(),
            max_age: cache.max_age,
        }
    }

    fn path(&self, file_name: &str) -> PathBuf {
        self.dir.join(file_name)
    }

    /// The cached bytes, when present and not older than the maximum age.
    fn read(&self, file_name: &str) -> Option<Vec<u8>> {
        let path = self.path(file_name);
        match self.max_age {
            MaxAge::Forever => {}
            MaxAge::For(age) if age == Duration::ZERO => return None,
            MaxAge::For(age) => {
                let modified = fs::metadata(&path).and_then(|m| m.modified()).ok()?;
                let elapsed = SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default();
                if elapsed > age {
                    return None;
                }
            }
        }
        fs::read(path).ok()
    }

    fn write(&self, file_name: &str, bytes: &[u8]) -> Result<(), ImageError> {
        fs::create_dir_all(&self.dir).map_err(|e| ImageError::io(&self.dir, e))?;
        let path = self.path(file_name);
        // Write then rename, so a concurrent reader never sees a partial file.
        let tmp = self.dir.join(format!(".{file_name}.tmp"));
        fs::write(&tmp, bytes).map_err(|e| ImageError::io(&tmp, e))?;
        fs::rename(&tmp, &path).map_err(|e| ImageError::io(&path, e))
    }
}

/// What the queue knows about a source file.
struct SourceMeta {
    /// xxh3 of the file's bytes.
    hash: u64,
    info: InputInfo,
    stem: String,
    /// The extension as spelled, with its dot (`.JPG`), or empty.
    ext: String,
}

/// A processed result, shared by the operations that differ only in their name.
type SharedResult = Arc<OnceLock<Arc<[u8]>>>;

struct Op {
    input: ImageInput,
    plan: Plan,
    out: Enqueued,
    /// The extension of the result as spelled in `out.file_name`.
    ext: String,
    stem: String,
    /// The hash of the source content and the operation (the digits of the name).
    digest: u64,
    /// Shared by every operation with the same `digest`: identical bytes are processed once.
    result: SharedResult,
}

/// Queued image operations, shared by every render of a build.
pub struct ImageQueue {
    imaging: Imaging,
    cache: Option<ImageCache>,
    ops: Mutex<BTreeMap<ImageOpId, Arc<Op>>>,
    sources: Mutex<BTreeMap<PathBuf, Arc<SourceMeta>>>,
    results: Mutex<BTreeMap<u64, SharedResult>>,
}

fn stem_of(name: &str) -> &str {
    // A processed image keeps the name of its source: `a_hu_1234.jpg` → `a`.
    name.rsplit_once("_hu_").map_or(name, |(s, _)| s)
}

impl ImageQueue {
    /// A queue with the site's `[imaging]` defaults and, unless `None`, the `[caches.images]`
    /// file cache.
    #[must_use]
    pub fn new(imaging: Imaging, cache: Option<ImageCache>) -> Self {
        Self {
            imaging,
            cache,
            ops: Mutex::new(BTreeMap::new()),
            sources: Mutex::new(BTreeMap::new()),
            results: Mutex::new(BTreeMap::new()),
        }
    }

    /// The `[imaging]` settings.
    #[must_use]
    pub fn imaging(&self) -> &Imaging {
        &self.imaging
    }

    fn op(&self, id: ImageOpId) -> Result<Arc<Op>, ImageError> {
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .cloned()
            .ok_or(ImageError::UnknownOp(id))
    }

    /// The metadata of a source file (read once per path).
    fn source(&self, path: &Path) -> Result<Arc<SourceMeta>, ImageError> {
        if let Some(m) = self
            .sources
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
        {
            return Ok(Arc::clone(m));
        }
        let bytes = fs::read(path).map_err(|e| ImageError::io(path, e))?;
        let what = path.display().to_string();
        let (size, format) = codec::probe(&bytes, &what)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        let (stem, ext) = match name.rfind('.') {
            Some(i) if i > 0 => (&name[..i], &name[i..]),
            _ => (&*name, ""),
        };
        let meta = Arc::new(SourceMeta {
            hash: xxh3_64(&bytes),
            info: InputInfo {
                size,
                format,
                orientation: exif::orientation(&bytes),
            },
            stem: stem_of(stem).to_owned(),
            ext: ext.to_owned(),
        });
        self.sources
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path.to_owned(), Arc::clone(&meta));
        Ok(meta)
    }

    fn input_ref(&self, input: &ImageInput) -> Result<InputRef, ImageError> {
        let identity = match input {
            ImageInput::File(p) => self.source(p)?.hash,
            ImageInput::Op(id) => self.op(*id)?.digest,
        };
        Ok(InputRef {
            input: input.clone(),
            identity,
        })
    }

    /// Plans `spec` (if any) followed by `filters` on `input` and queues it. Returns the
    /// result's name and size without processing pixels. Enqueueing the same operation twice
    /// returns the same id.
    ///
    /// # Errors
    /// An unreadable source or one whose format is unknown, an unknown input operation, or
    /// an operation whose result would be empty.
    pub fn enqueue(
        &self,
        input: &ImageInput,
        spec: Option<&ImageSpec>,
        filters: &[ImageFilter],
    ) -> Result<Enqueued, ImageError> {
        let (info, identity, stem, ext) = match input {
            ImageInput::File(p) => {
                let m = self.source(p)?;
                let info = InputInfo {
                    size: m.info.size,
                    format: m.info.format,
                    orientation: m.info.orientation,
                };
                (info, m.hash, m.stem.clone(), m.ext.clone())
            }
            ImageInput::Op(id) => {
                let op = self.op(*id)?;
                let info = InputInfo {
                    size: (op.out.width, op.out.height),
                    format: op.out.format,
                    // Processed results carry no EXIF data.
                    orientation: None,
                };
                (info, op.digest, op.stem.clone(), op.ext.clone())
            }
        };
        let plan = Plan::new(&info, spec, filters, &self.imaging, &mut |i| {
            self.input_ref(i)
        })?;
        let hash = xxh3_64(format!("{identity:016x}|{}", plan.key()).as_bytes());
        let format = plan.encode.format;
        // Keep the source's spelling (`.JPEG`) when it names the result's format.
        let ext = if ImageFormat::from_extension(&ext) == Some(format) {
            ext
        } else {
            format.extension().to_owned()
        };
        let file_name = format!("{stem}_hu_{hash:016x}{ext}");
        // The name is part of the identity: keyed by the digits alone, identical images in
        // two bundles would share whichever name was queued first (non-deterministic).
        let id = xxh3_64(format!("{hash:016x}|{file_name}").as_bytes());
        let out = Enqueued {
            id: ImageOpId::from_raw(id),
            file_name,
            width: plan.size.0,
            height: plan.size.1,
            format,
        };
        let mut ops = self.ops.lock().unwrap_or_else(PoisonError::into_inner);
        let op = ops.entry(out.id).or_insert_with(|| {
            let result = Arc::clone(
                self.results
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .entry(hash)
                    .or_default(),
            );
            Arc::new(Op {
                input: input.clone(),
                plan,
                out,
                ext,
                stem,
                digest: hash,
                result,
            })
        });
        Ok(op.out.clone())
    }

    /// The description of a queued operation.
    #[must_use]
    pub fn get(&self, id: ImageOpId) -> Option<Enqueued> {
        self.op(id).ok().map(|op| op.out.clone())
    }

    /// The number of queued operations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// Whether nothing is queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Decoded pixels of an input.
    fn pixels(&self, input: &ImageInput) -> Result<codec::Decoded, ImageError> {
        match input {
            ImageInput::File(p) => {
                let bytes = fs::read(p).map_err(|e| ImageError::io(p, e))?;
                codec::decode(&bytes, &p.display().to_string())
            }
            ImageInput::Op(id) => {
                let op = self.op(*id)?;
                let bytes = self.encoded(*id)?;
                codec::decode(&bytes, &op.out.file_name)
            }
        }
    }

    /// The encoded result of an operation, processing it (and the operations it reads) if
    /// neither this build nor the file cache has it.
    ///
    /// # Errors
    /// An unknown id, an unreadable input, or a failing filter or encoder.
    pub fn encoded(&self, id: ImageOpId) -> Result<Arc<[u8]>, ImageError> {
        let op = self.op(id)?;
        if let Some(bytes) = op.result.get() {
            return Ok(Arc::clone(bytes));
        }
        let cached = self.cache.as_ref().and_then(|c| c.read(&op.out.file_name));
        let bytes: Arc<[u8]> = if let Some(bytes) = cached {
            bytes.into()
        } else {
            let src = self.pixels(&op.input)?;
            let load = |r: &InputRef| self.pixels(&r.input).map(|d| d.image);
            let img = pixels::run(src.image, &op.plan.steps, &load)?;
            let bytes = codec::encode(img, src.gray, &op.plan.encode)?;
            if let Some(cache) = &self.cache {
                cache.write(&op.out.file_name, &bytes)?;
            }
            bytes.into()
        };
        Ok(Arc::clone(op.result.get_or_init(|| bytes)))
    }

    /// Processes the wanted operations in parallel and writes each result to its target.
    /// Must be called outside any render (build phase E6).
    ///
    /// # Errors
    /// The first failure in target order: processing (see [`ImageQueue::encoded`]) or
    /// writing to the sink.
    pub fn process(
        &self,
        wanted: &BTreeMap<OutputPath, ImageOpId>,
        sink: &dyn Sink,
    ) -> Result<(), ImageError> {
        let mut ids: Vec<ImageOpId> = wanted.values().copied().collect();
        ids.sort_unstable();
        ids.dedup();
        // Operations that differ only in their name share their pixels: process one of each
        // first, so the others find the shared result.
        let mut firsts: BTreeMap<u64, ImageOpId> = BTreeMap::new();
        for id in &ids {
            if let Ok(op) = self.op(*id) {
                firsts.entry(op.digest).or_insert(*id);
            }
        }
        firsts
            .into_values()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|id| {
                // Errors are reported below, in target order.
                let _ = self.encoded(id);
            });
        let mut results: BTreeMap<ImageOpId, Result<Arc<[u8]>, ImageError>> = ids
            .into_par_iter()
            .map(|id| (id, self.encoded(id)))
            .collect::<Vec<_>>()
            .into_iter()
            .collect();
        for (target, id) in wanted {
            if let Some(Err(_)) = results.get(id)
                && let Some(Err(e)) = results.remove(id)
            {
                return Err(e);
            }
            if let Some(Ok(bytes)) = results.get(id) {
                sink.write(target, bytes)
                    .map_err(|e| ImageError::io(target.as_str(), e))?;
            }
        }
        Ok(())
    }
}
