//! WalkContext events and post hooks against `tools/go-oracle/nh-doctree/events` (Go's
//! TestTreeEvents and 400 random trees): listeners on branch nodes raise their node's weight
//! and re-send, with or without StopPropagation, one or two event names, failing hooks.

mod common;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use common::{arr, b, fixture, i, s, u};
use nh_common::Error;
use nh_doctree::dimensions::{Dimension, DimensionFlag};
use nh_doctree::nodeshifttree::{NodeShiftTree, Shifter, WalkConfig};
use nh_doctree::support::{Event, WalkContext};

struct WNode {
    key: String,
    weight: Mutex<i64>,
    branch: bool,
}

type N = Arc<WNode>;

struct Echo;

impl Shifter<N> for Echo {
    fn for_each_in_dimension(&self, n: &N, _d: usize, f: &mut dyn FnMut(&N) -> bool) {
        f(n);
    }
    fn insert(&self, old: N, new: N) -> (N, Option<N>, bool) {
        (new, Some(old), true)
    }
    fn insert_into(&self, old: N, new: N, _: Dimension) -> (N, Option<N>, bool) {
        (new, Some(old), true)
    }
    fn delete(&self, v: N, _: Dimension) -> (Option<N>, bool, bool) {
        (Some(v), true, true)
    }
    fn shift(&self, v: &N, _: Dimension, _: bool) -> (Option<N>, bool, DimensionFlag) {
        (Some(v.clone()), true, DimensionFlag::LANGUAGE)
    }
}

fn weight(n: &N) -> i64 {
    *n.weight.lock().unwrap()
}

#[test]
fn events() {
    let fx = fixture("events/events.json.gz");
    let mut calls = 0usize;
    for sc in arr(&fx["scenarios"]) {
        let name = s(&sc["name"]);
        let names: Vec<String> = arr(&sc["names"]).iter().map(|n| s(n).to_string()).collect();
        let stop = b(&sc["stop"]);
        let fail_hook = i(&sc["failhook"]);

        let mut tree = NodeShiftTree::new(Arc::new(Echo));
        let all: Vec<N> = arr(&sc["nodes"])
            .iter()
            .map(|n| {
                let n = arr(n);
                Arc::new(WNode {
                    key: s(&n[0]).to_string(),
                    weight: Mutex::new(i(&n[1])),
                    branch: b(&n[2]),
                })
            })
            .collect();
        for n in &all {
            tree.insert_into_values_dimension([0], &n.key, n.clone());
        }

        let log = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut ctx: WalkContext<N> = WalkContext::new();
        let mut idx = 0;
        tree.walk(&WalkConfig::default(), |_, key, t, _| {
            let event_name = names[idx % names.len()].clone();
            idx += 1;
            if t.branch {
                let (t, key, log, sender) =
                    (t.clone(), key.to_string(), log.clone(), ctx.event_sender());
                let en = event_name.clone();
                ctx.add_event_listener(
                    &event_name,
                    &key.clone(),
                    Box::new(move |e: &mut Event<N>| {
                        let w = weight(&e.source);
                        log.borrow_mut()
                            .push(format!("{en}:{key}<-{}:{}:{w}", e.path, e.source.key));
                        if w > weight(&t) {
                            *t.weight.lock().unwrap() = w;
                            sender.send(Event::new(en.clone(), key.clone(), t.clone()));
                        }
                        if stop {
                            e.stop_propagation();
                        }
                    }),
                );
            } else {
                ctx.send_event(Event::new(event_name, key, t.clone()));
            }
            Ok(false)
        })
        .unwrap();

        for h in 0..3 {
            let log = log.clone();
            ctx.add_post_hook(Box::new(move || {
                log.borrow_mut().push(format!("hook{h}"));
                if h == fail_hook {
                    return Err(Error::new("hook failed"));
                }
                Ok(())
            }));
        }
        let result = match ctx.handle_events_and_hooks() {
            Ok(()) => "ok".to_string(),
            Err(e) => e.to_string(),
        };

        let want_log: Vec<String> = arr(&sc["log"]).iter().map(|l| s(l).to_string()).collect();
        assert_eq!(*log.borrow(), want_log, "{name}: handler calls");
        let weights: Vec<usize> = all.iter().map(|n| weight(n) as usize).collect();
        let want_weights: Vec<usize> = arr(&sc["weights"]).iter().map(u).collect();
        assert_eq!(weights, want_weights, "{name}: weights");
        assert_eq!(result, s(&sc["result"]), "{name}: result");
        calls += want_log.len();
    }
    assert!(calls > 2000, "{calls}");
    eprintln!("events: {calls} handler and hook calls");
}
