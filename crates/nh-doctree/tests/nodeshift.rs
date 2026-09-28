//! NodeShiftTree against `tools/go-oracle/nh-doctree/nodeshift`: operation sequences with a
//! shifter that mirrors Hugo's `contentNodeShifter` (en, th), replayed and compared result by
//! result.

mod common;

use std::sync::Arc;

use common::{TestShifter, Tn, arr, b, fixture, is_branch, repr, s, u};
use nh_doctree::dimensions::DIMENSION_LANGUAGE;
use nh_doctree::nodeshifttree::{NodeShiftTree, WalkConfig};
use serde_json::{Value, json};

/// Replays one operation and returns the result in the fixture's spelling.
fn apply(t: &mut NodeShiftTree<Tn>, o: &Value) -> Value {
    let k = || s(&o["k"]);
    let dims = || t_shape(o);
    match s(&o["o"]) {
        "ins" => {
            let (u, e, updated) =
                t.insert_into_values_dimension_with_lock(t.dims(), k(), Tn::from_def(&o["v"]));
            json!([repr(Some(&u)), repr(e.as_ref()), updated])
        }
        "insc" => {
            let dims = t.shape(DIMENSION_LANGUAGE, u(&o["d"]));
            let (u, e, updated) = t.insert_into_current_dimension(dims, k(), Tn::from_def(&o["v"]));
            json!([repr(Some(&u)), repr(e.as_ref()), updated])
        }
        "insr" => {
            let (old, existed) = t.insert_raw_with_lock(k(), Tn::from_def(&o["v"]));
            json!([repr(old.as_ref()), existed])
        }
        "del" => {
            let (d, ok) = t.delete_with_status(dims(), k());
            json!([repr(d.as_ref()), ok])
        }
        "delp" => json!(t.delete_prefix(dims(), k())),
        "delall" => {
            t.delete_all(k());
            Value::Null
        }
        "delpall" => json!(t.delete_prefix_all(k())),
        "get" => {
            let (g, h) = (t.get(dims(), k()), t.has(dims(), k()));
            json!([repr(g.as_ref()), h])
        }
        "raw" => {
            let r = t.get_raw(k());
            json!([repr(r), r.is_some()])
        }
        "lp" => {
            let not_branch = |n: &Tn| !is_branch(n);
            let pred: Option<&dyn Fn(&Tn) -> bool> = match u(&o["p"]) {
                0 => None,
                1 => Some(&is_branch),
                _ => Some(&not_branch),
            };
            match t.longest_prefix(dims(), k(), b(&o["x"]), pred) {
                Some((key, v)) => json!([key, repr(Some(&v))]),
                None => Value::Null,
            }
        }
        "lpa" => match t.longest_prefix_all(k()) {
            Some(key) => json!(key),
            None => Value::Null,
        },
        "fe" => {
            let stop_at = u(&o["n"]);
            let mut res = Vec::new();
            t.for_each_in_dimension(k(), DIMENSION_LANGUAGE, &mut |n| {
                res.push(repr(Some(n)));
                res.len() == stop_at
            });
            json!(res)
        }
        "walk" => {
            let cfg = WalkConfig {
                dims: dims(),
                prefix: k().to_string(),
                no_shift: b(&o["ns"]),
                exact: b(&o["x"]),
                ..Default::default()
            };
            let mut res = Vec::new();
            t.walk(&cfg, |_, key, n, flag| {
                res.push(json!([key, repr(Some(n)), flag.0]));
                Ok(false)
            })
            .unwrap();
            json!(res)
        }
        "wpr" => {
            let mut res = Vec::new();
            t.walk_prefix_raw(k(), &mut |key, n| {
                res.push(json!([key, repr(Some(n))]));
                false
            });
            json!(res)
        }
        "len" => json!(t.len()),
        other => panic!("unknown op {other}"),
    }
}

fn t_shape(o: &Value) -> [usize; 1] {
    [u(&o["d"])]
}

/// The expected result of an operation in the spelling of [`apply`].
fn expected(o: &Value) -> Value {
    match s(&o["o"]) {
        "delp" | "delpall" | "len" => o["n"].clone(),
        "delall" => Value::Null,
        "get" => json!([o["r"], o["h"]]),
        "raw" => json!([o["r"], o["f"]]),
        _ => o["r"].clone(),
    }
}

#[test]
fn nodeshift() {
    let fx = fixture("nodeshift/nodeshift.json.gz");
    let mut checks = 0usize;
    let mut walked = 0usize;
    for sc in arr(&fx["scenarios"]) {
        let name = s(&sc["name"]);
        let mut t = NodeShiftTree::new(Arc::new(TestShifter));
        for (n, o) in arr(&sc["ops"]).iter().enumerate() {
            let got = apply(&mut t, o);
            assert_eq!(got, expected(o), "{name} op #{n}: {o}");
            if s(&o["o"]) == "walk" {
                walked += arr(&got).len();
            }
            checks += 1;
        }
    }
    assert!(checks > 30_000, "{checks}");
    eprintln!("nodeshift: {checks} operations, {walked} walk visits");
}
