//! Go's `json.Marshal(*allconfig.Config)` (the struct walk of `encoding/json` over the config
//! types, with their `json` tags and `MarshalJSON` methods), `neohugo config` (Go
//! `commands/config.go`: `parser.ReplacingJSONMarshaller` through an indenting encoder) and
//! `neohugo config mounts` (`configModMounts.MarshalJSON`).
//!
//! NEW: Go reflects over the structs; the port lists the fields of every config type in Go's
//! declaration order (`json:"-"` fields and unexported fields left out).

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_json::{JsonField, JsonStruct};
use go_value::{HostCtx, Kind, Map, MapType, Object, SliceType, Value};
use nh_common::{Error, Result};
use nh_config::common_config::{BuildConfig, CommonDirs};
use nh_config::security::whitelist::Whitelist;
use nh_hugofs::modules::config::{Config as ModuleConfig, Mount};
use nh_hugofs::modules::module::Module;

use crate::allconfig::{Config, Configs, GoSlice, UglyUrls};

fn st(type_name: &str, fields: Vec<JsonField>) -> Value {
    Value::object(JsonStruct::new(type_name, fields))
}

fn f(name: &str, v: Value) -> JsonField {
    JsonField::new(name, v)
}

fn s(v: &str) -> Value {
    Value::string(v)
}

fn b(v: bool) -> Value {
    Value::Bool(v)
}

fn i(v: i64) -> Value {
    Value::int(v)
}

/// A `[]string` that is nil when empty.
fn strs_nil(v: &[String]) -> Value {
    if v.is_empty() {
        Value::TypedNil(Arc::from("[]string"))
    } else {
        Value::string_list(v.iter().map(|x| x.as_str()))
    }
}

/// A non-nil `[]string`.
fn strs(v: &[String]) -> Value {
    Value::string_list(v.iter().map(|x| x.as_str()))
}

fn go_slice(v: &GoSlice<String>) -> Value {
    match &v.0 {
        None => Value::TypedNil(Arc::from("[]string")),
        Some(v) => strs(v),
    }
}

fn opt_map(m: &Option<Map>, nil_type: &str) -> Value {
    match m {
        None => Value::TypedNil(Arc::from(nil_type)),
        Some(m) => Value::map(m.clone()),
    }
}

fn string_map(m: &BTreeMap<String, String>) -> Value {
    let mut out = Map::new(MapType::StringString);
    for (k, v) in m {
        out.insert(k.as_str(), s(v));
    }
    Value::map(out)
}

fn ns_source<S, C>(ns: &Option<Arc<nh_config::namespace::ConfigNamespace<S, C>>>) -> Value {
    match ns {
        None => Value::TypedNil(Arc::from("*config.ConfigNamespace")),
        Some(ns) => ns.source_structure.clone(),
    }
}

/// The cascade source structure is Go's `[]page.PageMatcherParamsConfig` (structs); nh-page
/// gives each as a map of its fields.
fn cascade_source(v: Value) -> Value {
    let Value::List(l) = &v else {
        return v;
    };
    let field = |m: &Map, k: &str| m.get(k.as_bytes()).cloned().unwrap_or(Value::Invalid);
    let items: Vec<Value> = l
        .items
        .iter()
        .map(|item| match item {
            Value::Map(m) => {
                let target = match field(m, "Target") {
                    Value::Map(t) => st(
                        "page.PageMatcher",
                        vec![
                            f("Path", field(&t, "Path")),
                            f("Kind", field(&t, "Kind")),
                            f("Lang", field(&t, "Lang")),
                            f("Environment", field(&t, "Environment")),
                        ],
                    ),
                    t => t,
                };
                st(
                    "page.PageMatcherParamsConfig",
                    vec![
                        f("Params", field(m, "Params")),
                        f("Fields", field(m, "Fields")),
                        f("Target", target),
                    ],
                )
            }
            other => other.clone(),
        })
        .collect();
    Value::list(l.ty.clone(), items)
}

/// `time.Duration` (an int64 of nanoseconds).
fn duration(d: go_time::Duration) -> Value {
    Value::Int(d.0, go_value::IntKind::Int64)
}

fn common_dirs(d: &CommonDirs) -> Vec<JsonField> {
    vec![
        f("ThemesDir", s(&d.themes_dir)),
        f("PublishDir", s(&d.publish_dir)),
        f("ResourceDir", s(&d.resource_dir)),
        f("WorkingDir", s(&d.working_dir)),
        f("CacheDir", s(&d.cache_dir)),
        f("ContentDir", s(&d.content_dir)),
        f("DataDir", s(&d.data_dir)),
        f("LayoutDir", s(&d.layout_dir)),
        f("I18nDir", s(&d.i18n_dir)),
        f("ArcheTypeDir", s(&d.arche_type_dir)),
        f("AssetDir", s(&d.asset_dir)),
    ]
}

fn build(c: &BuildConfig) -> Value {
    let bs = &c.build_stats;
    let busters: Vec<Value> = c
        .cache_busters
        .iter()
        .map(|cb| {
            st(
                "config.CacheBuster",
                vec![f("Source", s(&cb.source)), f("Target", s(&cb.target))],
            )
        })
        .collect();
    st(
        "config.BuildConfig",
        vec![
            f("UseResourceCacheWhen", s(&c.use_resource_cache_when)),
            f(
                "BuildStats",
                st(
                    "config.BuildStats",
                    vec![
                        f("Enable", b(bs.enable)),
                        f("DisableTags", b(bs.disable_tags)),
                        f("DisableClasses", b(bs.disable_classes)),
                        f("DisableIDs", b(bs.disable_ids)),
                    ],
                ),
            ),
            f("NoJSConfigInAssets", b(c.no_js_config_in_assets)),
            f(
                "CacheBusters",
                if busters.is_empty() {
                    Value::TypedNil(Arc::from("[]config.CacheBuster"))
                } else {
                    Value::list(SliceType::Named(Arc::from("[]config.CacheBuster")), busters)
                },
            ),
        ],
    )
}

fn caches(c: &nh_helpers::cache::filecache::filecache_config::Configs) -> Value {
    let mut m = Map::new(MapType::Named(Arc::from("filecache.Configs")));
    for (k, v) in c {
        m.insert(
            k.as_str(),
            st(
                "filecache.FileCacheConfig",
                vec![f("MaxAge", duration(v.max_age)), f("Dir", s(&v.dir))],
            ),
        );
    }
    Value::map(m)
}

fn glob_matcher(
    g: &nh_helpers::cache::httpcache::httpcache::GlobMatcher,
    includes_non_nil: bool,
) -> Value {
    st(
        "httpcache.GlobMatcher",
        vec![
            f("Excludes", strs_nil(&g.excludes)),
            f(
                "Includes",
                if includes_non_nil {
                    strs(&g.includes)
                } else {
                    strs_nil(&g.includes)
                },
            ),
        ],
    )
}

fn http_cache(c: &Config) -> Value {
    let h = &c.http_cache;
    let polls: Vec<Value> = h
        .polls
        .iter()
        .map(|p| {
            // PollConfig.MarshalJSON: the durations as strings.
            st(
                "httpcache.PollConfig",
                vec![
                    f("Low", s(&p.low.string())),
                    f("High", s(&p.high.string())),
                    f("For", glob_matcher(&p.for_, false)),
                    f("Disable", b(p.disable)),
                ],
            )
        })
        .collect();
    st(
        "httpcache.Config",
        vec![
            f(
                "Cache",
                st(
                    "httpcache.Cache",
                    // With ignoreCache the includes are set to an empty (non-nil) slice.
                    vec![f("For", glob_matcher(&h.cache.for_, c.root.ignore_cache))],
                ),
            ),
            f(
                "Polls",
                if polls.is_empty() {
                    Value::TypedNil(Arc::from("[]httpcache.PollConfig"))
                } else {
                    Value::list(SliceType::Named(Arc::from("[]httpcache.PollConfig")), polls)
                },
            ),
        ],
    )
}

fn markup(m: &nh_markup::markup_config::Config) -> Value {
    let h = &m.highlight;
    let toc = &m.table_of_contents;
    let g = &m.goldmark;
    let e = &g.extensions;
    let t = &e.typographer;
    let enable = |name: &str, v: bool| st(name, vec![f("Enable", b(v))]);
    let delims = |v: &Vec<Vec<String>>| {
        Value::list(
            SliceType::Named(Arc::from("[][]string")),
            v.iter().map(|x| strs(x)).collect(),
        )
    };
    let hook = |name: &str, h: &nh_markup::goldmark::goldmark_config::RenderHook| {
        st(
            name,
            vec![
                f(
                    "EnableDefault",
                    match h.enable_default.as_deref() {
                        None => Value::TypedNil(Arc::from("*bool")),
                        Some(v) => b(*v),
                    },
                ),
                f("UseEmbedded", s(&h.use_embedded)),
            ],
        )
    };
    let a = &m.asciidoc_ext;
    st(
        "markup_config.Config",
        vec![
            f("DefaultMarkdownHandler", s(&m.default_markdown_handler)),
            f(
                "Highlight",
                st(
                    "highlight.Config",
                    vec![
                        f("Style", s(&h.style)),
                        f("CodeFences", b(h.code_fences)),
                        f("WrapperClass", s(&h.wrapper_class)),
                        f("NoClasses", b(h.no_classes)),
                        f("LineNos", b(h.line_nos)),
                        f("LineNumbersInTable", b(h.line_numbers_in_table)),
                        f("AnchorLineNos", b(h.anchor_line_nos)),
                        f("LineAnchors", s(&h.line_anchors)),
                        f("LineNoStart", i(h.line_no_start)),
                        f("Hl_Lines", s(&h.hl_lines)),
                        f("Hl_inline", b(h.hl_inline)),
                        f("TabWidth", i(h.tab_width)),
                        f("GuessSyntax", b(h.guess_syntax)),
                    ],
                ),
            ),
            f(
                "TableOfContents",
                st(
                    "tableofcontents.Config",
                    vec![
                        f("StartLevel", i(toc.start_level)),
                        f("EndLevel", i(toc.end_level)),
                        f("Ordered", b(toc.ordered)),
                    ],
                ),
            ),
            f(
                "Goldmark",
                st(
                    "goldmark_config.Config",
                    vec![
                        f(
                            "Renderer",
                            st(
                                "goldmark_config.Renderer",
                                vec![
                                    f("HardWraps", b(g.renderer.hard_wraps)),
                                    f("XHTML", b(g.renderer.xhtml)),
                                    f("Unsafe", b(g.renderer.unsafe_)),
                                ],
                            ),
                        ),
                        f(
                            "Parser",
                            st(
                                "goldmark_config.Parser",
                                vec![
                                    f("AutoHeadingID", b(g.parser.auto_heading_id)),
                                    f("AutoDefinitionTermID", b(g.parser.auto_definition_term_id)),
                                    f("AutoIDType", s(&g.parser.auto_id_type)),
                                    f(
                                        "Attribute",
                                        st(
                                            "goldmark_config.ParserAttribute",
                                            vec![
                                                f("Title", b(g.parser.attribute.title)),
                                                f("Block", b(g.parser.attribute.block)),
                                            ],
                                        ),
                                    ),
                                    f(
                                        "WrapStandAloneImageWithinParagraph",
                                        b(g.parser.wrap_stand_alone_image_within_paragraph),
                                    ),
                                ],
                            ),
                        ),
                        f(
                            "Extensions",
                            st(
                                "goldmark_config.Extensions",
                                vec![
                                    f(
                                        "Typographer",
                                        st(
                                            "goldmark_config.Typographer",
                                            vec![
                                                f("Disable", b(t.disable)),
                                                f("LeftSingleQuote", s(&t.left_single_quote)),
                                                f("RightSingleQuote", s(&t.right_single_quote)),
                                                f("LeftDoubleQuote", s(&t.left_double_quote)),
                                                f("RightDoubleQuote", s(&t.right_double_quote)),
                                                f("EnDash", s(&t.en_dash)),
                                                f("EmDash", s(&t.em_dash)),
                                                f("Ellipsis", s(&t.ellipsis)),
                                                f("LeftAngleQuote", s(&t.left_angle_quote)),
                                                f("RightAngleQuote", s(&t.right_angle_quote)),
                                                f("Apostrophe", s(&t.apostrophe)),
                                            ],
                                        ),
                                    ),
                                    f("Footnote", b(e.footnote)),
                                    f("DefinitionList", b(e.definition_list)),
                                    f(
                                        "Extras",
                                        st(
                                            "goldmark_config.Extras",
                                            vec![
                                                f(
                                                    "Delete",
                                                    enable(
                                                        "goldmark_config.Delete",
                                                        e.extras.delete.enable,
                                                    ),
                                                ),
                                                f(
                                                    "Insert",
                                                    enable(
                                                        "goldmark_config.Insert",
                                                        e.extras.insert.enable,
                                                    ),
                                                ),
                                                f(
                                                    "Mark",
                                                    enable(
                                                        "goldmark_config.Mark",
                                                        e.extras.mark.enable,
                                                    ),
                                                ),
                                                f(
                                                    "Subscript",
                                                    enable(
                                                        "goldmark_config.Subscript",
                                                        e.extras.subscript.enable,
                                                    ),
                                                ),
                                                f(
                                                    "Superscript",
                                                    enable(
                                                        "goldmark_config.Superscript",
                                                        e.extras.superscript.enable,
                                                    ),
                                                ),
                                            ],
                                        ),
                                    ),
                                    f(
                                        "Passthrough",
                                        st(
                                            "goldmark_config.Passthrough",
                                            vec![
                                                f("Enable", b(e.passthrough.enable)),
                                                f(
                                                    "Delimiters",
                                                    st(
                                                        "goldmark_config.DelimitersConfig",
                                                        vec![
                                                            f(
                                                                "Inline",
                                                                delims(
                                                                    &e.passthrough
                                                                        .delimiters
                                                                        .inline,
                                                                ),
                                                            ),
                                                            f(
                                                                "Block",
                                                                delims(
                                                                    &e.passthrough.delimiters.block,
                                                                ),
                                                            ),
                                                        ],
                                                    ),
                                                ),
                                            ],
                                        ),
                                    ),
                                    f("Table", b(e.table)),
                                    f("Strikethrough", b(e.strikethrough)),
                                    f("Linkify", b(e.linkify)),
                                    f("LinkifyProtocol", s(&e.linkify_protocol)),
                                    f("TaskList", b(e.task_list)),
                                    f(
                                        "CJK",
                                        st(
                                            "goldmark_config.CJK",
                                            vec![
                                                f("Enable", b(e.cjk.enable)),
                                                f(
                                                    "EastAsianLineBreaks",
                                                    b(e.cjk.east_asian_line_breaks),
                                                ),
                                                f(
                                                    "EastAsianLineBreaksStyle",
                                                    s(&e.cjk.east_asian_line_breaks_style),
                                                ),
                                                f("EscapedSpace", b(e.cjk.escaped_space)),
                                            ],
                                        ),
                                    ),
                                ],
                            ),
                        ),
                        f("DuplicateResourceFiles", b(g.duplicate_resource_files)),
                        f(
                            "RenderHooks",
                            st(
                                "goldmark_config.RenderHooks",
                                vec![
                                    f(
                                        "Image",
                                        hook(
                                            "goldmark_config.ImageRenderHook",
                                            &g.render_hooks.image,
                                        ),
                                    ),
                                    f(
                                        "Link",
                                        hook(
                                            "goldmark_config.LinkRenderHook",
                                            &g.render_hooks.link,
                                        ),
                                    ),
                                ],
                            ),
                        ),
                    ],
                ),
            ),
            f(
                "AsciidocExt",
                st(
                    "asciidocext_config.Config",
                    vec![
                        f("Backend", s(&a.backend)),
                        f("Extensions", strs(&a.extensions)),
                        f("Attributes", string_map(&a.attributes)),
                        f("NoHeaderOrFooter", b(a.no_header_or_footer)),
                        f("SafeMode", s(&a.safe_mode)),
                        f("SectionNumbers", b(a.section_numbers)),
                        f("Verbose", b(a.verbose)),
                        f("Trace", b(a.trace)),
                        f("FailureLevel", s(&a.failure_level)),
                        f("WorkingFolderCurrent", b(a.working_folder_current)),
                        f("PreserveTOC", b(a.preserve_toc)),
                    ],
                ),
            ),
        ],
    )
}

fn outputs(m: &BTreeMap<String, Vec<String>>) -> Value {
    let mut out = Map::new(MapType::Named(Arc::from("map[string][]string")));
    for (k, v) in m {
        out.insert(k.as_str(), strs(v));
    }
    Value::map(out)
}

fn deployment(d: &crate::deployconfig::DeployConfig) -> Value {
    let targets = d.targets.0.as_ref().map(|t| {
        t.iter()
            .map(|t| {
                st(
                    "deployconfig.Target",
                    vec![
                        f("Name", s(&t.name)),
                        f("URL", s(&t.url)),
                        f(
                            "CloudFrontDistributionID",
                            s(&t.cloud_front_distribution_id),
                        ),
                        f("GoogleCloudCDNOrigin", s(&t.google_cloud_cdn_origin)),
                        f("Include", s(&t.include)),
                        f("Exclude", s(&t.exclude)),
                        f("StripIndexHTML", b(t.strip_index_html)),
                    ],
                )
            })
            .collect::<Vec<_>>()
    });
    let matchers = d.matchers.0.as_ref().map(|m| {
        m.iter()
            .map(|m| {
                st(
                    "deployconfig.Matcher",
                    vec![
                        f("Pattern", s(&m.pattern)),
                        f("CacheControl", s(&m.cache_control)),
                        f("ContentEncoding", s(&m.content_encoding)),
                        f("ContentType", s(&m.content_type)),
                        f("Gzip", b(m.gzip)),
                        f("Force", b(m.force)),
                    ],
                )
            })
            .collect::<Vec<_>>()
    });
    let list = |v: Option<Vec<Value>>, ty: &str| match v {
        None => Value::TypedNil(Arc::from(ty)),
        Some(v) => Value::list(SliceType::Named(Arc::from(ty)), v),
    };
    st(
        "deployconfig.DeployConfig",
        vec![
            f("Targets", list(targets, "[]*deployconfig.Target")),
            f("Matchers", list(matchers, "[]*deployconfig.Matcher")),
            f("Order", go_slice(&d.order)),
            f("Target", s(&d.target)),
            f("Confirm", b(d.confirm)),
            f("DryRun", b(d.dry_run)),
            f("Force", b(d.force)),
            f("InvalidateCDN", b(d.invalidate_cdn)),
            f("MaxDeletes", i(d.max_deletes)),
            f("Workers", i(d.workers)),
        ],
    )
}

fn mount(m: &Mount, raw: (Value, Value)) -> Value {
    st(
        "modules.Mount",
        vec![
            f("Source", s(&m.source)),
            f("Target", s(&m.target)),
            f("Lang", s(&m.lang)),
            f("IncludeFiles", raw.0).interface_typed(),
            f("ExcludeFiles", raw.1).interface_typed(),
            f("DisableWatch", b(m.disable_watch)),
        ],
    )
}

/// `json.Marshal(modules.Config)`.
pub fn module_config_value(c: &ModuleConfig) -> Value {
    let mounts: Vec<Value> = c
        .mounts
        .iter()
        .enumerate()
        .map(|(i, m)| mount(m, c.mount_raw_files(i)))
        .collect();
    let imports: Vec<Value> = c
        .imports
        .iter()
        .map(|imp| {
            let mounts: Vec<Value> = imp
                .mounts
                .iter()
                .map(|m| {
                    mount(
                        m,
                        (
                            nh_hugofs::modules::config::raw_files(&m.include_files),
                            nh_hugofs::modules::config::raw_files(&m.exclude_files),
                        ),
                    )
                })
                .collect();
            st(
                "modules.Import",
                vec![
                    f("Path", s(&imp.path)),
                    f("IgnoreConfig", b(imp.ignore_config)),
                    f("IgnoreImports", b(imp.ignore_imports)),
                    f("NoMounts", b(imp.no_mounts)),
                    f("NoVendor", b(imp.no_vendor)),
                    f("Disable", b(imp.disable)),
                    f(
                        "Mounts",
                        if mounts.is_empty() {
                            Value::TypedNil(Arc::from("[]modules.Mount"))
                        } else {
                            Value::list(SliceType::Named(Arc::from("[]modules.Mount")), mounts)
                        },
                    ),
                ],
            )
        })
        .collect();
    st(
        "modules.Config",
        vec![
            f(
                "Mounts",
                if mounts.is_empty() {
                    Value::TypedNil(Arc::from("[]modules.Mount"))
                } else {
                    Value::list(SliceType::Named(Arc::from("[]modules.Mount")), mounts)
                },
            ),
            f(
                "Imports",
                if imports.is_empty() {
                    Value::TypedNil(Arc::from("[]modules.Import"))
                } else {
                    Value::list(SliceType::Named(Arc::from("[]modules.Import")), imports)
                },
            ),
            f("Params", opt_map(&c.params, "map[string]interface {}")),
            f(
                "HugoVersion",
                st(
                    "modules.HugoVersion",
                    vec![
                        f("Min", s(&c.hugo_version_min)),
                        f("Max", s(&c.hugo_version_max)),
                    ],
                ),
            ),
            f("NoVendor", s(&c.no_vendor)),
            f("VendorClosest", b(c.vendor_closest)),
            f("Replacements", strs_nil(&c.replacements)),
            f("Proxy", s(&c.proxy)),
            f("NoProxy", s(&c.no_proxy)),
            f("Private", s(&c.private)),
            f("Auth", s(&c.auth)),
            f("Workspace", s(&c.workspace)),
        ],
    )
}

fn frontmatter(c: &nh_page::pagemeta::page_frontmatter::FrontmatterConfig) -> Value {
    st(
        "pagemeta.FrontmatterConfig",
        vec![
            f("Date", strs(&c.date)),
            f("Lastmod", strs(&c.lastmod)),
            f("PublishDate", strs(&c.publish_date)),
            f("ExpiryDate", strs(&c.expiry_date)),
        ],
    )
}

fn minify(c: &nh_transform::minifiers::config::MinifyConfig) -> Value {
    let t = &c.tdewolff;
    st(
        "minifiers.MinifyConfig",
        vec![
            f("MinifyOutput", b(c.minify_output)),
            f("DisableHTML", b(c.disable_html)),
            f("DisableCSS", b(c.disable_css)),
            f("DisableJS", b(c.disable_js)),
            f("DisableJSON", b(c.disable_json)),
            f("DisableSVG", b(c.disable_svg)),
            f("DisableXML", b(c.disable_xml)),
            f(
                "Tdewolff",
                st(
                    "minifiers.TdewolffConfig",
                    vec![
                        f(
                            "HTML",
                            st(
                                "html.Minifier",
                                vec![
                                    f("KeepComments", b(t.html.keep_comments)),
                                    f(
                                        "KeepConditionalComments",
                                        b(t.html.keep_conditional_comments),
                                    ),
                                    f("KeepSpecialComments", b(t.html.keep_special_comments)),
                                    f("KeepDefaultAttrVals", b(t.html.keep_default_attr_vals)),
                                    f("KeepDocumentTags", b(t.html.keep_document_tags)),
                                    f("KeepEndTags", b(t.html.keep_end_tags)),
                                    f("KeepQuotes", b(t.html.keep_quotes)),
                                    f("KeepWhitespace", b(t.html.keep_whitespace)),
                                    f("TemplateDelims", strs(&t.html.template_delims)),
                                ],
                            ),
                        ),
                        f(
                            "CSS",
                            st(
                                "css.Minifier",
                                vec![
                                    f("KeepCSS2", b(t.css.keep_css2)),
                                    f("Precision", i(t.css.precision)),
                                    f("Inline", b(t.css.inline)),
                                ],
                            ),
                        ),
                        f(
                            "JS",
                            st(
                                "js.Minifier",
                                vec![
                                    f("Precision", i(t.js.precision)),
                                    f("KeepVarNames", b(t.js.keep_var_names)),
                                    f("Version", i(t.js.version)),
                                ],
                            ),
                        ),
                        f(
                            "JSON",
                            st(
                                "json.Minifier",
                                vec![
                                    f("Precision", i(t.json.precision)),
                                    f("KeepNumbers", b(t.json.keep_numbers)),
                                ],
                            ),
                        ),
                        f(
                            "SVG",
                            st(
                                "svg.Minifier",
                                vec![
                                    f("KeepComments", b(t.svg.keep_comments)),
                                    f("Precision", i(t.svg.precision)),
                                    f("Inline", b(t.svg.inline)),
                                ],
                            ),
                        ),
                        f(
                            "XML",
                            st(
                                "xml.Minifier",
                                vec![f("KeepWhitespace", b(t.xml.keep_whitespace))],
                            ),
                        ),
                    ],
                ),
            ),
        ],
    )
}

fn permalinks(m: &BTreeMap<String, BTreeMap<String, String>>) -> Value {
    let mut out = Map::new(MapType::Named(Arc::from("map[string]map[string]string")));
    for (k, v) in m {
        out.insert(k.as_str(), string_map(v));
    }
    Value::map(out)
}

fn related(c: &nh_page::related::Config) -> Value {
    let indices: Vec<Value> = c
        .indices
        .iter()
        .map(|x| {
            st(
                "related.IndexConfig",
                vec![
                    f("Name", s(&x.name)),
                    f("Type", s(&x.type_)),
                    f("ApplyFilter", b(x.apply_filter)),
                    f("Pattern", s(&x.pattern)),
                    f("Weight", i(x.weight)),
                    f("CardinalityThreshold", i(x.cardinality_threshold)),
                    f("ToLower", b(x.to_lower)),
                ],
            )
        })
        .collect();
    st(
        "related.Config",
        vec![
            f("Threshold", i(c.threshold)),
            f("IncludeNewer", b(c.include_newer)),
            f("ToLower", b(c.to_lower)),
            f(
                "Indices",
                if indices.is_empty() {
                    Value::TypedNil(Arc::from("related.IndicesConfig"))
                } else {
                    Value::list(
                        SliceType::Named(Arc::from("related.IndicesConfig")),
                        indices,
                    )
                },
            ),
        ],
    )
}

fn server(c: &nh_config::common_config::Server) -> Value {
    let headers: Vec<Value> = c
        .headers
        .iter()
        .map(|h| {
            st(
                "config.Headers",
                vec![
                    f("For", s(&h.for_)),
                    f(
                        "Values",
                        if h.values.is_empty() {
                            Value::TypedNil(Arc::from("map[string]interface {}"))
                        } else {
                            Value::map(h.values.clone())
                        },
                    ),
                ],
            )
        })
        .collect();
    let redirects: Vec<Value> = c
        .redirects
        .iter()
        .map(|r| {
            st(
                "config.Redirect",
                vec![
                    f("From", s(&r.from)),
                    f("FromRe", s(&r.from_re)),
                    f("To", s(&r.to)),
                    f(
                        "FromHeaders",
                        if r.from_headers.is_empty() {
                            Value::TypedNil(Arc::from("map[string]string"))
                        } else {
                            string_map(&r.from_headers)
                        },
                    ),
                    f("Status", i(r.status)),
                    f("Force", b(r.force)),
                ],
            )
        })
        .collect();
    let list = |v: Vec<Value>, ty: &str| {
        if v.is_empty() {
            Value::TypedNil(Arc::from(ty))
        } else {
            Value::list(SliceType::Named(Arc::from(ty)), v)
        }
    };
    st(
        "config.Server",
        vec![
            f("Headers", list(headers, "[]config.Headers")),
            f("Redirects", list(redirects, "[]config.Redirect")),
        ],
    )
}

fn privacy(c: &nh_config::privacy::Config) -> Value {
    st(
        "privacy.Config",
        vec![
            f(
                "Disqus",
                st(
                    "privacy.Disqus",
                    vec![f("Disable", b(c.disqus.service.disable))],
                ),
            ),
            f(
                "GoogleAnalytics",
                st(
                    "privacy.GoogleAnalytics",
                    vec![
                        f("Disable", b(c.google_analytics.service.disable)),
                        f(
                            "RespectDoNotTrack",
                            b(c.google_analytics.respect_do_not_track),
                        ),
                    ],
                ),
            ),
            f(
                "Instagram",
                st(
                    "privacy.Instagram",
                    vec![
                        f("Disable", b(c.instagram.service.disable)),
                        f("Simple", b(c.instagram.simple)),
                    ],
                ),
            ),
            f(
                "Twitter",
                st(
                    "privacy.Twitter",
                    vec![
                        f("Disable", b(c.twitter.service.disable)),
                        f("EnableDNT", b(c.twitter.enable_dnt)),
                        f("Simple", b(c.twitter.simple)),
                    ],
                ),
            ),
            f(
                "Vimeo",
                st(
                    "privacy.Vimeo",
                    vec![
                        f("Disable", b(c.vimeo.service.disable)),
                        f("EnableDNT", b(c.vimeo.enable_dnt)),
                        f("Simple", b(c.vimeo.simple)),
                    ],
                ),
            ),
            f(
                "YouTube",
                st(
                    "privacy.YouTube",
                    vec![
                        f("Disable", b(c.youtube.service.disable)),
                        f("PrivacyEnhanced", b(c.youtube.privacy_enhanced)),
                    ],
                ),
            ),
            f(
                "X",
                st(
                    "privacy.X",
                    vec![
                        f("Disable", b(c.x.service.disable)),
                        f("EnableDNT", b(c.x.enable_dnt)),
                        f("Simple", b(c.x.simple)),
                    ],
                ),
            ),
        ],
    )
}

/// `security.Whitelist.MarshalJSON`.
fn whitelist(w: &Whitelist) -> Value {
    if w.accept_none() {
        return s("none");
    }
    match w.patterns_strings() {
        None => Value::TypedNil(Arc::from("[]string")),
        Some(p) => strs(p),
    }
}

fn security(c: &nh_config::security::security_config::Config) -> Value {
    st(
        "security.Config",
        vec![
            f(
                "exec",
                st(
                    "security.Exec",
                    vec![
                        f("allow", whitelist(&c.exec.allow)),
                        f("osEnv", whitelist(&c.exec.os_env)),
                    ],
                ),
            ),
            f(
                "funcs",
                st(
                    "security.Funcs",
                    vec![f("getenv", whitelist(&c.funcs.getenv))],
                ),
            ),
            f(
                "http",
                st(
                    "security.HTTP",
                    vec![
                        f("urls", whitelist(&c.http.urls)),
                        f("methods", whitelist(&c.http.methods)),
                        f("mediaTypes", whitelist(&c.http.media_types)),
                    ],
                ),
            ),
            f("enableInlineShortcodes", b(c.enable_inline_shortcodes)),
        ],
    )
}

fn services(c: &nh_config::services::Config) -> Value {
    st(
        "services.Config",
        vec![
            f(
                "Disqus",
                st(
                    "services.Disqus",
                    vec![f("Shortname", s(&c.disqus.shortname))],
                ),
            ),
            f(
                "GoogleAnalytics",
                st(
                    "services.GoogleAnalytics",
                    vec![f("ID", s(&c.google_analytics.id))],
                ),
            ),
            f(
                "Instagram",
                st(
                    "services.Instagram",
                    vec![
                        f("DisableInlineCSS", b(c.instagram.disable_inline_css)),
                        f("AccessToken", s(&c.instagram.access_token)),
                    ],
                ),
            ),
            f(
                "Twitter",
                st(
                    "services.Twitter",
                    vec![f("DisableInlineCSS", b(c.twitter.disable_inline_css))],
                ),
            ),
            f(
                "X",
                st(
                    "services.X",
                    vec![f("DisableInlineCSS", b(c.x.disable_inline_css))],
                ),
            ),
            f("RSS", st("services.RSS", vec![f("Limit", i(c.rss.limit))])),
        ],
    )
}

fn languages(c: &Config) -> Value {
    let mut out = Map::new(MapType::Named(Arc::from("map[string]langs.LanguageConfig")));
    for (k, l) in c.languages.get() {
        out.insert(
            k.as_str(),
            st(
                "langs.LanguageConfig",
                vec![
                    f("LanguageName", s(&l.language_name)),
                    f("LanguageCode", s(&l.language_code)),
                    f("Title", s(&l.title)),
                    f("LanguageDirection", s(&l.language_direction)),
                    f("Weight", i(l.weight)),
                    f("Disabled", b(l.disabled)),
                ],
            ),
        );
    }
    Value::map(out)
}

fn ugly_urls(u: &UglyUrls) -> Value {
    match u {
        UglyUrls::Nil => Value::TypedNil(Arc::from("map[string]bool")),
        UglyUrls::Bool(v) => b(*v),
        UglyUrls::Sections(m) => {
            let mut out = Map::new(MapType::Named(Arc::from("map[string]bool")));
            for (k, v) in m {
                out.insert(k.as_str(), b(*v));
            }
            Value::map(out)
        }
    }
}

/// The Go value `json.Marshal(*allconfig.Config)` encodes (a `JsonStruct` tree).
pub fn config_value(c: &Config) -> Value {
    let r = &c.root;
    let mut fields = vec![
        f("BaseURL", s(&r.base_url)),
        f("BuildDrafts", b(r.build_drafts)),
        f("BuildExpired", b(r.build_expired)),
        f("BuildFuture", b(r.build_future)),
        f("Copyright", s(&r.copyright)),
        f("DefaultContentLanguage", s(&r.default_content_language)),
        f(
            "DefaultContentLanguageInSubdir",
            b(r.default_content_language_in_subdir),
        ),
        f("DefaultOutputFormat", s(&r.default_output_format)),
        f(
            "DisableDefaultLanguageRedirect",
            b(r.disable_default_language_redirect),
        ),
        f("DisableAliases", b(r.disable_aliases)),
        f("DisablePathToLower", b(r.disable_path_to_lower)),
        f("DisableKinds", go_slice(&r.disable_kinds)),
        f("DisableLanguages", go_slice(&r.disable_languages)),
        f("RenderSegments", go_slice(&r.render_segments)),
        f(
            "DisableHugoGeneratorInject",
            b(r.disable_hugo_generator_inject),
        ),
        f("DisableLiveReload", b(r.disable_live_reload)),
        f("EnableEmoji", b(r.enable_emoji)),
        f("MainSections", go_slice(&r.main_sections)),
        f("EnableRobotsTXT", b(r.enable_robots_txt)),
        f("EnableGitInfo", b(r.enable_git_info)),
        f("TemplateMetrics", b(r.template_metrics)),
        f("TemplateMetricsHints", b(r.template_metrics_hints)),
        f("NoBuildLock", b(r.no_build_lock)),
        f("IgnoreLogs", go_slice(&r.ignore_logs)),
        f("IgnoreFiles", go_slice(&r.ignore_files)),
        f("IgnoreCache", b(r.ignore_cache)),
        f(
            "EnableMissingTranslationPlaceholders",
            b(r.enable_missing_translation_placeholders),
        ),
        f("PanicOnWarning", b(r.panic_on_warning)),
        f("Environment", s(&r.environment)),
        f("LanguageCode", s(&r.language_code)),
        f("HasCJKLanguage", b(r.has_cjk_language)),
        f("Paginate", i(r.paginate)),
        f("PaginatePath", s(&r.paginate_path)),
        f("PluralizeListTitles", b(r.pluralize_list_titles)),
        f("CapitalizeListTitles", b(r.capitalize_list_titles)),
        f("CanonifyURLs", b(r.canonify_urls)),
        f("RelativeURLs", b(r.relative_urls)),
        f("RemovePathAccents", b(r.remove_path_accents)),
        f("PrintUnusedTemplates", b(r.print_unused_templates)),
        f("PrintI18nWarnings", b(r.print_i18n_warnings)),
        f("PrintPathWarnings", b(r.print_path_warnings)),
        f("RefLinksNotFoundURL", s(&r.ref_links_not_found_url)),
        f("RefLinksErrorLevel", s(&r.ref_links_error_level)),
        f("SectionPagesMenu", s(&r.section_pages_menu)),
        f("SummaryLength", i(r.summary_length)),
        f("Title", s(&r.title)),
        f("Theme", go_slice(&r.theme)),
        f("Timeout", s(&r.timeout)),
        f("TimeZone", s(&r.time_zone)),
        f("TitleCaseStyle", s(&r.title_case_style)),
        f("NewContentEditor", s(&r.new_content_editor)),
        f("NoTimes", b(r.no_times)),
        f("NoChmod", b(r.no_chmod)),
        f("CleanDestinationDir", b(r.clean_destination_dir)),
        f("IgnoreVendorPaths", s(&r.ignore_vendor_paths)),
    ];
    fields.extend(common_dirs(&r.common_dirs));
    fields.push(f("StaticDir", go_slice(&r.static_dir)));
    for (n, d) in r.static_dir_n.iter().enumerate() {
        fields.push(f(&format!("StaticDir{n}"), go_slice(d)));
    }
    fields.extend([
        f("Author", opt_map(&c.author, "map[string]interface {}")),
        f("Social", opt_map(&c.social, "map[string]string")),
        f("Build", build(&c.build)),
        f("Caches", caches(&c.caches)),
        f("HTTPCache", http_cache(c)),
        f("Markup", markup(&c.markup)),
        f("ContentTypes", ns_source(&c.content_types)),
        f("MediaTypes", ns_source(&c.media_types)),
        f("Imaging", ns_source(&c.imaging)),
        f("OutputFormats", ns_source(&c.output_formats)),
        f("Outputs", outputs(&c.outputs)),
        f("Cascade", cascade_source(ns_source(&c.cascade))),
        f("Segments", ns_source(&c.segments)),
        f("Menus", ns_source(&c.menus)),
        f("Deployment", deployment(&c.deployment)),
        f("Module", module_config_value(&c.module)),
        f("Frontmatter", frontmatter(&c.frontmatter)),
        f("Minify", minify(&c.minify)),
        f("Permalinks", permalinks(&c.permalinks)),
        f("Taxonomies", string_map(&c.taxonomies)),
        f(
            "Sitemap",
            st(
                "config.SitemapConfig",
                vec![
                    f("ChangeFreq", s(&c.sitemap.change_freq)),
                    f("Priority", Value::float64(c.sitemap.priority)),
                    f("Filename", s(&c.sitemap.filename)),
                    f("Disable", b(c.sitemap.disable)),
                ],
            ),
        ),
        f("Related", related(&c.related)),
        f("Server", server(&c.server)),
        f(
            "Pagination",
            st(
                "config.Pagination",
                vec![
                    f("PagerSize", i(c.pagination.pager_size)),
                    f("Path", s(&c.pagination.path)),
                    f("DisableAliases", b(c.pagination.disable_aliases)),
                ],
            ),
        ),
        f(
            "Page",
            st(
                "config.PageConfig",
                vec![
                    f("NextPrevSortOrder", s(&c.page.next_prev_sort_order)),
                    f(
                        "NextPrevInSectionSortOrder",
                        s(&c.page.next_prev_in_section_sort_order),
                    ),
                ],
            ),
        ),
        f("Privacy", privacy(&c.privacy)),
        f("Security", security(&c.security)),
        f("Services", services(&c.services)),
        f("Params", Value::map((*c.params).clone())),
        f("Languages", languages(c)),
        f("UglyURLs", ugly_urls(&c.ugly_urls)).interface_typed(),
    ]);
    st("allconfig.Config", fields)
}

/// Go: `json.Marshal(config)`.
pub fn marshal_config(c: &Config) -> Result<Vec<u8>> {
    go_json::marshal(&config_value(c)).map_err(|e| Error::new(e.to_string()))
}

/// A `json.Marshaler` returning fixed bytes (Go `parser.ReplacingJSONMarshaller` as the
/// encoder sees it).
struct Marshaled(Vec<u8>);

impl Object for Marshaled {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("parser.ReplacingJSONMarshaller")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Ok(self.0.clone()))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `neohugo config --format json [--lang lang] [--printZero]` (`commands/config.go`): the
/// config of `lang` (the first of `LanguageConfigSlice` when empty) through
/// `ReplacingJSONMarshaller{KeysToLower: true, OmitEmpty: !printZero}` and an encoder with
/// `SetIndent("", "  ")` and `SetEscapeHTML(false)`.
pub fn config_dump(configs: &Configs, lang: &str, print_zero: bool) -> Result<String> {
    let conf = if !lang.is_empty() {
        configs.language_config_map.get(lang).ok_or_else(|| {
            Error::new(format!(
                "language {} not found",
                go_strconv::quote(lang.as_bytes())
            ))
        })?
    } else {
        &configs.language_config_slice[0]
    };
    let m = nh_parser::frontmatter::ReplacingJsonMarshaller {
        value: config_value(conf),
        keys_to_lower: true,
        omit_empty: !print_zero,
    };
    let bytes = m.marshal_json()?;
    let mut buf: Vec<u8> = Vec::new();
    let mut enc = go_json::Encoder::new(&mut buf);
    enc.set_indent("", "  ");
    enc.set_escape_html(false);
    enc.encode(&Value::object(Marshaled(bytes)))
        .map_err(|e| Error::new(e.to_string()))?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Go: `configModMounts.MarshalJSON` (not verbose) of one module.
pub fn module_mounts_value(m: &Module) -> Value {
    let mounts: Vec<Value> = m
        .mounts()
        .iter()
        .map(|mount| {
            st(
                "commands.configModMount",
                vec![
                    f("source", s(&mount.source)),
                    f("target", s(&mount.target)),
                    f("lang", s(&mount.lang)).omit_empty(),
                ],
            )
        })
        .collect();
    let owner_path = m.owner().map(|o| o.path().to_string()).unwrap_or_default();
    st(
        "struct",
        vec![
            f("path", s(m.path())),
            f("version", s(m.version())),
            f("time", Value::Time(m.time())),
            f("owner", s(&owner_path)),
            f("dir", s(m.dir())),
            f(
                "mounts",
                if mounts.is_empty() {
                    Value::TypedNil(Arc::from("[]commands.configModMount"))
                } else {
                    Value::list(
                        SliceType::Named(Arc::from("[]commands.configModMount")),
                        mounts,
                    )
                },
            ),
        ],
    )
}

/// Go: `neohugo config mounts` (`parser.InterfaceToConfig(&configModMounts{m}, JSON, w)` for
/// each module).
pub fn mounts_dump(configs: &Configs) -> Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    for m in configs.modules.iter() {
        nh_parser::frontmatter::interface_to_config(
            &module_mounts_value(m),
            nh_parser::metadecoders::format::Format::Json,
            &mut buf,
        )?;
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
