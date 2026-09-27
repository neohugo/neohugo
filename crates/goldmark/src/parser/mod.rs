//! Go: github.com/yuin/goldmark@v1.7.12/parser — parsing a Markdown text
//! into an AST.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::ast::{self, Ast, NodeId, NodeKind};
use crate::goslice::GoSlice;
use crate::text::{self, Reader, new_block_reader};
use crate::util::{self, PrioritizedValue};

mod attribute;
mod atx_heading;
mod auto_link;
mod blockquote;
mod code_block;
mod code_span;
mod delimiter;
mod emphasis;
mod fcode_block;
mod html_block;
mod link;
mod link_ref;
mod list;
mod list_item;
mod paragraph;
mod raw_html;
#[doc(hidden)]
pub mod regexps;
mod setext_headings;
mod thematic_break;

pub use attribute::{Attribute, Attributes, find as attributes_find, parse_attributes};
pub use atx_heading::{
    HeadingConfig, HeadingOption, WithAutoHeadingID, WithHeadingAttribute, new_atx_heading_parser,
    with_auto_heading_id, with_heading_attribute,
};
pub use auto_link::new_auto_link_parser;
pub use blockquote::new_blockquote_parser;
pub use code_block::new_code_block_parser;
pub use code_span::new_code_span_parser;
pub use delimiter::{
    Delimiter, DelimiterBottom, DelimiterProcessor, new_delimiter, process_delimiters,
    scan_delimiter,
};
pub use emphasis::new_emphasis_parser;
pub use fcode_block::new_fenced_code_block_parser;
pub use html_block::new_html_block_parser;
pub use link::{LinkLabelState, new_link_parser};
pub use link_ref::LinkReferenceParagraphTransformer;
pub use list::new_list_parser;
pub use list_item::new_list_item_parser;
pub use paragraph::new_paragraph_parser;
pub use raw_html::new_raw_html_parser;
pub use setext_headings::new_setext_heading_parser;
pub use thematic_break::new_thematic_break_parser;

/// Go: `parser.OptionName`.
pub type OptionName = String;

/// An option value (Go `interface{}`).
pub type OptionValue = Arc<dyn Any + Send + Sync>;

/// A Reference interface represents a link reference in Markdown text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    label: Vec<u8>,
    destination: Vec<u8>,
    title: Option<Vec<u8>>,
}

// Go: parser/parser.go:NewReference
/// NewReference returns a new Reference.
pub fn new_reference(label: Vec<u8>, destination: Vec<u8>, title: Option<Vec<u8>>) -> Reference {
    Reference {
        label,
        destination,
        title,
    }
}

impl Reference {
    /// Label returns a label of the reference.
    pub fn label(&self) -> &[u8] {
        &self.label
    }

    /// Destination returns a destination(URL) of the reference.
    pub fn destination(&self) -> &[u8] {
        &self.destination
    }

    /// Title returns a title of the reference (`None` is Go's nil).
    pub fn title(&self) -> Option<&[u8]> {
        self.title.as_deref()
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Reference{{Label:{}, Destination:{}, Title:{}}}",
            String::from_utf8_lossy(&self.label),
            String::from_utf8_lossy(&self.destination),
            String::from_utf8_lossy(self.title.as_deref().unwrap_or(b""))
        )
    }
}

/// An IDs interface is a collection of the element ids.
pub trait IDs: Send {
    /// Generate generates a new element id.
    fn generate(&mut self, value: &[u8], kind: NodeKind) -> Vec<u8>;

    /// Put puts a given element id to the used ids table.
    fn put(&mut self, value: &[u8]);

    /// Downcasting support.
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
}

struct DefaultIDs {
    values: HashSet<Vec<u8>>,
}

// Go: parser/parser.go:newIDs
fn new_ids() -> Box<dyn IDs> {
    Box::new(DefaultIDs {
        values: HashSet::new(),
    })
}

impl IDs for DefaultIDs {
    // Go: parser/parser.go:ids.Generate
    fn generate(&mut self, value: &[u8], kind: NodeKind) -> Vec<u8> {
        let value = util::trim_left_space(value);
        let value = util::trim_right_space(value);
        let mut result: Vec<u8> = Vec::new();
        let mut i = 0usize;
        while i < value.len() {
            let mut v = value[i];
            let l = util::utf8_len(v);
            i += l as u8 as usize;
            if l != 1 {
                continue;
            }
            if util::is_alpha_numeric(v) {
                if v.is_ascii_uppercase() {
                    v += b'a' - b'A';
                }
                result.push(v);
            } else if util::is_space(v) || v == b'-' || v == b'_' {
                result.push(b'-');
            }
        }
        if result.is_empty() {
            if kind == ast::KIND_HEADING {
                result = b"heading".to_vec();
            } else {
                result = b"id".to_vec();
            }
        }
        if !self.values.contains(&result) {
            self.values.insert(result.clone());
            return result;
        }
        let mut i = 1;
        loop {
            let mut new_result = result.clone();
            new_result.push(b'-');
            new_result.extend_from_slice(go_strconv::itoa(i).as_bytes());
            if !self.values.contains(&new_result) {
                self.values.insert(new_result.clone());
                return new_result;
            }
            i += 1;
        }
    }

    // Go: parser/parser.go:ids.Put
    fn put(&mut self, value: &[u8]) {
        self.values.insert(value.to_vec());
    }
}

/// ContextKey is a key that is used to set arbitrary values to the context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextKey(pub usize);

static CONTEXT_KEY_MAX: AtomicUsize = AtomicUsize::new(0);

// Go: parser/parser.go:NewContextKey
/// NewContextKey return a new ContextKey value.
pub fn new_context_key() -> ContextKey {
    ContextKey(CONTEXT_KEY_MAX.fetch_add(1, Ordering::SeqCst) + 1)
}

/// A context value (Go `interface{}`).
pub type ContextValue = Box<dyn Any + Send + Sync>;

/// A Block struct holds a node and correspond parser pair.
#[derive(Clone)]
pub struct Block {
    /// Node is a BlockNode.
    pub node: NodeId,
    /// Parser is a BlockParser.
    pub parser: Arc<dyn BlockParser>,
}

/// Go's `unsafe.Sizeof(Block{})` (two interfaces), for slice growth.
const BLOCK_SIZE: usize = 32;

/// A Context holds a information that are necessary to parse Markdown
/// text (Go: the `parser.Context` interface and its `parseContext`
/// implementation; custom implementations are not supported, custom IDs are).
pub struct Context {
    store: Vec<Option<ContextValue>>,
    ids: Box<dyn IDs>,
    refs: HashMap<Vec<u8>, Reference>,
    block_offset: i64,
    block_indent: i64,
    delimiters: Option<NodeId>,
    last_delimiter: Option<NodeId>,
    opened_blocks: GoSlice<Block>,
}

/// A ContextConfig struct is a data structure that holds configuration of the Context.
pub struct ContextConfig {
    pub ids: Box<dyn IDs>,
}

/// An ContextOption is a functional option type for the Context.
pub type ContextOption = Box<dyn FnOnce(&mut ContextConfig)>;

// Go: parser/parser.go:WithIDs
/// WithIDs is a functional option for the Context.
pub fn with_ids(ids: Box<dyn IDs>) -> ContextOption {
    Box::new(move |c: &mut ContextConfig| c.ids = ids)
}

// Go: parser/parser.go:NewContext
/// NewContext returns a new Context.
pub fn new_context(options: Vec<ContextOption>) -> Context {
    let mut cfg = ContextConfig { ids: new_ids() };
    for option in options {
        option(&mut cfg);
    }
    let mut store = Vec::new();
    store.resize_with(CONTEXT_KEY_MAX.load(Ordering::SeqCst) + 1, || None);
    Context {
        store,
        refs: HashMap::new(),
        ids: cfg.ids,
        block_offset: -1,
        block_indent: -1,
        delimiters: None,
        last_delimiter: None,
        // Go: openedBlocks: []Block{} (empty, capacity 0)
        opened_blocks: GoSlice::nil(BLOCK_SIZE),
    }
}

impl Default for Context {
    fn default() -> Self {
        new_context(Vec::new())
    }
}

impl Context {
    fn slot(&mut self, key: ContextKey) -> &mut Option<ContextValue> {
        if key.0 >= self.store.len() {
            self.store.resize_with(key.0 + 1, || None);
        }
        &mut self.store[key.0]
    }

    // Go: parser/parser.go:parseContext.Get
    /// Get returns a value associated with the given key (`None` is nil).
    pub fn get(&self, key: ContextKey) -> Option<&(dyn Any + Send + Sync)> {
        self.store.get(key.0).and_then(|v| v.as_deref())
    }

    /// Get, downcast to `T`.
    pub fn get_as<T: 'static>(&self, key: ContextKey) -> Option<&T> {
        self.get(key).and_then(|v| v.downcast_ref::<T>())
    }

    /// Get, downcast to `T`, mutably.
    pub fn get_as_mut<T: 'static>(&mut self, key: ContextKey) -> Option<&mut T> {
        self.slot(key)
            .as_deref_mut()
            .and_then(|v| v.downcast_mut::<T>())
    }

    // Go: parser/parser.go:parseContext.ComputeIfAbsent
    /// ComputeIfAbsent computes a value if a value associated with the given key is absent and returns the value.
    pub fn compute_if_absent(
        &mut self,
        key: ContextKey,
        f: impl FnOnce() -> ContextValue,
    ) -> &mut (dyn Any + Send + Sync) {
        let slot = self.slot(key);
        if slot.is_none() {
            *slot = Some(f());
        }
        slot.as_deref_mut().unwrap()
    }

    // Go: parser/parser.go:parseContext.Set
    /// Set sets the given value to the context (`None` is nil).
    pub fn set(&mut self, key: ContextKey, value: Option<ContextValue>) {
        *self.slot(key) = value;
    }

    /// Set a typed value.
    pub fn set_value<T: Any + Send + Sync>(&mut self, key: ContextKey, value: T) {
        self.set(key, Some(Box::new(value)));
    }

    // Go: parser/parser.go:parseContext.IDs
    /// IDs returns a collection of the element ids.
    pub fn ids(&mut self) -> &mut dyn IDs {
        self.ids.as_mut()
    }

    // Go: parser/parser.go:parseContext.BlockOffset
    /// BlockOffset returns a first non-space character position on current line.
    /// This value is valid only for BlockParser.Open.
    /// BlockOffset returns -1 if current line is blank.
    pub fn block_offset(&self) -> i64 {
        self.block_offset
    }

    // Go: parser/parser.go:parseContext.SetBlockOffset
    /// BlockOffset sets a first non-space character position on current line.
    pub fn set_block_offset(&mut self, v: i64) {
        self.block_offset = v;
    }

    // Go: parser/parser.go:parseContext.BlockIndent
    /// BlockIndent returns an indent width on current line.
    /// BlockIndent returns -1 if current line is blank.
    pub fn block_indent(&self) -> i64 {
        self.block_indent
    }

    // Go: parser/parser.go:parseContext.SetBlockIndent
    /// BlockIndent sets an indent width on current line.
    pub fn set_block_indent(&mut self, v: i64) {
        self.block_indent = v;
    }

    // Go: parser/parser.go:parseContext.LastDelimiter
    /// LastDelimiter returns a last delimiter of the current delimiter list.
    pub fn last_delimiter(&self) -> Option<NodeId> {
        self.last_delimiter
    }

    // Go: parser/parser.go:parseContext.FirstDelimiter
    /// FirstDelimiter returns a first delimiter of the current delimiter list.
    pub fn first_delimiter(&self) -> Option<NodeId> {
        self.delimiters
    }

    // Go: parser/parser.go:parseContext.PushDelimiter
    /// PushDelimiter appends the given delimiter to the tail of the current
    /// delimiter list.
    pub fn push_delimiter(&mut self, ast: &mut Ast, d: NodeId) {
        match self.delimiters {
            None => {
                self.delimiters = Some(d);
                self.last_delimiter = Some(d);
            }
            Some(_) => {
                let l = self.last_delimiter.expect("last delimiter");
                self.last_delimiter = Some(d);
                ast.delimiter_mut(l).next_delimiter = Some(d);
                ast.delimiter_mut(d).previous_delimiter = Some(l);
            }
        }
    }

    // Go: parser/parser.go:parseContext.RemoveDelimiter
    /// RemoveDelimiter removes the given delimiter from the current delimiter list.
    pub fn remove_delimiter(&mut self, ast: &mut Ast, d: NodeId) {
        let (prev, next) = {
            let dd = ast.delimiter(d);
            (dd.previous_delimiter, dd.next_delimiter)
        };
        match prev {
            None => self.delimiters = next,
            Some(p) => {
                ast.delimiter_mut(p).next_delimiter = next;
                if let Some(n) = next {
                    ast.delimiter_mut(n).previous_delimiter = Some(p);
                }
            }
        }
        if next.is_none() {
            self.last_delimiter = prev;
        }
        if let Some(first) = self.delimiters {
            ast.delimiter_mut(first).previous_delimiter = None;
        }
        if let Some(last) = self.last_delimiter {
            ast.delimiter_mut(last).next_delimiter = None;
        }
        let dd = ast.delimiter_mut(d);
        dd.next_delimiter = None;
        dd.previous_delimiter = None;
        let (length, segment) = (dd.length, dd.segment);
        let parent = ast.parent(d).expect("delimiter has no parent");
        if length != 0 {
            ast::merge_or_replace_text_segment(ast, parent, d, segment);
        } else {
            ast.remove_child(parent, d);
        }
    }

    // Go: parser/parser.go:parseContext.ClearDelimiters
    /// ClearDelimiters clears the current delimiter list.
    pub fn clear_delimiters(&mut self, ast: &mut Ast, bottom: Option<NodeId>) {
        let Some(last) = self.last_delimiter else {
            return;
        };
        let mut c = Some(last);
        while let Some(cid) = c {
            if Some(cid) == bottom {
                break;
            }
            let prev = ast.previous_sibling(cid);
            if matches!(ast.value(cid), ast::NodeValue::Delimiter(_)) {
                self.remove_delimiter(ast, cid);
            }
            c = prev;
        }
    }

    // Go: parser/parser.go:parseContext.AddReference
    /// AddReference adds the given reference to this context.
    pub fn add_reference(&mut self, r: Reference) {
        let key = util::to_link_reference(r.label());
        self.refs.entry(key).or_insert(r);
    }

    // Go: parser/parser.go:parseContext.Reference
    /// Reference returns (a reference, true) if a reference associated with
    /// the given label exists, otherwise (nil, false).
    pub fn reference(&self, label: &[u8]) -> Option<&Reference> {
        self.refs.get(label)
    }

    // Go: parser/parser.go:parseContext.References
    /// References returns a list of references (Go returns them in map
    /// order; sorted by key here).
    pub fn references(&self) -> Vec<&Reference> {
        let mut keys: Vec<&Vec<u8>> = self.refs.keys().collect();
        keys.sort();
        keys.into_iter().map(|k| &self.refs[k]).collect()
    }

    // Go: parser/parser.go:parseContext.OpenedBlocks
    /// OpenedBlocks returns a list of nodes that are currently in parsing
    /// (a Go slice header sharing the backing array).
    pub fn opened_blocks(&self) -> GoSlice<Block> {
        self.opened_blocks.clone()
    }

    // Go: parser/parser.go:parseContext.SetOpenedBlocks
    /// SetOpenedBlocks sets a list of nodes that are currently in parsing.
    pub fn set_opened_blocks(&mut self, v: GoSlice<Block>) {
        self.opened_blocks = v;
    }

    // Go: parser/parser.go:parseContext.LastOpenedBlock
    /// LastOpenedBlock returns a last node that is currently in parsing
    /// (`None` is Go's zero `Block{}`).
    pub fn last_opened_block(&self) -> Option<Block> {
        let l = self.opened_blocks.len();
        if l != 0 {
            return Some(self.opened_blocks.get(l - 1));
        }
        None
    }

    // Go: parser/parser.go:parseContext.IsInLinkLabel
    /// IsInLinkLabel returns true if current position seems to be in link label.
    pub fn is_in_link_label(&self) -> bool {
        self.get(*link::LINK_LABEL_STATE_KEY).is_some()
    }
}

/// State represents parser's state.
/// State is designed to use as a bit flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State(pub u32);

impl State {
    /// None is a default value of the [State].
    pub const NONE: State = State(1 << 0);
    /// Continue indicates parser can continue parsing.
    pub const CONTINUE: State = State(1 << 1);
    /// Close indicates parser cannot parse anymore.
    pub const CLOSE: State = State(1 << 2);
    /// HasChildren indicates parser may have child blocks.
    pub const HAS_CHILDREN: State = State(1 << 3);
    /// NoChildren indicates parser does not have child blocks.
    pub const NO_CHILDREN: State = State(1 << 4);
    /// RequireParagraph indicates parser requires that the last node
    /// must be a paragraph and is not converted to other nodes by
    /// ParagraphTransformers.
    pub const REQUIRE_PARAGRAPH: State = State(1 << 5);

    /// `s & flag != 0`
    pub fn has(self, flag: State) -> bool {
        self.0 & flag.0 != 0
    }
}

impl std::ops::BitOr for State {
    type Output = State;
    fn bitor(self, rhs: State) -> State {
        State(self.0 | rhs.0)
    }
}

/// A Config struct is a data structure that holds configuration of the Parser.
#[derive(Default)]
pub struct Config {
    pub options: HashMap<OptionName, OptionValue>,
    pub block_parsers: Vec<PrioritizedValue<Box<dyn BlockParser>>>,
    pub inline_parsers: Vec<PrioritizedValue<Box<dyn InlineParser>>>,
    pub paragraph_transformers: Vec<PrioritizedValue<Box<dyn ParagraphTransformer>>>,
    pub ast_transformers: Vec<PrioritizedValue<Box<dyn AstTransformer>>>,
    pub escaped_space: bool,
}

// Go: parser/parser.go:NewConfig
/// NewConfig returns a new Config.
pub fn new_config() -> Config {
    Config::default()
}

/// An Option interface is a functional option type for the Parser.
pub trait ParserOption: Send {
    /// SetParserOption applies this option.
    fn set_parser_option(self: Box<Self>, c: &mut Config);
}

/// Attribute is an option name that spacify attributes of elements.
pub const OPT_ATTRIBUTE: &str = "Attribute";

struct WithAttribute;

impl ParserOption for WithAttribute {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        c.options.insert(OPT_ATTRIBUTE.to_string(), Arc::new(true));
    }
}

// Go: parser/parser.go:WithAttribute
/// WithAttribute is a functional option that enables custom attributes.
pub fn with_attribute() -> Box<dyn ParserOption> {
    Box::new(WithAttribute)
}

/// A SetOptioner sets the given option to the object (a default no-op on
/// the parser traits below; Go only calls it on types implementing it).
pub type SetOption = fn(&OptionName, &OptionValue);

/// A BlockParser interface parses a block level element like Paragraph, List,
/// Blockquote etc.
pub trait BlockParser: Send + Sync {
    /// Trigger returns a list of characters that triggers Parse method of
    /// this parser.
    /// If Trigger returns a nil, Open will be called with any lines.
    fn trigger(&self) -> Option<&[u8]>;

    /// Open parses the current line and returns a result of parsing.
    fn open<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State);

    /// Continue parses the current line and returns a result of parsing.
    fn continue_<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> State;

    /// Close will be called when the parser returns Close.
    fn close<'a>(&self, ast: &mut Ast, node: NodeId, reader: &mut dyn Reader<'a>, pc: &mut Context);

    /// CanInterruptParagraph returns true if the parser can interrupt paragraphs,
    /// otherwise false.
    fn can_interrupt_paragraph(&self) -> bool;

    /// CanAcceptIndentedLine returns true if the parser can open new node when
    /// the given line is being indented more than 3 spaces.
    fn can_accept_indented_line(&self) -> bool;

    /// SetOptioner.SetOption.
    fn set_option(&mut self, _name: &str, _value: &OptionValue) {}
}

/// An InlineParser interface parses an inline level element like CodeSpan, Link etc.
pub trait InlineParser: Send + Sync {
    /// Trigger returns a list of characters that triggers Parse method of
    /// this parser.
    /// Trigger characters must be a punctuation or a halfspace.
    /// Halfspaces triggers this parser when character is any spaces characters or
    /// a head of line
    fn trigger(&self) -> &[u8];

    /// Parse parse the given block into an inline node.
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId>;

    /// True if this parser implements CloseBlocker.
    fn is_close_blocker(&self) -> bool {
        false
    }

    /// CloseBlocker.CloseBlock: called when a block is closed in the inline
    /// parsing.
    fn close_block<'a>(
        &self,
        _ast: &mut Ast,
        _parent: NodeId,
        _block: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
    }

    /// SetOptioner.SetOption.
    fn set_option(&mut self, _name: &str, _value: &OptionValue) {}
}

/// A ParagraphTransformer transforms parsed Paragraph nodes.
/// For example, link references are searched in parsed Paragraphs.
pub trait ParagraphTransformer: Send + Sync {
    /// Transform transforms the given paragraph.
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    );

    /// SetOptioner.SetOption.
    fn set_option(&mut self, _name: &str, _value: &OptionValue) {}
}

/// ASTTransformer transforms entire Markdown document AST tree.
pub trait AstTransformer: Send + Sync {
    /// Transform transforms the given AST tree.
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    );

    /// SetOptioner.SetOption.
    fn set_option(&mut self, _name: &str, _value: &OptionValue) {}
}

// Go: parser/parser.go:DefaultBlockParsers
/// DefaultBlockParsers returns a new list of default BlockParsers.
/// Priorities of default BlockParsers are:
///
/// - SetextHeadingParser: 100
/// - ThematicBreakParser: 200
/// - ListParser: 300
/// - ListItemParser: 400
/// - CodeBlockParser: 500
/// - ATXHeadingParser: 600
/// - FencedCodeBlockParser: 700
/// - BlockquoteParser: 800
/// - HTMLBlockParser: 900
/// - ParagraphParser: 1000
pub fn default_block_parsers() -> Vec<PrioritizedValue<Box<dyn BlockParser>>> {
    vec![
        util::prioritized(new_setext_heading_parser(vec![]), 100),
        util::prioritized(new_thematic_break_parser(), 200),
        util::prioritized(new_list_parser(), 300),
        util::prioritized(new_list_item_parser(), 400),
        util::prioritized(new_code_block_parser(), 500),
        util::prioritized(new_atx_heading_parser(vec![]), 600),
        util::prioritized(new_fenced_code_block_parser(), 700),
        util::prioritized(new_blockquote_parser(), 800),
        util::prioritized(new_html_block_parser(), 900),
        util::prioritized(new_paragraph_parser(), 1000),
    ]
}

// Go: parser/parser.go:DefaultInlineParsers
/// DefaultInlineParsers returns a new list of default InlineParsers.
/// Priorities of default InlineParsers are:
///
/// - CodeSpanParser: 100
/// - LinkParser: 200
/// - AutoLinkParser: 300
/// - RawHTMLParser: 400
/// - EmphasisParser: 500
pub fn default_inline_parsers() -> Vec<PrioritizedValue<Box<dyn InlineParser>>> {
    vec![
        util::prioritized(new_code_span_parser(), 100),
        util::prioritized(new_link_parser(), 200),
        util::prioritized(new_auto_link_parser(), 300),
        util::prioritized(new_raw_html_parser(), 400),
        util::prioritized(new_emphasis_parser(), 500),
    ]
}

// Go: parser/parser.go:DefaultParagraphTransformers
/// DefaultParagraphTransformers returns a new list of default ParagraphTransformers.
/// Priorities of default ParagraphTransformers are:
///
/// - LinkReferenceParagraphTransformer: 100
pub fn default_paragraph_transformers() -> Vec<PrioritizedValue<Box<dyn ParagraphTransformer>>> {
    vec![util::prioritized(
        Box::new(LinkReferenceParagraphTransformer) as Box<dyn ParagraphTransformer>,
        100,
    )]
}

struct WithBlockParsers(Vec<PrioritizedValue<Box<dyn BlockParser>>>);

impl ParserOption for WithBlockParsers {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        c.block_parsers.extend(self.0);
    }
}

// Go: parser/parser.go:WithBlockParsers
/// WithBlockParsers is a functional option that allow you to add
/// BlockParsers to the parser.
pub fn with_block_parsers(
    bs: Vec<PrioritizedValue<Box<dyn BlockParser>>>,
) -> Box<dyn ParserOption> {
    Box::new(WithBlockParsers(bs))
}

struct WithInlineParsers(Vec<PrioritizedValue<Box<dyn InlineParser>>>);

impl ParserOption for WithInlineParsers {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        c.inline_parsers.extend(self.0);
    }
}

// Go: parser/parser.go:WithInlineParsers
/// WithInlineParsers is a functional option that allow you to add
/// InlineParsers to the parser.
pub fn with_inline_parsers(
    bs: Vec<PrioritizedValue<Box<dyn InlineParser>>>,
) -> Box<dyn ParserOption> {
    Box::new(WithInlineParsers(bs))
}

struct WithParagraphTransformers(Vec<PrioritizedValue<Box<dyn ParagraphTransformer>>>);

impl ParserOption for WithParagraphTransformers {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        c.paragraph_transformers.extend(self.0);
    }
}

// Go: parser/parser.go:WithParagraphTransformers
/// WithParagraphTransformers is a functional option that allow you to add
/// ParagraphTransformers to the parser.
pub fn with_paragraph_transformers(
    ps: Vec<PrioritizedValue<Box<dyn ParagraphTransformer>>>,
) -> Box<dyn ParserOption> {
    Box::new(WithParagraphTransformers(ps))
}

struct WithASTTransformers(Vec<PrioritizedValue<Box<dyn AstTransformer>>>);

impl ParserOption for WithASTTransformers {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        c.ast_transformers.extend(self.0);
    }
}

// Go: parser/parser.go:WithASTTransformers
/// WithASTTransformers is a functional option that allow you to add
/// ASTTransformers to the parser.
pub fn with_ast_transformers(
    ps: Vec<PrioritizedValue<Box<dyn AstTransformer>>>,
) -> Box<dyn ParserOption> {
    Box::new(WithASTTransformers(ps))
}

struct WithEscapedSpace;

impl ParserOption for WithEscapedSpace {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        c.escaped_space = true;
    }
}

// Go: parser/parser.go:WithEscapedSpace
/// WithEscapedSpace is a functional option indicates that a '\' escaped half-space(0x20) should not trigger parsers.
pub fn with_escaped_space() -> Box<dyn ParserOption> {
    Box::new(WithEscapedSpace)
}

struct WithOption(OptionName, OptionValue);

impl ParserOption for WithOption {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        c.options.insert(self.0, self.1);
    }
}

// Go: parser/parser.go:WithOption
/// WithOption is a functional option that allow you to set
/// an arbitrary option to the parser.
pub fn with_option(name: &str, value: OptionValue) -> Box<dyn ParserOption> {
    Box::new(WithOption(name.to_string(), value))
}

/// The initialized dispatch tables (Go: the `parser` struct fields filled
/// by `initSync`).
struct ParserState {
    block_parsers: Vec<Option<Vec<Arc<dyn BlockParser>>>>,
    free_block_parsers: Option<Vec<Arc<dyn BlockParser>>>,
    inline_parsers: Vec<Option<Vec<Arc<dyn InlineParser>>>>,
    close_blockers: Vec<Arc<dyn InlineParser>>,
    paragraph_transformers: Vec<Arc<dyn ParagraphTransformer>>,
    ast_transformers: Vec<Arc<dyn AstTransformer>>,
    escaped_space: bool,
}

/// A Parser interface parses Markdown text into AST nodes.
pub struct Parser {
    config: Mutex<Option<Config>>,
    state: OnceLock<ParserState>,
}

// Go: parser/parser.go:NewParser
/// NewParser returns a new Parser with given options.
pub fn new_parser(options: Vec<Box<dyn ParserOption>>) -> Parser {
    let mut config = new_config();
    for opt in options {
        opt.set_parser_option(&mut config);
    }
    Parser {
        config: Mutex::new(Some(config)),
        state: OnceLock::new(),
    }
}

/// A parsed document: the arena and the Document node.
pub struct ParseResult {
    pub ast: Ast,
    pub root: NodeId,
}

impl Parser {
    // Go: parser/parser.go:parser.AddOptions
    /// AddOption adds the given option to this parser. Options added after
    /// the first Parse are ignored (Go dereferences a nil config).
    pub fn add_options(&self, opts: Vec<Box<dyn ParserOption>>) {
        let mut cfg = self.config.lock().unwrap();
        if let Some(cfg) = cfg.as_mut() {
            for opt in opts {
                opt.set_parser_option(cfg);
            }
        }
    }

    fn state(&self) -> &ParserState {
        self.state.get_or_init(|| {
            let mut config = self.config.lock().unwrap().take().expect("parser config");
            let mut st = ParserState {
                block_parsers: vec![None; 256],
                free_block_parsers: None,
                inline_parsers: vec![None; 256],
                close_blockers: Vec::new(),
                paragraph_transformers: Vec::new(),
                ast_transformers: Vec::new(),
                escaped_space: false,
            };
            util::sort_prioritized(&mut config.block_parsers);
            for v in config.block_parsers.drain(..) {
                add_block_parser(&mut st, v, &config.options);
            }
            let free = st.free_block_parsers.clone();
            for bp in st.block_parsers.iter_mut() {
                if let Some(v) = bp.as_mut()
                    && let Some(free) = &free
                {
                    v.extend(free.iter().cloned());
                }
            }

            util::sort_prioritized(&mut config.inline_parsers);
            for v in config.inline_parsers.drain(..) {
                add_inline_parser(&mut st, v, &config.options);
            }
            util::sort_prioritized(&mut config.paragraph_transformers);
            for v in config.paragraph_transformers.drain(..) {
                let mut pt = v.value;
                for (oname, ovalue) in &config.options {
                    pt.set_option(oname, ovalue);
                }
                st.paragraph_transformers.push(Arc::from(pt));
            }
            util::sort_prioritized(&mut config.ast_transformers);
            for v in config.ast_transformers.drain(..) {
                let mut at = v.value;
                for (oname, ovalue) in &config.options {
                    at.set_option(oname, ovalue);
                }
                st.ast_transformers.push(Arc::from(at));
            }
            st.escaped_space = config.escaped_space;
            st
        })
    }

    // Go: parser/parser.go:parser.Parse
    /// Parse parses the given Markdown text into AST nodes, with a new Context.
    pub fn parse<'a>(&self, reader: &mut dyn Reader<'a>) -> ParseResult {
        let mut pc = new_context(Vec::new());
        self.parse_with_context(reader, &mut pc)
    }

    // Go: parser/parser.go:parser.Parse with parser.WithContext
    /// Parse with the given context.
    pub fn parse_with_context<'a>(
        &self,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> ParseResult {
        let st = self.state();
        let mut ast = Ast::new();
        let root = ast.new_document();
        parse_blocks(st, &mut ast, root, reader, pc);

        let mut block_reader = new_block_reader(reader.source(), None);
        walk_block(&mut ast, root, &mut |ast, node| {
            parse_block(st, ast, &mut block_reader, node, pc);
        });
        for at in &st.ast_transformers {
            at.transform(&mut ast, root, reader, pc);
        }

        ParseResult { ast, root }
    }
}

// Go: parser/parser.go:parser.addBlockParser
fn add_block_parser(
    st: &mut ParserState,
    v: PrioritizedValue<Box<dyn BlockParser>>,
    options: &HashMap<OptionName, OptionValue>,
) {
    let mut bp = v.value;
    for (oname, ovalue) in options {
        bp.set_option(oname, ovalue);
    }
    let tcs = bp.trigger().map(|t| t.to_vec());
    let bp: Arc<dyn BlockParser> = Arc::from(bp);
    match tcs {
        None => st.free_block_parsers.get_or_insert_with(Vec::new).push(bp),
        Some(tcs) => {
            for tc in tcs {
                st.block_parsers[tc as usize]
                    .get_or_insert_with(Vec::new)
                    .push(bp.clone());
            }
        }
    }
}

// Go: parser/parser.go:parser.addInlineParser
fn add_inline_parser(
    st: &mut ParserState,
    v: PrioritizedValue<Box<dyn InlineParser>>,
    options: &HashMap<OptionName, OptionValue>,
) {
    let mut ip = v.value;
    for (oname, ovalue) in options {
        ip.set_option(oname, ovalue);
    }
    let tcs = ip.trigger().to_vec();
    let ip: Arc<dyn InlineParser> = Arc::from(ip);
    if ip.is_close_blocker() {
        st.close_blockers.push(ip.clone());
    }
    for tc in tcs {
        st.inline_parsers[tc as usize]
            .get_or_insert_with(Vec::new)
            .push(ip.clone());
    }
}

// Go: parser/parser.go:parser.transformParagraph
fn transform_paragraph<'a>(
    st: &ParserState,
    ast: &mut Ast,
    node: NodeId,
    reader: &mut dyn Reader<'a>,
    pc: &mut Context,
) -> bool {
    for pt in &st.paragraph_transformers {
        pt.transform(ast, node, reader, pc);
        if ast.parent(node).is_none() {
            return true;
        }
    }
    false
}

// Go: parser/parser.go:parser.closeBlocks
fn close_blocks<'a>(
    st: &ParserState,
    ast: &mut Ast,
    from: i64,
    to: i64,
    reader: &mut dyn Reader<'a>,
    pc: &mut Context,
) {
    let blocks = pc.opened_blocks();
    let mut i = from;
    while i >= to {
        let b = blocks.get(i as usize);
        let node = b.node;
        if ast.is_paragraph(Some(node)) && ast.parent(node).is_some() {
            transform_paragraph(st, ast, node, reader, pc);
        }
        if ast.parent(node).is_some() {
            // closes only if node has not been transformed
            b.parser.close(ast, b.node, reader, pc);
        }
        i -= 1;
    }
    assert!(to >= 0, "slice bounds out of range");
    let blocks = if from == blocks.len() as i64 - 1 {
        blocks.slice(0, to as usize)
    } else {
        let tail = blocks.slice((from + 1) as usize, blocks.len());
        blocks.slice(0, to as usize).append_slice(&tail)
    };
    pc.set_opened_blocks(blocks);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockOpenResult {
    ParagraphContinuation = 1,
    NewBlocksOpened = 2,
    NoBlocksOpened = 3,
}

// Go: parser/parser.go:parser.openBlocks
fn open_blocks<'a>(
    st: &ParserState,
    ast: &mut Ast,
    mut parent: NodeId,
    blank_line: bool,
    reader: &mut dyn Reader<'a>,
    pc: &mut Context,
) -> BlockOpenResult {
    let mut result = BlockOpenResult::NoBlocksOpened;
    let mut continuable = false;
    let mut last_block = pc.last_opened_block();
    if let Some(lb) = &last_block {
        continuable = ast.is_paragraph(Some(lb.node));
    }
    'retry: loop {
        let (line, _) = reader.peek_line();
        let line_offset = reader.line_offset();
        let lb: &[u8] = line.as_deref().unwrap_or(&[]);
        let (w, pos) = util::indent_width(lb, line_offset);
        if w >= lb.len() as i64 {
            pc.set_block_offset(-1);
            pc.set_block_indent(-1);
        } else {
            pc.set_block_offset(pos);
            pc.set_block_indent(w);
        }
        if line.is_none() || lb[0] == b'\n' {
            break 'retry; // goto continuable
        }
        let mut bps = st.free_block_parsers.as_ref();
        if pos < lb.len() as i64 {
            bps = st.block_parsers[lb[pos as usize] as usize].as_ref();
            if bps.is_none() {
                bps = st.free_block_parsers.as_ref();
            }
        }
        let Some(bps) = bps else {
            break 'retry; // goto continuable
        };

        for bp in bps.iter() {
            if continuable
                && result == BlockOpenResult::NoBlocksOpened
                && !bp.can_interrupt_paragraph()
            {
                continue;
            }
            if w > 3 && !bp.can_accept_indented_line() {
                continue;
            }
            last_block = pc.last_opened_block();
            let last = last_block.as_ref().map(|b| b.node);
            let (node, state) = bp.open(ast, parent, reader, pc);
            if let Some(node) = node {
                // Parser requires last node to be a paragraph.
                if state.has(State::REQUIRE_PARAGRAPH) && last == ast.last_child(parent) {
                    // Opened paragraph may be transformed by ParagraphTransformers in
                    // closeBlocks().
                    let lbk = last_block
                        .as_ref()
                        .expect("RequireParagraph without an opened block");
                    let last = last.unwrap();
                    lbk.parser.close(ast, last, reader, pc);
                    let blocks = pc.opened_blocks();
                    pc.set_opened_blocks(blocks.slice(0, blocks.len() - 1));
                    assert!(
                        ast.is_paragraph(Some(last)),
                        "interface conversion: not a *ast.Paragraph"
                    );
                    if transform_paragraph(st, ast, last, reader, pc) {
                        // Paragraph has been transformed.
                        // So this parser is considered as failing.
                        continuable = false;
                        continue 'retry;
                    }
                }
                ast.set_blank_previous_lines(node, blank_line);
                if let Some(last) = last
                    && ast.parent(last).is_none()
                {
                    let last_pos = pc.opened_blocks().len() as i64 - 1;
                    close_blocks(st, ast, last_pos, last_pos, reader, pc);
                }
                ast.append_child(parent, node);
                result = BlockOpenResult::NewBlocksOpened;
                let be = Block {
                    node,
                    parser: bp.clone(),
                };
                let ob = pc.opened_blocks().append(be);
                pc.set_opened_blocks(ob);
                if state.has(State::HAS_CHILDREN) {
                    parent = node;
                    continue 'retry; // try child block
                }
                break; // no children, can not open more blocks on this line
            }
        }
        break 'retry;
    }

    // continuable:
    if result == BlockOpenResult::NoBlocksOpened && continuable {
        let lb = last_block.as_ref().expect("last block");
        let state = lb.parser.continue_(ast, lb.node, reader, pc);
        if state.has(State::CONTINUE) {
            result = BlockOpenResult::ParagraphContinuation;
        }
    }
    result
}

#[derive(Debug, Clone, Copy)]
struct LineStat {
    line_num: i64,
    level: i64,
    is_blank: bool,
}

// Go: parser/parser.go:isBlankLine
fn is_blank_line(line_num: i64, level: i64, stats: &[LineStat]) -> bool {
    let l = stats.len() as i64;
    if l == 0 {
        return true;
    }
    let mut i = l - 1 - level;
    while i >= 0 {
        let s = stats[i as usize];
        if s.line_num == line_num && s.level <= level {
            return s.is_blank;
        } else if s.line_num < line_num {
            break;
        }
        i -= 1;
    }
    false
}

// Go: parser/parser.go:parser.parseBlocks
fn parse_blocks<'a>(
    st: &ParserState,
    ast: &mut Ast,
    parent: NodeId,
    reader: &mut dyn Reader<'a>,
    pc: &mut Context,
) {
    pc.set_opened_blocks(GoSlice::nil(BLOCK_SIZE));
    let mut blank_lines: Vec<LineStat> = Vec::with_capacity(128);
    loop {
        // process blocks separated by blank lines
        let (_, _, ok) = reader.skip_blank_lines();
        if !ok {
            return;
        }
        // first, we try to open blocks
        if open_blocks(st, ast, parent, true, reader, pc) != BlockOpenResult::NewBlocksOpened {
            return;
        }
        reader.advance_line();
        blank_lines.clear();
        loop {
            // process opened blocks line by line
            let opened_blocks = pc.opened_blocks();
            let l = opened_blocks.len() as i64;
            if l == 0 {
                break;
            }
            let mut last_index = l - 1;
            let mut i: i64 = 0;
            while i < l {
                let be = opened_blocks.get(i as usize);
                let (line, _) = reader.peek_line();
                let Some(line) = line else {
                    close_blocks(st, ast, last_index, 0, reader, pc);
                    reader.advance_line();
                    return;
                };
                let (line_num, _) = reader.position();
                blank_lines.push(LineStat {
                    line_num,
                    level: i,
                    is_blank: util::is_blank(&line),
                });
                // If node is a paragraph, p.openBlocks determines whether it is continuable.
                // So we do not process paragraphs here.
                if !ast.is_paragraph(Some(be.node)) {
                    let state = be.parser.continue_(ast, be.node, reader, pc);
                    if state.has(State::CONTINUE) {
                        // When current node is a container block and has no children,
                        // we try to open new child nodes
                        if state.has(State::HAS_CHILDREN) && i == last_index {
                            let is_blank = is_blank_line(line_num - 1, i + 1, &blank_lines);
                            open_blocks(st, ast, be.node, is_blank, reader, pc);
                            break;
                        }
                        i += 1;
                        continue;
                    }
                }
                // current node may be closed or lazy continuation
                let is_blank = is_blank_line(line_num - 1, i, &blank_lines);
                let mut this_parent = parent;
                if i != 0 {
                    this_parent = opened_blocks.get((i - 1) as usize).node;
                }
                let last_node = opened_blocks.get(last_index as usize).node;
                let result = open_blocks(st, ast, this_parent, is_blank, reader, pc);
                if result != BlockOpenResult::ParagraphContinuation {
                    // lastNode is a paragraph and was transformed by the paragraph
                    // transformers.
                    if opened_blocks.get(last_index as usize).node != last_node {
                        last_index -= 1;
                    }
                    close_blocks(st, ast, last_index, i, reader, pc);
                }
                break;
            }

            reader.advance_line();
        }
    }
}

// Go: parser/parser.go:parser.walkBlock (post-order; an explicit stack
// instead of recursion, reading FirstChild before and NextSibling after
// each child's walk as the recursive Go code does)
fn walk_block(ast: &mut Ast, block: NodeId, cb: &mut dyn FnMut(&mut Ast, NodeId)) {
    // (node, child being walked; None = first child not read yet)
    let mut stack: Vec<(NodeId, Option<NodeId>)> = vec![(block, None)];
    while let Some(top) = stack.last_mut() {
        let next = match top.1 {
            None => ast.first_child(top.0),
            Some(c) => ast.next_sibling(c),
        };
        match next {
            Some(c) => {
                top.1 = Some(c);
                stack.push((c, None));
            }
            None => {
                let node = top.0;
                stack.pop();
                cb(ast, node);
            }
        }
    }
}

const LINE_BREAK_HARD: u8 = 1 << 0;
const LINE_BREAK_SOFT: u8 = 1 << 1;
const LINE_BREAK_VISIBLE: u8 = 1 << 2;

// Go: parser/parser.go:parser.parseBlock
fn parse_block<'a>(
    st: &ParserState,
    ast: &mut Ast,
    block: &mut dyn Reader<'a>,
    parent: NodeId,
    pc: &mut Context,
) {
    if ast.is_raw(parent) {
        return;
    }
    let mut escaped = false;
    let source = block.source();
    block.reset(&ast.lines(parent).clone());
    'retry: loop {
        let (line, _) = block.peek_line();
        let Some(line) = line else {
            break;
        };
        let mut line_length = line.len();
        let mut line_break_flags: u8 = 0;
        let has_new_line = line[line_length - 1] == b'\n';
        if ((line_length >= 3 && line[line_length - 2] == b'\\' && line[line_length - 3] != b'\\')
            || (line_length == 2 && line[line_length - 2] == b'\\'))
            && has_new_line
        {
            // ends with \\n
            line_length -= 2;
            line_break_flags |= LINE_BREAK_HARD | LINE_BREAK_VISIBLE;
        } else if ((line_length >= 4
            && line[line_length - 3] == b'\\'
            && line[line_length - 2] == b'\r'
            && line[line_length - 4] != b'\\')
            || (line_length == 3
                && line[line_length - 3] == b'\\'
                && line[line_length - 2] == b'\r'))
            && has_new_line
        {
            // ends with \\r\n
            line_length -= 3;
            line_break_flags |= LINE_BREAK_HARD | LINE_BREAK_VISIBLE;
        } else if line_length >= 3
            && line[line_length - 3] == b' '
            && line[line_length - 2] == b' '
            && has_new_line
        {
            // ends with [space][space]\n
            line_length -= 3;
            line_break_flags |= LINE_BREAK_HARD;
        } else if line_length >= 4
            && line[line_length - 4] == b' '
            && line[line_length - 3] == b' '
            && line[line_length - 2] == b'\r'
            && has_new_line
        {
            // ends with [space][space]\r\n
            line_length -= 4;
            line_break_flags |= LINE_BREAK_HARD;
        } else if has_new_line {
            // If the line ends with a newline character, but it is not a hardlineBreak, then it is a softLinebreak
            // If the line ends with a hardlineBreak, then it cannot end with a softLinebreak
            // See https://spec.commonmark.org/0.30/#soft-line-breaks
            line_break_flags |= LINE_BREAK_SOFT;
        }

        let (l, mut start_position) = block.position();
        let mut n: i64 = 0;
        let mut i = 0usize;
        while i < line_length {
            let c = line[i];
            if c == b'\n' {
                break;
            }
            let is_space = util::is_space(c) && c != b'\r' && c != b'\n';
            let is_punct = util::is_punct(c);
            if (is_punct && !escaped) || is_space && !(escaped && st.escaped_space) || i == 0 {
                let mut parser_char = c;
                if is_space || (i == 0 && !is_punct) {
                    parser_char = b' ';
                }
                if let Some(ips) = &st.inline_parsers[parser_char as usize] {
                    block.advance(n);
                    n = 0;
                    let (saved_line, saved_position) = block.position();
                    if i != 0 {
                        let (_, current_position) = block.position();
                        ast::merge_or_append_text_segment(
                            ast,
                            parent,
                            start_position.between(current_position),
                        );
                        (_, start_position) = block.position();
                    }
                    let mut inline_node: Option<NodeId> = None;
                    for ip in ips {
                        inline_node = ip.parse(ast, parent, block, pc);
                        if inline_node.is_some() {
                            break;
                        }
                        block.set_position(saved_line, saved_position);
                    }
                    if let Some(inline_node) = inline_node {
                        ast.append_child(parent, inline_node);
                        continue 'retry;
                    }
                }
            }
            if escaped {
                escaped = false;
                n += 1;
                i += 1;
                continue;
            }

            if c == b'\\' {
                escaped = true;
                n += 1;
                i += 1;
                continue;
            }

            escaped = false;
            n += 1;
            i += 1;
        }
        if n != 0 {
            block.advance(n);
        }
        let (current_l, current_position) = block.position();
        if l != current_l {
            continue;
        }
        let diff = start_position.between(current_position);
        let text = if line_break_flags & (LINE_BREAK_HARD | LINE_BREAK_VISIBLE)
            == LINE_BREAK_HARD | LINE_BREAK_VISIBLE
        {
            ast.new_text_segment(diff)
        } else {
            ast.new_text_segment(diff.trim_right_space(source))
        };
        {
            let t = ast.text_node_mut(text).unwrap();
            t.set_soft_line_break(line_break_flags & LINE_BREAK_SOFT != 0);
            t.set_hard_line_break(line_break_flags & LINE_BREAK_HARD != 0);
        }
        ast.append_child(parent, text);
        block.advance_line();
    }

    process_delimiters(ast, DelimiterBottom::Nil, pc);
    for ip in &st.close_blockers {
        ip.close_block(ast, parent, block, pc);
    }
}

/// Re-exports for extension authors.
pub use text::FindClosureOptions;
