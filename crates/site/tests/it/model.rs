//! The T23a acceptance cases by name, the tree and cascade helpers, and real sites.

use serde_json::Value as J;
use ssg_base::paths::ContentKey;
use ssg_base::{Idx, LangIdx, PageKind, Value};
use ssg_page::{Cascade, ListMode, RenderMode};
use ssg_site::{CascadeIndex, Model, PageRole, SiteTree};
use ssg_testkit::fixture::oracle;

use crate::support::Site;

fn key(s: &str) -> ContentKey {
    ContentKey::from_source(s)
}

fn seeksnack() -> (Site, Model) {
    let f: J = oracle("oracle/hugolib/assemble/seeksnack.json.gz");
    let site = Site::new(&f["site"]);
    let m = site.model().unwrap();
    (site, m)
}

fn lang(m: &Model, k: &str) -> LangIdx {
    m.config.site(k).unwrap().lang
}

/// R `kind-override`, `lang-override` (`lang: TH`) and `path-override`.
#[test]
fn front_matter_overrides() {
    let (_site, m) = seeksnack();
    let (en, th) = (lang(&m, "en"), lang(&m, "th"));

    let kind = m.sites[en].tree.get(&key("blog/kind-override")).unwrap();
    assert_eq!(m.page(kind).kind, PageKind::Section);

    assert!(m.sites[en].tree.get(&key("blog/lang-override")).is_none());
    let moved = m.sites[th].tree.get(&key("blog/lang-override")).unwrap();
    assert_eq!(m.page(moved).lang, th);
    assert_eq!(
        m.page(moved).params().get("lang"),
        Some(&Value::string("th"))
    );

    // A language that is not enabled (`fr` is disabled) leaves the page where it was.
    let disabled = m.sites[en].tree.get(&key("blog/lang-disabled")).unwrap();
    assert_eq!(
        m.page(disabled).params().get("lang"),
        Some(&Value::string("fr"))
    );

    assert!(m.sites[en].tree.get(&key("blog/path-override")).is_none());
    let path = m.sites[en].tree.get(&key("custom/moved")).unwrap();
    let src = m.page(path).source.as_ref().unwrap();
    assert_eq!(src.file.rel, "blog/path-override.md");
    assert_eq!(src.info.key, key("custom/moved"));
}

/// Drafts, future and expired pages are removed; a headless page stays, switched off.
#[test]
fn drafts_future_expired() {
    let (_site, m) = seeksnack();
    let (en, th) = (lang(&m, "en"), lang(&m, "th"));
    for k in ["biscuit/hello-panda", "biscuit/future", "biscuit/expired"] {
        assert!(m.sites[en].tree.get(&key(k)).is_none(), "{k}");
    }
    // The Thai translation of the English draft is not a draft.
    assert!(m.sites[th].tree.get(&key("biscuit/hello-panda")).is_some());
    // The draft's bundle files in its language go with it.
    assert!(
        !m.sites[en]
            .resources
            .contains_key(&key("biscuit/hello-panda/panda.png"))
    );
    let headless = m.page(m.sites[en].tree.get(&key("blog/headless")).unwrap());
    assert_eq!(headless.meta.build.list, ListMode::Never);
    assert_eq!(headless.meta.build.render, RenderMode::Never);
}

/// Content files inside leaf bundles are bundled pages owned by the bundle's index page.
#[test]
fn bundle_roles() {
    let (_site, m) = seeksnack();
    let (en, th) = (lang(&m, "en"), lang(&m, "th"));
    let notes = m.sites[th].resources[&key("biscuit/koalas-march-chocolate/notes.md")];
    let page = m.bundle_resources[notes].page.unwrap();
    assert_eq!(
        m.page(page).role,
        PageRole::Bundled {
            bundle: key("biscuit/koalas-march-chocolate")
        }
    );
    assert_eq!(
        m.bundle_owner(page),
        m.sites[th].tree.get(&key("biscuit/koalas-march-chocolate"))
    );
    let img = m.sites[en].resources[&key("biscuit/koalas-march-chocolate/koala.jpg")];
    assert!(m.bundle_resources[img].page.is_none());
}

/// The cascade of `/blog` reaches its pages, not the pages it does not target.
#[test]
fn cascade_down_the_tree() {
    let (_site, m) = seeksnack();
    let en = lang(&m, "en");
    let get = |k: &str| m.page(m.sites[en].tree.get(&key(k)).unwrap());
    let banner = |k: &str| get(k).params().get("banner").cloned();
    assert_eq!(
        banner("blog/nested/deeper/post"),
        Some(Value::string("blog.jpg"))
    );
    assert_eq!(banner("blog/kind-override"), None, "a section");
    assert_eq!(banner("custom/moved"), None, "moved out of /blog");
    assert!(!m.sites[en].cascade.received(&key("blog/x")).is_empty());
}

#[test]
fn site_tree_segments() {
    let mut t = SiteTree::default();
    let id = |i: usize| <ssg_base::PageId as Idx>::from_index(i);
    assert!(t.insert(key(""), id(0)));
    assert!(t.insert(key("blog"), id(1)));
    assert!(t.insert(key("blog/post"), id(2)));
    assert!(t.insert(key("blog-x"), id(3)));
    assert!(!t.insert(key("blog"), id(4)), "a taken key");
    assert_eq!(
        t.longest_prefix(&key("blog/post/img.jpg")).unwrap().1,
        id(2)
    );
    assert_eq!(t.longest_prefix(&key("blog-y")).unwrap().1, id(0));
    assert_eq!(
        t.longest_prefix(&key("blogx/a")).unwrap().1,
        id(0),
        "segment-wise"
    );
    let below: Vec<_> = t.descendants(&key("blog")).map(|(_, p)| p).collect();
    assert_eq!(below, [id(2)]);
    assert_eq!(t.descendants(&key("")).count(), 3);
}

#[test]
fn cascade_index_nearest_branch() {
    let site = Cascade::decode(&Value::from_toml_str("[params]\na = 1").unwrap()).unwrap();
    let own = Cascade::decode(&Value::from_toml_str("[params]\nb = 2").unwrap()).unwrap();
    let mut idx = CascadeIndex::new(site.clone());
    idx.add_branch(&key("docs"), &own);
    assert_eq!(idx.received(&key("")), &site);
    assert_eq!(idx.received(&key("docs")), &site);
    let below = idx.received(&key("docs/a/b"));
    assert_eq!(below.rules().len(), 1, "same target: merged");
    assert!(below.rules()[0].params.get("a").is_some());
    assert!(below.rules()[0].params.get("b").is_some());
    assert_eq!(idx.received(&key("docsx")), &site);
}

/// Loading twice gives the same model (parallel capture and meta are order-independent).
#[test]
fn deterministic() {
    let (site, a) = seeksnack();
    let b = site.model().unwrap();
    let summary = |m: &Model| {
        m.pages
            .iter()
            .map(|p| (p.id, p.lang, p.key.clone(), p.kind))
            .collect::<Vec<_>>()
    };
    assert_eq!(summary(&a), summary(&b));
}

/// Real sites written by `tools/rust-port/i01/sites.py make <site> <dir>`: set
/// `FUGO_SITES=<dir>[:<dir>…]` and run with `--ignored`.
#[test]
#[ignore = "needs sites written by sites.py (FUGO_SITES)"]
fn real_sites() {
    let dirs = std::env::var("FUGO_SITES").expect("FUGO_SITES");
    for dir in dirs.split(':') {
        let tmp = tempfile::tempdir().unwrap();
        let site = Site::at(tmp, dir.into());
        let start = std::time::Instant::now();
        let m = site.model().unwrap_or_else(|e| panic!("{dir}: {e}"));
        let per_lang: Vec<(String, usize, usize)> = m
            .sites
            .iter()
            .map(|s| {
                (
                    m.config.sites[s.lang].language.key.clone(),
                    s.tree.len(),
                    s.resources.len(),
                )
            })
            .collect();
        eprintln!(
            "{dir}: {} pages, {} bundle files, per language (key, pages, files) {per_lang:?}, \
             {} data keys, {} diagnostics, {:?}",
            m.pages.len(),
            m.bundle_resources.len(),
            m.data.len(),
            m.diagnostics.len(),
            start.elapsed()
        );
        for d in &m.diagnostics {
            eprintln!("  {d}");
        }
        // T23b: made pages, terms, outputs and target collisions.
        let mut made: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for p in m.pages.iter().filter(|p| p.source.is_none()) {
            *made.entry(p.kind.as_str()).or_default() += 1;
        }
        let terms: usize = m
            .sites
            .iter()
            .flat_map(|s| s.taxonomies.iter())
            .map(|t| t.terms.len())
            .sum();
        let mut targets: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        let mut outputs = 0;
        for p in m.pages.iter().filter(|p| p.rendered()) {
            for u in &p.urls {
                outputs += 1;
                targets
                    .entry(u.paths.target.to_string())
                    .or_default()
                    .push(format!(
                        "{} ({})",
                        p.path(),
                        m.config.sites[p.lang].language.key
                    ));
            }
        }
        let collisions: Vec<_> = targets.iter().filter(|(_, v)| v.len() > 1).collect();
        let resources = m
            .bundle_resources
            .iter()
            .filter(|r| r.target_base.is_some())
            .count();
        eprintln!(
            "{dir}: made pages {made:?}, {terms} terms, {outputs} (page, format) outputs, \
             {resources} placed bundle files, {} shared target files",
            collisions.len()
        );
        for (t, pages) in collisions.iter().take(20) {
            eprintln!("  {t}: {pages:?}");
        }
    }
}
