//! Shared helpers of the nh-commands oracle tests: fixture loading, temporary directories,
//! site trees and the goval typed value encoding (tools/go-oracle/nh-common/goval).

#![allow(dead_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use go_value::{Map, Value};
use serde_json::{Value as J, json};

/// Reads `tests/fixtures/<rel>` (gunzipped when it ends in `.gz`).
pub fn fixture(rel: &str) -> J {
    let path = format!("{}/tests/fixtures/{rel}", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
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

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temporary directory removed on drop (its path has no symlinks).
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let base = std::env::temp_dir().canonicalize().unwrap();
        let path = base.join(format!(
            "nh-commands-{}-{}-{}",
            prefix.replace('/', "_"),
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }

    pub fn str(&self) -> String {
        self.path.to_str().unwrap().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Writes `files` (slash path -> content; a path ending in "/" is an empty directory) below
/// `dir`, replacing "$ROOT" in the contents with `root`.
pub fn materialize(dir: &Path, files: &serde_json::Map<String, J>, root: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let mut names: Vec<&String> = files.keys().collect();
    names.sort();
    for n in names {
        let p = dir.join(n);
        if n.ends_with('/') {
            std::fs::create_dir_all(&p).unwrap();
            continue;
        }
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let content = files[n].as_str().unwrap().replace("$ROOT", root);
        std::fs::write(&p, content).unwrap();
    }
}

/// A string as goval encodes it.
pub fn str_enc(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => json!({"hex": b.iter().map(|c| format!("{c:02x}")).collect::<String>()}),
    }
}

/// goval.Encode for the values a flags provider holds (bool, string, []string, maps.Params).
pub fn encode(v: &Value) -> J {
    match v {
        Value::Invalid => json!({"t": "nil"}),
        Value::Bool(b) => json!({"t": "bool", "v": b}),
        Value::String(s) => json!({"t": "string", "s": str_enc(s.as_bytes())}),
        Value::List(l) => {
            let t = match l.ty {
                go_value::SliceType::String => "[]string",
                _ => "[]interface {}",
            };
            json!({"t": t, "items": l.items.iter().map(encode).collect::<Vec<_>>()})
        }
        Value::Map(m) => encode_map(m),
        _ => json!({"t": "unsupported"}),
    }
}

fn encode_map(m: &Map) -> J {
    let entries: Vec<J> = m
        .entries
        .iter()
        .map(|(k, v)| json!([str_enc(k.as_bytes()), encode(v)]))
        .collect();
    json!({"t": m.ty.go_name(), "entries": entries})
}

/// The oracles' output normalization: the temporary root becomes "$ROOT"; the version line
/// loses its VCS revision and gets "$OS/$ARCH" and "$DATE"; `GOOS`/`GOARCH` lines of `env`
/// become `GOOS="$GOOS"` / `GOARCH="$GOARCH"`; "Total in N ms".
pub fn norm_output(s: &str, root: &str) -> String {
    let s = s.replace(root, "$ROOT");
    let mut out = String::new();
    for line in s.split_inclusive('\n') {
        let (body, nl) = match line.strip_suffix('\n') {
            Some(b) => (b, "\n"),
            None => (line, ""),
        };
        out.push_str(&mask_line(body));
        out.push_str(nl);
    }
    out
}

fn mask_line(body: &str) -> String {
    for w in ["Total", "Built"] {
        if let Some(rest) = body.strip_prefix(&format!("{w} in "))
            && let Some(n) = rest.strip_suffix(" ms")
            && !n.is_empty()
            && n.bytes().all(|b| b.is_ascii_digit())
        {
            return format!("{w} in N ms");
        }
    }
    for k in ["GOOS", "GOARCH"] {
        if body.starts_with(&format!("{k}=\"")) && body.ends_with('"') {
            return format!("{k}=\"${k}\"");
        }
    }
    if body.starts_with("neohugo v")
        && let Some(i) = body.find(" BuildDate=")
        && let Some((version, _os_arch)) = body[..i].rsplit_once(' ')
    {
        let end = body[i + 1..]
            .find(' ')
            .map(|j| i + 1 + j)
            .unwrap_or(body.len());
        return format!("{version} $OS/$ARCH BuildDate=$DATE{}", &body[end..]);
    }
    body.to_string()
}
