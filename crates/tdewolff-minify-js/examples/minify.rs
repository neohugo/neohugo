//! Minifies stdin to stdout with an oracle configuration name (the Rust
//! twin of `tools/go-oracle/tdewolff-minify-js stdin CFG`):
//!
//! ```sh
//! cargo run --release --example minify -- v2022 < in.js
//! ```

#[path = "../tests/common/mod.rs"]
mod common;

use std::io::{Read, Write};

fn main() {
    let cfg = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "v2022".to_string());
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    let run = move || common::minify_buf(&cfg, common::cp(&input));
    // STACK_KB=n runs on a thread with an n KiB stack (to measure the depth
    // an input needs); the default is 1 GiB.
    let (out, err) = match std::env::var("STACK_KB") {
        Ok(kb) => std::thread::Builder::new()
            .stack_size(kb.parse::<usize>().unwrap() << 10)
            .spawn(run)
            .unwrap()
            .join()
            .unwrap(),
        Err(_) => common::with_big_stack(run),
    };
    std::io::stdout().write_all(&out).unwrap();
    if !err.is_empty() {
        eprintln!("\nerror: {}", String::from_utf8_lossy(&err));
        std::process::exit(1);
    }
}
