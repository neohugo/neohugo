//! Port of `minifiers/config.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).

use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_config::decode::{FieldRef, weak_decode_into};
use nh_config::decode_struct;

/// Go: `minifiers.MinifyConfig`.
///
/// `Default` is Go's zero value (what mapstructure's `ZeroFields` would set); Go's
/// `defaultConfig` is [`default_config`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MinifyConfig {
    /// Whether to minify the published output (`--minify` -> `minify.minifyOutput`).
    pub minify_output: bool,
    pub disable_html: bool,
    pub disable_css: bool,
    pub disable_js: bool,
    pub disable_json: bool,
    pub disable_svg: bool,
    pub disable_xml: bool,
    pub tdewolff: TdewolffConfig,
}

decode_struct!(MinifyConfig, "minifiers.MinifyConfig", |s| vec![
    FieldRef::new("MinifyOutput", &mut s.minify_output),
    FieldRef::new("DisableHTML", &mut s.disable_html),
    FieldRef::new("DisableCSS", &mut s.disable_css),
    FieldRef::new("DisableJS", &mut s.disable_js),
    FieldRef::new("DisableJSON", &mut s.disable_json),
    FieldRef::new("DisableSVG", &mut s.disable_svg),
    FieldRef::new("DisableXML", &mut s.disable_xml),
    FieldRef::new("Tdewolff", &mut s.tdewolff),
]);

/// Go: `minifiers.TdewolffConfig` — the tdewolff minifier options, one struct per Go
/// `{html,css,js,json,svg,xml}.Minifier` (same fields, including the unexported ones that
/// mapstructure matches but never sets).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TdewolffConfig {
    pub html: HtmlOptions,
    pub css: CssOptions,
    pub js: JsOptions,
    pub json: JsonOptions,
    pub svg: SvgOptions,
    pub xml: XmlOptions,
}

decode_struct!(TdewolffConfig, "minifiers.TdewolffConfig", |s| vec![
    FieldRef::new("HTML", &mut s.html),
    FieldRef::new("CSS", &mut s.css),
    FieldRef::new("JS", &mut s.js),
    FieldRef::new("JSON", &mut s.json),
    FieldRef::new("SVG", &mut s.svg),
    FieldRef::new("XML", &mut s.xml),
]);

/// Go: `html.Minifier` options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HtmlOptions {
    pub keep_comments: bool,
    pub keep_conditional_comments: bool,
    pub keep_special_comments: bool,
    pub keep_default_attr_vals: bool,
    pub keep_document_tags: bool,
    pub keep_end_tags: bool,
    pub keep_quotes: bool,
    pub keep_whitespace: bool,
    pub template_delims: [String; 2],
}

decode_struct!(HtmlOptions, "html.Minifier", |s| vec![
    FieldRef::new("KeepComments", &mut s.keep_comments),
    FieldRef::new("KeepConditionalComments", &mut s.keep_conditional_comments),
    FieldRef::new("KeepSpecialComments", &mut s.keep_special_comments),
    FieldRef::new("KeepDefaultAttrVals", &mut s.keep_default_attr_vals),
    FieldRef::new("KeepDocumentTags", &mut s.keep_document_tags),
    FieldRef::new("KeepEndTags", &mut s.keep_end_tags),
    FieldRef::new("KeepQuotes", &mut s.keep_quotes),
    FieldRef::new("KeepWhitespace", &mut s.keep_whitespace),
    FieldRef::new("TemplateDelims", &mut s.template_delims),
]);

/// Go: `css.Minifier` options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CssOptions {
    pub keep_css2: bool,
    /// number of significant digits
    pub precision: i64,
    /// Go's unexported `newPrecision` (matched by mapstructure, never set).
    pub new_precision: i64,
    pub inline: bool,
}

decode_struct!(CssOptions, "css.Minifier", |s| vec![
    FieldRef::new("KeepCSS2", &mut s.keep_css2),
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("newPrecision", &mut s.new_precision).unexported(),
    FieldRef::new("Inline", &mut s.inline),
]);

/// Go: `js.Minifier` options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JsOptions {
    /// number of significant digits
    pub precision: i64,
    pub keep_var_names: bool,
    /// Go's unexported `useAlphabetVarNames` (matched by mapstructure, never set).
    pub use_alphabet_var_names: bool,
    pub version: i64,
}

decode_struct!(JsOptions, "js.Minifier", |s| vec![
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("KeepVarNames", &mut s.keep_var_names),
    FieldRef::new("useAlphabetVarNames", &mut s.use_alphabet_var_names).unexported(),
    FieldRef::new("Version", &mut s.version),
]);

/// Go: `json.Minifier` options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JsonOptions {
    /// number of significant digits
    pub precision: i64,
    /// prevent numbers from being minified
    pub keep_numbers: bool,
}

decode_struct!(JsonOptions, "json.Minifier", |s| vec![
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("KeepNumbers", &mut s.keep_numbers),
]);

/// Go: `svg.Minifier` options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgOptions {
    pub keep_comments: bool,
    /// number of significant digits
    pub precision: i64,
    /// Go's unexported `newPrecision` (matched by mapstructure, never set).
    pub new_precision: i64,
    pub inline: bool,
}

decode_struct!(SvgOptions, "svg.Minifier", |s| vec![
    FieldRef::new("KeepComments", &mut s.keep_comments),
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("newPrecision", &mut s.new_precision).unexported(),
    FieldRef::new("Inline", &mut s.inline),
]);

/// Go: `xml.Minifier` options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XmlOptions {
    pub keep_whitespace: bool,
}

decode_struct!(XmlOptions, "xml.Minifier", |s| vec![FieldRef::new(
    "KeepWhitespace",
    &mut s.keep_whitespace
),]);

impl HtmlOptions {
    /// The tdewolff minifier with these options.
    pub fn minifier(&self) -> tdewolff_minify::html::Minifier {
        tdewolff_minify::html::Minifier {
            keep_comments: self.keep_comments,
            keep_conditional_comments: self.keep_conditional_comments,
            keep_special_comments: self.keep_special_comments,
            keep_default_attr_vals: self.keep_default_attr_vals,
            keep_document_tags: self.keep_document_tags,
            keep_end_tags: self.keep_end_tags,
            keep_quotes: self.keep_quotes,
            keep_whitespace: self.keep_whitespace,
            template_delims: self.template_delims.clone(),
        }
    }
}

impl CssOptions {
    /// The tdewolff minifier with these options (`newPrecision` is recomputed by `Minify`).
    pub fn minifier(&self) -> tdewolff_minify::css::Minifier {
        tdewolff_minify::css::Minifier {
            keep_css2: self.keep_css2,
            precision: self.precision,
            inline: self.inline,
        }
    }
}

impl JsOptions {
    /// The tdewolff minifier with these options.
    pub fn minifier(&self) -> tdewolff_minify_js::Minifier {
        tdewolff_minify_js::Minifier {
            precision: self.precision,
            keep_var_names: self.keep_var_names,
            use_alphabet_var_names: self.use_alphabet_var_names,
            version: self.version,
        }
    }
}

impl JsonOptions {
    /// The tdewolff minifier with these options.
    pub fn minifier(&self) -> tdewolff_minify::json::Minifier {
        tdewolff_minify::json::Minifier {
            precision: self.precision,
            keep_numbers: self.keep_numbers,
        }
    }
}

impl SvgOptions {
    /// The tdewolff minifier with these options (`newPrecision` is recomputed by `Minify`).
    pub fn minifier(&self) -> tdewolff_minify::svg::Minifier {
        tdewolff_minify::svg::Minifier {
            keep_comments: self.keep_comments,
            precision: self.precision,
            inline: self.inline,
        }
    }
}

impl XmlOptions {
    /// The tdewolff minifier with these options.
    pub fn minifier(&self) -> tdewolff_minify::xml::Minifier {
        tdewolff_minify::xml::Minifier {
            keep_whitespace: self.keep_whitespace,
        }
    }
}

/// Go: `defaultTdewolffConfig`.
pub fn default_tdewolff_config() -> TdewolffConfig {
    TdewolffConfig {
        html: HtmlOptions {
            keep_document_tags: true,
            keep_special_comments: true,
            keep_end_tags: true,
            keep_default_attr_vals: true,
            keep_whitespace: false,
            ..Default::default()
        },
        css: CssOptions {
            precision: 0,
            keep_css2: true,
            ..Default::default()
        },
        js: JsOptions {
            version: 2022,
            ..Default::default()
        },
        json: JsonOptions::default(),
        svg: SvgOptions {
            keep_comments: false,
            precision: 0,
            ..Default::default()
        },
        xml: XmlOptions {
            keep_whitespace: false,
        },
    }
}

/// Go: `defaultConfig`.
pub fn default_config() -> MinifyConfig {
    MinifyConfig {
        tdewolff: default_tdewolff_config(),
        ..Default::default()
    }
}

/// Go `maps.ToStringMap(v)` returns `v` itself (the same map) for a `map[string]interface {}` or
/// `maps.Params`, and a new map otherwise; only in the first case do `DecodeConfig`'s renames
/// reach the map that is decoded.
fn to_string_map_aliases(v: &Value) -> bool {
    matches!(v, Value::Map(m) if matches!(m.ty, MapType::StringAny | MapType::Params))
}

/// Go: `minifiers.DecodeConfig(v)` (css/svg `decimal`->`precision`, html `keepConditionalComments`
/// -> `keepSpecialComments` renames, then WeakDecode over the defaults).
// Go: minifiers/config.go:DecodeConfig
pub fn decode_config(v: &Value) -> Result<MinifyConfig> {
    let mut conf = default_config();

    if matches!(v, Value::Invalid) {
        return Ok(conf);
    }

    let mut m: Map = nh_common::maps::maps::to_string_map(v);

    // Handle upstream renames.
    if let Some(td) = m.get(b"tdewolff").cloned() {
        let mut tdm = nh_common::maps::maps::to_string_map(&td);

        for key in ["css", "svg"] {
            if let Some(v) = tdm.get(key.as_bytes()).cloned() {
                let mut vm = nh_common::maps::maps::to_string_map(&v);
                let ko: &[u8] = b"decimal";
                let kn: &[u8] = b"precision";
                if let Some(vv) = vm.get(ko).cloned() {
                    if vm.get(kn).is_none() {
                        let vvi = nh_common::cast::caste::to_int(&vv);
                        if vvi > 0 {
                            vm.insert(kn, Value::int(vvi));
                        }
                    }
                    vm.entries.remove(ko);
                }
                if to_string_map_aliases(&v) {
                    tdm.insert(key, Value::map(vm));
                }
            }
        }

        // keepConditionalComments was renamed to keepSpecialComments
        if let Some(v) = tdm.get(b"html").cloned() {
            let mut vm = nh_common::maps::maps::to_string_map(&v);
            let ko: &[u8] = b"keepconditionalcomments";
            let kn: &[u8] = b"keepspecialcomments";
            if let Some(vv) = vm.get(ko).cloned() {
                // Set keepspecialcomments, if not already set
                if vm.get(kn).is_none() {
                    vm.insert(kn, Value::Bool(nh_common::cast::caste::to_bool(&vv)));
                }
                // Remove the old key to prevent deprecation warnings
                vm.entries.remove(ko);
            }
            if to_string_map_aliases(&v) {
                tdm.insert("html", Value::map(vm));
            }
        }

        if to_string_map_aliases(&td) {
            m.insert("tdewolff", Value::map(tdm));
        }
    }

    weak_decode_into(&Value::map(m), &mut conf)?;

    Ok(conf)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: minifiers/config.go (134 lines; 1/1 funcs executed)
//   types: TdewolffConfig, MinifyConfig
// OK L81-134: DecodeConfig(v any) (conf MinifyConfig, err error)
// ---------------------------------------------------------------------------
