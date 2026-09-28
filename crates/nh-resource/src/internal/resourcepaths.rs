//! Port of `resources/internal/resourcepaths.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).

/// Go: `internal.ResourcePaths` — the target path parts of a resource. All directories have
/// Unix-style slashes, with leading slash, but no trailing slash; empty directories are "".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourcePaths {
    /// This is the directory component for the target file or link.
    pub dir: String,
    /// Any base directory for the target file. Will be prepended to Dir.
    pub base_dir_target: String,
    /// This is the directory component for the link will be prepended to Dir.
    pub base_dir_link: String,
    /// Set when publishing in a multihost setup.
    pub target_base_paths: Vec<String>,
    /// This is the File component, e.g. "data.json".
    pub file: String,
}

impl ResourcePaths {
    // Go: resources/internal/resourcepaths.go:join
    fn join(p: &[&str]) -> String {
        let mut s = String::new();
        for (i, pp) in p.iter().enumerate() {
            if pp.is_empty() {
                continue;
            }
            if i > 0 && !pp.starts_with('/') {
                s.push('/');
            }
            s.push_str(pp);
        }
        if !s.starts_with('/') {
            s.insert(0, '/');
        }
        s
    }

    // Go: resources/internal/resourcepaths.go:TargetLink
    pub fn target_link(&self) -> String {
        Self::join(&[&self.base_dir_link, &self.dir, &self.file])
    }

    // Go: resources/internal/resourcepaths.go:TargetPath
    pub fn target_path(&self) -> String {
        Self::join(&[&self.base_dir_target, &self.dir, &self.file])
    }

    // Go: resources/internal/resourcepaths.go:Path
    pub fn path(&self) -> String {
        Self::join(&[&self.dir, &self.file])
    }

    // Go: resources/internal/resourcepaths.go:TargetPaths
    pub fn target_paths(&self) -> Vec<String> {
        if self.target_base_paths.is_empty() {
            return vec![self.target_path()];
        }
        self.target_base_paths
            .iter()
            .map(|p| format!("{p}{}", self.target_path()))
            .collect()
    }

    /// Go: `TargetFilenames()` — `TargetPaths` with OS separators (unix: unchanged).
    // Go: resources/internal/resourcepaths.go:TargetFilenames
    pub fn target_filenames(&self) -> Vec<String> {
        self.target_paths()
            .into_iter()
            .map(|p| go_path::filepath::from_slash(&p).to_string())
            .collect()
    }

    // Go: resources/internal/resourcepaths.go:FromTargetPath
    pub fn from_target_path(&self, target_path: &str) -> ResourcePaths {
        let target_path = go_path::filepath::to_slash(target_path).to_string();
        let (dir, file) = go_path::path::split(&target_path);
        let mut dir = nh_common::paths::path::to_slash_preserve_leading(dir);
        if dir == "/" {
            dir = String::new();
        }
        let mut d = self.clone();
        d.dir = dir;
        d.file = file.to_string();
        d.base_dir_link = String::new();
        d.base_dir_target = String::new();

        d
    }
}

/// Convenience constructor (no Go counterpart; Go builds the struct literal): the base
/// directories and multihost base paths, with `Dir`/`File` split from `target_path` like
/// [`ResourcePaths::from_target_path`].
pub fn new_resource_paths(
    target_path: &str,
    base_dir_target: &str,
    base_dir_link: &str,
    target_base_paths: &[String],
) -> ResourcePaths {
    let mut d = ResourcePaths {
        target_base_paths: target_base_paths.to_vec(),
        ..Default::default()
    }
    .from_target_path(target_path);
    d.base_dir_target = base_dir_target.to_string();
    d.base_dir_link = base_dir_link.to_string();
    d
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/internal/resourcepaths.go (107 lines; 6/7 funcs executed)
//   types: ResourcePaths
// OK L44-60: (d ResourcePaths) join(p ...string) string
// OK L62-64: (d ResourcePaths) TargetLink() string
// OK L66-68: (d ResourcePaths) TargetPath() string
// OK L70-72: (d ResourcePaths) Path() string
// OK L74-84: (d ResourcePaths) TargetPaths() []string
// OK L86-92: (d ResourcePaths) TargetFilenames() []string
// OK L94-107: (d ResourcePaths) FromTargetPath(targetPath string) ResourcePaths
// ---------------------------------------------------------------------------
