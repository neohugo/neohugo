//! Port of `transform/chain.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).

//! Go `transform.Chain`: transformers applied in order, ping-ponging between two buffers.

use nh_common::Result;

/// Go: `transform.FromTo` — the input bytes and the output buffer of one step.
///
/// Go's `From()` is a `*bytes.Buffer` (a `BytesReader`); every transformer of the publish chain
/// reads it through `Bytes()` in one piece, so the port hands out the slice. `To()` is the other
/// pooled buffer, reset before the step.
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

    /// Go: `NewEmpty()` (a chain with capacity 20).
    // Go: transform/chain.go:NewEmpty
    pub fn new_empty() -> Self {
        Chain(Vec::with_capacity(20))
    }

    /// Go: `append(transformers, t)`.
    pub fn push(&mut self, t: Transformer) {
        self.0.push(t);
    }

    /// Go: `len(c)`.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Go: `len(c) == 0`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Go: `Chain.Apply(to, from)`: with no transformers `from` is copied to `to`.
    ///
    /// Go reads `from` completely into the first pooled buffer (`b1.ReadFrom`) before the first
    /// step, so how the source was chunked never reaches a transformer. The output of step `i` is
    /// the input of step `i+1`; the result is the output of the last step.
    // Go: transform/chain.go:Apply
    pub fn apply(&self, from: &[u8]) -> Result<Vec<u8>> {
        if self.0.is_empty() {
            return Ok(from.to_vec());
        }

        let mut b1: Vec<u8> = from.to_vec();
        let mut b2: Vec<u8> = Vec::new();

        // Go's fromToBuffer: `from` is b1 for the first step, then the buffers swap.
        let mut from_is_b1 = true;

        for (i, tr) in self.0.iter().enumerate() {
            if i > 0 {
                from_is_b1 = !from_is_b1;
                if from_is_b1 {
                    b2.clear();
                } else {
                    b1.clear();
                }
            }

            let res = if from_is_b1 {
                tr(&mut FromTo {
                    from: &b1,
                    to: &mut b2,
                })
            } else {
                tr(&mut FromTo {
                    from: &b2,
                    to: &mut b1,
                })
            };

            // Go writes the failing step's input to a temp file and wraps the error in a
            // `herrors.FileError` naming it; the port returns the transformer's error (see
            // PORTING.md, deviations).
            res?;
        }

        Ok(if from_is_b1 { b2 } else { b1 })
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/chain.go (124 lines; 4/5 funcs executed)
//   types: Transformer, BytesReader, FromTo, Chain, fromToBuffer
// OK L50-52: New(trs ...Transformer) Chain
// OK L55-57: NewEmpty() Chain
// OK L66-68: (ft fromToBuffer) From() BytesReader
// OK L70-72: (ft fromToBuffer) To() io.Writer
// OK L76-124: (c *Chain) Apply(to io.Writer, from io.Reader) error
// ---------------------------------------------------------------------------
