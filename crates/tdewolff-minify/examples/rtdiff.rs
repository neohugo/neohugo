//! `rtdiff [FILE...]` — the red-team checker: reads oracle records (plain TSV
//! from stdin, e.g. `tdewolff-minify rt html 100000 1 - | rtdiff`, or
//! `.txt`/`.txt.gz` files) and checks every `min`/`num`/`dec`/`mt`/`duri`/
//! `path` record against the port exactly like `tests/fixtures.rs` (output,
//! error string, input buffer after the call). Mismatching record lines are
//! appended to `$RTDIFF_OUT` when set (for `tdewolff-minify rerun` with the
//! arm64 oracle); the first few are described on stderr. Exits 1 on any
//! mismatch. Runs on a 1 GiB stack (PORTING.md, "Deep nesting").

#[path = "../tests/common/mod.rs"]
mod common;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

use common::*;
use tdewolff_minify::{M, data_uri, decimal, mediatype, number, svg};

#[derive(Default)]
struct State {
    ms: HashMap<String, M>,
    total: u64,
    bad: u64,
    panics: u64,
    kinds: HashMap<String, (u64, u64)>,
    out: Option<std::fs::File>,
}

fn show(s: &str) -> String {
    if s == "=" || s == "-" {
        s.to_string()
    } else {
        lossy(&unhex(s))
    }
}

/// The configuration named `cfg`, cached (the "o:" names are unbounded).
fn m_of<'a>(ms: &'a mut HashMap<String, M>, cfg: &str) -> &'a M {
    if ms.len() > 256 && !ms.contains_key(cfg) {
        ms.clear(); // every M holds compiled regexps
    }
    ms.entry(cfg.to_string())
        .or_insert_with(|| configs::config(cfg))
}

/// Runs one record: (got, want, description).
fn run(ms: &mut HashMap<String, M>, r: &[String]) -> (Vec<String>, Vec<String>, String) {
    let helper = |out: tdewolff_minify::GoBytes, buf: &tdewolff_minify::GoBytes, input: &[u8]| {
        vec![same_or(&out.to_vec(), input), same_or(&buf.to_vec(), input)]
    };
    match r[0].as_str() {
        "min" => {
            let (mt, input) = (unhex(&r[2]), unhex(&r[3]));
            let (out, err, after) = run_m(m_of(ms, &r[1]), &mt, &input);
            let got = vec![
                same_or(&out, &input),
                err_str(&err),
                same_or(&after, &input),
            ];
            (
                got,
                r[4..7].to_vec(),
                format!("[min {} {}] in={}", r[1], lossy(&mt), lossy(&input)),
            )
        }
        "num" | "dec" => {
            let prec: i64 = r[1].parse().unwrap();
            let input = unhex(&r[2]);
            let buf = cp(&input);
            let out = if r[0] == "num" {
                number(buf.clone(), prec)
            } else {
                decimal(buf.clone(), prec)
            };
            let desc = format!("[{} {}] in={}", r[0], prec, lossy(&input));
            (helper(out, &buf, &input), r[3..5].to_vec(), desc)
        }
        "mt" => {
            let input = unhex(&r[1]);
            let buf = cp(&input);
            let out = mediatype(buf.clone());
            (
                helper(out, &buf, &input),
                r[2..4].to_vec(),
                format!("[mt] in={}", lossy(&input)),
            )
        }
        "duri" => {
            let input = unhex(&r[2]);
            let buf = cp(&input);
            let out = data_uri(m_of(ms, &r[1]), buf.clone());
            let desc = format!("[duri {}] in={}", r[1], lossy(&input));
            (helper(out, &buf, &input), r[3..5].to_vec(), desc)
        }
        "path" => {
            let prec: i64 = r[1].parse().unwrap();
            let input = unhex(&r[2]);
            let buf = cp(&input);
            let mut p = svg::PathData::new(&svg::Minifier {
                precision: prec,
                ..Default::default()
            });
            let out = p.shorten_path_data(buf.clone());
            let desc = format!("[path {}] in={}", prec, lossy(&input));
            (helper(out, &buf, &input), r[3..5].to_vec(), desc)
        }
        k => panic!("unknown record kind {}", k),
    }
}

fn check(st: &mut State, line: &str) {
    if line.is_empty() || line.starts_with('#') {
        return;
    }
    let r: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
    let res = catch_unwind(AssertUnwindSafe(|| run(&mut st.ms, &r)));
    st.total += 1;
    let e = st.kinds.entry(r[0].clone()).or_insert((0, 0));
    e.0 += 1;
    if matches!(&res, Ok((got, want, _)) if got == want) {
        return;
    }
    if r[0] == "min" && r[4] == "!" && res.is_err() {
        st.panics += 1;
        return; // Go panicked on this input (`! HEX(panic value)`) and so did the port
    }
    e.1 += 1;
    st.bad += 1;
    if let Some(out) = &mut st.out {
        writeln!(out, "{}", line).unwrap();
    }
    if st.bad <= 12 {
        match res {
            Ok((got, want, desc)) => {
                let join = |v: &[String]| v.iter().map(|s| show(s)).collect::<Vec<_>>().join(" | ");
                eprintln!(
                    "MISMATCH {}\n  go: {}\n  rs: {}",
                    desc,
                    join(&want),
                    join(&got)
                );
            }
            Err(_) => eprintln!("PANIC {}", &line[..line.len().min(400)]),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bad = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(move || {
            let mut st = State {
                out: std::env::var("RTDIFF_OUT").ok().map(|p| {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(p)
                        .unwrap()
                }),
                ..Default::default()
            };
            std::panic::set_hook(Box::new(|_| {}));
            let mut feed = |rd: Box<dyn Read>| {
                for line in BufReader::with_capacity(1 << 20, rd).split(b'\n') {
                    check(&mut st, &String::from_utf8(line.unwrap()).unwrap());
                }
            };
            if args.is_empty() {
                feed(Box::new(std::io::stdin()));
            }
            for a in &args {
                let f = std::fs::File::open(a).unwrap();
                if a.ends_with(".gz") {
                    feed(Box::new(flate2::read::GzDecoder::new(f)));
                } else {
                    feed(Box::new(f));
                }
            }
            let mut ks: Vec<_> = st.kinds.iter().collect();
            ks.sort();
            for (k, (n, b)) in ks {
                eprintln!("  {}: {}/{} ok", k, n - b, n);
            }
            eprintln!(
                "rtdiff: {} records, {} mismatches ({} Go panics matched)",
                st.total, st.bad, st.panics
            );
            st.bad
        })
        .unwrap()
        .join()
        .unwrap();
    if bad > 0 {
        std::process::exit(1);
    }
}
