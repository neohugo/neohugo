//! `post_css`: the `postcss` CLI with the project's PostCSS configuration, the CSS on stdin.

use serde_json::Value as Json;

use super::assets::AssetsView;
use super::css_imports::{InlineImports, Inliner};
use super::exec::{self, Tool};
use super::{Output, PipeError, TransformEnv, opt_bool, opt_string, option_entries, text};
use crate::store::{Resource, ResourceStore};

/// The `post_css` options.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PostCssOptions {
    /// `config`: the configuration file (default `postcss.config.js`; a directory is searched
    /// by postcss-cli itself).
    pub config: Option<String>,
    /// `noMap` (also `no-map`): `--no-map`.
    pub no_map: bool,
    /// `use`: plugins when there is no configuration file (space separated).
    pub use_plugins: Option<String>,
    pub parser: Option<String>,
    pub stringifier: Option<String>,
    pub syntax: Option<String>,
    /// `inlineImports` (default off) and `skipInlineImportsNotFound`.
    pub inline_imports: InlineImports,
}

impl PostCssOptions {
    /// Decodes the template's options map (keys case-insensitive; `null`: the defaults).
    ///
    /// # Errors
    /// An option of the wrong type.
    pub fn from_json(v: &Json) -> Result<Self, PipeError> {
        let mut o = Self::default();
        let (mut inline, mut skip) = (false, false);
        let non_empty = |s: String| (!s.is_empty()).then_some(s);
        for (key, v) in option_entries(v)? {
            match key.as_str() {
                "config" => o.config = non_empty(opt_string("config", v)?),
                "nomap" | "no-map" => o.no_map |= opt_bool("noMap", v)?,
                "use" => o.use_plugins = non_empty(opt_string("use", v)?),
                "parser" => o.parser = non_empty(opt_string("parser", v)?),
                "stringifier" => o.stringifier = non_empty(opt_string("stringifier", v)?),
                "syntax" => o.syntax = non_empty(opt_string("syntax", v)?),
                "inlineimports" => inline = opt_bool("inlineImports", v)?,
                "skipinlineimportsnotfound" => skip = opt_bool("skipInlineImportsNotFound", v)?,
                _ => {}
            }
        }
        o.inline_imports = InlineImports::new(inline, skip);
        Ok(o)
    }

    fn args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if self.no_map {
            args.push("--no-map".to_owned());
        }
        if let Some(u) = &self.use_plugins {
            args.push("--use".to_owned());
            args.extend(u.split_whitespace().map(str::to_owned));
        }
        for (flag, v) in [
            ("--parser", &self.parser),
            ("--stringifier", &self.stringifier),
            ("--syntax", &self.syntax),
        ] {
            if let Some(v) = v {
                args.push(flag.to_owned());
                args.push(v.clone());
            }
        }
        args
    }
}

pub(super) fn run(
    store: &ResourceStore,
    env: &TransformEnv,
    src: &Resource,
    o: &PostCssOptions,
    input: &[u8],
) -> Result<Output, PipeError> {
    const DEFAULT_CONFIG: &str = "postcss.config.js";
    let name = o.config.as_deref().unwrap_or(DEFAULT_CONFIG);
    let config = exec::config_file(store, env, name);
    if config.is_none() && o.config.is_some() && !env.project_dir.join(name).is_dir() {
        return Err(PipeError::ConfigNotFound {
            tool: "postcss",
            name: name.to_owned(),
        });
    }
    let mut args = Vec::new();
    if let Some(c) = config.or_else(|| o.config.as_ref().map(|c| env.project_dir.join(c))) {
        args.push("--config".to_owned());
        args.push(c.display().to_string());
    }
    args.extend(o.args());
    let css = match o.inline_imports.missing() {
        Some(missing) => Inliner::new(AssetsView::new(store.cfg.vfs.as_deref()), missing)
            .inline(text(input)?, src.link.as_str())?
            .into_bytes(),
        None => input.to_vec(),
    };
    Ok(Output {
        bytes: exec::run(store, env, Tool::PostCss, &args, &css)?,
        source_map: None,
    })
}
