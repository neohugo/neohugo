//! The command line itself: `version`, `--help`, usage errors, `config`.

use crate::{neohugo, site_from, stderr, stdout};

#[test]
fn version_help_and_usage_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let o = neohugo(dir.path(), &["version"], &[]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(
        stdout(&o),
        format!("neohugo-rs {}\n", env!("CARGO_PKG_VERSION"))
    );
    let o = neohugo(dir.path(), &["--version"], &[]);
    assert_eq!(o.status.code(), Some(0));
    let o = neohugo(dir.path(), &["--help"], &[]);
    assert_eq!(o.status.code(), Some(0));
    for cmd in ["build", "templates", "config", "version"] {
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
