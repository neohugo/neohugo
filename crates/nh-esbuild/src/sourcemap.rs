//! Port of `internal/js/esbuild/sourcemap.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).


// Go `sourcemap.go`: source maps are off for seeksnack; port when needed.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: internal/js/esbuild/sourcemap.go (80 lines; 1/4 funcs executed)
//   types: sourceMap
// EX L32-41: fixOutputFile(o *api.OutputFile, resolve func(string) string) error
//    L43-57: fixSourceMap(s []byte, resolve func(string) string) ([]byte, error)
//    L59-71: fixSourceMapSources(s []string, resolve func(string) string) []string
//    L74-80: SourcesFromSourceMap(s string) []string
// ---------------------------------------------------------------------------
