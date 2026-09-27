//! Corpus differential test (ignored by default: needs the corpora, which
//! live outside the repository). Compares the FNV digests of every mode's
//! dump for every JS file of the corpora with the Go oracle's digests in
//! tests/fixtures/corpus.tsv.
//!
//!     TDEWOLFF_PARSE_JS_CORPUS_ROOTS=<dir>:<dir>... cargo test --release --test corpus -- --ignored
//!
//! Each root is matched to the TSV's first column by its base name.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use common::*;

/// The corpus roots: TDEWOLFF_PARSE_JS_CORPUS_ROOTS (a ':' list), or else the
/// four standard subdirectories of TDEWOLFF_PARSE_JS_CORPUS (a scratch root
/// holding work/minify/corpus2, golden/{canonical,nominify} and
/// canon/pristine-seeksnack). The corpora are not in the repository: with
/// neither variable set the test prints a note and passes without checking.
fn roots() -> Option<HashMap<String, PathBuf>> {
    let list = match std::env::var("TDEWOLFF_PARSE_JS_CORPUS_ROOTS") {
        Ok(l) => l,
        Err(_) => {
            let Ok(scratch) = std::env::var("TDEWOLFF_PARSE_JS_CORPUS") else {
                eprintln!(
                    "note: TDEWOLFF_PARSE_JS_CORPUS_ROOTS / TDEWOLFF_PARSE_JS_CORPUS not set; skipping corpus test"
                );
                return None;
            };
            [
                "work/minify/corpus2",
                "golden/canonical",
                "golden/nominify",
                "canon/pristine-seeksnack",
            ]
            .iter()
            .map(|r| format!("{}/{}", scratch, r))
            .collect::<Vec<_>>()
            .join(":")
        }
    };
    let mut m = HashMap::new();
    for r in list.split(':').filter(|r| !r.is_empty()) {
        let p = PathBuf::from(r);
        let base = p.file_name().unwrap().to_string_lossy().into_owned();
        m.insert(base, p);
    }
    Some(m)
}

#[test]
#[ignore]
fn corpus() {
    let tsv_path = std::env::var("TDEWOLFF_PARSE_JS_CORPUS_TSV")
        .map(PathBuf::from)
        .unwrap_or_else(|_| fixtures_dir().join("corpus.tsv"));
    let tsv = std::fs::read_to_string(&tsv_path).unwrap();
    let Some(roots) = roots() else {
        return;
    };
    let mut jobs = Vec::new();
    for line in tsv.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
        let Some(root) = roots.get(&f[0]).filter(|r| r.is_dir()).cloned() else {
            eprintln!("note: corpus root {} is absent; skipping", f[0]);
            return;
        };
        jobs.push((root.join(&f[1]), f));
    }
    // largest files first for better load balancing
    jobs.sort_by_key(|(_, f)| std::cmp::Reverse(f[2].parse::<usize>().unwrap()));
    let n = jobs.len();
    let jobs = Arc::new(Mutex::new(jobs));
    let fails = Arc::new(Mutex::new(Vec::<String>::new()));
    let bytes = Arc::new(Mutex::new(0usize));
    let threads: Vec<_> = (0..std::thread::available_parallelism().map_or(4, |n| n.get()))
        .map(|_| {
            let jobs = jobs.clone();
            let fails = fails.clone();
            let bytes = bytes.clone();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || {
                    loop {
                        let job = jobs.lock().unwrap().pop();
                        let Some((path, f)) = job else { break };
                        let src = std::fs::read(&path)
                            .unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
                        assert_eq!(src.len().to_string(), f[2], "{}", path.display());
                        *bytes.lock().unwrap() += src.len();
                        for (k, mode) in MODES.iter().enumerate() {
                            let got = digest(&run_mode(mode, &src));
                            if got != f[3 + k] {
                                fails
                                    .lock()
                                    .unwrap()
                                    .push(format!("{} {}", mode, path.display()));
                            }
                        }
                    }
                })
                .unwrap()
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let fails = fails.lock().unwrap();
    for f in fails.iter().take(40) {
        eprintln!("DIFF {}", f);
    }
    assert!(
        fails.is_empty(),
        "{} of {} files x {} modes differ",
        fails.len(),
        n,
        MODES.len()
    );
    eprintln!(
        "corpus: {} files ({} bytes) x {} modes identical",
        n,
        bytes.lock().unwrap(),
        MODES.len()
    );
}
