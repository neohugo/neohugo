//! T23 acceptance: the template API of pages and sites against the Go oracle
//! `tools/go-oracle/nh-hugolib/site`. For each recorded site (the assemble oracle's 17 sites and
//! the menus/refs sites) the sites are processed and assembled, and then, in the oracle's order:
//!
//! * the `*page.siteWrapper` and `*hugolib.Site` methods are called through their template tables
//!   (`Object::call_method`), and `.Site.Params` must be the same map on every call and the
//!   language's params;
//! * every page method that does not render content is called through the page tables, with the
//!   oracle's arguments, on the page and on its wrappers (`*pageForShortcode`,
//!   `*pageForRenderHooks`, `pageWithWeight0`, `*pageWithOrdinal`);
//! * the paginators of the nodes (`.Paginate` before `.Paginator` on every other node: the first
//!   call wins; again after a rendering-site shift, which resets them);
//! * the probe templates (no template functions: the test's func map is names-only) run through
//!   the template store with the page in the context: nil results (`.Parent` of home through a
//!   Scratch, `with`, a variable, a chain), `.GetPage` misses (the nop page), `mainsections`;
//!
//! and every result (with its Go type, pages as indexes into the page list), the log (REF_NOT_FOUND
//! errors, menu warnings) must be identical. A Go panic in a method is an error in the port.
//!
//! Go map order: with sections tied for the most regular pages (asm-taxo), Go's automatic main
//! section is any of them; the oracle records the first in byte order, the port's choice.
//! Results the oracle does not encode (e.g. `[]interface {}`) are compared by Go type only.

mod support;

use std::collections::HashMap;
use std::sync::Arc;

use go_value::{MapType, Object, SafeKind, Value};
use nh_hugolib::HugoSites;
use nh_hugolib::content_map_trees::ContentNode;
use nh_hugolib::hugo_sites_build::BuildCfg;
use nh_hugolib::page::{PageHandle, PageId, PageWrapper};
use nh_hugolib::site::{HugolibSiteObject, SiteHandle};
use nh_hugolib::tplapi::page_methods::PageHeadingsFiltered;
use nh_page::page::PageRef;
use nh_tpl::template::TplContext;
use serde_json::{Value as J, json};
use support::*;

use nh_doctree::nodeshifttree::WalkConfig;

struct Dumper {
    h: Arc<HugoSites>,
    dir: String,
    pages: Vec<PageId>,
    seen: HashMap<PageId, usize>,
    ctx: TplContext,
}

fn handle(h: &Arc<HugoSites>, id: PageId, wrapper: PageWrapper) -> PageHandle {
    PageHandle {
        h: h.clone(),
        id,
        wrapper,
    }
}

impl Dumper {
    fn idx(&mut self, id: PageId) -> usize {
        if let Some(i) = self.seen.get(&id) {
            return *i;
        }
        let i = self.pages.len();
        self.seen.insert(id, i);
        self.pages.push(id);
        i
    }

    fn norm(&self, s: &str) -> String {
        s.replace(&self.dir, "/SITE")
    }

    fn str(&self, s: &[u8]) -> J {
        let s = String::from_utf8_lossy(s);
        encode(&Value::string(self.norm(&s).as_str()))
    }

    fn items(&mut self, t: &str, items: &[Value]) -> J {
        let items: Vec<J> = items.iter().map(|v| self.any(v)).collect();
        json!({"t": t, "items": items})
    }

    fn entries(&mut self, t: &str, m: &go_value::Map) -> J {
        let mut out = Vec::new();
        for (k, v) in &m.entries {
            let e = self.any(v);
            out.push(json!([k.to_str_lossy(), e]));
        }
        json!({"t": t, "entries": out})
    }

    /// Mirrors the oracle's `nhSiteDumper.any` (and `value`).
    fn any(&mut self, v: &Value) -> J {
        match v {
            Value::Invalid => json!({"t": "nil"}),
            Value::TypedNil(t) => json!({"t": format!("nil:{t}")}),
            Value::String(s) => self.str(s.as_bytes()),
            Value::Safe(SafeKind::Html, s) => {
                let s = String::from_utf8_lossy(s.as_bytes());
                encode(&Value::html(self.norm(&s).as_str()))
            }
            Value::List(l) => {
                let t = l.ty.go_name().into_owned();
                match t.as_str() {
                    "page.Pages" | "page.Sites" | "resource.Resources" | "page.OutputFormats"
                    | "langs.Languages" | "navigation.Menu" => self.items(&t, &l.items),
                    "page.WeightedPages" => {
                        let mut out = Vec::new();
                        for it in &l.items {
                            let w = it
                                .downcast::<nh_page::weighted::WeightedPage>()
                                .expect("a weighted page");
                            let p = self.any(&w.page.to_value());
                            out.push(json!({"w": w.weight, "p": p}));
                        }
                        json!({"t": t, "items": out})
                    }
                    "[]string" => encode(v),
                    // The oracle encodes only the types it lists.
                    _ => json!({"t": t, "unencoded": true}),
                }
            }
            Value::Map(m) => {
                let t = m.ty.go_name().into_owned();
                match t.as_str() {
                    "page.Data"
                    | "page.Taxonomy"
                    | "page.TaxonomyList"
                    | "navigation.Menus"
                    | "navigation.PageMenus"
                    | "page.AuthorList" => self.entries(&t, m),
                    "maps.Params" | "map[string]interface {}" | "map[string]string" => encode(v),
                    _ => json!({"t": t, "unencoded": true}),
                }
            }
            Value::Object(o) => self.object(o.as_ref(), v),
            _ => encode(v),
        }
    }

    fn page(&mut self, p: &PageRef) -> J {
        let t = p.0.tpl_type_name().into_owned();
        if let Some(h) = p.0.as_any().downcast_ref::<PageHandle>() {
            let i = self.idx(h.id);
            return match h.wrapper {
                PageWrapper::Weight0(w) => json!({"t": t, "p": i, "w0": w}),
                PageWrapper::Ordinal(o) => json!({"t": t, "p": i, "ord": o}),
                _ => json!({"t": t, "p": i}),
            };
        }
        if let Some(h) = p.0.as_any().downcast_ref::<PageHeadingsFiltered>() {
            let i = self.idx(h.p.id);
            return json!({"t": t, "p": i});
        }
        if let Some(n) = p.0.as_any().downcast_ref::<nh_page::page_nop::NopPage>() {
            return json!({"t": t, "nil": n.nil});
        }
        json!({"t": t})
    }

    fn object(&mut self, o: &dyn Object, v: &Value) -> J {
        let t = o.type_name().into_owned();
        let any = o.as_any();
        if let Some(p) = any.downcast_ref::<PageRef>() {
            return self.page(p);
        }
        if let Some(s) = any.downcast_ref::<nh_page::site::SiteRef>() {
            return json!({"t": t, "lang": s.0.language().lang});
        }
        if let Some(s) = any.downcast_ref::<HugolibSiteObject>() {
            return json!({"t": t, "lang": s.0.site().language.lang});
        }
        if let Some(of) = any.downcast_ref::<nh_page::page_outputformat::OutputFormat>() {
            return json!({"t": t, "name": of.name(), "rel": of.rel, "relPermalink": of.rel_permalink(),
                "permalink": of.permalink(), "mediaType": of.media_type().typ});
        }
        if let Some(of) = any.downcast_ref::<nh_page::page_outputformat::OutputFormatPtr>() {
            let of = &of.0;
            return json!({"t": t, "name": of.name(), "rel": of.rel, "relPermalink": of.rel_permalink(),
                "permalink": of.permalink(), "mediaType": of.media_type().typ});
        }
        if let Some(r) = any.downcast_ref::<nh_resource::resourcetypes::ResourceRef>() {
            let r = &r.0;
            return json!({
                "t": t, "name": r.name(), "title": r.title(), "type": r.resource_type(),
                "mediaType": r.media_type().typ, "params": encode_params(&r.params()),
                "relPermalink": r.rel_permalink(),
            });
        }
        if let Some(l) = any.downcast_ref::<nh_langs::language::LanguageObject>() {
            return json!({"t": t, "lang": l.0.lang});
        }
        if let Some(s) = any.downcast_ref::<nh_common::maps::scratch::Scratch>() {
            return json!({"t": t, "values": encode(&s.values())});
        }
        if let Some(s) = any.downcast_ref::<nh_config::common_config::SitemapConfig>() {
            return json!({"t": t, "changeFreq": s.change_freq, "priority": encode(&Value::float64(s.priority)),
                "filename": s.filename, "disable": s.disable});
        }
        if let Some(s) = any.downcast_ref::<nh_page::site::SiteConfig>() {
            return json!({"t": t, "rssLimit": s.services.rss.limit});
        }
        if let Some(m) = any.downcast_ref::<nh_media::media::media_type::MediaType>() {
            return json!({"t": t, "type": m.typ});
        }
        if let Some(hi) = any.downcast_ref::<nh_config::neohugo::neohugo::HugoInfo>() {
            return json!({"t": t, "environment": hi.environment});
        }
        if let Some(f) = any.downcast_ref::<nh_helpers::source::file_info::FileObject>() {
            return match &f.0 {
                None => json!({"t": "nil:*source.File"}),
                Some(f) => json!({"t": t, "path": f.path(), "uniqueID": f.unique_id(),
                    "lang": f.lang(), "section": f.section()}),
            };
        }
        if let Some(m) = any.downcast_ref::<nh_page::navigation::menu::MenuEntryRef>() {
            let me = &m.0;
            let page = match me.page_ref() {
                Some(p) => self.page(&p),
                None => J::Null,
            };
            let children = if me.children.is_empty() {
                json!({"t": "nil:navigation.Menu"})
            } else {
                let items: Vec<Value> = me
                    .children
                    .iter()
                    .map(|c| Value::object(nh_page::navigation::menu::MenuEntryRef(c.clone())))
                    .collect();
                self.items("navigation.Menu", &items)
            };
            let params = match &me.config.params {
                None => json!({"t": "nil:maps.Params"}),
                Some(p) => encode_params(p),
            };
            return json!({
                "t": t, "identifier": me.config.identifier, "parent": me.config.parent, "name": me.config.name,
                "pre": me.config.pre.to_str_lossy(), "post": me.config.post.to_str_lossy(),
                "url": me.url(), "configuredURL": me.configured_url, "pageRef": me.config.page_ref,
                "weight": me.config.weight, "title": me.config.title, "menu": me.menu, "params": params,
                "page": page, "children": children, "hasChildren": me.has_children(), "keyName": me.key_name(),
            });
        }
        if let Some(p) = any.downcast_ref::<nh_page::pagination::PagerRef>() {
            let pg = &p.0;
            let pages = match pg.pages_opt() {
                Some(ps) => self.items(
                    "page.Pages",
                    &ps.iter().map(|p| p.to_value()).collect::<Vec<_>>(),
                ),
                None => json!({"t": "nil:page.Pages"}),
            };
            return json!({
                "t": t, "number": pg.page_number(), "url": pg.url(), "totalPages": pg.paginator.total_pages(),
                "pagerSize": pg.paginator.pager_size(), "numberOfElements": pg.number_of_elements(),
                "totalNumberOfElements": pg.paginator.total_number_of_elements(),
                "hasNext": pg.has_next(), "hasPrev": pg.has_prev(), "pages": pages,
            });
        }
        if t.starts_with("func(") {
            return json!({"t": t});
        }
        if o.go_string().is_some() || o.underlying().is_some() {
            return encode(v);
        }
        json!({"t": t, "unencoded": true})
    }

    /// Mirrors the oracle's `call`: the result of a template method call.
    fn call(&mut self, recv: &Value, name: &str, args: &[Value]) -> J {
        let o = recv.as_object().expect("an object receiver");
        if !o.has_method(name) {
            return json!({"missing": true});
        }
        let r = o.call_method(&self.ctx, name, args).expect("has_method");
        match r {
            Ok(v) => {
                let v = self.any(&v);
                json!({ "ok": v })
            }
            Err(e) => json!({"err": self.norm(e.message())}),
        }
    }
}

fn s(v: &str) -> Value {
    Value::string(v)
}

fn map_arg(entries: &[(&str, Value)]) -> Value {
    let mut m = go_value::Map::new(MapType::StringAny);
    for (k, v) in entries {
        m.insert(*k, v.clone());
    }
    Value::map(m)
}

/// The probed pages' nodes of a site in walk order, then its bundled pages.
fn site_nodes(h: &Arc<HugoSites>, si: usize) -> Vec<PageId> {
    let cfg = WalkConfig {
        dims: h.sites[si].page_map.dims,
        ..Default::default()
    };
    let mut nodes = Vec::new();
    h.page_trees
        .tree_pages
        .walk(&cfg, |_w, _key, n, _m| {
            nodes.push(n.page_id().unwrap());
            Ok(false)
        })
        .unwrap();
    h.page_trees
        .tree_resources
        .walk(&cfg, |_w, _key, n, _m| {
            if let ContentNode::Resource(rs) = n
                && let Some(id) = rs.page
            {
                nodes.push(id);
            }
            Ok(false)
        })
        .unwrap();
    nodes
}

const ZERO_ARG: &[&str] = &[
    "Aliases",
    "AllTranslations",
    "AlternativeOutputFormats",
    "Ancestors",
    "BundleType",
    "CodeOwners",
    "CurrentSection",
    "Data",
    "Date",
    "Description",
    "Draft",
    "ExpiryDate",
    "File",
    "FirstSection",
    "GitInfo",
    "IdentifierBase",
    "IsHome",
    "IsNode",
    "IsPage",
    "IsSection",
    "IsTranslated",
    "Keywords",
    "Kind",
    "Lang",
    "Language",
    "Lastmod",
    "Layout",
    "LinkTitle",
    "MediaType",
    "Menus",
    "Name",
    "Next",
    "NextInSection",
    "NextPage",
    "OutputFormats",
    "Page",
    "Pages",
    "PagesRecursive",
    "Params",
    "Parent",
    "Path",
    "Permalink",
    "Prev",
    "PrevInSection",
    "PrevPage",
    "PublishDate",
    "RawContent",
    "RegularPages",
    "RegularPagesRecursive",
    "RelPermalink",
    "ResourceType",
    "Resources",
    "Section",
    "Sections",
    "SectionsEntries",
    "SectionsPath",
    "Site",
    "Sitemap",
    "Sites",
    "Slug",
    "StaleVersion",
    "String",
    "Title",
    "TranslationKey",
    "Translations",
    "Type",
    "Weight",
    "HeadingsFiltered",
    "Weight0",
    "Ordinal",
    "Unwrapv",
];

const WRAPPER_ONLY: &[&str] = &[
    "Content",
    "ContentWithoutSummary",
    "Plain",
    "PlainWords",
    "Summary",
    "Truncated",
    "FuzzyWordCount",
    "WordCount",
    "ReadingTime",
    "Len",
    "Markup",
];

impl Dumper {
    fn page_value(&self, id: PageId) -> Value {
        handle(&self.h, id, PageWrapper::None).page_ref().to_value()
    }

    /// Mirrors the oracle's `pageArgs`.
    fn page_args(&mut self, id: PageId, si: usize) -> Vec<(String, Vec<Value>)> {
        let h = self.h.clone();
        let s = &h.sites[si];
        let p = self.page_value(id);
        let mut out: Vec<(String, Vec<Value>)> = Vec::new();
        let add = |out: &mut Vec<(String, Vec<Value>)>, n: &str, a: Vec<Value>| {
            out.push((n.to_string(), a));
        };
        let mut others = vec![
            p.clone(),
            nh_page::page_nop::nil_page_value(),
            s_("str"),
            Value::Invalid,
        ];
        if let Some(home) = s.home {
            others.push(self.page_value(home));
        }
        if let Some(pp) = nh_hugolib::page__tree::parent_id(&h, id) {
            others.push(self.page_value(pp));
        }
        for o in &others {
            for n in ["Eq", "InSection", "IsAncestor", "IsDescendant"] {
                add(&mut out, n, vec![o.clone()]);
            }
        }
        for r in [
            "",
            ".",
            "..",
            "/",
            "nope",
            "/nope",
            "index.md",
            "_index.md",
            "../_index.md",
        ] {
            add(&mut out, "GetPage", vec![s_(r)]);
        }
        for v in &s.page_map.cfg.taxonomy_config.views {
            add(&mut out, "GetTerms", vec![s_(&v.plural)]);
        }
        add(&mut out, "GetTerms", vec![s_("nope")]);
        for n in ["ref", "relref", "badge", "note", "box", "nope"] {
            add(&mut out, "HasShortcode", vec![s_(n)]);
        }
        let ps = h.page(id);
        let mut pkeys: Vec<String> = ps
            .meta
            .params()
            .map(|m| {
                m.entries
                    .keys()
                    .map(|k| k.to_str_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        pkeys.sort();
        pkeys.truncate(6);
        pkeys.extend(["Title".to_string(), "nope".into(), "a.b".into()]);
        for k in &pkeys {
            add(&mut out, "Param", vec![s_(k)]);
        }
        let path = ps.meta.path();
        let mut targets = vec![
            path.clone(),
            "/".into(),
            "nope".into(),
            "/nope".into(),
            "".into(),
            "#frag".into(),
            format!("{path}#frag"),
            "..".into(),
        ];
        if let Some(home) = s.home {
            targets.push(format!("{}#top", h.page(home).meta.path()));
        }
        let langs: Vec<String> = h.sites.iter().map(|s| s.language.lang.clone()).collect();
        for t in &targets {
            let m = map_arg(&[("path", s_(t))]);
            add(&mut out, "Ref", vec![m.clone()]);
            add(&mut out, "RelRef", vec![m.clone()]);
            add(&mut out, "RefFrom", vec![m.clone(), p.clone()]);
            add(&mut out, "RelRefFrom", vec![m, p.clone()]);
        }
        for lang in langs.iter().map(|s| s.as_str()).chain(["xx"]) {
            let m = map_arg(&[("path", s_("/")), ("lang", s_(lang))]);
            add(&mut out, "Ref", vec![m.clone()]);
            add(&mut out, "RelRef", vec![m]);
        }
        for of in ["json", "rss", "html", "nope"] {
            let m = map_arg(&[("path", s_("/")), ("outputFormat", s_(of))]);
            add(&mut out, "Ref", vec![m.clone()]);
            add(&mut out, "RelRef", vec![m]);
        }
        add(
            &mut out,
            "Ref",
            vec![map_arg(&[
                ("Path", s_(&path)),
                ("LANG", s_(&s.language.lang)),
            ])],
        );
        add(&mut out, "Ref", vec![map_arg(&[("path", Value::int(42))])]);
        add(
            &mut out,
            "Ref",
            vec![map_arg(&[("path", Value::string_list(["a"]))])],
        );
        add(&mut out, "Ref", vec![map_arg(&[])]);
        add(
            &mut out,
            "Ref",
            vec![Value::TypedNil(Arc::from("map[string]interface {}"))],
        );
        let menus = nh_hugolib::site::site_menus(&h, si);
        for (mn, menu) in menus.iter() {
            for me in menu.iter().take(3) {
                let mev = Value::object(nh_page::navigation::menu::MenuEntryRef(me.clone()));
                add(&mut out, "HasMenuCurrent", vec![s_(mn), mev.clone()]);
                add(&mut out, "IsMenuCurrent", vec![s_(mn), mev]);
                for c in me.children.iter().take(2) {
                    let cv = Value::object(nh_page::navigation::menu::MenuEntryRef(c.clone()));
                    add(&mut out, "HasMenuCurrent", vec![s_(mn), cv.clone()]);
                    add(&mut out, "IsMenuCurrent", vec![s_(mn), cv]);
                }
            }
        }
        out
    }

    /// Mirrors the oracle's `pageCalls`.
    fn page_calls(
        &mut self,
        recv: &Value,
        id: PageId,
        si: usize,
        wrapper: &str,
        with_args: bool,
    ) -> J {
        let mut calls = Vec::new();
        let o = recv.as_object().unwrap();
        for name in ZERO_ARG {
            if o.has_method(name) {
                let r = self.call(recv, name, &[]);
                calls.push(json!([name, J::Null, r]));
            }
        }
        if !wrapper.is_empty() {
            for name in WRAPPER_ONLY {
                let r = self.call(recv, name, &[]);
                calls.push(json!([name, J::Null, r]));
            }
            if wrapper == "shortcode" {
                let r = self.call(recv, "TableOfContents", &[]);
                calls.push(json!(["TableOfContents", J::Null, r]));
            }
        }
        let args = if with_args {
            self.page_args(id, si)
        } else {
            Vec::new()
        };
        for (name, a) in args {
            let enc_args: Vec<J> = a.iter().map(|x| self.any(x)).collect();
            let r = self.call(recv, &name, &a);
            let enc_args = if enc_args.is_empty() {
                J::Null
            } else {
                J::Array(enc_args)
            };
            calls.push(json!([name, enc_args, r]));
        }
        // The page's store is shared by .Scratch and .Store.
        let sc = o
            .call_method(&self.ctx, "Scratch", &[])
            .and_then(|r| r.ok());
        if let Some(sc @ Value::Object(_)) = sc {
            let i = self.idx(id);
            sc.as_object()
                .unwrap()
                .call_method(
                    &self.ctx,
                    "Set",
                    &[s_(&format!("probe-{wrapper}")), Value::int(i as i64)],
                )
                .unwrap()
                .unwrap();
            let r = self.call(recv, "Store", &[]);
            calls.push(json!(["Store", J::Null, r]));
        }
        J::Array(calls)
    }

    fn pager_dump(&mut self, r: Result<Value, go_value::Error>) -> J {
        match r {
            Err(e) => json!({"err": self.norm(e.message())}),
            Ok(Value::TypedNil(_)) => json!({"nil": true}),
            Ok(v) => {
                let p = v
                    .downcast::<nh_page::pagination::PagerRef>()
                    .expect("a pager")
                    .clone();
                let current = self.any(&v);
                let mut pagers = Vec::new();
                for pp in p.0.paginator.pagers() {
                    pagers.push(self.any(&Value::object(nh_page::pagination::PagerRef(pp))));
                }
                json!({"current": current, "pagers": pagers, "first": p.0.first().page_number(),
                    "last": p.0.last().page_number()})
            }
        }
    }

    fn page_call(
        &mut self,
        id: PageId,
        name: &str,
        args: &[Value],
    ) -> Result<Value, go_value::Error> {
        let p = self.page_value(id);
        p.as_object()
            .unwrap()
            .call_method(&self.ctx, name, args)
            .expect("a page method")
    }

    fn dump(&mut self, probes: &[(String, String)]) -> J {
        let h = self.h.clone();
        let mut site_nodes_all = Vec::new();
        for si in 0..h.sites.len() {
            let nodes = site_nodes(&h, si);
            for &id in &nodes {
                self.idx(id);
            }
            site_nodes_all.push(nodes);
        }

        // Sites.
        let mut site_dumps = Vec::new();
        for (si, site) in h.sites.iter().enumerate() {
            let sh = SiteHandle {
                h: h.clone(),
                idx: si,
            };
            let sw = sh.site_ref().to_value();
            let mut calls = Vec::new();
            for name in [
                "AllPages",
                "Author",
                "Authors",
                "BaseURL",
                "BuildDrafts",
                "Config",
                "Copyright",
                "Current",
                "Data",
                "Home",
                "Hugo",
                "IsMultiLingual",
                "Key",
                "Language",
                "LanguageCode",
                "LanguagePrefix",
                "Languages",
                "LastChange",
                "Lastmod",
                "MainSections",
                "Menus",
                "Pages",
                "Params",
                "RegularPages",
                "Sections",
                "ServerPort",
                "Sites",
                "Social",
                "Store",
                "Taxonomies",
                "Title",
            ] {
                let r = self.call(&sw, name, &[]);
                calls.push(json!([name, r]));
            }
            let mut refs: Vec<String> = [
                "", "/", "home", "section", "/nope", "404", "sitemap", "/tags", "tags",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect();
            for &id in &site_nodes_all[si] {
                refs.push(h.page(id).meta.path());
            }
            for r in &refs {
                let res = self.call(&sw, "GetPage", &[s_(r)]);
                calls.push(json!(["GetPage", r, res]));
            }
            let res = self.call(&sw, "GetPage", &[s_("section"), s_("blog")]);
            calls.push(json!(["GetPage", "section|blog", res]));
            let res = self.call(&sw, "GetPage", &[s_("a"), s_("b"), s_("c")]);
            calls.push(json!(["GetPage", "a|b|c", res]));
            let mut pkeys: Vec<String> = site
                .conf
                .params
                .entries
                .keys()
                .map(|k| k.to_str_lossy().into_owned())
                .collect();
            pkeys.sort();
            pkeys.extend(["nope".to_string(), "MAINSECTIONS".into(), "a.b".into()]);
            for k in &pkeys {
                let res = self.call(&sw, "Param", &[s_(k)]);
                calls.push(json!(["Param", k, res]));
            }

            let hs = Value::object(HugolibSiteObject(sh.clone()));
            let mut hcalls = Vec::new();
            for name in [
                "SitemapAbsURL",
                "Lastmod",
                "BaseURL",
                "Title",
                "GetLanguagePrefix",
                "GetTargetLanguageBasePath",
                "Lang",
                "LanguagePrefix",
                "ServerPort",
                "Site",
                "Current",
                "MainSections",
            ] {
                let r = self.call(&hs, name, &[]);
                hcalls.push(json!([name, r]));
            }
            let b = Value::Bool;
            let hargs: Vec<(&str, Vec<Value>)> = vec![
                ("AbsURL", vec![s_("a/b.html"), b(true)]),
                ("AbsURL", vec![s_("/a/b/"), b(false)]),
                ("AbsURL", vec![s_("https://x.org/y"), b(false)]),
                ("RelURL", vec![s_("a/b.html"), b(true)]),
                ("RelURL", vec![s_("/a/b/"), b(false)]),
                ("URLize", vec![s_("Hello World/Ünïcode & Co")]),
                ("URLizeFilename", vec![s_("a b/c.md")]),
                ("URLEscape", vec![s_("a b?c=d#e")]),
                ("MakePath", vec![s_("A B/c.md")]),
                ("MakePathSanitized", vec![s_("A B/c.md")]),
                ("PrependBasePath", vec![s_("/a/b"), b(false)]),
                ("PrependBasePath", vec![s_("/a/b"), b(true)]),
                (
                    "PermalinkForBaseURL",
                    vec![s_("/x/"), s_("https://b.org/sub/")],
                ),
                ("IsAbsURL", vec![s_("https://x.org")]),
                ("IsAbsURL", vec![s_("/x")]),
                ("GetBasePath", vec![b(true)]),
                ("GetBasePath", vec![b(false)]),
                ("SanitizeAnchorName", vec![s_("Hello World ÄÖ")]),
                ("ResolveMarkup", vec![s_("md")]),
            ];
            for (name, a) in hargs {
                // The oracle records these arguments as plain JSON strings/bools.
                let enc_args: Vec<J> = a
                    .iter()
                    .map(|x| match x {
                        Value::Bool(v) => json!(v),
                        Value::String(s) => json!(s.to_str_lossy()),
                        v => panic!("unexpected argument {v:?}"),
                    })
                    .collect();
                let r = self.call(&hs, name, &a);
                hcalls.push(json!([name, enc_args, r]));
            }

            let p1 = nh_page::site::Site::params(&sh);
            let p2 = nh_page::site::Site::params(&sh);
            let same = Arc::ptr_eq(&p1, &p2) && Arc::ptr_eq(&p1, &site.language.params());
            site_dumps.push(json!({
                "lang": site.language.lang,
                "siteWrapper": calls,
                "hugolibSite": hcalls,
                "paramsSame": same,
            }));
        }

        let dump_page = |n: usize, wi: usize| n <= 200 || wi.is_multiple_of(10);
        let arg_page = |n: usize, wi: usize| n <= 60 || wi.is_multiple_of(4);

        // Pages.
        let mut page_dumps = Vec::new();
        for (si, nodes) in site_nodes_all.iter().cloned().enumerate() {
            let n = nodes.len();
            for (wi, &id) in nodes.iter().enumerate() {
                if !dump_page(n, wi) {
                    continue;
                }
                let pi = self.idx(id);
                let mut pd = serde_json::Map::new();
                pd.insert("p".into(), json!(pi));
                pd.insert("site".into(), json!(si));
                let pv = self.page_value(id);
                let calls = self.page_calls(&pv, id, si, "", arg_page(n, wi));
                pd.insert("calls".into(), calls);
                if wi % 5 == 0 {
                    let sc = handle(&h, id, PageWrapper::ForShortcode)
                        .page_ref()
                        .to_value();
                    let c = self.page_calls(&sc, id, si, "shortcode", false);
                    pd.insert("shortcode".into(), c);
                    let rh = handle(&h, id, PageWrapper::ForRenderHooks)
                        .page_ref()
                        .to_value();
                    let c = self.page_calls(&rh, id, si, "renderhooks", false);
                    pd.insert("renderHooks".into(), c);
                }
                let pages = self.page_call(id, "Pages", &[]).unwrap();
                if let Value::List(l) = &pages {
                    for t in &l.items {
                        let pr = nh_page::page::page_from_value(t).unwrap();
                        let ph = pr.0.as_any().downcast_ref::<PageHandle>().unwrap();
                        if matches!(ph.wrapper, PageWrapper::Weight0(_)) {
                            let c = self.page_calls(t, ph.id, si, "", false);
                            pd.insert("weight0".into(), c);
                            break;
                        }
                    }
                }
                for v in h.sites[si].page_map.cfg.taxonomy_config.views.clone() {
                    let ts = self.page_call(id, "GetTerms", &[s_(&v.plural)]).unwrap();
                    if let Value::List(l) = &ts
                        && let Some(t) = l.items.first()
                    {
                        let pr = nh_page::page::page_from_value(t).unwrap();
                        let ph = pr.0.as_any().downcast_ref::<PageHandle>().unwrap();
                        let c = self.page_calls(t, ph.id, si, "", false);
                        pd.insert("ordinal".into(), c);
                        break;
                    }
                }
                page_dumps.push(J::Object(pd));
            }
        }

        // Paginators.
        let mut paginators = Vec::new();
        for nodes in &site_nodes_all {
            let n = nodes.len();
            for (wi, &id) in nodes.iter().enumerate() {
                if !h.page(id).meta.is_node() || !dump_page(n, wi) {
                    continue;
                }
                let mut e = serde_json::Map::new();
                e.insert("p".into(), json!(self.idx(id)));
                if wi % 2 == 1 {
                    let pages = self.page_call(id, "Pages", &[]).unwrap();
                    let r = self.page_call(id, "Paginate", &[pages, Value::int(2)]);
                    e.insert("paginate".into(), self.pager_dump(r));
                }
                let r = self.page_call(id, "Paginator", &[]);
                e.insert("paginator".into(), self.pager_dump(r));
                let rp = self.page_call(id, "RegularPages", &[]).unwrap();
                let r = self.page_call(id, "Paginate", &[rp, Value::int(1)]);
                e.insert("paginateAfter".into(), self.pager_dump(r));
                paginators.push(J::Object(e));
            }
        }
        for (si, nodes) in site_nodes_all.iter().cloned().enumerate() {
            h.prepare_pages_for_render(si, true, 0).unwrap();
            let n = nodes.len();
            for (wi, &id) in nodes.iter().enumerate() {
                if !h.page(id).meta.is_node() || !dump_page(n, wi) {
                    continue;
                }
                let r = self.page_call(id, "Paginator", &[Value::int(3)]);
                let pi = self.idx(id);
                let d = self.pager_dump(r);
                paginators.push(json!({"p": pi, "afterReset": d}));
            }
        }

        // Probe templates.
        let mut probe_dumps = Vec::new();
        for (si, site) in h.sites.iter().enumerate() {
            let nodes = site_nodes_all[si].clone();
            let n = nodes.len();
            for (wi, &id) in nodes.iter().enumerate() {
                if (wi % 3 != 0 && !h.page(id).meta.is_home()) || !dump_page(n, wi) {
                    continue;
                }
                for (name, src) in probes {
                    let store = site.deps.get_template_store();
                    let ti = store.text_parse(&format!("probe-{name}"), src).unwrap();
                    let pv = self.page_value(id);
                    let ctx = TplContext {
                        page: Some(pv.clone()),
                        ..Default::default()
                    };
                    let mut buf = Vec::new();
                    let r = store.execute_with_context(&ctx, &ti, &mut buf, &pv);
                    let mut e = serde_json::Map::new();
                    e.insert("p".into(), json!(self.idx(id)));
                    e.insert("probe".into(), json!(name));
                    e.insert(
                        "out".into(),
                        json!(self.norm(&String::from_utf8_lossy(&buf))),
                    );
                    if let Err(err) = r {
                        let msg = self.norm(&err.to_string());
                        e.insert("err".into(), json!(msg));
                    }
                    probe_dumps.push(J::Object(e));
                }
            }
        }

        let mut list = Vec::new();
        let mut i = 0;
        while i < self.pages.len() {
            let p = h.page(self.pages[i]);
            list.push(json!({"site": p.site_idx, "kind": p.meta.kind(), "path": p.meta.path(), "lang": p.meta.lang()}));
            i += 1;
        }

        json!({
            "sites": site_dumps,
            "pages": page_dumps,
            "paginators": paginators,
            "probes": probe_dumps,
            "pageList": list,
        })
    }
}

fn s_(v: &str) -> Value {
    s(v)
}

/// Go panics in methods are errors in the port.
fn normalize_panics(v: &mut J) {
    match v {
        J::Object(m) => {
            if let Some(p) = m.remove("panic") {
                m.insert("err".into(), p);
            }
            for (_, x) in m.iter_mut() {
                normalize_panics(x);
            }
        }
        J::Array(a) => a.iter_mut().for_each(normalize_panics),
        _ => {}
    }
}

fn run_case(name: &str) {
    nh_hugolib::tplapi::named_types::init();
    let mut fx = fixture(&format!("site/{name}.json.gz"));
    normalize_panics(&mut fx);
    let probes: Vec<(String, String)> = fx["probes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p[0].as_str().unwrap().to_string(),
                p[1].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let tmp = TempDir::new(&format!("site-{name}"));
    let mut b = new_sites(&fx["site"], &tmp.0).unwrap_or_else(|e| panic!("{name}: {e}"));
    let mut res = nh_hugolib::build_process::process(&mut b.h, &BuildCfg::default());
    if res.is_ok() {
        res = nh_hugolib::build_assemble::assemble(&mut b.h, &BuildCfg::default());
    }
    let Built { h, dir, log } = b;
    let mut got = serde_json::Map::new();
    got.insert("site".into(), fx["site"].clone());
    got.insert("probes".into(), fx["probes"].clone());
    match res {
        Ok(()) => {
            let h = h.freeze();
            let mut d = Dumper {
                h,
                dir: dir.clone(),
                pages: Vec::new(),
                seen: HashMap::new(),
                ctx: TplContext::default(),
            };
            got.insert("dump".into(), d.dump(&probes));
        }
        Err(e) => {
            got.insert("err".into(), json!(e.to_string().replace(&dir, "/SITE")));
        }
    }
    let lines: Vec<String> = {
        let bytes = log.lock().unwrap().clone();
        String::from_utf8_lossy(&bytes)
            .replace(&dir, "/SITE")
            .split('\n')
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect()
    };
    got.insert("log".into(), json!(lines));
    let got = J::Object(got);
    if let Ok(p) = std::env::var("NH_T23_DUMP") {
        std::fs::write(
            format!("{p}/{name}.got.json"),
            serde_json::to_string_pretty(&got).unwrap(),
        )
        .unwrap();
        std::fs::write(
            format!("{p}/{name}.want.json"),
            serde_json::to_string_pretty(&fx).unwrap(),
        )
        .unwrap();
    }
    if let Some(d) = first_diff(name, &fx, &got) {
        panic!("{name}: template API differs from Go at {d}");
    }
}

macro_rules! cases {
    ($($f:ident => $n:expr),* $(,)?) => {
        $(#[test] fn $f() { run_case($n); })*
    };
}

cases! {
    site_asm_build => "asm-build",
    site_asm_cascade => "asm-cascade",
    site_asm_flags => "asm-flags",
    site_asm_i18n => "asm-i18n",
    site_asm_multihost => "asm-multihost",
    site_asm_taxo => "asm-taxo",
    site_asm_ugly => "asm-ugly",
    site_content => "content",
    site_contentdir => "contentdir",
    site_docs => "docs",
    site_edge_tree => "edge-tree",
    site_homeleaf => "homeleaf",
    site_nokinds => "nokinds",
    site_seeksnack => "seeksnack",
    site_shortcodes => "shortcodes",
    site_synthetic => "synthetic",
    site_testsite => "testsite",
    site_menus => "site-menus",
    site_refs => "site-refs",
}
