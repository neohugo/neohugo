//! The `neohugo` binary (see the `neohugo` library for the commands).

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use neohugo::{Cli, Exit};

fn main() -> ExitCode {
    match Cli::try_parse() {
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
