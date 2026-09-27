//! Port of `resources/resource_transformers/tocss/scss/client.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).


// Wave B: see the checklist below (the public API of this Go file is declared in the sibling modules / stubbed).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/tocss/scss/client.go (93 lines; 3/4 funcs executed)
//   types: Client, Options
// EX L35-37: New(fs *filesystems.SourceFilesystem, rs *resources.Spec) (*Client, error)
// EX L68-79: DecodeOptions(m map[string]any) (opts Options, err error)
// EX L86-89: replaceRegularImportsIn(s string) (string, bool)
//    L91-93: replaceRegularImportsOut(s string) string
// ---------------------------------------------------------------------------
