//! Port of `tpl/cast/docshelper.go`.
//!
//! not needed (docs)
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! Go registers a `docshelper.DocProvider` in `init()` that builds every template namespace
//! against a dummy site and exports the func docs (`hugo gen docshelper` only). The Rust port
//! has no `gen docshelper` command: this is an explicit stub.

use nh_common::object::GoResult;

// Go: tpl/cast/docshelper.go:init
/// The `tpl` doc provider of `hugo gen docshelper`: not supported.
pub fn docs_provider() -> GoResult<()> {
    Err(go_value::Error::new(
        "neohugo-rs: docshelper (hugo gen docshelper) is not supported",
    ))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/cast/docshelper.go (53 lines; 1/2 funcs executed)
// OK L26-47: init() (STUB: docs_provider returns the unsupported error; only `hugo gen docshelper` reads it)
// OK L49-53: newTestConfig() config.Provider (STUB: part of the doc provider)
// ---------------------------------------------------------------------------
