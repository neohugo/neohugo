//! Shared helpers for the differential tests: configuration names (as in the
//! Go oracle, tools/go-oracle/tdewolff-minify-js), fixture loading and
//! minification runs that mirror the oracle's `runMin`/`runSub`.
#![allow(dead_code)]

use std::io::Read;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use tdewolff_minify_js::{GoBytes, M, Minifier, Params};

/// Go `config(name)`: '-'-separated tokens vN, pN, keep, alpha, inline; "0"
/// is the zero Minifier.
pub fn config(name: &str) -> (Minifier, Option<Params>) {
    let mut o = Minifier::default();
    let mut params = None;
    if name == "0" {
        return (o, None);
    }
    for tok in name.split('-') {
        if tok == "keep" {
            o.keep_var_names = true;
        } else if tok == "alpha" {
            o.use_alphabet_var_names = true;
        } else if tok == "inline" {
            let mut p = Params::new();
            p.insert(b"inline".to_vec(), b"1".to_vec());
            params = Some(p);
        } else if let Some(n) = tok.strip_prefix('v') {
            o.version = n.parse().unwrap();
        } else if let Some(n) = tok.strip_prefix('p') {
            o.precision = n.parse().unwrap();
        } else {
            panic!("bad config {}", name);
        }
    }
    (o, params)
}

/// Go `cp(s)`: `append([]byte(nil), s...)` (Go size-class capacity).
pub fn cp(s: &[u8]) -> GoBytes {
    GoBytes::nil().append(s)
}

/// Go `minifyBuf`: minifies `buf` in place through `buffer.NewReader(buf)`.
/// A panic is reported as the error "PANIC".
pub fn minify_buf(cfg: &str, buf: GoBytes) -> (Vec<u8>, Vec<u8>) {
    let (o, params) = config(cfg);
    let mut out = Vec::new();
    let res = catch_unwind(AssertUnwindSafe(|| {
        let mut r = tdewolff_parse::buffer::Reader::new(buf);
        o.minify(&M::new(), &mut out, &mut r, params.as_ref())
    }));
    match res {
        Ok(Ok(())) => (out, Vec::new()),
        Ok(Err(e)) => (out, e.error_bytes()),
        Err(_) => (out, b"PANIC".to_vec()),
    }
}

fn after_field(buf: &GoBytes, pristine: &[u8]) -> Vec<u8> {
    let whole = buf.slice(0, buf.cap()).to_vec();
    if whole == pristine {
        Vec::new()
    } else {
        let mut a = b"A".to_vec();
        a.extend_from_slice(&whole);
        a
    }
}

/// Go `runMin`: `[output, err, after]` for a private copy of `input`.
pub fn run_min(cfg: &str, input: &[u8]) -> [Vec<u8>; 3] {
    let buf = cp(input);
    let pristine = buf.slice(0, buf.cap()).to_vec();
    let (out, err) = minify_buf(cfg, buf.clone());
    let after = after_field(&buf, &pristine);
    [out, err, after]
}

/// Go `runSub`: minifies `src` embedded between `prefix` and `suffix` of one
/// buffer; `after` covers the whole buffer.
pub fn run_sub(cfg: &str, prefix: &[u8], src: &[u8], suffix: &[u8]) -> [Vec<u8>; 3] {
    let mut whole = prefix.to_vec();
    whole.extend_from_slice(src);
    whole.extend_from_slice(suffix);
    let buf = cp(&whole);
    let pristine = buf.slice(0, buf.cap()).to_vec();
    let (out, err) = minify_buf(cfg, buf.slice(prefix.len(), prefix.len() + src.len()));
    let after = after_field(&buf, &pristine);
    [out, err, after]
}

/// Regexps registered by neohugo's minifiers.New (minifiers/minifiers.go).
pub const JS_PATTERN: &str = "^(application|text)/(x-)?(java|ecma)script$";
pub const JSON_PATTERN: &str = r"^(application|text)/(x-|(ld|manifest)\+)?json$";

/// The oracle's `hugoM` (neohugo's minifiers.New with the seeksnack config,
/// JS included) and `upstreamHTMLM` (html_test.go:TestHTMLCSSJS).
pub fn html_m(cfg: &str) -> M {
    use std::sync::Arc;
    use tdewolff_minify::{Regexp, css, html, json, svg, xml};
    let mut m = M::new();
    match cfg {
        "hugo" => {
            m.add(
                "text/css",
                Arc::new(css::Minifier {
                    precision: 0,
                    keep_css2: true,
                    ..Default::default()
                }),
            );
            let js: Arc<dyn tdewolff_minify::Minifier> = Arc::new(Minifier {
                version: 2022,
                ..Default::default()
            });
            m.add("text/javascript", js.clone());
            m.add_regexp(Regexp::must_compile(JS_PATTERN), js);
            let j: Arc<dyn tdewolff_minify::Minifier> = Arc::new(json::Minifier::default());
            m.add("application/json", j.clone());
            m.add_regexp(Regexp::must_compile(JSON_PATTERN), j);
            m.add(
                "image/svg+xml",
                Arc::new(svg::Minifier {
                    keep_comments: false,
                    precision: 0,
                    ..Default::default()
                }),
            );
            let x: Arc<dyn tdewolff_minify::Minifier> = Arc::new(xml::Minifier {
                keep_whitespace: false,
            });
            m.add("application/rss+xml", x.clone());
            m.add("application/xml", x);
            m.add(
                "text/html",
                Arc::new(html::Minifier {
                    keep_document_tags: true,
                    keep_special_comments: true,
                    keep_end_tags: true,
                    keep_default_attr_vals: true,
                    keep_whitespace: false,
                    ..Default::default()
                }),
            );
        }
        "upstream" => {
            m.add_func("text/html", html::minify);
            m.add_func("text/css", css::minify);
            m.add_func("application/javascript", tdewolff_minify_js::minify);
            m.add_func("image/svg+xml", svg::minify);
        }
        _ => panic!("bad html config {}", cfg),
    }
    m
}

/// The oracle's `runHTML`: `[output, err, after]` of an HTML document.
pub fn run_html(cfg: &str, input: &[u8]) -> [Vec<u8>; 3] {
    let buf = cp(input);
    let pristine = buf.slice(0, buf.cap()).to_vec();
    let m = html_m(cfg);
    let mut out = Vec::new();
    let res = catch_unwind(AssertUnwindSafe(|| {
        let mut r = tdewolff_parse::buffer::Reader::new(buf.clone());
        m.minify("text/html", &mut out, &mut r)
    }));
    let err = match res {
        Ok(Ok(())) => Vec::new(),
        Ok(Err(e)) => e.error_bytes(),
        Err(_) => b"PANIC".to_vec(),
    };
    let after = after_field(&buf, &pristine);
    [out, err, after]
}

/// FNV-1a 64 hex digest (the oracle's `digest`).
pub fn digest(b: &[u8]) -> Vec<u8> {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h).into_bytes()
}

pub fn fixtures_dir() -> PathBuf {
    if let Ok(d) = std::env::var("TDEWOLFF_MINIFY_JS_FIXTURES") {
        return PathBuf::from(d);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Reads `<name>.rec.gz`: `#rec N\n` then N fields `<len>\n<bytes>\n`. The
/// first record (the generator comment) is skipped.
pub fn records(name: &str) -> Vec<Vec<Vec<u8>>> {
    let p = fixtures_dir().join(format!("{}.rec.gz", name));
    let f = std::fs::File::open(&p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(f)
        .read_to_end(&mut data)
        .unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
    let mut recs = Vec::new();
    let mut pos = 0;
    let line = |pos: &mut usize| -> String {
        let start = *pos;
        while data[*pos] != b'\n' {
            *pos += 1;
        }
        let s = String::from_utf8(data[start..*pos].to_vec()).unwrap();
        *pos += 1;
        s
    };
    while pos < data.len() {
        let head = line(&mut pos);
        let n: usize = head
            .strip_prefix("#rec ")
            .expect("record header")
            .parse()
            .unwrap();
        let mut fields = Vec::with_capacity(n);
        for _ in 0..n {
            let len: usize = line(&mut pos).parse().unwrap();
            fields.push(data[pos..pos + len].to_vec());
            pos += len + 1;
        }
        recs.push(fields);
    }
    recs.remove(0);
    recs
}

pub fn lossy(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    if s.len() > 400 {
        format!("{}…({} bytes)", &s[..s.floor_char_boundary(400)], b.len())
    } else {
        s.into_owned()
    }
}

/// Collects mismatches and fails at the end with the first few.
pub struct Mismatches {
    name: String,
    total: usize,
    bad: Vec<String>,
    nbad: usize,
}

impl Mismatches {
    pub fn new(name: &str) -> Mismatches {
        Mismatches {
            name: name.to_string(),
            total: 0,
            bad: Vec::new(),
            nbad: 0,
        }
    }

    pub fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        self.total += 1;
        if !ok {
            self.nbad += 1;
            if self.bad.len() < 20 {
                self.bad.push(msg());
            }
        }
    }

    pub fn finish(self) {
        eprintln!(
            "{}: {}/{} match",
            self.name,
            self.total - self.nbad,
            self.total
        );
        if self.nbad != 0 {
            panic!(
                "{}: {} of {} mismatch:\n{}",
                self.name,
                self.nbad,
                self.total,
                self.bad.join("\n")
            );
        }
    }
}

/// Runs `f` on a thread with a large stack (the parser and the printer are
/// recursive like Go's, whose goroutine stacks grow up to 1 GB).
pub fn with_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}
