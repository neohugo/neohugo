//! Differential test of the x/net/html port (`html.Parse`) against the Go oracle
//! `tools/go-oracle/nh-publisher/xnethtml`: every html5lib tree-construction input shipped with
//! x/net/html, the collector's adversarial element strings and random tag soup. The trees are
//! compared through a full dump (type, namespace, data, atom and attributes of every node).

mod common;

use nh_publisher::xnethtml::{self, Document, NodeId, NodeType};

/// The oracle's `Dump`.
fn dump(d: &Document, id: NodeId, level: usize, out: &mut String) {
    let q = go_strconv::quote_to_ascii;
    let n = &d.nodes[id];
    out.push_str(&" ".repeat(level));
    match n.typ {
        NodeType::Document => out.push_str("#document"),
        NodeType::Element => out.push_str(&format!(
            "E {} {} {:#x}",
            q(&n.namespace),
            q(&n.data),
            n.data_atom
        )),
        NodeType::Text => out.push_str(&format!("T {}", q(&n.data))),
        NodeType::Comment => out.push_str(&format!("C {}", q(&n.data))),
        NodeType::Doctype => out.push_str(&format!("D {}", q(&n.data))),
        other => out.push_str(&format!("? {other:?} {}", q(&n.data))),
    }
    out.push('\n');
    for a in &n.attr {
        out.push_str(&" ".repeat(level + 1));
        out.push_str(&format!(
            "@ {} {} {}\n",
            q(&a.namespace),
            q(&a.key),
            q(&a.val)
        ));
    }
    for c in d.children(id) {
        dump(d, c, level + 1, out);
    }
}

#[test]
fn parse_matches_go() {
    let recs = common::read_jsonl_gz(&common::fixture("xnethtml/parse.jsonl.gz"));
    assert!(recs.len() > 25_000, "{} records", recs.len());
    let mut failures = Vec::new();
    let mut n_failed = 0;
    for r in &recs {
        let input = common::bytes(&r["in"]);
        let got = xnethtml::parse(&input).map(|(d, root)| {
            let mut s = String::new();
            dump(&d, root, 0, &mut s);
            s
        });
        let ok = match (&got, r.get("dump"), r.get("panic")) {
            (Ok(s), Some(want), None) => s == want.as_str().unwrap(),
            (Err(e), None, Some(p)) => e == p.as_str().unwrap(),
            _ => false,
        };
        if !ok {
            n_failed += 1;
            if failures.len() < 5 {
                failures.push(format!(
                    "input {}\n--- got\n{}\n--- want\n{}",
                    common::show(&input),
                    match &got {
                        Ok(s) => s.clone(),
                        Err(e) => format!("error {e}"),
                    },
                    r.get("dump")
                        .or(r.get("panic"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{n_failed} of {} differ:\n{}",
        recs.len(),
        failures.join("\n")
    );
}

#[test]
fn atoms() {
    use nh_publisher::xnethtml::atom;
    assert_eq!(atom::string(atom::FOREIGN_OBJECT), b"foreignObject");
    assert_eq!(atom::lookup(b"foreignObject"), atom::FOREIGN_OBJECT);
    assert_eq!(atom::lookup(b"foreignobject"), atom::FOREIGNOBJECT);
    assert_eq!(atom::lookup(b"annotation-xml"), atom::ANNOTATION_XML);
    assert_eq!(atom::lookup(b"Div"), 0);
    assert_eq!(atom::lookup(b""), 0);
    assert_eq!(atom::string(0), b"");
}
