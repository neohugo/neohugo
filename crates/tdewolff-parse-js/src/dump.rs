//! Deterministic serializations of token streams and of the whole AST graph
//! (tree, scopes, VarDecls, Vars with pointer identities numbered by first
//! encounter). Mirrored byte for byte by
//! `tools/go-oracle/tdewolff-parse-js/dump.go`; used by the differential
//! tests and handy for debugging downstream crates.

use std::collections::HashMap;

use tdewolff_parse::{GoBytes, GoError, Input};

use crate::ast::*;
use crate::lex::Lexer;
use crate::parse::{Options, parse};
use crate::tokentype::*;

/// Escapes bytes: printable ASCII except `"` and `\` verbatim, everything
/// else as `\xHH`.
pub fn esc(buf: &mut Vec<u8>, b: &[u8]) {
    const HEXD: &[u8; 16] = b"0123456789abcdef";
    for &c in b {
        if (0x20..0x7f).contains(&c) && c != b'"' && c != b'\\' {
            buf.push(c);
        } else {
            buf.extend_from_slice(b"\\x");
            buf.push(HEXD[(c >> 4) as usize]);
            buf.push(HEXD[(c & 15) as usize]);
        }
    }
}

/// A `[]byte` field: `nil`, or `"escaped"/cap`.
pub fn dump_bytes(buf: &mut Vec<u8>, b: &GoBytes) {
    if b.is_nil() {
        buf.extend_from_slice(b"nil");
        return;
    }
    buf.push(b'"');
    esc(buf, &b.to_vec());
    buf.extend_from_slice(b"\"/");
    buf.extend_from_slice(b.cap().to_string().as_bytes());
}

/// Go `err.Error()` (`nil`/`EOF` spelled out).
pub fn err_string(err: &Option<GoError>) -> Vec<u8> {
    match err {
        None => b"nil".to_vec(),
        Some(e) => e.error_bytes(),
    }
}

fn input_of(src: &[u8]) -> Input {
    Input::new_bytes(GoBytes::from_slice(src))
}

/// The plain token stream, continuing after lexer errors. With `regexps`, a
/// DivToken/DivEqToken where a regular expression may start (heuristic on
/// the previous significant token) is re-lexed with `RegExp()`.
pub fn lex_dump(src: &[u8], regexps: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        lex_dump_into(&mut buf, src, regexps)
    }));
    if r.is_err() {
        // the lexer can run past the end of the input after an error (Go panics too)
        buf.extend_from_slice(b"PANIC\n");
    }
    buf
}

fn lex_dump_into(buf: &mut Vec<u8>, src: &[u8], regexps: bool) {
    let mut l = Lexer::new(input_of(src));
    let mut prev = ErrorToken;
    for _ in 0..(1 << 22) {
        let (mut tt, mut data) = l.next();
        if regexps && (tt == DivToken || tt == DivEqToken) && regex_allowed(prev) {
            buf.extend_from_slice(b"R ");
            (tt, data) = l.reg_exp();
        }
        buf.extend_from_slice(tt.0.to_string().as_bytes());
        buf.push(b' ');
        dump_bytes(buf, &data);
        buf.push(b'\n');
        if tt == ErrorToken {
            let err = l.err();
            if tdewolff_parse::is_eof(&err) {
                buf.extend_from_slice(b"EOF\n");
                return;
            }
            buf.extend_from_slice(b"ERR ");
            esc(buf, &err_string(&err));
            buf.push(b'\n');
        }
        if tt != WhitespaceToken
            && tt != LineTerminatorToken
            && tt != CommentToken
            && tt != CommentLineTerminatorToken
        {
            prev = tt;
        }
    }
    buf.extend_from_slice(b"LIMIT\n");
}

/// The regular-expression heuristic of [`lex_dump`].
pub fn regex_allowed(prev: TokenType) -> bool {
    if is_identifier(prev) || is_numeric(prev) {
        return false;
    }
    !matches!(
        prev,
        StringToken
            | RegExpToken
            | TemplateToken
            | TemplateEndToken
            | PrivateIdentifierToken
            | CloseParenToken
            | CloseBracketToken
            | CloseBraceToken
            | ThisToken
            | SuperToken
            | NullToken
            | TrueToken
            | FalseToken
            | IncrToken
            | DecrToken
    )
}

struct Dumper<'a> {
    ast: &'a Ast,
    buf: Vec<u8>,
    vars: HashMap<NodeId, usize>,
    var_list: Vec<NodeId>,
    scopes: HashMap<ScopeId, usize>,
    scope_list: Vec<ScopeId>,
    decls: HashMap<NodeId, usize>,
    decl_list: Vec<NodeId>,
    decl_printed: HashMap<NodeId, bool>,
}

impl<'a> Dumper<'a> {
    fn s(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    fn b(&mut self, b: &GoBytes) {
        dump_bytes(&mut self.buf, b);
    }

    fn i(&mut self, n: i64) {
        self.buf.extend_from_slice(n.to_string().as_bytes());
    }

    fn bool(&mut self, v: bool) {
        self.s(if v { "1" } else { "0" });
    }

    fn v(&mut self, v: NodeId) {
        if v.is_nil() {
            self.s("nil");
            return;
        }
        let id = match self.vars.get(&v) {
            Some(&id) => id,
            None => {
                self.var_list.push(v);
                let id = self.var_list.len();
                self.vars.insert(v, id);
                id
            }
        };
        self.s("v");
        self.i(id as i64);
    }

    fn sc(&mut self, s: ScopeId) {
        if s.is_nil() {
            self.s("nil");
            return;
        }
        let id = match self.scopes.get(&s) {
            Some(&id) => id,
            None => {
                self.scope_list.push(s);
                let id = self.scope_list.len();
                self.scopes.insert(s, id);
                id
            }
        };
        self.s("s");
        self.i(id as i64);
    }

    fn decl_ref(&mut self, v: NodeId) -> usize {
        match self.decls.get(&v) {
            Some(&id) => id,
            None => {
                self.decl_list.push(v);
                let id = self.decl_list.len();
                self.decls.insert(v, id);
                id
            }
        }
    }

    fn lit(&mut self, n: &LiteralExpr) {
        self.s("(Lit ");
        self.i(n.token_type.0 as i64);
        self.s(" ");
        self.b(&n.data);
        self.s(")");
    }

    fn pn(&mut self, n: Option<&PropertyName>) {
        let Some(n) = n else {
            self.s("nil");
            return;
        };
        self.s("(PN ");
        self.lit(&n.literal);
        self.s(" ");
        self.n(n.computed);
        self.s(")");
    }

    fn be(&mut self, n: &BindingElement) {
        self.s("(BE ");
        self.n(n.binding);
        self.s(" ");
        self.n(n.default);
        self.s(")");
    }

    fn bes(&mut self, l: &[BindingElement]) {
        self.s("[");
        for (i, item) in l.iter().enumerate() {
            if i != 0 {
                self.s(" ");
            }
            self.be(item);
        }
        self.s("]");
    }

    fn params(&mut self, n: &Params) {
        self.s("(Params ");
        self.bes(&n.list);
        self.s(" ");
        self.n(n.rest);
        self.s(")");
    }

    fn stmts(&mut self, l: &[NodeId]) {
        self.s("[");
        for &item in l {
            self.s("\n");
            self.n(item);
        }
        self.s("]");
    }

    fn aliases(&mut self, l: &[Alias]) {
        self.s("[");
        for (i, a) in l.iter().enumerate() {
            if i != 0 {
                self.s(" ");
            }
            self.s("(Alias ");
            self.b(&a.name);
            self.s(" ");
            self.b(&a.binding);
            self.s(")");
        }
        self.s("]");
    }

    fn args(&mut self, n: &Args) {
        self.s("(Args [");
        for (i, a) in n.list.iter().enumerate() {
            if i != 0 {
                self.s(" ");
            }
            self.s("(Arg ");
            self.n(a.value);
            self.s(" ");
            self.bool(a.rest);
            self.s(")");
        }
        self.s("])");
    }

    fn block(&mut self, id: NodeId) {
        if id.is_nil() {
            self.s("nil");
            return;
        }
        let ast = self.ast;
        let n = ast.block(id);
        self.s("(Block ");
        self.sc(n.scope);
        self.s(" ");
        self.stmts(&n.list);
        self.s(")");
    }

    fn var_decl(&mut self, id: NodeId) {
        let ast = self.ast;
        let n = ast.var_decl(id);
        let did = self.decl_ref(id);
        if self.decl_printed.get(&id).copied().unwrap_or(false) {
            self.s("d");
            self.i(did as i64);
            return;
        }
        self.decl_printed.insert(id, true);
        self.s("(VarDecl d");
        self.i(did as i64);
        self.s(" ");
        self.i(n.token_type.0 as i64);
        self.s(" ");
        self.sc(n.scope);
        self.s(" ");
        self.bool(n.in_for);
        self.s(" ");
        self.bool(n.in_for_in_of);
        self.s(" ");
        self.bes(&n.list);
        self.s(")");
    }

    fn n(&mut self, id: NodeId) {
        let ast = self.ast;
        match ast.node(id) {
            Node::Nil => self.s("nil"),
            Node::Comment(n) => {
                self.s("(Comment ");
                self.b(&n.value);
                self.s(")");
            }
            Node::BlockStmt(_) => self.block(id),
            Node::EmptyStmt => self.s("(Empty)"),
            Node::ExprStmt(n) => {
                self.s("(Expr ");
                self.n(n.value);
                self.s(")");
            }
            Node::IfStmt(n) => {
                self.s("(If ");
                self.n(n.cond);
                self.s(" ");
                self.n(n.body);
                self.s(" ");
                self.n(n.else_);
                self.s(")");
            }
            Node::DoWhileStmt(n) => {
                self.s("(DoWhile ");
                self.n(n.cond);
                self.s(" ");
                self.n(n.body);
                self.s(")");
            }
            Node::WhileStmt(n) => {
                self.s("(While ");
                self.n(n.cond);
                self.s(" ");
                self.n(n.body);
                self.s(")");
            }
            Node::ForStmt(n) => {
                self.s("(For ");
                self.n(n.init);
                self.s(" ");
                self.n(n.cond);
                self.s(" ");
                self.n(n.post);
                self.s(" ");
                self.block(n.body);
                self.s(")");
            }
            Node::ForInStmt(n) => {
                self.s("(ForIn ");
                self.n(n.init);
                self.s(" ");
                self.n(n.value);
                self.s(" ");
                self.block(n.body);
                self.s(")");
            }
            Node::ForOfStmt(n) => {
                self.s("(ForOf ");
                self.bool(n.await_);
                self.s(" ");
                self.n(n.init);
                self.s(" ");
                self.n(n.value);
                self.s(" ");
                self.block(n.body);
                self.s(")");
            }
            Node::SwitchStmt(n) => {
                self.s("(Switch ");
                self.sc(n.scope);
                self.s(" ");
                self.n(n.init);
                self.s(" [");
                for (i, c) in n.list.iter().enumerate() {
                    if i != 0 {
                        self.s(" ");
                    }
                    self.s("(Case ");
                    self.i(c.token_type.0 as i64);
                    self.s(" ");
                    self.n(c.cond);
                    self.s(" ");
                    self.stmts(&c.list);
                    self.s(")");
                }
                self.s("])");
            }
            Node::BranchStmt(n) => {
                self.s("(Branch ");
                self.i(n.type_.0 as i64);
                self.s(" ");
                self.b(&n.label);
                self.s(")");
            }
            Node::ReturnStmt(n) => {
                self.s("(Return ");
                self.n(n.value);
                self.s(")");
            }
            Node::WithStmt(n) => {
                self.s("(With ");
                self.n(n.cond);
                self.s(" ");
                self.n(n.body);
                self.s(")");
            }
            Node::LabelledStmt(n) => {
                self.s("(Label ");
                self.b(&n.label);
                self.s(" ");
                self.n(n.value);
                self.s(")");
            }
            Node::ThrowStmt(n) => {
                self.s("(Throw ");
                self.n(n.value);
                self.s(")");
            }
            Node::TryStmt(n) => {
                self.s("(Try ");
                self.block(n.body);
                self.s(" ");
                self.n(n.binding);
                self.s(" ");
                self.block(n.catch);
                self.s(" ");
                self.block(n.finally);
                self.s(")");
            }
            Node::DebuggerStmt => self.s("(Debugger)"),
            Node::ImportStmt(n) => {
                self.s("(Import ");
                match &n.list {
                    None => self.s("nil"),
                    Some(l) => self.aliases(l),
                }
                self.s(" ");
                self.b(&n.default);
                self.s(" ");
                self.b(&n.module);
                self.s(")");
            }
            Node::ExportStmt(n) => {
                self.s("(Export ");
                self.aliases(&n.list);
                self.s(" ");
                self.b(&n.module);
                self.s(" ");
                self.bool(n.default);
                self.s(" ");
                self.n(n.decl);
                self.s(")");
            }
            Node::DirectivePrologueStmt(n) => {
                self.s("(Directive ");
                self.b(&n.value);
                self.s(")");
            }
            Node::VarDecl(_) => self.var_decl(id),
            Node::FuncDecl(n) => {
                self.s("(Func ");
                self.bool(n.async_);
                self.s(" ");
                self.bool(n.generator);
                self.s(" ");
                self.v(n.name);
                self.s(" ");
                self.params(&n.params);
                self.s(" ");
                self.block(n.body);
                self.s(")");
            }
            Node::MethodDecl(n) => {
                self.s("(Method ");
                self.bool(n.static_);
                self.s(" ");
                self.bool(n.async_);
                self.s(" ");
                self.bool(n.generator);
                self.s(" ");
                self.bool(n.get);
                self.s(" ");
                self.bool(n.set);
                self.s(" ");
                self.pn(Some(&n.name));
                self.s(" ");
                self.params(&n.params);
                self.s(" ");
                self.block(n.body);
                self.s(")");
            }
            Node::ClassDecl(n) => {
                self.s("(Class ");
                self.v(n.name);
                self.s(" ");
                self.n(n.extends);
                self.s(" [");
                for (i, e) in n.list.iter().enumerate() {
                    if i != 0 {
                        self.s(" ");
                    }
                    self.s("(Elem ");
                    self.block(e.static_block);
                    self.s(" ");
                    self.n(e.method);
                    self.s(" (Field ");
                    self.bool(e.field.static_);
                    self.s(" ");
                    self.pn(Some(&e.field.name));
                    self.s(" ");
                    self.n(e.field.init);
                    self.s("))");
                }
                self.s("])");
            }
            Node::Var(_) => self.v(id),
            Node::BindingArray(n) => {
                self.s("(BArray ");
                self.bes(&n.list);
                self.s(" ");
                self.n(n.rest);
                self.s(")");
            }
            Node::BindingObject(n) => {
                self.s("(BObject [");
                for (i, item) in n.list.iter().enumerate() {
                    if i != 0 {
                        self.s(" ");
                    }
                    self.s("(BItem ");
                    self.pn(item.key.as_deref());
                    self.s(" ");
                    self.be(&item.value);
                    self.s(")");
                }
                self.s("] ");
                self.v(n.rest);
                self.s(")");
            }
            Node::LiteralExpr(n) => self.lit(n),
            Node::ArrayExpr(n) => {
                self.s("(Array [");
                for (i, e) in n.list.iter().enumerate() {
                    if i != 0 {
                        self.s(" ");
                    }
                    self.s("(El ");
                    self.n(e.value);
                    self.s(" ");
                    self.bool(e.spread);
                    self.s(")");
                }
                self.s("])");
            }
            Node::ObjectExpr(n) => {
                self.s("(Object [");
                for (i, p) in n.list.iter().enumerate() {
                    if i != 0 {
                        self.s(" ");
                    }
                    self.s("(Prop ");
                    self.pn(p.name.as_deref());
                    self.s(" ");
                    self.bool(p.spread);
                    self.s(" ");
                    self.n(p.value);
                    self.s(" ");
                    self.n(p.init);
                    self.s(")");
                }
                self.s("])");
            }
            Node::TemplateExpr(n) => {
                self.s("(Template ");
                self.n(n.tag);
                self.s(" [");
                for (i, p) in n.list.iter().enumerate() {
                    if i != 0 {
                        self.s(" ");
                    }
                    self.s("(Part ");
                    self.b(&p.value);
                    self.s(" ");
                    self.n(p.expr);
                    self.s(")");
                }
                self.s("] ");
                self.b(&n.tail);
                self.s(" ");
                self.i(n.prec.0);
                self.s(" ");
                self.bool(n.optional);
                self.s(")");
            }
            Node::GroupExpr(n) => {
                self.s("(Group ");
                self.n(n.x);
                self.s(")");
            }
            Node::IndexExpr(n) => {
                self.s("(Index ");
                self.n(n.x);
                self.s(" ");
                self.n(n.y);
                self.s(" ");
                self.i(n.prec.0);
                self.s(" ");
                self.bool(n.optional);
                self.s(")");
            }
            Node::DotExpr(n) => {
                self.s("(Dot ");
                self.n(n.x);
                self.s(" ");
                self.lit(&n.y);
                self.s(" ");
                self.i(n.prec.0);
                self.s(" ");
                self.bool(n.optional);
                self.s(")");
            }
            Node::NewTargetExpr => self.s("(NewTarget)"),
            Node::ImportMetaExpr => self.s("(ImportMeta)"),
            Node::NewExpr(n) => {
                self.s("(New ");
                self.n(n.x);
                self.s(" ");
                match &n.args {
                    None => self.s("nil"),
                    Some(a) => self.args(a),
                }
                self.s(")");
            }
            Node::CallExpr(n) => {
                self.s("(Call ");
                self.n(n.x);
                self.s(" ");
                self.args(&n.args);
                self.s(" ");
                self.bool(n.optional);
                self.s(")");
            }
            Node::UnaryExpr(n) => {
                self.s("(Unary ");
                self.i(n.op.0 as i64);
                self.s(" ");
                self.n(n.x);
                self.s(")");
            }
            Node::BinaryExpr(n) => {
                self.s("(Binary ");
                self.i(n.op.0 as i64);
                self.s(" ");
                self.n(n.x);
                self.s(" ");
                self.n(n.y);
                self.s(")");
            }
            Node::CondExpr(n) => {
                self.s("(Cond ");
                self.n(n.cond);
                self.s(" ");
                self.n(n.x);
                self.s(" ");
                self.n(n.y);
                self.s(")");
            }
            Node::YieldExpr(n) => {
                self.s("(Yield ");
                self.bool(n.generator);
                self.s(" ");
                self.n(n.x);
                self.s(")");
            }
            Node::ArrowFunc(n) => {
                self.s("(Arrow ");
                self.bool(n.async_);
                self.s(" ");
                self.params(&n.params);
                self.s(" ");
                self.block(n.body);
                self.s(")");
            }
            Node::CommaExpr(n) => {
                self.s("(Comma [");
                for (i, &e) in n.list.iter().enumerate() {
                    if i != 0 {
                        self.s(" ");
                    }
                    self.n(e);
                }
                self.s("])");
            }
        }
    }

    fn var_list2(&mut self, l: &[NodeId]) {
        self.s("[");
        for (i, &v) in l.iter().enumerate() {
            if i != 0 {
                self.s(" ");
            }
            self.v(v);
        }
        self.s("]");
    }
}

/// Serializes the whole AST graph: the tree, then every reachable scope,
/// VarDecl and Var.
pub fn ast_dump(ast: &Ast) -> Vec<u8> {
    let mut d = Dumper {
        ast,
        buf: Vec::new(),
        vars: HashMap::new(),
        var_list: Vec::new(),
        scopes: HashMap::new(),
        scope_list: Vec::new(),
        decls: HashMap::new(),
        decl_list: Vec::new(),
        decl_printed: HashMap::new(),
    };
    d.block(ast.block_stmt);
    d.s("\n== scopes\n");
    let (mut si, mut di) = (0, 0);
    while si < d.scope_list.len() || di < d.decl_list.len() {
        while si < d.scope_list.len() {
            let s = ast.scope(d.scope_list[si]);
            si += 1;
            d.s("s");
            d.i(si as i64);
            d.s(" ");
            d.sc(s.parent);
            d.s(" ");
            d.sc(s.func);
            d.s(" ");
            d.var_list2(&s.declared);
            d.s(" ");
            d.var_list2(&s.undeclared);
            d.s(" [");
            for (i, &vd) in s.var_decls.iter().enumerate() {
                if i != 0 {
                    d.s(" ");
                }
                d.s("d");
                let id = d.decl_ref(vd);
                d.i(id as i64);
            }
            d.s("] ");
            d.i(s.num_for_decls as i64);
            d.s(" ");
            d.i(s.num_func_args as i64);
            d.s(" ");
            d.i(s.num_arg_uses as i64);
            d.s(" ");
            d.bool(s.is_global_or_func);
            d.s(" ");
            d.bool(s.has_with);
            d.s("\n");
        }
        while di < d.decl_list.len() {
            let vd = d.decl_list[di];
            di += 1;
            if !d.decl_printed.get(&vd).copied().unwrap_or(false) {
                d.var_decl(vd);
                d.s("\n");
            }
        }
    }
    d.s("== vars\n");
    let mut vi = 0;
    while vi < d.var_list.len() {
        let v = ast.var(d.var_list[vi]);
        vi += 1;
        d.s("v");
        d.i(vi as i64);
        d.s(" ");
        d.b(&v.data);
        d.s(" ");
        d.v(v.link);
        d.s(" ");
        d.i(v.uses as i64);
        d.s(" ");
        d.i(v.decl.0 as i64);
        d.s("\n");
    }
    d.buf
}

/// Parses `src` with the given options and serializes the result (or the
/// error).
pub fn parse_dump(src: &[u8], o: Options) -> Vec<u8> {
    let r = std::panic::catch_unwind(|| match parse(&input_of(src), o) {
        Err(err) => {
            let mut buf = b"ERR ".to_vec();
            esc(&mut buf, &err.error_bytes());
            buf.push(b'\n');
            buf
        }
        Ok(ast) => ast_dump(&ast),
    });
    r.unwrap_or_else(|_| b"PANIC\n".to_vec())
}

/// `ast.String()` of a successful parse (`ERR` on error, `PANIC` when Go's
/// `String()` panics).
pub fn string_dump(src: &[u8], o: Options) -> Vec<u8> {
    // Go recovers panics of Parse too
    let r = std::panic::catch_unwind(|| parse(&input_of(src), o));
    let Ok(res) = r else {
        return b"PANIC\n".to_vec();
    };
    match res {
        Err(_) => b"ERR\n".to_vec(),
        Ok(ast) => {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ast.string()));
            match r {
                Ok(s) => {
                    let mut buf = Vec::new();
                    esc(&mut buf, &s);
                    buf.push(b'\n');
                    buf
                }
                Err(_) => b"PANIC\n".to_vec(),
            }
        }
    }
}

/// `ast.JSString()` of a successful parse (`ERR` on error, `PANIC` when Go's
/// `JS()` panics).
pub fn js_dump(src: &[u8], o: Options) -> Vec<u8> {
    // Go recovers panics of Parse too
    let r = std::panic::catch_unwind(|| parse(&input_of(src), o));
    let Ok(res) = r else {
        return b"PANIC\n".to_vec();
    };
    match res {
        Err(_) => b"ERR\n".to_vec(),
        Ok(ast) => {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ast.js_string()));
            match r {
                Ok(s) => {
                    let mut buf = Vec::new();
                    esc(&mut buf, &s);
                    buf.push(b'\n');
                    buf
                }
                Err(_) => b"PANIC\n".to_vec(),
            }
        }
    }
}

/// `ast.JSONString()` of a successful parse: the output and the error.
pub fn json_dump(src: &[u8], o: Options) -> Vec<u8> {
    // Go recovers panics of Parse too
    let r = std::panic::catch_unwind(|| parse(&input_of(src), o));
    let Ok(res) = r else {
        return b"PANIC\n".to_vec();
    };
    match res {
        Err(_) => b"ERR\n".to_vec(),
        Ok(ast) => {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ast.json_string()));
            match r {
                Ok((s, err)) => {
                    let mut buf = Vec::new();
                    esc(&mut buf, &s);
                    buf.push(b'\n');
                    match err {
                        Ok(()) => buf.extend_from_slice(b"nil"),
                        Err(e) => esc(&mut buf, &e),
                    }
                    buf.push(b'\n');
                    buf
                }
                Err(_) => b"PANIC\n".to_vec(),
            }
        }
    }
}
