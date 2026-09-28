//! Port of `resources/resource_factories/create/create.go`.
//!
//! Owner: Wave B task T15 (resource-factories).

//! Go `resources/resource_factories/create`: `resources.Get` (assets fs, LazyPublish, cache key
//! `cleanKey(path)+"__get"`, missing -> nil), `resources.FromString`, `resources.GetMatch/Match`,
//! `resources.Copy`, and the GetRemote client (remote.go).
//!
//! Go caches nil results (a missing `Get`, a 404 `GetRemote`) in the resource cache; the Rust
//! cache holds resources only, so a nil result is not cached and is computed again on the next
//! call (the same result: the assets file system and the getresource file cache do not change
//! during a build).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::hugio::{OpenReadSeekCloser, ReadSeekCloser};
use nh_helpers::cache::filecache::filecache::Cache as FileCache;
use nh_helpers::cache::httpcache::httpcache::ConfigCompiled as HttpCacheConfig;
use nh_helpers::cache::httpcache::transport::Transport;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_resource::resourcetypes::{Resource, Resources};
use nh_resources::resource::ResourceSourceDescriptor;
use nh_resources::resource_spec::Spec;

/// Go: `create.Client` — methods to create Resource objects.
pub struct Client {
    pub rs: Arc<Spec>,
    /// GetRemote HTTP client over the getresource file cache (Go `httpClient.Transport`; the
    /// per-request cache key is set by `FromRemote`, Go's `resourceIDDispatcher`).
    pub http_client: Arc<Transport>,
    /// Go `httpCacheConfig`.
    pub http_cache_config: HttpCacheConfig,
    /// Go `cacheGetResource`.
    pub cache_get_resource: Arc<FileCache>,
}

/// The value a nil resource result travels as through the resource cache (never stored).
const NIL_RESOURCE: &str = "\u{0}neohugo-rs: nil resource";

pub(crate) fn nil_resource() -> Error {
    Error::new(NIL_RESOURCE)
}

/// `Ok(None)` for a nil result that came back through the cache as [`nil_resource`].
pub(crate) fn nil_to_none(r: Result<Arc<dyn Resource>>) -> Result<Option<Arc<dyn Resource>>> {
    match r {
        Ok(r) => Ok(Some(r)),
        Err(e) if e.message() == NIL_RESOURCE => Ok(None),
        Err(e) => Err(e),
    }
}

/// Go: `create.Options`.
pub struct Options {
    /// The target path relative to the publish directory. Unix style path, i.e.
    /// "images/logo.png".
    pub target_path: String,
    /// Whether the TargetPath has a hash in it which will change if the resource changes. If
    /// not, we will calculate a hash from the content.
    pub target_path_has_hash: bool,
    /// The content to create the Resource from.
    pub create_content: Arc<dyn Fn() -> Result<OpenReadSeekCloser> + Send + Sync>,
}

impl Client {
    /// New creates a new Client with the given specification.
    // Go: resources/resource_factories/create/create.go:New
    pub fn new(rs: Arc<Spec>) -> Result<Arc<Client>> {
        let file_cache = rs
            .common
            .file_caches
            .get("getresource")
            .ok_or_else(|| Error::new("the getresource file cache is not configured"))?;
        let http_cache_config = (*nh_config::config_provider::config_section::<HttpCacheConfig>(
            rs.path_spec.cfg.as_ref(),
            "httpCacheCompiled",
        ))
        .clone();
        if rs.path_spec.cfg.watching() && !http_cache_config.is_polling_disabled() {
            // Go starts a `tasks.RunEvery` remote resource checker (server/watch mode only).
            return Err(Error::new(
                "neohugo-rs: polling remote resources (watch mode) is not supported",
            ));
        }

        // (Go's http.Client timeout, max(2m, timeout+30s), bounds network requests; the port
        // never goes to the network.)

        let mut t = Transport::new(file_cache.as_http_cache());
        let fc = file_cache.clone();
        t.around = Some(Arc::new(move |_req, key: &str| {
            Box::new(fc.named_lock(key)) as Box<dyn std::any::Any>
        }));
        let hc = http_cache_config.clone();
        t.always_use_cached_response = Some(Arc::new(move |req, _key| !(hc.for_)(&req.url)));
        t.should_cache = Some(Arc::new(|_req, resp, _key| {
            super::remote::should_cache(resp.status_code)
        }));
        t.mark_cached_responses = true;
        t.enable_etag_pair = true;
        // Transport: the inner transport (Go `create.transport` with its retries over
        // `http.DefaultTransport`) is the network: `None` = `NoNetwork`.

        Ok(Arc::new(Client {
            rs,
            http_client: Arc::new(t),
            http_cache_config,
            cache_get_resource: file_cache,
        }))
    }

    /// Go: `Client.Get(pathname)` — `None` when the file does not exist.
    // Go: resources/resource_factories/create/create.go:Get
    pub fn get(&self, pathname: &str) -> Result<Option<Arc<dyn Resource>>> {
        let pathname = go_path::path::clean(pathname).to_string();
        let key = format!("{}__get", nh_common::dynacache::clean_key(&pathname));

        nil_to_none(self.rs.resource_cache().get_or_create(&key, || {
            // The resource file will not be read before it gets used (e.g. in .Content), so we
            // need to check that the file exists here.
            let filename = go_path::filepath::from_slash(&pathname);
            let fi = match self.assets_fs().stat(filename) {
                Ok(fi) => fi,
                Err(err) => {
                    if err.is_not_exist() {
                        return Err(nil_resource());
                    }
                    // A real error.
                    return Err(err);
                }
            };

            self.get_or_create_file_resource(&fi)
        }))
    }

    /// Go `c.rs.Assets.Fs`.
    fn assets_fs(&self) -> Arc<dyn nh_hugofs::afero::Fs> {
        self.rs
            .path_spec
            .base_fs
            .source_filesystems
            .assets
            .fs
            .clone()
    }

    /// Match gets the resources matching the given pattern from the assets filesystem (an
    /// empty list is Go's nil `resource.Resources`).
    // Go: resources/resource_factories/create/create.go:Match
    pub fn match_(&self, pattern: &str) -> Result<Resources> {
        self.do_match("__match", pattern, None, false)
    }

    /// Go: `ByType(tp)` (Go panics on an error; returned here).
    // Go: resources/resource_factories/create/create.go:ByType
    pub fn by_type(&self, tp: &str) -> Result<Resources> {
        let tp2 = tp.to_string();
        let match_func = move |r: &Arc<dyn Resource>| r.resource_type() == tp2;
        self.do_match(
            &go_path::path::join(&["_byType", tp]),
            "**",
            Some(&match_func),
            false,
        )
    }

    /// GetMatch gets first resource matching the given pattern from the assets filesystem.
    // Go: resources/resource_factories/create/create.go:GetMatch
    pub fn get_match(&self, pattern: &str) -> Result<Option<Arc<dyn Resource>>> {
        let res = self.do_match("__get-match", pattern, None, true)?;
        Ok(res.into_iter().next())
    }

    // Go: resources/resource_factories/create/create.go:getOrCreateFileResource
    fn get_or_create_file_resource(&self, info: &FileMetaInfo) -> Result<Arc<dyn Resource>> {
        let meta = info.meta.clone();
        let filename = go_path::filepath::to_slash(&meta.filename).to_string();
        self.rs.resource_cache().get_or_create_file(&filename, || {
            let path_info = meta.path_info.clone().ok_or_else(|| {
                Error::new("runtime error: invalid memory address or nil pointer dereference")
            })?;
            let m = meta.clone();
            let open: OpenReadSeekCloser = Arc::new(move || {
                let f = m.open()?;
                Ok(Box::new(f) as Box<dyn ReadSeekCloser>)
            });
            self.rs.new_resource(ResourceSourceDescriptor {
                lazy_publish: true,
                open_read_seek_closer: Some(open),
                name_normalized: path_info.path().to_string(),
                name_original: path_info.unnormalized().path().to_string(),
                target_path: path_info.unnormalized().path().to_string(),
                source_filename_or_path: meta.filename.clone(),
                ..Default::default()
            })
        })
    }

    // Go: resources/resource_factories/create/create.go:match
    fn do_match(
        &self,
        name: &str,
        pattern: &str,
        match_func: Option<&dyn Fn(&Arc<dyn Resource>) -> bool>,
        first_only: bool,
    ) -> Result<Resources> {
        let pattern = nh_common::glob::glob::normalize_path(pattern);
        let partitions = nh_common::glob::glob::filter_glob_parts(
            pattern.split('/').map(str::to_string).collect(),
        );
        let partitions: Vec<&str> = partitions.iter().map(String::as_str).collect();
        let key = go_path::path::join(&[name, &go_path::path::join(&partitions)]);
        let key = go_path::path::join(&[&key, &pattern]);

        self.rs.resource_cache().get_or_create_resources(&key, || {
            let mut res: Resources = Vec::new();

            let mut handle = |info: &FileMetaInfo| -> Result<bool> {
                let r = self.get_or_create_file_resource(info)?;

                if let Some(f) = match_func
                    && !f(&r)
                {
                    return Ok(false);
                }

                res.push(r);

                Ok(first_only)
            };

            nh_hugofs::glob::glob(self.assets_fs(), &pattern, &mut handle)?;

            Ok(res)
        })
    }

    /// FromOpts creates a new Resource from the given Options. Make sure to set
    /// `target_path_has_hash` if the TargetPath already contains a hash, as this avoids the need
    /// to calculate it.
    // Go: resources/resource_factories/create/create.go:FromOpts
    pub fn from_opts(&self, opts: Options) -> Result<Arc<dyn Resource>> {
        let target_path = go_path::path::clean(&opts.target_path).to_string();
        let mut hash = String::new();
        let mut new_read_seeker: Option<OpenReadSeekCloser> = None;
        if !opts.target_path_has_hash {
            let nrs = (opts.create_content)()?;
            let mut r = nrs()?;
            hash = nh_common::hashing::xxhash_from_reader_hex_encoded(&mut r)?;
            new_read_seeker = Some(nrs);
        }

        let key = format!("{}{}", nh_common::dynacache::clean_key(&target_path), hash);
        self.rs.resource_cache().get_or_create(&key, || {
            let nrs = match new_read_seeker {
                Some(n) => n,
                None => (opts.create_content)()?,
            };
            self.rs.new_resource(ResourceSourceDescriptor {
                lazy_publish: true,
                open_read_seek_closer: Some(nrs),
                target_path: target_path.clone(),
                ..Default::default()
            })
        })
    }

    // Go: resources/resource_factories/create/create.go:FromString
    pub fn from_string(&self, target_path: &str, content: &[u8]) -> Result<Arc<dyn Resource>> {
        let content = content.to_vec();
        self.from_opts(Options {
            target_path: target_path.to_string(),
            target_path_has_hash: false,
            create_content: Arc::new(move || {
                let content = content.clone();
                let open: OpenReadSeekCloser =
                    Arc::new(move || Ok(nh_common::hugio::read_seeker_from_bytes(content.clone())));
                Ok(open)
            }),
        })
    }

    /// Copy copies r to the new targetPath.
    // Go: resources/resource_factories/create/create.go:Copy
    pub fn copy(&self, r: &Arc<dyn Resource>, target_path: &str) -> Result<Arc<dyn Resource>> {
        let key = format!("{}__copy", nh_common::dynacache::clean_key(target_path));
        self.rs
            .resource_cache()
            .get_or_create(&key, || nh_resources::resource::copy(r, target_path))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_factories/create/create.go (304 lines; 3/10 funcs executed)
//   types: Client, contextKey, Options
// OK L65-122: New(rs *resources.Spec) *Client
// OK L125-130: (c *Client) Copy(r resource.Resource, targetPath string) (resource.Resource, error)
// OK L133-152: (c *Client) Get(pathname string) (resource.Resource, error)
// OK L155-157: (c *Client) Match(pattern string) (resource.Resources, error)
// OK L159-165: (c *Client) ByType(tp string) resource.Resources
// OK L168-174: (c *Client) GetMatch(pattern string) (resource.Resource, error)
// OK L176-191: (c *Client) getOrCreateFileResource(info hugofs.FileMetaInfo) (resource.Resource, error)
// OK L193-223: (c *Client) match(name, pattern string, matchFunc func(r resource.Resource) bool, firstOnly bool) (resource.Resources, error)
// OK L244-292: (c *Client) FromOpts(opts Options) (resource.Resource, error)
// OK L295-304: (c *Client) FromString(targetPath, content string) (resource.Resource, error)
// ---------------------------------------------------------------------------
