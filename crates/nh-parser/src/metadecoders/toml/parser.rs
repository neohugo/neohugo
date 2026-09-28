//! Port of `github.com/pelletier/go-toml/v2@v2.2.4/unstable` (`ast.go`, `builder.go`, `kind.go`,
//! `parser.go`, `scanner.go`).
//!
//! Go works on sub-slices of the document and recovers positions with pointer arithmetic
//! (`danger.SubsliceOffset`); the port does the same with `&'a [u8]` sub-slices of
//! [`Parser::data`] and [`Parser::offset_of`].

use std::borrow::Cow;

use super::characters::{utf8_toml_valid_already_escaped, utf8_valid_next};
use super::fmt_byte_c;
use super::fmt_byte_sharp_u;

/// Go: `unstable.Kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    #[allow(dead_code)] // Go's zero value; the parser never produces it.
    Invalid,
    Comment,
    Key,

    // Top level structures.
    Table,
    ArrayTable,
    KeyValue,

    // Containers values.
    Array,
    InlineTable,

    // Values.
    String,
    Bool,
    Float,
    Integer,
    LocalDate,
    LocalTime,
    LocalDateTime,
    DateTime,
}

impl Kind {
    // Go: unstable/kind.go:String
    pub(crate) fn string(self) -> &'static str {
        match self {
            Kind::Invalid => "Invalid",
            Kind::Comment => "Comment",
            Kind::Key => "Key",
            Kind::Table => "Table",
            Kind::ArrayTable => "ArrayTable",
            Kind::KeyValue => "KeyValue",
            Kind::Array => "Array",
            Kind::InlineTable => "InlineTable",
            Kind::String => "String",
            Kind::Bool => "Bool",
            Kind::Float => "Float",
            Kind::Integer => "Integer",
            Kind::LocalDate => "LocalDate",
            Kind::LocalTime => "LocalTime",
            Kind::LocalDateTime => "LocalDateTime",
            Kind::DateTime => "DateTime",
        }
    }
}

/// Go: `unstable.Range`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Range {
    pub(crate) offset: u32,
    pub(crate) length: u32,
}

/// Go: `unstable.Node`. `next`/`child` are absolute indexes into the expression's node array
/// (Go stores relative strides; 0 = none, here `None`).
#[derive(Clone, Debug)]
pub(crate) struct Node<'a> {
    pub(crate) kind: Kind,
    /// Raw bytes from the input.
    pub(crate) raw: Range,
    /// Node value (either allocated or referencing the input).
    pub(crate) data: Cow<'a, [u8]>,
    pub(crate) next: Option<usize>,
    pub(crate) child: Option<usize>,
}

impl<'a> Node<'a> {
    fn new(kind: Kind) -> Node<'a> {
        Node {
            kind,
            raw: Range::default(),
            data: Cow::Borrowed(&[]),
            next: None,
            child: None,
        }
    }
}

/// Go: `unstable.Iterator` over the nodes of one expression (a `Copy` value, like Go's struct).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Iter {
    started: bool,
    node: Option<usize>,
}

impl Iter {
    // Go: unstable/ast.go:Next
    pub(crate) fn next(&mut self, nodes: &[Node<'_>]) -> bool {
        if !self.started {
            self.started = true;
        } else if let Some(n) = self.node {
            self.node = nodes[n].next;
        }
        self.node.is_some()
    }

    // Go: unstable/ast.go:IsLast
    pub(crate) fn is_last(&self, nodes: &[Node<'_>]) -> bool {
        nodes[self.node.expect("IsLast on an invalid node")]
            .next
            .is_none()
    }

    // Go: unstable/ast.go:Node
    pub(crate) fn node(&self) -> usize {
        self.node.expect("nil node")
    }
}

/// Go: `(*Node).Key()`.
// Go: unstable/ast.go:Key
pub(crate) fn node_key(nodes: &[Node<'_>], n: usize) -> Iter {
    match nodes[n].kind {
        Kind::KeyValue => {
            let value = nodes[n]
                .child
                .expect("KeyValue should have at least two children");
            Iter {
                started: false,
                node: nodes[value].next,
            }
        }
        Kind::Table | Kind::ArrayTable => Iter {
            started: false,
            node: nodes[n].child,
        },
        k => panic!("Key() is not supported on a {}", k.string()),
    }
}

/// Go: `(*Node).Value()`.
// Go: unstable/ast.go:Value
pub(crate) fn node_value(nodes: &[Node<'_>], n: usize) -> usize {
    nodes[n].child.expect("nil value")
}

/// Go: `(*Node).Children()`.
// Go: unstable/ast.go:Children
pub(crate) fn node_children(nodes: &[Node<'_>], n: usize) -> Iter {
    Iter {
        started: false,
        node: nodes[n].child,
    }
}

/// Go: `unstable.ParserError`. `highlight` is a sub-slice of the document.
#[derive(Clone, Debug)]
pub(crate) struct ParserError<'a> {
    pub(crate) highlight: &'a [u8],
    pub(crate) message: Vec<u8>,
}

/// Go: `NewParserError(highlight, format, args...)` (callers format the message).
// Go: unstable/parser.go:NewParserError
pub(crate) fn new_parser_error<'a>(
    highlight: &'a [u8],
    message: impl Into<Vec<u8>>,
) -> ParserError<'a> {
    ParserError {
        highlight,
        message: message.into(),
    }
}

type PResult<'a, T> = Result<T, ParserError<'a>>;

/// A scanned string: (raw token, decoded value, rest of the input).
type StrToken<'a> = (&'a [u8], Cow<'a, [u8]>, &'a [u8]);

/// Go: `invalidReference` is `None`.
type Reference = Option<usize>;

/// Go: `unstable.builder`.
#[derive(Default)]
struct Builder<'a> {
    nodes: Vec<Node<'a>>,
    last_idx: usize,
}

impl<'a> Builder<'a> {
    // Go: unstable/builder.go:Reset
    fn reset(&mut self) {
        self.nodes.clear();
        self.last_idx = 0;
    }

    // Go: unstable/builder.go:Push
    fn push(&mut self, n: Node<'a>) -> usize {
        self.last_idx = self.nodes.len();
        self.nodes.push(n);
        self.last_idx
    }

    // Go: unstable/builder.go:PushAndChain
    fn push_and_chain(&mut self, n: Node<'a>) -> usize {
        let new_idx = self.nodes.len();
        self.nodes.push(n);
        self.nodes[self.last_idx].next = Some(new_idx);
        self.last_idx = new_idx;
        self.last_idx
    }

    // Go: unstable/builder.go:AttachChild
    fn attach_child(&mut self, parent: usize, child: usize) {
        self.nodes[parent].child = Some(child);
    }

    // Go: unstable/builder.go:Chain
    fn chain(&mut self, from: usize, to: usize) {
        self.nodes[from].next = Some(to);
    }
}

/// Go: `unstable.Parser`.
pub(crate) struct Parser<'a> {
    data: &'a [u8],
    builder: Builder<'a>,
    reference: Reference,
    left: &'a [u8],
    err: Option<ParserError<'a>>,
    first: bool,
    /// Go: `KeepComments` (false for `Unmarshal`).
    keep_comments: bool,
}

impl<'a> Parser<'a> {
    /// Go: `Reset(b)` on a zero parser.
    // Go: unstable/parser.go:Reset
    pub(crate) fn new(b: &'a [u8]) -> Parser<'a> {
        Parser {
            data: b,
            builder: Builder::default(),
            reference: None,
            left: b,
            err: None,
            first: true,
            keep_comments: false,
        }
    }

    // Go: unstable/parser.go:Data
    pub(crate) fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Go: `danger.SubsliceOffset(p.data, b)`.
    pub(crate) fn offset_of(&self, b: &[u8]) -> usize {
        let base = self.data.as_ptr() as usize;
        let p = b.as_ptr() as usize;
        assert!(
            p >= base && p + b.len() <= base + self.data.len(),
            "subslice is not part of the document"
        );
        p - base
    }

    /// Range returns a range description that corresponds to a given slice of the input.
    // Go: unstable/parser.go:Range
    fn range(&self, b: &[u8]) -> Range {
        Range {
            offset: self.offset_of(b) as u32,
            length: b.len() as u32,
        }
    }

    /// Raw returns the slice corresponding to the bytes in the given range.
    // Go: unstable/parser.go:Raw
    pub(crate) fn raw(&self, raw: Range) -> &'a [u8] {
        &self.data[raw.offset as usize..(raw.offset + raw.length) as usize]
    }

    /// The nodes of the current expression.
    pub(crate) fn nodes(&self) -> &[Node<'a>] {
        &self.builder.nodes
    }

    /// NextExpression parses the next top-level expression. If an expression was
    /// successfully parsed, it returns true. If the parser is at the end of the
    /// document or an error occurred, it returns false.
    // Go: unstable/parser.go:NextExpression
    pub(crate) fn next_expression(&mut self) -> bool {
        if self.left.is_empty() || self.err.is_some() {
            return false;
        }

        self.builder.reset();
        self.reference = None;

        loop {
            if self.left.is_empty() || self.err.is_some() {
                return false;
            }

            if !self.first {
                match self.parse_newline(self.left) {
                    Ok(rest) => self.left = rest,
                    Err(e) => {
                        self.left = &[];
                        self.err = Some(e);
                    }
                }
            }

            if self.left.is_empty() || self.err.is_some() {
                return false;
            }

            match self.parse_expression(self.left) {
                Ok((r, rest)) => {
                    self.reference = r;
                    self.left = rest;
                }
                Err(e) => {
                    self.reference = None;
                    self.left = &[];
                    self.err = Some(e);
                }
            }

            if self.err.is_some() {
                return false;
            }

            self.first = false;

            if self.reference.is_some() {
                return true;
            }
        }
    }

    /// Expression returns the index of the node representing the last successfully parsed
    /// expression.
    // Go: unstable/parser.go:Expression
    pub(crate) fn expression(&self) -> usize {
        self.reference.expect("no expression")
    }

    /// Error returns any error that has occurred during parsing.
    // Go: unstable/parser.go:Error
    pub(crate) fn error(&self) -> Option<ParserError<'a>> {
        self.err.clone()
    }

    // Go: unstable/parser.go:parseNewline
    fn parse_newline(&self, b: &'a [u8]) -> PResult<'a, &'a [u8]> {
        if b[0] == b'\n' {
            return Ok(&b[1..]);
        }

        if b[0] == b'\r' {
            let (_, rest) = scan_windows_newline(b)?;
            return Ok(rest);
        }

        let mut m = b"expected newline but got ".to_vec();
        m.extend_from_slice(&fmt_byte_sharp_u(b[0]));
        Err(new_parser_error(&b[0..1], m))
    }

    // Go: unstable/parser.go:parseComment
    fn parse_comment(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        let mut r = None;
        let (data, rest) = scan_comment(b)?;
        if self.keep_comments {
            let raw = self.range(data);
            r = Some(self.builder.push(Node {
                kind: Kind::Comment,
                raw,
                data: Cow::Borrowed(data),
                next: None,
                child: None,
            }));
        }
        Ok((r, rest))
    }

    // Go: unstable/parser.go:parseExpression
    fn parse_expression(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // expression =  ws [ comment ]
        // expression =/ ws keyval ws [ comment ]
        // expression =/ ws table ws [ comment ]
        let r: Reference = None;

        let mut b = self.parse_whitespace(b);

        if b.is_empty() {
            return Ok((r, b));
        }

        if b[0] == b'#' {
            return self.parse_comment(b);
        }

        if b[0] == b'\n' || b[0] == b'\r' {
            return Ok((r, b));
        }

        let r;
        if b[0] == b'[' {
            let (rr, rest) = self.parse_table(b)?;
            r = rr;
            b = rest;
        } else {
            let (rr, rest) = self.parse_keyval(b)?;
            r = rr;
            b = rest;
        }

        b = self.parse_whitespace(b);

        if !b.is_empty() && b[0] == b'#' {
            let (cref, rest) = self.parse_comment(b)?;
            if let (Some(c), Some(rr)) = (cref, r) {
                self.builder.chain(rr, c);
            }
            return Ok((r, rest));
        }

        Ok((r, b))
    }

    // Go: unstable/parser.go:parseTable
    fn parse_table(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // table = std-table / array-table
        if b.len() > 1 && b[1] == b'[' {
            return self.parse_array_table(b);
        }

        self.parse_std_table(b)
    }

    // Go: unstable/parser.go:parseArrayTable
    fn parse_array_table(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // array-table = array-table-open key array-table-close
        // array-table-open  = %x5B.5B ws  ; [[ Double left square bracket
        // array-table-close = ws %x5D.5D  ; ]] Double right square bracket
        let r = self.builder.push(Node::new(Kind::ArrayTable));

        let b = &b[2..];
        let b = self.parse_whitespace(b);

        let (k, b) = self.parse_key(b)?;

        self.builder.attach_child(r, k);
        let b = self.parse_whitespace(b);

        let b = expect(b']', b)?;

        let b = expect(b']', b)?;

        Ok((Some(r), b))
    }

    // Go: unstable/parser.go:parseStdTable
    fn parse_std_table(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // std-table = std-table-open key std-table-close
        // std-table-open  = %x5B ws     ; [ Left square bracket
        // std-table-close = ws %x5D     ; ] Right square bracket
        let r = self.builder.push(Node::new(Kind::Table));

        let b = &b[1..];
        let b = self.parse_whitespace(b);

        let (key, b) = self.parse_key(b)?;

        self.builder.attach_child(r, key);

        let b = self.parse_whitespace(b);

        let b = expect(b']', b)?;

        Ok((Some(r), b))
    }

    // Go: unstable/parser.go:parseKeyval
    fn parse_keyval(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // keyval = key keyval-sep val
        let r = self.builder.push(Node::new(Kind::KeyValue));

        let (key, b) = self.parse_key(b)?;

        // keyval-sep = ws %x3D ws ; =

        let b = self.parse_whitespace(b);

        if b.is_empty() {
            return Err(new_parser_error(
                b,
                "expected = after a key, but the document ends there",
            ));
        }

        let b = expect(b'=', b)?;

        let b = self.parse_whitespace(b);

        let (val_ref, b) = self.parse_val(b)?;
        let val_ref = val_ref.expect("value reference");

        self.builder.chain(val_ref, key);
        self.builder.attach_child(r, val_ref);

        Ok((Some(r), b))
    }

    // Go: unstable/parser.go:parseVal
    fn parse_val(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // val = string / boolean / array / inline-table / date-time / float / integer
        if b.is_empty() {
            return Err(new_parser_error(b, "expected value, not eof"));
        }

        let c = b[0];

        match c {
            b'"' => {
                let (raw, v, b) = if scan_follows_multiline_basic_string_delimiter(b) {
                    self.parse_multiline_basic_string(b)?
                } else {
                    self.parse_basic_string(b)?
                };

                let raw = self.range(raw);
                let r = self.builder.push(Node {
                    kind: Kind::String,
                    raw,
                    data: v,
                    next: None,
                    child: None,
                });

                Ok((Some(r), b))
            }
            b'\'' => {
                let (raw, v, b) = if scan_follows_multiline_literal_string_delimiter(b) {
                    self.parse_multiline_literal_string(b)?
                } else {
                    self.parse_literal_string(b)?
                };

                let raw = self.range(raw);
                let r = self.builder.push(Node {
                    kind: Kind::String,
                    raw,
                    data: Cow::Borrowed(v),
                    next: None,
                    child: None,
                });

                Ok((Some(r), b))
            }
            b't' => {
                if !scan_follows_true(b) {
                    return Err(new_parser_error(atmost(b, 4), "expected 'true'"));
                }

                let r = self.builder.push(Node {
                    kind: Kind::Bool,
                    raw: Range::default(),
                    data: Cow::Borrowed(&b[..4]),
                    next: None,
                    child: None,
                });

                Ok((Some(r), &b[4..]))
            }
            b'f' => {
                if !scan_follows_false(b) {
                    return Err(new_parser_error(atmost(b, 5), "expected 'false'"));
                }

                let r = self.builder.push(Node {
                    kind: Kind::Bool,
                    raw: Range::default(),
                    data: Cow::Borrowed(&b[..5]),
                    next: None,
                    child: None,
                });

                Ok((Some(r), &b[5..]))
            }
            b'[' => self.parse_val_array(b),
            b'{' => self.parse_inline_table(b),
            _ => self.parse_int_or_float_or_date_time(b),
        }
    }

    // Go: unstable/parser.go:parseLiteralString
    fn parse_literal_string(&self, b: &'a [u8]) -> PResult<'a, (&'a [u8], &'a [u8], &'a [u8])> {
        let (v, rest) = scan_literal_string(b)?;

        Ok((v, &v[1..v.len() - 1], rest))
    }

    // Go: unstable/parser.go:parseInlineTable
    fn parse_inline_table(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // inline-table = inline-table-open [ inline-table-keyvals ] inline-table-close
        // inline-table-open  = %x7B ws     ; {
        // inline-table-close = ws %x7D     ; }
        // inline-table-sep   = ws %x2C ws  ; , Comma
        // inline-table-keyvals = keyval [ inline-table-sep inline-table-keyvals ]
        let raw = self.range(&b[..1]);
        let parent = self.builder.push(Node {
            kind: Kind::InlineTable,
            raw,
            data: Cow::Borrowed(&[]),
            next: None,
            child: None,
        });

        let mut first = true;

        let mut child: usize = 0;

        let mut b = &b[1..];

        while !b.is_empty() {
            let previous_b = b;
            b = self.parse_whitespace(b);

            if b.is_empty() {
                return Err(new_parser_error(
                    &previous_b[..1],
                    "inline table is incomplete",
                ));
            }

            if b[0] == b'}' {
                break;
            }

            if !first {
                b = expect(b',', b)?;
                b = self.parse_whitespace(b);
            }

            let (kv, rest) = self.parse_keyval(b)?;
            b = rest;
            let kv = kv.expect("keyval reference");

            if first {
                self.builder.attach_child(parent, kv);
            } else {
                self.builder.chain(child, kv);
            }
            child = kv;

            first = false;
        }

        let rest = expect(b'}', b)?;

        Ok((Some(parent), rest))
    }

    // Go: unstable/parser.go:parseValArray
    fn parse_val_array(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // array = array-open [ array-values ] ws-comment-newline array-close
        // array-open =  %x5B ; [
        // array-close = %x5D ; ]
        // array-values =  ws-comment-newline val ws-comment-newline array-sep array-values
        // array-values =/ ws-comment-newline val ws-comment-newline [ array-sep ]
        // array-sep = %x2C  ; , Comma
        // ws-comment-newline = *( wschar / [ comment ] newline )
        let array_start = b;
        let mut b = &b[1..];

        let parent = self.builder.push(Node::new(Kind::Array));

        // First indicates whether the parser is looking for the first element
        // (non-comment) of the array.
        let mut first = true;

        let mut last_child: Reference = None;

        let add_child =
            |builder: &mut Builder<'a>, last_child: &mut Reference, value_ref: usize| {
                match *last_child {
                    None => builder.attach_child(parent, value_ref),
                    Some(lc) => builder.chain(lc, value_ref),
                }
                *last_child = Some(value_ref);
            };

        while !b.is_empty() {
            let (cref, rest) = self.parse_optional_whitespace_comment_newline(b)?;
            b = rest;

            if let Some(c) = cref {
                add_child(&mut self.builder, &mut last_child, c);
            }

            if b.is_empty() {
                return Err(new_parser_error(&array_start[..1], "array is incomplete"));
            }

            if b[0] == b']' {
                break;
            }

            if b[0] == b',' {
                if first {
                    return Err(new_parser_error(&b[0..1], "array cannot start with comma"));
                }
                b = &b[1..];

                let (cref, rest) = self.parse_optional_whitespace_comment_newline(b)?;
                b = rest;
                if let Some(c) = cref {
                    add_child(&mut self.builder, &mut last_child, c);
                }
            } else if !first {
                return Err(new_parser_error(
                    &b[0..1],
                    "array elements must be separated by commas",
                ));
            }

            // TOML allows trailing commas in arrays.
            if !b.is_empty() && b[0] == b']' {
                break;
            }

            let (value_ref, rest) = self.parse_val(b)?;
            b = rest;

            add_child(
                &mut self.builder,
                &mut last_child,
                value_ref.expect("value reference"),
            );

            let (cref, rest) = self.parse_optional_whitespace_comment_newline(b)?;
            b = rest;
            if let Some(c) = cref {
                add_child(&mut self.builder, &mut last_child, c);
            }

            first = false;
        }

        let rest = expect(b']', b)?;

        Ok((Some(parent), rest))
    }

    // Go: unstable/parser.go:parseOptionalWhitespaceCommentNewline
    fn parse_optional_whitespace_comment_newline(
        &mut self,
        b: &'a [u8],
    ) -> PResult<'a, (Reference, &'a [u8])> {
        let mut root_comment_ref: Reference = None;
        let mut latest_comment_ref: Reference = None;

        let mut b = b;
        while !b.is_empty() {
            b = self.parse_whitespace(b);

            if !b.is_empty() && b[0] == b'#' {
                let (r, rest) = self.parse_comment(b)?;
                b = rest;
                if let Some(r) = r {
                    // addComment
                    match (root_comment_ref, latest_comment_ref) {
                        (None, _) => root_comment_ref = Some(r),
                        (Some(root), None) => {
                            self.builder.attach_child(root, r);
                            latest_comment_ref = Some(r);
                        }
                        (Some(_), Some(latest)) => {
                            self.builder.chain(latest, r);
                            latest_comment_ref = Some(r);
                        }
                    }
                }
            }

            if b.is_empty() {
                break;
            }

            if b[0] == b'\n' || b[0] == b'\r' {
                b = self.parse_newline(b)?;
            } else {
                break;
            }
        }

        Ok((root_comment_ref, b))
    }

    // Go: unstable/parser.go:parseMultilineLiteralString
    fn parse_multiline_literal_string(
        &self,
        b: &'a [u8],
    ) -> PResult<'a, (&'a [u8], &'a [u8], &'a [u8])> {
        let (token, rest) = scan_multiline_literal_string(b)?;

        let mut i = 3;

        // skip the immediate new line
        if token[i] == b'\n' {
            i += 1;
        } else if token[i] == b'\r' && token[i + 1] == b'\n' {
            i += 2;
        }

        Ok((token, &token[i..token.len() - 3], rest))
    }

    // Go: unstable/parser.go:parseMultilineBasicString
    fn parse_multiline_basic_string(&self, b: &'a [u8]) -> PResult<'a, StrToken<'a>> {
        // ml-basic-string = ml-basic-string-delim [ newline ] ml-basic-body
        // ml-basic-string-delim
        // ml-basic-string-delim = 3quotation-mark
        // ml-basic-body = *mlb-content *( mlb-quotes 1*mlb-content ) [ mlb-quotes ]
        //
        // mlb-content = mlb-char / newline / mlb-escaped-nl
        // mlb-char = mlb-unescaped / escaped
        // mlb-quotes = 1*2quotation-mark
        // mlb-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
        // mlb-escaped-nl = escape ws newline *( wschar / newline )
        let (token, escaped, rest) = scan_multiline_basic_string(b)?;

        let mut i = 3;

        // skip the immediate new line
        if token[i] == b'\n' {
            i += 1;
        } else if token[i] == b'\r' && token[i + 1] == b'\n' {
            i += 2;
        }

        // fast path
        let start_idx = i;
        let end_idx = token.len() - 3;

        if !escaped {
            let str = &token[start_idx..end_idx];
            let verr = utf8_toml_valid_already_escaped(str);
            if verr.zero() {
                return Ok((token, Cow::Borrowed(str), rest));
            }
            return Err(new_parser_error(
                &str[verr.index..verr.index + verr.size],
                "invalid UTF-8",
            ));
        }

        let mut builder: Vec<u8> = Vec::new();

        // The scanner ensures that the token starts and ends with quotes and that
        // escapes are balanced.
        while i < token.len() - 3 {
            let mut c = token[i];

            if c == b'\\' {
                // When the last non-whitespace character on a line is an unescaped \,
                // it will be trimmed along with all whitespace (including newlines) up
                // to the next non-whitespace character or closing delimiter.

                let mut is_last_non_whitespace_on_line = false;
                let mut j = 1;
                while j < token.len() - 3 - i {
                    match token[i + j] {
                        b' ' | b'\t' => {
                            j += 1;
                            continue;
                        }
                        b'\r' => {
                            if token[i + j + 1] == b'\n' {
                                j += 1;
                                continue;
                            }
                        }
                        b'\n' => {
                            is_last_non_whitespace_on_line = true;
                        }
                        _ => {}
                    }
                    break;
                }
                if is_last_non_whitespace_on_line {
                    i += j;
                    while i < token.len() - 3 {
                        let c = token[i];
                        if !(c == b'\n' || c == b'\r' || c == b' ' || c == b'\t') {
                            i -= 1;
                            break;
                        }
                        i += 1;
                    }
                    i += 1;
                    continue;
                }

                // handle escaping
                i += 1;
                c = token[i];

                match c {
                    b'"' | b'\\' => builder.push(c),
                    b'b' => builder.push(0x08),
                    b'f' => builder.push(0x0C),
                    b'n' => builder.push(b'\n'),
                    b'r' => builder.push(b'\r'),
                    b't' => builder.push(b'\t'),
                    b'e' => builder.push(0x1B),
                    b'u' => {
                        let x = hex_to_rune(atmost(&token[i + 1..], 4), 4)?;
                        write_rune(&mut builder, x);
                        i += 4;
                    }
                    b'U' => {
                        let x = hex_to_rune(atmost(&token[i + 1..], 8), 8)?;
                        write_rune(&mut builder, x);
                        i += 8;
                    }
                    _ => {
                        let mut m = b"invalid escaped character ".to_vec();
                        m.extend_from_slice(&fmt_byte_sharp_u(c));
                        return Err(new_parser_error(&token[i..i + 1], m));
                    }
                }
                i += 1;
            } else {
                let size = utf8_valid_next(&token[i..]);
                if size == 0 {
                    let mut m = b"invalid character ".to_vec();
                    m.extend_from_slice(&fmt_byte_sharp_u(c));
                    return Err(new_parser_error(&token[i..i + 1], m));
                }
                builder.extend_from_slice(&token[i..i + size]);
                i += size;
            }
        }

        Ok((token, Cow::Owned(builder), rest))
    }

    // Go: unstable/parser.go:parseKey
    fn parse_key(&mut self, b: &'a [u8]) -> PResult<'a, (usize, &'a [u8])> {
        // key = simple-key / dotted-key
        // simple-key = quoted-key / unquoted-key
        //
        // unquoted-key = 1*( ALPHA / DIGIT / %x2D / %x5F ) ; A-Z / a-z / 0-9 / - / _
        // quoted-key = basic-string / literal-string
        // dotted-key = simple-key 1*( dot-sep simple-key )
        //
        // dot-sep   = ws %x2E ws  ; . Period
        let (raw, key, mut b) = self.parse_simple_key(b)?;

        let range = self.range(raw);
        let r = self.builder.push(Node {
            kind: Kind::Key,
            raw: range,
            data: key,
            next: None,
            child: None,
        });

        loop {
            b = self.parse_whitespace(b);
            if !b.is_empty() && b[0] == b'.' {
                b = self.parse_whitespace(&b[1..]);

                let (raw, key, rest) = self.parse_simple_key(b)?;
                b = rest;

                let range = self.range(raw);
                self.builder.push_and_chain(Node {
                    kind: Kind::Key,
                    raw: range,
                    data: key,
                    next: None,
                    child: None,
                });
            } else {
                break;
            }
        }

        Ok((r, b))
    }

    // Go: unstable/parser.go:parseSimpleKey
    fn parse_simple_key(&self, b: &'a [u8]) -> PResult<'a, StrToken<'a>> {
        if b.is_empty() {
            return Err(new_parser_error(b, "expected key but found none"));
        }

        // simple-key = quoted-key / unquoted-key
        // unquoted-key = 1*( ALPHA / DIGIT / %x2D / %x5F ) ; A-Z / a-z / 0-9 / - / _
        // quoted-key = basic-string / literal-string
        if b[0] == b'\'' {
            let (raw, key, rest) = self.parse_literal_string(b)?;
            Ok((raw, Cow::Borrowed(key), rest))
        } else if b[0] == b'"' {
            self.parse_basic_string(b)
        } else if is_unquoted_key_char(b[0]) {
            let (key, rest) = scan_unquoted_key(b);
            Ok((key, Cow::Borrowed(key), rest))
        } else {
            let mut m = b"invalid character at start of key: ".to_vec();
            m.extend_from_slice(&fmt_byte_c(b[0]));
            Err(new_parser_error(&b[0..1], m))
        }
    }

    // Go: unstable/parser.go:parseBasicString
    fn parse_basic_string(&self, b: &'a [u8]) -> PResult<'a, StrToken<'a>> {
        // basic-string = quotation-mark *basic-char quotation-mark
        // quotation-mark = %x22            ; "
        // basic-char = basic-unescaped / escaped
        // basic-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
        // escaped = escape escape-seq-char
        let (token, escaped, rest) = scan_basic_string(b)?;

        let start_idx = 1;
        let end_idx = token.len() - 1;

        // Fast path. If there is no escape sequence, the string should just be
        // an UTF-8 encoded string, which is the same as Go. In that case,
        // validate the string and return a direct reference to the buffer.
        if !escaped {
            let str = &token[start_idx..end_idx];
            let verr = utf8_toml_valid_already_escaped(str);
            if verr.zero() {
                return Ok((token, Cow::Borrowed(str), rest));
            }
            return Err(new_parser_error(
                &str[verr.index..verr.index + verr.size],
                "invalid UTF-8",
            ));
        }

        let mut i = start_idx;

        let mut builder: Vec<u8> = Vec::new();

        // The scanner ensures that the token starts and ends with quotes and that
        // escapes are balanced.
        while i < token.len() - 1 {
            let mut c = token[i];
            if c == b'\\' {
                i += 1;
                c = token[i];

                match c {
                    b'"' | b'\\' => builder.push(c),
                    b'b' => builder.push(0x08),
                    b'f' => builder.push(0x0C),
                    b'n' => builder.push(b'\n'),
                    b'r' => builder.push(b'\r'),
                    b't' => builder.push(b'\t'),
                    b'e' => builder.push(0x1B),
                    b'u' => {
                        let x = hex_to_rune(&token[i + 1..token.len() - 1], 4)?;
                        write_rune(&mut builder, x);
                        i += 4;
                    }
                    b'U' => {
                        let x = hex_to_rune(&token[i + 1..token.len() - 1], 8)?;
                        write_rune(&mut builder, x);
                        i += 8;
                    }
                    _ => {
                        let mut m = b"invalid escaped character ".to_vec();
                        m.extend_from_slice(&fmt_byte_sharp_u(c));
                        return Err(new_parser_error(&token[i..i + 1], m));
                    }
                }
                i += 1;
            } else {
                let size = utf8_valid_next(&token[i..]);
                if size == 0 {
                    let mut m = b"invalid character ".to_vec();
                    m.extend_from_slice(&fmt_byte_sharp_u(c));
                    return Err(new_parser_error(&token[i..i + 1], m));
                }
                builder.extend_from_slice(&token[i..i + size]);
                i += size;
            }
        }

        Ok((token, Cow::Owned(builder), rest))
    }

    // Go: unstable/parser.go:parseWhitespace
    fn parse_whitespace(&self, b: &'a [u8]) -> &'a [u8] {
        // ws = *wschar
        // wschar =  %x20  ; Space
        // wschar =/ %x09  ; Horizontal tab
        let (_, rest) = scan_whitespace(b);

        rest
    }

    // Go: unstable/parser.go:parseIntOrFloatOrDateTime
    fn parse_int_or_float_or_date_time(
        &mut self,
        b: &'a [u8],
    ) -> PResult<'a, (Reference, &'a [u8])> {
        match b[0] {
            b'i' => {
                if !scan_follows_inf(b) {
                    return Err(new_parser_error(atmost(b, 3), "expected 'inf'"));
                }

                let raw = self.range(&b[..3]);
                return Ok((
                    Some(self.builder.push(Node {
                        kind: Kind::Float,
                        raw,
                        data: Cow::Borrowed(&b[..3]),
                        next: None,
                        child: None,
                    })),
                    &b[3..],
                ));
            }
            b'n' => {
                if !scan_follows_nan(b) {
                    return Err(new_parser_error(atmost(b, 3), "expected 'nan'"));
                }

                let raw = self.range(&b[..3]);
                return Ok((
                    Some(self.builder.push(Node {
                        kind: Kind::Float,
                        raw,
                        data: Cow::Borrowed(&b[..3]),
                        next: None,
                        child: None,
                    })),
                    &b[3..],
                ));
            }
            b'+' | b'-' => return self.scan_int_or_float(b),
            _ => {}
        }

        if b.len() < 3 {
            return self.scan_int_or_float(b);
        }

        let s = b.len().min(5);

        for (idx, &c) in b[..s].iter().enumerate() {
            if is_digit(c) {
                continue;
            }

            if (idx == 2 && c == b':') || (idx == 4 && c == b'-') {
                return self.scan_date_time(b);
            }

            break;
        }

        self.scan_int_or_float(b)
    }

    // Go: unstable/parser.go:scanDateTime
    fn scan_date_time(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        // scans for contiguous characters in [0-9T:Z.+-], and up to one space if
        // followed by a digit.
        let mut has_date = false;
        let mut has_time = false;
        let mut has_tz = false;
        let mut seen_space = false;

        let mut i = 0;
        while i < b.len() {
            let c = b[i];

            if is_digit(c) {
            } else if c == b'-' {
                has_date = true;
                const MIN_OFFSET_OF_TZ: usize = 8;
                if i >= MIN_OFFSET_OF_TZ {
                    has_tz = true;
                }
            } else if c == b'T' || c == b't' || c == b':' || c == b'.' {
                has_time = true;
            } else if c == b'+' || c == b'-' || c == b'Z' || c == b'z' {
                has_tz = true;
            } else if c == b' ' {
                if !seen_space && i + 1 < b.len() && is_digit(b[i + 1]) {
                    i += 2;
                    // Avoid reaching past the end of the document in case the time
                    // is malformed. See TestIssue585.
                    if i >= b.len() {
                        i -= 1;
                    }
                    seen_space = true;
                    has_time = true;
                } else {
                    break;
                }
            } else {
                break;
            }
            i += 1;
        }

        let kind = if has_time {
            if has_date {
                if has_tz {
                    Kind::DateTime
                } else {
                    Kind::LocalDateTime
                }
            } else {
                Kind::LocalTime
            }
        } else {
            Kind::LocalDate
        };

        Ok((
            Some(self.builder.push(Node {
                kind,
                raw: Range::default(),
                data: Cow::Borrowed(&b[..i]),
                next: None,
                child: None,
            })),
            &b[i..],
        ))
    }

    // Go: unstable/parser.go:scanIntOrFloat
    fn scan_int_or_float(&mut self, b: &'a [u8]) -> PResult<'a, (Reference, &'a [u8])> {
        let mut i = 0;

        if b.len() > 2 && b[0] == b'0' && b[1] != b'.' && b[1] != b'e' && b[1] != b'E' {
            let is_valid_rune: Option<fn(u8) -> bool> = match b[1] {
                b'x' => Some(is_valid_hex_rune),
                b'o' => Some(is_valid_octal_rune),
                b'b' => Some(is_valid_binary_rune),
                _ => {
                    i += 1;
                    None
                }
            };

            if let Some(is_valid_rune) = is_valid_rune {
                i += 2;
                while i < b.len() {
                    if !is_valid_rune(b[i]) {
                        break;
                    }
                    i += 1;
                }
            }

            let raw = self.range(&b[..i]);
            return Ok((
                Some(self.builder.push(Node {
                    kind: Kind::Integer,
                    raw,
                    data: Cow::Borrowed(&b[..i]),
                    next: None,
                    child: None,
                })),
                &b[i..],
            ));
        }

        let mut is_float = false;

        while i < b.len() {
            let c = b[i];

            if c.is_ascii_digit() || c == b'+' || c == b'-' || c == b'_' {
                i += 1;
                continue;
            }

            if c == b'.' || c == b'e' || c == b'E' {
                is_float = true;
                i += 1;
                continue;
            }

            if c == b'i' {
                if scan_follows_inf(&b[i..]) {
                    let raw = self.range(&b[..i + 3]);
                    return Ok((
                        Some(self.builder.push(Node {
                            kind: Kind::Float,
                            raw,
                            data: Cow::Borrowed(&b[..i + 3]),
                            next: None,
                            child: None,
                        })),
                        &b[i + 3..],
                    ));
                }

                return Err(new_parser_error(
                    &b[i..i + 1],
                    "unexpected character 'i' while scanning for a number",
                ));
            }

            if c == b'n' {
                if scan_follows_nan(&b[i..]) {
                    let raw = self.range(&b[..i + 3]);
                    return Ok((
                        Some(self.builder.push(Node {
                            kind: Kind::Float,
                            raw,
                            data: Cow::Borrowed(&b[..i + 3]),
                            next: None,
                            child: None,
                        })),
                        &b[i + 3..],
                    ));
                }

                return Err(new_parser_error(
                    &b[i..i + 1],
                    "unexpected character 'n' while scanning for a number",
                ));
            }

            break;
        }

        if i == 0 {
            return Err(new_parser_error(b, "incomplete number"));
        }

        let kind = if is_float { Kind::Float } else { Kind::Integer };

        let raw = self.range(&b[..i]);
        Ok((
            Some(self.builder.push(Node {
                kind,
                raw,
                data: Cow::Borrowed(&b[..i]),
                next: None,
                child: None,
            })),
            &b[i..],
        ))
    }
}

/// Go: `bytes.Buffer.WriteRune` (a rune from `hexToRune` is always valid).
fn write_rune(b: &mut Vec<u8>, r: i32) {
    go_unicode::utf8::append_rune(b, r);
}

// Go: unstable/parser.go:atmost
fn atmost(b: &[u8], n: usize) -> &[u8] {
    if n >= b.len() {
        return b;
    }

    &b[..n]
}

// Go: unstable/parser.go:hexToRune
fn hex_to_rune(b: &[u8], length: usize) -> Result<i32, ParserError<'_>> {
    if b.len() < length {
        return Err(new_parser_error(
            b,
            format!("unicode point needs {} character, not {}", length, b.len()),
        ));
    }
    let b = &b[..length];

    let mut r: u32 = 0;
    for (i, &c) in b.iter().enumerate() {
        let d: u32 = if c.is_ascii_digit() {
            (c - b'0') as u32
        } else if (b'a'..=b'f').contains(&c) {
            (c - b'a' + 10) as u32
        } else if (b'A'..=b'F').contains(&c) {
            (c - b'A' + 10) as u32
        } else {
            return Err(new_parser_error(&b[i..i + 1], "non-hex character"));
        };
        r = r.wrapping_mul(16).wrapping_add(d);
    }

    if r > 0x10FFFF || (0xD800..0xE000).contains(&r) {
        return Err(new_parser_error(
            b,
            "escape sequence is invalid Unicode code point",
        ));
    }

    Ok(r as i32)
}

// Go: unstable/parser.go:isDigit
fn is_digit(r: u8) -> bool {
    r.is_ascii_digit()
}

// Go: unstable/parser.go:isValidHexRune
fn is_valid_hex_rune(r: u8) -> bool {
    r.is_ascii_hexdigit() || r == b'_'
}

// Go: unstable/parser.go:isValidOctalRune
fn is_valid_octal_rune(r: u8) -> bool {
    (b'0'..=b'7').contains(&r) || r == b'_'
}

// Go: unstable/parser.go:isValidBinaryRune
fn is_valid_binary_rune(r: u8) -> bool {
    r == b'0' || r == b'1' || r == b'_'
}

// Go: unstable/parser.go:expect
fn expect(x: u8, b: &[u8]) -> Result<&[u8], ParserError<'_>> {
    if b.is_empty() {
        let mut m = b"expected character ".to_vec();
        m.extend_from_slice(&fmt_byte_c(x));
        m.extend_from_slice(b" but the document ended here");
        return Err(new_parser_error(b, m));
    }

    if b[0] != x {
        let mut m = b"expected character ".to_vec();
        m.extend_from_slice(&fmt_byte_c(x));
        return Err(new_parser_error(&b[0..1], m));
    }

    Ok(&b[1..])
}

// ---------------------------------------------------------------------------
// scanner.go

// Go: unstable/scanner.go:scanFollows
fn scan_follows(b: &[u8], pattern: &[u8]) -> bool {
    b.starts_with(pattern)
}

// Go: unstable/scanner.go:scanFollowsMultilineBasicStringDelimiter
fn scan_follows_multiline_basic_string_delimiter(b: &[u8]) -> bool {
    scan_follows(b, b"\"\"\"")
}

// Go: unstable/scanner.go:scanFollowsMultilineLiteralStringDelimiter
fn scan_follows_multiline_literal_string_delimiter(b: &[u8]) -> bool {
    scan_follows(b, b"'''")
}

// Go: unstable/scanner.go:scanFollowsTrue
fn scan_follows_true(b: &[u8]) -> bool {
    scan_follows(b, b"true")
}

// Go: unstable/scanner.go:scanFollowsFalse
fn scan_follows_false(b: &[u8]) -> bool {
    scan_follows(b, b"false")
}

// Go: unstable/scanner.go:scanFollowsInf
fn scan_follows_inf(b: &[u8]) -> bool {
    scan_follows(b, b"inf")
}

// Go: unstable/scanner.go:scanFollowsNan
fn scan_follows_nan(b: &[u8]) -> bool {
    scan_follows(b, b"nan")
}

// Go: unstable/scanner.go:scanUnquotedKey
fn scan_unquoted_key(b: &[u8]) -> (&[u8], &[u8]) {
    // unquoted-key = 1*( ALPHA / DIGIT / %x2D / %x5F ) ; A-Z / a-z / 0-9 / - / _
    for i in 0..b.len() {
        if !is_unquoted_key_char(b[i]) {
            return (&b[..i], &b[i..]);
        }
    }

    (b, &b[b.len()..])
}

// Go: unstable/scanner.go:isUnquotedKeyChar
fn is_unquoted_key_char(r: u8) -> bool {
    r.is_ascii_uppercase() || r.is_ascii_lowercase() || r.is_ascii_digit() || r == b'-' || r == b'_'
}

// Go: unstable/scanner.go:scanLiteralString
fn scan_literal_string(b: &[u8]) -> Result<(&[u8], &[u8]), ParserError<'_>> {
    // literal-string = apostrophe *literal-char apostrophe
    // apostrophe = %x27 ; ' apostrophe
    // literal-char = %x09 / %x20-26 / %x28-7E / non-ascii
    let mut i = 1;
    while i < b.len() {
        match b[i] {
            b'\'' => return Ok((&b[..i + 1], &b[i + 1..])),
            b'\n' | b'\r' => {
                return Err(new_parser_error(
                    &b[i..i + 1],
                    "literal strings cannot have new lines",
                ));
            }
            _ => {}
        }
        let size = utf8_valid_next(&b[i..]);
        if size == 0 {
            return Err(new_parser_error(&b[i..i + 1], "invalid character"));
        }
        i += size;
    }

    Err(new_parser_error(
        &b[b.len()..],
        "unterminated literal string",
    ))
}

// Go: unstable/scanner.go:scanMultilineLiteralString
fn scan_multiline_literal_string(b: &[u8]) -> Result<(&[u8], &[u8]), ParserError<'_>> {
    // ml-literal-string = ml-literal-string-delim [ newline ] ml-literal-body
    // ml-literal-string-delim
    // ml-literal-string-delim = 3apostrophe
    // ml-literal-body = *mll-content *( mll-quotes 1*mll-content ) [ mll-quotes ]
    //
    // mll-content = mll-char / newline
    // mll-char = %x09 / %x20-26 / %x28-7E / non-ascii
    // mll-quotes = 1*2apostrophe
    let mut i = 3;
    while i < b.len() {
        match b[i] {
            b'\'' => {
                if scan_follows_multiline_literal_string_delimiter(&b[i..]) {
                    i += 3;

                    // At that point we found 3 apostrophe, and i is the
                    // index of the byte after the third one. The scanner
                    // needs to be eager, because there can be an extra 2
                    // apostrophe that can be accepted at the end of the
                    // string.

                    if i >= b.len() || b[i] != b'\'' {
                        return Ok((&b[..i], &b[i..]));
                    }
                    i += 1;

                    if i >= b.len() || b[i] != b'\'' {
                        return Ok((&b[..i], &b[i..]));
                    }
                    i += 1;

                    if i < b.len() && b[i] == b'\'' {
                        return Err(new_parser_error(
                            &b[i - 3..i + 1],
                            "''' not allowed in multiline literal string",
                        ));
                    }

                    return Ok((&b[..i], &b[i..]));
                }
            }
            b'\r' => {
                if b.len() < i + 2 {
                    return Err(new_parser_error(&b[b.len()..], "need a \\n after \\r"));
                }
                if b[i + 1] != b'\n' {
                    return Err(new_parser_error(&b[i..i + 2], "need a \\n after \\r"));
                }
                i += 2; // skip the \n
                continue;
            }
            _ => {}
        }
        let size = utf8_valid_next(&b[i..]);
        if size == 0 {
            return Err(new_parser_error(&b[i..i + 1], "invalid character"));
        }
        i += size;
    }

    Err(new_parser_error(
        &b[b.len()..],
        "multiline literal string not terminated by '''",
    ))
}

// Go: unstable/scanner.go:scanWindowsNewline
fn scan_windows_newline(b: &[u8]) -> Result<(&[u8], &[u8]), ParserError<'_>> {
    const LEN_CRLF: usize = 2;
    if b.len() < LEN_CRLF {
        return Err(new_parser_error(b, "windows new line expected"));
    }

    if b[1] != b'\n' {
        return Err(new_parser_error(b, "windows new line should be \\r\\n"));
    }

    Ok((&b[..LEN_CRLF], &b[LEN_CRLF..]))
}

// Go: unstable/scanner.go:scanWhitespace
fn scan_whitespace(b: &[u8]) -> (&[u8], &[u8]) {
    for i in 0..b.len() {
        match b[i] {
            b' ' | b'\t' => continue,
            _ => return (&b[..i], &b[i..]),
        }
    }

    (b, &b[b.len()..])
}

// Go: unstable/scanner.go:scanComment
fn scan_comment(b: &[u8]) -> Result<(&[u8], &[u8]), ParserError<'_>> {
    // comment-start-symbol = %x23 ; #
    // non-ascii = %x80-D7FF / %xE000-10FFFF
    // non-eol = %x09 / %x20-7F / non-ascii
    //
    // comment = comment-start-symbol *non-eol

    let mut i = 1;
    while i < b.len() {
        if b[i] == b'\n' {
            return Ok((&b[..i], &b[i..]));
        }
        if b[i] == b'\r' {
            if i + 1 < b.len() && b[i + 1] == b'\n' {
                return Ok((&b[..i + 1], &b[i + 1..]));
            }
            return Err(new_parser_error(
                &b[i..i + 1],
                "invalid character in comment",
            ));
        }
        let size = utf8_valid_next(&b[i..]);
        if size == 0 {
            return Err(new_parser_error(
                &b[i..i + 1],
                "invalid character in comment",
            ));
        }

        i += size;
    }

    Ok((b, &b[b.len()..]))
}

// Go: unstable/scanner.go:scanBasicString
fn scan_basic_string(b: &[u8]) -> Result<(&[u8], bool, &[u8]), ParserError<'_>> {
    // basic-string = quotation-mark *basic-char quotation-mark
    // quotation-mark = %x22            ; "
    // basic-char = basic-unescaped / escaped
    // basic-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
    // escaped = escape escape-seq-char
    let mut escaped = false;
    let mut i = 1;

    while i < b.len() {
        match b[i] {
            b'"' => return Ok((&b[..i + 1], escaped, &b[i + 1..])),
            b'\n' | b'\r' => {
                return Err(new_parser_error(
                    &b[i..i + 1],
                    "basic strings cannot have new lines",
                ));
            }
            b'\\' => {
                if b.len() < i + 2 {
                    return Err(new_parser_error(&b[i..i + 1], "need a character after \\"));
                }
                escaped = true;
                i += 1; // skip the next character
            }
            _ => {}
        }
        i += 1;
    }

    Err(new_parser_error(
        &b[b.len()..],
        "basic string not terminated by \"",
    ))
}

// Go: unstable/scanner.go:scanMultilineBasicString
fn scan_multiline_basic_string(b: &[u8]) -> Result<(&[u8], bool, &[u8]), ParserError<'_>> {
    // ml-basic-string = ml-basic-string-delim [ newline ] ml-basic-body
    // ml-basic-string-delim
    // ml-basic-string-delim = 3quotation-mark
    // ml-basic-body = *mlb-content *( mlb-quotes 1*mlb-content ) [ mlb-quotes ]
    //
    // mlb-content = mlb-char / newline / mlb-escaped-nl
    // mlb-char = mlb-unescaped / escaped
    // mlb-quotes = 1*2quotation-mark
    // mlb-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
    // mlb-escaped-nl = escape ws newline *( wschar / newline )

    let mut escaped = false;
    let mut i = 3;

    while i < b.len() {
        match b[i] {
            b'"' => {
                if scan_follows_multiline_basic_string_delimiter(&b[i..]) {
                    i += 3;

                    // At that point we found 3 apostrophe, and i is the
                    // index of the byte after the third one. The scanner
                    // needs to be eager, because there can be an extra 2
                    // apostrophe that can be accepted at the end of the
                    // string.

                    if i >= b.len() || b[i] != b'"' {
                        return Ok((&b[..i], escaped, &b[i..]));
                    }
                    i += 1;

                    if i >= b.len() || b[i] != b'"' {
                        return Ok((&b[..i], escaped, &b[i..]));
                    }
                    i += 1;

                    if i < b.len() && b[i] == b'"' {
                        return Err(new_parser_error(
                            &b[i - 3..i + 1],
                            "\"\"\" not allowed in multiline basic string",
                        ));
                    }

                    return Ok((&b[..i], escaped, &b[i..]));
                }
            }
            b'\\' => {
                if b.len() < i + 2 {
                    return Err(new_parser_error(&b[b.len()..], "need a character after \\"));
                }
                escaped = true;
                i += 1; // skip the next character
            }
            b'\r' => {
                if b.len() < i + 2 {
                    return Err(new_parser_error(&b[b.len()..], "need a \\n after \\r"));
                }
                if b[i + 1] != b'\n' {
                    return Err(new_parser_error(&b[i..i + 2], "need a \\n after \\r"));
                }
                i += 1; // skip the \n
            }
            _ => {}
        }
        i += 1;
    }

    Err(new_parser_error(
        &b[b.len()..],
        "multiline basic string not terminated by \"\"\"",
    ))
}
