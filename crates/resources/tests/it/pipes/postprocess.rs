//! `post_process`: per-field placeholders filled in build phase E5.
//!
//! - Against the `transform` oracle's post-processed resources (`resources.PostProcess` of
//!   14 assets' `fingerprint` chains): content, links, integrity and media type after
//!   replacement equal Go's.
//! - The reconstruction's CSS chain (`to_css | post_css | minify | fingerprint |
//!   post_process`, head.html): nothing runs until the placeholders are resolved, so PostCSS
//!   reads the `neohugo_stats.json` written after rendering (E4). With a fake PostCSS that reports
//!   the stats it read; with postcss-cli (`NEOHUGO_POSTCSS_BIN`) the reconstruction's own
//!   purge configuration drops the unused rules.

use std::str::FromStr as _;

use neohugo_base::{Idx as _, LangIdx};
use neohugo_resources::pipes::{PostCssOptions, ToCssOptions};
use neohugo_resources::{Body, HashAlgo, PpField, Transform};
use serde_json::Value as J;

use super::{Project, fake_tool, have_node, mini_site, project, real_tool};
use crate::support::{MemSink, sha, store, synth_site};

/// Go's placeholders in `doc` and the values `replaced` holds for them, in order.
fn go_values(doc: &str, replaced: &str) -> Vec<(String, String)> {
    let mut literals = Vec::new();
    let mut placeholders = Vec::new();
    let mut rest = doc;
    while let Some(i) = rest.find("__h_pp_l1_") {
        let end = i + rest[i..].find("__e=").unwrap() + 4;
        literals.push(&rest[..i]);
        placeholders.push(rest[i..end].to_owned());
        rest = &rest[end..];
    }
    literals.push(rest);
    let mut out = Vec::new();
    let mut pos = literals[0].len();
    for (i, ph) in placeholders.into_iter().enumerate() {
        let next = literals[i + 1];
        let len = if next.is_empty() {
            replaced.len() - pos
        } else {
            replaced[pos..].find(next).unwrap()
        };
        out.push((ph, replaced[pos..pos + len].to_owned()));
        pos += len + next.len();
    }
    out
}

#[test]
fn post_process_oracle() {
    let fx: J = neohugo_testkit::fixture::oracle("oracle/resources/transform/transform.json.gz");
    let go = go_values(
        fx["doc"].as_str().unwrap(),
        fx["replaced"].as_str().unwrap(),
    );
    let home = tempfile::tempdir().unwrap();
    let store = store(&synth_site(home.path()), home.path());
    let lang = LangIdx::from_index(0);
    let mut compared = 0;
    let mut failures = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let Some(pp) = c.get("pp") else { continue };
        let chain: Vec<&str> = c["chain"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        if !chain.iter().all(|s| s.starts_with("fingerprint:")) {
            continue; // minify bytes differ (minify.rs); the tool chains fail.
        }
        // Go's placeholder prefix of this case: `__h_pp_l1_<n>_`.
        let content_ph = pp["content"].as_str().unwrap();
        let prefix = content_ph.trim_end_matches("Content__e=");
        let mut id = store
            .get_asset(lang, c["asset"].as_str().unwrap())
            .unwrap()
            .unwrap();
        for step in &chain {
            let algo = HashAlgo::from_str(step.trim_start_matches("fingerprint:")).unwrap();
            id = store.transform(id, Transform::Fingerprint(algo)).unwrap();
        }
        let ours = store.post_process(id);
        assert_eq!(store.post_process(id), ours, "one id per resource");
        for (field, go_field) in [
            (PpField::Content, "Content"),
            (PpField::RelPermalink, "RelPermalink"),
            (PpField::Permalink, "Permalink"),
            (PpField::Integrity, "Data.Integrity"),
            (PpField::MediaType, "MediaType.Type"),
        ] {
            let go_ph = format!("{prefix}{go_field}__e=");
            let Some((_, want)) = go.iter().find(|(ph, _)| *ph == go_ph) else {
                continue;
            };
            let text = format!("[{}]", ours.placeholder(field));
            let got = store.resolve_post_process(&text).unwrap().unwrap();
            if got != format!("[{want}]") {
                failures.push(format!(
                    "{} {chain:?} {go_field}: {got} want [{want}]",
                    c["asset"]
                ));
            }
            compared += 1;
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(
        compared,
        13 * 5,
        "13 post-processed fingerprint chains, 5 fields"
    );
    assert_eq!(store.resolve_post_process("no placeholder").unwrap(), None);
    assert_eq!(
        store
            .resolve_post_process("__nh_pp_999_content__ __nh_pp_1_nope__")
            .unwrap()
            .as_deref(),
        Some("__nh_pp_999_content__ __nh_pp_1_nope__"),
        "unknown placeholders stay"
    );
    eprintln!("post_process: {compared} fields equal to Go's after replacement");
}

/// The reconstruction's SCSS and PostCSS config (tools/rust-port/i01/seeksnack.txtar).
fn reconstruction(extra: &[(&str, &str)]) -> tempfile::TempDir {
    let txtar = std::fs::read_to_string(
        crate::support::repo_dir().join("tools/rust-port/i01/seeksnack.txtar"),
    )
    .unwrap();
    let mut files: Vec<(String, String)> = Vec::new();
    for line in txtar.split_inclusive('\n') {
        if let Some(name) = line
            .trim_end()
            .strip_prefix("-- ")
            .and_then(|r| r.strip_suffix(" --"))
        {
            files.push((name.to_owned(), String::new()));
        } else if let Some((_, body)) = files.last_mut() {
            body.push_str(line);
        }
    }
    let mut wanted: Vec<(&str, &str)> = files
        .iter()
        .filter(|(n, _)| n.starts_with("assets/scss/") || n == "postcss.config.js")
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect();
    wanted.push(("neohugo.toml", "baseURL = \"https://example.org/\"\n"));
    wanted.extend_from_slice(extra);
    mini_site(&wanted)
}

/// The head.html chain; returns the post-process placeholders of content, link, integrity.
fn head_chain(p: &Project) -> (neohugo_base::ResourceId, [String; 3]) {
    let s = &p.store;
    let src = p.asset("scss/website.scss");
    let css = s
        .transform(
            src,
            Transform::ToCss(
                ToCssOptions::from_json(&serde_json::json!({
                    "enableSourceMap": false, "includePaths": ["node_modules", "assets/scss"],
                    "outputStyle": "compressed"
                }))
                .unwrap(),
            ),
        )
        .unwrap();
    let post = s
        .transform(css, Transform::PostCss(PostCssOptions::default()))
        .unwrap();
    let min = s.transform(post, Transform::Minify).unwrap();
    let fp = s
        .transform(min, Transform::Fingerprint(HashAlgo::Sha256))
        .unwrap();
    let pp = s.post_process(fp);
    (
        fp,
        [PpField::Content, PpField::RelPermalink, PpField::Integrity].map(|f| pp.placeholder(f)),
    )
}

/// Renders the head, writes the stats (E4), resolves (E5), publishes by the resolved links.
fn render_and_resolve(p: &Project, stats: &str) -> (String, String, String, MemSink) {
    let (fp, [content, link, integrity]) = head_chain(p);
    let head = format!(
        "<style>{content}</style><link rel=\"preload\" href=\"{link}\" as=\"style\" integrity=\"{integrity}\">"
    );
    // Nothing ran yet: the fingerprint's record is provisional.
    assert!(matches!(p.store.resource(fp).body, Body::Pending));
    assert_eq!(p.tool_calls(), "");
    std::fs::write(p.dir.join("neohugo_stats.json"), stats).unwrap();
    let html = p.store.resolve_post_process(&head).unwrap().unwrap();
    let r = p.store.resource(fp);
    let body = String::from_utf8(p.store.content(fp).unwrap().to_vec()).unwrap();
    assert_eq!(
        html,
        format!(
            "<style>{body}</style><link rel=\"preload\" href=\"{}\" as=\"style\" integrity=\"{}\">",
            r.rel_permalink,
            r.integrity().unwrap()
        )
    );
    assert_eq!(
        r.rel_permalink,
        format!("/scss/website.min.{}.css", sha(body.as_bytes()))
    );
    let sink = MemSink::default();
    p.store.publish([r.rel_permalink.as_str()], &sink).unwrap();
    (html, body, r.rel_permalink.clone(), sink)
}

const STATS: &str =
    r#"{"htmlElements":{"tags":["body","li"],"classes":["card","pagination"],"ids":[]}}"#;

#[test]
fn post_process_reconstruction_chain_fake_postcss() {
    if !have_node("post_process_reconstruction_chain_fake_postcss") {
        return;
    }
    // Reads ./neohugo_stats.json like the purge plugin (fails without it) and reports it in a rule.
    let script = r"
const fs = require('fs'), path = require('path');
fs.appendFileSync(path.join(process.env.HOME, 'tool-calls.log'), 'postcss\n');
const css = fs.readFileSync(0, 'utf8');
const stats = JSON.parse(fs.readFileSync('./neohugo_stats.json', 'utf8')).htmlElements;
process.stdout.write(css + '.stats-seen{content:' + JSON.stringify(stats.classes.join(' ')) + '}');
";
    let site = reconstruction(&[]);
    let tmp = tempfile::tempdir().unwrap();
    let bin = fake_tool(tmp.path(), "postcss", script);
    let p = project(site.path(), |env| env.tools.postcss = Some(bin));
    let (_, body, link, sink) = render_and_resolve(&p, STATS);
    assert!(
        body.contains(r#".stats-seen{content:"card pagination"}"#),
        "{body}"
    );
    assert!(body.contains(".pagination li{padding:6px}"), "{body}");
    assert_eq!(p.tool_calls(), "postcss\n", "PostCSS ran once, in E5");
    let files = sink.0.into_inner().unwrap();
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [link.trim_start_matches('/')]
    );
}

#[test]
fn post_process_reconstruction_chain_real_postcss() {
    let Some(bin) = real_tool(
        "NEOHUGO_POSTCSS_BIN",
        "post_process_reconstruction_chain_real_postcss",
    ) else {
        return;
    };
    let site = reconstruction(&[]);
    let p = project(site.path(), |env| env.tools.postcss = Some(bin));
    let (_, body, _, _) = render_and_resolve(&p, STATS);
    // purge-lite keeps rules with a used class, drops the others, and appends the environment.
    assert!(body.contains(".card"), "{body}");
    assert!(body.contains(".pagination"), "{body}");
    assert!(!body.contains("never-used-class"), "{body}");
    assert!(!body.contains("card-unused"), "{body}");
}
