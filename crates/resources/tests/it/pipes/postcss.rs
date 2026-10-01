//! `post_css` against the `postcss` oracle (Hugo running postcss-cli with t16site's
//! dependency-free `postcss.config.js`, 15 cases).
//!
//! - With a real postcss-cli (`NEOHUGO_POSTCSS_BIN`), every case is compared with the oracle:
//!   contents exactly (cases through `to_css` normalised, see `tocss.rs`), errors by kind.
//! - Always (with `node`), the cases run with a fake `postcss` that applies the fixture
//!   config's declaration rewrite (`color: red` → `#f00`) and appends the comment the config
//!   appends (`env`, `node_env`, `cwd`, the `HUGO_FILE_*` variables); contents are compared
//!   with the oracle normalised, the comment exactly. That checks the argument list, the
//!   working directory, the environment (Hugo's variables set, `NODE_ENV` filtered out) and
//!   `@import` inlining against Go without postcss installed.

use std::collections::BTreeMap;
use std::path::Path;

use neohugo_resources::{PipeError, ResourceError};
use serde_json::Value as J;

use super::tocss::normalise;
use super::{Project, fake_tool, fixture_site, have_node, project, real_tool, run_steps};

/// The fake postcss: see the module documentation. Each call is logged to
/// `$HOME/tool-calls.log`.
pub const FAKE_POSTCSS: &str = r"
const fs = require('fs'), path = require('path');
const args = process.argv.slice(2);
fs.appendFileSync(path.join(process.env.HOME, 'tool-calls.log'), JSON.stringify({tool: 'postcss', args, cwd: process.cwd()}) + '\n');
let css = fs.readFileSync(0, 'utf8');
css = css.replace(/(color:\s*)red\b/g, '$1#f00');
const files = Object.keys(process.env).filter((k) => k.startsWith('HUGO_FILE_')).sort().join(',');
process.stdout.write(css + '/* env=' + process.env.HUGO_ENVIRONMENT + ' node_env=' +
  (process.env.NODE_ENV || 'development') + ' cwd=' + path.basename(process.cwd()) +
  ' files=' + files + ' */\n');
";

/// Cases the fake cannot run: postcss's own parse errors.
const REAL_ONLY: [&str; 2] = ["err-broken", "err-imports-broken"];

/// The trailing `/* env=… */` comment and the CSS before it.
fn split_comment(css: &str) -> (&str, &str) {
    match css.rfind("/* env=") {
        Some(i) => (&css[..i], css[i..].trim_end()),
        None => (css, ""),
    }
}

fn run(p: &Project, fx: &J, real: bool) -> (usize, Vec<String>) {
    let accepted = crate::support::expected_diffs()["postcss"]
        .as_table()
        .unwrap()
        .clone();
    let mut failures = Vec::new();
    let mut compared = 0;
    for (case, want) in fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fx["results"].as_array().unwrap())
    {
        let name = case["name"].as_str().unwrap();
        if !real && REAL_ONLY.contains(&name) {
            continue;
        }
        let via_sass = case["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["op"] == "tocss");
        let got = run_steps(p, case, &BTreeMap::new());
        match (got, want.get("contentErr").and_then(J::as_str)) {
            (Ok(id), None) => {
                let css = String::from_utf8(p.store.content(id).unwrap().to_vec()).unwrap();
                let w = want["content"].as_str().unwrap();
                let (wb, wc) = split_comment(w);
                let (gb, gc) = split_comment(&css);
                let same = if via_sass {
                    // grass output (see tocss.rs); the minified `final` case is checked there.
                    accepted.contains_key(name) || normalise(wb) == normalise(gb)
                } else if real {
                    w == css
                } else {
                    normalise(wb) == normalise(gb) && wc == gc
                };
                if same {
                    compared += 1;
                } else {
                    failures.push(format!("{name}:\n  want {w:?}\n  got  {css:?}"));
                }
                if let Some(link) = want.get("relPermalink").and_then(J::as_str)
                    && link != p.store.resource(id).rel_permalink
                    && !accepted.contains_key(name)
                {
                    failures.push(format!(
                        "{name}: link {}",
                        p.store.resource(id).rel_permalink
                    ));
                }
            }
            (Err((_, e)), Some(w)) => {
                let line = w
                    .split('"')
                    .find(|s| s.starts_with("$SITE/"))
                    .and_then(|s| s.rsplit(':').nth(1));
                match line {
                    Some(l) if !e.contains(&format!(":{l}")) && !accepted.contains_key(name) => {
                        failures.push(format!("{name}: {e}, want line {l}"));
                    }
                    _ => compared += 1,
                }
            }
            (Ok(_), Some(_)) if accepted.contains_key(name) => compared += 1,
            (Ok(_), Some(w)) => failures.push(format!("{name}: no error, Go: {w}")),
            (Err((_, e)), None) => failures.push(format!("{name}: {e}")),
        }
    }
    (compared, failures)
}

fn check(p: &Project, real: bool) {
    let fx: J =
        neohugo_testkit::fixture::oracle("oracle/resource-transformers/postcss/synth.json.gz");
    let (compared, failures) = run(p, &fx, real);
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    let total = if real { 15 } else { 15 - REAL_ONLY.len() };
    assert_eq!(compared, total);
    eprintln!(
        "postcss ({}): {compared} of 15 oracle cases",
        if real { "postcss-cli" } else { "fake postcss" }
    );
}

/// The t16site project with NODE_ENV set in the inherited environment (the tool must not see
/// it) and `postcss` at `bin`.
fn t16(bin: &Path) -> Project {
    let bin = bin.to_owned();
    project(&fixture_site("t16site"), move |env| {
        env.os_env.retain(|(k, _)| k != "NODE_ENV");
        env.os_env.push(("NODE_ENV".into(), "staging".into()));
        env.tools.postcss = Some(bin);
    })
}

#[test]
fn postcss_oracle_fake_tool() {
    if !have_node("postcss_oracle_fake_tool") {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let bin = fake_tool(tmp.path(), "postcss", FAKE_POSTCSS);
    let p = t16(&bin);
    check(&p, false);
    // The arguments Hugo passes: the resolved config file, then the switches.
    let log = p.tool_calls();
    let config = p.dir.join("postcss.config.js").display().to_string();
    let alt = p.dir.join("postcss-alt.config.js").display().to_string();
    let cwd = p.dir.display();
    assert!(
        log.contains(&format!(
            r#"{{"tool":"postcss","args":["--config","{config}"],"cwd":"{cwd}"}}"#
        )),
        "{log}"
    );
    assert!(
        log.contains(&format!(r#""args":["--config","{alt}"]"#)),
        "{log}"
    );
    assert!(
        log.contains(&format!(r#""args":["--config","{config}","--no-map"]"#)),
        "{log}"
    );
}

#[test]
fn postcss_oracle_real_tool() {
    let Some(bin) = real_tool("NEOHUGO_POSTCSS_BIN", "postcss_oracle_real_tool") else {
        return;
    };
    check(&t16(&bin), true);
}

#[test]
fn postcss_missing_config_is_an_error() {
    let p = project(&fixture_site("t16site"), |_| {});
    let src = p.asset("css/plain.css");
    let o = neohugo_resources::pipes::PostCssOptions::from_json(
        &serde_json::json!({"config": "nope.js"}),
    )
    .unwrap();
    let id = p
        .store
        .transform(src, neohugo_resources::Transform::PostCss(o))
        .unwrap();
    let err = p.store.realize(id).unwrap_err();
    assert!(
        matches!(&err, ResourceError::Pipe { source, .. } if matches!(**source, PipeError::ConfigNotFound { .. })),
        "{err}"
    );
}
