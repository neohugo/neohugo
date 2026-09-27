//! Byte-exact port of [goldmark](https://github.com/yuin/goldmark) v1.7.12,
//! a CommonMark compliant Markdown parser and HTML renderer, with Go
//! semantics (byte strings, Go's Unicode tables, Go's quirks).
//!
//! Go package → Rust module:
//! - `goldmark` (markdown.go) → crate root ([`Markdown`], [`new`], [`Extender`])
//! - `ast` → [`ast`] (arena of nodes addressed by [`ast::NodeId`])
//! - `text` → [`text`]
//! - `util` → [`util`]
//! - `parser` → [`parser`]
//! - `renderer`, `renderer/html` → [`renderer`], [`renderer::html`]
//!
//! See PORTING.md for the plugin (extension) API and the deviations.

// A faithful port keeps Go's index loops, argument lists and branch shapes.
#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::len_without_is_empty,
    clippy::manual_range_contains,
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::nonminimal_bool,
    clippy::overly_complex_bool_expr,
    clippy::type_complexity,
    clippy::new_without_default,
    clippy::module_inception,
    clippy::len_zero,
    clippy::comparison_chain,
    clippy::unnecessary_unwrap,
    clippy::needless_late_init,
    clippy::enum_variant_names,
    clippy::let_and_return
)]

pub mod ast;
pub mod extension;
mod goslice;
pub mod parser;
pub mod renderer;
pub mod text;
pub mod util;

pub use goslice::GoSlice;

/// Go `error` (renderer functions and walkers return it).
pub type Error = Box<dyn std::error::Error + Send + Sync>;

use parser::{Parser, ParserOption};
use renderer::{Renderer, RendererOption};
use text::Reader;
use util::BufWriter;

// Go: markdown.go:DefaultParser
/// DefaultParser returns a new Parser that is configured by default values.
pub fn default_parser() -> Parser {
    parser::new_parser(vec![
        parser::with_block_parsers(parser::default_block_parsers()),
        parser::with_inline_parsers(parser::default_inline_parsers()),
        parser::with_paragraph_transformers(parser::default_paragraph_transformers()),
    ])
}

// Go: markdown.go:DefaultRenderer
/// DefaultRenderer returns a new Renderer that is configured by default values.
pub fn default_renderer() -> Renderer {
    renderer::new_renderer(vec![renderer::with_node_renderers(vec![
        util::prioritized(renderer::html::new_renderer(Vec::new()), 1000),
    ])])
}

// Go: markdown.go:Convert
/// Convert interprets a UTF-8 bytes source in Markdown and
/// write rendered contents to a writer w (with a default Markdown).
pub fn convert(source: &[u8], w: &mut dyn BufWriter) -> Result<(), Error> {
    static DEFAULT_MARKDOWN: std::sync::LazyLock<Markdown> =
        std::sync::LazyLock::new(|| new(Vec::new()));
    DEFAULT_MARKDOWN.convert(source, w)
}

/// Option is a functional option type for Markdown objects.
pub type MdOption = Box<dyn FnOnce(&mut Markdown)>;

// Go: markdown.go:WithExtensions
/// WithExtensions adds extensions.
pub fn with_extensions(ext: Vec<Box<dyn Extender>>) -> MdOption {
    Box::new(move |m: &mut Markdown| m.extensions.extend(ext))
}

// Go: markdown.go:WithParser
/// WithParser allows you to override the default parser.
pub fn with_parser(p: Parser) -> MdOption {
    Box::new(move |m: &mut Markdown| m.parser = p)
}

// Go: markdown.go:WithParserOptions
/// WithParserOptions applies options for the parser.
pub fn with_parser_options(opts: Vec<Box<dyn ParserOption>>) -> MdOption {
    Box::new(move |m: &mut Markdown| m.parser.add_options(opts))
}

// Go: markdown.go:WithRenderer
/// WithRenderer allows you to override the default renderer.
pub fn with_renderer(r: Renderer) -> MdOption {
    Box::new(move |m: &mut Markdown| m.renderer = r)
}

// Go: markdown.go:WithRendererOptions
/// WithRendererOptions applies options for the renderer.
pub fn with_renderer_options(opts: Vec<Box<dyn RendererOption>>) -> MdOption {
    Box::new(move |m: &mut Markdown| m.renderer.add_options(opts))
}

/// A Markdown interface offers functions to convert Markdown text to
/// a desired format.
pub struct Markdown {
    parser: Parser,
    renderer: Renderer,
    extensions: Vec<Box<dyn Extender>>,
}

// Go: markdown.go:New
/// New returns a new Markdown with given options.
pub fn new(options: Vec<MdOption>) -> Markdown {
    let mut md = Markdown {
        parser: default_parser(),
        renderer: default_renderer(),
        extensions: Vec::new(),
    };
    for opt in options {
        opt(&mut md);
    }
    let extensions = std::mem::take(&mut md.extensions);
    for e in &extensions {
        e.extend(&mut md);
    }
    md.extensions = extensions;
    md
}

impl Markdown {
    // Go: markdown.go:markdown.Convert
    /// Convert interprets a UTF-8 bytes source in Markdown and write rendered
    /// contents to a writer w.
    pub fn convert(&self, source: &[u8], writer: &mut dyn BufWriter) -> Result<(), Error> {
        let mut reader = text::new_reader(source);
        let doc = self.parser.parse(&mut reader);
        self.renderer.render(writer, source, &doc.ast, doc.root)
    }

    /// Convert with a caller-provided parser context (Go:
    /// `Convert(source, w, parser.WithContext(pc))`).
    pub fn convert_with_context(
        &self,
        source: &[u8],
        writer: &mut dyn BufWriter,
        pc: &mut parser::Context,
    ) -> Result<(), Error> {
        let mut reader = text::new_reader(source);
        let doc = self.parser.parse_with_context(&mut reader, pc);
        self.renderer.render(writer, source, &doc.ast, doc.root)
    }

    /// Parse only (Go: `m.Parser().Parse(text.NewReader(source), ...)`).
    pub fn parse(&self, source: &[u8], pc: &mut parser::Context) -> parser::ParseResult {
        let mut reader = text::new_reader(source);
        self.parser
            .parse_with_context(&mut reader as &mut dyn Reader, pc)
    }

    // Go: markdown.go:markdown.Parser
    /// Parser returns a Parser that will be used for conversion.
    pub fn parser(&self) -> &Parser {
        &self.parser
    }

    // Go: markdown.go:markdown.SetParser
    /// SetParser sets a Parser to this object.
    pub fn set_parser(&mut self, v: Parser) {
        self.parser = v;
    }

    // Go: markdown.go:markdown.Renderer
    /// Parser returns a Renderer that will be used for conversion.
    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    // Go: markdown.go:markdown.SetRenderer
    /// SetRenderer sets a Renderer to this object.
    pub fn set_renderer(&mut self, v: Renderer) {
        self.renderer = v;
    }
}

/// An Extender interface is used for extending Markdown.
pub trait Extender: Send + Sync {
    /// Extend extends the Markdown.
    fn extend(&self, m: &mut Markdown);
}
