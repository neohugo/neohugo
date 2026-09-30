//! `js.Build` against the `jsbuild` Go oracle (`testdata/oracle/resource-transformers/jsbuild`):
//! Hugo's `js.Build` with esbuild linked in, run on the `t16site` fixture site (62 cases) and on
//! the docs site's scripts (6 cases).
//!
//! A case gets an asset (or concatenates earlier cases' results), then optionally runs
//! `js.Build` and fingerprints. Scripts must be byte-identical (inline source maps are compared
//! decoded, as the oracle records them); the set of bundled modules (esbuild's `// <module>`
//! comments) is compared separately so a mismatch names what differs. Errors must be the same
//! kind of error at the same position with the same esbuild text.
//!
//! Deliberate difference: external/linked source maps name every bundled module in `sources`
//! (Hugo drops the modules resolved as assets, leaving `sources` shorter than
//! `sourcesContent`); the other map fields must be identical and Hugo's sources must be among
//! ours.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use neohugo_esbuild::{
    JsBuildError, JsBuildOptions, JsBuildOutput, JsBuilder, MountedDirs, OptionsError, Source,
};
use neohugo_testkit::fixture::oracle;
use serde_json::Value as Json;

/// What a case produced.
enum Outcome {
    Built(JsBuildOutput),
    /// No `js` step: the asset itself.
    Raw(Vec<u8>),
    Options(OptionsError),
    Build(JsBuildError),
}

struct Site {
    root: PathBuf,
    assets: MountedDirs,
}

/// The fixture's site: t16site is copied (its `_node_modules` becomes `node_modules`, mounted
/// at `assets/vendor` as its hugo.toml says); the docs site is used in place.
fn site(fixture_dir: &str) -> Site {
    if fixture_dir == "docs" {
        let root = crate::repo_root().join("docs").canonicalize().unwrap();
        let assets = MountedDirs::new().mount(root.join("assets"), "");
        return Site { root, assets };
    }
    let name = Path::new(fixture_dir).file_name().unwrap();
    let src = neohugo_testkit::fixture::testdata("oracle/resource-transformers").join(name);
    let root = crate::scratch("jsbuild-site")
        .canonicalize()
        .unwrap()
        .join("site");
    copy_tree(&src, &root);
    // T00's fixture conversion re-serialized the site's JSON sources compactly; the oracle ran
    // on the original text, which ends up in source maps. Restore it.
    for (file, text) in [
        (
            "assets/js/data.json",
            "{\"items\": [1, 2, 3], \"name\": \"data\"}\n",
        ),
        ("assets/js/data/config.json", "{\"mode\": \"test\"}\n"),
    ] {
        std::fs::write(root.join(file), text).unwrap();
    }
    let assets = MountedDirs::new()
        .mount(root.join("assets"), "")
        .mount(root.join("node_modules"), "vendor");
    Site { root, assets }
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let name = e.file_name();
        let target_name = if name == "_node_modules" {
            "node_modules".into()
        } else {
            name
        };
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &to.join(target_name));
        } else {
            std::fs::copy(e.path(), to.join(target_name)).unwrap();
        }
    }
}

fn media_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "js" => "text/javascript",
        "ts" => "text/typescript",
        "tsx" => "text/tsx",
        "jsx" => "text/jsx",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

fn run_case(
    builder: &JsBuilder,
    site: &Site,
    case: &Json,
    done: &BTreeMap<String, Vec<u8>>,
) -> Outcome {
    let steps = case["steps"].as_array().unwrap();
    let (path, contents) = match steps[0]["op"].as_str().unwrap() {
        "get" => {
            let path = steps[0]["path"].as_str().unwrap().to_owned();
            let file = site.root.join("assets").join(&path);
            let file = if file.exists() {
                file
            } else {
                site.root
                    .join("node_modules")
                    .join(path.strip_prefix("vendor/").unwrap())
            };
            (path, std::fs::read(file).unwrap())
        }
        "concat" => {
            // Hugo separates concatenated scripts with "\n;\n".
            let parts: Vec<&[u8]> = steps[0]["refs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| done[r.as_str().unwrap()].as_slice())
                .collect();
            (
                steps[0]["target"].as_str().unwrap().to_owned(),
                parts.join(&b"\n;\n"[..]),
            )
        }
        op => panic!("unknown op {op}"),
    };
    let Some(js) = steps.iter().find(|s| s["op"] == "js") else {
        return Outcome::Raw(contents);
    };
    let opts = match JsBuildOptions::from_json(&js["opts"]) {
        Ok(o) => o,
        Err(e) => return Outcome::Options(e),
    };
    let source = Source {
        path: &path,
        media_type: media_type(&path),
        contents: &contents,
    };
    match builder.build(&site.assets, &source, &opts) {
        Ok(out) => Outcome::Built(out),
        Err(e) => Outcome::Build(e),
    }
}

/// Replaces an inline source map's base64 with `DECODED(<map>)`, as the oracle records it.
fn decode_inline_map(code: &str) -> String {
    const MARK: &str = "sourceMappingURL=data:application/json;base64,";
    let Some(i) = code.find(MARK) else {
        return code.to_owned();
    };
    let start = i + MARK.len();
    let end = code[start..].find('\n').map_or(code.len(), |n| start + n);
    let decoded = String::from_utf8(base64_decode(&code[start..end])).unwrap();
    format!("{}DECODED({decoded}){}", &code[..start], &code[end..])
}

fn base64_decode(s: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes().filter(|&c| c != b'=') {
        let v = ALPHABET.iter().position(|&a| a == c).expect("base64");
        acc = (acc << 6) | u32::try_from(v).unwrap();
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((acc >> bits) & 0xFF).unwrap());
        }
    }
    out
}

/// esbuild's `// <module>` comments: the modules in the bundle.
fn modules(code: &str) -> BTreeSet<String> {
    code.lines()
        .filter_map(|l| l.trim_start().strip_prefix("// "))
        .filter(|m| {
            *m == "<stdin>"
                || m.starts_with("ns-hugo")
                || m.starts_with("node_modules/")
                || m.starts_with("../")
        })
        .map(str::to_owned)
        .collect()
}

/// The oracle's error text, split into Go's `"file:line:col": message` position (if any) and
/// its message.
fn expected_error(err: &str) -> (Option<(String, u32, u32)>, String) {
    let detail = err.split_once("): ").map_or(err, |(_, d)| d);
    if let Some(rest) = detail.strip_prefix('"')
        && let Some((pos, msg)) = rest.split_once("\": ")
    {
        let mut it = pos.rsplitn(3, ':');
        let col = it.next().unwrap().parse().unwrap();
        let line = it.next().unwrap().parse().unwrap();
        let file = it.next().unwrap().to_owned();
        return (Some((file, line, col)), msg.to_owned());
    }
    (None, detail.to_owned())
}

/// Checks an error outcome against the oracle's text; `Err` explains a mismatch.
fn check_error(outcome: &Outcome, want: &str, site: &str) -> Result<(), String> {
    let (pos, msg) = expected_error(&want.replace("$SITE", site));
    let quoted = msg.split('"').nth(1).map(str::to_owned).unwrap_or_default();
    let ok = match outcome {
        Outcome::Build(JsBuildError::Build(diags)) => {
            let d = &diags[0];
            let got_pos = d
                .position
                .as_ref()
                .map(|p| (p.file.to_string_lossy().into_owned(), p.line, p.column));
            got_pos == pos && d.text == msg
        }
        Outcome::Options(OptionsError::Value { value, .. }) => pos.is_none() && *value == quoted,
        Outcome::Options(OptionsError::Type { option, .. }) => {
            msg.contains("error(s) decoding") && msg.to_lowercase().contains(&format!("'{option}'"))
        }
        Outcome::Build(JsBuildError::InjectNotFound(p)) => {
            msg.starts_with("inject: file") && *p == quoted
        }
        Outcome::Build(JsBuildError::InjectAbsolute(_)) => msg.starts_with("inject: absolute"),
        Outcome::Build(JsBuildError::UnsupportedMediaType(m)) => {
            msg.starts_with("unsupported Media Type") && *m == quoted
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("want error {want:?}, got {}", describe(outcome)))
    }
}

fn describe(o: &Outcome) -> String {
    match o {
        Outcome::Built(out) => format!("output {:?}", String::from_utf8_lossy(&out.code)),
        Outcome::Raw(b) => format!("raw {:?}", String::from_utf8_lossy(b)),
        Outcome::Options(e) => format!("options error {e:?}"),
        Outcome::Build(e) => format!("build error {e:?}"),
    }
}

/// Compares an external/linked source map with the oracle's (see the module docs).
fn check_map(got: &[u8], want: &str) -> Result<(), String> {
    let got: Json = serde_json::from_slice(got).map_err(|e| e.to_string())?;
    let want: Json = serde_json::from_str(want).map_err(|e| e.to_string())?;
    for k in ["version", "sourcesContent", "mappings", "names"] {
        if got[k] != want[k] {
            return Err(format!("source map {k}: got {}, want {}", got[k], want[k]));
        }
    }
    let ours = got["sources"].as_array().cloned().unwrap_or_default();
    let content_len = got["sourcesContent"].as_array().map_or(0, Vec::len);
    if content_len > 0 && ours.len() != content_len {
        return Err(format!("{} sources for {content_len} contents", ours.len()));
    }
    for s in want["sources"].as_array().into_iter().flatten() {
        if !ours.contains(s) {
            return Err(format!("source {s} missing from {ours:?}"));
        }
    }
    Ok(())
}

fn check_case(case: &Json, want: &Json, outcome: &Outcome, site: &str) -> Result<(), String> {
    if let Some(err) = want["contentErr"].as_str() {
        return check_error(outcome, err, site);
    }
    let expected = want["content"].as_str().unwrap().replace("$SITE", site);
    let (code, out) = match outcome {
        Outcome::Built(out) => (String::from_utf8(out.code.clone()).unwrap(), Some(out)),
        Outcome::Raw(b) => (String::from_utf8(b.clone()).unwrap(), None),
        other => return Err(format!("want output, got {}", describe(other))),
    };
    let code = decode_inline_map(&code);
    let (got_modules, want_modules) = (modules(&code), modules(&expected));
    if got_modules != want_modules {
        return Err(format!(
            "modules: got {got_modules:?}, want {want_modules:?}"
        ));
    }
    if code != expected {
        return Err(format!(
            "output differs:\n--- got\n{code}\n--- want\n{expected}"
        ));
    }
    let fingerprinted = case["steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["op"] == "fingerprint");
    for (path, published) in want["published"].as_object().into_iter().flatten() {
        let Some(out) = out else { continue };
        let published = published.as_str().unwrap().replace("$SITE", site);
        if let Some(script) = path.strip_suffix(".map") {
            if script != out.target_path {
                return Err(format!("map at {path}, target {}", out.target_path));
            }
            let map = out.source_map.as_deref().ok_or("no source map")?;
            check_map(map, &published)?;
        } else if !fingerprinted && *path != out.target_path {
            return Err(format!("published at {path}, target {}", out.target_path));
        }
    }
    Ok(())
}

fn run_topic(topic: &str) {
    let test = format!("jsbuild_{topic}");
    let Some(service) = crate::service(&test) else {
        return;
    };
    let fx: Json = oracle(&format!(
        "oracle/resource-transformers/jsbuild/{topic}.json.gz"
    ));
    let site = site(fx["dir"].as_str().unwrap());
    let builder = JsBuilder::new(service, site.root.clone(), site.root.join("public"));
    let site_text = site.root.to_string_lossy().into_owned();

    let cases = fx["cases"].as_array().unwrap();
    let results = fx["results"].as_array().unwrap();
    assert_eq!(cases.len(), results.len());
    let mut done = BTreeMap::new();
    let mut failures = Vec::new();
    for (case, want) in cases.iter().zip(results) {
        let name = case["name"].as_str().unwrap();
        let outcome = run_case(&builder, &site, case, &done);
        match &outcome {
            Outcome::Built(out) => {
                done.insert(name.to_owned(), out.code.clone());
            }
            Outcome::Raw(b) => {
                done.insert(name.to_owned(), b.clone());
            }
            _ => {}
        }
        if let Err(e) = check_case(case, want, &outcome, &site_text) {
            failures.push(format!("{name}: {e}"));
        }
    }
    eprintln!(
        "jsbuild/{topic}: {}/{} cases match",
        cases.len() - failures.len(),
        cases.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn jsbuild_synth() {
    run_topic("synth");
}

#[test]
fn jsbuild_docs() {
    run_topic("docs");
}
