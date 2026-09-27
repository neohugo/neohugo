//! Go: parse/v2/js/parse.go — the recursive-descent parser with ASI,
//! regular-expression detection, arrow-function speculation and scope
//! analysis.

use tdewolff_parse::{GoBytes, GoError, Input, buffer, copy as parse_copy, new_error};

use crate::ast::*;
use crate::lex::Lexer;
use crate::table::*;
use crate::tokentype::*;
use crate::util::{as_decimal_literal, as_identifier_name, is_lhs_expr};

/// Go: parse.go:NestedStmtLimit
pub const NESTED_STMT_LIMIT: i64 = 1000;
/// Go: parse.go:NestedExprLimit
pub const NESTED_EXPR_LIMIT: i64 = 1000;

/// Go: parse.go:Options
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub while_to_for: bool,
    pub inline: bool,
}

/// Go: parse.go:Parser — the state for the parser.
struct Parser {
    ast: Ast,
    l: Lexer,
    o: Options,
    /// Go `err error`: only its `Error()` string is ever used.
    err: Option<Vec<u8>>,

    data: GoBytes,
    tt: TokenType,
    prev_lt: bool,
    in_: bool,
    await_: bool,
    yield_: bool,
    deflt: bool,
    retrn: bool,
    assume_arrow_func: bool,
    allow_directive_prologue: bool,
    comments: Vec<NodeId>,

    stmt_level: i64,
    expr_level: i64,

    scope: ScopeId,
}

// Go: parse.go:Parse
/// Returns a JS AST tree of the input. The error is a `*parse.Error`
/// (`GoError::Parse`) for syntax errors, or the lexer's/reader's error.
///
/// The parser is recursive (as in Go, bounded by the 1000-level statement
/// and expression limits); run it on a thread with a large stack for
/// adversarial input.
pub fn parse(r: &Input, o: Options) -> Result<Ast, GoError> {
    let mut p = Parser {
        ast: Ast::new(),
        l: Lexer::new(r.clone()),
        o,
        err: None,
        data: GoBytes::nil(),
        tt: WhitespaceToken, // trick so that next() works
        prev_lt: false,
        in_: true,
        await_: true,
        yield_: false,
        deflt: false,
        retrn: false,
        assume_arrow_func: false,
        allow_directive_prologue: false,
        comments: Vec::new(),
        stmt_level: 0,
        expr_level: 0,
        scope: ScopeId::NIL,
    };

    if o.inline {
        p.next();
        p.retrn = true;
        p.allow_directive_prologue = true;
        let (scope, _) = p.enter_scope(true);
        let block = p.ast.alloc(Node::BlockStmt(BlockStmt {
            list: Vec::new(),
            scope,
        }));
        p.ast.block_stmt = block;
        loop {
            if p.tt == ErrorToken {
                break;
            }
            let stmt = p.parse_stmt(true);
            p.ast.block_mut(block).list.push(stmt);
        }
    } else {
        // catch shebang in first line
        let mut shebang = GoBytes::nil();
        if r.peek(0) == b'#' && r.peek(1) == b'!' {
            r.move_(2);
            p.l.consume_single_line_comment(); // consume till end-of-line
            shebang = r.shift();
        }

        // parse JS module
        p.next();
        let module = p.parse_module();
        let block = p.ast.alloc(Node::BlockStmt(module));
        p.ast.block_stmt = block;

        if 0 < shebang.len() {
            let c = p.ast.alloc(Node::Comment(Comment { value: shebang }));
            let list = &mut p.ast.block_mut(block).list;
            let mut l2 = Vec::with_capacity(list.len() + 1);
            l2.push(c);
            l2.extend_from_slice(list);
            *list = l2;
        }
    }

    if let Some(err) = p.err {
        let offset = p.l.input().offset() as isize - p.data.len() as isize;
        let mut rd = buffer::Reader::new(p.l.input().bytes());
        return Err(GoError::Parse(Box::new(new_error(
            Some(&mut rd),
            offset,
            err,
        ))));
    } else {
        let lerr = p.l.err();
        if let Some(e) = lerr {
            if !e.is_eof() {
                return Err(e);
            }
        }
    }
    Ok(p.ast)
}

////////////////////////////////////////////////////////////////

fn app(msg: &mut Vec<u8>, s: &[u8]) {
    msg.extend_from_slice(s);
}

impl Parser {
    // Go: parse.go:Parser.next
    fn next(&mut self) {
        self.prev_lt = false;
        (self.tt, self.data) = self.l.next();
        loop {
            match self.tt {
                WhitespaceToken => {
                    // no-op
                }
                LineTerminatorToken => {
                    self.prev_lt = true;
                }
                CommentToken | CommentLineTerminatorToken => {
                    if 2 < self.data.len() && self.data.at(2) == b'!' {
                        let c = self.ast.alloc(Node::Comment(Comment {
                            value: self.data.clone(),
                        }));
                        self.comments.push(c);
                    }
                    if self.tt == CommentLineTerminatorToken {
                        self.prev_lt = true;
                    }
                }
                _ => break,
            }
            (self.tt, self.data) = self.l.next();
        }
    }

    // Go: parse.go:Parser.failMessage (the message is already formatted)
    fn fail_message(&mut self, msg: Vec<u8>) {
        if self.err.is_none() {
            self.err = Some(msg);
            self.tt = ErrorToken;
        }
    }

    // Go: parse.go:Parser.fail
    fn fail(&mut self, in_: &str, expected: &[TokenType]) {
        if self.err.is_none() {
            let mut msg = b"unexpected".to_vec();
            if 0 < expected.len() {
                msg = b"expected".to_vec();
                for (i, tt) in expected[..expected.len() - 1].iter().enumerate() {
                    if 0 < i {
                        app(&mut msg, b",");
                    }
                    app(&mut msg, b" ");
                    app(&mut msg, tt.string().as_bytes());
                }
                if 2 < expected.len() {
                    app(&mut msg, b", or");
                } else if 1 < expected.len() {
                    app(&mut msg, b" or");
                }
                app(&mut msg, b" ");
                app(&mut msg, expected[expected.len() - 1].string().as_bytes());
                app(&mut msg, b" instead of");
            }

            if self.tt == ErrorToken {
                let lerr = self.l.err();
                if tdewolff_parse::is_eof(&lerr) {
                    app(&mut msg, b" EOF");
                } else if let Some(GoError::Parse(lexer_err)) = lerr {
                    msg = lexer_err.message.clone();
                } else {
                    // does not happen
                }
            } else {
                app(&mut msg, b" ");
                self.data.write_to(&mut msg);
            }
            if !in_.is_empty() {
                app(&mut msg, b" in ");
                app(&mut msg, in_.as_bytes());
            }

            self.err = Some(msg);
            self.tt = ErrorToken;
        }
    }

    // Go: parse.go:Parser.consume
    fn consume(&mut self, in_: &str, tt: TokenType) -> bool {
        if self.tt != tt {
            self.fail(in_, &[tt]);
            return false;
        }
        self.next();
        true
    }

    // Go: parse.go:Parser.enterScope
    /// Creates a new scope (Go: resets the scope embedded in a new node) and
    /// makes it current; returns `(new scope, previous scope)`.
    fn enter_scope(&mut self, is_func: bool) -> (ScopeId, ScopeId) {
        // create a new scope object and add it to the parent
        let parent = self.scope;
        let scope = self.ast.alloc_scope(Scope {
            parent,
            ..Scope::default()
        });
        self.scope = scope;
        if is_func {
            self.ast.scope_mut(scope).func = scope;
        } else if !parent.is_nil() {
            let f = self.ast.scope(parent).func;
            self.ast.scope_mut(scope).func = f;
        }
        (scope, parent)
    }

    // Go: parse.go:Parser.exitScope
    fn exit_scope(&mut self, parent: ScopeId) {
        self.ast.hoist_undeclared(self.scope);
        self.scope = parent;
    }

    /// `p.scope.Func`
    fn func_scope(&self) -> ScopeId {
        self.ast.scope(self.scope).func
    }

    // Go: parse.go:Parser.parseModule
    fn parse_module(&mut self) -> BlockStmt {
        let mut module = BlockStmt::default();
        let (scope, _) = self.enter_scope(true);
        module.scope = scope;
        self.allow_directive_prologue = true;
        loop {
            match self.tt {
                ErrorToken => {
                    if 0 < self.comments.len() {
                        let mut l = std::mem::take(&mut self.comments);
                        l.extend_from_slice(&module.list);
                        module.list = l;
                        self.comments.clear();
                    }
                    return module;
                }
                ImportToken => {
                    self.next();
                    if self.tt == OpenParenToken {
                        // could be an import call expression
                        let left = self.ast.alloc(Node::LiteralExpr(LiteralExpr {
                            token_type: ImportToken,
                            data: go_string_bytes(b"import"),
                        }));
                        self.expr_level += 1;
                        let expr = self.parse_expression_suffix(left, OpExpr, OpCall);
                        self.expr_level -= 1;
                        let s = self.ast.alloc(Node::ExprStmt(ExprStmt { value: expr }));
                        module.list.push(s);
                        if !self.prev_lt && self.tt == SemicolonToken {
                            self.next();
                        }
                    } else if self.tt == DotToken {
                        self.next();
                        if !self.consume("import.meta expression", MetaToken) {
                            return module;
                        }
                        let left = self.ast.alloc(Node::ImportMetaExpr);
                        self.expr_level += 1;
                        let expr = self.parse_expression_suffix(left, OpExpr, OpMember);
                        self.expr_level -= 1;
                        let s = self.ast.alloc(Node::ExprStmt(ExprStmt { value: expr }));
                        module.list.push(s);
                    } else {
                        let import_stmt = self.parse_import_stmt();
                        let s = self.ast.alloc(Node::ImportStmt(import_stmt));
                        module.list.push(s);
                    }
                }
                ExportToken => {
                    let export_stmt = self.parse_export_stmt();
                    let s = self.ast.alloc(Node::ExportStmt(export_stmt));
                    module.list.push(s);
                }
                _ => {
                    let s = self.parse_stmt(true);
                    module.list.push(s);
                }
            }
        }
    }

    /// `!p.prevLT && p.tt != SemicolonToken && p.tt != CloseBraceToken && p.tt != ErrorToken`
    fn no_asi(&self) -> bool {
        !self.prev_lt
            && self.tt != SemicolonToken
            && self.tt != CloseBraceToken
            && self.tt != ErrorToken
    }

    // Go: parse.go:Parser.parseStmt
    fn parse_stmt(&mut self, allow_declaration: bool) -> NodeId {
        let mut stmt = NodeId::NIL;
        self.stmt_level += 1;
        if NESTED_STMT_LIMIT < self.stmt_level {
            self.fail_message(b"too many nested statements".to_vec());
            return NodeId::NIL;
        }

        let allow_directive_prologue = self.allow_directive_prologue;
        self.allow_directive_prologue = false;

        let tt = self.tt;
        match tt {
            OpenBraceToken => {
                stmt = self.parse_block_stmt("block statement");
            }
            ConstToken | VarToken => {
                if !allow_declaration && tt == ConstToken {
                    self.fail("statement", &[]);
                    return stmt;
                }
                self.next();
                let var_decl = self.parse_var_decl(tt, true);
                stmt = var_decl;
                if self.no_asi() {
                    if tt == ConstToken {
                        self.fail("const declaration", &[]);
                    } else {
                        self.fail("var statement", &[]);
                    }
                    return stmt;
                }
            }
            LetToken => {
                let let_ = self.data.clone();
                self.next();
                if allow_declaration
                    && (is_identifier(self.tt)
                        || self.tt == YieldToken
                        || self.tt == AwaitToken
                        || self.tt == OpenBracketToken
                        || self.tt == OpenBraceToken)
                {
                    stmt = self.parse_var_decl(tt, false);
                    if self.no_asi() {
                        self.fail("let declaration", &[]);
                        return stmt;
                    }
                } else if self.tt == OpenBracketToken {
                    self.fail_message(b"unexpected let [ in single-statement context".to_vec());
                    return stmt;
                } else {
                    // expression
                    let e = self.parse_identifier_expression(OpExpr, let_);
                    stmt = self.ast.alloc(Node::ExprStmt(ExprStmt { value: e }));
                    if self.no_asi() {
                        self.fail("expression", &[]);
                        return stmt;
                    }
                }
            }
            IfToken => {
                self.next();
                if !self.consume("if statement", OpenParenToken) {
                    return stmt;
                }
                let cond = self.parse_expression(OpExpr);
                if !self.consume("if statement", CloseParenToken) {
                    return stmt;
                }
                let body = self.parse_stmt(false);

                let mut else_body = NodeId::NIL;
                if self.tt == ElseToken {
                    self.next();
                    else_body = self.parse_stmt(false);
                }
                stmt = self.ast.alloc(Node::IfStmt(IfStmt {
                    cond,
                    body,
                    else_: else_body,
                }));
            }
            ContinueToken | BreakToken => {
                let tt = self.tt;
                self.next();
                let mut label = GoBytes::nil();
                if !self.prev_lt && self.is_identifier_reference(self.tt) {
                    label = self.data.clone();
                    self.next();
                }
                stmt = self
                    .ast
                    .alloc(Node::BranchStmt(BranchStmt { type_: tt, label }));
            }
            WithToken => {
                self.next();
                if !self.consume("with statement", OpenParenToken) {
                    return stmt;
                }
                let cond = self.parse_expression(OpExpr);
                if !self.consume("with statement", CloseParenToken) {
                    return stmt;
                }

                let f = self.func_scope();
                self.ast.scope_mut(f).has_with = true;
                let body = self.parse_stmt(false);
                stmt = self.ast.alloc(Node::WithStmt(WithStmt { cond, body }));
            }
            DoToken => {
                stmt = self.ast.alloc(Node::DoWhileStmt(DoWhileStmt {
                    cond: NodeId::NIL,
                    body: NodeId::NIL,
                }));
                self.next();
                let body = self.parse_stmt(false);
                if !self.consume("do-while statement", WhileToken) {
                    return stmt;
                }
                if !self.consume("do-while statement", OpenParenToken) {
                    return stmt;
                }
                let cond = self.parse_expression(OpExpr);
                stmt = self
                    .ast
                    .alloc(Node::DoWhileStmt(DoWhileStmt { cond, body }));
                if !self.consume("do-while statement", CloseParenToken) {
                    return stmt;
                }
            }
            WhileToken => {
                self.next();
                if !self.consume("while statement", OpenParenToken) {
                    return stmt;
                }
                let cond = self.parse_expression(OpExpr);
                if !self.consume("while statement", CloseParenToken) {
                    return stmt;
                }
                let body = self.parse_stmt(false);
                if self.o.while_to_for {
                    let var_decl = self.ast.alloc(Node::VarDecl(VarDecl {
                        token_type: VarToken,
                        list: Vec::new(),
                        scope: self.scope,
                        in_for: true,
                        in_for_in_of: false,
                    }));
                    let f = self.func_scope();
                    self.ast.scope_mut(f).var_decls.push(var_decl);

                    let block = if matches!(self.ast.node(body), Node::BlockStmt(_)) {
                        body
                    } else {
                        // Go: &BlockStmt{List: []IStmt{body}} with a zero Scope
                        self.ast.alloc_block(vec![body])
                    };
                    stmt = self.ast.alloc(Node::ForStmt(ForStmt {
                        init: var_decl,
                        cond,
                        post: NodeId::NIL,
                        body: block,
                    }));
                } else {
                    stmt = self.ast.alloc(Node::WhileStmt(WhileStmt { cond, body }));
                }
            }
            ForToken => {
                self.next();
                let await_ = self.await_ && self.tt == AwaitToken;
                if await_ {
                    self.next();
                }
                if !self.consume("for statement", OpenParenToken) {
                    return stmt;
                }

                let (body_scope, parent) = self.enter_scope(false);
                let body = self.ast.alloc(Node::BlockStmt(BlockStmt {
                    list: Vec::new(),
                    scope: body_scope,
                }));

                let mut init = NodeId::NIL;
                self.in_ = false;
                if self.tt == VarToken || self.tt == LetToken || self.tt == ConstToken {
                    let tt = self.tt;
                    self.next();
                    let var_decl = self.parse_var_decl(tt, true);
                    if self.err.is_some() {
                        return stmt;
                    }
                    let (n, first) = {
                        let vd = self.ast.var_decl(var_decl);
                        (vd.list.len(), vd.list[0])
                    };
                    if self.tt != SemicolonToken && (1 < n || first.default.is_some()) {
                        self.fail("for statement", &[]);
                        return stmt;
                    } else if self.tt == SemicolonToken && first.default.is_nil() {
                        // all but the first item were already verified
                        if !self.ast.is_var(first.binding) {
                            self.fail("for statement", &[]);
                            return stmt;
                        }
                    }
                    init = var_decl;
                } else if await_ {
                    init = self.parse_expression(OpLHS);
                } else if self.tt != SemicolonToken {
                    init = self.parse_expression(OpExpr);
                }
                self.in_ = true;

                let is_lhs = is_lhs_expr(&self.ast, init);
                if is_lhs && self.tt == InToken {
                    if await_ {
                        self.fail("for statement", &[OfToken]);
                        return stmt;
                    }
                    self.next();
                    let value = self.parse_expression(OpExpr);
                    if !self.consume("for statement", CloseParenToken) {
                        return stmt;
                    }
                    self.ast.mark_for_stmt(self.scope);
                    self.parse_for_body(body);
                    if let Node::VarDecl(vd) = self.ast.node_mut_or_nil(init) {
                        vd.in_for_in_of = true;
                    }
                    stmt = self
                        .ast
                        .alloc(Node::ForInStmt(ForInStmt { init, value, body }));
                } else if is_lhs && self.tt == OfToken {
                    self.next();
                    let value = self.parse_expression(OpAssign);
                    if !self.consume("for statement", CloseParenToken) {
                        return stmt;
                    }
                    self.ast.mark_for_stmt(self.scope);
                    self.parse_for_body(body);
                    if let Node::VarDecl(vd) = self.ast.node_mut_or_nil(init) {
                        vd.in_for_in_of = true;
                    }
                    stmt = self.ast.alloc(Node::ForOfStmt(ForOfStmt {
                        await_,
                        init,
                        value,
                        body,
                    }));
                } else if self.tt == SemicolonToken {
                    let mut cond = NodeId::NIL;
                    let mut post = NodeId::NIL;
                    if await_ {
                        self.fail("for statement", &[OfToken]);
                        return stmt;
                    }
                    self.next();
                    if self.tt != SemicolonToken {
                        cond = self.parse_expression(OpExpr);
                    }
                    if !self.consume("for statement", SemicolonToken) {
                        return stmt;
                    }
                    if self.tt != CloseParenToken {
                        post = self.parse_expression(OpExpr);
                    }
                    if !self.consume("for statement", CloseParenToken) {
                        return stmt;
                    }
                    self.ast.mark_for_stmt(self.scope);
                    self.parse_for_body(body);
                    if init.is_nil() {
                        let var_decl = self.ast.alloc(Node::VarDecl(VarDecl {
                            token_type: VarToken,
                            list: Vec::new(),
                            scope: self.scope,
                            in_for: true,
                            in_for_in_of: false,
                        }));
                        let f = self.func_scope();
                        self.ast.scope_mut(f).var_decls.push(var_decl);
                        init = var_decl;
                    } else if let Node::VarDecl(vd) = self.ast.node_mut_or_nil(init) {
                        vd.in_for = true;
                    }
                    stmt = self.ast.alloc(Node::ForStmt(ForStmt {
                        init,
                        cond,
                        post,
                        body,
                    }));
                } else if is_lhs {
                    self.fail("for statement", &[InToken, OfToken, SemicolonToken]);
                    return stmt;
                } else {
                    self.fail("for statement", &[SemicolonToken]);
                    return stmt;
                }
                self.exit_scope(parent);
            }
            SwitchToken => {
                self.next();
                if !self.consume("switch statement", OpenParenToken) {
                    return stmt;
                }
                let init = self.parse_expression(OpExpr);
                if !self.consume("switch statement", CloseParenToken) {
                    return stmt;
                }

                // case block
                if !self.consume("switch statement", OpenBraceToken) {
                    return stmt;
                }

                let (sw_scope, parent) = self.enter_scope(false);
                let switch_stmt = self.ast.alloc(Node::SwitchStmt(SwitchStmt {
                    init,
                    list: Vec::new(),
                    scope: sw_scope,
                }));
                loop {
                    if self.tt == ErrorToken {
                        self.fail("switch statement", &[]);
                        return stmt;
                    } else if self.tt == CloseBraceToken {
                        self.next();
                        break;
                    }

                    let clause = self.tt;
                    let mut list = NodeId::NIL;
                    if self.tt == CaseToken {
                        self.next();
                        list = self.parse_expression(OpExpr);
                    } else if self.tt == DefaultToken {
                        self.next();
                    } else {
                        self.fail("switch statement", &[CaseToken, DefaultToken]);
                        return stmt;
                    }
                    if !self.consume("switch statement", ColonToken) {
                        return stmt;
                    }

                    let mut stmts = Vec::new();
                    while self.tt != CaseToken
                        && self.tt != DefaultToken
                        && self.tt != CloseBraceToken
                        && self.tt != ErrorToken
                    {
                        let s = self.parse_stmt(true);
                        stmts.push(s);
                    }
                    self.ast.switch_stmt_mut(switch_stmt).list.push(CaseClause {
                        token_type: clause,
                        cond: list,
                        list: stmts,
                    });
                }
                self.exit_scope(parent);
                stmt = switch_stmt;
            }
            FunctionToken => {
                if !allow_declaration {
                    self.fail("statement", &[]);
                    return stmt;
                }
                stmt = self.parse_func_decl();
            }
            AsyncToken => {
                // async function
                let async_ = self.data.clone();
                self.next();
                if self.tt == FunctionToken && !self.prev_lt {
                    if !allow_declaration {
                        self.fail("statement", &[]);
                        return stmt;
                    }
                    stmt = self.parse_async_func_decl();
                } else {
                    // expression
                    let e = self.parse_async_expression(OpExpr, async_);
                    stmt = self.ast.alloc(Node::ExprStmt(ExprStmt { value: e }));
                    if self.no_asi() {
                        self.fail("expression", &[]);
                        return stmt;
                    }
                }
            }
            ClassToken => {
                if !allow_declaration {
                    self.fail("statement", &[]);
                    return stmt;
                }
                stmt = self.parse_class_decl();
            }
            ThrowToken => {
                self.next();
                if self.prev_lt {
                    self.fail_message(b"unexpected newline in throw statement".to_vec());
                    return stmt;
                }
                let value = self.parse_expression(OpExpr);
                stmt = self.ast.alloc(Node::ThrowStmt(ThrowStmt { value }));
            }
            TryToken => {
                self.next();
                let body = self.parse_block_stmt("try statement");
                let mut binding = NodeId::NIL;
                let mut catch = NodeId::NIL;
                let mut finally = NodeId::NIL;
                if self.tt == CatchToken {
                    self.next();
                    let (catch_scope, parent) = self.enter_scope(false);
                    catch = self.ast.alloc(Node::BlockStmt(BlockStmt {
                        list: Vec::new(),
                        scope: catch_scope,
                    }));
                    if self.tt == OpenParenToken {
                        self.next();
                        binding = self.parse_binding(CatchDecl); // local to block scope of catch
                        if !self.consume("try-catch statement", CloseParenToken) {
                            return stmt;
                        }
                    }
                    let list = self.parse_stmt_list("try-catch statement");
                    self.ast.block_mut(catch).list = list;
                    self.exit_scope(parent);
                } else if self.tt != FinallyToken {
                    self.fail("try statement", &[CatchToken, FinallyToken]);
                    return stmt;
                }
                if self.tt == FinallyToken {
                    self.next();
                    finally = self.parse_block_stmt("try-finally statement");
                }
                stmt = self.ast.alloc(Node::TryStmt(TryStmt {
                    body,
                    binding,
                    catch,
                    finally,
                }));
            }
            DebuggerToken => {
                stmt = self.ast.alloc(Node::DebuggerStmt);
                self.next();
            }
            SemicolonToken => {
                stmt = self.ast.alloc(Node::EmptyStmt);
                self.next();
            }
            ErrorToken => {
                stmt = self.ast.alloc(Node::EmptyStmt);
                return stmt;
            }
            _ => {
                if self.retrn && self.tt == ReturnToken {
                    self.next();
                    let mut value = NodeId::NIL;
                    if self.no_asi() {
                        value = self.parse_expression(OpExpr);
                    }
                    stmt = self.ast.alloc(Node::ReturnStmt(ReturnStmt { value }));
                } else if self.is_identifier_reference(self.tt) {
                    // LabelledStatement, Expression
                    let label = self.data.clone();
                    self.next();
                    if self.tt == ColonToken {
                        self.next();
                        let prev_deflt = self.deflt;
                        if self.tt == FunctionToken {
                            self.deflt = false;
                        }
                        let value = self.parse_stmt(true); // allows illegal async function, generator function, let, const, or class declarations
                        stmt = self
                            .ast
                            .alloc(Node::LabelledStmt(LabelledStmt { label, value }));
                        self.deflt = prev_deflt;
                    } else {
                        // expression
                        let e = self.parse_identifier_expression(OpExpr, label);
                        stmt = self.ast.alloc(Node::ExprStmt(ExprStmt { value: e }));
                        if self.no_asi() {
                            self.fail("expression", &[]);
                            return stmt;
                        }
                    }
                } else {
                    // expression
                    let e = self.parse_expression(OpExpr);
                    stmt = self.ast.alloc(Node::ExprStmt(ExprStmt { value: e }));
                    if self.no_asi() {
                        self.fail("expression", &[]);
                        return stmt;
                    } else if let Node::LiteralExpr(lit) = self.ast.node(e) {
                        if allow_directive_prologue
                            && lit.token_type == StringToken
                            && lit.data.len() == 12
                            && lit.data.slice(1, 11).equal(b"use strict")
                        {
                            let value = lit.data.clone();
                            stmt = self.ast.alloc(Node::DirectivePrologueStmt(
                                DirectivePrologueStmt { value },
                            ));
                            self.allow_directive_prologue = true;
                        }
                    }
                }
            }
        }
        if !self.prev_lt && self.tt == SemicolonToken {
            self.next();
        }
        self.stmt_level -= 1;
        stmt
    }

    /// The shared tail of the three `for` forms: parses the body statement(s)
    /// into the for block.
    fn parse_for_body(&mut self, body: NodeId) {
        if self.tt == OpenBraceToken {
            let list = self.parse_stmt_list("");
            self.ast.block_mut(body).list = list;
        } else if self.tt != SemicolonToken {
            let s = self.parse_stmt(false);
            self.ast.block_mut(body).list = vec![s];
        } else {
            self.next();
        }
    }

    // Go: parse.go:Parser.parseStmtList
    fn parse_stmt_list(&mut self, in_: &str) -> Vec<NodeId> {
        let mut list = Vec::new();
        let comments = self.comments.len();
        if !self.consume(in_, OpenBraceToken) {
            return list;
        }
        loop {
            if self.tt == ErrorToken {
                self.fail("", &[]);
                return list;
            } else if self.tt == CloseBraceToken {
                self.next();
                break;
            }
            let s = self.parse_stmt(true);
            list.push(s);
        }
        if comments < self.comments.len() {
            let mut list2 = Vec::with_capacity(self.comments.len() - comments + list.len());
            list2.extend_from_slice(&self.comments[comments..]);
            list2.extend_from_slice(&list);
            list = list2;
            self.comments.truncate(comments);
        }
        list
    }

    // Go: parse.go:Parser.parseBlockStmt
    fn parse_block_stmt(&mut self, in_: &str) -> NodeId {
        let (scope, parent) = self.enter_scope(false);
        let block_stmt = self.ast.alloc(Node::BlockStmt(BlockStmt {
            list: Vec::new(),
            scope,
        }));
        let list = self.parse_stmt_list(in_);
        self.ast.block_mut(block_stmt).list = list;
        self.exit_scope(parent);
        block_stmt
    }

    // Go: parse.go:Parser.parseImportStmt
    fn parse_import_stmt(&mut self) -> ImportStmt {
        let mut import_stmt = ImportStmt::default();
        // assume we're passed import
        if self.tt == StringToken {
            import_stmt.module = self.data.clone();
            self.next();
        } else {
            let mut expect_clause = true;
            if is_identifier(self.tt) || self.tt == YieldToken {
                import_stmt.default = self.data.clone();
                self.next();
                expect_clause = self.tt == CommaToken;
                if expect_clause {
                    self.next();
                }
            }
            if expect_clause && self.tt == MulToken {
                let star = self.data.clone();
                self.next();
                if !self.consume("import statement", AsToken) {
                    return import_stmt;
                }
                if !is_identifier(self.tt) && self.tt != YieldToken {
                    self.fail("import statement", &[IdentifierToken]);
                    return import_stmt;
                }
                import_stmt.list = Some(vec![Alias {
                    name: star,
                    binding: self.data.clone(),
                }]);
                self.next();
            } else if expect_clause && self.tt == OpenBraceToken {
                self.next();
                import_stmt.list = Some(Vec::new());
                while is_identifier_name(self.tt) || self.tt == StringToken {
                    let tt = self.tt;
                    let mut name = GoBytes::nil();
                    let mut binding = self.data.clone();
                    self.next();
                    if self.tt == AsToken {
                        self.next();
                        if !is_identifier(self.tt) && self.tt != YieldToken {
                            self.fail("import statement", &[IdentifierToken]);
                            return import_stmt;
                        }
                        name = binding;
                        binding = self.data.clone();
                        self.next();
                    } else if !is_identifier(tt) && tt != YieldToken || tt == StringToken {
                        self.fail("import statement", &[IdentifierToken, StringToken]);
                        return import_stmt;
                    }
                    import_stmt
                        .list
                        .as_mut()
                        .unwrap()
                        .push(Alias { name, binding });
                    if self.tt == CommaToken {
                        self.next();
                        if self.tt == CloseBraceToken {
                            import_stmt.list.as_mut().unwrap().push(Alias::default());
                            break;
                        }
                    }
                }
                if !self.consume("import statement", CloseBraceToken) {
                    return import_stmt;
                }
            } else if expect_clause && !import_stmt.default.is_nil() {
                self.fail("import statement", &[MulToken, OpenBraceToken]);
                return import_stmt;
            } else if import_stmt.default.is_nil() {
                self.fail(
                    "import statement",
                    &[StringToken, IdentifierToken, MulToken, OpenBraceToken],
                );
                return import_stmt;
            }

            if !self.consume("import statement", FromToken) {
                return import_stmt;
            }
            if self.tt != StringToken {
                self.fail("import statement", &[StringToken]);
                return import_stmt;
            }
            import_stmt.module = self.data.clone();
            self.next();
        }
        if self.tt == SemicolonToken {
            self.next();
        }
        import_stmt
    }

    // Go: parse.go:Parser.parseExportStmt
    fn parse_export_stmt(&mut self) -> ExportStmt {
        let mut export_stmt = ExportStmt::default();
        // assume we're at export
        self.next();
        let (prev_yield, prev_await, prev_deflt) = (self.yield_, self.await_, self.deflt);
        (self.yield_, self.await_, self.deflt) = (false, true, true);
        if self.tt == MulToken || self.tt == OpenBraceToken {
            if self.tt == MulToken {
                let star = self.data.clone();
                self.next();
                if self.tt == AsToken {
                    self.next();
                    if !is_identifier_name(self.tt) && self.tt != StringToken {
                        self.fail("export statement", &[IdentifierToken, StringToken]);
                        return export_stmt;
                    }
                    export_stmt.list = vec![Alias {
                        name: star,
                        binding: self.data.clone(),
                    }];
                    self.next();
                } else {
                    export_stmt.list = vec![Alias {
                        name: GoBytes::nil(),
                        binding: star,
                    }];
                }
                if self.tt != FromToken {
                    self.fail("export statement", &[FromToken]);
                    return export_stmt;
                }
            } else {
                self.next();
                while is_identifier_name(self.tt) || self.tt == StringToken {
                    let mut name = GoBytes::nil();
                    let mut binding = self.data.clone();
                    self.next();
                    if self.tt == AsToken {
                        self.next();
                        if !is_identifier_name(self.tt) && self.tt != StringToken {
                            self.fail("export statement", &[IdentifierToken, StringToken]);
                            return export_stmt;
                        }
                        name = binding;
                        binding = self.data.clone();
                        self.next();
                    }
                    export_stmt.list.push(Alias { name, binding });
                    if self.tt == CommaToken {
                        self.next();
                        if self.tt == CloseBraceToken {
                            export_stmt.list.push(Alias::default());
                            break;
                        }
                    }
                }
                if !self.consume("export statement", CloseBraceToken) {
                    return export_stmt;
                }
            }
            if self.tt == FromToken {
                self.next();
                if self.tt != StringToken {
                    self.fail("export statement", &[StringToken]);
                    return export_stmt;
                }
                export_stmt.module = self.data.clone();
                self.next();
            }
        } else if self.tt == VarToken || self.tt == ConstToken || self.tt == LetToken {
            let tt = self.tt;
            self.next();
            export_stmt.decl = self.parse_var_decl(tt, false);
        } else if self.tt == FunctionToken {
            export_stmt.decl = self.parse_func_decl();
        } else if self.tt == AsyncToken {
            // async function
            self.next();
            if self.tt != FunctionToken || self.prev_lt {
                self.fail("export statement", &[FunctionToken]);
                return export_stmt;
            }
            export_stmt.decl = self.parse_async_func_decl();
        } else if self.tt == ClassToken {
            export_stmt.decl = self.parse_class_decl();
        } else if self.tt == DefaultToken {
            export_stmt.default = true;
            self.next();
            if self.tt == FunctionToken {
                // hoistable declaration
                export_stmt.decl = self.parse_func_decl();
            } else if self.tt == AsyncToken {
                // async function or async arrow function
                let async_ = self.data.clone();
                self.next();
                if self.tt == FunctionToken && !self.prev_lt {
                    // hoistable declaration
                    export_stmt.decl = self.parse_async_func_decl();
                } else {
                    // expression
                    export_stmt.decl = self.parse_async_expression(OpAssign, async_);
                }
            } else if self.tt == ClassToken {
                export_stmt.decl = self.parse_class_decl();
            } else {
                export_stmt.decl = self.parse_expression(OpAssign);
            }
        } else {
            self.fail(
                "export statement",
                &[
                    MulToken,
                    OpenBraceToken,
                    VarToken,
                    LetToken,
                    ConstToken,
                    FunctionToken,
                    AsyncToken,
                    ClassToken,
                    DefaultToken,
                ],
            );
            return export_stmt;
        }
        if self.tt == SemicolonToken {
            self.next();
        }
        (self.yield_, self.await_, self.deflt) = (prev_yield, prev_await, prev_deflt);
        export_stmt
    }

    // Go: parse.go:Parser.parseVarDecl
    fn parse_var_decl(&mut self, tt: TokenType, can_be_hoisted: bool) -> NodeId {
        // assume we're past var, let or const
        let var_decl = self.ast.alloc(Node::VarDecl(VarDecl {
            token_type: tt,
            list: Vec::new(),
            scope: self.scope,
            in_for: false,
            in_for_in_of: false,
        }));
        let mut decl_type = LexicalDecl;
        if tt == VarToken {
            decl_type = VariableDecl;
            if can_be_hoisted {
                let f = self.func_scope();
                self.ast.scope_mut(f).var_decls.push(var_decl);
            }
        }
        loop {
            // binding element, var declaration in for-in or for-of can never have a default
            let mut binding_element = BindingElement::default();
            binding_element.binding = self.parse_binding(decl_type);
            if self.tt == EqToken {
                self.next();
                binding_element.default = self.parse_expression(OpAssign);
            } else if !self.ast.is_var(binding_element.binding)
                && (self.in_ || 0 < self.ast.var_decl(var_decl).list.len())
            {
                self.fail("var statement", &[EqToken]);
                return var_decl;
            } else if tt == ConstToken
                && (self.in_ || !self.in_ && self.tt != OfToken && self.tt != InToken)
            {
                self.fail("const statement", &[EqToken]);
            }

            self.ast.var_decl_mut(var_decl).list.push(binding_element);
            if self.tt == CommaToken {
                self.next();
            } else {
                break;
            }
        }
        var_decl
    }

    // Go: parse.go:Parser.parseFuncParams
    fn parse_func_params(&mut self, in_: &str) -> Params {
        let mut params = Params::default();
        // FormalParameters
        if !self.consume(in_, OpenParenToken) {
            return params;
        }

        while self.tt != CloseParenToken && self.tt != ErrorToken {
            if self.tt == EllipsisToken {
                // binding rest element
                self.next();
                params.rest = self.parse_binding(ArgumentDecl);
                self.consume(in_, CloseParenToken);
                return params;
            }
            let be = self.parse_binding_element(ArgumentDecl);
            params.list.push(be);
            if self.tt != CommaToken {
                break;
            }
            self.next();
        }
        if self.tt != CloseParenToken {
            self.fail(in_, &[]);
            return params;
        }
        self.next();

        // mark undeclared vars as arguments in `function f(a=b){var b}` where the b's are different vars
        self.ast.mark_func_args(self.scope);
        params
    }

    // Go: parse.go:Parser.parseFuncDecl
    fn parse_func_decl(&mut self) -> NodeId {
        self.parse_func(false, false)
    }

    // Go: parse.go:Parser.parseAsyncFuncDecl
    fn parse_async_func_decl(&mut self) -> NodeId {
        self.parse_func(true, false)
    }

    // Go: parse.go:Parser.parseFuncExpr
    fn parse_func_expr(&mut self) -> NodeId {
        self.parse_func(false, true)
    }

    // Go: parse.go:Parser.parseAsyncFuncExpr
    fn parse_async_func_expr(&mut self) -> NodeId {
        self.parse_func(true, true)
    }

    // Go: parse.go:Parser.parseFunc
    fn parse_func(&mut self, async_: bool, expr: bool) -> NodeId {
        // assume we're at function
        self.next();
        let func_decl = self.ast.alloc(Node::FuncDecl(FuncDecl::default()));
        let generator = self.tt == MulToken;
        {
            let fd = self.ast.func_decl_mut(func_decl);
            fd.async_ = async_;
            fd.generator = generator;
        }
        if generator {
            self.next();
        }
        let mut name = GoBytes::nil();
        if expr && (is_identifier(self.tt) || self.tt == YieldToken || self.tt == AwaitToken)
            || !expr && self.is_identifier_reference(self.tt)
        {
            name = self.data.clone();
            if !expr {
                let (v, ok) = self
                    .ast
                    .declare(self.scope, FunctionDecl, self.data.clone());
                self.ast.func_decl_mut(func_decl).name = v;
                if !ok {
                    let msg = already_declared(&self.data);
                    self.fail_message(msg);
                    return func_decl;
                }
            }
            self.next();
        } else if !expr && !self.deflt {
            self.fail("function declaration", &[IdentifierToken]);
            return func_decl;
        } else if self.tt != OpenParenToken {
            self.fail("function declaration", &[IdentifierToken, OpenParenToken]);
            return func_decl;
        }
        let (body_scope, parent) = self.enter_scope(true);
        let body = self.ast.alloc(Node::BlockStmt(BlockStmt {
            list: Vec::new(),
            scope: body_scope,
        }));
        self.ast.func_decl_mut(func_decl).body = body;
        let (prev_await, prev_yield, prev_retrn) = (self.await_, self.yield_, self.retrn);
        (self.await_, self.yield_, self.retrn) = (async_, generator, true);

        if expr && !name.is_nil() {
            let (v, _) = self.ast.declare(self.scope, ExprDecl, name); // cannot fail
            self.ast.func_decl_mut(func_decl).name = v;
        }
        let params = self.parse_func_params("function declaration");
        self.ast.func_decl_mut(func_decl).params = params;

        let (prev_allow_directive_prologue, prev_expr_level) =
            (self.allow_directive_prologue, self.expr_level);
        (self.allow_directive_prologue, self.expr_level) = (true, 0);
        let list = self.parse_stmt_list("function declaration");
        self.ast.block_mut(body).list = list;
        (self.allow_directive_prologue, self.expr_level) =
            (prev_allow_directive_prologue, prev_expr_level);

        (self.await_, self.yield_, self.retrn) = (prev_await, prev_yield, prev_retrn);
        self.exit_scope(parent);
        func_decl
    }

    // Go: parse.go:Parser.parseClassDecl
    fn parse_class_decl(&mut self) -> NodeId {
        self.parse_any_class(false)
    }

    // Go: parse.go:Parser.parseClassExpr
    fn parse_class_expr(&mut self) -> NodeId {
        self.parse_any_class(true)
    }

    // Go: parse.go:Parser.parseAnyClass
    fn parse_any_class(&mut self, expr: bool) -> NodeId {
        // assume we're at class
        self.next();
        let class_decl = self.ast.alloc(Node::ClassDecl(ClassDecl::default()));
        if is_identifier(self.tt) || self.tt == YieldToken || self.tt == AwaitToken {
            if !expr {
                let (v, ok) = self.ast.declare(self.scope, LexicalDecl, self.data.clone());
                self.ast.class_decl_mut(class_decl).name = v;
                if !ok {
                    let msg = already_declared(&self.data);
                    self.fail_message(msg);
                    return class_decl;
                }
            } else {
                //classDecl.Name, ok = p.scope.Declare(ExprDecl, p.data) // classes do not register vars
                let v = self
                    .ast
                    .alloc_var(self.data.clone(), NodeId::NIL, 1, ExprDecl);
                self.ast.class_decl_mut(class_decl).name = v;
            }
            self.next();
        } else if !expr && !self.deflt {
            self.fail("class declaration", &[IdentifierToken]);
            return class_decl;
        }
        if self.tt == ExtendsToken {
            self.next();
            let extends = self.parse_expression(OpLHS);
            self.ast.class_decl_mut(class_decl).extends = extends;
        }

        if !self.consume("class declaration", OpenBraceToken) {
            return class_decl;
        }
        loop {
            if self.tt == ErrorToken {
                self.fail("class declaration", &[]);
                return class_decl;
            } else if self.tt == SemicolonToken {
                self.next();
                continue;
            } else if self.tt == CloseBraceToken {
                self.next();
                break;
            }

            let elem = self.parse_class_element();
            self.ast.class_decl_mut(class_decl).list.push(elem);
        }
        class_decl
    }

    // Go: parse.go:Parser.parseClassElement
    fn parse_class_element(&mut self) -> ClassElement {
        let mut method = MethodDecl::default();
        let mut data = GoBytes::nil(); // either static, async, get, or set
        if self.tt == StaticToken {
            method.static_ = true;
            data = self.data.clone();
            self.next();
            if self.tt == OpenBraceToken {
                let (prev_yield, prev_await, prev_retrn) = (self.yield_, self.await_, self.retrn);
                (self.yield_, self.await_, self.retrn) = (false, true, false);
                let elem = ClassElement {
                    static_block: self.parse_block_stmt("class static block"),
                    ..ClassElement::default()
                };
                (self.yield_, self.await_, self.retrn) = (prev_yield, prev_await, prev_retrn);
                return elem;
            }
        }
        if self.tt == MulToken {
            method.generator = true;
            self.next();
        } else if self.tt == AsyncToken {
            data = self.data.clone();
            self.next();
            if !self.prev_lt {
                method.async_ = true;
                if self.tt == MulToken {
                    method.generator = true;
                    data = GoBytes::nil();
                    self.next();
                }
            }
        } else if self.tt == GetToken {
            method.get = true;
            data = self.data.clone();
            self.next();
        } else if self.tt == SetToken {
            method.set = true;
            data = self.data.clone();
            self.next();
        }

        let mut is_field = false;
        if !data.is_nil() && self.tt == OpenParenToken {
            // (static) method name is: static, async, get, or set
            method.name.literal = LiteralExpr {
                token_type: IdentifierToken,
                data: data.clone(),
            };
            if method.async_ || method.get || method.set {
                method.async_ = false;
                method.get = false;
                method.set = false;
            } else {
                method.static_ = false;
            }
        } else if !data.is_nil()
            && (self.tt == EqToken || self.tt == SemicolonToken || self.tt == CloseBraceToken)
        {
            // (static) field name is: static, async, get, or set
            method.name.literal = LiteralExpr {
                token_type: IdentifierToken,
                data: data.clone(),
            };
            if !method.async_ && !method.get && !method.set {
                method.static_ = false;
            }
            is_field = true;
        } else {
            if self.tt == PrivateIdentifierToken {
                method.name.literal = LiteralExpr {
                    token_type: self.tt,
                    data: self.data.clone(),
                };
                self.next();
            } else {
                method.name = self.parse_property_name("method or field definition");
            }
            if (data.is_nil() || method.static_) && self.tt != OpenParenToken {
                is_field = true;
            }
        }

        if is_field {
            let mut init = NodeId::NIL;
            if self.tt == EqToken {
                self.next();
                init = self.parse_expression(OpAssign);
            }
            return ClassElement {
                field: Field {
                    static_: method.static_,
                    name: method.name,
                    init,
                },
                ..ClassElement::default()
            };
        }

        let (body_scope, parent) = self.enter_scope(true);
        let body = self.ast.alloc(Node::BlockStmt(BlockStmt {
            list: Vec::new(),
            scope: body_scope,
        }));
        method.body = body;
        let (prev_await, prev_yield, prev_retrn) = (self.await_, self.yield_, self.retrn);
        (self.await_, self.yield_, self.retrn) = (method.async_, method.generator, true);

        method.params = self.parse_func_params("method definition");

        let (prev_allow_directive_prologue, prev_expr_level) =
            (self.allow_directive_prologue, self.expr_level);
        (self.allow_directive_prologue, self.expr_level) = (true, 0);
        let list = self.parse_stmt_list("method function");
        self.ast.block_mut(body).list = list;
        (self.allow_directive_prologue, self.expr_level) =
            (prev_allow_directive_prologue, prev_expr_level);

        (self.await_, self.yield_, self.retrn) = (prev_await, prev_yield, prev_retrn);
        self.exit_scope(parent);
        let m = self.ast.alloc(Node::MethodDecl(method));
        ClassElement {
            method: m,
            ..ClassElement::default()
        }
    }

    // Go: parse.go:Parser.parsePropertyName
    fn parse_property_name(&mut self, in_: &str) -> PropertyName {
        let mut property_name = PropertyName::default();
        if is_identifier_name(self.tt) {
            property_name.literal = LiteralExpr {
                token_type: IdentifierToken,
                data: self.data.clone(),
            };
            self.next();
        } else if self.tt == StringToken {
            // reinterpret string as identifier or number if we can, except for empty strings
            let inner = self.data.slice(1, self.data.len() - 1);
            if as_identifier_name(&inner) {
                property_name.literal = LiteralExpr {
                    token_type: IdentifierToken,
                    data: inner,
                };
            } else if as_decimal_literal(&inner) {
                property_name.literal = LiteralExpr {
                    token_type: DecimalToken,
                    data: inner,
                };
            } else {
                property_name.literal = LiteralExpr {
                    token_type: self.tt,
                    data: self.data.clone(),
                };
            }
            self.next();
        } else if is_numeric(self.tt) {
            property_name.literal = LiteralExpr {
                token_type: self.tt,
                data: self.data.clone(),
            };
            self.next();
        } else if self.tt == OpenBracketToken {
            self.next();
            property_name.computed = self.parse_expression(OpAssign);
            if !self.consume(in_, CloseBracketToken) {
                return property_name;
            }
        } else {
            self.fail(
                in_,
                &[IdentifierToken, StringToken, NumericToken, OpenBracketToken],
            );
            return property_name;
        }
        property_name
    }

    // Go: parse.go:Parser.parseBindingElement
    fn parse_binding_element(&mut self, decl: DeclType) -> BindingElement {
        let mut binding_element = BindingElement::default();
        // BindingElement
        binding_element.binding = self.parse_binding(decl);
        if self.tt == EqToken {
            self.next();
            binding_element.default = self.parse_expression(OpAssign);
        }
        binding_element
    }

    // Go: parse.go:Parser.parseBinding
    fn parse_binding(&mut self, decl: DeclType) -> NodeId {
        let mut binding = NodeId::NIL;
        // BindingIdentifier, BindingPattern
        if self.is_identifier_reference(self.tt) {
            let (v, ok) = self.ast.declare(self.scope, decl, self.data.clone());
            binding = v;
            if !ok {
                let msg = already_declared(&self.data);
                self.fail_message(msg);
                return binding;
            }
            self.next();
        } else if self.tt == OpenBracketToken {
            self.next();
            let mut array = BindingArray::default();
            if self.tt == CommaToken {
                array.list.push(BindingElement::default());
            }
            let mut last = 0;
            while self.tt != CloseBracketToken {
                // elision
                while self.tt == CommaToken {
                    self.next();
                    if self.tt == CommaToken {
                        array.list.push(BindingElement::default());
                    }
                }
                // binding rest element
                if self.tt == EllipsisToken {
                    self.next();
                    array.rest = self.parse_binding(decl);
                    if self.tt != CloseBracketToken {
                        self.fail("array binding pattern", &[CloseBracketToken]);
                        return binding;
                    }
                    break;
                } else if self.tt == CloseBracketToken {
                    array.list.truncate(last);
                    break;
                }

                let be = self.parse_binding_element(decl);
                array.list.push(be);
                last = array.list.len();

                if self.tt != CommaToken && self.tt != CloseBracketToken {
                    self.fail("array binding pattern", &[CommaToken, CloseBracketToken]);
                    return binding;
                }
            }
            self.next(); // always CloseBracketToken
            binding = self.ast.alloc(Node::BindingArray(array));
        } else if self.tt == OpenBraceToken {
            self.next();
            let mut object = BindingObject::default();
            while self.tt != CloseBraceToken {
                // binding rest property
                if self.tt == EllipsisToken {
                    self.next();
                    if !self.is_identifier_reference(self.tt) {
                        self.fail("object binding pattern", &[IdentifierToken]);
                        return binding;
                    }
                    let (v, ok) = self.ast.declare(self.scope, decl, self.data.clone());
                    object.rest = v;
                    if !ok {
                        let msg = already_declared(&self.data);
                        self.fail_message(msg);
                        return binding;
                    }
                    self.next();
                    if self.tt != CloseBraceToken {
                        self.fail("object binding pattern", &[CloseBraceToken]);
                        return binding;
                    }
                    break;
                }

                let mut item = BindingObjectItem::default();
                if self.is_identifier_reference(self.tt) {
                    let name = self.data.clone();
                    let mut key = Box::new(PropertyName {
                        literal: LiteralExpr {
                            token_type: IdentifierToken,
                            data: self.data.clone(),
                        },
                        computed: NodeId::NIL,
                    });
                    self.next();
                    if self.tt == ColonToken {
                        // property name + : + binding element
                        item.key = Some(key);
                        self.next();
                        item.value = self.parse_binding_element(decl);
                    } else {
                        // single name binding
                        key.literal.data = parse_copy(&key.literal.data); // copy so that renaming doesn't rename the key
                        item.key = Some(key);
                        let (v, ok) = self.ast.declare(self.scope, decl, name.clone());
                        item.value.binding = v;
                        if !ok {
                            let msg = already_declared(&name);
                            self.fail_message(msg);
                            return binding;
                        }
                        if self.tt == EqToken {
                            self.next();
                            item.value.default = self.parse_expression(OpAssign);
                        }
                    }
                } else {
                    let property_name = self.parse_property_name("object binding pattern");
                    item.key = Some(Box::new(property_name));
                    if !self.consume("object binding pattern", ColonToken) {
                        return binding;
                    }
                    item.value = self.parse_binding_element(decl);
                }
                object.list.push(item);

                if self.tt == CommaToken {
                    self.next();
                } else if self.tt != CloseBraceToken {
                    self.fail("object binding pattern", &[CommaToken, CloseBraceToken]);
                    return binding;
                }
            }
            self.next(); // always CloseBracketToken
            binding = self.ast.alloc(Node::BindingObject(object));
        } else {
            self.fail("binding", &[]);
            return binding;
        }
        binding
    }

    // Go: parse.go:Parser.parseArrayLiteral
    fn parse_array_literal(&mut self) -> ArrayExpr {
        let mut array = ArrayExpr::default();
        // assume we're on [
        self.next();
        let mut prev_comma = true;
        loop {
            if self.tt == ErrorToken {
                self.fail("expression", &[]);
                return array;
            } else if self.tt == CloseBracketToken {
                self.next();
                break;
            } else if self.tt == CommaToken {
                if prev_comma {
                    array.list.push(Element::default());
                }
                prev_comma = true;
                self.next();
            } else {
                let spread = self.tt == EllipsisToken;
                if spread {
                    self.next();
                }
                let value = self.parse_assign_expr_or_param();
                array.list.push(Element { value, spread });
                prev_comma = false;
                if spread && self.tt != CloseBracketToken {
                    self.assume_arrow_func = false;
                }
            }
        }
        array
    }

    // Go: parse.go:Parser.parseObjectLiteral
    fn parse_object_literal(&mut self) -> ObjectExpr {
        let mut object = ObjectExpr::default();
        // assume we're on {
        self.next();
        loop {
            if self.tt == ErrorToken {
                self.fail("object literal", &[CloseBraceToken]);
                return object;
            } else if self.tt == CloseBraceToken {
                self.next();
                break;
            }

            let mut property = Property::default();
            if self.tt == EllipsisToken {
                self.next();
                property.spread = true;
                property.value = self.parse_assign_expr_or_param();
                if !self.ast.is_var(property.value) || self.tt != CloseBraceToken {
                    self.assume_arrow_func = false;
                }
            } else {
                // try to parse as MethodDefinition, otherwise fall back to PropertyName:AssignExpr or IdentifierReference
                let mut data = GoBytes::nil();
                let mut method = MethodDecl::default();
                if self.tt == MulToken {
                    self.next();
                    method.generator = true;
                } else if self.tt == AsyncToken {
                    data = self.data.clone();
                    self.next();
                    if !self.prev_lt {
                        method.async_ = true;
                        if self.tt == MulToken {
                            self.next();
                            method.generator = true;
                            data = GoBytes::nil();
                        }
                    } else {
                        method.name.literal = LiteralExpr {
                            token_type: IdentifierToken,
                            data: data.clone(),
                        };
                        data = GoBytes::nil();
                    }
                } else if self.tt == GetToken {
                    data = self.data.clone();
                    self.next();
                    method.get = true;
                } else if self.tt == SetToken {
                    data = self.data.clone();
                    self.next();
                    method.set = true;
                }

                // PropertyName
                if !data.is_nil()
                    && !method.generator
                    && (self.tt == EqToken
                        || self.tt == CommaToken
                        || self.tt == CloseBraceToken
                        || self.tt == ColonToken
                        || self.tt == OpenParenToken)
                {
                    method.name.literal = LiteralExpr {
                        token_type: IdentifierToken,
                        data: data.clone(),
                    };
                    method.async_ = false;
                    method.get = false;
                    method.set = false;
                } else if !method.name.is_set() {
                    // did not parse async [LT]
                    method.name = self.parse_property_name("object literal");
                    if !method.name.is_set() {
                        return object;
                    }
                }

                if self.tt == OpenParenToken {
                    // MethodDefinition
                    let (body_scope, parent) = self.enter_scope(true);
                    let body = self.ast.alloc(Node::BlockStmt(BlockStmt {
                        list: Vec::new(),
                        scope: body_scope,
                    }));
                    method.body = body;
                    let (prev_await, prev_yield, prev_retrn) =
                        (self.await_, self.yield_, self.retrn);
                    (self.await_, self.yield_, self.retrn) =
                        (method.async_, method.generator, true);

                    method.params = self.parse_func_params("method definition");
                    let list = self.parse_stmt_list("method definition");
                    self.ast.block_mut(body).list = list;

                    (self.await_, self.yield_, self.retrn) = (prev_await, prev_yield, prev_retrn);
                    self.exit_scope(parent);
                    property.value = self.ast.alloc(Node::MethodDecl(method));
                    self.assume_arrow_func = false;
                } else if self.tt == ColonToken {
                    // PropertyName : AssignmentExpression
                    self.next();
                    property.name = Some(Box::new(method.name));
                    property.value = self.parse_assign_expr_or_param();
                } else if method.name.is_computed()
                    || !self.is_identifier_reference(method.name.literal.token_type)
                {
                    self.fail("object literal", &[ColonToken, OpenParenToken]);
                    return object;
                } else {
                    // IdentifierReference (= AssignmentExpression)?
                    let name = method.name.literal.data.clone();
                    method.name.literal.data = parse_copy(&method.name.literal.data); // copy so that renaming doesn't rename the key
                    property.name = Some(Box::new(method.name)); // set key explicitly so after renaming the original is still known
                    if self.assume_arrow_func {
                        let (v, ok) = self.ast.declare(self.scope, ArgumentDecl, name.clone());
                        property.value = v;
                        if !ok {
                            property.value = self.ast.use_var(self.scope, name);
                            self.assume_arrow_func = false;
                        }
                    } else {
                        property.value = self.ast.use_var(self.scope, name);
                    }
                    if self.tt == EqToken {
                        self.next();
                        let prev_assume_arrow_func = self.assume_arrow_func;
                        self.assume_arrow_func = false;
                        property.init = self.parse_expression(OpAssign);
                        self.assume_arrow_func = prev_assume_arrow_func;
                    }
                }
            }
            object.list.push(property);
            if self.tt == CommaToken {
                self.next();
            } else if self.tt != CloseBraceToken {
                self.fail("object literal", &[]);
                return object;
            }
        }
        object
    }

    // Go: parse.go:Parser.parseTemplateLiteral
    fn parse_template_literal(&mut self, prec_left: OpPrec) -> TemplateExpr {
        let mut template = TemplateExpr::default();
        // assume we're on 'Template' or 'TemplateStart'
        template.prec = OpMember;
        if prec_left < OpMember {
            template.prec = OpCall;
        }
        while self.tt == TemplateStartToken || self.tt == TemplateMiddleToken {
            let tpl = self.data.clone();
            self.next();
            let expr = self.parse_expression(OpExpr);
            template.list.push(TemplatePart { value: tpl, expr });
        }
        if self.tt != TemplateToken && self.tt != TemplateEndToken {
            self.fail("template literal", &[TemplateToken]);
            return template;
        }
        template.tail = self.data.clone();
        self.next(); // TemplateEndToken
        template
    }

    // Go: parse.go:Parser.parseArguments
    fn parse_arguments(&mut self) -> Args {
        let mut args = Args::default();
        // assume we're on (
        self.next();
        args.list = Vec::with_capacity(4);
        while self.tt != CloseParenToken && self.tt != ErrorToken {
            let rest = self.tt == EllipsisToken;
            if rest {
                self.next();
            }
            let value = self.parse_expression(OpAssign);
            args.list.push(Arg { value, rest });
            if self.tt != CloseParenToken {
                if self.tt != CommaToken {
                    self.fail("arguments", &[CommaToken, CloseParenToken]);
                    return args;
                } else {
                    self.next(); // CommaToken
                }
            }
        }
        self.consume("arguments", CloseParenToken);
        args
    }

    // Go: parse.go:Parser.parseAsyncArrowFunc
    fn parse_async_arrow_func(&mut self) -> NodeId {
        // expect we're at Identifier or Yield or (
        let arrow_func = self.ast.alloc(Node::ArrowFunc(ArrowFunc::default()));
        let (body_scope, parent) = self.enter_scope(true);
        let body = self.ast.alloc(Node::BlockStmt(BlockStmt {
            list: Vec::new(),
            scope: body_scope,
        }));
        self.ast.arrow_func_mut(arrow_func).body = body;
        let (prev_await, prev_yield) = (self.await_, self.yield_);
        (self.await_, self.yield_) = (true, false);

        if is_identifier(self.tt) || !prev_yield && self.tt == YieldToken {
            let (r, _) = self
                .ast
                .declare(self.scope, ArgumentDecl, self.data.clone()); // cannot fail
            self.next();
            self.ast.arrow_func_mut(arrow_func).params.list = vec![BindingElement {
                binding: r,
                default: NodeId::NIL,
            }];
        } else {
            let params = self.parse_func_params("arrow function");
            self.ast.arrow_func_mut(arrow_func).params = params;
            // CallExpression of 'async(params)' already handled
        }

        self.ast.arrow_func_mut(arrow_func).async_ = true;
        let list = self.parse_arrow_func_body();
        self.ast.block_mut(body).list = list;

        (self.await_, self.yield_) = (prev_await, prev_yield);
        self.exit_scope(parent);
        arrow_func
    }

    // Go: parse.go:Parser.parseIdentifierArrowFunc
    fn parse_identifier_arrow_func(&mut self, v: NodeId) -> NodeId {
        let mut v = v;
        // expect we're at =>
        let arrow_func = self.ast.alloc(Node::ArrowFunc(ArrowFunc::default()));
        let (body_scope, parent) = self.enter_scope(true);
        let body = self.ast.alloc(Node::BlockStmt(BlockStmt {
            list: Vec::new(),
            scope: body_scope,
        }));
        self.ast.arrow_func_mut(arrow_func).body = body;
        let (prev_await, prev_yield) = (self.await_, self.yield_);
        (self.await_, self.yield_) = (false, false);

        if 1 < self.ast.var(v).uses {
            let vv = self.ast.var_mut(v);
            vv.uses = vv.uses.wrapping_sub(1);
            let data = parse_copy(&self.ast.var(v).data);
            (v, _) = self.ast.declare(self.scope, ArgumentDecl, data); // cannot fail
        } else {
            // if v.Uses==1 it must be undeclared and be the last added
            let p = self.ast.scope(self.scope).parent;
            let und = &mut self.ast.scope_mut(p).undeclared;
            assert!(!und.is_empty(), "slice bounds out of range [:-1]");
            und.pop();
            self.ast.var_mut(v).decl = ArgumentDecl;
            self.ast.scope_mut(self.scope).declared.push(v);
        }

        self.ast.arrow_func_mut(arrow_func).params.list = vec![BindingElement {
            binding: v,
            default: NodeId::NIL,
        }];
        let list = self.parse_arrow_func_body();
        self.ast.block_mut(body).list = list;

        (self.await_, self.yield_) = (prev_await, prev_yield);
        self.exit_scope(parent);
        arrow_func
    }

    // Go: parse.go:Parser.parseArrowFuncBody
    fn parse_arrow_func_body(&mut self) -> Vec<NodeId> {
        let mut list = Vec::new();
        // expect we're at arrow
        if self.tt != ArrowToken {
            self.fail("arrow function", &[ArrowToken]);
            return list;
        } else if self.prev_lt {
            self.fail("expression", &[]);
            return list;
        }
        self.next();

        // mark undeclared vars as arguments in `function f(a=b){var b}` where the b's are different vars
        self.ast.mark_func_args(self.scope);

        if self.tt == OpenBraceToken {
            let (prev_in, prev_retrn) = (self.in_, self.retrn);
            (self.in_, self.retrn) = (true, true);

            let (prev_allow_directive_prologue, prev_expr_level) =
                (self.allow_directive_prologue, self.expr_level);
            (self.allow_directive_prologue, self.expr_level) = (true, 0);
            list = self.parse_stmt_list("arrow function");
            (self.allow_directive_prologue, self.expr_level) =
                (prev_allow_directive_prologue, prev_expr_level);

            (self.in_, self.retrn) = (prev_in, prev_retrn);
        } else {
            let value = self.parse_expression(OpAssign);
            list = vec![self.ast.alloc(Node::ReturnStmt(ReturnStmt { value }))];
        }
        list
    }

    // Go: parse.go:Parser.parseIdentifierExpression
    fn parse_identifier_expression(&mut self, prec: OpPrec, ident: GoBytes) -> NodeId {
        let left = self.ast.use_var(self.scope, ident);
        self.parse_expression_suffix(left, prec, OpPrimary)
    }

    // Go: parse.go:Parser.parseAsyncExpression
    fn parse_async_expression(&mut self, prec: OpPrec, async_: GoBytes) -> NodeId {
        // IdentifierReference, AsyncFunctionExpression, AsyncGeneratorExpression
        // CoverCallExpressionAndAsyncArrowHead, AsyncArrowFunction
        // assume we're at a token after async
        let left;
        let mut prec_left = OpPrimary;
        if !self.prev_lt && self.tt == FunctionToken {
            // primary expression
            left = self.parse_async_func_expr();
        } else if !self.prev_lt
            && prec <= OpAssign
            && (self.tt == OpenParenToken
                || is_identifier(self.tt)
                || self.tt == YieldToken
                || self.tt == AwaitToken)
        {
            // async arrow function expression or call expression
            if self.tt == AwaitToken || self.yield_ && self.tt == YieldToken {
                self.fail("arrow function", &[]);
                return NodeId::NIL;
            } else if self.tt == OpenParenToken {
                return self.parse_parenthesized_expression(prec, async_);
            }
            left = self.parse_async_arrow_func();
            prec_left = OpAssign;
        } else {
            left = self.ast.use_var(self.scope, async_);
        }
        // can be async(args), async => ..., or e.g. async + ...
        self.parse_expression_suffix(left, prec, prec_left)
    }

    // Go: parse.go:Parser.parseExpression
    /// Parses an expression that has a precedence of prec or higher.
    fn parse_expression(&mut self, prec: OpPrec) -> NodeId {
        self.expr_level += 1;
        if NESTED_EXPR_LIMIT < self.expr_level {
            self.fail_message(b"too many nested expressions".to_vec());
            return NodeId::NIL;
        }

        // reparse input if we have / or /= as the beginning of a new expression, this should be a regular expression!
        if self.tt == DivToken || self.tt == DivEqToken {
            (self.tt, self.data) = self.l.reg_exp();
            if self.tt == ErrorToken {
                self.fail("regular expression", &[]);
                return NodeId::NIL;
            }
        }

        let mut left;
        let mut prec_left = OpPrimary;

        if is_identifier(self.tt) && self.tt != AsyncToken {
            left = self.ast.use_var(self.scope, self.data.clone());
            self.next();
            let suffix = self.parse_expression_suffix(left, prec, prec_left);
            self.expr_level -= 1;
            return suffix;
        } else if is_numeric(self.tt) {
            left = self.ast.alloc(Node::LiteralExpr(LiteralExpr {
                token_type: self.tt,
                data: self.data.clone(),
            }));
            self.next();
            let suffix = self.parse_expression_suffix(left, prec, prec_left);
            self.expr_level -= 1;
            return suffix;
        }

        let tt = self.tt;
        match tt {
            StringToken | ThisToken | NullToken | TrueToken | FalseToken | RegExpToken => {
                left = self.ast.alloc(Node::LiteralExpr(LiteralExpr {
                    token_type: self.tt,
                    data: self.data.clone(),
                }));
                self.next();
            }
            OpenBracketToken => {
                let prev_in = self.in_;
                self.in_ = true;
                let array = self.parse_array_literal();
                self.in_ = prev_in;
                left = self.ast.alloc(Node::ArrayExpr(array));
            }
            OpenBraceToken => {
                let prev_in = self.in_;
                self.in_ = true;
                let object = self.parse_object_literal();
                self.in_ = prev_in;
                left = self.ast.alloc(Node::ObjectExpr(object));
            }
            OpenParenToken => {
                // parenthesized expression or arrow parameter list
                if OpAssign < prec {
                    // must be a parenthesized expression
                    self.next();
                    let prev_in = self.in_;
                    self.in_ = true;
                    let x = self.parse_expression(OpExpr);
                    left = self.ast.alloc(Node::GroupExpr(GroupExpr { x }));
                    self.in_ = prev_in;
                    if !self.consume("expression", CloseParenToken) {
                        return NodeId::NIL;
                    }
                } else {
                    let suffix = self.parse_parenthesized_expression(prec, GoBytes::nil());
                    self.expr_level -= 1;
                    return suffix;
                }
            }
            NotToken | BitNotToken | TypeofToken | VoidToken | DeleteToken => {
                if OpUnary < prec {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                }
                self.next();
                let x = self.parse_expression(OpUnary);
                left = self.ast.alloc(Node::UnaryExpr(UnaryExpr { op: tt, x }));
                prec_left = OpUnary;
            }
            AddToken => {
                if OpUnary < prec {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                }
                self.next();
                let x = self.parse_expression(OpUnary);
                left = self
                    .ast
                    .alloc(Node::UnaryExpr(UnaryExpr { op: PosToken, x }));
                prec_left = OpUnary;
            }
            SubToken => {
                if OpUnary < prec {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                }
                self.next();
                let x = self.parse_expression(OpUnary);
                left = self
                    .ast
                    .alloc(Node::UnaryExpr(UnaryExpr { op: NegToken, x }));
                prec_left = OpUnary;
            }
            IncrToken => {
                if OpUpdate < prec {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                }
                self.next();
                let x = self.parse_expression(OpUnary);
                left = self.ast.alloc(Node::UnaryExpr(UnaryExpr {
                    op: PreIncrToken,
                    x,
                }));
                prec_left = OpUnary;
            }
            DecrToken => {
                if OpUpdate < prec {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                }
                self.next();
                let x = self.parse_expression(OpUnary);
                left = self.ast.alloc(Node::UnaryExpr(UnaryExpr {
                    op: PreDecrToken,
                    x,
                }));
                prec_left = OpUnary;
            }
            AwaitToken => {
                // either accepted as IdentifierReference or as AwaitExpression
                if self.await_ && prec <= OpUnary {
                    self.next();
                    let x = self.parse_expression(OpUnary);
                    left = self.ast.alloc(Node::UnaryExpr(UnaryExpr { op: tt, x }));
                    prec_left = OpUnary;
                } else if self.await_ {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                } else {
                    left = self.ast.use_var(self.scope, self.data.clone());
                    self.next();
                }
            }
            NewToken => {
                self.next();
                if self.tt == DotToken {
                    self.next();
                    if !self.consume("new.target expression", TargetToken) {
                        return NodeId::NIL;
                    }
                    left = self.ast.alloc(Node::NewTargetExpr);
                    prec_left = OpMember;
                } else {
                    let x = self.parse_expression(OpNew);
                    let new_expr = self.ast.alloc(Node::NewExpr(NewExpr { x, args: None }));
                    if self.tt == OpenParenToken {
                        let args = self.parse_arguments();
                        if !args.list.is_empty() {
                            if let Node::NewExpr(n) = self.ast.node_mut(new_expr) {
                                n.args = Some(args);
                            }
                        }
                        prec_left = OpMember;
                    } else {
                        prec_left = OpNew;
                    }
                    left = new_expr;
                }
            }
            ImportToken => {
                // OpMember < prec does never happen
                left = self.ast.alloc(Node::LiteralExpr(LiteralExpr {
                    token_type: self.tt,
                    data: self.data.clone(),
                }));
                self.next();
                if self.tt == DotToken {
                    self.next();
                    if !self.consume("import.meta expression", MetaToken) {
                        return NodeId::NIL;
                    }
                    left = self.ast.alloc(Node::ImportMetaExpr);
                    prec_left = OpMember;
                } else if self.tt != OpenParenToken {
                    self.fail("import expression", &[OpenParenToken]);
                    return NodeId::NIL;
                } else if OpCall < prec {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                } else {
                    prec_left = OpCall;
                }
            }
            SuperToken => {
                // OpMember < prec does never happen
                left = self.ast.alloc(Node::LiteralExpr(LiteralExpr {
                    token_type: self.tt,
                    data: self.data.clone(),
                }));
                self.next();
                if OpCall < prec && self.tt != DotToken && self.tt != OpenBracketToken {
                    self.fail("super expression", &[OpenBracketToken, DotToken]);
                    return NodeId::NIL;
                } else if self.tt != DotToken
                    && self.tt != OpenBracketToken
                    && self.tt != OpenParenToken
                {
                    self.fail(
                        "super expression",
                        &[OpenBracketToken, OpenParenToken, DotToken],
                    );
                    return NodeId::NIL;
                }
                if OpCall < prec {
                    prec_left = OpMember;
                } else {
                    prec_left = OpCall;
                }
            }
            YieldToken => {
                // either accepted as IdentifierReference or as YieldExpression
                if self.yield_ && prec <= OpAssign {
                    // YieldExpression
                    self.next();
                    let mut yield_expr = YieldExpr::default();
                    if !self.prev_lt {
                        yield_expr.generator = self.tt == MulToken;
                        if yield_expr.generator {
                            self.next();
                            yield_expr.x = self.parse_expression(OpAssign);
                        } else if self.tt != CloseBraceToken
                            && self.tt != CloseBracketToken
                            && self.tt != CloseParenToken
                            && self.tt != ColonToken
                            && self.tt != CommaToken
                            && self.tt != SemicolonToken
                        {
                            yield_expr.x = self.parse_expression(OpAssign);
                        }
                    }
                    left = self.ast.alloc(Node::YieldExpr(yield_expr));
                    prec_left = OpAssign;
                } else if self.yield_ {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                } else {
                    left = self.ast.use_var(self.scope, self.data.clone());
                    self.next();
                }
            }
            AsyncToken => {
                let async_ = self.data.clone();
                self.next();
                let prev_in = self.in_;
                self.in_ = true;
                left = self.parse_async_expression(prec, async_);
                self.in_ = prev_in;
            }
            ClassToken => {
                let prev_in = self.in_;
                self.in_ = true;
                left = self.parse_class_expr();
                self.in_ = prev_in;
            }
            FunctionToken => {
                let prev_in = self.in_;
                self.in_ = true;
                left = self.parse_func_expr();
                self.in_ = prev_in;
            }
            TemplateToken | TemplateStartToken => {
                let prev_in = self.in_;
                self.in_ = true;
                let template = self.parse_template_literal(prec_left);
                left = self.ast.alloc(Node::TemplateExpr(template));
                self.in_ = prev_in;
            }
            PrivateIdentifierToken => {
                if OpCompare < prec || !self.in_ {
                    self.fail("expression", &[]);
                    return NodeId::NIL;
                }
                left = self.ast.alloc(Node::LiteralExpr(LiteralExpr {
                    token_type: self.tt,
                    data: self.data.clone(),
                }));
                self.next();
                if self.tt != InToken {
                    self.fail("relational expression", &[InToken]);
                    return NodeId::NIL;
                }
            }
            _ => {
                self.fail("expression", &[]);
                return NodeId::NIL;
            }
        }
        let suffix = self.parse_expression_suffix(left, prec, prec_left);
        self.expr_level -= 1;
        suffix
    }

    // Go: parse.go:Parser.parseExpressionSuffix
    fn parse_expression_suffix(&mut self, left: NodeId, prec: OpPrec, prec_left: OpPrec) -> NodeId {
        let mut left = left;
        let mut prec_left = prec_left;
        let mut i: i64 = 0;
        loop {
            if 1000 < self.expr_level + i {
                self.fail_message(b"too many nested expressions".to_vec());
                return NodeId::NIL;
            }

            let tt = self.tt;
            match tt {
                EqToken | MulEqToken | DivEqToken | ModEqToken | ExpEqToken | AddEqToken
                | SubEqToken | LtLtEqToken | GtGtEqToken | GtGtGtEqToken | BitAndEqToken
                | BitXorEqToken | BitOrEqToken | AndEqToken | OrEqToken | NullishEqToken => {
                    if OpAssign < prec {
                        return left;
                    } else if prec_left < OpLHS {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpAssign);
                    left = self.binary(tt, left, y);
                    prec_left = OpAssign;
                }
                LtToken | LtEqToken | GtToken | GtEqToken | InToken | InstanceofToken => {
                    if OpCompare < prec || !self.in_ && tt == InToken {
                        return left;
                    } else if prec_left < OpCompare {
                        // can only fail after a yield or arrow function expression
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpShift);
                    left = self.binary(tt, left, y);
                    prec_left = OpCompare;
                }
                EqEqToken | NotEqToken | EqEqEqToken | NotEqEqToken => {
                    if OpEquals < prec {
                        return left;
                    } else if prec_left < OpEquals {
                        // can only fail after a yield or arrow function expression
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpCompare);
                    left = self.binary(tt, left, y);
                    prec_left = OpEquals;
                }
                AndToken => {
                    if OpAnd < prec {
                        return left;
                    } else if prec_left < OpAnd {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpBitOr);
                    left = self.binary(tt, left, y);
                    prec_left = OpAnd;
                }
                OrToken => {
                    if OpOr < prec {
                        return left;
                    } else if prec_left < OpOr {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpAnd);
                    left = self.binary(tt, left, y);
                    prec_left = OpOr;
                }
                NullishToken => {
                    if OpCoalesce < prec {
                        return left;
                    } else if prec_left < OpBitOr && prec_left != OpCoalesce {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpBitOr);
                    left = self.binary(tt, left, y);
                    prec_left = OpCoalesce;
                }
                DotToken => {
                    // OpMember < prec does never happen
                    if prec_left < OpCall {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    if !is_identifier_name(self.tt) && self.tt != PrivateIdentifierToken {
                        self.fail("dot expression", &[IdentifierToken]);
                        return NodeId::NIL;
                    }
                    let mut expr_prec = OpMember;
                    if prec_left < OpMember {
                        expr_prec = OpCall;
                    }
                    if self.tt != PrivateIdentifierToken {
                        self.tt = IdentifierToken;
                    }
                    left = self.ast.alloc(Node::DotExpr(DotExpr {
                        x: left,
                        y: LiteralExpr {
                            token_type: self.tt,
                            data: self.data.clone(),
                        },
                        prec: expr_prec,
                        optional: false,
                    }));
                    self.next();
                    if prec_left < OpMember {
                        prec_left = OpCall;
                    } else {
                        prec_left = OpMember;
                    }
                }
                OpenBracketToken => {
                    // OpMember < prec does never happen
                    if prec_left < OpCall {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let mut expr_prec = OpMember;
                    if prec_left < OpMember {
                        expr_prec = OpCall;
                    }
                    let prev_in = self.in_;
                    self.in_ = true;
                    let y = self.parse_expression(OpExpr);
                    left = self.ast.alloc(Node::IndexExpr(IndexExpr {
                        x: left,
                        y,
                        prec: expr_prec,
                        optional: false,
                    }));
                    self.in_ = prev_in;
                    if !self.consume("index expression", CloseBracketToken) {
                        return NodeId::NIL;
                    }
                    if prec_left < OpMember {
                        prec_left = OpCall;
                    } else {
                        prec_left = OpMember;
                    }
                }
                OpenParenToken => {
                    if OpCall < prec {
                        return left;
                    } else if prec_left < OpCall {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    let prev_in = self.in_;
                    self.in_ = true;
                    let args = self.parse_arguments();
                    left = self.ast.alloc(Node::CallExpr(CallExpr {
                        x: left,
                        args,
                        optional: false,
                    }));
                    prec_left = OpCall;
                    self.in_ = prev_in;
                }
                TemplateToken | TemplateStartToken => {
                    // OpMember < prec does never happen
                    if prec_left < OpCall {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    let prev_in = self.in_;
                    self.in_ = true;
                    let mut template = self.parse_template_literal(prec_left);
                    template.tag = left;
                    left = self.ast.alloc(Node::TemplateExpr(template));
                    if prec_left < OpMember {
                        prec_left = OpCall;
                    } else {
                        prec_left = OpMember;
                    }
                    self.in_ = prev_in;
                }
                OptChainToken => {
                    if OpCall < prec {
                        return left;
                    } else if prec_left < OpCall {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    if self.tt == OpenParenToken {
                        let args = self.parse_arguments();
                        left = self.ast.alloc(Node::CallExpr(CallExpr {
                            x: left,
                            args,
                            optional: true,
                        }));
                    } else if self.tt == OpenBracketToken {
                        self.next();
                        let y = self.parse_expression(OpExpr);
                        left = self.ast.alloc(Node::IndexExpr(IndexExpr {
                            x: left,
                            y,
                            prec: OpCall,
                            optional: true,
                        }));
                        if !self.consume("optional chaining expression", CloseBracketToken) {
                            return NodeId::NIL;
                        }
                    } else if self.tt == TemplateToken || self.tt == TemplateStartToken {
                        let mut template = self.parse_template_literal(prec_left);
                        template.prec = OpCall;
                        template.tag = left;
                        template.optional = true;
                        left = self.ast.alloc(Node::TemplateExpr(template));
                    } else if is_identifier_name(self.tt) {
                        left = self.ast.alloc(Node::DotExpr(DotExpr {
                            x: left,
                            y: LiteralExpr {
                                token_type: IdentifierToken,
                                data: self.data.clone(),
                            },
                            prec: OpCall,
                            optional: true,
                        }));
                        self.next();
                    } else if self.tt == PrivateIdentifierToken {
                        left = self.ast.alloc(Node::DotExpr(DotExpr {
                            x: left,
                            y: LiteralExpr {
                                token_type: self.tt,
                                data: self.data.clone(),
                            },
                            prec: OpCall,
                            optional: true,
                        }));
                        self.next();
                    } else {
                        self.fail(
                            "optional chaining expression",
                            &[
                                IdentifierToken,
                                OpenParenToken,
                                OpenBracketToken,
                                TemplateToken,
                            ],
                        );
                        return NodeId::NIL;
                    }
                    prec_left = OpCall;
                }
                IncrToken => {
                    if self.prev_lt || OpUpdate < prec {
                        return left;
                    } else if prec_left < OpLHS {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    left = self.ast.alloc(Node::UnaryExpr(UnaryExpr {
                        op: PostIncrToken,
                        x: left,
                    }));
                    prec_left = OpUpdate;
                }
                DecrToken => {
                    if self.prev_lt || OpUpdate < prec {
                        return left;
                    } else if prec_left < OpLHS {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    left = self.ast.alloc(Node::UnaryExpr(UnaryExpr {
                        op: PostDecrToken,
                        x: left,
                    }));
                    prec_left = OpUpdate;
                }
                ExpToken => {
                    if OpExp < prec {
                        return left;
                    } else if prec_left < OpUpdate {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpExp);
                    left = self.binary(tt, left, y);
                    prec_left = OpExp;
                }
                MulToken | DivToken | ModToken => {
                    if OpMul < prec {
                        return left;
                    } else if prec_left < OpMul {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpExp);
                    left = self.binary(tt, left, y);
                    prec_left = OpMul;
                }
                AddToken | SubToken => {
                    if OpAdd < prec {
                        return left;
                    } else if prec_left < OpAdd {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpMul);
                    left = self.binary(tt, left, y);
                    prec_left = OpAdd;
                }
                LtLtToken | GtGtToken | GtGtGtToken => {
                    if OpShift < prec {
                        return left;
                    } else if prec_left < OpShift {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpAdd);
                    left = self.binary(tt, left, y);
                    prec_left = OpShift;
                }
                BitAndToken => {
                    if OpBitAnd < prec {
                        return left;
                    } else if prec_left < OpBitAnd {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpEquals);
                    left = self.binary(tt, left, y);
                    prec_left = OpBitAnd;
                }
                BitXorToken => {
                    if OpBitXor < prec {
                        return left;
                    } else if prec_left < OpBitXor {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpBitAnd);
                    left = self.binary(tt, left, y);
                    prec_left = OpBitXor;
                }
                BitOrToken => {
                    if OpBitOr < prec {
                        return left;
                    } else if prec_left < OpBitOr {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let y = self.parse_expression(OpBitXor);
                    left = self.binary(tt, left, y);
                    prec_left = OpBitOr;
                }
                QuestionToken => {
                    if OpAssign < prec {
                        return left;
                    } else if prec_left < OpCoalesce {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }
                    self.next();
                    let prev_in = self.in_;
                    self.in_ = true;
                    let if_expr = self.parse_expression(OpAssign);
                    self.in_ = prev_in;
                    if !self.consume("conditional expression", ColonToken) {
                        return NodeId::NIL;
                    }
                    let else_expr = self.parse_expression(OpAssign);
                    left = self.ast.alloc(Node::CondExpr(CondExpr {
                        cond: left,
                        x: if_expr,
                        y: else_expr,
                    }));
                    prec_left = OpAssign;
                }
                CommaToken => {
                    if OpExpr < prec {
                        return left;
                    }
                    self.next();
                    if matches!(self.ast.node(left), Node::CommaExpr(_)) {
                        let e = self.parse_expression(OpAssign);
                        if let Node::CommaExpr(c) = self.ast.node_mut(left) {
                            c.list.push(e);
                        }
                        i -= 1; // adjust expression nesting limit
                    } else {
                        let e = self.parse_expression(OpAssign);
                        left = self.ast.alloc(Node::CommaExpr(CommaExpr {
                            list: vec![left, e],
                        }));
                    }
                    prec_left = OpExpr;
                }
                ArrowToken => {
                    // handle identifier => ..., where identifier could also be yield or await
                    if OpAssign < prec {
                        return left;
                    } else if prec_left < OpPrimary || self.prev_lt {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }

                    if !self.ast.is_var(left) {
                        self.fail("expression", &[]);
                        return NodeId::NIL;
                    }

                    left = self.parse_identifier_arrow_func(left);
                    prec_left = OpAssign;
                }
                _ => {
                    return left;
                }
            }
            i += 1;
        }
    }

    fn binary(&mut self, op: TokenType, x: NodeId, y: NodeId) -> NodeId {
        self.ast.alloc(Node::BinaryExpr(BinaryExpr { op, x, y }))
    }

    // Go: parse.go:Parser.parseAssignExprOrParam
    fn parse_assign_expr_or_param(&mut self) -> NodeId {
        // this could be a BindingElement or an AssignmentExpression. Here we handle BindingIdentifier with a possible Initializer, BindingPattern will be handled by parseArrayLiteral or parseObjectLiteral
        if self.assume_arrow_func && self.is_identifier_reference(self.tt) {
            let tt = self.tt;
            let data = self.data.clone();
            self.next();
            if self.tt == EqToken
                || self.tt == CommaToken
                || self.tt == CloseParenToken
                || self.tt == CloseBraceToken
                || self.tt == CloseBracketToken
            {
                let (left, ok) = self.ast.declare(self.scope, ArgumentDecl, data.clone());
                if ok {
                    self.assume_arrow_func = false;
                    let left = self.parse_expression_suffix(left, OpAssign, OpPrimary);
                    self.assume_arrow_func = true;
                    return left;
                }
            }
            self.assume_arrow_func = false;
            if tt == AsyncToken {
                return self.parse_async_expression(OpAssign, data);
            }
            return self.parse_identifier_expression(OpAssign, data);
        } else if self.tt != OpenBracketToken && self.tt != OpenBraceToken {
            self.assume_arrow_func = false;
        }
        self.parse_expression(OpAssign)
    }

    // Go: parse.go:Parser.parseParenthesizedExpression
    fn parse_parenthesized_expression(&mut self, prec: OpPrec, async_: GoBytes) -> NodeId {
        // parse ArrowFunc, AsyncArrowFunc, AsyncCallExpr, ParenthesizedExpr
        let left;
        let mut prec_left = OpPrimary;

        // expect to be at (
        self.next();

        let is_async = !async_.is_nil(); // prevLT is false before open parenthesis
        let arrow_func = self.ast.alloc(Node::ArrowFunc(ArrowFunc::default()));
        let (body_scope, parent) = self.enter_scope(true);
        let body = self.ast.alloc(Node::BlockStmt(BlockStmt {
            list: Vec::new(),
            scope: body_scope,
        }));
        self.ast.arrow_func_mut(arrow_func).body = body;
        let (prev_assume_arrow_func, prev_in) = (self.assume_arrow_func, self.in_);
        (self.assume_arrow_func, self.in_) = (true, true);

        // parse an Arguments expression but assume we might be parsing an (async) arrow function or ParenthesisedExpression. If this is really an arrow function, parsing as an Arguments expression cannot fail as AssignmentExpression, ArrayLiteral, and ObjectLiteral are supersets of SingleNameBinding, ArrayBindingPattern, and ObjectBindingPattern respectively. Any identifier that would be a BindingIdentifier in case of an arrow function, will be added as such to the scope. If finally this is not an arrow function, we will demote those variables as undeclared and merge them with the parent scope.

        let mut rests = 0usize;
        let mut args = Args::default();
        while self.tt != CloseParenToken && self.tt != ErrorToken {
            if 0 < args.list.len() && args.list[args.list.len() - 1].rest {
                // only last parameter can have ellipsis
                self.assume_arrow_func = false;
                if !is_async {
                    self.fail("arrow function", &[CloseParenToken]);
                }
            }

            let rest = self.tt == EllipsisToken;
            if rest {
                self.next();
                rests += 1;
            }

            let value = self.parse_assign_expr_or_param();
            args.list.push(Arg { value, rest });
            if self.tt != CommaToken {
                break;
            }
            self.next();
        }
        if self.tt != CloseParenToken {
            self.fail("expression", &[]);
            return NodeId::NIL;
        }
        self.next();
        let is_arrow_func = !self.prev_lt && self.tt == ArrowToken && self.assume_arrow_func;
        let has_last_rest = 0 < rests && self.assume_arrow_func;
        (self.assume_arrow_func, self.in_) = (prev_assume_arrow_func, prev_in);

        if is_arrow_func {
            let (prev_await, prev_yield) = (self.await_, self.yield_);
            (self.await_, self.yield_) = (is_async, false);

            // arrow function
            self.ast.arrow_func_mut(arrow_func).async_ = is_async;
            let mut params = Params {
                list: Vec::with_capacity(args.list.len() - rests),
                rest: NodeId::NIL,
            };
            for arg in &args.list {
                if arg.rest {
                    params.rest = self.expr_to_binding(arg.value);
                } else {
                    let be = self.expr_to_binding_element(arg.value); // can not fail when assumArrowFunc is set
                    params.list.push(be);
                }
            }
            self.ast.arrow_func_mut(arrow_func).params = params;
            let list = self.parse_arrow_func_body();
            self.ast.block_mut(body).list = list;

            (self.await_, self.yield_) = (prev_await, prev_yield);
            self.exit_scope(parent);

            left = arrow_func;
            prec_left = OpAssign;
        } else if !is_async && (args.list.is_empty() || has_last_rest) {
            self.fail("arrow function", &[ArrowToken]);
            return NodeId::NIL;
        } else if is_async && OpCall < prec || !is_async && 0 < rests {
            self.fail("expression", &[]);
            return NodeId::NIL;
        } else {
            // for any nested FuncExpr/ArrowFunc scope, Parent will point to the temporary scope created in case this was an arrow function instead of a parenthesized expression. This is not a problem as Parent is only used for defining new variables, and we already parsed all the nested scopes so that Parent (not Func) are not relevant anymore. Anyways, the Parent will just point to an empty scope, whose Parent/Func will point to valid scopes. This should not be a big deal.
            // Here we move all declared ArgumentDecls (in case of an arrow function) to its parent scope as undeclared variables (identifiers used in a parenthesized expression).
            self.exit_scope(parent);
            self.ast.undeclare_scope(body_scope);

            if is_async {
                // call expression
                let x = self.ast.use_var(self.scope, async_);
                left = self.ast.alloc(Node::CallExpr(CallExpr {
                    x,
                    args,
                    optional: false,
                }));
                prec_left = OpCall;
            } else {
                // parenthesized expression
                if 1 < args.list.len() {
                    let mut comma_expr = CommaExpr::default();
                    for arg in &args.list {
                        comma_expr.list.push(arg.value);
                    }
                    let c = self.ast.alloc(Node::CommaExpr(comma_expr));
                    left = self.ast.alloc(Node::GroupExpr(GroupExpr { x: c }));
                } else {
                    left = self.ast.alloc(Node::GroupExpr(GroupExpr {
                        x: args.list[0].value,
                    }));
                }
            }
        }
        self.parse_expression_suffix(left, prec, prec_left)
    }

    // Go: parse.go:Parser.exprToBindingElement
    /// exprToBindingElement and exprToBinding convert a
    /// CoverParenthesizedExpressionAndArrowParameterList into
    /// FormalParameters. Any unbound variables of the parameters
    /// (Initializer, ComputedPropertyName) are kept in the parent scope.
    fn expr_to_binding_element(&mut self, expr: NodeId) -> BindingElement {
        let mut binding_element = BindingElement::default();
        let assign = match self.ast.node(expr) {
            Node::BinaryExpr(b) if b.op == EqToken => Some((b.x, b.y)),
            _ => None,
        };
        if let Some((x, y)) = assign {
            binding_element.binding = self.expr_to_binding(x);
            binding_element.default = y;
        } else {
            binding_element.binding = self.expr_to_binding(expr);
        }
        binding_element
    }

    // Go: parse.go:Parser.exprToBinding
    fn expr_to_binding(&mut self, expr: NodeId) -> NodeId {
        let mut binding = NodeId::NIL;
        match self.ast.node(expr) {
            Node::Nil => {
                // no-op
            }
            Node::Var(_) => {
                binding = expr;
            }
            Node::ArrayExpr(array) => {
                let items = array.list.clone();
                let mut binding_array = BindingArray::default();
                for item in items {
                    if item.spread {
                        // can only BindingIdentifier or BindingPattern
                        binding_array.rest = self.expr_to_binding(item.value);
                        break;
                    }
                    let be = self.expr_to_binding_element(item.value);
                    binding_array.list.push(be);
                }
                binding = self.ast.alloc(Node::BindingArray(binding_array));
            }
            Node::ObjectExpr(object) => {
                let items = object.list.clone();
                let mut binding_object = BindingObject::default();
                for item in items {
                    if item.spread {
                        // can only be BindingIdentifier
                        if !self.ast.is_var(item.value) {
                            panic!(
                                "interface conversion: js.IExpr is {}, not *js.Var",
                                node_kind(self.ast.node(item.value))
                            );
                        }
                        binding_object.rest = item.value;
                        break;
                    }

                    let mut be = self.expr_to_binding_element(item.value);
                    let ident = match (&item.name, self.ast.node(item.value)) {
                        (Some(name), Node::Var(v)) => name.is_ident(&v.data),
                        _ => false,
                    };
                    if item.name.is_none() || ident {
                        // IdentifierReference : Initializer
                        be.default = item.init;
                    }
                    binding_object.list.push(BindingObjectItem {
                        key: item.name,
                        value: be,
                    });
                }
                binding = self.ast.alloc(Node::BindingObject(binding_object));
            }
            _ => {
                self.fail_message(b"invalid parameters in arrow function".to_vec());
            }
        }
        binding
    }

    // Go: parse.go:Parser.isIdentifierReference
    fn is_identifier_reference(&self, tt: TokenType) -> bool {
        is_identifier(tt) || !self.yield_ && tt == YieldToken || !self.await_ && tt == AwaitToken
    }
}

/// Go `fmt.Errorf("identifier %s has already been declared", string(name))`
fn already_declared(name: &GoBytes) -> Vec<u8> {
    let mut msg = b"identifier ".to_vec();
    name.write_to(&mut msg);
    msg.extend_from_slice(b" has already been declared");
    msg
}

/// Go `[]byte("...")` of a constant string: go1.27.1 allocates exactly
/// `len` bytes here (`cap == len`, verified by the oracle's cap dumps).
fn go_string_bytes(s: &[u8]) -> GoBytes {
    GoBytes::from_slice(s)
}

impl Ast {
    /// Mutable access that yields `Node::Nil` for nil (for type switches on
    /// possibly-nil interfaces).
    pub(crate) fn node_mut_or_nil(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.0 as usize]
    }
}
