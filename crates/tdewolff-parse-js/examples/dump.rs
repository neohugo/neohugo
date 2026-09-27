//! Prints one serialization of one file, like the Go oracle's `dump` command
//! (`tdewolff-parse-js dump MODE FILE`), or times the parser (`time FILE`).
//!
//!     cargo run --release --example dump -- parsew2f file.js

#[path = "../tests/common/mod.rs"]
mod common;

use std::io::Write;

use tdewolff_parse::{GoBytes, Input};
use tdewolff_parse_js::Options;
use tdewolff_parse_js::dump::{lex_dump, parse_dump, string_dump};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: dump lex|lexre|parse|parsew2f|parseinline|string|js|json|time FILE");
        std::process::exit(2);
    }
    let src = std::fs::read(&args[2]).unwrap();
    let mode = args[1].clone();
    let out = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(move || match mode.as_str() {
            "lex" => lex_dump(&src, false),
            "lexre" => lex_dump(&src, true),
            "parse" => parse_dump(&src, Options::default()),
            "parsew2f" => parse_dump(
                &src,
                Options {
                    while_to_for: true,
                    inline: false,
                },
            ),
            "parseinline" => parse_dump(
                &src,
                Options {
                    while_to_for: true,
                    inline: true,
                },
            ),
            "string" => string_dump(&src, Options::default()),
            "js" | "json" => common::run_mode(&mode, &src),
            "time" => {
                let t = std::time::Instant::now();
                let n = 5;
                for _ in 0..n {
                    let input = Input::new_bytes(GoBytes::from_slice(&src));
                    let ast = tdewolff_parse_js::parse(
                        &input,
                        Options {
                            while_to_for: true,
                            inline: false,
                        },
                    );
                    assert!(ast.is_ok());
                }
                format!("{:?} per parse\n", t.elapsed() / n).into_bytes()
            }
            m => panic!("unknown mode {}", m),
        })
        .unwrap()
        .join()
        .unwrap();
    std::io::stdout().write_all(&out).unwrap();
}
