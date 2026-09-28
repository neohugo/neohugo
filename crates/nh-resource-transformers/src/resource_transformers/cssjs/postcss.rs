//! Port of `resources/resource_transformers/cssjs/postcss.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `cssjs/postcss.go`: spawn `node_modules/.bin/postcss --config <abs postcss.config.js>` via
//! `hexec.Npx` with stdin = CSS, stdout = result, env = `GetExecEnviron` (NODE_PATH, PWD,
//! HUGO_ENVIRONMENT=production, HUGO_FILE_*...). Go leaves the child's cwd to the process (the
//! golden build ran from the site dir; purgecss reads `./hugo_stats.json`); the port sets it to
//! the working dir (PORTING.md, deviation).

use std::io::Write;
use std::sync::{Arc, Mutex};

use go_value::{Map, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;
use nh_config::hexec::CommandOptions;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;
use nh_resources::transform::{ResourceTransformation, ResourceTransformationCtx};

use super::inline_imports::{InlineImports, new_import_resolver};
use crate::resource_transformers::js::transform::opts_value;

/// Go: `cssjs.PostCSSOptions` — some of the options from https://github.com/postcss/postcss-cli.
#[derive(Clone, Debug, Default)]
pub struct PostCssOptions {
    /// Set a custom path to look for a config file.
    pub config: String,
    /// Disable the default inline sourcemaps
    pub no_map: bool,
    /// Options for when not using a config file: list of postcss plugins to use
    pub use_: String,
    /// Custom postcss parser
    pub parser: String,
    /// Custom postcss stringifier
    pub stringifier: String,
    /// Custom postcss syntax
    pub syntax: String,
    /// Go `InlineImports.InlineImports` (squashed).
    pub inline_imports: bool,
    /// Go `InlineImports.SkipInlineImportsNotFound` (squashed).
    pub skip_inline_imports_not_found: bool,
    /// Go `InlineImports.DisableInlineImports` (squashed; tailwindcss only).
    pub disable_inline_imports: bool,
}

/// The mapstructure view of [`PostCssOptions`] (Go's field order, `InlineImports` squashed).
#[derive(Clone, Default)]
struct DecodedPostCssOptions {
    config: String,
    no_map: bool,
    inline_imports: InlineImports,
    use_: String,
    parser: String,
    stringifier: String,
    syntax: String,
}

nh_config::decode_struct!(DecodedPostCssOptions, "cssjs.PostCSSOptions", |s| vec![
    FieldRef::new("Config", &mut s.config),
    FieldRef::new("NoMap", &mut s.no_map),
    FieldRef::squash("InlineImports", &mut s.inline_imports),
    FieldRef::new("Use", &mut s.use_),
    FieldRef::new("Parser", &mut s.parser),
    FieldRef::new("Stringifier", &mut s.stringifier),
    FieldRef::new("Syntax", &mut s.syntax),
]);

/// Go: `cssjs.PostCSSClient`.
pub struct PostCssClient {
    pub rs: Arc<Spec>,
}

/// NewPostCSSClient creates a new PostCSSClient with the given specification.
// Go: resources/resource_transformers/cssjs/postcss.go:NewPostCSSClient
pub fn new_post_css_client(rs: Arc<Spec>) -> PostCssClient {
    PostCssClient { rs }
}

// Go: resources/resource_transformers/cssjs/postcss.go:decodePostCSSOptions
fn decode_post_css_options(m: Option<&Map>) -> Result<PostCssOptions> {
    let mut opts = PostCssOptions::default();
    let Some(m) = m else {
        return Ok(opts);
    };
    let mut d = DecodedPostCssOptions::default();
    let err = nh_config::decode::weak_decode_into(&Value::map(m.clone()), &mut d).err();
    opts.config = d.config;
    opts.no_map = d.no_map;
    opts.use_ = d.use_;
    opts.parser = d.parser;
    opts.stringifier = d.stringifier;
    opts.syntax = d.syntax;
    opts.inline_imports = d.inline_imports.inline_imports;
    opts.disable_inline_imports = d.inline_imports.disable_inline_imports;
    opts.skip_inline_imports_not_found = d.inline_imports.skip_inline_imports_not_found;

    if !opts.no_map {
        // There was for a long time a discrepancy between documentation and
        // implementation for the noMap property, so we need to support both
        // camel and snake case.
        let v = m
            .entries
            .get(&go_value::GoString::from("no-map"))
            .cloned()
            .unwrap_or(Value::Invalid);
        opts.no_map = nh_common::cast::caste::to_bool_e(&v).unwrap_or(false);
    }

    match err {
        Some(e) => Err(e),
        None => Ok(opts),
    }
}

impl PostCssOptions {
    // Go: resources/resource_transformers/cssjs/postcss.go:(PostCSSOptions).toArgs
    fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if self.no_map {
            args.push("--no-map".to_string());
        }
        if !self.use_.is_empty() {
            args.push("--use".to_string());
            args.extend(
                go_unicode::strings::fields(self.use_.as_bytes())
                    .into_iter()
                    .map(|f| String::from_utf8_lossy(f).into_owned()),
            );
        }
        if !self.parser.is_empty() {
            args.push("--parser".to_string());
            args.push(self.parser.clone());
        }
        if !self.stringifier.is_empty() {
            args.push("--stringifier".to_string());
            args.push(self.stringifier.clone());
        }
        if !self.syntax.is_empty() {
            args.push("--syntax".to_string());
            args.push(self.syntax.clone());
        }
        args
    }
}

impl PostCssClient {
    /// Process transforms the given Resource with the PostCSS processor. `r` must be a resource
    /// adapter; Go calls `res.Transform` (no context).
    // Go: resources/resource_transformers/cssjs/postcss.go:Process
    pub fn process(
        &self,
        _ctx: &nh_tpl::template::TplContext,
        r: Arc<dyn Resource>,
        options: Option<&Map>,
    ) -> Result<Arc<dyn Resource>> {
        let res = nh_resources::transform::resource_adapter(&r)
            .ok_or_else(|| Error::new(format!("{} can not be transformed", r.tpl_type_name())))?;
        Ok(res.transform(vec![Arc::new(PostcssTransformation {
            rs: self.rs.clone(),
            optionsm: options.cloned(),
        })])?)
    }
}

/// Go: `postcssTransformation`.
struct PostcssTransformation {
    optionsm: Option<Map>,
    rs: Arc<Spec>,
}

/// A shared writer (Go's `bytes.Buffer` the command writes into).
#[derive(Clone, Default)]
pub(crate) struct SharedBuf(pub(crate) Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SharedBuf {
    pub(crate) fn take(&self) -> Vec<u8> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

/// Go: `neohugo.GetExecEnviron(workDir, cfg, rs.Assets.Fs)`: the `_jsconfig` files of the assets
/// fs become `HUGO_FILE_<NAME>` variables.
pub(crate) fn exec_environ(rs: &Spec) -> Vec<String> {
    let bc = rs.path_spec.cfg.base_config();
    let mut js_config_files = Vec::new();
    if let Ok(fis) = nh_hugofs::afero::read_dir(
        rs.path_spec.base_fs.assets.fs.as_ref(),
        nh_common::files::FOLDER_JS_CONFIG,
    ) {
        for fi in fis {
            js_config_files.push((fi.name().to_string(), fi.meta.filename.clone()));
        }
    }
    nh_config::neohugo::neohugo::get_exec_environ(
        &bc.working_dir,
        &rs.path_spec.cfg.environment(),
        &bc.publish_dir,
        &js_config_files,
    )
}

impl ResourceTransformation for PostcssTransformation {
    // Go: resources/resource_transformers/cssjs/postcss.go:(*postcssTransformation).Key
    fn key(&self) -> ResourceTransformationKey {
        ResourceTransformationKey::new("postcss", vec![opts_value(self.optionsm.as_ref())])
    }

    /// Transform shells out to postcss-cli to do the heavy lifting.
    // Go: resources/resource_transformers/cssjs/postcss.go:(*postcssTransformation).Transform
    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()> {
        const BINARY_NAME: &str = "postcss";

        let ex = self.rs.exec_helper.clone();

        let options = decode_post_css_options(self.optionsm.as_ref())?;

        let mut config_file = if !options.config.is_empty() {
            options.config.clone()
        } else {
            "postcss.config.js".to_string()
        };

        config_file = go_path::filepath::clean(&config_file).to_string();

        // We need an absolute filename to the config file.
        if !go_path::filepath::is_abs(&config_file) {
            config_file = self
                .rs
                .path_spec
                .base_fs
                .resolve_js_config_file(&config_file);
            if config_file.is_empty() && !options.config.is_empty() {
                // Only fail if the user specified config file is not found.
                return Err(Error::new(format!(
                    "postcss config {} not found",
                    go_strconv::quote(&options.config)
                )));
            }
        }

        let mut cmd_args: Vec<String> = Vec::new();

        if !config_file.is_empty() {
            self.rs.logger.infof(format!(
                "{BINARY_NAME}: use config file {}",
                go_strconv::quote(&config_file)
            ));
            cmd_args = vec!["--config".to_string(), config_file.clone()];
        }

        cmd_args.extend(options.to_args());

        let err_buf = SharedBuf::default();
        let out_buf = SharedBuf::default();

        let working_dir = self.rs.path_spec.cfg.base_config().working_dir.clone();

        let src = ctx.from.read_all()?;
        let mut imp = new_import_resolver(
            src.clone(),
            &ctx.in_path,
            InlineImports {
                inline_imports: options.inline_imports,
                disable_inline_imports: options.disable_inline_imports,
                skip_inline_imports_not_found: options.skip_inline_imports_not_found,
            },
            self.rs.path_spec.base_fs.assets.fs.clone(),
        );

        let mut input = src;
        if options.inline_imports {
            input = imp.resolve()?;
        }

        let cmd = ex.npx(
            BINARY_NAME,
            CommandOptions {
                args: cmd_args,
                stdin: Some(Box::new(std::io::Cursor::new(input))),
                stdout: Some(Box::new(out_buf.clone())),
                stderr: Some(Box::new(err_buf.clone())),
                env: exec_environ(&self.rs),
                dir: Some(working_dir),
            },
        );
        let cmd = match cmd {
            Ok(c) => c,
            Err(e) => {
                if nh_config::hexec::is_not_found(&e) {
                    // This may be on a CI server etc. Will fall back to pre-built assets.
                    return Err(Error::feature_not_available(e.message().to_string()));
                }
                return Err(e);
            }
        };

        let res = cmd.run();
        ctx.to.extend_from_slice(&out_buf.take());
        let stderr = err_buf.take();
        if !stderr.is_empty() {
            self.rs.logger.infof(format!(
                "{BINARY_NAME}: {}",
                String::from_utf8_lossy(&stderr)
            ));
        }
        if let Err(e) = res {
            if nh_config::hexec::is_not_found(&e) {
                return Err(Error::feature_not_available(e.message().to_string()));
            }
            return Err(imp.to_file_error(&String::from_utf8_lossy(&stderr)));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/cssjs/postcss.go (239 lines; 6/6 funcs executed)
//   types: PostCSSClient, InlineImports, PostCSSOptions, postcssTransformation
// OK L41-43: NewPostCSSClient(rs *resources.Spec) *PostCSSClient
// OK L45-59: decodePostCSSOptions(m map[string]any) (opts PostCSSOptions, err error)
// OK L67-69: (c *PostCSSClient) Process(res resources.ResourceTransformer, options map[string]any) (resource.Resource, error)
// OK L108-127: (opts PostCSSOptions) toArgs() []string
// OK L134-136: (t *postcssTransformation) Key() internal.ResourceTransformationKey
// OK L142-239: (t *postcssTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// ---------------------------------------------------------------------------
