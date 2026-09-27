//! Port of `resources/postpub/fields.go`.
//!
//! Owner: Wave B task T14 (resources-core).


// Go `postpub/fields.go`: struct -> map with placeholders (MediaType/Data fields).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/postpub/fields.go (59 lines; 0/3 funcs executed)
//    L24-28: structToMapWithPlaceholders(root string, in any, createPlaceholder func(s string) string) map[string]any
//    L30-52: structToMap(s any) map[string]any
//    L55-59: insertFieldPlaceholders(root string, m map[string]any, createPlaceholder func(s string) string)
// ---------------------------------------------------------------------------
