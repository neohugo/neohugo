//! Port of `hugolib/paths/paths.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


use std::sync::Arc;

use nh_config::config_provider::AllProvider;
use nh_common::Result;

use crate::fs::Fs;
use crate::modules::module::Modules;

/// Go: `hugolib/paths.Paths`.
#[derive(Clone)]
pub struct Paths {
    pub fs: Fs,
    pub cfg: Arc<dyn AllProvider>,
    pub abs_resources_dir: String,
    pub abs_publish_dir: String,
    /// Multihost only.
    pub multihost_target_base_paths: Vec<String>,
}

impl Paths {
    // Go: hugolib/paths/paths.go:New
    pub fn new(fs: Fs, cfg: Arc<dyn AllProvider>) -> Result<Paths> {
        todo!()
    }

    // Go: hugolib/paths/paths.go:AllModules
    pub fn all_modules(&self) -> Arc<Modules> {
        todo!()
    }

    // Go: hugolib/paths/paths.go:GetBasePath
    pub fn get_base_path(&self, is_relative_url: bool) -> String {
        todo!()
    }

    // Go: hugolib/paths/paths.go:Lang
    pub fn lang(&self) -> String {
        todo!()
    }

    // Go: hugolib/paths/paths.go:GetTargetLanguageBasePath
    pub fn get_target_language_base_path(&self) -> String {
        todo!()
    }

    // Go: hugolib/paths/paths.go:GetLanguagePrefix
    pub fn get_language_prefix(&self) -> String {
        todo!()
    }

    // Go: hugolib/paths/paths.go:AbsPathify
    pub fn abs_pathify(&self, in_path: &str) -> String {
        todo!()
    }

    // Go: hugolib/paths/paths.go:RelPathify
    pub fn rel_pathify(&self, filename: &str) -> String {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/paths/paths.go (133 lines; 5/8 funcs executed)
//   types: Paths
// EX L44-83: New(fs *hugofs.Fs, cfg config.AllProvider) (*Paths, error)
// EX L85-87: (p *Paths) AllModules() modules.Modules
// EX L91-97: (p *Paths) GetBasePath(isRelativeURL bool) string
// EX L99-104: (p *Paths) Lang() string
//    L106-112: (p *Paths) GetTargetLanguageBasePath() string
// EX L114-116: (p *Paths) GetLanguagePrefix() string
//    L120-122: (p *Paths) AbsPathify(inPath string) string
//    L126-133: (p *Paths) RelPathify(filename string) string
// ---------------------------------------------------------------------------
