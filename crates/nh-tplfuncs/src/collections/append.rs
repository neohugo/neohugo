//! Port of `tpl/collections/append.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! `Append` is ported in `collections.rs` (`Namespace::append`): it checks the argument count
//! and calls `nh_common::collections::append::append(to, from...)`.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/append.go (40 lines; 0/1 funcs executed)
// OK L31-40: (ns *Namespace) Append(args ...any) (any, error) (collections.rs)
// ---------------------------------------------------------------------------
