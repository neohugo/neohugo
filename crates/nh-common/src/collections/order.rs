//! Port of `common/collections/order.go`.
//!
//! Owner: Wave B task T01 (common-values).


/// Go: `collections.Order` interface (`Ordinal() int`) — kept for API shape.
pub trait Order {
    fn ordinal(&self) -> i64;
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/collections/order.go (20 lines; 0/0 funcs executed)
//   types: Order
// ---------------------------------------------------------------------------
