//! Differential test of the absURL / absURLInXML replacer and `transform.Chain` against the Go
//! oracle `tools/go-oracle/nh-transform/absurl` (upstream test tables, the output-publishing
//! spec vectors, a prefix × quote × value × suffix grid and random documents).

mod common;

use nh_transform::chain::Chain;
use nh_transform::urlreplacers::absurl::{new_abs_url_in_xml_transformer, new_abs_url_transformer};

#[test]
fn absurl_matches_go() {
    let cases = common::read_jsonl_gz(&common::fixture("absurl/cases.jsonl.gz"));
    assert!(cases.len() > 90_000, "{} cases", cases.len());
    let mut failures = Vec::new();
    let mut n_failed = 0;
    let mut panics = 0;
    for c in &cases {
        let kind = c["k"].as_str().unwrap();
        let path = common::string(&c["p"]);
        let input = common::bytes(&c["in"]);
        let tr = if kind == "html" {
            new_abs_url_transformer(&path)
        } else {
            new_abs_url_in_xml_transformer(&path)
        };
        let got = Chain::new(vec![tr]).apply(&input);
        let ok = match (&got, c.get("panic"), common::opt_bytes(c, "out")) {
            (Err(e), Some(p), _) => {
                panics += 1;
                e.to_string().contains(p.as_str().unwrap())
            }
            (Ok(out), None, Some(want)) => *out == want,
            _ => false,
        };
        if !ok {
            n_failed += 1;
            if failures.len() < 20 {
                failures.push(format!(
                    "{kind} {path:?} in={} got={:?} want={}",
                    common::show(&input),
                    got.map(|o| common::show(&o)),
                    c
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{n_failed} failures:\n{}",
        failures.join("\n")
    );
    assert!(panics > 0, "the Go panics are covered");
}

/// The upstream `absurlreplacer_test.go` expectations, verbatim.
#[test]
fn upstream_tables() {
    let base = "http://base/";
    let apply = |xml: bool, path: &str, s: &str| {
        let tr = if xml {
            new_abs_url_in_xml_transformer(path)
        } else {
            new_abs_url_transformer(path)
        };
        String::from_utf8(Chain::new(vec![tr]).apply(s.as_bytes()).unwrap()).unwrap()
    };
    assert_eq!(
        apply(
            false,
            base,
            "<script src=\"/barfoo.js\"></script><a href='/foobar'>"
        ),
        "<script src=\"http://base/barfoo.js\"></script><a href='http://base/foobar'>"
    );
    assert_eq!(
        apply(false, base, "Link: <a href=/asdf   >ASDF</a>"),
        "Link: <a href=http://base/asdf   >ASDF</a>"
    );
    assert_eq!(
        apply(
            false,
            "../../",
            r#"PRE. a href="/img/small.jpg" input action="/foo.html" meta url=/redirect/to/page/ POST."#
        ),
        r#"PRE. a href="../../img/small.jpg" input action="../../foo.html" meta url=../../redirect/to/page/ POST."#
    );
    assert_eq!(
        apply(
            true,
            base,
            "Pre. <img srcset=&#34;/img/small.jpg 200w, /img/big.jpg 700w&#34; alt=&#34;text&#34; src=&#34;/img/foo.jpg&#34;>"
        ),
        "Pre. <img srcset=&#34;http://base/img/small.jpg 200w, http://base/img/big.jpg 700w&#34; alt=&#34;text&#34; src=&#34;http://base/img/foo.jpg&#34;>"
    );
    // The chain with no transformers copies.
    assert_eq!(Chain::default().apply(b"abc").unwrap(), b"abc");
}
