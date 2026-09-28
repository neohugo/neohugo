//! Walk order and prefix lookups against `tools/go-oracle/nh-doctree/order`: every key set is
//! inserted into a SimpleTree and a NodeShiftTree and compared with Go's Walk / All /
//! WalkPrefixRaw order and, per query string, Get, LongestPrefix, WalkPrefix, WalkPath,
//! LongestPrefixAll and a prefix walk. Also checks the plan's claim that the walk order is the
//! byte order of a `BTreeMap<String, _>`.

mod common;

use std::sync::Arc;

use common::{arr, fixture, i, s, u};
use nh_doctree::dimensions::{Dimension, DimensionFlag};
use nh_doctree::nodeshifttree::{NodeShiftTree, NodeShiftTreeWalker, Shifter, WalkConfig};
use nh_doctree::simpletree::SimpleTree;
use serde_json::Value;

/// Go's `echo` shifter of the oracle.
struct Echo;

impl Shifter<String> for Echo {
    fn for_each_in_dimension(&self, n: &String, _d: usize, f: &mut dyn FnMut(&String) -> bool) {
        f(n);
    }
    fn insert(&self, old: String, new: String) -> (String, Option<String>, bool) {
        (new, Some(old), true)
    }
    fn insert_into(
        &self,
        old: String,
        new: String,
        _: Dimension,
    ) -> (String, Option<String>, bool) {
        (new, Some(old), true)
    }
    fn delete(&self, v: String, _: Dimension) -> (Option<String>, bool, bool) {
        (Some(v), true, true)
    }
    fn shift(&self, v: &String, _: Dimension, _: bool) -> (Option<String>, bool, DimensionFlag) {
        (Some(v.clone()), true, DimensionFlag::LANGUAGE)
    }
}

fn idxs(v: &Value) -> Vec<usize> {
    arr(v).iter().map(u).collect()
}

fn opt_idx(v: &Value) -> Option<usize> {
    let n = i(v);
    (n >= 0).then_some(n as usize)
}

#[test]
fn order() {
    let fx = fixture("order/order.json.gz");
    let mut checks = 0usize;
    for set in arr(&fx["sets"]) {
        let name = s(&set["name"]);
        let keys: Vec<String> = arr(&set["keys"]).iter().map(|k| s(k).to_string()).collect();
        let index = |k: &str| keys.iter().position(|x| x == k).expect("known key");
        let pos: std::collections::HashMap<&str, usize> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.as_str(), i))
            .collect();

        let mut st = SimpleTree::new();
        let mut nt = NodeShiftTree::new(Arc::new(Echo));
        for k in &keys {
            st.insert(k, format!("v{k}"));
            nt.insert_raw(k, format!("v{k}"));
        }

        let walk = idxs(&set["walk"]);
        let mut got = Vec::new();
        st.walk(&mut |k, v| {
            assert_eq!(v, &format!("v{k}"));
            got.push(pos[k]);
            Ok(false)
        })
        .unwrap();
        assert_eq!(got, walk, "{name}: Walk");

        // The plan's claim: radix walk order == byte order (BTreeMap<String, _>).
        let mut sorted: Vec<usize> = (0..keys.len()).collect();
        sorted.sort_by(|a, b| keys[*a].cmp(&keys[*b]));
        assert_eq!(sorted, walk, "{name}: byte order");

        let all: Vec<usize> = st.all().map(|(k, _)| pos[k.as_str()]).collect();
        assert_eq!(all, idxs(&set["all"]), "{name}: All");

        let mut raw = Vec::new();
        nt.walk_prefix_raw("", &mut |k, _| {
            raw.push(pos[k]);
            false
        });
        assert_eq!(raw, idxs(&set["raw"]), "{name}: WalkPrefixRaw");
        checks += 4;

        for q in arr(&set["queries"]) {
            let qs = s(&q["q"]);
            let ctx = format!("{name}: {qs:?}");

            let get = st.get(qs).map(|v| index(&v[1..]));
            assert_eq!(get, opt_idx(&q["g"]), "{ctx}: Get");

            let lp = st.longest_prefix(qs).map(|(k, v)| {
                assert_eq!(&v[1..], k);
                pos[k]
            });
            assert_eq!(lp, opt_idx(&q["l"]), "{ctx}: LongestPrefix");

            let mut p = Vec::new();
            st.walk_prefix(qs, &mut |k, _| {
                p.push(pos[k]);
                Ok(false)
            })
            .unwrap();
            assert_eq!(p, idxs(&q["p"]), "{ctx}: WalkPrefix");

            let mut w = Vec::new();
            st.walk_path(qs, &mut |k, _| {
                w.push(pos[k]);
                Ok(false)
            })
            .unwrap();
            assert_eq!(w, idxs(&q["w"]), "{ctx}: WalkPath");

            let lpa = nt.longest_prefix_all(qs).map(|k| pos[k.as_str()]);
            assert_eq!(lpa, opt_idx(&q["a"]), "{ctx}: LongestPrefixAll");

            let mut n = Vec::new();
            let mut handle = |k: &str, _: &String, _: DimensionFlag| {
                n.push(pos[k]);
                Ok(false)
            };
            let mut walker = NodeShiftTreeWalker::new(&nt, [0], &mut handle);
            walker.prefix = qs.to_string();
            walker.walk().unwrap();
            assert_eq!(n, idxs(&q["n"]), "{ctx}: Walk with prefix");

            // Same through the WalkState API.
            let mut n2 = Vec::new();
            let cfg = WalkConfig {
                prefix: qs.to_string(),
                ..Default::default()
            };
            nt.walk(&cfg, |_, k, _, _| {
                n2.push(pos[k]);
                Ok(false)
            })
            .unwrap();
            assert_eq!(n2, n, "{ctx}: walk");
            checks += 7;
        }
    }
    assert!(checks > 200_000, "{checks}");
    eprintln!("order: {checks} checks");
}
