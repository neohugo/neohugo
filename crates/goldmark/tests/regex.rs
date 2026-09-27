//! The hand-written matchers of parser/regexps.rs vs Go's regexp running
//! goldmark's patterns (tools/go-oracle/goldmark regex.go).

mod common;

use common::fixture;
use goldmark::parser::regexps;
use goldmark::text::{self, Reader};

fn s(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn html_block_and_tag_regexps() {
    let recs = fixture("regex.gmf.gz");
    let mut fails = Vec::new();
    let mut n = 0;
    for r in recs.iter().filter(|r| r.name.starts_with("regex/")) {
        let v = r.get("in");
        let b = |x: bool| x.to_string();
        let mut got: Vec<(&str, String)> = vec![
            ("type1open", b(regexps::html_block_type1_open(v))),
            ("type1close", b(regexps::html_block_type1_close(v))),
            ("type2open", b(regexps::html_block_type2_open(v))),
            ("type3open", b(regexps::html_block_type3_open(v))),
            ("type4open", b(regexps::html_block_type4_open(v))),
            ("type5open", b(regexps::html_block_type5_open(v))),
            (
                "type6",
                match regexps::html_block_type6(v) {
                    Some((a, e)) => s(&v[a..e]),
                    None => "nil".into(),
                },
            ),
            (
                "type7",
                match regexps::html_block_type7(v) {
                    Some(m) => format!(
                        "{} {} {}",
                        m.is_close_tag,
                        m.has_attr,
                        s(&v[m.tag.0..m.tag.1])
                    ),
                    None => "nil".into(),
                },
            ),
        ];
        for (name, f) in [
            (
                "opentag",
                regexps::open_tag as fn(&mut text::RuneStream<'_>) -> Option<i64>,
            ),
            ("closetag", regexps::close_tag),
        ] {
            let mut rd = text::new_reader(v);
            let mut m = f;
            let ok = rd.match_with(&mut m);
            let (l, pos) = rd.position();
            got.push((
                name,
                format!("{} {} {} {} {}", ok, l, pos.start, pos.stop, pos.padding),
            ));

            let mut segs = text::new_segments();
            let mut start = 0usize;
            let mut li = 0;
            while start < v.len() {
                let stop = match v[start..].iter().position(|&c| c == b'\n') {
                    Some(e) => start + e + 1,
                    None => v.len(),
                };
                let pad = if li == 1 { 1 } else { 0 };
                segs.append(text::new_segment_padding(start as i64, stop as i64, pad));
                start = stop;
                li += 1;
            }
            let mut br = text::new_block_reader(v, Some(&segs));
            let ok = br.match_with(&mut m);
            let (l, pos) = br.position();
            got.push((
                if name == "opentag" {
                    "opentag/block"
                } else {
                    "closetag/block"
                },
                format!("{} {} {} {} {}", ok, l, pos.start, pos.stop, pos.padding),
            ));
        }
        for (k, g) in got {
            n += 1;
            if r.get(k) != g.as_bytes() {
                fails.push(format!(
                    "{} {k} in={:?} want={:?} got={:?}",
                    r.name,
                    s(v),
                    s(r.get(k)),
                    g
                ));
            }
        }
    }
    for r in recs.iter().filter(|r| r.name.starts_with("email/")) {
        let v = r.get("in");
        let got = match goldmark::util::email_domain_regexp_match_end(v) {
            Some(e) => format!("[0 {e}]"),
            None => "[]".into(),
        };
        n += 1;
        if r.get("domain") != got.as_bytes() {
            fails.push(format!(
                "{} domain in={:?} want={:?} got={:?}",
                r.name,
                s(v),
                s(r.get("domain")),
                got
            ));
        }
        let mut ab = b"ab@".to_vec();
        ab.extend_from_slice(v);
        let got = goldmark::util::find_email_index(&ab).to_string();
        if r.get("FindEmailIndex") != got.as_bytes() {
            fails.push(format!("{} FindEmailIndex in={:?}", r.name, s(v)));
        }
    }
    assert!(
        fails.is_empty(),
        "{} failures:\n{}",
        fails.len(),
        fails[..fails.len().min(20)].join("\n")
    );
    assert!(n > 50_000, "{n}");
}
