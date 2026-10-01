//! `to_css`: Sass and SCSS compiled with grass (dart-sass semantics).
//!
//! Imports are resolved like dart-sass does (relative to the importing file, then the load
//! paths; partials, `index` files, `.sass`/`.scss`/`.css`), with the entry placed at its asset
//! path inside a virtual assets root, so relative imports see the whole assets union view
//! (theme and module mounts included). The load paths are the entry's directory in the assets
//! view, then each `includePaths` entry that exists (relative to the project directory).
//! `@import "hugo:vars"` reads a stylesheet of the `vars` option. Plain CSS imports
//! (`@import "x.css"`, `url(…)`, media queries) stay in the output.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use super::assets::AssetsView;
use super::{Output, PipeError, bad, opt_bool, opt_string, opt_strings, option_entries, text};
use crate::store::{Resource, ResourceStore};

/// The virtual directory the assets view is mounted at for the compiler.
const ROOT: &str = "/@neohugo-assets";
/// The file name `@import "hugo:vars"` resolves to.
const VARS_FILE: &str = "hugo:vars.scss";

/// `outputStyle`. grass writes `expanded` and `compressed`; `nested` (LibSass's default) and
/// `compact` are written expanded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OutputStyle {
    #[default]
    Nested,
    Expanded,
    Compact,
    Compressed,
}

impl OutputStyle {
    /// Case-insensitive; an unknown style is `nested`, as LibSass has it.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "expanded" => Self::Expanded,
            "compact" => Self::Compact,
            "compressed" => Self::Compressed,
            _ => Self::Nested,
        }
    }

    fn grass(self) -> grass::OutputStyle {
        match self {
            Self::Compressed => grass::OutputStyle::Compressed,
            Self::Nested | Self::Expanded | Self::Compact => grass::OutputStyle::Expanded,
        }
    }
}

/// A value of the `vars` option.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SassVar {
    /// A plain string: written as is when it looks like a CSS value (`#fff`, `24px`,
    /// `calc(…)`), else as `unquote("…")`.
    Auto(String),
    /// A number (its text).
    Number(String),
    /// A quoted string (`css.Quoted`).
    Quoted(String),
    /// Written as is (`css.Unquoted`); booleans too.
    Unquoted(String),
}

impl SassVar {
    /// Plain strings, numbers and booleans, or the typed form
    /// `{"t": "css.QuotedString" | "css.UnquotedString", "v": "…"}`.
    fn from_json(name: &str, v: &Json) -> Result<Self, PipeError> {
        Ok(match v {
            Json::String(s) => Self::Auto(s.clone()),
            Json::Number(n) => Self::Number(n.to_string()),
            Json::Bool(b) => Self::Unquoted(b.to_string()),
            Json::Object(m) => {
                let value = m.get("v").map(|v| opt_string(name, v)).transpose()?;
                match (m.get("t").and_then(Json::as_str), value) {
                    (Some("css.QuotedString"), Some(s)) => Self::Quoted(s),
                    (Some("css.UnquotedString"), Some(s)) => Self::Unquoted(s),
                    _ => return Err(bad(name, format!("unsupported Sass variable {v}"))),
                }
            }
            other => return Err(bad(name, format!("unsupported Sass variable {other}"))),
        })
    }

    fn css(&self) -> String {
        match self {
            Self::Auto(s) if is_css_value(s) => s.clone(),
            Self::Auto(s) => format!("unquote({})", quote(s)),
            Self::Number(s) | Self::Unquoted(s) => s.clone(),
            Self::Quoted(s) => quote(s),
        }
    }
}

/// The `to_css` options.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ToCssOptions {
    /// `targetPath`: where the CSS is published (default: the source with `.css`).
    pub target_path: Option<String>,
    pub output_style: OutputStyle,
    /// `includePaths`, relative to the project directory.
    pub include_paths: Vec<String>,
    /// `vars`: the `hugo:vars` stylesheet (names without `$`).
    pub vars: BTreeMap<String, SassVar>,
    /// `precision` (accepted; grass always writes up to 10 decimals).
    pub precision: Option<u32>,
    /// `enableSourceMap` (accepted; grass writes no source maps).
    pub enable_source_map: bool,
}

impl ToCssOptions {
    /// Decodes the template's options map (keys case-insensitive; `null`: the defaults).
    /// `transpiler`, `sourceMapIncludeSources` and `silenceDeprecations` are accepted and
    /// have no effect; unknown keys are ignored.
    ///
    /// # Errors
    /// An option of the wrong type (`precision: "x"`).
    pub fn from_json(v: &Json) -> Result<Self, PipeError> {
        let mut o = Self::default();
        for (key, v) in option_entries(v)? {
            match key.as_str() {
                "targetpath" => {
                    let p = opt_string("targetPath", v)?.replace('\\', "/");
                    let p = p.trim_start_matches('/');
                    o.target_path = (!p.is_empty()).then(|| p.to_owned());
                }
                "outputstyle" => {
                    o.output_style = OutputStyle::parse(&opt_string("outputStyle", v)?);
                }
                "includepaths" => o.include_paths = opt_strings("includePaths", v)?,
                "precision" => {
                    let s = opt_string("precision", v)?;
                    let p: u32 = s
                        .parse()
                        .map_err(|_| bad("precision", format!("expected an integer, got {s:?}")))?;
                    o.precision = (p > 0).then_some(p);
                }
                "enablesourcemap" => o.enable_source_map = opt_bool("enableSourceMap", v)?,
                "vars" => {
                    let Json::Object(m) = v else {
                        return Err(bad("vars", format!("expected a map, got {v}")));
                    };
                    for (name, value) in m {
                        let name = name.trim_start_matches('$').to_owned();
                        let var = SassVar::from_json(&name, value)?;
                        o.vars.insert(name, var);
                    }
                }
                _ => {}
            }
        }
        Ok(o)
    }

    /// The `hugo:vars` stylesheet.
    fn vars_sheet(&self) -> String {
        self.vars
            .iter()
            .map(|(k, v)| format!("${k}: {};\n", v.css()))
            .collect()
    }
}

/// A CSS string literal.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\a "),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Whether a plain string is written unquoted: a hex colour, a function call (`calc(…)`,
/// `url(…)`, `rgba(…)`) or a number with a unit (`24px`, `1.5rem`, `50%`).
fn is_css_value(s: &str) -> bool {
    if let Some(hex) = s.strip_prefix('#') {
        return (3..=6).contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    if let Some(open) = s.find('(') {
        return open > 0
            && s[..open]
                .bytes()
                .all(|b| b.is_ascii_alphabetic() || b == b'-');
    }
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return false;
    }
    let mut rest = &s[digits..];
    if let Some(frac) = rest.strip_prefix('.') {
        let n = frac.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return false;
        }
        rest = &frac[n..];
    }
    !rest.is_empty()
        && rest
            .bytes()
            .all(|b| b.is_ascii_alphabetic() || b == b'%' || b == b'-')
}

/// grass's file system: the virtual assets root, the entry, `hugo:vars`, and real files.
#[derive(Debug)]
struct SassFs<'a> {
    view: Assets<'a>,
    entry: PathBuf,
    input: &'a str,
    vars: Option<String>,
}

/// [`AssetsView`] with a `Debug` grass requires.
#[derive(Clone, Copy)]
struct Assets<'a>(AssetsView<'a>);

impl std::fmt::Debug for Assets<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AssetsView")
    }
}

/// The asset path of a virtual path.
fn virtual_rel(p: &Path) -> Option<String> {
    let rel = p.strip_prefix(ROOT).ok()?;
    Some(rel.to_str()?.replace('\\', "/"))
}

impl SassFs<'_> {
    fn is_vars(&self, p: &Path) -> bool {
        self.vars.is_some() && p.file_name().is_some_and(|n| n == VARS_FILE)
    }
}

impl grass::Fs for SassFs<'_> {
    fn is_dir(&self, path: &Path) -> bool {
        match virtual_rel(path) {
            Some(rel) => self.view.0.is_dir(&rel),
            None => path.is_dir(),
        }
    }

    fn is_file(&self, path: &Path) -> bool {
        if path == self.entry || self.is_vars(path) {
            return true;
        }
        match virtual_rel(path) {
            Some(rel) => self.view.0.file(&rel).is_some(),
            None => path.is_file(),
        }
    }

    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        if path == self.entry {
            return Ok(self.input.as_bytes().to_vec());
        }
        if self.is_vars(path) {
            return Ok(self.vars.clone().unwrap_or_default().into_bytes());
        }
        match virtual_rel(path) {
            Some(rel) => match self.view.0.file(&rel) {
                Some(real) => std::fs::read(real),
                None => Err(std::io::ErrorKind::NotFound.into()),
            },
            None => std::fs::read(path),
        }
    }
}

/// Compiles `input` (the content of `src`).
pub(super) fn run(
    store: &ResourceStore,
    src: &Resource,
    o: &ToCssOptions,
    input: &[u8],
) -> Result<Output, PipeError> {
    let input = text(input)?;
    let env = &store.cfg.transforms;
    let view = AssetsView::new(store.cfg.vfs.as_deref());
    let link = src.link.as_str();
    let entry = PathBuf::from(format!("{ROOT}{link}"));
    let base_dir = entry
        .parent()
        .map_or_else(|| PathBuf::from(ROOT), Path::to_path_buf);
    let mut load_paths = vec![base_dir];
    for ip in &o.include_paths {
        let p = Path::new(ip);
        let p = if p.is_absolute() {
            p.to_path_buf()
        } else {
            env.project_dir
                .join(neohugo_base::paths::clean(&format!("/{ip}")).trim_start_matches('/'))
        };
        if p.is_dir() {
            load_paths.push(p);
        }
    }
    let fs = SassFs {
        view: Assets(view),
        entry: entry.clone(),
        input,
        vars: (!o.vars.is_empty()).then(|| o.vars_sheet()),
    };
    let syntax = if link.to_ascii_lowercase().ends_with(".sass") {
        grass::InputSyntax::Sass
    } else {
        grass::InputSyntax::Scss
    };
    let options = grass::Options::default()
        .fs(&fs)
        .style(o.output_style.grass())
        .load_paths(&load_paths)
        .input_syntax(syntax)
        .quiet(true);
    match grass::from_path(&entry, &options) {
        Ok(css) => Ok(Output {
            bytes: css.into_bytes(),
            source_map: None,
        }),
        Err(e) => Err(match e.kind() {
            grass::ErrorKind::ParseError { message, loc, .. } => PipeError::Sass {
                file: real_name(&view, &entry, src, loc.file.name()),
                line: loc.begin.line + 1,
                column: loc.begin.column + 1,
                message,
            },
            grass::ErrorKind::IoError(e) => PipeError::SassInput(e.to_string()),
            grass::ErrorKind::FromUtf8Error(s) => PipeError::SassInput(s),
            _ => PipeError::SassInput("Sass compilation failed".to_owned()),
        }),
    }
}

/// The real file behind a name grass reports.
fn real_name(view: &AssetsView<'_>, entry: &Path, src: &Resource, name: &str) -> String {
    let p = Path::new(name);
    if p == entry {
        return view
            .file(src.link.as_str())
            .map_or_else(|| src.name.clone(), |f| f.display().to_string());
    }
    if p.file_name().is_some_and(|n| n == VARS_FILE) {
        return "hugo:vars".to_owned();
    }
    match virtual_rel(p) {
        Some(rel) => view.file(&rel).map_or(rel, |f| f.display().to_string()),
        None => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_values() {
        for s in [
            "#fff",
            "#3a7bd5",
            "calc(10px + 2px)",
            "url(a.png)",
            "24px",
            "1.5rem",
            "50%",
        ] {
            assert!(is_css_value(s), "{s}");
        }
        for s in ["Helvetica Neue", "#12345678", "(x)", "3", "px", "1.px", ""] {
            assert!(!is_css_value(s), "{s}");
        }
        assert_eq!(quote(r#"a "b" \c"#), r#""a \"b\" \\c""#);
    }
}
