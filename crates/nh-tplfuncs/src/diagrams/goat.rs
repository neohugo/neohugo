//! Port of `tpl/diagrams/goat.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! STUB: the GoAT ASCII-art renderer (`github.com/bep/goat`) is not ported (seeksnack has no
//! `goat` code block or `diagrams.Goat` call), so `Goat` is an explicit `neohugo-rs` error
//! instead of an approximated SVG.

use go_value::{HostCtx, Value};
use nh_common::object::{GoResult, args};

/// Goat creates a new SVG diagram from input v. STUB (see the module docs).
// Go: tpl/diagrams/goat.go:Goat
pub fn goat(_ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
    args::exactly(a, 1, "Goat")?;
    Err(go_value::Error::new(
        "neohugo-rs: diagrams.Goat (bep/goat) is not supported",
    ))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/diagrams/goat.go (68 lines; 0/5 funcs executed)
//   types: goatDiagram, Namespace
// STUB L31-33: (d goatDiagram) Inner() template.HTML
// STUB L35-37: (d goatDiagram) Wrapped() template.HTML
// STUB L39-41: (d goatDiagram) Width() int
// STUB L43-45: (d goatDiagram) Height() int
// STUB L53-68: (d *Namespace) Goat(v any) SVGDiagram
// ---------------------------------------------------------------------------
