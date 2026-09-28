//! Port of `resources/resource_transformers/tocss/scss/tocss.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `tocss/scss`: LibSass 3.6.6 via libsass-sys (golibsass wrapper): precision 0 -> 8,
//! outputStyle "compressed" = 3, include paths `[<assets real dirs>/scss, node_modules, assets/scss]`,
//! the Hugo importer (resolves `@import` in the assets fs; `prev == "stdin"` -> baseDir), entry
//! `@import "x.css"` protection regexes. OutPath: ReplaceOutPathExtension(".css").

use std::sync::Arc;

use go_value::{Map, Value};
use libsass_sys::transpiler::Transpiler;
use nh_common::Result;
use nh_common::herrors::{Error, FilePos};
use nh_config::decode::FieldRef;
use nh_hugofs::afero::Fs;
use nh_hugofs::filesystems::basefs::SourceFilesystem;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resources::resource_spec::Spec;
use nh_resources::transform::{ResourceTransformation, ResourceTransformationCtx};

use super::client::{replace_regular_imports_in, replace_regular_imports_out};
use super::client_extended::{ToCssTransformation, register_options_key};
use crate::resource_transformers::tocss::sass::helpers::{
    HUGO_VARS_NAMESPACE, TRANSPILER_LIB_SASS, create_vars_style_sheet,
};

/// Go: `scss.Options`.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Hugo, will by default, just replace the extension of the source to .css, e.g.
    /// "scss/main.scss" becomes "scss/main.css". You can control this by setting this, e.g.
    /// "styles/main.css" will create a Resource with that as a base for RelPermalink etc.
    pub target_path: String,
    /// Hugo automatically adds the entry directories (where the main.scss lives) for project and
    /// themes to the list of include paths sent to LibSASS. Any paths set in this setting will
    /// be appended. Note that these will be treated as relative to the working dir, i.e. no
    /// include paths outside the project/themes.
    pub include_paths: Vec<String>,
    /// Default is nested. One of nested, expanded, compact, compressed.
    pub output_style: String,
    /// Precision of floating point math.
    pub precision: i64,
    /// When enabled, Hugo will generate a source map.
    pub enable_source_map: bool,
    /// Not a libsass option (Dart Sass only): never decoded, kept for the skeleton's callers.
    pub source_map_include_sources: bool,
    /// Vars will be available in 'hugo:vars', e.g: `@import "hugo:vars";` (`None` = nil map).
    pub vars: Option<Map>,
}

/// The mapstructure view of [`Options`] (Go's struct fields in order; `Vars` as a map).
#[derive(Clone)]
struct DecodedOptions {
    target_path: String,
    include_paths: Vec<String>,
    output_style: String,
    precision: i64,
    enable_source_map: bool,
    /// A nil and an empty map give the same stylesheet and the same key hash.
    vars: Map,
}

impl Default for DecodedOptions {
    fn default() -> Self {
        DecodedOptions {
            target_path: String::new(),
            include_paths: Vec::new(),
            output_style: String::new(),
            precision: 0,
            enable_source_map: false,
            vars: Map::new(go_value::MapType::StringAny),
        }
    }
}

nh_config::decode_struct!(DecodedOptions, "scss.Options", |s| vec![
    FieldRef::new("TargetPath", &mut s.target_path),
    FieldRef::new("IncludePaths", &mut s.include_paths),
    FieldRef::new("OutputStyle", &mut s.output_style),
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("EnableSourceMap", &mut s.enable_source_map),
    FieldRef::new("Vars", &mut s.vars),
]);

/// Go: `scss.Client`.
#[derive(Clone)]
pub struct Client {
    pub rs: Arc<Spec>,
    pub sfs: Arc<SourceFilesystem>,
    pub work_fs: Arc<dyn Fs>,
}

/// Go: `scss.DecodeOptions(m)`.
// Go: resources/resource_transformers/tocss/scss/client.go:DecodeOptions
pub fn decode_options(m: Option<&Map>) -> Result<Options> {
    let mut opts = Options::default();
    let Some(m) = m else {
        return Ok(opts);
    };
    let mut d = DecodedOptions::default();
    let err = nh_config::decode::weak_decode_into(&Value::map(m.clone()), &mut d).err();
    opts.target_path = d.target_path;
    opts.include_paths = d.include_paths;
    opts.output_style = d.output_style;
    opts.precision = d.precision;
    opts.enable_source_map = d.enable_source_map;
    opts.vars = (!d.vars.entries.is_empty()).then_some(d.vars);

    if !opts.target_path.is_empty() {
        opts.target_path = nh_common::paths::path::to_slash_trim_leading(&opts.target_path);
    }

    match err {
        Some(e) => Err(e),
        None => Ok(opts),
    }
}

/// Go: `Supports()` (used in tests).
// Go: resources/resource_transformers/tocss/scss/tocss.go:Supports
pub fn supports() -> bool {
    true
}

impl ResourceTransformation for ToCssTransformation {
    fn key(&self) -> ResourceTransformationKey {
        register_options_key();
        self.transformation_key()
    }

    // Go: resources/resource_transformers/tocss/scss/tocss.go:(*toCSSTransformation).Transform
    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()> {
        ctx.out_media_type = nh_media::media::builtin::builtin().css_type.clone();

        if !self.options.from.target_path.is_empty() {
            ctx.out_path = self.options.from.target_path.clone();
        } else {
            ctx.replace_out_path_extension(".css");
        }

        let out_name = go_path::path::base(&ctx.out_path).to_string();

        let mut options = self.options.clone();
        let base_dir = go_path::path::dir(&ctx.source_path).to_string();
        options.to.include_paths = self
            .c
            .sfs
            .real_dirs(&base_dir)
            .into_iter()
            .map(String::into_bytes)
            .collect();

        // Append any workDir relative include paths
        for ip in &options.from.include_paths {
            if let Ok(info) = self.c.work_fs.stat(&go_path::filepath::clean(ip)) {
                let filename = info.meta.filename.clone();
                options.to.include_paths.push(filename.into_bytes());
            }
        }

        let vars_stylesheet =
            create_vars_style_sheet(TRANSPILER_LIB_SASS, options.from.vars.as_ref());

        // To allow for overrides of SCSS files anywhere in the project/theme hierarchy, we need
        // to help libsass revolve the filename by looking in the composite filesystem first.
        // We add the entry directories for both project and themes to the include paths list,
        // but that only work for overrides on the top level.
        let sfs = self.c.sfs.clone();
        let resolver_base_dir = base_dir.clone();
        options.to.import_resolver = Some(Arc::new(move |url: &[u8], prev: &[u8]| {
            import_resolver(&sfs, &resolver_base_dir, &vars_stylesheet, url, prev)
        }));

        if ctx.in_media_type.sub_type == nh_media::media::builtin::builtin().sass_type.sub_type {
            options.to.sass_syntax = true;
        }

        let working_dir = self.c.rs.path_spec.cfg.base_config().working_dir.clone();
        if options.from.enable_source_map {
            options.to.source_map_options.filename = format!("{out_name}.map").into_bytes();
            options.to.source_map_options.root = working_dir.clone().into_bytes();

            // Setting this to the relative input filename will get the source map
            // more correct for the main entry path (main.scss typically), but
            // it will mess up the import mappings. As a workaround, we do a replacement
            // in the source map itself (see below).
            // options.InputPath = inputPath
            options.to.source_map_options.output_path = out_name.clone().into_bytes();
            options.to.source_map_options.contents = true;
            options.to.source_map_options.omit_url = false;
            options.to.source_map_options.enable_embedded = false;
        }

        let res = match self.c.to_css_impl(options.to.clone(), ctx) {
            Ok(res) => res,
            Err(mut sasserr) => {
                if sasserr.file == "stdin" && !ctx.source_path.is_empty() {
                    sasserr.file = self.c.sfs.real_filename(&ctx.source_path);
                }
                return Err(new_file_error_from_libsass(&sasserr));
            }
        };

        if options.from.enable_source_map && !res.source_map_content.is_empty() {
            let mut source_path = self.c.sfs.real_filename(&ctx.source_path);

            let prefix = format!("{working_dir}{}", nh_helpers::path::FILE_PATH_SEPARATOR);
            if source_path.starts_with(&working_dir) {
                source_path = source_path
                    .strip_prefix(&prefix)
                    .unwrap_or(&source_path)
                    .to_string();
            }

            // This needs to be Unix-style slashes, even on Windows.
            // See https://github.com/gohugoio/hugo/issues/4968
            let source_path = go_path::filepath::to_slash(&source_path).to_string();

            // This is a workaround for what looks like a bug in Libsass. But
            // getting this resolution correct in tools like Chrome Workspaces
            // is important enough to go this extra mile.
            let map_content = String::from_utf8_lossy(&res.source_map_content).replacen(
                "stdin\"",
                &format!("{source_path}\""),
                1,
            );

            return ctx.publish_source_map(&map_content);
        }
        Ok(())
    }
}

// Go: resources/resource_transformers/tocss/scss/tocss.go:(*toCSSTransformation).Transform (ImportResolver)
fn import_resolver(
    sfs: &SourceFilesystem,
    base_dir: &str,
    vars_stylesheet: &str,
    url: &[u8],
    prev: &[u8],
) -> (Vec<u8>, Vec<u8>, bool) {
    if url == HUGO_VARS_NAMESPACE.as_bytes() {
        return (url.to_vec(), vars_stylesheet.as_bytes().to_vec(), true);
    }

    // We get URL paths from LibSASS, but we need file paths.
    let url = String::from_utf8_lossy(url).into_owned();
    let prev = String::from_utf8_lossy(prev).into_owned();
    let url = go_path::filepath::from_slash(&url).to_string();
    let prev = go_path::filepath::from_slash(&prev).to_string();

    let url_dir = go_path::filepath::dir(&url).to_string();
    let prev_dir = if prev == "stdin" {
        base_dir.to_string()
    } else {
        match sfs.make_path_relative(&go_path::filepath::dir(&prev), true) {
            Some(d) if !d.is_empty() => d,
            // Not a member of this filesystem. Let LibSASS handle it.
            _ => return (Vec::new(), Vec::new(), false),
        }
    };

    let base_path = go_path::filepath::join(&[prev_dir.as_str(), url_dir.as_str()]);
    let name = go_path::filepath::base(&url).to_string();

    // Libsass throws an error in cases where you have several possible candidates.
    // We make this simpler and pick the first match.
    let name_patterns: &[&str] = if name.contains('.') {
        &["_%s", "%s"]
    } else if name.starts_with('_') {
        &["_%s.scss", "_%s.sass"]
    } else {
        &[
            "_%s.scss",
            "%s.scss",
            "_%s.sass",
            "%s.sass",
            "%s/_index.scss",
            "%s/_index.sass",
            "%s/index.scss",
            "%s/index.sass",
        ]
    };

    let name = name.strip_prefix('_').unwrap_or(&name);

    for name_pattern in name_patterns {
        let filename_to_check =
            go_path::filepath::join(&[base_path.as_str(), &name_pattern.replacen("%s", name, 1)]);
        if let Ok(fi) = sfs.fs.stat(&filename_to_check) {
            return (fi.meta.filename.clone().into_bytes(), Vec::new(), true);
        }
    }

    // Not found, let LibSASS handle it
    (Vec::new(), Vec::new(), false)
}

/// Go: `herrors.NewFileErrorFromFileInErr(sasserr, hugofs.Os, nil)` for a libsass error: the
/// position is the error's file, line and column (line 1, column 1 and no file when the error
/// has no line); the message is the libsass message (Go's `causeString`). Reading the file
/// (`UpdateContent` with the simple line matcher) never changes the position.
fn new_file_error_from_libsass(e: &libsass_sys::libsasserrors::Error) -> Error {
    if e.line > 0 {
        return Error::new(e.message.clone()).at(FilePos {
            filename: e.file.clone(),
            line: e.line,
            column: e.column,
        });
    }
    // extractFileTypePos: no libsass position; the line number extractors run on Error().
    let full = nh_common::herrors::new_file_error(Error::new(e.to_string()));
    let (line, column) = full.pos().map(|p| (p.line, p.column)).unwrap_or((1, 1));
    Error::new(e.message.clone()).at(FilePos {
        filename: String::new(),
        line,
        column,
    })
}

impl Client {
    // Go: resources/resource_transformers/tocss/scss/tocss.go:(*Client).toCSS
    fn to_css_impl(
        &self,
        options: libsass_sys::transpiler::Options,
        ctx: &mut ResourceTransformationCtx<'_>,
    ) -> std::result::Result<libsass_sys::transpiler::SassResult, libsass_sys::libsasserrors::Error>
    {
        let transpiler = libsass_sys::transpiler::new(options)?;

        let src = nh_helpers::general::reader_to_string(Some(&mut ctx.from));
        let input = src.as_slice();

        // See https://github.com/gohugoio/hugo/issues/7059
        // We need to preserve the regular CSS imports. This is by far
        // a perfect solution, and only works for the main entry file, but
        // that should cover many use cases, e.g. using SCSS as a preprocessor
        // for Tailwind.
        let (input, imports_replaced) = replace_regular_imports_in(input);

        let res = transpiler.execute(&input)?;

        let mut out = res.css.clone();
        if imports_replaced {
            out = replace_regular_imports_out(&out);
        }

        ctx.to.extend_from_slice(&out);

        Ok(res)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/tocss/scss/tocss.go (214 lines; 2/3 funcs executed)
// OK L35-37: Supports() bool
// OK L39-181: (t *toCSSTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// OK L183-214: (c *Client) toCSS(options libsass.Options, dst io.Writer, src io.Reader) (libsass.Result, error)
// ---------------------------------------------------------------------------
