//! Go: parse/v2/js/table.go

use std::fmt;

use crate::tokentype::*;

/// Go: `type OpPrec int` — the operator precedence.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct OpPrec(pub i64);

// OpPrec values.
pub const OpExpr: OpPrec = OpPrec(0); // a,b
pub const OpAssign: OpPrec = OpPrec(1); // a?b:c, yield x, ()=>x, async ()=>x, a=b, a+=b, ...
pub const OpCoalesce: OpPrec = OpPrec(2); // a??b
pub const OpOr: OpPrec = OpPrec(3); // a||b
pub const OpAnd: OpPrec = OpPrec(4); // a&&b
pub const OpBitOr: OpPrec = OpPrec(5); // a|b
pub const OpBitXor: OpPrec = OpPrec(6); // a^b
pub const OpBitAnd: OpPrec = OpPrec(7); // a&b
pub const OpEquals: OpPrec = OpPrec(8); // a==b, a!=b, a===b, a!==b
pub const OpCompare: OpPrec = OpPrec(9); // a<b, a>b, a<=b, a>=b, a instanceof b, a in b
pub const OpShift: OpPrec = OpPrec(10); // a<<b, a>>b, a>>>b
pub const OpAdd: OpPrec = OpPrec(11); // a+b, a-b
pub const OpMul: OpPrec = OpPrec(12); // a*b, a/b, a%b
pub const OpExp: OpPrec = OpPrec(13); // a**b
pub const OpUnary: OpPrec = OpPrec(14); // ++x, --x, delete x, void x, typeof x, +x, -x, ~x, !x, await x
pub const OpUpdate: OpPrec = OpPrec(15); // x++, x--
pub const OpLHS: OpPrec = OpPrec(16); // CallExpr/OptChainExpr or NewExpr
pub const OpCall: OpPrec = OpPrec(17); // a?.b, a(b), super(a), import(a)
pub const OpNew: OpPrec = OpPrec(18); // new a
pub const OpMember: OpPrec = OpPrec(19); // a[b], a.b, a`b`, super[x], super.x, new.target, import.meta, new a(b)
pub const OpPrimary: OpPrec = OpPrec(20); // literal, function, class, parenthesized

impl OpPrec {
    // Go: table.go:OpPrec.String
    pub fn string(self) -> String {
        match self {
            OpExpr => "OpExpr",
            OpAssign => "OpAssign",
            OpCoalesce => "OpCoalesce",
            OpOr => "OpOr",
            OpAnd => "OpAnd",
            OpBitOr => "OpBitOr",
            OpBitXor => "OpBitXor",
            OpBitAnd => "OpBitAnd",
            OpEquals => "OpEquals",
            OpCompare => "OpCompare",
            OpShift => "OpShift",
            OpAdd => "OAdd",
            OpMul => "OpMul",
            OpExp => "OpExp",
            OpUnary => "OpUnary",
            OpUpdate => "OpUpdate",
            OpLHS => "OpLHS",
            OpCall => "OpCall",
            OpNew => "OpNew",
            OpMember => "OpMember",
            OpPrimary => "OpPrimary",
            _ => return format!("Invalid({})", self.0),
        }
        .to_string()
    }
}

impl fmt::Debug for OpPrec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}

/// Go: table.go:Keywords — the map of reserved, strict, and other keywords,
/// as a list (Go map iteration order is random, so consumers only use it as
/// a set). Use [`keyword`] for lookups.
pub static KEYWORDS: [(&[u8], TokenType); 54] = [
    // reserved
    (b"await", AwaitToken),
    (b"break", BreakToken),
    (b"case", CaseToken),
    (b"catch", CatchToken),
    (b"class", ClassToken),
    (b"const", ConstToken),
    (b"continue", ContinueToken),
    (b"debugger", DebuggerToken),
    (b"default", DefaultToken),
    (b"delete", DeleteToken),
    (b"do", DoToken),
    (b"else", ElseToken),
    (b"enum", EnumToken),
    (b"export", ExportToken),
    (b"extends", ExtendsToken),
    (b"false", FalseToken),
    (b"finally", FinallyToken),
    (b"for", ForToken),
    (b"function", FunctionToken),
    (b"if", IfToken),
    (b"import", ImportToken),
    (b"in", InToken),
    (b"instanceof", InstanceofToken),
    (b"new", NewToken),
    (b"null", NullToken),
    (b"return", ReturnToken),
    (b"super", SuperToken),
    (b"switch", SwitchToken),
    (b"this", ThisToken),
    (b"throw", ThrowToken),
    (b"true", TrueToken),
    (b"try", TryToken),
    (b"typeof", TypeofToken),
    (b"var", VarToken),
    (b"void", VoidToken),
    (b"while", WhileToken),
    (b"with", WithToken),
    (b"yield", YieldToken),
    // strict mode
    (b"let", LetToken),
    (b"static", StaticToken),
    (b"implements", ImplementsToken),
    (b"interface", InterfaceToken),
    (b"package", PackageToken),
    (b"private", PrivateToken),
    (b"protected", ProtectedToken),
    (b"public", PublicToken),
    // extra
    (b"as", AsToken),
    (b"async", AsyncToken),
    (b"from", FromToken),
    (b"get", GetToken),
    (b"meta", MetaToken),
    (b"of", OfToken),
    (b"set", SetToken),
    (b"target", TargetToken),
];

/// Go `Keywords[string(b)]` lookup (`ok == false` → `None`).
pub fn keyword(b: &[u8]) -> Option<TokenType> {
    Some(match b {
        b"await" => AwaitToken,
        b"break" => BreakToken,
        b"case" => CaseToken,
        b"catch" => CatchToken,
        b"class" => ClassToken,
        b"const" => ConstToken,
        b"continue" => ContinueToken,
        b"debugger" => DebuggerToken,
        b"default" => DefaultToken,
        b"delete" => DeleteToken,
        b"do" => DoToken,
        b"else" => ElseToken,
        b"enum" => EnumToken,
        b"export" => ExportToken,
        b"extends" => ExtendsToken,
        b"false" => FalseToken,
        b"finally" => FinallyToken,
        b"for" => ForToken,
        b"function" => FunctionToken,
        b"if" => IfToken,
        b"import" => ImportToken,
        b"in" => InToken,
        b"instanceof" => InstanceofToken,
        b"new" => NewToken,
        b"null" => NullToken,
        b"return" => ReturnToken,
        b"super" => SuperToken,
        b"switch" => SwitchToken,
        b"this" => ThisToken,
        b"throw" => ThrowToken,
        b"true" => TrueToken,
        b"try" => TryToken,
        b"typeof" => TypeofToken,
        b"var" => VarToken,
        b"void" => VoidToken,
        b"while" => WhileToken,
        b"with" => WithToken,
        b"yield" => YieldToken,
        b"let" => LetToken,
        b"static" => StaticToken,
        b"implements" => ImplementsToken,
        b"interface" => InterfaceToken,
        b"package" => PackageToken,
        b"private" => PrivateToken,
        b"protected" => ProtectedToken,
        b"public" => PublicToken,
        b"as" => AsToken,
        b"async" => AsyncToken,
        b"from" => FromToken,
        b"get" => GetToken,
        b"meta" => MetaToken,
        b"of" => OfToken,
        b"set" => SetToken,
        b"target" => TargetToken,
        _ => return None,
    })
}
