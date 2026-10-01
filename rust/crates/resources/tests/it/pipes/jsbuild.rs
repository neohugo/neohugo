//! `js_build` through the store against the `jsbuild` oracle (Hugo's `js.Build` on t16site,
//! 62 cases, and on the docs site's scripts, 6 cases): asset → (concat) → `js_build` →
//! (`fingerprint`). Scripts must be byte-identical (inline source maps decoded, as the
//! oracle records them), media types, links and `Data.Integrity` equal; errors must be errors
//! at the same file and line. Published files: the script and, for external and linked
//! source maps, the map next to it (the maps' `sources` differ, see neohugo-esbuild's tests).

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine as _;
use neohugo_base::ResourceId;
use serde_json::Value as J;

use super::{Project, data, esbuild_binary, fixture_site, project, project_in_place, run_steps};
use crate::support::MemSink;

/// Replaces an inline source map's base64 with `DECODED(<map>)`, as the oracle records it.
fn decode_inline_map(code: &str) -> String {
    const MARK: &str = "sourceMappingURL=data:application/json;base64,";
    let Some(i) = code.find(MARK) else {
        return code.to_owned();
    };
    let start = i + MARK.len();
    let end = code[start..].find('\n').map_or(code.len(), |n| start + n);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&code[start..end])
        .unwrap();
    format!(
        "{}DECODED({}){}",
        &code[..start],
        String::from_utf8(decoded).unwrap(),
        &code[end..]
    )
}

fn run(p: &Project, fixture: &str) -> (usize, usize, Vec<String>) {
    let fx: J = neohugo_testkit::fixture::oracle(fixture);
    let mut done: BTreeMap<String, ResourceId> = BTreeMap::new();
    let mut failures = Vec::new();
    let (mut built, mut errors) = (0, 0);
    let mut links = Vec::new();
    let mut want_published = BTreeSet::new();
    for (case, want) in fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fx["results"].as_array().unwrap())
    {
        let name = case["name"].as_str().unwrap();
        let err = want
            .get("contentErr")
            .or_else(|| want.get("stepErr"))
            .and_then(J::as_str);
        match (run_steps(p, case, &done), err) {
            (Ok(id), None) => {
                done.insert(name.to_owned(), id);
                let r = p.store.resource(id);
                let code = String::from_utf8(p.store.content(id).unwrap().to_vec()).unwrap();
                let want_code = p.site(want["content"].as_str().unwrap());
                if decode_inline_map(&code) != want_code {
                    failures.push(format!(
                        "{name}: content differs:\n{code}\n---\n{want_code}"
                    ));
                }
                if want["mediaType"] != r.media_type_string() {
                    failures.push(format!("{name}: media type {}", r.media_type_string()));
                }
                if data(&want["data"]) != serde_json::to_value(&r.data).unwrap() {
                    failures.push(format!("{name}: data {:?}", r.data));
                }
                if let Some(link) = want.get("relPermalink").and_then(J::as_str) {
                    if link != r.rel_permalink {
                        failures.push(format!("{name}: link {}", r.rel_permalink));
                    }
                    links.push(r.rel_permalink.clone());
                    want_published.extend(want["published"].as_object().unwrap().keys().cloned());
                }
                built += 1;
            }
            (Err((_, e)), Some(w)) => {
                let at = w
                    .split('"')
                    .find(|s| s.starts_with("$SITE/"))
                    .map(|s| p.site(s));
                if let Some(at) = at {
                    let file_line = at.rsplit_once(':').unwrap().0;
                    if !e.contains(&format!("{file_line}:")) {
                        failures.push(format!("{name}: error {e:?}, want at {at}"));
                    }
                }
                errors += 1;
            }
            (Ok(_), Some(w)) => failures.push(format!("{name}: built, Go fails: {w}")),
            (Err((_, e)), None) => failures.push(format!("{name}: {e}")),
        }
    }
    // What the links publish: the scripts and their source maps.
    let sink = MemSink::default();
    p.store
        .publish(links.iter().map(String::as_str), &sink)
        .unwrap();
    let got: BTreeSet<String> = sink.0.lock().unwrap().keys().cloned().collect();
    if got != want_published {
        failures.push(format!("published {got:?}, want {want_published:?}"));
    }
    (built, errors, failures)
}

fn check(p: &Project, fixture: &str, cases: usize) {
    let (built, errors, failures) = run(p, fixture);
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(built + errors, cases);
    eprintln!(
        "js_build {fixture}: {built} scripts identical to Go, {errors} errors at Go's positions"
    );
}

fn have_esbuild(test: &str) -> bool {
    let found = esbuild_binary().is_file();
    if !found {
        eprintln!(
            "SKIPPED {test}: no esbuild binary (tools/neohugo/node.sh && tools/esbuild/install.sh)"
        );
    }
    found
}

#[test]
fn js_build_t16site() {
    if !have_esbuild("js_build_t16site") {
        return;
    }
    let p = project(&fixture_site("t16site"), |_| {});
    check(&p, "oracle/resource-transformers/jsbuild/synth.json.gz", 62);
}

#[test]
fn js_build_docs() {
    if !have_esbuild("js_build_docs") {
        return;
    }
    let p = project_in_place(&crate::support::repo_dir().join("docs"), |_| {});
    check(&p, "oracle/resource-transformers/jsbuild/docs.json.gz", 6);
}
