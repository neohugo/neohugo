//! Port of `resources/resource_transformers/cssjs/inline_imports.go`.
//!
//! optional (inlineImports=false)
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `cssjs/inline_imports.go`: the `@import` inliner of postCSS (`inlineImports: true`) and
//! tailwindcss, and the mapping of postcss error line numbers back to the imported files.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use nh_common::Result;
use nh_common::goregexp::Regexp;
use nh_common::herrors::{Error, FilePos};
use nh_config::decode::FieldRef;
use nh_hugofs::afero::Fs;

const IMPORT_IDENTIFIER: &str = "@import";

static CSS_SYNTAX_ERROR_RE: LazyLock<Regexp> =
    LazyLock::new(|| Regexp::must_compile(r"> (\d+) \|"));
static SHOULD_IMPORT_RE: LazyLock<Regexp> =
    LazyLock::new(|| Regexp::must_compile(r#"^@import ["'](.*?)["'];?\s*(/\*.*\*/)?$"#));

/// Go: `cssjs.InlineImports` (squashed into the postcss and tailwindcss options).
#[derive(Clone, Debug, Default)]
pub struct InlineImports {
    /// Enable inlining of @import statements. Does so recursively, but currently once only per
    /// file; that is, it's not possible to import the same file in different scopes (root,
    /// media query...) Note that this import routine does not care about the CSS spec, so you
    /// can have @import anywhere in the file.
    pub inline_imports: bool,
    /// Disable inlining of @import statements. This is currenty only used for css.TailwindCSS.
    pub disable_inline_imports: bool,
    /// When InlineImports is enabled, we fail the build if an import cannot be resolved. You can
    /// enable this to allow the build to continue and leave the import statement in place.
    pub skip_inline_imports_not_found: bool,
}

nh_config::decode_struct!(InlineImports, "cssjs.InlineImports", |s| vec![
    FieldRef::new("InlineImports", &mut s.inline_imports),
    FieldRef::new("DisableInlineImports", &mut s.disable_inline_imports),
    FieldRef::new(
        "SkipInlineImportsNotFound",
        &mut s.skip_inline_imports_not_found
    ),
]);

/// Go: `fileOffset`.
#[derive(Clone, Debug)]
struct FileOffset {
    filename: String,
    offset: i64,
}

/// Go: `importResolver`.
pub(crate) struct ImportResolver {
    r: Vec<u8>,
    in_path: String,
    opts: InlineImports,

    content_seen: HashMap<String, bool>,
    linemap: HashMap<i64, FileOffset>,
    fs: Arc<dyn Fs>,
}

// Go: resources/resource_transformers/cssjs/inline_imports.go:newImportResolver
pub(crate) fn new_import_resolver(
    r: Vec<u8>,
    in_path: &str,
    opts: InlineImports,
    fs: Arc<dyn Fs>,
) -> ImportResolver {
    ImportResolver {
        r,
        in_path: in_path.to_string(),
        fs,
        linemap: HashMap::new(),
        content_seen: HashMap::new(),
        opts,
    }
}

impl ImportResolver {
    // Go: resources/resource_transformers/cssjs/inline_imports.go:(*importResolver).contentHash
    fn content_hash(&self, filename: &str) -> (Option<Vec<u8>>, String) {
        let Ok(b) = nh_hugofs::afero::read_file(self.fs.as_ref(), filename) else {
            return (None, String::new());
        };
        use sha2::Digest;
        let d = sha2::Sha256::digest(&b);
        let hex: String = d.iter().map(|x| format!("{x:02x}")).collect();
        (Some(b), hex)
    }

    // Go: resources/resource_transformers/cssjs/inline_imports.go:(*importResolver).importRecursive
    fn import_recursive(
        &mut self,
        line_num: i64,
        content: &[u8],
        in_path: &str,
    ) -> Result<(i64, Vec<u8>)> {
        let base_path = go_path::path::dir(in_path).to_string();

        let mut replacements: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        let lines: Vec<&[u8]> = content.split(|&b| b == b'\n').collect();

        let mut i: i64 = 0;
        for (offset, line) in lines.iter().enumerate() {
            let offset = offset as i64;
            i += 1;
            let line_trimmed = go_unicode::strings::trim_space(line);
            // strings.Index(line, lineTrimmed): the first occurrence is the trimmed part itself
            // (the bytes before it are spaces, it starts with a non-space), 0 when empty.
            let column = if line_trimmed.is_empty() {
                0
            } else {
                (line_trimmed.as_ptr() as usize - line.as_ptr() as usize) as i64
            };
            let line = line_trimmed;

            if !self.should_import(line) {
                self.track_line(i + line_num, in_path, offset);
            } else {
                let rest = line
                    .strip_prefix(IMPORT_IDENTIFIER.as_bytes())
                    .unwrap_or(line);
                let path = trim_bytes(rest, b" \"';");
                let path = String::from_utf8_lossy(path).into_owned();
                let filename = go_path::filepath::join(&[base_path.as_str(), path.as_str()]);
                let (import_content, hash) = self.content_hash(&filename);

                let Some(import_content) = import_content else {
                    if self.opts.skip_inline_imports_not_found {
                        self.track_line(i + line_num, in_path, offset);
                        continue;
                    }
                    let pos = FilePos {
                        filename: in_path.to_string(),
                        line: offset + 1,
                        column: column + 1,
                    };
                    return Err(new_file_error_from_file_in_pos(
                        Error::new(format!("failed to resolve CSS @import \"{filename}\"")),
                        pos,
                        self.fs.as_ref(),
                    ));
                };

                i -= 1;

                if self.content_seen.get(&hash).copied().unwrap_or(false) {
                    i += 1;
                    // Just replace the line with an empty string.
                    replacements.push((line.to_vec(), Vec::new()));
                    self.track_line(i + line_num, in_path, offset);
                    continue;
                }

                self.content_seen.insert(hash, true);

                // Handle recursive imports.
                let (l, nested) = self.import_recursive(
                    i + line_num,
                    &import_content,
                    go_path::filepath::to_slash(&filename),
                )?;

                self.track_line(i + line_num, in_path, offset);

                i += l;

                replacements.push((line.to_vec(), nested));
            }
        }

        let mut content = content.to_vec();
        if !replacements.is_empty() {
            content = replace_all(&content, &replacements);
        }

        Ok((i, content))
    }

    fn track_line(&mut self, key: i64, in_path: &str, offset: i64) {
        // TODO(bep) this is not very efficient.
        self.linemap.insert(
            key,
            FileOffset {
                filename: in_path.to_string(),
                offset,
            },
        );
    }

    // Go: resources/resource_transformers/cssjs/inline_imports.go:(*importResolver).resolve
    pub(crate) fn resolve(&mut self) -> Result<Vec<u8>> {
        let content = self.r.clone();
        let in_path = self.in_path.clone();
        let (_, new_content) = self.import_recursive(0, &content, &in_path)?;
        Ok(new_content)
    }

    /// See https://www.w3schools.com/cssref/pr_import_rule.asp. We currently only support simple
    /// file imports, no urls, no media queries. So this is OK: `@import "navigation.css";`. This
    /// is not: `@import url("navigation.css");`, `@import "mobstyle.css" screen and
    /// (max-width: 768px);`.
    // Go: resources/resource_transformers/cssjs/inline_imports.go:(*importResolver).shouldImport
    fn should_import(&self, s: &[u8]) -> bool {
        if !s.starts_with(IMPORT_IDENTIFIER.as_bytes()) {
            return false;
        }
        if s.windows(4).any(|w| w == b"url(") {
            return false;
        }

        let Some(m) = SHOULD_IMPORT_RE.find_submatch(s) else {
            return false;
        };

        if m.len() != 3 {
            return false;
        }

        if super::tailwindcss::tailwind_import_exclude(m[1].unwrap_or_default()) {
            return false;
        }

        true
    }

    // Go: resources/resource_transformers/cssjs/inline_imports.go:(*importResolver).toFileError
    pub(crate) fn to_file_error(&self, output: &str) -> Error {
        let in_err = Error::new(output.to_string());

        let Some(m) = CSS_SYNTAX_ERROR_RE.find_string_submatch(output) else {
            return in_err;
        };

        let Ok(line_num) = m[1].parse::<i64>() else {
            return in_err;
        };

        let Some(file) = self.linemap.get(&line_num) else {
            return in_err;
        };

        let Ok(fi) = self.fs.stat(&file.filename) else {
            return in_err;
        };

        let real_filename = fi.meta.filename.clone();
        if fi.meta.open().is_err() {
            return in_err;
        }

        let ferr = nh_common::herrors::new_file_error_from_name(in_err, &real_filename);
        let column = ferr.pos().map(|p| p.column).unwrap_or(1);
        Error::new(output.to_string()).at(FilePos {
            filename: real_filename,
            line: file.offset + 1,
            column,
        })
    }
}

/// Go `strings.Trim(s, cutset)`.
fn trim_bytes<'a>(s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    let start = s
        .iter()
        .position(|b| !cutset.contains(b))
        .unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|b| !cutset.contains(b))
        .map_or(start, |e| e + 1);
    &s[start..end.max(start)]
}

/// Go `strings.NewReplacer(pairs...).Replace(s)`: left to right, at each position the first
/// pair (in argument order) whose old string matches is replaced; no overlapping matches.
fn replace_all(s: &[u8], pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if let Some((old, new)) = pairs
            .iter()
            .find(|(old, _)| !old.is_empty() && s[i..].starts_with(old))
        {
            out.extend_from_slice(new);
            i += old.len();
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

/// Go: `herrors.NewFileErrorFromFileInPos(err, pos, fs, nil)`: the position is kept (the simple
/// line matcher never moves it); the filename becomes the real filename when the file can be
/// opened.
fn new_file_error_from_file_in_pos(err: Error, mut pos: FilePos, fs: &dyn Fs) -> Error {
    if let Ok(fi) = fs.stat(&pos.filename) {
        pos.filename = fi.meta.filename.clone();
    }
    err.at(pos)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/cssjs/inline_imports.go (247 lines; 1/6 funcs executed)
//   types: fileOffset, importResolver
// OK L60-69: newImportResolver(r io.Reader, inPath string, opts InlineImports, fs afero.Fs, logger loggers.Logger, dependencyManager identity.Manager) *importRe...
// OK L71-79: (imp *importResolver) contentHash(filename string) ([]byte, string)
// OK L81-158: (imp *importResolver) importRecursive( lineNum int, content string, inPath string, ) (int, string, error)
// OK L160-174: (imp *importResolver) resolve() (io.Reader, error)
// OK L186-208: (imp *importResolver) shouldImport(s string) bool
// OK L210-247: (imp *importResolver) toFileError(output string) error
// ---------------------------------------------------------------------------
