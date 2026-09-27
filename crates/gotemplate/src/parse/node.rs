//! Parse nodes.
//!
//! Go: tpl/internal/go_templates/texttemplate/parse/node.go
//!
//! Go's `Node` interface plus one pointer type per node kind becomes the
//! [`Node`] enum over plain structs. Children are owned (`Vec`/`Box`), so a
//! tree is a value; sharing and mutation of whole trees (Go `*parse.Tree`)
//! is modelled by [`crate::parse::SharedTree`].
//!
//! Go identifies nodes by pointer (the html/template escaper records edits
//! in `map[*parse.ActionNode]...`). The nodes that are edit targets
//! ([`ActionNode`], [`TemplateNode`], [`TextNode`]) carry a [`NodeId`]
//! instead. `copy()` (Go `Copy`) allocates fresh ids, exactly like Go
//! allocates new nodes; Rust's `Clone` keeps them (it is used for
//! copy-on-write of a shared tree, which is still "the same" Go tree).

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::lex::ItemType;

/// Go: `parse.Pos` — a byte position in the original input text.
pub type Pos = usize;

/// Identity of an edit-target node (replaces Go pointer identity).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct NodeId(u64);

impl NodeId {
    /// A fresh, process-unique id.
    pub fn fresh() -> NodeId {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        NodeId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for NodeId {
    fn default() -> Self {
        NodeId::fresh()
    }
}

/// The parts of a Go `*parse.Tree` that nodes point back to (Go: the
/// unexported `Node.tree()`), used by `ErrorContext`: the top-level template
/// name during parsing and the full parsed text. All trees produced by one
/// `Parse` call share one `TreeSrc`.
#[derive(Debug)]
pub struct TreeSrc {
    pub parse_name: String,
    pub text: Arc<[u8]>,
}

/// Go: a node's `tr *Tree` (nil for nodes created outside the parser).
pub type TreeRef = Option<Arc<TreeSrc>>;

/// Go: `parse.NodeType`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum NodeType {
    Text,
    Action,
    Bool,
    Chain,
    Command,
    Dot,
    Else,
    End,
    Field,
    Identifier,
    If,
    List,
    Nil,
    Number,
    Pipe,
    Range,
    String,
    Template,
    Variable,
    With,
    Comment,
    Break,
    Continue,
}

/// Common accessors of every node type (Go `Node` interface minus `Copy`).
pub trait NodeLike {
    fn node_type(&self) -> NodeType;
    fn position(&self) -> Pos;
    /// Go: `tree()`.
    fn tree(&self) -> Option<&Arc<TreeSrc>>;
    /// Go: `writeTo(*strings.Builder)`.
    fn write_to(&self, sb: &mut Vec<u8>);
    /// Go: `String()`. Go strings are bytes.
    fn to_bytes(&self) -> Vec<u8> {
        let mut sb = Vec::new();
        self.write_to(&mut sb);
        sb
    }
    /// `String()` for messages (invalid UTF-8 replaced).
    fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.to_bytes()).into_owned()
    }
}

macro_rules! node_common {
    ($t:ty, $nt:expr) => {
        impl NodeLike for $t {
            fn node_type(&self) -> NodeType {
                $nt
            }
            fn position(&self) -> Pos {
                self.pos
            }
            fn tree(&self) -> Option<&Arc<TreeSrc>> {
                self.tr.as_ref()
            }
            fn write_to(&self, sb: &mut Vec<u8>) {
                self.write_to_impl(sb)
            }
        }
    };
}

/// Go: `parse.Node`.
#[derive(Clone, Debug)]
pub enum Node {
    Text(TextNode),
    Action(ActionNode),
    Bool(BoolNode),
    Chain(ChainNode),
    Command(CommandNode),
    Dot(DotNode),
    Field(FieldNode),
    Identifier(IdentifierNode),
    If(BranchNode),
    List(ListNode),
    Nil(NilNode),
    Number(NumberNode),
    Pipe(PipeNode),
    Range(BranchNode),
    String(StringNode),
    Template(TemplateNode),
    Variable(VariableNode),
    With(BranchNode),
    Comment(CommentNode),
    Break(BreakNode),
    Continue(ContinueNode),
    /// Parser-internal (`elseNode`); never in a finished tree.
    Else(ElseNode),
    /// Parser-internal (`endNode`); never in a finished tree.
    End(EndNode),
}

impl Node {
    fn inner(&self) -> &dyn NodeLike {
        match self {
            Node::Text(n) => n,
            Node::Action(n) => n,
            Node::Bool(n) => n,
            Node::Chain(n) => n,
            Node::Command(n) => n,
            Node::Dot(n) => n,
            Node::Field(n) => n,
            Node::Identifier(n) => n,
            Node::If(n) | Node::Range(n) | Node::With(n) => n,
            Node::List(n) => n,
            Node::Nil(n) => n,
            Node::Number(n) => n,
            Node::Pipe(n) => n,
            Node::String(n) => n,
            Node::Template(n) => n,
            Node::Variable(n) => n,
            Node::Comment(n) => n,
            Node::Break(n) => n,
            Node::Continue(n) => n,
            Node::Else(n) => n,
            Node::End(n) => n,
        }
    }

    /// Go: `Node.Copy()` — a deep copy with fresh node identities.
    pub fn copy(&self) -> Node {
        match self {
            Node::Text(n) => Node::Text(n.copy()),
            Node::Action(n) => Node::Action(n.copy()),
            Node::Bool(n) => Node::Bool(n.clone()),
            Node::Chain(n) => Node::Chain(n.copy()),
            Node::Command(n) => Node::Command(n.copy()),
            Node::Dot(n) => Node::Dot(n.clone()),
            Node::Field(n) => Node::Field(n.clone()),
            Node::Identifier(n) => Node::Identifier(n.clone()),
            Node::If(n) => Node::If(n.copy()),
            Node::List(n) => Node::List(n.copy_list()),
            Node::Nil(n) => Node::Nil(n.clone()),
            Node::Number(n) => Node::Number(n.clone()),
            Node::Pipe(n) => Node::Pipe(n.copy_pipe()),
            Node::Range(n) => Node::Range(n.copy()),
            Node::String(n) => Node::String(n.clone()),
            Node::Template(n) => Node::Template(n.copy()),
            Node::Variable(n) => Node::Variable(n.clone()),
            Node::With(n) => Node::With(n.copy()),
            Node::Comment(n) => Node::Comment(n.clone()),
            Node::Break(n) => Node::Break(n.clone()),
            Node::Continue(n) => Node::Continue(n.clone()),
            Node::Else(n) => Node::Else(n.clone()),
            Node::End(n) => Node::End(n.clone()),
        }
    }
}

impl NodeLike for Node {
    fn node_type(&self) -> NodeType {
        self.inner().node_type()
    }
    fn position(&self) -> Pos {
        self.inner().position()
    }
    fn tree(&self) -> Option<&Arc<TreeSrc>> {
        self.inner().tree()
    }
    fn write_to(&self, sb: &mut Vec<u8>) {
        self.inner().write_to(sb)
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

// ---------------------------------------------------------------------------
// ListNode

/// Go: `ListNode` — a sequence of nodes.
#[derive(Clone, Debug, Default)]
pub struct ListNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// The element nodes in lexical order.
    pub nodes: Vec<Node>,
}

impl ListNode {
    pub fn new(tr: TreeRef, pos: Pos) -> ListNode {
        ListNode {
            pos,
            tr,
            nodes: Vec::new(),
        }
    }

    pub(crate) fn append(&mut self, n: Node) {
        self.nodes.push(n);
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        for n in &self.nodes {
            n.write_to(sb);
        }
    }

    // Go: node.go:(*ListNode).CopyList
    pub fn copy_list(&self) -> ListNode {
        let mut n = ListNode::new(self.tr.clone(), self.pos);
        for elem in &self.nodes {
            n.append(elem.copy());
        }
        n
    }
}
node_common!(ListNode, NodeType::List);

// ---------------------------------------------------------------------------
// TextNode

/// Go: `TextNode` — plain text.
#[derive(Clone, Debug)]
pub struct TextNode {
    pub pos: Pos,
    pub tr: TreeRef,
    pub id: NodeId,
    /// The text; may span newlines.
    pub text: Vec<u8>,
}

impl TextNode {
    pub fn new(tr: TreeRef, pos: Pos, text: Vec<u8>) -> TextNode {
        TextNode {
            pos,
            tr,
            id: NodeId::fresh(),
            text,
        }
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        // textFormat is "%s".
        sb.extend_from_slice(&self.text);
    }

    pub fn copy(&self) -> TextNode {
        TextNode::new(self.tr.clone(), self.pos, self.text.clone())
    }
}
node_common!(TextNode, NodeType::Text);

// ---------------------------------------------------------------------------
// CommentNode

/// Go: `CommentNode`.
#[derive(Clone, Debug)]
pub struct CommentNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// Comment text.
    pub text: Vec<u8>,
}

impl CommentNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(b"{{");
        sb.extend_from_slice(&self.text);
        sb.extend_from_slice(b"}}");
    }
}
node_common!(CommentNode, NodeType::Comment);

// ---------------------------------------------------------------------------
// PipeNode

/// Go: `PipeNode` — a pipeline with optional declaration.
#[derive(Clone, Debug)]
pub struct PipeNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// The line number in the input. Deprecated: Kept for compatibility.
    pub line: usize,
    /// The variables are being assigned, not declared.
    pub is_assign: bool,
    /// Variables in lexical order.
    pub decl: Vec<VariableNode>,
    /// The commands in lexical order.
    pub cmds: Vec<CommandNode>,
}

impl PipeNode {
    pub fn new(tr: TreeRef, pos: Pos, line: usize, vars: Vec<VariableNode>) -> PipeNode {
        PipeNode {
            pos,
            tr,
            line,
            is_assign: false,
            decl: vars,
            cmds: Vec::new(),
        }
    }

    pub(crate) fn append(&mut self, command: CommandNode) {
        self.cmds.push(command);
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        if !self.decl.is_empty() {
            for (i, v) in self.decl.iter().enumerate() {
                if i > 0 {
                    sb.extend_from_slice(b", ");
                }
                v.write_to(sb);
            }
            if self.is_assign {
                sb.extend_from_slice(b" = ");
            } else {
                sb.extend_from_slice(b" := ");
            }
        }
        for (i, c) in self.cmds.iter().enumerate() {
            if i > 0 {
                sb.extend_from_slice(b" | ");
            }
            c.write_to(sb);
        }
    }

    // Go: node.go:(*PipeNode).CopyPipe
    pub fn copy_pipe(&self) -> PipeNode {
        let vars = self.decl.clone();
        let mut n = PipeNode::new(self.tr.clone(), self.pos, self.line, vars);
        n.is_assign = self.is_assign;
        for c in &self.cmds {
            n.append(c.copy());
        }
        n
    }
}
node_common!(PipeNode, NodeType::Pipe);

// ---------------------------------------------------------------------------
// ActionNode

/// Go: `ActionNode` — an action (something bounded by delimiters).
#[derive(Clone, Debug)]
pub struct ActionNode {
    pub pos: Pos,
    pub tr: TreeRef,
    pub id: NodeId,
    /// The line number in the input. Deprecated: Kept for compatibility.
    pub line: usize,
    /// The pipeline in the action.
    pub pipe: PipeNode,
}

impl ActionNode {
    pub fn new(tr: TreeRef, pos: Pos, line: usize, pipe: PipeNode) -> ActionNode {
        ActionNode {
            pos,
            tr,
            id: NodeId::fresh(),
            line,
            pipe,
        }
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(b"{{");
        self.pipe.write_to(sb);
        sb.extend_from_slice(b"}}");
    }

    pub fn copy(&self) -> ActionNode {
        ActionNode::new(self.tr.clone(), self.pos, self.line, self.pipe.copy_pipe())
    }
}
node_common!(ActionNode, NodeType::Action);

// ---------------------------------------------------------------------------
// CommandNode

/// Go: `CommandNode` — an element of a pipeline.
#[derive(Clone, Debug)]
pub struct CommandNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// Arguments in lexical order: Identifier, field, or constant.
    pub args: Vec<Node>,
}

impl CommandNode {
    pub fn new(tr: TreeRef, pos: Pos) -> CommandNode {
        CommandNode {
            pos,
            tr,
            args: Vec::new(),
        }
    }

    pub(crate) fn append(&mut self, arg: Node) {
        self.args.push(arg);
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                sb.push(b' ');
            }
            if let Node::Pipe(arg) = arg {
                sb.push(b'(');
                arg.write_to(sb);
                sb.push(b')');
                continue;
            }
            arg.write_to(sb);
        }
    }

    pub fn copy(&self) -> CommandNode {
        let mut n = CommandNode::new(self.tr.clone(), self.pos);
        for c in &self.args {
            n.append(c.copy());
        }
        n
    }
}
node_common!(CommandNode, NodeType::Command);

// ---------------------------------------------------------------------------
// IdentifierNode

/// Go: `IdentifierNode` — an identifier; always a function name.
#[derive(Clone, Debug)]
pub struct IdentifierNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// The identifier's name.
    pub ident: String,
}

impl IdentifierNode {
    /// Go: `NewIdentifier(ident).SetTree(tr).SetPos(pos)`.
    pub fn new(ident: impl Into<String>, tr: TreeRef, pos: Pos) -> IdentifierNode {
        IdentifierNode {
            pos,
            tr,
            ident: ident.into(),
        }
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(self.ident.as_bytes());
    }
}
node_common!(IdentifierNode, NodeType::Identifier);

// ---------------------------------------------------------------------------
// VariableNode

/// Go: `VariableNode` — a list of variable names, possibly with chained
/// field accesses. The dollar sign is part of the (first) name.
#[derive(Clone, Debug)]
pub struct VariableNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// Variable name and fields in lexical order.
    pub ident: Vec<String>,
}

impl VariableNode {
    pub fn new(tr: TreeRef, pos: Pos, ident: &str) -> VariableNode {
        VariableNode {
            pos,
            tr,
            ident: ident.split('.').map(str::to_string).collect(),
        }
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        for (i, id) in self.ident.iter().enumerate() {
            if i > 0 {
                sb.push(b'.');
            }
            sb.extend_from_slice(id.as_bytes());
        }
    }
}
node_common!(VariableNode, NodeType::Variable);

// ---------------------------------------------------------------------------
// DotNode, NilNode

/// Go: `DotNode` — the special identifier '.'.
#[derive(Clone, Debug)]
pub struct DotNode {
    pub pos: Pos,
    pub tr: TreeRef,
}

impl DotNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.push(b'.');
    }
}
node_common!(DotNode, NodeType::Dot);

/// Go: `NilNode` — the untyped nil constant.
#[derive(Clone, Debug)]
pub struct NilNode {
    pub pos: Pos,
    pub tr: TreeRef,
}

impl NilNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(b"nil");
    }
}
node_common!(NilNode, NodeType::Nil);

// ---------------------------------------------------------------------------
// FieldNode

/// Go: `FieldNode` — a field (identifier starting with '.'). The names may
/// be chained ('.x.y'). The period is dropped from each ident.
#[derive(Clone, Debug)]
pub struct FieldNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// The identifiers in lexical order.
    pub ident: Vec<String>,
}

impl FieldNode {
    pub fn new(tr: TreeRef, pos: Pos, ident: &str) -> FieldNode {
        // [1:] to drop leading period
        FieldNode {
            pos,
            tr,
            ident: ident[1..].split('.').map(str::to_string).collect(),
        }
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        for id in &self.ident {
            sb.push(b'.');
            sb.extend_from_slice(id.as_bytes());
        }
    }
}
node_common!(FieldNode, NodeType::Field);

// ---------------------------------------------------------------------------
// ChainNode

/// Go: `ChainNode` — a term followed by a chain of field accesses.
#[derive(Clone, Debug)]
pub struct ChainNode {
    pub pos: Pos,
    pub tr: TreeRef,
    pub node: Box<Node>,
    /// The identifiers in lexical order.
    pub field: Vec<String>,
}

impl ChainNode {
    pub fn new(tr: TreeRef, pos: Pos, node: Node) -> ChainNode {
        ChainNode {
            pos,
            tr,
            node: Box::new(node),
            field: Vec::new(),
        }
    }

    // Go: node.go:(*ChainNode).Add
    /// Adds the named field (which should start with a period) to the end of
    /// the chain.
    pub fn add(&mut self, field: &str) {
        if field.is_empty() || !field.starts_with('.') {
            panic!("no dot in field");
        }
        let field = &field[1..]; // Remove leading dot.
        if field.is_empty() {
            panic!("empty field");
        }
        self.field.push(field.to_string());
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        if let Node::Pipe(p) = &*self.node {
            sb.push(b'(');
            p.write_to(sb);
            sb.push(b')');
        } else {
            self.node.write_to(sb);
        }
        for field in &self.field {
            sb.push(b'.');
            sb.extend_from_slice(field.as_bytes());
        }
    }

    /// Go: `(*ChainNode).Copy` — note that Go does NOT deep-copy `Node`
    /// (the copy shares it). Nothing mutates a chain's inner node, so a
    /// structural clone is equivalent.
    pub fn copy(&self) -> ChainNode {
        self.clone()
    }
}
node_common!(ChainNode, NodeType::Chain);

// ---------------------------------------------------------------------------
// BoolNode

/// Go: `BoolNode` — a boolean constant.
#[derive(Clone, Debug)]
pub struct BoolNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// The value of the boolean constant.
    pub true_: bool,
}

impl BoolNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(if self.true_ { b"true" } else { b"false" });
    }
}
node_common!(BoolNode, NodeType::Bool);

// ---------------------------------------------------------------------------
// NumberNode

/// Go: `NumberNode` — a number: signed or unsigned integer, float, or
/// complex. The value is parsed and stored under all the types that can
/// represent the value.
#[derive(Clone, Debug, Default)]
pub struct NumberNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// Number has an integral value.
    pub is_int: bool,
    /// Number has an unsigned integral value.
    pub is_uint: bool,
    /// Number has a floating-point value.
    pub is_float: bool,
    /// Number is complex.
    pub is_complex: bool,
    /// The signed integer value.
    pub int64: i64,
    /// The unsigned integer value.
    pub uint64: u64,
    /// The floating-point value.
    pub float64: f64,
    /// The complex value (real, imaginary).
    pub complex128: (f64, f64),
    /// The original textual representation from the input (bytes: a
    /// character constant may hold any byte).
    pub text: Vec<u8>,
}

impl NumberNode {
    // Go: node.go:(*Tree).newNumber
    /// Parses the number item `text` of lexer type `typ` (`CharConstant`,
    /// `Complex` or `Number`).
    pub fn new(tr: TreeRef, pos: Pos, text: &[u8], typ: ItemType) -> Result<NumberNode, String> {
        let mut n = NumberNode {
            pos,
            tr,
            text: text.to_vec(),
            ..Default::default()
        };
        match typ {
            ItemType::CharConstant => {
                let (rune, _, tail) =
                    go_strconv::unquote_char(&text[1..], text[0]).map_err(|e| e.to_string())?;
                if tail != b"'" {
                    return Err(format!(
                        "malformed character constant: {}",
                        String::from_utf8_lossy(text)
                    ));
                }
                n.int64 = rune as i64;
                n.is_int = true;
                n.uint64 = rune as i64 as u64;
                n.is_uint = true;
                n.float64 = rune as f64; // odd but those are the rules.
                n.is_float = true;
                return Ok(n);
            }
            ItemType::Complex => {
                // fmt.Sscan can parse the pair, so let it do the work.
                let c = sscan_complex(text)?;
                n.complex128 = c;
                n.is_complex = true;
                n.simplify_complex();
                return Ok(n);
            }
            _ => {}
        }
        // Imaginary constants can only be complex unless they are zero.
        if !text.is_empty() && text.ends_with(b"i") {
            if let Ok(f) = go_strconv::parse_float(&text[..text.len() - 1], 64) {
                n.is_complex = true;
                n.complex128 = (0.0, f);
                n.simplify_complex();
                return Ok(n);
            }
        }
        // Do integer test first so we get 0x123 etc.
        let u = go_strconv::parse_uint(text, 0, 64); // will fail for -0; fixed below.
        if let Ok(u) = u {
            n.is_uint = true;
            n.uint64 = u;
        }
        if let Ok(i) = go_strconv::parse_int(text, 0, 64) {
            n.is_int = true;
            n.int64 = i;
            if i == 0 {
                n.is_uint = true; // in case of -0.
                n.uint64 = u.as_ref().copied().unwrap_or(0);
            }
        }
        // If an integer extraction succeeded, promote the float.
        if n.is_int {
            n.is_float = true;
            n.float64 = n.int64 as f64;
        } else if n.is_uint {
            n.is_float = true;
            n.float64 = n.uint64 as f64;
        } else if let Ok(f) = go_strconv::parse_float(text, 64) {
            // If we parsed it as a float but it looks like an integer,
            // it's a huge number too large to fit in an int. Reject it.
            if !text.iter().any(|c| b".eEpP".contains(c)) {
                return Err(format!("integer overflow: {}", go_strconv::quote(text)));
            }
            n.is_float = true;
            n.float64 = f;
            // If a floating-point extraction succeeded, extract the int if needed.
            if !n.is_int && go_int64(f) as f64 == f {
                n.is_int = true;
                n.int64 = go_int64(f);
            }
            if !n.is_uint && go_uint64(f) as f64 == f {
                n.is_uint = true;
                n.uint64 = go_uint64(f);
            }
        }
        if !n.is_int && !n.is_uint && !n.is_float {
            return Err(format!(
                "illegal number syntax: {}",
                go_strconv::quote(text)
            ));
        }
        Ok(n)
    }

    // Go: node.go:(*NumberNode).simplifyComplex
    /// Pulls out any other types that are represented by the complex number.
    /// These all require that the imaginary part be zero.
    fn simplify_complex(&mut self) {
        self.is_float = self.complex128.1 == 0.0;
        if self.is_float {
            self.float64 = self.complex128.0;
            self.is_int = go_int64(self.float64) as f64 == self.float64;
            if self.is_int {
                self.int64 = go_int64(self.float64);
            }
            self.is_uint = go_uint64(self.float64) as f64 == self.float64;
            if self.is_uint {
                self.uint64 = go_uint64(self.float64);
            }
        }
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(&self.text);
    }
}
node_common!(NumberNode, NodeType::Number);

/// Go `int64(f)` as compiled for darwin/arm64 (FCVTZS: saturating, NaN → 0),
/// which is also Rust's `as`.
fn go_int64(f: f64) -> i64 {
    f as i64
}

/// Go `uint64(f)` on arm64 (FCVTZU: saturating, negative/NaN → 0).
fn go_uint64(f: f64) -> u64 {
    f as u64
}

// ---------------------------------------------------------------------------
// fmt.Sscan(text, &complex128)

/// The part of Go's `fmt` scanner (`fmt/scan.go`, `ss` reading a string)
/// that `fmt.Sscan(text, &c)` with a `complex128` operand runs: `doScan` →
/// `scanOne('v')` → `scanComplex(verb, 128)`. Errors are the scanner's
/// (`errComplex`, the `strconv.NumError` of `convertFloat`, `io.EOF`).
struct ComplexScanner<'a> {
    input: &'a [u8],
    pos: usize,
    buf: Vec<u8>,
}

const SCAN_SIGN: &[u8] = b"+-";
const SCAN_PERIOD: &[u8] = b".";
const SCAN_EXPONENT: &[u8] = b"eEpP";
const ERR_COMPLEX: &str = "syntax error scanning complex number";

impl ComplexScanner<'_> {
    /// Go `ss.getRune` on a string reader: the next rune, `None` at EOF.
    fn get_rune(&mut self) -> Option<(go_unicode::Rune, usize)> {
        if self.pos >= self.input.len() {
            return None;
        }
        let (r, w) = go_unicode::utf8::decode_rune(&self.input[self.pos..]);
        self.pos += w;
        Some((r, w))
    }

    // Go: fmt/scan.go:(*ss).consume (with accept == true)
    /// Reads the next rune in the input and reports whether it is in the ok
    /// string, putting it into the token buffer if so.
    fn accept(&mut self, ok: &[u8]) -> bool {
        let Some((r, w)) = self.get_rune() else {
            return false;
        };
        if r >= 0 && r < 0x80 && ok.contains(&(r as u8)) {
            self.buf.push(r as u8);
            return true;
        }
        self.pos -= w; // UnreadRune
        false
    }

    // Go: fmt/scan.go:(*ss).SkipSpace (nlIsSpace, as for Sscan)
    fn skip_space(&mut self) {
        while let Some((r, w)) = self.get_rune() {
            if !scan_is_space(r) {
                self.pos -= w;
                break;
            }
        }
    }

    // Go: fmt/scan.go:(*ss).floatToken
    /// Returns the floating-point number starting here. It's not rigorous
    /// about syntax because it doesn't check that we have at least some
    /// digits, but Atof will do that.
    fn float_token(&mut self) -> Vec<u8> {
        self.buf.clear();
        // NaN?
        if self.accept(b"nN") && self.accept(b"aA") && self.accept(b"nN") {
            return self.buf.clone();
        }
        // leading sign?
        self.accept(SCAN_SIGN);
        // Inf?
        if self.accept(b"iI") && self.accept(b"nN") && self.accept(b"fF") {
            return self.buf.clone();
        }
        // decimalDigits + "_", hexadecimalDigits + "_"
        let mut digits: &[u8] = b"0123456789_";
        let mut exp = SCAN_EXPONENT;
        if self.accept(b"0") && self.accept(b"xX") {
            digits = b"0123456789aAbBcCdDeEfF_";
            exp = b"pP";
        }
        // digits?
        while self.accept(digits) {}
        // decimal point?
        if self.accept(SCAN_PERIOD) {
            // fraction?
            while self.accept(digits) {}
        }
        // exponent?
        if self.accept(exp) {
            // leading sign?
            self.accept(SCAN_SIGN);
            // digits?
            while self.accept(b"0123456789_") {}
        }
        self.buf.clone()
    }

    // Go: fmt/scan.go:(*ss).complexTokens
    /// Returns the real and imaginary parts of the complex number starting
    /// here. The number might be parenthesized and has the format (N+Ni)
    /// where N is a floating-point number and there are no spaces within.
    fn complex_tokens(&mut self) -> Result<(Vec<u8>, Vec<u8>), String> {
        let parens = self.accept(b"(");
        let real = self.float_token();
        self.buf.clear();
        // Must now have a sign.
        if !self.accept(b"+-") {
            return Err(ERR_COMPLEX.to_string());
        }
        // Sign is now in buffer
        let mut imag = self.buf.clone();
        imag.extend_from_slice(&self.float_token());
        if !self.accept(b"i") {
            return Err(ERR_COMPLEX.to_string());
        }
        if parens && !self.accept(b")") {
            return Err(ERR_COMPLEX.to_string());
        }
        Ok((real, imag))
    }
}

// Go: fmt/scan.go:isSpace (the space table)
fn scan_is_space(r: go_unicode::Rune) -> bool {
    const SPACE: &[(u16, u16)] = &[
        (0x0009, 0x000d),
        (0x0020, 0x0020),
        (0x0085, 0x0085),
        (0x00a0, 0x00a0),
        (0x1680, 0x1680),
        (0x2000, 0x200a),
        (0x2028, 0x2029),
        (0x202f, 0x202f),
        (0x205f, 0x205f),
        (0x3000, 0x3000),
    ];
    if r >= 1 << 16 {
        return false;
    }
    let rx = r as u16;
    for &(lo, hi) in SPACE {
        if rx < lo {
            return false;
        }
        if rx <= hi {
            return true;
        }
    }
    false
}

// Go: fmt/scan.go:(*ss).convertFloat (n == 64)
/// Converts the string to a float64 value.
fn scan_convert_float(s: &[u8]) -> Result<f64, String> {
    // strconv.ParseFloat will handle "+0x1.fp+2",
    // but we have to implement our non-standard
    // decimal+binary exponent mix (1.2p4) ourselves.
    let has_x = s.iter().any(|&c| c == b'x' || c == b'X');
    if let Some(p) = s.iter().position(|&c| c == b'p').filter(|_| !has_x) {
        // Atof doesn't handle power-of-2 exponents,
        // but they're easy to evaluate.
        let f = go_strconv::parse_float(&s[..p], 64).map_err(|mut e| {
            // Put full string into error.
            e.num = s.to_vec();
            e.to_string()
        })?;
        let m = go_strconv::atoi(&s[p + 1..]).map_err(|mut e| {
            e.num = s.to_vec();
            e.to_string()
        })?;
        return Ok(ldexp(f, m));
    }
    go_strconv::parse_float(s, 64).map_err(|e| e.to_string())
}

/// Go `fmt.Sscan(text, &c)` for a `complex128` `c` (fmt/scan.go:
/// `scanComplex`): the value, or the error Sscan returns.
fn sscan_complex(text: &[u8]) -> Result<(f64, f64), String> {
    let mut s = ComplexScanner {
        input: text,
        pos: 0,
        buf: Vec::new(),
    };
    s.skip_space();
    // Go: notEOF — guarantee there is data to be read.
    if s.pos >= s.input.len() {
        return Err("EOF".to_string());
    }
    let (sreal, simag) = s.complex_tokens()?;
    let real = scan_convert_float(&sreal)?;
    let imag = scan_convert_float(&simag)?;
    Ok((real, imag))
}

// Go: math/ldexp.go:ldexp (the portable version; no arch assembly on
// amd64/arm64)
/// Ldexp is the inverse of Frexp. It returns frac × 2**exp.
fn ldexp(frac: f64, exp: i64) -> f64 {
    const SHIFT: u32 = 64 - 11 - 1;
    const MASK: u64 = 0x7FF;
    const BIAS: i64 = 1023;
    // special cases
    if frac == 0.0 || frac.is_infinite() || frac.is_nan() {
        return frac; // correctly return -0
    }
    // Go: normalize
    let (frac, e) = if frac.abs() < 2.2250738585072014e-308 {
        (frac * (1u64 << 52) as f64, -52)
    } else {
        (frac, 0)
    };
    let mut exp = exp.saturating_add(e);
    let mut x = frac.to_bits();
    exp = exp.saturating_add(((x >> SHIFT) & MASK) as i64 - BIAS);
    if exp < -1075 {
        return 0f64.copysign(frac); // underflow
    }
    if exp > 1023 {
        // overflow
        return if frac < 0.0 {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let mut m = 1.0;
    if exp < -1022 {
        // denormal
        exp += 53;
        m = 1.0 / (1u64 << 53) as f64; // 2**-53
    }
    x &= !(MASK << SHIFT);
    x |= ((exp + BIAS) as u64) << SHIFT;
    m * f64::from_bits(x)
}

// ---------------------------------------------------------------------------
// StringNode

/// Go: `StringNode` — a string constant. The value has been "unquoted".
#[derive(Clone, Debug)]
pub struct StringNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// The original text of the string, with quotes.
    pub quoted: Vec<u8>,
    /// The string, after quote processing.
    pub text: Vec<u8>,
}

impl StringNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(&self.quoted);
    }
}
node_common!(StringNode, NodeType::String);

// ---------------------------------------------------------------------------
// endNode, elseNode (parser internal)

/// Go: `endNode` — an `{{end}}` action. Does not appear in the final tree.
#[derive(Clone, Debug)]
pub struct EndNode {
    pub pos: Pos,
    pub tr: TreeRef,
}

impl EndNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(b"{{end}}");
    }
}
node_common!(EndNode, NodeType::End);

/// Go: `elseNode` — an `{{else}}` action. Does not appear in the final tree.
#[derive(Clone, Debug)]
pub struct ElseNode {
    pub pos: Pos,
    pub tr: TreeRef,
    /// The line number in the input. Deprecated: Kept for compatibility.
    pub line: usize,
}

impl ElseNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(b"{{else}}");
    }
}
node_common!(ElseNode, NodeType::Else);

// ---------------------------------------------------------------------------
// BranchNode (IfNode, RangeNode, WithNode)

/// Go: `BranchNode` — the common representation of if, range, and with.
/// Go's `IfNode`/`RangeNode`/`WithNode` are the [`Node::If`],
/// [`Node::Range`] and [`Node::With`] variants.
#[derive(Clone, Debug)]
pub struct BranchNode {
    /// `NodeType::If`, `NodeType::Range` or `NodeType::With`.
    pub node_type: NodeType,
    pub pos: Pos,
    pub tr: TreeRef,
    /// The line number in the input. Deprecated: Kept for compatibility.
    pub line: usize,
    /// The pipeline to be evaluated.
    pub pipe: PipeNode,
    /// What to execute if the value is non-empty.
    pub list: ListNode,
    /// What to execute if the value is empty (nil if absent).
    pub else_list: Option<ListNode>,
}

impl BranchNode {
    pub fn new(
        node_type: NodeType,
        tr: TreeRef,
        pos: Pos,
        line: usize,
        pipe: PipeNode,
        list: ListNode,
        else_list: Option<ListNode>,
    ) -> BranchNode {
        BranchNode {
            node_type,
            pos,
            tr,
            line,
            pipe,
            list,
            else_list,
        }
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        let name: &[u8] = match self.node_type {
            NodeType::If => b"if",
            NodeType::Range => b"range",
            NodeType::With => b"with",
            _ => panic!("unknown branch type"),
        };
        sb.extend_from_slice(b"{{");
        sb.extend_from_slice(name);
        sb.push(b' ');
        self.pipe.write_to(sb);
        sb.extend_from_slice(b"}}");
        self.list.write_to(sb);
        if let Some(else_list) = &self.else_list {
            sb.extend_from_slice(b"{{else}}");
            else_list.write_to(sb);
        }
        sb.extend_from_slice(b"{{end}}");
    }

    /// Go: `(*IfNode).Copy` / `(*RangeNode).Copy` / `(*WithNode).Copy`.
    pub fn copy(&self) -> BranchNode {
        BranchNode::new(
            self.node_type,
            self.tr.clone(),
            self.pos,
            self.line,
            self.pipe.copy_pipe(),
            self.list.copy_list(),
            self.else_list.as_ref().map(|l| l.copy_list()),
        )
    }
}

impl NodeLike for BranchNode {
    fn node_type(&self) -> NodeType {
        self.node_type
    }
    fn position(&self) -> Pos {
        self.pos
    }
    fn tree(&self) -> Option<&Arc<TreeSrc>> {
        self.tr.as_ref()
    }
    fn write_to(&self, sb: &mut Vec<u8>) {
        self.write_to_impl(sb)
    }
}

// ---------------------------------------------------------------------------
// BreakNode, ContinueNode

/// Go: `BreakNode` — a `{{break}}` action.
#[derive(Clone, Debug)]
pub struct BreakNode {
    pub pos: Pos,
    pub tr: TreeRef,
    pub line: usize,
}

impl BreakNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(b"{{break}}");
    }
}
node_common!(BreakNode, NodeType::Break);

/// Go: `ContinueNode` — a `{{continue}}` action.
#[derive(Clone, Debug)]
pub struct ContinueNode {
    pub pos: Pos,
    pub tr: TreeRef,
    pub line: usize,
}

impl ContinueNode {
    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(b"{{continue}}");
    }
}
node_common!(ContinueNode, NodeType::Continue);

// ---------------------------------------------------------------------------
// TemplateNode

/// Go: `TemplateNode` — a `{{template}}` action.
#[derive(Clone, Debug)]
pub struct TemplateNode {
    pub pos: Pos,
    pub tr: TreeRef,
    pub id: NodeId,
    /// The line number in the input. Deprecated: Kept for compatibility.
    pub line: usize,
    /// The name of the template (unquoted).
    pub name: String,
    /// The command to evaluate as dot for the template.
    pub pipe: Option<PipeNode>,
}

impl TemplateNode {
    pub fn new(
        tr: TreeRef,
        pos: Pos,
        line: usize,
        name: String,
        pipe: Option<PipeNode>,
    ) -> TemplateNode {
        TemplateNode {
            pos,
            tr,
            id: NodeId::fresh(),
            line,
            name,
            pipe,
        }
    }

    fn write_to_impl(&self, sb: &mut Vec<u8>) {
        sb.extend_from_slice(b"{{template ");
        go_strconv::append_quote(sb, self.name.as_bytes());
        if let Some(pipe) = &self.pipe {
            sb.push(b' ');
            pipe.write_to(sb);
        }
        sb.extend_from_slice(b"}}");
    }

    pub fn copy(&self) -> TemplateNode {
        TemplateNode::new(
            self.tr.clone(),
            self.pos,
            self.line,
            self.name.clone(),
            self.pipe.as_ref().map(|p| p.copy_pipe()),
        )
    }
}
node_common!(TemplateNode, NodeType::Template);
