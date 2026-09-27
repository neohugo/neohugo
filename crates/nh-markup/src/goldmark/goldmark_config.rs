//! Port of `markup/goldmark/goldmark_config/config.go`.
//!
//! Owner: Wave B task T06 (markup).


//! Go `goldmark_config.Config` (defaults: typographer substitutions `&lsquo;`..., linkify protocol
//! `https`, autoHeadingID github, wrapStandAloneImageWithinParagraph true, renderHooks
//! useEmbedded "auto" -> "fallback" for multilingual single-host sites (set by allconfig)).

pub const AUTO_ID_TYPE_BLACKFRIDAY: &str = "blackfriday";
pub const AUTO_ID_TYPE_GITHUB: &str = "github";
pub const AUTO_ID_TYPE_GITHUB_ASCII: &str = "github-ascii";
pub const RENDER_HOOK_USE_EMBEDDED_ALWAYS: &str = "always";
pub const RENDER_HOOK_USE_EMBEDDED_AUTO: &str = "auto";
pub const RENDER_HOOK_USE_EMBEDDED_FALLBACK: &str = "fallback";
pub const RENDER_HOOK_USE_EMBEDDED_NEVER: &str = "never";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub renderer: Renderer,
    pub parser: Parser,
    pub extensions: Extensions,
    pub duplicate_resource_files: bool,
    pub render_hooks: RenderHooks,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderHooks {
    pub image: RenderHook,
    pub link: RenderHook,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderHook {
    pub enable_default: Option<bool>,
    pub use_embedded: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extensions {
    pub typographer: Typographer,
    pub footnote: bool,
    pub definition_list: bool,
    pub extras: Extras,
    pub passthrough: Passthrough,
    pub table: bool,
    pub strikethrough: bool,
    pub linkify: bool,
    pub linkify_protocol: String,
    pub task_list: bool,
    pub cjk: Cjk,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Typographer {
    pub disable: bool,
    pub left_single_quote: String,
    pub right_single_quote: String,
    pub left_double_quote: String,
    pub right_double_quote: String,
    pub en_dash: String,
    pub em_dash: String,
    pub ellipsis: String,
    pub left_angle_quote: String,
    pub right_angle_quote: String,
    pub apostrophe: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extras {
    pub delete: bool,
    pub insert: bool,
    pub mark: bool,
    pub subscript: bool,
    pub superscript: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Passthrough {
    pub enable: bool,
    pub delimiters_inline: Vec<Vec<String>>,
    pub delimiters_block: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cjk {
    pub enable: bool,
    pub east_asian_line_breaks: bool,
    pub east_asian_line_breaks_style: String,
    pub escaped_space: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Renderer {
    pub hard_wraps: bool,
    pub xhtml: bool,
    pub unsafe_: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Parser {
    pub auto_heading_id: bool,
    pub auto_definition_term_id: bool,
    pub auto_id_type: String,
    pub attribute_title: bool,
    pub attribute_block: bool,
    pub wrap_stand_alone_image_within_paragraph: bool,
}

/// Go: `goldmark_config.Default`.
pub fn default_config() -> Config {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/goldmark_config/config.go (309 lines; 2/2 funcs executed)
//   types: Config, RenderHooks, ImageRenderHook, LinkRenderHook, Extensions, Typographer, Extras, Delete, Insert,
//          Mark, Subscript, Superscript, Passthrough, DelimitersConfig, CJK, Renderer, Parser, ParserAttribute
// EX L113-121: (c *Config) Init() error
// EX L296-302: (p *Parser) Init() error
// ---------------------------------------------------------------------------
