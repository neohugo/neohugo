//! Port of `hugolib/doctree/dimensions.go`.
//!
//! Owner: Wave B task T27 (doctree).

/// Go: `doctree.DimensionLanguage.Index()` (the language's index in a [`Dimension`]).
pub const DIMENSION_LANGUAGE: usize = 0;

/// Go: `doctree.Dimension` (`[1]int`: the language index).
pub type Dimension = [usize; 1];

/// Go: `doctree.DimensionFlag`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DimensionFlag(pub u8);

impl DimensionFlag {
    /// Go: `DimensionLanguage` (`1 << iota`: language is currently the only dimension).
    pub const LANGUAGE: DimensionFlag = DimensionFlag(1);

    /// Go: `Has(o)` — whether the given flag is set.
    // Go: hugolib/doctree/dimensions.go:Has
    pub fn has(self, o: DimensionFlag) -> bool {
        self.0 & o.0 == o.0
    }

    /// Go: `Set(o)`.
    // Go: hugolib/doctree/dimensions.go:Set
    pub fn set(self, o: DimensionFlag) -> DimensionFlag {
        DimensionFlag(self.0 | o.0)
    }

    /// Go: `Index()` — this flag's index in the [`Dimension`] array. Panics like Go
    /// ("dimension flag not set") for the zero flag; Hugo only calls it on `DimensionLanguage`.
    // Go: hugolib/doctree/dimensions.go:Index
    pub fn index(self) -> usize {
        if self.0 == 0 {
            panic!("dimension flag not set");
        }
        (self.0 - 1) as usize
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/doctree/dimensions.go (43 lines; 2/3 funcs executed)
//   types: Dimension, DimensionFlag
// OK L28-30: (d DimensionFlag) Has(o DimensionFlag) bool
// OK L33-35: (d DimensionFlag) Set(o DimensionFlag) DimensionFlag
// OK L38-43: (d DimensionFlag) Index() int
// ---------------------------------------------------------------------------
