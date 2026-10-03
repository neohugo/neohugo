//! Inlining of CSS `@import` statements from the assets view (Tailwind's, unless its
//! `disableInlineImports` option is set).
//!
//! A line that is only `@import "path";` (optionally followed by a comment) is replaced by
//! the imported file, itself inlined recursively. The path is relative to the importing
//! file's directory (a leading `/` included). `url(…)` imports, imports with media queries and
//! imports of Tailwind's own package (`tailwindcss`, `tailwindcss/…`) stay. A file whose content was
//! inlined before is inlined once: later imports of it become empty lines.

use std::collections::HashSet;

use xxhash_rust::xxh3::xxh3_64;

use super::PipeError;
use super::assets::AssetsView;

/// Whether a CSS tool's input gets its `@import`s inlined first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InlineImports {
    #[default]
    Disabled,
    /// Inline; an import of a missing file is an error.
    Enabled,
    /// Inline; imports of missing files stay (`skipInlineImportsNotFound`).
    EnabledSkipMissing,
}

impl InlineImports {
    /// From the `inlineImports` and `skipInlineImportsNotFound` switches.
    #[must_use]
    pub const fn new(inline: bool, skip_missing: bool) -> Self {
        match (inline, skip_missing) {
            (false, _) => Self::Disabled,
            (true, false) => Self::Enabled,
            (true, true) => Self::EnabledSkipMissing,
        }
    }

    pub(super) const fn missing(self) -> Option<Missing> {
        match self {
            Self::Disabled => None,
            Self::Enabled => Some(Missing::Error),
            Self::EnabledSkipMissing => Some(Missing::Keep),
        }
    }
}

/// What to do with an import of a missing file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Missing {
    Error,
    Keep,
}

pub(super) struct Inliner<'a> {
    view: AssetsView<'a>,
    missing: Missing,
    seen: HashSet<u64>,
}

impl<'a> Inliner<'a> {
    pub(super) fn new(view: AssetsView<'a>, missing: Missing) -> Self {
        Self {
            view,
            missing,
            seen: HashSet::new(),
        }
    }

    /// `content`, the file at asset path `path`, with its imports inlined.
    pub(super) fn inline(&mut self, content: &str, path: &str) -> Result<String, PipeError> {
        let dir = ssg_base::paths::dir(&format!("/{}", path.trim_start_matches('/'))).to_owned();
        let mut out = String::with_capacity(content.len());
        for (i, line) in content.split_inclusive('\n').enumerate() {
            let stmt = line.trim();
            let Some(import) = import_path(stmt).filter(|p| !tailwind_package(p)) else {
                out.push_str(line);
                continue;
            };
            let rel = ssg_base::paths::join(&[dir.as_str(), import]);
            let Some(file) = self.view.file(&rel) else {
                if self.missing == Missing::Keep {
                    out.push_str(line);
                    continue;
                }
                return Err(PipeError::CssImport {
                    file: path.trim_start_matches('/').to_owned(),
                    line: i + 1,
                    path: rel,
                });
            };
            let bytes = std::fs::read(&file).map_err(|source| PipeError::Io {
                what: file.display().to_string(),
                source,
            })?;
            let replacement = if self.seen.insert(xxh3_64(&bytes)) {
                let nested = String::from_utf8(bytes).map_err(|_| PipeError::NotUtf8)?;
                self.inline(&nested, &rel)?
            } else {
                String::new()
            };
            out.push_str(&line.replacen(stmt, &replacement, 1));
        }
        Ok(out)
    }
}

/// The path of an `@import "path";` statement (a trailing `/* … */` comment allowed), unless it
/// has a media query or uses `url(…)`.
fn import_path(stmt: &str) -> Option<&str> {
    let rest = stmt.strip_prefix("@import")?;
    if rest.contains("url(") || !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim_start();
    let quote = rest.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let rest = &rest[1..];
    let end = rest.find(quote)?;
    let (path, after) = (&rest[..end], &rest[end + 1..]);
    let after = after.trim_start();
    let after = after.strip_prefix(';').unwrap_or(after).trim();
    let tail_ok = after.is_empty() || (after.starts_with("/*") && after.ends_with("*/"));
    (tail_ok && !path.is_empty()).then_some(path)
}

/// Tailwind's own imports (`tailwindcss`, `tailwindcss/preflight`): a package, not a file.
fn tailwind_package(path: &str) -> bool {
    (path == "tailwindcss" || path.starts_with("tailwindcss/")) && !path.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statements() {
        assert_eq!(import_path(r#"@import "a.css";"#), Some("a.css"));
        assert_eq!(import_path("@import 'sub/x.css'"), Some("sub/x.css"));
        assert_eq!(import_path(r#"@import "a.css"; /* c */"#), Some("a.css"));
        assert_eq!(import_path(r#"@import url("a.css");"#), None);
        assert_eq!(import_path(r#"@import "print.css" print;"#), None);
        assert_eq!(import_path(r#"@importx "a.css";"#), None);
        assert!(tailwind_package("tailwindcss"));
        assert!(tailwind_package("tailwindcss/utilities"));
        assert!(!tailwind_package("tailwindcss/x.css"));
    }
}
