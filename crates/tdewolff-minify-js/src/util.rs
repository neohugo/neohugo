//! Go: minify/v2@v2.23.8/js/util.go — byte constants, statement/expression
//! predicates, precedence maps, the expression optimizers, string/regexp
//! minification and number literal conversions.
//!
//! Every function that Go writes as `f(i js.IExpr)` takes the arena
//! (`&Ast` / `&mut Ast`) plus a [`NodeId`]; Go `nil` is [`NodeId::NIL`].

use tdewolff_parse::GoBytes;
use tdewolff_parse_js::*;

////////////////////////////////////////////////////////////////
// Go: util.go byte constants (package-level `[]byte` globals).

pub(crate) const SPACE_BYTES: &[u8] = b" ";
pub(crate) const NEWLINE_BYTES: &[u8] = b"\n";
pub(crate) const STAR_BYTES: &[u8] = b"*";
#[allow(dead_code)]
pub(crate) const PLUS_BYTES: &[u8] = b"+";
pub(crate) const EXP_BYTES: &[u8] = b"**";
pub(crate) const BIT_OR_BYTES: &[u8] = b"|";
pub(crate) const COLON_BYTES: &[u8] = b":";
pub(crate) const SEMICOLON_BYTES: &[u8] = b";";
pub(crate) const COMMA_BYTES: &[u8] = b",";
pub(crate) const DOT_BYTES: &[u8] = b".";
pub(crate) const ELLIPSIS_BYTES: &[u8] = b"...";
pub(crate) const OPEN_BRACE_BYTES: &[u8] = b"{";
pub(crate) const CLOSE_BRACE_BYTES: &[u8] = b"}";
pub(crate) const OPEN_PAREN_BYTES: &[u8] = b"(";
pub(crate) const CLOSE_PAREN_BYTES: &[u8] = b")";
pub(crate) const OPEN_BRACKET_BYTES: &[u8] = b"[";
pub(crate) const CLOSE_BRACKET_BYTES: &[u8] = b"]";
pub(crate) const OPEN_PAREN_BRACKET_BYTES: &[u8] = b"({";
pub(crate) const CLOSE_PAREN_OPEN_BRACKET_BYTES: &[u8] = b"){";
pub(crate) const NOT_BYTES: &[u8] = b"!";
pub(crate) const QUESTION_BYTES: &[u8] = b"?";
pub(crate) const EQUAL_BYTES: &[u8] = b"=";
pub(crate) const OPT_CHAIN_BYTES: &[u8] = b"?.";
pub(crate) const ARROW_BYTES: &[u8] = b"=>";
pub(crate) const NOT_EQUAL_BYTES: &[u8] = b"!=";
pub(crate) const ZERO_BYTES: &[u8] = b"0";
pub(crate) const ONE_BYTES: &[u8] = b"1";
pub(crate) const LET_BYTES: &[u8] = b"let";
pub(crate) const GET_BYTES: &[u8] = b"get";
pub(crate) const SET_BYTES: &[u8] = b"set";
pub(crate) const ASYNC_BYTES: &[u8] = b"async";
pub(crate) const FUNCTION_BYTES: &[u8] = b"function";
pub(crate) const STATIC_BYTES: &[u8] = b"static";
pub(crate) const IF_OPEN_BYTES: &[u8] = b"if(";
pub(crate) const ELSE_BYTES: &[u8] = b"else";
pub(crate) const WITH_OPEN_BYTES: &[u8] = b"with(";
pub(crate) const DO_BYTES: &[u8] = b"do";
pub(crate) const WHILE_OPEN_BYTES: &[u8] = b"while(";
pub(crate) const FOR_OPEN_BYTES: &[u8] = b"for(";
pub(crate) const FOR_AWAIT_OPEN_BYTES: &[u8] = b"for await(";
pub(crate) const IN_BYTES: &[u8] = b"in";
pub(crate) const OF_BYTES: &[u8] = b"of";
pub(crate) const SWITCH_OPEN_BYTES: &[u8] = b"switch(";
pub(crate) const THROW_BYTES: &[u8] = b"throw";
pub(crate) const TRY_BYTES: &[u8] = b"try";
pub(crate) const CATCH_BYTES: &[u8] = b"catch";
pub(crate) const FINALLY_BYTES: &[u8] = b"finally";
pub(crate) const IMPORT_BYTES: &[u8] = b"import";
pub(crate) const EXPORT_BYTES: &[u8] = b"export";
pub(crate) const FROM_BYTES: &[u8] = b"from";
pub(crate) const RETURN_BYTES: &[u8] = b"return";
pub(crate) const CLASS_BYTES: &[u8] = b"class";
pub(crate) const AS_SPACE_BYTES: &[u8] = b"as ";
pub(crate) const ASYNC_SPACE_BYTES: &[u8] = b"async ";
pub(crate) const SPACE_DEFAULT_BYTES: &[u8] = b" default";
pub(crate) const SPACE_EXTENDS_BYTES: &[u8] = b" extends";
pub(crate) const YIELD_BYTES: &[u8] = b"yield";
pub(crate) const NEW_BYTES: &[u8] = b"new";
pub(crate) const OPEN_NEW_BYTES: &[u8] = b"(new";
pub(crate) const NEW_TARGET_BYTES: &[u8] = b"new.target";
pub(crate) const IMPORT_META_BYTES: &[u8] = b"import.meta";
pub(crate) const NAN_BYTES: &[u8] = b"NaN";
pub(crate) const UNDEFINED_BYTES: &[u8] = b"undefined";
pub(crate) const INFINITY_BYTES: &[u8] = b"Infinity";
pub(crate) const NULL_BYTES: &[u8] = b"null";
pub(crate) const ZERO_INDEX_BYTES: &[u8] = b"0[0]";
pub(crate) const GROUPED_ZERO_INDEX_BYTES: &[u8] = b"(0[0])";
pub(crate) const ONE_DIV_ZERO_BYTES: &[u8] = b"1/0";
pub(crate) const GROUPED_ONE_DIV_ZERO_BYTES: &[u8] = b"(1/0)";
pub(crate) const NOT_ZERO_BYTES: &[u8] = b"!0";
pub(crate) const GROUPED_NOT_ZERO_BYTES: &[u8] = b"(!0)";
pub(crate) const NOT_ONE_BYTES: &[u8] = b"!1";
pub(crate) const GROUPED_NOT_ONE_BYTES: &[u8] = b"(!1)";
pub(crate) const DEBUGGER_BYTES: &[u8] = b"debugger";
pub(crate) const REG_EXP_SCRIPT_BYTES: &[u8] = b"/script>";
pub(crate) const IS_NAN_BYTES: &[u8] = b"isNaN";
pub(crate) const NUMBER_BYTES: &[u8] = b"Number";
pub(crate) const MATH_BYTES: &[u8] = b"Math";

////////////////////////////////////////////////////////////////
// Small typed views of nodes (Go type assertions `x, ok := i.(*js.T)`).

/// `u, ok := i.(*js.UnaryExpr)` → `(u.Op, u.X)`
#[inline]
pub(crate) fn as_unary(ast: &Ast, i: NodeId) -> Option<(TokenType, NodeId)> {
    match ast.node(i) {
        Node::UnaryExpr(u) => Some((u.op, u.x)),
        _ => None,
    }
}

/// `b, ok := i.(*js.BinaryExpr)` → `(b.Op, b.X, b.Y)`
#[inline]
pub(crate) fn as_binary(ast: &Ast, i: NodeId) -> Option<(TokenType, NodeId, NodeId)> {
    match ast.node(i) {
        Node::BinaryExpr(b) => Some((b.op, b.x, b.y)),
        _ => None,
    }
}

/// `g, ok := i.(*js.GroupExpr)` → `g.X`
#[inline]
pub(crate) fn as_group(ast: &Ast, i: NodeId) -> Option<NodeId> {
    match ast.node(i) {
        Node::GroupExpr(g) => Some(g.x),
        _ => None,
    }
}

/// `l, ok := i.(*js.LiteralExpr)`
#[inline]
pub(crate) fn as_literal(ast: &Ast, i: NodeId) -> Option<&LiteralExpr> {
    match ast.node(i) {
        Node::LiteralExpr(l) => Some(l),
        _ => None,
    }
}

/// `c, ok := i.(*js.CommaExpr)` → `c.List`
#[inline]
pub(crate) fn as_comma(ast: &Ast, i: NodeId) -> Option<&Vec<NodeId>> {
    match ast.node(i) {
        Node::CommaExpr(c) => Some(&c.list),
        _ => None,
    }
}

#[inline]
pub(crate) fn binary_mut(ast: &mut Ast, i: NodeId) -> &mut BinaryExpr {
    match ast.node_mut(i) {
        Node::BinaryExpr(b) => b,
        _ => panic!("interface conversion: not *js.BinaryExpr"),
    }
}

#[inline]
pub(crate) fn comma_mut(ast: &mut Ast, i: NodeId) -> &mut CommaExpr {
    match ast.node_mut(i) {
        Node::CommaExpr(c) => c,
        _ => panic!("interface conversion: not *js.CommaExpr"),
    }
}

#[inline]
pub(crate) fn cond_mut(ast: &mut Ast, i: NodeId) -> &mut CondExpr {
    match ast.node_mut(i) {
        Node::CondExpr(c) => c,
        _ => panic!("interface conversion: not *js.CondExpr"),
    }
}

#[inline]
pub(crate) fn group_mut(ast: &mut Ast, i: NodeId) -> &mut GroupExpr {
    match ast.node_mut(i) {
        Node::GroupExpr(g) => g,
        _ => panic!("interface conversion: not *js.GroupExpr"),
    }
}

#[inline]
pub(crate) fn try_mut(ast: &mut Ast, i: NodeId) -> &mut TryStmt {
    match ast.node_mut(i) {
        Node::TryStmt(s) => s,
        _ => panic!("interface conversion: not *js.TryStmt"),
    }
}

#[inline]
pub(crate) fn if_mut(ast: &mut Ast, i: NodeId) -> &mut IfStmt {
    match ast.node_mut(i) {
        Node::IfStmt(s) => s,
        _ => panic!("interface conversion: not *js.IfStmt"),
    }
}

/// `&js.GroupExpr{X: x}`
#[inline]
pub(crate) fn new_group(ast: &mut Ast, x: NodeId) -> NodeId {
    ast.alloc(Node::GroupExpr(GroupExpr { x }))
}

/// `&js.BinaryExpr{op, x, y}`
#[inline]
pub(crate) fn new_binary(ast: &mut Ast, op: TokenType, x: NodeId, y: NodeId) -> NodeId {
    ast.alloc(Node::BinaryExpr(BinaryExpr { op, x, y }))
}

/// `&js.UnaryExpr{op, x}`
#[inline]
pub(crate) fn new_unary(ast: &mut Ast, op: TokenType, x: NodeId) -> NodeId {
    ast.alloc(Node::UnaryExpr(UnaryExpr { op, x }))
}

/// `&js.ExprStmt{Value: v}`
#[inline]
pub(crate) fn new_expr_stmt(ast: &mut Ast, value: NodeId) -> NodeId {
    ast.alloc(Node::ExprStmt(ExprStmt { value }))
}

/// `&js.CondExpr{cond, x, y}`
#[inline]
pub(crate) fn new_cond(ast: &mut Ast, cond: NodeId, x: NodeId, y: NodeId) -> NodeId {
    ast.alloc(Node::CondExpr(CondExpr { cond, x, y }))
}

////////////////////////////////////////////////////////////////

// Go: util.go:isEmptyStmt
pub(crate) fn is_empty_stmt(ast: &Ast, stmt: NodeId) -> bool {
    if stmt.is_nil() {
        return true;
    }
    match ast.node(stmt) {
        Node::EmptyStmt => true,
        Node::BlockStmt(block) => {
            for &item in &block.list {
                if !is_empty_stmt(ast, item) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

// Go: util.go:isFlowStmt
pub(crate) fn is_flow_stmt(ast: &Ast, stmt: NodeId) -> bool {
    matches!(
        ast.node(stmt),
        Node::ReturnStmt(_) | Node::ThrowStmt(_) | Node::BranchStmt(_)
    )
}

// Go: util.go:lastStmt
pub(crate) fn last_stmt(ast: &Ast, stmt: NodeId) -> NodeId {
    if let Node::BlockStmt(block) = ast.node(stmt) {
        if 0 < block.list.len() {
            return last_stmt(ast, block.list[block.list.len() - 1]);
        }
    }
    stmt
}

// Go: util.go:endsInIf
/// Note: like Go, this runs `optimizeStmt` on a trailing `if` without
/// `else`, which rewrites that statement in place.
pub(crate) fn ends_in_if(ast: &mut Ast, istmt: NodeId) -> bool {
    match ast.node(istmt) {
        Node::IfStmt(stmt) => {
            if stmt.else_.is_nil() {
                let r = crate::stmtlist::optimize_stmt(ast, istmt);
                return matches!(ast.node(r), Node::IfStmt(_));
            }
            let e = stmt.else_;
            ends_in_if(ast, e)
        }
        Node::BlockStmt(stmt) => {
            if 0 < stmt.list.len() {
                let last = stmt.list[stmt.list.len() - 1];
                return ends_in_if(ast, last);
            }
            false
        }
        Node::LabelledStmt(stmt) => {
            let v = stmt.value;
            ends_in_if(ast, v)
        }
        Node::WithStmt(stmt) => {
            let b = stmt.body;
            ends_in_if(ast, b)
        }
        Node::WhileStmt(stmt) => {
            let b = stmt.body;
            ends_in_if(ast, b)
        }
        Node::ForStmt(stmt) => {
            let b = stmt.body;
            ends_in_if(ast, b)
        }
        Node::ForInStmt(stmt) => {
            let b = stmt.body;
            ends_in_if(ast, b)
        }
        Node::ForOfStmt(stmt) => {
            let b = stmt.body;
            ends_in_if(ast, b)
        }
        _ => false,
    }
}

////////////////////////////////////////////////////////////////
// Precedence maps (Go maps: a missing key yields the zero value, OpExpr).

// Go: util.go:unaryPrecMap — precedence inside the operation
pub(crate) fn unary_prec_map(tt: TokenType) -> OpPrec {
    match tt {
        PostIncrToken | PostDecrToken => OpLHS,
        PreIncrToken | PreDecrToken | NotToken | BitNotToken | TypeofToken | VoidToken
        | DeleteToken | PosToken | NegToken | AwaitToken => OpUnary,
        _ => OpExpr,
    }
}

// Go: util.go:binaryLeftPrecMap
pub(crate) fn binary_left_prec_map(tt: TokenType) -> OpPrec {
    match tt {
        EqToken | MulEqToken | DivEqToken | ModEqToken | ExpEqToken | AddEqToken | SubEqToken
        | LtLtEqToken | GtGtEqToken | GtGtGtEqToken | BitAndEqToken | BitXorEqToken
        | BitOrEqToken => OpLHS,
        ExpToken => OpUpdate,
        MulToken | DivToken | ModToken => OpMul,
        AddToken | SubToken => OpAdd,
        LtLtToken | GtGtToken | GtGtGtToken => OpShift,
        LtToken | LtEqToken | GtToken | GtEqToken | InToken | InstanceofToken => OpCompare,
        EqEqToken | NotEqToken | EqEqEqToken | NotEqEqToken => OpEquals,
        BitAndToken => OpBitAnd,
        BitXorToken => OpBitXor,
        BitOrToken => OpBitOr,
        AndToken => OpAnd,
        OrToken => OpOr,
        NullishToken => OpBitOr, // or OpCoalesce
        CommaToken => OpExpr,
        _ => OpExpr,
    }
}

// Go: util.go:binaryRightPrecMap
pub(crate) fn binary_right_prec_map(tt: TokenType) -> OpPrec {
    match tt {
        EqToken | MulEqToken | DivEqToken | ModEqToken | ExpEqToken | AddEqToken | SubEqToken
        | LtLtEqToken | GtGtEqToken | GtGtGtEqToken | BitAndEqToken | BitXorEqToken
        | BitOrEqToken => OpAssign,
        ExpToken => OpExp,
        MulToken | DivToken | ModToken => OpExp,
        AddToken | SubToken => OpMul,
        LtLtToken | GtGtToken | GtGtGtToken => OpAdd,
        LtToken | LtEqToken | GtToken | GtEqToken | InToken | InstanceofToken => OpShift,
        EqEqToken | NotEqToken | EqEqEqToken | NotEqEqToken => OpCompare,
        BitAndToken => OpEquals,
        BitXorToken => OpBitAnd,
        BitOrToken => OpBitXor,
        AndToken => OpAnd,       // changes order in AST but not in execution
        OrToken => OpOr,         // changes order in AST but not in execution
        NullishToken => OpBitOr, // or OpCoalesce
        CommaToken => OpAssign,
        _ => OpExpr,
    }
}

// Go: util.go:unaryOpPrecMap — precedence of the operation itself
pub(crate) fn unary_op_prec_map(tt: TokenType) -> OpPrec {
    match tt {
        PostIncrToken | PostDecrToken | PreIncrToken | PreDecrToken => OpUpdate,
        NotToken | BitNotToken | TypeofToken | VoidToken | DeleteToken | PosToken | NegToken
        | AwaitToken => OpUnary,
        _ => OpExpr,
    }
}

// Go: util.go:binaryOpPrecMap
pub(crate) fn binary_op_prec_map(tt: TokenType) -> OpPrec {
    match tt {
        EqToken | MulEqToken | DivEqToken | ModEqToken | ExpEqToken | AddEqToken | SubEqToken
        | LtLtEqToken | GtGtEqToken | GtGtGtEqToken | BitAndEqToken | BitXorEqToken
        | BitOrEqToken => OpAssign,
        ExpToken => OpExp,
        MulToken | DivToken | ModToken => OpMul,
        AddToken | SubToken => OpAdd,
        LtLtToken | GtGtToken | GtGtGtToken => OpShift,
        LtToken | LtEqToken | GtToken | GtEqToken | InToken | InstanceofToken => OpCompare,
        EqEqToken | NotEqToken | EqEqEqToken | NotEqEqToken => OpEquals,
        BitAndToken => OpBitAnd,
        BitXorToken => OpBitXor,
        BitOrToken => OpBitOr,
        AndToken => OpAnd,
        OrToken => OpOr,
        NullishToken => OpCoalesce,
        CommaToken => OpExpr,
        _ => OpExpr,
    }
}

// Go: util.go:exprPrec
pub(crate) fn expr_prec(ast: &Ast, i: NodeId) -> OpPrec {
    match ast.node(i) {
        Node::Var(_)
        | Node::LiteralExpr(_)
        | Node::ArrayExpr(_)
        | Node::ObjectExpr(_)
        | Node::FuncDecl(_)
        | Node::ClassDecl(_) => OpPrimary,
        Node::UnaryExpr(expr) => unary_op_prec_map(expr.op),
        Node::BinaryExpr(expr) => binary_op_prec_map(expr.op),
        Node::NewExpr(expr) => {
            if expr.args.is_none() {
                return OpNew;
            }
            OpMember
        }
        Node::TemplateExpr(expr) => {
            if expr.tag.is_nil() {
                return OpPrimary;
            }
            expr.prec
        }
        Node::DotExpr(expr) => expr.prec,
        Node::IndexExpr(expr) => expr.prec,
        Node::NewTargetExpr | Node::ImportMetaExpr => OpMember,
        Node::CallExpr(_) => OpCall,
        Node::CondExpr(_) | Node::YieldExpr(_) | Node::ArrowFunc(_) => OpAssign,
        Node::GroupExpr(expr) => expr_prec(ast, expr.x),
        _ => OpExpr, // CommaExpr
    }
}

// Go: util.go:hasSideEffects
pub(crate) fn has_side_effects(ast: &Ast, i: NodeId) -> bool {
    // assume that variable usage and that the index operator themselves have no side effects
    match ast.node(i) {
        Node::Var(_) => true,
        Node::LiteralExpr(_)
        | Node::FuncDecl(_)
        | Node::ClassDecl(_)
        | Node::ArrowFunc(_)
        | Node::NewTargetExpr
        | Node::ImportMetaExpr => false,
        Node::NewExpr(_) | Node::CallExpr(_) | Node::YieldExpr(_) => true,
        Node::GroupExpr(expr) => has_side_effects(ast, expr.x),
        Node::DotExpr(_) => true,
        Node::IndexExpr(_) => true,
        Node::CondExpr(expr) => {
            has_side_effects(ast, expr.cond)
                || has_side_effects(ast, expr.x)
                || has_side_effects(ast, expr.y)
        }
        Node::CommaExpr(expr) => {
            for &item in &expr.list {
                if has_side_effects(ast, item) {
                    return true;
                }
            }
            // Go falls out of the switch here and returns true
            true
        }
        Node::ArrayExpr(expr) => {
            for item in &expr.list {
                if has_side_effects(ast, item.value) {
                    return true;
                }
            }
            false
        }
        Node::ObjectExpr(expr) => {
            for item in &expr.list {
                if has_side_effects(ast, item.value)
                    || item.init.is_some() && has_side_effects(ast, item.init)
                    || match &item.name {
                        Some(name) => name.is_computed() && has_side_effects(ast, name.computed),
                        None => false,
                    }
                {
                    return true;
                }
            }
            false
        }
        Node::TemplateExpr(expr) => {
            if has_side_effects(ast, expr.tag) {
                return true;
            }
            for item in &expr.list {
                if has_side_effects(ast, item.expr) {
                    return true;
                }
            }
            false
        }
        Node::UnaryExpr(expr) => {
            if expr.op == DeleteToken
                || expr.op == PreIncrToken
                || expr.op == PreDecrToken
                || expr.op == PostIncrToken
                || expr.op == PostDecrToken
            {
                return true;
            }
            has_side_effects(ast, expr.x)
        }
        Node::BinaryExpr(expr) => binary_op_prec_map(expr.op) == OpAssign,
        _ => true,
    }
}

// Go: util.go:groupExpr
pub(crate) fn group_expr(ast: &mut Ast, i: NodeId, prec: OpPrec) -> NodeId {
    let prec_inside = expr_prec(ast, i);
    if as_group(ast, i).is_none()
        && prec_inside < prec
        && (prec_inside != OpCoalesce || prec != OpBitOr)
    {
        return new_group(ast, i);
    }
    i
}

// Go: util.go:condExpr
pub(crate) fn cond_expr(ast: &mut Ast, cond: NodeId, x: NodeId, y: NodeId) -> NodeId {
    if let Some(list) = as_comma(ast, cond) {
        let last = list[list.len() - 1];
        let c = group_expr(ast, last, OpCoalesce);
        let gx = group_expr(ast, x, OpAssign);
        let gy = group_expr(ast, y, OpAssign);
        let ce = new_cond(ast, c, gx, gy);
        let l = &mut comma_mut(ast, cond).list;
        let n = l.len();
        l[n - 1] = ce;
        return cond;
    }
    let c = group_expr(ast, cond, OpCoalesce);
    let gx = group_expr(ast, x, OpAssign);
    let gy = group_expr(ast, y, OpAssign);
    new_cond(ast, c, gx, gy)
}

// Go: util.go:commaExpr
pub(crate) fn comma_expr(ast: &mut Ast, x: NodeId, y: NodeId) -> NodeId {
    let comma = if as_comma(ast, x).is_some() {
        x
    } else {
        ast.alloc(Node::CommaExpr(CommaExpr { list: vec![x] }))
    };
    if let Some(list2) = as_comma(ast, y) {
        let list2 = list2.clone();
        comma_mut(ast, comma).list.extend(list2);
    } else {
        comma_mut(ast, comma).list.push(y);
    }
    comma
}

// Go: util.go:innerExpr
pub(crate) fn inner_expr(ast: &Ast, mut i: NodeId) -> NodeId {
    loop {
        if let Some(x) = as_group(ast, i) {
            i = x;
        } else {
            return i;
        }
    }
}

// Go: util.go:finalExpr
pub(crate) fn final_expr(ast: &Ast, i: NodeId) -> NodeId {
    let mut i = inner_expr(ast, i);
    if let Some(list) = as_comma(ast, i) {
        i = list[list.len() - 1];
    }
    if let Some((op, x, _)) = as_binary(ast, i) {
        if op == EqToken {
            i = x; // return first
        }
    }
    i
}

// Go: util.go:isTrue
pub(crate) fn is_true(ast: &Ast, i: NodeId) -> bool {
    let i = inner_expr(ast, i);
    if let Some(lit) = as_literal(ast, i) {
        if lit.token_type == TrueToken {
            return true;
        }
    }
    if let Some((op, x)) = as_unary(ast, i) {
        if op == NotToken {
            let (ret, _) = is_falsy(ast, x);
            return ret;
        }
    }
    false
}

// Go: util.go:isFalse
pub(crate) fn is_false(ast: &Ast, i: NodeId) -> bool {
    let i = inner_expr(ast, i);
    if let Some(lit) = as_literal(ast, i) {
        return lit.token_type == FalseToken;
    } else if let Some((op, x)) = as_unary(ast, i) {
        if op == NotToken {
            let (ret, _) = is_truthy(ast, x);
            return ret;
        }
    }
    false
}

// Go: util.go:isEqualExpr
pub(crate) fn is_equal_expr(ast: &Ast, a: NodeId, b: NodeId) -> bool {
    let a = inner_expr(ast, a);
    let b = inner_expr(ast, b);
    if ast.is_var(a) && ast.is_var(b) {
        return ast.var_name(a) == ast.var_name(b);
    }
    // TODO: use reflect.DeepEqual?
    false
}

// Go: util.go:toNullishExpr
pub(crate) fn to_nullish_expr(ast: &mut Ast, cond_expr: NodeId) -> Option<NodeId> {
    let (cond, cx, cy) = match ast.node(cond_expr) {
        Node::CondExpr(c) => (c.cond, c.x, c.y),
        _ => unreachable!(),
    };
    if let Some((v, not)) = is_undefined_or_null_var(ast, cond) {
        let (mut left, mut right) = (cx, cy);
        if not {
            std::mem::swap(&mut left, &mut right);
        }
        if is_equal_expr(ast, v, right) {
            // convert conditional expression to nullish:  a==null?b:a  =>  a??b
            let l = group_expr(ast, right, binary_left_prec_map(NullishToken));
            let r = group_expr(ast, left, binary_right_prec_map(NullishToken));
            return Some(new_binary(ast, NullishToken, l, r));
        } else if is_undefined(ast, left) {
            // convert conditional expression to optional expr:  a==null?undefined:a.b  =>  a?.b
            let mut expr = right;
            let mut parent = NodeId::NIL;
            loop {
                let prev_expr = expr;
                match ast.node(expr) {
                    Node::CallExpr(c) => expr = c.x,
                    Node::DotExpr(d) => expr = d.x,
                    Node::IndexExpr(ix) => expr = ix.x,
                    Node::TemplateExpr(t) => expr = t.tag,
                    _ => break,
                }
                parent = prev_expr;
            }
            if parent.is_some() && is_equal_expr(ast, v, expr) {
                match ast.node_mut(parent) {
                    Node::CallExpr(c) => c.optional = true,
                    Node::DotExpr(d) => d.optional = true,
                    Node::IndexExpr(ix) => ix.optional = true,
                    Node::TemplateExpr(t) => t.optional = true,
                    _ => {}
                }
                return Some(right);
            }
        }
    }
    None
}

// Go: util.go:isUndefinedOrNullVar
/// Returns `(var, not)` when ok.
pub(crate) fn is_undefined_or_null_var(ast: &Ast, i: NodeId) -> Option<(NodeId, bool)> {
    let i = inner_expr(ast, i);
    let binary = as_binary(ast, i);
    if let Some((bop, bx, by)) = binary.filter(|b| b.0 == OrToken || b.0 == AndToken) {
        let mut eq_eq_op = EqEqToken;
        let mut eq_eq_eq_op = EqEqEqToken;
        if bop == AndToken {
            eq_eq_op = NotEqToken;
            eq_eq_eq_op = NotEqEqToken;
        }

        let left = as_binary(ast, inner_expr(ast, bx));
        let right = as_binary(ast, inner_expr(ast, by));
        if let (Some((lop, lx, ly)), Some((rop, rx, ry))) = (left, right) {
            if (lop == eq_eq_op || lop == eq_eq_eq_op) && (rop == eq_eq_op || rop == eq_eq_eq_op) {
                let mut left_var = NodeId::NIL;
                let mut right_var = NodeId::NIL;
                if ast.is_var(lx) && is_undefined_or_null(ast, ly) {
                    left_var = lx;
                } else if ast.is_var(ly) && is_undefined_or_null(ast, lx) {
                    left_var = ly;
                }
                if ast.is_var(rx) && is_undefined_or_null(ast, ry) {
                    right_var = rx;
                } else if ast.is_var(ry) && is_undefined_or_null(ast, rx) {
                    right_var = ry;
                }
                if left_var.is_some() && left_var == right_var {
                    return Some((left_var, bop == AndToken));
                }
            }
        }
    } else if let Some((bop, bx, by)) = binary.filter(|b| b.0 == EqEqToken || b.0 == NotEqToken) {
        let mut variable = NodeId::NIL;
        if ast.is_var(bx) && is_undefined_or_null(ast, by) {
            variable = bx;
        } else if ast.is_var(by) && is_undefined_or_null(ast, bx) {
            variable = by;
        }
        if variable.is_some() {
            return Some((variable, bop == NotEqToken));
        }
    }
    None
}

// Go: util.go:isUndefinedOrNull
pub(crate) fn is_undefined_or_null(ast: &Ast, i: NodeId) -> bool {
    let i = inner_expr(ast, i);
    if let Some(lit) = as_literal(ast, i) {
        return lit.token_type == NullToken;
    }
    is_undefined(ast, i)
}

// Go: util.go:isUndefined
pub(crate) fn is_undefined(ast: &Ast, i: NodeId) -> bool {
    let i = inner_expr(ast, i);
    if ast.is_var(i) {
        if ast.var_name(i).equal(UNDEFINED_BYTES) {
            // TODO: only if not defined
            return true;
        }
    } else if let Some((op, x)) = as_unary(ast, i) {
        if op == VoidToken {
            return !has_side_effects(ast, x);
        }
    }
    false
}

// Go: util.go:isTruthy
/// Returns whether truthy and whether it could be coerced to a boolean (i.e.
/// when returns (false,true) this means it is falsy).
pub(crate) fn is_truthy(ast: &Ast, i: NodeId) -> (bool, bool) {
    let (falsy, ok) = is_falsy(ast, i);
    if ok {
        return (!falsy, true);
    }
    (false, false)
}

// Go: util.go:isFalsy
/// Returns whether falsy and whether it could be coerced to a boolean (i.e.
/// when returns (false,true) this means it is truthy).
pub(crate) fn is_falsy(ast: &Ast, i: NodeId) -> (bool, bool) {
    let mut i = i;
    let mut negated = false;
    loop {
        if let Some(x) = as_group(ast, i) {
            i = x;
        } else if let Some((op, x)) = as_unary(ast, i) {
            if op != NotToken {
                break;
            }
            i = x;
            negated = !negated;
        } else {
            break;
        }
    }
    if let Some(lit) = as_literal(ast, i) {
        let tt = lit.token_type;
        let d = &lit.data;
        if tt == FalseToken || tt == NullToken || tt == StringToken && lit.data.len() == 0 {
            return (!negated, true); // falsy
        } else if tt == TrueToken || tt == StringToken {
            return (negated, true); // truthy
        } else if tt == DecimalToken
            || tt == BinaryToken
            || tt == OctalToken
            || tt == HexadecimalToken
            || tt == IntegerToken
        {
            for c in d.iter() {
                if c == b'e' || c == b'E' || c == b'n' {
                    break;
                } else if c != b'0'
                    && c != b'.'
                    && c != b'x'
                    && c != b'X'
                    && c != b'b'
                    && c != b'B'
                    && c != b'o'
                    && c != b'O'
                {
                    return (negated, true); // truthy
                }
            }
            return (!negated, true); // falsy
        }
    } else if is_undefined(ast, i) {
        return (!negated, true); // falsy
    } else if ast.is_var(i) && ast.var_name(i).equal(NAN_BYTES) {
        return (!negated, true); // falsy
    }
    (false, false) // unknown
}

// Go: util.go:isBooleanExpr
pub(crate) fn is_boolean_expr(ast: &Ast, expr: NodeId) -> bool {
    match ast.node(expr) {
        Node::UnaryExpr(u) => u.op == NotToken,
        Node::BinaryExpr(b) => {
            let op = binary_op_prec_map(b.op);
            if op == OpAnd || op == OpOr {
                return is_boolean_expr(ast, b.x) && is_boolean_expr(ast, b.y);
            }
            op == OpCompare || op == OpEquals
        }
        Node::LiteralExpr(l) => l.token_type == TrueToken || l.token_type == FalseToken,
        Node::GroupExpr(g) => is_boolean_expr(ast, g.x),
        _ => false,
    }
}

// Go: util.go:invertBooleanOp
pub(crate) fn invert_boolean_op(op: TokenType) -> TokenType {
    if op == EqEqToken {
        NotEqToken
    } else if op == NotEqToken {
        EqEqToken
    } else if op == EqEqEqToken {
        NotEqEqToken
    } else if op == NotEqEqToken {
        EqEqEqToken
    } else {
        ErrorToken
    }
}

// Go: util.go:optimizeBooleanExpr
pub(crate) fn optimize_boolean_expr(
    ast: &mut Ast,
    expr: NodeId,
    invert: bool,
    prec: OpPrec,
) -> NodeId {
    if invert {
        // unary !(boolean) has already been handled
        if let Some((op, _, _)) = as_binary(ast, expr) {
            if binary_op_prec_map(op) == OpEquals {
                binary_mut(ast, expr).op = invert_boolean_op(op);
                return expr;
            }
        }
        let g = group_expr(ast, expr, OpUnary);
        let u = new_unary(ast, NotToken, g);
        optimize_unary_expr(ast, u, prec)
    } else if is_boolean_expr(ast, expr) {
        group_expr(ast, expr, prec)
    } else {
        let g = group_expr(ast, expr, OpUnary);
        let u = new_unary(ast, NotToken, g);
        new_unary(ast, NotToken, u)
    }
}

// Go: util.go:optimizeUnaryExpr
pub(crate) fn optimize_unary_expr(ast: &mut Ast, expr: NodeId, prec: OpPrec) -> NodeId {
    let (eop, ex) = as_unary(ast, expr).unwrap();
    if eop == NotToken {
        let mut invert = true;
        let mut expr2 = ex;
        loop {
            if let Some((op, x)) = as_unary(ast, expr2).filter(|u| u.0 == NotToken) {
                let _ = op;
                invert = !invert;
                expr2 = x;
            } else if let Some(x) = as_group(ast, expr2) {
                expr2 = x;
            } else {
                break;
            }
        }
        if !invert && is_boolean_expr(ast, expr2) {
            return group_expr(ast, expr2, prec);
        } else if let Some((bop, bx, by)) = as_binary(ast, expr2).filter(|_| invert) {
            let binary = expr2;
            if binary_op_prec_map(bop) == OpEquals {
                binary_mut(ast, binary).op = invert_boolean_op(bop);
                return group_expr(ast, binary, prec);
            } else if bop == AndToken || bop == OrToken {
                let mut op = AndToken;
                if bop == AndToken {
                    op = OrToken;
                }
                let prec_inside = binary_op_prec_map(op);
                let needs_group =
                    prec_inside < prec && (prec_inside != OpCoalesce || prec != OpBitOr);

                // rewrite !(a||b) to !a&&!b
                // rewrite !(a==0||b==0) to a!=0&&b!=0
                let mut score: i64 = 3; // savings if rewritten (group parentheses and not-token)
                if needs_group {
                    score -= 2;
                }
                score -= 2; // add two not-tokens for left and right

                // == and === can become != and !==
                let mut is_eq_x = false;
                let mut is_eq_y = false;
                if let Some((xop, _, _)) = as_binary(ast, bx) {
                    if binary_op_prec_map(xop) == OpEquals {
                        score += 1;
                        is_eq_x = true;
                    }
                }
                if let Some((yop, _, _)) = as_binary(ast, by) {
                    if binary_op_prec_map(yop) == OpEquals {
                        score += 1;
                        is_eq_y = true;
                    }
                }

                // add group if it wasn't already there
                let mut needs_group_x = false;
                let mut needs_group_y = false;
                if !is_eq_x
                    && binary_left_prec_map(bop) <= expr_prec(ast, bx)
                    && expr_prec(ast, bx) < OpUnary
                {
                    score -= 2;
                    needs_group_x = true;
                }
                if !is_eq_y
                    && binary_right_prec_map(bop) <= expr_prec(ast, by)
                    && expr_prec(ast, by) < OpUnary
                {
                    score -= 2;
                    needs_group_y = true;
                }

                // remove group
                if op == OrToken {
                    if expr_prec(ast, bx) == OpOr {
                        score += 2;
                    }
                    if expr_prec(ast, by) == OpAnd {
                        score += 2;
                    }
                }

                if 0 < score {
                    binary_mut(ast, binary).op = op;
                    if is_eq_x {
                        let x = binary_mut(ast, binary).x;
                        let xb = binary_mut(ast, x);
                        xb.op = invert_boolean_op(xb.op);
                    }
                    if is_eq_y {
                        let y = binary_mut(ast, binary).y;
                        let yb = binary_mut(ast, y);
                        yb.op = invert_boolean_op(yb.op);
                    }
                    if needs_group_x {
                        let x = binary_mut(ast, binary).x;
                        let g = new_group(ast, x);
                        binary_mut(ast, binary).x = g;
                    }
                    if needs_group_y {
                        let y = binary_mut(ast, binary).y;
                        let g = new_group(ast, y);
                        binary_mut(ast, binary).y = g;
                    }
                    if !is_eq_x {
                        let x = binary_mut(ast, binary).x;
                        let u = new_unary(ast, NotToken, x);
                        binary_mut(ast, binary).x = u;
                    }
                    if !is_eq_y {
                        let y = binary_mut(ast, binary).y;
                        let u = new_unary(ast, NotToken, y);
                        binary_mut(ast, binary).y = u;
                    }
                    if needs_group {
                        return new_group(ast, binary);
                    }
                    return binary;
                }
            }
        }
    }
    expr
}

// Go: util.go:jsMinifier.optimizeCondExpr (only reads m.o.minVersion)
pub(crate) fn optimize_cond_expr(
    ast: &mut Ast,
    expr: NodeId,
    prec: OpPrec,
    min_version_2020: bool,
) -> NodeId {
    // remove double negative !! in condition, or switch cases for single negative !
    let cond0 = match ast.node(expr) {
        Node::CondExpr(c) => c.cond,
        _ => unreachable!(),
    };
    if let Some((op1, x1)) = as_unary(ast, cond0) {
        if op1 == NotToken {
            if let Some((op2, x2)) = as_unary(ast, x1).filter(|u| u.0 == NotToken) {
                let _ = op2;
                if is_boolean_expr(ast, x2) {
                    cond_mut(ast, expr).cond = x2;
                }
            } else {
                let c = cond_mut(ast, expr);
                c.cond = x1;
                std::mem::swap(&mut c.x, &mut c.y);
            }
        }
    }

    let (ecnd, ex, ey) = match ast.node(expr) {
        Node::CondExpr(c) => (c.cond, c.x, c.y),
        _ => unreachable!(),
    };
    let final_cond = final_expr(ast, ecnd);
    let (truthy, ok) = is_truthy(ast, ecnd);
    if truthy && ok {
        // if condition is truthy
        return ex;
    } else if !truthy && ok {
        // if condition is falsy
        return ey;
    } else if is_equal_expr(ast, final_cond, ex)
        && (expr_prec(ast, final_cond) < OpAssign
            || binary_left_prec_map(OrToken) <= expr_prec(ast, final_cond))
        && (expr_prec(ast, ey) < OpAssign || binary_right_prec_map(OrToken) <= expr_prec(ast, ey))
    {
        // if condition is equal to true body
        // for higher prec we need to add group parenthesis, and for lower prec we have parenthesis anyways. This only is shorter if len(expr.X) >= 3. isEqualExpr only checks for literal variables, which is a name will be minified to a one or two character name.
        let l = group_expr(ast, ecnd, binary_left_prec_map(OrToken));
        return new_binary(ast, OrToken, l, ey);
    } else if is_equal_expr(ast, final_cond, ey)
        && (expr_prec(ast, final_cond) < OpAssign
            || binary_left_prec_map(AndToken) <= expr_prec(ast, final_cond))
        && (expr_prec(ast, ex) < OpAssign || binary_right_prec_map(AndToken) <= expr_prec(ast, ex))
    {
        // if condition is equal to false body
        let l = group_expr(ast, ecnd, binary_left_prec_map(AndToken));
        return new_binary(ast, AndToken, l, ex);
    } else if is_equal_expr(ast, ex, ey) {
        // if true and false bodies are equal
        let c = ast.alloc(Node::CommaExpr(CommaExpr {
            list: vec![ecnd, ex],
        }));
        return group_expr(ast, c, prec);
    } else {
        if min_version_2020 {
            if let Some(nullish_expr) = to_nullish_expr(ast, expr) {
                // no need to check whether left/right need to add groups, as the space saving is always more
                return nullish_expr;
            }
        }
        let call_x = match ast.node(ex) {
            Node::CallExpr(c) => Some((c.x, c.args.list.clone())),
            _ => None,
        };
        let call_y = match ast.node(ey) {
            Node::CallExpr(c) => Some((c.x, c.args.list.clone())),
            _ => None,
        };
        if let (Some((cxx, cxargs)), Some((cyx, cyargs))) = (&call_x, &call_y) {
            if cxargs.len() == 1
                && cyargs.len() == 1
                && !cxargs[0].rest
                && !cyargs[0].rest
                && is_equal_expr(ast, *cxx, *cyx)
            {
                let c = cond_mut(ast, expr);
                c.x = cxargs[0].value;
                c.y = cyargs[0].value;
                return ast.alloc(Node::CallExpr(CallExpr {
                    x: *cxx,
                    args: Args {
                        list: vec![Arg {
                            value: expr,
                            rest: false,
                        }],
                    },
                    optional: false,
                })); // recompress the conditional expression inside
            }
        }

        // shorten when true and false bodies are true and false
        let (true_x, false_x) = (is_true(ast, ex), is_false(ast, ex));
        let (true_y, false_y) = (is_true(ast, ey), is_false(ast, ey));
        if true_x && false_y || false_x && true_y {
            return optimize_boolean_expr(ast, ecnd, false_x, prec);
        } else if true_x || true_y {
            // trueX != trueY
            let cond = optimize_boolean_expr(ast, ecnd, true_y, binary_left_prec_map(OrToken));
            if true_y {
                let r = group_expr(ast, ex, binary_right_prec_map(OrToken));
                return new_binary(ast, OrToken, cond, r);
            } else {
                let r = group_expr(ast, ey, binary_right_prec_map(OrToken));
                return new_binary(ast, OrToken, cond, r);
            }
        } else if false_x || false_y {
            // falseX != falseY
            let cond = optimize_boolean_expr(ast, ecnd, false_x, binary_left_prec_map(AndToken));
            if false_x {
                let r = group_expr(ast, ey, binary_right_prec_map(AndToken));
                return new_binary(ast, AndToken, cond, r);
            } else {
                let r = group_expr(ast, ex, binary_right_prec_map(AndToken));
                return new_binary(ast, AndToken, cond, r);
            }
        } else if let Some((ccond, cx, cy)) = match ast.node(ex) {
            Node::CondExpr(c) => Some((c.cond, c.x, c.y)),
            _ => None,
        }
        .filter(|c| is_equal_expr(ast, ey, c.2))
        {
            let _ = cy;
            // nested conditional expression with same false bodies
            let l = group_expr(ast, ecnd, binary_left_prec_map(AndToken));
            let r = group_expr(ast, ccond, binary_right_prec_map(AndToken));
            let b = new_binary(ast, AndToken, l, r);
            return new_cond(ast, b, cx, ey);
        } else if prec <= OpExpr {
            // regular conditional expression
            // convert  (a,b)?c:d  =>  a,b?c:d
            if let Some(gx) = as_group(ast, ecnd) {
                if let Some(list) = as_comma(ast, gx) {
                    let last = list[list.len() - 1];
                    if OpCoalesce <= expr_prec(ast, last) {
                        cond_mut(ast, expr).cond = last;
                        let l = &mut comma_mut(ast, gx).list;
                        let n = l.len();
                        l[n - 1] = expr;
                        return gx; // recompress the conditional expression inside
                    }
                }
            }
        }
    }
    expr
}

// Go: util.go:isHexDigit
#[inline]
pub(crate) fn is_hex_digit(b: u8) -> bool {
    b'0' <= b && b <= b'9' || b'a' <= b && b <= b'f' || b'A' <= b && b <= b'F'
}

// Go: util.go:mergeBinaryExpr
/// Merges string concatenations which may be intertwined with other
/// additions.
pub(crate) fn merge_binary_expr(ast: &mut Ast, expr: NodeId) {
    let mut expr = expr;
    loop {
        let (eop, _, ey) = as_binary(ast, expr).unwrap();
        if eop != AddToken {
            break;
        }
        if let Some(lit) = as_literal(ast, ey).filter(|l| l.token_type == StringToken) {
            let mut left = expr;
            let mut strings: Vec<NodeId> = vec![ey];
            let mut n: isize = lit.data.len() as isize - 2;
            loop {
                let (lop, lx, _) = as_binary(ast, left).unwrap();
                if lop != AddToken {
                    break;
                }
                if 50 < strings.len() {
                    return; // limit recursion
                }
                if let Some(lit) = as_literal(ast, lx).filter(|l| l.token_type == StringToken) {
                    n += lit.data.len() as isize - 2;
                    strings.push(lx);
                    binary_mut(ast, left).x = NodeId::NIL;
                } else if let Some((_, _, nly)) = as_binary(ast, lx) {
                    if let Some(lit) = as_literal(ast, nly).filter(|l| l.token_type == StringToken)
                    {
                        n += lit.data.len() as isize - 2;
                        strings.push(nly);
                        left = lx;
                        continue;
                    }
                }
                break;
            }

            if 1 < strings.len() {
                // unescaped quotes will be repaired in minifyString later on
                let data = |ast: &Ast, id: NodeId| ast.literal(id).data.clone();
                let mut b = GoBytes::make(0, (n + 2) as usize);
                let last = data(ast, strings[strings.len() - 1]);
                b = b.append_bytes(&last.slice_to(last.len() - 1));
                let mut i = strings.len() as isize - 2;
                while 0 < i {
                    let s = data(ast, strings[i as usize]);
                    b = b.append_bytes(&s.slice(1, s.len() - 1));
                    i -= 1;
                }
                let first = data(ast, strings[0]);
                b = b.append_bytes(&first.slice_from(1));
                b.set(b.len() - 1, b.at(0));

                let lx = binary_mut(ast, left).x;
                binary_mut(ast, expr).x = lx;
                let y = binary_mut(ast, expr).y;
                ast.literal_mut(y).data = b;
            }
        }
        let x = as_binary(ast, expr).unwrap().1;
        if as_binary(ast, x).is_some() {
            expr = x;
        } else {
            break;
        }
    }
}

// Go: util.go:minifyString
pub(crate) fn minify_string(b: GoBytes, allow_template: bool) -> GoBytes {
    if b.len() < 3 {
        return GoBytes::from_slice(b"\"\"");
    }

    // switch quotes if more optimal
    let mut single_quotes = 0i64;
    let mut double_quotes = 0i64;
    let mut backtick_quotes = 0i64;
    let mut newlines = 0i64;
    let mut dollar_signs = 0i64;
    let n = b.len();
    let at = |k: usize| b.at(k);
    let mut i = 1usize;
    while i < n - 1 {
        if at(i) == b'\'' {
            single_quotes += 1;
        } else if at(i) == b'"' {
            double_quotes += 1;
        } else if at(i) == b'`' {
            backtick_quotes += 1;
        } else if at(i) == b'$' && i + 1 < n && at(i + 1) == b'{' {
            dollar_signs += 1;
        } else if at(i) == b'\\' && i + 1 < n {
            if at(i + 1) == b'n' {
                newlines += 1;
            } else if b'1' <= at(i + 1) && at(i + 1) <= b'9' && i + 2 < n {
                if at(i + 1) == b'1' && at(i + 2) == b'2' {
                    newlines += 1;
                } else if at(i + 1) == b'4' && at(i + 2) == b'2' {
                    double_quotes += 1;
                } else if at(i + 1) == b'4' && at(i + 2) == b'7' {
                    single_quotes += 1;
                } else if i + 3 < n && at(i + 1) == b'1' && at(i + 2) == b'4' && at(i + 3) == b'0' {
                    backtick_quotes += 1;
                }
            } else if at(i + 1) == b'x' && i + 3 < n {
                if at(i + 2) == b'0' && at(i + 3) | 0x20 == b'a' {
                    newlines += 1;
                } else if at(i + 2) == b'2' && at(i + 3) == b'2' {
                    double_quotes += 1;
                } else if at(i + 2) == b'2' && at(i + 3) == b'7' {
                    single_quotes += 1;
                } else if at(i + 2) == b'6' && at(i + 3) == b'0' {
                    backtick_quotes += 1;
                }
            } else if at(i + 1) == b'u' && i + 5 < n && at(i + 2) == b'0' && at(i + 3) == b'0' {
                if at(i + 4) == b'0' && at(i + 5) | 0x20 == b'a' {
                    newlines += 1;
                } else if at(i + 4) == b'2' && at(i + 5) == b'2' {
                    double_quotes += 1;
                } else if at(i + 4) == b'2' && at(i + 5) == b'7' {
                    single_quotes += 1;
                } else if at(i + 4) == b'6' && at(i + 5) == b'0' {
                    backtick_quotes += 1;
                }
            } else if at(i + 1) == b'u' && i + 4 < n && at(i + 2) == b'{' {
                let mut j = i + 3;
                while j < n && at(j) == b'0' {
                    j += 1;
                }
                if j + 1 < n && at(j) | 0x20 == b'a' && at(j + 1) == b'}' {
                    newlines += 1;
                } else if j + 2 < n && at(j + 2) == b'}' {
                    if at(j) == b'2' && at(j + 1) == b'2' {
                        double_quotes += 1;
                    } else if at(j) == b'2' && at(j + 1) == b'7' {
                        single_quotes += 1;
                    } else if at(j) == b'6' && at(j + 1) == b'0' {
                        backtick_quotes += 1;
                    }
                }
            }
        }
        i += 1;
    }
    let mut quote = b'"'; // default to " for better GZIP compression
    let mut quotes = double_quotes;
    if double_quotes < single_quotes {
        quote = b'"';
    } else if single_quotes < double_quotes {
        quote = b'\'';
        quotes = single_quotes;
    }
    if allow_template && backtick_quotes + dollar_signs < quotes + newlines {
        quote = b'`';
    }
    b.set(0, quote);
    b.set(n - 1, quote);

    // strip unnecessary escapes
    replace_escapes(b, quote, 1, 1)
}

// Go: util.go:replaceEscapes
pub(crate) fn replace_escapes(b: GoBytes, quote: u8, prefix: isize, suffix: isize) -> GoBytes {
    let mut b = b;
    // strip unnecessary escapes
    let mut j: isize = 0;
    let mut start: isize = 0;
    let len = |b: &GoBytes| b.len() as isize;
    let at = |b: &GoBytes, k: isize| b.at(k as usize);
    let set = |b: &GoBytes, k: isize, c: u8| b.set(k as usize, c);
    let mut i: isize = prefix;
    while i < len(&b) - suffix {
        let mut c = at(&b, i);
        if c == b'\\' {
            c = at(&b, i + 1);
            if c == quote
                || c == b'\\'
                || c == b'r'
                || quote != b'`' && c == b'n'
                || c == b'0'
                    && (len(&b) - suffix <= i + 2 || at(&b, i + 2) < b'0' || b'7' < at(&b, i + 2))
            {
                // keep escape sequence
                i += 1;
                i += 1;
                continue;
            }
            let mut n: isize = 1; // number of characters to skip
            if c == b'\n'
                || c == b'\r'
                || c == 0xE2
                    && i + 3 < len(&b) - 1
                    && at(&b, i + 2) == 0x80
                    && (at(&b, i + 3) == 0xA8 || at(&b, i + 3) == 0xA9)
            {
                // line continuations
                if c == 0xE2 {
                    n = 4;
                } else if c == b'\r' && i + 2 < len(&b) - 1 && at(&b, i + 2) == b'\n' {
                    n = 3;
                } else {
                    n = 2;
                }
            } else if c == b'x' {
                if i + 3 < len(&b) - 1
                    && is_hex_digit(at(&b, i + 2))
                    && at(&b, i + 2) < b'8'
                    && is_hex_digit(at(&b, i + 3))
                    && (!(at(&b, i + 2) == b'0' && at(&b, i + 3) == b'0')
                        || i + 3 == len(&b)
                        || at(&b, i + 3) != b'\\' && (at(&b, i + 3) < b'0' && b'7' < at(&b, i + 3)))
                {
                    // don't convert \x00 to \0 if it may be an octal number
                    // hexadecimal escapes
                    let v = (unhex(at(&b, i + 2)) << 4) | unhex(at(&b, i + 3));
                    set(&b, i, v);
                    n = 4;
                    let bi = at(&b, i);
                    if bi == b'\\'
                        || bi == quote
                        || bi == b'\r'
                        || quote != b'`' && bi == b'\n'
                        || bi == 0
                    {
                        if bi == b'\n' {
                            set(&b, i + 1, b'n');
                        } else if bi == b'\r' {
                            set(&b, i + 1, b'r');
                        } else {
                            set(&b, i + 1, bi);
                        }
                        set(&b, i, b'\\');
                        i += 1;
                        n -= 1;
                    }
                    i += 1;
                    n -= 1;
                } else {
                    i += 1;
                    i += 1;
                    continue;
                }
            } else if c == b'u' && i + 2 < len(&b) {
                let mut l = i + 2;
                if at(&b, i + 2) == b'{' {
                    l += 1;
                }
                let mut r = l;
                while r < len(&b) && (at(&b, i + 2) == b'{' || r < l + 4) {
                    let br = at(&b, r);
                    if br < b'0' || b'9' < br && br < b'A' || b'F' < br && br < b'a' || b'f' < br {
                        break;
                    }
                    r += 1;
                }
                if at(&b, i + 2) == b'{' && (6 < r - l || len(&b) <= r || at(&b, r) != b'}')
                    || at(&b, i + 2) != b'{' && r - l != 4
                {
                    i += 1;
                    i += 1;
                    continue;
                }
                let digits = b.slice(l as usize, r as usize).to_vec();
                let num = match go_strconv::parse_int(&digits, 16, 32) {
                    Ok(v) if v < 0x10FFFF => v,
                    _ => {
                        i += 1;
                        i += 1;
                        continue;
                    }
                };

                n = 2 + r - l;
                if at(&b, i + 2) == b'{' {
                    n += 2;
                }
                if num == 0 {
                    // don't convert NULL to literal NULL (gives JS parsing problems)
                    if r == len(&b) || at(&b, r) != b'\\' && (at(&b, r) < b'0' && b'7' < at(&b, r))
                    {
                        set(&b, i + 1, b'0');
                        i += 2;
                        n -= 2;
                    } else {
                        // don't convert NULL to \0 (may be an octal number)
                        set(&b, i + 1, b'x');
                        set(&b, i + 2, b'0');
                        set(&b, i + 3, b'0');
                        i += 4;
                        n -= 4;
                    }
                } else if num != 13 && (quote == b'`' || num != 10) {
                    // decode unicode character to UTF-8 and put at the end of the escape sequence
                    // then skip the first part of the escape sequence until the decoded character
                    let m = go_unicode::utf8::rune_len(num as go_unicode::Rune);
                    if m == -1 {
                        i += 1;
                        i += 1;
                        continue;
                    } else if num < 256 && quote == num as u8 {
                        set(&b, i, b'\\');
                        i += 1;
                        n -= 1;
                    }
                    let mut enc = [0u8; 4];
                    let k = go_unicode::utf8::encode_rune(&mut enc, num as go_unicode::Rune);
                    for (q, &e) in enc[..k].iter().enumerate() {
                        set(&b, i + q as isize, e);
                    }
                    i += m;
                    n -= m;
                } else {
                    if num == 10 {
                        set(&b, i + 1, b'n');
                    } else {
                        set(&b, i + 1, b'r');
                    }
                    i += 2;
                    n -= 2;
                }
            } else if b'0' <= c && c <= b'7' {
                // octal escapes (legacy), \0 already handled (quote != `)
                let mut num: u8 = c.wrapping_sub(b'0');
                n += 1;
                if i + 2 < len(&b) - 1 && b'0' <= at(&b, i + 2) && at(&b, i + 2) <= b'7' {
                    num = num
                        .wrapping_mul(8)
                        .wrapping_add(at(&b, i + 2))
                        .wrapping_sub(b'0');
                    n += 1;
                    if num < 32
                        && i + 3 < len(&b) - 1
                        && b'0' <= at(&b, i + 3)
                        && at(&b, i + 3) <= b'7'
                    {
                        num = num
                            .wrapping_mul(8)
                            .wrapping_add(at(&b, i + 3))
                            .wrapping_sub(b'0');
                        n += 1;
                    }
                }
                set(&b, i, num);
                if num == 0
                    || num == b'\\'
                    || num == quote
                    || num == b'\r'
                    || quote != b'`' && num == b'\n'
                {
                    if num == 0 {
                        set(&b, i + 1, b'0');
                    } else if num == b'\n' {
                        set(&b, i + 1, b'n');
                    } else if num == b'\r' {
                        set(&b, i + 1, b'r');
                    } else {
                        let bi = at(&b, i);
                        set(&b, i + 1, bi);
                    }
                    set(&b, i, b'\\');
                    i += 1;
                    n -= 1;
                }
                i += 1;
                n -= 1;
            } else if quote == b'`' && c == b'n' {
                set(&b, i, b'\n');
                i += 1;
            } else if c == b't' {
                set(&b, i, b'\t');
                i += 1;
            } else if c == b'f' {
                set(&b, i, 0x0C);
                i += 1;
            } else if c == b'v' {
                set(&b, i, 0x0B);
                i += 1;
            } else if c == b'b' {
                set(&b, i, 0x08);
                i += 1;
            }
            // remove unnecessary escape character, anything but 0x00, 0x0A, 0x0D, \, ' or "
            if start != 0 {
                j += b
                    .slice_from(j as usize)
                    .copy_from(&b.slice(start as usize, i as usize)) as isize;
            } else {
                j = i;
            }
            start = i + n;
            i += n - 1;
        } else if c == quote
            || c == b'$'
                && quote == b'`'
                && (i + 1 < len(&b) && at(&b, i + 1) == b'{'
                    || i + 2 < len(&b) && at(&b, i + 1) == b'\\' && at(&b, i + 2) == b'{')
        {
            // may not be escaped properly when changing quotes
            if j < start {
                // avoid append
                j += b
                    .slice_from(j as usize)
                    .copy_from(&b.slice(start as usize, i as usize)) as isize;
                set(&b, j, b'\\');
                j += 1;
                start = i;
            } else {
                let head = b.slice_to(i as usize).append_byte(b'\\');
                let tail = b.slice_from(i as usize);
                b = head.append_bytes(&tail);
                i += 1;
                set(&b, i, c); // was overwritten above
            }
        } else if c == b'<' && 9 <= len(&b) - 1 - i {
            if at(&b, i + 1) == b'\\'
                && 10 <= len(&b) - 1 - i
                && b.slice((i + 2) as usize, (i + 10) as usize)
                    .equal(b"/script>")
            {
                i += 9;
            } else if b
                .slice((i + 1) as usize, (i + 9) as usize)
                .equal(b"/script>")
            {
                i += 1;
                if j < start {
                    // avoid append
                    j += b
                        .slice_from(j as usize)
                        .copy_from(&b.slice(start as usize, i as usize))
                        as isize;
                    set(&b, j, b'\\');
                    j += 1;
                    start = i;
                } else {
                    let head = b.slice_to(i as usize).append_byte(b'\\');
                    let tail = b.slice_from(i as usize);
                    b = head.append_bytes(&tail);
                    i += 1;
                    set(&b, i, b'/'); // was overwritten above
                }
            }
        }
        i += 1;
    }
    if start != 0 {
        j += b
            .slice_from(j as usize)
            .copy_from(&b.slice_from(start as usize)) as isize;
        return b.slice_to(j as usize);
    }
    b
}

/// Go `encoding/hex` digit value (only called on validated hex digits).
#[inline]
fn unhex(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

// Go: util.go:regexpEscapeTable
static REGEXP_ESCAPE_TABLE: [bool; 256] = {
    let mut t = [false; 256];
    let set = b"$()*+./0123456789?BDPSW[\\]^bcdfknprstuvwx{|}";
    let mut k = 0;
    while k < set.len() {
        t[set[k] as usize] = true;
        k += 1;
    }
    t
};

// Go: util.go:regexpClassEscapeTable
static REGEXP_CLASS_ESCAPE_TABLE: [bool; 256] = {
    let mut t = [false; 256];
    let set = b"0123456789DPSW\\]bcdfnprstuvwx";
    let mut k = 0;
    while k < set.len() {
        t[set[k] as usize] = true;
        k += 1;
    }
    t
};

// Go: util.go:minifyRegExp
pub(crate) fn minify_reg_exp(b: GoBytes) -> GoBytes {
    let mut b = b;
    let mut in_class = false;
    let mut after_dash = 0i64;
    let mut i_class = 0usize;
    let mut i = 1usize;
    while (i as isize) < b.len() as isize - 1 {
        if in_class {
            after_dash += 1;
        }
        if b.at(i) == b'\\' {
            let c = b.at(i + 1);
            let escape = if in_class {
                REGEXP_CLASS_ESCAPE_TABLE[c as usize]
                    || c == b'-' && 2 < after_dash && i + 2 < b.len() && b.at(i + 2) != b']'
                    || c == b'^' && i == i_class + 1
            } else {
                REGEXP_ESCAPE_TABLE[c as usize]
            };
            if !escape {
                b = b.slice_to(i).append_bytes(&b.slice_from(i + 1));
                if in_class && 2 < after_dash && c == b'-' {
                    after_dash = 0;
                } else if in_class && c == b'^' {
                    after_dash = 1;
                }
            } else {
                i += 1;
            }
        } else if b.at(i) == b'[' {
            if b.at(i + 1) == b'^' {
                i += 1;
            }
            after_dash = 1;
            in_class = true;
            i_class = i;
        } else if in_class && b.at(i) == b']' {
            in_class = false;
        } else if b.at(i) == b'/' {
            break;
        } else if in_class && 2 < after_dash && b.at(i) == b'-' {
            after_dash = 0;
        }
        i += 1;
    }
    b
}

// Go: util.go:removeUnderscoresAndSuffix
pub(crate) fn remove_underscores_and_suffix(b: GoBytes) -> (GoBytes, bool) {
    let mut b = b;
    let mut i: isize = 0;
    while i < b.len() as isize {
        if b.at(i as usize) == b'_' {
            b = b
                .slice_to(i as usize)
                .append_bytes(&b.slice_from(i as usize + 1));
            i -= 1;
        }
        i += 1;
    }
    if 0 < b.len() && b.at(b.len() - 1) == b'n' {
        return (b.slice_to(b.len() - 1), true);
    }
    (b, false)
}

// Go: util.go:decimalNumber
pub(crate) fn decimal_number(b: GoBytes, prec: i64) -> GoBytes {
    let (b, suffix) = remove_underscores_and_suffix(b);
    if suffix {
        return b.append_byte(b'n');
    }
    tdewolff_minify::number(b, prec)
}

/// The shared tail of binaryNumber/octalNumber/hexadecimalNumber.
fn int_to_decimal(b: GoBytes, n: i64, suffix: bool, prec: i64) -> GoBytes {
    let mut n = n;
    let mut i = tdewolff_parse::strconv::len_int(n) as isize - 1;
    let b = b.slice_to((i + 1) as usize);
    while 0 <= i {
        b.set(i as usize, (b'0' as i64 + n % 10) as u8);
        n /= 10;
        i -= 1;
    }
    if suffix {
        return b.append_byte(b'n');
    }
    tdewolff_minify::number(b, prec)
}

// Go: util.go:binaryNumber
pub(crate) fn binary_number(b: GoBytes, prec: i64) -> GoBytes {
    let (b, suffix) = remove_underscores_and_suffix(b);
    if b.len() <= 2 || 65 < b.len() {
        return b;
    }
    let mut n: i64 = 0;
    for c in b.slice_from(2).iter() {
        n = n.wrapping_mul(2);
        n = n.wrapping_add(c.wrapping_sub(b'0') as i64);
    }
    int_to_decimal(b, n, suffix, prec)
}

// Go: util.go:octalNumber
pub(crate) fn octal_number(b: GoBytes, prec: i64) -> GoBytes {
    let (b, suffix) = remove_underscores_and_suffix(b);
    if b.len() <= 2 || 23 < b.len() {
        return b;
    }
    let mut n: i64 = 0;
    for c in b.slice_from(2).iter() {
        n = n.wrapping_mul(8);
        n = n.wrapping_add(c.wrapping_sub(b'0') as i64);
    }
    int_to_decimal(b, n, suffix, prec)
}

// Go: util.go:hexadecimalNumber
pub(crate) fn hexadecimal_number(b: GoBytes, prec: i64) -> GoBytes {
    let (b, suffix) = remove_underscores_and_suffix(b);
    if b.len() <= 2
        || 12 < b.len()
        || b.len() == 12 && (b'D' < b.at(2) && b.at(2) <= b'F' || b'd' < b.at(2))
    {
        return b;
    }
    let mut n: i64 = 0;
    for c in b.slice_from(2).iter() {
        n = n.wrapping_mul(16);
        if c <= b'9' {
            n = n.wrapping_add(c.wrapping_sub(b'0') as i64);
        } else if c <= b'F' {
            n = n.wrapping_add(10 + c.wrapping_sub(b'A') as i64);
        } else {
            n = n.wrapping_add(10 + c.wrapping_sub(b'a') as i64);
        }
    }
    int_to_decimal(b, n, suffix, prec)
}
