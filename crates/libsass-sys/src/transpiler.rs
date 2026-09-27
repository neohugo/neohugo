//! Port of `github.com/bep/golibsass@v1.2.0/libsass/transpiler.go`.

use crate::internal::{self, ImportResolver};
use crate::libsasserrors::{self, Error};

/// Go: `type OutputStyle int`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct OutputStyle(pub i64);

impl OutputStyle {
    pub const NESTED: OutputStyle = OutputStyle(0);
    pub const EXPANDED: OutputStyle = OutputStyle(1);
    pub const COMPACT: OutputStyle = OutputStyle(2);
    pub const COMPRESSED: OutputStyle = OutputStyle(3);
}

/// Go: `NestedStyle`, `ExpandedStyle`, `CompactStyle`, `CompressedStyle`.
pub const NESTED_STYLE: OutputStyle = OutputStyle::NESTED;
pub const EXPANDED_STYLE: OutputStyle = OutputStyle::EXPANDED;
pub const COMPACT_STYLE: OutputStyle = OutputStyle::COMPACT;
pub const COMPRESSED_STYLE: OutputStyle = OutputStyle::COMPRESSED;

// Go: transpiler.go:ParseOutputStyle
/// ParseOutputStyle will convert s into OutputStyle.
/// Case insensitive, returns NestedStyle for unknown values.
///
/// Go uses `strings.ToLower`; no non-ASCII rune lowers to one of the ASCII
/// letters in the four style names, so ASCII case folding is equivalent.
pub fn parse_output_style(s: &[u8]) -> OutputStyle {
    let eq = |name: &[u8]| s.eq_ignore_ascii_case(name);
    if eq(b"nested") {
        return NESTED_STYLE;
    }
    if eq(b"expanded") {
        return EXPANDED_STYLE;
    }
    if eq(b"compact") {
        return COMPACT_STYLE;
    }
    if eq(b"compressed") {
        return COMPRESSED_STYLE;
    }
    NESTED_STYLE
}

/// Go: `libsass.Options`.
#[derive(Clone, Default)]
pub struct Options {
    /// Default is nested.
    pub output_style: OutputStyle,

    /// Precision of floating point math. (Go `int`; 0 keeps LibSass's
    /// default.)
    pub precision: i64,

    /// File paths to use to resolve imports.
    pub include_paths: Vec<Vec<u8>>,

    /// ImportResolver can be used to supply a custom import resolver, both to
    /// redirect to another URL or to return the body.
    /// `(url, prev) -> (new_url, body, resolved)`.
    pub import_resolver: Option<ImportResolver>,

    /// Used to indicate "old style" SASS for the input stream.
    pub sass_syntax: bool,

    pub source_map_options: SourceMapOptions,
}

impl std::fmt::Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("output_style", &self.output_style)
            .field("precision", &self.precision)
            .field("include_paths", &self.include_paths)
            .field(
                "import_resolver",
                &self.import_resolver.as_ref().map(|_| "func"),
            )
            .field("sass_syntax", &self.sass_syntax)
            .field("source_map_options", &self.source_map_options)
            .finish()
    }
}

/// Go: `libsass.SourceMapOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SourceMapOptions {
    pub filename: Vec<u8>,
    pub root: Vec<u8>,
    pub input_path: Vec<u8>,
    pub output_path: Vec<u8>,
    pub contents: bool,
    pub omit_url: bool,
    pub enable_embedded: bool,
}

/// Go: `libsass.Result`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SassResult {
    pub css: Vec<u8>,

    /// If source maps are configured.
    pub source_map_filename: Vec<u8>,
    pub source_map_content: Vec<u8>,
}

/// Go: `libsass.Transpiler`.
pub trait Transpiler {
    fn execute(&self, src: &[u8]) -> Result<SassResult, Error>;
}

/// Go: `libsassTranspiler`.
#[derive(Clone, Debug)]
pub struct LibsassTranspiler {
    options: Options,
}

// Go: transpiler.go:New
/// New creates a new libsass transpiler configured with the given options.
/// (Go returns `(Transpiler, error)`; the error is always nil.)
pub fn new(options: Options) -> Result<LibsassTranspiler, Error> {
    Ok(LibsassTranspiler { options })
}

/// Go `strings.Join(t.options.IncludePaths, string(os.PathListSeparator))`.
fn join_include_paths(paths: &[Vec<u8>]) -> Vec<u8> {
    let sep: &[u8] = if cfg!(windows) { b";" } else { b":" };
    paths.join(sep)
}

/// Deletes the import resolver registration on drop (Go: `defer
/// libsass.DeleteImportResolver(idx)`).
struct ResolverGuard(i64);

impl Drop for ResolverGuard {
    fn drop(&mut self) {
        internal::delete_import_resolver(self.0);
    }
}

/// Deletes the compiler and the data context on drop (Go: `defer
/// libsass.SassDeleteCompiler(compiler)`; Go never deletes the data context
/// and instead frees the output string it reads).
struct ContextGuard {
    data_ctx: *mut crate::ffi::Sass_Data_Context,
    compiler: *mut crate::ffi::Sass_Compiler,
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.compiler.is_null() {
                internal::sass_delete_compiler(self.compiler);
            }
            internal::sass_delete_data_context(self.data_ctx);
        }
    }
}

impl Transpiler for LibsassTranspiler {
    // Go: transpiler.go:(libsassTranspiler).Execute
    /// Execute transpiles the SCSS or SASS from src into dst.
    fn execute(&self, src: &[u8]) -> Result<SassResult, Error> {
        let mut result = SassResult::default();

        let converted;
        let mut src = src;
        if self.options.sass_syntax {
            // LibSass does not support this directly, so have to handle the main SASS content
            // special.
            converted = internal::sass_to_scss(src);
            src = &converted;
        }

        let data_ctx = internal::sass_make_data_context(src);
        let mut guard = ContextGuard {
            data_ctx,
            compiler: std::ptr::null_mut(),
        };

        unsafe {
            let opts = internal::sass_data_context_get_options(data_ctx);
            let _resolver_guard;
            {
                // Set options

                if let Some(resolver) = &self.options.import_resolver {
                    let idx = internal::add_import_resolver(opts, resolver.clone());
                    _resolver_guard = ResolverGuard(idx);
                }

                if self.options.precision != 0 {
                    internal::sass_option_set_precision(opts, self.options.precision);
                }

                let sm = &self.options.source_map_options;
                if !sm.filename.is_empty() {
                    internal::sass_option_set_source_map_file(opts, &sm.filename);
                }

                if !sm.root.is_empty() {
                    internal::sass_option_set_source_map_root(opts, &sm.root);
                }

                if !sm.output_path.is_empty() {
                    internal::sass_option_set_output_path(opts, &sm.output_path);
                }
                if !sm.input_path.is_empty() {
                    internal::sass_option_set_input_path(opts, &sm.input_path);
                }

                internal::sass_option_set_source_map_contents(opts, sm.contents);
                internal::sass_option_set_omit_source_map_url(opts, sm.omit_url);
                internal::sass_option_set_source_map_embed(opts, sm.enable_embedded);
                internal::sass_option_set_include_path(
                    opts,
                    &join_include_paths(&self.options.include_paths),
                );
                internal::sass_option_set_output_style(opts, self.options.output_style.0);
                internal::sass_option_set_source_comments(opts, false);
                internal::sass_data_context_set_options(data_ctx, opts);
            }

            let ctx = internal::sass_data_context_get_context(data_ctx);
            let compiler = internal::sass_make_data_compiler(data_ctx);
            guard.compiler = compiler;

            internal::sass_compiler_parse(compiler);
            internal::sass_compiler_execute(compiler);

            if let Some(payload) = internal::take_resolver_panic() {
                drop(guard);
                std::panic::resume_unwind(payload);
            }

            let status = internal::sass_context_get_error_status(ctx);
            if status != 0 {
                Err(libsasserrors::json_to_error(
                    &internal::sass_context_get_error_json(ctx),
                ))
            } else {
                result.css = internal::sass_context_get_output_string(ctx);
                result.source_map_filename = internal::sass_option_get_source_map_file(opts);
                result.source_map_content = internal::sass_context_get_source_map_string(ctx);
                Ok(result)
            }
        }
    }
}
