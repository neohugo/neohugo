//! Corpus parity (`#[ignore]`: needs the JS files of the Go module cache):
//! every .js/.mjs/.cjs file of `github.com/tdewolff/minify/v2@v2.23.8`
//! (upstream test corpus and `_benchmarks`: jquery, jquery-ui, ace, moment,
//! victory, echarts, antd, typescript.js 10 MB, ...), `evanw/esbuild@v0.25.6`
//! (test scripts), `golang.org/x/tools@v0.34.0` and the neohugo repository,
//! through 6 configurations; `tests/fixtures/corpus.tsv` holds Go's FNV
//! digests of the output, the error and the buffer after the call.
//!
//! ```sh
//! GOMODCACHE=$(go env GOMODCACHE) cargo test --release --test corpus -- --ignored --nocapture
//! ```
//!
//! Files that are missing (or differ from the recorded input digest) are
//! skipped and counted.

mod common;

use std::path::PathBuf;

use common::*;

fn mod_cache() -> PathBuf {
    if let Ok(d) = std::env::var("GOMODCACHE") {
        return PathBuf::from(d);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("go/pkg/mod")
}

#[test]
#[ignore]
fn corpus() {
    with_big_stack(|| {
        let tsv = std::fs::read_to_string(fixtures_dir().join("corpus.tsv")).unwrap();
        let mut lines = tsv.lines().filter(|l| !l.starts_with("# generated"));
        let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
        let cfgs = &header[4..];
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut mm = Mismatches::new("corpus");
        let (mut skipped, mut bytes) = (0, 0);
        for line in lines {
            let f: Vec<&str> = line.split('\t').collect();
            let root = if f[0] == "repo" {
                repo.clone()
            } else {
                mod_cache().join(f[0])
            };
            let Ok(input) = std::fs::read(root.join(f[1])) else {
                skipped += 1;
                continue;
            };
            if digest(&input) != f[2].as_bytes() {
                skipped += 1;
                continue;
            }
            bytes += input.len();
            for (k, cfg) in cfgs.iter().enumerate() {
                let got: Vec<String> = run_min(cfg, &input)
                    .iter()
                    .map(|x| String::from_utf8(digest(x)).unwrap())
                    .collect();
                mm.check(got.join(",") == f[4 + k], || {
                    format!("[{}] {}/{}", cfg, f[0], f[1])
                });
            }
        }
        eprintln!("corpus: {} bytes checked, {} files skipped", bytes, skipped);
        mm.finish();
    });
}
