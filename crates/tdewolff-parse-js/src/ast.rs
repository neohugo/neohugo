//! Go: parse/v2/js/ast.go — node types, `Var`, `Scope`, `VarArray`,
//! `VarsByUses` and the scope analysis methods.
//!
//! # Arena model
//!
//! Go's AST is a pointer graph: `*Var` values are shared between the scopes
//! and every reference, `*VarDecl` nodes are shared between the tree and
//! `Scope.VarDecls`, `Scope`s are embedded in blocks but referenced through
//! `Parent`/`Func`/`VarDecl.Scope`, and the minifier mutates all of them in
//! place. The port keeps every node that Go handles through a pointer or an
//! interface (`IStmt`, `IExpr`, `IBinding`, `*Var`, `*VarDecl`,
//! `*BlockStmt`, `*MethodDecl`, ...) in one arena ([`Ast::nodes`]) and refers
//! to it by [`NodeId`]; scopes live in [`Ast::scopes`] and are referred to by
//! [`ScopeId`]. Pointer identity is id equality, `nil` is [`NodeId::NIL`] /
//! [`ScopeId::NIL`], and a Go type switch is a `match` on [`Ast::node`]
//! (which yields [`Node::Nil`] for `NIL`, so a nil interface matches no
//! concrete type, as in Go).
//!
//! Go value types (`BindingElement`, `Params`, `PropertyName`, `Property`,
//! `Element`, `Arg`, `Args`, `CaseClause`, `Alias`, `Field`,
//! `ClassElement`, `TemplatePart`, `LiteralExpr` when embedded) are plain
//! Rust values inside their parent node.
//!
//! Deviation: `BlockStmt` values embedded in `FuncDecl`, `MethodDecl` and
//! `ArrowFunc` (`Body BlockStmt`) and the module (`AST.BlockStmt`) are
//! separate `Node::BlockStmt` nodes referenced by id, so every block can be
//! handed around like Go's `&decl.Body`.

use std::fmt;

use tdewolff_parse::GoBytes;

use crate::table::OpPrec;
use crate::tokentype::*;

/// A Go pointer/interface to a node: an index into [`Ast::nodes`];
/// [`NodeId::NIL`] is Go's `nil`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Go `nil`.
    pub const NIL: NodeId = NodeId(0);

    /// `x == nil`
    #[inline]
    pub fn is_nil(self) -> bool {
        self.0 == 0
    }

    /// `x != nil`
    #[inline]
    pub fn is_some(self) -> bool {
        self.0 != 0
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            write!(f, "nil")
        } else {
            write!(f, "#{}", self.0)
        }
    }
}

/// A Go `*Scope`: an index into [`Ast::scopes`]; [`ScopeId::NIL`] is `nil`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ScopeId(pub u32);

impl ScopeId {
    /// Go `nil`.
    pub const NIL: ScopeId = ScopeId(0);

    #[inline]
    pub fn is_nil(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for ScopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            write!(f, "nil")
        } else {
            write!(f, "s{}", self.0)
        }
    }
}

////////////////////////////////////////////////////////////////

/// Go: `type DeclType uint16` — the kind of declaration.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct DeclType(pub u16);

// DeclType values.
pub const NoDecl: DeclType = DeclType(0); // undeclared variables
pub const VariableDecl: DeclType = DeclType(1); // var
pub const FunctionDecl: DeclType = DeclType(2); // function
pub const ArgumentDecl: DeclType = DeclType(3); // function and method arguments
pub const LexicalDecl: DeclType = DeclType(4); // let, const, class
pub const CatchDecl: DeclType = DeclType(5); // catch statement argument
pub const ExprDecl: DeclType = DeclType(6); // function expression name or class expression name

impl DeclType {
    // Go: ast.go:DeclType.String
    pub fn string(self) -> String {
        match self {
            NoDecl => "NoDecl",
            VariableDecl => "VariableDecl",
            FunctionDecl => "FunctionDecl",
            ArgumentDecl => "ArgumentDecl",
            LexicalDecl => "LexicalDecl",
            CatchDecl => "CatchDecl",
            ExprDecl => "ExprDecl",
            _ => return format!("Invalid({})", self.0),
        }
        .to_string()
    }
}

impl fmt::Debug for DeclType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}

/// Go: ast.go:Var — a variable, where Decl is the type of declaration and can
/// be var|function for function scoped variables, let|const|class for block
/// scoped variables.
#[derive(Clone, Debug)]
pub struct Var {
    pub data: GoBytes,
    /// Is set when merging variable uses, as in: `{a} {var a}` where the first
    /// links to the second, only used for undeclared variables (a `*Var`,
    /// `NIL` = nil).
    pub link: NodeId,
    /// Go `uint16`: wraps on overflow.
    pub uses: u16,
    pub decl: DeclType,
}

/// Go: ast.go:Scope — a function or block scope with a list of variables
/// declared and used.
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// Parent is nil for global scope.
    pub parent: ScopeId,
    pub func: ScopeId,
    /// Go `VarArray` (`[]*Var`). Link in Var are always nil.
    pub declared: Vec<NodeId>,
    pub undeclared: Vec<NodeId>,
    /// Go `[]*VarDecl`.
    pub var_decls: Vec<NodeId>,
    /// offset into Declared to mark variables used in for statements
    pub num_for_decls: u16,
    /// offset into Declared to mark variables used in function arguments
    pub num_func_args: u16,
    /// offset into Undeclared to mark variables used in arguments
    pub num_arg_uses: u16,
    pub is_global_or_func: bool,
    pub has_with: bool,
}

////////////////////////////////////////////////////////////////
// Statements

/// Go: ast.go:Comment — comment block or line, usually a bang comment.
#[derive(Clone, Debug)]
pub struct Comment {
    pub value: GoBytes,
}

/// Go: ast.go:BlockStmt — a block statement (embeds `Scope`).
#[derive(Clone, Debug, Default)]
pub struct BlockStmt {
    pub list: Vec<NodeId>,
    pub scope: ScopeId,
}

/// Go: ast.go:ExprStmt — an expression statement.
#[derive(Clone, Debug)]
pub struct ExprStmt {
    pub value: NodeId,
}

/// Go: ast.go:IfStmt — an if statement.
#[derive(Clone, Debug)]
pub struct IfStmt {
    pub cond: NodeId,
    pub body: NodeId,
    /// can be nil
    pub else_: NodeId,
}

/// Go: ast.go:DoWhileStmt — a do-while iteration statement.
#[derive(Clone, Debug)]
pub struct DoWhileStmt {
    pub cond: NodeId,
    pub body: NodeId,
}

/// Go: ast.go:WhileStmt — a while iteration statement.
#[derive(Clone, Debug)]
pub struct WhileStmt {
    pub cond: NodeId,
    pub body: NodeId,
}

/// Go: ast.go:ForStmt — a regular for iteration statement.
#[derive(Clone, Debug)]
pub struct ForStmt {
    /// can be nil
    pub init: NodeId,
    /// can be nil
    pub cond: NodeId,
    /// can be nil
    pub post: NodeId,
    /// `*BlockStmt`
    pub body: NodeId,
}

/// Go: ast.go:ForInStmt — a for-in iteration statement.
#[derive(Clone, Debug)]
pub struct ForInStmt {
    pub init: NodeId,
    pub value: NodeId,
    /// `*BlockStmt`
    pub body: NodeId,
}

/// Go: ast.go:ForOfStmt — a for-of iteration statement.
#[derive(Clone, Debug)]
pub struct ForOfStmt {
    pub await_: bool,
    pub init: NodeId,
    pub value: NodeId,
    /// `*BlockStmt`
    pub body: NodeId,
}

/// Go: ast.go:CaseClause — a case clause or default clause for a switch
/// statement (embeds `TokenType`).
#[derive(Clone, Debug)]
pub struct CaseClause {
    pub token_type: TokenType,
    /// can be nil
    pub cond: NodeId,
    pub list: Vec<NodeId>,
}

/// Go: ast.go:SwitchStmt — a switch statement (embeds `Scope`).
#[derive(Clone, Debug)]
pub struct SwitchStmt {
    pub init: NodeId,
    pub list: Vec<CaseClause>,
    pub scope: ScopeId,
}

/// Go: ast.go:BranchStmt — a continue or break statement.
#[derive(Clone, Debug)]
pub struct BranchStmt {
    pub type_: TokenType,
    /// can be nil
    pub label: GoBytes,
}

/// Go: ast.go:ReturnStmt — a return statement.
#[derive(Clone, Debug)]
pub struct ReturnStmt {
    /// can be nil
    pub value: NodeId,
}

/// Go: ast.go:WithStmt — a with statement.
#[derive(Clone, Debug)]
pub struct WithStmt {
    pub cond: NodeId,
    pub body: NodeId,
}

/// Go: ast.go:LabelledStmt — a labelled statement.
#[derive(Clone, Debug)]
pub struct LabelledStmt {
    pub label: GoBytes,
    pub value: NodeId,
}

/// Go: ast.go:ThrowStmt — a throw statement.
#[derive(Clone, Debug)]
pub struct ThrowStmt {
    pub value: NodeId,
}

/// Go: ast.go:TryStmt — a try statement.
#[derive(Clone, Debug)]
pub struct TryStmt {
    /// `*BlockStmt`
    pub body: NodeId,
    /// can be nil
    pub binding: NodeId,
    /// `*BlockStmt`, can be nil
    pub catch: NodeId,
    /// `*BlockStmt`, can be nil
    pub finally: NodeId,
}

/// Go: ast.go:Alias — a name space import or import/export specifier for
/// import/export statements.
#[derive(Clone, Debug, Default)]
pub struct Alias {
    /// can be nil
    pub name: GoBytes,
    /// can be nil
    pub binding: GoBytes,
}

/// Go: ast.go:ImportStmt — an import statement.
#[derive(Clone, Debug, Default)]
pub struct ImportStmt {
    /// Go `[]Alias`; `None` is a nil slice (observable: `List != nil`).
    pub list: Option<Vec<Alias>>,
    /// can be nil
    pub default: GoBytes,
    pub module: GoBytes,
}

/// Go: ast.go:ExportStmt — an export statement.
#[derive(Clone, Debug, Default)]
pub struct ExportStmt {
    pub list: Vec<Alias>,
    /// can be nil
    pub module: GoBytes,
    pub default: bool,
    pub decl: NodeId,
}

/// Go: ast.go:DirectivePrologueStmt — a string literal at the beginning of a
/// function or module (usually "use strict").
#[derive(Clone, Debug)]
pub struct DirectivePrologueStmt {
    pub value: GoBytes,
}

////////////////////////////////////////////////////////////////
// Bindings and declarations

/// Go: ast.go:PropertyName — a property name for binding properties, method
/// names, and in object literals.
#[derive(Clone, Debug, Default)]
pub struct PropertyName {
    pub literal: LiteralExpr,
    /// can be nil
    pub computed: NodeId,
}

impl PropertyName {
    // Go: ast.go:PropertyName.IsSet
    /// Returns true is PropertyName is not nil.
    pub fn is_set(&self) -> bool {
        self.is_computed() || self.literal.token_type != ErrorToken
    }

    // Go: ast.go:PropertyName.IsComputed
    /// Returns true if PropertyName is computed.
    pub fn is_computed(&self) -> bool {
        self.computed.is_some()
    }

    // Go: ast.go:PropertyName.IsIdent
    /// Returns true if PropertyName equals the given identifier name.
    pub fn is_ident(&self, data: &GoBytes) -> bool {
        !self.is_computed()
            && self.literal.token_type == IdentifierToken
            && *data == self.literal.data
    }
}

/// Go: ast.go:BindingArray — an array binding pattern.
#[derive(Clone, Debug, Default)]
pub struct BindingArray {
    pub list: Vec<BindingElement>,
    /// can be nil
    pub rest: NodeId,
}

/// Go: ast.go:BindingObjectItem — a binding property.
#[derive(Clone, Debug, Default)]
pub struct BindingObjectItem {
    /// Go `*PropertyName`, can be nil
    pub key: Option<Box<PropertyName>>,
    pub value: BindingElement,
}

/// Go: ast.go:BindingObject — an object binding pattern.
#[derive(Clone, Debug, Default)]
pub struct BindingObject {
    pub list: Vec<BindingObjectItem>,
    /// `*Var`, can be nil
    pub rest: NodeId,
}

/// Go: ast.go:BindingElement — a binding element.
#[derive(Clone, Copy, Debug, Default)]
pub struct BindingElement {
    /// can be nil (in case of ellision)
    pub binding: NodeId,
    /// can be nil
    pub default: NodeId,
}

/// Go: ast.go:VarDecl — a variable statement or lexical declaration (embeds
/// `TokenType`).
#[derive(Clone, Debug)]
pub struct VarDecl {
    pub token_type: TokenType,
    pub list: Vec<BindingElement>,
    /// `*Scope`
    pub scope: ScopeId,
    pub in_for: bool,
    pub in_for_in_of: bool,
}

/// Go: ast.go:Params — a list of parameters for functions, methods, and arrow
/// function.
#[derive(Clone, Debug, Default)]
pub struct Params {
    pub list: Vec<BindingElement>,
    /// can be nil
    pub rest: NodeId,
}

/// Go: ast.go:FuncDecl — an (async) (generator) function declaration or
/// expression.
#[derive(Clone, Debug, Default)]
pub struct FuncDecl {
    pub async_: bool,
    pub generator: bool,
    /// `*Var`, can be nil
    pub name: NodeId,
    pub params: Params,
    /// `BlockStmt` (a separate node in the port)
    pub body: NodeId,
}

/// Go: ast.go:MethodDecl — a method definition in a class declaration.
#[derive(Clone, Debug, Default)]
pub struct MethodDecl {
    pub static_: bool,
    pub async_: bool,
    pub generator: bool,
    pub get: bool,
    pub set: bool,
    pub name: PropertyName,
    pub params: Params,
    /// `BlockStmt` (a separate node in the port)
    pub body: NodeId,
}

/// Go: ast.go:Field — a field definition in a class declaration.
#[derive(Clone, Debug, Default)]
pub struct Field {
    pub static_: bool,
    pub name: PropertyName,
    pub init: NodeId,
}

/// Go: ast.go:ClassElement — a class element that is either a static block,
/// a field definition, or a class method (embeds `Field`).
#[derive(Clone, Debug, Default)]
pub struct ClassElement {
    /// `*BlockStmt`, can be nil
    pub static_block: NodeId,
    /// `*MethodDecl`, can be nil
    pub method: NodeId,
    pub field: Field,
}

/// Go: ast.go:ClassDecl — a class declaration.
#[derive(Clone, Debug, Default)]
pub struct ClassDecl {
    /// `*Var`, can be nil
    pub name: NodeId,
    /// can be nil
    pub extends: NodeId,
    pub list: Vec<ClassElement>,
}

////////////////////////////////////////////////////////////////
// Expressions

/// Go: ast.go:LiteralExpr — can be this, null, boolean, numeric, string, or
/// regular expression literals (embeds `TokenType`).
#[derive(Clone, Debug, Default)]
pub struct LiteralExpr {
    pub token_type: TokenType,
    pub data: GoBytes,
}

/// Go: ast.go:Element — an array literal element.
#[derive(Clone, Copy, Debug, Default)]
pub struct Element {
    /// can be nil
    pub value: NodeId,
    pub spread: bool,
}

/// Go: ast.go:ArrayExpr — an array literal.
#[derive(Clone, Debug, Default)]
pub struct ArrayExpr {
    pub list: Vec<Element>,
}

/// Go: ast.go:Property — a property definition in an object literal. Either
/// Name or Spread are set. When Spread is set then Value is
/// AssignmentExpression; if Init is set then Value is IdentifierReference,
/// otherwise it can also be MethodDefinition.
#[derive(Clone, Debug, Default)]
pub struct Property {
    /// Go `*PropertyName`, can be nil
    pub name: Option<Box<PropertyName>>,
    pub spread: bool,
    pub value: NodeId,
    /// can be nil
    pub init: NodeId,
}

/// Go: ast.go:ObjectExpr — an object literal.
#[derive(Clone, Debug, Default)]
pub struct ObjectExpr {
    pub list: Vec<Property>,
}

/// Go: ast.go:TemplatePart — a template head or middle.
#[derive(Clone, Debug)]
pub struct TemplatePart {
    pub value: GoBytes,
    pub expr: NodeId,
}

/// Go: ast.go:TemplateExpr — a template literal or member/call expression,
/// super property, or optional chain with template literal.
#[derive(Clone, Debug, Default)]
pub struct TemplateExpr {
    /// can be nil
    pub tag: NodeId,
    pub list: Vec<TemplatePart>,
    pub tail: GoBytes,
    pub prec: OpPrec,
    pub optional: bool,
}

/// Go: ast.go:GroupExpr — a parenthesized expression.
#[derive(Clone, Debug)]
pub struct GroupExpr {
    pub x: NodeId,
}

/// Go: ast.go:IndexExpr — a member/call expression, super property, or
/// optional chain with an index expression.
#[derive(Clone, Debug)]
pub struct IndexExpr {
    pub x: NodeId,
    pub y: NodeId,
    pub prec: OpPrec,
    pub optional: bool,
}

/// Go: ast.go:DotExpr — a member/call expression, super property, or optional
/// chain with a dot expression.
#[derive(Clone, Debug)]
pub struct DotExpr {
    pub x: NodeId,
    pub y: LiteralExpr,
    pub prec: OpPrec,
    pub optional: bool,
}

/// Go: ast.go:Arg
#[derive(Clone, Copy, Debug)]
pub struct Arg {
    pub value: NodeId,
    pub rest: bool,
}

/// Go: ast.go:Args — a list of arguments as used by new and call expressions.
#[derive(Clone, Debug, Default)]
pub struct Args {
    pub list: Vec<Arg>,
}

/// Go: ast.go:NewExpr — a new expression or new member expression.
#[derive(Clone, Debug)]
pub struct NewExpr {
    pub x: NodeId,
    /// Go `*Args`, can be nil
    pub args: Option<Args>,
}

/// Go: ast.go:CallExpr — a call expression.
#[derive(Clone, Debug)]
pub struct CallExpr {
    pub x: NodeId,
    pub args: Args,
    pub optional: bool,
}

/// Go: ast.go:UnaryExpr — an update or unary expression.
#[derive(Clone, Debug)]
pub struct UnaryExpr {
    pub op: TokenType,
    pub x: NodeId,
}

/// Go: ast.go:BinaryExpr — a binary expression.
#[derive(Clone, Debug)]
pub struct BinaryExpr {
    pub op: TokenType,
    pub x: NodeId,
    pub y: NodeId,
}

/// Go: ast.go:CondExpr — a conditional expression.
#[derive(Clone, Debug)]
pub struct CondExpr {
    pub cond: NodeId,
    pub x: NodeId,
    pub y: NodeId,
}

/// Go: ast.go:YieldExpr — a yield expression.
#[derive(Clone, Debug, Default)]
pub struct YieldExpr {
    pub generator: bool,
    /// can be nil
    pub x: NodeId,
}

/// Go: ast.go:ArrowFunc — an (async) arrow function.
#[derive(Clone, Debug, Default)]
pub struct ArrowFunc {
    pub async_: bool,
    pub params: Params,
    /// `BlockStmt` (a separate node in the port)
    pub body: NodeId,
}

/// Go: ast.go:CommaExpr — a series of comma expressions.
#[derive(Clone, Debug, Default)]
pub struct CommaExpr {
    pub list: Vec<NodeId>,
}

////////////////////////////////////////////////////////////////

/// Every node type Go handles through a pointer or interface.
#[derive(Clone, Debug, Default)]
pub enum Node {
    /// The target of [`NodeId::NIL`] (a nil interface).
    #[default]
    Nil,

    // IStmt
    Comment(Comment),
    BlockStmt(BlockStmt),
    EmptyStmt,
    ExprStmt(ExprStmt),
    IfStmt(IfStmt),
    DoWhileStmt(DoWhileStmt),
    WhileStmt(WhileStmt),
    ForStmt(ForStmt),
    ForInStmt(ForInStmt),
    ForOfStmt(ForOfStmt),
    SwitchStmt(SwitchStmt),
    BranchStmt(BranchStmt),
    ReturnStmt(ReturnStmt),
    WithStmt(WithStmt),
    LabelledStmt(LabelledStmt),
    ThrowStmt(ThrowStmt),
    TryStmt(TryStmt),
    DebuggerStmt,
    ImportStmt(ImportStmt),
    ExportStmt(ExportStmt),
    DirectivePrologueStmt(DirectivePrologueStmt),

    // declarations (IStmt and IExpr)
    VarDecl(VarDecl),
    FuncDecl(FuncDecl),
    ClassDecl(ClassDecl),
    /// IExpr (object literal methods) and `ClassElement.Method`
    MethodDecl(MethodDecl),

    // IBinding (Var is also an IExpr)
    Var(Var),
    BindingArray(BindingArray),
    BindingObject(BindingObject),

    // IExpr
    LiteralExpr(LiteralExpr),
    ArrayExpr(ArrayExpr),
    ObjectExpr(ObjectExpr),
    TemplateExpr(TemplateExpr),
    GroupExpr(GroupExpr),
    IndexExpr(IndexExpr),
    DotExpr(DotExpr),
    NewTargetExpr,
    ImportMetaExpr,
    NewExpr(NewExpr),
    CallExpr(CallExpr),
    UnaryExpr(UnaryExpr),
    BinaryExpr(BinaryExpr),
    CondExpr(CondExpr),
    YieldExpr(YieldExpr),
    ArrowFunc(ArrowFunc),
    CommaExpr(CommaExpr),
}

/// Go: ast.go:AST — the full ECMAScript abstract syntax tree, together with
/// the arena holding every node and scope.
#[derive(Clone, Debug)]
pub struct Ast {
    /// Node arena; index 0 is [`Node::Nil`].
    pub nodes: Vec<Node>,
    /// Scope arena; index 0 is unused (nil).
    pub scopes: Vec<Scope>,
    /// Go `AST.BlockStmt` (the module): a `Node::BlockStmt`.
    pub block_stmt: NodeId,
}

impl Default for Ast {
    fn default() -> Self {
        Ast::new()
    }
}

macro_rules! typed_accessors {
    ($get:ident, $get_mut:ident, $variant:ident, $ty:ty) => {
        /// Typed access; panics when the node is of another type (a failed
        /// Go type assertion).
        #[track_caller]
        pub fn $get(&self, id: NodeId) -> &$ty {
            match &self.nodes[id.0 as usize] {
                Node::$variant(n) => n,
                n => panic!(
                    concat!(
                        "interface conversion: node is {:?}, not ",
                        stringify!($variant)
                    ),
                    node_kind(n)
                ),
            }
        }

        /// Typed mutable access; panics when the node is of another type.
        #[track_caller]
        pub fn $get_mut(&mut self, id: NodeId) -> &mut $ty {
            match &mut self.nodes[id.0 as usize] {
                Node::$variant(n) => n,
                n => panic!(
                    concat!(
                        "interface conversion: node is {:?}, not ",
                        stringify!($variant)
                    ),
                    node_kind(n)
                ),
            }
        }
    };
}

/// The Go type name of a node (for diagnostics).
pub fn node_kind(n: &Node) -> &'static str {
    match n {
        Node::Nil => "nil",
        Node::Comment(_) => "*js.Comment",
        Node::BlockStmt(_) => "*js.BlockStmt",
        Node::EmptyStmt => "*js.EmptyStmt",
        Node::ExprStmt(_) => "*js.ExprStmt",
        Node::IfStmt(_) => "*js.IfStmt",
        Node::DoWhileStmt(_) => "*js.DoWhileStmt",
        Node::WhileStmt(_) => "*js.WhileStmt",
        Node::ForStmt(_) => "*js.ForStmt",
        Node::ForInStmt(_) => "*js.ForInStmt",
        Node::ForOfStmt(_) => "*js.ForOfStmt",
        Node::SwitchStmt(_) => "*js.SwitchStmt",
        Node::BranchStmt(_) => "*js.BranchStmt",
        Node::ReturnStmt(_) => "*js.ReturnStmt",
        Node::WithStmt(_) => "*js.WithStmt",
        Node::LabelledStmt(_) => "*js.LabelledStmt",
        Node::ThrowStmt(_) => "*js.ThrowStmt",
        Node::TryStmt(_) => "*js.TryStmt",
        Node::DebuggerStmt => "*js.DebuggerStmt",
        Node::ImportStmt(_) => "*js.ImportStmt",
        Node::ExportStmt(_) => "*js.ExportStmt",
        Node::DirectivePrologueStmt(_) => "*js.DirectivePrologueStmt",
        Node::VarDecl(_) => "*js.VarDecl",
        Node::FuncDecl(_) => "*js.FuncDecl",
        Node::ClassDecl(_) => "*js.ClassDecl",
        Node::MethodDecl(_) => "*js.MethodDecl",
        Node::Var(_) => "*js.Var",
        Node::BindingArray(_) => "*js.BindingArray",
        Node::BindingObject(_) => "*js.BindingObject",
        Node::LiteralExpr(_) => "*js.LiteralExpr",
        Node::ArrayExpr(_) => "*js.ArrayExpr",
        Node::ObjectExpr(_) => "*js.ObjectExpr",
        Node::TemplateExpr(_) => "*js.TemplateExpr",
        Node::GroupExpr(_) => "*js.GroupExpr",
        Node::IndexExpr(_) => "*js.IndexExpr",
        Node::DotExpr(_) => "*js.DotExpr",
        Node::NewTargetExpr => "*js.NewTargetExpr",
        Node::ImportMetaExpr => "*js.ImportMetaExpr",
        Node::NewExpr(_) => "*js.NewExpr",
        Node::CallExpr(_) => "*js.CallExpr",
        Node::UnaryExpr(_) => "*js.UnaryExpr",
        Node::BinaryExpr(_) => "*js.BinaryExpr",
        Node::CondExpr(_) => "*js.CondExpr",
        Node::YieldExpr(_) => "*js.YieldExpr",
        Node::ArrowFunc(_) => "*js.ArrowFunc",
        Node::CommaExpr(_) => "*js.CommaExpr",
    }
}

impl Ast {
    /// An empty arena (no module block yet).
    pub fn new() -> Ast {
        Ast {
            nodes: vec![Node::Nil],
            scopes: vec![Scope::default()],
            block_stmt: NodeId::NIL,
        }
    }

    /// Go `&T{...}`: allocates a node and returns its pointer.
    pub fn alloc(&mut self, n: Node) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(n);
        id
    }

    /// Allocates a scope (Go: a `Scope` value embedded in a new block).
    pub fn alloc_scope(&mut self, s: Scope) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(s);
        id
    }

    /// Go `&BlockStmt{List: list}`: a new block with its own zero `Scope`
    /// (Parent and Func nil, no variables).
    pub fn alloc_block(&mut self, list: Vec<NodeId>) -> NodeId {
        let scope = self.alloc_scope(Scope::default());
        self.alloc(Node::BlockStmt(BlockStmt { list, scope }))
    }

    /// Go `*Var{name, nil, 0, decl}`-style allocation.
    pub fn alloc_var(&mut self, data: GoBytes, link: NodeId, uses: u16, decl: DeclType) -> NodeId {
        self.alloc(Node::Var(Var {
            data,
            link,
            uses,
            decl,
        }))
    }

    /// The node behind a pointer ([`Node::Nil`] for nil).
    #[inline]
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0 as usize]
    }

    /// Mutable access to a node. Panics on nil (Go nil dereference).
    #[inline]
    #[track_caller]
    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        assert!(
            id.is_some(),
            "invalid memory address or nil pointer dereference"
        );
        &mut self.nodes[id.0 as usize]
    }

    /// The scope behind a pointer. Panics on nil (Go nil dereference).
    #[inline]
    #[track_caller]
    pub fn scope(&self, id: ScopeId) -> &Scope {
        assert!(
            !id.is_nil(),
            "invalid memory address or nil pointer dereference"
        );
        &self.scopes[id.0 as usize]
    }

    /// Mutable access to a scope. Panics on nil.
    #[inline]
    #[track_caller]
    pub fn scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        assert!(
            !id.is_nil(),
            "invalid memory address or nil pointer dereference"
        );
        &mut self.scopes[id.0 as usize]
    }

    typed_accessors!(var, var_mut, Var, Var);
    typed_accessors!(block, block_mut, BlockStmt, BlockStmt);
    typed_accessors!(var_decl, var_decl_mut, VarDecl, VarDecl);
    typed_accessors!(func_decl, func_decl_mut, FuncDecl, FuncDecl);
    typed_accessors!(method_decl, method_decl_mut, MethodDecl, MethodDecl);
    typed_accessors!(class_decl, class_decl_mut, ClassDecl, ClassDecl);
    typed_accessors!(arrow_func, arrow_func_mut, ArrowFunc, ArrowFunc);
    typed_accessors!(literal, literal_mut, LiteralExpr, LiteralExpr);
    typed_accessors!(switch_stmt, switch_stmt_mut, SwitchStmt, SwitchStmt);

    /// Go `ast.List` (the module's statement list).
    pub fn list(&self) -> &Vec<NodeId> {
        &self.block(self.block_stmt).list
    }

    /// Go `ast.Scope` (the module's scope).
    pub fn module_scope(&self) -> ScopeId {
        self.block(self.block_stmt).scope
    }

    /// `v, ok := n.(*Var)`
    #[inline]
    pub fn as_var(&self, id: NodeId) -> Option<&Var> {
        match self.node(id) {
            Node::Var(v) => Some(v),
            _ => None,
        }
    }

    /// `_, ok := n.(*Var)`
    #[inline]
    pub fn is_var(&self, id: NodeId) -> bool {
        matches!(self.node(id), Node::Var(_))
    }

    // Go: ast.go:Var.Name
    /// Returns the variable name (following links).
    pub fn var_name(&self, mut v: NodeId) -> &GoBytes {
        while self.var(v).link.is_some() {
            v = self.var(v).link;
        }
        &self.var(v).data
    }

    ////////////////////////////////////////////////////////////////
    // Scope methods (Go: ast.go:Scope.*), on the scope `s`.

    // Go: ast.go:Scope.Declare
    /// Declares a new variable.
    pub fn declare(&mut self, s: ScopeId, decl: DeclType, name: GoBytes) -> (NodeId, bool) {
        let mut s = s;
        // refer to new variable for previously undeclared symbols in the current and lower scopes
        // this happens in `{ a = 5; } var a` where both a's refer to the same variable
        let mut cur_scope = s;
        if decl == VariableDecl || decl == FunctionDecl {
            // find function scope for var and function declarations
            while s != self.scope(s).func {
                // make sure that `{let i;{var i}}` is an error
                let v = self.find_declared(s, &name, false);
                if v.is_some() && self.var(v).decl != decl && self.var(v).decl != CatchDecl {
                    return (NodeId::NIL, false);
                }
                s = self.scope(s).parent;
            }
        }

        let v = self.find_declared(s, &name, true);
        if v.is_some() {
            // variable already declared, might be an error or a duplicate declaration
            let vdecl = self.var(v).decl;
            if (ArgumentDecl < vdecl || FunctionDecl < decl) && vdecl != ExprDecl {
                // only allow (v.Decl,decl) of: (var|function|argument,var|function), (expr,*), any other combination is a syntax error
                return (NodeId::NIL, false);
            }
            if vdecl == ExprDecl {
                self.var_mut(v).decl = decl;
            }
            let vv = self.var_mut(v);
            vv.uses = vv.uses.wrapping_add(1);
            while s != cur_scope {
                self.add_undeclared(cur_scope, v); // add variable declaration as used variable to the current scope
                cur_scope = self.scope(cur_scope).parent;
            }
            return (v, true);
        }

        let mut v = NodeId::NIL;
        // reuse variable if previously used, as in:  a;var a
        if decl != ArgumentDecl {
            // in case of function f(a=b,b), where the first b is different from the second
            let num_arg_uses = self.scope(s).num_arg_uses as usize;
            let n = self.scope(s).undeclared.len();
            assert!(
                num_arg_uses <= n,
                "slice bounds out of range [{}:{}]",
                num_arg_uses,
                n
            );
            for i in 0..n - num_arg_uses {
                let uv = self.scope(s).undeclared[num_arg_uses + i];
                // no need to evaluate v.Link as v.Data stays the same and Link is nil in the active scope
                let uvv = self.var(uv);
                if 0 < uvv.uses && uvv.decl == NoDecl && bytes_equal(&name, &uvv.data) {
                    // must be NoDecl so that it can't be a var declaration that has been added
                    v = uv;
                    self.scope_mut(s).undeclared.remove(num_arg_uses + i);
                    break;
                }
            }
        }
        if v.is_nil() {
            // add variable to the context list and to the scope
            v = self.alloc_var(name, NodeId::NIL, 0, decl);
        } else {
            self.var_mut(v).decl = decl;
        }
        let vv = self.var_mut(v);
        vv.uses = vv.uses.wrapping_add(1);
        self.scope_mut(s).declared.push(v);
        while s != cur_scope {
            self.add_undeclared(cur_scope, v); // add variable declaration as used variable to the current scope
            cur_scope = self.scope(cur_scope).parent;
        }
        (v, true)
    }

    // Go: ast.go:Scope.Use
    /// Increments the usage of a variable.
    pub fn use_var(&mut self, s: ScopeId, name: GoBytes) -> NodeId {
        // check if variable is declared in the current scope
        let mut v = self.find_declared(s, &name, false);
        if v.is_nil() {
            // check if variable is already used before in the current or lower scopes
            v = self.find_undeclared(s, &name);
            if v.is_nil() {
                // add variable to the context list and to the scope's undeclared
                v = self.alloc_var(name, NodeId::NIL, 0, NoDecl);
                self.scope_mut(s).undeclared.push(v);
            }
        }
        let vv = self.var_mut(v);
        vv.uses = vv.uses.wrapping_add(1);
        v
    }

    // Go: ast.go:Scope.findDeclared
    /// Finds a declared variable in the current scope.
    pub fn find_declared(&self, s: ScopeId, name: &GoBytes, skip_for_declared: bool) -> NodeId {
        let sc = self.scope(s);
        let mut start = 0;
        if skip_for_declared {
            // we skip the for initializer for declarations (only has effect for let/const)
            start = sc.num_for_decls as usize;
        }
        // reverse order to find the inner let first in `for(let a in []){let a; {a}}`
        let mut i = sc.declared.len() as isize - 1;
        while start as isize <= i {
            let v = sc.declared[i as usize];
            // no need to evaluate v.Link as v.Data stays the same, and Link is always nil in Declared
            if bytes_equal(name, &self.var(v).data) {
                return v;
            }
            i -= 1;
        }
        NodeId::NIL
    }

    // Go: ast.go:Scope.findUndeclared
    /// Finds an undeclared variable in the current and contained scopes.
    pub fn find_undeclared(&self, s: ScopeId, name: &GoBytes) -> NodeId {
        for &v in &self.scope(s).undeclared {
            // no need to evaluate v.Link as v.Data stays the same and Link is nil in the active scope
            let vv = self.var(v);
            if 0 < vv.uses && bytes_equal(name, &vv.data) {
                return v;
            }
        }
        NodeId::NIL
    }

    // Go: ast.go:Scope.AddUndeclared
    /// Adds an undeclared variable to the scope, this is called for the block
    /// scope when declaring a var in it.
    pub fn add_undeclared(&mut self, s: ScopeId, v: NodeId) {
        // don't add undeclared symbol if it's already there
        if self.scope(s).undeclared.contains(&v) {
            return;
        }
        self.scope_mut(s).undeclared.push(v); // add variable declaration as used variable to the current scope
    }

    // Go: ast.go:Scope.MarkForStmt
    /// Marks the declared variables in current scope as for statement
    /// initializer to distinguish from declarations in body.
    pub fn mark_for_stmt(&mut self, s: ScopeId) {
        let sc = self.scope_mut(s);
        sc.num_for_decls = sc.declared.len() as u16;
        sc.num_arg_uses = sc.undeclared.len() as u16; // ensures for different b's in for(var a in b){let b}
    }

    // Go: ast.go:Scope.MarkFuncArgs
    /// Marks the declared/undeclared variables in the current scope as
    /// function arguments.
    pub fn mark_func_args(&mut self, s: ScopeId) {
        let sc = self.scope_mut(s);
        sc.num_func_args = sc.declared.len() as u16;
        sc.num_arg_uses = sc.undeclared.len() as u16; // ensures different b's in `function f(a=b){var b}`.
    }

    // Go: ast.go:Scope.HoistUndeclared
    /// Copies all undeclared variables of the current scope to the parent
    /// scope.
    pub fn hoist_undeclared(&mut self, s: ScopeId) {
        let n = self.scope(s).undeclared.len();
        for i in 0..n {
            let vorig = self.scope(s).undeclared[i];
            // no need to evaluate vorig.Link as vorig.Data stays the same
            let (uses, decl) = {
                let vv = self.var(vorig);
                (vv.uses, vv.decl)
            };
            if 0 < uses && decl == NoDecl {
                let parent = self.scope(s).parent;
                let data = self.var(vorig).data.clone();
                let v = self.find_declared(parent, &data, false);
                if v.is_some() {
                    // check if variable is declared in parent scope
                    let add = self.var(vorig).uses;
                    let vv = self.var_mut(v);
                    vv.uses = vv.uses.wrapping_add(add);
                    self.var_mut(vorig).link = v;
                    self.scope_mut(s).undeclared[i] = v; // point reference to existing var (to avoid many Link chains)
                } else {
                    let v = self.find_undeclared(parent, &data);
                    if v.is_some() {
                        // check if variable is already used before in parent scope
                        let add = self.var(vorig).uses;
                        let vv = self.var_mut(v);
                        vv.uses = vv.uses.wrapping_add(add);
                        self.var_mut(vorig).link = v;
                        self.scope_mut(s).undeclared[i] = v; // point reference to existing var (to avoid many Link chains)
                    } else {
                        // add variable to the context list and to the scope's undeclared
                        self.scope_mut(parent).undeclared.push(vorig);
                    }
                }
            }
        }
    }

    // Go: ast.go:Scope.UndeclareScope
    /// Undeclares all declared variables in the current scope and adds them to
    /// the parent scope. Called when possible arrow func ends up being a
    /// parenthesized expression, scope is not further used.
    pub fn undeclare_scope(&mut self, s: ScopeId) {
        // look if the variable already exists in the parent scope, if so replace the Var pointer in original use
        let n = self.scope(s).declared.len();
        for i in 0..n {
            let vorig = self.scope(s).declared[i];
            // no need to evaluate vorig.Link as vorig.Data stays the same, and Link is always nil in Declared
            // vorig.Uses will be atleast 1
            let parent = self.scope(s).parent;
            let data = self.var(vorig).data.clone();
            let v = self.find_declared(parent, &data, false);
            if v.is_some() {
                // check if variable has been declared in this scope
                let add = self.var(vorig).uses;
                let vv = self.var_mut(v);
                vv.uses = vv.uses.wrapping_add(add);
                self.var_mut(vorig).link = v;
            } else {
                let v = self.find_undeclared(parent, &data);
                if v.is_some() {
                    // check if variable is already used before in the current or lower scopes
                    let add = self.var(vorig).uses;
                    let vv = self.var_mut(v);
                    vv.uses = vv.uses.wrapping_add(add);
                    self.var_mut(vorig).link = v;
                } else {
                    // add variable to the context list and to the scope's undeclared
                    self.var_mut(vorig).decl = NoDecl;
                    self.scope_mut(parent).undeclared.push(vorig);
                }
            }
        }
        let sc = self.scope_mut(s);
        sc.declared.clear();
        sc.undeclared.clear();
    }

    // Go: ast.go:Scope.Unscope
    /// Moves all declared variables of the current scope to the parent scope.
    /// Undeclared variables are already in the parent scope.
    pub fn unscope(&mut self, s: ScopeId) {
        let parent = self.scope(s).parent;
        let declared = self.scope(s).declared.clone();
        for vorig in declared {
            // no need to evaluate vorig.Link as vorig.Data stays the same, and Link is always nil in Declared
            // vorig.Uses will be atleast 1
            self.scope_mut(parent).declared.push(vorig);
        }
        let sc = self.scope_mut(s);
        sc.declared.clear();
        sc.undeclared.clear();
    }

    /// Go `sort.Sort(VarsByUses(scope.Declared[from:]))` (pdqsort with Go's
    /// tie order, `Less = Uses >`).
    pub fn sort_vars_by_uses(&mut self, s: ScopeId, from: usize) {
        let Ast { nodes, scopes, .. } = self;
        assert!(
            !s.is_nil(),
            "invalid memory address or nil pointer dereference"
        );
        let declared = &mut scopes[s.0 as usize].declared[from..];
        sort_vars_by_uses(nodes, declared);
    }
}

/// Go `bytes.Equal(a, b)` with cheap early outs (the scope lookups compare
/// a name against many variables).
#[inline]
fn bytes_equal(a: &GoBytes, b: &GoBytes) -> bool {
    a.len() == b.len() && (a.is_empty() || a.at(0) == b.at(0)) && a == b
}

// Go: ast.go:VarsByUses (sort.Interface, sortable by uses in descending order)
/// `sort.Sort(VarsByUses(vs))` over any `[]*Var`.
pub fn sort_vars_by_uses(nodes: &[Node], vs: &mut [NodeId]) {
    struct VarsByUses<'a> {
        nodes: &'a [Node],
        vs: &'a mut [NodeId],
    }
    impl go_sort::sort::Interface for VarsByUses<'_> {
        fn len(&self) -> usize {
            self.vs.len()
        }
        fn less(&mut self, i: usize, j: usize) -> bool {
            uses_of(self.nodes, self.vs[i]) > uses_of(self.nodes, self.vs[j])
        }
        fn swap(&mut self, i: usize, j: usize) {
            self.vs.swap(i, j);
        }
    }
    fn uses_of(nodes: &[Node], v: NodeId) -> u16 {
        match &nodes[v.0 as usize] {
            Node::Var(v) => v.uses,
            _ => panic!("invalid memory address or nil pointer dereference"),
        }
    }
    go_sort::sort::sort(&mut VarsByUses { nodes, vs });
}
