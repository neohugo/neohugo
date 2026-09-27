//! Differential tests against the Go oracle fixtures
//! (tools/go-oracle/tdewolff-minify `fixtures`): every upstream test
//! literal and random mutations through many minifier configurations, and
//! the byte-level helpers (Number, Decimal, Mediatype, DataURI, PathData).
//! Outputs, error strings and the input buffer after the call (in-place
//! rewrites) must all match Go.

mod common;

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use common::*;
use tdewolff_minify::{M, data_uri, decimal, mediatype, number, svg};

fn check_min(file: &str) {
    check_min_records(file, records(file));
}

fn check_min_records(file: &str, recs: Vec<Vec<String>>) {
    let mut ms: HashMap<String, M> = HashMap::new();
    let mut mm = Mismatches::new(file);
    for r in recs.iter().filter(|r| r[0] == "min") {
        let cfg = r[1].clone();
        let mt = unhex(&r[2]);
        let input = unhex(&r[3]);
        let m = ms
            .entry(cfg.clone())
            .or_insert_with(|| configs::config(&cfg));
        let res = catch_unwind(AssertUnwindSafe(|| run_m(m, &mt, &input)));
        if r[4] == "!" {
            // Go panicked (red-team records: `! HEX(panic value)`); so must the port.
            mm.check(res.is_err(), || {
                format!(
                    "[{} {}] in={}: Go panics ({}), the port returned",
                    cfg,
                    lossy(&mt),
                    lossy(&input),
                    lossy(&unhex(&r[5]))
                )
            });
            continue;
        }
        match res {
            Ok((out, err, after)) => {
                let got = (
                    same_or(&out, &input),
                    err_str(&err),
                    same_or(&after, &input),
                );
                let ok = got.0 == r[4] && got.1 == r[5] && got.2 == r[6];
                mm.check(ok, || {
                    format!(
                        "[{} {}] in={}\n  go  out={} err={} after={}\n  rs  out={} err={} after={}",
                        cfg,
                        lossy(&mt),
                        lossy(&input),
                        if r[4] == "=" {
                            "=".into()
                        } else {
                            lossy(&unhex(&r[4]))
                        },
                        if r[5] == "-" {
                            "-".into()
                        } else {
                            lossy(&unhex(&r[5]))
                        },
                        r[6],
                        if got.0 == "=" {
                            "=".into()
                        } else {
                            lossy(&out)
                        },
                        if got.1 == "-" {
                            "-".into()
                        } else {
                            lossy(&unhex(&got.1))
                        },
                        got.2
                    )
                });
            }
            Err(_) => mm.check(false, || format!("[{}] PANIC in={}", cfg, lossy(&input))),
        }
    }
    mm.finish();
}

#[test]
fn literals() {
    check_min("literals");
}

#[test]
fn fuzz() {
    check_min("fuzz");
}

#[test]
fn units() {
    let recs = records("units");
    let mut ms: HashMap<String, M> = HashMap::new();
    let mut mm = Mismatches::new("units");
    for r in &recs {
        let res = catch_unwind(AssertUnwindSafe(|| -> (String, String, String, String) {
            match r[0].as_str() {
                "num" | "dec" => {
                    let prec: i64 = r[1].parse().unwrap();
                    let input = unhex(&r[2]);
                    let buf = cp(&input);
                    let out = if r[0] == "num" {
                        number(buf.clone(), prec)
                    } else {
                        decimal(buf.clone(), prec)
                    };
                    (
                        same_or(&out.to_vec(), &input),
                        same_or(&buf.to_vec(), &input),
                        r[3].clone(),
                        r[4].clone(),
                    )
                }
                "mt" => {
                    let input = unhex(&r[1]);
                    let buf = cp(&input);
                    let out = mediatype(buf.clone());
                    (
                        same_or(&out.to_vec(), &input),
                        same_or(&buf.to_vec(), &input),
                        r[2].clone(),
                        r[3].clone(),
                    )
                }
                "duri" => {
                    let m = ms
                        .entry(r[1].clone())
                        .or_insert_with(|| configs::config(&r[1]));
                    let input = unhex(&r[2]);
                    let buf = cp(&input);
                    let out = data_uri(m, buf.clone());
                    (
                        same_or(&out.to_vec(), &input),
                        same_or(&buf.to_vec(), &input),
                        r[3].clone(),
                        r[4].clone(),
                    )
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
                    (
                        same_or(&out.to_vec(), &input),
                        same_or(&buf.to_vec(), &input),
                        r[3].clone(),
                        r[4].clone(),
                    )
                }
                k => panic!("unknown record kind {}", k),
            }
        }));
        match res {
            Ok((out, after, want_out, want_after)) => {
                mm.check(out == want_out && after == want_after, || {
                    format!(
                        "{:?}: rs out={} after={} | go out={} after={}",
                        r, out, after, want_out, want_after
                    )
                })
            }
            Err(_) => mm.check(false, || format!("{:?}: PANIC", r)),
        }
    }
    mm.finish();
}

#[test]
fn structured() {
    check_min("structured");
}

/// Red-team regressions (tools/go-oracle/tdewolff-minify `rt regress`,
/// generated by the linux/arm64 oracle): CSS functions nested 100000 deep
/// (Go minifies them; the recursive `parseFunction`/`writeFunction` and the
/// derived `Clone`/`Drop` of `css::Token` overflowed the stack) and deep
/// nesting in the other minifiers. Runs on a 1 MiB stack so any recursion
/// over the nesting depth aborts the test.
#[test]
fn redteam() {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| check_min_records("redteam", records_in(&checked_in_dir(), "redteam")))
        .unwrap()
        .join()
        .unwrap();
}

/// Red-team regressions for nested minifier calls 3333 deep (`<iframe>`
/// content and `<script type=text/html>` re-enter the HTML minifier through
/// `M`, one recursion per level as in Go). The port needs ~2.2 KB of stack
/// per level (release), so like the js crates it runs on a 1 GiB (virtual)
/// stack; with that, the output equals Go's.
#[test]
fn redteam_nested() {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(|| {
            check_min_records(
                "redteam-nested",
                records_in(&checked_in_dir(), "redteam-nested"),
            )
        })
        .unwrap()
        .join()
        .unwrap();
}
