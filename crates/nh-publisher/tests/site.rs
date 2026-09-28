//! The publisher against real Go builds (`tools/go-oracle/nh-publisher/site`): every `Publish`
//! call recorded from in-process builds of this repository's docs/ site and hugolib/testsite in
//! twelve config variants (canonifyURLs / relativeURLs, a baseURL with and without a path,
//! minify off / default / customised, buildStats configs) is replayed through
//! `DestinationPublisher::publish` onto an in-memory fs; the written bytes must equal the Go
//! publisher's (absURL + minify chain), and the collector's elements Go's `hugo_stats.json`
//! elements of the same HTML files.
//!
//! The checked-in fixtures hold every testsite record and a subset of each docs build (its
//! non-HTML outputs and 16 HTML pages). `site_full_matches_go` (ignored) replays the complete
//! records from `NH_T07_SITE_FULL` (the oracle's `site-full-arm64` directory) and compares the
//! elements with each build's `hugo_stats.json`.

mod common;

use std::path::Path;
use std::sync::atomic::AtomicU64;

use nh_config::common_config::BuildStats;
use nh_hugofs::afero::{new_mem_map_fs, read_file};
use nh_media::media::media_type::{MediaType, Types};
use nh_media::output::output_format::{Formats, OutputFormat};
use nh_publisher::html_elements_collector::HtmlElements;
use nh_publisher::publisher::{Descriptor, DestinationPublisher, Publisher};
use nh_transform::minifiers::config::MinifyConfig;
use nh_transform::minifiers::minifiers::Client;
use serde_json::Value as J;
use sha2::{Digest, Sha256};

fn media_type(t: &str) -> MediaType {
    let mut m = MediaType::default();
    m.typ = t.to_string();
    m
}

fn conf_from_dump(d: &J) -> MinifyConfig {
    let b = |v: &J| v.as_bool().unwrap();
    let i = |v: &J| v.as_i64().unwrap();
    let mut c = MinifyConfig {
        minify_output: b(&d["MinifyOutput"]),
        disable_html: b(&d["DisableHTML"]),
        disable_css: b(&d["DisableCSS"]),
        disable_js: b(&d["DisableJS"]),
        disable_json: b(&d["DisableJSON"]),
        disable_svg: b(&d["DisableSVG"]),
        disable_xml: b(&d["DisableXML"]),
        ..Default::default()
    };
    let t = &mut c.tdewolff;
    let h = &d["HTML"];
    t.html.keep_comments = b(&h["KeepComments"]);
    t.html.keep_conditional_comments = b(&h["KeepConditionalComments"]);
    t.html.keep_special_comments = b(&h["KeepSpecialComments"]);
    t.html.keep_default_attr_vals = b(&h["KeepDefaultAttrVals"]);
    t.html.keep_document_tags = b(&h["KeepDocumentTags"]);
    t.html.keep_end_tags = b(&h["KeepEndTags"]);
    t.html.keep_quotes = b(&h["KeepQuotes"]);
    t.html.keep_whitespace = b(&h["KeepWhitespace"]);
    t.html.template_delims = [
        h["TemplateDelims"][0].as_str().unwrap().to_string(),
        h["TemplateDelims"][1].as_str().unwrap().to_string(),
    ];
    t.css.keep_css2 = b(&d["CSS"]["KeepCSS2"]);
    t.css.precision = i(&d["CSS"]["Precision"]);
    t.css.inline = b(&d["CSS"]["Inline"]);
    t.js.precision = i(&d["JS"]["Precision"]);
    t.js.keep_var_names = b(&d["JS"]["KeepVarNames"]);
    t.js.version = i(&d["JS"]["Version"]);
    t.json.precision = i(&d["JSON"]["Precision"]);
    t.json.keep_numbers = b(&d["JSON"]["KeepNumbers"]);
    t.svg.keep_comments = b(&d["SVG"]["KeepComments"]);
    t.svg.precision = i(&d["SVG"]["Precision"]);
    t.svg.inline = b(&d["SVG"]["Inline"]);
    t.xml.keep_whitespace = b(&d["XML"]["KeepWhitespace"]);
    c
}

fn list(v: &J) -> Option<Vec<String>> {
    match v {
        J::Null => None,
        J::Array(a) => Some(a.iter().map(|x| x.as_str().unwrap().to_string()).collect()),
        _ => panic!("list {v}"),
    }
}

fn elements(v: &J) -> HtmlElements {
    // Go's encoding of publisher.HTMLElements: "tags", "classes", "ids".
    HtmlElements {
        tags: list(&v["tags"]),
        classes: list(&v["classes"]),
        ids: list(&v["ids"]),
    }
}

/// Replays one variant's records; returns (publishes checked, failures).
fn replay(path: &Path, full: bool) -> (usize, Vec<String>) {
    let recs = common::read_jsonl_gz(path);
    let br = &recs[0];
    assert_eq!(br["t"], "build");
    let variant = br["variant"].as_str().unwrap().to_string();

    let mut types = Types(Vec::new());
    for t in br["types"].as_array().unwrap() {
        let sufs: Vec<&str> = t[1]
            .as_array()
            .map(|a| a.iter().map(|s| s.as_str().unwrap()).collect())
            .unwrap_or_default();
        types
            .0
            .push(MediaType::from_string_and_ext(t[0].as_str().unwrap(), &sufs).unwrap());
    }
    let formats = Formats(
        br["formats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| OutputFormat {
                name: f[0].as_str().unwrap().to_string(),
                media_type: media_type(f[1].as_str().unwrap()),
                is_html: f[2].as_bool().unwrap(),
                ..Default::default()
            })
            .collect(),
    );
    let conf = conf_from_dump(&br["minify"]);
    let client = Client::from_config(&types, &formats, &conf).unwrap();
    let bs = br["buildStats"].as_array().unwrap();
    let build_stats = BuildStats {
        enable: bs[0].as_bool().unwrap(),
        disable_tags: bs[1].as_bool().unwrap(),
        disable_classes: bs[2].as_bool().unwrap(),
        disable_ids: bs[3].as_bool().unwrap(),
    };

    let fs = new_mem_map_fs();
    let publisher = DestinationPublisher::new(fs.clone(), client, &build_stats);
    let counter = AtomicU64::new(0);

    let mut failures = Vec::new();
    let mut n = 0;
    let mut published = 0;
    for r in &recs[1..] {
        n += 1;
        let input = common::bytes(&r["in"]);
        let target = r["path"].as_str().unwrap().to_string();
        let d = Descriptor {
            src: &input,
            output_format: OutputFormat {
                name: r["format"].as_str().unwrap().to_string(),
                media_type: media_type(r["mt"].as_str().unwrap()),
                is_html: r["html"].as_bool().unwrap(),
                ..Default::default()
            },
            target_path: target.clone(),
            stat_counter: Some(&counter),
            live_reload_base_url: None,
            add_hugo_generator_tag: r["gen"].as_bool().unwrap(),
            abs_url_path: r["abs"].as_str().unwrap().to_string(),
            minify: false,
        };
        if let Err(e) = publisher.publish(d) {
            failures.push(format!("{variant} {target}: publish: {e}"));
            continue;
        }
        published += 1;
        let written = read_file(fs.as_ref(), &target).unwrap_or_default();
        let ok = match (common::opt_bytes(r, "out"), r.get("sha")) {
            (Some(want), _) if full => written == want,
            (_, Some(sha)) => hex(&Sha256::digest(&written)) == sha.as_str().unwrap(),
            (Some(want), None) => written == want,
            _ => false,
        };
        if !ok {
            failures.push(format!(
                "{variant} {target} ({}, abs {:?}): written {} bytes differ; in {}",
                r["mt"],
                r["abs"],
                written.len(),
                common::show(&input[..input.len().min(300)])
            ));
        }
    }
    assert_eq!(
        counter.load(std::sync::atomic::Ordering::SeqCst) as usize,
        published
    );

    let got = publisher.publish_stats().html_elements;
    if build_stats.enabled() {
        let want = elements(&br["elements"]);
        if got != want {
            failures.push(format!(
                "{variant}: elements differ:\n got {got:?}\nwant {want:?}"
            ));
        }
        if full && let Some(stats) = br["stats"].as_str() {
            let stats: J = serde_json::from_str(stats).unwrap();
            let want = elements(&stats["htmlElements"]);
            if got != want {
                failures.push(format!("{variant}: hugo_stats.json elements differ"));
            }
        }
    } else {
        assert_eq!(got, HtmlElements::default());
    }
    (n, failures)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn run_dir(dir: &Path, full: bool) {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".jsonl.gz"))
        .collect();
    files.sort();
    assert!(files.len() >= 12, "{} variants", files.len());
    let mut total = 0;
    let mut failures = Vec::new();
    for f in &files {
        let (n, fl) = replay(f, full);
        total += n;
        failures.extend(fl);
    }
    assert!(
        failures.is_empty(),
        "{} of {total} failed:\n{}",
        failures.len(),
        failures[..failures.len().min(10)].join("\n")
    );
    eprintln!("site: {total} publishes in {} variants", files.len());
}

/// Runs `f` on a thread with a large stack (the minifiers recurse like Go's).
fn big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn site_matches_go() {
    big_stack(|| run_dir(&common::fixture("site"), false));
}

#[test]
#[ignore = "needs the full records (NH_T07_SITE_FULL)"]
fn site_full_matches_go() {
    let dir = std::env::var("NH_T07_SITE_FULL").expect("NH_T07_SITE_FULL");
    big_stack(move || run_dir(Path::new(&dir), true));
}
