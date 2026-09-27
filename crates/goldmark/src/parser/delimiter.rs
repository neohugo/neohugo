// Go: github.com/yuin/goldmark@v1.7.12/parser/delimiter.go

use std::fmt;
use std::sync::Arc;

use go_unicode::Rune;

use super::Context;
use crate::ast::{Ast, NodeId, NodeValue};
use crate::text::Segment;
use crate::util;

/// A DelimiterProcessor interface provides a set of functions about
/// Delimiter nodes.
pub trait DelimiterProcessor: Send + Sync {
    /// IsDelimiter returns true if given character is a delimiter, otherwise false.
    fn is_delimiter(&self, b: u8) -> bool;

    /// CanOpenCloser returns true if given opener can close given closer, otherwise false.
    fn can_open_closer(&self, opener: &Delimiter, closer: &Delimiter) -> bool;

    /// OnMatch will be called when new matched delimiter found.
    /// OnMatch should return a new Node correspond to the matched delimiter.
    fn on_match(&self, ast: &mut Ast, consumes: i64) -> NodeId;
}

/// A Delimiter struct represents a delimiter like '*' of the Markdown text.
#[derive(Clone)]
pub struct Delimiter {
    pub segment: Segment,

    /// CanOpen is set true if this delimiter can open a span for a new node.
    /// See https://spec.commonmark.org/0.30/#can-open-emphasis for details.
    pub can_open: bool,

    /// CanClose is set true if this delimiter can close a span for a new node.
    /// See https://spec.commonmark.org/0.30/#can-open-emphasis for details.
    pub can_close: bool,

    /// Length is a remaining length of this delimiter.
    pub length: i64,

    /// OriginalLength is a original length of this delimiter.
    pub original_length: i64,

    /// Char is a character of this delimiter.
    pub char: u8,

    /// PreviousDelimiter is a previous sibling delimiter node of this delimiter.
    pub previous_delimiter: Option<NodeId>,

    /// NextDelimiter is a next sibling delimiter node of this delimiter.
    pub next_delimiter: Option<NodeId>,

    /// Processor is a DelimiterProcessor associated with this delimiter.
    pub processor: Arc<dyn DelimiterProcessor>,
}

impl fmt::Debug for Delimiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Delimiter")
            .field("segment", &self.segment)
            .field("can_open", &self.can_open)
            .field("can_close", &self.can_close)
            .field("length", &self.length)
            .field("original_length", &self.original_length)
            .field("char", &(self.char as char))
            .field("previous_delimiter", &self.previous_delimiter)
            .field("next_delimiter", &self.next_delimiter)
            .finish()
    }
}

impl Delimiter {
    // Go: parser/delimiter.go:Delimiter.ConsumeCharacters
    /// ConsumeCharacters consumes delimiters.
    pub fn consume_characters(&mut self, n: i64) {
        self.length -= n;
        self.segment = self.segment.with_stop(self.segment.start + self.length);
    }

    // Go: parser/delimiter.go:Delimiter.CalcComsumption
    /// CalcComsumption calculates how many characters should be used for opening
    /// a new span correspond to given closer.
    pub fn calc_comsumption(&self, closer: &Delimiter) -> i64 {
        if (self.can_close || closer.can_open)
            && (self.original_length + closer.original_length) % 3 == 0
            && closer.original_length % 3 != 0
        {
            return 0;
        }
        if self.length >= 2 && closer.length >= 2 {
            return 2;
        }
        1
    }
}

// Go: parser/delimiter.go:NewDelimiter
/// NewDelimiter returns a new Delimiter (not yet in the arena; see
/// [`Ast::new_delimiter`]).
pub fn new_delimiter(
    can_open: bool,
    can_close: bool,
    length: i64,
    char: u8,
    processor: Arc<dyn DelimiterProcessor>,
) -> Delimiter {
    Delimiter {
        segment: Segment::default(),
        can_open,
        can_close,
        length,
        original_length: length,
        char,
        previous_delimiter: None,
        next_delimiter: None,
        processor,
    }
}

impl Ast {
    /// Adds a Delimiter node to the arena (Go: the `*Delimiter` itself).
    pub fn new_delimiter(&mut self, d: Delimiter) -> NodeId {
        self.new_node(NodeValue::Delimiter(d))
    }

    /// The Delimiter fields of `n`; panics if `n` is not a Delimiter.
    pub fn delimiter(&self, n: NodeId) -> &Delimiter {
        match self.value(n) {
            NodeValue::Delimiter(d) => d,
            _ => panic!("not a Delimiter"),
        }
    }

    /// The Delimiter fields of `n`, mutably; panics if `n` is not a Delimiter.
    pub fn delimiter_mut(&mut self, n: NodeId) -> &mut Delimiter {
        match self.value_mut(n) {
            NodeValue::Delimiter(d) => d,
            _ => panic!("not a Delimiter"),
        }
    }

    /// True if `n` is a Delimiter node.
    pub fn is_delimiter(&self, n: NodeId) -> bool {
        matches!(self.value(n), NodeValue::Delimiter(_))
    }
}

// Go: parser/delimiter.go:ScanDelimiter
/// ScanDelimiter scans a delimiter by given DelimiterProcessor.
pub fn scan_delimiter(
    line: &[u8],
    before: Rune,
    minimum: i64,
    processor: Arc<dyn DelimiterProcessor>,
) -> Option<Delimiter> {
    let i = 0usize;
    let c = line[i];
    let mut j = i;
    if !processor.is_delimiter(c) {
        return None;
    }
    while j < line.len() && c == line[j] {
        j += 1;
    }
    if (j - i) as i64 >= minimum {
        let mut after = ' ' as Rune;
        if j != line.len() {
            after = util::to_rune(line, j);
        }

        let before_is_punctuation = util::is_punct_rune(before);
        let before_is_whitespace = util::is_space_rune(before);
        let after_is_punctuation = util::is_punct_rune(after);
        let after_is_whitespace = util::is_space_rune(after);

        let is_left = !after_is_whitespace
            && (!after_is_punctuation || before_is_whitespace || before_is_punctuation);
        let is_right = !before_is_whitespace
            && (!before_is_punctuation || after_is_whitespace || after_is_punctuation);

        let (can_open, can_close) = if line[i] == b'_' {
            (
                is_left && (!is_right || before_is_punctuation),
                is_right && (!is_left || after_is_punctuation),
            )
        } else {
            (is_left, is_right)
        };
        return Some(new_delimiter(
            can_open,
            can_close,
            (j - i) as i64,
            c,
            processor,
        ));
    }
    None
}

/// The `bottom ast.Node` argument of ProcessDelimiters. Go distinguishes a
/// nil interface from an interface holding a nil `*Delimiter` (what
/// `pushLinkBottom` stores when there is no delimiter yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterBottom {
    /// A nil `ast.Node`.
    Nil,
    /// An `ast.Node` holding a nil `*Delimiter`.
    TypedNil,
    /// A node.
    Node(NodeId),
}

impl DelimiterBottom {
    fn node(self) -> Option<NodeId> {
        match self {
            DelimiterBottom::Node(n) => Some(n),
            _ => None,
        }
    }
}

// Go: parser/delimiter.go:ProcessDelimiters
/// ProcessDelimiters processes the delimiter list in the context.
/// Processing will be stop when reaching the bottom.
///
/// If you implement an inline parser that can have other inline nodes as
/// children, you should call this function when nesting span has closed.
pub fn process_delimiters(ast: &mut Ast, bottom: DelimiterBottom, pc: &mut Context) {
    let Some(last_delimiter) = pc.last_delimiter() else {
        return;
    };
    let bottom_node = bottom.node();
    let mut closer: Option<NodeId> = None;
    if bottom != DelimiterBottom::Nil {
        if bottom_node != Some(last_delimiter) {
            let mut c = ast.previous_sibling(last_delimiter);
            while let Some(cid) = c {
                if Some(cid) == bottom_node {
                    break;
                }
                if ast.is_delimiter(cid) {
                    closer = Some(cid);
                }
                c = ast.previous_sibling(cid);
            }
        }
    } else {
        closer = pc.first_delimiter();
    }
    if closer.is_none() {
        pc.clear_delimiters(ast, bottom_node);
        return;
    }
    while let Some(cl) = closer {
        if !ast.delimiter(cl).can_close {
            closer = ast.delimiter(cl).next_delimiter;
            continue;
        }
        let mut consume = 0;
        let mut found = false;
        let mut maybe_opener = false;
        let mut opener = ast.delimiter(cl).previous_delimiter;
        while let Some(op) = opener {
            if Some(op) == bottom_node {
                break;
            }
            let od = ast.delimiter(op);
            let cd = ast.delimiter(cl);
            if od.can_open && od.processor.can_open_closer(od, cd) {
                maybe_opener = true;
                consume = od.calc_comsumption(cd);
                if consume > 0 {
                    found = true;
                    break;
                }
            }
            opener = od.previous_delimiter;
        }
        if !found {
            let next = ast.delimiter(cl).next_delimiter;
            if !maybe_opener && !ast.delimiter(cl).can_open {
                pc.remove_delimiter(ast, cl);
            }
            closer = next;
            continue;
        }
        let op = opener.unwrap();
        ast.delimiter_mut(op).consume_characters(consume);
        ast.delimiter_mut(cl).consume_characters(consume);

        let processor = ast.delimiter(op).processor.clone();
        let node = processor.on_match(ast, consume);

        let parent = ast.parent(op).expect("delimiter has no parent");
        let mut child = ast.next_sibling(op);

        while let Some(ch) = child {
            if ch == cl {
                break;
            }
            let next = ast.next_sibling(ch);
            ast.append_child(node, ch);
            child = next;
        }
        ast.insert_after(parent, op, node);

        let mut c = ast.delimiter(op).next_delimiter;
        while let Some(cid) = c {
            if cid == cl {
                break;
            }
            let next = ast.delimiter(cid).next_delimiter;
            pc.remove_delimiter(ast, cid);
            c = next;
        }

        if ast.delimiter(op).length == 0 {
            pc.remove_delimiter(ast, op);
        }

        if ast.delimiter(cl).length == 0 {
            let next = ast.delimiter(cl).next_delimiter;
            pc.remove_delimiter(ast, cl);
            closer = next;
        }
    }
    pc.clear_delimiters(ast, bottom_node);
}
