//! T23 acceptance: the method sets of the types templates receive, against the Go oracle
//! `tools/go-oracle/nh-hugolib/site` (`methodsets.json.gz`: Go's `reflect` method set of every
//! type, with each method's parameters, results and whether text/template can call it, and the
//! exported struct fields).
//!
//! A receiver of every type is taken from a processed and assembled site (the menus site of the
//! site oracle: pages and their wrappers, the nop page, `*hugolib.Site` and `page.Site`, pagers,
//! groups, taxonomies, menus, output formats, ...; the named slice/map types are dispatched by
//! the template store's `NamedTypeRegistry`). For every type:
//!
//! * its Go type string is the value's;
//! * every Go method is found (`has_method`, or the registry for named slices/maps) and no other
//!   name of any type's method set or fields is;
//! * a call with one argument too many (too few for a variadic method with fixed parameters)
//!   fails with Go's `wrong number of args` message (the injected `context.Context` counted as
//!   Go counts it);
//! * a method text/template cannot call (`goodFunc`: not one result or a result and an error)
//!   fails with Go's `can't call method/function %q with %d results`;
//! * the exported fields are readable (`Object::field`).
//!
//! The mismatches must be exactly the known ones ([`KNOWN`]): gaps of other crates' tables
//! and Go features the port does not have (embedded interface fields of `*pageState`).

mod support;

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use go_value::{MapType, SliceType, Value};
use nh_common::object::NamedTypeRegistry;
use nh_hugolib::hugo_sites_build::BuildCfg;
use nh_hugolib::page::{PageHandle, PageWrapper};
use nh_hugolib::site::{HugolibSiteObject, SiteHandle};
use nh_hugolib::tplapi::page_methods::PageHeadingsFiltered;
use nh_page::page::PageRef;
use nh_tpl::template::TplContext;
use serde_json::Value as J;
use support::*;

struct Env {
    reg: NamedTypeRegistry,
    ctx: TplContext,
}

impl Env {
    fn has(&self, recv: &Value, name: &str) -> bool {
        match recv.as_object() {
            Some(o) => o.has_method(name),
            None => self.reg.has_method(recv, name),
        }
    }

    fn call(&self, recv: &Value, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        let r = match recv.as_object() {
            Some(o) => o.call_method(&self.ctx, name, args),
            None => self.reg.call(&self.ctx, recv, name, args),
        };
        r.map(|r| r.map_err(|e| e.message().to_string()))
    }

    fn get(&self, recv: &Value, name: &str, args: &[Value]) -> Value {
        match self.call(recv, name, args) {
            Some(Ok(v)) => v,
            r => panic!("{name}: {r:?}"),
        }
    }
}

fn items(v: &Value) -> Vec<Value> {
    match v {
        Value::List(l) => l.items.clone(),
        _ => v
            .as_object()
            .and_then(|o| o.list())
            .unwrap_or_else(|| panic!("not a list: {v:?}")),
    }
}

fn named_list(t: &str) -> Value {
    Value::list(SliceType::Named(t.into()), Vec::new())
}

fn named_map(t: &str) -> Value {
    Value::map(go_value::Map::new(MapType::Named(t.into())))
}

fn outcome(r: &Option<Result<Value, String>>) -> String {
    match r {
        None => "not found".to_string(),
        Some(Ok(_)) => "succeeds".to_string(),
        Some(Err(e)) => format!("{e:?}"),
    }
}

/// Known mismatches. All but the first group are in tables other crates own (reported to them).
const KNOWN: &[&str] = &[
    // T23 (not supported): Go templates can read the exported embedded fields of the hugolib
    // types (`.PageMetaProvider`, `.Positioner`, ... of a page; `.Deps`, `.PathSpec`, ... of
    // `*hugolib.Site`), i.e. the internal provider values; the port exposes none of them.
    "*hugolib.Site: none of the 46 fields",
    "*hugolib.pageForRenderHooks: none of the 4 fields",
    "*hugolib.pageForShortcode: none of the 4 fields",
    "*hugolib.pageHeadingsFiltered: none of the 32 fields",
    "*hugolib.pageState: none of the 32 fields",
    "*hugolib.pageWithOrdinal: none of the 32 fields",
    "hugolib.pageWithWeight0: none of the 32 fields",
    // nh-langs: the embedded `LanguageConfig` struct itself (its fields are promoted).
    "*langs.Language: missing field LanguageConfig",
    // nh-page: the embedded `*Paginator` of `*page.Pager`.
    "*page.Pager: none of the 1 fields",
    // nh-page (page_nop.rs): the nop page's `context.Context` methods do not count the context
    // (Go: `want 1 got 2`).
    "*page.nopPage: Content arity: \"wrong number of args for Content: want 0 got 1\"",
    "*page.nopPage: ContentWithoutSummary arity: \"wrong number of args for ContentWithoutSummary: want 0 got 1\"",
    "*page.nopPage: Fragments arity: \"wrong number of args for Fragments: want 0 got 1\"",
    "*page.nopPage: FuzzyWordCount arity: \"wrong number of args for FuzzyWordCount: want 0 got 1\"",
    "*page.nopPage: HeadingsFiltered arity: \"wrong number of args for HeadingsFiltered: want 0 got 1\"",
    "*page.nopPage: Len arity: \"wrong number of args for Len: want 0 got 1\"",
    "*page.nopPage: Plain arity: \"wrong number of args for Plain: want 0 got 1\"",
    "*page.nopPage: PlainWords arity: \"wrong number of args for PlainWords: want 0 got 1\"",
    "*page.nopPage: ReadingTime arity: \"wrong number of args for ReadingTime: want 0 got 1\"",
    "*page.nopPage: RenderShortcodes arity: \"wrong number of args for RenderShortcodes: want 0 got 1\"",
    "*page.nopPage: Summary arity: \"wrong number of args for Summary: want 0 got 1\"",
    "*page.nopPage: TableOfContents arity: \"wrong number of args for TableOfContents: want 0 got 1\"",
    "*page.nopPage: Truncated arity: \"wrong number of args for Truncated: want 0 got 1\"",
    "*page.nopPage: WordCount arity: \"wrong number of args for WordCount: want 0 got 1\"",
    // nh-page (pages.rs, shared by `page.PageGroup`): the same for the context methods of
    // `page.Pages`.
    "page.PageGroup: ByLength arity: \"wrong number of args for ByLength: want 0 got 1\"",
    "page.PageGroup: GroupBy arity: \"wrong number of args for GroupBy: want at least 1 got 0\"",
    "page.PageGroup: Related arity: \"wrong number of args for Related: want 1 got 2\"",
    "page.PageGroup: RelatedIndices arity: \"wrong number of args for RelatedIndices: want at least 1 got 0\"",
    "page.Pages: ByLength arity: \"wrong number of args for ByLength: want 0 got 1\"",
    "page.Pages: GroupBy arity: \"wrong number of args for GroupBy: want at least 1 got 0\"",
    "page.Pages: Related arity: \"wrong number of args for Related: want 1 got 2\"",
    "page.Pages: RelatedIndices arity: \"wrong number of args for RelatedIndices: want at least 1 got 0\"",
    // nh-common / nh-page: methods text/template cannot call report that before the argument
    // count (Go checks the count first; only a template that is wrong twice sees it).
    "maps.Params: GetMergeStrategy arity: \"can't call method/function \\\"GetMergeStrategy\\\" with 2 results\"",
    "maps.Params: SetMergeStrategy arity: \"can't call method/function \\\"SetMergeStrategy\\\" with 0 results\"",
    "page.OrderedTaxonomyEntry: Sort arity: \"can't call method/function \\\"Sort\\\" with 0 results\"",
    "page.OrderedTaxonomyEntry: Swap arity: \"can't call method/function \\\"Swap\\\" with 0 results\"",
    "page.WeightedPages: Sort arity: \"can't call method/function \\\"Sort\\\" with 0 results\"",
    "page.WeightedPages: Swap arity: \"can't call method/function \\\"Swap\\\" with 0 results\"",
    // nh-helpers: the unsupported `*source.File` methods fail before the argument count.
    "*source.File: FileInfo arity: \"neohugo-rs: .File.FileInfo is not supported in templates\"",
    "*source.File: Open arity: \"neohugo-rs: .File.Open is not supported in templates\"",
    // nh-media: `media.Type` checks no argument counts and has no `HasSuffix`.
    "media.Type: IsHTML arity: succeeds",
    "media.Type: IsMarkdown arity: succeeds",
    "media.Type: IsText arity: succeeds",
    "media.Type: IsZero arity: succeeds",
    "media.Type: MarshalJSON arity: succeeds",
    "media.Type: String arity: succeeds",
    "media.Type: Suffixes arity: succeeds",
    "media.Type: missing method HasSuffix",
    // nh-config: `neohugo.HugoInfo` checks no argument counts.
    "neohugo.HugoInfo: Deps arity: succeeds",
    "neohugo.HugoInfo: Generator arity: succeeds",
    "neohugo.HugoInfo: IsDevelopment arity: succeeds",
    "neohugo.HugoInfo: IsMultiHost arity: succeeds",
    "neohugo.HugoInfo: IsMultihost arity: succeeds",
    "neohugo.HugoInfo: IsMultilingual arity: succeeds",
    "neohugo.HugoInfo: IsProduction arity: succeeds",
    "neohugo.HugoInfo: IsServer arity: succeeds",
    "neohugo.HugoInfo: Store arity: succeeds",
    "neohugo.HugoInfo: Version arity: succeeds",
    "neohugo.HugoInfo: WorkingDir arity: succeeds",
    // nh-page (weighted.rs): `page.WeightedPage` forwards every method of the page it holds,
    // where Go's set is the `page.Page` interface's (`Page` and `Weight` are its fields).
    "page.WeightedPage: extra method ApplyFilterToHeadings",
    "page.WeightedPage: extra method ForEeachIdentity",
    "page.WeightedPage: extra method GetDependencyManager",
    "page.WeightedPage: extra method GetDependencyManagerForScope",
    "page.WeightedPage: extra method GetDependencyManagerForScopesAll",
    "page.WeightedPage: extra method GetIdentity",
    "page.WeightedPage: extra method GetInternalRelatedDocsHandler",
    "page.WeightedPage: extra method GetInternalTemplateBasePathAndDescriptor",
    "page.WeightedPage: extra method Group",
    "page.WeightedPage: extra method IdentifierBase",
    "page.WeightedPage: extra method Key",
    "page.WeightedPage: extra method MarkStale",
    "page.WeightedPage: extra method MarshalJSON",
    "page.WeightedPage: extra method Page",
    "page.WeightedPage: extra method PagesRecursive",
    "page.WeightedPage: extra method StaleVersion",
    "page.WeightedPage: extra method Weight",
];

#[test]
fn method_sets() {
    nh_hugolib::tplapi::named_types::init();
    let fx = fixture("site/methodsets.json.gz");
    let site_fx = fixture("site/site-menus.json.gz");
    let tmp = TempDir::new("methodsets");
    let mut b = new_sites(&site_fx["site"], &tmp.0).unwrap();
    nh_hugolib::build_process::process(&mut b.h, &BuildCfg::default()).unwrap();
    nh_hugolib::build_assemble::assemble(&mut b.h, &BuildCfg::default()).unwrap();
    let Built { h, .. } = b;
    let h = h.freeze();
    let env = Env {
        reg: nh_hugolib::tplapi::named_types::registry(),
        ctx: TplContext::default(),
    };

    let sh = SiteHandle {
        h: h.clone(),
        idx: 0,
    };
    let hs = Value::object(HugolibSiteObject(sh.clone()));
    let sw = sh.site_ref().to_value();
    let home = env.get(&hs, "Home", &[]);
    let home_id = home
        .as_object()
        .unwrap()
        .as_any()
        .downcast_ref::<PageRef>()
        .unwrap()
        .0
        .as_any()
        .downcast_ref::<PageHandle>()
        .unwrap()
        .id;
    let wrap = |w: PageWrapper| {
        PageHandle {
            h: h.clone(),
            id: home_id,
            wrapper: w,
        }
        .page_ref()
        .to_value()
    };
    let filtered = PageRef(Arc::new(PageHeadingsFiltered {
        p: PageHandle {
            h: h.clone(),
            id: home_id,
            wrapper: PageWrapper::None,
        },
        headings: Default::default(),
    }))
    .to_value();

    let pages = env.get(&home, "RegularPagesRecursive", &[]);
    let groups = env.get(&pages, "GroupBy", &[Value::string("Section")]);
    let pager = env.get(&home, "Paginator", &[]);
    let taxonomies = env.get(&sw, "Taxonomies", &[]);
    let tags = taxonomies
        .as_map()
        .and_then(|m| m.get(b"tags").cloned())
        .expect("the tags taxonomy");
    let weighted = env.get(&tags, "Get", &[Value::string("x")]);
    let ordered = env.get(&tags, "Alphabetical", &[]);
    let ofs = env.get(&home, "OutputFormats", &[]);
    let menus = env.get(&sw, "Menus", &[]);
    let main_menu = menus
        .as_map()
        .and_then(|m| m.get(b"main").cloned())
        .expect("the main menu");
    let page_menus = {
        let about = env.get(&home, "GetPage", &[Value::string("/about")]);
        env.get(&about, "Menus", &[])
    };

    let mut values: HashMap<&str, Value> = HashMap::new();
    values.insert("*hugolib.pageState", wrap(PageWrapper::None));
    values.insert("hugolib.pageWithWeight0", wrap(PageWrapper::Weight0(1)));
    values.insert("*hugolib.pageWithOrdinal", wrap(PageWrapper::Ordinal(1)));
    values.insert("*hugolib.pageForShortcode", wrap(PageWrapper::ForShortcode));
    values.insert(
        "*hugolib.pageForRenderHooks",
        wrap(PageWrapper::ForRenderHooks),
    );
    values.insert("*hugolib.pageHeadingsFiltered", filtered);
    values.insert("*page.nopPage", nh_page::page_nop::nil_page_value());
    values.insert("*hugolib.Site", hs.clone());
    values.insert("*page.siteWrapper", sw.clone());
    values.insert("page.PageGroup", items(&groups)[0].clone());
    values.insert("page.PagesGroup", groups);
    values.insert("page.Pages", pages);
    values.insert("*page.Pager", pager.clone());
    values.insert("page.pagers", env.get(&pager, "Pagers", &[]));
    values.insert("page.WeightedPage", items(&weighted)[0].clone());
    values.insert("page.WeightedPages", weighted);
    values.insert("page.Taxonomy", tags);
    values.insert("page.TaxonomyList", taxonomies);
    values.insert("page.OrderedTaxonomyEntry", items(&ordered)[0].clone());
    values.insert("page.OrderedTaxonomy", ordered);
    values.insert("page.Data", env.get(&home, "Data", &[]));
    values.insert("page.OutputFormat", items(&ofs)[0].clone());
    values.insert(
        "*page.OutputFormat",
        env.get(&ofs, "Get", &[Value::string("html")]),
    );
    values.insert("page.OutputFormats", ofs);
    values.insert("resource.Resources", named_list("resource.Resources"));
    values.insert("maps.Params", env.get(&home, "Params", &[]));
    values.insert("*maps.Scratch", env.get(&home, "Scratch", &[]));
    values.insert("*navigation.MenuEntry", items(&main_menu)[0].clone());
    values.insert("navigation.Menu", main_menu);
    values.insert("navigation.Menus", menus);
    values.insert("navigation.PageMenus", page_menus);
    values.insert("langs.Languages", env.get(&sw, "Languages", &[]));
    values.insert("*langs.Language", env.get(&home, "Language", &[]));
    values.insert("config.SitemapConfig", env.get(&home, "Sitemap", &[]));
    values.insert("page.SiteConfig", env.get(&sw, "Config", &[]));
    values.insert("*source.File", env.get(&home, "File", &[]));
    values.insert("neohugo.HugoInfo", env.get(&sw, "Hugo", &[]));
    values.insert("media.Type", env.get(&home, "MediaType", &[]));
    values.insert("page.Sites", env.get(&home, "Sites", &[]));
    let _ = named_map;

    let types = fx.as_array().unwrap();
    let mut all_names = BTreeSet::new();
    for t in types {
        for m in t["methods"].as_array().unwrap() {
            all_names.insert(m["name"].as_str().unwrap().to_string());
        }
        for f in t["fields"].as_array().into_iter().flatten() {
            all_names.insert(f.as_str().unwrap().to_string());
        }
    }

    let mut mismatches = BTreeSet::new();
    let mut checked = 0;
    for t in types {
        let tn = t["type"].as_str().unwrap();
        let v = values
            .get(tn)
            .unwrap_or_else(|| panic!("no receiver for {tn}"));
        let got_t = v.go_type_name().into_owned();
        if got_t != tn {
            mismatches.insert(format!("{tn}: type is {got_t}"));
        }
        let methods: Vec<&J> = t["methods"].as_array().unwrap().iter().collect();
        let own: BTreeSet<&str> = methods
            .iter()
            .map(|m| m["name"].as_str().unwrap())
            .collect();
        for n in &all_names {
            if !own.contains(n.as_str()) && env.has(v, n) {
                mismatches.insert(format!("{tn}: extra method {n}"));
            }
        }
        for m in methods {
            let name = m["name"].as_str().unwrap();
            if !env.has(v, name) {
                mismatches.insert(format!("{tn}: missing method {name}"));
                continue;
            }
            checked += 1;
            let num_in = m["numIn"].as_u64().unwrap() as usize;
            let ctx = usize::from(m["ctx"].as_bool().unwrap());
            let variadic = m["variadic"].as_bool().unwrap();
            // The arity check.
            let (args, want) = if variadic {
                if num_in - 1 > ctx {
                    let n = num_in - 2 - ctx;
                    (
                        n,
                        Some(format!(
                            "wrong number of args for {name}: want at least {} got {n}",
                            num_in - 1
                        )),
                    )
                } else {
                    (0, None)
                }
            } else {
                let n = num_in - ctx + 1;
                (
                    n,
                    Some(format!(
                        "wrong number of args for {name}: want {num_in} got {}",
                        n + ctx
                    )),
                )
            };
            if let Some(want) = want {
                let r = env.call(v, name, &vec![Value::Invalid; args]);
                match r {
                    Some(Err(e)) if e == want => {}
                    r => {
                        mismatches.insert(format!("{tn}: {name} arity: {}", outcome(&r)));
                    }
                }
            }
            // goodFunc.
            if !m["good"].as_bool().unwrap() {
                let n = num_in - ctx - usize::from(variadic);
                let want = format!(
                    "can't call method/function {name:?} with {} results",
                    m["numOut"]
                );
                match env.call(v, name, &vec![Value::Invalid; n]) {
                    Some(Err(e)) if e == want => {}
                    r => {
                        mismatches.insert(format!("{tn}: {name} results: {}", outcome(&r)));
                    }
                }
            }
        }
        if let Some(o) = v.as_object() {
            let fields: Vec<&str> = t["fields"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|f| f.as_str().unwrap())
                .collect();
            let missing: Vec<&str> = fields
                .iter()
                .copied()
                .filter(|f| o.field(f).is_none())
                .collect();
            if !missing.is_empty() && missing.len() == fields.len() {
                // The embedded (provider/`*deps.Deps`) fields of the hugolib types: one entry.
                mismatches.insert(format!("{tn}: none of the {} fields", fields.len()));
            } else {
                for f in missing {
                    mismatches.insert(format!("{tn}: missing field {f}"));
                }
            }
        }
    }
    assert!(checked > 900, "{checked}");
    let known: BTreeSet<String> = KNOWN.iter().map(|s| s.to_string()).collect();
    let new: Vec<_> = mismatches.difference(&known).collect();
    let fixed: Vec<_> = known.difference(&mismatches).collect();
    assert!(
        new.is_empty() && fixed.is_empty(),
        "method sets differ from Go\nnew mismatches ({}):\n{}\nno longer mismatching ({}):\n{}",
        new.len(),
        new.iter()
            .map(|s| format!("    {s:?},"))
            .collect::<Vec<_>>()
            .join("\n"),
        fixed.len(),
        fixed
            .iter()
            .map(|s| format!("    {s:?},"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}
