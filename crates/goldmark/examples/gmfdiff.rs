//! Debug helper: renders the records of a GMF fixture with the Rust port
//! and prints the ones that differ from the Go output.
//!
//!     cargo run --example gmfdiff -- <file.gmf[.gz]> [name-substring] [max]
#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let recs = common::parse_gmf(&common::read_file(std::path::Path::new(&args[1])));
    let filter = args.get(2).cloned().unwrap_or_default();
    let max: usize = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(5);
    let mut mds = common::Markdowns::new();
    let mut shown = 0;
    let mut bad = 0;
    let mut total = 0;
    std::panic::set_hook(Box::new(|_| {}));
    for r in &recs {
        if !r.name.contains(&filter) {
            continue;
        }
        total += 1;
        let cfg = r.str("cfg");
        let got = mds.convert(&cfg, r.get("md"));
        let want = r.get("html");
        let ok = if want.starts_with(b"PANIC: ") {
            got == b"PANIC"
        } else {
            got == want
        };
        if ok {
            continue;
        }
        bad += 1;
        if shown >= max {
            continue;
        }
        shown += 1;
        let p = got
            .iter()
            .zip(want.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(got.len().min(want.len()));
        let lo = p.saturating_sub(120);
        println!("--- {}", r.name);
        println!("md:   {:?}", String::from_utf8_lossy(r.get("md")));
        println!(
            "want@{p}: {:?}",
            String::from_utf8_lossy(&want[lo..(p + 200).min(want.len())])
        );
        println!(
            "got @{p}: {:?}",
            String::from_utf8_lossy(&got[lo..(p + 200).min(got.len())])
        );
    }
    println!("{bad}/{total} differ");
}
