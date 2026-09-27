//! Port of `transform/chain.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).


//! Go `transform.Chain`: transformers applied in order, ping-ponging between two buffers.

use nh_common::Result;

/// Go: `transform.FromTo` — the input bytes and the output buffer of one step.
pub struct FromTo<'a> {
    pub from: &'a [u8],
    pub to: &'a mut Vec<u8>,
}

/// Go: `transform.Transformer func(ft FromTo) error`.
pub type Transformer = Box<dyn Fn(&mut FromTo<'_>) -> Result<()> + Send + Sync>;

/// Go: `transform.Chain`.
#[derive(Default)]
pub struct Chain(pub Vec<Transformer>);

impl Chain {
    // Go: transform/chain.go:New
    pub fn new(trs: Vec<Transformer>) -> Self {
        Chain(trs)
    }

    /// Go: `Chain.Apply(to, from)`: with no transformers `from` is copied to `to`.
    // Go: transform/chain.go:Apply
    pub fn apply(&self, from: &[u8]) -> Result<Vec<u8>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/chain.go (124 lines; 4/5 funcs executed)
//   types: Transformer, BytesReader, FromTo, Chain, fromToBuffer
//    L50-52: New(trs ...Transformer) Chain
// EX L55-57: NewEmpty() Chain
// EX L66-68: (ft fromToBuffer) From() BytesReader
// EX L70-72: (ft fromToBuffer) To() io.Writer
// EX L76-124: (c *Chain) Apply(to io.Writer, from io.Reader) error
// ---------------------------------------------------------------------------
