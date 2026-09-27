//! Shared helpers for the text/template and parse differential tests:
//! fixture loading (gzip), Go quoting, and the record format written by
//! `tools/go-oracle/gotemplate/{parse,exec}*.go`.

#![allow(dead_code)]

use std::io::Read;
use std::path::PathBuf;

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("text")
        .join(name)
}

/// Reads a fixture, gunzipping `*.gz` files (bytes: fixtures are Go
/// strings and may hold any bytes, but every field is `strconv.Quote`d).
pub fn read_fixture(name: &str) -> String {
    let raw = std::fs::read(fixture_path(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let bytes = if name.ends_with(".gz") {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_end(&mut out)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        out
    } else {
        raw
    };
    String::from_utf8(bytes).expect("fixtures are ASCII (all strings are quoted)")
}

/// Reads a (possibly gzip-compressed) corpus outside the fixtures
/// directory (the red-team corpora).
pub fn read_gz_path(path: &std::path::Path) -> String {
    let raw = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let bytes = if path.extension().is_some_and(|e| e == "gz") {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_end(&mut out)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        out
    } else {
        raw
    };
    String::from_utf8(bytes).expect("corpora are ASCII (all strings are quoted)")
}

/// Go `strconv.Unquote` of a fixture field.
pub fn unquote(s: &str) -> Vec<u8> {
    go_strconv::unquote(s).unwrap_or_else(|e| panic!("unquote {s:?}: {e}"))
}

/// [`unquote`] into a Rust string (the field must be valid UTF-8).
pub fn unquote_str(s: &str) -> String {
    String::from_utf8(unquote(s)).unwrap_or_else(|_| panic!("not UTF-8: {s}"))
}

/// Go `strconv.Quote`.
pub fn q(b: impl AsRef<[u8]>) -> String {
    go_strconv::quote(b.as_ref())
}

/// Splits a line into space-separated fields, keeping Go-quoted strings
/// (which never contain a raw space... except inside quotes) whole.
pub fn fields(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && b[i] == b' ' {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let start = i;
        if b[i] == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
        } else {
            while i < b.len() && b[i] != b' ' {
                i += 1;
            }
        }
        out.push(&line[start..i]);
    }
    out
}

/// One fixture record: the `#case` header fields and the lines up to
/// `#end`.
pub struct Record<'a> {
    pub header: Vec<&'a str>,
    pub lines: Vec<&'a str>,
}

/// Splits a fixture into `#case ... #end` records.
pub fn records(text: &str) -> Vec<Record<'_>> {
    let mut out = Vec::new();
    let mut cur: Option<Record<'_>> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("#case ") {
            cur = Some(Record {
                header: fields(rest),
                lines: Vec::new(),
            });
        } else if line == "#end" {
            out.push(cur.take().expect("#end without #case"));
        } else if let Some(r) = cur.as_mut() {
            r.lines.push(line);
        }
    }
    out
}

/// Reports the first differing line of two dumps.
pub fn first_diff(want: &[&str], got: &[String]) -> Option<String> {
    for i in 0..want.len().max(got.len()) {
        let w = want.get(i).copied();
        let g = got.get(i).map(|s| s.as_str());
        if w != g {
            return Some(format!(
                "line {i}:\n    go:   {}\n    rust: {}",
                w.unwrap_or("<none>"),
                g.unwrap_or("<none>")
            ));
        }
    }
    None
}

pub mod gotests;
pub mod model;
