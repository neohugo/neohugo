# nh-esbuild — porting notes

neohugo internal/js + internal/js/esbuild (options, build client, resolve plugins) over the pinned esbuild 0.25.6 binary (--service protocol).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `api` | `internal/js/api.go` | T16 js-css-pipeline |  |
| `options` | `internal/js/esbuild/options.go` | T16 js-css-pipeline |  |
| `build` | `internal/js/esbuild/build.go` | T16 js-css-pipeline |  |
| `resolve` | `internal/js/esbuild/resolve.go` | T16 js-css-pipeline |  |
| `sourcemap` | `internal/js/esbuild/sourcemap.go` | T16 js-css-pipeline |  |
| `helpers` | `internal/js/esbuild/helpers.go` | T16 js-css-pipeline |  |
| `service::protocol` | — | T16 js-css-pipeline | NEW: esbuild stdio service packet codec (evanw/esbuild@v0.25.6 cmd/esbuild/service.go + internal/helpers stdio_protocol.go) |
| `service::client` | — | T16 js-css-pipeline | NEW: esbuild service client: build request, on-resolve/on-load plugin callbacks |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-resource, nh-resources
- Wave A (to add when available): go-json
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
