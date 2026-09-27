//! Profiling helper: decode + convert the files once, then encode them N times at q75.
use go_image::draw::{self, Op};
use go_image::{RGBA, jpeg};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = std::path::PathBuf::from(&args[1]);
    let list = std::fs::read_to_string(&args[2]).unwrap();
    let n: usize = args[3].parse().unwrap();
    let imgs: Vec<RGBA> = list
        .lines()
        .map(|l| {
            let d = std::fs::read(root.join(l)).unwrap();
            let m = jpeg::decode(&mut &d[..]).unwrap();
            let b = m.bounds();
            let mut rgba = RGBA::new(b);
            draw::draw(&mut rgba, b, m.as_ref(), b.min, Op::Src);
            rgba
        })
        .collect();
    let t = std::time::Instant::now();
    let mut k = 0usize;
    for _ in 0..n {
        for m in &imgs {
            let mut out = Vec::new();
            jpeg::encode(&mut out, m, Some(&jpeg::Options { quality: 75 })).unwrap();
            k += out.len();
        }
    }
    println!("{:?} {}", t.elapsed(), k);
}
