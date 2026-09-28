//! `postCSS` against the `postcss` oracle (tools/go-oracle/nh-resource-transformers/postcss):
//! neohugo's cssjs client running postcss-cli (node) against the port, on the same site with the
//! same dependency-free postcss.config.js, including the chains toCSS | postCSS and
//! toCSS | postCSS | minify | fingerprint (the final CSS).
//!
//! Needs node and postcss-cli (11.0.1, the seeksnack version): `NEOHUGO_POSTCSS_BIN` names its
//! `node_modules/.bin/postcss`, which is linked into the site copy as
//! `node_modules/.bin/postcss` (Hugo's lookup order). Without it the test prints `SKIPPED` to
//! stderr and passes. The oracle ran from the site dir; this test does not change its working
//! directory: the port starts postcss in the site dir itself (PORTING.md).

mod t16_support;

use std::path::PathBuf;

use t16_support::*;

#[test]
fn postcss_synth() {
    let bin = match std::env::var("NEOHUGO_POSTCSS_BIN") {
        Ok(b) if !b.is_empty() => PathBuf::from(b),
        _ => {
            skip(
                "NEOHUGO_POSTCSS_BIN is not set (node_modules/.bin/postcss of postcss-cli 11.0.1)",
            );
            return;
        }
    };
    let bin = std::fs::canonicalize(&bin).unwrap_or(bin);
    let fx = fixture("postcss/synth.json.gz");
    let src = repo_root().join(fx["dir"].as_str().unwrap());
    let tmp = copy_site(
        &src,
        None,
        &[("node_modules/.bin/postcss".to_string(), bin)],
    );
    let dir = tmp.path.join("site");
    // The oracle had NODE_ENV=staging in its environment; Hugo filters it out.
    let mut environ = process_environ();
    environ.retain(|e| !e.starts_with("NODE_ENV="));
    environ.push("NODE_ENV=staging".to_string());
    let mut site = load_site(&dir.to_string_lossy(), &environ);

    let got: Vec<_> = fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| site.run(c))
        .collect();
    let diffs = diff_results("postcss", &fx["results"], &got);
    assert!(
        diffs.is_empty(),
        "{} differences:\n{}",
        diffs.len(),
        diffs.join("\n")
    );
    eprintln!("postcss/synth: {} cases identical", got.len());
}
