//! Port of `resources/resource.go`.
//!
//! Owner: Wave B task T14 (resources-core).

//! Go `resources/resource.go`: `genericResource` (source bytes, target paths, publish-once, Key),
//! `ResourceSourceDescriptor`, `Copy`.
//!
//! COLD-CACHE RULE (HUGO_LAYER.md §4.5): `Key()` = RelPermalink minus base path, plus
//! `"_" + decimal(xxhash64(ORIGINAL source bytes))` when `include_hash_in_key && !source_filename_is_hash`.
//! The Rust port never sets `source_filename_is_hash` (it never reads the images file cache), so
//! processed images always carry the hash suffix — exactly the golden (cold) behaviour. Clones share
//! the root's hash (`Arc<ResourceHash>`).

use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use go_value::{GoString, Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::hugio::{OpenReadSeekCloser, ReadSeekCloser};
use nh_common::paths::pathparser::Path;
use nh_media::media::media_type::MediaType;
use nh_resource::internal::resourcepaths::ResourcePaths;
use nh_resource::resourcetypes::Resource;

use crate::resource_spec::Spec;
use crate::transform::TransformationUpdate;

/// Go: `resources.ResourceSourceDescriptor`.
#[derive(Clone, Default)]
pub struct ResourceSourceDescriptor {
    /// The source content.
    pub open_read_seek_closer: Option<OpenReadSeekCloser>,
    /// The canonical source path.
    pub path: Option<Arc<Path>>,
    /// The normalized name of the resource.
    pub name_normalized: String,
    /// The name of the resource as it was read from the source.
    pub name_original: String,
    /// Any base paths prepended to the target path (multihost).
    pub target_base_paths: Vec<String>,
    /// The target path relative to the publish dir (defaults to the path).
    pub target_path: String,
    pub base_path_rel_permalink: String,
    pub base_path_target_path: String,
    /// The source filename or path (for error messages / cache keys).
    pub source_filename_or_path: String,
    pub title: String,
    /// Go `Data map[string]any` (`None` = nil).
    pub data: Option<Map>,
    /// Go `Params maps.Params` (`None` = nil; `init` makes it an empty map).
    pub params: Option<Map>,
    /// Delay publishing until either Permalink or RelPermalink is called. Maybe never.
    pub lazy_publish: bool,
    /// Set if a specific media type is wanted (else from the target path extension).
    pub media_type: Option<MediaType>,
}

impl ResourceSourceDescriptor {
    /// Go panics on a nil opener or an empty target path; the port returns the panic message.
    // Go: resources/resource.go:init
    pub(crate) fn init(&mut self, r: &Spec) -> Result<()> {
        if self.target_base_paths.is_empty() {
            // If not set, we publish the same resource to all hosts.
            self.target_base_paths = r.multihost_target_base_paths();
        }

        if self.open_read_seek_closer.is_none() {
            return Err(Error::new("OpenReadSeekCloser is nil"));
        }

        if self.target_path.is_empty() {
            return Err(Error::new("RelPath is empty"));
        }

        if self.params.is_none() {
            self.params = Some(Map::new(MapType::Params));
        }

        if self.path.is_none() {
            self.path = Some(Arc::new(
                r.path_spec.cfg.path_parser().parse("", &self.target_path),
            ));
        }

        // (Go's `if fd.TargetPath == ""` branch cannot be reached: it returned above.)
        self.target_path = nh_common::paths::path::to_slash_preserve_leading(&self.target_path);

        self.base_path_rel_permalink =
            nh_common::paths::path::to_slash_preserve_leading(&self.base_path_rel_permalink);
        if self.base_path_rel_permalink == "/" {
            self.base_path_rel_permalink = String::new();
        }
        self.base_path_target_path =
            nh_common::paths::path::to_slash_preserve_leading(&self.base_path_target_path);
        if self.base_path_target_path == "/" {
            self.base_path_target_path = String::new();
        }

        self.target_path = nh_common::paths::path::to_slash_preserve_leading(&self.target_path);

        if self.name_normalized.is_empty() {
            self.name_normalized = self.target_path.clone();
        }

        if self.name_original.is_empty() {
            self.name_original = self.name_normalized.clone();
        }

        if self.title.is_empty() {
            self.title = self.name_original.clone();
        }

        let mut media_type = self.media_type.clone().unwrap_or_default();
        if media_type.is_zero() {
            let path = self.path.as_ref().expect("path set above");
            let ext = path.ext().to_string();
            let media_types = r.media_types();
            let (mut mt, suffix_info, mut found) = media_types.get_by_suffix_found_first(&ext);
            // TODO(bep) we need to handle these ambiguous types better, but in this context
            // we most likely want the application/xml type.
            if suffix_info.suffix == "xml" && mt.sub_type == "rss" {
                let (m2, f2) = media_types.get_by_type_found("application/xml");
                mt = m2;
                found = f2;
            }

            if !found {
                // A fallback. Note that mime.TypeByExtension is slow by Hugo standards,
                // so we should configure media types to avoid this lookup for most
                // situations.
                let mut dot_ext = b".".to_vec();
                dot_ext.extend_from_slice(ext.as_bytes());
                let mime_str = crate::mime::type_by_extension(&dot_ext);
                if !mime_str.is_empty() {
                    let mime_str = String::from_utf8_lossy(&mime_str).into_owned();
                    // Go ignores the error (and keeps the zero value it returns with it).
                    mt = MediaType::from_string_and_ext(&mime_str, &[ext.as_str()])
                        .unwrap_or_default();
                }
            }
            media_type = mt;
        }

        self.media_type = Some(media_type);

        // (DependencyManager: identity tracking is not ported.)

        Ok(())
    }
}

/// Helpers over `nh_media::Types` that return Go's triple.
trait TypesExt {
    fn get_by_suffix_found_first(
        &self,
        suffix: &str,
    ) -> (MediaType, nh_media::media::media_type::SuffixInfo, bool);
}

impl TypesExt for nh_media::media::media_type::Types {
    // Go: media/mediaType.go:GetFirstBySuffix
    fn get_by_suffix_found_first(
        &self,
        suffix: &str,
    ) -> (MediaType, nh_media::media::media_type::SuffixInfo, bool) {
        match self.get_first_by_suffix(suffix) {
            Some((m, s)) => (m, s, true),
            None => (MediaType::default(), Default::default(), false),
        }
    }
}

/// Go: `resources.AtomicStaler` (and `StaleValue`'s `StaleVersion`).
#[derive(Debug, Default)]
pub struct AtomicStaler {
    stale: AtomicU32,
}

impl AtomicStaler {
    // Go: resources/resource.go:MarkStale
    pub fn mark_stale(&self) {
        self.stale.fetch_add(1, Ordering::SeqCst);
    }

    // Go: resources/resource.go:StaleVersion
    pub fn stale_version(&self) -> u32 {
        self.stale.load(Ordering::SeqCst)
    }
}

/// Go: `resources.StaleValue[V]`.
pub struct StaleValue<V> {
    /// The value.
    pub value: V,
    /// StaleVersionFunc reports the current version of the value. This always starts out at 0
    /// and get incremented on staleness.
    pub stale_version_func: Box<dyn Fn() -> u32 + Send + Sync>,
}

impl<V> StaleValue<V> {
    // Go: resources/resource.go:(*StaleValue[V]).StaleVersion
    pub fn stale_version(&self) -> u32 {
        (self.stale_version_func)()
    }
}

/// Go: `resourceHash` (xxhash64 of the original source + size), shared by clones.
#[derive(Default)]
pub struct ResourceHash {
    /// `Err` holds the message of Go's `initErr`.
    pub(crate) value: OnceLock<std::result::Result<(u64, i64), String>>,
}

impl ResourceHash {
    /// Go: `init(l)`; only the call that ran the `sync.Once` returns the error in Go, the port
    /// returns it on every call (the value is unusable either way: Go panics in `hash()`).
    // Go: resources/resource.go:(*resourceHash).init
    pub(crate) fn init(&self, open: &OpenReadSeekCloser) -> Result<(u64, i64)> {
        let v = self.value.get_or_init(|| {
            let mut f = match open() {
                Ok(f) => f,
                Err(e) => return Err(format!("failed to open source: {e}")),
            };
            match hash_image(&mut f) {
                Ok(v) => Ok(v),
                Err(e) => Err(format!("failed to calculate hash: {e}")),
            }
        });
        v.clone().map_err(Error::new)
    }
}

/// Go: `hashImage(r)` = `hashing.XXHashFromReader(r)`. The source is an `afero.File` or a
/// string reader; the size is only used for the Exif cache file name (never read or written by
/// the port), so the `ReadFrom` size rule is used for all.
// Go: resources/resource.go:hashImage
fn hash_image(r: &mut dyn Read) -> Result<(u64, i64)> {
    nh_common::hashing::xxhash_from_reader(r, nh_common::hashing::ReaderKind::ReadFrom)
}

/// Go: `genericResource`.
pub struct GenericResource {
    pub(crate) spec: Arc<Spec>,
    pub(crate) sd: ResourceSourceDescriptor,
    pub(crate) paths: ResourcePaths,
    pub(crate) include_hash_in_key: bool,
    /// Always false in the Rust port (cold semantics).
    pub(crate) source_filename_is_hash: bool,
    pub(crate) h: Arc<ResourceHash>,
    /// Go `resource.Staler` (a pointer shared by clones).
    pub(crate) staler: Arc<AtomicStaler>,
    pub(crate) title: String,
    pub(crate) name: String,
    /// Go `params maps.Params` (a reference: shared by clones and front matter metadata).
    pub(crate) params: crate::resource_metadata::SharedParams,
    /// Go `key` + `keyInit`.
    pub(crate) key: OnceLock<String>,
    /// Go `publishInit *lazy.OnceMore`.
    pub(crate) published: OnceLock<()>,
}

impl nh_images::image::Spec for GenericResource {
    fn read_seek_closer(&self) -> Result<Box<dyn ReadSeekCloser>> {
        GenericResource::read_seek_closer(self)
    }
}

impl GenericResource {
    // Go: resources/resource.go:(*genericResource).GetDependencyManager
    // Go: resources/resource.go:(*genericResource).GetIdentityGroup
    // Go: resources/resource.go:(*genericResource).IdentifierBase
    /// Go: `IdentifierBase()` (the identity system itself is not ported).
    pub fn identifier_base(&self) -> String {
        self.sd
            .path
            .as_ref()
            .map(|p| p.identifier_base().to_string())
            .unwrap_or_default()
    }

    // Go: resources/resource.go:(*genericResource).ReadSeekCloser
    pub fn read_seek_closer(&self) -> Result<Box<dyn ReadSeekCloser>> {
        match &self.sd.open_read_seek_closer {
            Some(open) => open(),
            None => Err(Error::new("OpenReadSeekCloser is nil")),
        }
    }

    /// Go: `Clone()` (a copy with fresh publish/key state).
    // Go: resources/resource.go:(*genericResource).Clone
    pub fn clone_generic(&self) -> GenericResource {
        self.clone_resource()
    }

    // Go: resources/resource.go:(*genericResource).size
    pub fn size(&self) -> i64 {
        self.hash_result().map(|(_, s)| s).unwrap_or(0)
    }

    fn hash_result(&self) -> Result<(u64, i64)> {
        match &self.sd.open_read_seek_closer {
            Some(open) => self.h.init(open),
            None => Err(Error::new(
                "failed to open source: OpenReadSeekCloser is nil",
            )),
        }
    }

    /// Go: `hash()` — xxhash64 of the source (lazily, once, shared by clones). Go panics when
    /// the source cannot be read; so does the port (the message is Go's `initErr`).
    // Go: resources/resource.go:(*genericResource).hash
    pub(crate) fn hash(&self) -> u64 {
        match self.hash_result() {
            Ok((h, _)) => h,
            Err(e) => panic!("{e}"),
        }
    }

    /// [`GenericResource::hash`] as a `Result` (for callers that can return the error).
    pub fn try_hash(&self) -> Result<u64> {
        self.hash_result().map(|(h, _)| h)
    }

    // Go: resources/resource.go:(*genericResource).setOpenSource
    pub(crate) fn set_open_source(&mut self, open_source: OpenReadSeekCloser) {
        self.sd.open_read_seek_closer = Some(open_source);
    }

    // Go: resources/resource.go:(*genericResource).setSourceFilenameIsHash
    pub fn set_source_filename_is_hash(&mut self, b: bool) {
        self.source_filename_is_hash = b;
    }

    // Go: resources/resource.go:(*genericResource).setTargetPath
    pub(crate) fn set_target_path(&mut self, d: ResourcePaths) {
        self.paths = d;
    }

    // Go: resources/resource.go:(*genericResource).cloneTo
    pub(crate) fn clone_to(&self, target_path: &str) -> GenericResource {
        let mut c = self.clone_resource();
        c.paths = c.paths.from_target_path(target_path);
        c
    }

    /// Go: `Content(ctx)` — the source as a string.
    // Go: resources/resource.go:(*genericResource).Content
    pub fn content(&self) -> Result<Value> {
        let mut r = self.read_seek_closer()?;
        let b = nh_common::hugio::read_all(&mut r)?;
        Ok(Value::String(GoString::from(b)))
    }

    /// Go: `Data()` — `sd.Data` (a nil `map[string]any` when unset).
    // Go: resources/resource.go:(*genericResource).Data
    pub fn data(&self) -> Value {
        match &self.sd.data {
            Some(m) => Value::map(m.clone()),
            None => Value::TypedNil(Arc::from("map[string]interface {}")),
        }
    }

    /// Go: `Key()` (see the module doc for the cold-cache rule).
    // Go: resources/resource.go:(*genericResource).Key
    pub fn key(&self) -> String {
        self.key
            .get_or_init(|| {
                let base_path = self
                    .spec
                    .path_spec
                    .cfg
                    .base_url()
                    .base_path_no_trailing_slash;
                let mut key = if base_path.is_empty() {
                    self.rel_permalink()
                } else {
                    let rp = self.rel_permalink();
                    rp.strip_prefix(base_path.as_str())
                        .map(str::to_string)
                        .unwrap_or(rp)
                };

                if self.spec.path_spec.cfg.is_multihost() {
                    key = format!("{}{key}", self.spec.lang());
                }

                if self.include_hash_in_key && !self.source_filename_is_hash {
                    key.push_str(&format!("_{}", self.hash()));
                }
                key
            })
            .clone()
    }

    // Go: resources/resource.go:(*genericResource).TransientKey
    pub fn transient_key(&self) -> String {
        self.key()
    }

    // Go: resources/resource.go:(*genericResource).targetPath
    pub(crate) fn target_path_internal(&self) -> String {
        self.paths.target_path()
    }

    // Go: resources/resource.go:(*genericResource).sourcePath
    pub(crate) fn source_path(&self) -> String {
        self.sd.source_filename_or_path.clone()
    }

    // Go: resources/resource.go:(*genericResource).MediaType
    pub fn media_type(&self) -> MediaType {
        self.sd.media_type.clone().unwrap_or_default()
    }

    // Go: resources/resource.go:(*genericResource).setMediaType
    pub(crate) fn set_media_type(&mut self, media_type: MediaType) {
        self.sd.media_type = Some(media_type);
    }

    // Go: resources/resource.go:(*genericResource).Name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    // Go: resources/resource.go:(*genericResource).NameNormalized
    pub fn name_normalized(&self) -> String {
        self.sd.name_normalized.clone()
    }

    // Go: resources/resource.go:(*genericResource).Params
    pub fn params(&self) -> Arc<Map> {
        self.params.get()
    }

    /// Go: `Publish()` — copy the source to every target filename (publish fs), once. Only the
    /// call that ran the copy returns its error (Go's `lazy.OnceMore` + a captured `err`).
    // Go: resources/resource.go:(*genericResource).Publish
    pub fn publish(&self) -> Result<()> {
        let mut err: Result<()> = Ok(());
        self.published.get_or_init(|| {
            err = self.do_publish();
        });
        err
    }

    fn do_publish(&self) -> Result<()> {
        let publish_fs = self.spec.publish_fs();
        let mut target_filenames = self.get_resource_paths().target_filenames();

        if self.source_filename_is_hash {
            // This is a processed image. We want to avoid copying it if it hasn't changed.
            let mut changed_filenames = Vec::new();
            for target_filename in &target_filenames {
                if publish_fs.stat(target_filename).is_ok() {
                    continue;
                }
                changed_filenames.push(target_filename.clone());
            }
            if changed_filenames.is_empty() {
                return Ok(());
            }
            target_filenames = changed_filenames;
        }
        let mut fr = self.read_seek_closer()?;

        let mut fw =
            nh_helpers::path::open_files_for_writing(publish_fs.as_ref(), &target_filenames)?;

        let res = std::io::copy(&mut fr, &mut fw).map(|_| ());
        let close = fw.close();
        res?;
        close?;
        Ok(())
    }

    // Go: resources/resource.go:(*genericResource).isPublished
    pub(crate) fn is_published(&self) -> bool {
        self.published.get().is_some()
    }

    /// Go: `RelPermalink()` = basePath + PathEscape(TargetLink()).
    // Go: resources/resource.go:(*genericResource).RelPermalink
    pub fn rel_permalink(&self) -> String {
        format!(
            "{}{}",
            self.spec.path_spec.get_base_path(false),
            nh_common::paths::path::path_escape(&self.paths.target_link())
        )
    }

    /// Go: `Permalink()` = BaseURL.WithPathNoTrailingSlash + PathEscape(TargetPath()).
    // Go: resources/resource.go:(*genericResource).Permalink
    pub fn permalink(&self) -> String {
        format!(
            "{}{}",
            self.spec
                .path_spec
                .cfg
                .base_url()
                .with_path_no_trailing_slash,
            nh_common::paths::path::path_escape(&self.paths.target_path())
        )
    }

    // Go: resources/resource.go:(*genericResource).ResourceType
    pub fn resource_type(&self) -> String {
        self.media_type().main_type.clone()
    }

    // Go: resources/resource.go:(*genericResource).String
    pub fn string(&self) -> String {
        format!("Resource({}: {})", self.resource_type(), self.name)
    }

    /// Path is stored with Unix style slashes.
    // Go: resources/resource.go:(*genericResource).TargetPath
    pub fn target_path(&self) -> String {
        self.paths.target_path()
    }

    // Go: resources/resource.go:(*genericResource).Title
    pub fn title(&self) -> String {
        self.title.clone()
    }

    // Go: resources/resource.go:(*genericResource).getSpec
    pub fn get_spec(&self) -> &Arc<Spec> {
        &self.spec
    }

    // Go: resources/resource.go:(*genericResource).getResourcePaths
    pub(crate) fn get_resource_paths(&self) -> ResourcePaths {
        self.paths.clone()
    }

    /// Go: `tryTransformedFileCache(key, u)`. COLD-CACHE RULE: the port never reads
    /// `resources/_gen` (HUGO_LAYER.md §4.5), so nothing is ever found in the file cache (Go's
    /// behaviour with an empty `resources/_gen/assets`).
    // Go: resources/resource.go:(*genericResource).tryTransformedFileCache
    pub(crate) fn try_transformed_file_cache(
        &self,
        _key: &str,
        _u: &mut TransformationUpdate,
    ) -> Option<Box<dyn Read + Send>> {
        None
    }

    // Go: resources/resource.go:(*genericResource).mergeData
    pub(crate) fn merge_data(&mut self, input: Option<&Map>) {
        let Some(input) = input else {
            return;
        };
        if input.is_empty() {
            return;
        }
        let data = self
            .sd
            .data
            .get_or_insert_with(|| Map::new(MapType::StringAny));
        for (k, v) in &input.entries {
            if !data.entries.contains_key(k) {
                data.entries.insert(k.clone(), v.clone());
            }
        }
    }

    // Go: resources/resource.go:(*genericResource).cloneWithUpdates
    pub(crate) fn clone_with_updates(&self, u: &TransformationUpdate) -> Result<GenericResource> {
        let mut r = self.clone_resource();

        if let Some(content) = &u.content {
            r.sd.open_read_seek_closer = Some(
                nh_common::hugio::new_open_read_seek_closer_from_bytes(Arc::new(content.clone())),
            );
        }

        r.sd.media_type = Some(u.media_type.clone());

        if let Some(source_filename) = &u.source_filename {
            let Some(source_fs) = u.source_fs.clone() else {
                return Err(Error::new("sourceFs is nil"));
            };
            let name = source_filename.clone();
            r.set_open_source(Arc::new(move || -> Result<Box<dyn ReadSeekCloser>> {
                let f = source_fs.open(&name)?;
                Ok(Box::new(FileReadSeeker(f)))
            }));
        } else if u.source_fs.is_some() {
            return Err(Error::new("sourceFs is set without sourceFilename"));
        }

        if u.target_path.is_empty() {
            return Err(Error::new("missing targetPath"));
        }

        let paths = r.paths.from_target_path(&u.target_path);
        r.set_target_path(paths);
        r.merge_data(u.data.as_ref());

        Ok(r)
    }

    /// Go: `clone()` (copies the struct; the hash and staler pointers are shared, the publish and
    /// key state is fresh).
    // Go: resources/resource.go:(genericResource).clone
    pub(crate) fn clone_resource(&self) -> GenericResource {
        GenericResource {
            spec: self.spec.clone(),
            sd: self.sd.clone(),
            paths: self.paths.clone(),
            include_hash_in_key: self.include_hash_in_key,
            source_filename_is_hash: self.source_filename_is_hash,
            h: self.h.clone(),
            staler: self.staler.clone(),
            title: self.title.clone(),
            name: self.name.clone(),
            params: self.params.clone(),
            key: OnceLock::new(),
            published: OnceLock::new(),
        }
    }

    // Go: resources/resource.go:(*genericResource).openPublishFileForWriting
    pub(crate) fn open_publish_file_for_writing(
        &self,
        rel_target_path: &str,
    ) -> Result<nh_common::hugio::MultiWriteCloser> {
        let filenames = self
            .paths
            .from_target_path(rel_target_path)
            .target_filenames();
        nh_helpers::path::open_files_for_writing(self.spec.publish_fs().as_ref(), &filenames)
    }

    // Go: resources/resource.go:StaleVersion (promoted from the embedded Staler)
    pub(crate) fn stale_version(&self) -> u32 {
        self.staler.stale_version()
    }
}

/// An `afero.File` as a `hugio.ReadSeekCloser`.
struct FileReadSeeker(Box<dyn nh_hugofs::afero::File>);

impl Read for FileReadSeeker {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Seek for FileReadSeeker {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

impl Write for FileReadSeeker {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

/// Go: `resources.Copy(r, targetPath)` (Go panics when `r` is not a `resourceCopier`; the port
/// returns that as an error).
// Go: resources/resource.go:Copy
pub fn copy(r: &Arc<dyn Resource>, target_path: &str) -> Result<Arc<dyn Resource>> {
    match r
        .clone()
        .as_any_arc()
        .downcast::<crate::transform::ResourceAdapter>()
    {
        Ok(a) => Ok(a.clone_to(target_path)),
        Err(_) => Err(Error::new(format!(
            "interface conversion: {} is not resources.resourceCopier: missing method cloneTo",
            r.tpl_type_name()
        ))),
    }
}

/// Go: `commonResource.Slice(in)` — for the template function `slice` (`collections.Slice`):
/// a `resource.Resources` is returned as is, a `[]interface {}` of resources becomes a
/// `resource.Resources`, anything else is an error.
// Go: resources/resource.go:(commonResource).Slice
pub fn common_resource_slice(input: &Value) -> Result<Value> {
    match input {
        Value::List(l) if &*l.ty.go_name() == nh_resource::resourcetypes::RESOURCES_TYPE => {
            Ok(input.clone())
        }
        Value::TypedNil(t) if &**t == nh_resource::resourcetypes::RESOURCES_TYPE => {
            Ok(input.clone())
        }
        Value::List(l) if matches!(l.ty, go_value::SliceType::Any) => {
            let mut groups: Vec<Value> = Vec::with_capacity(l.items.len());
            for v in &l.items {
                if nh_resource::resourcetypes::resource_from_value_any(v).is_none() {
                    return Err(Error::new(format!(
                        "type {} is not a Resource",
                        go_type_of(v)
                    )));
                }
                groups.push(v.clone());
            }
            Ok(Value::list(
                go_value::SliceType::Named(Arc::from(nh_resource::resourcetypes::RESOURCES_TYPE)),
                groups,
            ))
        }
        _ => Err(Error::new(format!(
            "invalid slice type {}",
            go_type_of(input)
        ))),
    }
}

/// Go's `%T` of a value (`<nil>` for an untyped nil).
fn go_type_of(v: &Value) -> String {
    match v {
        Value::Invalid => "<nil>".to_string(),
        v => v.go_type_name().into_owned(),
    }
}

/// Go: `InternalResourceTargetPath(r)` (Go panics when `r` is not a `targetPathProvider`).
// Go: resources/resource.go:InternalResourceTargetPath
pub fn internal_resource_target_path(r: &Arc<dyn Resource>) -> Option<String> {
    r.as_any()
        .downcast_ref::<crate::transform::ResourceAdapter>()
        .map(|a| a.target_path_internal())
}

/// InternalResourceSourcePath is used internally to get the source path for a Resource. It
/// returns an empty string if the source path is not available.
// Go: resources/resource.go:InternalResourceSourcePath
pub fn internal_resource_source_path(r: &Arc<dyn Resource>) -> String {
    if let Some(a) = r
        .as_any()
        .downcast_ref::<crate::transform::ResourceAdapter>()
    {
        let p = a.source_path();
        if !p.is_empty() {
            return p;
        }
    }
    String::new()
}

/// InternalResourceSourcePathBestEffort is used internally to get the source path for a
/// Resource. Used for error messages etc. It will fall back to the target path if the source
/// path is not available.
// Go: resources/resource.go:InternalResourceSourcePathBestEffort
pub fn internal_resource_source_path_best_effort(r: &Arc<dyn Resource>) -> String {
    let s = internal_resource_source_path(r);
    if !s.is_empty() {
        return s;
    }
    internal_resource_target_path(r).unwrap_or_default()
}

/// isPublished returns true if the resource is published (false for resources that are not
/// `isPublishedProvider`s; Go panics there).
// Go: resources/resource.go:IsPublished
pub fn is_published(r: &Arc<dyn Resource>) -> bool {
    r.as_any()
        .downcast_ref::<crate::transform::ResourceAdapter>()
        .is_some_and(|a| a.is_published())
}

/// Go: `GenericResourceTestInfo`.
pub struct GenericResourceTestInfo {
    pub paths: ResourcePaths,
}

/// For internal use (tests).
// Go: resources/resource.go:GetTestInfoForResource
pub fn get_test_info_for_resource(r: &Arc<dyn Resource>) -> Option<GenericResourceTestInfo> {
    let a = r
        .as_any()
        .downcast_ref::<crate::transform::ResourceAdapter>()?;
    match a.target() {
        crate::transform::BaseResource::Generic(g) => Some(GenericResourceTestInfo {
            paths: g.paths.clone(),
        }),
        crate::transform::BaseResource::Image(_) => None,
    }
}

/// Go: `NewFeatureNotAvailableTransformer(key, elements...)` — a transformation whose
/// `Transform` fails with `herrors.ErrFeatureNotAvailable`.
// Go: resources/resource.go:NewFeatureNotAvailableTransformer
pub fn new_feature_not_available_transformer(
    key: &str,
    elements: Vec<Value>,
) -> Arc<dyn crate::transform::ResourceTransformation> {
    Arc::new(TransformerNotAvailable {
        key: nh_resource::internal::key::ResourceTransformationKey::new(key, elements),
    })
}

/// Go: `transformerNotAvailable`.
struct TransformerNotAvailable {
    key: nh_resource::internal::key::ResourceTransformationKey,
}

impl crate::transform::ResourceTransformation for TransformerNotAvailable {
    // Go: resources/resource.go:(transformerNotAvailable).Key
    fn key(&self) -> nh_resource::internal::key::ResourceTransformationKey {
        self.key.clone()
    }

    // Go: resources/resource.go:(transformerNotAvailable).Transform
    fn transform(&self, _ctx: &mut crate::transform::ResourceTransformationCtx<'_>) -> Result<()> {
        Err(nh_common::herrors::err_feature_not_available())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource.go (733 lines; 32/53 funcs executed)
//   types: ResourceSourceDescriptor, ResourceTransformer, Transformer, transformerNotAvailable, resourceCopier,
//          baseResourceResource, baseResourceInternal, specProvider, baseResource, commonResource, fileInfo,
//          hashProvider, StaleValue[V, AtomicStaler, GenericResourceTestInfo, genericResource, targetPather,
//          isPublishedProvider, resourceHash, targetPathProvider, sourcePathProvider
// OK L109-192: (fd *ResourceSourceDescriptor) init(r *Spec) error
// OK L204-208: NewFeatureNotAvailableTransformer(key string, elements ...any) ResourceTransformation
// OK L214-216: (t transformerNotAvailable) Transform(ctx *ResourceTransformationCtx) error
// OK L218-220: (t transformerNotAvailable) Key() internal.ResourceTransformationKey
// OK L228-230: Copy(r resource.Resource, targetPath string) resource.Resource
// OK L278-297: (commonResource) Slice(in any) (any, error)
// OK L322-324: (s *StaleValue[V]) StaleVersion() uint32
// OK L330-332: (s *AtomicStaler) MarkStale()
// OK L334-336: (s *AtomicStaler) StaleVersion() uint32
// OK L344-357: GetTestInfoForResource(r resource.Resource) GenericResourceTestInfo
// OK L383-385: (l *genericResource) IdentifierBase() string
// OK L387-389: (l *genericResource) GetIdentityGroup() identity.Identity (identity not ported)
// OK L391-393: (l *genericResource) GetDependencyManager() identity.Manager (identity not ported)
// OK L395-397: (l *genericResource) ReadSeekCloser() (hugio.ReadSeekCloser, error)
// OK L399-401: (l *genericResource) Clone() resource.Resource
// OK L403-406: (l *genericResource) size() int64
// OK L408-413: (l *genericResource) hash() uint64
// OK L415-417: (l *genericResource) setOpenSource(openSource hugio.OpenReadSeekCloser)
// OK L419-421: (l *genericResource) setSourceFilenameIsHash(b bool)
// OK L423-425: (l *genericResource) setTargetPath(d internal.ResourcePaths)
// OK L427-431: (l *genericResource) cloneTo(targetPath string) resource.Resource
// OK L433-441: (l *genericResource) Content(context.Context) (any, error)
// OK L443-445: (l *genericResource) Data() any
// OK L447-466: (l *genericResource) Key() string
// OK L468-470: (l *genericResource) TransientKey() string
// OK L472-474: (l *genericResource) targetPath() string
// OK L476-481: (l *genericResource) sourcePath() string
// OK L483-485: (l *genericResource) MediaType() media.Type
// OK L487-489: (l *genericResource) setMediaType(mediaType media.Type)
// OK L491-493: (l *genericResource) Name() string
// OK L495-497: (l *genericResource) NameNormalized() string
// OK L499-501: (l *genericResource) Params() maps.Params
// OK L503-540: (l *genericResource) Publish() error
// OK L542-544: (l *genericResource) isPublished() bool
// OK L546-548: (l *genericResource) RelPermalink() string
// OK L550-552: (l *genericResource) Permalink() string
// OK L554-556: (l *genericResource) ResourceType() string
// OK L558-560: (l *genericResource) String() string
// OK L563-565: (l *genericResource) TargetPath() string
// OK L567-569: (l *genericResource) Title() string
// OK L571-573: (l *genericResource) getSpec() *Spec
// OK L575-577: (l *genericResource) getResourcePaths() internal.ResourcePaths
// OK L579-590: (r *genericResource) tryTransformedFileCache(key string, u *transformationUpdate) io.ReadCloser (cold: never found)
// OK L592-604: (r *genericResource) mergeData(in map[string]any)
// OK L606-636: (rc *genericResource) cloneWithUpdates(u *transformationUpdate) (baseResource, error)
// OK L638-642: (l genericResource) clone() *genericResource
// OK L644-647: (r *genericResource) openPublishFileForWriting(relTargetPath string) (io.WriteCloser, error)
// OK L663-684: (r *resourceHash) init(l hugio.ReadSeekCloserProvider) error
// OK L686-688: hashImage(r io.ReadSeeker) (uint64, int64, error)
// OK L691-693: InternalResourceTargetPath(r resource.Resource) string
// OK L697-704: InternalResourceSourcePath(r resource.Resource) string
// OK L709-714: InternalResourceSourcePathBestEffort(r resource.Resource) string
// OK L717-719: IsPublished(r resource.Resource) bool
// ---------------------------------------------------------------------------
