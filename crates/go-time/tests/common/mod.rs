//! Shared helpers for the Go-oracle fixture tests
//! (fixtures from tools/go-oracle/go-time).
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Once, OnceLock};

use go_time::{GoTimeExt, Location, Time};

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Reads a fixture (`name`, or `name.gz`), from `$GO_TIME_FIXTURES` when set
/// (a larger corpus written with the oracle's `-big` flag), else the checked-in one.
pub fn read_fixture(name: &str) -> Vec<u8> {
    let dirs: Vec<PathBuf> = match std::env::var("GO_TIME_FIXTURES") {
        Ok(d) if !d.is_empty() => vec![PathBuf::from(d), fixtures_dir()],
        _ => vec![fixtures_dir()],
    };
    for dir in dirs {
        let p = dir.join(name);
        if let Ok(b) = std::fs::read(&p) {
            return b;
        }
        let gz = dir.join(format!("{}.gz", name));
        if let Ok(b) = std::fs::read(&gz) {
            let mut d = flate2::read::GzDecoder::new(&b[..]);
            let mut out = Vec::new();
            d.read_to_end(&mut out).expect("gunzip fixture");
            return out;
        }
    }
    panic!("fixture {} not found", name);
}

/// Undoes the oracle's field escaping.
pub fn unesc(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'\\' && i + 1 < s.len() {
            match s[i + 1] {
                b'\\' => {
                    out.push(b'\\');
                    i += 2;
                }
                b't' => {
                    out.push(b'\t');
                    i += 2;
                }
                b'n' => {
                    out.push(b'\n');
                    i += 2;
                }
                b'r' => {
                    out.push(b'\r');
                    i += 2;
                }
                b'x' => {
                    let h = std::str::from_utf8(&s[i + 2..i + 4]).unwrap();
                    out.push(u8::from_str_radix(h, 16).unwrap());
                    i += 4;
                }
                other => panic!("bad escape \\{}", other as char),
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// A fixture record: its fields (unescaped, as bytes).
pub struct Rec {
    pub f: Vec<Vec<u8>>,
    pub line: usize,
}

impl Rec {
    pub fn s(&self, i: usize) -> String {
        String::from_utf8(self.f[i].clone())
            .unwrap_or_else(|_| panic!("line {}: field {} not UTF-8", self.line, i))
    }
    pub fn b(&self, i: usize) -> &[u8] {
        &self.f[i]
    }
    pub fn i(&self, i: usize) -> i64 {
        self.s(i)
            .parse()
            .unwrap_or_else(|_| panic!("line {}: field {} not int", self.line, i))
    }
    pub fn tag(&self) -> &[u8] {
        &self.f[0]
    }
}

pub fn records(name: &str) -> Vec<Rec> {
    let data = read_fixture(name);
    let mut v = Vec::new();
    for (n, line) in data.split(|&b| b == b'\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        let f = line.split(|&b| b == b'\t').map(unesc).collect();
        v.push(Rec { f, line: n + 1 });
    }
    v
}

static SETUP: Once = Once::new();

/// Pins `Local` exactly like the oracle: Asia/Bangkok named "Local".
pub fn setup() {
    SETUP.call_once(|| {
        let data = zone_data("Asia/Bangkok");
        let l = go_time::load_location_from_tz_data("Local", &data).expect("Bangkok");
        go_time::set_local(l);
    });
}

pub fn zone_data(name: &str) -> Vec<u8> {
    std::fs::read(
        fixtures_dir()
            .join("zoneinfo")
            .join(name.replace('/', "__")),
    )
    .expect("zone fixture")
}

fn loc_cache() -> &'static Mutex<HashMap<String, Arc<Location>>> {
    static C: OnceLock<Mutex<HashMap<String, Arc<Location>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolves an oracle location id.
pub fn loc(id: &str) -> Arc<Location> {
    setup();
    if id == "UTC" {
        return go_time::utc();
    }
    if id == "Local" {
        return go_time::local();
    }
    let mut c = loc_cache().lock().unwrap();
    if let Some(l) = c.get(id) {
        return l.clone();
    }
    let l = if let Some(rest) = id.strip_prefix("Fixed:") {
        let (name, off) = rest.rsplit_once(':').unwrap();
        go_time::fixed_zone(name, off.parse().unwrap())
    } else if let Some(z) = id.strip_prefix("Zone:") {
        go_time::load_location_from_tz_data(z, &zone_data(z)).unwrap()
    } else {
        panic!("unknown location id {}", id)
    };
    c.insert(id.to_string(), l.clone());
    l
}

/// `time.Unix(sec, nsec).In(loc)`.
pub fn t_in(sec: i64, nsec: i64, id: &str) -> Time {
    let l = loc(id);
    go_time::unix(sec, nsec).in_loc(&l)
}

pub fn loc_kind(t: &Time) -> &'static str {
    match &t.loc {
        None => "UTC",
        Some(l) if go_time::is_utc_loc(l) => "UTC",
        Some(l) if go_time::is_local_loc(l) => "Local",
        Some(_) => "other",
    }
}

/// The oracle's `timeFields`.
pub fn time_fields(t: &Time) -> Vec<String> {
    let (name, off) = t.zone();
    vec![
        t.go_unix().to_string(),
        t.nanosecond().to_string(),
        loc_kind(t).to_string(),
        go_time::location_string(t.loc.as_ref()),
        name,
        off.to_string(),
        t.string(),
    ]
}

/// Compares a computed list of fields with record fields starting at `start`.
pub fn fields_eq(r: &Rec, start: usize, got: &[String]) -> bool {
    if r.f.len() < start + got.len() {
        return false;
    }
    got.iter()
        .enumerate()
        .all(|(i, g)| r.f[start + i] == g.as_bytes())
}

pub fn show(r: &Rec) -> String {
    r.f.iter()
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Collects mismatches; panics at the end with the first few.
pub struct Checker {
    pub name: &'static str,
    pub total: usize,
    pub bad: Vec<String>,
}

impl Checker {
    pub fn new(name: &'static str) -> Checker {
        Checker {
            name,
            total: 0,
            bad: Vec::new(),
        }
    }
    pub fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        self.total += 1;
        if !ok {
            self.bad.push(msg());
        }
    }
    pub fn finish(self) {
        eprintln!(
            "{}: {} checks, {} mismatches",
            self.name,
            self.total,
            self.bad.len()
        );
        if !self.bad.is_empty() {
            let show: usize = std::env::var("GO_TIME_SHOW")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(25);
            for b in self.bad.iter().take(show) {
                eprintln!("MISMATCH {}", b);
            }
            panic!(
                "{}: {} of {} checks mismatched",
                self.name,
                self.bad.len(),
                self.total
            );
        }
        assert!(self.total > 0, "{}: no checks ran", self.name);
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
