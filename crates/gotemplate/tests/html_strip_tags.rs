//! Hugo's exported `StripTags` (`gotemplate::html::strip_tags`, used by
//! `tpl.StripHTML` for `.Plain`, `plainify`, `countwords`, ...) against the
//! Go oracle's `stripTags` dump (`tests/fixtures/html/leaf.txt.gz`, lines
//! `L striptags <input> <output>`, written by
//! `tools/go-oracle/gotemplate/escfuncs.go`).
//!
//! The other html/template leaf functions are crate-private and are tested
//! by the unit tests in `src/html/tests`.

mod common;

use common::{q, read_gz_fixture, unquote};

#[test]
fn strip_tags_matches_go() {
    let data = read_gz_fixture("html/leaf.txt.gz");
    let mut n = 0;
    let mut failures = Vec::new();
    for line in data.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 4 || fields[0] != "L" || fields[1] != "striptags" {
            continue;
        }
        let input = unquote(fields[2]);
        let want = unquote(fields[3]);
        let got = gotemplate::html::strip_tags(&input);
        n += 1;
        if got != want {
            failures.push(format!("{}: want {} got {}", q(&input), q(&want), q(&got)));
        }
    }
    assert!(n > 3000, "only {n} cases");
    assert!(
        failures.is_empty(),
        "{} of {n} failed:\n{}",
        failures.len(),
        failures[..failures.len().min(20)].join("\n")
    );
}

#[test]
fn strip_tags_examples() {
    // Go: html.go:stripTags doc comment and TestStripTags.
    let s = gotemplate::html::strip_tags;
    assert_eq!(s(b"<b>&iexcl;Hi!</b> <script>...</script>"), b"&iexcl;Hi! ");
    assert_eq!(s(b"I <3 Ponies!"), b"I <3 Ponies!");
    assert_eq!(s(br#"Foo<div title="1>2">Bar"#), b"FooBar");
    assert_eq!(s(b"Foo <!-- Bar --> Baz"), b"Foo  Baz");
}
