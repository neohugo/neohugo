//! TreeShiftTree (one SimpleTree per language) and SimpleTree walks against
//! `tools/go-oracle/nh-doctree/treeshift`.

mod common;

use common::{arr, fixture, i, s, u};
use nh_common::{Error, Result};
use nh_doctree::treeshifttree::TreeShiftTree;
use serde_json::{Value, json};

/// Go's `walkFn`: records the visits; stops at visit `stop`, fails at visit `fail`.
fn walk_fn(
    res: &mut Vec<Value>,
    stop: i64,
    fail: i64,
) -> impl FnMut(&str, &String) -> Result<bool> + '_ {
    move |k, v| {
        res.push(json!([k, v]));
        let n = res.len() as i64;
        if n == fail {
            return Err(Error::new("boom"));
        }
        Ok(n == stop)
    }
}

fn err_string(r: Result<()>) -> String {
    match r {
        Ok(()) => String::new(),
        Err(e) => e.to_string(),
    }
}

/// Replays `o` and returns (result, error) in the fixture's spelling.
fn apply(t: &mut TreeShiftTree<String>, o: &Value) -> (Value, Option<String>) {
    let k = || s(&o["k"]);
    let v = || t.shape(0, u(&o["d"]));
    match s(&o["o"]) {
        "ins" => {
            let val = s(&o["v"]).to_string();
            let d = v();
            t.insert(d, k(), val.clone());
            // Go's Insert returns the inserted value.
            (json!(val), None)
        }
        "get" => (json!(t.get(v(), k()).cloned().unwrap_or_default()), None),
        "lp" => {
            let r = t
                .longest_prefix(v(), k())
                .map_or(["".to_string(), "".to_string()], |(a, b)| {
                    [a.to_string(), b.clone()]
                });
            (json!(r), None)
        }
        "wp" | "wpath" | "wpr" => {
            let (stop, fail) = (i(&o["s"]), i(&o["f"]));
            let mut res = Vec::new();
            let err = {
                let mut f = walk_fn(&mut res, stop, fail);
                match s(&o["o"]) {
                    "wp" => t.walk_prefix(v(), k(), &mut f),
                    "wpath" => t.walk_path(v(), k(), &mut f),
                    _ => t.walk_prefix_raw(k(), &mut f),
                }
            };
            (json!(res), Some(err_string(err)))
        }
        "all" => {
            let res: Vec<Value> = t.all(v()).map(|(k, v)| json!([k, v])).collect();
            (json!(res), None)
        }
        "len" => (json!(t.len_raw()), None),
        "del" => {
            t.delete(k());
            (Value::Null, None)
        }
        "delp" => (json!(t.delete_prefix(k())), None),
        "delf" => {
            let mut res = Vec::new();
            t.delete_all_func(k(), &mut |key, val| {
                res.push(json!([key, val]));
                let n: u64 = val[1..].parse().unwrap();
                n % 2 == 1
            });
            (json!(res), None)
        }
        other => panic!("unknown op {other}"),
    }
}

#[test]
fn treeshift() {
    let fx = fixture("treeshift/treeshift.json.gz");
    let mut checks = 0usize;
    for sc in arr(&fx["scenarios"]) {
        let name = s(&sc["name"]);
        let mut t = TreeShiftTree::new(2);
        for (n, o) in arr(&sc["ops"]).iter().enumerate() {
            let (got, err) = apply(&mut t, o);
            let want = o.get("r").cloned().unwrap_or(Value::Null);
            assert_eq!(got, want, "{name} op #{n}: {o}");
            if let Some(err) = err {
                assert_eq!(err, s(&o["e"]), "{name} op #{n}: error of {o}");
            }
            checks += 1;
        }
    }
    assert!(checks > 40_000, "{checks}");
    eprintln!("treeshift: {checks} operations");
}

#[test]
#[should_panic(expected = "length must be > 0")]
fn new_tree_shift_tree_zero_length() {
    // Go: NewTreeShiftTree panics on length 0.
    let _ = TreeShiftTree::<String>::new(0);
}

#[test]
#[should_panic(expected = "value out of range")]
fn shape_out_of_range() {
    TreeShiftTree::<String>::new(2).shape(0, 2);
}

#[test]
#[should_panic(expected = "dimension mismatch")]
fn shape_dimension_mismatch() {
    TreeShiftTree::<String>::new(2).shape(1, 0);
}
