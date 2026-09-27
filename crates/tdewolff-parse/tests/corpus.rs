//! Corpus differential tests: for every file of the seeksnack corpora the Go
//! oracle recorded the FNV-1a hash of the serialized token/grammar stream
//! (tests/fixtures/corpus/<label>.<kind>.tsv). The corpora live outside the
//! repository, so these tests are `#[ignore]`d; run them with
//!
//!   TDEWOLFF_PARSE_SCRATCH=<scratch> cargo test --release --test corpus -- --ignored
//!
//! where `<scratch>` contains `golden/`, `canon/pristine-seeksnack/`,
//! `work/minify/corpus2/` and `work/tdewolff-parse/embedded/`.

mod common;
use common::*;

use std::path::{Path, PathBuf};

const DEFAULT_SCRATCH: &str = "/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad";

fn scratch() -> PathBuf {
    std::env::var("TDEWOLFF_PARSE_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SCRATCH))
}

fn root_for(label: &str) -> PathBuf {
    let s = scratch();
    match label {
        "golden-nominify" => s.join("golden/nominify"),
        "golden-canonical" => s.join("golden/canonical"),
        "corpus2-css" => s.join("work/minify/corpus2/css"),
        "corpus2-css-resource" => s.join("work/minify/corpus2/css-resource"),
        "corpus2-svg" => s.join("work/minify/corpus2/svg"),
        "corpus2-json" => s.join("work/minify/corpus2/json"),
        "embedded-css" => s.join("work/tdewolff-parse/embedded/css"),
        "embedded-cssinline" => s.join("work/tdewolff-parse/embedded/cssinline"),
        "embedded-svg" => s.join("work/tdewolff-parse/embedded/svg"),
        "embedded-json" => s.join("work/tdewolff-parse/embedded/json"),
        "pristine" => s.join("canon/pristine-seeksnack"),
        _ => panic!("unknown corpus label {}", label),
    }
}

struct Entry {
    rel: String,
    size: usize,
    hash: u64,
}

fn run_digest_file(path: &Path) -> (usize, usize, usize) {
    let name = path.file_name().unwrap().to_str().unwrap();
    let parts: Vec<&str> = name.trim_end_matches(".tsv").rsplitn(2, '.').collect();
    let (kind, label) = (parts[0], parts[1]);
    let root = root_for(label);
    if !root.exists() {
        eprintln!("SKIP {}: {} not found", name, root.display());
        return (0, 0, 0);
    }
    let text = std::fs::read_to_string(path).unwrap();
    let entries: Vec<Entry> = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Entry {
                rel: f[0].to_string(),
                size: f[1].parse().unwrap(),
                hash: u64::from_str_radix(f[2], 16).unwrap(),
            }
        })
        .collect();
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let chunk = entries.len().div_ceil(nthreads).max(1);
    let results: Vec<(usize, usize, Vec<String>)> = std::thread::scope(|sc| {
        let handles: Vec<_> = entries
            .chunks(chunk)
            .map(|es| {
                let root = root.clone();
                sc.spawn(move || {
                    let (mut ok, mut bytes, mut fails) = (0usize, 0usize, Vec::new());
                    for e in es {
                        let p = root.join(&e.rel);
                        let data = match std::fs::read(&p) {
                            Ok(d) => d,
                            Err(err) => {
                                fails.push(format!("{}: {}", p.display(), err));
                                continue;
                            }
                        };
                        if data.len() != e.size {
                            fails.push(format!("{}: size {} != {}", e.rel, data.len(), e.size));
                            continue;
                        }
                        let s = stream_for(kind, &data);
                        if fnv64a(s.as_bytes()) == e.hash {
                            ok += 1;
                            bytes += data.len();
                        } else {
                            fails.push(format!("{} ({}): stream hash differs", e.rel, kind));
                        }
                    }
                    (ok, bytes, fails)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let ok: usize = results.iter().map(|r| r.0).sum();
    let bytes: usize = results.iter().map(|r| r.1).sum();
    let fails: Vec<&String> = results.iter().flat_map(|r| r.2.iter()).collect();
    for f in fails.iter().take(20) {
        eprintln!("FAIL {}: {}", name, f);
    }
    eprintln!(
        "{}: {}/{} files identical ({} bytes)",
        name,
        ok,
        entries.len(),
        bytes
    );
    (ok, entries.len(), fails.len())
}

#[test]
#[ignore = "needs the seeksnack corpora outside the repository; run with --ignored"]
fn corpus_digests() {
    let dir = fixtures_dir().join("corpus");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "tsv"))
        .collect();
    files.sort();
    let (mut ok, mut total, mut failed) = (0, 0, 0);
    for f in &files {
        let (o, t, fl) = run_digest_file(f);
        ok += o;
        total += t;
        failed += fl;
    }
    eprintln!(
        "corpus total: {}/{} identical, {} failed",
        ok, total, failed
    );
    assert_eq!(failed, 0);
}
