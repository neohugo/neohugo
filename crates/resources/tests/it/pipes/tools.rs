//! External tools: a missing tool is an error naming the binary; `security.exec.allow` is
//! enforced; tools run in the project directory with Hugo's environment.

use neohugo_resources::pipes::{BabelOptions, PostCssOptions, TailwindOptions};
use neohugo_resources::{PipeError, ResourceError, Transform};

use super::{fake_tool, have_node, mini_site, project};

fn pipe_error(e: &ResourceError) -> &PipeError {
    match e {
        ResourceError::Pipe { source, .. } => source,
        other => panic!("not a pipe error: {other}"),
    }
}

#[test]
fn a_missing_tool_is_an_error_naming_the_binary() {
    let site = mini_site(&[
        (
            "neohugo.toml",
            "baseURL = \"https://example.org/\"\n[security.exec]\nallow = ['^postcss$', '^tailwindcss$', '^babel$']\n",
        ),
        ("assets/css/a.css", ".a { color: red; }\n"),
        ("assets/js/a.js", "export const a = 1;\n"),
    ]);
    let p = project(site.path(), |env| env.os_env.retain(|(k, _)| k != "PATH"));
    let css = p.asset("css/a.css");
    let js = p.asset("js/a.js");
    for (src, t, binary, var) in [
        (
            css,
            Transform::PostCss(PostCssOptions::default()),
            "postcss",
            "NEOHUGO_POSTCSS_BIN",
        ),
        (
            css,
            Transform::TailwindCss(TailwindOptions::default()),
            "tailwindcss",
            "NEOHUGO_TAILWINDCSS_BIN",
        ),
        (
            js,
            Transform::Babel(BabelOptions::default()),
            "babel",
            "NEOHUGO_BABEL_BIN",
        ),
    ] {
        let id = p.store.transform(src, t).unwrap();
        let err = p.store.realize(id).unwrap_err();
        match pipe_error(&err) {
            PipeError::ToolNotFound {
                tool,
                env,
                searched,
            } => {
                assert_eq!(*tool, binary);
                assert_eq!(*env, var);
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
        // Content and publishing report the same error.
        assert!(p.store.content(id).is_err());
    }
}

#[test]
fn exec_allow_is_enforced() {
    // Babel is not in Hugo's default allow list; a site must allow it.
    let site = mini_site(&[
        ("neohugo.toml", "baseURL = \"https://example.org/\"\n"),
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

#[test]
fn tools_get_hugo_environment() {
    if !have_node("tools_get_hugo_environment") {
        return;
    }
    let site = mini_site(&[
        ("neohugo.toml", "baseURL = \"https://example.org/\"\n"),
        ("package.json", "{}\n"),
        ("postcss.config.js", "module.exports = {};\n"),
        ("assets/css/a.css", ".a { color: red; }\n"),
    ]);
    let tmp = tempfile::tempdir().unwrap();
    let script = r"
const keys = ['NODE_PATH', 'PWD', 'NEOHUGO_ENVIRONMENT', 'NEOHUGO_PUBLISHDIR', 'NEOHUGO_FILE_PACKAGE_JSON', 'NEOHUGO_FILE_POSTCSS_CONFIG_JS', 'HUGO_ENVIRONMENT', 'HUGO_ENV', 'HUGO_PUBLISHDIR', 'HUGO_FILE_PACKAGE_JSON', 'HUGO_FILE_POSTCSS_CONFIG_JS', 'SECRET_TOKEN'];
require('fs').readFileSync(0);
process.stdout.write(JSON.stringify({cwd: process.cwd(), env: Object.fromEntries(keys.map((k) => [k, process.env[k] || null]))}));
";
    let bin = fake_tool(tmp.path(), "postcss", script);
    let extra = tmp.path().join("tools-node-modules");
    std::fs::create_dir(&extra).unwrap();
    let missing = tmp.path().join("missing-node-modules");
    let p = project(site.path(), |env| {
        env.tools.postcss = Some(bin);
        env.tools.node_modules = vec![missing.clone(), extra.clone()];
        env.os_env.push(("SECRET_TOKEN".into(), "x".into()));
        env.environment = "staging".into();
    });
    let id = p
        .store
        .transform(
            p.asset("css/a.css"),
            Transform::PostCss(PostCssOptions::default()),
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
    assert_eq!(env["NEOHUGO_ENVIRONMENT"], "staging");
    assert_eq!(env["NEOHUGO_PUBLISHDIR"], format!("{dir}/public").as_str());
    assert_eq!(
        env["NEOHUGO_FILE_PACKAGE_JSON"],
        format!("{dir}/package.json").as_str()
    );
    assert_eq!(
        env["NEOHUGO_FILE_POSTCSS_CONFIG_JS"],
        format!("{dir}/postcss.config.js").as_str()
    );
    // Hugo's names are not set.
    for k in [
        "HUGO_ENVIRONMENT",
        "HUGO_ENV",
        "HUGO_PUBLISHDIR",
        "HUGO_FILE_PACKAGE_JSON",
        "HUGO_FILE_POSTCSS_CONFIG_JS",
    ] {
        assert_eq!(env[k], serde_json::Value::Null, "{k}");
    }
    assert_eq!(
        env["SECRET_TOKEN"],
        serde_json::Value::Null,
        "osEnv filters the environment"
    );
}
