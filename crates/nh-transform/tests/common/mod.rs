//! Fixture reading shared by the integration tests (gzipped JSON lines written by the Go
//! oracles in `tools/go-oracle/nh-transform/*`, see `osupport`).

#![allow(dead_code)]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::Value;

/// The path of a fixture file below `tests/fixtures`.
pub fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

/// Reads a `*.jsonl.gz` fixture: one JSON record per line.
pub fn read_jsonl_gz(path: &Path) -> Vec<Value> {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let r = BufReader::new(flate2::read::GzDecoder::new(f));
    r.lines()
        .map(|l| serde_json::from_str(&l.expect("line")).expect("json"))
        .collect()
}

/// Decodes a byte field (`osupport.B`): a JSON string, or `{"b64": ...}`.
pub fn bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Object(m) => base64::engine::general_purpose::STANDARD
            .decode(m["b64"].as_str().expect("b64"))
            .expect("base64"),
        _ => panic!("not a byte field: {v}"),
    }
}

/// `bytes` of an optional field.
pub fn opt_bytes(v: &Value, key: &str) -> Option<Vec<u8>> {
    v.get(key).filter(|x| !x.is_null()).map(bytes)
}

/// A string field decoded as bytes, as a `String` (fields known to be valid UTF-8).
pub fn string(v: &Value) -> String {
    String::from_utf8(bytes(v)).expect("utf-8")
}

/// Shows bytes for a failure message.
pub fn show(b: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(b))
}
