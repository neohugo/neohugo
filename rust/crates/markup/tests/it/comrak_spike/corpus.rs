//! The two inputs: the 959 docs bodies of the `markup/convert` and `markup/hooks` oracles
//! (Hugo's goldmark converter, six configurations), and the 251 seeksnack bodies of the
//! goldmark corpus (`corpus/goldmark/corpus{,-ext}.gmf.gz`, plain goldmark instances).

use std::collections::BTreeMap;
use std::io::Read;

use neohugo_testkit::fixture::{GoString, oracle, testdata};
use serde::Deserialize;

/// Hugo markup configurations of the convert oracle, in fixture order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HugoCfg {
    Default,
    Seeksnack,
    Ascii,
    Blackfriday,
    Cjk,
    Noattr,
}

impl HugoCfg {
    pub const ALL: [Self; 6] = [
        Self::Default,
        Self::Seeksnack,
        Self::Ascii,
        Self::Blackfriday,
        Self::Cjk,
        Self::Noattr,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Seeksnack => "seeksnack",
            Self::Ascii => "ascii",
            Self::Blackfriday => "blackfriday",
            Self::Cjk => "cjk",
            Self::Noattr => "noattr",
        }
    }
}

pub struct DocsCorpus {
    /// `(name, markdown body)` of the 959 `docs/content` files.
    pub docs: Vec<(String, String)>,
    /// Hugo's HTML per `[doc][cfg]`.
    pub html: Vec<[String; 6]>,
    /// Decoded hook records of the `seeksnack` configuration, per doc.
    pub hooks: Vec<Vec<HookRecord>>,
}

/// One hook invocation of the hooks oracle (`kind` plus its `Field` lines).
pub struct HookRecord {
    pub kind: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Doc {
    name: String,
    src: GoString,
}

#[derive(Deserialize)]
struct Convert {
    docs: Vec<Doc>,
    results: Vec<ConvertResult>,
}

#[derive(Deserialize)]
struct ConvertResult {
    doc: usize,
    cfg: usize,
    html: Option<GoString>,
    html_same: Option<usize>,
}

#[derive(Deserialize)]
struct Hooks {
    docs: Vec<Doc>,
    results: Vec<HooksResult>,
}

#[derive(Deserialize)]
struct HooksResult {
    doc: usize,
    cfg: usize,
    records: Option<Vec<GoString>>,
}

fn text(s: &GoString) -> String {
    String::from_utf8_lossy(&s.0).into_owned()
}

pub fn docs() -> DocsCorpus {
    let convert: Convert = oracle("oracle/markup/convert/convert.json.gz");
    let keep: Vec<usize> = (0..convert.docs.len())
        .filter(|&i| convert.docs[i].name.starts_with("docs/content/"))
        .collect();
    let mut html: Vec<[String; 6]> = vec![Default::default(); convert.docs.len()];
    let mut same = Vec::new();
    for r in &convert.results {
        match (&r.html, r.html_same) {
            (Some(h), _) => html[r.doc][r.cfg] = text(h),
            (None, Some(k)) => same.push((r.doc, r.cfg, k)),
            (None, None) => {}
        }
    }
    for (doc, cfg, k) in same {
        html[doc][cfg] = html[doc][k].clone();
    }

    let hooks_fixture: Hooks = oracle("oracle/markup/hooks/hooks.json.gz");
    let mut hooks: Vec<Vec<HookRecord>> = (0..convert.docs.len()).map(|_| Vec::new()).collect();
    for r in hooks_fixture.results {
        let seeksnack = r.cfg == 1
            && convert
                .docs
                .get(r.doc)
                .is_some_and(|d| d.name == hooks_fixture.docs[r.doc].name);
        if seeksnack {
            hooks[r.doc] = r
                .records
                .unwrap_or_default()
                .iter()
                .map(|g| record(&text(g)))
                .collect();
        }
    }

    DocsCorpus {
        docs: keep
            .iter()
            .map(|&i| (convert.docs[i].name.clone(), text(&convert.docs[i].src)))
            .collect(),
        html: keep.iter().map(|&i| html[i].clone()).collect(),
        hooks: keep
            .iter()
            .map(|&i| std::mem::take(&mut hooks[i]))
            .collect(),
    }
}

/// Parses the oracle's `name:len:value` field dump (values may span lines; `len` is in bytes).
fn record(s: &str) -> HookRecord {
    let mut fields = BTreeMap::new();
    let mut rest = s;
    while let Some((name, tail)) = rest.split_once(':') {
        let Some((len, tail)) = tail.split_once(':') else {
            break;
        };
        let Ok(len) = len.parse::<usize>() else {
            break;
        };
        let value = tail.get(..len).unwrap_or(tail);
        fields.insert(name.to_owned(), value.to_owned());
        rest = tail
            .get(len..)
            .unwrap_or("")
            .strip_prefix('\n')
            .unwrap_or("");
    }
    HookRecord {
        kind: fields.remove("kind").unwrap_or_default(),
        fields,
    }
}

/// One seeksnack body with goldmark's output per corpus configuration.
pub struct SeeksnackDoc {
    pub name: String,
    pub md: String,
    /// `(goldmark configuration, html)`.
    pub html: Vec<(String, String)>,
}

pub fn seeksnack() -> Vec<SeeksnackDoc> {
    let mut by_name: BTreeMap<String, SeeksnackDoc> = BTreeMap::new();
    for file in [
        "corpus/goldmark/corpus.gmf.gz",
        "corpus/goldmark/corpus-ext.gmf.gz",
    ] {
        for (path, fields) in gmf(&gunzip(file)) {
            // `corpus-full/*` carries front matter; `corpus/<cfg>/<name>` is the body.
            let Some(rest) = path.strip_prefix("corpus/") else {
                continue;
            };
            let Some((cfg, name)) = rest.split_once('/') else {
                continue;
            };
            let md = fields.get("md").cloned().unwrap_or_default();
            let html = fields.get("html").cloned().unwrap_or_default();
            let doc = by_name
                .entry(name.to_owned())
                .or_insert_with(|| SeeksnackDoc {
                    name: name.to_owned(),
                    md,
                    html: Vec::new(),
                });
            doc.html.push((cfg.to_owned(), html));
        }
    }
    by_name.into_values().collect()
}

fn gunzip(rel: &str) -> String {
    let raw = std::fs::read(testdata(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    let mut out = String::new();
    flate2::read::GzDecoder::new(raw.as_slice())
        .read_to_string(&mut out)
        .unwrap_or_else(|e| panic!("{rel}: {e}"));
    out
}

/// `=== <path>` records of `<field> <byte len>\n<bytes>\n` fields.
fn gmf(text: &str) -> Vec<(String, BTreeMap<String, String>)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(tail) = rest.strip_prefix("=== ") {
        let (path, mut body) = tail.split_once('\n').unwrap_or((tail, ""));
        let mut fields = BTreeMap::new();
        while !body.is_empty() && !body.starts_with("=== ") {
            let (head, tail) = body.split_once('\n').unwrap_or((body, ""));
            let (name, len) = head.split_once(' ').unwrap_or((head, "0"));
            let len: usize = len.parse().unwrap_or(0);
            fields.insert(name.to_owned(), tail[..len].to_owned());
            body = tail[len..].strip_prefix('\n').unwrap_or(&tail[len..]);
        }
        out.push((path.to_owned(), fields));
        rest = body;
    }
    out
}
