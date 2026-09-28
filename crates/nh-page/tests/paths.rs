//! Oracle test: `page.CreateTargetPaths`, `TargetPaths.RelPermalink` and
//! `TargetPaths.PermalinkForOutputFormat` against `tools/go-oracle/nh-page/paths`
//! (fixtures/paths/<site>.json.gz): every descriptor the Go builds of the oracle sites created
//! (recorded through an overlaid hook) plus the adversarial variants.

mod support;

use std::sync::{Arc, Mutex};

use nh_page::page_paths::{TargetPathDescriptor, create_target_paths};
use serde_json::Value as J;
use support::*;

fn result_of(d: &TargetPathDescriptor) -> J {
    match catch(|| {
        let tp = create_target_paths(d);
        serde_json::json!({
            "targetFilename": enc(tp.target_filename.as_bytes()),
            "subResourceBaseTarget": enc(tp.sub_resource_base_target.as_bytes()),
            "subResourceBaseLink": enc(tp.sub_resource_base_link.as_bytes()),
            "link": enc(tp.link.as_bytes()),
            "relPermalink": enc(tp.rel_permalink(&d.path_spec).as_bytes()),
            "permalink": enc(tp.permalink_for_output_format(&d.path_spec, &d.type_).as_bytes()),
        })
    }) {
        Ok(v) => serde_json::json!({ "ok": v }),
        Err(p) => serde_json::json!({ "panic": p }),
    }
}

/// Go's panic texts from `url.Parse` errors are compared up to the `parse` prefix, as the Rust
/// panic carries the same `*url.Error` text.
fn same(want: &J, got: &J) -> bool {
    if let (Some(w), Some(g)) = (want.get("panic"), got.get("panic")) {
        return g.as_str().unwrap().contains(w.as_str().unwrap());
    }
    want == got
}

fn run_site(file: &str) -> (usize, usize, Vec<String>) {
    let fx = fixture(&format!("paths/{file}"));
    let misses: Misses = Arc::new(Mutex::new(Vec::new()));
    let pp = build_parser(&fx["parser"], &misses);
    let paths = build_paths(&fx["paths"], &pp);
    let specs: Vec<_> = fx["pathspecs"]
        .as_array()
        .unwrap()
        .iter()
        .map(build_path_spec)
        .collect();
    let formats: Vec<_> = fx["formats"]
        .as_array()
        .unwrap()
        .iter()
        .map(output_format)
        .collect();
    assert!(
        misses.lock().unwrap().is_empty(),
        "{file}: parser misses {:?}",
        misses.lock().unwrap()
    );

    let path_at = |v: &J| -> Option<Arc<nh_common::paths::pathparser::Path>> {
        let i = v.as_i64().unwrap();
        if i < 0 {
            None
        } else {
            Some(paths[i as usize].clone())
        }
    };

    let mut n = 0;
    let mut n_build = 0;
    let mut fails = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let d = TargetPathDescriptor {
            path_spec: specs[c["ps"].as_u64().unwrap() as usize].clone(),
            type_: formats[c["type"].as_u64().unwrap() as usize].clone(),
            kind: c["kind"].as_str().unwrap().to_string(),
            path: path_at(&c["path"]).expect("descriptor path"),
            section: path_at(&c["section"]),
            base_name: gostring(&c["baseName"]),
            prefix_file_path: gostring(&c["prefixFilePath"]),
            prefix_link: gostring(&c["prefixLink"]),
            force_prefix: c["forcePrefix"].as_bool().unwrap(),
            url: gostring(&c["url"]),
            addends: gostring(&c["addends"]),
            expanded_permalink: gostring(&c["expandedPermalink"]),
            ugly_urls: c["uglyURLs"].as_bool().unwrap(),
        };
        let got = result_of(&d);
        n += 1;
        if c["src"] == "build" {
            n_build += 1;
        }
        if !same(&c["want"], &got) {
            fails.push(format!("{file}: {c}\n   got {got}"));
        }
    }
    (n, n_build, fails)
}

#[test]
fn create_target_paths_matches_go() {
    nh_page::page::init();
    let mut total = 0;
    let mut total_build = 0;
    let mut fails = Vec::new();
    for f in fixture_files("paths") {
        let (n, nb, fl) = run_site(&f);
        eprintln!(
            "paths {f}: {n} cases ({nb} from the build), {} failures",
            fl.len()
        );
        total += n;
        total_build += nb;
        fails.extend(fl);
    }
    eprintln!(
        "paths: {total} cases ({total_build} recorded from Go builds), {} failures",
        fails.len()
    );
    for f in fails.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(fails.is_empty());
    assert!(total_build > 5000);
}

/// content-model.md §9.4: the seeksnack examples, from the seeksnack-like build.
#[test]
fn cm_9_4_examples() {
    let fx = fixture("paths/seeksnack.json.gz");
    let mut links = std::collections::BTreeMap::new();
    for c in fx["cases"].as_array().unwrap() {
        if c["src"] != "build" {
            continue;
        }
        let w = &c["want"]["ok"];
        links.insert(
            gostring(&w["targetFilename"]),
            (gostring(&w["relPermalink"]), gostring(&w["permalink"])),
        );
    }
    let expect = [
        ("/index.html", "/", "https://seeksnack.com/"),
        (
            "/index.json",
            "/index.json",
            "https://seeksnack.com/index.json",
        ),
        (
            "/index.xml",
            "/index.xml",
            "https://seeksnack.com/index.xml",
        ),
        ("/th/index.html", "/th/", "https://seeksnack.com/th/"),
        (
            "/biscuit/index.html",
            "/biscuit/",
            "https://seeksnack.com/biscuit/",
        ),
        (
            "/biscuit/index.xml",
            "/biscuit/index.xml",
            "https://seeksnack.com/biscuit/index.xml",
        ),
        (
            "/biscuit/koalas-march-chocolate/index.html",
            "/biscuit/koalas-march-chocolate/",
            "https://seeksnack.com/biscuit/koalas-march-chocolate/",
        ),
        (
            "/disclaimer/index.html",
            "/disclaimer/",
            "https://seeksnack.com/disclaimer/",
        ),
        (
            "/potato-chips/herrs-salt--vinegar-potato-chips/index.html",
            "/potato-chips/herrs-salt--vinegar-potato-chips/",
            "https://seeksnack.com/potato-chips/herrs-salt--vinegar-potato-chips/",
        ),
        (
            "/potato-chips/wise-chili-olé-chili--spice-flavor-potato-chips/index.html",
            "/potato-chips/wise-chili-ol%C3%A9-chili--spice-flavor-potato-chips/",
            "https://seeksnack.com/potato-chips/wise-chili-ol%C3%A9-chili--spice-flavor-potato-chips/",
        ),
        (
            "/companies/berli-jucker-foods-ltd.berli-jucker-plc/index.html",
            "/companies/berli-jucker-foods-ltd.berli-jucker-plc/",
            "https://seeksnack.com/companies/berli-jucker-foods-ltd.berli-jucker-plc/",
        ),
        (
            "/th/tags/คริสปี้พาย-/index.html",
            "/th/tags/%E0%B8%84%E0%B8%A3%E0%B8%B4%E0%B8%AA%E0%B8%9B%E0%B8%B5%E0%B9%89%E0%B8%9E%E0%B8%B2%E0%B8%A2-/",
            "https://seeksnack.com/th/tags/%E0%B8%84%E0%B8%A3%E0%B8%B4%E0%B8%AA%E0%B8%9B%E0%B8%B5%E0%B9%89%E0%B8%9E%E0%B8%B2%E0%B8%A2-/",
        ),
        (
            "/categories/no-salt/low-salt-chips/index.html",
            "/categories/no-salt/low-salt-chips/",
            "https://seeksnack.com/categories/no-salt/low-salt-chips/",
        ),
        (
            "/brands/index.html",
            "/brands/",
            "https://seeksnack.com/brands/",
        ),
        (
            "/brands/index.xml",
            "/brands/index.xml",
            "https://seeksnack.com/brands/index.xml",
        ),
        (
            "/brands/lays/index.html",
            "/brands/lays/",
            "https://seeksnack.com/brands/lays/",
        ),
        (
            "/blog/page/2/index.html",
            "/blog/page/2/",
            "https://seeksnack.com/blog/page/2/",
        ),
        (
            "/blog/page/1/index.html",
            "/blog/page/1/",
            "https://seeksnack.com/blog/page/1/",
        ),
        ("/404.html", "/404.html", "https://seeksnack.com/404.html"),
        (
            "/th/404.html",
            "/th/404.html",
            "https://seeksnack.com/th/404.html",
        ),
        (
            "/en/sitemap.xml",
            "/en/sitemap.xml",
            "https://seeksnack.com/en/sitemap.xml",
        ),
        (
            "/th/sitemap.xml",
            "/th/sitemap.xml",
            "https://seeksnack.com/th/sitemap.xml",
        ),
    ];
    let mut missing = Vec::new();
    for (tf, rel, perma) in expect {
        match links.get(tf) {
            Some((r, p)) => {
                assert_eq!(r, rel, "{tf}");
                assert_eq!(p, perma, "{tf}");
            }
            None => missing.push(tf),
        }
    }
    assert!(
        missing.is_empty(),
        "not built: {missing:?}; built: {:?}",
        links.keys().collect::<Vec<_>>()
    );
}
