//! T20 acceptance: the capture phase (`HugoSites::new` + `process`) against the Go oracle
//! `tools/go-oracle/nh-hugolib/capture`. For each recorded site the trees (`treePages`,
//! `treeResources` in walk order, with node kinds and language slots), every page reached from
//! them (kind, path info, language, file, the PageConfig filled by setMetaPre, the parsed
//! content items with the shortcodes) and the log must be identical.

mod support;

use std::collections::HashMap;

use nh_common::paths::pathparser::{Path, PathType};
use nh_hugolib::HugoSites;
use nh_hugolib::content_map::ResourceSource;
use nh_hugolib::content_map_trees::ContentNode;
use nh_hugolib::hugo_sites_build::BuildCfg;
use nh_hugolib::page::PageId;
use nh_hugolib::page__content_parse::ContentItem;
use nh_hugolib::shortcode_parse::{SHORTCODE_PLACEHOLDER_PREFIX, Shortcode, ShortcodeInner};
use serde_json::{Value as J, json};
use support::*;

/// The Go `paths.PathType` value (iota order).
fn path_type(t: PathType) -> i64 {
    match t {
        PathType::File => 0,
        PathType::ContentResource => 1,
        PathType::ContentSingle => 2,
        PathType::Leaf => 3,
        PathType::Branch => 4,
        PathType::ContentData => 5,
        PathType::Markup => 6,
        PathType::Shortcode => 7,
        PathType::Partial => 8,
        PathType::Baseof => 9,
    }
}

fn path_dump(p: &Path) -> J {
    json!({
        "path": p.path(),
        "base": p.base(),
        "unnormalizedPath": p.unnormalized().path(),
        "unnormalizedBase": p.unnormalized().base(),
        "type": path_type(p.path_type()),
        "lang": p.lang(),
        "section": p.section(),
        "baseNameNoIdentifier": p.base_name_no_identifier(),
        "ext": p.ext(),
        "component": p.component(),
        "identifiers": p.identifiers(),
        "disabled": p.disabled(),
    })
}

/// The Rust side of the oracle's dump (overlay_capture.go.txt).
struct Dumper<'a> {
    h: &'a HugoSites,
    b: &'a Built,
    pages: Vec<PageId>,
    seen: HashMap<PageId, usize>,
}

impl Dumper<'_> {
    fn add_page(&mut self, id: PageId) -> usize {
        if let Some(i) = self.seen.get(&id) {
            return *i;
        }
        self.seen.insert(id, self.pages.len());
        self.pages.push(id);
        self.pages.len() - 1
    }

    fn page_node(&mut self, id: PageId) -> J {
        let lang = self.h.page(id).site_idx;
        json!({"t": "page", "lang": lang, "page": self.add_page(id)})
    }

    fn res_node(&mut self, r: &ResourceSource) -> J {
        let mut m = serde_json::Map::new();
        m.insert("t".into(), json!("res"));
        m.insert("lang".into(), json!(r.lang_index));
        if let Some(p) = &r.path {
            m.insert("path".into(), json!(p.path()));
        }
        if let Some(fi) = &r.fi {
            m.insert("filename".into(), json!(self.b.norm(&fi.meta().filename)));
        }
        if let Some(id) = r.page {
            m.insert("page".into(), json!(self.add_page(id)));
        }
        J::Object(m)
    }

    fn node(&mut self, n: &ContentNode) -> J {
        match n {
            ContentNode::Page(id, _) => self.page_node(*id),
            ContentNode::Pages(v) => {
                let nodes: Vec<J> = v
                    .iter()
                    .map(|p| match p {
                        None => J::Null,
                        Some(id) => self.page_node(*id),
                    })
                    .collect();
                json!({"t": "pages", "nodes": nodes})
            }
            ContentNode::Resource(r) => self.res_node(r),
            ContentNode::Resources(v) => {
                let nodes: Vec<J> = v
                    .iter()
                    .map(|r| match r {
                        None => J::Null,
                        Some(r) => self.res_node(r),
                    })
                    .collect();
                json!({"t": "resources", "nodes": nodes})
            }
        }
    }

    fn sc(&self, sc: &Shortcode, pid: u64) -> J {
        let inner: Vec<J> = sc
            .inner
            .iter()
            .map(|i| match i {
                ShortcodeInner::Text(t) => json!({"text": String::from_utf8_lossy(t)}),
                ShortcodeInner::Shortcode(s) => json!({"sc": self.sc(s, pid)}),
            })
            .collect();
        let prefix = format!("{SHORTCODE_PLACEHOLDER_PREFIX}{pid}");
        json!({
            "name": sc.name,
            "ordinal": sc.ordinal,
            "doMarkup": sc.do_markup,
            "isInline": sc.is_inline,
            "isClosing": sc.is_closing,
            "params": encode(&sc.params),
            "inner": inner,
            "indentation": String::from_utf8_lossy(&sc.indentation),
            "pos": sc.pos,
            "length": sc.length,
            "placeholder": sc.placeholder.replacen(&prefix, &format!("{SHORTCODE_PLACEHOLDER_PREFIX}PID"), 1),
            "templ": sc.templ.as_ref().map(|t| t.name()),
            "insertPlaceholder": sc.insert_placeholder(),
            "needsInner": sc.needs_inner(),
            "configVersion": sc.config_version(),
        })
    }

    fn page(&self, id: PageId) -> J {
        let p = self.h.page(id);
        let m = &p.meta;
        let mut pd = json!({
            "kind": m.kind(),
            "lang": m.lang(),
            "siteIdx": p.site_idx,
            "term": m.term,
            "singular": m.singular,
            "bundled": m.bundled,
            "resourcePath": m.resource_path,
            "standalone": m.is_standalone(),
            "pathInfo": path_dump(&m.path_info),
        });
        if let Some(f) = &m.f {
            pd["file"] = json!({
                "filename": self.b.norm(f.filename()),
                "path": f.path(),
                "lang": f.lang(),
            });
        }
        let pc = &m.page_config;
        let cascade = pc.cascade_compiled.as_ref().map(|c| {
            let mut out = Vec::new();
            c.range(|k, v| {
                out.push(json!({
                    "target": {"path": k.path, "kind": k.kind, "lang": k.lang, "environment": k.environment},
                    "params": encode_params(&v.params),
                    "fields": encode_params(&v.fields),
                }));
                true
            });
            out
        });
        pd["pageConfig"] = json!({
            "kind": pc.kind,
            "path": pc.path,
            "lang": pc.lang,
            "params": pc.params.as_ref().map(encode_params).unwrap_or(json!({"t": "nil:maps.Params"})),
            "cascade": cascade,
        });
        let c = p.content.as_ref().expect("content");
        let pi = &c.pi;
        let items: Vec<J> = pi
            .items_step2
            .iter()
            .map(|it| match it {
                ContentItem::Source { low, high } => json!({"t": "src", "lo": low, "hi": high}),
                ContentItem::Replacement(v) => {
                    json!({"t": "repl", "val": String::from_utf8_lossy(v)})
                }
                ContentItem::Shortcode(s) => json!({"t": "sc", "sc": self.sc(s, pi.pid)}),
            })
            .collect();
        let names: Vec<String> = c.shortcode_state.names();
        let source_key = if m.f.is_none() {
            "PID".to_string()
        } else {
            self.b.norm(&pi.source_key)
        };
        pd["content"] = json!({
            "sourceKey": source_key,
            "hasOpenSource": pi.open_source.is_some(),
            "frontMatterNil": pi.front_matter.is_none(),
            "hasSummaryDivider": pi.has_summary_divider,
            "posMainContent": pi.pos_main_content,
            "hasNonMarkdownShortcode": pi.has_non_markdown_shortcode,
            "items": items,
            "shortcodeNames": names,
            "numShortcodes": c.shortcode_state.shortcodes.len(),
            "filename": self.b.norm(&c.shortcode_state.filename),
            "enableInlineShortcodes": c.shortcode_state.enable_inline_shortcodes,
            "enableEmoji": c.enable_emoji,
        });
        pd
    }
}

fn dump(b: &Built) -> J {
    let h = &b.h;
    let mut d = Dumper {
        h,
        b,
        pages: Vec::new(),
        seen: HashMap::new(),
    };
    let mut trees = serde_json::Map::new();
    for (name, tree) in [
        ("pages", &h.page_trees.tree_pages),
        ("resources", &h.page_trees.tree_resources),
    ] {
        let mut entries = Vec::new();
        tree.walk_prefix_raw("", &mut |key, n| {
            entries.push(json!({"key": key, "node": d.node(n)}));
            false
        });
        trees.insert(name.to_string(), J::Array(entries));
    }
    // Each site's (shaped) walk.
    let mut site_walks = Vec::new();
    for s in &h.sites {
        let mut sw = serde_json::Map::new();
        for (name, tree) in [
            ("pages", &h.page_trees.tree_pages),
            ("resources", &h.page_trees.tree_resources),
        ] {
            let mut entries = Vec::new();
            let cfg = nh_doctree::nodeshifttree::WalkConfig {
                dims: s.page_map.dims,
                ..Default::default()
            };
            tree.walk(&cfg, |_w, key, n, m| {
                entries.push(json!({"key": key, "node": d.node(n), "match": m.0}));
                Ok(false)
            })
            .unwrap();
            sw.insert(name.to_string(), J::Array(entries));
        }
        site_walks.push(J::Object(sw));
    }
    let pages: Vec<J> = d.pages.clone().into_iter().map(|id| d.page(id)).collect();
    json!({"trees": trees, "siteWalks": site_walks, "pages": pages})
}

fn run_case(name: &str) {
    let fx = fixture(&format!("capture/{name}.json.gz"));
    let tmp = TempDir::new(&format!("capture-{name}"));
    let mut b = new_sites(&fx["site"], &tmp.0).unwrap_or_else(|e| panic!("{name}: {e}"));
    let res = nh_hugolib::build_process::process(&mut b.h, &BuildCfg::default());
    let mut got = serde_json::Map::new();
    got.insert("site".into(), fx["site"].clone());
    match res {
        Ok(()) => {
            got.insert("dump".into(), dump(&b));
        }
        Err(e) => {
            got.insert("err".into(), json!(b.norm(&e.to_string())));
        }
    }
    got.insert("log".into(), json!(b.log_lines()));
    let got = J::Object(got);
    if let Some(d) = first_diff(name, &fx, &got) {
        panic!("{name}: capture differs from Go at {d}");
    }
}

#[test]
fn capture_docs() {
    run_case("docs");
}

#[test]
fn capture_testsite() {
    run_case("testsite");
}

#[test]
fn capture_synthetic() {
    run_case("synthetic");
}

#[test]
fn capture_seeksnack() {
    run_case("seeksnack");
}

#[test]
fn capture_edge_tree() {
    run_case("edge-tree");
}

#[test]
fn capture_contentdir() {
    run_case("contentdir");
}

#[test]
fn capture_homeleaf() {
    run_case("homeleaf");
}

#[test]
fn capture_nokinds() {
    run_case("nokinds");
}

#[test]
fn capture_shortcodes() {
    run_case("shortcodes");
}

#[test]
fn capture_shortcode_errors() {
    for name in [
        "sc-err-notfound",
        "sc-err-closing",
        "sc-err-unclosed",
        "sc-err-noname",
        "sc-err-mixed-params",
    ] {
        run_case(name);
    }
}
