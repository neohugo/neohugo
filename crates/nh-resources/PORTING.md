# nh-resources — porting notes

neohugo `resources/*.go` (Spec, genericResource, resourceAdapter + transformation chain, caches,
image resource), `resources/postpub`, `resources/jsconfig`. Owner and crate lead: Wave B task
T14 (resources-core).

## Go file → Rust module

| Rust module | Go source(s) | Status |
|---|---|---|
| `resource` | `resources/resource.go` | ported (all 53 functions) |
| `resource_spec` | `resources/resource_spec.go` | ported (all 7) |
| `transform` | `resources/transform.go` | ported (all 56); the identity methods (`GetIdentity`, `GetIdentityGroup`, `GetDependencyManager`, `ForEeachIdentityByName`) are explicit errors in the template table |
| `resource_cache` | `resources/resource_cache.go` | ported (all 9); the file cache is never read (cold-cache rule) |
| `resource_metadata` | `resources/resource_metadata.go` | ported (all 10) |
| `image` | `resources/image.go` | ported (all 21); `Colors` is a STUB (explicit error) |
| `image_cache` | `resources/image_cache.go` | ported (both) |
| `post_publish` | `resources/post_publish.go` | ported |
| `postpub::postpub` | `resources/postpub/postpub.go` | ported (all 15) |
| `postpub::fields` | `resources/postpub/fields.go` | ported (all 3); `structToMap`'s reflection over `media.Type` is a table |
| `jsconfig` | `resources/jsconfig/jsconfig.go` | ported (all 4) |
| `mime` (private) | go1.27.1 `mime/type.go`, `type_unix.go`, `mediatype.go`, `grammar.go` | the parts `TypeByExtension` needs (the fallback of `ResourceSourceDescriptor.init`) |

Every GO PORTING CHECKLIST entry is `OK`; every ported function carries a `// Go:` line.

### Stubs

| feature | error | why |
|---|---|---|
| `.Colors` (`github.com/marekm4/color-extractor`) | `neohugo-rs: image Colors (color-extractor) is not supported` (`FeatureNotAvailable`) | not on the seeksnack path; a histogram library with its own float code |
| the identity methods of `*resources.resourceAdapter` from a template | `neohugo-rs: resourceAdapter.<M> (identity tracking) is not supported` | the identity system (server rebuilds) is not ported |
| `DecodeImage`, `ReadSeekCloser`, `Transform`, `TransformWithContext`, `WithResourceMeta` from a template | explicit errors | Go would return values templates cannot use |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-resource, nh-images, nh-page,
  nh-allconfig, nh-tpl.
- Wave A: go-value, go-hashstructure, go-json, go-image, go-path, go-strconv, go-unicode, gift.
- crates.io: none.
- dev: `serde_json`, `flate2` (`rust_backend`, gunzip of the fixtures), `sha2`, `md-5`,
  `base64` (the test port of integrity's fingerprint; digests and encodings cannot change
  output bytes), nh-transform (T07's minifier client for the test port of the `minify`
  transformation), nh-langs.

## Public API (changes to the skeleton; none was used by another crate)

- `Spec::new(ps, common, file_caches, mem_cache, incr, exec, logger)`: Go's `memCache` (the
  build's `dynacache.Cache`, shared by all sites: the resource partitions `/res1`, `/res2`,
  `/resr`, `/ress`, `/res1/tra` and the image partitions `/imgs`, `/imgs/enc` come from it) and
  the logger are parameters. `Spec` gained `logger`, `error_sender` (Go `ErrorSender`; unset =
  log like Go's nil sender), `publish_fs()`, `lang()`, `multihost_target_base_paths()`,
  `resource_cache()`, `post_process(r)`, `new_resource_adapter(&mut rd)` (the concrete adapter).
- `ResourceCache` / `ImageCache` fields are `Arc<Partition<..>>` from the shared memory cache;
  `ResourceCache::new_with`, `ImageCache::new_with`; `ImageCache.encoded` holds the ENCODED bytes
  of every processed image (key = Go's file cache id `cleanID(relTargetPath)`;
  `encoded_bytes(rel_target_path)`).
- `ResourceTransformationCtx`: `from` is a `TransformFrom` (Go's `io.Reader`, with
  `as_read_seeker()` for Go's `ctx.From.(io.ReadSeeker)` check that integrity's fingerprint
  does, and `read_all()`); `to` is the chain buffer (`&mut Vec<u8>`); `data` is a `Map`;
  `open_resource_publisher` gives a `MultiWriteCloser`; `publish_source_map`.
- `ResourceAdapter`: all Go methods (`content(ctx)`, `data`, `key`, `rel_permalink`, `permalink`,
  `resize`/`fit`/`fill`/`crop`/`process`/`filter`, `width`/`height` (`Result`: Go panics for
  non-image resources), `transform`, `transform_with_context`, `with_resource_meta`, `clone_to`,
  `transformation_key`, `decode_image`, `exif`, …), `impl nh_resource::Resource` with the
  `*resources.resourceAdapter` method table (`go_methods!`, including `Slice`), and
  `impl nh_images::ImageSource` (the overlay/mask source decodes the ENCODED bytes).
- `transform::image_source_from_value(&Value)` (for T19's `images.Overlay`/`images.Mask`),
  `transform::resource_adapter(&Arc<dyn Resource>)`.
- `resource::common_resource_slice(&Value)` (Go `commonResource.Slice`),
  `resource::new_feature_not_available_transformer`, `resource::copy`,
  `image::rel_target_path_for_hash` (the `_hu_` name with an explicit root hash),
  `image::ExifInfoObject` (`*exif.ExifInfo` as a template value).
- `resource_metadata::{SharedParams, MetaResource, clone_with_metadata_from_map_if_needed,
  clone_with_metadata_from_resource_config_if_needed, assign_metadata_to}`;
  `assign_metadata(metadata, &mut resources)` is what hugolib does per page resource.
- `postpub::postpub::PostPublishResource::new(id: i64, r)`, `PostPublishRef` (the template
  object `*postpub.PostPublishResource`; it is not a `resource.Resource`, as in Go, where its
  `MediaType` returns a map), `post_publish_from_value`.
- `jsconfig::Builder::build(dir)` returns the bytes hugolib writes
  (`json.MarshalIndent(cfg, "", " ")`); `build_config(dir)` the struct.

## Deliberate deviations

1. **Cold cache: `resources/_gen` is never read** (HUGO_LAYER.md §4.5, images.md §9.1). Go reads
   processed images and cached transformation results from its file caches; the port never
   does: `ImageCache` takes Go's create path (so `Key()` carries the root hash, the golden
   build's cold naming) unless this build already created the file under another memory key
   (multihost languages; see below), `tryTransformedFileCache`/`getFromFile` find nothing (Go's
   behaviour with an empty cache: a `tocss`/`postcss` step that is not available fails, with
   Go's message), and processed images read their ENCODED bytes from `ImageCache.encoded`
   (`DecodeImage` of every `Resize`/`Filter` step and of the overlay source). Writing
   `resources/_gen` is off: the processed images and the `<key>.json`/`<key>.content` of
   `tocss`/`postcss` chains are not written (nothing reads them); the transformation content
   stays in memory instead of being re-read from the cache file (same bytes).
2. **No image-processing semaphore.** Go serialises image creation (`imageProcSem`, one
   worker); it has no effect on the bytes, and the port computes nothing under a lock (cache
   values are computed outside the lock, first writer wins, HUGO_LAYER.md §4.8).
3. **Go panics become errors** (README rule 9) where the API can return one: a nil opener or
   empty target path in `init`, `getImageOps` on non-image resources (Go's messages, including
   the SVG one), `Copy`/`PostProcess` of a non-adapter, `GetFieldString` of an unknown accessor
   or a pattern without the suffix, `PostPublishResource.Params`, the Exif `metadata init
   failed`. `genericResource.hash()` still panics like Go (it is called from `Key()`).
4. **`resourceHash.init`** returns its error on every call (Go only on the first; the value is
   unusable either way, Go panics in `hash()`). The size uses the `ReadFrom` rule of
   go-hashstructure (Go's size depends on the reader type; it only names Exif cache files,
   which the port never uses).
5. **Exif** is decoded directly (Go's create path of the images file cache); a decode error is
   logged as in Go.
6. **Identity/dependency tracking** (`GroupIdentity`, `DependencyManager`, the identity methods)
   is not ported (server rebuilds only).
7. **Error text formatting.** Go builds the transformation error with
   `fmt.Errorf(msg+": %w", err)`, so a `%` in the target path would be read as a verb; the port
   formats the path literally.
8. **`mime.TypeByExtension`** is ported with its built-in table and the unix system databases
   (`/usr/share/mime/globs2`, or `/etc/mime.types`, `/etc/apache2/mime.types`, …), read like Go
   reads them: the result depends on the machine, as in Go (the darwin golden build reads
   `/etc/apache2/mime.types`). Only extensions without a Hugo media type reach it.
9. **Sharing.** Go copies the adapter struct (`TransformWithContext`, `cloneTo`,
   `WithResourceMeta`) and shares pointers: the copies share the `*resourceTransformations`
   (the `sync.Once` and its error), and an inner whose target the first transformation
   replaces in place. The port keeps the same sharing (`Arc`s, the inner's target and publish
   state behind short-lived mutexes). `maps.Params` is a reference in Go: the front matter
   metadata writes into the resource's own params (`SharedParams`).

## Go behaviour reproduced on purpose

- Multihost: the second language's processed image finds the file the first language created
  in the same build (Go's `ReadOrCreate` read path over `ImageCache.encoded`): its `Key()` has
  no source hash (`fr/images/watermark_hu_….png`), it keeps its parent's format, it is published
  under each language's dir.

- `Name`/`Title`/`Params` of a transformed resource come from the adapter's `metaProvider`, the
  target at creation time: `(resources.Get "a.scss" | toCSS).Name` is still `/a.scss`, and a
  processed image keeps its original's name.
- The transformation cache is keyed by `TransformationKey` (global): a second adapter with the
  same chain reuses the first one's result, and when the first ran for `.Content` only, a later
  link publishes through `publishOnce` (the fixture's `links-first` pass).
- A failing transformation leaves the target unchanged: its links point to the source, which is
  published (once) when the adapter was lazily published.
- Front matter metadata: `:counter` is per `assignMetadata` call (so always 1 per resource), a
  meta entry without `src` stops the assignment (earlier changes stay), the metadata params are
  copied into the resource's own params (a translation that shares the resources sees them).
- `Key()` trims the base URL path, prefixes the language in multihost mode and appends the
  decimal root source hash for images; clones share the hash.
- `ResourceSourceDescriptor.init`: the XML/RSS ambiguity picks `application/xml`, unknown
  suffixes fall back to `mime.TypeByExtension` (`.ico` → `image/vnd.microsoft.icon`, not an
  image resource for processing).
- `fingerprint` as the first step hashes the source `io.ReadSeeker` without writing `To`, so
  the next step reads the source (the ping-pong buffers are only swapped after writes).
- `GetFieldString("…MediaType.X…")`: fields by name, then value-receiver methods without
  arguments, through `cast.ToString` (`FirstSuffix` and `Suffixes` give `""`, `IsText` gives
  `true`, `MarshalJSON` the JSON).
- Processed images: the background fill (`#hex` or the imaging `bgColor`) when the target has no
  alpha, Floyd–Steinberg to the source palette (plus the background colour) for PNG targets of
  paletted sources; errors wrapped like Go's `os.PathError` (`<action> <target path>: …`).

## Verification

Oracles in `tools/go-oracle/nh-resources/<topic>/main.go` (package `main` in the neohugo module,
shared helpers in `rsupport`) write gzipped JSON to `tests/fixtures/<topic>/`; the Rust tests
are `tests/<topic>.rs`. The synthetic site (`tests/fixtures/site`: bundles with images, JSON,
text and CSS, `resources` front matter metadata with globs, `:counter` and params, two
languages, a base path, assets of every media type kind, the 600×480 RGBA watermark) is made by
`tools/go-oracle/nh-resources/gensite.py`. `cargo test` needs neither Go, qemu nor the network.

| topic | inputs | checks |
|---|---|---|
| `resources` | every asset and bundle resource of the synthetic site, the repository's `docs/` site and `hugolib/testsite` (132 resources), found with an in-memory hugolib build (bundles) and `resources.Get` (assets); the descriptor hugolib/create used, recreated per language with shared `SpecCommon`, the page's metadata applied in hugolib's order (shared resources reused) | RelPermalink, Permalink, Key, Name, Title, NameNormalized, MediaType (+ JSON), ResourceType, Data, Params, Content (text) / sha256, Width/Height, before and after the metadata, the resource's own params after it, and every published file (len + sha256); the oracle checks that the recreated resources equal hugolib's |
| `keys` | a multihost config (`tests/fixtures/site-multihost`: two languages with their own base URLs) with resizes in both languages; 12 transformation chains (the seeksnack SCSS chain, fingerprint/minify/jsbuild/execute-as-template/postcss/tocss keys with struct, map, nil-map, slice, int/int64/float/bool elements, case and escaping of the target key, an image target); the golden watermark vector; the synthetic watermark's resizes, chained resizes and Overlay filter keys (int, int64 and float offsets) | `TransformationKey` **`scss/website.scss_ce37005bb9b0d2e87a9f0d33876c2b52`**, each key's `Value()`; the cold key **`/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147`** (and the filter key `1682858112077426900` T10 reproduced from it) |
| `transform` (arm64) | 14 assets × 15 chains × 2 access orders (420 cases): fingerprint md5/sha256/sha384/sha512 (and an unsupported algorithm), minify, their combinations, `resources.Copy`, FeatureNotAvailable steps (postcss, tocss with the cache fallback, babel), a minifier error | TransformationKey, Content or its error, RelPermalink/Permalink (`.<hash>`, `.min` target paths), Key, Name, Title, MediaType, ResourceType, Data (`Integrity`), Params, the files each case published; `resources.PostProcess` (memoized) placeholders, `Data`/`MediaType` placeholder maps, `GetFieldString` of every field through hugolib's replacement loop, unknown accessors; `slice`/`Slice` results and `resources.Concat`'s type check (`resource.Resources` accepted, `[]interface {}` rejected) |
| `images` (arm64) | 12 repository images (5 seeksnack JPEGs of the go-image fixtures, sunset, an EXIF-orientation JPEG, the paletted/RGB/RGBA seeksnack PNGs of the go-png fixtures, gopher-hero8, a PNG with alpha) and the watermark: 336 operations through the resource API — the seeksnack chains (Resize → Filter(Overlay watermark) → webp, index.json's 600×480 webp, render-image's Filter on the original), Fit/Fill/Crop with anchors and filters, JPEG targets with and without `#bg`, PNG targets (Floyd–Steinberg), rotation, Process, grayscale, a multi-filter chain with `images.Process` and an overlay at an offset, AutoOrient, two-step chains | Key, Name, MediaType, Width, Height, length and sha256 of the encoded bytes (full bytes of the seeksnack list chain of one source: the filtered JPEG and its WebP), RelPermalink, the published files; the encoded bytes equal `ImageCache.encoded` |
| `gen` | the synthetic site with a poisoned `resources/_gen` (garbage under the names Go would read) and a source file system that records every `_gen` access | the images fixture's bytes, a `tocss` chain's error, zero `_gen` accesses (a control read through the images file cache is seen) |
| `template_api` | the synthetic assets through `ResourceRef` | the method table agrees with the API; image methods on non-image and SVG resources give Go's panic messages; the PostProcess object |

Results: 0 differences. GIF resources: Go decodes their size (1×1 `pix.gif`), nh-images does not
decode GIF configs (T10 stub), so `Width`/`Height` are 0 (2 records, asserted as such).

The `transform` and `images` oracles are built for linux/arm64 (cgo libwebp through zig) and run
under `qemu-aarch64-static`: amd64 Go gives different bytes for 38 of the 336 image operations
(gift/flate/libwebp float fusion). `resources` and `keys` have no float code and run natively.

```sh
python3 tools/go-oracle/nh-resources/gensite.py              # the synthetic site
CC_ARM64=<zig cc wrapper> tools/go-oracle/nh-resources/regen.sh   # all four topics
```

Every fixture regenerates byte for byte (about 2 minutes).

## Known gaps

- `.Colors` (color-extractor) — STUB.
- GIF sizes and GIF/WebP/TIFF/BMP sources (nh-images decoding stubs).
- The identity system.
