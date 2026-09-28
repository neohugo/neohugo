//! Port of `hugolib/page__content.go` (parse half).
//!
//! Owner: Wave B task T20 (hugolib-capture).

//! Split from `page__content.go` because page creation (`page__new.go`, T20) calls
//! `pageMeta.parseFrontMatter` and `newCachedContent` (page__new.go:66, 192). This module holds
//! page__content.go lines 60-154 (`pageContentReplacement`, `parseFrontMatter`,
//! `newCachedContent`), the `cachedContent`/`contentParseInfo` types (156-224) and 266-490
//! (`IsZero`, `parseContentFile`, `parseFrontMatter`, `failMap`, `mapFrontMatter`,
//! `mapItemsAfterFrontMatter`, `mustSource`, `contentSource`, `readSourceAll`). Rendering
//! (`contentToRender`, scopes, TOC, summary, plain) is `page__content.rs` (T22).
//!
//! Shortcode placeholders are created here (`HAHAHUGOSHORTCODE<pid>s<ordinal>HBHB`); `pid` is only
//! an identity (any unique numbering works: placeholders never reach the output).
//!
//! Deviation: Go reads the source through the `/cont/src` dynacache partition of `HugoSites`
//! (keyed by `sourceKey`, re-read when evicted or stale); the port reads it once at page
//! creation and keeps the bytes in `ContentParseInfo::source`.

use std::io::Read;
use std::sync::Arc;

use go_value::{Map, Value};
use nh_common::Result;
use nh_common::dynacache::Partition;
use nh_common::herrors::{Error, FilePos};
use nh_common::hugio::OpenReadSeekCloser;
use nh_parser::metadecoders::decoder::Decoder;
use nh_parser::metadecoders::format::Format;
use nh_parser::pageparser::item::{ItemType, Items};
use nh_parser::pageparser::pagelexer::Config as ParseConfig;
use nh_parser::pageparser::pageparser as pp;
use nh_tplimpl::templatestore::TemplateStore;

use crate::page__content::CachedContentScope;
use crate::page__meta::PageMeta;
use crate::shortcode_parse::{
    Shortcode, ShortcodeHandler, create_shortcode_placeholder, pos_from_input,
};

/// Go: `internalSummaryDividerBase`.
pub const INTERNAL_SUMMARY_DIVIDER_BASE: &str = "HUGOMORE42";

/// Go: `internalSummaryDividerPre` (`"\n\n" + internalSummaryDividerBase + "\n\n"`).
pub const INTERNAL_SUMMARY_DIVIDER_PRE: &[u8] = b"\n\nHUGOMORE42\n\n";

/// Go: `contentParseInfo`.
pub struct ContentParseInfo {
    pub pid: u64,
    /// Go `sourceKey`: the slash-separated filename, or the pid for pages without a file. Part
    /// of the rendered-content cache keys (PageMap partitions, page__content.go:522-530).
    pub source_key: String,
    /// Go `openSource` (nil for pages without a file).
    pub open_source: Option<OpenReadSeekCloser>,
    /// The source bytes (Go reads them through `contentSource`, `h.cacheContentSource`
    /// "/cont/src"; see the module docs).
    pub source: Arc<Vec<u8>>,
    /// Go `frontMatter` (`None` = nil: no front matter, or a `null` document).
    pub front_matter: Option<Map>,
    /// Whether the parsed content contains a summary separator.
    pub has_summary_divider: bool,
    /// The position in bytes after any front matter.
    pub pos_main_content: i64,
    /// Indicates whether we must do placeholder replacements.
    pub has_non_markdown_shortcode: bool,
    /// Items from the page parser (Go `itemsStep1`). These map directly to the source.
    pub items_step1: Items,
    /// Mapped items: source ranges, shortcodes, summary divider (Go `itemsStep2 []any`).
    pub items_step2: Vec<ContentItem>,
}

/// Go `itemsStep2` elements (`pageparser.Item`, `pageContentReplacement`, `*shortcode`).
#[derive(Clone)]
pub enum ContentItem {
    /// A range of the source (Go `pageparser.Item`; `contentToRender` appends
    /// `source[v.Pos():v.Pos()+len(v.Val(source))]`).
    Source {
        low: usize,
        high: usize,
    },
    Shortcode(Arc<Shortcode>),
    /// Replacement bytes (Go `pageContentReplacement`, e.g. `internalSummaryDividerPre`).
    Replacement(Vec<u8>),
}

impl ContentParseInfo {
    /// Go: `AddBytes(item)`.
    // Go: hugolib/page__content.go:AddBytes
    pub fn add_bytes(&mut self, item: &nh_parser::pageparser::item::Item, source: &[u8]) {
        let low = item.pos();
        let high = low + item.val(source).len();
        self.items_step2.push(ContentItem::Source { low, high });
    }

    /// Go: `AddReplacement(val, source)`.
    // Go: hugolib/page__content.go:AddReplacement
    pub fn add_replacement(&mut self, val: &[u8]) {
        self.items_step2
            .push(ContentItem::Replacement(val.to_vec()));
    }

    /// Go: `AddShortcode(s)`.
    // Go: hugolib/page__content.go:AddShortcode
    pub fn add_shortcode(&mut self, s: Arc<Shortcode>) {
        let insert = s.insert_placeholder();
        self.items_step2.push(ContentItem::Shortcode(s));
        if insert {
            self.has_non_markdown_shortcode = true;
        }
    }

    /// Go: `contentSource(s)` — the source bytes (read once, see the module docs).
    // Go: hugolib/page__content.go:contentSource
    pub fn content_source(&self) -> &Arc<Vec<u8>> {
        &self.source
    }
}

/// Go: `cachedContent` — created at page creation; immutable after capture except `scopes`.
pub struct CachedContent {
    pub pi: ContentParseInfo,
    /// Go `shortcodeState`: filled during parsing (mutable phase), read-only afterwards, so no
    /// lock is needed.
    pub shortcode_state: ShortcodeHandler,
    pub enable_emoji: bool,
    /// Go `scopes` (`maps.Cache`): one scope per (markup scope + output format name). Dynacache
    /// pattern (HUGO_LAYER.md §4.8): the entry is created outside the lock; first stored wins.
    /// The rendered results themselves are cached in the site's `PageMap` content partitions.
    pub scopes: Partition<String, Arc<CachedContentScope>>,
}

impl CachedContent {
    // Go: hugolib/page__content.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.pi.items_step2.is_empty()
    }

    /// Go: `mustSource()`.
    // Go: hugolib/page__content.go:mustSource
    pub fn must_source(&self) -> &[u8] {
        &self.pi.source
    }
}

/// Go: `(m *pageMeta) parseFrontMatter(h, pid)` — read the source, `pageparser.ParseBytes`,
/// `mapFrontMatter`.
// Go: hugolib/page__content.go:parseFrontMatter
pub fn parse_front_matter(m: &PageMeta, pid: u64) -> Result<ContentParseInfo> {
    let is_from_content_adapter = m.page_config.is_from_content_adapter;
    if is_from_content_adapter {
        return Err(Error::new(
            "neohugo-rs: content adapters (_content.gotmpl) are not supported",
        ));
    }

    let mut source_key = String::new();
    let mut open_source: Option<OpenReadSeekCloser> = None;
    if let Some(f) = &m.f {
        source_key = go_path::filepath::to_slash(f.filename()).to_string();
        let meta = f.file_info().meta().clone();
        open_source = Some(Arc::new(move || {
            let r = meta.open().map_err(|err| {
                Error::new(format!(
                    "failed to open file {}: {}",
                    go_strconv::quote(&meta.filename),
                    err.message()
                ))
            })?;
            Ok(Box::new(r) as Box<dyn nh_common::hugio::ReadSeekCloser>)
        }));
    }

    if source_key.is_empty() {
        source_key = pid.to_string();
    }

    let source = read_source_all(open_source.as_ref())?;

    let items = pp::parse_bytes(
        &source,
        ParseConfig {
            no_front_matter: is_from_content_adapter,
            ..Default::default()
        },
    )?;

    let mut pi = ContentParseInfo {
        pid,
        source_key,
        open_source,
        source: Arc::new(source),
        front_matter: None,
        has_summary_divider: false,
        pos_main_content: 0,
        has_non_markdown_shortcode: false,
        items_step1: items,
        items_step2: Vec::new(),
    };

    map_front_matter(&mut pi)?;

    Ok(pi)
}

/// Go: `(m *pageMeta) newCachedContent(h, pi)` — `newShortcodeHandler` + `parseContentFile`
/// (`mapItemsAfterFrontMatter`, which extracts shortcodes and looks their templates up in the
/// template store: capture needs the store). `enable_inline_shortcodes` and `enable_emoji` are
/// the page site's (`s.ExecHelper.Sec().EnableInlineShortcodes`, `s.conf.EnableEmoji`).
// Go: hugolib/page__content.go:newCachedContent
pub fn new_cached_content(
    m: &PageMeta,
    store: &TemplateStore,
    enable_inline_shortcodes: bool,
    enable_emoji: bool,
    pi: ContentParseInfo,
) -> Result<CachedContent> {
    let filename =
        m.f.as_ref()
            .map(|f| f.filename().to_string())
            .unwrap_or_default();

    let mut c = CachedContent {
        pi,
        shortcode_state: ShortcodeHandler::new(&filename, enable_inline_shortcodes),
        enable_emoji,
        scopes: Partition::new("scopes"),
    };

    parse_content_file(&mut c, store)?;

    Ok(c)
}

/// Go: `(c *cachedContent) parseContentFile(source)`.
// Go: hugolib/page__content.go:parseContentFile
fn parse_content_file(c: &mut CachedContent, store: &TemplateStore) -> Result<()> {
    if c.pi.open_source.is_none() {
        return Ok(());
    }

    map_items_after_front_matter(&mut c.pi, &mut c.shortcode_state, store)
}

/// Go: `(c *contentParseInfo) parseFrontMatter(it, iter, source)`.
// Go: hugolib/page__content.go:parseFrontMatter
fn parse_front_matter_item(
    c: &mut ContentParseInfo,
    it: &nh_parser::pageparser::item::Item,
    iter: &pp::Iterator<'_>,
    source: &[u8],
) -> Result<()> {
    if c.front_matter.is_some() {
        return Ok(());
    }

    let f = pp::format_from_front_matter_type(it.typ);
    match Decoder::default().unmarshal_to_map_nilable(Some(&it.val(source)), f) {
        Ok(m) => {
            c.front_matter = m;
            Ok(())
        }
        Err(err) => {
            if let Some(pos) = err.pos() {
                let mut pos = pos.clone();
                // Offset the starting position of front matter.
                let mut offset = iter.line_number(source) as i64 - 1;
                if f == Format::Yaml {
                    offset -= 1;
                }
                pos.line += offset;
                // It will be set later.
                pos.filename = String::new();
                return Err(err.at(pos));
            }
            Err(err)
        }
    }
}

/// Go: `failMap` / `mapItemsAfterFrontMatter`'s `fail`: an error without a position gets the
/// item's position in the source.
// Go: hugolib/page__content.go:failMap
fn fail_map(source: &[u8], err: Error, i: &nh_parser::pageparser::item::Item) -> Error {
    if err.pos().is_some() {
        return err;
    }

    let pos = pos_from_input("", source, i.pos() as i64);

    nh_common::herrors::new_file_error_from_pos(
        err,
        FilePos {
            filename: pos.filename,
            line: pos.line_number,
            column: pos.column_number,
        },
    )
}

/// The error of a lexer error item.
fn item_error(it: &nh_parser::pageparser::item::Item) -> Error {
    Error::new(it.err.clone().unwrap_or_default())
}

/// Go: `contentParseInfo.mapFrontMatter(source)`.
// Go: hugolib/page__content.go:mapFrontMatter
pub fn map_front_matter(pi: &mut ContentParseInfo) -> Result<()> {
    if pi.items_step1.is_empty() {
        return Ok(());
    }
    let items = pi.items_step1.clone();
    let source = pi.source.clone();
    let mut iter = pp::new_iterator(&items);

    loop {
        let it = iter.next_item();
        if it.is_front_matter() {
            parse_front_matter_item(pi, it, &iter, &source)?;
            let next = iter.peek();
            if !next.is_done() {
                pi.pos_main_content = next.pos() as i64;
            }
            // Done.
            break;
        } else if it.is_eof() {
            break;
        } else if it.is_error() {
            return Err(fail_map(&source, item_error(it), it));
        }
    }

    Ok(())
}

/// Go: `contentParseInfo.mapItemsAfterFrontMatter(source, s)`.
// Go: hugolib/page__content.go:mapItemsAfterFrontMatter
pub fn map_items_after_front_matter(
    pi: &mut ContentParseInfo,
    s: &mut ShortcodeHandler,
    store: &TemplateStore,
) -> Result<()> {
    if pi.items_step1.is_empty() {
        return Ok(());
    }

    let items = pi.items_step1.clone();
    let source = pi.source.clone();
    let source: &[u8] = &source;
    let mut iter = pp::new_iterator(&items);

    // the parser is guaranteed to return items in proper order or fail, so …
    // … it's safe to keep some "global" state
    let mut ordinal: i64 = 0;

    loop {
        let it = iter.next_item();

        if it.typ == ItemType::Ignore || it.is_front_matter() {
            // Ignore.
        } else if it.typ == ItemType::LeadSummaryDivider {
            // Go computes the position of the body after the divider (unused).
            iter.peek_walk(|item| !item.is_non_whitespace(source));

            pi.has_summary_divider = true;

            // The content may be rendered by Goldmark or similar,
            // and we need to track the summary.
            pi.add_replacement(INTERNAL_SUMMARY_DIVIDER_PRE);
        } else if it.is_left_shortcode_delim() {
            // Handle shortcode
            // let extractShortcode handle left delim (will do so recursively)
            iter.backup();

            let mut curr_shortcode = match s.extract_shortcode(store, ordinal, 0, source, &mut iter)
            {
                Ok(sc) => sc,
                Err(err) => return Err(fail_map(source, err, it)),
            };

            curr_shortcode.pos = it.pos() as i64;
            curr_shortcode.length = iter.current().pos() as i64 - it.pos() as i64;
            if curr_shortcode.placeholder.is_empty() {
                curr_shortcode.placeholder =
                    create_shortcode_placeholder("s", pi.pid, curr_shortcode.ordinal);
            }

            if !curr_shortcode.name.is_empty() {
                s.add_name(&curr_shortcode.name);
            }

            if matches!(curr_shortcode.params, Value::Invalid) {
                // Go: `var s []string; currShortcode.params = s` (a nil []string).
                curr_shortcode.params = Value::TypedNil("[]string".into());
            }

            curr_shortcode.placeholder = create_shortcode_placeholder("s", pi.pid, ordinal);
            ordinal += 1;
            let curr_shortcode = Arc::new(curr_shortcode);
            s.shortcodes.push(curr_shortcode.clone());

            pi.add_shortcode(curr_shortcode);
        } else if it.is_eof() {
            break;
        } else if it.is_error() {
            return Err(fail_map(source, item_error(it), it));
        } else {
            pi.add_bytes(it, source);
        }
    }

    Ok(())
}

/// Go: `(c *contentParseInfo) readSourceAll()` — empty (non-nil) without a source.
// Go: hugolib/page__content.go:readSourceAll
pub fn read_source_all(open_source: Option<&OpenReadSeekCloser>) -> Result<Vec<u8>> {
    let Some(open_source) = open_source else {
        return Ok(Vec::new());
    };
    let mut r = open_source()?;
    let mut b = Vec::new();
    r.read_to_end(&mut b)
        .map_err(|err| Error::new(err.to_string()))?;
    Ok(b)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__content.go (parse half; the render half is in page__content.rs)
//   types: pageContentReplacement, cachedContent, contentParseInfo
// OK L66-127: (m *pageMeta) parseFrontMatter(h *HugoSites, pid uint64) (*contentParseInfo, error)
// OK L129-154: (m *pageMeta) newCachedContent(h *HugoSites, pi *contentParseInfo) (*cachedContent, error)
// OK L211-213: (p *contentParseInfo) AddBytes(item pageparser.Item)
// OK L215-217: (p *contentParseInfo) AddReplacement(val []byte, source pageparser.Item)
// OK L219-224: (p *contentParseInfo) AddShortcode(s *shortcode)
// OK L266-268: (c *cachedContent) IsZero() bool
// OK L270-276: (c *cachedContent) parseContentFile(source []byte) error
// OK L278-307: (c *contentParseInfo) parseFrontMatter(it pageparser.Item, iter *pageparser.Iterator, source []byte) error
// OK L309-317: (rn *contentParseInfo) failMap(source []byte, err error, i pageparser.Item) error
// OK L319-349: (rn *contentParseInfo) mapFrontMatter(source []byte) error
// OK L351-445: (rn *contentParseInfo) mapItemsAfterFrontMatter( source []byte, s *shortcodeHandler, ) error
// OK L447-453: (c *cachedContent) mustSource() []byte
// OK L455-477: (c *contentParseInfo) contentSource(s resource.StaleInfo) ([]byte, error)
// OK L479-490: (c *contentParseInfo) readSourceAll() ([]byte, error)
// ---------------------------------------------------------------------------
