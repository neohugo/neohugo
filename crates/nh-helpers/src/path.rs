//! Port of `helpers/path.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


use std::io::Write;
use std::sync::Arc;

use nh_common::Result;
use nh_hugofs::afero::{File, Fs};

use crate::pathspec::PathSpec;

/// Go: `helpers.FilePathSeparator`.
pub const FILE_PATH_SEPARATOR: &str = "/";

impl PathSpec {
    /// Go: `MakePath(s)` = `paths.Sanitize(s)` (+ RemoveAccents if configured).
    // Go: helpers/path.go:MakePath
    pub fn make_path(&self, s: &str) -> String { todo!() }
    /// Go: `MakePathSanitized(s)` — lower-cased unless `disablePathToLower`.
    // Go: helpers/path.go:MakePathSanitized
    pub fn make_path_sanitized(&self, s: &str) -> String { todo!() }
}

/// Go: `helpers.GetDottedRelativePath(inPath)` (relativeURLs).
// Go: helpers/path.go:GetDottedRelativePath
pub fn get_dotted_relative_path(in_path: &str) -> String { todo!() }

/// Go: `helpers.MakeTitle(inpath)`.
pub fn make_title(in_path: &str) -> String { todo!() }

/// Go: `helpers.OpenFileForWriting(fs, filename)` — Create (truncate, 0666&^umask), MkdirAll(0777)
/// of the parent on ENOENT and retry.
// Go: helpers/path.go:OpenFileForWriting
pub fn open_file_for_writing(fs: &dyn Fs, filename: &str) -> Result<Box<dyn File>> { todo!() }

/// Go: `helpers.OpenFilesForWriting(fs, filenames...)` — a multi-writer.
// Go: helpers/path.go:OpenFilesForWriting
pub fn open_files_for_writing(fs: &dyn Fs, filenames: &[String]) -> Result<Vec<Box<dyn File>>> { todo!() }

/// Go: `helpers.GetCacheDir(fs, cacheDir)`: `cacheDir` config, `$HUGO_CACHEDIR`, else
/// `os.UserCacheDir()/hugo_cache` (`~/Library/Caches/hugo_cache` on macOS).
// Go: helpers/path.go:GetCacheDir
pub fn get_cache_dir(cache_dir: &str) -> Result<String> { todo!() }

/// Go: `helpers.WriteToDisk(inpath, r, fs)`.
pub fn write_to_disk(in_path: &str, data: &[u8], fs: &dyn Fs) -> Result<()> { todo!() }

/// Go: `helpers.Exists`, `IsDir`, `DirExists`, `IsEmpty`.
pub fn exists(path: &str, fs: &dyn Fs) -> Result<bool> { todo!() }

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/path.go (428 lines; 8/23 funcs executed)
//   types: NamedSlice
// EX L43-49: (p *PathSpec) MakePath(s string) string
//    L52-56: (p *PathSpec) MakePathsSanitized(paths []string)
// EX L59-64: (p *PathSpec) MakePathSanitized(s string) string
//    L68-70: MakeTitle(inpath string) string
//    L73-80: MakePathRelative(inPath string, possibleDirectories ...string) (string, error)
//    L87-117: GetDottedRelativePath(inPath string) string
//    L124-129: (n NamedSlice) String() string
//    L131-218: ExtractAndGroupRootPaths(paths []string) []NamedSlice
//    L224-238: ExtractRootPaths(paths []string) []string
//    L242-261: FindCWD() (string, error)
// EX L265-276: Walk(fs afero.Fs, root string, walker hugofs.WalkFunc) error
//    L280-282: SafeWriteToDisk(inpath string, r io.Reader, fs afero.Fs) (err error)
//    L285-287: WriteToDisk(inpath string, r io.Reader, fs afero.Fs) (err error)
// EX L290-304: OpenFilesForWriting(fs afero.Fs, filenames ...string) (io.WriteCloser, error)
// EX L308-324: OpenFileForWriting(fs afero.Fs, filename string) (afero.File, error)
// EX L328-365: GetCacheDir(fs afero.Fs, cacheDir string) (string, error)
// EX L367-388: cacheDirDefault(cacheDir string) string
//    L390-395: addTrailingFileSeparator(s string) string
//    L398-400: GetTempDir(subPath string, fs afero.Fs) string
//    L403-405: DirExists(path string, fs afero.Fs) (bool, error)
//    L408-410: IsDir(path string, fs afero.Fs) (bool, error)
//    L413-423: IsEmpty(path string, fs afero.Fs) (bool, error)
// EX L426-428: Exists(path string, fs afero.Fs) (bool, error)
// ---------------------------------------------------------------------------
