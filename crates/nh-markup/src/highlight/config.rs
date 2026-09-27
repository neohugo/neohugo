//! Port of `markup/highlight/config.go`.
//!
//! Owner: Wave B task T06 (markup).


use nh_common::Result;

/// Go: `highlight.Config` (chroma options; only decoded — no code block is highlighted by seeksnack).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub style: String,
    pub code_fences: bool,
    pub no_classes: bool,
    pub no_hl: bool,
    pub line_nos: bool,
    pub line_numbers_in_table: bool,
    pub anchor_line_nos: bool,
    pub line_anchors: String,
    pub line_no_start: i64,
    pub hl_lines: String,
    pub hl_inline: bool,
    pub tab_width: i64,
    pub guess_syntax: bool,
    pub wrapper_class: String,
}

/// Go: `highlight.ApplyLegacyConfig(cfg, conf)` (`pygmentsStyle`, `pygmentsCodeFences`,
/// `pygmentsCodefencesGuessSyntax`, `pygmentsOptions="linenos=table"`).
// Go: markup/highlight/config.go:ApplyLegacyConfig
pub fn apply_legacy_config(cfg: &dyn nh_config::config_provider::Provider, conf: &mut Config) -> Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/highlight/config.go (296 lines; 4/9 funcs executed)
//   types: Config
//    L88-121: (cfg Config) toHTMLOptions() []html.Option
//    L123-137: applyOptions(opts any, cfg *Config) error
// EX L139-145: applyOptionsFromString(opts string, cfg *Config) error
//    L147-150: applyOptionsFromMap(optsm map[string]any, cfg *Config) error
//    L152-160: applyOptionsFromCodeBlockContext(ctx hooks.CodeblockContext, cfg *Config) error
// EX L164-190: ApplyLegacyConfig(cfg config.Provider, conf *Config) error
// EX L192-213: parseHighlightOptions(in string) (map[string]any, error)
// EX L215-251: normalizeHighlightOptions(m map[string]any)
//    L254-296: hlLinesToRanges(startLine int, s string) ([][2]int, error)
// ---------------------------------------------------------------------------
