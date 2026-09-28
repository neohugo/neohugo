//! Port of `markup/markup.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup.ConverterProvider`: registry of markup converters by name/extension
//! (goldmark + stubs for asciidocext, pandoc, rst, org, html).

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;
use nh_media::media::config::ContentTypes;

use crate::converter::converter::{Provider, ProviderConfig};
use crate::highlight::highlight::Highlighter;
use crate::markup_config::Config as MarkupConfig;

/// Go: `markup.ConverterProvider`.
pub trait ConverterProvider: Send + Sync {
    /// Lookup by name or file extension (`md`, `markdown`, `goldmark`, `html`, ...).
    fn get(&self, name: &str) -> Option<Arc<dyn Provider>>;
    fn is_goldmark(&self, name: &str) -> bool;
    fn get_markup_config(&self) -> Arc<MarkupConfig>;
    fn get_highlighter(&self) -> Arc<dyn Highlighter>;
}

/// Go: `markup.converterRegistry`.
pub struct ConverterRegistry {
    /// Maps name (md, markdown, goldmark etc.) to a converter provider. Note that this is also
    /// used for aliasing, so the same converter may be registered multiple times. All names are
    /// lower case.
    pub converters: BTreeMap<String, Arc<dyn Provider>>,
    pub config: ProviderConfig,
    markup_config: Arc<MarkupConfig>,
    highlighter: Arc<dyn Highlighter>,
}

/// Go: `markup.NewConverterProvider(cfg)`. The content types are the site's
/// (`cfg.Conf.ContentTypes().(media.ContentTypes)`); `config::ContentTypesProvider` cannot be
/// downcast, so they are passed in (`new_converter_provider` uses the defaults).
// Go: markup/markup.go:NewConverterProvider
pub fn new_converter_provider_with_content_types(
    mut cfg: ProviderConfig,
    content_types: &ContentTypes,
) -> Result<Arc<dyn ConverterProvider>> {
    let mut converters: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();

    let mcfg = cfg.markup_config();

    if cfg.highlighter.is_none() {
        cfg.highlighter = Some(crate::highlight::highlight::new(mcfg.highlight.clone()));
    }

    let default_handler = mcfg.default_markdown_handler.clone();
    let mut default_found = false;

    let mut add = |c: Arc<dyn Provider>, sub_type: &str, mut aliases: Vec<String>| {
        let name = c.name().to_string();

        aliases.push(name.clone());
        aliases.push(sub_type.to_string());

        if go_unicode::strings::equal_fold(name.as_bytes(), default_handler.as_bytes()) {
            aliases.push("markdown".to_string());
            default_found = true;
        }

        add_converter(&mut converters, c, &aliases);
    };

    let ct = content_types;
    add(
        crate::goldmark::convert::GoldmarkProvider::new_from_config(
            mcfg.clone(),
            cfg.conf.enable_emoji(),
            cfg.logger.clone(),
        )?,
        &ct.markdown.sub_type,
        ct.markdown.suffixes(),
    );
    add(
        crate::asciidocext::new_provider(&cfg)?,
        &ct.ascii_doc.sub_type,
        ct.ascii_doc.suffixes(),
    );
    add(
        crate::rst::new_provider(&cfg)?,
        &ct.re_structured_text.sub_type,
        ct.re_structured_text.suffixes(),
    );
    add(
        crate::pandoc::new_provider(&cfg)?,
        &ct.pandoc.sub_type,
        ct.pandoc.suffixes(),
    );
    add(
        crate::org::new_provider(&cfg)?,
        &ct.emacs_org_mode.sub_type,
        ct.emacs_org_mode.suffixes(),
    );

    if !default_found {
        let mut msg = format!(
            "markup: Configured defaultMarkdownHandler {} not found.",
            go_strconv::quote(default_handler.as_bytes())
        );
        if default_handler == "blackfriday" {
            msg.push_str(
                " Did you mean to use goldmark? Blackfriday was removed in Hugo v0.100.0.",
            );
        }
        return Err(Error::new(msg));
    }

    let highlighter = cfg.highlighter.clone().expect("highlighter set above");
    Ok(Arc::new(ConverterRegistry {
        config: cfg,
        converters,
        markup_config: mcfg,
        highlighter,
    }))
}

/// Go: `markup.NewConverterProvider(cfg)` with the default content types.
// Go: markup/markup.go:NewConverterProvider
pub fn new_converter_provider(cfg: ProviderConfig) -> Result<Arc<dyn ConverterProvider>> {
    new_converter_provider_with_content_types(
        cfg,
        &nh_media::media::config::default_content_types(),
    )
}

impl ConverterProvider for ConverterRegistry {
    // Go: markup/markup.go:Get
    fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        let key = go_unicode::strings::to_lower_str(name);
        self.converters.get(key.as_ref()).cloned()
    }

    // Go: markup/markup.go:IsGoldmark
    fn is_goldmark(&self, name: &str) -> bool {
        self.get(name).is_some_and(|cp| cp.name() == "goldmark")
    }

    // Go: markup/markup.go:GetMarkupConfig
    fn get_markup_config(&self) -> Arc<MarkupConfig> {
        self.markup_config.clone()
    }

    // Go: markup/markup.go:GetHighlighter
    fn get_highlighter(&self) -> Arc<dyn Highlighter> {
        self.highlighter.clone()
    }
}

// Go: markup/markup.go:addConverter
fn add_converter(
    m: &mut BTreeMap<String, Arc<dyn Provider>>,
    c: Arc<dyn Provider>,
    aliases: &[String],
) {
    for alias in aliases {
        m.insert(alias.clone(), c.clone());
    }
}

/// Go: `markup.ResolveMarkup(s)` — normalises markup names (`goldmark` -> `markdown`,
/// `asciidocext` -> `asciidoc`).
// Go: markup/markup.go:ResolveMarkup
pub fn resolve_markup(s: &str) -> String {
    let s = go_unicode::strings::to_lower_str(s).into_owned();
    let dct = nh_media::media::config::default_content_types();
    match s.as_str() {
        "goldmark" => dct.markdown.sub_type.clone(),
        "asciidocext" => dct.ascii_doc.sub_type.clone(),
        _ => s,
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/markup.go (152 lines; 5/7 funcs executed)
//   types: ConverterProvider, converterRegistry
// OK L36-98: NewConverterProvider(cfg converter.ProviderConfig) (ConverterProvider, error)
// OK L118-121: (r *converterRegistry) IsGoldmark(name string) bool
// OK L123-125: (r *converterRegistry) Get(name string) converter.Provider
// OK L127-129: (r *converterRegistry) GetHighlighter() highlight.Highlighter
// OK L131-133: (r *converterRegistry) GetMarkupConfig() markup_config.Config
// OK L135-139: addConverter(m map[string]converter.Provider, c converter.Provider, aliases ...string)
// OK L142-152: ResolveMarkup(s string) string
// ---------------------------------------------------------------------------
