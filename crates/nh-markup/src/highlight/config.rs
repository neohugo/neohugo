//! Port of `markup/highlight/config.go`.
//!
//! Owner: Wave B task T06 (markup).

use std::collections::BTreeMap;

use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;
use nh_config::decode_struct;

const LINEANCHORS_KEY: &str = "lineanchors";
const LINE_NOS_KEY: &str = "linenos";
const HL_LINES_KEY: &str = "hl_lines";
const LINOS_START_KEY: &str = "linenostart";

/// Go: `highlight.Config` (chroma options; only decoded — no code block is highlighted by
/// seeksnack).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub style: String,
    pub code_fences: bool,
    pub wrapper_class: String,
    pub no_classes: bool,
    pub line_nos: bool,
    pub line_numbers_in_table: bool,
    pub anchor_line_nos: bool,
    pub line_anchors: String,
    pub line_no_start: i64,
    /// Go `Hl_Lines`.
    pub hl_lines: String,
    /// Go `Hl_inline`.
    pub hl_inline: bool,
    /// Go `HL_lines_parsed` (Go's nil and empty slices are both empty).
    pub hl_lines_parsed: Vec<[i64; 2]>,
    pub tab_width: i64,
    pub guess_syntax: bool,
}

decode_struct!(Config, "highlight.Config", |s| vec![
    FieldRef::new("Style", &mut s.style),
    FieldRef::new("CodeFences", &mut s.code_fences),
    FieldRef::new("WrapperClass", &mut s.wrapper_class),
    FieldRef::new("NoClasses", &mut s.no_classes),
    FieldRef::new("LineNos", &mut s.line_nos),
    FieldRef::new("LineNumbersInTable", &mut s.line_numbers_in_table),
    FieldRef::new("AnchorLineNos", &mut s.anchor_line_nos),
    FieldRef::new("LineAnchors", &mut s.line_anchors),
    FieldRef::new("LineNoStart", &mut s.line_no_start),
    FieldRef::new("Hl_Lines", &mut s.hl_lines),
    FieldRef::new("Hl_inline", &mut s.hl_inline),
    FieldRef::new("HL_lines_parsed", &mut s.hl_lines_parsed),
    FieldRef::new("TabWidth", &mut s.tab_width),
    FieldRef::new("GuessSyntax", &mut s.guess_syntax),
]);

/// Go: `highlight.DefaultConfig`.
pub fn default_config() -> Config {
    Config {
        style: "monokai".into(),
        line_no_start: 1,
        code_fences: true,
        no_classes: true,
        line_numbers_in_table: true,
        tab_width: 4,
        wrapper_class: "highlight".into(),
        ..Default::default()
    }
}

// Go: markup/highlight/config.go:applyOptionsFromString
fn apply_options_from_string(opts: &str, cfg: &mut Config) -> Result<()> {
    let optsm = parse_highlight_options(opts)?;
    nh_config::decode::weak_decode_into(&Value::map(optsm), cfg)
}

/// Go: `highlight.ApplyLegacyConfig(cfg, conf)` (`pygmentsStyle`, `pygmentsCodeFences`,
/// `pygmentsCodefencesGuessSyntax`, `pygmentsOptions="linenos=table"`).
// Go: markup/highlight/config.go:ApplyLegacyConfig
pub fn apply_legacy_config(
    cfg: &dyn nh_config::config_provider::Provider,
    conf: &mut Config,
) -> Result<()> {
    let def = default_config();
    if conf.style == def.style {
        let s = cfg.get_string("pygmentsStyle");
        if !s.is_empty() {
            conf.style = s;
        }
    }

    if conf.no_classes == def.no_classes && cfg.is_set("pygmentsUseClasses") {
        conf.no_classes = !cfg.get_bool("pygmentsUseClasses");
    }

    if conf.code_fences == def.code_fences && cfg.is_set("pygmentsCodeFences") {
        conf.code_fences = cfg.get_bool("pygmentsCodeFences");
    }

    if conf.guess_syntax == def.guess_syntax && cfg.is_set("pygmentsCodefencesGuessSyntax") {
        conf.guess_syntax = cfg.get_bool("pygmentsCodefencesGuessSyntax");
    }

    if cfg.is_set("pygmentsOptions") {
        apply_options_from_string(&cfg.get_string("pygmentsOptions"), conf)?;
    }

    Ok(())
}

// Go: markup/highlight/config.go:parseHighlightOptions
fn parse_highlight_options(input: &str) -> Result<Map> {
    let input = input.trim_matches(' ');
    let mut opts = Map::new(MapType::StringAny);

    if input.is_empty() {
        return Ok(opts);
    }

    for v in input.split(',') {
        let key_val: Vec<&str> = v.split('=').collect();
        let key = key_val[0].trim_matches(' ');
        if key_val.len() != 2 {
            return Err(Error::new(format!("invalid Highlight option: {key}")));
        }
        opts.insert(key, Value::string(key_val[1]));
    }

    normalize_highlight_options(&mut opts);

    Ok(opts)
}

// Go: markup/highlight/config.go:normalizeHighlightOptions
/// Go iterates the map while deleting and re-inserting lower-cased keys; with two keys that
/// fold to the same lower-case key the surviving value is random in Go. The port applies the
/// keys in byte order (the last one wins).
fn normalize_highlight_options(m: &mut Map) {
    // lowercase all keys
    let entries = std::mem::take(&mut m.entries);
    let mut lowered: BTreeMap<go_value::GoString, Value> = BTreeMap::new();
    for (k, v) in entries {
        let lk = go_unicode::strings::to_lower(k.as_bytes()).into_owned();
        lowered.insert(go_value::GoString::new(lk), v);
    }
    m.entries = lowered;

    let mut base_line_number: i64 = 1;
    if let Some(v) = m.get(LINOS_START_KEY.as_bytes()) {
        base_line_number = nh_common::cast::caste::to_int(v);
    }

    let keys: Vec<go_value::GoString> = m.entries.keys().cloned().collect();
    for k in keys {
        let v = m.get(k.as_bytes()).cloned().unwrap_or(Value::Invalid);
        if k.as_bytes() == LINE_NOS_KEY.as_bytes() {
            if let Value::String(s) = &v {
                if s.as_bytes() == b"table" || s.as_bytes() == b"inline" {
                    m.insert("lineNumbersInTable", Value::Bool(s.as_bytes() == b"table"));
                }
                m.insert(k.clone(), Value::Bool(s.as_bytes() != b"false"));
            }
        } else if k.as_bytes() == HL_LINES_KEY.as_bytes() {
            // Only a `[][2]int` (the code block attribute ranges) is rewritten.
            if let Value::List(l) = &v
                && l.ty.go_name() == "[][2]int"
            {
                let ranges: Vec<Value> = l
                    .items
                    .iter()
                    .map(|r| {
                        let r = r.as_list().expect("[2]int");
                        let n = |v: &Value| match v {
                            Value::Int(i, _) => *i,
                            _ => 0,
                        };
                        Value::List(std::sync::Arc::new(go_value::List::new(
                            go_value::SliceType::Named(std::sync::Arc::from("[2]int")),
                            vec![
                                Value::int(n(&r.items[0]).wrapping_add(base_line_number)),
                                Value::int(n(&r.items[1]).wrapping_add(base_line_number)),
                            ],
                        )))
                    })
                    .collect();
                m.entries.remove(k.as_bytes());
                m.insert(
                    format!("{}_parsed", HL_LINES_KEY),
                    Value::List(std::sync::Arc::new(go_value::List::new(
                        go_value::SliceType::Named(std::sync::Arc::from("[][2]int")),
                        ranges,
                    ))),
                );
            }
        }
    }
    let _ = LINEANCHORS_KEY;
}

/// Go: `applyOptions(opts any, cfg)`: a map, or anything `cast.ToStringE` accepts.
// Go: markup/highlight/config.go:applyOptions
pub fn apply_options(opts: &Value, cfg: &mut Config) -> Result<()> {
    match opts {
        Value::Invalid => Ok(()),
        Value::Map(m) if m.ty == MapType::StringAny => {
            let mut m = (**m).clone();
            apply_options_from_map(&mut m, cfg)
        }
        _ => {
            let s = nh_common::cast::caste::to_string_e(opts)?;
            apply_options_from_string(&s.to_str_lossy(), cfg)
        }
    }
}

// Go: markup/highlight/config.go:applyOptionsFromMap
fn apply_options_from_map(optsm: &mut Map, cfg: &mut Config) -> Result<()> {
    normalize_highlight_options(optsm);
    nh_config::decode::weak_decode_into(&Value::map(optsm.clone()), cfg)
}

/// Go: `applyOptionsFromCodeBlockContext(ctx, cfg)`: the line anchors default to
/// `hl-<ordinal>`.
// Go: markup/highlight/config.go:applyOptionsFromCodeBlockContext
pub fn apply_options_from_code_block_context(ordinal: i64, cfg: &mut Config) -> Result<()> {
    if cfg.line_anchors.is_empty() {
        const LINE_ANCHOR_PREFIX: &str = "hl-";
        // Set it to the ordinal with a prefix.
        cfg.line_anchors = format!("{LINE_ANCHOR_PREFIX}{ordinal}");
    }
    Ok(())
}

/// startLine compensates for https://github.com/alecthomas/chroma/issues/30
// Go: markup/highlight/config.go:hlLinesToRanges
pub fn hl_lines_to_ranges(start_line: i64, s: &str) -> Result<Vec<[i64; 2]>> {
    let mut ranges = Vec::new();
    let s = go_unicode::strings::trim_space_str(s);

    if s.is_empty() {
        return Ok(ranges);
    }

    // Variants:
    // 1 2 3 4
    // 1-2 3-4
    // 1-2 3
    // 1 3-4
    // 1    3-4
    for field in s.split(' ') {
        let field = go_unicode::strings::trim_space_str(field);
        if field.is_empty() {
            continue;
        }
        let numbers: Vec<&str> = field.split('-').collect();
        let mut r = [0i64; 2];
        let first = go_strconv::atoi(numbers[0]).map_err(|e| Error::new(e.to_string()))?;
        let first = first.wrapping_add(start_line).wrapping_sub(1);
        r[0] = first;
        if numbers.len() > 1 {
            let second = go_strconv::atoi(numbers[1]).map_err(|e| Error::new(e.to_string()))?;
            r[1] = second.wrapping_add(start_line).wrapping_sub(1);
        } else {
            r[1] = first;
        }

        ranges.push(r);
    }
    Ok(ranges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nh_config::config_provider::Provider;

    // Go: markup/highlight/config_test.go:TestConfig (applyLegacyConfig, parseOptions)
    #[test]
    fn legacy_config_and_options() {
        let v = nh_config::default_config_provider::DefaultConfigProvider::new();
        v.set("pygmentsStyle", Value::string("hugo"));
        v.set("pygmentsUseClasses", Value::Bool(false));
        v.set("pygmentsCodeFences", Value::Bool(false));
        v.set("pygmentsOptions", Value::string("linenos=inline"));

        let mut cfg = default_config();
        apply_legacy_config(&v, &mut cfg).unwrap();
        assert_eq!(cfg.style, "hugo");
        assert!(cfg.no_classes);
        assert!(!cfg.code_fences);
        assert!(cfg.line_nos);
        assert!(!cfg.line_numbers_in_table);

        let mut cfg = default_config();
        apply_options_from_string(
            "noclasses=true,linenos=inline,linenostart=32,hl_lines=3-8 10-20",
            &mut cfg,
        )
        .unwrap();
        assert!(cfg.no_classes);
        assert!(cfg.line_nos);
        assert!(!cfg.line_numbers_in_table);
        assert_eq!(cfg.line_no_start, 32);
        assert_eq!(cfg.hl_lines, "3-8 10-20");

        let mut cfg = default_config();
        let err = apply_options_from_string("linenos", &mut cfg).unwrap_err();
        assert_eq!(err.to_string(), "invalid Highlight option: linenos");
    }

    // Go: markup/highlight/config_test.go:TestConfig (applyOptionsFromMap)
    #[test]
    fn options_from_map() {
        let mut cfg = default_config();
        let mut m = Map::new(MapType::StringAny);
        m.insert("noclasses", Value::Bool(true));
        // mixed case key, should work after normalization
        m.insert("lineNos", Value::string("inline"));
        m.insert("linenostart", Value::int(32));
        m.insert("hl_lines", Value::string("3-8 10-20"));
        apply_options(&Value::map(m), &mut cfg).unwrap();
        assert!(cfg.no_classes);
        assert!(cfg.line_nos);
        assert!(!cfg.line_numbers_in_table);
        assert_eq!(cfg.line_no_start, 32);
        assert_eq!(cfg.hl_lines, "3-8 10-20");
    }

    #[test]
    fn hl_lines_ranges() {
        assert_eq!(
            hl_lines_to_ranges(1, " 1 2-3  5 ").unwrap(),
            vec![[1, 1], [2, 3], [5, 5]]
        );
        assert_eq!(hl_lines_to_ranges(10, "3-4").unwrap(), vec![[12, 13]]);
        assert!(hl_lines_to_ranges(1, "a").is_err());
        let mut cfg = default_config();
        apply_options_from_code_block_context(3, &mut cfg).unwrap();
        assert_eq!(cfg.line_anchors, "hl-3");
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/highlight/config.go (296 lines; 4/9 funcs executed)
//   types: Config
//    L88-121: (cfg Config) toHTMLOptions() []html.Option STUB (Chroma options)
// OK L123-137: applyOptions(opts any, cfg *Config) error
// OK L139-145: applyOptionsFromString(opts string, cfg *Config) error
// OK L147-150: applyOptionsFromMap(optsm map[string]any, cfg *Config) error
// OK L152-160: applyOptionsFromCodeBlockContext(ctx hooks.CodeblockContext, cfg *Config) error
// OK L164-190: ApplyLegacyConfig(cfg config.Provider, conf *Config) error
// OK L192-213: parseHighlightOptions(in string) (map[string]any, error)
// OK L215-251: normalizeHighlightOptions(m map[string]any)
// OK L254-296: hlLinesToRanges(startLine int, s string) ([][2]int, error)
// ---------------------------------------------------------------------------
