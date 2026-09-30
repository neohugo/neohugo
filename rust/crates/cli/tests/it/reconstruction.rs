//! Gate A-R (REWRITE_PLAN.md §7.3, T62): the seeksnack reconstruction built by the
//! `neohugo-rs` binary against the committed golden data of the Go build
//! (`rust/testdata/golden/seeksnack/`, T01), through the acceptance harness:
//! `tools/neohugo/compare.sh seeksnack --ref golden` (`sites.py make seeksnack --overlay
//! rust/sites/seeksnack`, both passes, `structdiff.py` and the ratchet of
//! `rust/testdata/baselines/seeksnack.json`).
//!
//! The gate: L1 713/713 in both passes, the structure oracle (records, aliases, pagers, pages
//! and resource URLs) without a difference, L2 on every file, A7 ≥ 0.95 and a clean ratchet
//! (every remaining difference is a baseline entry with the same fingerprint).
//!
//! The site's PostCSS step and `js_build` need the node tools (`tools/neohugo/node.sh`,
//! `NEOHUGO_NODE_MODULES`) and esbuild (`NEOHUGO_ESBUILD_BINARY`, or
//! `tools/esbuild/bin/esbuild` of the main checkout); without them, or without `python3` and
//! `bash`, the test prints `SKIPPED` and passes. The Go binaries are not needed.

use std::path::{Path, PathBuf};
use std::process::Command;

use neohugo_testkit::fixture::rust_dir;

/// `program --version` runs.
fn runs(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The main checkout of the repository (a worktree's shared git directory's parent), where
/// `tools/esbuild/build.sh` puts its binary.
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

/// The node modules and the esbuild binary, or `None` (printing `SKIPPED`).
fn tools(repo: &Path) -> Option<(PathBuf, PathBuf)> {
    let skip = |why: String| {
        eprintln!("SKIPPED gate_a_r: {why}");
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
    if !node_modules.join(".bin/postcss").exists() {
        return skip(format!(
            "no postcss in {} (set NEOHUGO_NODE_MODULES or run tools/neohugo/node.sh)",
            node_modules.display()
        ));
    }
    let esbuild = std::env::var_os("NEOHUGO_ESBUILD_BINARY")
        .map(PathBuf::from)
        .or_else(|| main_checkout(repo).map(|m| m.join("tools/esbuild/bin/esbuild")))
        .unwrap_or_default();
    if !esbuild.is_file() {
        return skip(format!(
            "no esbuild binary at {} (set NEOHUGO_ESBUILD_BINARY or run tools/esbuild/build.sh)",
            esbuild.display()
        ));
    }
    Some((node_modules, esbuild))
}

#[test]
fn gate_a_r() {
    let repo = rust_dir()
        .join("..")
        .canonicalize()
        .expect("repository root");
    let Some((node_modules, esbuild)) = tools(&repo) else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let out = Command::new("bash")
        .arg(repo.join("tools/neohugo/compare.sh"))
        .args(["seeksnack", "--ref", "golden", "--show", "20"])
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

    let json = std::fs::read_to_string(work.path().join("seeksnack/structdiff.json"))
        .expect("structdiff.json");
    let d: serde_json::Value = serde_json::from_str(&json).expect("structdiff.json parses");
    assert_eq!(d["ref"], "golden");
    assert_eq!(d["ratchet"]["verdict"], "pass");
    for pass in ["minified", "unminified"] {
        let p = &d["passes"][pass];
        assert_eq!(
            (
                p["matched"].as_u64(),
                p["missing"].as_u64(),
                p["extra"].as_u64()
            ),
            (Some(713), Some(0), Some(0)),
            "L1 {pass}: {p}"
        );
    }
    let summary = &d["summary"];
    for level in ["L1", "L2", "L4", "S"] {
        let s = &summary[level];
        assert_eq!(
            s["ok"], s["compared"],
            "{level} must be equal everywhere: {s}"
        );
    }
    let a7 = d["a7"]["ratio"].as_f64().expect("A7 ratio");
    assert!(a7 >= 0.95, "A7 {a7} < 0.95");
}
