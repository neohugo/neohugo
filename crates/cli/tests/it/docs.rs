//! Gate A-D2 (REWRITE_PLAN.md §7.3, T66): the Hugo documentation site with the `reduced` docs
//! patches, built by the `neohugo-rs` binary against the committed golden data of the Go build
//! (`testdata/golden/docs-reduced/`) through the acceptance harness:
//! `tools/neohugo/compare.sh docs-reduced --ref golden` (`sites.py make docs-reduced --overlay
//! sites/docs`, both passes, `structdiff.py` and the ratchet of
//! `testdata/baselines/docs-reduced.json`).
//!
//! The reduced variant keeps Chroma highlighting (with `hl` inline/`noClasses` and
//! `shortcodes/highlight.md`), GoAT diagrams (`diagrams_goat`), emoji, passthrough with
//! `to_math`, `remarshal` in `code-toggle`, Tailwind through `defer` and the real Alpine/Turbo
//! `js_build`. The gate: L1 889/889 in both passes (888 in `public` plus `hugo_stats.json`),
//! the structure oracle and L2 on every file, L4 on every asset, and a clean ratchet (the L3
//! differences are the baseline's: the math and goat pages `accepted-deviation`).
//!
//! Tailwind and `js_build` need the node tools and esbuild; without them the test prints
//! `SKIPPED` and passes ([`crate::acceptance`]). The Go binaries are not needed. The binary
//! must have the default features `goat` and `math`.

use crate::acceptance;

#[test]
fn gate_a_d2() {
    let Some(d) = acceptance::compare("gate_a_d2", "docs-reduced", &["tailwindcss"]) else {
        return;
    };
    acceptance::assert_l1(&d, 889);
    acceptance::assert_equal(&d, &["L1", "L2", "L4", "S"]);
    let a7 = d["a7"]["ratio"].as_f64().expect("A7 ratio");
    assert!(a7 >= 0.98, "A7 {a7} < 0.98");
}
