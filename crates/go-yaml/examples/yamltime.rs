//! Timing helper: yaml.Unmarshal into interface{} for each file argument.
fn main() {
    for a in std::env::args().skip(1) {
        let b = std::fs::read(&a).unwrap();
        let t = std::time::Instant::now();
        let r = std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(move || go_yaml::unmarshal(&b).err().map(|e| e.message()))
            .unwrap()
            .join()
            .unwrap();
        println!("{a}: {:?} err={:?}", t.elapsed(), r);
    }
}
