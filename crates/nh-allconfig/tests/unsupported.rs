//! Go module resolution (`go.mod` + module imports: `go list`, `go get`, `go mod download`)
//! is not ported: loading such a project fails with an explicit error instead of fetching.
//! Without a `go.mod`, a module-like import path is looked up in `themes/` like Go does.

mod support;

use serde_json::json;
use support::*;

#[test]
fn go_module_import_is_unsupported() {
    let c = json!({
        "name": "go-module-import",
        "environment": "production",
        "files": {
            "go.mod": "module example.com/site\n\ngo 1.22\n",
            "hugo.toml": "baseURL = \"https://example.org/\"\n[[module.imports]]\npath = \"github.com/foo/bar\"\n",
        },
    });
    let loaded = load_case(&c);
    let err = loaded.result.err().expect("error").to_string();
    assert!(
        err.contains("neohugo-rs: Go modules") && err.contains("is not supported"),
        "{err}"
    );
}

#[test]
fn module_path_without_go_mod_uses_themes_dir() {
    let c = json!({
        "name": "module-path-themes",
        "environment": "production",
        "files": {
            "hugo.toml": "baseURL = \"https://example.org/\"\n[[module.imports]]\npath = \"github.com/foo/bar\"\n",
            "themes/github.com/foo/bar/layouts/index.html": "home",
        },
    });
    let loaded = load_case(&c);
    let confs = loaded.result.as_ref().unwrap();
    let paths: Vec<String> = confs.modules.iter().map(|m| m.path().to_string()).collect();
    assert_eq!(paths, ["project", "github.com/foo/bar"]);

    // Missing: Go's "module does not exist" error text.
    let c = json!({
        "name": "module-path-missing",
        "environment": "production",
        "files": {
            "hugo.toml": "baseURL = \"https://example.org/\"\n[[module.imports]]\npath = \"github.com/foo/bar\"\n",
        },
    });
    let loaded = load_case(&c);
    let err = loaded.result.err().expect("error").to_string();
    assert!(
        err.contains("module \"github.com/foo/bar\" not found in")
            && err.contains(": module does not exist"),
        "{err}"
    );
}
