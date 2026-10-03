//! The binary (see the library for the commands).

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::process::ExitCode;

use clap::Parser;
use ssg_cli::{Cli, Exit};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    // `npm`: the child process that runs an npm package's program (the Tailwind and Babel
    // pipes start it), and the pipes' runner is this binary.
    #[cfg(feature = "npm")]
    {
        if args
            .get(1)
            .is_some_and(|a| *a == ssg_base::RUN_PACKAGE_COMMAND)
        {
            let code = ssg_npm::run_main(&args[2..]);
            return ExitCode::from(u8::try_from(code).unwrap_or(1));
        }
        if let Ok(exe) = std::env::current_exe() {
            ssg_resources::pipes::set_package_runner(exe);
        }
    }
    // Flags may come before the command, as in the Go build (`args::command_first`).
    match Cli::try_parse_from(ssg_cli::args::command_first(args)) {
        Ok(cli) => ssg_cli::run(cli).into(),
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
