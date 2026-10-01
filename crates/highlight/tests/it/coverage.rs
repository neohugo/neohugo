//! Docs corpus acceptance (T25): every piece of code the docs highlight renders, the classes
//! written are Chroma's, the token types agree with Chroma's (`tests/data/chroma-tokens.json.gz`)
//! and the HTML is compared with Hugo's (`tests/data/hugo-docs-html.json.gz`, hashes); see the
//! crate README for how the fixtures are made.
//!
//! `NEOHUGO_HL_PAIRS=1` prints the most frequent class differences per lexer;
//! `NEOHUGO_HL_SHOW=<lexer>:<class>` prints our scopes where Chroma writes that class;
//! `NEOHUGO_HL_OURS=<file>` writes our HTML per item (JSON) for comparing with Hugo's.

use std::collections::{BTreeMap, BTreeSet};

use neohugo_config::markup::HighlightConfig;
use neohugo_highlight::{Highlight, OptionsArg, TokenType};
use neohugo_testkit::fixture::{read_json, repo_dir};
use serde::Deserialize;

use crate::corpus::{self, Item, Opts};

#[derive(Deserialize)]
struct Tokens {
    key: String,
    lexer: String,
    #[serde(default)]
    runs: Vec<(String, usize)>,
}

#[derive(Deserialize)]
struct Html {
    key: String,
    hash: String,
}

/// The docs site's `[markup.highlight]` (docs/hugo.toml).
fn docs_config() -> HighlightConfig {
    HighlightConfig {
        line_numbers_in_table: false,
        no_classes: false,
        style: "solarized-dark".to_owned(),
        wrapper_class: "highlight not-prose".to_owned(),
        ..HighlightConfig::default()
    }
}

fn fixture<T: serde::de::DeserializeOwned>(name: &str) -> T {
    read_json(&repo_dir().join("crates/highlight/tests/data").join(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// `NEOHUGO_HL_SCOPES=<lang>|<code>`: prints the scopes of `code` (for studying the map).
#[test]
fn print_scopes() {
    let Ok(arg) = std::env::var("NEOHUGO_HL_SCOPES") else {
        return;
    };
    let (lang, code) = arg.split_once('|').expect("lang|code");
    let code = code.replace("\\n", "\n");
    let hl = Highlight::new(&docs_config());
    for (scopes, text) in hl.scopes(&code, lang) {
        println!("{text:?}\n    {scopes}");
    }
}

/// `NEOHUGO_HL_DUMP=<file>`: writes the corpus as the oracle's input (README).
#[test]
fn dump_corpus() {
    let Some(path) = std::env::var_os("NEOHUGO_HL_DUMP") else {
        return;
    };
    let items: Vec<_> = corpus::load()
        .into_iter()
        .map(|i| {
            serde_json::json!({
                "key": i.key, "html_key": i.html_key, "lang": i.lang, "code": i.code,
                "opts": i.options.to_json(),
            })
        })
        .collect();
    let text = serde_json::to_string(&items).expect("json");
    std::fs::write(path, text).expect("write");
}

/// The class family used for "same kind of token": the category, with strings, numbers and
/// other literals apart.
fn family(t: TokenType) -> Option<TokenType> {
    let c = t.category()?;
    if c == TokenType::Literal {
        t.sub_category()
    } else {
        Some(c)
    }
}

fn type_of_class(class: &str) -> Option<TokenType> {
    TokenType::ALL
        .iter()
        .copied()
        .find(|t| t.is_standard() && t.class() == class)
}

/// Characters compared: not whitespace, and classified by Chroma (not text or whitespace).
#[derive(Default)]
struct Tally {
    chars: usize,
    /// We write a class too.
    classified: usize,
    /// Same [`family`].
    same_family: usize,
    /// Same class.
    same_class: usize,
}

impl Tally {
    fn add(&mut self, o: &Self) {
        self.chars += o.chars;
        self.classified += o.classified;
        self.same_family += o.same_family;
        self.same_class += o.same_class;
    }
}

fn pct(n: usize, d: usize) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "a percentage")]
    let p = if d == 0 {
        100.0
    } else {
        100.0 * n as f64 / d as f64
    };
    p
}

type Pairs = BTreeMap<(String, String, String), usize>;

/// Per character: Chroma's class against ours.
fn compare(ours: &[(TokenType, &str)], o: &Tokens, pairs: &mut Pairs) -> Tally {
    let theirs = o
        .runs
        .iter()
        .flat_map(|(class, n)| std::iter::repeat_n(class.as_str(), *n));
    let mine = ours
        .iter()
        .flat_map(|(t, text)| text.chars().map(move |c| (*t, c)));
    let mut tally = Tally::default();
    for (class, (t, c)) in theirs.zip(mine) {
        if c.is_whitespace() || class.is_empty() || class == "w" {
            continue;
        }
        tally.chars += 1;
        let mine = t.class();
        if !mine.is_empty() && mine != "w" {
            tally.classified += 1;
        }
        if type_of_class(class).and_then(family) == family(t) {
            tally.same_family += 1;
        }
        if mine == class {
            tally.same_class += 1;
        } else {
            let key = (o.lexer.clone(), class.to_owned(), mine.to_owned());
            *pairs.entry(key).or_default() += 1;
        }
    }
    tally
}

/// The docs' call: `transform.Highlight` with the fence's options or the shortcode's string.
fn render(hl: &Highlight, item: &Item) -> String {
    let opts = match &item.options {
        Opts::Map(m) => OptionsArg::Map(m),
        Opts::Str(s) => OptionsArg::Str(s),
    };
    hl.highlight_with(&item.code, &item.lang, opts)
        .unwrap_or_else(|e| panic!("{}: {e}", item.file))
}

#[test]
fn docs_corpus() {
    let items = corpus::load();
    let started = std::time::Instant::now();
    let hl = Highlight::new(&docs_config());
    let loaded = started.elapsed();
    let started = std::time::Instant::now();
    for item in &items {
        render(&hl, item);
    }
    println!(
        "Highlight::new {loaded:?}; {} items rendered in {:?}",
        items.len(),
        started.elapsed()
    );
    let tokens: Vec<Tokens> = fixture("chroma-tokens.json.gz");
    let tokens: BTreeMap<&str, &Tokens> = tokens.iter().map(|o| (o.key.as_str(), o)).collect();
    let hugo: Vec<Html> = fixture("hugo-docs-html.json.gz");
    let hugo: BTreeMap<&str, &str> = hugo
        .iter()
        .map(|h| (h.key.as_str(), h.hash.as_str()))
        .collect();

    let chroma_classes: BTreeSet<&str> = TokenType::classes().collect();
    let mut by_lexer: BTreeMap<String, Tally> = BTreeMap::new();
    let mut total = Tally::default();
    let mut pairs = Pairs::new();
    let mut shown = BTreeSet::new();
    let (mut known, mut identical, mut compared) = (0, 0, 0);
    let mut unknown: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
    let mut ours_html = BTreeMap::new();
    for item in &items {
        *by_source.entry(format!("{:?}", item.source)).or_default() += 1;
        let html = render(&hl, item);
        for class in classes_in(&html) {
            assert!(
                chroma_classes.contains(class) || is_wrapper_class(class),
                "{}: class {class:?} is not Chroma's",
                item.file
            );
        }
        if hl.can_highlight(&item.lang) {
            known += 1;
            // Hugo's wrapper, not the plain `<pre>` of an unknown language.
            assert!(!html.starts_with("<pre"), "{}: {html}", item.file);
        } else {
            *unknown.entry(&item.lang).or_default() += 1;
        }
        ours_html.insert(item.html_key.clone(), html.clone());
        if let Some(want) = hugo.get(item.html_key.as_str()) {
            compared += 1;
            if corpus::fnv(&[&html]) == *want {
                identical += 1;
            }
        }
        let (Some(o), Some(ours)) = (
            tokens.get(item.key.as_str()),
            hl.tokens(&item.code, &item.lang),
        ) else {
            continue;
        };
        let t = compare(&ours, o, &mut pairs);
        by_lexer.entry(o.lexer.clone()).or_default().add(&t);
        total.add(&t);
        if let Ok(show) = std::env::var("NEOHUGO_HL_SHOW") {
            show_scopes(&hl, item, o, &show, &mut shown);
        }
    }

    if let Some(path) = std::env::var_os("NEOHUGO_HL_OURS") {
        std::fs::write(path, serde_json::to_string(&ours_html).expect("json")).expect("write");
    }
    println!("items {} {by_source:?}", items.len());
    println!("known language {known}, unknown {unknown:?}");
    println!("byte-identical to Hugo: {identical}/{compared}");
    println!("| lexer | chars | classified % | same family % | same class % |");
    println!("|---|---|---|---|---|");
    for (lexer, t) in by_lexer.iter().chain([(&"all".to_owned(), &total)]) {
        println!(
            "| {lexer} | {} | {:.1} | {:.1} | {:.1} |",
            t.chars,
            pct(t.classified, t.chars),
            pct(t.same_family, t.chars),
            pct(t.same_class, t.chars)
        );
    }
    if std::env::var_os("NEOHUGO_HL_PAIRS").is_some() {
        let mut pairs: Vec<_> = pairs.into_iter().collect();
        pairs.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        for ((lexer, theirs, mine), n) in pairs.iter().take(60) {
            println!("{lexer:>20} chroma {theirs:>4} ours {mine:>4}: {n}");
        }
    }

    // Everything renders; only languages Chroma does not know stay plain (as in Hugo).
    assert_eq!(known + unknown.values().sum::<usize>(), items.len());
    assert!(items.len() > 2200, "corpus: {}", items.len());
    assert!(unknown.values().sum::<usize>() <= 1, "{unknown:?}");
    // Floors just under the measured values (README), so a regression fails.
    assert!(
        compared > 2200 && identical * 100 >= compared * 95,
        "{identical}/{compared}"
    );
    assert!(pct(total.classified, total.chars) >= 99.5);
    assert!(pct(total.same_family, total.chars) >= 99.0);
    assert!(pct(total.same_class, total.chars) >= 99.0);
    let go = &by_lexer["Go HTML Template"];
    assert!(pct(go.same_class, go.chars) >= 99.5);
}

/// Wrapper classes Hugo writes around Chroma's output.
fn is_wrapper_class(class: &str) -> bool {
    matches!(class, "highlight" | "not-prose" | "code-inline") || class.starts_with("language-")
}

/// The class names of every `class="…"` attribute.
fn classes_in(html: &str) -> impl Iterator<Item = &str> {
    html.split("class=\"")
        .skip(1)
        .filter_map(|s| s.split_once('"'))
        .flat_map(|(v, _)| v.split_whitespace())
}

/// `NEOHUGO_HL_SHOW=<lexer>:<chroma class>`: our scopes where Chroma writes that class.
fn show_scopes(hl: &Highlight, item: &Item, o: &Tokens, show: &str, shown: &mut BTreeSet<String>) {
    let Some((lexer, theirs)) = show.split_once(':') else {
        return;
    };
    if o.lexer != lexer || shown.len() > 60 {
        return;
    }
    let chroma: Vec<&str> = o
        .runs
        .iter()
        .flat_map(|(c, n)| std::iter::repeat_n(c.as_str(), *n))
        .collect();
    let mut at = 0;
    for (scopes, text) in hl.scopes(&item.code, &item.lang) {
        let n = text.chars().count();
        let window = &chroma[at.min(chroma.len())..(at + n).min(chroma.len())];
        at += n;
        if window.contains(&theirs) && !text.trim().is_empty() && shown.insert(scopes.clone()) {
            println!("[{}] {text:?}\n    {scopes}", item.file);
        }
    }
}
