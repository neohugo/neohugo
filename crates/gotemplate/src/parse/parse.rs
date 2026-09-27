//! The template parser.
//!
//! Go: tpl/internal/go_templates/texttemplate/parse/parse.go (go1.24.0)
//!
//! Go keeps the parsing state inside `*Tree` and reports errors by
//! panicking; here the state lives in [`TreeParser`] (one per tree being
//! built, sharing the lexer and the tree set exactly like Go's nested
//! `startParse` calls) and errors are returned as `Result`.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::lex::{Item, ItemType, LexOptions, Lexer};
use super::node::*;

/// Go: `parse.Mode` — flags that control parser behavior.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Mode(pub u32);

impl Mode {
    /// parse comments and add them to AST
    pub const PARSE_COMMENTS: Mode = Mode(1);
    /// do not check that functions are defined
    pub const SKIP_FUNC_CHECK: Mode = Mode(2);

    pub fn has(self, m: Mode) -> bool {
        self.0 & m.0 != 0
    }
}

impl std::ops::BitOr for Mode {
    type Output = Mode;
    fn bitor(self, rhs: Mode) -> Mode {
        Mode(self.0 | rhs.0)
    }
}

/// A set of function names known at parse time (Go: the `funcs
/// ...map[string]any` passed to `Parse`; only membership is used).
pub trait FuncNames {
    fn has_function(&self, name: &str) -> bool;
}

impl<V> FuncNames for std::collections::HashMap<String, V> {
    fn has_function(&self, name: &str) -> bool {
        self.contains_key(name)
    }
}

impl<V> FuncNames for BTreeMap<String, V> {
    fn has_function(&self, name: &str) -> bool {
        self.contains_key(name)
    }
}

impl FuncNames for std::collections::HashSet<String> {
    fn has_function(&self, name: &str) -> bool {
        self.contains(name)
    }
}

impl FuncNames for [&str] {
    fn has_function(&self, name: &str) -> bool {
        self.contains(&name)
    }
}

/// Go: `parse.Tree` — the representation of a single parsed template.
#[derive(Clone, Debug)]
pub struct Tree {
    /// name of the template represented by the tree.
    pub name: String,
    /// name of the top-level template during parsing, for error messages.
    pub parse_name: String,
    /// top-level root of the tree.
    pub root: Option<ListNode>,
    /// parsing mode.
    pub mode: Mode,
    /// text parsed to create the template (or its parent), shared with the
    /// nodes of the tree.
    pub src: TreeRef,
}

impl Tree {
    /// Go: `parse.New(name)` without funcs (and `&parse.Tree{Name: name}`).
    pub fn new(name: impl Into<String>) -> Tree {
        Tree {
            name: name.into(),
            parse_name: String::new(),
            root: None,
            mode: Mode::default(),
            src: None,
        }
    }

    // Go: parse.go:(*Tree).Copy
    /// Returns a copy of the Tree. Any parsing state is discarded.
    pub fn copy(&self) -> Tree {
        Tree {
            name: self.name.clone(),
            parse_name: self.parse_name.clone(),
            root: self.root.as_ref().map(|r| r.copy_list()),
            mode: Mode::default(),
            src: self.src.clone(),
        }
    }

    // Go: parse.go:(*Tree).ErrorContext
    /// Returns a textual representation of the location of the node in the
    /// input text. The receiver is only used when the node does not have a
    /// pointer to the tree inside.
    pub fn error_context(&self, n: &dyn NodeLike) -> (String, String) {
        error_context(Some(self), n)
    }
}

// Go: parse.go:(*Tree).ErrorContext
/// `ErrorContext` with an optional receiver (Go allows a nil `*Tree`
/// receiver when the node knows its tree).
pub fn error_context(t: Option<&Tree>, n: &dyn NodeLike) -> (String, String) {
    let pos = n.position();
    let src = n.tree().or_else(|| t.and_then(|t| t.src.as_ref()));
    let (parse_name, text): (&str, &[u8]) = match src {
        Some(s) => (&s.parse_name, &s.text),
        None => (t.map(|t| t.parse_name.as_str()).unwrap_or(""), b""),
    };
    // Go would panic slicing past the end of the text (only possible for
    // nodes of synthesized trees); clamp instead.
    let text = &text[..pos.min(text.len())];
    let byte_num = match text.iter().rposition(|&b| b == b'\n') {
        None => pos, // On first line.
        Some(i) => pos - (i + 1),
    };
    let line_num = 1 + text.iter().filter(|&&b| b == b'\n').count();
    let context = n.to_string_lossy();
    (format!("{parse_name}:{line_num}:{byte_num}"), context)
}

/// A parse error (Go: the `error` returned by `Parse`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

type PResult<T> = Result<T, ParseError>;

// Go: parse.go:Parse
/// Returns a map from template name to Tree, created by parsing the
/// templates described in the argument string. The top-level template will
/// be given the specified name. If an error is encountered, parsing stops
/// and the error is returned.
pub fn parse(
    name: &str,
    text: &[u8],
    left_delim: &str,
    right_delim: &str,
    funcs: &[&dyn FuncNames],
) -> PResult<BTreeMap<String, Tree>> {
    parse_with_mode(name, text, left_delim, right_delim, Mode::default(), funcs)
}

/// [`parse`] with an explicit [`Mode`] (Go: set `Tree.Mode` before
/// `(*Tree).Parse`).
pub fn parse_with_mode(
    name: &str,
    text: &[u8],
    left_delim: &str,
    right_delim: &str,
    mode: Mode,
    funcs: &[&dyn FuncNames],
) -> PResult<BTreeMap<String, Tree>> {
    let mut tree_set = BTreeMap::new();
    // Go: (*Tree).Parse
    let src = Arc::new(TreeSrc {
        parse_name: name.to_string(),
        text: Arc::from(text),
    });
    let mut lexer = Lexer::new(name, text, left_delim, right_delim);
    let mut t = Tree::new(name);
    t.mode = mode;
    t.parse_name = name.to_string();
    t.src = Some(src);
    let mut p = TreeParser::start_parse(t, funcs, &mut lexer, &mut tree_set);
    p.parse()?;
    p.add()?;
    Ok(tree_set)
}

// Go: parse.go:IsEmptyTree
/// Reports whether this tree (node) is empty of everything but space or
/// comments.
pub fn is_empty_tree(n: Option<&ListNode>) -> bool {
    match n {
        None => true,
        Some(l) => is_empty_list(l),
    }
}

fn is_empty_list(l: &ListNode) -> bool {
    for node in &l.nodes {
        if !is_empty_node(node) {
            return false;
        }
    }
    true
}

/// [`is_empty_tree`] for any node.
pub fn is_empty_node(n: &Node) -> bool {
    match n {
        Node::Action(_) => false,
        Node::Comment(_) => true,
        Node::If(_) => false,
        Node::List(l) => is_empty_list(l),
        Node::Range(_) => false,
        Node::Template(_) => false,
        Node::Text(t) => go_unicode::bytes::trim_space(&t.text).is_empty(),
        Node::With(_) => false,
        // Go panics with "unknown node"; such nodes cannot appear in a list.
        _ => false,
    }
}

/// The parsing state of one tree (Go: the parse-only fields of `Tree`).
struct TreeParser<'p, 'a> {
    t: Tree,
    funcs: &'p [&'p dyn FuncNames],
    lex: &'p mut Lexer<'a>,
    /// three-token lookahead for parser.
    token: [Item; 3],
    peek_count: usize,
    /// variables defined at the moment.
    vars: Vec<String>,
    tree_set: &'p mut BTreeMap<String, Tree>,
    /// line of left delim starting action
    action_line: usize,
    range_depth: usize,
}

impl<'p, 'a> TreeParser<'p, 'a> {
    // Go: parse.go:(*Tree).startParse
    /// Initializes the parser, using the lexer.
    fn start_parse(
        mut t: Tree,
        funcs: &'p [&'p dyn FuncNames],
        lex: &'p mut Lexer<'a>,
        tree_set: &'p mut BTreeMap<String, Tree>,
    ) -> TreeParser<'p, 'a> {
        t.root = None;
        let mut p = TreeParser {
            t,
            funcs,
            lex,
            token: Default::default(),
            peek_count: 0,
            vars: vec!["$".to_string()],
            tree_set,
            action_line: 0,
            range_depth: 0,
        };
        p.lex.options = LexOptions {
            emit_comment: p.t.mode.has(Mode::PARSE_COMMENTS),
            break_ok: !p.has_function("break"),
            continue_ok: !p.has_function("continue"),
        };
        p
    }

    fn tr(&self) -> TreeRef {
        self.t.src.clone()
    }

    // Go: parse.go:(*Tree).next
    /// Returns the next token.
    fn next(&mut self) -> Item {
        if self.peek_count > 0 {
            self.peek_count -= 1;
        } else {
            self.token[0] = self.lex.next_item();
        }
        self.token[self.peek_count].clone()
    }

    // Go: parse.go:(*Tree).backup
    /// Backs the input stream up one token.
    fn backup(&mut self) {
        self.peek_count += 1;
    }

    // Go: parse.go:(*Tree).backup2
    /// Backs the input stream up two tokens. The zeroth token is already there.
    fn backup2(&mut self, t1: Item) {
        self.token[1] = t1;
        self.peek_count = 2;
    }

    // Go: parse.go:(*Tree).backup3
    /// Backs the input stream up three tokens. The zeroth token is already there.
    fn backup3(&mut self, t2: Item, t1: Item) {
        // Reverse order: we're pushing back.
        self.token[1] = t1;
        self.token[2] = t2;
        self.peek_count = 3;
    }

    // Go: parse.go:(*Tree).peek
    /// Returns but does not consume the next token.
    fn peek(&mut self) -> Item {
        if self.peek_count > 0 {
            return self.token[self.peek_count - 1].clone();
        }
        self.peek_count = 1;
        self.token[0] = self.lex.next_item();
        self.token[0].clone()
    }

    // Go: parse.go:(*Tree).nextNonSpace
    /// Returns the next non-space token.
    fn next_non_space(&mut self) -> Item {
        loop {
            let token = self.next();
            if token.typ != ItemType::Space {
                return token;
            }
        }
    }

    // Go: parse.go:(*Tree).peekNonSpace
    /// Returns but does not consume the next non-space token.
    fn peek_non_space(&mut self) -> Item {
        let token = self.next_non_space();
        self.backup();
        token
    }

    // Go: parse.go:(*Tree).errorf
    /// Formats the error and terminates processing.
    fn errorf<T>(&mut self, msg: impl std::fmt::Display) -> PResult<T> {
        self.t.root = None;
        Err(ParseError(format!(
            "template: {}:{}: {}",
            self.t.parse_name, self.token[0].line, msg
        )))
    }

    // Go: parse.go:(*Tree).expect
    /// Consumes the next token and guarantees it has the required type.
    fn expect(&mut self, expected: ItemType, context: &str) -> PResult<Item> {
        let token = self.next_non_space();
        if token.typ != expected {
            return self.unexpected(&token, context);
        }
        Ok(token)
    }

    // Go: parse.go:(*Tree).expectOneOf
    /// Consumes the next token and guarantees it has one of the required types.
    fn expect_one_of(&mut self, expected1: ItemType, expected2: ItemType, context: &str) -> PResult<Item> {
        let token = self.next_non_space();
        if token.typ != expected1 && token.typ != expected2 {
            return self.unexpected(&token, context);
        }
        Ok(token)
    }

    // Go: parse.go:(*Tree).unexpected
    /// Complains about the token and terminates processing.
    fn unexpected<T>(&mut self, token: &Item, context: &str) -> PResult<T> {
        if token.typ == ItemType::Error {
            let mut extra = String::new();
            if self.action_line != 0 && self.action_line != token.line {
                extra = format!(" in action started at {}:{}", self.t.parse_name, self.action_line);
                if token.val.ends_with(b" action") {
                    extra = extra[" in action".len()..].to_string(); // avoid "action in action"
                }
            }
            return self.errorf(format!("{}{}", token.string(), extra));
        }
        self.errorf(format!("unexpected {} in {}", token.string(), context))
    }

    // Go: parse.go:(*Tree).add
    /// Adds tree to t.treeSet.
    fn add(&mut self) -> PResult<()> {
        let existing_empty = match self.tree_set.get(&self.t.name) {
            None => true,
            Some(tree) => is_empty_tree(tree.root.as_ref()),
        };
        if existing_empty {
            self.tree_set.insert(self.t.name.clone(), self.t.clone());
            return Ok(());
        }
        if !is_empty_tree(self.t.root.as_ref()) {
            let name = go_strconv::quote(&self.t.name);
            return self.errorf(format!("template: multiple definition of template {name}"));
        }
        Ok(())
    }

    /// Go: `New(name)` + copying text/mode/ParseName + `startParse` for a
    /// nested `{{define}}`/`{{block}}` tree sharing this parser's lexer.
    fn child<'q>(&'q mut self, name: &str) -> TreeParser<'q, 'a> {
        let mut t = Tree::new(name);
        t.src = self.t.src.clone();
        t.mode = self.t.mode;
        t.parse_name = self.t.parse_name.clone();
        TreeParser::start_parse(t, self.funcs, &mut *self.lex, &mut *self.tree_set)
    }

    // Go: parse.go:(*Tree).parse
    /// The top-level parser for a template, essentially the same
    /// as itemList except it also parses {{define}} actions.
    /// It runs to EOF.
    fn parse(&mut self) -> PResult<()> {
        let pos = self.peek().pos;
        self.t.root = Some(ListNode::new(self.tr(), pos));
        while self.peek().typ != ItemType::Eof {
            if self.peek().typ == ItemType::LeftDelim {
                let delim = self.next();
                if self.next_non_space().typ == ItemType::Define {
                    // name will be updated once we know it.
                    let mut new_t = self.child("definition");
                    new_t.parse_definition()?;
                    continue;
                }
                self.backup2(delim);
            }
            let n = self.text_or_action()?;
            match n.node_type() {
                NodeType::End | NodeType::Else => {
                    return self.errorf(format!("unexpected {n}"));
                }
                _ => {
                    if let Some(root) = self.t.root.as_mut() {
                        root.append(n);
                    }
                }
            }
        }
        Ok(())
    }

    // Go: parse.go:(*Tree).parseDefinition
    /// Parses a {{define}} ...  {{end}} template definition and
    /// installs the definition in t.treeSet. The "define" keyword has already
    /// been scanned.
    fn parse_definition(&mut self) -> PResult<()> {
        const CONTEXT: &str = "define clause";
        let name = self.expect_one_of(ItemType::String, ItemType::RawString, CONTEXT)?;
        match go_strconv::unquote(&name.val) {
            Ok(s) => self.t.name = String::from_utf8_lossy(&s).into_owned(),
            Err(e) => return self.errorf(e),
        }
        self.expect(ItemType::RightDelim, CONTEXT)?;
        let (root, end) = self.item_list()?;
        self.t.root = Some(root);
        if end.node_type() != NodeType::End {
            return self.errorf(format!("unexpected {end} in {CONTEXT}"));
        }
        self.add()?;
        Ok(())
    }

    // Go: parse.go:(*Tree).itemList
    /// itemList:
    ///
    ///     textOrAction*
    ///
    /// Terminates at {{end}} or {{else}}, returned separately.
    fn item_list(&mut self) -> PResult<(ListNode, Node)> {
        let pos = self.peek_non_space().pos;
        let mut list = ListNode::new(self.tr(), pos);
        while self.peek_non_space().typ != ItemType::Eof {
            let n = self.text_or_action()?;
            match n.node_type() {
                NodeType::End | NodeType::Else => return Ok((list, n)),
                _ => {}
            }
            list.append(n);
        }
        self.errorf("unexpected EOF")
    }

    // Go: parse.go:(*Tree).textOrAction
    /// textOrAction:
    ///
    ///     text | comment | action
    fn text_or_action(&mut self) -> PResult<Node> {
        let token = self.next_non_space();
        match token.typ {
            ItemType::Text => Ok(Node::Text(TextNode::new(self.tr(), token.pos, token.val))),
            ItemType::LeftDelim => {
                self.action_line = token.line;
                let r = self.action();
                // defer t.clearActionLine()
                self.action_line = 0;
                r
            }
            ItemType::Comment => Ok(Node::Comment(CommentNode {
                pos: token.pos,
                tr: self.tr(),
                text: token.val,
            })),
            _ => self.unexpected(&token, "input"),
        }
    }

    // Go: parse.go:(*Tree).action
    /// Action:
    ///
    ///     control
    ///     command ("|" command)*
    ///
    /// Left delim is past. Now get actions.
    /// First word could be a keyword such as range.
    fn action(&mut self) -> PResult<Node> {
        let token = self.next_non_space();
        match token.typ {
            ItemType::Block => return self.block_control(),
            ItemType::Break => return self.break_control(token.pos, token.line),
            ItemType::Continue => return self.continue_control(token.pos, token.line),
            ItemType::Else => return self.else_control(),
            ItemType::End => return self.end_control(),
            ItemType::If => return self.if_control(),
            ItemType::Range => return self.range_control(),
            ItemType::Template => return self.template_control(),
            ItemType::With => return self.with_control(),
            _ => {}
        }
        self.backup();
        let token = self.peek();
        // Do not pop variables; they persist until "end".
        let pipe = self.pipeline("command", ItemType::RightDelim)?;
        Ok(Node::Action(ActionNode::new(self.tr(), token.pos, token.line, pipe)))
    }

    // Go: parse.go:(*Tree).breakControl
    /// Break:
    ///
    ///     {{break}}
    ///
    /// Break keyword is past.
    fn break_control(&mut self, pos: Pos, line: usize) -> PResult<Node> {
        let token = self.next_non_space();
        if token.typ != ItemType::RightDelim {
            return self.unexpected(&token, "{{break}}");
        }
        if self.range_depth == 0 {
            return self.errorf("{{break}} outside {{range}}");
        }
        Ok(Node::Break(BreakNode {
            pos,
            tr: self.tr(),
            line,
        }))
    }

    // Go: parse.go:(*Tree).continueControl
    /// Continue:
    ///
    ///     {{continue}}
    ///
    /// Continue keyword is past.
    fn continue_control(&mut self, pos: Pos, line: usize) -> PResult<Node> {
        let token = self.next_non_space();
        if token.typ != ItemType::RightDelim {
            return self.unexpected(&token, "{{continue}}");
        }
        if self.range_depth == 0 {
            return self.errorf("{{continue}} outside {{range}}");
        }
        Ok(Node::Continue(ContinueNode {
            pos,
            tr: self.tr(),
            line,
        }))
    }

    // Go: parse.go:(*Tree).pipeline
    /// Pipeline:
    ///
    ///     declarations? command ('|' command)*
    fn pipeline(&mut self, context: &str, end: ItemType) -> PResult<PipeNode> {
        let token = self.peek_non_space();
        let mut pipe = PipeNode::new(self.tr(), token.pos, token.line, Vec::new());
        // Are there declarations or assignments?
        'decls: loop {
            let v = self.peek_non_space();
            if v.typ == ItemType::Variable {
                self.next();
                // Since space is a token, we need 3-token look-ahead here in the worst case:
                // in "$x foo" we need to read "foo" (as opposed to ":=") to know that $x is an
                // argument variable rather than a declaration. So remember the token
                // adjacent to the variable so we can push it back if necessary.
                let token_after_variable = self.peek();
                let next = self.peek_non_space();
                if next.typ == ItemType::Assign || next.typ == ItemType::Declare {
                    pipe.is_assign = next.typ == ItemType::Assign;
                    self.next_non_space();
                    pipe.decl.push(VariableNode::new(self.tr(), v.pos, v.val_str()));
                    self.vars.push(v.val_str().to_string());
                } else if next.typ == ItemType::Char && next.val == b"," {
                    self.next_non_space();
                    pipe.decl.push(VariableNode::new(self.tr(), v.pos, v.val_str()));
                    self.vars.push(v.val_str().to_string());
                    if context == "range" && pipe.decl.len() < 2 {
                        match self.peek_non_space().typ {
                            ItemType::Variable | ItemType::RightDelim | ItemType::RightParen => {
                                // second initialized variable in a range pipeline
                                continue 'decls;
                            }
                            _ => return self.errorf("range can only initialize variables"),
                        }
                    }
                    return self.errorf(format!("too many declarations in {context}"));
                } else if token_after_variable.typ == ItemType::Space {
                    self.backup3(v, token_after_variable);
                } else {
                    self.backup2(v);
                }
            }
            break;
        }
        loop {
            let token = self.next_non_space();
            if token.typ == end {
                // At this point, the pipeline is complete
                self.check_pipeline(&pipe, context)?;
                return Ok(pipe);
            }
            match token.typ {
                ItemType::Bool
                | ItemType::CharConstant
                | ItemType::Complex
                | ItemType::Dot
                | ItemType::Field
                | ItemType::Identifier
                | ItemType::Number
                | ItemType::Nil
                | ItemType::RawString
                | ItemType::String
                | ItemType::Variable
                | ItemType::LeftParen => {
                    self.backup();
                    let cmd = self.command()?;
                    pipe.append(cmd);
                }
                _ => return self.unexpected(&token, context),
            }
        }
    }

    // Go: parse.go:(*Tree).checkPipeline
    fn check_pipeline(&mut self, pipe: &PipeNode, context: &str) -> PResult<()> {
        // Reject empty pipelines
        if pipe.cmds.is_empty() {
            return self.errorf(format!("missing value for {context}"));
        }
        // Only the first command of a pipeline can start with a non executable operand
        for (i, c) in pipe.cmds[1..].iter().enumerate() {
            match c.args[0].node_type() {
                NodeType::Bool | NodeType::Dot | NodeType::Nil | NodeType::Number | NodeType::String => {
                    // With A|B|C, pipeline stage 2 is B
                    return self.errorf(format!("non executable command in pipeline stage {}", i + 2));
                }
                _ => {}
            }
        }
        Ok(())
    }

    // Go: parse.go:(*Tree).parseControl
    fn parse_control(&mut self, context: &str) -> PResult<(Pos, usize, PipeNode, ListNode, Option<ListNode>)> {
        let mark = self.vars.len();
        let r = self.parse_control_inner(context);
        // defer t.popVars(len(t.vars))
        self.vars.truncate(mark);
        r
    }

    fn parse_control_inner(&mut self, context: &str) -> PResult<(Pos, usize, PipeNode, ListNode, Option<ListNode>)> {
        let pipe = self.pipeline(context, ItemType::RightDelim)?;
        if context == "range" {
            self.range_depth += 1;
        }
        let (list, next) = self.item_list()?;
        if context == "range" {
            self.range_depth -= 1;
        }
        let mut else_list = None;
        match next.node_type() {
            NodeType::End => {} // done
            NodeType::Else => {
                // Special case for "else if" and "else with".
                // If the "else" is followed immediately by an "if" or "with",
                // the elseControl will have left the "if" or "with" token pending. Treat
                //	{{if a}}_{{else if b}}_{{end}}
                //  {{with a}}_{{else with b}}_{{end}}
                // as
                //	{{if a}}_{{else}}{{if b}}_{{end}}{{end}}
                //  {{with a}}_{{else}}{{with b}}_{{end}}{{end}}.
                // To do this, parse the "if" or "with" as usual and stop at it {{end}};
                // the subsequent{{end}} is assumed. This technique works even for long if-else-if chains.
                if context == "if" && self.peek().typ == ItemType::If {
                    self.next(); // Consume the "if" token.
                    let mut l = ListNode::new(self.tr(), next.position());
                    l.append(self.if_control()?);
                    else_list = Some(l);
                } else if context == "with" && self.peek().typ == ItemType::With {
                    self.next();
                    let mut l = ListNode::new(self.tr(), next.position());
                    l.append(self.with_control()?);
                    else_list = Some(l);
                } else {
                    let (l, next) = self.item_list()?;
                    if next.node_type() != NodeType::End {
                        return self.errorf(format!("expected end; found {next}"));
                    }
                    else_list = Some(l);
                }
            }
            _ => {}
        }
        Ok((pipe.pos, pipe.line, pipe, list, else_list))
    }

    fn branch(&mut self, nt: NodeType, context: &str) -> PResult<BranchNode> {
        let (pos, line, pipe, list, else_list) = self.parse_control(context)?;
        Ok(BranchNode::new(nt, self.tr(), pos, line, pipe, list, else_list))
    }

    // Go: parse.go:(*Tree).ifControl
    /// If:
    ///
    ///     {{if pipeline}} itemList {{end}}
    ///     {{if pipeline}} itemList {{else}} itemList {{end}}
    ///
    /// If keyword is past.
    fn if_control(&mut self) -> PResult<Node> {
        Ok(Node::If(self.branch(NodeType::If, "if")?))
    }

    // Go: parse.go:(*Tree).rangeControl
    /// Range:
    ///
    ///     {{range pipeline}} itemList {{end}}
    ///     {{range pipeline}} itemList {{else}} itemList {{end}}
    ///
    /// Range keyword is past.
    fn range_control(&mut self) -> PResult<Node> {
        Ok(Node::Range(self.branch(NodeType::Range, "range")?))
    }

    // Go: parse.go:(*Tree).withControl
    /// With:
    ///
    ///     {{with pipeline}} itemList {{end}}
    ///     {{with pipeline}} itemList {{else}} itemList {{end}}
    ///
    /// If keyword is past.
    fn with_control(&mut self) -> PResult<Node> {
        Ok(Node::With(self.branch(NodeType::With, "with")?))
    }

    // Go: parse.go:(*Tree).endControl
    /// End:
    ///
    ///     {{end}}
    ///
    /// End keyword is past.
    fn end_control(&mut self) -> PResult<Node> {
        let pos = self.expect(ItemType::RightDelim, "end")?.pos;
        Ok(Node::End(EndNode { pos, tr: self.tr() }))
    }

    // Go: parse.go:(*Tree).elseControl
    /// Else:
    ///
    ///     {{else}}
    ///
    /// Else keyword is past.
    fn else_control(&mut self) -> PResult<Node> {
        let peek = self.peek_non_space();
        // The "{{else if ... " and "{{else with ..." will be
        // treated as "{{else}}{{if ..." and "{{else}}{{with ...".
        // So return the else node here.
        if peek.typ == ItemType::If || peek.typ == ItemType::With {
            return Ok(Node::Else(ElseNode {
                pos: peek.pos,
                tr: self.tr(),
                line: peek.line,
            }));
        }
        let token = self.expect(ItemType::RightDelim, "else")?;
        Ok(Node::Else(ElseNode {
            pos: token.pos,
            tr: self.tr(),
            line: token.line,
        }))
    }

    // Go: parse.go:(*Tree).blockControl
    /// Block:
    ///
    ///     {{block stringValue pipeline}}
    ///
    /// Block keyword is past.
    /// The name must be something that can evaluate to a string.
    /// The pipeline is mandatory.
    fn block_control(&mut self) -> PResult<Node> {
        const CONTEXT: &str = "block clause";

        let token = self.next_non_space();
        let name = self.parse_template_name(&token, CONTEXT)?;
        let pipe = self.pipeline(CONTEXT, ItemType::RightDelim)?;

        let (root, end) = {
            let mut block = self.child(&name); // name will be updated once we know it.
            let r = block.item_list();
            match r {
                Ok((root, end)) => {
                    block.t.root = Some(root.clone());
                    if end.node_type() == NodeType::End {
                        block.add()?;
                    }
                    (root, end)
                }
                Err(e) => return Err(e),
            }
        };
        let _ = root;
        if end.node_type() != NodeType::End {
            return self.errorf(format!("unexpected {end} in {CONTEXT}"));
        }

        Ok(Node::Template(TemplateNode::new(
            self.tr(),
            token.pos,
            token.line,
            name,
            Some(pipe),
        )))
    }

    // Go: parse.go:(*Tree).templateControl
    /// Template:
    ///
    ///     {{template stringValue pipeline}}
    ///
    /// Template keyword is past. The name must be something that can evaluate
    /// to a string.
    fn template_control(&mut self) -> PResult<Node> {
        const CONTEXT: &str = "template clause";
        let token = self.next_non_space();
        let name = self.parse_template_name(&token, CONTEXT)?;
        let mut pipe = None;
        if self.next_non_space().typ != ItemType::RightDelim {
            self.backup();
            // Do not pop variables; they persist until "end".
            pipe = Some(self.pipeline(CONTEXT, ItemType::RightDelim)?);
        }
        Ok(Node::Template(TemplateNode::new(
            self.tr(),
            token.pos,
            token.line,
            name,
            pipe,
        )))
    }

    // Go: parse.go:(*Tree).parseTemplateName
    fn parse_template_name(&mut self, token: &Item, context: &str) -> PResult<String> {
        match token.typ {
            ItemType::String | ItemType::RawString => match go_strconv::unquote(&token.val) {
                Ok(s) => Ok(String::from_utf8_lossy(&s).into_owned()),
                Err(e) => self.errorf(e),
            },
            _ => self.unexpected(token, context),
        }
    }

    // Go: parse.go:(*Tree).command
    /// command:
    ///
    ///     operand (space operand)*
    ///
    /// space-separated arguments up to a pipeline character or right delimiter.
    /// we consume the pipe character but leave the right delim to terminate the action.
    fn command(&mut self) -> PResult<CommandNode> {
        let pos = self.peek_non_space().pos;
        let mut cmd = CommandNode::new(self.tr(), pos);
        loop {
            self.peek_non_space(); // skip leading spaces.
            if let Some(operand) = self.operand()? {
                cmd.append(operand);
            }
            let token = self.next();
            match token.typ {
                ItemType::Space => continue,
                ItemType::RightDelim | ItemType::RightParen => self.backup(),
                ItemType::Pipe => {
                    // nothing here; break loop below
                }
                _ => return self.unexpected(&token, "operand"),
            }
            break;
        }
        if cmd.args.is_empty() {
            return self.errorf("empty command");
        }
        Ok(cmd)
    }

    // Go: parse.go:(*Tree).operand
    /// operand:
    ///
    ///     term .Field*
    ///
    /// An operand is a space-separated component of a command,
    /// a term possibly followed by field accesses.
    /// A nil return means the next item is not an operand.
    fn operand(&mut self) -> PResult<Option<Node>> {
        let Some(mut node) = self.term()? else {
            return Ok(None);
        };
        if self.peek().typ == ItemType::Field {
            let pos = self.peek().pos;
            let mut chain = ChainNode::new(self.tr(), pos, node);
            while self.peek().typ == ItemType::Field {
                let f = self.next();
                chain.add(f.val_str());
            }
            // Compatibility with original API: If the term is of type NodeField
            // or NodeVariable, just put more fields on the original.
            // Otherwise, keep the Chain node.
            // Obvious parsing errors involving literal values are detected here.
            // More complex error cases will have to be handled at execution time.
            match chain.node.node_type() {
                NodeType::Field => {
                    let s = chain.to_string_lossy();
                    node = Node::Field(FieldNode::new(self.tr(), chain.pos, &s));
                }
                NodeType::Variable => {
                    let s = chain.to_string_lossy();
                    node = Node::Variable(VariableNode::new(self.tr(), chain.pos, &s));
                }
                NodeType::Bool | NodeType::String | NodeType::Number | NodeType::Nil | NodeType::Dot => {
                    let q = go_strconv::quote(chain.node.to_bytes());
                    return self.errorf(format!("unexpected . after term {q}"));
                }
                _ => node = Node::Chain(chain),
            }
        }
        Ok(Some(node))
    }

    // Go: parse.go:(*Tree).term
    /// term:
    ///
    ///     literal (number, string, nil, boolean)
    ///     function (identifier)
    ///     .
    ///     .Field
    ///     $
    ///     '(' pipeline ')'
    ///
    /// A term is a simple "expression".
    /// A nil return means the next item is not a term.
    fn term(&mut self) -> PResult<Option<Node>> {
        let token = self.next_non_space();
        let tr = self.tr();
        match token.typ {
            ItemType::Identifier => {
                let check_func = !self.t.mode.has(Mode::SKIP_FUNC_CHECK);
                if check_func && !self.has_function(token.val_str()) {
                    let q = go_strconv::quote(&token.val);
                    return self.errorf(format!("function {q} not defined"));
                }
                Ok(Some(Node::Identifier(IdentifierNode::new(token.val_str(), tr, token.pos))))
            }
            ItemType::Dot => Ok(Some(Node::Dot(DotNode { pos: token.pos, tr }))),
            ItemType::Nil => Ok(Some(Node::Nil(NilNode { pos: token.pos, tr }))),
            ItemType::Variable => Ok(Some(self.use_var(token.pos, token.val_str())?)),
            ItemType::Field => Ok(Some(Node::Field(FieldNode::new(tr, token.pos, token.val_str())))),
            ItemType::Bool => Ok(Some(Node::Bool(BoolNode {
                pos: token.pos,
                tr,
                true_: token.val == b"true",
            }))),
            ItemType::CharConstant | ItemType::Complex | ItemType::Number => {
                // Number items are ASCII (the lexer only accepts ASCII digits,
                // signs and letters after a digit); char constants may not be.
                let text = String::from_utf8_lossy(&token.val).into_owned();
                let text_bytes_ok = std::str::from_utf8(&token.val).is_ok();
                let r = if text_bytes_ok {
                    NumberNode::new(tr, token.pos, &text, token.typ)
                } else {
                    number_from_bytes(tr, token.pos, &token.val, token.typ)
                };
                match r {
                    Ok(n) => Ok(Some(Node::Number(n))),
                    Err(e) => self.errorf(e),
                }
            }
            ItemType::LeftParen => Ok(Some(Node::Pipe(
                self.pipeline("parenthesized pipeline", ItemType::RightParen)?,
            ))),
            ItemType::String | ItemType::RawString => match go_strconv::unquote(&token.val) {
                Ok(s) => Ok(Some(Node::String(StringNode {
                    pos: token.pos,
                    tr,
                    quoted: token.val,
                    text: s,
                }))),
                Err(e) => self.errorf(e),
            },
            _ => {
                self.backup();
                Ok(None)
            }
        }
    }

    // Go: parse.go:(*Tree).hasFunction
    /// Reports if a function name exists in the Tree's maps.
    fn has_function(&self, name: &str) -> bool {
        self.funcs.iter().any(|f| f.has_function(name))
    }

    // Go: parse.go:(*Tree).useVar
    /// Returns a node for a variable reference. It errors if the
    /// variable is not defined.
    fn use_var(&mut self, pos: Pos, name: &str) -> PResult<Node> {
        let v = VariableNode::new(self.tr(), pos, name);
        if self.vars.iter().any(|var_name| *var_name == v.ident[0]) {
            return Ok(Node::Variable(v));
        }
        let q = go_strconv::quote(&v.ident[0]);
        self.errorf(format!("undefined variable {q}"))
    }
}

/// A character constant containing invalid UTF-8: `strconv.UnquoteChar`
/// decides; the node text keeps Go's bytes as far as a Rust `String` can.
fn number_from_bytes(tr: TreeRef, pos: Pos, val: &[u8], typ: ItemType) -> Result<NumberNode, String> {
    if typ == ItemType::CharConstant {
        let (rune, _, tail) = go_strconv::unquote_char(&val[1..], val[0]).map_err(|e| e.to_string())?;
        if tail != b"'" {
            return Err(format!(
                "malformed character constant: {}",
                String::from_utf8_lossy(val)
            ));
        }
        return Ok(NumberNode {
            pos,
            tr,
            is_int: true,
            is_uint: true,
            is_float: true,
            int64: rune as i64,
            uint64: rune as i64 as u64,
            float64: rune as f64,
            text: String::from_utf8_lossy(val).into_owned(),
            ..Default::default()
        });
    }
    NumberNode::new(tr, pos, &String::from_utf8_lossy(val), typ)
}
