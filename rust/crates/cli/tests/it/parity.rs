//! Gate A-T (REWRITE_PLAN.md §7.3, T60): the testsite built by the `neohugo-rs` binary against
//! the Go build of the same site (`crates/build/tests/it/testsite-go.txtar`).
//!
//! The gate lives here, not in `neohugo-build`, because only this crate's tests can run the
//! binary (`CARGO_BIN_EXE_neohugo-rs`): the command line, the disk sink, the static copy and
//! `hugo_stats.json` in the project directory are part of what is compared.
//!
//! - **L1** paths: the file set, after §7.2's normalisation, equals Go's 55 files in `public`
//!   plus `hugo_stats.json` in the project directory (56; the reference holds the 55 of
//!   `public`, Go writes `hugo_stats.json` next to `hugo.toml`).
//! - **L2** links: per HTML file the `<title>`, `<link rel=canonical|alternate>` and the set of
//!   internal `href`/`src`/`srcset` URLs (percent-decoded); the alias → target map; the
//!   `<link>`/`<loc>`/`<guid>` lists of RSS and sitemaps; the URL leaves of JSON; link
//!   integrity (an internal link that resolves to none of our files is dangling in Go's
//!   output too: the testsite's layouts link to files that do not exist). Byte equality of
//!   every file is checked as well; it implies the rest while it holds.
//! - **L3** text: per HTML file the visible text (tags, comments, `script` and `style`
//!   removed, entities decoded, typographic characters mapped to ASCII, whitespace collapsed)
//!   and the heading-ID list; `hugo_stats.json` tag, class and id sets equal the collector's
//!   (`neohugo-publish`, checked against Go's collector by `oracle/publisher/collector`) over
//!   Go's HTML files.
//! - **Structure oracle** (per (page, format) target, permalink and template): TODO(T01) — the
//!   Go `structure` oracle does not exist yet (`tools/go-oracle/structure/`,
//!   `rust/testdata/golden/testsite/structure.json`); `neohugo-layouts`'
//!   `structure_oracle_template_and_baseof` skips until it does. Add the (page, format)
//!   comparison here when T01 has written the dump.
//!
//! Accepted deviations are listed per level below (a ratchet in miniature until T03's
//! `rust/testdata/baselines/` exists); every list is empty.
//!
//! The full output tree (every file's content, `hugo_stats.json` included) is an insta
//! snapshot: `snapshots/it__parity__testsite_output.snap`.

use std::collections::{BTreeMap, BTreeSet};

use neohugo_publish::HtmlElements;
use neohugo_testkit::fixture::rust_dir;
use neohugo_testkit::txtar::Archive;

use crate::build::{testsite, tree};
use crate::{neohugo, stderr};

/// Files of the Go build whose bytes may differ, with the reason (L2 bytes).
const ACCEPTED_BYTE_DIFFS: &[(&str, &str)] = &[];
/// Files whose L2 link sets may differ, with the reason.
const ACCEPTED_LINK_DIFFS: &[(&str, &str)] = &[];
/// Files whose L3 visible text or heading IDs may differ, with the reason.
const ACCEPTED_TEXT_DIFFS: &[(&str, &str)] = &[];

/// Go's output: the 55 files of `public`.
fn go_public() -> BTreeMap<String, Vec<u8>> {
    let go = Archive::read(&rust_dir().join("crates/build/tests/it/testsite-go.txtar"))
        .expect("go tree");
    go.files
        .into_iter()
        .map(|f| (f.name, f.data.into_bytes()))
        .collect()
}

/// The testsite built by the binary: `public` and `hugo_stats.json`.
struct Built {
    _tmp: tempfile::TempDir,
    public: BTreeMap<String, Vec<u8>>,
    stats: String,
}

fn build_testsite() -> Built {
    let tmp = tempfile::tempdir().expect("tempdir");
    let site = tmp.path().join("testsite");
    testsite(&site);
    let o = neohugo(&site, &["--clock", "2026-01-01T00:00:00Z"], &[]);
    assert!(o.status.success(), "{}", stderr(&o));
    let public = tree(&site.join("public"));
    let stats =
        std::fs::read_to_string(site.join("hugo_stats.json")).expect("hugo_stats.json written");
    Built {
        _tmp: tmp,
        public,
        stats,
    }
}

// ── L1 ───────────────────────────────────────────────────────────────────────────────────────

/// §7.2's path normalisation: fingerprints `.[0-9a-f]{16,64}.` → `.H.`, processed-image
/// hashes `_hu_[0-9a-f]+` → `_hu_H`.
fn normalize_path(path: &str) -> String {
    let hex = |s: &str| {
        s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    let mut parts: Vec<String> = path.split('.').map(str::to_owned).collect();
    let last = parts.len().saturating_sub(1);
    for (i, p) in parts.iter_mut().enumerate() {
        if i > 0 && i < last && (16..=64).contains(&p.len()) && hex(p) {
            *p = "H".to_owned();
        }
    }
    let mut out = parts.join(".");
    let mut from = 0;
    while let Some(i) = out[from..].find("_hu_").map(|i| i + from) {
        let rest = &out[i + 4..];
        let n = rest.bytes().take_while(u8::is_ascii_hexdigit).count();
        if n > 0 {
            out = format!("{}_hu_H{}", &out[..i], &rest[n..]);
        }
        from = i + 4;
    }
    out
}

// ── A small HTML/XML scanner (the tags and attributes of well-formed output) ─────────────────

struct Tag {
    name: String,
    attrs: Vec<(String, String)>,
    /// Byte offset just after `>`.
    end: usize,
}

impl Tag {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// The start tags of `doc` (names and attribute names lower-cased), skipping comments.
fn tags(doc: &str) -> Vec<Tag> {
    let b = doc.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(off) = doc[i..].find('<') {
        let start = i + off;
        if doc[start..].starts_with("<!--") {
            i = doc[start..]
                .find("-->")
                .map_or(doc.len(), |e| start + e + 3);
            continue;
        }
        let mut j = start + 1;
        if j >= b.len() || !b[j].is_ascii_alphabetic() {
            i = j;
            continue;
        }
        while j < b.len() && !b[j].is_ascii_whitespace() && b[j] != b'>' && b[j] != b'/' {
            j += 1;
        }
        let name = doc[start + 1..j].to_ascii_lowercase();
        let mut attrs = Vec::new();
        loop {
            while j < b.len() && (b[j].is_ascii_whitespace() || b[j] == b'/') {
                j += 1;
            }
            if j >= b.len() || b[j] == b'>' {
                break;
            }
            let k0 = j;
            while j < b.len() && !b[j].is_ascii_whitespace() && !b"=>/".contains(&b[j]) {
                j += 1;
            }
            let key = doc[k0..j].to_ascii_lowercase();
            let mut value = String::new();
            if j < b.len() && b[j] == b'=' {
                j += 1;
                if j < b.len() && (b[j] == b'"' || b[j] == b'\'') {
                    let q = b[j];
                    let v0 = j + 1;
                    j = v0;
                    while j < b.len() && b[j] != q {
                        j += 1;
                    }
                    value = decode_entities(&doc[v0..j.min(b.len())]);
                    j += 1;
                } else {
                    let v0 = j;
                    while j < b.len() && !b[j].is_ascii_whitespace() && b[j] != b'>' {
                        j += 1;
                    }
                    value = decode_entities(&doc[v0..j]);
                }
            }
            attrs.push((key, value));
        }
        out.push(Tag {
            name,
            attrs,
            end: (j + 1).min(doc.len()),
        });
        i = j.min(doc.len());
    }
    out
}

/// The text of element `name` after each of its start tags, up to its end tag.
fn element_texts(doc: &str, name: &str) -> Vec<String> {
    let close = format!("</{name}>");
    tags(doc)
        .into_iter()
        .filter(|t| t.name == name)
        .filter_map(|t| {
            let rest = &doc[t.end..];
            rest.find(&close).map(|e| decode_entities(&rest[..e]))
        })
        .collect()
}

/// Decodes the entities of HTML output: the named ones Go and Tera write, `&#N;`, `&#xN;`.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let c = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            "copy" => Some('©'),
            "laquo" => Some('«'),
            "raquo" => Some('»'),
            "ldquo" => Some('“'),
            "rdquo" => Some('”'),
            "lsquo" => Some('‘'),
            "rsquo" => Some('’'),
            "ndash" => Some('–'),
            "mdash" => Some('—'),
            "hellip" => Some('…'),
            _ => ent
                .strip_prefix("#x")
                .or_else(|| ent.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(c) = c {
            out.push(c);
            rest = &rest[end + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Percent-decodes a URL (§7.2: Go percent-encodes non-ASCII, Tera does not).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Some(Ok(v)) = s.get(i + 1..i + 3).map(|h| u8::from_str_radix(h, 16))
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ── L2 ───────────────────────────────────────────────────────────────────────────────────────

const BASE: &str = "https://example.org/";

/// An internal URL as a site path (`/x/`), or `None` for an external one.
fn internal(url: &str) -> Option<String> {
    let url = url.trim();
    let path = if let Some(p) = url.strip_prefix(BASE) {
        format!("/{p}")
    } else if url.starts_with('/') && !url.starts_with("//") {
        url.to_owned()
    } else {
        return None;
    };
    let path = path.split(['#', '?']).next().unwrap_or_default();
    Some(percent_decode(path))
}

/// What L2 compares of one output file.
#[derive(Debug, Default, PartialEq, Eq)]
struct Links {
    title: Vec<String>,
    rel_links: Vec<(String, String)>,
    internal: BTreeSet<String>,
    alias_target: Option<String>,
    feed: Vec<String>,
    json: Vec<String>,
}

fn links(name: &str, bytes: &[u8]) -> Links {
    let doc = String::from_utf8_lossy(bytes);
    let mut l = Links::default();
    if name.ends_with(".html") {
        l.title = element_texts(&doc, "title");
        for t in tags(&doc) {
            if t.name == "link"
                && let (Some(rel), Some(href)) = (t.attr("rel"), t.attr("href"))
                && (rel == "canonical" || rel == "alternate")
            {
                l.rel_links.push((rel.to_owned(), percent_decode(href)));
            }
            for a in ["href", "src"] {
                if let Some(u) = t.attr(a).and_then(internal) {
                    l.internal.insert(u);
                }
            }
            if let Some(set) = t.attr("srcset") {
                for c in set.split(',') {
                    if let Some(u) = c.split_whitespace().next().and_then(internal) {
                        l.internal.insert(u);
                    }
                }
            }
            if t.name == "meta"
                && t.attr("http-equiv") == Some("refresh")
                && let Some(c) = t.attr("content")
            {
                l.alias_target = c.split_once("url=").map(|(_, u)| percent_decode(u));
            }
        }
    } else if name.ends_with(".xml") {
        for e in ["link", "loc", "guid"] {
            l.feed.extend(
                element_texts(&doc, e)
                    .into_iter()
                    .map(|u| format!("{e} {u}")),
            );
        }
        for t in tags(&doc) {
            if let Some(h) = t.attr("href") {
                l.feed
                    .push(format!("{} href {}", t.name, percent_decode(h)));
            }
        }
    } else if name.ends_with(".json") {
        let v: serde_json::Value = serde_json::from_slice(bytes).expect("JSON output parses");
        json_urls(&v, "", &mut l.json);
    }
    l
}

/// The URL leaves of a JSON document (strings that look like URLs), with their key paths.
fn json_urls(v: &serde_json::Value, at: &str, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) if s.starts_with('/') || s.contains("://") => {
            out.push(format!("{at} {}", percent_decode(s)));
        }
        serde_json::Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                json_urls(x, &format!("{at}[{i}]"), out);
            }
        }
        serde_json::Value::Object(m) => {
            for (k, x) in m {
                json_urls(x, &format!("{at}.{k}"), out);
            }
        }
        _ => {}
    }
}

/// Whether site path `p` is one of `files` (`/x/` → `x/index.html`).
fn resolves(p: &str, files: &BTreeMap<String, Vec<u8>>) -> bool {
    let rel = p.trim_start_matches('/');
    rel.is_empty() && files.contains_key("index.html")
        || files.contains_key(rel)
        || files.contains_key(&format!("{}/index.html", rel.trim_end_matches('/')))
}

/// Internal links of `files` that resolve to none of them.
fn dangling(files: &BTreeMap<String, Vec<u8>>) -> BTreeSet<String> {
    files
        .iter()
        .filter(|(n, _)| n.ends_with(".html"))
        .flat_map(|(n, b)| links(n, b).internal)
        .filter(|p| !resolves(p, files))
        .collect()
}

// ── L3 ───────────────────────────────────────────────────────────────────────────────────────

/// Visible text: comments, `script`/`style` content and tags removed, entities decoded,
/// typographic characters mapped to ASCII, whitespace collapsed.
fn visible_text(doc: &str) -> String {
    let mut s = String::with_capacity(doc.len());
    let mut rest = doc;
    loop {
        let Some(i) = rest.find('<') else {
            s.push_str(rest);
            break;
        };
        s.push_str(&rest[..i]);
        rest = &rest[i..];
        if rest.starts_with("<!--") {
            rest = rest.find("-->").map_or("", |e| &rest[e + 3..]);
            continue;
        }
        let lower = rest.get(..7).unwrap_or(rest).to_ascii_lowercase();
        let skip_to = if lower.starts_with("<script") {
            Some("</script>")
        } else if lower.starts_with("<style") {
            Some("</style>")
        } else {
            None
        };
        if let Some(close) = skip_to {
            rest = rest
                .to_ascii_lowercase()
                .find(close)
                .map_or("", |e| &rest[e + close.len()..]);
            continue;
        }
        rest = rest.find('>').map_or("", |e| &rest[e + 1..]);
        s.push(' ');
    }
    let text = decode_entities(&s);
    let mapped: String = text
        .chars()
        .map(|c| match c {
            '‘' | '’' | '‚' => '\'',
            '“' | '”' | '„' | '«' | '»' => '"',
            '–' | '—' => '-',
            '\u{a0}' => ' ',
            c => c,
        })
        .collect::<String>()
        .replace('…', "...");
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The `id`s of `h1`–`h6`, in document order.
fn heading_ids(doc: &str) -> Vec<String> {
    tags(doc)
        .into_iter()
        .filter(|t| {
            t.name.len() == 2
                && t.name.starts_with('h')
                && (b'1'..=b'6').contains(&t.name.as_bytes()[1])
        })
        .filter_map(|t| t.attr("id").map(str::to_owned))
        .collect()
}

// ── The gate ─────────────────────────────────────────────────────────────────────────────────

fn accepted(list: &[(&str, &str)]) -> Vec<String> {
    list.iter().map(|(p, _)| (*p).to_owned()).collect()
}

#[test]
fn testsite_gate_a_t() {
    let go = go_public();
    assert_eq!(go.len(), 55, "Go's public directory");
    let built = build_testsite();
    let ours = &built.public;

    // L1: 55 files in `public` plus `hugo_stats.json` in the project directory.
    let norm = |m: &BTreeMap<String, Vec<u8>>| -> Vec<String> {
        let mut v: Vec<String> = m.keys().map(|k| normalize_path(k)).collect();
        v.push("../hugo_stats.json".to_owned());
        v.sort();
        v
    };
    let (want, got) = (norm(&go), norm(ours));
    assert_eq!(got.len(), 56);
    assert_eq!(got, want, "L1: the file multiset differs from Go's");

    // L2, bytes.
    let differ: Vec<String> = go
        .iter()
        .filter(|(k, v)| ours.get(*k) != Some(*v))
        .map(|(k, _)| k.clone())
        .collect();
    assert_eq!(differ, accepted(ACCEPTED_BYTE_DIFFS), "L2: bytes differ");

    // L2, links.
    let link_diffs: Vec<String> = go
        .iter()
        .filter(|(k, v)| links(k, v) != links(k, &ours[*k]))
        .map(|(k, _)| k.clone())
        .collect();
    assert_eq!(
        link_diffs,
        accepted(ACCEPTED_LINK_DIFFS),
        "L2: links differ"
    );
    let aliases = ours
        .iter()
        .filter_map(|(k, v)| links(k, v).alias_target.map(|t| (k.clone(), t)))
        .count();
    assert_eq!(aliases, 15, "the alias files (incl. page/1/ and en/)");
    let (d_ours, d_go) = (dangling(ours), dangling(&go));
    assert!(
        d_ours.is_subset(&d_go),
        "L2: links dangling only in our output: {:?}",
        d_ours.difference(&d_go).collect::<Vec<_>>()
    );
    // The testsite's layouts link to files neither build has.
    assert_eq!(
        d_ours.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            "/docs/a/",
            "/favicon.ico",
            "/img/404.png",
            "/img/a.png",
            "/img/a@2x.png",
            "/img/x.png",
            "/js/app.js",
            "/unquoted/",
            "/upper",
            "/x"
        ]
    );

    // L3: visible text and heading IDs of every HTML page.
    let text_diffs: Vec<String> = go
        .iter()
        .filter(|(k, _)| k.ends_with(".html"))
        .filter(|(k, v)| {
            let (g, o) = (
                String::from_utf8_lossy(v),
                String::from_utf8_lossy(&ours[*k]),
            );
            visible_text(&g) != visible_text(&o) || heading_ids(&g) != heading_ids(&o)
        })
        .map(|(k, _)| k.clone())
        .collect();
    assert_eq!(
        text_diffs,
        accepted(ACCEPTED_TEXT_DIFFS),
        "L3: text differs"
    );

    // L3: `hugo_stats.json` sets equal the collector's over Go's HTML files.
    let mut go_elements = HtmlElements::default();
    for (_, v) in go.iter().filter(|(k, _)| k.ends_with(".html")) {
        go_elements.add_html(&String::from_utf8_lossy(v));
    }
    let stats: serde_json::Value = serde_json::from_str(&built.stats).expect("stats JSON");
    let set = |key: &str| -> BTreeSet<String> {
        stats["htmlElements"][key]
            .as_array()
            .unwrap_or_else(|| panic!("hugo_stats.json: no {key}"))
            .iter()
            .map(|v| v.as_str().expect("string").to_owned())
            .collect()
    };
    assert_eq!(set("tags"), go_elements.tags, "hugo_stats.json tags");
    assert_eq!(
        set("classes"),
        go_elements.classes,
        "hugo_stats.json classes"
    );
    assert_eq!(set("ids"), go_elements.ids, "hugo_stats.json ids");

    println!(
        "A-T: L1 {}/56 paths; L2 {}/55 files byte-identical, links equal, {} aliases, {} dangling \
         links (all dangling in Go's output too); L3 {} HTML pages equal, hugo_stats.json sets equal \
         ({} tags, {} classes, {} ids); structure oracle: TODO(T01)",
        got.len(),
        go.len() - differ.len(),
        aliases,
        d_ours.len(),
        go.keys().filter(|k| k.ends_with(".html")).count() - text_diffs.len(),
        go_elements.tags.len(),
        go_elements.classes.len(),
        go_elements.ids.len(),
    );

    // The full output tree, reviewed with `INSTA_UPDATE=always` plus `git diff`.
    let mut snap = String::new();
    let mut files: Vec<(String, &[u8])> = ours
        .iter()
        .map(|(k, v)| (format!("public/{k}"), v.as_slice()))
        .collect();
    files.push(("hugo_stats.json".to_owned(), built.stats.as_bytes()));
    files.sort();
    for (name, bytes) in files {
        snap.push_str(&format!("-- {name} --\n"));
        snap.push_str(&String::from_utf8_lossy(bytes));
        if !snap.ends_with('\n') {
            snap.push('\n');
        }
    }
    neohugo_testkit::snapshot::settings().bind(|| {
        insta::assert_snapshot!("testsite_output", snap);
    });
}

/// The scanner and normalisations on inputs the testsite does not have.
#[test]
fn parity_helpers() {
    assert_eq!(
        normalize_path("css/main.0123456789abcdef.css"),
        "css/main.H.css"
    );
    assert_eq!(normalize_path("img/a_hu_0f3a_12.png"), "img/a_hu_H_12.png");
    assert_eq!(percent_decode("/th/%E0%B8%81/"), "/th/ก/");
    assert_eq!(decode_entities("a&amp;b&#39;c&#x41;&nope;"), "a&b'cA&nope;");
    assert_eq!(
        visible_text("<p>A &ldquo;b&rdquo;</p><script>x<y</script><!-- c --><style>s</style>\n d"),
        "A \"b\" d"
    );
    assert_eq!(
        heading_ids(r#"<h2 id="a">A</h2><h7 id="no"></h7><H3 ID='b'>B</H3>"#),
        ["a", "b"]
    );
    let l = links(
        "x.html",
        br#"<title>T</title><link rel="canonical" href="https://example.org/c/"><a href=/u/>u</a><img srcset="/a.png 1x, https://example.org/b.png 2x"><a href="//cdn/x">c</a>"#,
    );
    assert_eq!(l.title, ["T"]);
    assert_eq!(
        l.rel_links,
        [("canonical".to_owned(), "https://example.org/c/".to_owned())]
    );
    assert_eq!(
        l.internal.into_iter().collect::<Vec<_>>(),
        ["/a.png", "/b.png", "/c/", "/u/"]
    );
}
