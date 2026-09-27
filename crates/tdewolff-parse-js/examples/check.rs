//! Runs every mode over a record file produced by the Go oracle (digests or
//! full dumps) and writes the inputs whose dumps differ to OUTDIR (one file
//! per input, `NNNNN.js`, plus `NNNNN.modes` listing the differing modes).
//!
//!     cargo run --release --example check -- FILE.rec.gz OUTDIR

#[path = "../tests/common/mod.rs"]
mod common;

use std::path::PathBuf;

use common::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: check FILE.rec.gz OUTDIR");
        std::process::exit(2);
    }
    let file = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out).unwrap();
    // the dumps record Go's panics; keep the output readable
    std::panic::set_hook(Box::new(|_| {}));
    let (n, bad) = big_stack(move || {
        // read_records resolves names against fixtures_dir(); pass an absolute path
        let recs = read_records(file.to_str().unwrap());
        let mut bad = 0usize;
        for (i, rec) in recs.iter().enumerate() {
            let src = &rec[0];
            let mut modes = Vec::new();
            for (k, mode) in MODES.iter().enumerate() {
                let got = std::panic::catch_unwind(|| run_mode(mode, src))
                    .unwrap_or_else(|_| b"UNCAUGHT PANIC\n".to_vec());
                // full dump or FNV digest of it
                let ok = got == rec[k + 1] || digest(&got).as_bytes() == &rec[k + 1][..];
                if !ok {
                    modes.push(*mode);
                }
            }
            if !modes.is_empty() {
                bad += 1;
                std::fs::write(out.join(format!("{:05}.js", i)), src).unwrap();
                std::fs::write(out.join(format!("{:05}.modes", i)), modes.join(" ")).unwrap();
            }
        }
        (recs.len(), bad)
    });
    println!("{} records, {} differ", n, bad);
}
