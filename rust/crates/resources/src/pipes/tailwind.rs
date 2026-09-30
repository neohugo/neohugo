//! `tailwind_css`: the Tailwind CSS v4 CLI (`tailwindcss --input=- --cwd <project>`).
//!
//! The input's `@import`s of asset files are inlined first (Tailwind's own `tailwindcss`
//! imports stay), since the CLI reads the CSS from stdin and would resolve relative imports
//! against the project directory. `@source` and `@plugin` are the CLI's: relative to the
//! project directory (`--cwd`), plugins from its `node_modules` (and `NODE_PATH`).

use serde_json::Value as Json;

use super::assets::AssetsView;
use super::css_imports::{InlineImports, Inliner};
use super::exec::{self, Tool};
use super::{Output, PipeError, TransformEnv, opt_bool, option_entries, text};
use crate::store::{Resource, ResourceStore};

/// The `tailwind_css` options.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TailwindOptions {
    /// `minify`: `--minify`.
    pub minify: bool,
    /// `optimize`: `--optimize`.
    pub optimize: bool,
    /// On unless `disableInlineImports`; `skipInlineImportsNotFound` keeps missing ones.
    pub inline_imports: InlineImports,
}

impl Default for TailwindOptions {
    fn default() -> Self {
        Self {
            minify: false,
            optimize: false,
            inline_imports: InlineImports::Enabled,
        }
    }
}

impl TailwindOptions {
    /// Decodes the template's options map (keys case-insensitive; `null`: the defaults).
    ///
    /// # Errors
    /// An option of the wrong type.
    pub fn from_json(v: &Json) -> Result<Self, PipeError> {
        let mut o = Self::default();
        let (mut disable, mut skip) = (false, false);
        for (key, v) in option_entries(v)? {
            match key.as_str() {
                "minify" => o.minify = opt_bool("minify", v)?,
                "optimize" => o.optimize = opt_bool("optimize", v)?,
                "disableinlineimports" => disable = opt_bool("disableInlineImports", v)?,
                "skipinlineimportsnotfound" => skip = opt_bool("skipInlineImportsNotFound", v)?,
                _ => {}
            }
        }
        o.inline_imports = InlineImports::new(!disable, skip);
        Ok(o)
    }

    fn args(&self, env: &TransformEnv) -> Vec<String> {
        let mut args = vec![
            "--input=-".to_owned(),
            "--cwd".to_owned(),
            env.project_dir.display().to_string(),
        ];
        if self.minify {
            args.push("--minify".to_owned());
        }
        if self.optimize {
            args.push("--optimize".to_owned());
        }
        args
    }
}

pub(super) fn run(
    store: &ResourceStore,
    env: &TransformEnv,
    src: &Resource,
    o: &TailwindOptions,
    input: &[u8],
) -> Result<Output, PipeError> {
    let css = match o.inline_imports.missing() {
        Some(missing) => Inliner::new(AssetsView::new(store.cfg.vfs.as_deref()), missing)
            .inline(text(input)?, src.link.as_str())?
            .into_bytes(),
        None => input.to_vec(),
    };
    Ok(Output {
        bytes: exec::run(store, env, Tool::TailwindCss, &o.args(env), &css)?,
        source_map: None,
    })
}
