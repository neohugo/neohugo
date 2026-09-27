//! Port of `minifiers/config.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).


use nh_common::Result;

/// Go: `minifiers.MinifyConfig`.
#[derive(Clone, Debug, Default)]
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

/// Go: `minifiers.TdewolffConfig` — the tdewolff minifier options (Wave B: hold the
/// `tdewolff_minify::{html,css,js,json,svg,xml}::Minifier` option structs directly).
#[derive(Clone, Debug, Default)]
pub struct TdewolffConfig {
    pub html: HtmlOptions,
    pub css_precision: i64,
    pub css_keep_css2: bool,
    pub js_precision: i64,
    pub js_keep_var_names: bool,
    pub js_version: i64,
    pub json_precision: i64,
    pub json_keep_numbers: bool,
    pub svg_keep_comments: bool,
    pub svg_precision: i64,
    pub xml_keep_whitespace: bool,
}

/// Go: `html.Minifier` options.
#[derive(Clone, Debug, Default)]
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

/// Go: `minifiers.DecodeConfig(v)` (css/svg `decimal`->`precision`, html `keepConditionalComments`
/// -> `keepSpecialComments` renames, then WeakDecode over the defaults).
// Go: minifiers/config.go:DecodeConfig
pub fn decode_config(v: &go_value::Value) -> Result<MinifyConfig> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: minifiers/config.go (134 lines; 1/1 funcs executed)
//   types: TdewolffConfig, MinifyConfig
// EX L81-134: DecodeConfig(v any) (conf MinifyConfig, err error)
// ---------------------------------------------------------------------------
