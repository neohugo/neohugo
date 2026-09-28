//! T21: the page-output lifecycle (`init_page` = Go `initLazyProviders`,
//! `shift_to_output_format`) against Go's behaviour (hugolib/page__meta.go:884-947,
//! hugolib/page.go:675-747):
//!
//! * page outputs are shared by format NAME across the global render formats (en/html and
//!   th/html are the same output); a standalone page has one slot and always uses it;
//! * the first output gets a content output at init; a rendering-site shift keeps an output's
//!   own `pco`, creates one when it has none, and reuses another output's only when
//!   `canReusePageOutputContent` (variations state 1); it resets a BUILT paginator only;
//! * a non-rendering-site shift installs a `LazyContentProvider` on the current output without
//!   touching its `pco`, resets it on the next such shift, and the lazy provider creates the
//!   content output of the page's CURRENT output when first used; the next rendering-site shift
//!   installs the `pco` again.
//!
//! The sites are the oracle's `asm-i18n` (en/th/fr, four formats each) and `asm-cascade`
//! (en/th) sites, assembled by `process` + `assemble`.

mod support;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use go_value::{SliceType, Value};
use nh_hugolib::HugoSites;
use nh_hugolib::hugo_sites_build::BuildCfg;
use nh_hugolib::page::PageId;
use nh_hugolib::page__output::{ContentProviderSlot, PageOutput};
use nh_hugolib::page__paginator::PagePaginatorInit;
use support::*;

fn assembled(name: &str) -> (Arc<HugoSites>, TempDir) {
    let fx = fixture(&format!("assemble/{name}.json.gz"));
    let tmp = TempDir::new(&format!("lifecycle-{name}"));
    let mut b = new_sites(&fx["site"], &tmp.0).unwrap_or_else(|e| panic!("{name}: {e}"));
    nh_hugolib::build_process::process(&mut b.h, &BuildCfg::default()).unwrap();
    nh_hugolib::build_assemble::assemble(&mut b.h, &BuildCfg::default()).unwrap();
    (b.h.freeze(), tmp)
}

/// The page of site `site` at tree key `path`.
fn page_at(h: &HugoSites, site: usize, path: &str) -> PageId {
    h.page_trees
        .tree_pages
        .get(h.sites[site].page_map.dims, path)
        .and_then(|n| n.page_id())
        .unwrap_or_else(|| panic!("no page {path} in site {site}"))
}

fn outputs(h: &HugoSites, id: PageId) -> Vec<Arc<PageOutput>> {
    h.page(id)
        .lazy
        .get()
        .expect("initialised during assembly")
        .as_ref()
        .unwrap()
        .outputs
        .clone()
}

fn built_pager() -> Arc<nh_page::pagination::Pager> {
    let pag = nh_page::pagination::paginate_with(
        Arc::new(|n| format!("/page/{n}/")),
        &Value::list(
            SliceType::Named(Arc::from(nh_page::page::PAGES_TYPE)),
            vec![],
        ),
        10,
    )
    .unwrap();
    pag.pagers()[0].clone()
}

#[test]
fn outputs_are_shared_by_format_name() {
    let (h, _tmp) = assembled("asm-i18n");
    let names: Vec<&str> = h.render_formats.0.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "feed", "html", "print", "rss", "feed", "html", "print", "rss", "feed", "html",
            "print", "rss"
        ]
    );

    for site in 0..h.sites.len() {
        let home = page_at(&h, site, "");
        let outs = outputs(&h, home);
        assert_eq!(outs.len(), 12);
        for i in 0..12 {
            for j in 0..12 {
                assert_eq!(
                    Arc::ptr_eq(&outs[i], &outs[j]),
                    names[i] == names[j],
                    "site {site}: slots {i} and {j}"
                );
            }
        }
        // The home page renders its configured formats only.
        let render: Vec<bool> = outs.iter().take(4).map(|o| o.render).collect();
        assert_eq!(render, [true, true, true, true]);

        // A standalone page has a single slot, whatever the index.
        let p404 = page_at(&h, site, "/404");
        let outs = outputs(&h, p404);
        assert_eq!(outs.len(), 1);
        assert_eq!(outs[0].f.name, "404");
        h.page(p404).shift_to_output_format(&h, true, 7).unwrap();
        assert_eq!(h.page(p404).current_output_idx.load(Ordering::SeqCst), 0);
    }

    // A page in a section of en: html and print render, feed and rss do not.
    let p = page_at(&h, 0, "/blog/a");
    let render: Vec<(String, bool)> = outputs(&h, p)
        .iter()
        .take(4)
        .map(|o| (o.f.name.clone(), o.render))
        .collect();
    assert_eq!(
        render,
        [
            ("feed".to_string(), false),
            ("html".to_string(), true),
            ("print".to_string(), true),
            ("rss".to_string(), false)
        ]
    );
}

#[test]
fn rendering_site_shift() {
    let (h, _tmp) = assembled("asm-i18n");
    let id = page_at(&h, 0, "/blog");
    let ps = h.page(id);
    let outs = outputs(&h, id);

    // Assembly shifted to 0: the first output got its content output at init and keeps it.
    assert_eq!(ps.current_output_idx.load(Ordering::SeqCst), 0);
    let first = outs[0].pco().expect("first output has a content output");
    assert!(
        matches!(outs[0].provider_slot(), ContentProviderSlot::Pco(p) if Arc::ptr_eq(&p, &first))
    );
    for o in &outs[1..4] {
        assert!(o.pco().is_none());
        assert!(matches!(o.provider_slot(), ContentProviderSlot::Nop));
    }

    // Shifting to an output without content creates one (no reuse: variations state 0).
    ps.shift_to_output_format(&h, true, 1).unwrap();
    assert_eq!(ps.current_output_idx.load(Ordering::SeqCst), 1);
    let html = outs[1].pco().expect("created");
    assert!(!Arc::ptr_eq(&html, &first));
    assert_eq!(html.output_idx, 1);
    assert!(
        matches!(outs[1].provider_slot(), ContentProviderSlot::Pco(p) if Arc::ptr_eq(&p, &html))
    );

    // The same format of another site selects the same output and keeps its content.
    ps.shift_to_output_format(&h, true, 5).unwrap();
    assert_eq!(ps.current_output_idx.load(Ordering::SeqCst), 5);
    assert!(Arc::ptr_eq(&outs[5], &outs[1]));
    assert!(Arc::ptr_eq(&outs[5].pco().unwrap(), &html));

    // With variations state 1 an output without content reuses the first other one's.
    ps.page_output_template_variations_state
        .store(1, Ordering::SeqCst);
    ps.shift_to_output_format(&h, true, 2).unwrap();
    assert!(Arc::ptr_eq(&outs[2].pco().unwrap(), &first));
    // Any other state creates a new one.
    ps.page_output_template_variations_state
        .store(2, Ordering::SeqCst);
    ps.shift_to_output_format(&h, true, 3).unwrap();
    let rss = outs[3].pco().unwrap();
    assert!(!Arc::ptr_eq(&rss, &first) && !Arc::ptr_eq(&rss, &html));

    // A built paginator is reset on a rendering-site shift; an unbuilt one is left alone.
    let pag = outs[1]
        .paginator
        .as_ref()
        .expect("a rendered node has a paginator");
    *pag.init.lock().unwrap() = PagePaginatorInit {
        done: true,
        current: Some(built_pager()),
    };
    ps.shift_to_output_format(&h, true, 1).unwrap();
    assert!(!pag.is_built());
    assert!(!pag.init.lock().unwrap().done);
    *pag.init.lock().unwrap() = PagePaginatorInit {
        done: true,
        current: None,
    };
    ps.shift_to_output_format(&h, true, 5).unwrap();
    assert!(pag.init.lock().unwrap().done);

    // A non-rendering shift never resets the paginator.
    *pag.init.lock().unwrap() = PagePaginatorInit {
        done: true,
        current: Some(built_pager()),
    };
    ps.shift_to_output_format(&h, false, 1).unwrap();
    assert!(pag.is_built());

    // Outputs that do not render (sections have no print format) have no paginator; pages have
    // none either.
    assert!(!outs[2].render && outs[2].paginator.is_none());
    let page = page_at(&h, 0, "/blog/a");
    assert!(outputs(&h, page).iter().all(|o| o.paginator.is_none()));
}

#[test]
fn non_rendering_site_shift() {
    let (h, _tmp) = assembled("asm-cascade");
    let id = page_at(&h, 0, "/docs/p");
    let ps = h.page(id);
    let outs = outputs(&h, id);
    assert_eq!(outs.len(), 4);
    let html = outs[0].pco().expect("init content output");

    // Another site renders html (index 2, the same output): the provider becomes lazy, the
    // pco stays.
    ps.shift_to_output_format(&h, false, 2).unwrap();
    assert_eq!(ps.current_output_idx.load(Ordering::SeqCst), 2);
    let ContentProviderSlot::Lazy(lcp) = outs[0].provider_slot() else {
        panic!("a lazy content provider");
    };
    assert!(Arc::ptr_eq(&outs[0].pco().unwrap(), &html));

    // First use creates the content output of the page's CURRENT output.
    let created = outs[0].content_renderer().expect("lazily created");
    assert!(!Arc::ptr_eq(&created, &html));
    assert!(Arc::ptr_eq(&created.po, &outs[2]));
    assert_eq!(created.output_idx, 2);
    // Stable until reset.
    assert!(Arc::ptr_eq(&outs[0].content_renderer().unwrap(), &created));

    // The next non-rendering shift resets the same lazy provider (no new one).
    ps.shift_to_output_format(&h, false, 3).unwrap();
    assert_eq!(ps.current_output_idx.load(Ordering::SeqCst), 3);
    let ContentProviderSlot::Lazy(lcp2) = outs[0].provider_slot() else {
        panic!("still lazy");
    };
    assert!(Arc::ptr_eq(&lcp, &lcp2));
    // The rss output (index 3) got its own lazy provider.
    let ContentProviderSlot::Lazy(rss_lcp) = outs[3].provider_slot() else {
        panic!("rss lazy");
    };
    assert!(!Arc::ptr_eq(&rss_lcp, &lcp));
    ps.shift_to_output_format(&h, false, 2).unwrap();
    // Reset: the next use creates a new content output (for the current output).
    let again = outs[0].content_renderer().unwrap();
    assert!(!Arc::ptr_eq(&again, &created));
    assert!(Arc::ptr_eq(&again.po, &outs[2]));

    // The page's own site renders again: the pco is installed as the provider.
    ps.shift_to_output_format(&h, true, 0).unwrap();
    assert!(
        matches!(outs[0].provider_slot(), ContentProviderSlot::Pco(p) if Arc::ptr_eq(&p, &html))
    );
    assert!(Arc::ptr_eq(&outs[0].content_renderer().unwrap(), &html));

    // An output without a pco shifted by another site: lazy, and pco stays empty.
    assert!(outs[1].pco().is_none());
    ps.shift_to_output_format(&h, false, 1).unwrap();
    assert!(matches!(
        outs[1].provider_slot(),
        ContentProviderSlot::Lazy(_)
    ));
    assert!(outs[1].pco().is_none());
    // Then its own site renders it: a new content output.
    ps.shift_to_output_format(&h, true, 1).unwrap();
    assert!(outs[1].pco().is_some());
}
