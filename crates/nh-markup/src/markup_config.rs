//! Port of `markup/markup_config/config.go`.
//!
//! Owner: Wave B task T06 (markup).


use go_value::Map;
use nh_common::Result;

use crate::goldmark::goldmark_config::Config as GoldmarkConfig;
use crate::highlight::config::Config as HighlightConfig;
use crate::tableofcontents::Config as TocConfig;

/// Go: `markup_config.Config`.
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Default markdown handler for md/markdown extensions ("goldmark").
    pub default_markdown_handler: String,
    pub highlight: HighlightConfig,
    pub table_of_contents: TocConfig,
    pub goldmark: GoldmarkConfig,
    /// AsciiDoc options are decoded but the converter is a stub.
    pub asciidoc_ext: Option<Map>,
}

/// Go: `markup_config.Decode(cfg)` + `normalizeConfig` (a bool `typographer = true` is DELETED so
/// the default substitutions apply; legacy keys handled).
// Go: markup/markup_config/config.go:Decode
pub fn decode(cfg: &dyn nh_config::config_provider::Provider) -> Result<Config> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/markup_config/config.go (117 lines; 3/3 funcs executed)
//   types: Config
// EX L45-47: (c *Config) Init() error
// EX L49-74: Decode(cfg config.Provider) (conf Config, err error)
// EX L76-107: normalizeConfig(m map[string]any)
// ---------------------------------------------------------------------------
