//! External tools: a missing tool is an error naming the binary; `security.exec.allow` is
//! enforced; tools run in the project directory with fugo's environment (`FUGO_*`), not the Go
//! program's.

use ssg_resources::pipes::{BabelOptions, TailwindOptions};
use ssg_resources::{PipeError, ResourceError, Transform};

use super::{fake_tool, have_node, mini_site, project};

fn pipe_error(e: &ResourceError) -> &PipeError {
    match e {
        ResourceError::Pipe { source, .. } => source,
        other => panic!("not a pipe error: {other}"),
    }
}

#[test]
fn a_missing_tool_is_an_error_naming_the_binary() {
    // Programs of the tools' names on PATH do not count: the tools come from package.json.
    let on_path = tempfile::tempdir().unwrap();
    fake_tool(on_path.path(), "tailwindcss", "");
    fake_tool(on_path.path(), "babel", "");
    let path = on_path.path().join(".bin").display().to_string();
    let site = mini_site(&[
        (
            "config.toml",
            "baseURL = \"https://example.org/\"\n[security.exec]\nallow = ['^tailwindcss$', '^babel$']\n",
        ),
        ("assets/css/a.css", ".a { color: red; }\n"),
        ("assets/js/a.js", "export const a = 1;\n"),
    ]);
    let p = project(site.path(), |env| {
        env.os_env.retain(|(k, _)| k != "PATH");
        env.os_env.push(("PATH".into(), path));
    });
    let css = p.asset("css/a.css");
    let js = p.asset("js/a.js");
    for (src, t, binary, npm_package) in [
        (
            css,
            Transform::TailwindCss(TailwindOptions::default()),
            "tailwindcss",
            "@tailwindcss/cli",
        ),
        (
            js,
            Transform::Babel(BabelOptions::default()),
            "babel",
            "@babel/cli",
        ),
    ] {
        let id = p.store.transform(src, t).unwrap();
        let err = p.store.realize(id).unwrap_err();
        match pipe_error(&err) {
            PipeError::ToolNotFound {
                tool,
                package,
                searched,
            } => {
                assert_eq!(*tool, binary);
                assert_eq!(*package, npm_package);
                assert!(
                    searched.contains(&format!("node_modules/.bin/{binary}")),
                    "{searched}"
                );
            }
            other => panic!("{binary}: {other}"),
        }
        let msg = err.to_string();
        assert!(
            msg.contains(&format!("the {binary} binary was not found")),
            "{msg}"
        );
        assert!(
            msg.contains(&format!(
                "add {npm_package} to the devDependencies of package.json"
            )),
            "{msg}"
        );
        // Content and publishing report the same error.
        assert!(p.store.content(id).is_err());
    }
}

#[test]
fn exec_allow_is_enforced() {
    // Babel is not in Go's default allow list; a site must allow it.
    let site = mini_site(&[
        ("config.toml", "baseURL = \"https://example.org/\"\n"),
        ("assets/js/a.js", "export const a = 1;\n"),
    ]);
    let p = project(site.path(), |_| {});
    let id = p
        .store
        .transform(
            p.asset("js/a.js"),
            Transform::Babel(BabelOptions::default()),
        )
        .unwrap();
    let err = p.store.realize(id).unwrap_err();
    assert!(
        matches!(pipe_error(&err), PipeError::ExecDenied { tool: "babel" }),
        "{err}"
    );
}

/// The Go program's environment variables for tools (recorded names), which this port does not
/// set.
const GO_TOOL_VARS: [&str; 5] = [
    "HUGO_ENVIRONMENT",
    "HUGO_ENV",
    "HUGO_PUBLISHDIR",
    "HUGO_FILE_PACKAGE_JSON",
    "HUGO_FILE_TAILWIND_CONFIG_JS",
];

#[test]
fn tools_get_fugo_environment() {
    if !have_node("tools_get_fugo_environment") {
        return;
    }
    let site = mini_site(&[
        ("config.toml", "baseURL = \"https://example.org/\"\n"),
        ("package.json", "{}\n"),
        ("tailwind.config.js", "module.exports = {};\n"),
        ("assets/css/a.css", ".a { color: red; }\n"),
    ]);
    let tmp = tempfile::tempdir().unwrap();
    let keys: Vec<&str> = [
        "NODE_PATH",
        "PWD",
        "FUGO_ENVIRONMENT",
        "FUGO_PUBLISHDIR",
        "FUGO_FILE_PACKAGE_JSON",
        "FUGO_FILE_TAILWIND_CONFIG_JS",
        "SECRET_TOKEN",
    ]
    .into_iter()
    .chain(GO_TOOL_VARS)
    .collect();
    let script = format!(
        "\nconst keys = {};\n{}",
        serde_json::to_string(&keys).unwrap(),
        r"require('fs').readFileSync(0);
process.stdout.write(JSON.stringify({cwd: process.cwd(), env: Object.fromEntries(keys.map((k) => [k, process.env[k] || null]))}));
"
    );
    let extra = tmp.path().join("tools-node-modules");
    fake_tool(&extra, "tailwindcss", &script);
    let missing = tmp.path().join("missing-node-modules");
    let p = project(site.path(), |env| {
        env.tools.node_modules = vec![missing.clone(), extra.clone()];
        env.os_env.push(("SECRET_TOKEN".into(), "x".into()));
        env.environment = "staging".into();
    });
    let id = p
        .store
        .transform(
            p.asset("css/a.css"),
            Transform::TailwindCss(TailwindOptions::default()),
        )
        .unwrap();
    let out: serde_json::Value = serde_json::from_slice(&p.store.content(id).unwrap()).unwrap();
    let dir = p.dir.display().to_string();
    assert_eq!(out["cwd"], dir.as_str());
    let env = &out["env"];
    // Only existing directories (Tailwind 4 reads NODE_PATH as one directory): the project has
    // no node_modules and one tools directory is missing.
    assert_eq!(env["NODE_PATH"], extra.display().to_string().as_str());
    assert_eq!(env["PWD"], dir.as_str());
    // The environment is not passed on (only the flag chooses it; no variable names it).
    assert_eq!(env["FUGO_ENVIRONMENT"], serde_json::Value::Null);
    assert_eq!(env["FUGO_PUBLISHDIR"], format!("{dir}/public").as_str());
    assert_eq!(
        env["FUGO_FILE_PACKAGE_JSON"],
        format!("{dir}/package.json").as_str()
    );
    assert_eq!(
        env["FUGO_FILE_TAILWIND_CONFIG_JS"],
        format!("{dir}/tailwind.config.js").as_str()
    );
    // The Go program's names are not set.
    for k in GO_TOOL_VARS {
        assert_eq!(env[k], serde_json::Value::Null, "{k}");
    }
    assert_eq!(
        env["SECRET_TOKEN"],
        serde_json::Value::Null,
        "osEnv filters the environment"
    );
}
