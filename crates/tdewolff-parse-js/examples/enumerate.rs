//! Checks an `enumerate` file of the Go oracle
//! (`tdewolff-parse-js enumerate OUT ALPHAHEX MAXLEN`): enumerates every
//! sequence of 1..MAXLEN symbols of the alphabet (hex bytes, or
//! comma-separated hex tokens, read from the file's header) in the same
//! order and compares the combined digest of all modes line by line;
//! differing inputs are printed (at most 50).
//!
//! ```sh
//! cargo run --release --example enumerate -- OUT.gz
//! ```

#[path = "../tests/common/mod.rs"]
mod common;

use common::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: enumerate OUT.gz");
        std::process::exit(2);
    }
    let path = std::path::PathBuf::from(&args[1]);
    // the dumps record Go's panics; keep the output readable
    std::panic::set_hook(Box::new(|_| {}));
    let (n, bad) = big_stack(move || check_enumerate(&path));
    for src in bad.iter().take(50) {
        println!("DIFF {:?}", String::from_utf8_lossy(src));
    }
    println!("{} inputs, {} differ", n, bad.len());
}
