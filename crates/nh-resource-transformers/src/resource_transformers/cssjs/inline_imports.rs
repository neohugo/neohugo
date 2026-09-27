//! Port of `resources/resource_transformers/cssjs/inline_imports.go`.
//!
//! optional (inlineImports=false)
//!
//! Owner: Wave B task T16 (js-css-pipeline).


// Wave B: see the checklist below (the public API of this Go file is declared in the sibling modules / stubbed).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/cssjs/inline_imports.go (247 lines; 1/6 funcs executed)
//   types: fileOffset, importResolver
// EX L60-69: newImportResolver(r io.Reader, inPath string, opts InlineImports, fs afero.Fs, logger loggers.Logger, dependencyManager identity.Manager) *importRe...
//    L71-79: (imp *importResolver) contentHash(filename string) ([]byte, string)
//    L81-158: (imp *importResolver) importRecursive( lineNum int, content string, inPath string, ) (int, string, error)
//    L160-174: (imp *importResolver) resolve() (io.Reader, error)
//    L186-208: (imp *importResolver) shouldImport(s string) bool
//    L210-247: (imp *importResolver) toFileError(output string) error
// ---------------------------------------------------------------------------
