//! Timing harness: decode every JPEG listed in a file (paths relative to a
//! root), convert to RGBA with draw.Draw(Src) and re-encode at q75.
//! Usage: cargo run --release --example jpegbench <root> <listfile>

use go_image::draw::{self, Op};
use go_image::{RGBA, jpeg};
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = std::path::PathBuf::from(&args[1]);
    let list = std::fs::read_to_string(&args[2]).unwrap();
    let (mut td, mut tc, mut te) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut total = 0usize;
    for rel in list.lines() {
        let data = std::fs::read(root.join(rel)).unwrap();
        let t = Instant::now();
        let m = jpeg::decode(&mut &data[..]).unwrap();
        td += t.elapsed();
        let t = Instant::now();
        let b = m.bounds();
        let mut rgba = RGBA::new(b);
        draw::draw(&mut rgba, b, m.as_ref(), b.min, Op::Src);
        tc += t.elapsed();
        let t = Instant::now();
        let mut out = Vec::new();
        jpeg::encode(&mut out, &rgba, Some(&jpeg::Options { quality: 75 })).unwrap();
        te += t.elapsed();
        total += out.len();
    }
    println!(
        "decode {:?} convert {:?} encode {:?} bytes {}",
        td, tc, te, total
    );
}
