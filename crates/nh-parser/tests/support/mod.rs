//! Shared helpers for the T03 oracle tests: fixture reading and the typed JSON format of
//! `tools/go-oracle/nh-parser/tval` (Rust values are encoded and compared with Go's JSON).

#![allow(dead_code)]

use std::io::Read;

use go_time::GoTimeExt;
use go_value::{FloatKind, Time, Value};
use nh_parser::metadecoders::toml::TomlLocal;
use serde_json::{Value as J, json};

/// Reads `tests/fixtures/<rel>` (gunzipped when it ends in `.gz`).
pub fn fixture(rel: &str) -> J {
    fixture_path(&format!(
        "{}/tests/fixtures/{rel}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

/// Reads the fixture named by the environment variable `var` if it is set (a larger corpus
/// generated outside the repository with the same oracle), else `tests/fixtures/<rel>`.
pub fn fixture_or_env(rel: &str, var: &str) -> J {
    match std::env::var(var) {
        Ok(p) if !p.is_empty() => {
            eprintln!("{var}: reading {p}");
            fixture_path(&p)
        }
        _ => fixture(rel),
    }
}

fn fixture_path(path: &str) -> J {
    let rel = path;
    let raw = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let text = if rel.ends_with(".gz") {
        let mut s = String::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_string(&mut s)
            .unwrap();
        s
    } else {
        String::from_utf8(raw).unwrap()
    };
    serde_json::from_str(&text).unwrap()
}

/// Go's `goval.Str`: the string itself when valid UTF-8, else `{"hex": ...}`.
pub fn str_j(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => json!({ "hex": hex(b) }),
    }
}

/// Decodes a `goval.Str` value.
pub fn j_bytes(j: &J) -> Vec<u8> {
    match j {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => unhex(o["hex"].as_str().unwrap()),
        other => panic!("not a string: {other}"),
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn ict() -> std::sync::Arc<go_value::Location> {
    go_time::fixed_zone("ICT", 7 * 3600)
}

pub fn enc_time(t: &Time) -> J {
    let (abbr, off) = t.zone();
    json!({
        "t": "time.Time",
        "unix": t.unix_sec,
        "nsec": t.nsec,
        "loc": go_time::location_string(t.loc.as_ref()),
        "abbr": abbr,
        "off": off,
    })
}

/// Encodes a Rust value like `tval.Encode`.
pub fn enc(v: &Value) -> J {
    match v {
        Value::Invalid => json!({"t": "nil"}),
        Value::TypedNil(t) => json!({ "t": format!("nil:{t}") }),
        Value::Bool(b) => json!({"t": "bool", "v": b}),
        Value::Int(i, k) => json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Uint(u, k) => json!({"t": k.go_name(), "v": u.to_string()}),
        Value::Float(f, FloatKind::F64) => {
            json!({"t": "float64", "v": format!("{:016x}", f.to_bits())})
        }
        Value::Float(f, FloatKind::F32) => {
            json!({"t": "float32", "v": format!("{:016x}", f.to_bits())})
        }
        Value::String(s) => json!({"t": "string", "s": str_j(s.as_bytes())}),
        Value::Time(t) => enc_time(t),
        Value::List(l) => json!({
            "t": l.ty.go_name(),
            "items": l.items.iter().map(enc).collect::<Vec<_>>(),
        }),
        Value::Map(m) => json!({
            "t": v.go_type_name(),
            "entries": m
                .entries
                .iter()
                .map(|(k, v)| json!([str_j(k.as_bytes()), enc(v)]))
                .collect::<Vec<_>>(),
        }),
        Value::Object(o) => {
            if let Some(l) = o.as_any().downcast_ref::<TomlLocal>() {
                return enc_local(l);
            }
            panic!("cannot encode object {}", o.type_name())
        }
        other => panic!("cannot encode {other:?}"),
    }
}

fn enc_local(l: &TomlLocal) -> J {
    match l {
        TomlLocal::Date(d) => json!({
            "t": "toml.LocalDate",
            "s": d.string(),
            "f": [d.year, d.month, d.day],
            "utc": enc_time(&d.as_time(&go_time::utc())),
            "ict": enc_time(&d.as_time(&ict())),
        }),
        TomlLocal::Time(t) => json!({
            "t": "toml.LocalTime",
            "s": t.string(),
            "f": [t.hour, t.minute, t.second, t.nanosecond, t.precision],
        }),
        TomlLocal::DateTime(dt) => json!({
            "t": "toml.LocalDateTime",
            "s": dt.string(),
            "f": [dt.date.year, dt.date.month, dt.date.day, dt.time.hour, dt.time.minute,
                  dt.time.second, dt.time.nanosecond, dt.time.precision],
            "utc": enc_time(&dt.as_time(&go_time::utc())),
            "ict": enc_time(&dt.as_time(&ict())),
        }),
    }
}

/// Collects mismatches and fails with the first few.
#[derive(Default)]
pub struct Diffs {
    pub count: usize,
    pub checks: usize,
    pub first: Vec<String>,
}

impl Diffs {
    pub fn check(&mut self, ok: bool, what: impl FnOnce() -> String) {
        self.checks += 1;
        if !ok {
            self.count += 1;
            if self.first.len() < 20 {
                self.first.push(what());
            }
        }
    }

    pub fn eq<T: PartialEq + std::fmt::Debug>(
        &mut self,
        got: T,
        want: T,
        ctx: impl FnOnce() -> String,
    ) {
        self.checks += 1;
        if got != want {
            self.count += 1;
            if self.first.len() < 20 {
                self.first
                    .push(format!("{}:\n  got:  {:?}\n  want: {:?}", ctx(), got, want));
            }
        }
    }

    pub fn finish(&self, name: &str) {
        eprintln!("{name}: {} checks, {} mismatches", self.checks, self.count);
        assert!(
            self.count == 0,
            "{name}: {} mismatches of {} checks:\n{}",
            self.count,
            self.checks,
            self.first.join("\n")
        );
    }
}
