# nh-resource-transformers — porting notes

neohugo resources/resource_factories/{create,bundler} and resources/resource_transformers/{integrity,minifier,templates,js,cssjs,tocss}.
Crate lead: Wave B task T15 (resource-factories).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Status |
|---|---|---|---|
| `resource_factories::create::create` | `resources/resource_factories/create/create.go` | T15 | ported (all 10) |
| `resource_factories::create::remote` | `resources/resource_factories/create/remote.go` | T15 | ported (13 of 14); `transport.RoundTrip` (the network, with retries) is the httpcache transport's `NoNetwork` |
| `resource_factories::create::mime` (private) | go1.27.1 `mime/type.go`, `type_unix.go`, `mediatype.go`, `grammar.go` | T15 | NEW: a copy of nh-resources' private `mime` module plus `ExtensionsByType` |
| `resource_factories::bundler` | `resources/resource_factories/bundler/bundler.go` | T15 | ported (all 7) |
| `resource_transformers::integrity` | `resources/resource_transformers/integrity/integrity.go` | T15 | ported (all 7) |
| `resource_transformers::minifier` | `resources/resource_transformers/minifier/minify.go` | T15 | ported (all 4) |
| `resource_transformers::templates` | `resources/resource_transformers/templates/execute_as_template.go` | T15 | ported (all 4) |
| `resource_transformers::js::build` | `resources/resource_transformers/js/build.go` | T16 js-css-pipeline | ported (all 3) |
| `resource_transformers::js::transform` | `resources/resource_transformers/js/transform.go` | T16 js-css-pipeline | ported (both) |
| `resource_transformers::cssjs::postcss` | `resources/resource_transformers/cssjs/postcss.go` | T16 js-css-pipeline | ported (all 6) |
| `resource_transformers::cssjs::inline_imports` | `resources/resource_transformers/cssjs/inline_imports.go` | T16 js-css-pipeline | ported (all 6) |
| `resource_transformers::cssjs::tailwindcss` | `resources/resource_transformers/cssjs/tailwindcss.go` | T16 js-css-pipeline | STUB: `Process` is an explicit error; client, options, `toArgs`, the import exclusion ported |
| `resource_transformers::babel` | `resources/resource_transformers/babel/babel.go` | T16 js-css-pipeline | STUB: `Process` is an explicit error; client, options, `toArgs` ported |
| `resource_transformers::tocss::scss::client` | `resources/resource_transformers/tocss/scss/client.go` | T16 js-css-pipeline | ported (all 4; `Options`/`DecodeOptions` in `tocss`) |
| `resource_transformers::tocss::scss::client_extended` | `resources/resource_transformers/tocss/scss/client_extended.go` | T16 js-css-pipeline | ported (both) |
| `resource_transformers::tocss::scss::tocss` | `resources/resource_transformers/tocss/scss/tocss.go` | T16 js-css-pipeline | ported (all 3) |
| `resource_transformers::tocss::sass::helpers` | `resources/resource_transformers/tocss/sass/helpers.go` | T16 js-css-pipeline | ported (both) |

The tpl `resources` namespace (`nh_tplfuncs::resources`, `nh_tplfuncs::internal::resourcehelpers`)
is T15's too; its notes are here.

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-resource, nh-resources,
  nh-transform, nh-tplimpl, nh-tpl, nh-esbuild.
- Wave A: go-value, go-hashstructure, libsass-sys, go-path, go-url, go-strconv, go-unicode.
- crates.io: `sha2`, `md-5` (the fingerprint digests) and `base64` (`STANDARD`, the `Integrity`
  value): pure digests/encodings, they cannot change output bytes (README rule 2). Hex is
  written out.
- dev: `serde_json`, `flate2` (`rust_backend`, gunzip of fixtures), nh-allconfig (the test
  site's config), go-json (the JSON of resource data).

## Public API (changes to the skeleton; no other crate used these items)

- `create::Client` gained `http_cache_config`, `cache_get_resource`, `by_type`, `from_opts` +
  `Options`; `remote::FromRemoteOptions` has Go's fields (`method`, `headers: Map`, `body`,
  `response_headers`); `remote::HttpError` + `Client::from_remote_http` (Go's `*HTTPError` with
  its `Data`); `remote_resource_keys` takes the options by reference and clones them.
- `templates::Client.t` is a `StoreProvider` (`Arc<dyn TemplateStoreProvider + Send + Sync>`),
  not a `TemplateStore`: Go's client holds `*deps.Deps` and reads the store when a
  transformation runs (after the site's store is set). `templates::Client::new(rs, t)`.
- `integrity::Client::new`; `minifier::Client::new` returns `Result`.
- nh-tplfuncs: `Namespace::try_new` (Go's `New` error; `new` panics like Go's init);
  `resourcehelpers::{resolve_args_go, resolve_if_first_arg_is_string_go, transformer_from_value}`
  are the Go-shaped forms, the skeleton forms are kept.

## Deliberate deviations

1. **No network.** The transport's inner round tripper is `NoNetwork` (T08): a GetRemote whose
   response is not in the getresource file cache fails with
   `Get "<url>": neohugo-rs: network access is not supported (...)` where Go fetches (or fails
   with its network error). Go's `transport.RoundTrip` (retries of 408/429/5xx until the
   `timeout`) is therefore not ported.
2. **Nil results are not cached.** Go caches a nil `Get` (missing file) and a nil `GetRemote`
   (404) in the resource cache; the Rust cache holds resources, so the nil result is computed
   again (same result: nothing changes during a build).
3. **Errors carry no data.** `resource.NewResourceError(err, data)` keeps only the message on
   its way to the template (go_value errors have no data): `.Err.Data` of `try` is not
   available. `Client::from_remote_http` keeps Go's `HTTPError.Data`.
4. **Watch mode.** `create::Client::new` errors when watching with polling enabled (Go starts a
   remote resource poller; server mode only). `configurePollingIfEnabled` never polls.
5. **Map order.** `addUserProvidedHeaders` adds headers in sorted key order (Go: map order; it
   only matters for keys that canonicalise to the same header, and the request is never sent).
6. **`resources.ToCSS`/`PostCSS`/`Babel`** delegate to a css/js namespace created over the same
   deps (Go uses the func map's instances, found in `OnCreated`).
7. **Go panics become errors**: a resource over a directory (`resources.Get "js"`) panics in Go
   when read (`this operation is not supported`), the port returns that error; `Get`/`Match`/
   `GetMatch`/`ByType`/`Copy` panics are template errors in both.
8. The ExtensionsByType copy of the `mime` tables is machine dependent like Go (globs2 or
   mime.types); only content types without an accepted media type reach it.

## Go behaviour reproduced on purpose

- Concat is cached by `path.Clean(targetPath)` only: the first part list wins (`js/../js/ab.js`
  reuses `js/ab.js`; `/js/ab.js` is another key with the same link, and the later publish
  overwrites). JavaScript parts are joined with `"\n;\n"` (no trailing separator); the media
  type check reads every part's `MediaType()` (running its chain) and reports the LAST type seen
  as "got X and Y".
- ExecuteAsTemplate's key is `("execute-as-template", targetPath)` plus the source resource:
  the same source and target keep the first data; another source with the same target is a new
  transformation.
- FromString keys by `cleanKey(target) + xxhash(content)`; Copy by `cleanKey(target)+"__copy"`
  (first source wins).
- GetRemote: `remoteResourceKeys` (the `key` option, case-insensitive, is removed before the
  options are hashed; an empty options map hashes like nil); a user key selects the file cache
  entry, so another URL with the same key reads the first URL's response; HEAD and accepted
  content types use `media.FromString` (no suffix: `/data_<key>`); `text/plain` hints `.txt`,
  then the file name's extension; RFC 2231 `Content-Disposition` file names; the method option
  must be a Go `string` (`interface conversion` otherwise); `url.Error` texts.

## Verification

Oracles in `tools/go-oracle/nh-resource-transformers/<topic>/main.go` (shared helpers in
`rtsupport`, which reuses T14's `rsupport`) write gzipped JSON to `tests/fixtures/<topic>/`. The
synthetic site `tests/fixtures/site` is written by `gensite.py`. Every case is a script of tpl
`resources` namespace calls; the Rust tests replay it through `nh_tplfuncs::resources`
(`crates/nh-tplfuncs/tests/resources.rs`) and record the same attributes (Name, Title, Key,
NameNormalized, MediaType (+JSON), ResourceType, Data, Params, Content or its error, Width/Height,
RelPermalink, Permalink; PostProcess placeholders and GetFieldString) and the published files.

| topic | inputs | result |
|---|---|---|
| `factories` (native) | synthetic site (25 assets: mixed case, spaces/ü, nested dirs, every media type) and `docs/` (54 assets): Get of every asset and 20 path variants/bad args, 41 GetMatch and 41 Match globs (case folding, `**`, classes, bad patterns), 9 ByType, 14 FromString, 8 Copy | 155 + 187 calls (103 published files), 0 differences |
| `transformers` (arm64 under qemu, CGO off via `arm64build`) | synthetic site: 23 Concat (both orders, empty parts, CSS/TS/JSON, mixed types, bad args, first writer), 58 Fingerprint (every algorithm incl. unsupported, arg forms), 25 Minify (every minifier, no-minifier types), 20 ExecuteAsTemplate (first writer, data kinds, parse and exec errors, define, chains), 5 PostProcess, FromString/Copy chains; docs: fingerprint md5/sha256 + minify of every js/css/svg/json and Concat of all JS both orders | 159 + 176 calls (276 published files), 0 differences (1 accepted: gotemplate deviation 15, `in type interface {}` vs `in type string`) |
| `getremote` | a local httptest server (via `http.DefaultTransport` dialing it for every host, so URLs and keys are port-independent): JSON, PNG, text, CSV, HTML, CSS, SVG, Content-Disposition, accepted vendor type, unknown binary, ETag, no-store, 404, 403, 418, 500 with retries, redirect, echoed headers/method/body, POST, HEAD, key option, responseHeaders, bad options/URLs; live run then a fresh client over the recorded cache with the network disabled | 51 namespace calls (6 not cached: network error in Go, NoNetwork in Rust), 32 recorded entries in `tests/fixtures/getremote/cache`; 48 `remoteResourceKeys` vectors (also with reversed option order); the 51 golden seeksnack getresource entries through FromRemote: 0 differences |

The plan's key vector `iIsZs0m-BVU → 5844198154546968338` needs the site's private YouTube API
key in the URL; the entry itself (`5844198154546968338`) is among the 51 read through
`FromRemote` (placed under the key of a synthetic URL).

```sh
export GOTOOLCHAIN=go1.27.1
python3 tools/go-oracle/nh-resource-transformers/gensite.py
go run ./tools/go-oracle/nh-resource-transformers/getremote -root .
go run ./tools/go-oracle/nh-resource-transformers/factories -root .
go run ./tools/go-oracle/nh-resource-transformers/arm64build -o /tmp/transformers.arm64
TZ=UTC qemu-aarch64-static /tmp/transformers.arm64 -root .
```

Every fixture regenerates byte for byte.

## Known gaps

- Multilingual sharing (Concat/ExecuteAsTemplate first writer across languages) relies on the
  shared `SpecCommon` resource cache (T14); the oracles use one language.
- `.Err.Data` of GetRemote errors (deviation 3).

## T16 (js-css-pipeline): js.Build, toCSS, postCSS

The modules `resource_transformers::{js, cssjs, tocss, babel}` are Wave B task T16's; the esbuild
side (options, resolver plugins, the `--service` client) is crates/nh-esbuild (see its
PORTING.md). Dependency added by T16: `go-fmt` (the `%q`/`%v` of `hugo:vars` values).

### Public API

- `js::build::Client::new(rs, c)` (skeleton) and `Client::new_default(rs)` (Go's `New(fs, rs)`:
  the assets fs, an esbuild service started on the first build); `process(ctx, r, opts)`.
- `tocss::scss::tocss::Client` has Go's fields (`rs`, `sfs`, `work_fs`); `Client::new(fs, rs)`
  (in `client`); `to_css(ctx, r, opts)` (in `client_extended`); `decode_options(m)`;
  `client_extended::key_value(&opts)` (the `tocss_<hash>` key value; `scss.Options` is hashed
  like Go's struct through a go-hashstructure registration); `Options.vars` is `None` for a nil
  or empty map (same stylesheet, same hash).
- `cssjs::postcss::new_post_css_client(rs)`, `PostCssClient::process(ctx, r, options)`;
  `cssjs::tailwindcss::new_tailwind_css_client(rs)`; `babel::Client::new(rs)`,
  `babel::decode_options`.

### Deliberate deviations

1. **PostCSS runs in the working dir.** Go leaves the child's cwd to the process (the golden
   build ran from the site dir; purgecss reads `./hugo_stats.json`, postcss-cli names its input
   `<cwd>/stdin`); the port sets `dir = workingDir`, so it does not depend on where neohugo-rs is
   started. The environment is Go's (`GetExecEnviron`: filtered OS env, `NODE_PATH`, `PWD`,
   `HUGO_ENVIRONMENT`, `HUGO_ENV`, `HUGO_PUBLISHDIR`, `HUGO_FILE_*` of `assets/_jsconfig`).
2. **stdin is written before the command runs** (Go copies it in a goroutine while the command
   runs; same bytes). postcss's stderr goes to the info log after the run.
3. **tailwindcss and babel are stubs** (HUGO_LAYER.md §1 rule 5): `Process` returns
   `neohugo-rs: css.TailwindCSS is not supported` / `neohugo-rs: js.Babel is not supported`.
4. **LibSass errors** keep Go's position (file, line, column; the entry file's `stdin` becomes
   its real filename) and message; the error text is `"file:line:col": message` like Go's
   `FileError`, and `TOCSS: failed to transform "…" (…): "file:line:col": message` once nh-resources
   wraps it (nh-common's `wrap` keeps Go's order; the tests compare the texts as they are).

### Go behaviour reproduced on purpose

- toCSS: precision 0 → 8; `outputStyle` case-insensitive, unknown → nested; include paths =
  `RealDirs(dir(source))` then each user path `Stat`ed in the work fs (missing ones dropped), the
  seeksnack order `[assets/scss, node_modules, assets/scss]`; the Hugo importer (`hugo:vars`,
  `prev == "stdin"` → the entry's dir, importers outside the assets fs left to LibSass, the
  `_%s`/`%s`/`_%s.scss`/… patterns); `.sass` entries; the entry file's `@import "x.css"`
  protection; `hugo:vars` sorted, `unquote(%q)` for plain strings (a bool gives Go's
  `%!q(bool=true)`, a LibSass error in `hugo:vars`), `%q` for `css.Quoted`, `%v` for numbers,
  units, colors, CSS functions and `css.Unquoted`; source maps with the working dir as root,
  `stdin"` replaced by the source path, published as `<target>.map`. LibSass makes the map's
  `sources` relative to the process working directory (in Go too: run from the site dir).
- js.Build: `OutPath` = targetPath or `.js`; the source map of `external`/`linked` builds is
  published, `linked` rewrites the `sourceMappingURL`; a JS import of CSS gives esbuild a second
  output that Go ignores.
- postCSS: `no-map` (snake case) read from the raw map; the config from `assets/_jsconfig`, then
  the work dir; an explicit config that is missing is an error; `--config` omitted when none is
  found; the `@import` inliner's quirks (a trailing comment stays in the path; `url()`, media
  queries and `tailwindcss` are not inlined; repeated imports inlined once; line numbers of
  postcss errors mapped back to the imported file).

### Verification

Oracles in `tools/go-oracle/nh-resource-transformers/{jsbuild,tocss,postcss}` (shared runner
`t16support`) over hermetic copies of the synthetic site `tests/fixtures/t16site` (written by
`t16site.py`; `_node_modules` becomes `node_modules`, mounted as `assets/vendor` like seeksnack's)
and, for js.Build, of the repository's `docs/` assets. Each case is a script (Get, Concat,
js.Build, toCSS, postCSS, minify, fingerprint); the Rust tests (`tests/{jsbuild,tocss,postcss}.rs`,
runner `tests/t16_support`) replay it and compare TransformationKey, Content or its error,
MediaType, Data, RelPermalink and every published file. Absolute paths of the site copy are
`$SITE` (also inside inline source maps, which are decoded).

| topic | inputs | result |
|---|---|---|
| `jsbuild` (native) | 62 synthetic cases (TS/JS/JSX/TSX, Hugo-resolved and node_modules imports incl. the vendor mount, JSON/text/base64/dataurl loaders, @params, defines, targets, formats, platforms, minify, source maps inline/external/linked, externals, shims, inject, JSX automatic/preserve, drop, CSS imports, option and esbuild errors, the seeksnack level-1/level-2 chains with Concat and fingerprint) + 6 docs cases | 68/68 identical (esbuild binary over `--service` vs Go's linked esbuild) |
| `tocss` (arm64, qemu) | 31 cases (styles, precisions, include paths, partial/index/.sass/vendor resolution, css import protection, hugo:vars of every kind, source maps, targetPath, errors, toCSS \| minify \| fingerprint) + the seeksnack key `tocss_7149566072694007861` | 31/31 identical |
| `postcss` (arm64, qemu; needs node + postcss-cli) | 15 cases (configs, noMap, the inliner, syntax errors mapped to imported files, env filtering, cwd, toCSS \| postCSS \| minify \| fingerprint) | 15/15 identical |

`jsbuild` needs `NEOHUGO_ESBUILD_BINARY` (`tools/esbuild/build.sh`), `postcss` needs
`NEOHUGO_POSTCSS_BIN` (the `node_modules/.bin/postcss` of postcss-cli 11.0.1 with postcss 8.5.8,
installed outside the repository); without them the tests print `SKIPPED: …` to stderr and
pass. `tocss` needs nothing.

```sh
python3 tools/go-oracle/nh-resource-transformers/t16site.py .
CC_ARM64=<zig cc wrapper> CXX_ARM64=<zig c++ wrapper> POSTCSS_BIN=<…/node_modules/.bin/postcss> \
  tools/go-oracle/nh-resource-transformers/t16regen.sh
```

Every fixture regenerates byte for byte.
