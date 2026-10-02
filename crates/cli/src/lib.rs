//! The command line (REWRITE_PLAN.md §2.1, §4.8, §7.5): `build` (also the command
//! line without a command), `server` (alias `serve`), `templates check`, `config` and
//! `version`.
//!
//! Exit codes ([`Exit`]): 0 success; 1 the build or the check failed (errors, printed with their
//! positions), or the project could not be loaded; 2 a usage error (clap).

#![forbid(unsafe_code)]

pub mod args;
mod build;
mod check;
mod config;
mod report;
mod server;
pub mod version;

use std::path::PathBuf;
use std::process::ExitCode;

use ssg_base::diag::Diagnostic;
use ssg_config::{CliOverrides, LoadOptions};

pub use args::Cli;
use args::{Command, ProjectArgs, TemplatesCommand};

/// The version (`[workspace.package]` of `Cargo.toml`): the `v<version>` of [`version::line`].
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How a command ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// 0.
    Success,
    /// 1: errors (or warnings with `--deny-warnings`), or a project that does not load.
    Failure,
    /// 2: the command line is invalid (clap's own exit code).
    Usage,
}

impl From<Exit> for ExitCode {
    fn from(e: Exit) -> Self {
        match e {
            Exit::Success => Self::SUCCESS,
            Exit::Failure => Self::from(1),
            Exit::Usage => Self::from(2),
        }
    }
}

/// Runs a parsed command line.
#[must_use]
pub fn run(cli: Cli) -> Exit {
    warn_ignored(&cli);
    let result = match cli.command {
        None => build::run(&cli.build),
        Some(Command::Build(b)) => build::run(&b),
        Some(Command::Server(s)) => server::run(&s),
        Some(Command::Templates(TemplatesCommand::Check(c))) => check::run(&c),
        Some(Command::Config(c)) => config::run(&c),
        Some(Command::Version) => {
            println!("{}", version::line());
            Ok(Exit::Success)
        }
    };
    result.unwrap_or_else(|e| {
        report::fatal(&e);
        Exit::Failure
    })
}

/// Warns about the Go build's flags given that this port accepts but does not act on
/// ([`args::HugoFlags::ignored`]).
fn warn_ignored(cli: &Cli) {
    let command = match &cli.command {
        Some(Command::Build(b)) => Some(&b.hugo),
        Some(Command::Server(s)) => Some(&s.build.hugo),
        _ => None,
    };
    let mut warnings: Vec<Diagnostic> = Vec::new();
    for message in std::iter::once(&cli.build.hugo)
        .chain(command)
        .flat_map(args::HugoFlags::ignored)
    {
        if !warnings.iter().any(|w| w.message == message) {
            warnings.push(Diagnostic::warning(message).with_id("ignored-flag"));
        }
    }
    report::diagnostics(&warnings);
}

impl ProjectArgs {
    /// The project directory: `--source`, else the working directory.
    fn source_dir(&self) -> anyhow::Result<PathBuf> {
        let cwd = std::env::current_dir()?;
        Ok(match &self.source {
            Some(s) => cwd.join(s),
            None => cwd,
        })
    }

    /// The configuration overrides of the flags (`destination` and `minify` are `build`'s).
    fn overrides(&self) -> CliOverrides {
        CliOverrides {
            base_url: self.base_url.clone(),
            environment: self.environment.clone(),
            build_drafts: self.include.build_drafts,
            build_future: self.include.build_future,
            build_expired: self.include.build_expired,
            cache_dir: self.cache_dir.clone(),
            themes_dir: self.themes_dir.clone(),
            theme: (!self.theme.is_empty()).then(|| self.theme.clone()),
            ignore_cache: self.ignore_cache,
            config_dir: self.config_dir.clone(),
            ..CliOverrides::default()
        }
    }

    /// What `ssg_config::load` needs, with the process's `FUGO_*` environment.
    fn load_options(&self) -> anyhow::Result<LoadOptions> {
        Ok(LoadOptions {
            source: self.source_dir()?,
            config_files: self.config.clone(),
            cli: self.overrides(),
            env: ssg_build::process_env(),
        })
    }
}
