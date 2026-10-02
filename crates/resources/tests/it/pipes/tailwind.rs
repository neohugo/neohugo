//! `tailwind_css` on the docs site's `assets/css/styles.css` (used in place): the CLI runs in
//! the project directory with `--input=- --cwd <project>`; the asset `@import`s
//! (`components/all.css` and its `./content.css` …) are inlined from the assets view, while
//! `@import "tailwindcss"`, `@plugin` and `@source "hugo_stats.json"` (the frozen docs fixture names Go's file) are left to the CLI.
//!
//! With a fake `tailwindcss` (node) the input and arguments are checked; with the real CLI
//! (`NEOHUGO_TAILWINDCSS_BIN`, plugins through `NEOHUGO_NODE_MODULES`) the docs CSS compiles
//! and holds classes the docs' (Go's) `hugo_stats.json` names.

use neohugo_resources::Transform;
use neohugo_resources::pipes::{TailwindOptions, ToolPaths};

use super::{fake_tool, have_node, project_in_place, real_tool};

const FAKE_TAILWIND: &str = r"
const fs = require('fs'), path = require('path');
const args = process.argv.slice(2);
fs.appendFileSync(path.join(process.env.HOME, 'tool-calls.log'), JSON.stringify({tool: 'tailwindcss', args, cwd: process.cwd()}) + '\n');
process.stdout.write(fs.readFileSync(0, 'utf8'));
";

fn docs() -> std::path::PathBuf {
    crate::support::repo_dir().join("docs")
}

#[test]
fn tailwind_docs_styles_fake_tool() {
    if !have_node("tailwind_docs_styles_fake_tool") {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let bin = fake_tool(tmp.path(), "tailwindcss", FAKE_TAILWIND);
    let p = project_in_place(&docs(), |env| env.tools.tailwindcss = Some(bin));
    let src = p.asset("css/styles.css");
    let opts = TailwindOptions::from_json(&serde_json::json!({"minify": true})).unwrap();
    let id = p
        .store
        .transform(src, Transform::TailwindCss(opts))
        .unwrap();
    let css = String::from_utf8(p.store.content(id).unwrap().to_vec()).unwrap();
    // Left to the CLI.
    assert!(
        css.starts_with("@import \"tailwindcss\";\n@plugin \"@tailwindcss/typography\";\n"),
        "{css}"
    );
    assert!(css.contains("@source \"hugo_stats.json\";"), "{css}");
    assert!(!css.contains("@import \"components/all.css\""), "{css}");
    // Inlined: every component file of all.css.
    let components = docs().join("assets/css/components");
    for name in ["content.css", "fonts.css", "helpers.css", "shortcodes.css"] {
        let body = std::fs::read_to_string(components.join(name)).unwrap();
        let first = body
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with("@import"))
            .unwrap();
        assert!(css.contains(first), "{name}: {first}");
        assert!(!css.contains(&format!("@import \"./{name}\"")), "{name}");
    }
    let calls = p.tool_calls();
    let dir = p.dir.display();
    assert_eq!(
        calls.trim(),
        format!(
            r#"{{"tool":"tailwindcss","args":["--input=-","--cwd","{dir}","--minify"],"cwd":"{dir}"}}"#
        )
    );
    assert_eq!(p.store.resource(id).rel_permalink, "/css/styles.css");
}

#[test]
fn tailwind_docs_styles_real_tool() {
    let Some(bin) = real_tool("NEOHUGO_TAILWINDCSS_BIN", "tailwind_docs_styles_real_tool") else {
        return;
    };
    let p = project_in_place(&docs(), |env| {
        env.tools = ToolPaths {
            tailwindcss: Some(bin),
            ..ToolPaths::from_env()
        };
    });
    let src = p.asset("css/styles.css");
    let id = p
        .store
        .transform(src, Transform::TailwindCss(TailwindOptions::default()))
        .unwrap();
    let css = String::from_utf8(p.store.content(id).unwrap().to_vec()).unwrap();
    // Utilities the docs' hugo_stats.json (Go's) names, and the typography plugin's `prose`.
    assert!(css.contains(".prose"), "no typography plugin output");
    assert!(css.contains("--color-primary"), "no theme variables");
    assert!(!css.contains("@import"), "imports left");
    eprintln!("tailwind: docs styles.css compiled ({} bytes)", css.len());
}
