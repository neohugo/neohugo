//! Fixture reader and test scaffolding for tests/oracle.rs (mirrors
//! tools/go-oracle/libsass-sys).

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use libsass_sys::{ImportResolver, Options, OutputStyle, SourceMapOptions};

/// Reads the zlib-compressed `@<key> <len>\n<bytes>\n` record stream.
pub fn read_records(path: &Path) -> Vec<(String, Vec<u8>)> {
    let z = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let b = miniz_oxide::inflate::decompress_to_vec_zlib(&z).expect("zlib");
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        assert_eq!(b[i], b'@');
        let nl = i + b[i..].iter().position(|&c| c == b'\n').unwrap();
        let hdr = std::str::from_utf8(&b[i + 1..nl]).unwrap();
        let (key, len) = hdr.split_once(' ').unwrap();
        let len: usize = len.parse().unwrap();
        let start = nl + 1;
        out.push((key.to_string(), b[start..start + len].to_vec()));
        assert_eq!(b[start + len], b'\n');
        i = start + len + 1;
    }
    out
}

/// Extracts site.pack.zz into `<fresh temp dir>/site`; returns that
/// canonical path.
pub fn extract_site(pack: &Path) -> PathBuf {
    let base = std::env::temp_dir().join(format!("libsass-sys-oracle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    // A fixed base name (as in the oracle): source-map paths that climb out
    // of the root (e.g. OutputPath "../o.css") contain it.
    let root = base.canonicalize().unwrap().join("site");
    std::fs::create_dir_all(&root).unwrap();
    let recs = read_records(pack);
    for pair in recs.chunks(2) {
        assert_eq!(pair[0].0, "file");
        assert_eq!(pair[1].0, "data");
        let p = root.join(std::str::from_utf8(&pair[0].1).unwrap());
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, &pair[1].1).unwrap();
    }
    root
}

pub fn replace(s: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return s.to_vec();
    }
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

#[derive(Default, Debug)]
pub struct Expect {
    pub status: String,
    pub css: Vec<u8>,
    pub smfile: Vec<u8>,
    pub smcontent: Vec<u8>,
    pub err: Vec<u8>,
    pub err_status: i64,
    pub err_line: i64,
    pub err_column: i64,
    pub err_file: Vec<u8>,
    pub err_message: Vec<u8>,
    pub trace: Vec<u8>,
}

/// (url, new_url, body, ok)
pub type TableEntry = (Vec<u8>, Vec<u8>, Vec<u8>, bool);

#[derive(Default, Debug)]
pub struct Case {
    pub name: String,
    pub src: Vec<u8>,
    pub style: i64,
    pub precision: i64,
    pub include: Vec<u8>,
    pub sass_syntax: bool,
    pub sm_filename: Vec<u8>,
    pub sm_root: Vec<u8>,
    pub sm_input: Vec<u8>,
    pub sm_output: Vec<u8>,
    pub sm_contents: bool,
    pub sm_omit: bool,
    pub sm_embed: bool,
    pub resolver: String,
    pub hugo_basedir: Vec<u8>,
    pub hugo_vars: Vec<u8>,
    /// (url, new_url, body, ok)
    pub table: Vec<TableEntry>,
    pub expect: Expect,
}

fn s(v: &[u8]) -> String {
    String::from_utf8(v.to_vec()).unwrap()
}
fn int(v: &[u8]) -> i64 {
    s(v).parse().unwrap()
}

pub fn read_cases(path: &Path) -> Vec<Case> {
    let mut cases = Vec::new();
    let mut c = Case::default();
    for (k, v) in read_records(path) {
        match k.as_str() {
            "case" => {
                c = Case {
                    name: s(&v),
                    ..Default::default()
                };
            }
            "src" => c.src = v,
            "style" => c.style = int(&v),
            "precision" => c.precision = int(&v),
            "include" => c.include = v,
            "sass_syntax" => c.sass_syntax = v == b"1",
            "sm_filename" => c.sm_filename = v,
            "sm_root" => c.sm_root = v,
            "sm_input" => c.sm_input = v,
            "sm_output" => c.sm_output = v,
            "sm_contents" => c.sm_contents = v == b"1",
            "sm_omit" => c.sm_omit = v == b"1",
            "sm_embed" => c.sm_embed = v == b"1",
            "resolver" => c.resolver = s(&v),
            "hugo_basedir" => c.hugo_basedir = v,
            "hugo_vars" => c.hugo_vars = v,
            "rt_url" => c.table.push((v, Vec::new(), Vec::new(), false)),
            "rt_new" => c.table.last_mut().unwrap().1 = v,
            "rt_body" => c.table.last_mut().unwrap().2 = v,
            "rt_ok" => c.table.last_mut().unwrap().3 = v == b"1",
            "x_status" => c.expect.status = s(&v),
            "x_css" => c.expect.css = v,
            "x_smfile" => c.expect.smfile = v,
            "x_smcontent" => c.expect.smcontent = v,
            "x_err" => c.expect.err = v,
            "x_err_status" => c.expect.err_status = int(&v),
            "x_err_line" => c.expect.err_line = int(&v),
            "x_err_column" => c.expect.err_column = int(&v),
            "x_err_file" => c.expect.err_file = v,
            "x_err_message" => c.expect.err_message = v,
            "x_trace" => c.expect.trace = v,
            "end" => cases.push(std::mem::take(&mut c)),
            k => panic!("unknown record {k}"),
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// Go path/filepath (unix) helpers used by the Hugo resolver emulation.

/// Go `path.Clean` (== `filepath.Clean` on unix).
pub fn clean(p: &[u8]) -> Vec<u8> {
    if p.is_empty() {
        return b".".to_vec();
    }
    let rooted = p[0] == b'/';
    let n = p.len();
    let mut out: Vec<u8> = Vec::with_capacity(n);
    let (mut r, mut dotdot) = (0, 0);
    if rooted {
        out.push(b'/');
        r = 1;
        dotdot = 1;
    }
    while r < n {
        if p[r] == b'/' || (p[r] == b'.' && (r + 1 == n || p[r + 1] == b'/')) {
            r += 1;
        } else if p[r] == b'.' && p[r + 1] == b'.' && (r + 2 == n || p[r + 2] == b'/') {
            r += 2;
            if out.len() > dotdot {
                let mut w = out.len() - 1;
                while w > dotdot && out[w] != b'/' {
                    w -= 1;
                }
                out.truncate(w);
            } else if !rooted {
                if !out.is_empty() {
                    out.push(b'/');
                }
                out.extend_from_slice(b"..");
                dotdot = out.len();
            }
        } else {
            if (rooted && out.len() != 1) || (!rooted && !out.is_empty()) {
                out.push(b'/');
            }
            while r < n && p[r] != b'/' {
                out.push(p[r]);
                r += 1;
            }
        }
    }
    if out.is_empty() {
        return b".".to_vec();
    }
    out
}

/// Go `filepath.Join`.
pub fn join(elems: &[&[u8]]) -> Vec<u8> {
    let parts: Vec<&[u8]> = elems.iter().copied().filter(|e| !e.is_empty()).collect();
    if parts.is_empty() {
        return Vec::new();
    }
    clean(&parts.join(&b'/'))
}

/// Go `filepath.Dir`.
pub fn dir(p: &[u8]) -> Vec<u8> {
    let i = p
        .iter()
        .rposition(|&c| c == b'/')
        .map(|i| i + 1)
        .unwrap_or(0);
    clean(&p[..i])
}

/// Go `filepath.Base`.
pub fn base(p: &[u8]) -> Vec<u8> {
    if p.is_empty() {
        return b".".to_vec();
    }
    let mut p = p;
    while p.len() > 1 && p[p.len() - 1] == b'/' {
        p = &p[..p.len() - 1];
    }
    let p = match p.iter().rposition(|&c| c == b'/') {
        Some(i) if p.len() > 1 => &p[i + 1..],
        _ => p,
    };
    if p.is_empty() {
        b"/".to_vec()
    } else {
        p.to_vec()
    }
}

fn exists(p: &[u8]) -> bool {
    std::fs::metadata(std::str::from_utf8(p).unwrap()).is_ok()
}

// ---------------------------------------------------------------------------
// Hugo resolver emulation (tools/go-oracle/libsass-sys/hugo.go).

#[derive(Clone)]
pub struct HugoFs {
    pub root: Vec<u8>,
}

impl HugoFs {
    fn assets(&self) -> Vec<u8> {
        join(&[&self.root, b"assets"])
    }
    fn vendor(&self) -> Vec<u8> {
        join(&[&self.root, b"node_modules"])
    }

    pub fn stat(&self, rel: &[u8]) -> Vec<u8> {
        let rel = clean(rel);
        if rel == b"vendor" || rel.starts_with(b"vendor/") {
            let mut r = self.vendor();
            r.extend_from_slice(&rel[b"vendor".len()..]);
            if exists(&r) {
                return r;
            }
        }
        let r = join(&[&self.assets(), &rel]);
        if exists(&r) {
            return r;
        }
        Vec::new()
    }

    pub fn make_path_relative(&self, filename: &[u8]) -> Vec<u8> {
        let f = clean(filename);
        for (real, target) in [(self.assets(), &b""[..]), (self.vendor(), &b"vendor"[..])] {
            let mut real_slash = real.clone();
            real_slash.push(b'/');
            if f == real || f.starts_with(&real_slash) {
                let joined = join(&[b"/", target, &f[real.len()..]]);
                let p = joined.strip_prefix(b"/").unwrap_or(&joined).to_vec();
                if p.is_empty() {
                    return Vec::new();
                }
                if !self.stat(&p).is_empty() {
                    return p;
                }
                return Vec::new();
            }
        }
        Vec::new()
    }
}

pub type Trace = Arc<Mutex<Vec<Vec<u8>>>>;

fn record(trace: &Trace, url: &[u8], prev: &[u8]) {
    let mut t = url.to_vec();
    t.push(b'\t');
    t.extend_from_slice(prev);
    trace.lock().unwrap().push(t);
}

pub fn hugo_resolver(h: HugoFs, base_dir: Vec<u8>, vars: Vec<u8>, trace: Trace) -> ImportResolver {
    Arc::new(move |url: &[u8], prev: &[u8]| {
        record(&trace, url, prev);
        if url == b"hugo:vars" {
            return (url.to_vec(), vars.clone(), true);
        }
        let url_dir = dir(url);
        let prev_dir = if prev == b"stdin" {
            base_dir.clone()
        } else {
            let pd = h.make_path_relative(&dir(prev));
            if pd.is_empty() {
                return (Vec::new(), Vec::new(), false);
            }
            pd
        };
        let base_path = join(&[&prev_dir, &url_dir]);
        let mut name = base(url);
        let patterns: &[&str] = if name.contains(&b'.') {
            &["_%s", "%s"]
        } else if name.starts_with(b"_") {
            &["_%s.scss", "_%s.sass"]
        } else {
            &[
                "_%s.scss",
                "%s.scss",
                "_%s.sass",
                "%s.sass",
                "%s/_index.scss",
                "%s/_index.sass",
                "%s/index.scss",
                "%s/index.sass",
            ]
        };
        if name.starts_with(b"_") {
            name.remove(0);
        }
        for pat in patterns {
            let f = replace(pat.as_bytes(), b"%s", &name);
            let real = h.stat(&join(&[&base_path, &f]));
            if !real.is_empty() {
                return (real, Vec::new(), true);
            }
        }
        (Vec::new(), Vec::new(), false)
    })
}

/// Builds Options (with `@SITE@` substituted) and the resolver for a case.
pub fn options_for(c: &Case, root: &[u8], trace: &Trace) -> Options {
    let sub = |v: &[u8]| replace(v, b"@SITE@", root);
    let include_paths = if c.include.is_empty() {
        Vec::new()
    } else {
        c.include.split(|&b| b == b'\n').map(sub).collect()
    };
    let import_resolver: Option<ImportResolver> = match c.resolver.as_str() {
        "none" => None,
        "hugo" => Some(hugo_resolver(
            HugoFs {
                root: root.to_vec(),
            },
            c.hugo_basedir.clone(),
            c.hugo_vars.clone(),
            trace.clone(),
        )),
        "echo" => {
            let trace = trace.clone();
            Some(Arc::new(move |url: &[u8], prev: &[u8]| {
                record(&trace, url, prev);
                (url.to_vec(), b"$white:    #fff".to_vec(), true)
            }))
        }
        "table" => {
            let trace = trace.clone();
            let table: Vec<_> = c
                .table
                .iter()
                .map(|(u, n, b, ok)| (u.clone(), sub(n), b.clone(), *ok))
                .collect();
            Some(Arc::new(move |url: &[u8], prev: &[u8]| {
                record(&trace, url, prev);
                for (u, n, b, ok) in &table {
                    if u == url {
                        return (n.clone(), b.clone(), *ok);
                    }
                }
                (Vec::new(), Vec::new(), false)
            }))
        }
        "nested" => {
            // Re-entrant resolver (tools/go-oracle/libsass-sys main.go): runs
            // a nested transpile of the table body, whose own imports are
            // answered with `$inner: 7px;`.
            let trace = trace.clone();
            let table: Vec<_> = c
                .table
                .iter()
                .map(|(u, n, b, ok)| (u.clone(), sub(n), b.clone(), *ok))
                .collect();
            Some(Arc::new(move |url: &[u8], prev: &[u8]| {
                record(&trace, url, prev);
                for (u, n, b, ok) in &table {
                    if u == url {
                        let inner_trace = trace.clone();
                        let inner = libsass_sys::new(Options {
                            output_style: libsass_sys::COMPRESSED_STYLE,
                            import_resolver: Some(Arc::new(move |u: &[u8], p: &[u8]| {
                                let mut u2 = b"inner:".to_vec();
                                u2.extend_from_slice(u);
                                record(&inner_trace, &u2, p);
                                (u.to_vec(), b"$inner: 7px;".to_vec(), true)
                            })),
                            ..Default::default()
                        })
                        .unwrap();
                        return match libsass_sys::Transpiler::execute(&inner, b) {
                            Ok(r) => (n.clone(), r.css, *ok),
                            Err(e) => (n.clone(), format!("/* {e} */").into_bytes(), true),
                        };
                    }
                }
                (Vec::new(), Vec::new(), false)
            }))
        }
        r => panic!("unknown resolver {r}"),
    };
    Options {
        output_style: OutputStyle(c.style),
        precision: c.precision,
        include_paths,
        import_resolver,
        sass_syntax: c.sass_syntax,
        source_map_options: SourceMapOptions {
            filename: sub(&c.sm_filename),
            root: sub(&c.sm_root),
            input_path: sub(&c.sm_input),
            output_path: sub(&c.sm_output),
            contents: c.sm_contents,
            omit_url: c.sm_omit,
            enable_embedded: c.sm_embed,
        },
    }
}
