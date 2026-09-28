//! Differential tests of the mapstructure port and the config decoders against
//! `tools/go-oracle/nh-config/decode` (fixture `decode/decode.json.gz`, arm64 Go).

mod support;

use std::collections::BTreeMap;

use go_value::{Map, MapType, Value};
use nh_config::common_config::{CommonDirs, PageConfig, Pagination, SitemapConfig};
use nh_config::config_provider::Provider;
use nh_config::decode::{
    AnyValue, Decode, Decoder, DecoderConfig, FieldRef, Int64, Uint64, string_to_time_duration_hook,
};
use nh_config::decode_struct;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_config::{privacy, security, services};
use serde_json::{Value as J, json};
use support::dump::{Dump, dump_map, dump_struct};
use support::{catch, decode, str_enc};

// ---------------------------------------------------------------------------
// The oracle's structs

#[derive(Clone, Debug, Default, PartialEq)]
struct Inner {
    a: String,
    b: i64,
    c: Vec<String>,
}
decode_struct!(Inner, "main.Inner", |s| vec![
    FieldRef::new("A", &mut s.a),
    FieldRef::new("B", &mut s.b),
    FieldRef::new("C", &mut s.c),
]);
dump_struct!(Inner, "main.Inner", |s| { "A" => s.a.dump(), "B" => s.b.dump(), "C" => s.c.dump() });

#[derive(Clone, Debug, Default, PartialEq)]
struct Embedded {
    e1: String,
    e2: bool,
    e3: Option<Box<Inner>>,
}
decode_struct!(Embedded, "main.Embedded", |s| vec![
    FieldRef::new("E1", &mut s.e1),
    FieldRef::new("E2", &mut s.e2),
    FieldRef::new("E3", &mut s.e3),
]);
dump_struct!(Embedded, "main.Embedded", |s| { "E1" => s.e1.dump(), "E2" => s.e2.dump(), "E3" => s.e3.dump() });

#[derive(Clone, Debug, PartialEq)]
struct Kitchen {
    s: String,
    b: bool,
    i: i64,
    i8_: i8,
    i16_: i16,
    i32_: i32,
    i64_: Int64,
    u: u64,
    u8_: u8,
    u16_: u16,
    u32_: u32,
    u64_: Uint64,
    f32_: f32,
    f64_: f64,
    ss: Vec<String>,
    is: Vec<i64>,
    bs: Vec<u8>,
    fs: Vec<f64>,
    as_: Vec<AnyValue>,
    msa: Map,
    mss: BTreeMap<String, String>,
    msi: BTreeMap<String, i64>,
    msin: BTreeMap<String, Inner>,
    any: Value,
    in_: Inner,
    in_p: Option<Box<Inner>>,
    ins: Vec<Inner>,
    in_ps: Vec<Option<Box<Inner>>>,
    arr: [String; 2],
    arr_i: [i64; 3],
    embedded: Embedded,
    tagged: String,
    tagged_opt: String,
    dash: String,
    dur: go_time::Duration,
    params: Map,
    unexp: String,
}

impl Default for Kitchen {
    fn default() -> Self {
        Kitchen {
            s: String::new(),
            b: false,
            i: 0,
            i8_: 0,
            i16_: 0,
            i32_: 0,
            i64_: Int64(0),
            u: 0,
            u8_: 0,
            u16_: 0,
            u32_: 0,
            u64_: Uint64(0),
            f32_: 0.0,
            f64_: 0.0,
            ss: Vec::new(),
            is: Vec::new(),
            bs: Vec::new(),
            fs: Vec::new(),
            as_: Vec::new(),
            msa: Map::new(MapType::StringAny),
            mss: BTreeMap::new(),
            msi: BTreeMap::new(),
            msin: BTreeMap::new(),
            any: Value::Invalid,
            in_: Inner::default(),
            in_p: None,
            ins: Vec::new(),
            in_ps: Vec::new(),
            arr: Default::default(),
            arr_i: [0; 3],
            embedded: Embedded::default(),
            tagged: String::new(),
            tagged_opt: String::new(),
            dash: String::new(),
            dur: go_time::Duration(0),
            params: Map::new(MapType::Params),
            unexp: String::new(),
        }
    }
}

decode_struct!(Kitchen, "main.Kitchen", |s| vec![
    FieldRef::new("S", &mut s.s),
    FieldRef::new("B", &mut s.b),
    FieldRef::new("I", &mut s.i),
    FieldRef::new("I8", &mut s.i8_),
    FieldRef::new("I16", &mut s.i16_),
    FieldRef::new("I32", &mut s.i32_),
    FieldRef::new("I64", &mut s.i64_),
    FieldRef::new("U", &mut s.u),
    FieldRef::new("U8", &mut s.u8_),
    FieldRef::new("U16", &mut s.u16_),
    FieldRef::new("U32", &mut s.u32_),
    FieldRef::new("U64", &mut s.u64_),
    FieldRef::new("F32", &mut s.f32_),
    FieldRef::new("F64", &mut s.f64_),
    FieldRef::new("SS", &mut s.ss),
    FieldRef::new("IS", &mut s.is),
    FieldRef::new("BS", &mut s.bs),
    FieldRef::new("FS", &mut s.fs),
    FieldRef::new("AS", &mut s.as_),
    FieldRef::new("MSA", &mut s.msa),
    FieldRef::new("MSS", &mut s.mss),
    FieldRef::new("MSI", &mut s.msi),
    FieldRef::new("MSIn", &mut s.msin),
    FieldRef::new("Any", &mut s.any),
    FieldRef::new("In", &mut s.in_),
    FieldRef::new("InP", &mut s.in_p),
    FieldRef::new("Ins", &mut s.ins),
    FieldRef::new("InPs", &mut s.in_ps),
    FieldRef::new("Arr", &mut s.arr),
    FieldRef::new("ArrI", &mut s.arr_i),
    FieldRef::squash("Embedded", &mut s.embedded),
    FieldRef::new("Tagged", &mut s.tagged).tag("renamed"),
    FieldRef::new("TaggedOpt", &mut s.tagged_opt).tag("other,omitempty"),
    FieldRef::new("Dash", &mut s.dash).tag("-"),
    FieldRef::new("Dur", &mut s.dur),
    FieldRef::new("Params", &mut s.params),
    FieldRef::new("unexp", &mut s.unexp).unexported(),
]);

dump_struct!(Kitchen, "main.Kitchen", |s| {
    "S" => s.s.dump(), "B" => s.b.dump(), "I" => s.i.dump(), "I8" => s.i8_.dump(),
    "I16" => s.i16_.dump(), "I32" => s.i32_.dump(), "I64" => s.i64_.dump(), "U" => s.u.dump(),
    "U8" => s.u8_.dump(), "U16" => s.u16_.dump(), "U32" => s.u32_.dump(), "U64" => s.u64_.dump(),
    "F32" => s.f32_.dump(), "F64" => s.f64_.dump(), "SS" => s.ss.dump(), "IS" => s.is.dump(),
    "BS" => s.bs.dump(), "FS" => s.fs.dump(), "AS" => s.as_.dump(), "MSA" => s.msa.dump(),
    "MSS" => dump_map("map[string]string", &s.mss), "MSI" => dump_map("map[string]int", &s.msi),
    "MSIn" => dump_map("map[string]main.Inner", &s.msin), "Any" => s.any.dump(),
    "In" => s.in_.dump(), "InP" => s.in_p.dump(), "Ins" => s.ins.dump(), "InPs" => s.in_ps.dump(),
    "Arr" => s.arr.dump(), "ArrI" => s.arr_i.dump(), "Embedded" => s.embedded.dump(),
    "Tagged" => s.tagged.dump(), "TaggedOpt" => s.tagged_opt.dump(), "Dash" => s.dash.dump(),
    "Dur" => s.dur.dump(), "Params" => s.params.dump(),
});

#[derive(Clone, Debug, PartialEq)]
struct Remain {
    name: String,
    rest: Map,
}
impl Default for Remain {
    fn default() -> Self {
        Remain {
            name: String::new(),
            rest: Map::new(MapType::StringAny),
        }
    }
}
decode_struct!(Remain, "main.Remain", |s| vec![
    FieldRef::new("Name", &mut s.name),
    FieldRef::new("Rest", &mut s.rest).tag(",remain"),
]);
dump_struct!(Remain, "main.Remain", |s| { "Name" => s.name.dump(), "Rest" => s.rest.dump() });

#[derive(Clone, Debug, Default, PartialEq)]
struct EmbedNoTag {
    inner: Inner,
    x: i64,
}
decode_struct!(EmbedNoTag, "main.EmbedNoTag", |s| vec![
    FieldRef::new("Inner", &mut s.inner).anonymous(),
    FieldRef::new("X", &mut s.x),
]);
dump_struct!(EmbedNoTag, "main.EmbedNoTag", |s| { "Inner" => s.inner.dump(), "X" => s.x.dump() });

#[derive(Clone, Debug, Default, PartialEq)]
struct BadSquash {
    s: String,
    t: i64,
}
decode_struct!(BadSquash, "main.BadSquash", |s| vec![
    FieldRef::new("S", &mut s.s).tag(",squash"),
    FieldRef::new("T", &mut s.t),
]);
dump_struct!(BadSquash, "main.BadSquash", |s| { "S" => s.s.dump(), "T" => s.t.dump() });

#[derive(Clone, Debug, PartialEq)]
struct RootLike {
    base_url: String,
    title: String,
    timeout: String,
    ugly_urls: Value,
    disable_kinds: Vec<String>,
    taxonomies: BTreeMap<String, String>,
    summary_length: i64,
    build_drafts: bool,
    default_content_language: String,
    common_dirs: CommonDirs,
    internal: String,
}
impl Default for RootLike {
    fn default() -> Self {
        RootLike {
            base_url: String::new(),
            title: String::new(),
            timeout: String::new(),
            ugly_urls: Value::Invalid,
            disable_kinds: Vec::new(),
            taxonomies: BTreeMap::new(),
            summary_length: 0,
            build_drafts: false,
            default_content_language: String::new(),
            common_dirs: CommonDirs::default(),
            internal: String::new(),
        }
    }
}
decode_struct!(RootLike, "main.RootLike", |s| vec![
    FieldRef::new("BaseURL", &mut s.base_url),
    FieldRef::new("Title", &mut s.title),
    FieldRef::new("Timeout", &mut s.timeout),
    FieldRef::new("UglyURLs", &mut s.ugly_urls),
    FieldRef::new("DisableKinds", &mut s.disable_kinds),
    FieldRef::new("Taxonomies", &mut s.taxonomies),
    FieldRef::new("SummaryLength", &mut s.summary_length),
    FieldRef::new("BuildDrafts", &mut s.build_drafts),
    FieldRef::new("DefaultContentLanguage", &mut s.default_content_language),
    FieldRef::squash("CommonDirs", &mut s.common_dirs),
    FieldRef::new("Internal", &mut s.internal).tag("-"),
]);
dump_struct!(CommonDirs, "config.CommonDirs", |s| {
    "ThemesDir" => s.themes_dir.dump(), "PublishDir" => s.publish_dir.dump(),
    "ResourceDir" => s.resource_dir.dump(), "WorkingDir" => s.working_dir.dump(),
    "CacheDir" => s.cache_dir.dump(), "ContentDir" => s.content_dir.dump(),
    "DataDir" => s.data_dir.dump(), "LayoutDir" => s.layout_dir.dump(),
    "I18nDir" => s.i18n_dir.dump(), "ArcheTypeDir" => s.arche_type_dir.dump(),
    "AssetDir" => s.asset_dir.dump(),
});
dump_struct!(RootLike, "main.RootLike", |s| {
    "BaseURL" => s.base_url.dump(), "Title" => s.title.dump(), "Timeout" => s.timeout.dump(),
    "UglyURLs" => s.ugly_urls.dump(), "DisableKinds" => s.disable_kinds.dump(),
    "Taxonomies" => dump_map("map[string]string", &s.taxonomies),
    "SummaryLength" => s.summary_length.dump(), "BuildDrafts" => s.build_drafts.dump(),
    "DefaultContentLanguage" => s.default_content_language.dump(),
    "CommonDirs" => s.common_dirs.dump(), "Internal" => s.internal.dump(),
});

// ---------------------------------------------------------------------------
// The minifiers config (T07 owns `minifiers.MinifyConfig`; this mirrors its struct to check the
// decoder on it).

#[derive(Clone, Debug, Default, PartialEq)]
struct HtmlMin {
    keep_comments: bool,
    keep_conditional_comments: bool,
    keep_special_comments: bool,
    keep_default_attr_vals: bool,
    keep_document_tags: bool,
    keep_end_tags: bool,
    keep_quotes: bool,
    keep_whitespace: bool,
    template_delims: [String; 2],
}
decode_struct!(HtmlMin, "html.Minifier", |s| vec![
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
dump_struct!(HtmlMin, "html.Minifier", |s| {
    "KeepComments" => s.keep_comments.dump(), "KeepConditionalComments" => s.keep_conditional_comments.dump(),
    "KeepSpecialComments" => s.keep_special_comments.dump(), "KeepDefaultAttrVals" => s.keep_default_attr_vals.dump(),
    "KeepDocumentTags" => s.keep_document_tags.dump(), "KeepEndTags" => s.keep_end_tags.dump(),
    "KeepQuotes" => s.keep_quotes.dump(), "KeepWhitespace" => s.keep_whitespace.dump(),
    "TemplateDelims" => s.template_delims.dump(),
});

#[derive(Clone, Debug, Default, PartialEq)]
struct CssMin {
    keep_css2: bool,
    precision: i64,
    new_precision: i64,
    inline: bool,
}
decode_struct!(CssMin, "css.Minifier", |s| vec![
    FieldRef::new("KeepCSS2", &mut s.keep_css2),
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("newPrecision", &mut s.new_precision).unexported(),
    FieldRef::new("Inline", &mut s.inline),
]);
dump_struct!(CssMin, "css.Minifier", |s| {
    "KeepCSS2" => s.keep_css2.dump(), "Precision" => s.precision.dump(), "Inline" => s.inline.dump(),
});

#[derive(Clone, Debug, Default, PartialEq)]
struct JsMin {
    precision: i64,
    keep_var_names: bool,
    use_alphabet_var_names: bool,
    version: i64,
}
decode_struct!(JsMin, "js.Minifier", |s| vec![
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("KeepVarNames", &mut s.keep_var_names),
    FieldRef::new("useAlphabetVarNames", &mut s.use_alphabet_var_names).unexported(),
    FieldRef::new("Version", &mut s.version),
]);
dump_struct!(JsMin, "js.Minifier", |s| {
    "Precision" => s.precision.dump(), "KeepVarNames" => s.keep_var_names.dump(), "Version" => s.version.dump(),
});

#[derive(Clone, Debug, Default, PartialEq)]
struct JsonMin {
    precision: i64,
    keep_numbers: bool,
}
decode_struct!(JsonMin, "json.Minifier", |s| vec![
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("KeepNumbers", &mut s.keep_numbers),
]);
dump_struct!(JsonMin, "json.Minifier", |s| { "Precision" => s.precision.dump(), "KeepNumbers" => s.keep_numbers.dump() });

#[derive(Clone, Debug, Default, PartialEq)]
struct SvgMin {
    keep_comments: bool,
    precision: i64,
    new_precision: i64,
    inline: bool,
}
decode_struct!(SvgMin, "svg.Minifier", |s| vec![
    FieldRef::new("KeepComments", &mut s.keep_comments),
    FieldRef::new("Precision", &mut s.precision),
    FieldRef::new("newPrecision", &mut s.new_precision).unexported(),
    FieldRef::new("Inline", &mut s.inline),
]);
dump_struct!(SvgMin, "svg.Minifier", |s| {
    "KeepComments" => s.keep_comments.dump(), "Precision" => s.precision.dump(), "Inline" => s.inline.dump(),
});

#[derive(Clone, Debug, Default, PartialEq)]
struct XmlMin {
    keep_whitespace: bool,
}
decode_struct!(XmlMin, "xml.Minifier", |s| vec![FieldRef::new(
    "KeepWhitespace",
    &mut s.keep_whitespace
)]);
dump_struct!(XmlMin, "xml.Minifier", |s| { "KeepWhitespace" => s.keep_whitespace.dump() });

#[derive(Clone, Debug, Default, PartialEq)]
struct Tdewolff {
    html: HtmlMin,
    css: CssMin,
    js: JsMin,
    json: JsonMin,
    svg: SvgMin,
    xml: XmlMin,
}
decode_struct!(Tdewolff, "minifiers.TdewolffConfig", |s| vec![
    FieldRef::new("HTML", &mut s.html),
    FieldRef::new("CSS", &mut s.css),
    FieldRef::new("JS", &mut s.js),
    FieldRef::new("JSON", &mut s.json),
    FieldRef::new("SVG", &mut s.svg),
    FieldRef::new("XML", &mut s.xml),
]);
dump_struct!(Tdewolff, "minifiers.TdewolffConfig", |s| {
    "HTML" => s.html.dump(), "CSS" => s.css.dump(), "JS" => s.js.dump(),
    "JSON" => s.json.dump(), "SVG" => s.svg.dump(), "XML" => s.xml.dump(),
});

#[derive(Clone, Debug, Default, PartialEq)]
struct MinifyConfig {
    minify_output: bool,
    disable_html: bool,
    disable_css: bool,
    disable_js: bool,
    disable_json: bool,
    disable_svg: bool,
    disable_xml: bool,
    tdewolff: Tdewolff,
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
dump_struct!(MinifyConfig, "minifiers.MinifyConfig", |s| {
    "MinifyOutput" => s.minify_output.dump(), "DisableHTML" => s.disable_html.dump(),
    "DisableCSS" => s.disable_css.dump(), "DisableJS" => s.disable_js.dump(),
    "DisableJSON" => s.disable_json.dump(), "DisableSVG" => s.disable_svg.dump(),
    "DisableXML" => s.disable_xml.dump(), "Tdewolff" => s.tdewolff.dump(),
});

/// Go: `minifiers.defaultConfig`.
fn default_minify_config() -> MinifyConfig {
    MinifyConfig {
        tdewolff: Tdewolff {
            html: HtmlMin {
                keep_document_tags: true,
                keep_special_comments: true,
                keep_end_tags: true,
                keep_default_attr_vals: true,
                keep_whitespace: false,
                ..Default::default()
            },
            css: CssMin {
                precision: 0,
                keep_css2: true,
                ..Default::default()
            },
            js: JsMin {
                version: 2022,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------

fn decoder_config(cfg: &str) -> DecoderConfig<'static> {
    static DURATION: fn(&Value, &dyn Decode) -> Result<Value, String> =
        string_to_time_duration_hook;
    let mut dc = DecoderConfig {
        weakly_typed_input: true,
        ..Default::default()
    };
    match cfg {
        "weak" => {}
        "strict" => dc.weakly_typed_input = false,
        "unused" => dc.error_unused = true,
        "unset" => dc.error_unset = true,
        "zero" => dc.zero_fields = true,
        "squash" => dc.squash = true,
        "duration" => dc.decode_hook = Some(&DURATION),
        other => panic!("cfg {other}"),
    }
    dc
}

/// Decodes into a new `T` (after the prefill) and returns Go's `{"ok": dump, "err"?: msg}`.
fn run_struct<T: Decode + Dump + Default>(cfg: &str, prefill: Option<&Value>, input: &Value) -> J {
    let mut t = T::default();
    if let Some(p) = prefill {
        nh_config::decode::weak_decode_into(p, &mut t).expect("prefill decodes");
    }
    let d = Decoder::new(decoder_config(cfg));
    let r = d.decode_input(input, &mut t);
    let mut out = json!({"ok": t.dump()});
    if let Err(e) = r {
        out["err"] = str_enc(e.message().as_bytes());
    }
    out
}

fn run_target(target: &str, cfg: &str, prefill: Option<&Value>, input: &Value) -> J {
    match target {
        "Kitchen" => run_struct::<Kitchen>(cfg, prefill, input),
        "Remain" => run_struct::<Remain>(cfg, prefill, input),
        "EmbedNoTag" => run_struct::<EmbedNoTag>(cfg, prefill, input),
        "BadSquash" => run_struct::<BadSquash>(cfg, prefill, input),
        "RootLike" => run_struct::<RootLike>(cfg, prefill, input),
        "Pagination" => {
            let mut t = Pagination {
                pager_size: 10,
                path: "page".into(),
                ..Default::default()
            };
            let r = Decoder::new(decoder_config(cfg)).decode_input(input, &mut t);
            with_err(t.dump(), r.err().map(|e| e.message()))
        }
        "PageConfig" => {
            let mut t = PageConfig {
                next_prev_sort_order: "desc".into(),
                next_prev_in_section_sort_order: "desc".into(),
            };
            let r = Decoder::new(decoder_config(cfg)).decode_input(input, &mut t);
            with_err(t.dump(), r.err().map(|e| e.message()))
        }
        "SitemapConfig" => {
            let mut t = SitemapConfig {
                priority: -1.0,
                filename: "sitemap.xml".into(),
                ..Default::default()
            };
            let r = Decoder::new(decoder_config(cfg)).decode_input(input, &mut t);
            with_err(t.dump(), r.err().map(|e| e.message()))
        }
        "Minify" => {
            let mut t = default_minify_config();
            let r = Decoder::new(decoder_config(cfg)).decode_input(input, &mut t);
            with_err(t.dump(), r.err().map(|e| e.message()))
        }
        other => panic!("target {other}"),
    }
}

fn with_err(ok: J, err: Option<String>) -> J {
    let mut out = json!({ "ok": ok });
    if let Some(e) = err {
        out["err"] = str_enc(e.as_bytes());
    }
    out
}

fn run_provider(key: &str, root: &Value, merge_strategy: bool) -> J {
    let cfg = DefaultConfigProvider::new();
    cfg.set("", root.clone());
    if merge_strategy {
        cfg.set_default_merge_strategy();
    }
    match key {
        "build" => with_err(
            nh_config::common_config::decode_build_config(&cfg).dump(),
            None,
        ),
        "server" => {
            let s = nh_config::common_config::decode_server(&cfg).unwrap();
            with_err(s.dump(), None)
        }
        "security" => {
            let (c, e) = security::security_config::decode_config_partial(&cfg);
            with_err(c.dump(), e.map(|e| e.to_string()))
        }
        "services" => {
            let (c, e) = services::decode_config_partial(&cfg);
            with_err(c.dump(), e.map(|e| e.to_string()))
        }
        "privacy" => {
            let (c, e) = privacy::decode_config_partial(&cfg);
            with_err(c.dump(), e.map(|e| e.to_string()))
        }
        "sitemap" => {
            let proto = SitemapConfig {
                priority: -1.0,
                filename: "sitemap.xml".into(),
                ..Default::default()
            };
            let (c, e) = nh_config::common_config::decode_sitemap_partial(
                proto,
                &Value::map(cfg.get_string_map(key)),
            );
            with_err(c.dump(), e.map(|e| e.to_string()))
        }
        other => panic!("key {other}"),
    }
}

#[test]
fn decode_matches_go() {
    let fx = support::fixture("decode/decode.json.gz");
    assert_eq!(fx["goarch"], "arm64");
    let cases = fx["cases"].as_array().unwrap();
    let mut mismatches = Vec::new();
    let mut panics = 0;
    for (i, c) in cases.iter().enumerate() {
        let target = c["target"].as_str().unwrap();
        let input = decode(&c["in"]);
        let got = catch(|| {
            if let Some(key) = target.strip_prefix("provider:") {
                run_provider(key, &input, c["mergeStrategy"].as_bool().unwrap())
            } else {
                let prefill = c.get("prefill").map(decode);
                run_target(target, c["cfg"].as_str().unwrap(), prefill.as_ref(), &input)
            }
        });
        let want = if let Some(p) = c.get("panic") {
            // Go panics inside reflect; the port returns the panic text as the error.
            panics += 1;
            json!({"err": p})
        } else {
            let mut w = json!({"ok": c["ok"]});
            if let Some(e) = c.get("err") {
                w["err"] = e.clone();
            }
            w
        };
        let got = match got {
            Ok(g) if c.get("panic").is_some() => {
                json!({"err": g.get("err").cloned().unwrap_or(J::Null)})
            }
            Ok(g) => g,
            Err(p) => json!({"panic": p}),
        };
        if got != want {
            mismatches.push(format!(
                "#{i} {target} cfg={} in={}:\n  got:  {got}\n  want: {want}",
                c["cfg"], c["in"]
            ));
        }
    }
    eprintln!(
        "decode: {} cases ({panics} Go panics), {} mismatches",
        cases.len(),
        mismatches.len()
    );
    assert!(
        mismatches.is_empty(),
        "{}",
        mismatches[..mismatches.len().min(15)].join("\n")
    );
}
