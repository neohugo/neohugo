//! T21 acceptance: the assembly phase (`process` + `assemble`) against the Go oracle
//! `tools/go-oracle/nh-hugolib/assemble`. For each recorded site the page set of every site in
//! walk order, every page's meta after `setMetaPost` (params with Go types, dates, default
//! titles, cascade, build options), the target path descriptors, page output formats, page
//! outputs (shared by format name) with their target paths and permalinks, the tree relations,
//! the collections (queried in the oracle's order: the term `.Pages`/`.RegularPages` share a
//! cache key), `Site.Taxonomies`, `.Data`, `GetTerms`, `GetPage` (with and without a context
//! page), the resources and the keys of the query caches must be identical.
//!
//! Go picks the main section at random among the sections with the highest page count (a map
//! range); the fixture records the candidates and any of them is accepted.

mod support;

use std::collections::HashMap;
use std::sync::Arc;

use go_value::Value;
use nh_hugolib::HugoSites;
use nh_hugolib::content_map_page::{
    PageMap, PageMapQueryPagesBelowPath, PageMapQueryPagesInSection, page_predicates as pp,
    site_taxonomies,
};
use nh_hugolib::content_map_trees::ContentNode;
use nh_hugolib::hugo_sites_build::BuildCfg;
use nh_hugolib::page::{PageHandle, PageId, PageWrapper};
use nh_hugolib::pagecollections::new_page_finder;
use nh_page::page::{PageRef, Pages};
use nh_page::weighted::WeightedPages;
use serde_json::{Value as J, json};
use support::*;

use nh_common::kinds;
use nh_doctree::nodeshifttree::WalkConfig;

/// The Rust side of the oracle's dump (overlay_assemble.go.txt).
struct Dumper {
    h: Arc<HugoSites>,
    norm: Box<dyn Fn(&str) -> String>,
    pages: Vec<PageId>,
    seen: HashMap<PageId, usize>,
}

fn handle_of(p: &PageRef) -> &PageHandle {
    p.0.as_any()
        .downcast_ref::<PageHandle>()
        .expect("a page of these sites")
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

    fn ref_idx(&mut self, p: Option<&PageRef>) -> J {
        match p {
            None => J::Null,
            Some(p) => json!(self.idx(handle_of(p).id)),
        }
    }

    fn list(&mut self, ps: &Pages) -> J {
        let mut out = Vec::new();
        for p in ps {
            let hd = handle_of(p);
            let mut e = serde_json::Map::new();
            e.insert("p".into(), json!(self.idx(hd.id)));
            match hd.wrapper {
                PageWrapper::Weight0(w) => {
                    e.insert("w0".into(), json!(w));
                }
                PageWrapper::Ordinal(o) => {
                    e.insert("ord".into(), json!(o));
                }
                _ => {}
            }
            out.push(J::Object(e));
        }
        J::Array(out)
    }

    fn weighted(&mut self, wp: Option<&WeightedPages>) -> J {
        let Some(wp) = wp else {
            return J::Null;
        };
        let mut out = Vec::new();
        for w in wp {
            let p = self.ref_idx(Some(&w.page));
            out.push(json!({"w": w.weight, "p": p}));
        }
        J::Array(out)
    }

    fn taxonomy(&mut self, t: Option<&nh_page::taxonomy::Taxonomy>) -> J {
        let Some(t) = t else {
            return J::Null;
        };
        let mut out = Vec::new();
        for (k, v) in t {
            let w = self.weighted(Some(v));
            out.push(json!([k, w]));
        }
        J::Array(out)
    }

    fn result(&mut self, r: nh_common::Result<Option<PageRef>>) -> J {
        match r {
            Ok(p) => {
                let p = self.ref_idx(p.as_ref());
                json!({ "p": p })
            }
            Err(e) => json!({ "err": e.to_string() }),
        }
    }

    fn handle(&self, id: PageId) -> PageHandle {
        PageHandle {
            h: self.h.clone(),
            id,
            wrapper: PageWrapper::None,
        }
    }

    /// Go `p.Pages()` (page.go).
    fn pages_of(&self, id: PageId) -> Pages {
        let ps = self.h.page(id);
        let si = ps.site_idx;
        let path = ps.meta.path();
        match ps.meta.kind() {
            kinds::KIND_PAGE => Vec::new(),
            kinds::KIND_SECTION | kinds::KIND_HOME => PageMap::get_pages_in_section(
                &self.h,
                si,
                &PageMapQueryPagesInSection {
                    path,
                    key_part: "page-section".into(),
                    include: Some(pp::and(
                        pp::should_list_local(),
                        vec![pp::or(pp::kind_page(), vec![pp::kind_section()])],
                    )),
                    ..Default::default()
                },
            ),
            kinds::KIND_TERM => PageMap::get_pages_with_term(
                &self.h,
                si,
                &PageMapQueryPagesBelowPath {
                    path,
                    ..Default::default()
                },
            ),
            kinds::KIND_TAXONOMY => PageMap::get_pages_in_section(
                &self.h,
                si,
                &PageMapQueryPagesInSection {
                    path,
                    key_part: "term".into(),
                    include: Some(pp::and(pp::should_list_local(), vec![pp::kind_term()])),
                    recursive: true,
                    ..Default::default()
                },
            ),
            _ => self.site_pages(si),
        }
    }

    /// Go `p.RegularPages()` (page.go).
    fn regular_pages_of(&self, id: PageId) -> Pages {
        let ps = self.h.page(id);
        let si = ps.site_idx;
        let path = ps.meta.path();
        match ps.meta.kind() {
            kinds::KIND_PAGE => Vec::new(),
            kinds::KIND_SECTION | kinds::KIND_HOME | kinds::KIND_TAXONOMY => {
                PageMap::get_pages_in_section(
                    &self.h,
                    si,
                    &PageMapQueryPagesInSection {
                        path,
                        include: Some(pp::and(pp::should_list_local(), vec![pp::kind_page()])),
                        ..Default::default()
                    },
                )
            }
            kinds::KIND_TERM => PageMap::get_pages_with_term(
                &self.h,
                si,
                &PageMapQueryPagesBelowPath {
                    path,
                    include: Some(pp::and(pp::should_list_local(), vec![pp::kind_page()])),
                    ..Default::default()
                },
            ),
            _ => self.site_regular_pages(si),
        }
    }

    /// Go `s.Pages()` (site.go).
    fn site_pages(&self, si: usize) -> Pages {
        PageMap::get_pages_in_section(
            &self.h,
            si,
            &PageMapQueryPagesInSection {
                path: String::new(),
                key_part: "global".into(),
                include: Some(pp::should_list_global()),
                recursive: true,
                include_self: true,
            },
        )
    }

    /// Go `s.RegularPages()` (site.go).
    fn site_regular_pages(&self, si: usize) -> Pages {
        PageMap::get_pages_in_section(
            &self.h,
            si,
            &PageMapQueryPagesInSection {
                path: String::new(),
                key_part: "global".into(),
                include: Some(pp::and(pp::should_list_global(), vec![pp::kind_page()])),
                recursive: true,
                include_self: false,
            },
        )
    }

    fn dump(&mut self) -> J {
        let h = self.h.clone();
        let render_formats: Vec<String> =
            h.render_formats.0.iter().map(|f| f.name.clone()).collect();

        let mut site_dumps = Vec::new();
        for (si, s) in h.sites.iter().enumerate() {
            let m = &s.page_map;
            let mut sd = serde_json::Map::new();
            sd.insert("lang".into(), json!(s.language.lang));
            sd.insert(
                "renderFormats".into(),
                json!(
                    s.render_formats
                        .0
                        .iter()
                        .map(|f| f.name.clone())
                        .collect::<Vec<_>>()
                ),
            );
            let home = s.home.map(|id| json!(self.idx(id))).unwrap_or(J::Null);
            sd.insert("home".into(), home);
            sd.insert("lastmod".into(), encode(&Value::Time(s.lastmod.clone())));
            sd.insert(
                "mainSections".into(),
                json!(s.conf.compiled().main_sections()),
            );

            let mut walk = Vec::new();
            let mut nodes: Vec<PageId> = Vec::new();
            let cfg = WalkConfig {
                dims: m.dims,
                ..Default::default()
            };
            h.page_trees
                .tree_pages
                .walk(&cfg, |_w, key, n, _m| {
                    nodes.push(n.page_id().unwrap());
                    walk.push((key.to_string(), n.page_id().unwrap()));
                    Ok(false)
                })
                .unwrap();
            let walk: Vec<J> = walk
                .into_iter()
                .map(|(k, id)| json!([k, self.idx(id)]))
                .collect();
            sd.insert("walk".into(), J::Array(walk));

            let mut bundled = Vec::new();
            h.page_trees
                .tree_resources
                .walk(&cfg, |_w, key, n, _m| {
                    if let ContentNode::Resource(rs) = n
                        && let Some(id) = rs.page
                    {
                        bundled.push((key.to_string(), id));
                    }
                    Ok(false)
                })
                .unwrap();
            let bundled: Vec<J> = bundled
                .into_iter()
                .map(|(k, id)| json!([k, self.idx(id)]))
                .collect();
            sd.insert(
                "bundled".into(),
                if bundled.is_empty() {
                    J::Null
                } else {
                    J::Array(bundled)
                },
            );

            let mut colls = Vec::new();
            for (i, &id) in nodes.iter().enumerate() {
                let (pages_l, regular_l) =
                    if h.page(id).meta.kind() == kinds::KIND_TERM && i % 2 == 1 {
                        let r = self.regular_pages_of(id);
                        (self.pages_of(id), r)
                    } else {
                        let p = self.pages_of(id);
                        (p, self.regular_pages_of(id))
                    };
                let pi = self.idx(id);
                let pl = self.list(&pages_l);
                let rl = self.list(&regular_l);
                colls.push(json!({"p": pi, "pages": pl, "regularPages": rl}));
            }
            sd.insert("collections".into(), J::Array(colls));
            let sp = self.site_pages(si);
            let sp = self.list(&sp);
            sd.insert("sitePages".into(), sp);
            let srp = self.site_regular_pages(si);
            let srp = self.list(&srp);
            sd.insert("siteRegularPages".into(), srp);

            let tl = site_taxonomies(&h, si);
            let mut taxos = Vec::new();
            for (k, t) in tl.iter() {
                let t = self.taxonomy(Some(t));
                taxos.push(json!([k, t]));
            }
            sd.insert("taxonomies".into(), J::Array(taxos));

            let mut datas = Vec::new();
            for &id in &nodes {
                let d = nh_hugolib::page__data::data(&self.handle(id));
                let Value::Map(d) = d else {
                    panic!("page.Data is a map")
                };
                let mut dd = Vec::new();
                for (k, v) in &d.entries {
                    let v = match v {
                        Value::String(s) => json!({"s": str_enc(s.as_bytes())}),
                        Value::List(_) | Value::TypedNil(_)
                            if nh_page::weighted::weighted_pages_from_value(v).is_some()
                                && v.go_type_name() == "page.WeightedPages" =>
                        {
                            let wp = match v {
                                Value::TypedNil(_) => None,
                                _ => nh_page::weighted::weighted_pages_from_value(v),
                            };
                            let w = self.weighted(wp.as_ref());
                            json!({ "weighted": w })
                        }
                        Value::Map(_) | Value::TypedNil(_)
                            if v.go_type_name() == "page.Taxonomy" =>
                        {
                            let t = match v {
                                Value::TypedNil(_) => None,
                                _ => nh_page::taxonomy::taxonomy_from_value(v),
                            };
                            let t = self.taxonomy(t.as_ref());
                            json!({ "taxonomy": t })
                        }
                        Value::Object(o)
                            if o.as_any()
                                .downcast_ref::<nh_page::page_data::LazyPages>()
                                .is_some() =>
                        {
                            json!({"func": true})
                        }
                        other => json!({"other": other.go_type_name()}),
                    };
                    dd.push(json!([str_enc(k.as_bytes()), v]));
                }
                let pi = self.idx(id);
                datas.push(json!({"p": pi, "data": dd}));
            }
            sd.insert("data".into(), J::Array(datas));

            let mut terms = Vec::new();
            for &id in &nodes {
                for v in &m.cfg.taxonomy_config.views {
                    let ts = PageMap::get_terms_for_page_in_taxonomy(
                        &h,
                        si,
                        &h.page(id).meta.path(),
                        &v.plural,
                    );
                    if ts.is_empty() {
                        continue;
                    }
                    let pi = self.idx(id);
                    let tl = self.list(&ts);
                    terms.push(json!({"p": pi, "taxonomy": v.plural, "terms": tl}));
                }
            }
            sd.insert("terms".into(), J::Array(terms));

            // GetPage.
            let mut refs: std::collections::BTreeSet<String> = [
                "",
                "/",
                "home",
                "section",
                "/_index.md",
                "_index.md",
                "index.md",
                "404",
                "/404",
                "sitemap",
                "/nope",
                "nope.md",
                "./x",
                "../x",
                ".",
                "..",
                "/tags/",
                "tags",
                "TAGS",
                "/categories",
                "p.md",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect();
            for &id in &nodes {
                let p = h.page(id);
                let path = p.meta.path();
                refs.insert(path.clone());
                refs.insert(format!("{path}/"));
                refs.insert(p.meta.path_info.path().to_string());
                refs.insert(go_unicode::strings::to_upper_str(&path).into_owned());
                refs.insert(p.meta.path_info.base_name_no_identifier().to_string());
                refs.insert(path.strip_prefix('/').unwrap_or(&path).to_string());
                refs.insert(p.meta.path_info.unnormalized().path().to_string());
                if let Some(f) = &p.meta.f {
                    refs.insert(f.path());
                    refs.insert(f.logical_name());
                }
            }
            let finder = new_page_finder(&h, si);
            let mut get_pages = Vec::new();
            for r in &refs {
                let res = finder.get_page_for_refs(std::slice::from_ref(r));
                let res = self.result(res);
                get_pages.push(json!([r, res]));
            }
            for rs in [
                vec!["section", "blog"],
                vec!["home", ""],
                vec!["home", "/"],
                vec!["page", "/about"],
                vec!["section", ""],
                vec!["a", "b", "c"],
                vec!["section", "s"],
                vec!["taxonomy", "tags"],
            ] {
                let rr: Vec<String> = rs.iter().map(|s| s.to_string()).collect();
                let res = finder.get_page_for_refs(&rr);
                let res = self.result(res);
                get_pages.push(json!([rs.join("|"), res]));
            }
            sd.insert("getPage".into(), J::Array(get_pages));

            let mut ctxs = Vec::new();
            for &id in &nodes {
                let p = h.page(id);
                let pi = p.meta.path_info.clone();
                let ctx = self.handle(id).page_ref();
                let crefs = [
                    format!("./{}", pi.base_name_no_identifier()),
                    format!("../{}", go_path::path::base(pi.container_dir())),
                    format!("{}.md", pi.base_name_no_identifier()),
                    ".".to_string(),
                    "..".to_string(),
                    "./index.md".to_string(),
                    "../_index.md".to_string(),
                ];
                for r in &crefs {
                    let a = finder.get_page(Some(&ctx), r);
                    let a = self.result(a);
                    let b = finder.get_page_ref(Some(&ctx), r);
                    let b = self.result(b);
                    let i = self.idx(id);
                    ctxs.push(json!([i, r, a, b]));
                }
            }
            sd.insert("getPageCtx".into(), J::Array(ctxs));

            let mut ress = Vec::new();
            for &id in &nodes {
                let rr = PageMap::get_or_create_resources_for_page(&h, id);
                if rr.is_empty() {
                    continue;
                }
                let mut out = Vec::new();
                for r in &rr {
                    if let Some(hd) = r.as_any().downcast_ref::<PageHandle>() {
                        out.push(json!({"page": self.idx(hd.id)}));
                        continue;
                    }
                    out.push(json!({
                        "type": r.resource_type(),
                        "name": r.name(),
                        "nameNorm": nh_resource::resourcetypes::name_normalized_or_name(&**r),
                        "title": r.title(),
                        "params": encode_params(&r.params()),
                        "relPermalink": r.rel_permalink(),
                        "permalink": r.permalink(),
                    }));
                }
                let pi = self.idx(id);
                ress.push(json!({"p": pi, "resources": out}));
            }
            sd.insert("resources".into(), J::Array(ress));

            let mut keys = m.cache_pages1.keys();
            keys.sort();
            sd.insert("cachePages1".into(), json!(keys));
            let mut keys = m.cache_get_terms.keys();
            keys.sort();
            sd.insert("cacheGetTerms".into(), json!(keys));
            let mut keys = m.cache_resources.keys();
            keys.sort();
            sd.insert("cacheResources".into(), json!(keys));

            site_dumps.push(J::Object(sd));
        }

        let mut page_dumps = Vec::new();
        let mut i = 0;
        while i < self.pages.len() {
            let id = self.pages[i];
            i += 1;
            page_dumps.push(self.page_dump(id));
        }

        json!({
            "renderFormats": render_formats,
            "sites": site_dumps,
            "pages": page_dumps,
        })
    }

    fn page_dump(&mut self, id: PageId) -> J {
        let h = self.h.clone();
        let p = h.page(id);
        let m = &p.meta;
        let pc = &m.page_config;
        let file =
            m.f.as_ref()
                .map(|f| (self.norm)(f.filename()))
                .unwrap_or_default();
        let cascade = match &pc.cascade_compiled {
            None => J::Null,
            Some(c) => {
                let mut out = Vec::new();
                c.range(|k, v| {
                    out.push(json!({
                        "target": {"path": k.path, "kind": k.kind, "lang": k.lang, "environment": k.environment},
                        "params": encode_params(&v.params),
                        "fields": encode_params(&v.fields),
                    }));
                    true
                });
                J::Array(out)
            }
        };
        let params = match &pc.params {
            None => json!({"t": "nil:maps.Params"}),
            Some(p) => encode_params(p),
        };
        let time = |t: &go_value::Time| encode(&Value::Time(t.clone()));
        let mut pd = serde_json::Map::new();
        pd.insert("site".into(), json!(p.site_idx));
        pd.insert("kind".into(), json!(m.kind()));
        pd.insert("path".into(), json!(m.path()));
        pd.insert("pathInfo".into(), json!(m.path_info.path()));
        pd.insert("lang".into(), json!(m.lang()));
        pd.insert("file".into(), json!(file));
        pd.insert("title".into(), json!(m.title()));
        pd.insert("linkTitle".into(), json!(m.link_title()));
        pd.insert("name".into(), json!(m.name()));
        pd.insert("type".into(), json!(m.page_type()));
        pd.insert("section".into(), json!(m.section()));
        pd.insert("layout".into(), json!(m.layout()));
        pd.insert("description".into(), json!(m.description()));
        pd.insert("summary".into(), json!(pc.summary));
        pd.insert("slug".into(), json!(pc.slug));
        pd.insert("url".into(), json!(pc.url));
        pd.insert("weight".into(), json!(pc.weight));
        pd.insert("draft".into(), json!(pc.draft));
        pd.insert("isCJK".into(), json!(pc.is_cjk_language));
        pd.insert("translationKey".into(), json!(pc.translation_key));
        pd.insert("keywords".into(), json!(pc.keywords));
        pd.insert("aliases".into(), json!(pc.aliases));
        pd.insert("outputs".into(), json!(pc.outputs));
        pd.insert(
            "outputFormats".into(),
            json!(
                pc.configured_output_formats
                    .0
                    .iter()
                    .map(|f| f.name.clone())
                    .collect::<Vec<_>>()
            ),
        );
        pd.insert("markup".into(), json!(pc.content.markup));
        pd.insert("mediaType".into(), json!(pc.content_media_type.typ));
        pd.insert("term".into(), json!(m.term));
        pd.insert("singular".into(), json!(m.singular));
        pd.insert("resourcePath".into(), json!(m.resource_path));
        pd.insert("bundled".into(), json!(m.bundled));
        pd.insert(
            "standalone".into(),
            json!(
                m.standalone_output_format
                    .as_ref()
                    .map(|f| f.name.clone())
                    .unwrap_or_default()
            ),
        );
        pd.insert(
            "dates".into(),
            json!({
                "date": time(&pc.dates.date),
                "lastmod": time(&pc.dates.lastmod),
                "publishDate": time(&pc.dates.publish_date),
                "expiryDate": time(&pc.dates.expiry_date),
            }),
        );
        pd.insert(
            "build".into(),
            json!({"list": pc.build.list, "render": pc.build.render, "publishResources": pc.build.publish_resources}),
        );
        pd.insert(
            "sitemap".into(),
            json!({
                "changeFreq": pc.sitemap.change_freq,
                "priority": encode(&Value::float64(pc.sitemap.priority)),
                "filename": pc.sitemap.filename,
                "disable": pc.sitemap.disable,
            }),
        );
        pd.insert("params".into(), params);
        pd.insert("cascade".into(), cascade);
        pd.insert("resourcesMeta".into(), json!(pc.resources_meta.len()));
        pd.insert("noLink".into(), json!(m.no_link()));
        pd.insert("noRender".into(), json!(m.no_render()));
        pd.insert(
            "shouldList".into(),
            json!([m.should_list(false), m.should_list(true)]),
        );
        pd.insert("setMetaPost".into(), json!(m.set_meta_post_count));

        let lazy = match p.init_page(&h) {
            Ok(l) => l,
            Err(e) => {
                pd.insert("initErr".into(), json!(e.to_string()));
                return J::Object(pd);
            }
        };

        match p.common.target_path_descriptor.get() {
            None => {
                pd.insert("tpd".into(), J::Null);
            }
            Some(d) => {
                let sec = d
                    .section
                    .as_ref()
                    .map(|s| s.path().to_string())
                    .unwrap_or_default();
                pd.insert(
                    "tpd".into(),
                    json!({
                        "kind": d.kind, "path": d.path.path(), "section": sec, "baseName": d.base_name,
                        "prefixFilePath": d.prefix_file_path, "prefixLink": d.prefix_link, "forcePrefix": d.force_prefix,
                        "url": d.url, "expandedPermalink": d.expanded_permalink, "uglyURLs": d.ugly_urls,
                    }),
                );
            }
        }

        let ofs: Vec<J> = lazy
            .paths
            .output_formats
            .iter()
            .map(|of| {
                json!({"name": of.name(), "rel": of.rel, "relPermalink": of.rel_permalink(), "permalink": of.permalink()})
            })
            .collect();
        pd.insert("pageOutputFormats".into(), J::Array(ofs));

        let mut outs = Vec::new();
        for (oi, po) in lazy.outputs.iter().enumerate() {
            let slot = (0..oi)
                .find(|&j| Arc::ptr_eq(&lazy.outputs[j], po))
                .unwrap_or(oi);
            let tp = &po.target_paths.paths;
            outs.push(json!({
                "slot": slot,
                "name": po.f.name,
                "render": po.render,
                "paginator": po.paginator.is_some(),
                "relURL": po.target_paths.rel_url,
                "target": {
                    "filename": tp.target_filename,
                    "subTarget": tp.sub_resource_base_target,
                    "subLink": tp.sub_resource_base_link,
                    "link": tp.link,
                },
                "relPermalink": po.target_paths.output_format.rel_permalink(),
                "permalink": po.target_paths.output_format.permalink(),
            }));
        }
        pd.insert("outputs".into(), J::Array(outs));
        pd.insert(
            "currentOutput".into(),
            json!(
                p.current_output_idx
                    .load(std::sync::atomic::Ordering::SeqCst)
            ),
        );

        let parent = nh_hugolib::page__tree::parent_id(&h, id).map(|i| self.idx(i));
        pd.insert("parent".into(), json!(parent));
        let cs = nh_hugolib::page__tree::current_section_id(&h, id).map(|i| self.idx(i));
        pd.insert("currentSection".into(), json!(cs));
        let fs = nh_hugolib::page__tree::first_section_id(&h, id).map(|i| self.idx(i));
        pd.insert("firstSection".into(), json!(fs));
        pd.insert(
            "sectionsPath".into(),
            json!(nh_hugolib::page__tree::sections_path_id(&h, id)),
        );
        pd.insert(
            "sectionsEntries".into(),
            json!(nh_hugolib::page__tree::sections_entries_id(&h, id).unwrap_or_default()),
        );
        if m.is_node() {
            let secs = nh_hugolib::page__tree::sections(&self.handle(id));
            let l = self.list(&secs);
            pd.insert("sections".into(), l);
        }

        J::Object(pd)
    }
}

/// Accepts Go's random main-section choice: any candidate with the highest page count.
/// When the fixture records a tie (`mainSectionsTie`: Go chose one of these at random, its
/// `mainSections` is then null), the port's single main section must be one of them.
fn accept_main_sections(want: &J, got: &mut J) {
    let Some(sites) = got["dump"]["sites"].as_array_mut() else {
        return;
    };
    for (i, s) in sites.iter_mut().enumerate() {
        let tie = &want["dump"]["sites"][i]["mainSectionsTie"];
        if let Some(t) = tie.as_array()
            && let Some(ga) = s["mainSections"].as_array()
            && ga.len() == 1
            && t.contains(&ga[0])
        {
            s["mainSections"] = J::Null;
        }
        s.as_object_mut()
            .unwrap()
            .insert("mainSectionsTie".into(), tie.clone());
    }
}

fn run_case(name: &str) {
    let fx = fixture(&format!("assemble/{name}.json.gz"));
    let tmp = TempDir::new(&format!("assemble-{name}"));
    let mut b = new_sites(&fx["site"], &tmp.0).unwrap_or_else(|e| panic!("{name}: {e}"));
    let mut res = nh_hugolib::build_process::process(&mut b.h, &BuildCfg::default());
    if res.is_ok() {
        res = nh_hugolib::build_assemble::assemble(&mut b.h, &BuildCfg::default());
    }
    let Built { h, dir, log } = b;
    let mut got = serde_json::Map::new();
    got.insert("site".into(), fx["site"].clone());
    match res {
        Ok(()) => {
            let h = h.freeze();
            let d = dir.clone();
            let mut dumper = Dumper {
                h,
                norm: Box::new(move |s: &str| s.replace(&d, "/SITE")),
                pages: Vec::new(),
                seen: HashMap::new(),
            };
            got.insert("dump".into(), dumper.dump());
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
    let mut got = J::Object(got);
    accept_main_sections(&fx, &mut got);
    if let Ok(p) = std::env::var("NH_T21_DUMP") {
        std::fs::write(
            format!("{p}/{name}.got.json"),
            serde_json::to_string_pretty(&got).unwrap(),
        )
        .unwrap();
    }
    if let Some(d) = first_diff(name, &fx, &got) {
        panic!("{name}: assembly differs from Go at {d}");
    }
}

#[test]
fn assemble_docs() {
    run_case("docs");
}

#[test]
fn assemble_testsite() {
    run_case("testsite");
}

#[test]
fn assemble_synthetic() {
    run_case("synthetic");
}

#[test]
fn assemble_seeksnack() {
    run_case("seeksnack");
}

#[test]
fn assemble_shortcodes() {
    run_case("shortcodes");
}

#[test]
fn assemble_edge_tree() {
    run_case("edge-tree");
}

#[test]
fn assemble_contentdir() {
    run_case("contentdir");
}

#[test]
fn assemble_homeleaf() {
    run_case("homeleaf");
}

#[test]
fn assemble_nokinds() {
    run_case("nokinds");
}

#[test]
fn assemble_content() {
    run_case("content");
}

#[test]
fn assemble_asm_taxo() {
    run_case("asm-taxo");
}

#[test]
fn assemble_asm_build() {
    run_case("asm-build");
}

#[test]
fn assemble_asm_flags() {
    run_case("asm-flags");
}

#[test]
fn assemble_asm_cascade() {
    run_case("asm-cascade");
}

#[test]
fn assemble_asm_i18n() {
    run_case("asm-i18n");
}

#[test]
fn assemble_asm_multihost() {
    run_case("asm-multihost");
}

#[test]
fn assemble_asm_ugly() {
    run_case("asm-ugly");
}
