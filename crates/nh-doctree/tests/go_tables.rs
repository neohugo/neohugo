//! hugolib/doctree's `*_test.go`, ported (dimensions_test.go, nodeshiftree_test.go,
//! treeshifttree_test.go), plus checks of the Rust-only API edges.

use std::sync::{Arc, Mutex};

use nh_common::Result;
use nh_doctree::dimensions::{DIMENSION_LANGUAGE, Dimension, DimensionFlag};
use nh_doctree::nodeshifttree::{NodeShiftTree, NodeShiftTreeWalker, Shifter, WalkConfig};
use nh_doctree::simpletree::SimpleTree;
use nh_doctree::support::{Event, WalkContext, validate_key};
use nh_doctree::treeshifttree::TreeShiftTree;

// Go: dimensions_test.go:TestDimensionFlag
#[test]
fn test_dimension_flag() {
    let zero = DimensionFlag(0);
    let mut d = DimensionFlag(0);
    let o = DimensionFlag(1);
    let p = DimensionFlag(12);

    assert!(!d.has(o));
    d = d.set(o);
    assert!(d.has(o));
    assert!(d.has(d));
    let r = std::panic::catch_unwind(|| zero.index());
    let msg = r.unwrap_err();
    assert_eq!(msg.downcast_ref::<&str>(), Some(&"dimension flag not set"));
    assert_eq!(DimensionFlag::LANGUAGE.index(), 0);
    assert_eq!(DimensionFlag::LANGUAGE.index(), DIMENSION_LANGUAGE);
    assert_eq!(p.index(), 11);
}

/// Go: `testValue`.
#[derive(Debug)]
struct TestValue {
    id: String,
    lang: usize,
    weight: Mutex<i64>,
    is_branch: bool,
    no_copy: bool,
}

type Tv = Arc<TestValue>;

fn tv(id: &str) -> Tv {
    Arc::new(TestValue {
        id: id.to_string(),
        lang: 0,
        weight: Mutex::new(0),
        is_branch: false,
        no_copy: false,
    })
}

fn tvw(id: &str, weight: i64, is_branch: bool) -> Tv {
    Arc::new(TestValue {
        id: id.to_string(),
        lang: 0,
        weight: Mutex::new(weight),
        is_branch,
        no_copy: false,
    })
}

fn weight(v: &Tv) -> i64 {
    *v.weight.lock().unwrap()
}

/// Go's `eq` comparer: same pointer, or same ID and Lang.
fn eq(a: Option<&Tv>, id: &str, lang: usize) -> bool {
    a.is_some_and(|a| a.id == id && a.lang == lang)
}

/// Go: `testShifter`.
struct TestShifter {
    echo: bool,
}

impl Shifter<Tv> for TestShifter {
    fn for_each_in_dimension(&self, n: &Tv, d: usize, f: &mut dyn FnMut(&Tv) -> bool) {
        assert_eq!(d, DimensionFlag::LANGUAGE.index(), "not implemented");
        f(n);
    }
    fn insert(&self, old: Tv, new: Tv) -> (Tv, Option<Tv>, bool) {
        (new, Some(old), true)
    }
    fn insert_into(&self, old: Tv, new: Tv, _: Dimension) -> (Tv, Option<Tv>, bool) {
        (new, Some(old), true)
    }
    fn delete(&self, _v: Tv, _: Dimension) -> (Option<Tv>, bool, bool) {
        (None, true, true)
    }
    fn shift(
        &self,
        n: &Tv,
        dimension: Dimension,
        _exact: bool,
    ) -> (Option<Tv>, bool, DimensionFlag) {
        if self.echo {
            return (Some(n.clone()), true, DimensionFlag::LANGUAGE);
        }
        if n.no_copy {
            if n.lang == dimension[0] {
                return (Some(n.clone()), true, DimensionFlag::LANGUAGE);
            }
            return (None, false, DimensionFlag::LANGUAGE);
        }
        let c = Arc::new(TestValue {
            id: n.id.clone(),
            lang: dimension[0],
            weight: Mutex::new(weight(n)),
            is_branch: n.is_branch,
            no_copy: n.no_copy,
        });
        (Some(c), true, DimensionFlag::LANGUAGE)
    }
}

fn new_tree(echo: bool) -> NodeShiftTree<Tv> {
    NodeShiftTree::new(Arc::new(TestShifter { echo }))
}

// Go: nodeshiftree_test.go:TestTree
#[test]
fn test_tree() {
    let mut zero_zero = new_tree(false);
    let dims = zero_zero.dims();

    let a = tv("/a");
    zero_zero.insert_into_values_dimension(dims, "/a", a);
    let ab = tv("/a/b");
    zero_zero.insert_into_values_dimension(dims, "/a/b", ab.clone());

    assert!(eq(zero_zero.get(dims, "/a").as_ref(), "/a", 0));
    let (s, v) = zero_zero
        .longest_prefix(dims, "/a/b/c", true, None)
        .unwrap();
    assert!(eq(Some(&v), &ab.id, ab.lang));
    assert_eq!(s, "/a/b");

    // Change language.
    let one_zero = zero_zero.increment(dims, 0);
    assert!(eq(zero_zero.get(dims, "/a").as_ref(), "/a", 0));
    assert!(eq(zero_zero.get(one_zero, "/a").as_ref(), "/a", 1));
    assert_eq!(zero_zero.string(one_zero), "Root{[1]}");
}

// Go: nodeshiftree_test.go:TestTreeData
#[test]
fn test_tree_data() {
    let mut tree = new_tree(false);
    let dims = tree.dims();
    for (k, id) in [
        ("", "HOME"),
        ("/a", "/a"),
        ("/a/b", "/a/b"),
        ("/b", "/b"),
        ("/b/c", "/b/c"),
        ("/b/c/d", "/b/c/d"),
    ] {
        tree.insert_into_values_dimension(dims, k, tv(id));
    }

    let mut values: Vec<String> = Vec::new();
    // Go stores map[string]any{"id": t.ID} and prints it with %v.
    let mut ctx: WalkContext<Tv, String> = WalkContext::default();

    tree.walk(&WalkConfig::default(), |_, s, t, _| {
        ctx.data().insert(s, format!("map[id:{}]", t.id));
        if !s.is_empty() {
            let (p, v) = ctx.data().longest_prefix(&go_dir(s)).unwrap();
            values.push(format!("{s}:{p}:{v}"));
        }
        Ok(false)
    })
    .unwrap();

    assert_eq!(
        values.join("|"),
        "/a::map[id:HOME]|/a/b:/a:map[id:/a]|/b::map[id:HOME]|/b/c:/b:map[id:/b]|/b/c/d:/b/c:map[id:/b/c]"
    );
}

/// Go `path.Dir` for the keys of test_tree_data.
fn go_dir(s: &str) -> String {
    match s.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => s[..i].to_string(),
        None => ".".to_string(),
    }
}

// Go: nodeshiftree_test.go:TestTreeEvents
#[test]
fn test_tree_events() {
    let mut tree = new_tree(true);
    let dims = tree.dims();
    for (k, id, w, br) in [
        ("/a", "/a", 2, true),
        ("/a/p1", "/a/p1", 5, false),
        ("/a/p", "/a/p2", 6, false),
        ("/a/s1", "/a/s1", 5, true),
        ("/a/s1/p1", "/a/s1/p1", 8, false),
        ("/a/s1/p1", "/a/s1/p2", 9, false),
        ("/a/s1/s2", "/a/s1/s2", 6, true),
        ("/a/s1/s2/p1", "/a/s1/s2/p1", 8, false),
        ("/a/s1/s2/p2", "/a/s1/s2/p2", 7, false),
    ] {
        tree.insert_into_values_dimension(dims, k, tvw(id, w, br));
    }

    let mut ctx: WalkContext<Tv> = WalkContext::new();
    let sender = ctx.event_sender();
    tree.walk(&WalkConfig::default(), |_, s, t, _| {
        if t.is_branch {
            let (t, s2, sender) = (t.clone(), s.to_string(), sender.clone());
            ctx.add_event_listener(
                "weight",
                s,
                Box::new(move |e: &mut Event<Tv>| {
                    if weight(&e.source) > weight(&t) {
                        *t.weight.lock().unwrap() = weight(&e.source);
                        sender.send(Event::new("weight", s2.clone(), t.clone()));
                    }
                    // Reduces the amount of events bubbling up the tree. If the weight for this
                    // branch has increased, that will be announced in its own event.
                    e.stop_propagation();
                }),
            );
        } else {
            ctx.send_event(Event::new("weight", s, t.clone()));
        }
        Ok(false)
    })
    .unwrap();
    ctx.handle_events_and_hooks().unwrap();

    let w = |k: &str| weight(&tree.get(dims, k).unwrap());
    assert_eq!(w("/a"), 9);
    assert_eq!(w("/a/s1"), 9);
    assert_eq!(w("/a/p"), 6);
    assert_eq!(w("/a/s1/s2"), 8);
    assert_eq!(w("/a/s1/s2/p2"), 7);
}

// Go: nodeshiftree_test.go:TestTreeInsert
#[test]
fn test_tree_insert() {
    let mut tree = new_tree(false);
    let dims = tree.dims();

    tree.insert_into_values_dimension(dims, "/a", tv("/a"));
    tree.insert_into_values_dimension(dims, "/a/b", tv("/a/b"));

    assert!(eq(tree.get(dims, "/a").as_ref(), "/a", 0));
    assert!(tree.get(dims, "/notfound").is_none());

    let ab2 = tv("/a/b");
    let (v, _, ok) = tree.insert_into_values_dimension(dims, "/a/b", ab2.clone());
    assert!(ok);
    assert!(Arc::ptr_eq(&v, &ab2));

    let tree1 = tree.increment(dims, 0);
    assert!(eq(tree.get(tree1, "/a/b").as_ref(), "/a/b", 1));
}

// Go: nodeshiftree_test.go:TestTreePara (Go's Lock(true) is the Mutex).
#[test]
fn test_tree_para() {
    let tree = Arc::new(Mutex::new(new_tree(false)));
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let tree = tree.clone();
            std::thread::spawn(move || {
                let a = tv("/a");
                let mut tree = tree.lock().unwrap();
                let dims = tree.dims();
                tree.insert_into_values_dimension(dims, "/a", a);
                tree.insert_into_values_dimension(dims, "/a/b", tv("/a/b"));
                let key = format!("/a/b/c/{i}");
                let val = tv(&key);
                tree.insert_into_values_dimension(dims, &key, val.clone());
                assert!(eq(tree.get(dims, &key).as_ref(), &val.id, val.lang));
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(tree.lock().unwrap().len(), 10);
}

// Go: nodeshiftree_test.go:TestValidateKey
#[test]
fn test_validate_key() {
    assert!(validate_key("").is_ok());
    assert!(validate_key("/a/b/c").is_ok());
    assert!(validate_key("/").is_err());
    assert!(validate_key("a").is_err());
    assert!(validate_key("abc").is_err());
    assert!(validate_key("/abc/").is_err());

    // Go's messages (%q).
    assert_eq!(
        validate_key("/").unwrap_err().to_string(),
        r#"too short key: "/""#
    );
    assert_eq!(
        validate_key("abc").unwrap_err().to_string(),
        r#"key must start with '/': "abc""#
    );
    assert_eq!(
        validate_key("/abc/").unwrap_err().to_string(),
        r#"key must not end with '/': "/abc/""#
    );
    assert_eq!(
        validate_key("ข").unwrap_err().to_string(),
        r#"key must start with '/': "ข""#
    );
    assert_eq!(
        validate_key("\t").unwrap_err().to_string(),
        r#"too short key: "\t""#
    );
}

// Go: treeshifttree_test.go:TestTreeShiftTree
#[test]
fn test_tree_shift_tree() {
    let tree = TreeShiftTree::<String>::with_dimension(0, 10);
    assert_eq!(tree.len_raw(), 0);
    assert_eq!(tree.shape(0, 9), 9);
}

// ---------------------------------------------------------------------------
// Rust-only API edges.

#[test]
#[should_panic(expected = "key must not end with '/'")]
fn insert_invalid_key_panics_like_go() {
    // Go: mustValidateKey panics.
    let mut tree = new_tree(false);
    let dims = tree.dims();
    tree.insert_into_values_dimension(dims, "/a/", tv("/a"));
}

#[test]
fn longest_prefix_relative_path_terminates() {
    // Go loops forever here ("." is a fixed point of path.Dir); the port returns None.
    let mut tree = NodeShiftTree::new(Arc::new(TestShifter { echo: false }));
    let dims = tree.dims();
    let mut v = tv("/a");
    Arc::get_mut(&mut v).unwrap().no_copy = true;
    tree.insert_into_values_dimension(dims, "", v);
    assert!(tree.longest_prefix([1], "a/b", true, None).is_none());
    assert!(tree.longest_prefix([0], "a/b", true, None).is_some());
}

#[test]
#[should_panic(expected = "delete_in_place must be implemented")]
fn default_delete_in_place_refuses_partial_deletes() {
    struct Partial;
    impl Shifter<u8> for Partial {
        fn for_each_in_dimension(&self, _: &u8, _: usize, _: &mut dyn FnMut(&u8) -> bool) {}
        fn insert(&self, _: u8, new: u8) -> (u8, Option<u8>, bool) {
            (new, None, false)
        }
        fn insert_into(&self, _: u8, new: u8, _: Dimension) -> (u8, Option<u8>, bool) {
            (new, None, false)
        }
        fn delete(&self, v: u8, _: Dimension) -> (Option<u8>, bool, bool) {
            (Some(v), true, false)
        }
        fn shift(&self, v: &u8, _: Dimension, _: bool) -> (Option<u8>, bool, DimensionFlag) {
            (Some(*v), true, DimensionFlag::LANGUAGE)
        }
    }
    let mut tree = NodeShiftTree::new(Arc::new(Partial));
    tree.insert_raw("/a", 1);
    tree.delete([0], "/a");
}

#[test]
fn walker_resets_skipped_prefixes_like_go() {
    let mut tree = new_tree(true);
    let dims = tree.dims();
    for k in ["/a", "/a/b", "/b"] {
        tree.insert_into_values_dimension(dims, k, tv(k));
    }
    let mut seen = Vec::new();
    let mut handle = |k: &str, _: &Tv, _: DimensionFlag| -> Result<bool> {
        seen.push(k.to_string());
        Ok(false)
    };
    let mut w = NodeShiftTreeWalker::new(&tree, dims, &mut handle);
    // Go: Walk starts with resetLocalState, so a prefix skipped before the walk is visited.
    w.skip_prefix("/a");
    assert!(w.should_skip("/a/b"));
    w.walk().unwrap();
    assert!(!w.should_skip("/a/b"));
    assert_eq!(w.extend().dims, dims);
    assert_eq!(seen, ["/a", "/a/b", "/b"]);

    // SkipPrefix from the handle.
    let mut seen = Vec::new();
    tree.walk(&WalkConfig::default(), |w, k, _, _| {
        seen.push(k.to_string());
        w.skip_prefix(&format!("{k}/"));
        Ok(false)
    })
    .unwrap();
    assert_eq!(seen, ["/a", "/b"]);
}

#[test]
fn simple_tree_walk_stops_and_fails_like_go() {
    let mut t = SimpleTree::new();
    for k in ["", "/a", "/a/b", "/b"] {
        t.insert(k, k.len());
    }
    let mut seen = Vec::new();
    t.walk(&mut |k, _| {
        seen.push(k.to_string());
        Ok(k == "/a")
    })
    .unwrap();
    assert_eq!(seen, ["", "/a"]);
    let err = t
        .walk_path("/a/b/c", &mut |k, _| {
            if k == "/a" {
                return Err(nh_common::Error::new("boom"));
            }
            Ok(false)
        })
        .unwrap_err();
    assert_eq!(err.to_string(), "boom");
    assert_eq!(t.len(), 4);
    assert_eq!(t.get_mut("/b").map(|v| std::mem::replace(v, 9)), Some(2));
    assert_eq!(t.get("/b"), Some(&9));
    assert_eq!(format!("{t:?}"), r#"{"": 0, "/a": 2, "/a/b": 4, "/b": 9}"#);
}
