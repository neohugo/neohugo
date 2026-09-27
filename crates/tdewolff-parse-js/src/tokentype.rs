//! Go: parse/v2/js/tokentype.go

use std::fmt;

/// Go: `type TokenType uint16` — determines the type of token, eg. a number
/// or a semicolon. From LSB to MSB: 8 bits for tokens per category, 1 bit for
/// numeric, 1 bit for punctuator, 1 bit for operator, 1 bit for identifier,
/// 4 bits unused.
///
/// A newtype over `u16` (not a Rust enum) because Go code does arithmetic and
/// bit tests on token types and `String()` must work for any value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TokenType(pub u16);

// TokenType values.
pub const ErrorToken: TokenType = TokenType(0); // extra token when errors occur
pub const WhitespaceToken: TokenType = TokenType(1);
pub const LineTerminatorToken: TokenType = TokenType(2); // \r \n \r\n
pub const CommentToken: TokenType = TokenType(3);
pub const CommentLineTerminatorToken: TokenType = TokenType(4);
pub const StringToken: TokenType = TokenType(5);
pub const TemplateToken: TokenType = TokenType(6);
pub const TemplateStartToken: TokenType = TokenType(7);
pub const TemplateMiddleToken: TokenType = TokenType(8);
pub const TemplateEndToken: TokenType = TokenType(9);
pub const RegExpToken: TokenType = TokenType(10);
pub const PrivateIdentifierToken: TokenType = TokenType(11);

// Numeric token values.
pub const NumericToken: TokenType = TokenType(0x0100);
pub const DecimalToken: TokenType = TokenType(0x0101);
pub const BinaryToken: TokenType = TokenType(0x0102);
pub const OctalToken: TokenType = TokenType(0x0103);
pub const HexadecimalToken: TokenType = TokenType(0x0104);
pub const IntegerToken: TokenType = TokenType(0x0105);

// Punctuator token values.
pub const PunctuatorToken: TokenType = TokenType(0x0200);
pub const OpenBraceToken: TokenType = TokenType(0x0201); // {
pub const CloseBraceToken: TokenType = TokenType(0x0202); // }
pub const OpenParenToken: TokenType = TokenType(0x0203); // (
pub const CloseParenToken: TokenType = TokenType(0x0204); // )
pub const OpenBracketToken: TokenType = TokenType(0x0205); // [
pub const CloseBracketToken: TokenType = TokenType(0x0206); // ]
pub const DotToken: TokenType = TokenType(0x0207); // .
pub const SemicolonToken: TokenType = TokenType(0x0208); // ;
pub const CommaToken: TokenType = TokenType(0x0209); // ,
pub const QuestionToken: TokenType = TokenType(0x020A); // ?
pub const ColonToken: TokenType = TokenType(0x020B); // :
pub const ArrowToken: TokenType = TokenType(0x020C); // =>
pub const EllipsisToken: TokenType = TokenType(0x020D); // ...

// Operator token values.
pub const OperatorToken: TokenType = TokenType(0x0600);
pub const EqToken: TokenType = TokenType(0x0601); // =
pub const EqEqToken: TokenType = TokenType(0x0602); // ==
pub const EqEqEqToken: TokenType = TokenType(0x0603); // ===
pub const NotToken: TokenType = TokenType(0x0604); // !
pub const NotEqToken: TokenType = TokenType(0x0605); // !=
pub const NotEqEqToken: TokenType = TokenType(0x0606); // !==
pub const LtToken: TokenType = TokenType(0x0607); // <
pub const LtEqToken: TokenType = TokenType(0x0608); // <=
pub const LtLtToken: TokenType = TokenType(0x0609); // <<
pub const LtLtEqToken: TokenType = TokenType(0x060A); // <<=
pub const GtToken: TokenType = TokenType(0x060B); // >
pub const GtEqToken: TokenType = TokenType(0x060C); // >=
pub const GtGtToken: TokenType = TokenType(0x060D); // >>
pub const GtGtEqToken: TokenType = TokenType(0x060E); // >>=
pub const GtGtGtToken: TokenType = TokenType(0x060F); // >>>
pub const GtGtGtEqToken: TokenType = TokenType(0x0610); // >>>=
pub const AddToken: TokenType = TokenType(0x0611); // +
pub const AddEqToken: TokenType = TokenType(0x0612); // +=
pub const IncrToken: TokenType = TokenType(0x0613); // ++
pub const SubToken: TokenType = TokenType(0x0614); // -
pub const SubEqToken: TokenType = TokenType(0x0615); // -=
pub const DecrToken: TokenType = TokenType(0x0616); // --
pub const MulToken: TokenType = TokenType(0x0617); // *
pub const MulEqToken: TokenType = TokenType(0x0618); // *=
pub const ExpToken: TokenType = TokenType(0x0619); // **
pub const ExpEqToken: TokenType = TokenType(0x061A); // **=
pub const DivToken: TokenType = TokenType(0x061B); // /
pub const DivEqToken: TokenType = TokenType(0x061C); // /=
pub const ModToken: TokenType = TokenType(0x061D); // %
pub const ModEqToken: TokenType = TokenType(0x061E); // %=
pub const BitAndToken: TokenType = TokenType(0x061F); // &
pub const BitOrToken: TokenType = TokenType(0x0620); // |
pub const BitXorToken: TokenType = TokenType(0x0621); // ^
pub const BitNotToken: TokenType = TokenType(0x0622); // ~
pub const BitAndEqToken: TokenType = TokenType(0x0623); // &=
pub const BitOrEqToken: TokenType = TokenType(0x0624); // |=
pub const BitXorEqToken: TokenType = TokenType(0x0625); // ^=
pub const AndToken: TokenType = TokenType(0x0626); // &&
pub const OrToken: TokenType = TokenType(0x0627); // ||
pub const NullishToken: TokenType = TokenType(0x0628); // ??
pub const AndEqToken: TokenType = TokenType(0x0629); // &&=
pub const OrEqToken: TokenType = TokenType(0x062A); // ||=
pub const NullishEqToken: TokenType = TokenType(0x062B); // ??=
pub const OptChainToken: TokenType = TokenType(0x062C); // ?.

// unused in lexer
pub const PosToken: TokenType = TokenType(0x062D); // +a
pub const NegToken: TokenType = TokenType(0x062E); // -a
pub const PreIncrToken: TokenType = TokenType(0x062F); // ++a
pub const PreDecrToken: TokenType = TokenType(0x0630); // --a
pub const PostIncrToken: TokenType = TokenType(0x0631); // a++
pub const PostDecrToken: TokenType = TokenType(0x0632); // a--

// Reserved token values.
pub const ReservedToken: TokenType = TokenType(0x0800);
pub const AwaitToken: TokenType = TokenType(0x0801);
pub const BreakToken: TokenType = TokenType(0x0802);
pub const CaseToken: TokenType = TokenType(0x0803);
pub const CatchToken: TokenType = TokenType(0x0804);
pub const ClassToken: TokenType = TokenType(0x0805);
pub const ConstToken: TokenType = TokenType(0x0806);
pub const ContinueToken: TokenType = TokenType(0x0807);
pub const DebuggerToken: TokenType = TokenType(0x0808);
pub const DefaultToken: TokenType = TokenType(0x0809);
pub const DeleteToken: TokenType = TokenType(0x080A);
pub const DoToken: TokenType = TokenType(0x080B);
pub const ElseToken: TokenType = TokenType(0x080C);
pub const EnumToken: TokenType = TokenType(0x080D);
pub const ExportToken: TokenType = TokenType(0x080E);
pub const ExtendsToken: TokenType = TokenType(0x080F);
pub const FalseToken: TokenType = TokenType(0x0810);
pub const FinallyToken: TokenType = TokenType(0x0811);
pub const ForToken: TokenType = TokenType(0x0812);
pub const FunctionToken: TokenType = TokenType(0x0813);
pub const IfToken: TokenType = TokenType(0x0814);
pub const ImportToken: TokenType = TokenType(0x0815);
pub const InToken: TokenType = TokenType(0x0816);
pub const InstanceofToken: TokenType = TokenType(0x0817);
pub const NewToken: TokenType = TokenType(0x0818);
pub const NullToken: TokenType = TokenType(0x0819);
pub const ReturnToken: TokenType = TokenType(0x081A);
pub const SuperToken: TokenType = TokenType(0x081B);
pub const SwitchToken: TokenType = TokenType(0x081C);
pub const ThisToken: TokenType = TokenType(0x081D);
pub const ThrowToken: TokenType = TokenType(0x081E);
pub const TrueToken: TokenType = TokenType(0x081F);
pub const TryToken: TokenType = TokenType(0x0820);
pub const TypeofToken: TokenType = TokenType(0x0821);
pub const YieldToken: TokenType = TokenType(0x0822);
pub const VarToken: TokenType = TokenType(0x0823);
pub const VoidToken: TokenType = TokenType(0x0824);
pub const WhileToken: TokenType = TokenType(0x0825);
pub const WithToken: TokenType = TokenType(0x0826);

// Identifier token values.
pub const IdentifierToken: TokenType = TokenType(0x1000);
pub const AsToken: TokenType = TokenType(0x1001);
pub const AsyncToken: TokenType = TokenType(0x1002);
pub const FromToken: TokenType = TokenType(0x1003);
pub const GetToken: TokenType = TokenType(0x1004);
pub const ImplementsToken: TokenType = TokenType(0x1005);
pub const InterfaceToken: TokenType = TokenType(0x1006);
pub const LetToken: TokenType = TokenType(0x1007);
pub const MetaToken: TokenType = TokenType(0x1008);
pub const OfToken: TokenType = TokenType(0x1009);
pub const PackageToken: TokenType = TokenType(0x100A);
pub const PrivateToken: TokenType = TokenType(0x100B);
pub const ProtectedToken: TokenType = TokenType(0x100C);
pub const PublicToken: TokenType = TokenType(0x100D);
pub const SetToken: TokenType = TokenType(0x100E);
pub const StaticToken: TokenType = TokenType(0x100F);
pub const TargetToken: TokenType = TokenType(0x1010);

// Go: tokentype.go:IsNumeric
/// Returns true if token is numeric.
#[inline]
pub fn is_numeric(tt: TokenType) -> bool {
    tt.0 & 0x0100 != 0
}

// Go: tokentype.go:IsPunctuator
/// Returns true if token is a punctuator.
#[inline]
pub fn is_punctuator(tt: TokenType) -> bool {
    tt.0 & 0x0200 != 0
}

// Go: tokentype.go:IsOperator
/// Returns true if token is an operator.
#[inline]
pub fn is_operator(tt: TokenType) -> bool {
    tt.0 & 0x0400 != 0
}

// Go: tokentype.go:IsIdentifierName
/// Matches IdentifierName, i.e. any identifier.
#[inline]
pub fn is_identifier_name(tt: TokenType) -> bool {
    tt.0 & 0x1800 != 0
}

// Go: tokentype.go:IsReservedWord
/// Matches ReservedWord.
#[inline]
pub fn is_reserved_word(tt: TokenType) -> bool {
    tt.0 & 0x0800 != 0
}

// Go: tokentype.go:IsIdentifier
/// Matches Identifier, i.e. IdentifierName but not ReservedWord. Does not
/// match yield or await.
#[inline]
pub fn is_identifier(tt: TokenType) -> bool {
    tt.0 & 0x1000 != 0
}

static OPERATOR_BYTES: [&[u8]; 51] = [
    b"Operator",
    b"=",
    b"==",
    b"===",
    b"!",
    b"!=",
    b"!==",
    b"<",
    b"<=",
    b"<<",
    b"<<=",
    b">",
    b">=",
    b">>",
    b">>=",
    b">>>",
    b">>>=",
    b"+",
    b"+=",
    b"++",
    b"-",
    b"-=",
    b"--",
    b"*",
    b"*=",
    b"**",
    b"**=",
    b"/",
    b"/=",
    b"%",
    b"%=",
    b"&",
    b"|",
    b"^",
    b"~",
    b"&=",
    b"|=",
    b"^=",
    b"&&",
    b"||",
    b"??",
    b"&&=",
    b"||=",
    b"??=",
    b"?.",
    b"+",
    b"-",
    b"++",
    b"--",
    b"++",
    b"--",
];

static RESERVED_WORD_BYTES: [&[u8]; 39] = [
    b"Reserved",
    b"await",
    b"break",
    b"case",
    b"catch",
    b"class",
    b"const",
    b"continue",
    b"debugger",
    b"default",
    b"delete",
    b"do",
    b"else",
    b"enum",
    b"export",
    b"extends",
    b"false",
    b"finally",
    b"for",
    b"function",
    b"if",
    b"import",
    b"in",
    b"instanceof",
    b"new",
    b"null",
    b"return",
    b"super",
    b"switch",
    b"this",
    b"throw",
    b"true",
    b"try",
    b"typeof",
    b"yield",
    b"var",
    b"void",
    b"while",
    b"with",
];

static IDENTIFIER_BYTES: [&[u8]; 17] = [
    b"Identifier",
    b"as",
    b"async",
    b"from",
    b"get",
    b"implements",
    b"interface",
    b"let",
    b"meta",
    b"of",
    b"package",
    b"private",
    b"protected",
    b"public",
    b"set",
    b"static",
    b"target",
];

impl TokenType {
    // Go: tokentype.go:TokenType.Bytes
    /// Returns the string representation of a TokenType (Go returns nil for
    /// unknown values, here `None`).
    pub fn bytes(self) -> Option<&'static [u8]> {
        let tt = self.0;
        // Go: int(tt-OperatorToken) with uint16 wrap-around
        if is_operator(self) && (tt.wrapping_sub(OperatorToken.0) as usize) < OPERATOR_BYTES.len() {
            return Some(OPERATOR_BYTES[tt.wrapping_sub(OperatorToken.0) as usize]);
        } else if is_reserved_word(self)
            && (tt.wrapping_sub(ReservedToken.0) as usize) < RESERVED_WORD_BYTES.len()
        {
            return Some(RESERVED_WORD_BYTES[tt.wrapping_sub(ReservedToken.0) as usize]);
        } else if is_identifier(self)
            && (tt.wrapping_sub(IdentifierToken.0) as usize) < IDENTIFIER_BYTES.len()
        {
            return Some(IDENTIFIER_BYTES[tt.wrapping_sub(IdentifierToken.0) as usize]);
        }

        Some(match self {
            ErrorToken => b"Error",
            WhitespaceToken => b"Whitespace",
            LineTerminatorToken => b"LineTerminator",
            CommentToken => b"Comment",
            CommentLineTerminatorToken => b"CommentLineTerminator",
            StringToken => b"String",
            TemplateToken => b"Template",
            TemplateStartToken => b"TemplateStart",
            TemplateMiddleToken => b"TemplateMiddle",
            TemplateEndToken => b"TemplateEnd",
            RegExpToken => b"RegExp",
            PrivateIdentifierToken => b"PrivateIdentifier",
            NumericToken => b"Numeric",
            DecimalToken => b"Decimal",
            BinaryToken => b"Binary",
            OctalToken => b"Octal",
            HexadecimalToken => b"Hexadecimal",
            IntegerToken => b"Integer",
            PunctuatorToken => b"Punctuator",
            OpenBraceToken => b"{",
            CloseBraceToken => b"}",
            OpenParenToken => b"(",
            CloseParenToken => b")",
            OpenBracketToken => b"[",
            CloseBracketToken => b"]",
            DotToken => b".",
            SemicolonToken => b";",
            CommaToken => b",",
            QuestionToken => b"?",
            ColonToken => b":",
            ArrowToken => b"=>",
            EllipsisToken => b"...",
            _ => return None,
        })
    }

    // Go: tokentype.go:TokenType.String
    /// Go's `String()`: the token bytes, or `Invalid(N)`.
    pub fn string(self) -> String {
        match self.bytes() {
            // all token names are ASCII
            Some(b) => String::from_utf8_lossy(b).into_owned(),
            None => format!("Invalid({})", self.0),
        }
    }
}

impl fmt::Debug for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}
