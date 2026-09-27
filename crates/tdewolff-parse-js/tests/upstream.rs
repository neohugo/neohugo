//! The upstream tests of parse/v2@v2.8.1/js ported to Rust: lex_test.go,
//! parse_test.go (incl. the ScopeVars scope/uses tests) and util_test.go.
//! The tables are generated into upstream_tables.rs.

mod common;
#[allow(dead_code)]
mod upstream_tables;

use std::collections::HashMap;

use tdewolff_parse::{GoError, GoReader, Input, is_eof};
use tdewolff_parse_js::*;
use upstream_tables::*;

fn s(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn parse_str(js: &[u8]) -> Result<Ast, GoError> {
    parse(&Input::new_string(js), Options::default())
}

// Go: lex_test.go:TestTokens
#[test]
fn test_tokens() {
    for (js, expected) in TOKEN_TESTS {
        let mut l = Lexer::new(Input::new_string(js));
        let mut tokens = Vec::new();
        loop {
            let (token, _) = l.next();
            if token == ErrorToken {
                if !is_eof(&l.err()) {
                    tokens.push(token);
                }
                break;
            } else if token == WhitespaceToken {
                continue;
            }
            tokens.push(token);
        }
        assert_eq!(
            &tokens[..],
            *expected,
            "token types must match: {:?}",
            s(js)
        );
    }

    // coverage
    for start in [0u16, 0x0100, 0x0200, 0x0600, 0x0800] {
        let mut i = start;
        loop {
            if TokenType(i).string() == format!("Invalid({})", i) {
                break;
            }
            i += 1;
        }
    }

    assert!(is_punctuator(CommaToken));
    assert!(is_punctuator(GtGtEqToken));
    assert!(!is_punctuator(WhileToken));
    assert!(!is_operator(CommaToken));
    assert!(is_operator(GtGtEqToken));
    assert!(!is_operator(WhileToken));
    assert!(!is_identifier(CommaToken));
    assert!(!is_identifier(GtGtEqToken));
    assert!(is_reserved_word(WhileToken));
    assert!(is_identifier(AsyncToken));
    assert!(is_identifier_name(WhileToken));
    assert!(is_identifier_name(AsToken));

    assert!(is_identifier_start(b"a"));
    assert!(!is_identifier_start(b"6"));
    assert!(!is_identifier_start(b"["));
    assert!(is_identifier_continue(b"a"));
    assert!(is_identifier_continue(b"6"));
    assert!(!is_identifier_continue(b"["));
    assert!(is_identifier_end(b".a"));
    assert!(is_identifier_end(b".6"));
    assert!(!is_identifier_end(b".["));
}

// Go: lex_test.go:TestRegExp
#[test]
fn test_reg_exp() {
    for (js, expected) in REGEXP_TESTS {
        let mut l = Lexer::new(Input::new_string(js));
        let mut tokens = Vec::new();
        loop {
            let (mut token, _) = l.next();
            if token == DivToken || token == DivEqToken {
                (token, _) = l.reg_exp();
            }
            if token == ErrorToken {
                if !is_eof(&l.err()) {
                    tokens.push(token);
                }
                break;
            } else if token == WhitespaceToken {
                continue;
            }
            tokens.push(token);
        }
        assert_eq!(
            &tokens[..],
            *expected,
            "token types must match: {:?}",
            s(js)
        );
    }

    let (token, _) = Lexer::new(Input::new_string(b"")).reg_exp();
    assert_eq!(token, ErrorToken);
}

// Go: lex_test.go:TestOffset
#[test]
fn test_offset() {
    let z = Input::new_string(b"var i=5;");
    let mut l = Lexer::new(z.clone());
    assert_eq!(z.offset(), 0);
    let _ = l.next();
    assert_eq!(z.offset(), 3); // var
    let _ = l.next();
    assert_eq!(z.offset(), 4); // ws
    let _ = l.next();
    assert_eq!(z.offset(), 5); // i
    let _ = l.next();
    assert_eq!(z.offset(), 6); // =
    let _ = l.next();
    assert_eq!(z.offset(), 7); // 5
    let _ = l.next();
    assert_eq!(z.offset(), 8); // ;
}

fn lexer_message(err: &Option<GoError>) -> Vec<u8> {
    match err {
        Some(GoError::Parse(e)) => e.message.clone(),
        e => panic!("not a *parse.Error: {:?}", e),
    }
}

// Go: lex_test.go:TestLexerErrors
#[test]
fn test_lexer_errors() {
    for (js, err) in LEXER_ERROR_TESTS {
        let mut l = Lexer::new(Input::new_string(js));
        while l.err().is_none() {
            l.next();
        }
        assert_eq!(s(&lexer_message(&l.err())), s(err), "{:?}", s(js));
    }

    let mut l = Lexer::new(Input::new_string(b""));
    l.reg_exp();
    assert_eq!(lexer_message(&l.err()), b"expected / or /=");

    let mut l = Lexer::new(Input::new_string(b"/"));
    l.next();
    l.reg_exp();
    assert_eq!(lexer_message(&l.err()), b"unexpected EOF or newline");

    // see #118
    let mut l = Lexer::new(Input::new_string("\u{FFFD}a".as_bytes()));
    let (tt, data) = l.next();
    assert_eq!(tt, ErrorToken);
    assert_eq!(data.to_vec(), "\u{FFFD}".as_bytes());
    assert_eq!(lexer_message(&l.err()), "unexpected \u{FFFD}".as_bytes());
    let (tt, data) = l.next();
    assert_eq!(tt, IdentifierToken);
    assert_eq!(data.to_vec(), b"a");
}

// Go: lex_test.go:ExampleNewLexer
#[test]
fn example_new_lexer() {
    let mut l = Lexer::new(Input::new_string(b"var x = 'lorem ipsum';"));
    let mut out = Vec::new();
    loop {
        let (tt, data) = l.next();
        if tt == ErrorToken {
            break;
        }
        out.extend(data.to_vec());
    }
    assert_eq!(out, b"var x = 'lorem ipsum';");
}

// Go: parse_test.go:TestParse
#[test]
fn test_parse() {
    common::big_stack(|| {
        for (js, expected) in PARSE_TESTS {
            let ast = parse_str(js).unwrap_or_else(|e| panic!("{:?}: {}", s(js), e));
            assert_eq!(s(&ast.string()), s(expected), "{:?}", s(js));
        }
    });

    // coverage
    let mut i = 0;
    loop {
        if OpPrec(i).string() == format!("Invalid({})", i) {
            break;
        }
        i += 1;
    }
    let mut i = 0;
    loop {
        if DeclType(i).string() == format!("Invalid({})", i) {
            break;
        }
        i += 1;
    }
}

// Go: parse_test.go:TestParseError
#[test]
fn test_parse_error() {
    common::big_stack(|| {
        for (js, want) in PARSE_ERROR_TESTS {
            let err = match parse_str(js) {
                Ok(_) => panic!("{:?}: expected an error", s(js)),
                Err(e) => e,
            };
            assert!(!err.is_eof());
            let mut e = err.error_bytes();
            if want.len() < e.len() {
                e.truncate(want.len());
            }
            assert_eq!(s(&e), s(want), "{:?}", s(js));
        }
    });
}

// Go: parse_test.go:ScopeVars
struct ScopeVars<'a> {
    ast: &'a Ast,
    bound: Vec<u8>,
    uses: Vec<u8>,
    scopes: usize,
    refs: HashMap<NodeId, usize>,
}

impl<'a> ScopeVars<'a> {
    fn new(ast: &'a Ast) -> Self {
        ScopeVars {
            ast,
            bound: Vec::new(),
            uses: Vec::new(),
            scopes: 0,
            refs: HashMap::new(),
        }
    }

    fn string(&self) -> String {
        format!("bound:{} uses:{}", s(&self.bound), s(&self.uses))
    }

    fn ref_(&mut self, v: NodeId) -> usize {
        if let Some(&r) = self.refs.get(&v) {
            return r;
        }
        let n = self.refs.len() + 1;
        self.refs.insert(v, n);
        n
    }

    fn add_scope(&mut self, scope: ScopeId) {
        if self.scopes != 0 {
            self.bound.push(b'/');
            self.uses.push(b'/');
        }
        self.scopes += 1;

        let sc = self.ast.scope(scope);
        let mut bounds = Vec::new();
        for &v in &sc.declared {
            let mut b = self.ast.var(v).data.to_vec();
            b.extend(format!("={}", self.ref_(v)).as_bytes());
            bounds.push(b);
        }
        self.bound.extend(bounds.join(&b","[..]));

        let mut uses = Vec::new();
        for &v in &sc.undeclared {
            let mut v = v;
            let mut links = String::new();
            while self.ast.var(v).link.is_some() {
                v = self.ast.var(v).link;
                links.push('*');
            }
            let mut b = self.ast.var(v).data.to_vec();
            b.extend(format!("={}{}", self.ref_(v), links).as_bytes());
            uses.push(b);
        }
        self.uses.extend(uses.join(&b","[..]));
    }

    fn add_func_like(&mut self, scope: ScopeId, params: &Params, body: NodeId) {
        self.add_scope(scope);
        for item in &params.list {
            if item.binding.is_some() {
                self.add_binding(item.binding);
            }
            if item.default.is_some() {
                self.add_expr(item.default);
            }
        }
        if params.rest.is_some() {
            self.add_binding(params.rest);
        }
        for &item in &self.ast.block(body).list {
            self.add_stmt(item);
        }
    }

    fn add_expr(&mut self, iexpr: NodeId) {
        let ast = self.ast;
        match ast.node(iexpr) {
            Node::FuncDecl(expr) => {
                self.add_func_like(ast.block(expr.body).scope, &expr.params, expr.body)
            }
            Node::ClassDecl(expr) => {
                for item in &expr.list {
                    if item.method.is_some() {
                        let m = ast.method_decl(item.method);
                        self.add_scope(ast.block(m.body).scope);
                    }
                }
            }
            Node::ArrowFunc(expr) => {
                self.add_func_like(ast.block(expr.body).scope, &expr.params, expr.body)
            }
            Node::CondExpr(expr) => {
                self.add_expr(expr.cond);
                self.add_expr(expr.x);
                self.add_expr(expr.y);
            }
            Node::UnaryExpr(expr) => self.add_expr(expr.x),
            Node::BinaryExpr(expr) => {
                self.add_expr(expr.x);
                self.add_expr(expr.y);
            }
            Node::GroupExpr(expr) => self.add_expr(expr.x),
            Node::CommaExpr(expr) => {
                for &item in &expr.list {
                    self.add_expr(item);
                }
            }
            _ => {}
        }
    }

    fn add_binding(&mut self, ibinding: NodeId) {
        let ast = self.ast;
        match ast.node(ibinding) {
            Node::BindingArray(binding) => {
                for item in &binding.list {
                    if item.binding.is_some() {
                        self.add_binding(item.binding);
                    }
                    if item.default.is_some() {
                        self.add_expr(item.default);
                    }
                }
                if binding.rest.is_some() {
                    self.add_binding(binding.rest);
                }
            }
            Node::BindingObject(binding) => {
                for item in &binding.list {
                    let key = item.key.as_ref().expect("nil pointer dereference");
                    if key.is_computed() {
                        self.add_expr(key.computed);
                    }
                    if item.value.binding.is_some() {
                        self.add_binding(item.value.binding);
                    }
                    if item.value.default.is_some() {
                        self.add_expr(item.value.default);
                    }
                }
            }
            _ => {}
        }
    }

    fn add_stmt(&mut self, istmt: NodeId) {
        let ast = self.ast;
        match ast.node(istmt) {
            Node::BlockStmt(stmt) => {
                self.add_scope(stmt.scope);
                for &item in &stmt.list {
                    self.add_stmt(item);
                }
            }
            Node::FuncDecl(stmt) => {
                self.add_func_like(ast.block(stmt.body).scope, &stmt.params, stmt.body)
            }
            Node::ClassDecl(stmt) => {
                for item in &stmt.list {
                    if item.method.is_some() {
                        let m = ast.method_decl(item.method);
                        self.add_scope(ast.block(m.body).scope);
                    }
                }
            }
            Node::ReturnStmt(stmt) => self.add_expr(stmt.value),
            Node::ThrowStmt(stmt) => self.add_expr(stmt.value),
            Node::ForStmt(stmt) => self.add_stmt(stmt.body),
            Node::ForInStmt(stmt) => self.add_stmt(stmt.body),
            Node::ForOfStmt(stmt) => self.add_stmt(stmt.body),
            Node::IfStmt(stmt) => {
                self.add_stmt(stmt.body);
                if stmt.else_.is_some() {
                    self.add_stmt(stmt.else_);
                }
            }
            Node::TryStmt(stmt) => {
                if !ast.block(stmt.body).list.is_empty() {
                    self.add_stmt(stmt.body);
                }
                if stmt.catch.is_some() {
                    self.add_stmt(stmt.catch);
                }
                if stmt.finally.is_some() {
                    self.add_stmt(stmt.finally);
                }
            }
            Node::VarDecl(stmt) => {
                for item in &stmt.list {
                    if item.default.is_some() {
                        self.add_expr(item.default);
                    }
                }
            }
            Node::ExprStmt(stmt) => self.add_expr(stmt.value),
            _ => {}
        }
    }
}

// Go: parse_test.go:TestParseScope
#[test]
fn test_parse_scope() {
    for (js, bound, uses) in PARSE_SCOPE_TESTS {
        let ast = parse_str(js).unwrap_or_else(|e| panic!("{:?}: {}", s(js), e));
        let mut vars = ScopeVars::new(&ast);
        vars.add_scope(ast.module_scope());
        for &istmt in ast.list() {
            vars.add_stmt(istmt);
        }
        assert_eq!(
            vars.string(),
            format!("bound:{} uses:{}", s(bound), s(uses)),
            "{:?}",
            s(js)
        );
    }
}

// Go: parse_test.go:TestScope
#[test]
fn test_scope() {
    let js = b"let a,b; b = 5; var c; {d}{{d}}";
    let mut ast = parse_str(js).unwrap();
    let scope = ast.module_scope();

    // test output
    assert_eq!(
        s(&ast.scope_string(scope)),
        "Scope{Declared: [Var{LexicalDecl a 0 1}, Var{LexicalDecl b 0 2}, Var{VariableDecl c 0 1}], Undeclared: [Var{NoDecl d 0 2}]}"
    );

    // test sort
    ast.sort_vars_by_uses(scope, 0);
    assert_eq!(
        s(&ast.scope_string(scope)),
        "Scope{Declared: [Var{LexicalDecl b 0 2}, Var{LexicalDecl a 0 1}, Var{VariableDecl c 0 1}], Undeclared: [Var{NoDecl d 0 2}]}"
    );

    // test variable link
    let list = ast.list().clone();
    let b3 = ast.block(list[3]).scope;
    let b4 = ast.block(list[4]).scope;
    let b40 = ast.block(ast.block(list[4]).list[0]).scope;
    assert_eq!(
        s(&ast.scope_string(b3)),
        "Scope{Declared: [], Undeclared: [Var{NoDecl d 0 2}]}"
    );
    assert_eq!(
        s(&ast.scope_string(b4)),
        "Scope{Declared: [], Undeclared: [Var{NoDecl d 0 2}]}"
    );
    assert_eq!(
        s(&ast.scope_string(b40)),
        "Scope{Declared: [], Undeclared: [Var{NoDecl d 1 2}]}"
    );
}

// Go: tdewolff/test.NewErrorReader
struct ErrorReader(usize);

impl GoReader for ErrorReader {
    fn read(&mut self, b: &mut [u8]) -> (usize, Option<GoError>) {
        if b.is_empty() {
            return (0, None);
        }
        if self.0 == 0 {
            return (0, Some(GoError::Other(b"error".to_vec())));
        }
        self.0 -= 1;
        b[0] = b'.';
        (1, None)
    }
}

// Go: parse_test.go:TestParseInputError
#[test]
fn test_parse_input_error() {
    let err = parse(&Input::new(Some(&mut ErrorReader(0))), Options::default()).unwrap_err();
    assert_eq!(err, GoError::Other(b"error".to_vec()));

    let err = parse(&Input::new(Some(&mut ErrorReader(1))), Options::default()).unwrap_err();
    assert_eq!(err, GoError::Other(b"error".to_vec()));
}

// Go: util_test.go:TestAsIdentifierName
#[test]
fn test_as_identifier_name() {
    assert!(!as_identifier_name(b"".as_slice()));
    assert!(!as_identifier_name(b"5".as_slice()));
    assert!(as_identifier_name(b"ab".as_slice()));
    assert!(!as_identifier_name(b"a=".as_slice()));
}

// Go: util_test.go:TestAsDecimalLiteral
#[test]
fn test_as_decimal_literal() {
    assert!(!as_decimal_literal(b"".as_slice()));
    assert!(!as_decimal_literal(b"a".as_slice()));
    assert!(as_decimal_literal(b"12".as_slice()));
    assert!(as_decimal_literal(b"12.56".as_slice()));
    assert!(as_decimal_literal(b".56".as_slice()));
    assert!(!as_decimal_literal(b".56a".as_slice()));
    assert!(!as_decimal_literal(b".".as_slice()));
    assert!(as_decimal_literal(b"0".as_slice()));
    assert!(!as_decimal_literal(b"00".as_slice()));
}

/// The nesting limits produce Go's errors (and do not overflow the stack
/// on a large-stack thread).
#[test]
fn nesting_limits() {
    common::big_stack(|| {
        let deep_expr = format!("{}a{}", "(".repeat(1200), ")".repeat(1200));
        let err = parse_str(deep_expr.as_bytes()).unwrap_err();
        assert!(
            s(&err.error_bytes()).starts_with("too many nested expressions"),
            "{}",
            err
        );
        let deep_stmt = format!("{}{}", "{".repeat(1200), "}".repeat(1200));
        let err = parse_str(deep_stmt.as_bytes()).unwrap_err();
        assert!(
            s(&err.error_bytes()).starts_with("too many nested statements"),
            "{}",
            err
        );
        let ok_expr = format!("{}a{}", "(".repeat(900), ")".repeat(900));
        assert!(parse_str(ok_expr.as_bytes()).is_ok());
    });
}

/// Go `regexp.MustCompile("\n *").ReplaceAllString(src, " ")`
fn collapse_newline_indent(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if src[i] == b'\n' {
            i += 1;
            while i < src.len() && src[i] == b' ' {
                i += 1;
            }
            out.push(b' ');
        } else {
            out.push(src[i]);
            i += 1;
        }
    }
    out
}

// Go: ast_test.go:TestJS
#[test]
fn test_js() {
    common::big_stack(|| {
        for (js, expected) in JS_TESTS {
            let ast = parse_str(js).unwrap_or_else(|e| panic!("{:?}: {}", s(js), e));
            let src = collapse_newline_indent(&ast.js_string());
            assert_eq!(s(&src), s(expected), "{:?}", s(js));
        }
    });
}

// Go: ast_test.go:TestJSON
#[test]
fn test_json() {
    let input =
        br#"[{"key": [2.5, '\r'], '"': -2E+9}, null, false, true, 5.0e-6, "string", 'stri"ng']"#;
    let ast = parse_str(input).unwrap();
    let (json, err) = ast.json_string();
    assert!(err.is_ok(), "{:?}", err.map_err(|e| s(&e)));
    assert_eq!(
        s(&json),
        r#"[{"key": [2.5, "\r"], "\"": -2E+9}, null, false, true, 5.0e-6, "string", "stri\"ng"]"#
    );
}
