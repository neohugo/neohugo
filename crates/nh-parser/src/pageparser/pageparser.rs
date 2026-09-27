//! Port of `parser/pageparser/pageparser.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


use std::io::Read;

use go_value::Map;

use super::item::{Item, ItemType, Items};
use super::pagelexer::Config;
use crate::metadecoders::format::Format;
use nh_common::Result;

/// Go: `pageparser.Result` (the lexed items plus the input bytes).
pub struct ParseResult {
    pub input: Vec<u8>,
    pub items: Items,
}

impl ParseResult {
    pub fn iterator(&self) -> Iterator<'_> {
        Iterator { items: &self.items, last_pos: -1 }
    }
}

/// Go: `pageparser.ParseBytes(b, cfg)`.
// Go: parser/pageparser/pageparser.go:ParseBytes
pub fn parse_bytes(b: &[u8], cfg: Config) -> Result<Items> {
    todo!()
}

/// Go: `pageparser.ParseMain(r, cfg)` — lexes front matter, summary divider and shortcodes.
// Go: parser/pageparser/pageparser.go:ParseMain
pub fn parse_main(r: &mut dyn Read, cfg: Config) -> Result<ParseResult> {
    todo!()
}

/// Go: `pageparser.ContentFrontMatter`.
#[derive(Clone, Debug)]
pub struct ContentFrontMatter {
    pub content: Vec<u8>,
    pub front_matter: Map,
    pub front_matter_format: Format,
}

/// Go: `ParseFrontMatterAndContent(r)` (i18n/data helpers and oracles).
// Go: parser/pageparser/pageparser.go:ParseFrontMatterAndContent
pub fn parse_front_matter_and_content(r: &mut dyn Read) -> Result<ContentFrontMatter> {
    todo!()
}

/// Go: `FormatFromFrontMatterType(typ)`.
pub fn format_from_front_matter_type(typ: ItemType) -> Format {
    match typ {
        ItemType::FrontMatterJson => Format::Json,
        ItemType::FrontMatterOrg => Format::Org,
        ItemType::FrontMatterToml => Format::Toml,
        ItemType::FrontMatterYaml => Format::Yaml,
        _ => Format::Unknown,
    }
}

/// Go: `pageparser.HasShortcode(s)` (fast check used by `RenderString`).
// Go: parser/pageparser/pageparser.go:HasShortcode
pub fn has_shortcode(s: &[u8]) -> bool {
    todo!()
}

/// Go: `pageparser.Iterator`.
pub struct Iterator<'a> {
    pub(crate) items: &'a [Item],
    pub(crate) last_pos: isize,
}

impl<'a> Iterator<'a> {
    // Go: parser/pageparser/pageparser.go:Next
    pub fn next_item(&mut self) -> &'a Item { todo!() }
    pub fn current(&self) -> &'a Item { todo!() }
    pub fn backup(&mut self) { todo!() }
    pub fn pos(&self) -> isize { self.last_pos }
    pub fn is_value_next(&self) -> bool { todo!() }
    pub fn peek(&self) -> &'a Item { todo!() }
    pub fn consume(&mut self, cnt: usize) { todo!() }
    pub fn line_number(&self, source: &[u8]) -> usize { todo!() }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/pageparser.go (261 lines; 11/18 funcs executed)
//   types: Result, ContentFrontMatter, Iterator
// EX L38-48: ParseBytes(b []byte, cfg Config) (Items, error)
//    L58-92: ParseFrontMatterAndContent(r io.Reader) (ContentFrontMatter, error)
// EX L94-107: FormatFromFrontMatterType(typ ItemType) metadecoders.Format
//    L110-112: ParseMain(r io.Reader, cfg Config) (Result, error)
//    L114-120: parseSection(r io.Reader, cfg Config, start stateFunc) (Result, error)
// EX L122-126: parseBytes(b []byte, cfg Config, start stateFunc) (*pageLexer, error)
// EX L129-131: NewIterator(items Items) *Iterator
// EX L141-144: (t *Iterator) Next() Item
// EX L149-154: (t *Iterator) Current() Item
// EX L157-162: (t *Iterator) Backup()
// EX L165-167: (t *Iterator) Pos() int
// EX L170-173: (t *Iterator) IsValueNext() bool
// EX L177-179: (t *Iterator) Peek() Item
//    L183-190: (t *Iterator) PeekWalk(walkFn func(item Item) bool)
//    L194-202: (t *Iterator) Consume(cnt int)
//    L205-207: (t *Iterator) LineNumber(source []byte) int
//    L215-250: IsProbablySourceOfItems(source []byte, items Items) bool
// EX L255-261: HasShortcode(s string) bool
// ---------------------------------------------------------------------------
