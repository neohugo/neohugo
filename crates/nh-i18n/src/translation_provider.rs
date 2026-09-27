//! Port of `langs/i18n/translationProvider.go`.
//!
//! Owner: Wave B task T17 (i18n).


//! Go `langs/i18n/translationProvider.go`: loads every file of the i18n component (Walkway,
//! sorted by name) into a go-i18n Bundle (default language tag first), then sets `Deps.translate`
//! for each site.

use std::sync::Arc;

use nh_common::Result;
use nh_deps::deps::Deps;

/// Go: `i18n.TranslationProvider`.
#[derive(Default)]
pub struct TranslationProvider {
    pub(crate) t: std::sync::OnceLock<Arc<crate::i18n::Translator>>,
}

impl TranslationProvider {
    // Go: langs/i18n/translationProvider.go:NewTranslationProvider
    pub fn new() -> Self {
        Self::default()
    }

    /// Go: `NewResource(dst)` — load i18n files and set `dst.Translate`.
    // Go: langs/i18n/translationProvider.go:NewResource
    pub fn new_resource(&self, dst: &Deps) -> Result<()> {
        todo!()
    }

    /// Go: `CloneResource(dst, src)` — other sites reuse the translator.
    // Go: langs/i18n/translationProvider.go:CloneResource
    pub fn clone_resource(&self, dst: &Deps, src: &Deps) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: langs/i18n/translationProvider.go (139 lines; 4/5 funcs executed)
//   types: TranslationProvider
// EX L43-45: NewTranslationProvider() *TranslationProvider
// EX L48-82: (tp *TranslationProvider) NewResource(dst *deps.Deps) error
// EX L86-121: addTranslationFile(bundle *i18n.Bundle, r *source.File) error
// EX L124-127: (tp *TranslationProvider) CloneResource(dst, src *deps.Deps) error
//    L129-139: errWithFileContext(inerr error, r *source.File) error
// ---------------------------------------------------------------------------
