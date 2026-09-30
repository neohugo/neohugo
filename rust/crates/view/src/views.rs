//! The view structs (REWRITE_PLAN.md §2.5), serialised once into `tera::Value`s.
//!
//! **Skeleton subset (T38).** These are the fields the testsite layouts and the embedded
//! `rss.xml`, `sitemap.xml`, `sitemapindex.xml` and `alias.html` read; T33 completes them to
//! the plan's list (file, resources, menus, related, git info, …) with every documented key
//! always present.

use jiff::Zoned;
use neohugo_base::{PageKind, Params, Value};
use neohugo_config::Config;
use neohugo_config::site::SiteConfig;
use serde::Serialize;

use crate::content::RenderedContent;
use crate::interim::{FlatPage, FlatSite};

/// A date: compare instants with `.unix`, format with the `date` filter.
#[derive(Clone, Debug, Serialize)]
pub struct DateView {
    pub rfc3339: String,
    pub unix: i64,
}

impl DateView {
    #[must_use]
    pub fn new(d: &Zoned) -> Self {
        let rfc3339 = if d.timestamp().subsec_nanosecond() == 0 {
            d.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
        } else {
            d.strftime("%Y-%m-%dT%H:%M:%S%.f%:z").to_string()
        };
        Self {
            rfc3339,
            unix: d.timestamp().as_second(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SitemapView {
    pub change_freq: String,
    pub priority: f64,
    pub disable: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LanguageView {
    pub lang: String,
    pub name: String,
    pub code: String,
    pub direction: String,
    pub weight: i32,
    pub params: tera::Value,
}

impl LanguageView {
    #[must_use]
    pub fn new(site: &SiteConfig) -> Self {
        let l = &site.language;
        Self {
            lang: l.key.clone(),
            name: l.name.clone(),
            code: l.code.clone(),
            direction: match l.direction {
                neohugo_config::site::Direction::Ltr => "ltr",
                neohugo_config::site::Direction::Rtl => "rtl",
            }
            .to_owned(),
            weight: l.weight,
            params: params_value(&site.params),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MediaTypeView {
    pub r#type: String,
    pub main_type: String,
    pub sub_type: String,
    pub suffixes: Vec<String>,
    pub delimiter: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct OutputFormatView {
    pub name: String,
    pub rel: String,
    pub media_type: MediaTypeView,
    pub permalink: String,
    pub rel_permalink: String,
    pub is_plain_text: bool,
    pub is_html: bool,
}

/// A reference to a page (menus, terms).
#[derive(Clone, Debug, Serialize)]
pub struct PageLink {
    pub id: u32,
    pub kind: PageKind,
    pub path: String,
    pub lang: String,
    pub title: String,
    pub link_title: String,
    pub permalink: String,
    pub rel_permalink: String,
}

/// The content fields of a Full generation's page values.
#[derive(Clone, Debug, Serialize)]
pub struct ContentView {
    pub content: tera::Value,
    pub summary: tera::Value,
    pub truncated: bool,
    pub plain: String,
    pub raw_content: String,
    pub word_count: usize,
    pub fuzzy_word_count: usize,
    pub reading_time: usize,
    pub table_of_contents: tera::Value,
    pub len: usize,
}

impl ContentView {
    /// The content fields of `c` (`None`: a page without content, all fields empty).
    #[must_use]
    pub fn new(c: Option<&RenderedContent>, raw: &str) -> Self {
        let empty = RenderedContent::default();
        let c = c.unwrap_or(&empty);
        Self {
            content: tera::Value::safe_string(&c.html),
            summary: tera::Value::safe_string(&c.summary),
            truncated: c.truncated,
            plain: c.plain.clone(),
            raw_content: raw.to_owned(),
            word_count: c.word_count,
            fuzzy_word_count: c.fuzzy_word_count,
            reading_time: c.reading_time,
            table_of_contents: tera::Value::safe_string(&c.table_of_contents),
            len: c.html.len(),
        }
    }
}

/// The relation-free page value, Arc-shared in every list.
#[derive(Clone, Debug, Serialize)]
#[allow(clippy::struct_excessive_bools)] // template fields (`is_home`, `draft`, …), not state
pub struct PageSummaryView {
    pub id: u32,
    pub kind: PageKind,
    pub lang: String,
    pub path: String,
    pub section: String,
    pub r#type: String,
    pub layout: Option<String>,
    pub title: String,
    pub link_title: String,
    pub description: String,
    pub date: Option<DateView>,
    pub lastmod: Option<DateView>,
    pub publish_date: Option<DateView>,
    pub expiry_date: Option<DateView>,
    pub weight: i32,
    pub draft: bool,
    pub params: tera::Value,
    pub keywords: Vec<String>,
    pub aliases: Vec<String>,
    pub permalink: String,
    pub rel_permalink: String,
    pub is_home: bool,
    pub is_section: bool,
    pub is_page: bool,
    pub is_node: bool,
    pub is_translated: bool,
    pub sitemap: SitemapView,
    pub language: LanguageView,
    pub output_formats: tera::Value,
    pub resources: tera::Value,
    pub terms: tera::Value,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentView>,
}

/// The template value of front matter or configuration params.
#[must_use]
pub fn params_value(p: &Params) -> tera::Value {
    Value::Map(std::sync::Arc::new(p.as_map().clone())).to_tera()
}

fn output_formats(cfg: &Config, p: &FlatPage) -> tera::Value {
    let mut m = tera::Map::new();
    for o in &p.outputs {
        let f = cfg.output_formats.get(o.format);
        let mt = cfg.media_types.get(f.media_type);
        let v = OutputFormatView {
            name: f.name.clone(),
            rel: f.rel.clone(),
            media_type: MediaTypeView {
                r#type: mt.type_string(),
                main_type: mt.main.clone(),
                sub_type: mt.sub.clone(),
                suffixes: mt.suffixes.clone(),
                delimiter: mt.delimiter.clone(),
            },
            permalink: o.links.permalink.to_string(),
            rel_permalink: o.links.rel_permalink.escaped(),
            is_plain_text: f.escaping == neohugo_config::output::Escaping::Plain,
            is_html: f.is_html,
        };
        m.insert(f.name.clone().into(), tera::Value::from_serializable(&v));
    }
    tera::Value::from(m)
}

/// The link value of page `p`.
#[must_use]
pub fn page_link(flat: &FlatSite, p: &FlatPage) -> PageLink {
    let (permalink, rel_permalink) = p.links.as_ref().map_or_else(Default::default, |l| {
        (l.permalink.to_string(), l.rel_permalink.escaped())
    });
    PageLink {
        id: p.id.raw(),
        kind: p.kind,
        path: p.path.clone(),
        lang: flat.config.sites[p.lang].language.key.clone(),
        title: p.title.clone(),
        link_title: p.link_title.clone(),
        permalink,
        rel_permalink,
    }
}

/// The summary value of page `p`, with `content` for Full generations.
#[must_use]
pub fn page_summary(
    flat: &FlatSite,
    p: &FlatPage,
    content: Option<ContentView>,
) -> PageSummaryView {
    let cfg = &flat.config;
    let site = &cfg.sites[p.lang];
    let (permalink, rel_permalink) = p.links.as_ref().map_or_else(Default::default, |l| {
        (l.permalink.to_string(), l.rel_permalink.escaped())
    });
    let date = |d: Option<&Zoned>| d.map(DateView::new);
    let mut terms = tera::Map::new();
    for (t, pages) in flat.langs[p.lang].taxonomies.iter().zip(&p.terms) {
        let links: Vec<tera::Value> = pages
            .iter()
            .map(|&term| tera::Value::from_serializable(&page_link(flat, &flat.pages[term])))
            .collect();
        terms.insert(t.plural.clone().into(), tera::Value::from(links));
    }
    PageSummaryView {
        id: p.id.raw(),
        kind: p.kind,
        lang: site.language.key.clone(),
        path: p.path.clone(),
        section: p.section.clone(),
        r#type: p.r#type.clone(),
        layout: p.layout.clone(),
        title: p.title.clone(),
        link_title: p.link_title.clone(),
        description: p.description.clone(),
        date: date(p.dates.date.as_ref()),
        lastmod: date(p.dates.lastmod.as_ref()),
        publish_date: date(p.dates.publish_date.as_ref()),
        expiry_date: date(p.dates.expiry_date.as_ref()),
        weight: p.weight,
        draft: p.draft,
        params: params_value(&p.params),
        keywords: p.keywords.clone(),
        aliases: p.aliases.clone(),
        permalink,
        rel_permalink,
        is_home: p.kind == PageKind::Home,
        is_section: p.kind == PageKind::Section,
        is_page: p.kind == PageKind::Page,
        is_node: p.kind.is_branch() || p.kind == PageKind::NotFound,
        is_translated: !p.translations.is_empty(),
        sitemap: SitemapView {
            change_freq: p.sitemap.change_freq.clone(),
            priority: p.sitemap.priority,
            disable: p.sitemap.disable,
        },
        language: LanguageView::new(site),
        output_formats: output_formats(cfg, p),
        resources: tera::Value::from(Vec::<tera::Value>::new()),
        terms: tera::Value::from(terms),
        content,
    }
}

/// `hugo`.
#[derive(Clone, Debug, Serialize)]
pub struct HugoView {
    pub version: &'static str,
    pub neohugo_version: &'static str,
    pub environment: String,
    pub is_production: bool,
    pub is_development: bool,
    pub is_server: bool,
    pub generator: tera::Value,
}

impl HugoView {
    #[must_use]
    pub fn new(cfg: &Config) -> Self {
        Self {
            version: "0.149.0-DEV",
            neohugo_version: env!("CARGO_PKG_VERSION"),
            environment: cfg.environment.clone(),
            is_production: cfg.environment == "production",
            is_development: cfg.environment == "development",
            is_server: false,
            generator: tera::Value::safe_string(
                r#"<meta name="generator" content="Hugo 0.149.0-DEV">"#,
            ),
        }
    }
}
