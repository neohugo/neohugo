//! The command line itself: `version`, `--help`, usage errors, `config`.

use ::neohugo::version::BuildInfo;

use crate::{neohugo, site_from, stderr, stdout};

#[test]
fn version_help_and_usage_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let o = neohugo(dir.path(), &["version"], &[]);
    assert_eq!(o.status.code(), Some(0));
    let line = format!("{}\n", BuildInfo::CURRENT);
    assert_eq!(stdout(&o), line);
    let prefix = format!("neohugo v{}", env!("CARGO_PKG_VERSION"));
    assert!(line.starts_with(&prefix), "{line}");
    assert!(line.contains(" BuildDate="), "{line}");
    let o = neohugo(dir.path(), &["--version"], &[]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(stdout(&o), line);
    let o = neohugo(dir.path(), &["--help"], &[]);
    assert_eq!(o.status.code(), Some(0));
    for cmd in ["build", "server", "templates", "config", "version"] {
        assert!(stdout(&o).contains(cmd), "{cmd}: {}", stdout(&o));
    }
    let o = neohugo(dir.path(), &["templates", "check", "--help"], &[]);
    assert!(stdout(&o).contains("--coverage"), "{}", stdout(&o));
    for bad in [
        &["--nope"][..],
        &["build", "--clock", "yesterday"],
        &["templates"],
        &["templates", "check", "--coverage", "some"],
        &["config", "--format", "xml"],
    ] {
        let o = neohugo(dir.path(), bad, &[]);
        assert_eq!(o.status.code(), Some(2), "{bad:?}");
        assert!(stderr(&o).contains("error"), "{bad:?}: {}", stderr(&o));
    }
}

/// `version` prints Go's `BuildVersionString` (`common/neohugo/version.go` at 44529028).
#[test]
fn version_line_has_the_go_format() {
    let mut info = BuildInfo {
        version: "0.150.0",
        commit: None,
        os: "linux",
        arch: "amd64",
        date: None,
        vendor: None,
    };
    assert_eq!(
        info.to_string(),
        "neohugo v0.150.0 linux/amd64 BuildDate=unknown"
    );
    info.commit = Some("0123456789abcdef0123456789abcdef01234567");
    info.os = "darwin";
    info.arch = "arm64";
    info.date = Some("2026-10-01T12:34:56Z");
    info.vendor = Some("neohugo");
    assert_eq!(
        info.to_string(),
        "neohugo v0.150.0-0123456789abcdef0123456789abcdef01234567 darwin/arm64 \
         BuildDate=2026-10-01T12:34:56Z VendorInfo=neohugo"
    );

    // Go's GOOS/GOARCH names of the release targets.
    let current = BuildInfo::CURRENT;
    let expected = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(("linux", "amd64"))
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some(("linux", "arm64"))
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some(("darwin", "amd64"))
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(("darwin", "arm64"))
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some(("windows", "amd64"))
    } else {
        None
    };
    if let Some(os_arch) = expected {
        assert_eq!((current.os, current.arch), os_arch);
    }
    assert_eq!(current.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn config_prints_the_resolved_configuration() {
    let s = site_from(
        "-- hugo.toml --\nbaseURL = \"https://e.org/\"\ntitle = \"T\"\n-- config/production/params.toml --\ncolor = \"red\"\n",
    );
    let o = neohugo(s.path(), &["config"], &[("HUGO_PARAMS_SIZE", "9")]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).expect("json");
    assert_eq!(v["environment"], "production");
    let site = &v["sites"][0];
    assert_eq!(site["title"], "T", "{site}");
    let text = stdout(&o);
    assert!(text.contains("\"color\": \"red\""), "{text}");
    assert!(text.contains("\"size\": \"9\""), "{text}");

    let o = neohugo(
        s.path(),
        &["config", "-e", "dev", "--base-url", "https://b.org/"],
        &[],
    );
    let text = stdout(&o);
    assert!(text.contains("https://b.org/"), "{text}");
    assert!(!text.contains("\"color\""), "{text}");

    let o = neohugo(s.path(), &["config", "--format", "toml"], &[]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        stdout(&o).contains("environment = \"production\""),
        "{}",
        stdout(&o)
    );
}
