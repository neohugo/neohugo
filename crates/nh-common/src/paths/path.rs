//! Port of `common/paths/path.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

//! Path helpers from `common/paths/path.go`. `sanitize` is the output-path rule (keeps
//! `unicode.IsLetter/IsDigit/IsMark` per Go 1.27 tables — use go-unicode, never `char::is_*`).
//!
//! `path/filepath` is the unix flavour (the golden build ran on darwin): `filepath.ToSlash` and
//! `filepath.FromSlash` are the identity and the separator is `/`. Functions whose Go input is
//! arbitrary text (`Sanitize`, `MakeTitle`, `PathEscape`) also have a `_bytes` form with Go's
//! string-as-bytes semantics (invalid UTF-8 decodes to `U+FFFD`, width 1).

use std::borrow::Cow;

use go_path::{filepath, path};
use go_unicode::{self as unicode, strings, utf8};

use crate::herrors::{Error, Result};

/// Go: `paths.FilePathSeparator` (`os.PathSeparator` as a string).
pub const FILE_PATH_SEPARATOR: &str = "/";

/// Go: `filepathPathBridge` — the common functionality of `path/filepath` and `path`.
pub(crate) trait FilepathPathBridge {
    fn base<'a>(&self, s: &'a str) -> &'a str;
    fn clean(&self, s: &str) -> String;
    fn dir(&self, s: &str) -> String;
    fn ext<'a>(&self, s: &'a str) -> &'a str;
    fn join(&self, elem: &[&str]) -> String;
    fn separator(&self) -> &'static str;
}

/// Go: `filepathBridge`.
pub(crate) struct FilepathBridge;

impl FilepathPathBridge for FilepathBridge {
    // Go: common/paths/path.go:(filepathBridge).Base
    fn base<'a>(&self, s: &'a str) -> &'a str {
        filepath::base(s)
    }
    // Go: common/paths/path.go:(filepathBridge).Clean
    fn clean(&self, s: &str) -> String {
        filepath::clean(s)
    }
    // Go: common/paths/path.go:(filepathBridge).Dir
    fn dir(&self, s: &str) -> String {
        filepath::dir(s)
    }
    // Go: common/paths/path.go:(filepathBridge).Ext
    fn ext<'a>(&self, s: &'a str) -> &'a str {
        filepath::ext(s)
    }
    // Go: common/paths/path.go:(filepathBridge).Join
    fn join(&self, elem: &[&str]) -> String {
        filepath::join(elem)
    }
    // Go: common/paths/path.go:(filepathBridge).Separator
    fn separator(&self) -> &'static str {
        FILE_PATH_SEPARATOR
    }
}

/// Go: `fpb`.
pub(crate) const FPB: FilepathBridge = FilepathBridge;

/// Go: `paths.AbsPathify` — an absolute path given a working dir and a relative path. If already
/// absolute, the path is just cleaned.
// Go: common/paths/path.go:AbsPathify
pub fn abs_pathify(working_dir: &str, in_path: &str) -> String {
    if filepath::is_abs(in_path) {
        return filepath::clean(in_path);
    }
    filepath::join(&[working_dir, in_path])
}

/// Go: `paths.AddTrailingSlash` — adds a trailing Unix styled slash (/) if not already there.
// Go: common/paths/path.go:AddTrailingSlash
pub fn add_trailing_slash(p: &str) -> String {
    if !p.ends_with('/') {
        return format!("{p}/");
    }
    p.to_string()
}

/// Go: `paths.AddLeadingSlash` — adds a leading Unix styled slash (/) if not already there.
// Go: common/paths/path.go:AddLeadingSlash
pub fn add_leading_slash(p: &str) -> String {
    if !p.starts_with('/') {
        return format!("/{p}");
    }
    p.to_string()
}

/// Go: `paths.AddLeadingAndTrailingSlash`.
// Go: common/paths/path.go:AddLeadingAndTrailingSlash
pub fn add_leading_and_trailing_slash(p: &str) -> String {
    add_trailing_slash(&add_leading_slash(p))
}

/// Go: `paths.MakeTitle` — trims whitespace and replaces hyphens with whitespace.
// Go: common/paths/path.go:MakeTitle
pub fn make_title(inpath: &str) -> String {
    String::from_utf8(make_title_bytes(inpath.as_bytes()))
        .expect("MakeTitle of valid UTF-8 is valid UTF-8")
}

/// [`make_title`] over Go string bytes.
pub fn make_title_bytes(inpath: &[u8]) -> Vec<u8> {
    strings::replace_all(strings::trim_space(inpath), b"-", b" ").into_owned()
}

/// Go: `paths.ReplaceExtension` — strips the old extension (and the directory) and returns the
/// file name with the new extension.
// Go: common/paths/path.go:ReplaceExtension
pub fn replace_extension(p: &str, new_ext: &str) -> String {
    let (f, _) = file_and_ext_bridge(p, &FPB);
    format!("{f}.{new_ext}")
}

// Go: common/paths/path.go:makePathRelative
/// Go: `makePathRelative(inPath, possibleDirectories...)` (unexported; ported for the Go test).
pub fn make_path_relative(in_path: &str, possible_directories: &[&str]) -> Result<String> {
    for current_path in possible_directories {
        if let Some(rest) = in_path.strip_prefix(current_path) {
            return Ok(rest.to_string());
        }
    }
    Err(Error::new("can't extract relative path, unknown prefix"))
}

/// Go: `paths.ExtNoDelimiter` — the extension without the delimiter, i.e. "md".
// Go: common/paths/path.go:ExtNoDelimiter
pub fn ext_no_delimiter(p: &str) -> &str {
    let e = ext(p);
    e.strip_prefix('.').unwrap_or(e)
}

/// Go: `paths.Ext` — the extension including the delimiter, i.e. ".md".
// Go: common/paths/path.go:Ext
pub fn ext(p: &str) -> &str {
    FPB.ext(p)
}

/// Go: `PathAndExt` — `FileAndExt` using the `path` package: (file name without ext, ext).
// Go: common/paths/path.go:PathAndExt
pub fn path_and_ext(p: &str) -> (String, String) {
    let (f, e) = file_and_ext_bridge(p, &super::url::PB);
    (f.to_string(), e.to_string())
}

/// Go: `FileAndExt` — (file name without ext, ext including the delimiter).
// Go: common/paths/path.go:FileAndExt
pub fn file_and_ext(p: &str) -> (String, String) {
    let (f, e) = file_and_ext_bridge(p, &FPB);
    (f.to_string(), e.to_string())
}

/// Go: `FileAndExtNoDelimiter` — (file name without ext, ext without the delimiter).
// Go: common/paths/path.go:FileAndExtNoDelimiter
pub fn file_and_ext_no_delimiter(p: &str) -> (String, String) {
    let (f, e) = file_and_ext_bridge(p, &FPB);
    (f.to_string(), e.strip_prefix('.').unwrap_or(e).to_string())
}

/// Go: `paths.Filename` — strips out the directory and the extension.
// Go: common/paths/path.go:Filename
pub fn filename(p: &str) -> String {
    file_and_ext_bridge(p, &FPB).0.to_string()
}

// Go: common/paths/path.go:fileAndExt
/// The file name and any extension of a file path as two separate strings (see Go's doc).
pub(crate) fn file_and_ext_bridge<'a>(
    in_: &'a str,
    b: &dyn FilepathPathBridge,
) -> (&'a str, &'a str) {
    let ext = b.ext(in_);
    let base = b.base(in_);
    (extract_filename(in_, ext, base, b.separator()), ext)
}

// Go: common/paths/path.go:extractFilename
fn extract_filename<'a>(in_: &str, ext: &str, base: &'a str, path_separator: &str) -> &'a str {
    // No file name cases. These are defined as:
    // 1. any "in" path that ends in a pathSeparator
    // 2. any "base" consisting of just an pathSeparator
    // 3. any "base" consisting of just an empty string
    // 4. any "base" consisting of just the current directory i.e. "."
    // 5. any "base" consisting of just the parent directory i.e. ".."
    if strings::last_index(in_.as_bytes(), path_separator.as_bytes()) == in_.len() as isize - 1
        || base.is_empty()
        || base == "."
        || base == ".."
        || base == path_separator
    {
        "" // there is NO filename
    } else if !ext.is_empty() {
        // there was an Extension
        // return the filename minus the extension (and the ".")
        &base[..strings::last_index(base.as_bytes(), b".") as usize]
    } else {
        // no extension case so just return base, which will
        // be the filename
        base
    }
}

/// Go: `paths.GetRelativePath` — `filepath.Rel` of the cleaned path to the cleaned base, keeping a
/// trailing separator.
// Go: common/paths/path.go:GetRelativePath
pub fn get_relative_path(p: &str, base: &str) -> Result<String> {
    if filepath::is_abs(p) && base.is_empty() {
        return Err(Error::new("source: missing base directory"));
    }
    let name = filepath::clean(p);
    let base = filepath::clean(base);

    let mut name = filepath::rel(&base, &name).map_err(|e| Error::new(e.to_string()))?;

    if filepath::from_slash(p).ends_with(FILE_PATH_SEPARATOR)
        && !name.ends_with(FILE_PATH_SEPARATOR)
    {
        name.push_str(FILE_PATH_SEPARATOR);
    }
    Ok(name)
}

// Go: common/paths/path.go:prettifyPath
pub(crate) fn prettify_path(in_: &str, b: &dyn FilepathPathBridge) -> String {
    if filepath::ext(in_).is_empty() {
        // /section/name/  -> /section/name/index.html
        if in_.len() < 2 {
            return b.separator().to_string();
        }
        return b.join(&[in_, "index.html"]);
    }
    let (name, ext) = file_and_ext_bridge(in_, b);
    if name == "index" {
        // /section/name/index.html -> /section/name/index.html
        return b.clean(in_);
    }
    // /section/name.html -> /section/name/index.html
    b.join(&[&b.dir(in_), name, &format!("index{ext}")])
}

/// Go: `paths.CommonDirPath` — the common directory of the given paths.
// Go: common/paths/path.go:CommonDirPath
pub fn common_dir_path(path1: &str, path2: &str) -> String {
    if path1.is_empty() || path2.is_empty() {
        return String::new();
    }

    let had_leading_slash = path1.starts_with('/') || path2.starts_with('/');

    let path1 = trim_leading(path1);
    let path2 = trim_leading(path2);

    let p1: Vec<&str> = path1.split('/').collect();
    let p2: Vec<&str> = path2.split('/').collect();

    let mut common: Vec<&str> = Vec::new();

    let mut i = 0;
    while i < p1.len() && i < p2.len() {
        if p1[i] == p2[i] {
            common.push(p1[i]);
        } else {
            break;
        }
        i += 1;
    }

    let mut s = common.join("/");

    if had_leading_slash && !s.is_empty() {
        s = format!("/{s}");
    }

    s
}

/// Go: `paths.Sanitize(s)` — sanitizes a string to be used in Hugo's file paths and URLs,
/// allowing only a predefined set of special Unicode characters. Spaces are replaced with a
/// single hyphen. See specs/content-model.md §9.3.
// Go: common/paths/path.go:Sanitize
pub fn sanitize(s: &str) -> String {
    match sanitize_bytes(s.as_bytes()) {
        Cow::Borrowed(_) => s.to_string(),
        Cow::Owned(v) => String::from_utf8(v).expect("Sanitize emits encoded runes"),
    }
}

/// [`sanitize`] over Go string bytes (invalid UTF-8 decodes to `U+FFFD`, which is dropped).
pub fn sanitize_bytes(s: &[u8]) -> Cow<'_, [u8]> {
    let mut will_change = false;
    for (i, r) in utf8::runes(s) {
        will_change = !is_allowed_path_character(s, i, r);
        if will_change {
            break;
        }
    }

    if !will_change {
        // Prevent allocation when nothing changes.
        return Cow::Borrowed(s);
    }

    let mut target: Vec<u8> = Vec::with_capacity(s.len());
    let mut prepend_hyphen = false;
    let mut was_hyphen = false;

    for (i, r) in utf8::runes(s) {
        let is_allowed = is_allowed_path_character(s, i, r);

        if is_allowed {
            // track explicit hyphen in input; no need to add a new hyphen if
            // we just saw one.
            was_hyphen = r == '-' as i32;

            if prepend_hyphen {
                // if currently have a hyphen, don't prepend an extra one
                if !was_hyphen {
                    target.push(b'-');
                }
                prepend_hyphen = false;
            }
            utf8::append_rune(&mut target, r);
        } else if !target.is_empty() && !was_hyphen && unicode::is_space(r) {
            prepend_hyphen = true;
        }
    }

    Cow::Owned(target)
}

// Go: common/paths/path.go:isAllowedPathCharacter
fn is_allowed_path_character(s: &[u8], i: usize, r: i32) -> bool {
    if r == ' ' as i32 {
        return false;
    }
    // Check for the most likely first (faster).
    let mut is_allowed = unicode::is_letter(r) || unicode::is_digit(r);
    is_allowed = is_allowed
        || r == '.' as i32
        || r == '/' as i32
        || r == '\\' as i32
        || r == '_' as i32
        || r == '#' as i32
        || r == '+' as i32
        || r == '~' as i32
        || r == '-' as i32
        || r == '@' as i32;
    is_allowed = is_allowed || unicode::is_mark(r);
    is_allowed =
        is_allowed || (r == '%' as i32 && i + 2 < s.len() && ishex(s[i + 1]) && ishex(s[i + 2]));
    is_allowed
}

// Go: common/paths/path.go:ishex
fn ishex(c: u8) -> bool {
    c.is_ascii_digit() || (b'a'..=b'f').contains(&c) || (b'A'..=b'F').contains(&c)
}

/// Go: `paths.Dir(s)` — `path.Dir` without the `path.Clean` step. The returned path ends in a
/// slash only if it is the root "/".
// Go: common/paths/path.go:Dir
pub fn dir(p: &str) -> String {
    let (d, _) = path::split(p);
    if d.len() > 1 && d.ends_with('/') {
        return d[..d.len() - 1].to_string();
    }
    d.to_string()
}

/// Go: `paths.FieldsSlash` — cuts s into fields separated with '/'.
// Go: common/paths/path.go:FieldsSlash
pub fn fields_slash(p: &str) -> Vec<String> {
    strings::fields_func(p.as_bytes(), |r| r == '/' as i32)
        .into_iter()
        .map(|f| String::from_utf8(f.to_vec()).expect("split at an ASCII byte"))
        .collect()
}

/// Go: `paths.DirFile` — the result from `path.Split`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirFile {
    pub dir: String,
    pub file: String,
}

impl DirFile {
    // Go: common/paths/path.go:(DirFile).String
    pub fn string(&self) -> String {
        format!("{}|{}", self.dir, self.file)
    }
}

/// Go: `paths.PathEscape(pth)` = `url.Parse(pth).EscapedPath()` (uppercase `%XX`, RawPath rules).
/// Go panics when `url.Parse` fails; this does too (see [`try_path_escape`]).
// Go: common/paths/path.go:PathEscape
pub fn path_escape(p: &str) -> String {
    match try_path_escape(p.as_bytes()) {
        Ok(s) => s,
        Err(e) => panic!("{e}"),
    }
}

/// [`path_escape`] returning Go's panic value (the `url.Parse` error) as an error. The escaped
/// path is always ASCII.
pub fn try_path_escape(p: &[u8]) -> Result<String> {
    let u = go_url::parse(p).map_err(|e| Error::new(e.to_string()))?;
    Ok(String::from_utf8(u.escaped_path()).expect("EscapedPath is ASCII"))
}

/// Go: `paths.ToSlashTrimLeading` — `filepath.ToSlash` with an added / prefix trimmer.
// Go: common/paths/path.go:ToSlashTrimLeading
pub fn to_slash_trim_leading(p: &str) -> String {
    trim_leading(filepath::to_slash(p))
}

/// Go: `paths.TrimLeading` — trims the leading slash from the given string.
// Go: common/paths/path.go:TrimLeading
pub fn trim_leading(p: &str) -> String {
    p.strip_prefix('/').unwrap_or(p).to_string()
}

/// Go: `paths.ToSlashTrimTrailing` — `filepath.ToSlash` with an added / suffix trimmer.
// Go: common/paths/path.go:ToSlashTrimTrailing
pub fn to_slash_trim_trailing(p: &str) -> String {
    trim_trailing(filepath::to_slash(p))
}

/// Go: `paths.TrimTrailing` — trims the trailing slash from the given string.
// Go: common/paths/path.go:TrimTrailing
pub fn trim_trailing(p: &str) -> String {
    p.strip_suffix('/').unwrap_or(p).to_string()
}

/// Go: `paths.ToSlashTrim` — trims any leading and trailing slashes and converts to a forward
/// slash separated path.
// Go: common/paths/path.go:ToSlashTrim
pub fn to_slash_trim(p: &str) -> String {
    filepath::to_slash(p).trim_matches('/').to_string()
}

/// Go: `paths.ToSlashPreserveLeading` — a forward slash separated path with one leading slash
/// and no trailing slash.
// Go: common/paths/path.go:ToSlashPreserveLeading
pub fn to_slash_preserve_leading(p: &str) -> String {
    format!("/{}", filepath::to_slash(p).trim_matches('/'))
}

/// Go: `paths.IsSameFilePath`.
// Go: common/paths/path.go:IsSameFilePath
pub fn is_same_file_path(a: &str, b: &str) -> bool {
    path::clean(&to_slash_trim(a)) == path::clean(&to_slash_trim(b))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/paths/path.go (430 lines; 23/38 funcs executed)
//   types: filepathPathBridge, filepathBridge, DirFile
// OK L44-46: (filepathBridge) Base(in string) string
// OK L48-50: (filepathBridge) Clean(in string) string
// OK L52-54: (filepathBridge) Dir(in string) string
// OK L56-58: (filepathBridge) Ext(in string) string
// OK L60-62: (filepathBridge) Join(elem ...string) string
// OK L64-66: (filepathBridge) Separator() string
// OK L72-77: AbsPathify(workingDir, inPath string) string
// OK L81-86: AddTrailingSlash(path string) string
// OK L90-95: AddLeadingSlash(path string) string
// OK L99-101: AddLeadingAndTrailingSlash(path string) string
// OK L105-107: MakeTitle(inpath string) string
// OK L111-114: ReplaceExtension(path string, newExt string) string
// OK L116-123: makePathRelative(inPath string, possibleDirectories ...string) (string, error)
// OK L126-128: ExtNoDelimiter(in string) string
// OK L131-134: Ext(in string) string
// OK L137-139: PathAndExt(in string) (string, string)
// OK L143-145: FileAndExt(in string) (string, string)
// OK L149-152: FileAndExtNoDelimiter(in string) (string, string)
// OK L156-159: Filename(in string) (name string)
// OK L177-182: fileAndExt(in string, b filepathPathBridge) (name string, ext string)
// OK L184-202: extractFilename(in, ext, base, pathSeparator string) (name string)
// OK L205-221: GetRelativePath(path, base string) (final string, err error)
// OK L223-238: prettifyPath(in string, b filepathPathBridge) string
// OK L241-271: CommonDirPath(path1, path2 string) string
// OK L282-324: Sanitize(s string) string
// OK L326-336: isAllowedPathCharacter(s string, i int, r rune) bool
// OK L339-349: ishex(c byte) bool
// OK L358-364: Dir(s string) string
// OK L367-370: FieldsSlash(s string) []string
// OK L379-381: (df DirFile) String() string
// OK L388-394: PathEscape(pth string) string
// OK L397-399: ToSlashTrimLeading(s string) string
// OK L402-404: TrimLeading(s string) string
// OK L407-409: ToSlashTrimTrailing(s string) string
// OK L412-414: TrimTrailing(s string) string
// OK L417-419: ToSlashTrim(s string) string
// OK L423-425: ToSlashPreserveLeading(s string) string
// OK L428-430: IsSameFilePath(s1, s2 string) bool
// ---------------------------------------------------------------------------
