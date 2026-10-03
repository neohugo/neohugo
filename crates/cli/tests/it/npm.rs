//! The `npm` feature through the binary: a build installs the packages of `package.json`
//! (from the registry of `.npmrc`, a local one here) and the Tailwind pipe runs its package
//! with the embedded JavaScript runtime, without Node.js on `PATH`; the server installs again
//! when `package.json` changes.

use std::path::Path;

use serde_json::json;
use ssg_testkit::registry::{Package, Registry};

use crate::server::Running;
use crate::{binary, site_from, stderr, stdout};

/// A `PATH` without Node.js.
const NO_NODE: (&str, &str) = ("PATH", "/usr/bin:/bin");

const SITE: &str = "-- config.toml --\nbaseURL = \"https://example.org/\"\ntitle = \"Npm\"\n\
disableKinds = [\"taxonomy\", \"term\", \"sitemap\", \"rss\", \"robotsTXT\"]\n\
-- layouts/home.html --\n<html><head>\
{%- set css = get_asset(path=\"css/main.css\") | tailwind | fingerprint -%}\
<link rel=\"stylesheet\" href=\"{{ css.rel_permalink }}\"></head><body></body></html>\n\
-- assets/css/main.css --\n.a { color: red; }\n";

/// A stand-in for `@tailwindcss/cli`: a CommonJS program that writes a banner (from its
/// dependency, with its arguments) and the CSS of its standard input.
fn packages() -> Vec<Package> {
    vec![
        Package::new("banner", "1.0.0")
            .field("main", json!("index.js"))
            .file(
                "index.js",
                "module.exports = (args) => `/* fake tailwind ${args.join(' ')} */\\n`;\n",
            ),
        Package::new("@tailwindcss/cli", "4.1.0")
            .field("bin", json!({ "tailwindcss": "dist/index.js" }))
            .field("dependencies", json!({ "banner": "^1.0.0" }))
            .file(
                "dist/index.js",
                "#!/usr/bin/env node\nconst banner = require('banner');\n\
                 const css = require('node:fs').readFileSync(0, 'utf8');\n\
                 process.stdout.write(banner(process.argv.slice(2)) + css);\n",
            ),
        Package::new("extra", "1.0.0").file("index.js", "module.exports = 1;\n"),
    ]
}

fn write_project(dir: &Path, registry: &str, dependencies: &serde_json::Value) {
    let package_json = json!({ "private": true, "devDependencies": dependencies });
    std::fs::write(
        dir.join("package.json"),
        serde_json::to_vec_pretty(&package_json).expect("json"),
    )
    .expect("package.json");
    std::fs::write(dir.join(".npmrc"), format!("registry={registry}\n")).expect(".npmrc");
}

/// The published stylesheet (`css/main.<hash>.css`).
fn stylesheet(site: &Path) -> String {
    let dir = site.join("public/css");
    let file = std::fs::read_dir(&dir)
        .expect("public/css")
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("main."))
        .expect("main.*.css");
    std::fs::read_to_string(file.path()).expect("read css")
}

#[test]
fn a_build_installs_package_json_and_runs_tailwind_without_node() {
    let registry = Registry::start(&packages());
    let s = site_from(SITE);
    write_project(
        s.path(),
        registry.url(),
        &json!({ "@tailwindcss/cli": "^4.1.0" }),
    );

    let out = binary(s.path(), &["build"], &[NO_NODE]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("Installed the npm packages of package.json"),
        "{}",
        stdout(&out)
    );
    let css = stylesheet(s.path());
    assert!(
        css.starts_with("/* fake tailwind --input=- --cwd "),
        "{css}"
    );
    assert!(css.contains(".a { color: red; }"), "{css}");
    assert!(s.path().join("npm.lock").is_file());

    // Installed already: nothing fetched, nothing printed.
    let requests = registry.requests().len();
    let again = binary(s.path(), &["build"], &[NO_NODE]);
    assert!(again.status.success(), "{}", stderr(&again));
    assert!(!stdout(&again).contains("Installed"), "{}", stdout(&again));
    assert_eq!(registry.requests().len(), requests);
}

#[test]
fn a_registry_that_cannot_be_reached_fails_the_build() {
    let s = site_from(SITE);
    // Port 9 (discard) on the loopback address refuses connections.
    write_project(
        s.path(),
        "http://127.0.0.1:9/",
        &json!({ "@tailwindcss/cli": "^4.1.0" }),
    );
    let out = binary(s.path(), &["build"], &[NO_NODE]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("installing the npm packages of "), "{err}");
    assert!(err.contains("@tailwindcss/cli"), "{err}");
}

#[test]
fn the_server_installs_again_when_package_json_changes() {
    let registry = Registry::start(&packages());
    let s = site_from(SITE);
    write_project(
        s.path(),
        registry.url(),
        &json!({ "@tailwindcss/cli": "^4.1.0" }),
    );
    let run = Running::start(s.path(), &["server", "-p", "0"]);
    let started = run.until("Web Server is available at ").join("\n");
    assert!(started.contains("Installed the npm packages"), "{started}");
    assert!(!s.path().join("node_modules/extra").exists());

    write_project(
        s.path(),
        registry.url(),
        &json!({ "@tailwindcss/cli": "^4.1.0", "extra": "^1.0.0" }),
    );
    run.until("Installed the npm packages");
    assert!(s.path().join("node_modules/extra/index.js").is_file());
}
