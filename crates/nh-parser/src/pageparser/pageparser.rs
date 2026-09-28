//! Port of `parser/pageparser/pageparser.go`.
//!
//! Owner: Wave B task T03 (parser-langs).

use std::io::Read;
use std::sync::OnceLock;

use go_value::Map;

use super::item::{Item, ItemType, Items};
use super::pagelexer::{Config, PageLexer, StateFn, lex_main_section};
use super::pagelexer_intro::lex_intro_section;
use crate::metadecoders::decoder::Decoder;
use crate::metadecoders::format::Format;
use nh_common::{Error, Result};

/// Go: `pageparser.Result` (the lexed items plus the input bytes).
pub struct ParseResult {
    pub input: Vec<u8>,
    pub items: Items,
}

impl ParseResult {
    // Go: parser/pageparser/pagelexer.go:Iterator
    pub fn iterator(&self) -> Iterator<'_> {
        new_iterator(&self.items)
    }

    // Go: parser/pageparser/pagelexer.go:Input
    pub fn input(&self) -> &[u8] {
        &self.input
    }
}

/// Go: `pageparser.ParseBytes(b, cfg)`.
// Go: parser/pageparser/pageparser.go:ParseBytes
pub fn parse_bytes(b: &[u8], cfg: Config) -> Result<Items> {
    let start_lexer = if cfg.no_front_matter {
        StateFn(lex_main_section)
    } else {
        StateFn(lex_intro_section)
    };
    let l = parse_bytes_with(b.to_vec(), cfg, start_lexer)?;
    // Go: `return l.items, l.err` (the lexer never sets err).
    if let Some(err) = l.err {
        return Err(Error::new(err));
    }
    Ok(l.items)
}

/// Go: `pageparser.ContentFrontMatter`.
#[derive(Clone, Debug)]
pub struct ContentFrontMatter {
    /// Go's `Content` (nil when no front matter item was found: empty here).
    pub content: Vec<u8>,
    /// Go's `FrontMatter` (empty when Go's map is nil, see `front_matter_nil`).
    pub front_matter: Map,
    /// Go's `FrontMatter == nil` (a `null` YAML/JSON front matter).
    pub front_matter_nil: bool,
    pub front_matter_format: Format,
}

/// Go: `ParseFrontMatterAndContent(r)` (i18n/data helpers and oracles). A convenience method to
/// extract front matter and content from a content page.
// Go: parser/pageparser/pageparser.go:ParseFrontMatterAndContent
pub fn parse_front_matter_and_content(r: &mut dyn Read) -> Result<ContentFrontMatter> {
    let mut input = Vec::new();
    if let Err(err) = r.read_to_end(&mut input) {
        return Err(Error::new(format!("failed to read page content: {err}")));
    }
    parse_front_matter_and_content_bytes(&input)
}

/// [`parse_front_matter_and_content`] over bytes already read.
// Go: parser/pageparser/pageparser.go:ParseFrontMatterAndContent
pub fn parse_front_matter_and_content_bytes(input: &[u8]) -> Result<ContentFrontMatter> {
    let mut front_matter_format = Format::Unknown;
    let mut content: Vec<u8> = Vec::new();

    let psr = parse_bytes(input, Config::default())?;

    let mut front_matter_source: Option<Vec<u8>> = None;

    let iter = new_iterator(&psr);

    iter.peek_walk(|item| {
        if front_matter_source.is_some() {
            // The rest is content.
            content = input[item.low..].to_vec();
            // Done
            return false;
        } else if item.is_front_matter() {
            front_matter_format = format_from_front_matter_type(item.typ);
            front_matter_source = Some(item.val(input).into_owned());
        }
        true
    });

    let front_matter = Decoder::default()
        .unmarshal_to_map_nilable(front_matter_source.as_deref(), front_matter_format);
    match front_matter {
        Ok(front_matter) => Ok(ContentFrontMatter {
            content,
            front_matter_nil: front_matter.is_none(),
            front_matter: front_matter.unwrap_or_else(|| Map::new(go_value::MapType::StringAny)),
            front_matter_format,
        }),
        Err(e) => Err(e),
    }
}

/// Go: `FormatFromFrontMatterType(typ)`.
// Go: parser/pageparser/pageparser.go:FormatFromFrontMatterType
pub fn format_from_front_matter_type(typ: ItemType) -> Format {
    match typ {
        ItemType::FrontMatterJson => Format::Json,
        ItemType::FrontMatterOrg => Format::Org,
        ItemType::FrontMatterToml => Format::Toml,
        ItemType::FrontMatterYaml => Format::Yaml,
        _ => Format::Unknown,
    }
}

/// Go: `pageparser.ParseMain(r, cfg)` — parses starting with the main section. Used in tests.
// Go: parser/pageparser/pageparser.go:ParseMain
pub fn parse_main(r: &mut dyn Read, cfg: Config) -> Result<ParseResult> {
    parse_section(r, cfg, StateFn(lex_main_section))
}

// Go: parser/pageparser/pageparser.go:parseSection
fn parse_section(r: &mut dyn Read, cfg: Config, start: StateFn) -> Result<ParseResult> {
    let mut b = Vec::new();
    if let Err(err) = r.read_to_end(&mut b) {
        return Err(Error::new(format!("failed to read page content: {err}")));
    }
    let l = parse_bytes_with(b, cfg, start)?;
    Ok(ParseResult {
        input: l.input,
        items: l.items,
    })
}

/// [`parse_main`] over bytes.
pub fn parse_main_bytes(b: &[u8], cfg: Config) -> ParseResult {
    let l = parse_bytes_with(b.to_vec(), cfg, StateFn(lex_main_section))
        .unwrap_or_else(|_| unreachable!("parseBytes never fails"));
    ParseResult {
        input: l.input,
        items: l.items,
    }
}

// Go: parser/pageparser/pageparser.go:parseBytes
fn parse_bytes_with(b: Vec<u8>, cfg: Config, start: StateFn) -> Result<PageLexer> {
    let mut lexer = PageLexer::new(b, start, cfg);
    lexer.run();
    Ok(lexer)
}

/// NewIterator creates a new Iterator.
// Go: parser/pageparser/pageparser.go:NewIterator
pub fn new_iterator(items: &[Item]) -> Iterator<'_> {
    Iterator {
        items,
        last_pos: -1,
    }
}

/// Go: `pageparser.Iterator`. An Iterator has methods to iterate a parsed page with support
/// going back if needed.
pub struct Iterator<'a> {
    pub(crate) items: &'a [Item],
    /// position of the last item returned by nextItem
    pub(crate) last_pos: isize,
}

/// Go: `errIndexOutOfBounds`.
fn err_index_out_of_bounds() -> &'static Item {
    static ITEM: OnceLock<Item> = OnceLock::new();
    ITEM.get_or_init(|| {
        let mut it = Item::new(ItemType::Error, 0, 0);
        it.err = Some("no more tokens".to_string());
        it.err_bytes = Some(b"no more tokens".to_vec());
        it
    })
}

impl<'a> Iterator<'a> {
    /// consumes and returns the next item
    // Go: parser/pageparser/pageparser.go:Next
    pub fn next_item(&mut self) -> &'a Item {
        self.last_pos += 1;
        self.current()
    }

    /// Current will repeatably return the current item.
    ///
    /// Go indexes `items[-1]` (a panic) before the first `Next`; so does this.
    // Go: parser/pageparser/pageparser.go:Current
    pub fn current(&self) -> &'a Item {
        if self.last_pos >= self.items.len() as isize {
            return err_index_out_of_bounds();
        }
        &self.items[usize::try_from(self.last_pos).expect("index out of range [-1]")]
    }

    /// backs up one token.
    // Go: parser/pageparser/pageparser.go:Backup
    pub fn backup(&mut self) {
        if self.last_pos < 0 {
            panic!("need to go forward before going back");
        }
        self.last_pos -= 1;
    }

    /// Pos returns the current position in the input.
    // Go: parser/pageparser/pageparser.go:Pos
    pub fn pos(&self) -> isize {
        self.last_pos
    }

    /// check for non-error and non-EOF types coming next
    // Go: parser/pageparser/pageparser.go:IsValueNext
    pub fn is_value_next(&self) -> bool {
        let i = self.peek();
        i.typ != ItemType::Error && i.typ != ItemType::Eof
    }

    /// look at, but do not consume, the next item
    /// repeated, sequential calls will return the same item
    // Go: parser/pageparser/pageparser.go:Peek
    pub fn peek(&self) -> &'a Item {
        &self.items[(self.last_pos + 1) as usize]
    }

    /// PeekWalk will feed the next items in the iterator to walkFn
    /// until it returns false.
    // Go: parser/pageparser/pageparser.go:PeekWalk
    pub fn peek_walk(&self, mut walk_fn: impl FnMut(&'a Item) -> bool) {
        let mut i = (self.last_pos + 1) as usize;
        while i < self.items.len() {
            let item = &self.items[i];
            if !walk_fn(item) {
                break;
            }
            i += 1;
        }
    }

    /// Consume is a convenience method to consume the next n tokens,
    /// but back off Errors and EOF.
    // Go: parser/pageparser/pageparser.go:Consume
    pub fn consume(&mut self, cnt: usize) {
        for _ in 0..cnt {
            let token = self.next_item();
            if token.typ == ItemType::Error || token.typ == ItemType::Eof {
                self.backup();
                break;
            }
        }
    }

    /// LineNumber returns the current line number. Used for logging.
    // Go: parser/pageparser/pageparser.go:LineNumber
    pub fn line_number(&self, source: &[u8]) -> usize {
        go_unicode::bytes::count(&source[..self.current().low], b"\n") + 1
    }
}

/// IsProbablySourceOfItems returns true if the given source looks like original
/// source of the items.
/// There may be some false positives, but that is highly unlikely and good enough
/// for the planned purpose.
/// It will also return false if the last item is not EOF (error situations) and
/// true if both source and items are empty.
// Go: parser/pageparser/pageparser.go:IsProbablySourceOfItems
pub fn is_probably_source_of_items(source: &[u8], items: &[Item]) -> bool {
    if source.is_empty() && items.is_empty() {
        return false;
    }
    if items.is_empty() {
        return false;
    }

    let last = &items[items.len() - 1];
    if last.typ != ItemType::Eof {
        return false;
    }

    if last.pos() != source.len() {
        return false;
    }

    for item in items {
        if item.typ == ItemType::Error {
            return false;
        }
        if item.typ == ItemType::Eof {
            return true;
        }

        if item.pos() >= source.len() {
            return false;
        }

        if item.first_byte != source[item.pos()] {
            return false;
        }
    }

    true
}

/// Go: `pageparser.HasShortcode(s)` (fast check used by `RenderString`).
// Go: parser/pageparser/pageparser.go:HasShortcode
pub fn has_shortcode(s: &[u8]) -> bool {
    // Fast path for the common case.
    if !go_unicode::bytes::contains(s, b"{{") {
        return false;
    }
    has_shortcode_re(s)
}

/// Go: `hasShortcodeRe = {{[%,<][^\/]` (unanchored; `[^\/]` matches any rune but `/`,
/// including an invalid byte, which RE2 reads as U+FFFD; it needs a rune to be present).
fn has_shortcode_re(s: &[u8]) -> bool {
    s.windows(4)
        .any(|w| w[0] == b'{' && w[1] == b'{' && matches!(w[2], b'%' | b',' | b'<') && w[3] != b'/')
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/pageparser.go (261 lines; 11/18 funcs executed)
//   types: Result, ContentFrontMatter, Iterator
// OK L38-48: ParseBytes(b []byte, cfg Config) (Items, error)
// OK L58-92: ParseFrontMatterAndContent(r io.Reader) (ContentFrontMatter, error)
// OK L94-107: FormatFromFrontMatterType(typ ItemType) metadecoders.Format
// OK L110-112: ParseMain(r io.Reader, cfg Config) (Result, error)
// OK L114-120: parseSection(r io.Reader, cfg Config, start stateFunc) (Result, error)
// OK L122-126: parseBytes(b []byte, cfg Config, start stateFunc) (*pageLexer, error)
// OK L129-131: NewIterator(items Items) *Iterator
// OK L141-144: (t *Iterator) Next() Item
// OK L149-154: (t *Iterator) Current() Item
// OK L157-162: (t *Iterator) Backup()
// OK L165-167: (t *Iterator) Pos() int
// OK L170-173: (t *Iterator) IsValueNext() bool
// OK L177-179: (t *Iterator) Peek() Item
// OK L183-190: (t *Iterator) PeekWalk(walkFn func(item Item) bool)
// OK L194-202: (t *Iterator) Consume(cnt int)
// OK L205-207: (t *Iterator) LineNumber(source []byte) int
// OK L215-250: IsProbablySourceOfItems(source []byte, items Items) bool
// OK L255-261: HasShortcode(s string) bool
// ---------------------------------------------------------------------------
