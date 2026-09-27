//! Port of `hugolib/doctree/dimensions.go`.
//!
//! Owner: Wave B task T27 (doctree).


/// Go: `doctree.DimensionLanguage`.
pub const DIMENSION_LANGUAGE: usize = 0;

/// Go: `doctree.Dimension` (`[1]int`: the language index).
pub type Dimension = [usize; 1];

/// Go: `doctree.DimensionFlag`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DimensionFlag(pub u8);

impl DimensionFlag {
    /// Go: `DimensionLanguage.Flag()`.
    pub const LANGUAGE: DimensionFlag = DimensionFlag(1);

    // Go: hugolib/doctree/dimensions.go:Has
    pub fn has(self, o: DimensionFlag) -> bool {
        self.0 & o.0 == o.0
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/dimensions.go (43 lines; 2/3 funcs executed)
//   types: Dimension, DimensionFlag
// EX L28-30: (d DimensionFlag) Has(o DimensionFlag) bool
//    L33-35: (d DimensionFlag) Set(o DimensionFlag) DimensionFlag
// EX L38-43: (d DimensionFlag) Index() int
// ---------------------------------------------------------------------------
