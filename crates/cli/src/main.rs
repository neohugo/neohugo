//! The `neohugo` binary (see the `neohugo` library for the commands).

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use neohugo::{Cli, Exit};

fn main() -> ExitCode {
    // Flags may come before the command, as in the Go build (`args::command_first`).
    match Cli::try_parse_from(neohugo::args::command_first(std::env::args_os().collect())) {
        Ok(cli) => neohugo::run(cli).into(),
        Err(e) => {
            // --help and --version print and succeed; usage errors exit with 2.
            let _ = e.print();
            if e.use_stderr() {
                Exit::Usage.into()
            } else {
                Exit::Success.into()
            }
        }
    }
}
