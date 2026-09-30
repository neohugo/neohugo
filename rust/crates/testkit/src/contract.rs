//! The template contract (REWRITE_PLAN.md §4.1 item 3, §4.8): every converted template must load
//! into a Tera instance that knows exactly the names of `neohugo_funcs::spec::FUNCS`, and every
//! call must use declared kwargs.
//!
//! Tera validates names (filters, functions, tests, components, include targets, blocks) when
//! templates are added, but kwargs only when a call runs. [`scan_calls`] therefore reads the
//! calls of a template with a small tokenizer and [`kwarg_findings`] checks them against the spec.
//!
//! Template sets: one per site under `rust/sites/<site>/` (its `layouts/**` by relative name,
//! `assets/**` as `assets/<rel>`) and one more per docs patch variant (`patches/<variant>/**`
//! overlaid on the site by path), each plus the embedded templates of
//! `crates/layouts/embedded/**` (as `_embedded/<rel>`, with an empty stub for every
//! `spec::EMBEDDED_TEMPLATES` name not yet written).

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use neohugo_funcs::spec::{self, NameKind};
use tera::Tera;

/// One template: its Tera name, the file it came from and its source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateSource {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
}

/// The templates that are loaded together, e.g. `docs` or `docs+i01`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateSet {
    pub label: String,
    pub templates: Vec<TemplateSource>,
}

/// A contract violation found without Tera.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.path.display(), self.line, self.message)
    }
}

/// `rust/` of this checkout.
#[must_use]
pub fn rust_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `rust/docs/template-api.md`.
#[must_use]
pub fn template_api_path() -> PathBuf {
    rust_dir().join("docs/template-api.md")
}

/// The contract instance: `_embedded/` as fallback prefix, a placeholder for every `FUNCS` name,
/// then one `add_raw_templates` call with `templates` plus empty stubs for the embedded templates
/// they do not contain.
///
/// # Errors
/// Tera's load error: syntax, unknown names, inheritance, include cycles.
pub fn contract_instance(templates: &[TemplateSource]) -> Result<Tera, tera::Error> {
    let mut tera = Tera::default();
    tera.set_fallback_prefixes([spec::EMBEDDED_PREFIX])?;
    neohugo_funcs::register_placeholders(&mut tera);
    let stubs: Vec<(String, &str)> = spec::EMBEDDED_TEMPLATES
        .iter()
        .map(|name| format!("{}{name}", spec::EMBEDDED_PREFIX))
        .filter(|name| !templates.iter().any(|t| &t.name == name))
        .map(|name| (name, ""))
        .collect();
    tera.add_raw_templates(
        templates
            .iter()
            .map(|t| (t.name.as_str(), t.source.as_str()))
            .chain(stubs.iter().map(|(n, s)| (n.as_str(), *s))),
    )?;
    Ok(tera)
}

/// Every template set of the checkout (see the module docs), sorted by label.
///
/// # Errors
/// I/O errors reading `rust/sites` or `crates/layouts/embedded`.
pub fn template_sets() -> io::Result<Vec<TemplateSet>> {
    let root = rust_dir();
    let embedded = read_tree(&root.join("crates/layouts/embedded"), spec::EMBEDDED_PREFIX)?;
    let mut sets = Vec::new();
    let sites = root.join("sites");
    for site in sorted_dirs(&sites)? {
        let label = file_name(&site);
        let mut base = read_tree(&site.join("layouts"), "")?;
        base.extend(read_tree(&site.join("assets"), "assets/")?);
        for variant in sorted_dirs(&site.join("patches"))? {
            let mut templates = base.clone();
            for patch in read_tree(&variant, "")? {
                let name = patch
                    .name
                    .strip_prefix("layouts/")
                    .map_or_else(|| patch.name.clone(), str::to_owned);
                templates.retain(|t| t.name != name);
                templates.push(TemplateSource { name, ..patch });
            }
            templates.extend(embedded.iter().cloned());
            sets.push(TemplateSet {
                label: format!("{label}+{}", file_name(&variant)),
                templates,
            });
        }
        base.extend(embedded.iter().cloned());
        sets.push(TemplateSet {
            label,
            templates: base,
        });
    }
    if sets.is_empty() {
        sets.push(TemplateSet {
            label: "embedded".to_owned(),
            templates: embedded,
        });
    }
    sets.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(sets)
}

/// The kwargs findings of one template: calls of `FUNCS` names with an undeclared kwarg or
/// without a required one. Names unknown to `FUNCS` are left to Tera's own validation.
#[must_use]
pub fn kwarg_findings(t: &TemplateSource) -> Vec<Finding> {
    let mut out = Vec::new();
    for call in scan_calls(&t.source) {
        let Some(spec) = spec::FUNCS
            .iter()
            .find(|f| f.name == call.name && f.kind == call.kind)
        else {
            continue;
        };
        if let Err(message) =
            neohugo_funcs::check_kwargs(spec, call.kwargs.iter().map(String::as_str))
        {
            out.push(Finding {
                path: t.path.clone(),
                line: call.line,
                message,
            });
        }
    }
    out
}

/// One filter, function or test call found in a template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub kind: NameKind,
    pub name: String,
    pub kwargs: Vec<String>,
    /// 1-based line of the name.
    pub line: usize,
}

/// The filter, function and test calls of a template, in source order. Comments and
/// `{% raw %}` sections are skipped; component definitions are not calls.
#[must_use]
pub fn scan_calls(source: &str) -> Vec<Call> {
    let mut calls = Vec::new();
    for tag in tags(source) {
        calls_in_tag(source, &tag, &mut calls);
    }
    calls
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok<'s> {
    Ident(&'s str),
    Str,
    Num,
    Punct(&'s str),
}

/// The tokens of one `{{ … }}` or `{% … %}` tag, with byte offsets into the source.
type Tag<'s> = Vec<(Tok<'s>, usize)>;

const KEYWORDS: &[&str] = &[
    "and",
    "or",
    "not",
    "is",
    "in",
    "if",
    "elif",
    "else",
    "endif",
    "for",
    "endfor",
    "set",
    "set_global",
    "endset",
    "block",
    "endblock",
    "extends",
    "include",
    "filter",
    "endfilter",
    "component",
    "endcomponent",
    "raw",
    "endraw",
    "break",
    "continue",
    "true",
    "false",
    "none",
    "True",
    "False",
];

/// Splits the source into tags and tokenizes each; `{# #}` comments and raw sections are skipped.
fn tags(source: &str) -> Vec<Tag<'_>> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(off) = source[i..].find('{') {
        let start = i + off;
        match bytes.get(start + 1) {
            Some(b'#') => {
                i = source[start..]
                    .find("#}")
                    .map_or(bytes.len(), |e| start + e + 2);
            }
            Some(b'{' | b'%') => {
                let (tag, end) = lex_tag(source, start + 2);
                i = end;
                let is_raw = matches!(
                    tag.iter().find(|(t, _)| !matches!(t, Tok::Punct("-"))),
                    Some((Tok::Ident("raw"), _))
                ) && bytes[start + 1] == b'%';
                if is_raw {
                    i = find_endraw(source, end);
                } else {
                    out.push(tag);
                }
            }
            _ => i = start + 1,
        }
    }
    out
}

fn find_endraw(source: &str, from: usize) -> usize {
    let mut i = from;
    while let Some(off) = source[i..].find("{%") {
        let start = i + off;
        let (tag, end) = lex_tag(source, start + 2);
        if tag.iter().any(|(t, _)| *t == Tok::Ident("endraw")) {
            return end;
        }
        i = end;
    }
    source.len()
}

/// Tokenizes from `pos` to the closing `}}` or `%}` at brace depth 0; returns the tokens and the
/// offset after the tag.
fn lex_tag(source: &str, mut pos: usize) -> (Tag<'_>, usize) {
    let bytes = source.as_bytes();
    let mut toks = Vec::new();
    let mut depth = 0usize;
    while pos < bytes.len() {
        let c = bytes[pos];
        let next = bytes.get(pos + 1).copied();
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => pos += 1,
            b'}' if depth == 0 && next == Some(b'}') => return (toks, pos + 2),
            b'%' if depth == 0 && next == Some(b'}') => return (toks, pos + 2),
            b'"' | b'\'' | b'`' => {
                let close = source[pos + 1..]
                    .find(c as char)
                    .map_or(bytes.len(), |e| pos + 1 + e + 1);
                toks.push((Tok::Str, pos));
                pos = close;
            }
            b'0'..=b'9' => {
                let len = source[pos..]
                    .bytes()
                    .take_while(|b| b.is_ascii_alphanumeric() || *b == b'.' || *b == b'_')
                    .count();
                toks.push((Tok::Num, pos));
                pos += len;
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let len = source[pos..]
                    .bytes()
                    .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    .count();
                toks.push((Tok::Ident(&source[pos..pos + len]), pos));
                pos += len;
            }
            _ => {
                let two = source.get(pos..pos + 2).unwrap_or("");
                let len = if ["==", "!=", "<=", ">=", "?.", "?[", "//", "**", "</"].contains(&two) {
                    2
                } else {
                    // a multi-byte character outside a string: skip it whole
                    source[pos..].chars().next().map_or(1, char::len_utf8)
                };
                let p = &source[pos..pos + len];
                match p {
                    "(" | "[" | "{" => depth += 1,
                    ")" | "]" | "}" => depth = depth.saturating_sub(1),
                    _ => {}
                }
                toks.push((Tok::Punct(p), pos));
                pos += len;
            }
        }
    }
    (toks, pos)
}

fn line_of(source: &str, offset: usize) -> usize {
    source[..offset].bytes().filter(|b| *b == b'\n').count() + 1
}

fn calls_in_tag(source: &str, tag: &Tag<'_>, out: &mut Vec<Call>) {
    let first = tag.iter().position(|(t, _)| !matches!(t, Tok::Punct("-")));
    if matches!(first.map(|f| &tag[f].0), Some(Tok::Ident("component"))) {
        return; // a definition: its parameters are not kwargs of a call
    }
    for (i, (tok, offset)) in tag.iter().enumerate() {
        let Tok::Ident(name) = tok else { continue };
        let prev = i.checked_sub(1).map(|j| &tag[j].0);
        let prev2 = i.checked_sub(2).map(|j| &tag[j].0);
        let opens = matches!(tag.get(i + 1), Some((Tok::Punct("("), _)));
        let kind = match (prev, prev2) {
            (Some(Tok::Punct("|")), _) => NameKind::Filter,
            (Some(Tok::Ident("filter")), _) if first == Some(i - 1) => NameKind::Filter,
            (Some(Tok::Ident("is")), _) | (Some(Tok::Ident("not")), Some(Tok::Ident("is"))) => {
                if *name == "not" {
                    continue;
                }
                NameKind::Test
            }
            (Some(Tok::Punct("." | "?.")), _) => continue,
            _ if opens && !KEYWORDS.contains(name) => NameKind::Function,
            _ => continue,
        };
        let kwargs = if opens {
            kwargs_at(tag, i + 1)
        } else {
            Vec::new()
        };
        out.push(Call {
            kind,
            name: (*name).to_owned(),
            kwargs,
            line: line_of(source, *offset),
        });
    }
}

/// The kwarg names of the argument list opening at `open`: identifiers followed by `=` at
/// depth 1, right after `(` or `,`.
fn kwargs_at(tag: &Tag<'_>, open: usize) -> Vec<String> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    for j in open..tag.len() {
        match &tag[j].0 {
            Tok::Punct("(" | "[" | "{") => depth += 1,
            Tok::Punct(")" | "]" | "}") => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
            Tok::Ident(name)
                if depth == 1
                    && matches!(tag.get(j + 1), Some((Tok::Punct("="), _)))
                    && matches!(tag[j - 1].0, Tok::Punct("(" | ",")) =>
            {
                names.push((*name).to_owned());
            }
            _ => {}
        }
    }
    names
}

fn sorted_dirs(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    match fs::read_dir(dir) {
        Ok(entries) => {
            for e in entries {
                let e = e?;
                if e.file_type()?.is_dir() {
                    dirs.push(e.path());
                }
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    dirs.sort();
    Ok(dirs)
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Every file under `dir` (absent: none) as a template named `prefix` + its slash-separated
/// relative path, sorted by name.
fn read_tree(dir: &Path, prefix: &str) -> io::Result<Vec<TemplateSource>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_owned()];
    while let Some(d) = stack.pop() {
        for sub in sorted_dirs(&d)? {
            stack.push(sub);
        }
        let entries = match fs::read_dir(&d) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        for e in entries {
            let e = e?;
            if e.file_type()?.is_file() {
                let path = e.path();
                let rel = path
                    .strip_prefix(dir)
                    .expect("walked below dir")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push(TemplateSource {
                    name: format!("{prefix}{rel}"),
                    source: fs::read_to_string(&path)?,
                    path,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}
