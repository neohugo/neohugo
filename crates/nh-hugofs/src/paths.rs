//! Port of `hugolib/paths/paths.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

use std::sync::Arc;

use nh_common::Result;
use nh_common::paths::path as hpaths;
use nh_config::config_provider::{AllProvider, config_section};

use crate::fs::Fs;
use crate::modules::module::Modules;

/// Go: `paths.FilePathSeparator`.
pub const FILE_PATH_SEPARATOR: &str = "/";

/// Go: `hugolib/paths.Paths`.
#[derive(Clone)]
pub struct Paths {
    pub fs: Fs,
    pub cfg: Arc<dyn AllProvider>,
    /// Directories to store Resource related artifacts.
    pub abs_resources_dir: String,
    pub abs_publish_dir: String,
    /// When in multihost mode, this returns a list of base paths below PublishDir for each
    /// language.
    pub multihost_target_base_paths: Vec<String>,
}

impl Paths {
    // Go: hugolib/paths/paths.go:New
    pub fn new(fs: Fs, cfg: Arc<dyn AllProvider>) -> Result<Paths> {
        let bcfg = cfg.base_config();
        let publish_dir = &bcfg.publish_dir;
        if publish_dir.is_empty() {
            panic!("publishDir not set");
        }

        let mut abs_publish_dir = hpaths::abs_pathify(&bcfg.working_dir, publish_dir);
        if !abs_publish_dir.ends_with(FILE_PATH_SEPARATOR) {
            abs_publish_dir.push_str(FILE_PATH_SEPARATOR);
        }
        // If root, remove the second '/'
        if abs_publish_dir == "//" {
            abs_publish_dir = FILE_PATH_SEPARATOR.to_string();
        }
        let mut abs_resources_dir =
            hpaths::abs_pathify(&bcfg.working_dir, &cfg.dirs().resource_dir);
        if !abs_resources_dir.ends_with(FILE_PATH_SEPARATOR) {
            abs_resources_dir.push_str(FILE_PATH_SEPARATOR);
        }
        if abs_resources_dir == "//" {
            abs_resources_dir = FILE_PATH_SEPARATOR.to_string();
        }

        let mut multihost_target_base_paths = Vec::new();
        if cfg.is_multihost() && cfg.languages().len() > 1 {
            for l in cfg.languages() {
                multihost_target_base_paths.push(hpaths::to_slash_preserve_leading(&l.lang));
            }
        }

        Ok(Paths {
            fs,
            cfg,
            abs_resources_dir,
            abs_publish_dir,
            multihost_target_base_paths,
        })
    }

    /// Go: `AllModules()` — the `allModules` config section (a `modules.Modules`).
    // Go: hugolib/paths/paths.go:AllModules
    pub fn all_modules(&self) -> Arc<Modules> {
        config_section::<Modules>(self.cfg.as_ref(), "allModules")
    }

    /// Go: `GetBasePath(isRelativeURL)` — any path element in baseURL if needed, with a leading
    /// but no trailing slash.
    // Go: hugolib/paths/paths.go:GetBasePath
    pub fn get_base_path(&self, is_relative_url: bool) -> String {
        if is_relative_url && self.cfg.canonify_urls() {
            // The baseURL will be prepended later.
            return String::new();
        }
        self.cfg.base_url().base_path_no_trailing_slash
    }

    // Go: hugolib/paths/paths.go:Lang
    pub fn lang(&self) -> String {
        self.cfg.language().lang.clone()
    }

    // Go: hugolib/paths/paths.go:GetTargetLanguageBasePath
    pub fn get_target_language_base_path(&self) -> String {
        if self.cfg.is_multihost() {
            // In a multihost configuration all assets will be published below the language code.
            return self.lang();
        }
        self.get_language_prefix()
    }

    // Go: hugolib/paths/paths.go:GetLanguagePrefix
    pub fn get_language_prefix(&self) -> String {
        self.cfg.language_prefix()
    }

    /// Go: `AbsPathify(inPath)` — an absolute path if given a relative path; if already
    /// absolute, the path is just cleaned.
    // Go: hugolib/paths/paths.go:AbsPathify
    pub fn abs_pathify(&self, in_path: &str) -> String {
        hpaths::abs_pathify(&self.cfg.base_config().working_dir, in_path)
    }

    /// Go: `RelPathify(filename)` — trims any WorkingDir prefix from the given filename. If the
    /// filename is not considered to be absolute, the path is just cleaned.
    // Go: hugolib/paths/paths.go:RelPathify
    pub fn rel_pathify(&self, filename: &str) -> String {
        let filename = go_path::filepath::clean(filename);
        if !go_path::filepath::is_abs(&filename) {
            return filename;
        }

        let wd = self.cfg.base_config().working_dir;
        let s = filename.strip_prefix(wd.as_str()).unwrap_or(&filename);
        s.strip_prefix(FILE_PATH_SEPARATOR).unwrap_or(s).to_string()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/paths/paths.go (133 lines; 5/8 funcs executed)
//   types: Paths
// OK L44-83: New(fs *hugofs.Fs, cfg config.AllProvider) (*Paths, error)
// OK L85-87: (p *Paths) AllModules() modules.Modules
// OK L91-97: (p *Paths) GetBasePath(isRelativeURL bool) string
// OK L99-104: (p *Paths) Lang() string
// OK L106-112: (p *Paths) GetTargetLanguageBasePath() string
// OK L114-116: (p *Paths) GetLanguagePrefix() string
// OK L120-122: (p *Paths) AbsPathify(inPath string) string
// OK L126-133: (p *Paths) RelPathify(filename string) string
// ---------------------------------------------------------------------------
