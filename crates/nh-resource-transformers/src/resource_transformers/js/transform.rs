//! Port of `resources/resource_transformers/js/transform.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

// Go `js/transform.go`: the `jsbuild` ResourceTransformation (OutMediaType text/javascript; OutPath =
// TargetPath or ReplaceOutPathExtension(".js"); SourceDir = dir(SourcePath); Stdin with contents).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/js/transform.go (68 lines; 2/2 funcs executed)
//   types: buildTransformation
// EX L32-34: (t *buildTransformation) Key() internal.ResourceTransformationKey
// EX L36-68: (t *buildTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// ---------------------------------------------------------------------------
