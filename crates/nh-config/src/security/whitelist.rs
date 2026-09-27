//! Port of `config/security/whitelist.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use nh_common::Result;

/// Go: `security.Whitelist` — regexp patterns (RE2), or `none`.
#[derive(Clone, Debug, Default)]
pub struct Whitelist {
    pub acceptnone: bool,
    /// Source patterns (Wave B: compiled with the `regex` crate; check they are RE2-compatible).
    pub patterns: Vec<String>,
    pub patterns_strings: Vec<String>,
}

impl Whitelist {
    // Go: config/security/whitelist.go:NewWhitelist
    pub fn new(patterns: &[&str]) -> Result<Whitelist> { todo!() }
    // Go: config/security/whitelist.go:Accept
    pub fn accept(&self, name: &str) -> bool { todo!() }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/security/whitelist.go (116 lines; 3/5 funcs executed)
//   types: Whitelist
//    L37-43: (w Whitelist) MarshalJSON() ([]byte, error)
// EX L48-89: NewWhitelist(patterns ...string) (Whitelist, error)
// EX L92-98: MustNewWhitelist(patterns ...string) Whitelist
// EX L101-112: (w Whitelist) Accept(name string) bool
//    L114-116: (w Whitelist) String() string
// ---------------------------------------------------------------------------
