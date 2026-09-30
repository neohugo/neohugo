//! Integration tests of `neohugo` (the crate's single test binary, REWRITE_PLAN.md §2.2): the
//! `neohugo-rs` binary run on small sites.

mod build;
mod check;
mod cli;
mod embedded;
mod parity;
mod server;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use neohugo_testkit::txtar::Archive;

/// `neohugo-rs` with `args`, in `dir`, with an environment reduced to `PATH` plus `env`.
pub fn neohugo(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_neohugo-rs"));
    c.current_dir(dir).args(args).env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        c.env("PATH", path);
    }
    c.env("HOME", dir);
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().expect("run neohugo-rs")
}

pub fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

pub fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// `tests/it/<name>`.
pub fn fixture(name: &str) -> PathBuf {
    neohugo_testkit::fixture::rust_dir()
        .join("crates/cli/tests/it")
        .join(name)
}

/// The txtar site `tests/it/<name>` written into a new temporary directory.
pub fn site(name: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    Archive::read(&fixture(name))
        .expect("read txtar")
        .write_to(tmp.path())
        .expect("write site");
    tmp
}

/// A site from txtar text.
pub fn site_from(text: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    Archive::parse(text)
        .write_to(tmp.path())
        .expect("write site");
    tmp
}
