//! Checks a record file produced by the Go oracle (full outputs or FNV
//! digests, `min`/`sub`/`html`/`multi` records as in tests/fixtures.rs) and
//! writes every differing input to OUTDIR (`NNNNNNN.js` plus
//! `NNNNNNN.cfg` with the configuration and the kind of record):
//!
//! ```sh
//! cargo run --release --example check -- /abs/path/FILE.rec.gz OUTDIR
//! ```

#[path = "../tests/common/mod.rs"]
mod common;

use std::path::PathBuf;

use common::*;

fn same(got: &[Vec<u8>; 3], want: &[Vec<u8>]) -> bool {
    let panic_d = digest(b"PANIC");
    // on a Go panic only the panic itself is compared
    if want[1] == b"PANIC" || want[1] == panic_d {
        return got[1] == b"PANIC";
    }
    (0..3).all(|k| got[k] == want[k] || digest(&got[k]) == want[k])
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: check /abs/FILE.rec.gz OUTDIR");
        std::process::exit(2);
    }
    let file = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    // Go panics are expected on some inputs; keep the output readable
    std::panic::set_hook(Box::new(|_| {}));
    let (n, bad) = with_big_stack(move || {
        let recs = records_at(&file);
        let mut bad = 0usize;
        let mut n = 0usize;
        for (i, r) in recs.iter().enumerate() {
            let kind = r[0].as_slice();
            let mut fails: Vec<String> = Vec::new();
            let input: Vec<u8>;
            match kind {
                b"min" | b"sub" | b"html" => {
                    let cfg = String::from_utf8(r[1].clone()).unwrap();
                    let (got, want) = match kind {
                        b"min" => {
                            input = r[2].clone();
                            (run_min(&cfg, &r[2]), &r[3..6])
                        }
                        b"html" => {
                            input = r[2].clone();
                            (run_html(&cfg, &r[2]), &r[3..6])
                        }
                        _ => {
                            let mut v = r[2].clone();
                            v.extend_from_slice(b"\n--JS--\n");
                            v.extend_from_slice(&r[3]);
                            v.extend_from_slice(b"\n--JS--\n");
                            v.extend_from_slice(&r[4]);
                            input = v;
                            (run_sub(&cfg, &r[2], &r[3], &r[4]), &r[5..8])
                        }
                    };
                    n += 1;
                    if !same(&got, want) {
                        fails.push(format!("{} {}", String::from_utf8_lossy(kind), cfg));
                    }
                }
                b"multi" => {
                    input = r[2].clone();
                    for c in r[3..].chunks(4) {
                        let cfg = String::from_utf8(c[0].clone()).unwrap();
                        n += 1;
                        if !same(&run_min(&cfg, &r[2]), &c[1..4]) {
                            fails.push(format!("min {}", cfg));
                        }
                    }
                }
                k => panic!("unknown record kind {:?}", lossy(k)),
            }
            if !fails.is_empty() {
                bad += 1;
                if bad == 1 {
                    std::fs::create_dir_all(&out).unwrap();
                }
                if bad <= 2000 {
                    std::fs::write(out.join(format!("{:07}.js", i)), &input).unwrap();
                    std::fs::write(out.join(format!("{:07}.cfg", i)), fails.join("\n")).unwrap();
                }
            }
        }
        (n, bad)
    });
    println!("{} checks, {} records differ", n, bad);
}
