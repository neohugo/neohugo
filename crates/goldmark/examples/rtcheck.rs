//! Red-team checker: streams GMF records (from a file or stdin, e.g. piped
//! from `goldmark fuzz -mode ...`), renders each with the Rust port on a
//! thread with a bounded stack (like Hugo's worker threads) and compares the
//! HTML — and the AST dump / `Node.Text` values, when the record has an
//! `ast` / `texts` field — with the Go output; `attrvec` records (field `in`)
//! check `parser.ParseAttributes` directly. Reports the slowest records (Rust
//! vs Go time, when recorded with `-time`).
//!
//!     goldmark fuzz -mode long -n 100 -seed 1 | \
//!       cargo run --release --example rtcheck -- - [max-shown]
//!
//! Environment: `RT_STACK` (bytes, default 2 MiB), `RT_TRACE=<file>` writes
//! each record name to <file> before rendering it (to find the record that
//! aborts the process, e.g. on a stack overflow).
#[path = "../tests/common/mod.rs"]
mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::time::Instant;

fn read_record<R: BufRead>(r: &mut R, pending: &mut Option<String>) -> Option<common::Record> {
    let mut line = String::new();
    let name = match pending.take() {
        Some(n) => n,
        None => {
            line.clear();
            if r.read_line(&mut line).unwrap() == 0 {
                return None;
            }
            line.trim_end_matches('\n')
                .strip_prefix("=== ")
                .expect("record header")
                .to_string()
        }
    };
    let mut rec = common::Record {
        name,
        fields: Default::default(),
        keys: Vec::new(),
    };
    loop {
        line.clear();
        if r.read_line(&mut line).unwrap() == 0 {
            return Some(rec);
        }
        let l = line.trim_end_matches('\n');
        if let Some(n) = l.strip_prefix("=== ") {
            *pending = Some(n.to_string());
            return Some(rec);
        }
        let sp = l.rfind(' ').unwrap();
        let key = l[..sp].to_string();
        let n: usize = l[sp + 1..].parse().unwrap();
        let mut v = vec![0u8; n + 1];
        r.read_exact(&mut v).unwrap();
        assert_eq!(v.pop(), Some(b'\n'));
        rec.keys.push(key.clone());
        rec.fields.insert(key, v);
    }
}

fn show(label: &str, b: &[u8], p: usize) -> String {
    let lo = p.saturating_sub(150);
    let hi = (p + 250).min(b.len());
    format!(
        "{label}@{p} (len {}): {:?}",
        b.len(),
        String::from_utf8_lossy(&b[lo.min(hi)..hi])
    )
}

fn first_diff(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()))
}

fn run() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).cloned().unwrap_or("-".into());
    let max: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(5);
    let input: Box<dyn Read> = if path == "-" {
        Box::new(std::io::stdin())
    } else if path.ends_with(".gz") {
        Box::new(flate2::read::GzDecoder::new(
            std::fs::File::open(&path).unwrap(),
        ))
    } else {
        Box::new(std::fs::File::open(&path).unwrap())
    };
    let mut r = BufReader::with_capacity(1 << 20, input);
    let trace = std::env::var("RT_TRACE").ok();
    let mut mds = common::Markdowns::new();
    let (mut total, mut bad, mut shown, mut ast_checked) = (0usize, 0usize, 0usize, 0usize);
    let mut slow: Vec<(f64, f64, String, usize)> = Vec::new();
    let mut pending = None;
    std::panic::set_hook(Box::new(|_| {}));
    while let Some(rec) = read_record(&mut r, &mut pending) {
        total += 1;
        if let Some(t) = &trace {
            std::fs::write(t, rec.name.as_bytes()).unwrap();
        }
        if let Some(input) = rec.fields.get("in") {
            // attrvec records: parser.ParseAttributes called directly
            for (key, block) in [("reader", false), ("block", true)] {
                let got = common::astdump::attr_vec(input, block);
                let want = rec.get(key);
                if got != want {
                    bad += 1;
                    if shown < max {
                        shown += 1;
                        let p = first_diff(&got, want);
                        println!("--- {} ({key} differs)", rec.name);
                        println!(
                            "in: {:?}",
                            String::from_utf8_lossy(&input[..input.len().min(300)])
                        );
                        println!("{}", show("want", want, p));
                        println!("{}", show("got ", &got, p));
                    }
                    break;
                }
            }
            continue;
        }
        let cfg = rec.str("cfg");
        let md = rec.get("md");
        let t0 = Instant::now();
        let got = mds.convert(&cfg, md);
        let rust_s = t0.elapsed().as_secs_f64();
        let go_s = rec
            .fields
            .get("ns")
            .map(|v| String::from_utf8_lossy(v).parse::<f64>().unwrap() / 1e9)
            .unwrap_or(0.0);
        if rust_s > 0.05 || go_s > 0.05 {
            slow.push((rust_s, go_s, rec.name.clone(), md.len()));
        }
        let want = rec.get("html");
        let mut ok = if want.starts_with(b"PANIC: ") {
            got == b"PANIC"
        } else {
            got == want
        };
        let mut what = "html";
        let (mut w, mut g) = (want.to_vec(), got);
        if ok && let Some(want_ast) = rec.fields.get("ast") {
            ast_checked += 1;
            let got_ast = common::astdump::dump_cfg(&mut mds, &cfg, md);
            if &got_ast != want_ast {
                ok = false;
                what = "ast";
                w = want_ast.clone();
                g = got_ast;
            }
        }
        if ok && let Some(want_texts) = rec.fields.get("texts") {
            let got_texts = common::astdump::texts_cfg(&mut mds, &cfg, md);
            if &got_texts != want_texts {
                ok = false;
                what = "texts";
                w = want_texts.clone();
                g = got_texts;
            }
        }
        if ok {
            continue;
        }
        bad += 1;
        if shown >= max {
            continue;
        }
        shown += 1;
        let p = first_diff(&g, &w);
        println!("--- {} ({what} differs)", rec.name);
        let mdp = String::from_utf8_lossy(md);
        if mdp.len() > 600 {
            println!(
                "md (len {}): {:?}...",
                md.len(),
                &mdp[..mdp.floor_char_boundary(600)]
            );
        } else {
            println!("md:   {mdp:?}");
        }
        println!("{}", show("want", &w, p));
        println!("{}", show("got ", &g, p));
    }
    slow.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (rs, gs, name, len) in slow.iter().take(10) {
        println!("slow: rust {rs:.3}s go {gs:.3}s len {len} {name}");
    }
    println!("{bad}/{total} differ ({ast_checked} AST dumps compared)");
    std::io::stdout().flush().unwrap();
}

fn main() {
    let stack: usize = std::env::var("RT_STACK")
        .ok()
        .map(|s| s.parse().unwrap())
        .unwrap_or(2 << 20);
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
