//! The template store against the Go oracle (`tools/go-oracle/nh-tplimpl/store`): for every
//! site (this repository's docs/ and hugolib/testsite sites, the legacy, modern and themes
//! layout trees), the Rust store built from the same layouts filesystem and configuration has
//! the same templates (key, category, sub-category, name, descriptor, parse info, base
//! variants, overlays, content), the same shortcodes tree, templatesByPath, shortcodesByName,
//! templates() and unused templates; the layouts filesystem walks like Go's.

mod support;

use support::{SITES, diff_json, dump_store, integration_fixtures, site, site_from_fixture};

fn check_site(name: &'static str) {
    check(name, site(name));
}

fn check(name: &str, s: support::Site) {
    let go_store = s.fx["store"].clone();

    let walk = s.layouts_walk();
    let mut diffs = Vec::new();
    diff_json("layoutsWalk", &go_store["layoutsWalk"], &walk, &mut diffs);
    assert!(
        diffs.is_empty(),
        "{name}: layouts walk:\n{}",
        diffs.join("\n")
    );

    let store = s.store();
    let dump = dump_store(&store);
    let n = |k: &str| dump[k].as_array().map(|a| a.len()).unwrap_or(0);
    eprintln!(
        "{name}: {} main entries, {} shortcodes, {} by path, {} templates",
        n("main"),
        n("shortcodes"),
        n("byPath"),
        n("templates")
    );
    for k in [
        "main",
        "shortcodes",
        "byPath",
        "shortcodesByName",
        "templates",
        "unused",
    ] {
        let mut diffs = Vec::new();
        diff_json(k, &go_store[k], &dump[k], &mut diffs);
        assert!(diffs.is_empty(), "{name}: {k}:\n{}", diffs.join("\n"));
    }
}

#[test]
fn store_docs() {
    support::big_stack(|| check_site("docs"));
}

#[test]
fn store_testsite() {
    support::big_stack(|| check_site("testsite"));
}

#[test]
fn store_legacy() {
    support::big_stack(|| check_site("legacy"));
}

#[test]
fn store_modern() {
    support::big_stack(|| check_site("modern"));
}

#[test]
fn store_themes() {
    support::big_stack(|| check_site("themes"));
}

/// The layout trees of tplimpl's integration tests (tsupport.IntegrationSites).
#[test]
fn store_integration() {
    support::big_stack(|| {
        let fxs = integration_fixtures("store");
        assert!(fxs.len() > 50, "{} integration sites", fxs.len());
        for (name, fx) in fxs {
            check(&name, site_from_fixture(fx));
        }
    });
}

#[test]
fn store_sites_covered() {
    for name in SITES {
        assert!(
            support::fixture_dir("store")
                .join(format!("{name}.json.gz"))
                .exists()
        );
    }
}
