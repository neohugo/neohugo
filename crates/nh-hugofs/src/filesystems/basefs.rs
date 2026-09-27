//! Port of `hugolib/filesystems/basefs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


//! Go `hugolib/filesystems.BaseFs`: assembles the component filesystems from module mounts
//! (RootMappingFs per module, overlays, ComponentFs views). The `assets` view resolves
//! `/vendor/...` through the `node_modules -> assets/vendor` mount; `_jsconfig` files are mounted
//! under `assets/_jsconfig`.

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::Result;
use nh_config::config_provider::AllProvider;

use crate::afero::Fs;
use crate::overlayfs::OverlayFs;
use crate::paths::Paths;
use crate::rootmapping_fs::{ComponentPath, RootMappingFs};

/// Go: `filesystems.SourceFilesystem`.
#[derive(Clone)]
pub struct SourceFilesystem {
    /// Name matches one of the top level folders (i.e. "content", "assets").
    pub name: String,
    /// The component view (a ComponentFs over the overlay).
    pub fs: Arc<dyn Fs>,
    /// The source filesystem (usually the OS filesystem).
    pub source_fs: Arc<dyn Fs>,
    /// When syncing a source folder to the target (e.g. /public), this may be set to publish into a subfolder.
    pub publish_folder: String,
}

impl SourceFilesystem {
    /// Go: `MakePathRelative(filename, checkExists)` — real filename -> path relative to the component.
    // Go: hugolib/filesystems/basefs.go:MakePathRelative
    pub fn make_path_relative(&self, filename: &str, check_exists: bool) -> Option<String> {
        todo!()
    }

    // Go: hugolib/filesystems/basefs.go:ReverseLookup
    pub fn reverse_lookup(&self, filename: &str, check_exists: bool) -> Result<Vec<ComponentPath>> {
        todo!()
    }

    /// Go: `RealFilename(rel)`.
    // Go: hugolib/filesystems/basefs.go:RealFilename
    pub fn real_filename(&self, rel: &str) -> String {
        todo!()
    }

    /// Go: `RealDirs(from)` — for each mount dir: join(mountRealDir, from) if it exists
    /// (libsass include paths).
    // Go: hugolib/filesystems/basefs.go:RealDirs
    pub fn real_dirs(&self, from: &str) -> Vec<String> {
        todo!()
    }

    // Go: hugolib/filesystems/basefs.go:Contains
    pub fn contains(&self, filename: &str) -> bool {
        todo!()
    }
}

/// Go: `filesystems.SourceFilesystems`.
#[derive(Clone)]
pub struct SourceFilesystems {
    pub content: Arc<SourceFilesystem>,
    pub data: Arc<SourceFilesystem>,
    pub i18n: Arc<SourceFilesystem>,
    pub layouts: Arc<SourceFilesystem>,
    pub archetypes: Arc<SourceFilesystem>,
    pub assets: Arc<SourceFilesystem>,
    pub assets_with_duplicates_preserved: Arc<SourceFilesystem>,
    pub root_fss: Vec<Arc<RootMappingFs>>,
    /// Writable filesystem on top the project's resources directory.
    pub resources_cache: Arc<dyn Fs>,
    /// The work folder (may be a composite of project and theme components).
    pub work: Arc<dyn Fs>,
    /// When in multihost we have one static filesystem per language (key "" otherwise).
    pub static_: BTreeMap<String, Arc<SourceFilesystem>>,
}

impl SourceFilesystems {
    // Go: hugolib/filesystems/basefs.go:StaticFs
    pub fn static_fs(&self, lang: &str) -> Arc<dyn Fs> {
        todo!()
    }

    // Go: hugolib/filesystems/basefs.go:IsContent
    pub fn is_content(&self, filename: &str) -> bool {
        todo!()
    }
}

/// Go: `filesystems.BaseFs`.
#[derive(Clone)]
pub struct BaseFs {
    pub source_filesystems: Arc<SourceFilesystems>,
    /// The source filesystem (needs absolute filenames).
    pub source_fs: Arc<dyn Fs>,
    /// The project source.
    pub project_source_fs: Arc<dyn Fs>,
    /// The filesystem used to publish the rendered site (below the publish dir).
    pub publish_fs: Arc<dyn Fs>,
    /// The filesystem used for static files.
    pub publish_fs_static: Arc<dyn Fs>,
    /// A read-only filesystem starting from the project workDir.
    pub work_dir: Arc<dyn Fs>,
    pub(crate) the_big_fs: Arc<FilesystemsCollector>,
    pub(crate) working_dir: String,
}

/// Go: `filesystemsCollector`.
pub struct FilesystemsCollector {
    pub(crate) source_project: Arc<dyn Fs>,
    pub(crate) source_modules: Arc<dyn Fs>,
    pub(crate) overlay_mounts: OverlayFs,
    pub(crate) overlay_mounts_content: OverlayFs,
    pub(crate) overlay_mounts_static: OverlayFs,
    pub(crate) overlay_mounts_full: OverlayFs,
    pub(crate) overlay_full: OverlayFs,
    pub(crate) overlay_resources: OverlayFs,
    pub(crate) root_fss: Vec<Arc<RootMappingFs>>,
    pub(crate) static_per_language: BTreeMap<String, OverlayFs>,
}

impl BaseFs {
    /// Go: `filesystems.NewBase(p, logger, options...)`.
    // Go: hugolib/filesystems/basefs.go:NewBase
    pub fn new(p: &Paths) -> Result<Arc<BaseFs>> {
        todo!()
    }

    /// Go: `LockBuild()` — `.hugo_build.lock` file lock in the working dir (skipped with noBuildLock).
    // Go: hugolib/filesystems/basefs.go:LockBuild
    pub fn lock_build(&self) -> Result<Box<dyn FnOnce() + Send>> {
        todo!()
    }

    /// Go: `ResolveJSConfigFile(name)` — `assets/_jsconfig/<name>` real filename, else working dir.
    // Go: hugolib/filesystems/basefs.go:ResolveJSConfigFile
    pub fn resolve_js_config_file(&self, name: &str) -> String {
        todo!()
    }

    /// Go: `AbsProjectContentDir(filename)`.
    // Go: hugolib/filesystems/basefs.go:AbsProjectContentDir
    pub fn abs_project_content_dir(&self, filename: &str) -> Result<(String, String)> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/filesystems/basefs.go (852 lines; 16/29 funcs executed)
//   types: BaseFs, Lockable, fakeLockfileMutex, SourceFilesystems, SourceFilesystem, sourceFilesystemsBuilder,
//          filesystemsCollector, mountsDescriptor
//    L95-98: (f *fakeLockfileMutex) Lock() (func(), error)
// EX L101-103: (b *BaseFs) LockBuild() (unlock func(), err error)
//    L105-150: (b *BaseFs) WatchFilenames() []string
//    L152-161: (b *BaseFs) mountsForComponent(component string) []hugofs.FileMetaInfo
//    L165-208: (b *BaseFs) AbsProjectContentDir(filename string) (string, string, error)
// EX L212-225: (b *BaseFs) ResolveJSConfigFile(name string) string
//    L279-289: (s SourceFilesystems) StaticFs(lang string) afero.Fs
//    L297-307: (s SourceFilesystems) StatResource(lang, filename string) (fi os.FileInfo, fs afero.Fs, err error)
//    L311-318: (s SourceFilesystems) IsStatic(filename string) bool
//    L321-323: (s SourceFilesystems) IsContent(filename string) bool
//    L326-336: (s *SourceFilesystems) ResolvePaths(filename string) []hugofs.ComponentPath
//    L340-348: (s SourceFilesystems) MakeStaticPathRelative(filename string) string
// EX L351-361: (d *SourceFilesystem) MakePathRelative(filename string, checkExists bool) (string, bool)
// EX L364-385: (d *SourceFilesystem) ReverseLookup(filename string, checkExists bool) ([]hugofs.ComponentPath, error)
// EX L387-411: (d *SourceFilesystem) mounts() []hugofs.FileMetaInfo
//    L413-423: (d *SourceFilesystem) RealFilename(rel string) string
//    L426-436: (d *SourceFilesystem) Contains(filename string) bool
// EX L440-452: (d *SourceFilesystem) RealDirs(from string) []string
// EX L456-462: WithBaseFs(b *BaseFs) func(*BaseFs) error
// EX L465-513: NewBase(p *paths.Paths, logger loggers.Logger, options ...func(*BaseFs) error) (*BaseFs, error)
// EX L523-531: newSourceFilesystemsBuilder(p *paths.Paths, logger loggers.Logger, b *BaseFs) *sourceFilesystemsBuilder
// EX L533-539: (b *sourceFilesystemsBuilder) newSourceFilesystem(name string, fs afero.Fs) *SourceFilesystem
// EX L541-609: (b *sourceFilesystemsBuilder) Build() (*SourceFilesystems, error)
// EX L611-654: (b *sourceFilesystemsBuilder) createMainOverlayFs(p *paths.Paths) (*filesystemsCollector, error)
// EX L656-658: (b *sourceFilesystemsBuilder) isContentMount(mnt modules.Mount) bool
// EX L660-662: (b *sourceFilesystemsBuilder) isStaticMount(mnt modules.Mount) bool
// EX L664-802: (b *sourceFilesystemsBuilder) createOverlayFs( collector *filesystemsCollector, mounts []mountsDescriptor, ) error
//    L805-824: printFs(fs afero.Fs, path string, w io.Writer)
// EX L843-845: (c *filesystemsCollector) addRootFs(rfs *hugofs.RootMappingFs)
// ---------------------------------------------------------------------------
