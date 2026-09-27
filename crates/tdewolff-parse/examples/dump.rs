//! Prints the serialized token/grammar stream of one file, byte for byte like
//! `tools/go-oracle/tdewolff-parse dump KIND FILE` (for diffing failures).
//!
//!   cargo run --example dump -- KIND FILE

#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: dump KIND FILE");
        std::process::exit(2);
    }
    let input = std::fs::read(&args[2]).expect("read input");
    print!("{}", common::stream_for(&args[1], &input));
}
