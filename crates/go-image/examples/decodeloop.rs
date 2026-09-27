//! Profiling helper: decode the files in <listfile> (relative to <root>) N times.
use go_image::jpeg;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = std::path::PathBuf::from(&args[1]);
    let list = std::fs::read_to_string(&args[2]).unwrap();
    let n: usize = args[3].parse().unwrap();
    let datas: Vec<Vec<u8>> = list
        .lines()
        .map(|l| std::fs::read(root.join(l)).unwrap())
        .collect();
    let t = std::time::Instant::now();
    let mut k = 0usize;
    for _ in 0..n {
        for d in &datas {
            let m = jpeg::decode(&mut &d[..]).unwrap();
            k += m.bounds().dx() as usize;
        }
    }
    println!("{:?} {}", t.elapsed(), k);
}
