//! Port of `helpers/path.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).

use std::io::{Read, Write};
use std::sync::Arc;

use go_path::{filepath, path};
use go_unicode::utf8;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::hugio::{BoxWriter, MultiWriteCloser, new_multi_write_closer};
use nh_hugofs::afero::{File, Fs, OsFs};
use nh_hugofs::walk::{WalkFunc, Walkway, WalkwayConfig};

use crate::general::{unique_strings, unique_strings_sorted};
use crate::pathspec::PathSpec;

/// Go: `helpers.FilePathSeparator`.
pub const FILE_PATH_SEPARATOR: &str = "/";

impl PathSpec {
    /// Go: `MakePath(s)` = `paths.Sanitize(s)` (+ `text.RemoveAccentsString` if
    /// `removePathAccents` is set).
    // Go: helpers/path.go:MakePath
    pub fn make_path(&self, s: &str) -> String {
        let mut s = nh_common::paths::path::sanitize(s);
        if self.cfg.remove_path_accents() {
            s = nh_common::text::remove_accents_string(&s);
        }
        s
    }

    /// [`PathSpec::make_path`] as a `Result` (kept for callers; it never fails).
    // Go: helpers/path.go:MakePath
    pub fn try_make_path(&self, s: &str) -> Result<String> {
        Ok(self.make_path(s))
    }

    /// Go: `MakePathsSanitized(paths)` — applies `MakePathSanitized` to every item in place.
    // Go: helpers/path.go:MakePathsSanitized
    pub fn make_paths_sanitized(&self, paths: &mut [String]) {
        for p in paths.iter_mut() {
            *p = self.make_path_sanitized(p);
        }
    }

    /// Go: `MakePathSanitized(s)` — lower-cased unless `disablePathToLower`.
    // Go: helpers/path.go:MakePathSanitized
    pub fn make_path_sanitized(&self, s: &str) -> String {
        if self.cfg.disable_path_to_lower() {
            return self.make_path(s);
        }
        go_unicode::strings::to_lower_str(&self.make_path(s)).into_owned()
    }

    /// [`PathSpec::make_path_sanitized`] as a `Result` (kept for callers; it never fails).
    // Go: helpers/path.go:MakePathSanitized
    pub fn try_make_path_sanitized(&self, s: &str) -> Result<String> {
        Ok(self.make_path_sanitized(s))
    }
}

/// Go: `helpers.MakeTitle(inpath)` — trims whitespace and replaces hyphens with spaces.
// Go: helpers/path.go:MakeTitle
pub fn make_title(in_path: &str) -> String {
    go_unicode::strings::trim_space_str(in_path).replace('-', " ")
}

/// Go: `helpers.MakePathRelative(inPath, possibleDirectories...)`.
// Go: helpers/path.go:MakePathRelative
pub fn make_path_relative(in_path: &str, possible_directories: &[&str]) -> (String, Option<Error>) {
    for current_path in possible_directories {
        if let Some(rest) = in_path.strip_prefix(current_path) {
            return (rest.to_string(), None);
        }
    }
    (
        in_path.to_string(),
        Some(Error::new("can't extract relative path, unknown prefix")),
    )
}

/// Go: `isFileRe = regexp.MustCompile(`.*\..{1,6}$`)` — true when a `.` is followed by 1 to 6
/// runes (none of them `\n`) up to the end of the string. Invalid UTF-8 bytes are one rune each,
/// as in Go's regexp.
// Go: helpers/path.go:isFileRe
fn is_file_re_match(s: &[u8]) -> bool {
    let mut end = s.len();
    for _ in 0..6 {
        if end == 0 {
            return false;
        }
        let (r, n) = utf8::decode_last_rune(&s[..end]);
        if r == '\n' as i32 {
            return false;
        }
        end -= n;
        // The runes s[end..] are the 1..6 runes after the candidate dot.
        if end > 0 && s[end - 1] == b'.' {
            return true;
        }
    }
    false
}

/// Go: `helpers.GetDottedRelativePath(inPath)` (relativeURLs).
// Go: helpers/path.go:GetDottedRelativePath
pub fn get_dotted_relative_path(in_path: &str) -> String {
    let mut in_path = path::clean(filepath::to_slash(in_path));

    if in_path == "." {
        return "./".to_string();
    }

    if !is_file_re_match(in_path.as_bytes()) && !in_path.ends_with('/') {
        in_path.push('/');
    }

    if !in_path.starts_with('/') {
        in_path.insert(0, '/');
    }

    let (dir, _) = path::split(&in_path);

    let section_count = dir.matches('/').count();

    if section_count == 0 || dir == "/" {
        return "./".to_string();
    }

    let mut dotted_path = String::new();
    for _ in 1..section_count {
        dotted_path.push_str("../");
    }
    dotted_path
}

/// Go: `helpers.NamedSlice`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NamedSlice {
    pub name: String,
    /// `None` = Go's nil slice.
    pub slice: Option<Vec<String>>,
}

impl NamedSlice {
    // Go: helpers/path.go:(NamedSlice).String
    pub fn string(&self) -> String {
        match &self.slice {
            None => self.name.clone(),
            Some(s) if s.is_empty() => self.name.clone(),
            Some(s) => format!("{}{}{{{}}}", self.name, FILE_PATH_SEPARATOR, s.join(",")),
        }
    }
}

/// Go: `helpers.ExtractAndGroupRootPaths(paths)` (`None` = nil result).
// Go: helpers/path.go:ExtractAndGroupRootPaths
pub fn extract_and_group_root_paths(paths: &[String]) -> Option<Vec<NamedSlice>> {
    if paths.is_empty() {
        return None;
    }

    let had_slash_prefix = paths[0].starts_with(FILE_PATH_SEPARATOR);

    let mut paths_copy: Vec<String> = paths
        .iter()
        .map(|p| filepath::to_slash(p).trim_matches('/').to_string())
        .collect();

    paths_copy.sort();

    let paths_parts: Vec<Vec<&str>> = paths_copy.iter().map(|p| p.split('/').collect()).collect();

    let mut groups: Vec<Vec<&str>> = Vec::new();

    for (i, p1) in paths_parts.iter().enumerate() {
        let mut c1: i64 = -1;

        for (j, p2) in paths_parts.iter().enumerate() {
            if i == j {
                continue;
            }

            let mut c2: i64 = -1;

            for (k, v) in p1.iter().enumerate() {
                if k >= p2.len() {
                    break;
                }
                if *v != p2[k] {
                    break;
                }
                c2 = k as i64;
            }

            if c1 == -1 || (c2 != -1 && c2 < c1) {
                c1 = c2;
            }
        }

        if c1 != -1 {
            groups.push(p1[..(c1 + 1) as usize].to_vec());
        } else {
            groups.push(p1.clone());
        }
    }

    let groups_str: Vec<String> = groups.iter().map(|g| g.join("/")).collect();
    let groups_str = unique_strings_sorted(groups_str).unwrap_or_default();

    let mut result = Vec::new();

    for g in groups_str {
        let mut name = filepath::from_slash(&g).to_string();
        if had_slash_prefix {
            name.insert_str(0, FILE_PATH_SEPARATOR);
        }
        let mut slice: Option<Vec<String>> = None;
        for p in &paths_copy {
            let Some(rest) = p.strip_prefix(g.as_str()) else {
                continue;
            };
            if !rest.is_empty() {
                slice.get_or_insert_with(Vec::new).push(rest.to_string());
            }
        }

        // ns.Slice = UniqueStrings(ExtractRootPaths(ns.Slice)): never nil after this.
        let s = slice.unwrap_or_default();
        let ns = NamedSlice {
            name,
            slice: Some(unique_strings(&extract_root_paths(&s))),
        };
        result.push(ns);
    }

    Some(result)
}

/// Go: `helpers.ExtractRootPaths(paths)` — `/content/section/` becomes `content`.
// Go: helpers/path.go:ExtractRootPaths
pub fn extract_root_paths(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .map(|p| {
            let root = filepath::to_slash(p);
            for section in root.split('/') {
                if !section.is_empty() {
                    return section.to_string();
                }
            }
            root.to_string()
        })
        .collect()
}

/// Go: `helpers.FindCWD()` — the directory of the running executable (symlinks resolved).
// Go: helpers/path.go:FindCWD
pub fn find_cwd() -> Result<String> {
    let arg0 = std::env::args().next().unwrap_or_default();
    let server_file = go_path_abs(&arg0)
        .map_err(|e| Error::new(format!("can't get absolute path for executable: {e}")))?;

    let mut path = filepath::dir(&server_file);
    let mut real = std::fs::canonicalize(&server_file)
        .ok()
        .map(|p| p.to_string_lossy().into_owned());
    if real.is_none() && std::fs::metadata(format!("{server_file}.exe")).is_ok() {
        real = Some(filepath::clean(&format!("{server_file}.exe")));
    }
    if let Some(real_file) = real
        && real_file != server_file
    {
        path = filepath::dir(&real_file);
    }

    Ok(path)
}

/// Go `filepath.Abs` (unix): join with the working directory and clean.
fn go_path_abs(p: &str) -> std::result::Result<String, String> {
    if filepath::is_abs(p) {
        return Ok(filepath::clean(p));
    }
    let wd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(filepath::join(&[wd.to_string_lossy().as_ref(), p]))
}

/// Go: `helpers.Walk(fs, root, walker)` — an `*afero.OsFs` is wrapped in a base file decorator.
// Go: helpers/path.go:Walk
pub fn walk(fs: Arc<dyn Fs>, root: &str, walker: WalkFunc<'_>) -> Result<()> {
    let fs = if fs.as_any().is::<OsFs>() {
        nh_hugofs::decorators::new_base_file_decorator(fs, Vec::new())
    } else {
        fs
    };
    let mut cfg = WalkwayConfig::new(fs, walker);
    cfg.root = root.to_string();
    let mut w = Walkway::new(cfg);
    w.walk()
}

/// Go: `helpers.SafeWriteToDisk(inpath, r, fs)` = `afero.SafeWriteReader`.
// Go: helpers/path.go:SafeWriteToDisk
pub fn safe_write_to_disk(in_path: &str, r: &mut dyn Read, fs: &dyn Fs) -> Result<()> {
    let (dir, _) = filepath::split(in_path);
    let ospath = filepath::from_slash(dir);

    if !ospath.is_empty() {
        fs.mkdir_all(ospath, 0o777)?;
    }

    if nh_hugofs::afero::exists(fs, in_path)? {
        return Err(Error::new(format!("{in_path} already exists")));
    }

    let mut file = fs.create(in_path)?;
    let res = copy(r, &mut *file);
    let _ = file.close();
    res
}

/// Go: `helpers.WriteToDisk(inpath, r, fs)` = `afero.WriteReader`.
// Go: helpers/path.go:WriteToDisk
pub fn write_to_disk(in_path: &str, r: &mut dyn Read, fs: &dyn Fs) -> Result<()> {
    let (dir, _) = filepath::split(in_path);
    let ospath = filepath::from_slash(dir);

    if !ospath.is_empty() {
        // Go compares with os.ErrExist by identity; MkdirAll never returns it.
        fs.mkdir_all(ospath, 0o777)?;
    }

    let mut file = fs.create(in_path)?;
    let res = copy(r, &mut *file);
    let _ = file.close();
    res
}

/// Go `io.Copy(dst, src)` with the error mapped to a Hugo error.
fn copy(r: &mut dyn Read, w: &mut dyn Write) -> Result<()> {
    std::io::copy(r, w)
        .map(|_| ())
        .map_err(|e| Error::new(e.to_string()))
}

/// A file opened for writing, as a boxed writer (Go `io.WriteCloser`).
struct FileWriter(Box<dyn File>);

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl Drop for FileWriter {
    fn drop(&mut self) {
        let _ = self.0.close();
    }
}

/// Go: `helpers.OpenFilesForWriting(fs, filenames...)` — a multi-writer; on an error the files
/// opened so far are closed.
// Go: helpers/path.go:OpenFilesForWriting
pub fn open_files_for_writing(fs: &dyn Fs, filenames: &[String]) -> Result<MultiWriteCloser> {
    let mut write_closers: Vec<BoxWriter> = Vec::new();
    for filename in filenames {
        match open_file_for_writing(fs, filename) {
            Ok(f) => write_closers.push(Box::new(FileWriter(f))),
            Err(e) => {
                // Dropping closes them.
                drop(write_closers);
                return Err(e);
            }
        }
    }
    Ok(new_multi_write_closer(write_closers))
}

/// Go: `helpers.OpenFileForWriting(fs, filename)` — Create (truncate, 0666&^umask), MkdirAll(0777)
/// of the parent on ENOENT and retry.
// Go: helpers/path.go:OpenFileForWriting
pub fn open_file_for_writing(fs: &dyn Fs, filename: &str) -> Result<Box<dyn File>> {
    let filename = filepath::clean(filename);
    // Create will truncate if file already exists.
    match fs.create(&filename) {
        Ok(f) => Ok(f),
        Err(e) => {
            if !nh_common::herrors::is_not_exist(&e) {
                return Err(e);
            }
            fs.mkdir_all(&filepath::dir(&filename), 0o777)?; // before umask
            fs.create(&filename)
        }
    }
}

/// The environment [`get_cache_dir_env`] reads (Go `os.Getenv`) and whether Hugo runs as a test
/// (Go `htesting.IsTest`).
pub struct CacheDirEnv<'a> {
    pub getenv: &'a dyn Fn(&str) -> String,
    pub is_test: bool,
}

/// Go: `helpers.GetCacheDir(fs, cacheDir)`: the `cacheDir` config (which holds `HUGO_CACHEDIR`
/// when set: the config loader maps the environment into the config first), Netlify's cache,
/// else `os.UserCacheDir()/hugo_cache` (`~/Library/Caches/hugo_cache` on macOS,
/// `$XDG_CACHE_HOME` or `~/.cache` on Linux), else a `hugo_cache[_$USER]` dir in the temp dir.
// Go: helpers/path.go:GetCacheDir
pub fn get_cache_dir(fs: &dyn Fs, cache_dir: &str) -> Result<String> {
    let getenv = |k: &str| std::env::var(k).unwrap_or_default();
    get_cache_dir_env(
        fs,
        cache_dir,
        &CacheDirEnv {
            getenv: &getenv,
            is_test: false,
        },
    )
}

/// [`get_cache_dir`] with an explicit environment (the test seam).
// Go: helpers/path.go:GetCacheDir
pub fn get_cache_dir_env(fs: &dyn Fs, cache_dir: &str, env: &CacheDirEnv<'_>) -> Result<String> {
    let cache_dir = cache_dir_default(cache_dir, env.getenv);

    if !cache_dir.is_empty() {
        let exists = dir_exists(&cache_dir, fs)?;
        if !exists {
            fs.mkdir_all(&cache_dir, 0o777) // Before umask
                .map_err(|e| e.wrap("failed to create cache dir"))?;
        }
        return Ok(cache_dir);
    }

    const HUGO_CACHE_BASE: &str = "hugo_cache";

    // Avoid filling up the home dir with Hugo cache dirs from development.
    if !env.is_test
        && let Ok(user_cache_dir) = user_cache_dir(env.getenv)
    {
        let cache_dir = filepath::join(&[user_cache_dir.as_str(), HUGO_CACHE_BASE]);
        match fs.mkdir(&cache_dir, 0o777) {
            Ok(()) => return Ok(cache_dir),
            Err(e) if nh_common::herrors::is_exist(&e) => return Ok(cache_dir),
            Err(_) => {}
        }
    }

    // Fall back to a cache in /tmp.
    let user_name = (env.getenv)("USER");
    if !user_name.is_empty() {
        get_temp_dir_env(&format!("{HUGO_CACHE_BASE}_{user_name}"), fs, env.getenv)
    } else {
        get_temp_dir_env(HUGO_CACHE_BASE, fs, env.getenv)
    }
}

// Go: helpers/path.go:cacheDirDefault
fn cache_dir_default(cache_dir: &str, getenv: &dyn Fn(&str) -> String) -> String {
    // Always use the cacheDir config if set.
    if cache_dir.len() > 1 {
        return add_trailing_file_separator(cache_dir);
    }

    // See Issue #8714.
    // Turns out that Cloudflare also sets NETLIFY=true in its build environment,
    // but all of these 3 should not give any false positives.
    if getenv("NETLIFY") == "true"
        && !getenv("PULL_REQUEST").is_empty()
        && !getenv("DEPLOY_PRIME_URL").is_empty()
    {
        return "/opt/build/cache/hugo_cache/".to_string();
    }

    String::new()
}

/// Go `os.UserCacheDir()`.
fn user_cache_dir(getenv: &dyn Fn(&str) -> String) -> std::result::Result<String, Error> {
    if cfg!(target_os = "macos") {
        let dir = getenv("HOME");
        if dir.is_empty() {
            return Err(Error::new("$HOME is not defined"));
        }
        return Ok(format!("{dir}/Library/Caches"));
    }
    // Unix (Go os/file.go, the default case).
    let mut dir = getenv("XDG_CACHE_HOME");
    if dir.is_empty() {
        dir = getenv("HOME");
        if dir.is_empty() {
            return Err(Error::new("neither $XDG_CACHE_HOME nor $HOME are defined"));
        }
        dir.push_str("/.cache");
    } else if !filepath::is_abs(&dir) {
        return Err(Error::new("path in $XDG_CACHE_HOME is relative"));
    }
    Ok(dir)
}

// Go: helpers/path.go:addTrailingFileSeparator
fn add_trailing_file_separator(s: &str) -> String {
    if !s.ends_with(FILE_PATH_SEPARATOR) {
        return format!("{s}{FILE_PATH_SEPARATOR}");
    }
    s.to_string()
}

/// Go: `helpers.GetTempDir(subPath, fs)` = `afero.GetTempDir` (panics like Go when the dir cannot
/// be created; [`get_cache_dir`] returns that as an error).
// Go: helpers/path.go:GetTempDir
pub fn get_temp_dir(sub_path: &str, fs: &dyn Fs) -> String {
    let getenv = |k: &str| std::env::var(k).unwrap_or_default();
    match get_temp_dir_env(sub_path, fs, &getenv) {
        Ok(s) => s,
        Err(e) => panic!("{e}"),
    }
}

/// Go `afero.GetTempDir(fs, subPath)` with `os.TempDir()` from `getenv`.
fn get_temp_dir_env(
    sub_path: &str,
    fs: &dyn Fs,
    getenv: &dyn Fn(&str) -> String,
) -> Result<String> {
    let add_slash = |p: String| {
        if !p.ends_with(FILE_PATH_SEPARATOR) {
            p + FILE_PATH_SEPARATOR
        } else {
            p
        }
    };
    let mut tmp = getenv("TMPDIR");
    if tmp.is_empty() {
        tmp = "/tmp".to_string();
    }
    let mut dir = add_slash(tmp);

    if !sub_path.is_empty() {
        dir.push_str(&unicode_sanitize(sub_path));

        if nh_hugofs::afero::exists(fs, &dir).unwrap_or(false) {
            return Ok(add_slash(dir));
        }

        fs.mkdir_all(&dir, 0o777)?;
        dir = add_slash(dir);
    }
    Ok(dir)
}

/// Go `afero.UnicodeSanitize(s)`.
fn unicode_sanitize(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            let r = c as i32;
            go_unicode::is_letter(r)
                || go_unicode::is_digit(r)
                || go_unicode::is_mark(r)
                || matches!(c, '.' | '/' | '\\' | '_' | '-' | '%' | ' ' | '#')
        })
        .collect()
}

/// Go: `helpers.DirExists(path, fs)` = `afero.DirExists`.
// Go: helpers/path.go:DirExists
pub fn dir_exists(path: &str, fs: &dyn Fs) -> Result<bool> {
    match fs.stat(path) {
        Ok(fi) if fi.is_dir() => Ok(true),
        Ok(_) => Ok(false),
        Err(e) if e.is_not_exist() => Ok(false),
        Err(e) => Err(e),
    }
}

/// Go: `helpers.IsDir(path, fs)` = `afero.IsDir`.
// Go: helpers/path.go:IsDir
pub fn is_dir(path: &str, fs: &dyn Fs) -> Result<bool> {
    nh_hugofs::afero::is_dir(fs, path)
}

/// Go: `helpers.IsEmpty(path, fs)` — not supported (afero.Walk over a plain fs; no caller).
// Go: helpers/path.go:IsEmpty
pub fn is_empty(_path: &str, _fs: &dyn Fs) -> Result<bool> {
    Err(Error::new("neohugo-rs: helpers.IsEmpty is not supported"))
}

/// Go: `helpers.Exists(path, fs)` = `afero.Exists`.
// Go: helpers/path.go:Exists
pub fn exists(path: &str, fs: &dyn Fs) -> Result<bool> {
    nh_hugofs::afero::exists(fs, path)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/path.go (428 lines; 8/23 funcs executed)
//   types: NamedSlice
// OK L43-49: (p *PathSpec) MakePath(s string) string
// OK L52-56: (p *PathSpec) MakePathsSanitized(paths []string)
// OK L59-64: (p *PathSpec) MakePathSanitized(s string) string
// OK L68-70: MakeTitle(inpath string) string
// OK L73-80: MakePathRelative(inPath string, possibleDirectories ...string) (string, error)
// OK L87-117: GetDottedRelativePath(inPath string) string
// OK L124-129: (n NamedSlice) String() string
// OK L131-218: ExtractAndGroupRootPaths(paths []string) []NamedSlice
// OK L224-238: ExtractRootPaths(paths []string) []string
// OK L242-261: FindCWD() (string, error)
// OK L265-276: Walk(fs afero.Fs, root string, walker hugofs.WalkFunc) error
// OK L280-282: SafeWriteToDisk(inpath string, r io.Reader, fs afero.Fs) (err error)
// OK L285-287: WriteToDisk(inpath string, r io.Reader, fs afero.Fs) (err error)
// OK L290-304: OpenFilesForWriting(fs afero.Fs, filenames ...string) (io.WriteCloser, error)
// OK L308-324: OpenFileForWriting(fs afero.Fs, filename string) (afero.File, error)
// OK L328-365: GetCacheDir(fs afero.Fs, cacheDir string) (string, error)
// OK L367-388: cacheDirDefault(cacheDir string) string
// OK L390-395: addTrailingFileSeparator(s string) string
// OK L398-400: GetTempDir(subPath string, fs afero.Fs) string
// OK L403-405: DirExists(path string, fs afero.Fs) (bool, error)
// OK L408-410: IsDir(path string, fs afero.Fs) (bool, error)
//    L413-423: IsEmpty(path string, fs afero.Fs) (bool, error) — STUB (no caller)
// OK L426-428: Exists(path string, fs afero.Fs) (bool, error)
// ---------------------------------------------------------------------------
