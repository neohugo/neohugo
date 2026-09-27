//! Whole-site differential tests (ignored by default; they need the corpora
//! outside the repository). Run with
//!
//! ```text
//! TDEWOLFF_MINIFY_CORPUS=<scratch root with golden/ and work/minify/> \
//!   cargo test --release --test corpus -- --ignored --nocapture
//! ```
//!
//! * `golden_nominify_digests`: every .html/.xml/.json of the non-minified
//!   seeksnack build through the seeksnack configuration without a JS
//!   minifier; output digests must equal the Go oracle's
//!   (tests/fixtures/corpus/golden-nominify.tsv).
//! * `corpus2_nested`: the recorded nested minifier calls of a real build
//!   (CSS in `<style>`/`style=`, JSON-LD, SVG data URIs, the postcss
//!   resource) through the seeksnack M; must equal the Go oracle and the
//!   recorded Go outputs.
//! * `golden_canonical_with_js_stub`: with a JS stand-in that maps the six
//!   recorded inline scripts to their recorded Go outputs, the minified pages
//!   must equal the golden `--minify` build byte for byte.

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::configs::{JS_PATTERN, seeksnack_m};
use common::*;
use tdewolff_minify::{GoError, GoReader, M, MinifierFunc, Params, Regexp, Writer};

/// The corpus root (holding golden/, work/minify/, canon/) from
/// TDEWOLFF_MINIFY_CORPUS. The corpora are not in the repository: when the
/// variable is unset, or a needed directory is absent, the test prints a note
/// and passes without checking.
fn corpus_root() -> Option<PathBuf> {
    match std::env::var("TDEWOLFF_MINIFY_CORPUS") {
        Ok(p) => Some(PathBuf::from(p)),
        Err(_) => {
            eprintln!("note: TDEWOLFF_MINIFY_CORPUS is not set; skipping corpus test");
            None
        }
    }
}

/// `<corpus root>/<sub>` if it exists, else a note and None (skip).
fn corpus_dir(sub: &str) -> Option<PathBuf> {
    let d = corpus_root()?.join(sub);
    if d.is_dir() {
        Some(d)
    } else {
        eprintln!("note: corpus {} is absent; skipping", d.display());
        None
    }
}

fn tsv(name: &str) -> Vec<Vec<String>> {
    let p = fixtures_dir().join("corpus").join(name);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("{}: {}", p.display(), e))
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').map(|s| s.to_string()).collect())
        .collect()
}

fn mediatype_for(p: &str) -> &'static [u8] {
    if p.ends_with(".html") || p.ends_with(".htm") {
        b"text/html"
    } else if p.ends_with(".xml") {
        b"application/rss+xml"
    } else if p.ends_with(".css") || p.ends_with(".scss") {
        b"text/css"
    } else if p.ends_with(".svg") {
        b"image/svg+xml"
    } else {
        b"application/json"
    }
}

fn minify_file(m: &M, mediatype: &[u8], input: &[u8]) -> (Vec<u8>, Option<GoError>) {
    let buf = tdewolff_minify::GoBytes::from_slice(input);
    let mut out = Vec::with_capacity(input.len());
    let mut r = tdewolff_parse::buffer::Reader::new(buf);
    let err = m.minify(mediatype, &mut out, &mut r).err();
    (out, err)
}

#[test]
#[ignore]
fn golden_nominify_digests() {
    let Some(root) = corpus_dir("golden/nominify") else {
        return;
    };
    let m = seeksnack_m();
    let mut mm = Mismatches::new("golden-nominify (seeksnack config, no JS)");
    let mut bytes_in = 0usize;
    for r in tsv("golden-nominify.tsv") {
        let input = std::fs::read(root.join(&r[0])).unwrap();
        bytes_in += input.len();
        let (out, err) = minify_file(&m, mediatype_for(&r[0]), &input);
        let got = (out.len().to_string(), fnv64(&out), err_str(&err));
        mm.check(got.0 == r[2] && got.1 == r[3] && got.2 == r[4], || {
            format!(
                "{}: rs len={} fnv={} err={} | go {:?}",
                r[0],
                got.0,
                got.1,
                got.2,
                &r[2..]
            )
        });
    }
    eprintln!("minified {} input bytes", bytes_in);
    mm.finish();
}

#[test]
#[ignore]
fn corpus2_nested() {
    let Some(root) = corpus_dir("work/minify/corpus2") else {
        return;
    };
    let m = seeksnack_m();
    for (kind, mimetype) in [
        ("css", &b"text/css"[..]),
        ("json", b"application/json"),
        ("svg", b"image/svg+xml"),
        ("css-resource", b"text/css"),
    ] {
        let mut mm = Mismatches::new(&format!("corpus2/{}", kind));
        for r in tsv(&format!("corpus2-{}.tsv", kind)) {
            let base = root.join(kind).join(&r[0]);
            let input = std::fs::read(base.with_extension("in")).unwrap();
            let recorded = std::fs::read(base.with_extension("out")).unwrap();
            let params: Option<Params> = if r[1] == "-" {
                None
            } else {
                let mut p = Params::new();
                for kv in r[1].split(';') {
                    let (k, v) = kv.split_once('=').unwrap();
                    p.insert(k.as_bytes().to_vec(), v.as_bytes().to_vec());
                }
                Some(p)
            };
            let buf = tdewolff_minify::GoBytes::from_slice(&input);
            let mut out = Vec::new();
            let mut rd = tdewolff_parse::buffer::Reader::new(buf);
            let err = m
                .minify_mimetype(mimetype, &mut out, &mut rd, params.as_ref())
                .err();
            let ok = out.len().to_string() == r[3]
                && fnv64(&out) == r[4]
                && err_str(&err) == r[5]
                && out == recorded;
            mm.check(ok, || {
                format!("{}: rs len={} | go {:?}", r[0], out.len(), &r[2..])
            });
        }
        mm.finish();
    }
}

/// The six recorded inline scripts (corpus2/js): input -> Go output.
fn js_stub_table(root: &Path) -> HashMap<Vec<u8>, Vec<u8>> {
    let mut t = HashMap::new();
    for e in std::fs::read_dir(root.join("work/minify/corpus2/js")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "in").unwrap_or(false) {
            let input = std::fs::read(&p).unwrap();
            let out = std::fs::read(p.with_extension("out")).unwrap();
            t.insert(input, out);
        }
    }
    t
}

#[test]
#[ignore]
fn golden_canonical_with_js_stub() {
    let Some(root) = corpus_root() else {
        return;
    };
    let nominify = root.join("golden/nominify");
    let canonical = root.join("golden/canonical");
    let canonical2 = root.join("golden/canonical2");
    if !(nominify.is_dir() && canonical.is_dir() && root.join("work/minify/corpus2/js").is_dir()) {
        eprintln!(
            "note: golden/nominify, golden/canonical or work/minify/corpus2/js absent under {}; skipping",
            root.display()
        );
        return;
    }
    let table = Arc::new(js_stub_table(&root));
    let mut m = seeksnack_m();
    let t = table.clone();
    m.add_regexp(
        Regexp::must_compile(JS_PATTERN),
        Arc::new(MinifierFunc(
            move |_m: &M, w: &mut dyn Writer, r: &mut dyn GoReader, _p: Option<&Params>| {
                let (data, err) = tdewolff_parse::read_all(r);
                if let Some(e) = err {
                    return Err(e);
                }
                match t.get(&data.to_vec()) {
                    Some(out) => {
                        w.write(out)?;
                        Ok(())
                    }
                    None => Err(GoError::Other(b"js stub: unknown script".to_vec())),
                }
            },
        )),
    );
    let (mut total, mut same, mut same2) = (0, 0, 0);
    let mut diffs = Vec::new();
    let mut statics = Vec::new();
    for r in tsv("golden-nominify.tsv") {
        let input = std::fs::read(nominify.join(&r[0])).unwrap();
        let golden = match std::fs::read(canonical.join(&r[0])) {
            Ok(g) => g,
            Err(_) => continue,
        };
        total += 1;
        let (out, err) = minify_file(&m, mediatype_for(&r[0]), &input);
        if err.is_none() && out == golden {
            same += 1;
        } else if err.is_none() && std::fs::read(canonical2.join(&r[0])).ok() == Some(out.clone()) {
            same2 += 1;
        } else if golden == input {
            // static files (static/**) are published without minification
            statics.push(r[0].clone());
        } else {
            diffs.push(format!("{} (err={})", r[0], err_str(&err)));
            if let Ok(d) = std::env::var("TDEWOLFF_MINIFY_DUMP") {
                let p = Path::new(&d).join(&r[0]);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(p, &out).unwrap();
            }
        }
    }
    eprintln!(
        "golden canonical parity: {} files, {} identical to canonical, {} identical to canonical2, {} static (not minified by neohugo), {} differ",
        total,
        same,
        same2,
        statics.len(),
        diffs.len()
    );
    for d in diffs.iter().take(30) {
        eprintln!("  DIFF {}", d);
    }
    // The differing files are the known upstream nondeterminism (term-title
    // collisions in RSS/HTML between Go runs, see the minify spec); they
    // must stay few.
    assert!(
        diffs.len() <= 8,
        "{} files differ from the golden build",
        diffs.len()
    );
}

/// The pristine site sources (templates, SCSS/CSS, 2048 SVGs, JSON, XML)
/// through the seeksnack configuration: adversarial inputs the minifiers
/// never see in a real build (Go templates in HTML, raw SCSS, ...).
#[test]
#[ignore]
fn pristine_sources_digests() {
    let Some(root) = corpus_dir("canon/pristine-seeksnack") else {
        return;
    };
    let m = seeksnack_m();
    let mut mm = Mismatches::new("pristine sources (seeksnack config, no JS)");
    for r in tsv("pristine.tsv") {
        let input = std::fs::read(root.join(&r[0])).unwrap();
        let (out, err) = minify_file(&m, mediatype_for(&r[0]), &input);
        let got = (out.len().to_string(), fnv64(&out), err_str(&err));
        mm.check(got.0 == r[2] && got.1 == r[3] && got.2 == r[4], || {
            format!(
                "{}: rs len={} fnv={} err={} | go {:?}",
                r[0],
                got.0,
                got.1,
                got.2,
                &r[2..]
            )
        });
    }
    mm.finish();
}

/// Every .html/.xml/.json of `golden/nominify` through every named
/// configuration (html option combinations, `M.URL`, precisions, js
/// stand-ins, ...): output length and digest, error and the input buffer
/// after the call must equal the Go oracle (`cfgdigests` mode; the TSV lives
/// outside the repository, path in `TDEWOLFF_MINIFY_ALLCFG`, default
/// `<corpus root>/work/tdewolff-minify/golden-allcfg.tsv`).
#[test]
#[ignore]
fn golden_nominify_all_configs() {
    let Some(root) = corpus_root() else {
        return;
    };
    let tsv_path = std::env::var("TDEWOLFF_MINIFY_ALLCFG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("work/tdewolff-minify/golden-allcfg.tsv"));
    let nominify = root.join("golden/nominify");
    if !(tsv_path.is_file() && nominify.is_dir()) {
        eprintln!(
            "note: {} or {} absent; skipping",
            tsv_path.display(),
            nominify.display()
        );
        return;
    }
    let text = std::fs::read_to_string(&tsv_path)
        .unwrap_or_else(|e| panic!("{}: {}", tsv_path.display(), e));
    let mut ms: HashMap<String, M> = HashMap::new();
    let mut inputs: HashMap<String, Vec<u8>> = HashMap::new();
    let mut mm = Mismatches::new("golden-nominify (all configs)");
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let r: Vec<&str> = line.split('\t').collect();
        let m = ms
            .entry(r[0].to_string())
            .or_insert_with(|| common::configs::config(r[0]));
        let input = inputs
            .entry(r[1].to_string())
            .or_insert_with(|| std::fs::read(nominify.join(r[1])).unwrap());
        let (out, err, after) = run_m(m, mediatype_for(r[1]), input);
        let got = (
            out.len().to_string(),
            fnv64(&out),
            err_str(&err),
            fnv64(&after),
        );
        mm.check(
            got.0 == r[3] && got.1 == r[4] && got.2 == r[5] && got.3 == r[6],
            || format!("[{}] {}: rs {:?} | go {:?}", r[0], r[1], got, &r[3..]),
        );
    }
    mm.finish();
}
