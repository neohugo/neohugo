//! `run_main` in a child process (this test binary, re-run as [`child`]): V8 wants the thread
//! that starts it, and a program's `process.exit()` ends the process.

use std::ffi::OsString;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::json;
use ssg_npm::ensure_installed;
use ssg_testkit::registry::Registry;

use crate::install::{packages, project};

/// The child's arguments (JSON), when this test binary is the child.
const CHILD_ARGS: &str = "SSG_NPM_TEST_CHILD_ARGS";

/// Not a test: the entry point of the child process.
#[test]
fn child() {
    let Some(args) = std::env::var_os(CHILD_ARGS) else {
        return;
    };
    let args: Vec<String> = serde_json::from_str(args.to_str().expect("UTF-8")).expect("args");
    let args: Vec<OsString> = args.into_iter().map(OsString::from).collect();
    std::process::exit(ssg_npm::run_main(&args));
}

struct Output {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs `<project>/node_modules/<package>`'s `bin` with `args` and `stdin`, in `project`.
fn run(project: &Path, package: &str, bin: &str, args: &[&str], stdin: &str) -> Output {
    let node_modules = project.join("node_modules");
    let mut all = vec![
        node_modules.display().to_string(),
        package.to_owned(),
        bin.to_owned(),
    ];
    all.extend(args.iter().map(|a| (*a).to_owned()));
    let mut child = Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", "run::child", "--nocapture", "--test-threads=1"])
        .env(CHILD_ARGS, serde_json::to_string(&all).expect("json"))
        .env("GREETING", "hello")
        .current_dir(project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    Output {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn installed() -> (Registry, tempfile::TempDir) {
    let registry = Registry::start(&packages());
    let p = project(
        &registry,
        &json!({ "devDependencies": { "greet": "^1.0.0", "@scope/esm": "^2.0.0" } }),
    );
    ensure_installed(p.path(), &p.path().join(".cache")).expect("install");
    (registry, p)
}

#[test]
fn a_commonjs_program_reads_stdin_and_its_environment() {
    let (_registry, p) = installed();
    let out = run(p.path(), "greet", "greet", &["world"], "  quiet  \n");
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    let name = p
        .path()
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    let expected = format!("QUIET!! world hello {name}");
    assert!(out.stdout.contains(&expected), "stdout: {}", out.stdout);
    assert_eq!(
        std::fs::read_to_string(p.path().join("out.txt")).expect("out.txt"),
        expected
    );
}

#[test]
fn an_es_module_program_imports_commonjs_and_sets_its_exit_code() {
    let (_registry, p) = installed();
    let out = run(p.path(), "@scope/esm", "esm", &[], "loud");
    assert_eq!(out.code, Some(3), "{}", out.stderr);
    assert_eq!(
        std::fs::read_to_string(p.path().join("out.txt")).expect("out.txt"),
        "LOUD!!"
    );
}

#[test]
fn an_uncaught_exception_fails_the_program() {
    let (_registry, p) = installed();
    let out = run(p.path(), "@scope/esm", "fail", &[], "");
    assert_eq!(out.code, Some(1));
    assert!(out.stderr.contains("the program failed"), "{}", out.stderr);
}

#[test]
fn a_missing_program_is_an_error() {
    let (_registry, p) = installed();
    let out = run(p.path(), "greet", "nope", &[], "");
    assert_eq!(out.code, Some(1));
    assert!(out.stderr.contains("nope"), "{}", out.stderr);
}
