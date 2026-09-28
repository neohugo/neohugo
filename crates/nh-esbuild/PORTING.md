# nh-esbuild — porting notes

neohugo internal/js + internal/js/esbuild (options, build client, resolve plugins) over the pinned
esbuild 0.25.6 binary (`--service` protocol). Owner and crate lead: Wave B task T16
(js-css-pipeline).

## Go file → Rust module

| Rust module | Go source(s) | Status |
|---|---|---|
| `api` | `internal/js/api.go` | nothing executed (`js.Batch` interfaces; not ported) |
| `options` | `internal/js/esbuild/options.go` | ported (all 4) |
| `build` | `internal/js/esbuild/build.go` | ported (both) |
| `resolve` | `internal/js/esbuild/resolve.go` | ported (4 of 5; `ResolveResource` belongs to `js.Batch`) |
| `sourcemap` | `internal/js/esbuild/sourcemap.go` | ported (all 4) |
| `helpers` | `internal/js/esbuild/helpers.go` | nothing to port |
| `service::protocol` | evanw/esbuild@v0.25.6 `cmd/esbuild/stdio_protocol.go` | NEW: packet codec, ported function by function (`readUint32`, `writeUint32`, `readLengthPrefixedSlice`, `encodePacket`, `decodePacket`) |
| `service::client` | the host side of `cmd/esbuild/service.go` (esbuild's `lib/shared/common.ts`) | NEW: process start + version check, request/response routing, `build` requests, `on-start`/`on-resolve`/`on-load` callbacks, pings |

Every GO PORTING CHECKLIST entry that seeksnack executes is `OK`; every ported function carries a
`// Go:` line.

## Design: esbuild as a service

Go links esbuild and calls `api.Build(opts)` with two Go plugins. The port runs the same esbuild
code as a child process, `esbuild --service=0.25.6 --ping`, and talks to it like esbuild's own
JavaScript API does:

- **Startup.** The binary writes its version first (4-byte little-endian length + bytes); it must
  be `0.25.6` (`Cannot start service: Host version "0.25.6" does not match binary version "…"`,
  the JavaScript host's text). A reader thread then splits stdout into packets.
- **Build request.** `{command: "build", key, entries, flags, write: false, stdinContents,
  stdinResolveDir, absWorkingDir, nodePaths: [], context: false, plugins}`. The options travel as
  CLI flags; the service rebuilds `api.BuildOptions` with `cli.ParseBuildOptions`.
  `service::client::build_flags` writes exactly the flags `flagsForBuildOptions` writes for the
  fields Hugo's `compile` sets (target, format, platform, minify-*, sourcemap, sources-content,
  bundle, outdir, tsconfig, define, external, inject, loader map, drop, jsx*, the stdin loader),
  plus `--log-level=silent` (Go's zero `LogLevel`; messages are still returned) and
  `--log-limit=0`. A zero value is written as no flag, as in the Go API.
- **Plugins.** `hugo-import-resolver` (onResolve `.*`, onLoad `.*` in `ns-hugo-imp`) and
  `hugo-params-plugin` (onResolve `^@params(/config)?$`, onLoad `.*` in `ns-hugo-params`) are
  registered as `{name, onResolve: [{id, filter, namespace}], onLoad: [...]}` with ids numbered
  over all plugins in registration order. esbuild evaluates the filters itself (Go regexps, as with
  Go plugins) and sends `on-resolve`/`on-load` requests with the matching ids in order.
- **Callbacks run on the calling thread.** `ServiceClient::build_with_plugins` blocks until the
  build response arrives and answers its own callback requests meanwhile (the reader routes them by
  build key), so the plugin closures borrow the build's options and resolver. Pings are answered by
  the reader. Many builds may run at once on one service (tested with 8 threads).
- **Go plugin semantics, not the JavaScript host's.** An onResolve callback that returns no path
  and is not external passes on to the next matching callback (esbuild's `RunOnResolvePlugins`
  `continue`); an onLoad callback without contents passes on too. The JavaScript host would stop
  at the first callback that returns an object. This is what makes `@params` reach the params
  plugin after the import resolver declined it, as in Go.
- **Binary lookup** (HUGO_LAYER.md §6.8, `service::client::find_binary`):
  `$NEOHUGO_ESBUILD_BINARY`, `<workingDir>/node_modules/@esbuild/<os>-<arch>/bin/esbuild`,
  `<workingDir>/node_modules/.bin/esbuild`, `esbuild` on `PATH`; none → a `FeatureNotAvailable`
  error naming `NEOHUGO_ESBUILD_BINARY` and `tools/esbuild/build.sh`. `BuildClient::new_default`
  (Go's `NewBuildClient`) starts the service lazily on the first `js.Build`, so a build without
  `js.Build` needs no binary. The process ends when the client is dropped (stdin closed).
- **The pinned binary.** `tools/esbuild/build.sh` runs `go build -o tools/esbuild/bin/esbuild
  github.com/evanw/esbuild/cmd/esbuild` in the neohugo module: the go.mod-pinned v0.25.6, offline
  from the module cache (`GOPROXY=off`); it checks that go.mod/go.sum are unchanged and that the
  binary prints `0.25.6`. `tools/esbuild/.gitignore` ignores `bin/`.

## Deliberate deviations

1. **esbuild runs as a separate process** (above) instead of being linked. The output bytes are
   esbuild's own either way; `tests/service.rs` checks the service against the CLI, and
   crates/nh-resource-transformers/tests/jsbuild.rs checks whole `js.Build` results against Go's
   in-process esbuild (68 cases, 0 differences).
2. **Loader `none`.** Hugo's `nameLoader` maps `none` to `api.LoaderNone`, which has no CLI
   spelling; `js.Build` with a `none` loader is an explicit error
   (`neohugo-rs: js.Build loader "none" is not supported`). Go's `copy` maps to `file` (as Go does).
3. **Define keys containing `=`** cannot be written as `--define:` flags: an error
   (`Invalid define: …`, the JavaScript API's text). Go would accept them.
4. **Identity/dependency tracking** (`DependencyManager.AddIdentity`) and the `js.Batch` hooks of
   `InternalOptions` (`ImportOnResolveFunc`, `ImportOnLoadFunc`, `ImportParamsOnLoadFunc`,
   `ErrorMessageResolveFunc`, `ResolveSourceMapSource`) are not ported (server rebuilds and
   `js.Batch` only). `validate` keeps the `AbsWorkingDir` check.
5. **Error texts.** `createErr` builds a positioned `nh_common::Error` (`"file:line:col": msg`)
   where Go builds a `FileError` with the file content; the position is the same (Go's simple line
   matcher never moves it). When nh-resources wraps it, the text is Go's
   (`JSBUILD: failed to transform "…" (…): "file:line:col": msg`, nh-common's `Error::wrap`).
6. **Protocol decoding** returns `None` where Go panics (unknown tag, truncated bool) and refuses
   values nested deeper than 10,000 levels (esbuild never sends them).

## Go behaviour reproduced on purpose

- `DecodeExternalOptions` defaults `sourcesContent` to true, but a nil options map skips the
  decode (`js.Build` without options builds with `SourcesContent` false).
- `Target`/`Format` are lower-cased (other values are not); `target: "es6"` is `es2015`.
- An unknown `drop` value still compiles the options and then fails (Go sets `err` without
  returning).
- `inject` paths are resolved in the assets fs (absolute paths rejected) and passed as real
  filenames; `ns-hugo-*` prefixes are stripped from error paths and messages; `<stdin>` errors
  point at the resource's real file.
- External source maps are re-marshalled from Go's five-field `sourceMap` struct with `sources`
  turned into `file://` URLs (a nil list stays `null`).
- The resolver: shims, externals, `__hu_v` virtual importers, importers outside `/assets` left to
  esbuild, importers inside the `node_modules → assets/vendor` mount resolved by Hugo (so their
  relative imports come back in `ns-hugo-imp`), `ResolveComponent`'s extension/index/`.esm`/strip
  `.js` order, the first result cached per import path.

## Verification

- `tests/service.rs`: the codec against hand-encoded `encodePacket` vectors (sorted keys, id
  shift, response bit, `-1` as 0xFFFFFFFF, truncations, trailing bytes, unknown tags, duplicate
  keys), `build_flags` for Hugo's option sets, the binary lookup order, the version check (a fake
  binary), and — with the binary — service build == CLI build, plugin callbacks (pass-on semantics,
  namespaces, external results, errors carrying the plugin name), error locations, 8 concurrent
  builds on one service.
- The `js.Build` parity tests live in crates/nh-resource-transformers (`tests/jsbuild.rs`,
  oracle `tools/go-oracle/nh-resource-transformers/jsbuild`).

Tests that need esbuild read `NEOHUGO_ESBUILD_BINARY`; without it they print `SKIPPED: …` to
stderr (written directly, so the harness does not capture it) and pass:

```sh
tools/esbuild/build.sh
NEOHUGO_ESBUILD_BINARY=$PWD/tools/esbuild/bin/esbuild cargo test --offline   # in crates/nh-esbuild
```

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-resource, nh-resources.
- Wave A: go-value, go-json, go-path, go-strconv, go-unicode.
- crates.io: none (`std::process` for the service).

## Known gaps

- `js.Batch` (`internal/js/esbuild/batch.go`) is not ported (seeksnack does not use it).
- Metafile, splitting and entry points other than stdin are passed through but untested.
