//! The `resources` namespace (Wave B task T15) against the Go oracles in
//! tools/go-oracle/nh-resource-transformers (fixtures in
//! crates/nh-resource-transformers/tests/fixtures):
//!
//! - `factories`: Get/GetMatch/Match/ByType/FromString/Copy over the assets of the synthetic
//!   site and the repository's docs site;
//! - `transformers` (arm64 oracle): Concat, ExecuteAsTemplate, Fingerprint, Minify,
//!   PostProcess, FromString and Copy chains over the same sites;
//! - `getremote`: GetRemote from the getresource file cache (the entries the oracle recorded
//!   from a local HTTP server).
//!
//! Every call's result (attributes, content, links) and the published files must equal Go's.

mod resources_support;

use std::collections::HashMap;

use resources_support::*;
use serde_json::{Value as J, json};

/// Go panics reading a resource over a directory (`resources.Get "js"`: afero's
/// `this operation is not supported`); the Rust file read returns the error, which the record
/// shows as the content error.
fn go_panic_equivalent(want: &J, got: &J) -> bool {
    let Some(p) = want["res"]["panic"].as_str() else {
        return false;
    };
    got["res"]["contentErr"]
        .as_str()
        .is_some_and(|e| e.contains(p))
}

/// gotemplate's documented deviation 15 (crates/gotemplate/PORTING.md): Go names the static
/// type of an interface-typed slot (`can't evaluate field foo in type interface {}` for `.api`
/// read from a `map[string]any`), the value model knows only the dynamic type (`string`).
fn gotemplate_static_type(want: &J, got: &J) -> bool {
    let (Some(w), Some(g)) = (
        want["res"]["contentErr"].as_str(),
        got["res"]["contentErr"].as_str(),
    ) else {
        return false;
    };
    w.contains("in type interface {}") && w.replace("in type interface {}", "in type string") == g
}

fn run_topic(topic: &str, site_dir: &str, with_store: bool) {
    let topic = topic.to_string();
    let site_dir = site_dir.to_string();
    big_stack(move || {
        let fx = fixture(&topic);
        let dir = repo_root().join(&site_dir);
        let site = load_site(&dir.to_string_lossy());
        let store = with_store.then(|| {
            let names: Vec<String> = fx["funcNames"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect();
            store(&site, &names)
        });
        let d = deps(&site, store);
        let ns = nh_tplfuncs::resources::resources::Namespace::new(d);
        let mut vars = HashMap::new();
        let mut fails: Vec<String> = Vec::new();
        let mut n = 0;
        run_cases(&ns, &fx["cases"], &mut vars, &mut |what, _st, want, got| {
            n += 1;
            if go_panic_equivalent(want, got) {
                return;
            }
            if gotemplate_static_type(want, got) {
                // The rest of the record must still match.
                let mut w = want.clone();
                w["res"]["contentErr"] = got["res"]["contentErr"].clone();
                fails.extend(diff(what, &w, got));
                return;
            }
            fails.extend(diff(what, want, got));
        });
        fails.extend(diff("published", &fx["published"], &site.published_files()));
        if !fails.is_empty() {
            for f in fails.iter().take(60) {
                eprintln!("{f}");
            }
            panic!("{topic}: {} differences in {n} calls", fails.len());
        }
        eprintln!("{topic}: {n} calls, 0 differences");
    });
}

const SYNTH: &str = "crates/nh-resource-transformers/tests/fixtures/site";

#[test]
fn factories_synth() {
    run_topic("factories/synth.json.gz", SYNTH, false);
}

#[test]
fn factories_docs() {
    run_topic("factories/docs.json.gz", "docs", false);
}

#[test]
fn transformers_synth() {
    run_topic("transformers/synth.json.gz", SYNTH, true);
}

#[test]
fn transformers_docs() {
    run_topic("transformers/docs.json.gz", "docs", false);
}

/// GetRemote from the recorded getresource entries (the oracle's cached run); calls whose
/// response Go did not cache fail with a network error in Go and with the port's explicit
/// no-network error.
#[test]
fn getremote() {
    big_stack(|| {
        let fx = fixture("getremote/getremote.json.gz");
        let site = load_site(&repo_root().join(SYNTH).to_string_lossy());
        let cache_dir = site.getresource_dir();
        std::fs::create_dir_all(&cache_dir).unwrap();
        let fixtures =
            repo_root().join("crates/nh-resource-transformers/tests/fixtures/getremote/cache");
        for k in fx["cacheEntries"].as_array().unwrap() {
            let k = k.as_str().unwrap();
            std::fs::copy(fixtures.join(k), cache_dir.join(k)).unwrap();
        }
        let ns = nh_tplfuncs::resources::resources::Namespace::new(deps(&site, None));
        let mut vars = HashMap::new();
        let mut fails = Vec::new();
        let mut n = 0;
        let mut network = 0;
        for c in fx["calls"].as_array().unwrap() {
            let case = json!([{"name": c["name"], "steps": [{"op": "GetRemote", "args": c["args"]}], "results": [c["cached"]]}]);
            run_cases(&ns, &case, &mut vars, &mut |what, _st, want, got| {
                n += 1;
                // Go's network errors (the dialer the oracle disabled, or http.Transport's
                // scheme check before it dials).
                let net_err = want["err"].as_str().and_then(|e| {
                    ["oracle: network disabled", "unsupported protocol scheme"]
                        .iter()
                        .find_map(|m| e.find(m))
                        .map(|i| (e, i))
                });
                if let Some((e, i)) = net_err {
                    network += 1;
                    let prefix = &e[..i];
                    match got["err"].as_str() {
                        Some(g)
                            if g.starts_with(prefix)
                                && g.contains("neohugo-rs: network access is not supported") => {}
                        _ => fails.push(format!("{what}: want a network error {e:?}, got {got}")),
                    }
                    return;
                }
                fails.extend(diff(what, want, got));
            });
        }
        if !fails.is_empty() {
            for f in fails.iter().take(60) {
                eprintln!("{f}");
            }
            panic!("getremote: {} differences in {n} calls", fails.len());
        }
        eprintln!("getremote: {n} calls ({network} not cached), 0 differences");
    });
}
