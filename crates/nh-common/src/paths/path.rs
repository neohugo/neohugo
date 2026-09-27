//! Port of `common/paths/path.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).


//! Path helpers from `common/paths/path.go`. `sanitize` is the output-path rule (keeps
//! `unicode.IsLetter/IsDigit/IsMark` per Go 1.27 tables — use go-unicode, never `char::is_*`).

/// Go: `paths.Sanitize(s)` — see specs/content-model.md §9.3 for the exact algorithm.
// Go: common/paths/path.go:Sanitize
pub fn sanitize(s: &str) -> String {
    todo!()
}

/// Go: `paths.PathEscape(pth)` = `url.Parse(pth).EscapedPath()` (uppercase `%XX`, RawPath rules).
// Go: common/paths/path.go:PathEscape
pub fn path_escape(p: &str) -> String {
    todo!("go_url::parse(p).escaped_path()")
}

// Go: common/paths/path.go:AbsPathify
pub fn abs_pathify(working_dir: &str, in_path: &str) -> String { todo!() }
// Go: common/paths/path.go:AddTrailingSlash
pub fn add_trailing_slash(p: &str) -> String { todo!() }
// Go: common/paths/path.go:AddLeadingSlash
pub fn add_leading_slash(p: &str) -> String { todo!() }
// Go: common/paths/path.go:ExtNoDelimiter
pub fn ext_no_delimiter(p: &str) -> &str { todo!() }
// Go: common/paths/path.go:Ext
pub fn ext(p: &str) -> &str { todo!() }
/// Go: `PathAndExt` — (path without last ext, ext).
// Go: common/paths/path.go:PathAndExt
pub fn path_and_ext(p: &str) -> (String, String) { todo!() }
/// Go: `FileAndExt` — (file name without ext, ext).
// Go: common/paths/path.go:FileAndExt
pub fn file_and_ext(p: &str) -> (String, String) { todo!() }
// Go: common/paths/path.go:Filename
pub fn filename(p: &str) -> String { todo!() }
/// Go: `paths.Dir(s)` — `path.Dir` but "" for "" and "/" stays "/".
// Go: common/paths/path.go:Dir
pub fn dir(p: &str) -> String { todo!() }
// Go: common/paths/path.go:ToSlashTrimLeading
pub fn to_slash_trim_leading(p: &str) -> String { todo!() }
// Go: common/paths/path.go:TrimLeading
pub fn trim_leading(p: &str) -> String { todo!() }
// Go: common/paths/path.go:ToSlashTrimTrailing
pub fn to_slash_trim_trailing(p: &str) -> String { todo!() }
// Go: common/paths/path.go:TrimTrailing
pub fn trim_trailing(p: &str) -> String { todo!() }
// Go: common/paths/path.go:ToSlashPreserveLeading
pub fn to_slash_preserve_leading(p: &str) -> String { todo!() }
// Go: common/paths/path.go:FieldsSlash
pub fn fields_slash(p: &str) -> Vec<String> { todo!() }
// Go: common/paths/path.go:IsSameFilePath
pub fn is_same_file_path(a: &str, b: &str) -> bool { todo!() }

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/paths/path.go (430 lines; 23/38 funcs executed)
//   types: filepathPathBridge, filepathBridge, DirFile
// EX L44-46: (filepathBridge) Base(in string) string
//    L48-50: (filepathBridge) Clean(in string) string
//    L52-54: (filepathBridge) Dir(in string) string
// EX L56-58: (filepathBridge) Ext(in string) string
//    L60-62: (filepathBridge) Join(elem ...string) string
// EX L64-66: (filepathBridge) Separator() string
// EX L72-77: AbsPathify(workingDir, inPath string) string
// EX L81-86: AddTrailingSlash(path string) string
// EX L90-95: AddLeadingSlash(path string) string
//    L99-101: AddLeadingAndTrailingSlash(path string) string
//    L105-107: MakeTitle(inpath string) string
//    L111-114: ReplaceExtension(path string, newExt string) string
//    L116-123: makePathRelative(inPath string, possibleDirectories ...string) (string, error)
// EX L126-128: ExtNoDelimiter(in string) string
// EX L131-134: Ext(in string) string
// EX L137-139: PathAndExt(in string) (string, string)
// EX L143-145: FileAndExt(in string) (string, string)
//    L149-152: FileAndExtNoDelimiter(in string) (string, string)
// EX L156-159: Filename(in string) (name string)
// EX L177-182: fileAndExt(in string, b filepathPathBridge) (name string, ext string)
// EX L184-202: extractFilename(in, ext, base, pathSeparator string) (name string)
//    L205-221: GetRelativePath(path, base string) (final string, err error)
//    L223-238: prettifyPath(in string, b filepathPathBridge) string
//    L241-271: CommonDirPath(path1, path2 string) string
// EX L282-324: Sanitize(s string) string
// EX L326-336: isAllowedPathCharacter(s string, i int, r rune) bool
// EX L339-349: ishex(c byte) bool
// EX L358-364: Dir(s string) string
//    L367-370: FieldsSlash(s string) []string
//    L379-381: (df DirFile) String() string
// EX L388-394: PathEscape(pth string) string
// EX L397-399: ToSlashTrimLeading(s string) string
// EX L402-404: TrimLeading(s string) string
// EX L407-409: ToSlashTrimTrailing(s string) string
// EX L412-414: TrimTrailing(s string) string
//    L417-419: ToSlashTrim(s string) string
// EX L423-425: ToSlashPreserveLeading(s string) string
//    L428-430: IsSameFilePath(s1, s2 string) bool
// ---------------------------------------------------------------------------
