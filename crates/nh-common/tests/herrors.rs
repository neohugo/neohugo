//! `herrors` error texts: the differential test against
//! `tools/go-oracle/nh-common/herrors` (Go's `Error()` of positioned and plain errors under every
//! sequence of up to three `%w`/`%v`/`errors.Join`/`NewFileError*` steps).

mod support;

use nh_common::errorf;
use nh_common::herrors::{self, Error, FilePos};
use serde_json::{Value as J, json};
use support::fixture;

/// The oracle's `bases`, by name.
fn base(name: &str) -> Error {
    match name {
        "plain" => Error::new("boom"),
        "plainLine" => Error::new("yaml: line 4: did not find expected key"),
        "notExist" => Error::not_exist("file does not exist").wrap("open data/x.json"),
        "filePos" => herrors::new_file_error_from_pos(
            Error::new("boom"),
            FilePos {
                filename: "/site/assets/js/main.js".into(),
                line: 3,
                column: 5,
            },
        ),
        "filePosNoName" => herrors::new_file_error_from_pos(
            Error::new("boom"),
            FilePos {
                filename: String::new(),
                line: 2,
                column: 1,
            },
        ),
        "fileName" => herrors::new_file_error_from_name(
            Error::new("template: x.html:12:3: unexpected EOF"),
            "/site/layouts/x.html",
        ),
        "fileNoPos" => herrors::new_file_error(Error::new("(7, 9): bad bundle")),
        "fileNotExist" => herrors::new_file_error_from_name(
            Error::not_exist("file does not exist").wrap("open a.md"),
            "/site/content/a.md",
        ),
        _ => panic!("unknown base {name}"),
    }
}

const PREFIXES: [&str; 3] = [
    "readAndProcessContent",
    r#"JSBUILD: failed to transform "js/main.js" (text/javascript)"#,
    "template: t.html:9:4: executing",
];

/// The oracle's `apply`: step `step` at depth `i`.
fn apply(step: &str, i: usize, e: Error) -> Error {
    let p = PREFIXES[i];
    match step {
        "w" => e.wrap(p),
        // `%v` keeps only the text.
        "v" => errorf!("{p}: {e}"),
        "join" => herrors::join([Some(e), Some(Error::new("other"))]).unwrap(),
        "joinFile" => herrors::join([
            None,
            Some(herrors::new_file_error_from_name(
                Error::new("other"),
                "o.md",
            )),
            Some(e),
        ])
        .unwrap(),
        "name" => herrors::new_file_error_from_name(e, &format!("/site/outer{i}.html")),
        "pos" => herrors::new_file_error_from_pos(
            e,
            FilePos {
                filename: "p.md".into(),
                line: 10 + i as i64,
                column: 2,
            },
        ),
        "new" => herrors::new_file_error(e),
        _ => panic!("unknown step {step}"),
    }
}

#[test]
fn herrors_texts_match_go() {
    let f = fixture("herrors/herrors.json.gz");
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(
        cases.len() as u64,
        f["bases"].as_u64().unwrap() * f["sequences"].as_u64().unwrap()
    );
    let mut bad = Vec::new();
    for c in cases {
        let steps: Vec<&str> = c["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        let mut e = base(c["base"].as_str().unwrap());
        for (i, s) in steps.iter().enumerate() {
            e = apply(s, i, e);
        }
        let pos = match e.pos() {
            Some(p) => json!({"filename": p.filename, "line": p.line, "column": p.column}),
            None => J::Null,
        };
        let got = json!({
            "base": c["base"],
            "steps": c["steps"],
            "error": e.to_string(),
            "isNotExist": e.is_not_exist(),
            "pos": pos,
        });
        if &got != c {
            bad.push(format!("want {c}\n got {got}"));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} cases differ:\n{}",
        bad.len(),
        cases.len(),
        bad[..bad.len().min(20)].join("\n")
    );
}

/// The two orders Go prints: `NewFileErrorFromPos` puts the position first, a `%w` wrapper
/// around it puts its prefix first.
#[test]
fn wrapped_file_error_order() {
    let fe = herrors::new_file_error_from_pos(
        Error::new("msg"),
        FilePos {
            filename: "file".into(),
            line: 1,
            column: 2,
        },
    );
    assert_eq!(fe.to_string(), r#""file:1:2": msg"#);
    let w = fe.wrap("prefix");
    assert_eq!(w.to_string(), r#"prefix: "file:1:2": msg"#);
    assert_eq!(w.message(), "msg");
    // UpdatePosition of the inner file error keeps the prefix outside.
    let mut pos = w.pos().unwrap().clone();
    pos.line = 7;
    assert_eq!(w.at(pos).to_string(), r#"prefix: "file:7:2": msg"#);
}
