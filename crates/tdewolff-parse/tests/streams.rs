//! Differential token-stream tests: every record holds an input and the
//! serialized token/grammar stream produced by the Go code (see
//! tools/go-oracle/tdewolff-parse). Inputs are the string literals of the
//! upstream _test.go tables plus deterministic fuzz splices.

mod common;
use common::*;

fn run(file: &str) -> usize {
    let recs = records(file);
    check_all(&recs, |r| {
        let kind = r[0].as_str();
        let input = unhex_opt(&r[1]).unwrap_or_default();
        let want = String::from_utf8(unhex(&r[2])).unwrap();
        let got = stream_for(kind, &input);
        if got == want {
            Ok(())
        } else {
            Err(format!(
                "{} {:?}: {}",
                kind,
                String::from_utf8_lossy(&input),
                first_diff(&want, &got)
            ))
        }
    })
}

#[test]
fn html_streams() {
    let n = run("html.txt");
    eprintln!("html: {} records", n);
}

#[test]
fn css_streams() {
    let n = run("css.txt");
    eprintln!("css: {} records", n);
}

#[test]
fn xml_streams() {
    let n = run("xml.txt");
    eprintln!("xml: {} records", n);
}

#[test]
fn json_streams() {
    let n = run("json.txt");
    eprintln!("json: {} records", n);
}
