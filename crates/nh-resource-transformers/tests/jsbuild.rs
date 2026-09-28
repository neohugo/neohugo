//! `js.Build` against the `jsbuild` oracle (tools/go-oracle/nh-resource-transformers/jsbuild):
//! neohugo's resource_transformers/js with esbuild v0.25.6 linked in (Go) against the port with
//! the pinned esbuild binary over `--service` (the scripts, inputs and recorded attributes are in
//! the fixture; see t16_support).
//!
//! Needs `NEOHUGO_ESBUILD_BINARY` (tools/esbuild/build.sh builds it); without it the test prints
//! `SKIPPED` to stderr and passes.

mod t16_support;

use t16_support::*;

fn run_topic(name: &str) {
    if esbuild_binary().is_none() {
        return;
    }
    let fx = fixture(&format!("jsbuild/{name}.json.gz"));
    let src = repo_root().join(fx["dir"].as_str().unwrap());
    let only: Option<Vec<String>> = fx["only"]
        .as_array()
        .map(|a| a.iter().map(|s| s.as_str().unwrap().to_string()).collect());
    let tmp = copy_site(&src, only.as_deref(), &[]);
    let dir = tmp.path.join("site");
    std::fs::create_dir_all(dir.join("content/en")).unwrap();
    let mut site = load_site(&dir.to_string_lossy(), &process_environ());

    let got: Vec<_> = fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| site.run(c))
        .collect();
    let diffs = diff_results(name, &fx["results"], &got);
    assert!(
        diffs.is_empty(),
        "{} differences:\n{}",
        diffs.len(),
        diffs.join("\n")
    );
    eprintln!("jsbuild/{name}: {} cases identical", got.len());
}

#[test]
fn jsbuild_synth() {
    run_topic("synth");
}

#[test]
fn jsbuild_docs() {
    run_topic("docs");
}
