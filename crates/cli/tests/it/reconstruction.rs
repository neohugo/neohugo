//! Gate A-R (REWRITE_PLAN.md §7.3, T62): the seeksnack reconstruction built by the
//! `neohugo` binary against the committed golden data of the Go build
//! (`testdata/golden/seeksnack/`, T01), through the acceptance harness:
//! `tools/neohugo/compare.sh seeksnack --ref golden` (`sites.py make seeksnack --overlay
//! sites/seeksnack`, both passes, `structdiff.py` and the ratchet of
//! `testdata/baselines/seeksnack.json`).
//!
//! The gate: L1 713/713 in both passes, the structure oracle (records, aliases, pagers, pages
//! and resource URLs) without a difference, L2 on every file, A7 ≥ 0.95 and a clean ratchet
//! (every remaining difference is a baseline entry with the same fingerprint).
//!
//! The site's PostCSS step and `js_build` need the node tools and esbuild; without them the
//! test prints `SKIPPED` and passes ([`crate::acceptance`]). The Go binaries are not needed.

use crate::acceptance;

#[test]
fn gate_a_r() {
    let Some(d) = acceptance::compare("gate_a_r", "seeksnack", &["postcss"]) else {
        return;
    };
    acceptance::assert_l1(&d, 713);
    acceptance::assert_equal(&d, &["L1", "L2", "L4", "S"]);
    let a7 = d["a7"]["ratio"].as_f64().expect("A7 ratio");
    assert!(a7 >= 0.95, "A7 {a7} < 0.95");
}
