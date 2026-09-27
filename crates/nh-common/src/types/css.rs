//! Port of `common/types/css/csstypes.go`.
//!
//! Owner: Wave B task T01 (common-values).


/// Go: `css.QuotedString` (used by css.Quoted / Sass vars; unused by seeksnack).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotedString(pub String);

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/types/css/csstypes.go (20 lines; 0/0 funcs executed)
//   types: QuotedString, UnquotedString
// ---------------------------------------------------------------------------
