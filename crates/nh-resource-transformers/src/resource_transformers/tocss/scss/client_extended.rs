//! Port of `resources/resource_transformers/tocss/scss/client_extended.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).


// Wave B: see the checklist below (the public API of this Go file is declared in the sibling modules / stubbed).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/tocss/scss/client_extended.go (56 lines; 2/2 funcs executed)
//   types: options, toCSSTransformation
// EX L31-47: (c *Client) ToCSS(res resources.ResourceTransformer, opts Options) (resource.Resource, error)
// EX L54-56: (t *toCSSTransformation) Key() internal.ResourceTransformationKey
// ---------------------------------------------------------------------------
