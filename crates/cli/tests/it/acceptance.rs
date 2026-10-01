//! The acceptance harness from a test: `tools/neohugo/compare.sh <label> --ref golden` with the
//! test's `neohugo-rs` binary against the committed golden data of the Go build, and the parsed
//! `structdiff.json` (REWRITE_PLAN.md §7.2, §7.3). The Go binaries are not needed.
//!
//! The sites' asset pipelines need the node tools (`tools/neohugo/node.sh`,
//! `NEOHUGO_NODE_MODULES`) and esbuild (`NEOHUGO_ESBUILD_BINARY`, or `tools/esbuild/bin/esbuild`
//! of the main checkout); without them, or without `python3`, `bash` and `node`, [`compare`]
//! prints `SKIPPED` and returns `None`.

use std::path::{Path, PathBuf};
use std::process::Command;

use neohugo_testkit::fixture::repo_dir;

/// `program --version` runs.
fn runs(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The main checkout of the repository (a worktree's shared git directory's parent), where
/// `tools/esbuild/install.sh` puts its binary.
fn main_checkout(repo: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()?;
    let dir = String::from_utf8(out.stdout).ok()?;
    Path::new(dir.trim()).parent().map(Path::to_path_buf)
}

/// The node modules (with every one of `bins` in `.bin`) and the esbuild binary, or `None`
/// (printing `SKIPPED <gate>`).
fn tools(gate: &str, repo: &Path, bins: &[&str]) -> Option<(PathBuf, PathBuf)> {
    let skip = |why: String| {
        eprintln!("SKIPPED {gate}: {why}");
        None
    };
    if !runs("python3") || !runs("bash") || !runs("node") {
        return skip("python3, bash and node are needed on PATH".to_owned());
    }
    let node_modules = match std::env::var_os("NEOHUGO_NODE_MODULES") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let Ok(out) = Command::new(repo.join("tools/neohugo/node.sh"))
                .arg("path")
                .output()
            else {
                return skip("tools/neohugo/node.sh does not run".to_owned());
            };
            PathBuf::from(String::from_utf8_lossy(&out.stdout).trim())
        }
    };
    if let Some(bin) = bins
        .iter()
        .find(|b| !node_modules.join(".bin").join(b).exists())
    {
        return skip(format!(
            "no {bin} in {} (set NEOHUGO_NODE_MODULES or run tools/neohugo/node.sh)",
            node_modules.display()
        ));
    }
    let esbuild = std::env::var_os("NEOHUGO_ESBUILD_BINARY")
        .map(PathBuf::from)
        .or_else(|| main_checkout(repo).map(|m| m.join("tools/esbuild/bin/esbuild")))
        .unwrap_or_default();
    if !esbuild.is_file() {
        return skip(format!(
            "no esbuild binary at {} (set NEOHUGO_ESBUILD_BINARY or run tools/neohugo/node.sh && tools/esbuild/install.sh)",
            esbuild.display()
        ));
    }
    Some((node_modules, esbuild))
}

/// Runs `compare.sh <label> --ref golden` (ratchet against `testdata/baselines/<label>.json`)
/// and returns its `structdiff.json`; `None` when the tools are missing (`SKIPPED <gate>`).
///
/// # Panics
/// When compare.sh fails (an unlisted difference, or a build error), or its output is missing.
pub fn compare(gate: &str, label: &str, node_bins: &[&str]) -> Option<serde_json::Value> {
    let repo = repo_dir().canonicalize().expect("repository root");
    let (node_modules, esbuild) = tools(gate, &repo, node_bins)?;
    let work = tempfile::tempdir().expect("tempdir");
    let out = Command::new("bash")
        .arg(repo.join("tools/neohugo/compare.sh"))
        .args([label, "--ref", "golden", "--show", "20"])
        .env("NEOHUGO_RS", env!("CARGO_BIN_EXE_neohugo-rs"))
        .env("NEOHUGO_COMPARE_WORK", work.path())
        .env("NEOHUGO_NODE_MODULES", &node_modules)
        .env("NEOHUGO_ESBUILD_BINARY", &esbuild)
        .env_remove("NEOHUGO_TASK")
        .env_remove("KEEP")
        .output()
        .expect("run compare.sh");
    let report = String::from_utf8_lossy(&out.stdout);
    let log = String::from_utf8_lossy(&out.stderr);
    println!("{report}");
    assert!(
        out.status.success(),
        "compare.sh failed (an unlisted difference, or a build error):\n{report}\n{log}"
    );
    let json = std::fs::read_to_string(work.path().join(label).join("structdiff.json"))
        .expect("structdiff.json");
    let d: serde_json::Value = serde_json::from_str(&json).expect("structdiff.json parses");
    assert_eq!(d["ref"], "golden");
    assert_eq!(d["ratchet"]["verdict"], "pass");
    Some(d)
}

/// L1 of both passes: `files` matched, none missing or extra.
pub fn assert_l1(d: &serde_json::Value, files: u64) {
    for pass in ["minified", "unminified"] {
        let p = &d["passes"][pass];
        assert_eq!(
            (
                p["matched"].as_u64(),
                p["missing"].as_u64(),
                p["extra"].as_u64()
            ),
            (Some(files), Some(0), Some(0)),
            "L1 {pass}: {p}"
        );
    }
}

/// Every compared file or fact of each of `levels` is equal.
pub fn assert_equal(d: &serde_json::Value, levels: &[&str]) {
    for level in levels {
        let s = &d["summary"][level];
        assert_eq!(
            s["ok"], s["compared"],
            "{level} must be equal everywhere: {s}"
        );
    }
}
