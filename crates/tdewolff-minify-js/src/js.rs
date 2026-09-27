//! Go: minify/v2@v2.23.8/js/js.go — `Minify`, the `jsMinifier` printer and
//! all its methods (statements, declarations, bindings and expressions).
//!
//! The AST is the arena of `tdewolff-parse-js`: Go pointers are [`NodeId`]s,
//! and the in-place mutations Go performs while printing (`stmt.Body.List =
//! optimizeStmtList(...)`, `expr.Op = js.EqEqToken`, `lit.Data =
//! lit.Data[:len-1]`, the renamer writing into `Var.Data`, ...) are applied
//! to the arena at the same points. Lists Go ranges over are copied at the
//! start of the loop (Go's `range` evaluates the slice header once; no list
//! is modified in place while it is being printed).

use tdewolff_minify::{GoBytes, GoError, GoReader, M, Params, Writer, number, param_is};
use tdewolff_parse::Input;
use tdewolff_parse_js::*;

use crate::Minifier;
use crate::stmtlist::optimize_stmt_list;
use crate::util::*;
use crate::vars::Renamer;

// Go: js.go:blockType
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BlockType {
    Default,
    Function,
    Iteration,
}

// Go: js.go:expectExpr
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ExpectExpr {
    Any,
    /// in statement
    ExprStmt,
    /// in arrow function body
    ExprBody,
}

// Go: js.go:Minifier.Minify
/// Minifies JS data, it reads from `r` and writes to `w`.
///
/// Like Go, the input buffer is used in place (when `r` exposes `Bytes()`):
/// the NUL sentinel written past its end by `parse.NewInput` is not restored,
/// and the renamer and the literal minifiers rewrite bytes inside it.
pub(crate) fn minify(
    o: &Minifier,
    _m: &M,
    w: &mut dyn Writer,
    r: &mut dyn GoReader,
    params: Option<&Params>,
) -> Result<(), GoError> {
    let z = Input::new(Some(r));
    let ast = parse(
        &z,
        Options {
            while_to_for: true,
            inline: param_is(params, b"inline", b"1"),
        },
    )?;

    let mut m = JsMinifier {
        o,
        w,
        ast,
        prev: GoBytes::nil(),
        needs_semicolon: false,
        needs_space: false,
        expect_expr: ExpectExpr::Any,
        grouped_stmt: false,
        in_for: false,
        space_before: 0,
        renamer: Renamer::new(!o.keep_var_names, !o.use_alphabet_var_names),
    };
    let body = m.ast.block_stmt;
    m.hoist_vars(body);
    m.optimize_block_list(body, BlockType::Function);
    let list = m.ast.block(body).list.clone();
    for item in list {
        m.write_semicolon();
        m.minify_stmt(item);
    }

    m.w.write(&[])?;
    Ok(())
}

// Go: js.go:jsMinifier
pub(crate) struct JsMinifier<'a> {
    pub(crate) o: &'a Minifier,
    w: &'a mut dyn Writer,
    pub(crate) ast: Ast,

    prev: GoBytes,
    /// write a semicolon if required
    needs_semicolon: bool,
    /// write a space if next token is an identifier
    needs_space: bool,
    /// avoid ambiguous syntax such as an expression starting with function
    expect_expr: ExpectExpr,
    /// avoid ambiguous syntax by grouping the expression statement
    grouped_stmt: bool,
    in_for: bool,
    space_before: u8,

    pub(crate) renamer: Renamer,
}

/// Go `tt.Bytes()` as a slice (`nil` for an unknown token type).
fn tt_bytes(tt: TokenType) -> GoBytes {
    match tt.bytes() {
        Some(b) => GoBytes::from_static(b),
        None => GoBytes::nil(),
    }
}

/// Go `js.IsIdentifierContinue(b)`: only the first rune is decoded, so the
/// first `utf8.UTFMax` bytes decide.
fn is_ident_continue_go(b: &GoBytes) -> bool {
    let n = b.len().min(4);
    let mut a = [0u8; 4];
    for (k, c) in a.iter_mut().enumerate().take(n) {
        *c = b.at(k);
    }
    is_identifier_continue(&a[..n])
}

/// Go `js.IsIdentifierEnd(b)`: `utf8.DecodeLastRune` looks at no more than
/// the last `utf8.UTFMax` bytes (and returns `RuneError` for the same inputs
/// on the suffix as on the whole slice).
fn is_ident_end_go(b: &GoBytes) -> bool {
    let n = b.len().min(4);
    let off = b.len() - n;
    let mut a = [0u8; 4];
    for (k, c) in a.iter_mut().enumerate().take(n) {
        *c = b.at(off + k);
    }
    is_identifier_end(&a[..n])
}

impl JsMinifier<'_> {
    // Go: js.go:jsMinifier.write
    fn write(&mut self, b: GoBytes) {
        // 0 < len(b)
        if self.needs_space && is_ident_continue_go(&b) || self.space_before == b.at(0) {
            let _ = self.w.write(SPACE_BYTES);
        }
        let _ = self.w.write_go(&b);
        self.prev = b;
        self.needs_space = false;
        self.expect_expr = ExpectExpr::Any;
        self.space_before = 0;
    }

    /// `m.write(xxxBytes)` of a package-level constant.
    #[inline]
    fn write_s(&mut self, b: &'static [u8]) {
        self.write(GoBytes::from_static(b));
    }

    // Go: js.go:jsMinifier.writeSpaceAfterIdent
    fn write_space_after_ident(&mut self) {
        // space after identifier and after regular expression (to prevent confusion with its tag)
        if is_ident_end_go(&self.prev) || 1 < self.prev.len() && self.prev.at(0) == b'/' {
            let _ = self.w.write(SPACE_BYTES);
        }
    }

    // Go: js.go:jsMinifier.writeSpaceBeforeIdent
    fn write_space_before_ident(&mut self) {
        self.needs_space = true;
    }

    // Go: js.go:jsMinifier.writeSpaceBefore
    fn write_space_before(&mut self, c: u8) {
        self.space_before = c;
    }

    // Go: js.go:jsMinifier.requireSemicolon
    fn require_semicolon(&mut self) {
        self.needs_semicolon = true;
    }

    // Go: js.go:jsMinifier.writeSemicolon
    fn write_semicolon(&mut self) {
        if self.needs_semicolon {
            let _ = self.w.write(SEMICOLON_BYTES);
            self.needs_semicolon = false;
            self.needs_space = false;
        }
    }

    /// `block.List = optimizeStmtList(block.List, blockType)`
    pub(crate) fn optimize_block_list(&mut self, block: NodeId, block_type: BlockType) {
        let list = std::mem::take(&mut self.ast.block_mut(block).list);
        let list = optimize_stmt_list(&mut self.ast, list, block_type);
        self.ast.block_mut(block).list = list;
    }

    /// `m.renamer.renameScope(scope)`
    fn rename_scope(&mut self, scope: ScopeId) {
        self.renamer.rename_scope(&mut self.ast, scope);
    }

    // Go: js.go:jsMinifier.minifyStmt
    pub(crate) fn minify_stmt(&mut self, i: NodeId) {
        match self.ast.node(i) {
            Node::ExprStmt(stmt) => {
                let value = stmt.value;
                self.expect_expr = ExpectExpr::ExprStmt;
                self.minify_expr(value, OpExpr);
                if self.grouped_stmt {
                    self.write_s(CLOSE_PAREN_BYTES);
                    self.grouped_stmt = false;
                }
                self.require_semicolon();
            }
            Node::VarDecl(_) => {
                self.minify_var_decl(i, false);
                self.require_semicolon();
            }
            Node::IfStmt(stmt) => {
                let (body, else_) = (stmt.body, stmt.else_);
                let has_if = !is_empty_stmt(&self.ast, body);
                let has_else = !is_empty_stmt(&self.ast, else_);
                if !has_if && !has_else {
                    return;
                }

                self.write_s(IF_OPEN_BYTES);
                let cond = if_mut(&mut self.ast, i).cond;
                self.minify_expr(cond, OpExpr);
                self.write_s(CLOSE_PAREN_BYTES);

                if !has_if && has_else {
                    self.require_semicolon();
                } else if has_if {
                    let body = if_mut(&mut self.ast, i).body;
                    if has_else && ends_in_if(&mut self.ast, body) {
                        // prevent: if(a){if(b)c}else d;  =>  if(a)if(b)c;else d;
                        self.write_s(OPEN_BRACE_BYTES);
                        let body = if_mut(&mut self.ast, i).body;
                        self.minify_stmt(body);
                        self.write_s(CLOSE_BRACE_BYTES);
                        self.needs_semicolon = false;
                    } else {
                        let body = if_mut(&mut self.ast, i).body;
                        self.minify_stmt(body);
                    }
                }
                if has_else {
                    self.write_semicolon();
                    self.write_s(ELSE_BYTES);
                    self.write_space_before_ident();
                    let else_ = if_mut(&mut self.ast, i).else_;
                    self.minify_stmt(else_);
                }
            }
            Node::BlockStmt(stmt) => {
                let scope = stmt.scope;
                self.rename_scope(scope);
                self.minify_block_stmt(i);
            }
            Node::ReturnStmt(stmt) => {
                let value = stmt.value;
                self.write_s(RETURN_BYTES);
                self.write_space_before_ident();
                self.minify_expr(value, OpExpr);
                self.require_semicolon();
            }
            Node::LabelledStmt(stmt) => {
                let (label, value) = (stmt.label.clone(), stmt.value);
                self.write(label);
                self.write_s(COLON_BYTES);
                self.minify_stmt_or_block(value, BlockType::Default);
            }
            Node::BranchStmt(stmt) => {
                let (type_, label) = (stmt.type_, stmt.label.clone());
                self.write(tt_bytes(type_));
                if !label.is_nil() {
                    self.write_s(SPACE_BYTES);
                    self.write(label);
                }
                self.require_semicolon();
            }
            Node::WithStmt(stmt) => {
                let (cond, body) = (stmt.cond, stmt.body);
                self.write_s(WITH_OPEN_BYTES);
                self.minify_expr(cond, OpExpr);
                self.write_s(CLOSE_PAREN_BYTES);
                self.minify_stmt_or_block(body, BlockType::Default);
            }
            Node::DoWhileStmt(stmt) => {
                let (cond, body) = (stmt.cond, stmt.body);
                self.write_s(DO_BYTES);
                self.write_space_before_ident();
                self.minify_stmt_or_block(body, BlockType::Iteration);
                self.write_semicolon();
                self.write_s(WHILE_OPEN_BYTES);
                self.minify_expr(cond, OpExpr);
                self.write_s(CLOSE_PAREN_BYTES);
            }
            Node::WhileStmt(stmt) => {
                let (cond, body) = (stmt.cond, stmt.body);
                self.write_s(WHILE_OPEN_BYTES);
                self.minify_expr(cond, OpExpr);
                self.write_s(CLOSE_PAREN_BYTES);
                self.minify_stmt_or_block(body, BlockType::Iteration);
            }
            Node::ForStmt(stmt) => {
                let body = stmt.body;
                self.optimize_block_list(body, BlockType::Iteration);
                let scope = self.ast.block(body).scope;
                self.rename_scope(scope);
                self.write_s(FOR_OPEN_BYTES);
                self.in_for = true;
                let (init, cond, post) = match self.ast.node(i) {
                    Node::ForStmt(s) => (s.init, s.cond, s.post),
                    _ => unreachable!(),
                };
                if let Node::VarDecl(_) = self.ast.node(init) {
                    self.minify_var_decl(init, true);
                } else {
                    self.minify_expr(init, OpLHS);
                }
                self.in_for = false;
                self.write_s(SEMICOLON_BYTES);
                self.minify_expr(cond, OpExpr);
                self.write_s(SEMICOLON_BYTES);
                self.minify_expr(post, OpExpr);
                self.write_s(CLOSE_PAREN_BYTES);
                self.minify_block_as_stmt(body);
            }
            Node::ForInStmt(stmt) => {
                let (init, value, body) = (stmt.init, stmt.value, stmt.body);
                self.optimize_block_list(body, BlockType::Iteration);
                let scope = self.ast.block(body).scope;
                self.rename_scope(scope);
                self.write_s(FOR_OPEN_BYTES);
                self.in_for = true;
                if let Node::VarDecl(_) = self.ast.node(init) {
                    self.minify_var_decl(init, false);
                } else {
                    self.minify_expr(init, OpLHS);
                }
                self.in_for = false;
                self.write_space_after_ident();
                self.write_s(IN_BYTES);
                self.write_space_before_ident();
                self.minify_expr(value, OpExpr);
                self.write_s(CLOSE_PAREN_BYTES);
                self.minify_block_as_stmt(body);
            }
            Node::ForOfStmt(stmt) => {
                let (await_, init, value, body) = (stmt.await_, stmt.init, stmt.value, stmt.body);
                self.optimize_block_list(body, BlockType::Iteration);
                let scope = self.ast.block(body).scope;
                self.rename_scope(scope);
                if await_ {
                    self.write_s(FOR_AWAIT_OPEN_BYTES);
                } else {
                    self.write_s(FOR_OPEN_BYTES);
                }
                self.in_for = true;
                if let Node::VarDecl(_) = self.ast.node(init) {
                    self.minify_var_decl(init, false);
                } else {
                    self.minify_expr(init, OpLHS);
                }
                self.in_for = false;
                self.write_space_after_ident();
                self.write_s(OF_BYTES);
                self.write_space_before_ident();
                self.minify_expr(value, OpAssign);
                self.write_s(CLOSE_PAREN_BYTES);
                self.minify_block_as_stmt(body);
            }
            Node::SwitchStmt(stmt) => {
                let init = stmt.init;
                self.write_s(SWITCH_OPEN_BYTES);
                self.minify_expr(init, OpExpr);
                self.write_s(CLOSE_PAREN_OPEN_BRACKET_BYTES);
                self.needs_semicolon = false;
                let n = self.ast.switch_stmt(i).list.len();
                for k in 0..n {
                    let list = std::mem::take(&mut self.ast.switch_stmt_mut(i).list[k].list);
                    let list = optimize_stmt_list(&mut self.ast, list, BlockType::Default);
                    self.ast.switch_stmt_mut(i).list[k].list = list;
                }
                let scope = self.ast.switch_stmt(i).scope;
                self.rename_scope(scope);
                let clauses = self.ast.switch_stmt(i).list.clone();
                for clause in clauses {
                    self.write_semicolon();
                    self.write(tt_bytes(clause.token_type));
                    if clause.cond.is_some() {
                        self.write_space_before_ident();
                        self.minify_expr(clause.cond, OpExpr);
                    }
                    self.write_s(COLON_BYTES);
                    for item in clause.list {
                        self.write_semicolon();
                        self.minify_stmt(item);
                    }
                }
                self.write_s(CLOSE_BRACE_BYTES);
                self.needs_semicolon = false;
            }
            Node::ThrowStmt(stmt) => {
                let value = stmt.value;
                self.write_s(THROW_BYTES);
                self.write_space_before_ident();
                self.minify_expr(value, OpExpr);
                self.require_semicolon();
            }
            Node::TryStmt(stmt) => {
                let (body, catch, finally) = (stmt.body, stmt.catch, stmt.finally);
                self.write_s(TRY_BYTES);
                self.optimize_block_list(body, BlockType::Default);
                let scope = self.ast.block(body).scope;
                self.rename_scope(scope);
                self.minify_block_stmt(body);
                if catch.is_some() {
                    self.write_s(CATCH_BYTES);
                    self.optimize_block_list(catch, BlockType::Default);
                    let binding = try_mut(&mut self.ast, i).binding;
                    if let Some(v) = self.ast.as_var(binding) {
                        if v.uses == 1 && self.o.min_version(2019) {
                            let scope = self.ast.block(catch).scope;
                            let declared = &mut self.ast.scope_mut(scope).declared;
                            // Go: Declared = Declared[1:] (panics when empty)
                            assert!(!declared.is_empty(), "slice bounds out of range [1:0]");
                            declared.remove(0);
                            try_mut(&mut self.ast, i).binding = NodeId::NIL;
                        }
                    }
                    let scope = self.ast.block(catch).scope;
                    self.rename_scope(scope);
                    let binding = try_mut(&mut self.ast, i).binding;
                    if binding.is_some() {
                        self.write_s(OPEN_PAREN_BYTES);
                        self.minify_binding(binding);
                        self.write_s(CLOSE_PAREN_BYTES);
                    }
                    self.minify_block_stmt(catch);
                }
                if finally.is_some() {
                    self.write_s(FINALLY_BYTES);
                    self.optimize_block_list(finally, BlockType::Default);
                    let scope = self.ast.block(finally).scope;
                    self.rename_scope(scope);
                    self.minify_block_stmt(finally);
                }
            }
            Node::FuncDecl(_) => {
                self.minify_func_decl(i, false);
            }
            Node::ClassDecl(_) => {
                self.minify_class_decl(i);
            }
            Node::DebuggerStmt => {
                self.write_s(DEBUGGER_BYTES);
                self.require_semicolon();
            }
            Node::EmptyStmt => {}
            Node::ImportStmt(stmt) => {
                let stmt = stmt.clone();
                let list_len = stmt.list.as_ref().map_or(0, |l| l.len());
                if !stmt.default.is_nil() || stmt.list.is_none() || 0 < list_len {
                    self.write_s(IMPORT_BYTES);
                    if !stmt.default.is_nil() {
                        self.write_s(SPACE_BYTES);
                        self.write(stmt.default.clone());
                        if stmt.list.is_some() {
                            self.write_s(COMMA_BYTES);
                        } else if !stmt.default.is_nil() {
                            self.write_s(SPACE_BYTES);
                        }
                    }
                    let list: &[Alias] = stmt.list.as_deref().unwrap_or(&[]);
                    if list.len() == 1 && list[0].name.len() == 1 && list[0].name.at(0) == b'*' {
                        self.write_space_before_ident();
                        self.minify_alias(&list[0]);
                        if !stmt.default.is_nil() || !list.is_empty() {
                            self.write_s(SPACE_BYTES);
                        }
                    } else if stmt.list.is_some() {
                        self.write_s(OPEN_BRACE_BYTES);
                        for (k, item) in list.iter().enumerate() {
                            if k != 0 {
                                self.write_s(COMMA_BYTES);
                            }
                            self.minify_alias(item);
                        }
                        self.write_s(CLOSE_BRACE_BYTES);
                    }
                    if !stmt.default.is_nil() || stmt.list.is_some() {
                        self.write_s(FROM_BYTES);
                    }
                    self.write(minify_string(stmt.module.clone(), false));
                    self.require_semicolon();
                }
            }
            Node::ExportStmt(stmt) => {
                let stmt = stmt.clone();
                self.write_s(EXPORT_BYTES);
                if stmt.decl.is_some() {
                    if stmt.default {
                        self.write_s(SPACE_DEFAULT_BYTES);
                        self.write_space_before_ident();
                        self.minify_expr(stmt.decl, OpAssign);
                        let is_hoistable = matches!(self.ast.node(stmt.decl), Node::FuncDecl(_));
                        let is_class = matches!(self.ast.node(stmt.decl), Node::ClassDecl(_));
                        if !is_hoistable && !is_class {
                            self.require_semicolon();
                        }
                    } else {
                        self.write_space_before_ident();
                        self.minify_stmt(stmt.decl); // can only be variable, function, or class decl
                    }
                } else {
                    let list = &stmt.list;
                    if list.len() == 1
                        && (list[0].name.len() == 1 && list[0].name.at(0) == b'*'
                            || list[0].name.is_nil()
                                && list[0].binding.len() == 1
                                && list[0].binding.at(0) == b'*')
                    {
                        self.write_space_before_ident();
                        self.minify_alias(&list[0]);
                        if !stmt.module.is_nil() && !list[0].name.is_nil() {
                            self.write_s(SPACE_BYTES);
                        }
                    } else if 0 < list.len() {
                        self.write_s(OPEN_BRACE_BYTES);
                        for (k, item) in list.iter().enumerate() {
                            if k != 0 {
                                self.write_s(COMMA_BYTES);
                            }
                            self.minify_alias(item);
                        }
                        self.write_s(CLOSE_BRACE_BYTES);
                    }
                    if !stmt.module.is_nil() {
                        self.write_s(FROM_BYTES);
                        self.write(minify_string(stmt.module.clone(), false));
                    }
                    self.require_semicolon();
                }
            }
            Node::DirectivePrologueStmt(stmt) => {
                let value = stmt.value.clone();
                value.set(0, b'"');
                value.set(value.len() - 1, b'"');
                self.write(value);
                self.require_semicolon();
            }
            Node::Comment(stmt) => {
                // bang comment
                let value = stmt.value.clone();
                self.write(value.clone());
                if value.at(1) == b'/' {
                    self.write_s(NEWLINE_BYTES);
                }
            }
            _ => {}
        }
    }

    // Go: js.go:jsMinifier.minifyBlockStmt
    fn minify_block_stmt(&mut self, stmt: NodeId) {
        self.write_s(OPEN_BRACE_BYTES);
        self.needs_semicolon = false;
        let list = self.ast.block(stmt).list.clone();
        for item in list {
            self.write_semicolon();
            self.minify_stmt(item);
        }
        self.write_s(CLOSE_BRACE_BYTES);
        self.needs_semicolon = false;
    }

    // Go: js.go:jsMinifier.minifyBlockAsStmt
    /// Minifies a block when a statement is expected, i.e. a semicolon if
    /// empty or without braces for a single statement. Assumes the scope was
    /// already renamed.
    fn minify_block_as_stmt(&mut self, block_stmt: NodeId) {
        let scope = self.ast.block(block_stmt).scope;
        let mut has_lexical_vars = false;
        {
            let sc = self.ast.scope(scope);
            let from = sc.num_for_decls as usize;
            assert!(
                from <= sc.declared.len(),
                "slice bounds out of range [{}:{}]",
                from,
                sc.declared.len()
            );
            for &v in &sc.declared[from..] {
                if self.ast.var(v).decl == LexicalDecl {
                    has_lexical_vars = true;
                    break;
                }
            }
        }
        let n = self.ast.block(block_stmt).list.len();
        if 1 < n || has_lexical_vars {
            self.minify_block_stmt(block_stmt);
        } else if n == 1 {
            let first = self.ast.block(block_stmt).list[0];
            self.minify_stmt(first);
        } else {
            self.write_s(SEMICOLON_BYTES);
            self.needs_semicolon = false;
        }
    }

    // Go: js.go:jsMinifier.minifyStmtOrBlock
    fn minify_stmt_or_block(&mut self, i: NodeId, block_type: BlockType) {
        // minify stmt or a block
        if let Node::BlockStmt(_) = self.ast.node(i) {
            self.optimize_block_list(i, block_type);
            let scope = self.ast.block(i).scope;
            self.rename_scope(scope);
            self.minify_block_as_stmt(i);
        } else {
            // optimizeStmtList can in some cases expand one stmt to two shorter stmts
            let list = optimize_stmt_list(&mut self.ast, vec![i], block_type);
            if list.len() == 1 {
                self.minify_stmt(list[0]);
            } else if list.is_empty() {
                self.write_s(SEMICOLON_BYTES);
                self.needs_semicolon = false;
            } else {
                let block = self.ast.alloc_block(list);
                self.minify_block_stmt(block);
            }
        }
    }

    // Go: js.go:jsMinifier.minifyAlias
    fn minify_alias(&mut self, alias: &Alias) {
        if !alias.name.is_nil() {
            if alias.name.at(0) == b'"' || alias.name.at(0) == b'\'' {
                self.write(minify_string(alias.name.clone(), false));
            } else {
                self.write(alias.name.clone());
            }
            if !alias.name.equal(STAR_BYTES) {
                self.write_s(SPACE_BYTES);
            }
            self.write_s(AS_SPACE_BYTES);
        }
        if !alias.binding.is_nil() {
            if alias.binding.at(0) == b'"' || alias.binding.at(0) == b'\'' {
                self.write(minify_string(alias.binding.clone(), false));
            } else {
                self.write(alias.binding.clone());
            }
        }
    }

    // Go: js.go:jsMinifier.minifyParams
    fn minify_params(&mut self, params: &Params_, remove_unused: bool) {
        // remove unused parameters from the end
        let mut j = params.list.len();
        if remove_unused && params.rest.is_nil() {
            while 0 < j {
                match self.ast.as_var(params.list[j - 1].binding) {
                    Some(v) if v.uses <= 1 => {}
                    _ => break,
                }
                j -= 1;
            }
        }

        self.write_s(OPEN_PAREN_BYTES);
        for (i, &item) in params.list[..j].iter().enumerate() {
            if i != 0 {
                self.write_s(COMMA_BYTES);
            }
            self.minify_binding_element(item);
        }
        if params.rest.is_some() {
            if !params.list.is_empty() {
                self.write_s(COMMA_BYTES);
            }
            self.write_s(ELLIPSIS_BYTES);
            self.minify_binding(params.rest);
        }
        self.write_s(CLOSE_PAREN_BYTES);
    }

    // Go: js.go:jsMinifier.minifyArguments
    fn minify_arguments(&mut self, args: &Args) {
        self.write_s(OPEN_PAREN_BYTES);
        for (i, item) in args.list.iter().enumerate() {
            if i != 0 {
                self.write_s(COMMA_BYTES);
            }
            if item.rest {
                self.write_s(ELLIPSIS_BYTES);
            }
            self.minify_expr(item.value, OpAssign);
        }
        self.write_s(CLOSE_PAREN_BYTES);
    }

    // Go: js.go:jsMinifier.minifyVarDecl
    fn minify_var_decl(&mut self, decl: NodeId, only_defines: bool) {
        let (tt, n) = {
            let d = self.ast.var_decl(decl);
            (d.token_type, d.list.len())
        };
        if n == 0 {
            return;
        } else if tt == ErrorToken {
            // remove 'var' when hoisting variables
            let mut first = true;
            let list = self.ast.var_decl(decl).list.clone();
            for item in list {
                if item.default.is_some() || !only_defines {
                    if !first {
                        self.write_s(COMMA_BYTES);
                    }
                    self.minify_binding_element(item);
                    first = false;
                }
            }
        } else {
            if tt == VarToken {
                // Go: sort.SliceStable with a position-dependent less (`j != 0`),
                // so the exact insertion-sort/symMerge call sequence matters.
                let mut list = std::mem::take(&mut self.ast.var_decl_mut(decl).list);
                {
                    let ast = &self.ast;
                    let ident_order = &self.renamer.ident_order;
                    go_sort::sort::slice_stable(&mut list, |l: &[BindingElement], i, j| {
                        if l[i].default.is_nil() && l[j].default.is_nil() {
                            // sort single-length variables names
                            if let Some(a) = ast.as_var(l[i].binding) {
                                if a.data.len() == 1 {
                                    if let Some(b) = ast.as_var(l[j].binding) {
                                        if b.data.len() == 1 {
                                            return ident_order[a.data.at(0) as usize]
                                                < ident_order[b.data.at(0) as usize];
                                        }
                                    }
                                }
                            }
                        } else if l[i].default.is_nil() {
                            if j != 0 || ast.is_var(l[j].binding) {
                                // move non-define declarations to the front, except for the first array/object
                                return true;
                            }
                        }
                        false
                    });
                }
                self.ast.var_decl_mut(decl).list = list;
            }

            self.write(tt_bytes(tt));
            self.write_space_before_ident();
            let list = self.ast.var_decl(decl).list.clone();
            for (i, item) in list.into_iter().enumerate() {
                if i != 0 {
                    self.write_s(COMMA_BYTES);
                }
                self.minify_binding_element(item);
            }
        }
    }

    // Go: js.go:jsMinifier.minifyFuncDecl
    fn minify_func_decl(&mut self, decl: NodeId, in_expr: bool) {
        let parent_rename = self.renamer.rename;
        let body = self.ast.func_decl(decl).body;
        let body_scope = self.ast.block(body).scope;
        self.renamer.rename = !self.ast.scope(body_scope).has_with && !self.o.keep_var_names;
        self.hoist_vars(body);
        self.optimize_block_list(body, BlockType::Function);

        let (async_, generator) = {
            let d = self.ast.func_decl(decl);
            (d.async_, d.generator)
        };
        if async_ {
            self.write_s(ASYNC_SPACE_BYTES);
        }
        self.write_s(FUNCTION_BYTES);
        if generator {
            self.write_s(STAR_BYTES);
        }

        if in_expr {
            self.rename_scope(body_scope);
        }
        let name = self.ast.func_decl(decl).name;
        if name.is_some() && (!in_expr || 1 < self.ast.var(name).uses) {
            if !generator {
                self.write_s(SPACE_BYTES);
            }
            let data = self.ast.var(name).data.clone();
            self.write(data);
        }
        if !in_expr {
            self.rename_scope(body_scope);
        }

        let params = self.ast.func_decl(decl).params.clone();
        self.minify_params(&params, true);
        self.minify_block_stmt(body);
        self.renamer.rename = parent_rename;
    }

    // Go: js.go:jsMinifier.minifyMethodDecl
    fn minify_method_decl(&mut self, decl: NodeId) {
        let parent_rename = self.renamer.rename;
        let body = self.ast.method_decl(decl).body;
        let body_scope = self.ast.block(body).scope;
        self.renamer.rename = !self.ast.scope(body_scope).has_with && !self.o.keep_var_names;
        self.hoist_vars(body);
        self.optimize_block_list(body, BlockType::Function);

        let d = self.ast.method_decl(decl).clone();
        if d.static_ {
            self.write_s(STATIC_BYTES);
            self.write_space_before_ident();
        }
        if d.async_ {
            self.write_s(ASYNC_BYTES);
            if d.generator {
                self.write_s(STAR_BYTES);
            } else {
                self.write_space_before_ident();
            }
        } else if d.generator {
            self.write_s(STAR_BYTES);
        } else if d.get {
            self.write_s(GET_BYTES);
            self.write_space_before_ident();
        } else if d.set {
            self.write_s(SET_BYTES);
            self.write_space_before_ident();
        }
        self.minify_property_name(&d.name);
        self.rename_scope(body_scope);
        self.minify_params(&d.params, !d.set);
        self.minify_block_stmt(body);
        self.renamer.rename = parent_rename;
    }

    // Go: js.go:jsMinifier.minifyArrowFunc
    fn minify_arrow_func(&mut self, decl: NodeId) {
        let parent_rename = self.renamer.rename;
        let body = self.ast.arrow_func(decl).body;
        let body_scope = self.ast.block(body).scope;
        self.renamer.rename = !self.ast.scope(body_scope).has_with && !self.o.keep_var_names;
        self.hoist_vars(body);
        self.optimize_block_list(body, BlockType::Function);

        self.rename_scope(body_scope);
        let (async_, params) = {
            let d = self.ast.arrow_func(decl);
            (d.async_, d.params.clone())
        };
        if async_ {
            self.write_s(ASYNC_BYTES);
        }
        let mut remove_parens = false;
        if params.rest.is_nil() && params.list.len() == 1 && params.list[0].default.is_nil() {
            if params.list[0].binding.is_nil() || self.ast.is_var(params.list[0].binding) {
                remove_parens = true;
            }
        }
        if remove_parens {
            if async_ && params.list[0].binding.is_some() {
                // add space after async in: async a => ...
                self.write_s(SPACE_BYTES);
            }
            self.minify_binding_element(params.list[0]);
        } else {
            let parent_in_for = self.in_for;
            self.in_for = false;
            self.minify_params(&params, true);
            self.in_for = parent_in_for;
        }
        self.write_s(ARROW_BYTES);
        let mut remove_braces = false;
        let blist = self.ast.block(body).list.clone();
        if !blist.is_empty() {
            let last = blist[blist.len() - 1];
            let ret = match self.ast.node(last) {
                Node::ReturnStmt(r) => Some(r.value),
                _ => None,
            };
            if let Some(rv) = ret.filter(|v| v.is_some()) {
                // merge expression statements to final return statement, remove function body braces
                let mut list: Vec<NodeId> = Vec::new();
                remove_braces = true;
                for &item in &blist[..blist.len() - 1] {
                    if let Node::ExprStmt(e) = self.ast.node(item) {
                        list.push(e.value);
                    } else {
                        remove_braces = false;
                        break;
                    }
                }
                if remove_braces {
                    list.push(rv);
                    let mut expr = list[0];
                    if !list.is_empty() {
                        if 1 < list.len() {
                            expr = self.ast.alloc(Node::CommaExpr(CommaExpr { list }));
                        }
                        expr = new_group(&mut self.ast, expr);
                    }
                    self.expect_expr = ExpectExpr::ExprBody;
                    self.minify_expr(expr, OpAssign);
                    if self.grouped_stmt {
                        self.write_s(CLOSE_PAREN_BYTES);
                        self.grouped_stmt = false;
                    }
                }
            } else if ret.is_some() {
                // remove empty return
                self.ast.block_mut(body).list.pop();
            }
        }
        if !remove_braces {
            self.minify_block_stmt(body);
        }
        self.renamer.rename = parent_rename;
    }

    // Go: js.go:jsMinifier.minifyClassDecl
    fn minify_class_decl(&mut self, decl: NodeId) {
        let (name, extends) = {
            let d = self.ast.class_decl(decl);
            (d.name, d.extends)
        };
        self.write_s(CLASS_BYTES);
        if name.is_some() {
            self.write_s(SPACE_BYTES);
            let data = self.ast.var(name).data.clone();
            self.write(data);
        }
        if extends.is_some() {
            self.write_s(SPACE_EXTENDS_BYTES);
            self.write_space_before_ident();
            self.minify_expr(extends, OpLHS);
        }
        self.write_s(OPEN_BRACE_BYTES);
        self.needs_semicolon = false;
        let list = self.ast.class_decl(decl).list.clone();
        for item in list {
            self.write_semicolon();
            if item.static_block.is_some() {
                self.write_s(STATIC_BYTES);
                self.minify_block_stmt(item.static_block);
            } else if item.method.is_some() {
                self.minify_method_decl(item.method);
            } else {
                if item.field.static_ {
                    self.write_s(STATIC_BYTES);
                    if !item.field.name.is_computed()
                        && item.field.name.literal.token_type == IdentifierToken
                    {
                        self.write_s(SPACE_BYTES);
                    }
                }
                self.minify_property_name(&item.field.name);
                if item.field.init.is_some() {
                    self.write_s(EQUAL_BYTES);
                    self.minify_expr(item.field.init, OpAssign);
                }
                self.require_semicolon();
            }
        }
        self.write_s(CLOSE_BRACE_BYTES);
        self.needs_semicolon = false;
    }

    // Go: js.go:jsMinifier.minifyPropertyName
    fn minify_property_name(&mut self, name: &PropertyName) {
        if name.is_computed() {
            self.write_s(OPEN_BRACKET_BYTES);
            self.minify_expr(name.computed, OpAssign);
            self.write_s(CLOSE_BRACKET_BYTES);
        } else if name.literal.token_type == StringToken {
            self.write(minify_string(name.literal.data.clone(), false));
        } else {
            self.write(name.literal.data.clone());
        }
    }

    // Go: js.go:jsMinifier.minifyProperty
    fn minify_property(&mut self, property: &Property) {
        // property.Name is always set in ObjectLiteral
        if property.spread {
            self.write_s(ELLIPSIS_BYTES);
        } else if let Some(name) = &property.name {
            let is_ident = match self.ast.as_var(property.value) {
                Some(_) => name.is_ident(self.ast.var_name(property.value)),
                None => false,
            };
            if !is_ident {
                // add 'old-name:' before BindingName as the latter will be renamed
                self.minify_property_name(name);
                self.write_s(COLON_BYTES);
            }
        }
        self.minify_expr(property.value, OpAssign);
        if property.init.is_some() {
            self.write_s(EQUAL_BYTES);
            self.minify_expr(property.init, OpAssign);
        }
    }

    // Go: js.go:jsMinifier.minifyBindingElement
    fn minify_binding_element(&mut self, element: BindingElement) {
        if element.binding.is_some() {
            let parent_in_for = self.in_for;
            self.in_for = false;
            self.minify_binding(element.binding);
            self.in_for = parent_in_for;
            if element.default.is_some() {
                self.write_s(EQUAL_BYTES);
                self.minify_expr(element.default, OpAssign);
            }
        }
    }

    // Go: js.go:jsMinifier.minifyBinding
    fn minify_binding(&mut self, ibinding: NodeId) {
        match self.ast.node(ibinding) {
            Node::Var(binding) => {
                let data = binding.data.clone();
                self.write(data);
            }
            Node::BindingArray(binding) => {
                let (list, rest) = (binding.list.clone(), binding.rest);
                self.write_s(OPEN_BRACKET_BYTES);
                for (i, &item) in list.iter().enumerate() {
                    if i != 0 {
                        self.write_s(COMMA_BYTES);
                    }
                    self.minify_binding_element(item);
                }
                if rest.is_some() {
                    if !list.is_empty() {
                        self.write_s(COMMA_BYTES);
                    }
                    self.write_s(ELLIPSIS_BYTES);
                    self.minify_binding(rest);
                }
                self.write_s(CLOSE_BRACKET_BYTES);
            }
            Node::BindingObject(binding) => {
                let (list, rest) = (binding.list.clone(), binding.rest);
                self.write_s(OPEN_BRACE_BYTES);
                for (i, item) in list.iter().enumerate() {
                    if i != 0 {
                        self.write_s(COMMA_BYTES);
                    }
                    // item.Key is always set
                    let key = item
                        .key
                        .as_ref()
                        .expect("invalid memory address or nil pointer dereference");
                    if key.is_computed() {
                        self.minify_property_name(key);
                        self.write_s(COLON_BYTES);
                    } else {
                        let is_ident = match self.ast.as_var(item.value.binding) {
                            Some(v) => key.is_ident(&v.data),
                            None => false,
                        };
                        if !is_ident {
                            // add 'old-name:' before BindingName as the latter will be renamed
                            self.minify_property_name(key);
                            self.write_s(COLON_BYTES);
                        }
                    }
                    self.minify_binding_element(item.value);
                }
                if rest.is_some() {
                    if !list.is_empty() {
                        self.write_s(COMMA_BYTES);
                    }
                    self.write_s(ELLIPSIS_BYTES);
                    let data = self.ast.var(rest).data.clone();
                    self.write(data);
                }
                self.write_s(CLOSE_BRACE_BYTES);
            }
            _ => {}
        }
    }

    // Go: js.go:jsMinifier.minifyExpr
    pub(crate) fn minify_expr(&mut self, i: NodeId, prec: OpPrec) {
        let i = match self.ast.node(i) {
            Node::CondExpr(_) => {
                optimize_cond_expr(&mut self.ast, i, prec, self.o.min_version(2020))
            }
            Node::UnaryExpr(_) => optimize_unary_expr(&mut self.ast, i, prec),
            _ => i,
        };

        match self.ast.node(i) {
            Node::Var(_) => {
                let mut expr = i;
                while self.ast.var(expr).link.is_some() {
                    expr = self.ast.var(expr).link;
                }
                let (data, decl) = {
                    let v = self.ast.var(expr);
                    (v.data.clone(), v.decl)
                };
                if decl == NoDecl && data.equal(UNDEFINED_BYTES) {
                    if OpMember < prec {
                        self.write_s(GROUPED_ZERO_INDEX_BYTES);
                    } else {
                        self.write_s(ZERO_INDEX_BYTES);
                    }
                } else if decl == NoDecl && data.equal(INFINITY_BYTES) {
                    if OpMul < prec {
                        self.write_s(GROUPED_ONE_DIV_ZERO_BYTES);
                    } else {
                        self.write_s(ONE_DIV_ZERO_BYTES);
                    }
                } else {
                    self.write(data);
                }
            }
            Node::LiteralExpr(expr) => {
                let (tt, data) = (expr.token_type, expr.data.clone());
                if tt == DecimalToken || tt == IntegerToken {
                    self.write(decimal_number(data, self.o.precision));
                } else if tt == BinaryToken {
                    self.write(binary_number(data, self.o.precision));
                } else if tt == OctalToken {
                    self.write(octal_number(data, self.o.precision));
                } else if tt == HexadecimalToken {
                    self.write(hexadecimal_number(data, self.o.precision));
                } else if tt == TrueToken {
                    if OpUnary < prec {
                        self.write_s(GROUPED_NOT_ZERO_BYTES);
                    } else {
                        self.write_s(NOT_ZERO_BYTES);
                    }
                } else if tt == FalseToken {
                    if OpUnary < prec {
                        self.write_s(GROUPED_NOT_ONE_BYTES);
                    } else {
                        self.write_s(NOT_ONE_BYTES);
                    }
                } else if tt == StringToken {
                    self.write(minify_string(data, self.o.min_version(2015)));
                } else if tt == RegExpToken {
                    // </script>/ => < /script>/
                    if 0 < self.prev.len()
                        && self.prev.at(self.prev.len() - 1) == b'<'
                        && data.has_prefix(REG_EXP_SCRIPT_BYTES)
                    {
                        self.write_s(SPACE_BYTES);
                    }
                    self.write(minify_reg_exp(data));
                } else {
                    self.write(data);
                }
            }
            Node::BinaryExpr(_) => self.minify_binary_expr(i, prec),
            Node::UnaryExpr(expr) => {
                let (op, x) = (expr.op, expr.x);
                if op == PostIncrToken || op == PostDecrToken {
                    self.minify_expr(x, unary_prec_map(op));
                    self.write(tt_bytes(op));
                } else if op == VoidToken && !has_side_effects(&self.ast, x) {
                    self.write_s(ZERO_INDEX_BYTES);
                } else {
                    let is_lt_not = op == NotToken
                        && 0 < self.prev.len()
                        && self.prev.at(self.prev.len() - 1) == b'<';
                    self.write(tt_bytes(op));
                    if op == DeleteToken || op == VoidToken || op == TypeofToken || op == AwaitToken
                    {
                        self.write_space_before_ident();
                    } else if op == PosToken {
                        // +++  =>  + ++
                        self.write_space_before(b'+');
                    } else if op == NegToken || is_lt_not {
                        // ---  =>  - --
                        // <!--  =>  <! --
                        self.write_space_before(b'-');
                    } else if op == NotToken {
                        if let Some(lit) = as_literal(&self.ast, x) {
                            let ltt = lit.token_type;
                            if ltt == StringToken {
                                if lit.data.len() == 2 {
                                    // !""  =>  !0
                                    self.write_s(ZERO_BYTES);
                                } else {
                                    // !"string"  =>  !1
                                    self.write_s(ONE_BYTES);
                                }
                                return;
                            } else if ltt == RegExpToken {
                                // !/regexp/  =>  !1
                                self.write_s(ONE_BYTES);
                                return;
                            } else if ltt == DecimalToken || ltt == IntegerToken {
                                // !123  =>  !1 (except for !0)
                                let mut data = lit.data.clone();
                                if data.at(data.len() - 1) == b'n' {
                                    data = data.slice_to(data.len() - 1);
                                    self.ast.literal_mut(x).data = data.clone();
                                }
                                let num = number(data, self.o.precision);
                                if num.len() == 1 && num.at(0) == b'0' {
                                    self.write_s(ZERO_BYTES);
                                } else {
                                    self.write_s(ONE_BYTES);
                                }
                                return;
                            }
                        }
                    }
                    self.minify_expr(x, unary_prec_map(op));
                }
            }
            Node::DotExpr(expr) => {
                let x = expr.x;
                let mut optional_left = false;
                if let Some(gx) = as_group(&self.ast, x) {
                    match self.ast.node(gx) {
                        Node::LiteralExpr(lit)
                            if lit.token_type == DecimalToken || lit.token_type == IntegerToken =>
                        {
                            let (ltt, ldata) = (lit.token_type, lit.data.clone());
                            if ltt == DecimalToken {
                                self.write(number(ldata, self.o.precision));
                            } else {
                                self.write(ldata);
                                self.write_s(DOT_BYTES);
                            }
                            self.write_s(DOT_BYTES);
                            let y = dot_y(&self.ast, i);
                            self.write(y);
                            return;
                        }
                        Node::DotExpr(dot) => optional_left = dot.optional,
                        Node::CallExpr(call) => optional_left = call.optional,
                        _ => {}
                    }
                }
                if OpMember <= prec || optional_left {
                    self.minify_expr(x, OpMember);
                } else {
                    self.minify_expr(x, OpCall);
                }
                let optional = match self.ast.node(i) {
                    Node::DotExpr(d) => d.optional,
                    _ => unreachable!(),
                };
                if optional {
                    self.write_s(QUESTION_BYTES);
                } else {
                    // 0 < len(m.prev) always
                    let last = self.prev.at(self.prev.len() - 1);
                    if b'0' <= last && last <= b'9' {
                        let mut is_integer = true;
                        for k in 0..self.prev.len() - 1 {
                            let c = self.prev.at(k);
                            if c < b'0' || b'9' < c {
                                is_integer = false;
                                break;
                            }
                        }
                        if is_integer {
                            // prevent previous integer
                            self.write_s(DOT_BYTES);
                        }
                    }
                }
                self.write_s(DOT_BYTES);
                let y = dot_y(&self.ast, i);
                self.write(y);
            }
            Node::GroupExpr(expr) => {
                let gx = expr.x;
                if let Node::CondExpr(_) = self.ast.node(gx) {
                    let nx =
                        optimize_cond_expr(&mut self.ast, gx, OpExpr, self.o.min_version(2020));
                    group_mut(&mut self.ast, i).x = nx;
                }
                let x = group_mut(&mut self.ast, i).x;
                let prec_inside = expr_prec(&self.ast, x);
                if prec <= prec_inside || prec_inside == OpCoalesce && prec == OpBitOr {
                    self.minify_expr(x, prec);
                } else {
                    let parent_in_for = self.in_for;
                    self.in_for = false;
                    self.write_s(OPEN_PAREN_BYTES);
                    self.minify_expr(x, OpExpr);
                    self.write_s(CLOSE_PAREN_BYTES);
                    self.in_for = parent_in_for;
                }
            }
            Node::ArrayExpr(expr) => {
                let list = expr.list.clone();
                let parent_in_for = self.in_for;
                self.in_for = false;
                self.write_s(OPEN_BRACKET_BYTES);
                for (k, item) in list.iter().enumerate() {
                    if k != 0 {
                        self.write_s(COMMA_BYTES);
                    }
                    if item.spread {
                        self.write_s(ELLIPSIS_BYTES);
                    }
                    self.minify_expr(item.value, OpAssign);
                }
                if !list.is_empty() && list[list.len() - 1].value.is_nil() {
                    self.write_s(COMMA_BYTES);
                }
                self.write_s(CLOSE_BRACKET_BYTES);
                self.in_for = parent_in_for;
            }
            Node::ObjectExpr(expr) => {
                let list = expr.list.clone();
                let parent_in_for = self.in_for;
                self.in_for = false;
                let grouped_stmt = self.expect_expr != ExpectExpr::Any;
                if grouped_stmt {
                    self.write_s(OPEN_PAREN_BRACKET_BYTES);
                } else {
                    self.write_s(OPEN_BRACE_BYTES);
                }
                for (k, item) in list.iter().enumerate() {
                    if k != 0 {
                        self.write_s(COMMA_BYTES);
                    }
                    self.minify_property(item);
                }
                self.write_s(CLOSE_BRACE_BYTES);
                if grouped_stmt {
                    self.grouped_stmt = true;
                }
                self.in_for = parent_in_for;
            }
            Node::TemplateExpr(expr) => {
                let (tag, list, tail) = (expr.tag, expr.list.clone(), expr.tail.clone());
                if tag.is_some() {
                    if prec < OpMember {
                        self.minify_expr(tag, OpCall);
                    } else {
                        self.minify_expr(tag, OpMember);
                    }
                    let optional = match self.ast.node(i) {
                        Node::TemplateExpr(t) => t.optional,
                        _ => unreachable!(),
                    };
                    if optional {
                        self.write_s(OPT_CHAIN_BYTES);
                    }
                }
                let parent_in_for = self.in_for;
                self.in_for = false;
                for item in list {
                    if tag.is_nil() {
                        self.write(replace_escapes(item.value, b'`', 1, 2));
                    } else {
                        self.write(item.value);
                    }
                    self.minify_expr(item.expr, OpExpr);
                }
                if tag.is_nil() {
                    self.write(replace_escapes(tail, b'`', 1, 1));
                } else {
                    self.write(tail);
                }
                self.in_for = parent_in_for;
            }
            Node::NewExpr(expr) => {
                let (x, has_args) = (expr.x, expr.args.is_some());
                if !has_args && OpLHS < prec && prec != OpNew {
                    self.write_s(OPEN_NEW_BYTES);
                    self.write_space_before_ident();
                    self.minify_expr(x, OpNew);
                    self.write_s(CLOSE_PAREN_BYTES);
                } else {
                    self.write_s(NEW_BYTES);
                    self.write_space_before_ident();
                    if has_args {
                        self.minify_expr(x, OpMember);
                        let args = match self.ast.node(i) {
                            Node::NewExpr(n) => n.args.clone().unwrap_or_default(),
                            _ => unreachable!(),
                        };
                        self.minify_arguments(&args);
                    } else {
                        self.minify_expr(x, OpNew);
                    }
                }
            }
            Node::NewTargetExpr => {
                self.write_s(NEW_TARGET_BYTES);
                self.write_space_before_ident();
            }
            Node::ImportMetaExpr => {
                self.write_s(IMPORT_META_BYTES);
                self.write_space_before_ident();
            }
            Node::YieldExpr(expr) => {
                let (x, generator) = (expr.x, expr.generator);
                self.write_s(YIELD_BYTES);
                self.write_space_before_ident();
                if x.is_some() {
                    if generator {
                        self.write_s(STAR_BYTES);
                        self.minify_expr(x, OpAssign);
                    } else {
                        let skip = match self.ast.as_var(x) {
                            Some(v) => {
                                self.ast.var_name(x).equal(UNDEFINED_BYTES) && v.decl == NoDecl
                            }
                            None => false,
                        };
                        if !skip {
                            self.minify_expr(x, OpAssign);
                        }
                    }
                }
            }
            Node::CallExpr(_) => self.minify_call_expr(i, prec),
            Node::IndexExpr(expr) => {
                let (x, y) = (expr.x, expr.y);
                if self.expect_expr == ExpectExpr::ExprStmt {
                    if self.ast.is_var(x) && self.ast.var_name(x).equal(LET_BYTES) {
                        self.write_s(NOT_BYTES);
                    }
                }
                if prec < OpMember {
                    self.minify_expr(x, OpCall);
                } else {
                    self.minify_expr(x, OpMember);
                }

                let optional = match self.ast.node(i) {
                    Node::IndexExpr(e) => e.optional,
                    _ => unreachable!(),
                };
                if optional {
                    self.write_s(OPT_CHAIN_BYTES);
                }
                if let Some(lit) = as_literal(&self.ast, y) {
                    if lit.token_type == StringToken && 2 < lit.data.len() {
                        let inner = lit.data.slice(1, lit.data.len() - 1);
                        if as_identifier_name(&inner) {
                            if !optional {
                                self.write_s(DOT_BYTES);
                            }
                            self.write(inner);
                            return;
                        } else if as_decimal_literal(&inner) {
                            self.write_s(OPEN_BRACKET_BYTES);
                            self.write(number(inner, 0));
                            self.write_s(CLOSE_BRACKET_BYTES);
                            return;
                        }
                    }
                }
                let parent_in_for = self.in_for;
                self.in_for = false;
                self.write_s(OPEN_BRACKET_BYTES);
                self.minify_expr(y, OpExpr);
                self.write_s(CLOSE_BRACKET_BYTES);
                self.in_for = parent_in_for;
            }
            Node::CondExpr(expr) => {
                let (cond, x, y) = (expr.cond, expr.x, expr.y);
                self.minify_expr(cond, OpCoalesce);
                self.write_s(QUESTION_BYTES);
                self.minify_expr(x, OpAssign);
                self.write_s(COLON_BYTES);
                self.minify_expr(y, OpAssign);
            }
            Node::VarDecl(_) => {
                self.minify_var_decl(i, true); // happens in for statement or when vars were hoisted
            }
            Node::FuncDecl(_) => {
                let grouped = self.expect_expr == ExpectExpr::ExprStmt && prec != OpExpr;
                if grouped {
                    self.write_s(OPEN_PAREN_BYTES);
                } else if self.expect_expr == ExpectExpr::ExprStmt {
                    self.write_s(NOT_BYTES);
                }
                let (parent_in_for, parent_grouped_stmt) = (self.in_for, self.grouped_stmt);
                self.in_for = false;
                self.grouped_stmt = false;
                self.minify_func_decl(i, true);
                self.in_for = parent_in_for;
                self.grouped_stmt = parent_grouped_stmt;
                if grouped {
                    self.write_s(CLOSE_PAREN_BYTES);
                }
            }
            Node::ArrowFunc(_) => {
                let parent_grouped_stmt = self.grouped_stmt;
                self.grouped_stmt = false;
                self.minify_arrow_func(i);
                self.grouped_stmt = parent_grouped_stmt;
            }
            Node::MethodDecl(_) => {
                let parent_grouped_stmt = self.grouped_stmt;
                self.grouped_stmt = false;
                self.minify_method_decl(i); // only happens in object literal
                self.grouped_stmt = parent_grouped_stmt;
            }
            Node::ClassDecl(_) => {
                if self.expect_expr == ExpectExpr::ExprStmt {
                    self.write_s(NOT_BYTES);
                }
                let (parent_in_for, parent_grouped_stmt) = (self.in_for, self.grouped_stmt);
                self.in_for = false;
                self.grouped_stmt = false;
                self.minify_class_decl(i);
                self.in_for = parent_in_for;
                self.grouped_stmt = parent_grouped_stmt;
            }
            Node::CommaExpr(expr) => {
                let list = expr.list.clone();
                for (k, &item) in list.iter().enumerate() {
                    if k != 0 {
                        self.write_s(COMMA_BYTES);
                    }
                    self.minify_expr(item, OpAssign);
                }
            }
            _ => {}
        }
    }

    // Go: js.go:jsMinifier.minifyExpr, case *js.BinaryExpr
    fn minify_binary_expr(&mut self, i: NodeId, prec: OpPrec) {
        merge_binary_expr(&mut self.ast, i);
        let (op, x, y) = as_binary(&self.ast, i).unwrap();
        if x.is_nil() {
            self.minify_expr(y, prec);
            return;
        }

        let mut prec_left = binary_left_prec_map(op);
        // convert (a,b)&&c into a,b&&c but not a=(b,c)&&d into a=(b,c&&d)
        if prec <= OpExpr {
            if let Some(gx) = as_group(&self.ast, x) {
                if let Some(list) = as_comma(&self.ast, gx) {
                    if OpAnd <= expr_prec(&self.ast, list[list.len() - 1]) {
                        binary_mut(&mut self.ast, i).x = gx;
                        prec_left = OpExpr;
                    }
                }
            }
        }
        if op == InstanceofToken || op == InToken {
            let group = op == InToken && self.in_for;
            if group {
                self.write_s(OPEN_PAREN_BYTES);
            }
            let (_, x, y) = as_binary(&self.ast, i).unwrap();
            self.minify_expr(x, prec_left);
            self.write_space_after_ident();
            self.write(tt_bytes(op));
            self.write_space_before_ident();
            self.minify_expr(y, binary_right_prec_map(op));
            if group {
                self.write_s(CLOSE_PAREN_BYTES);
            }
        } else {
            let mut expr = i;
            if let Some((v, not)) = is_undefined_or_null_var(&self.ast, expr) {
                // change a===null||a===undefined to a==null
                let op = if not { NotEqToken } else { EqEqToken };
                let null = self.ast.alloc(Node::LiteralExpr(LiteralExpr {
                    token_type: NullToken,
                    data: GoBytes::from_static(NULL_BYTES),
                }));
                expr = new_binary(&mut self.ast, op, v, null);
            }

            let (_, x, _) = as_binary(&self.ast, expr).unwrap();
            self.minify_expr(x, prec_left);
            let (op, x, y) = as_binary(&self.ast, expr).unwrap();
            if op == GtToken && self.prev.at(self.prev.len() - 1) == b'-' {
                // 0 < len(m.prev) always
                self.write_s(SPACE_BYTES);
            } else if op == EqEqEqToken || op == NotEqEqToken {
                if let Some((lop, _)) = as_unary(&self.ast, x).filter(|u| u.0 == TypeofToken) {
                    let _ = lop;
                    if as_literal(&self.ast, y).is_some_and(|l| l.token_type == StringToken) {
                        binary_mut(&mut self.ast, expr).op = if op == EqEqEqToken {
                            EqEqToken
                        } else {
                            NotEqToken
                        };
                    }
                } else if as_unary(&self.ast, y).is_some_and(|u| u.0 == TypeofToken) {
                    if as_literal(&self.ast, x).is_some_and(|l| l.token_type == StringToken) {
                        binary_mut(&mut self.ast, expr).op = if op == EqEqEqToken {
                            EqEqToken
                        } else {
                            NotEqToken
                        };
                    }
                }
            }
            let (op, _, y) = as_binary(&self.ast, expr).unwrap();
            self.write(tt_bytes(op));
            if op == AddToken {
                // +++  =>  + ++
                self.write_space_before(b'+');
            } else if op == SubToken {
                // ---  =>  - --
                self.write_space_before(b'-');
            } else if op == DivToken {
                // //  =>  / /
                self.write_space_before(b'/');
            }
            self.minify_expr(y, binary_right_prec_map(op));
        }
    }

    // Go: js.go:jsMinifier.minifyExpr, case *js.CallExpr
    fn minify_call_expr(&mut self, i: NodeId, prec: OpPrec) {
        let (x, args) = match self.ast.node(i) {
            Node::CallExpr(c) => (c.x, c.args.list.clone()),
            _ => unreachable!(),
        };
        match self.ast.node(x) {
            Node::Var(v) if v.decl == NoDecl => {
                if v.data.equal(IS_NAN_BYTES) {
                    // isNaN(x) => x!=x
                    if args.len() == 1 {
                        let group_len = if OpEquals < prec { 2 } else { 0 };
                        if let Some(av) = self.ast.as_var(args[0].value) {
                            if av.data.len() + group_len < 7 {
                                let v = args[0].value;
                                if OpEquals < prec {
                                    self.write_s(OPEN_PAREN_BYTES);
                                }
                                self.minify_expr(v, OpEquals);
                                self.write_s(NOT_EQUAL_BYTES);
                                self.minify_expr(v, OpEquals);
                                if OpEquals < prec {
                                    self.write_s(CLOSE_PAREN_BYTES);
                                }
                                return;
                            }
                        }
                    }
                } else if v.data.equal(NUMBER_BYTES) {
                    // Number(x) => +x
                    if args.len() == 1 {
                        let a0 = args[0].value;
                        match self.ast.node(a0) {
                            Node::LiteralExpr(lit) => {
                                let tt = lit.token_type;
                                if tt == TrueToken {
                                    self.write_s(ONE_BYTES);
                                    return;
                                } else if tt == FalseToken || tt == NullToken {
                                    self.write_s(ZERO_BYTES);
                                    return;
                                } else if tt == DecimalToken {
                                    self.minify_expr(a0, prec);
                                    return;
                                } else if tt == IntegerToken
                                    || tt == BinaryToken
                                    || tt == OctalToken
                                    || tt == HexadecimalToken
                                {
                                    let data = lit.data.clone();
                                    if data.at(data.len() - 1) == b'n' {
                                        self.ast.literal_mut(a0).data =
                                            data.slice_to(data.len() - 1);
                                    }
                                    self.minify_expr(a0, prec);
                                    return;
                                }
                            }
                            Node::Var(av)
                                if av.decl == NoDecl && av.data.equal(UNDEFINED_BYTES) =>
                            {
                                self.write_s(NAN_BYTES);
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Node::DotExpr(dot) => {
                let (dx, dy) = (dot.x, dot.y.data.clone());
                if let Some(v) = self.ast.as_var(dx) {
                    if v.decl == NoDecl && v.data.equal(MATH_BYTES) {
                        if dy.equal(b"pow") {
                            // Math.pow(a,b) => a**b
                            if args.len() == 2 {
                                if OpExp < prec {
                                    self.write_s(OPEN_PAREN_BYTES);
                                }
                                let g0 = new_group(&mut self.ast, args[0].value);
                                self.minify_expr(g0, OpUpdate);
                                self.write_s(EXP_BYTES);
                                let g1 = new_group(&mut self.ast, args[1].value);
                                self.minify_expr(g1, OpExp);
                                if OpExp < prec {
                                    self.write_s(CLOSE_PAREN_BYTES);
                                }
                                return;
                            }
                        } else if dy.equal(b"trunc") {
                            // Math.trunc(x) => x|0
                            if args.len() == 1 {
                                if OpBitOr < prec {
                                    self.write_s(OPEN_PAREN_BYTES);
                                }
                                let g0 = new_group(&mut self.ast, args[0].value);
                                self.minify_expr(g0, OpBitOr);
                                self.write_s(BIT_OR_BYTES);
                                self.write_s(ZERO_BYTES);
                                if OpBitOr < prec {
                                    self.write_s(CLOSE_PAREN_BYTES);
                                }
                                return;
                            }
                        } else if dy.equal(b"abs") {
                            // Math.abs(x) => x<0?-x:x
                            if args.len() == 1 {
                                let group_len = if OpAssign < prec { 2 } else { 0 };
                                if let Some(av) = self.ast.as_var(args[0].value) {
                                    if av.data.len() * 2 + group_len + 5 < 10 {
                                        let v = args[0].value;
                                        if OpAssign < prec {
                                            self.write_s(OPEN_PAREN_BYTES);
                                        }
                                        self.minify_expr(v, OpCoalesce);
                                        self.write(GoBytes::from_slice(b"<0?-"));
                                        self.minify_expr(v, OpAssign);
                                        self.write_s(COLON_BYTES);
                                        self.minify_expr(v, OpAssign);
                                        if OpAssign < prec {
                                            self.write_s(CLOSE_PAREN_BYTES);
                                        }
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        self.minify_expr(x, OpCall);
        let parent_in_for = self.in_for;
        self.in_for = false;
        let (optional, args) = match self.ast.node(i) {
            Node::CallExpr(c) => (c.optional, c.args.clone()),
            _ => unreachable!(),
        };
        if optional {
            self.write_s(OPT_CHAIN_BYTES);
        }
        self.minify_arguments(&args);
        self.in_for = parent_in_for;
    }
}

/// Go `js.Params` (the name `Params` is taken by the minify parameter map).
type Params_ = tdewolff_parse_js::Params;

/// `expr.Y.Data` of a `*js.DotExpr`.
fn dot_y(ast: &Ast, i: NodeId) -> GoBytes {
    match ast.node(i) {
        Node::DotExpr(d) => d.y.data.clone(),
        _ => unreachable!(),
    }
}
