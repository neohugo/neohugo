//! Port of `markup/goldmark/goldmark_config/config.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `goldmark_config.Config` (defaults: typographer substitutions `&lsquo;`..., linkify protocol
//! `https`, autoHeadingID github, wrapStandAloneImageWithinParagraph true, renderHooks
//! useEmbedded "auto" -> "fallback" for multilingual single-host sites (set by allconfig)).
//!
//! The structs mirror Go's field layout so that `mapstructure.WeakDecode` (nh-config's
//! `decode`) sees the same field names.

use go_value::{Object, Value};
use nh_config::decode::FieldRef;
use nh_config::decode_struct;

pub const AUTO_ID_TYPE_BLACKFRIDAY: &str = "blackfriday";
pub const AUTO_ID_TYPE_GITHUB: &str = "github";
pub const AUTO_ID_TYPE_GITHUB_ASCII: &str = "github-ascii";
pub const RENDER_HOOK_USE_EMBEDDED_ALWAYS: &str = "always";
pub const RENDER_HOOK_USE_EMBEDDED_AUTO: &str = "auto";
pub const RENDER_HOOK_USE_EMBEDDED_FALLBACK: &str = "fallback";
pub const RENDER_HOOK_USE_EMBEDDED_NEVER: &str = "never";

/// Go: `goldmark_config.Config`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub renderer: Renderer,
    pub parser: Parser,
    pub extensions: Extensions,
    pub duplicate_resource_files: bool,
    pub render_hooks: RenderHooks,
}

decode_struct!(Config, "goldmark_config.Config", |s| vec![
    FieldRef::new("Renderer", &mut s.renderer),
    FieldRef::new("Parser", &mut s.parser),
    FieldRef::new("Extensions", &mut s.extensions),
    FieldRef::new("DuplicateResourceFiles", &mut s.duplicate_resource_files),
    FieldRef::new("RenderHooks", &mut s.render_hooks),
]);

impl Config {
    // Go: markup/goldmark/goldmark_config/config.go:Init
    pub fn init(&mut self) -> nh_common::Result<()> {
        self.parser.init()?;
        if self.parser.auto_definition_term_id && !self.extensions.definition_list {
            self.parser.auto_definition_term_id = false;
        }
        Ok(())
    }
}

/// Go: `goldmark_config.RenderHooks`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderHooks {
    pub image: RenderHook,
    pub link: RenderHook,
}

decode_struct!(RenderHooks, "goldmark_config.RenderHooks", |s| vec![
    FieldRef::new("Image", &mut s.image),
    FieldRef::new("Link", &mut s.link),
]);

/// Go: `goldmark_config.ImageRenderHook` / `LinkRenderHook` (same fields).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderHook {
    /// Deprecated in Go; a `*bool` (`None` = not set).
    pub enable_default: Option<Box<bool>>,
    pub use_embedded: String,
}

decode_struct!(RenderHook, "goldmark_config.ImageRenderHook", |s| vec![
    FieldRef::new("EnableDefault", &mut s.enable_default),
    FieldRef::new("UseEmbedded", &mut s.use_embedded),
]);

/// Go: `goldmark_config.Extensions`.
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

decode_struct!(Extensions, "goldmark_config.Extensions", |s| vec![
    FieldRef::new("Typographer", &mut s.typographer),
    FieldRef::new("Footnote", &mut s.footnote),
    FieldRef::new("DefinitionList", &mut s.definition_list),
    FieldRef::new("Extras", &mut s.extras),
    FieldRef::new("Passthrough", &mut s.passthrough),
    FieldRef::new("Table", &mut s.table),
    FieldRef::new("Strikethrough", &mut s.strikethrough),
    FieldRef::new("Linkify", &mut s.linkify),
    FieldRef::new("LinkifyProtocol", &mut s.linkify_protocol),
    FieldRef::new("TaskList", &mut s.task_list),
    FieldRef::new("CJK", &mut s.cjk),
]);

/// Go: `goldmark_config.Typographer`.
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

decode_struct!(Typographer, "goldmark_config.Typographer", |s| vec![
    FieldRef::new("Disable", &mut s.disable),
    FieldRef::new("LeftSingleQuote", &mut s.left_single_quote),
    FieldRef::new("RightSingleQuote", &mut s.right_single_quote),
    FieldRef::new("LeftDoubleQuote", &mut s.left_double_quote),
    FieldRef::new("RightDoubleQuote", &mut s.right_double_quote),
    FieldRef::new("EnDash", &mut s.en_dash),
    FieldRef::new("EmDash", &mut s.em_dash),
    FieldRef::new("Ellipsis", &mut s.ellipsis),
    FieldRef::new("LeftAngleQuote", &mut s.left_angle_quote),
    FieldRef::new("RightAngleQuote", &mut s.right_angle_quote),
    FieldRef::new("Apostrophe", &mut s.apostrophe),
]);

nh_common::go_methods!(Typographer {});

/// `markup_config.normalizeConfig` stores a `goldmark_config.Typographer` value in the config
/// map (`typographer = false`); mapstructure then assigns it as it is.
impl Object for Typographer {
    nh_common::object_basics!("goldmark_config.Typographer");
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
}

/// Go: `goldmark_config.Extras`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extras {
    pub delete: Enable,
    pub insert: Enable,
    pub mark: Enable,
    pub subscript: Enable,
    pub superscript: Enable,
}

decode_struct!(Extras, "goldmark_config.Extras", |s| vec![
    FieldRef::new("Delete", &mut s.delete),
    FieldRef::new("Insert", &mut s.insert),
    FieldRef::new("Mark", &mut s.mark),
    FieldRef::new("Subscript", &mut s.subscript),
    FieldRef::new("Superscript", &mut s.superscript),
]);

/// Go: `goldmark_config.Delete` / `Insert` / `Mark` / `Subscript` / `Superscript` (all
/// `struct{ Enable bool }`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Enable {
    pub enable: bool,
}

decode_struct!(Enable, "goldmark_config.Delete", |s| vec![FieldRef::new(
    "Enable",
    &mut s.enable
)]);

/// Go: `goldmark_config.Passthrough`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Passthrough {
    pub enable: bool,
    pub delimiters: DelimitersConfig,
}

decode_struct!(Passthrough, "goldmark_config.Passthrough", |s| vec![
    FieldRef::new("Enable", &mut s.enable),
    FieldRef::new("Delimiters", &mut s.delimiters),
]);

/// Go: `goldmark_config.DelimitersConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DelimitersConfig {
    pub inline: Vec<Vec<String>>,
    pub block: Vec<Vec<String>>,
}

decode_struct!(
    DelimitersConfig,
    "goldmark_config.DelimitersConfig",
    |s| vec![
        FieldRef::new("Inline", &mut s.inline),
        FieldRef::new("Block", &mut s.block),
    ]
);

/// Go: `goldmark_config.CJK`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cjk {
    pub enable: bool,
    pub east_asian_line_breaks: bool,
    pub east_asian_line_breaks_style: String,
    pub escaped_space: bool,
}

decode_struct!(Cjk, "goldmark_config.CJK", |s| vec![
    FieldRef::new("Enable", &mut s.enable),
    FieldRef::new("EastAsianLineBreaks", &mut s.east_asian_line_breaks),
    FieldRef::new(
        "EastAsianLineBreaksStyle",
        &mut s.east_asian_line_breaks_style
    ),
    FieldRef::new("EscapedSpace", &mut s.escaped_space),
]);

/// Go: `goldmark_config.Renderer`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Renderer {
    pub hard_wraps: bool,
    pub xhtml: bool,
    pub unsafe_: bool,
}

decode_struct!(Renderer, "goldmark_config.Renderer", |s| vec![
    FieldRef::new("HardWraps", &mut s.hard_wraps),
    FieldRef::new("XHTML", &mut s.xhtml),
    FieldRef::new("Unsafe", &mut s.unsafe_),
]);

/// Go: `goldmark_config.Parser`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Parser {
    pub auto_heading_id: bool,
    pub auto_definition_term_id: bool,
    pub auto_id_type: String,
    pub attribute: ParserAttribute,
    pub wrap_stand_alone_image_within_paragraph: bool,
    /// Renamed to `AutoIDType` in 0.144.0.
    pub auto_heading_id_type: String,
}

decode_struct!(Parser, "goldmark_config.Parser", |s| vec![
    FieldRef::new("AutoHeadingID", &mut s.auto_heading_id),
    FieldRef::new("AutoDefinitionTermID", &mut s.auto_definition_term_id),
    FieldRef::new("AutoIDType", &mut s.auto_id_type),
    FieldRef::new("Attribute", &mut s.attribute),
    FieldRef::new(
        "WrapStandAloneImageWithinParagraph",
        &mut s.wrap_stand_alone_image_within_paragraph
    ),
    FieldRef::new("AutoHeadingIDType", &mut s.auto_heading_id_type),
]);

impl Parser {
    // Go: markup/goldmark/goldmark_config/config.go:Init
    pub fn init(&mut self) -> nh_common::Result<()> {
        // Renamed from AutoHeadingIDType to AutoIDType in 0.144.0.
        if !self.auto_heading_id_type.is_empty() {
            self.auto_id_type = self.auto_heading_id_type.clone();
        }
        Ok(())
    }
}

/// Go: `goldmark_config.ParserAttribute`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParserAttribute {
    /// Enables custom attributes for titles.
    pub title: bool,
    /// Enables custom attributes for blocks.
    pub block: bool,
}

decode_struct!(
    ParserAttribute,
    "goldmark_config.ParserAttribute",
    |s| vec![
        FieldRef::new("Title", &mut s.title),
        FieldRef::new("Block", &mut s.block),
    ]
);

nh_common::go_methods!(ParserAttribute {});

/// `markup_config.normalizeConfig` replaces a bool `attribute` with a
/// `goldmark_config.ParserAttribute` value in the config map.
impl Object for ParserAttribute {
    nh_common::object_basics!("goldmark_config.ParserAttribute");
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
}

impl ParserAttribute {
    /// The value stored in the config map by `normalizeConfig`.
    pub fn value(self) -> Value {
        Value::object(self)
    }
}

/// Go: `goldmark_config.Default`.
pub fn default_config() -> Config {
    Config {
        extensions: Extensions {
            typographer: Typographer {
                disable: false,
                left_single_quote: "&lsquo;".into(),
                right_single_quote: "&rsquo;".into(),
                left_double_quote: "&ldquo;".into(),
                right_double_quote: "&rdquo;".into(),
                en_dash: "&ndash;".into(),
                em_dash: "&mdash;".into(),
                ellipsis: "&hellip;".into(),
                left_angle_quote: "&laquo;".into(),
                right_angle_quote: "&raquo;".into(),
                apostrophe: "&rsquo;".into(),
            },
            footnote: true,
            definition_list: true,
            table: true,
            strikethrough: true,
            linkify: true,
            linkify_protocol: "https".into(),
            task_list: true,
            cjk: Cjk {
                enable: false,
                east_asian_line_breaks: false,
                east_asian_line_breaks_style: "simple".into(),
                escaped_space: false,
            },
            extras: Extras::default(),
            passthrough: Passthrough {
                enable: false,
                delimiters: DelimitersConfig {
                    inline: Vec::new(),
                    block: Vec::new(),
                },
            },
        },
        renderer: Renderer {
            unsafe_: false,
            ..Default::default()
        },
        parser: Parser {
            auto_heading_id: true,
            auto_definition_term_id: false,
            auto_id_type: AUTO_ID_TYPE_GITHUB.into(),
            wrap_stand_alone_image_within_paragraph: true,
            attribute: ParserAttribute {
                title: true,
                block: false,
            },
            auto_heading_id_type: String::new(),
        },
        duplicate_resource_files: false,
        render_hooks: RenderHooks {
            image: RenderHook {
                enable_default: None,
                use_embedded: RENDER_HOOK_USE_EMBEDDED_AUTO.into(),
            },
            link: RenderHook {
                enable_default: None,
                use_embedded: RENDER_HOOK_USE_EMBEDDED_AUTO.into(),
            },
        },
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/goldmark_config/config.go (309 lines; 2/2 funcs executed)
//   types: Config, RenderHooks, ImageRenderHook, LinkRenderHook, Extensions, Typographer, Extras, Delete, Insert,
//          Mark, Subscript, Superscript, Passthrough, DelimitersConfig, CJK, Renderer, Parser, ParserAttribute
// OK L113-121: (c *Config) Init() error
// OK L296-302: (p *Parser) Init() error
// ---------------------------------------------------------------------------
