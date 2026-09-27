//! Red-team regression tests (tools/go-oracle/goldmark/redteam.go,
//! `goldmark redteamfixtures`): inputs that made the Rust port differ from Go
//! (stack overflows on deeply nested attribute values and `Node.Text`), a
//! fixed-seed sample of the red-team generators (Hugo extension interplay,
//! Unicode flanking/entities, reference-label case folding, tabs, CR/CRLF,
//! pathological inputs) compared on HTML *and* AST dumps, and direct
//! `parser.ParseAttributes` vectors.
//!
//! Everything runs on a thread with a 2 MiB stack (Rust's default for spawned
//! threads, as Hugo's worker threads will have): Go parses these inputs on
//! growable goroutine stacks.

mod common;

use common::{check_records, fixture};

const STACK: usize = 2 << 20;

fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn redteam_regressions() {
    let n = on_small_stack(|| check_records(&fixture("redteam.gmf.gz")));
    assert_eq!(n, 45);
}

#[test]
fn redteam_fuzz_fixed_seed() {
    let n = on_small_stack(|| check_records(&fixture("redteam-fuzz.gmf.gz")));
    assert_eq!(n, 800 + 800 + 500 + 800 + 40);
}

#[test]
fn parse_attributes_vectors() {
    let bad = on_small_stack(|| {
        let recs = fixture("redteam-attrs.gmf.gz");
        assert_eq!(recs.len(), 3000);
        let mut bad = Vec::new();
        for r in &recs {
            for (key, block) in [("reader", false), ("block", true)] {
                let got = common::astdump::attr_vec(r.get("in"), block);
                if got != r.get(key) {
                    bad.push(format!(
                        "{} ({key}): in {:?}\nwant {:?}\ngot  {:?}",
                        r.name,
                        String::from_utf8_lossy(&r.get("in")[..r.get("in").len().min(200)]),
                        String::from_utf8_lossy(&r.get(key)[..r.get(key).len().min(300)]),
                        String::from_utf8_lossy(&got[..got.len().min(300)]),
                    ));
                }
            }
        }
        bad
    });
    assert!(
        bad.is_empty(),
        "{} differ:\n{}",
        bad.len(),
        bad[..bad.len().min(10)].join("\n")
    );
}

/// The value of a 100k-deep attribute array is parsed, stored on the heading
/// and dropped without recursion.
#[test]
fn deep_attribute_value_is_kept_and_dropped() {
    on_small_stack(|| {
        use goldmark::ast::AttrValue;
        let md = format!("# h {{a={}{}}}\n", "[".repeat(100_000), "]".repeat(100_000));
        let m = common::new_markdown("hugo");
        let mut pc = goldmark::parser::new_context(vec![]);
        let doc = m.parse(md.as_bytes(), &mut pc);
        let h = doc.ast.first_child(doc.root).unwrap();
        let mut v = doc.ast.attribute(h, b"a").unwrap();
        let mut depth = 0;
        while let AttrValue::Array(a) = v {
            depth += 1;
            match a.first() {
                Some(x) => v = x,
                None => break,
            }
        }
        assert_eq!(depth, 100_000);
    });
}
