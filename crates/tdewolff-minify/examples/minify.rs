//! `minify MEDIATYPE [CONFIG] < in > out` — minifies stdin like the Go
//! oracle's `stdin` mode (default config: seeksnack, no JS).

#[path = "../tests/common/configs.rs"]
mod configs;

use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mediatype = args.get(1).expect("usage: minify MEDIATYPE [CONFIG]");
    let cfg = args.get(2).map(|s| s.as_str()).unwrap_or("seeksnack");
    let m = configs::config(cfg);
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    let (out, err) = m.string(mediatype.as_bytes(), &input);
    std::io::stdout().write_all(&out).unwrap();
    if let Some(err) = err {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}
