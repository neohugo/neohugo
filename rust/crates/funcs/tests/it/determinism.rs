//! Map outputs are sorted, and serde_json keeps its sorted `Map` (REWRITE_PLAN.md §8.2, T31).

use tera::{Context, Value};

use crate::support::Harness;

/// No dependency of this test build enables `serde_json/preserve_order` (feature unification
/// would make `serde_json::Map` an insertion-ordered map and change every JSON output).
#[test]
fn serde_json_map_is_sorted() {
    let mut m = serde_json::Map::new();
    m.insert("b".into(), serde_json::Value::Null);
    m.insert("a".into(), serde_json::Value::Null);
    let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    assert_eq!(keys, ["a", "b"], "serde_json/preserve_order is enabled");
}

fn keys(v: &Value) -> Vec<String> {
    v.as_map()
        .expect("a map")
        .keys()
        .map(|k| k.as_str().unwrap_or_default().to_owned())
        .collect()
}

fn assert_sorted(label: &str, v: &Value) {
    let k = keys(v);
    let mut sorted = k.clone();
    sorted.sort();
    assert_eq!(k, sorted, "{label}: keys not sorted");
    for inner in v.as_map().expect("a map").values() {
        if inner.is_map() {
            assert_sorted(label, inner);
        }
    }
}

#[test]
fn map_outputs_are_sorted() {
    let h = Harness::new();
    let ctx = Context::new();
    for (label, expr) in [
        (
            "merge",
            "{'z': 1, 'b': {'y': 1, 'x': 2} } | merge(with={'c': 1, 'a': {'q': 1} })",
        ),
        ("sort_keys", "{'z': 1, 'a': 2, 'm': 3} | sort_keys"),
        ("parse_url", "'https://example.org/x?q=1#f' | parse_url"),
        ("to_date", "'2024-01-01' | to_date"),
        ("now", "now()"),
    ] {
        let v = h
            .eval(expr, &ctx)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_sorted(label, &v);
    }
    // text outputs of maps: jsonify, dump and remarshal write keys in order
    let out = h
        .render(
            "{{ {'z': 1, 'a': {'y': 1, 'b': 2} } | jsonify }}|{{ {'z': 1, 'a': 2} | remarshal(format='yaml') }}",
            &ctx,
        )
        .unwrap();
    assert_eq!(out, "{\"a\":{\"b\":2,\"y\":1},\"z\":1}|a: 2\nz: 1\n");
}
