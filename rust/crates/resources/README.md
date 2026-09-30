# neohugo-resources

The `ResourceStore` of a build (REWRITE_PLAN.md §2.1, §2.4, §3.4; task T40): bundle, asset,
remote and named-target resources, the fingerprint transform, front matter `resources`
metadata and the `Resources` lookups, and publishing by URL token. The pipes (`to_css`,
PostCSS, Tailwind, Babel, `js_build`, minify, `post_process`, `execute_as_template`) are T42's
(`src/pipes/`).

## API

| Item | What it is |
|---|---|
| `ResourceStore::new(StoreConfig)` | `StoreConfig::from_config(&Config, Option<Arc<Vfs>>, Option<Arc<ImageQueue>>)`: per-language base URL and target prefix (`/<lang>` on multihost sites), media types, the assets view, the image queue, `RemoteConfig`. |
| `Resource` | Immutable: `id`, `kind` (`Image` = processable raster, `Page`, `Text`, `Other`), `origin`, `media_type` (empty when the extension is unknown), `name`, `name_normalized`, `title`, `params`, `data`, `lang`, `target: OutputPath`, `link: UrlPath` (unescaped, without base path), `rel_permalink` (escaped, with the base path), `permalink`, `body` (`File`, `Bytes`, `Generated`, `PendingImage`), `policy` (`Eager`, `OnReference`, `Never`). `resource_type()`, `media_type_string()`, `integrity()`. |
| `resource(id)`, `resources()`, `content(id)` | Lookup; `.Content` (reads the file, or asks the image queue). |
| `get_asset(lang, path)`, `find_assets(lang, glob)`, `find_asset` | `resources.Get` / `Match` / `GetMatch` over the assets view. Assets are one resource per path (per language on multihost sites). |
| `register_bundle(&BundleResource)` | A page bundle file (`lang`, `file`, `name` relative to the bundle, link `dir`, `policy`); one resource per target. |
| `from_string`, `from_template_output`, `concat`, `copy` (target, …, `&CallSite`) | Resources that name a target path. `CallSite { lang, position }`. |
| `transform(id, Transform::Fingerprint(HashAlgo))` | `fingerprint` (md5, sha256, sha384, sha512; `HashAlgo::from_str`, `""` = sha256): `.<hex>` before the extension, `Data.Integrity` = SRI. Memoized per `(id, transform)`. |
| `meta::ResourceMeta::parse(&Value)`, `apply_meta(id, &meta)` | Front matter `resources` metadata; a new resource (same target and content) only when name, title or params change. |
| `meta::{get, get_match, matches, by_type}` over `impl Named` | `.Resources.Get/GetMatch/Match/ByType` (bundle pages implement `Named` too). |
| `image_input(id)`, `register_image(from, &Enqueued)` | The seam to `neohugo-images`: what `ImageQueue::enqueue` reads, and the resource of a queued operation (sibling target `<stem>_hu_<hash>.<ext>`, `Body::PendingImage`). The store never decodes images. |
| `inject_generated(asset_path, bytes)` | Build phase E4: `hugo_stats.json` (or any asset path) now reads these bytes; already registered assets at that path see them too. |
| `mark_published(id)` | The `publish` filter. |
| `publish(tokens, &dyn Sink) -> PublishStats` | See below. `resolve_token(token)` for diagnostics. |
| `get_remote(lang, url, &RemoteOptions)` | `resources.GetRemote`; `Ok(None)` for a 404. `RemoteOptions::from_map(Option<&Map>)`; `RemoteError`; `cache_key`, `hugo_keys`. |

## Identity

- Assets memoize on the cleaned path (and the language on multihost sites), bundles on their
  target, transforms on `(source, transform)`, metadata views on `(source, name, title,
  params)`, processed images on `(source, operation)`, remote resources on the typed request.
- Named targets (`from_string`, `from_template_output`, `concat`, `copy`) memoize on the target
  path. A second claim with the same input returns the first resource. With other input: in
  the same language it is `ResourceError::TargetConflict { target, first, second }` naming
  both call sites; in another language the earlier language's resource is returned (language
  sub-waves render in order). On multihost sites every language has its own target, so nothing
  is shared.

## Publishing

`publish` writes what was not written yet, one writer per target (the lowest id), in target
order: `Eager` resources, resources marked with `mark_published`, and resources named by a URL
token (never `Never`). Tokens are canonicalised before lookup: HTML character references
(`&amp;`, `&#39;`, `&#x27;`) and JSON escapes (`\/`, `&`) decoded, query and fragment
dropped, percent-escapes decoded (either case), scheme dropped and host lower-cased for
absolute and protocol-relative URLs. Each resource is indexed under its relative permalink and
its permalink in the same form. Processed images go to `ImageQueue::process` (rayon) in one
batch.

## GetRemote and the getresource cache

- The request is typed (`RemoteOptions`: method, headers, body, `key`, `responseHeaders`);
  option maps that differ only in key order or key case are the same request.
- Cache entries (`[caches.getresource]`, `maxAge`) are raw HTTP responses, named by
  `cache_key` (xxh3-128 of the `key` option, else of the request).
- **Importer**: on a miss, the entry Hugo would have written for the call (named by
  `hugo_keys`, Hugo's hashstructure/xxHash64 of `[url, options]` or of `key`) is looked up in
  the cache directory itself and in `RemoteConfig::import_dirs`, and copied under this crate's
  name. A `HUGO_CACHEDIR` filled by the Go build (the 51 golden YouTube responses of seeksnack,
  via `sites.py cache`) is thereby replayed offline.
- Only then, if `RemoteConfig::network`, the URL is fetched with `ureq` (no retries); the
  response is cached unless it is a redirect or `maxAge` is 0.
- Security: `security.http.urls` and `.methods` are checked; a `Content-Type` matching
  `.mediaTypes` (or any, for HEAD) is trusted, otherwise the media type is sniffed from the
  content and narrowed by the extensions of the content type and the file name
  (`Content-Disposition` `filename`/`filename*` first).
- The resource is named `/<file stem>_<Hugo user key><suffix>` like Hugo's, so its URLs are the
  Go build's.

## Accepted deviations

Listed with their reasons in `expected_diffs.toml` (the tests read it):

- Front matter params are not written into a shared base resource (Go's aliasing makes a
  translation see the other language's params).
- `resources.Copy` of a different source to an already copied target in the same language is
  a `TargetConflict`; Go returns the first copy because its cache key ignores the source.
- Remote: equivalent option maps are one request (`echo-post-upper`); a HEAD response's
  parameterised `Content-Type` resolves to the configured type and suffix (`head`); a trusted
  vendor type drops its parameters (`vnd-accepted`).
- Go's resource `Key` is not modelled; image sizes come from the image queue.
- `:counter` in metadata names and titles is always `1`, as in Hugo, which applies metadata to
  one resource at a time.
- Error texts are this crate's own.

## Tests (`tests/it`)

| Test | Oracle | Cases |
|---|---|---|
| `collection` | `tests/it/data/collection.json` (Get/GetMatch/Match/ByType of the old `nh-resource` oracle, 5 sets) | 445 |
| `resources::{synth,docs}` | `oracle/resources/resources` (assets, bundles, metadata, publishing) | 43 + 89 records, 34 + 89 published files |
| `fingerprint` | `oracle/resources/transform` (fingerprint chains, SRI, copy) | 144 equal to Go, 28 sha1 errors, 52 copy conflicts |
| `remote` | `oracle/resource-transformers/getremote` (key vectors, 48 calls from the Hugo cache, 51 YouTube entries) | 48 + 48 + 51 |
| `identity`, `publish` | — | named targets, concat, metadata, `inject_generated`, image seam, multihost, token forms |
