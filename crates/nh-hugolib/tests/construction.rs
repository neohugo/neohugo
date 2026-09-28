//! T20 acceptance: `NewHugoSites` on the reconstructed seeksnack config
//! (crates/nh-allconfig/tests/fixtures/load/seeksnack/hugo.toml, as recorded in the capture
//! fixture) builds 2 sites (en, th) with their Deps, template stores and translators, using
//! the names-only func map; `freeze` binds each site's `page.Site`.

mod support;

use std::sync::Arc;

use go_value::Value;
use support::*;

#[test]
fn new_hugo_sites_seeksnack() {
    let fx = fixture("capture/seeksnack.json.gz");
    let tmp = TempDir::new("construction");
    let b = new_sites(&fx["site"], &tmp.0).unwrap();
    let h = &b.h;

    // 2 sites in language order (default content language first).
    let langs: Vec<&str> = h.sites.iter().map(|s| s.language.lang.as_str()).collect();
    assert_eq!(langs, ["en", "th"]);
    for (i, s) in h.sites.iter().enumerate() {
        assert_eq!(s.idx, i);
        assert_eq!(s.page_map.site_idx, i);
        assert_eq!(s.page_map.dims, [i]);
        assert_eq!(s.deps.conf.language().lang, s.language.lang);
        assert_eq!(s.page_map.cfg.lang, s.language.lang);
        // Taxonomy views sorted by plural, keyed by the cleaned plural.
        let plurals: Vec<&str> = s
            .page_map
            .cfg
            .taxonomy_config
            .views
            .iter()
            .map(|v| v.plural_tree_key.as_str())
            .collect();
        assert_eq!(
            plurals,
            [
                "/brands",
                "/categories",
                "/companies",
                "/countries",
                "/ingredients",
                "/tags"
            ]
        );
    }
    assert!(Arc::ptr_eq(&h.deps, &h.sites[0].deps));

    // Deps: every service set; the shared ones shared (Go's Clone), the per-site ones not.
    let (d0, d1) = (&h.sites[0].deps, &h.sites[1].deps);
    for d in [d0, d1] {
        let _ = d.exec_helper();
        let _ = d.fs();
        let _ = d.path_spec();
        let _ = d.content_spec();
        let _ = d.source_spec();
        let _ = d.resource_spec();
    }
    assert!(!Arc::ptr_eq(d0.path_spec(), d1.path_spec()));
    // Go `WithBaseFs`: a new BaseFs sharing the source filesystems.
    assert!(Arc::ptr_eq(
        &d0.path_spec().base_fs.source_filesystems,
        &d1.path_spec().base_fs.source_filesystems
    ));
    assert!(Arc::ptr_eq(d0.source_spec(), d1.source_spec()));
    assert!(!Arc::ptr_eq(d0.content_spec(), d1.content_spec()));
    assert!(!Arc::ptr_eq(d0.resource_spec(), d1.resource_spec()));
    assert!(Arc::ptr_eq(
        &d0.resource_spec().common,
        &d1.resource_spec().common
    ));
    assert!(Arc::ptr_eq(&d0.build_state, &d1.build_state));
    assert!(Arc::ptr_eq(&d0.mem_cache, &d1.mem_cache));
    assert!(Arc::ptr_eq(&d0.counters, &d1.counters));
    assert!(!Arc::ptr_eq(&d0.site, &d1.site));
    assert_eq!(d1.path_spec().lang(), "th");

    // The PostProcess ids come from the one BuildState (shared through SpecCommon).
    use nh_common::identity::Incrementer;
    assert_eq!(d0.resource_spec().common.incr.incr(), 1);
    assert_eq!(d1.build_state.incr(), 2);

    // Template stores: the first site's store, a `with_site_opts` view for the other (same
    // parsed templates, its own func map), with the embedded ref/relref shortcodes.
    for s in &h.sites {
        let ts = s.deps.get_template_store();
        assert!(ts.lookup_shortcode_by_name("ref").is_some());
        assert!(ts.lookup_shortcode_by_name("relref").is_some());
        assert!(ts.lookup_shortcode_by_name("badge").is_some());
        assert!(ts.get_func("relref").is_some());
        assert!(ts.get_func("partialCached").is_some());
        assert!(s.template_store.get().is_some());
    }
    assert!(Arc::ptr_eq(
        &d0.get_template_store().shared,
        &d1.get_template_store().shared
    ));
    assert!(!Arc::ptr_eq(
        &d0.get_template_store().store_site,
        &d1.get_template_store().store_site
    ));

    // Translators (no i18n files: empty translations).
    for d in [d0, d1] {
        assert_eq!(d.translate(&(), "missing", &Value::Invalid).unwrap(), "");
    }
    assert!(h.translation_provider.translator().is_some());

    // One publisher per site; HugoInfo from the first site's config.
    assert!(!Arc::ptr_eq(&h.sites[0].publisher, &h.sites[1].publisher));
    assert!(h.sites[0].publisher.html_elements_collector.is_some());
    assert_eq!(h.hugo_info.environment, "production");
    assert_eq!(h.resolve_site(""), Some(0));
    assert_eq!(h.resolve_site("th"), Some(1));
    assert_eq!(h.resolve_site("fr"), None);

    // Freezing binds the sites (Go `Deps.Site`).
    let Built { h, .. } = b;
    let h = h.freeze();
    for s in &h.sites {
        assert!(s.deps.site.get().is_some());
    }
}
