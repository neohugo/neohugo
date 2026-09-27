//! Port of `transform/metainject/hugogenerator.go`.
//!
//! STUB (never active: neohugo inverted the flag)
//!
//! Owner: Wave B task T07 (transform-publisher).


/// Go: `metainject.HugoGenerator` — never active in neohugo builds (hugolib sets
/// `AddHugoGeneratorTag = DisableHugoGeneratorInject`, i.e. false by default). Do not inject.
pub fn hugo_generator() -> Option<crate::chain::Transformer> {
    None
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/metainject/hugogenerator.go (56 lines; 0/1 funcs executed)
//    L32-56: HugoGenerator(ft transform.FromTo) error
// ---------------------------------------------------------------------------
