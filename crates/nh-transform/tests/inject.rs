//! The livereload and generator-tag transformers (server only / never active in neohugo builds,
//! ported anyway) against the Go oracle (`tools/go-oracle/nh-transform/absurl`, inject.go).

mod common;

use nh_transform::chain::FromTo;
use nh_transform::livereloadinject::new_transformer;
use nh_transform::metainject::hugo_generator_transform;

#[test]
fn inject_matches_go() {
    let recs = common::read_jsonl_gz(&common::fixture("absurl/inject.jsonl.gz"));
    assert_eq!(recs.len(), 200);
    for r in &recs {
        let input = common::bytes(&r["in"]);
        let want = common::bytes(&r["out"]);
        let mut to = Vec::new();
        let mut ft = FromTo {
            from: &input,
            to: &mut to,
        };
        match r["t"].as_str().unwrap() {
            "livereload" => {
                let u = go_url::parse(r["url"].as_str().unwrap().as_bytes()).unwrap();
                new_transformer(&u)(&mut ft).unwrap();
            }
            _ => hugo_generator_transform(&mut ft).unwrap(),
        }
        assert_eq!(
            common::show(&to),
            common::show(&want),
            "{} {}",
            r["t"],
            common::show(&input)
        );
        assert_eq!(to, want);
    }
}
