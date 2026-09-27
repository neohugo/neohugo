//! Go: minify/v2@v2.23.8/js/stmtlist.go — `optimizeStmt` and
//! `optimizeStmtList`.
//!
//! Go passes statement lists as slice headers that share their backing array
//! with the caller's `BlockStmt.List`; every caller stores the returned slice
//! back, and nothing reads the caller's stale header in between (the AST is a
//! tree), so the port moves the `Vec` in and out. In-place `append` tricks
//! (`append(list[:i+1], append(block.List, list[i+1:]...)...)`,
//! `append(list[:i], list[k:]...)`) are the equivalent `Vec` splices.

use tdewolff_parse_js::*;

use crate::js::BlockType;
use crate::util::*;
use crate::vars::{has_defines, merge_var_decl_expr_stmt, merge_var_decls};

// Go: stmtlist.go:optimizeStmt
/// Converts if/else into expression statements and optimizes blocks.
pub(crate) fn optimize_stmt(ast: &mut Ast, i: NodeId) -> NodeId {
    match ast.node(i) {
        Node::IfStmt(_) => {
            let if_stmt = i;
            let (body, else_) = {
                let s = if_mut(ast, if_stmt);
                (s.body, s.else_)
            };
            if body.is_some() {
                let b = optimize_stmt(ast, body);
                if_mut(ast, if_stmt).body = b;
            }
            if else_.is_some() {
                let e = optimize_stmt(ast, else_);
                if_mut(ast, if_stmt).else_ = e;
            }
            let (cond, body, else_) = {
                let s = if_mut(ast, if_stmt);
                (s.cond, s.body, s.else_)
            };
            let mut has_if = !is_empty_stmt(ast, body);
            let mut has_else = !is_empty_stmt(ast, else_);
            if let Some((op, ux)) = as_unary(ast, cond) {
                if op == NotToken && has_else {
                    let s = if_mut(ast, if_stmt);
                    s.cond = ux;
                    std::mem::swap(&mut s.body, &mut s.else_);
                    std::mem::swap(&mut has_if, &mut has_else);
                }
            }
            let (cond, body, else_) = {
                let s = if_mut(ast, if_stmt);
                (s.cond, s.body, s.else_)
            };
            if !has_if && !has_else {
                if has_side_effects(ast, cond) {
                    return new_expr_stmt(ast, cond);
                }
                return ast.alloc(Node::EmptyStmt);
            } else if has_if && !has_else {
                if let Node::ExprStmt(x) = ast.node(body) {
                    let xvalue = x.value;
                    if let Some((op, ux)) = as_unary(ast, cond) {
                        if op == NotToken {
                            let left = group_expr(ast, ux, binary_left_prec_map(OrToken));
                            let right = group_expr(ast, xvalue, binary_right_prec_map(OrToken));
                            let b = new_binary(ast, OrToken, left, right);
                            return new_expr_stmt(ast, b);
                        }
                    }
                    let left = group_expr(ast, cond, binary_left_prec_map(AndToken));
                    let right = group_expr(ast, xvalue, binary_right_prec_map(AndToken));
                    let b = new_binary(ast, AndToken, left, right);
                    return new_expr_stmt(ast, b);
                } else if let Node::IfStmt(x) = ast.node(body) {
                    let (xcond, xbody, xelse) = (x.cond, x.body, x.else_);
                    if is_empty_stmt(ast, xelse) {
                        let left = group_expr(ast, cond, binary_left_prec_map(AndToken));
                        let right = group_expr(ast, xcond, binary_right_prec_map(AndToken));
                        let b = new_binary(ast, AndToken, left, right);
                        let s = if_mut(ast, if_stmt);
                        s.cond = b;
                        s.body = xbody;
                        return if_stmt;
                    }
                }
            } else if !has_if && has_else {
                if let Node::ExprStmt(x) = ast.node(else_) {
                    let xvalue = x.value;
                    let left = group_expr(ast, cond, binary_left_prec_map(OrToken));
                    let right = group_expr(ast, xvalue, binary_right_prec_map(OrToken));
                    let b = new_binary(ast, OrToken, left, right);
                    return new_expr_stmt(ast, b);
                }
            } else if has_if && has_else {
                let x_expr = match ast.node(body) {
                    Node::ExprStmt(x) => Some(x.value),
                    _ => None,
                };
                let y_expr = match ast.node(else_) {
                    Node::ExprStmt(y) => Some(y.value),
                    _ => None,
                };
                if let (Some(xv), Some(yv)) = (x_expr, y_expr) {
                    let c = cond_expr(ast, cond, xv, yv);
                    return new_expr_stmt(ast, c);
                }
                let x_return = match ast.node(body) {
                    Node::ReturnStmt(x) => Some(x.value),
                    _ => None,
                };
                let y_return = match ast.node(else_) {
                    Node::ReturnStmt(y) => Some(y.value),
                    _ => None,
                };
                if let (Some(xv), Some(yv)) = (x_return, y_return) {
                    if xv.is_nil() && yv.is_nil() {
                        let zero = ast.alloc(Node::LiteralExpr(LiteralExpr {
                            token_type: NumericToken,
                            data: tdewolff_parse::GoBytes::from_static(ZERO_BYTES),
                        }));
                        let void = new_unary(ast, VoidToken, zero);
                        let c = comma_expr(ast, cond, void);
                        return ast.alloc(Node::ReturnStmt(ReturnStmt { value: c }));
                    } else if xv.is_some() && yv.is_some() {
                        let c = cond_expr(ast, cond, xv, yv);
                        return ast.alloc(Node::ReturnStmt(ReturnStmt { value: c }));
                    }
                    return if_stmt;
                }
                let x_throw = match ast.node(body) {
                    Node::ThrowStmt(x) => Some(x.value),
                    _ => None,
                };
                let y_throw = match ast.node(else_) {
                    Node::ThrowStmt(y) => Some(y.value),
                    _ => None,
                };
                if let (Some(xv), Some(yv)) = (x_throw, y_throw) {
                    let c = cond_expr(ast, cond, xv, yv);
                    return ast.alloc(Node::ThrowStmt(ThrowStmt { value: c }));
                }
            }
        }
        Node::VarDecl(decl) => {
            if decl.token_type == ErrorToken {
                // convert hoisted var declaration to expression or empty (if there are no defines) statement
                for item in &decl.list {
                    if item.default.is_some() {
                        return new_expr_stmt(ast, i);
                    }
                }
                return ast.alloc(Node::EmptyStmt);
            }
            return i;
        }
        Node::BlockStmt(_) => {
            let block_stmt = i;
            // merge body and remove braces if it is not a lexical declaration
            let list = std::mem::take(&mut ast.block_mut(block_stmt).list);
            let list = optimize_stmt_list(ast, list, BlockType::Default);
            ast.block_mut(block_stmt).list = list;
            let list = &ast.block(block_stmt).list;
            if list.len() == 1 {
                let first = list[0];
                if let Node::ClassDecl(_) = ast.node(first) {
                    return ast.alloc(Node::EmptyStmt);
                } else if let Node::VarDecl(var_decl) = ast.node(first) {
                    if var_decl.token_type != VarToken {
                        // remove let or const declaration in otherwise empty scope, but keep assignments
                        let mut exprs: Vec<NodeId> = Vec::new();
                        for item in &var_decl.list {
                            if item.default.is_some() && has_side_effects(ast, item.default) {
                                exprs.push(item.default);
                            }
                        }
                        if exprs.is_empty() {
                            return ast.alloc(Node::EmptyStmt);
                        } else if exprs.len() == 1 {
                            return new_expr_stmt(ast, exprs[0]);
                        }
                        let c = ast.alloc(Node::CommaExpr(CommaExpr { list: exprs }));
                        return new_expr_stmt(ast, c);
                    }
                }
                return optimize_stmt(ast, first);
            } else if list.is_empty() {
                return ast.alloc(Node::EmptyStmt);
            }
            return block_stmt;
        }
        _ => {}
    }
    i
}

/// `list[k]` of a Go `[]js.IStmt`.
#[inline]
fn is_node_empty(ast: &Ast, id: NodeId) -> bool {
    matches!(ast.node(id), Node::EmptyStmt)
}

/// `&js.ForStmt{Init: init, Cond: whileStmt.Cond, Post: nil, Body: body}`
/// from a `*js.WhileStmt` (body wrapped in a new block when needed).
fn while_to_for(ast: &mut Ast, while_stmt: NodeId, init: NodeId) -> NodeId {
    let (wcond, wbody) = match ast.node(while_stmt) {
        Node::WhileStmt(w) => (w.cond, w.body),
        _ => unreachable!(),
    };
    let body = if let Node::BlockStmt(_) = ast.node(wbody) {
        wbody
    } else {
        ast.alloc_block(vec![wbody])
    };
    ast.alloc(Node::ForStmt(ForStmt {
        init,
        cond: wcond,
        post: NodeId::NIL,
        body,
    }))
}

// Go: stmtlist.go:optimizeStmtList
/// Merges expression statements as well as if/else statements followed by
/// flow control statements.
pub(crate) fn optimize_stmt_list(
    ast: &mut Ast,
    list: Vec<NodeId>,
    block_type: BlockType,
) -> Vec<NodeId> {
    let mut list = list;
    if list.is_empty() {
        return list;
    }
    let mut j: isize = 0; // write index
    let mut i: isize = 0; // read index
    while i < list.len() as isize {
        let iu = i as usize;
        if let Node::IfStmt(if_s) = ast.node(list[iu]) {
            let if_stmt = list[iu];
            if !is_empty_stmt(ast, if_s.else_) {
                // if(!a)b;else c  =>  if(a)c; else b
                let (cond, else_) = (if_s.cond, if_s.else_);
                if let Some((op, ux)) = as_unary(ast, cond) {
                    if op == NotToken && is_flow_stmt(ast, last_stmt(ast, else_)) {
                        let s = if_mut(ast, if_stmt);
                        s.cond = ux;
                        std::mem::swap(&mut s.body, &mut s.else_);
                    }
                }
                let (body, else_) = {
                    let s = if_mut(ast, if_stmt);
                    (s.body, s.else_)
                };
                if is_flow_stmt(ast, last_stmt(ast, body)) {
                    // if body ends in flow statement (return, throw, break, continue), we can remove the else statement and put its body in the current scope
                    let tail = list.split_off(iu + 1);
                    if let Node::BlockStmt(block) = ast.node(else_) {
                        let scope = block.scope;
                        ast.unscope(scope);
                        let block_list = ast.block(else_).list.clone();
                        list.extend(block_list);
                    } else {
                        list.push(else_);
                    }
                    list.extend(tail);
                    if_mut(ast, if_stmt).else_ = NodeId::NIL;
                }
            }
        }

        list[iu] = optimize_stmt(ast, list[iu]);

        if is_node_empty(ast, list[iu]) {
            let mut k = iu + 1;
            while k < list.len() {
                if !is_node_empty(ast, list[k]) {
                    break;
                }
                k += 1;
            }
            list.drain(iu..k);
            i -= 1;
            i += 1;
            continue;
        }

        if 0 < i {
            let prev = list[iu - 1];
            let cur = list[iu];
            // merge expression statements with expression, return, and throw statements
            if let Node::ExprStmt(left) = ast.node(prev) {
                let left_value = left.value;
                let left_id = prev;
                match ast.node(cur) {
                    Node::ExprStmt(right) => {
                        let rv = right.value;
                        let c = comma_expr(ast, left_value, rv);
                        if let Node::ExprStmt(right) = ast.node_mut(cur) {
                            right.value = c;
                        }
                        j -= 1;
                    }
                    Node::ReturnStmt(ret) if ret.value.is_some() => {
                        let rv = ret.value;
                        let c = comma_expr(ast, left_value, rv);
                        if let Node::ReturnStmt(ret) = ast.node_mut(cur) {
                            ret.value = c;
                        }
                        j -= 1;
                    }
                    Node::ReturnStmt(_) => {
                        // `returnStmt.Value != nil` failed: try the remaining cases,
                        // none of which match a ReturnStmt
                    }
                    Node::ThrowStmt(thr) => {
                        let tv = thr.value;
                        let c = comma_expr(ast, left_value, tv);
                        if let Node::ThrowStmt(thr) = ast.node_mut(cur) {
                            thr.value = c;
                        }
                        j -= 1;
                    }
                    Node::ForStmt(for_s) => {
                        // TODO: only merge lhs expression that don't have 'in' or 'of' keywords (slow to check?)
                        let init = for_s.init;
                        let set_init = |ast: &mut Ast, v: NodeId| {
                            if let Node::ForStmt(f) = ast.node_mut(cur) {
                                f.init = v;
                            }
                        };
                        if init.is_nil() {
                            set_init(ast, left_value);
                            j -= 1;
                        } else if let Node::VarDecl(decl) = ast.node(init) {
                            if decl.list.is_empty() {
                                set_init(ast, left_value);
                                j -= 1;
                            } else if decl.token_type == VarToken || decl.token_type == ErrorToken {
                                // this is the second VarDecl, so we are hoisting var declarations, which means the forInit variables are already in 'left'
                                if merge_var_decl_expr_stmt(ast, init, left_id, true) {
                                    j -= 1;
                                }
                            }
                        }
                    }
                    Node::WhileStmt(_) => {
                        // TODO: only merge lhs expression that don't have 'in' or 'of' keywords (slow to check?)
                        let f = while_to_for(ast, cur, left_value);
                        list[iu] = f;
                        j -= 1;
                    }
                    Node::SwitchStmt(sw) => {
                        let init = sw.init;
                        let c = comma_expr(ast, left_value, init);
                        ast.switch_stmt_mut(cur).init = c;
                        j -= 1;
                    }
                    Node::WithStmt(with) => {
                        let wc = with.cond;
                        let c = comma_expr(ast, left_value, wc);
                        if let Node::WithStmt(with) = ast.node_mut(cur) {
                            with.cond = c;
                        }
                        j -= 1;
                    }
                    Node::IfStmt(if_s) => {
                        let ic = if_s.cond;
                        let c = comma_expr(ast, left_value, ic);
                        if_mut(ast, cur).cond = c;
                        j -= 1;
                    }
                    Node::VarDecl(var_decl) if var_decl.token_type == VarToken => {
                        if merge_var_decl_expr_stmt(ast, cur, left_id, true) {
                            j -= 1;
                        }
                    }
                    _ => {}
                }
            } else if let Node::VarDecl(left) = ast.node(prev) {
                let left_id = prev;
                let left_tt = left.token_type;
                let right_tt = match ast.node(cur) {
                    Node::VarDecl(right) => Some(right.token_type),
                    _ => None,
                };
                if right_tt == Some(left_tt) {
                    // merge const and let declarations, or non-hoisted var declarations
                    let mut merged = left.list.clone();
                    let left_scope = left.scope;
                    merged.extend(ast.var_decl(cur).list.iter().copied());
                    ast.var_decl_mut(cur).list = merged;
                    j -= 1;

                    // remove from vardecls list of scope
                    let scope = ast.scope(left_scope).func;
                    let var_decls = &mut ast.scope_mut(scope).var_decls;
                    for k in 0..var_decls.len() {
                        if var_decls[k] == left_id {
                            var_decls.remove(k);
                            break;
                        }
                    }
                } else if left_tt == VarToken {
                    match ast.node(cur) {
                        Node::ExprStmt(_) => {
                            // pull in assignments to variables into the declaration, e.g. var a;a=5  =>  var a=5
                            if merge_var_decl_expr_stmt(ast, left_id, cur, false) {
                                list[iu] = list[iu - 1];
                                j -= 1;
                            }
                        }
                        Node::ForStmt(for_s) => {
                            // TODO: only merge lhs expression that don't have 'in' or 'of' keywords (slow to check?)
                            let init = for_s.init;
                            let set_init = |ast: &mut Ast, v: NodeId| {
                                if let Node::ForStmt(f) = ast.node_mut(cur) {
                                    f.init = v;
                                }
                            };
                            if init.is_nil() {
                                set_init(ast, left_id);
                                j -= 1;
                            } else if let Node::VarDecl(decl) = ast.node(init) {
                                let dtt = decl.token_type;
                                if dtt == ErrorToken && !has_defines(ast, init) {
                                    set_init(ast, left_id);
                                    j -= 1;
                                } else if dtt == VarToken || dtt == ErrorToken {
                                    // this is the second VarDecl, so we are hoisting var declarations, which means the forInit variables are already in 'left'
                                    merge_var_decls(ast, left_id, init, false);
                                    ast.var_decl_mut(init).token_type = VarToken;
                                    set_init(ast, left_id);
                                    j -= 1;
                                }
                            }
                        }
                        Node::WhileStmt(_) => {
                            // TODO: only merge lhs expression that don't have 'in' or 'of' keywords (slow to check?)
                            let f = while_to_for(ast, cur, left_id);
                            list[iu] = f;
                            j -= 1;
                        }
                        _ => {}
                    }
                }
            }
        }
        list[j as usize] = list[iu];

        // merge if/else with return/throw when followed by return/throw
        'merge_if_return_throw: loop {
            if 0 < j {
                let ju = j as usize;
                // separate from expression merging in case of:  if(a)return b;b=c;return d
                if let Node::IfStmt(if_s) = ast.node(list[ju - 1]) {
                    let (icond, ibody, ielse) = (if_s.cond, if_s.body, if_s.else_);
                    if is_empty_stmt(ast, ibody) != is_empty_stmt(ast, ielse) {
                        // either the if body is empty or the else body is empty. In case where both bodies have return/throw, we already rewrote that if statement to an return/throw statement
                        let cur = list[ju];
                        if let Node::ReturnStmt(ret) = ast.node(cur) {
                            let rv = ret.value;
                            let body_ret = match ast.node(ibody) {
                                Node::ReturnStmt(r) => Some(r.value),
                                _ => None,
                            };
                            let else_ret = match ast.node(ielse) {
                                Node::ReturnStmt(r) => Some(r.value),
                                _ => None,
                            };
                            if rv.is_nil() {
                                if body_ret.is_some_and(|v| v.is_nil()) {
                                    list[ju - 1] = new_expr_stmt(ast, icond);
                                } else if else_ret.is_some_and(|v| v.is_nil()) {
                                    list[ju - 1] = new_expr_stmt(ast, icond);
                                }
                            } else if let Some(lv) = body_ret.filter(|v| v.is_some()) {
                                let c = cond_expr(ast, icond, lv, rv);
                                if let Node::ReturnStmt(ret) = ast.node_mut(cur) {
                                    ret.value = c;
                                }
                                list[ju - 1] = cur;
                                j -= 1;
                                continue 'merge_if_return_throw;
                            } else if let Some(lv) = else_ret.filter(|v| v.is_some()) {
                                let c = cond_expr(ast, icond, rv, lv);
                                if let Node::ReturnStmt(ret) = ast.node_mut(cur) {
                                    ret.value = c;
                                }
                                list[ju - 1] = cur;
                                j -= 1;
                                continue 'merge_if_return_throw;
                            }
                        } else if let Node::ThrowStmt(thr) = ast.node(cur) {
                            let tv = thr.value;
                            let body_thr = match ast.node(ibody) {
                                Node::ThrowStmt(t) => Some(t.value),
                                _ => None,
                            };
                            let else_thr = match ast.node(ielse) {
                                Node::ThrowStmt(t) => Some(t.value),
                                _ => None,
                            };
                            if let Some(lv) = body_thr {
                                let c = cond_expr(ast, icond, lv, tv);
                                if let Node::ThrowStmt(thr) = ast.node_mut(cur) {
                                    thr.value = c;
                                }
                                list[ju - 1] = cur;
                                j -= 1;
                                continue 'merge_if_return_throw;
                            } else if let Some(lv) = else_thr {
                                let c = cond_expr(ast, icond, tv, lv);
                                if let Node::ThrowStmt(thr) = ast.node_mut(cur) {
                                    thr.value = c;
                                }
                                list[ju - 1] = cur;
                                j -= 1;
                                continue 'merge_if_return_throw;
                            }
                        }
                    }
                }
            }
            break;
        }
        j += 1;
        i += 1;
    }

    // remove superfluous return or continue
    if 0 < j {
        let last = list[(j - 1) as usize];
        if block_type == BlockType::Function {
            if let Node::ReturnStmt(ret) = ast.node(last) {
                let rv = ret.value;
                if rv.is_nil() || is_undefined(ast, rv) {
                    j -= 1;
                } else if let Some(cl) = as_comma(ast, rv) {
                    if is_undefined(ast, cl[cl.len() - 1]) {
                        // rewrite function f(){return a,void 0} => function f(){a}
                        if cl.len() == 2 {
                            let first = cl[0];
                            list[(j - 1) as usize] = new_expr_stmt(ast, first);
                        } else {
                            comma_mut(ast, rv).list.pop();
                        }
                    }
                }
            }
        } else if block_type == BlockType::Iteration {
            if let Node::BranchStmt(br) = ast.node(last) {
                if br.type_ == ContinueToken && br.label.is_nil() {
                    j -= 1;
                }
            }
        }
    }
    list.truncate(j as usize);
    list
}
