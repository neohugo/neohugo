//! Port of `common/collections/collections.go`.
//!
//! Owner: Wave B task T01 (common-values).

/// Go: `collections.Grouper` (`Group(key any, items any) (any, error)`), used by `group`.
pub const GROUPER_METHOD: &str = "Group";

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/collections/collections.go (21 lines; 0/0 funcs executed)
//   types: Grouper
// ---------------------------------------------------------------------------
