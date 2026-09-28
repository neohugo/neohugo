//! Differential test of the host namespaces (Wave B task T19: css, data, debug, diagrams, hugo,
//! images, inflect, js, lang, openapi3, os, page, partials, path, site, strings, templates, time,
//! transform, urls) and of `tplimplinit::create_func_map` against
//! `tools/go-oracle/nh-tplfuncs/host` (linux/arm64 Go under qemu; see PORTING.md).
//!
//! The sites of the oracle are built with nh-hugolib (process + assemble + freeze, the real func
//! map), every case calls the namespace method or func map function with the recorded arguments
//! (pages of the site, nested calls) in the recorded order, and its typed result or error text
//! must be Go's. The functions the port stubs must fail with their explicit `neohugo-rs: ... is
//! not supported` error ([`STUBS`]); `js.Build` needs the pinned esbuild
//! (`NEOHUGO_ESBUILD_BINARY`, like T16's tests) and is skipped without it.

mod host_support;
mod support;

use std::collections::{BTreeMap, BTreeSet};

use host_support::*;
use serde_json::{Value as J, json};

/// Functions whose Go result the port replaces with an explicit unsupported error (HUGO_LAYER.md
/// §1 rule 5), with the reason.
const STUBS: &[(&str, &str)] = &[
    ("transform.Highlight", "Chroma"),
    ("transform.CanHighlight", "Chroma lexers"),
    ("transform.ToMath", "KaTeX"),
    ("transform.PortableText", "goportabletext"),
    ("transform.Emojify", "the kyokomi emoji table (nh-helpers)"),
    ("f:emojify", "the kyokomi emoji table (nh-helpers)"),
    ("diagrams.Goat", "bep/goat"),
    ("openapi3.Unmarshal", "kin-openapi"),
    ("f:getJSON", "data.GetJSON"),
    ("f:getCSV", "data.GetCSV"),
    ("images.QR", "rsc.io/qr"),
    ("images.Text", "x/image/font (nh-images)"),
    (
        "images.Dither",
        "makeworld-the-better-one/dither (nh-images)",
    ),
    ("js.Batch", "the JS batcher"),
    ("images.Config", "GIF decoding (nh-images)"),
    ("f:imageConfig", "GIF decoding (nh-images)"),
    // nh-parser's metadecoders: CSV and XML decoding, YAML/TOML/XML encoding.
    ("transform.Unmarshal", "CSV/XML decoding (nh-parser)"),
    ("f:unmarshal", "CSV/XML decoding (nh-parser)"),
    ("transform.Remarshal", "YAML/TOML/XML encoding (nh-parser)"),
];

fn is_stub(m: &str) -> bool {
    STUBS.iter().any(|(s, _)| *s == m)
}

/// Results the oracle encodes only by type (the printed form of a Go struct with pointers).
fn same_other_type(got: &J, want: &J) -> bool {
    let (g, w) = (&got["ok"]["t"], &want["ok"]["t"]);
    w.as_str().is_some_and(|t| t.starts_with("other:")) && g == w
}

/// Differences that follow from other crates (PORTING.md "Known divergences"): the reason, or
/// `None`.
fn known(m: &str, c: &J, want: &J) -> Option<&'static str> {
    let args = c["a"].to_string();
    // Invalid UTF-8 input to the functions whose helpers take `&str` (nh-config's
    // `AllProvider::create_title`, nh-helpers' `PathSpec::abs_url`/`rel_url`): the input is
    // converted lossily.
    if args.contains("\"hex\"")
        && matches!(
            m,
            "strings.Title"
                | "urls.AbsURL"
                | "urls.RelURL"
                | "urls.AbsLangURL"
                | "urls.RelLangURL"
                | "urls.URLize"
                | "urls.Anchorize"
        )
    {
        return Some("invalid UTF-8 through a &str helper");
    }
    // nh-hugofs' `afero::read_file` returns the plain I/O error of reading a directory; Go's is
    // the `*PathError` `read <path>: is a directory`.
    if m == "os.ReadFile"
        && want["err"]
            .as_str()
            .is_some_and(|e| e.starts_with("read ") && e.ends_with(": is a directory"))
    {
        return Some("nh-hugofs read_file error of a directory");
    }
    // Go's Unmarshal cache key of a string ignores the decoder options: a CSV document cached
    // by an earlier call (with a delimiter) is returned to later calls; the port's CSV decoding
    // is nh-parser's stub, so nothing was cached.
    if matches!(m, "transform.Unmarshal" | "f:unmarshal")
        && want["ok"]["t"].as_str() == Some("[][]string")
    {
        return Some("CSV decoding (nh-parser stub) via the Unmarshal cache");
    }
    None
}

/// The methods that need the esbuild binary.
fn needs_esbuild(c: &J) -> bool {
    c["m"].as_str() == Some("js.Build")
}

#[test]
fn host_namespaces() {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

fn run() {
    let sites_fx = fixture("sites.json.gz");
    // Hugo's --clock (the oracle set it before building its sites: page dates in the future of
    // the clock are not built).
    let clock = go_time::parse(go_time::RFC3339, sites_fx["clock"].as_str().unwrap()).unwrap();
    nh_common::htime::set_clock(std::sync::Arc::new(nh_common::htime::StartClock::new(
        &clock,
    )));
    let sites = build_sites(&sites_fx);
    let esbuild = std::env::var("NEOHUGO_ESBUILD_BINARY").is_ok_and(|b| !b.is_empty());
    if !esbuild {
        eprintln!(
            "SKIPPED: js.Build cases: NEOHUGO_ESBUILD_BINARY is not set (tools/esbuild/build.sh builds the pinned esbuild 0.25.6)"
        );
    }

    let mut total = 0;
    let mut failures: Vec<String> = Vec::new();
    let mut stubbed: BTreeMap<String, usize> = BTreeMap::new();
    let mut known_diffs: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut skipped = 0;
    for topic in topics() {
        let fx = fixture(&format!("{topic}.json.gz"));
        if topic == "funcnames" {
            check_func_names(&fx, &sites);
            continue;
        }
        let mut n = 0;
        for c in fx["cases"].as_array().unwrap() {
            let env = &sites.envs[c["env"].as_str().unwrap()];
            if needs_esbuild(c) && !esbuild {
                skipped += 1;
                continue;
            }
            total += 1;
            n += 1;
            let ctx = case_ctx(env, c);
            let m = c["m"].as_str().unwrap();
            let r = invoke(env, ctx.as_host(), m, c["a"].as_array().unwrap());
            let got = result(env, &r);
            let want = &c["r"];
            if topic == "time_now" {
                check_now(&got, &clock, &mut failures);
                continue;
            }
            if &got == want || same_other_type(&got, want) {
                continue;
            }
            if let Err(e) = &r
                && e.message().starts_with("neohugo-rs: ")
                && is_stub(m)
            {
                *stubbed.entry(m.to_string()).or_default() += 1;
                continue;
            }
            if let Some(why) = known(m, c, want) {
                *known_diffs.entry(why).or_default() += 1;
                continue;
            }
            failures.push(format!(
                "{topic} {} {m}({}){}:\n  want {want}\n  got  {got}",
                c["env"].as_str().unwrap(),
                c["a"],
                c.get("ctx")
                    .map(|x| format!(" ctx={x}"))
                    .unwrap_or_default()
            ));
        }
        eprintln!("host/{topic}: {n} cases");
    }
    // What the site builds and the cases logged (REF_NOT_FOUND errors, i18n warnings, the
    // partial prefix warning, deprecations).
    for (entry, (name, log)) in sites_fx["logs"].as_array().unwrap().iter().zip(&sites.logs) {
        assert_eq!(entry["site"].as_str(), Some(name.as_str()));
        let dir = sites.envs[&format!("{name}/0")].dir.clone();
        let got = String::from_utf8_lossy(&log.lock().unwrap()).replace(&dir, "/SITE");
        // Known divergence (nh-config's `deprecate`): Go logs deprecations with the command field
        // "deprecated" (`ERROR deprecated: ...`); the port logs the message without it.
        let want = support::gostr(&entry["log"])
            .to_str_lossy()
            .replace("ERROR deprecated: ", "ERROR ")
            .replace("WARN  deprecated: ", "WARN  ")
            .replace("INFO  deprecated: ", "INFO  ");
        let (got, want) = (log_lines(&got), log_lines(&want));
        if got != want {
            failures.push(format!("log of {name}:\n  want {want:#?}\n  got  {got:#?}"));
        }
        // The errors the cases sent to the error handler.
        let errs: Vec<String> = sites.envs[&format!("{name}/0")]
            .d
            .global_err_handler
            .stop_error_collector()
            .iter()
            .map(|e| e.to_string().replace(&dir, "/SITE"))
            .collect();
        let want_errs: Vec<String> = entry["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| support::gostr(e).to_str_lossy().into_owned())
            // (The js.Build cases did not run without esbuild.)
            .filter(|e| esbuild || !e.starts_with("JSBUILD: "))
            .collect();
        if errs != want_errs {
            failures.push(format!(
                "errors of {name}:\n  want {want_errs:#?}\n  got  {errs:#?}"
            ));
        }
    }
    eprintln!(
        "host: {total} cases, {} differences, {skipped} skipped, stubbed: {stubbed:?}, known: {known_diffs:?}",
        failures.len()
    );
    if let Ok(p) = std::env::var("NH_T19_DUMP") {
        std::fs::write(p, failures.join("\n")).unwrap();
    }
    if !failures.is_empty() {
        let shown: Vec<_> = failures.iter().take(60).cloned().collect();
        panic!(
            "{} of {total} cases differ from Go:\n{}",
            failures.len(),
            shown.join("\n")
        );
    }
}

/// The lines of a log, sorted (the order of the messages of one site is the order of the cases,
/// but the oracle's generators that ran natively (css) log after the others).
fn log_lines(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s
        .split('\n')
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    v.sort();
    v
}

/// The func map of every site has exactly Go's names (and T20's `funcnames` fixture).
fn check_func_names(fx: &J, sites: &Sites) {
    let want: BTreeSet<String> = fx["cases"][0]["names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap().to_string())
        .collect();
    for (name, env) in &sites.envs {
        let got: BTreeSet<String> = env.func_map.keys().cloned().collect();
        assert_eq!(got, want, "{name}: func map names");
    }
    let t20 = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../nh-hugolib/tests/fixtures/funcnames/funcnames.json.gz");
    if let Ok(f) = std::fs::File::open(&t20) {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(f), &mut s).unwrap();
        let j: J = serde_json::from_str(&s).unwrap();
        let t20: BTreeSet<String> = j["funcNames"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_str().unwrap().to_string())
            .collect();
        assert_eq!(t20, want, "T20's funcnames fixture");
    }
    eprintln!("host/funcnames: {} names", want.len());
}

/// `time.Now` under the clock: a `time.Time` less than an hour after the clock's start.
fn check_now(got: &J, clock: &go_value::Time, failures: &mut Vec<String>) {
    use go_time::GoTimeExt;
    let ok = &got["ok"];
    let unix = ok["unix"].as_i64();
    match unix {
        Some(u)
            if ok["t"] == json!("time.Time")
                && u >= clock.go_unix()
                && u < clock.go_unix() + 3600 => {}
        _ => failures.push(format!("time.Now: {got}")),
    }
}
