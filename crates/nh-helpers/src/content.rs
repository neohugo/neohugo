//! Port of `helpers/content.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::config_provider::AllProvider;
use nh_markup::converter::converter::{Converter, DocumentContext, ProviderConfig};
use nh_markup::markup::ConverterProvider;
use nh_media::media::config::ContentTypes;

/// Go: `helpers.ContentSpec`.
#[derive(Clone)]
pub struct ContentSpec {
    pub converters: Arc<dyn ConverterProvider>,
    /// Go `anchorNameSanitizer converter.AnchorNameSanitizer`: the markdown converter (or
    /// goldmark's) whose `sanitize_anchor_name` answers.
    pub anchor_name_sanitizer: Arc<dyn Converter>,
    pub cfg: Arc<dyn AllProvider>,
    /// Go `cfg.ContentTypes().(media.ContentTypes)`: `config.ContentTypesProvider` cannot be
    /// downcast in the port, so the site's content types are passed in (as for nh-markup's
    /// `new_converter_provider_with_content_types`).
    pub content_types: ContentTypes,
}

fn empty_document_context() -> DocumentContext {
    DocumentContext {
        document: go_value::Value::Invalid,
        document_lookup: None,
        document_id: String::new(),
        document_name: String::new(),
        filename: String::new(),
    }
}

impl ContentSpec {
    /// Go: `helpers.NewContentSpec(cfg, logger, contentFs, ex)` for a site without a
    /// `contentTypes` config (the default content types decoded over the default media types, what
    /// `cfg.ContentTypes()` holds then); see [`ContentSpec::new_with_content_types`].
    // Go: helpers/content.go:NewContentSpec
    pub fn new(
        cfg: Arc<dyn AllProvider>,
        exec: Arc<nh_config::hexec::Exec>,
    ) -> Result<Arc<ContentSpec>> {
        let content_types = nh_media::media::config::decode_content_types(
            &go_value::Map::new(go_value::MapType::StringAny),
            &nh_media::media::config::default_types(),
        )?
        .config;
        Self::new_with_content_types(cfg, exec, None, content_types)
    }

    /// Go: `helpers.NewContentSpec(cfg, logger, contentFs, ex)` with the site's content types
    /// (Go reads them from `cfg.ContentTypes()`). The content fs is not used by the ported
    /// converters (goldmark).
    // Go: helpers/content.go:NewContentSpec
    pub fn new_with_content_types(
        cfg: Arc<dyn AllProvider>,
        exec: Arc<nh_config::hexec::Exec>,
        logger: Option<Arc<nh_common::loggers::Logger>>,
        content_types: ContentTypes,
    ) -> Result<Arc<ContentSpec>> {
        let converter_provider = nh_markup::markup::new_converter_provider_with_content_types(
            ProviderConfig {
                conf: cfg.clone(),
                exec,
                highlighter: None,
                logger,
            },
            &content_types,
        )?;

        let p = converter_provider
            .get("markdown")
            .ok_or_else(|| Error::new("markup: no markdown converter"))?;
        let conv = p.new_converter(empty_document_context())?;
        let anchor_name_sanitizer = if conv.sanitize_anchor_name("").is_some() {
            conv
        } else {
            // Use Goldmark's sanitizer
            let p = converter_provider
                .get("goldmark")
                .ok_or_else(|| Error::new("markup: no goldmark converter"))?;
            p.new_converter(empty_document_context())?
        };

        Ok(Arc::new(ContentSpec {
            converters: converter_provider,
            anchor_name_sanitizer,
            cfg,
            content_types,
        }))
    }

    /// Go: `ContentSpec.ResolveMarkup(in)`.
    // Go: helpers/content.go:ResolveMarkup
    pub fn resolve_markup(&self, input: &str) -> String {
        let input = go_unicode::strings::to_lower_str(input).into_owned();

        if let Some(media_type) = self
            .content_types
            .types()
            .get_best_match(&nh_markup::markup::resolve_markup(&input))
        {
            return media_type.sub_type.clone();
        }

        if let Some(conv) = self.converters.get(&input) {
            return nh_markup::markup::resolve_markup(conv.name());
        }

        String::new()
    }

    /// Go: `ContentSpec.SanitizeAnchorName(s)`.
    // Go: helpers/content.go:SanitizeAnchorName
    pub fn sanitize_anchor_name(&self, s: &str) -> String {
        self.anchor_name_sanitizer
            .sanitize_anchor_name(s)
            .expect("the anchor name sanitizer is goldmark's")
    }

    /// Go: `ContentSpec.TrimShortHTML(input, markup)` — strips a single wrapping `<p>`..`</p>`
    /// when `bytes.Count(input, "<p>") == 1`.
    // Go: helpers/content.go:TrimShortHTML
    pub fn trim_short_html(&self, input: &[u8], markup: &str) -> Vec<u8> {
        let mut opening_tag: &[u8] = b"<p>";
        let mut closing_tag: &[u8] = b"</p>";

        if markup
            == nh_media::media::config::default_content_types()
                .ascii_doc
                .sub_type
        {
            opening_tag = b"<div class=\"paragraph\">\n<p>";
            closing_tag = b"</p>\n</div>";
        }

        let mut input = input;
        if go_unicode::bytes::count(input, opening_tag) == 1 {
            input = go_unicode::strings::trim_space(input);
            if input.starts_with(opening_tag) && input.ends_with(closing_tag) {
                input = &input[opening_tag.len()..];
                input = input.strip_suffix(closing_tag).unwrap_or(input);
                input = go_unicode::strings::trim_space(input);
            }
        }

        input.to_vec()
    }
}

/// Go: `stripEmptyNav(in)`.
// Go: helpers/content.go:stripEmptyNav
fn strip_empty_nav(input: &[u8]) -> Vec<u8> {
    go_unicode::strings::replace_all(input, b"<nav>\n</nav>\n\n", b"").into_owned()
}

/// Go: `helpers.ExtractTOC(content)` — `(newcontent, toc)`; `toc` `None` is Go's nil.
// Go: helpers/content.go:ExtractTOC
pub fn extract_toc(content: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    if !go_unicode::bytes::contains(content, b"<nav>") {
        return (content.to_vec(), None);
    }
    let orig_content = content.to_vec();
    let first: &[u8] = b"<nav>\n<ul>";
    let last: &[u8] = b"</ul>\n</nav>";
    let replacement: &[u8] = b"<nav id=\"TableOfContents\">\n<ul>";

    let start_of_toc = go_unicode::strings::index(content, first);

    let peek_end = std::cmp::min(content.len() as isize, 70 + start_of_toc);

    if start_of_toc < 0 {
        return (strip_empty_nav(content), None);
    }
    let start_of_toc = start_of_toc as usize;
    // Need to peek ahead to see if this nav element is actually the right one.
    let correct_nav = go_unicode::strings::index(
        &content[start_of_toc..peek_end as usize],
        b"<li><a href=\"#",
    );
    if correct_nav < 0 {
        // no match found
        return (content.to_vec(), None);
    }
    // Go: bytes.Index(...) + len(last); -1 + len(last) when `last` is missing.
    let length_of_toc =
        go_unicode::strings::index(&content[start_of_toc..], last) + last.len() as isize;
    let end_of_toc = start_of_toc as isize + length_of_toc;
    let end_of_toc = end_of_toc as usize;

    let mut newcontent = content[..start_of_toc].to_vec();
    newcontent.extend_from_slice(&content[end_of_toc..]);
    let mut toc = replacement.to_vec();
    toc.extend_from_slice(&orig_content[start_of_toc + first.len()..end_of_toc]);
    (newcontent, Some(toc))
}

/// Go: `helpers.TotalWords(s)` (count of non-space runs, `unicode.IsSpace`).
// Go: helpers/content.go:TotalWords
pub fn total_words(s: &[u8]) -> i64 {
    let mut n = 0i64;
    let mut in_word = false;
    for (_, r) in go_unicode::utf8::runes(s) {
        let was_in_word = in_word;
        in_word = !go_unicode::is_space(r);
        if in_word && !was_in_word {
            n += 1;
        }
    }
    n
}

/// Go: `helpers.BytesToHTML` -> `template.HTML`.
// Go: helpers/content.go:BytesToHTML
pub fn bytes_to_html(b: &[u8]) -> go_value::Value {
    go_value::Value::html(b.to_vec())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/content.go (186 lines; 4/8 funcs executed)
//   types: ContentSpec
// OK L48-82: NewContentSpec(cfg config.AllProvider, logger loggers.Logger, contentFs afero.Fs, ex *hexec.Exec) (*ContentSpec, error)
// OK L85-87: stripEmptyNav(in []byte) []byte
// OK L90-92: BytesToHTML(b []byte) template.HTML
// OK L95-128: ExtractTOC(content []byte) (newcontent []byte, toc []byte)
// OK L130-132: (c *ContentSpec) SanitizeAnchorName(s string) string
// OK L134-146: (c *ContentSpec) ResolveMarkup(in string) string
// OK L151-162: TotalWords(s string) int
// OK L167-186: (c *ContentSpec) TrimShortHTML(input []byte, markup string) []byte
// ---------------------------------------------------------------------------
