//! Phase A4: read every content file of the discovery, split and decode its front matter, and
//! read the capture overrides (`kind`, `lang`, `path`) and the page's own `cascade`.
//!
//! Files are read in parallel; the result keeps the discovery order (key, then language).

use std::path::Path;
use std::sync::Arc;

use jiff::Timestamp;
use neohugo_base::diag::Diagnostic;
use neohugo_base::{LangIdx, PageKind, Params, Value};
use neohugo_config::Config;
use neohugo_page::{Cascade, capture_overrides};
use neohugo_pageparser::{FrontMatterFormat, decode_front_matter, split_front_matter};
use neohugo_vfs::{BundleKind, Component, ContentFile, FileRef, Parsed, PathInfo, PathParser, Vfs};
use rayon::prelude::*;

use crate::ModelError;

/// The content file a page was read from.
#[derive(Clone, Debug)]
pub struct SourceFile {
    pub file: FileRef,
    /// The file's path as discovered.
    pub file_info: PathInfo,
    /// The page's path: `file_info`, or the path front matter `path` names.
    pub info: PathInfo,
    /// The front matter's format (`None`: the file has none).
    pub front_matter: Option<FrontMatterFormat>,
    /// The whole file.
    pub text: Arc<str>,
    /// Where the body starts in `text` (after the front matter and any byte order mark).
    pub body_offset: usize,
    /// The file's modification time (`:fileModTime`).
    pub mod_time: Option<Timestamp>,
}

impl SourceFile {
    /// The content after the front matter.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.text[self.body_offset..]
    }

    /// The file name `:filename` dates read: the bundle directory's name for a bundle index,
    /// else the file name without identifiers, in the file's own spelling.
    #[must_use]
    pub fn base_filename(&self) -> &str {
        &self.file_info.original.name
    }
}

/// A content file after phase A4: where it goes, and its folded front matter.
#[derive(Clone, Debug)]
pub(crate) struct CapturedPage {
    pub lang: LangIdx,
    /// Front matter `kind` (normalised); `None`: from the path.
    pub kind: Option<PageKind>,
    /// A content file inside a leaf bundle (`BundleKind::ContentResource`).
    pub bundled: bool,
    pub source: SourceFile,
    /// Folded front matter; `kind`, `lang` and `path` normalised as the overrides read them.
    pub params: Params,
    /// The page's own `cascade`.
    pub cascade: Cascade,
}

/// A bundle resource file that is not content (images, data, …).
#[derive(Clone, Debug)]
pub(crate) struct CapturedResource {
    pub lang: LangIdx,
    pub file: FileRef,
    pub info: PathInfo,
}

/// The output of phase A4.
#[derive(Debug, Default)]
pub(crate) struct Capture {
    pub pages: Vec<CapturedPage>,
    pub resources: Vec<CapturedResource>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Discovers and reads the content files of `vfs`.
pub(crate) fn capture(cfg: &Config, vfs: &Vfs) -> Result<Capture, ModelError> {
    let parser = PathParser::from_config(cfg);
    let discovery = vfs.discover_content(&parser)?;
    let mut out = Capture::default();
    for d in &discovery.duplicates {
        out.diagnostics.push(
            Diagnostic::warning(format!(
                "duplicate content path {:?}: {} is used, {} is ignored",
                d.key.to_path(),
                d.kept.display(),
                d.dropped.display()
            ))
            .with_id("duplicate-content-path"),
        );
    }

    let (content, resources): (Vec<ContentFile>, Vec<ContentFile>) = discovery
        .files
        .into_iter()
        .partition(|f| f.info.kind.is_content());
    if let Some(f) = content
        .iter()
        .find(|f| f.info.kind == BundleKind::ContentAdapter)
    {
        return Err(ModelError::ContentAdapter(f.file.abs.clone()));
    }
    out.resources = resources
        .into_iter()
        .map(|f| CapturedResource {
            lang: f.lang,
            file: f.file,
            info: f.info,
        })
        .collect();

    let read: Vec<Result<CapturedPage, ModelError>> = content
        .into_par_iter()
        .map(|f| read_page(f, &parser))
        .collect();
    for r in read {
        out.pages.push(r?);
    }
    Ok(out)
}

fn read_page(f: ContentFile, parser: &PathParser) -> Result<CapturedPage, ModelError> {
    let abs = f.file.abs.clone();
    let text: Arc<str> = std::fs::read_to_string(&abs)
        .map_err(|source| ModelError::Read {
            path: abs.clone(),
            source,
        })?
        .into();
    let split = split_front_matter(&text).map_err(|e| ModelError::FrontMatter {
        path: abs.clone(),
        message: e.to_string(),
    })?;
    let front_matter = split.front_matter.map(|(format, _)| format);
    let mut params = match split.front_matter {
        Some((format, fm)) => {
            decode_front_matter(format, fm).map_err(|e| ModelError::FrontMatter {
                path: abs.clone(),
                message: e.to_string(),
            })?
        }
        None => Params::default(),
    };
    let body_offset = split.body_offset;
    let page_err = |source| ModelError::Page {
        path: abs.clone(),
        source,
    };
    let overrides = capture_overrides(&params).map_err(page_err)?;
    let cascade = params
        .get("cascade")
        .map_or(Ok(Cascade::default()), Cascade::decode)
        .map_err(page_err)?;

    let mut lang = f.lang;
    if let Some(l) = overrides.lang.as_deref()
        && let Some(idx) = parser.language(l)
    {
        lang = idx;
        params.insert("lang", &Value::string(l));
    }
    if let Some(k) = overrides.kind {
        params.insert("kind", &Value::string(k.as_str()));
    }
    let bundled = f.info.kind == BundleKind::ContentResource;
    let mut info = f.info.clone();
    if let Some(Value::String(p)) = params.get("path").cloned()
        && !p.trim().is_empty()
    {
        let path = p.trim().replace('\\', "/");
        params.insert("path", &Value::string(&path));
        info = moved_path(&path, overrides.kind, &f.info, parser, &abs)?;
    }
    let mod_time = std::fs::metadata(&abs)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| Timestamp::try_from(t).ok());
    Ok(CapturedPage {
        lang,
        kind: overrides.kind,
        bundled,
        source: SourceFile {
            file: f.file,
            file_info: f.info,
            info,
            front_matter,
            text,
            body_offset,
            mod_time,
        },
        params,
        cascade,
    })
}

/// The path of a page moved by front matter `path`: a path without an extension names a
/// bundle (`/custom/moved` → `/custom/moved/index.md`, `_index.md` for branch kinds).
fn moved_path(
    path: &str,
    kind: Option<PageKind>,
    file: &PathInfo,
    parser: &PathParser,
    abs: &Path,
) -> Result<PathInfo, ModelError> {
    let has_ext = !neohugo_base::paths::ext(neohugo_base::paths::base(path)).is_empty();
    let full = if has_ext {
        path.to_owned()
    } else {
        let branch = kind.map_or(file.kind == BundleKind::Branch, PageKind::is_branch);
        let ext = if file.ext.is_empty() { "md" } else { &file.ext };
        let index = if branch { "_index" } else { "index" };
        format!("{}/{index}.{ext}", path.trim_end_matches('/'))
    };
    match parser.parse(Component::Content, &full) {
        Parsed::File(info) if info.kind.is_page() => Ok(*info),
        _ => Err(ModelError::Page {
            path: abs.to_owned(),
            source: neohugo_page::PageError::Field {
                key: "path".to_owned(),
                message: format!("{path:?} is not a content path"),
            },
        }),
    }
}
