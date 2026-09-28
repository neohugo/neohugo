//! `neohugo-rs` binary entry point. Go: main.go:main

use nh_commands::commandeer::{ExecOptions, execute_with};

fn main() {
    let opts = ExecOptions {
        func_map_factory: nh_commands::funcmap::production_func_map_factory(),
        ..Default::default()
    };
    // Go takes the arguments as bytes; an argument that is not UTF-8 is converted lossily.
    let args = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    std::process::exit(execute_with(args, opts));
}
