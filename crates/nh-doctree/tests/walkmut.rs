//! Walks whose handle modifies the walked tree, against `tools/go-oracle/nh-doctree/walkmut`:
//! Hugo's assembleTerms / addMissingRootSections / term-delete walks on the docs site's pages,
//! and 3,000 random walks over small dense trees that insert, delete (the visited key, other
//! keys, whole subtrees), skip prefixes, stop and fail. The visits, the walk's result (including
//! Go's nil-pointer panics) and the final tree must be Go's.
//!
//! For the record, the test also replays each scenario with the two `BTreeMap` strategies the
//! plan considered (a snapshot of the keys, re-read at visit time; and a live byte-order cursor)
//! and reports how many scenarios they would get wrong.

mod common;

use std::sync::Arc;

use common::{TestShifter, Tn, arr, b, fixture, repr, s, u};
use nh_common::Error;
use nh_doctree::dimensions::DimensionFlag;
use nh_doctree::nodeshifttree::{NodeShiftTree, WalkConfig};
use serde_json::{Value, json};

struct Outcome {
    visits: Vec<Value>,
    result: String,
    final_: Vec<Value>,
    len: usize,
}

fn build(sc: &Value) -> NodeShiftTree<Tn> {
    let mut t = NodeShiftTree::new(Arc::new(TestShifter));
    for kv in arr(&sc["init"]) {
        let kv = arr(kv);
        t.insert_into_values_dimension(t.dims(), s(&kv[0]), Tn::from_def(&kv[1]));
    }
    t
}

fn config(sc: &Value) -> WalkConfig {
    WalkConfig {
        dims: [u(&sc["d"])],
        prefix: s(&sc["k"]).to_string(),
        no_shift: b(&sc["ns"]),
        exact: b(&sc["x"]),
        ..Default::default()
    }
}

/// Applies the recorded action `a` at the visit of `key`: returns terminate, the prefix to skip.
fn act(
    t: &mut NodeShiftTree<Tn>,
    dims: [usize; 1],
    key: &str,
    a: &[Value],
) -> Result<(bool, Option<String>), Error> {
    match s(&a[0]) {
        "none" => {}
        "ins" => {
            t.insert_into_values_dimension(dims, s(&a[1]), Tn::from_def(&a[2]));
            if a.len() > 3 {
                assert_eq!(s(&a[3]), "skip");
                return Ok((false, Some(s(&a[4]).to_string())));
            }
        }
        "insc" => {
            t.insert_into_current_dimension(dims, s(&a[1]), Tn::from_def(&a[2]));
        }
        "delcur" => {
            t.delete(dims, key);
        }
        "del" => {
            t.delete([u(&a[2])], s(&a[1]));
        }
        "delall" => t.delete_all(s(&a[1])),
        "skip" => return Ok((false, Some(s(&a[1]).to_string()))),
        "stop" | "cap" => return Ok((true, None)),
        "err" => return Err(Error::new("boom")),
        other => panic!("unknown action {other}"),
    }
    Ok((false, None))
}

fn dump(t: &NodeShiftTree<Tn>) -> Vec<Value> {
    let mut out = Vec::new();
    t.walk_prefix_raw("", &mut |k, n| {
        out.push(json!([k, repr(Some(n))]));
        false
    });
    out
}

/// The port: `walk_mut`.
fn replay(sc: &Value) -> Outcome {
    let mut t = build(sc);
    let cfg = config(sc);
    let actions = arr(&sc["actions"]);
    let mut visits = Vec::new();
    let res = t.walk_mut(&cfg, |w, key, n, flag| {
        visits.push(json!([key, repr(Some(n)), flag.0]));
        let Some(a) = actions.get(visits.len() - 1) else {
            // Visiting more than Go: record and stop.
            return Ok(true);
        };
        let dims = w.dims();
        let (stop, skip) = act(w.tree_mut(), dims, key, arr(a))?;
        if let Some(p) = skip {
            w.skip_prefix(&p);
        }
        Ok(stop)
    });
    let result = match res {
        Ok(()) => "ok".to_string(),
        Err(e) if e.to_string() == "boom" => "err:boom".to_string(),
        Err(e) => format!("panic:{e}"),
    };
    Outcome {
        final_: dump(&t),
        len: t.len(),
        visits,
        result,
    }
}

/// A `BTreeMap`-style walk: `live` = the next key in byte order after the last visited one, from
/// the current key set; else a snapshot of the keys taken at the start (deleted keys skipped).
fn replay_btree(sc: &Value, live: bool) -> Vec<Value> {
    let mut t = build(sc);
    let cfg = config(sc);
    let actions = arr(&sc["actions"]);
    let snapshot = t.keys_with_prefix(&cfg.prefix);
    let mut visits = Vec::new();
    let mut skips: Vec<String> = Vec::new();
    let mut last: Option<String> = None;
    let mut next_snapshot = 0;
    loop {
        let key = if live {
            let keys = t.keys_with_prefix(&cfg.prefix);
            match keys
                .into_iter()
                .find(|k| last.as_ref().is_none_or(|l| k > l))
            {
                Some(k) => k,
                None => break,
            }
        } else {
            match snapshot.get(next_snapshot) {
                Some(k) => {
                    next_snapshot += 1;
                    k.clone()
                }
                None => break,
            }
        };
        last = Some(key.clone());
        if skips.iter().any(|p| key.starts_with(p.as_str())) {
            continue;
        }
        let Some(raw) = t.get_raw(&key).cloned() else {
            continue;
        };
        let (n, flag) = if cfg.no_shift {
            (raw, DimensionFlag(0))
        } else {
            let mut found = None;
            let cfg2 = WalkConfig {
                prefix: key.clone(),
                ..cfg.clone()
            };
            t.walk(&cfg2, |_, k, n, f| {
                if k == key {
                    found = Some((n.clone(), f));
                }
                Ok(true)
            })
            .unwrap();
            match found {
                Some(x) => x,
                None => continue,
            }
        };
        visits.push(json!([key, repr(Some(&n)), flag.0]));
        let Some(a) = actions.get(visits.len() - 1) else {
            break;
        };
        match act(&mut t, cfg.dims, &key, arr(a)) {
            Ok((stop, skip)) => {
                if let Some(p) = skip {
                    skips.push(p);
                }
                if stop {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    visits
}

#[test]
fn walkmut() {
    let fx = fixture("walkmut/walkmut.json.gz");
    let (mut visits, mut panics, mut hugo) = (0usize, 0usize, 0usize);
    let (mut snapshot_wrong, mut live_wrong, mut hugo_snapshot_wrong, mut hugo_live_wrong) =
        (0usize, 0usize, 0usize, 0usize);
    let scenarios = arr(&fx["scenarios"]);
    for sc in scenarios {
        let name = s(&sc["name"]);
        let got = replay(sc);
        let want: Vec<Value> = arr(&sc["visits"]).clone();
        assert_eq!(got.visits, want, "{name}: visits");
        assert_eq!(got.result, s(&sc["result"]), "{name}: result");
        assert_eq!(&got.final_, arr(&sc["final"]), "{name}: final tree");
        assert_eq!(got.len, u(&sc["len"]), "{name}: Len");
        visits += got.visits.len();
        if got.result.starts_with("panic:") {
            panics += 1;
        }

        let is_hugo = !name.starts_with("random-");
        hugo += is_hugo as usize;
        if replay_btree(sc, false) != want {
            snapshot_wrong += 1;
            hugo_snapshot_wrong += is_hugo as usize;
        }
        if replay_btree(sc, true) != want {
            live_wrong += 1;
            hugo_live_wrong += is_hugo as usize;
        }
    }
    assert!(visits > 15_000, "{visits}");
    assert!(panics > 0, "no scenario reaches Go's nil dereference");
    eprintln!(
        "walkmut: {} scenarios ({hugo} Hugo-shaped), {visits} visits, {panics} Go panics reproduced; \
         a BTreeMap key snapshot would differ in {snapshot_wrong} ({hugo_snapshot_wrong} Hugo-shaped), \
         a live BTreeMap cursor in {live_wrong} ({hugo_live_wrong} Hugo-shaped)",
        scenarios.len()
    );
}
