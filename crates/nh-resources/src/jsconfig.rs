//! Port of `resources/jsconfig/jsconfig.go`.
//!
//! Owner: Wave B task T14 (resources-core).


use std::collections::BTreeSet;
use std::sync::Mutex;

/// Go: `jsconfig.Builder` — records Hugo-resolved import roots; if non-empty, postProcess writes
/// `assets/jsconfig.json` (not the case for seeksnack).
#[derive(Default)]
pub struct Builder {
    pub(crate) source_roots: Mutex<BTreeSet<String>>,
}

impl Builder {
    // Go: resources/jsconfig/jsconfig.go:AddSourceRoot
    pub fn add_source_root(&self, root: &str) {
        self.source_roots.lock().unwrap().insert(root.to_string());
    }

    /// Go: `Build(dir)` — `None` when there are no roots.
    // Go: resources/jsconfig/jsconfig.go:Build
    pub fn build(&self, dir: &str) -> Option<Vec<u8>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/jsconfig/jsconfig.go (92 lines; 2/4 funcs executed)
//   types: Builder, CompilerOptions, Config
// EX L30-32: NewBuilder() *Builder
// EX L36-56: (b *Builder) Build(dir string) *Config
//    L60-72: (b *Builder) AddSourceRoot(root string)
//    L85-92: newJSConfig() *Config
// ---------------------------------------------------------------------------
