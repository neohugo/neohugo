//! Port of `markup/markup_config/config.go`.
//!
//! Owner: Wave B task T06 (markup).

use std::sync::Arc;

use go_value::{Map, Value};
use nh_common::Result;
use nh_config::decode::FieldRef;
use nh_config::decode_struct;

use crate::asciidocext::Config as AsciidocExtConfig;
use crate::goldmark::goldmark_config::{
    self, Config as GoldmarkConfig, ParserAttribute, Typographer,
};
use crate::highlight::config::{self as highlight_config, Config as HighlightConfig};
use crate::tableofcontents::Config as TocConfig;

/// Go: `markup_config.Config`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    /// Default markdown handler for md/markdown extensions ("goldmark").
    pub default_markdown_handler: String,
    pub highlight: HighlightConfig,
    pub table_of_contents: TocConfig,
    pub goldmark: GoldmarkConfig,
    /// AsciiDoc options are decoded but the converter is a stub.
    pub asciidoc_ext: AsciidocExtConfig,
}

decode_struct!(Config, "markup_config.Config", |s| vec![
    FieldRef::new("DefaultMarkdownHandler", &mut s.default_markdown_handler),
    FieldRef::new("Highlight", &mut s.highlight),
    FieldRef::new("TableOfContents", &mut s.table_of_contents),
    FieldRef::new("Goldmark", &mut s.goldmark),
    FieldRef::new("AsciidocExt", &mut s.asciidoc_ext),
]);

impl Config {
    // Go: markup/markup_config/config.go:Init
    pub fn init(&mut self) -> Result<()> {
        self.goldmark.init()
    }
}

/// Go: `markup_config.Default`.
pub fn default_config() -> Config {
    Config {
        default_markdown_handler: "goldmark".into(),
        table_of_contents: TocConfig::default(),
        highlight: highlight_config::default_config(),
        goldmark: goldmark_config::default_config(),
        asciidoc_ext: crate::asciidocext::default_config(),
    }
}

/// Go: `markup_config.Decode(cfg)` + `normalizeConfig` (a bool `typographer = true` is DELETED so
/// the default substitutions apply; legacy keys handled).
// Go: markup/markup_config/config.go:Decode
pub fn decode(cfg: &dyn nh_config::config_provider::Provider) -> Result<Config> {
    let mut conf = default_config();

    // Go: `m := cfg.GetStringMap("markup"); if m == nil { return conf, err }`. `GetStringMap` is
    // `maps.ToStringMap`, which is nil when the conversion fails (a missing key, a non-map
    // value that is not a JSON object string) and for a nil `maps.Params`. nh-config's
    // `get_string_map` returns an empty map in those cases, so the conversion is done here.
    // The early return also skips `ApplyLegacyConfig` (the legacy pygments keys are ignored
    // without a `[markup]` section).
    let v = cfg.get("markup");
    if matches!(&v, Value::TypedNil(t) if &**t == "maps.Params") {
        return Ok(conf);
    }
    let Ok(m) = nh_common::maps::maps::to_string_map_e(&v) else {
        return Ok(conf);
    };
    let mut m = nh_common::maps::params::clean_config_string_map(&m);

    normalize_config(&mut m);

    nh_config::decode::weak_decode_into(&Value::map(m), &mut conf)?;

    conf.init()?;

    highlight_config::apply_legacy_config(cfg, &mut conf.highlight)?;

    Ok(conf)
}

/// Decodes and wraps the config as `GetConfigSection("markup")` returns it.
pub fn decode_arc(cfg: &dyn nh_config::config_provider::Provider) -> Result<Arc<Config>> {
    decode(cfg).map(Arc::new)
}

// Go: markup/markup_config/config.go:normalizeConfig
fn normalize_config(m: &mut Map) {
    if let Some(vm) = nested_string_map_mut(m, "goldmark.parser") {
        // Changed from a bool in 0.81.0
        if let Some(Value::Bool(vvb)) = vm.get(b"attribute").cloned() {
            vm.insert(
                "attribute",
                Value::object(ParserAttribute {
                    title: vvb,
                    block: false,
                }),
            );
        }
    }

    // Changed from a bool in 0.112.0.
    if let Some(vm) = nested_string_map_mut(m, "goldmark.extensions") {
        const TYPOGRAPHER_KEY: &str = "typographer";
        if let Some(Value::Bool(vvb)) = vm.get(TYPOGRAPHER_KEY.as_bytes()).cloned() {
            if !vvb {
                vm.insert(
                    TYPOGRAPHER_KEY,
                    Value::object(Typographer {
                        disable: true,
                        ..Default::default()
                    }),
                );
            } else {
                vm.entries.remove(TYPOGRAPHER_KEY.as_bytes());
            }
        }
    }
}

/// `maps.ToStringMap(maps.GetNestedParam(key, ".", m))` for mutation in place: Go gets the same
/// map back (a `maps.Params` or `map[string]any` shares its storage with `m`), so the changes of
/// `normalizeConfig` are visible in `m`. Any other value converts to a fresh map whose changes
/// are lost, which is `None` here.
fn nested_string_map_mut<'a>(m: &'a mut Map, key: &str) -> Option<&'a mut Map> {
    let key = key.to_lowercase();
    // Try exact match first.
    if m.get(key.as_bytes()).is_some() {
        return as_string_any_map_mut(m.entries.get_mut(key.as_bytes())?);
    }
    let segments: Vec<&str> = key.split('.').collect();
    let mut cur: &'a mut Map = m;
    for (i, seg) in segments.iter().enumerate() {
        // Go's getNested lower-cases each segment (they already are).
        let v = cur.entries.get_mut(seg.as_bytes())?;
        if i == segments.len() - 1 {
            return as_string_any_map_mut(v);
        }
        match v {
            Value::Map(mm)
                if matches!(
                    mm.ty,
                    go_value::MapType::Params | go_value::MapType::StringAny
                ) =>
            {
                cur = Arc::make_mut(mm);
            }
            _ => return None,
        }
    }
    None
}

fn as_string_any_map_mut(v: &mut Value) -> Option<&mut Map> {
    match v {
        Value::Map(mm)
            if matches!(
                mm.ty,
                go_value::MapType::Params | go_value::MapType::StringAny
            ) =>
        {
            Some(Arc::make_mut(mm))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/markup_config/config.go (117 lines; 3/3 funcs executed)
//   types: Config
// OK L45-47: (c *Config) Init() error
// OK L49-74: Decode(cfg config.Provider) (conf Config, err error)
// OK L76-107: normalizeConfig(m map[string]any)
// ---------------------------------------------------------------------------
