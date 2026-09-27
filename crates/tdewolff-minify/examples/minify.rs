//! `minify MEDIATYPE [CONFIG] < in > out` — minifies stdin like the Go
//! oracle's `stdin` mode (default config: seeksnack, no JS): `io.ReadAll`
//! then `M.Bytes`, which on error returns the input as the minifier left it
//! (possibly rewritten in place).
//!
//! Runs on a thread with a 1 GiB stack (virtual), like Go's maximum goroutine
//! stack, because nested minifier calls recurse (PORTING.md, "Deep
//! nesting"); `STACK_KB=n` uses an n KiB stack instead.

#[path = "../tests/common/configs.rs"]
mod configs;

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mediatype = args
        .get(1)
        .expect("usage: minify MEDIATYPE [CONFIG]")
        .clone();
    let cfg = args.get(2).cloned().unwrap_or_else(|| "seeksnack".into());
    let stack = std::env::var("STACK_KB")
        .map(|kb| kb.parse::<usize>().unwrap() << 10)
        .unwrap_or(1 << 30);
    let (out, err) = std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            let (input, err) =
                tdewolff_parse::read_all(&mut tdewolff_parse::IoReader(std::io::stdin()));
            if let Some(err) = err {
                panic!("{}", err);
            }
            let m = configs::config(&cfg);
            let (out, err) = m.bytes(mediatype.as_bytes(), input);
            (out.to_vec(), err.map(|e| e.to_string()))
        })
        .unwrap()
        .join()
        .unwrap();
    std::io::stdout().write_all(&out).unwrap();
    if let Some(err) = err {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}
