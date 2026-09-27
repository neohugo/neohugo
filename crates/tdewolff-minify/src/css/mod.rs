//! Go: minify/css/css.go — minifies CSS3 following the specifications at
//! <http://www.w3.org/TR/css-syntax-3/>.

pub mod hash;
pub mod table;
mod util;

pub use hash::*;
pub use table::{optional_zero_dimension, shorten_color_hex, shorten_color_name};

use tdewolff_parse::css::{
    AtRuleGrammar, BeginAtRuleGrammar, BeginRulesetGrammar, CommaToken, CommentGrammar,
    CustomPropertyGrammar, DeclarationGrammar, DelimToken, DimensionToken, EndAtRuleGrammar,
    EndRulesetGrammar, ErrorGrammar, FunctionToken, HashToken, IdentToken, LeftBraceToken,
    LeftBracketToken, LeftParenthesisToken, NumberToken, Parser, PercentageToken,
    QualifiedRuleGrammar, RightBraceToken, RightBracketToken, RightParenthesisToken,
    SemicolonToken, StringToken, TokenType, URLToken, UnicodeRangeToken, WhitespaceToken, is_ident,
    is_url_unquoted,
};
use tdewolff_parse::{
    GoBytes, GoError, GoReader, Input, Params, equal_fold, is_eof, is_newline, is_whitespace,
    replace_multiple_whitespace, to_lower, to_lower_slice, trim_whitespace,
};

use crate::common::{EPSILON, data_uri, decimal, number};
use crate::{M, RestoreGuard, Writer, param_is};

use util::{remove_markup_newlines, rgb_to_token};

static SPACE_BYTES: &[u8] = b" ";
static COLON_BYTES: &[u8] = b":";
static SEMICOLON_BYTES: &[u8] = b";";
static COMMA_BYTES: &[u8] = b",";
static LEFT_BRACKET_BYTES: &[u8] = b"{";
static RIGHT_BRACKET_BYTES: &[u8] = b"}";
static RIGHT_PAREN_BYTES: &[u8] = b")";
static URL_BYTES: &[u8] = b"url(";
static VAR_BYTES: &[u8] = b"var(";
static ZERO_BYTES: &[u8] = b"0";
static ONE_BYTES: &[u8] = b"1";
static TRANSPARENT_BYTES: &[u8] = b"transparent";
static BLACK_BYTES: &[u8] = b"#0000";
static INITIAL_BYTES: &[u8] = b"initial";
static NONE_BYTES: &[u8] = b"none";
static AUTO_BYTES: &[u8] = b"auto";
static LEFT_BYTES: &[u8] = b"left";
static TOP_BYTES: &[u8] = b"top";
static N400_BYTES: &[u8] = b"400";
static N700_BYTES: &[u8] = b"700";
static N50P_BYTES: &[u8] = b"50%";
static N100P_BYTES: &[u8] = b"100%";
static REPEAT_X_BYTES: &[u8] = b"repeat-x";
static REPEAT_Y_BYTES: &[u8] = b"repeat-y";
static IMPORTANT_BYTES: &[u8] = b"!important";
static DATA_SCHEME_BYTES: &[u8] = b"data:";

/// A Go package-level `[]byte("…")` stored into token data. Each use gets a
/// private copy with `cap == len` (Go shares one array, which the minifier
/// never writes through).
#[inline]
fn gb(s: &'static [u8]) -> GoBytes {
    GoBytes::from_slice(s)
}

/// Go: css.Minifier — a CSS minifier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Minifier {
    pub keep_css2: bool,
    /// number of significant digits
    pub precision: i64,
    pub inline: bool,
}

// Go: css/css.go:Minify
/// Minifies CSS data with the default options, it reads from r and writes
/// to w.
pub fn minify(
    m: &M,
    w: &mut dyn Writer,
    r: &mut dyn GoReader,
    params: Option<&Params>,
) -> Result<(), GoError> {
    crate::Minifier::minify(&Minifier::default(), m, w, r, params)
}

/// Go: css.Token — a parsed token with extra information for functions.
///
/// `Clone`, `Drop` and [`Token::equal`] walk nested `args` with an explicit
/// stack instead of recursing: Go's goroutine stack grows to 1 GB, so Go
/// minifies functions nested ~1M deep (`a{b:f(f(f(…)))}`), while recursion
/// over such a tree overflows a Rust thread stack (PORTING.md, "Deep
/// nesting").
#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub data: GoBytes,
    /// only filled for functions
    pub args: Vec<Token>,
    /// only filled for functions
    pub fun: Hash,
    /// only filled for identifiers
    pub ident: Hash,
}

impl Clone for Token {
    /// Deep copy of the token tree without recursion (see [`Token`]).
    fn clone(&self) -> Token {
        let shallow = |t: &Token, args: Vec<Token>| Token {
            token_type: t.token_type,
            data: t.data.clone(),
            args,
            fun: t.fun,
            ident: t.ident,
        };
        // (source token, its cloned args so far)
        let mut stack: Vec<(&Token, Vec<Token>)> =
            vec![(self, Vec::with_capacity(self.args.len()))];
        loop {
            let (src, done) = stack.last_mut().unwrap();
            let src: &Token = src;
            if done.len() < src.args.len() {
                let child = &src.args[done.len()];
                if child.args.is_empty() {
                    done.push(shallow(child, Vec::new()));
                } else {
                    stack.push((child, Vec::with_capacity(child.args.len())));
                }
                continue;
            }
            let (src, args) = stack.pop().unwrap();
            let t = shallow(src, args);
            match stack.last_mut() {
                Some((_, parent)) => parent.push(t),
                Option::None => return t,
            }
        }
    }
}

impl Drop for Token {
    /// Drops nested args iteratively (see [`Token`]).
    fn drop(&mut self) {
        if self.args.is_empty() {
            return;
        }
        let mut stack = std::mem::take(&mut self.args);
        while let Some(mut t) = stack.pop() {
            stack.append(&mut t.args);
        }
    }
}

impl Default for Token {
    fn default() -> Token {
        Token {
            token_type: tdewolff_parse::css::ErrorToken,
            data: GoBytes::nil(),
            args: Vec::new(),
            fun: Hash(0),
            ident: Hash(0),
        }
    }
}

impl Token {
    /// `Token{tt, data, nil, fun, ident}`
    pub fn new(token_type: TokenType, data: GoBytes, fun: Hash, ident: Hash) -> Token {
        Token {
            token_type,
            data,
            args: Vec::new(),
            fun,
            ident,
        }
    }

    // Go: css/css.go:Token.String
    pub fn string(&self) -> Vec<u8> {
        if self.args.is_empty() {
            let mut s = self.token_type.string().into_bytes();
            s.push(b'(');
            self.data.write_to(&mut s);
            s.push(b')');
            return s;
        }
        let mut sb = Vec::new();
        self.data.write_to(&mut sb);
        for arg in &self.args {
            sb.extend_from_slice(&arg.string());
        }
        sb.push(b')');
        sb
    }

    // Go: css/css.go:Token.Equal
    /// Returns true if both tokens are equal.
    /// (Go recurses over `Args`; an explicit stack visits the same pairs.)
    pub fn equal(&self, t2: &Token) -> bool {
        let mut stack = vec![(self, t2)];
        while let Some((t, t2)) = stack.pop() {
            if t.token_type == t2.token_type && t.data == t2.data && t.args.len() == t2.args.len() {
                for i in (0..t.args.len()).rev() {
                    stack.push((&t.args[i], &t2.args[i]));
                }
            } else {
                return false;
            }
        }
        true
    }

    // Go: css/css.go:Token.IsZero
    /// Returns true if a dimension, percentage, or number token is zero.
    pub fn is_zero(&self) -> bool {
        // as each number is already minified, starting with a zero means it is zero
        (self.token_type == DimensionToken
            || self.token_type == PercentageToken
            || self.token_type == NumberToken)
            && self.data.at(0) == b'0'
    }

    // Go: css/css.go:Token.IsLength
    /// Returns true if the token is a length.
    pub fn is_length(&self) -> bool {
        if self.token_type == DimensionToken {
            return true;
        } else if self.token_type == NumberToken && self.data.at(0) == b'0' {
            return true;
        } else if self.token_type == FunctionToken {
            let fun = to_hash(&self.data.slice_to(self.data.len() - 1));
            if fun == Calc
                || fun == Min
                || fun == Max
                || fun == Clamp
                || fun == Attr
                || fun == Var
                || fun == Env
            {
                return true;
            }
        }
        false
    }

    // Go: css/css.go:Token.IsLengthPercentage
    /// Returns true if the token is a length or percentage token.
    pub fn is_length_percentage(&self) -> bool {
        self.token_type == PercentageToken || self.is_length()
    }
}

/// `ToHash(parse.ToLower(parse.Copy(b)))`
fn to_hash_lower(b: &GoBytes) -> Hash {
    let mut v = b.to_vec();
    to_lower_slice(&mut v);
    to_hash(&v[..])
}

/// Go `bytes.Split(s, sep)` for a one-byte separator: all but the last
/// piece are capped (`s[:m:m]`), the last keeps the capacity of `s`.
fn go_bytes_split(s: &GoBytes, sep: u8) -> Vec<GoBytes> {
    let mut a = Vec::new();
    let mut s = s.clone();
    loop {
        let m = s.index_byte(sep);
        if m < 0 {
            break;
        }
        let m = m as usize;
        a.push(s.slice3(0, m, m));
        s = s.slice_from(m + 1);
    }
    a.push(s);
    a
}

/// Go `fmt.Sprintf("%X", i)` for an `int`.
fn go_hex_upper(i: i64) -> String {
    if i < 0 {
        format!("-{:X}", (i as i128).unsigned_abs())
    } else {
        format!("{:X}", i)
    }
}

/// Go `int(math.Pow(16.0, float64(n)))` for `n >= 1` (math.Pow computes
/// powers of two exactly; overflow gives +Inf, and FCVTZSD saturates).
fn go_int_pow16(n: i64) -> i64 {
    if n == 0 {
        return 1;
    }
    let e = n.saturating_mul(4);
    if e >= 1024 {
        return i64::MAX; // int(+Inf)
    }
    let f = f64::from_bits(((1023 + e) as u64) << 52);
    f as i64
}

/// Go `math.Modf(f)` fractional part (arm64: `f - trunc(f)`).
fn go_modf_frac(f: f64) -> f64 {
    if f < 1.0 {
        if f < 0.0 {
            return -go_modf_frac(-f);
        } else if f == 0.0 {
            return f;
        }
        return f;
    }
    f - f.trunc()
}

struct CssMinifier<'a> {
    m: &'a M,
    w: &'a mut dyn Writer,
    p: Parser,
    o: &'a Minifier,

    /// Go `c.tokenBuffer == nil` (see `parse_declaration`)
    token_buffer_nil: bool,
    tokens_level: i32,
}

impl crate::Minifier for Minifier {
    // Go: css/css.go:Minifier.Minify
    /// Minifies CSS data, it reads from r and writes to w.
    fn minify(
        &self,
        m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        params: Option<&Params>,
    ) -> Result<(), GoError> {
        let mut o = self.clone();
        // o.newPrecision is only used by commented-out code in Go.
        if !o.inline {
            o.inline = param_is(params, b"inline", b"1");
        }

        let z = Input::new(Some(r));
        let _restore = RestoreGuard(z.clone());

        let mut c = CssMinifier {
            m,
            w,
            p: Parser::new(z, o.inline),
            o: &o,
            token_buffer_nil: true,
            tokens_level: 0,
        };
        c.minify_grammar();

        c.w.write(&[])?;
        let err = c.p.err();
        if is_eof(&err) {
            return Ok(());
        }
        match err {
            Some(e) => Err(e),
            Option::None => Ok(()),
        }
    }
}

impl CssMinifier<'_> {
    #[inline]
    fn wr(&mut self, b: &[u8]) {
        let _ = self.w.write(b);
    }

    #[inline]
    fn wrg(&mut self, b: &GoBytes) {
        let _ = self.w.write_go(b);
    }

    // Go: css/css.go:cssMinifier.minifyGrammar
    fn minify_grammar(&mut self) {
        let mut semicolon_queued = false;
        loop {
            let (gt, _, data) = self.p.next();
            match gt {
                ErrorGrammar => {
                    if self.p.has_parse_error() {
                        if semicolon_queued {
                            self.wr(SEMICOLON_BYTES);
                        }

                        // write out the offending declaration (but save the semicolon)
                        let mut vals = self.p.values().to_vec();
                        if 0 < vals.len() && vals[vals.len() - 1].token_type == SemicolonToken {
                            vals.truncate(vals.len() - 1);
                            semicolon_queued = true;
                        }
                        for val in &vals {
                            self.wrg(&val.data);
                        }
                        continue;
                    }
                    return;
                }
                EndAtRuleGrammar | EndRulesetGrammar => {
                    self.wr(RIGHT_BRACKET_BYTES);
                    semicolon_queued = false;
                    continue;
                }
                _ => {}
            }

            if semicolon_queued {
                self.wr(SEMICOLON_BYTES);
                semicolon_queued = false;
            }

            match gt {
                AtRuleGrammar => {
                    self.wrg(&data);
                    let mut values = self.p.values().to_vec();
                    if to_hash(&data.slice_from(1)) == Import
                        && values.len() == 2
                        && values[1].token_type == URLToken
                        && 4 < values[1].data.len()
                        && values[1].data.at(values[1].data.len() - 1) == b')'
                    {
                        let mut url = values[1].data.clone();
                        if url.at(4) != b'"' && url.at(4) != b'\'' {
                            let mut a = 4;
                            while is_whitespace(url.at(a)) || is_newline(url.at(a)) {
                                a += 1;
                            }
                            let mut b = url.len() - 2;
                            while a < b && (is_whitespace(url.at(b)) || is_newline(url.at(b))) {
                                b -= 1;
                            }
                            if a == b {
                                url = url.slice_to(2);
                            } else {
                                url = url.slice(a - 1, b + 2);
                            }
                            url.set(0, b'"');
                            url.set(url.len() - 1, b'"');
                        } else {
                            url = url.slice(4, url.len() - 1);
                        }
                        values[1].data = url;
                    }
                    for val in &values {
                        self.wrg(&val.data);
                    }
                    semicolon_queued = true;
                }
                BeginAtRuleGrammar => {
                    self.wrg(&data);
                    let values = self.p.values().to_vec();
                    for val in &values {
                        self.wrg(&val.data);
                    }
                    self.wr(LEFT_BRACKET_BYTES);
                }
                QualifiedRuleGrammar => {
                    self.minify_selectors(&data);
                    self.wr(COMMA_BYTES);
                }
                BeginRulesetGrammar => {
                    self.minify_selectors(&data);
                    self.wr(LEFT_BRACKET_BYTES);
                }
                DeclarationGrammar => {
                    let values = self.p.values().to_vec();
                    self.minify_declaration(&data, values);
                    semicolon_queued = true;
                }
                CustomPropertyGrammar => {
                    self.wrg(&data);
                    self.wr(COLON_BYTES);
                    let v0 = self.p.values()[0].data.clone();
                    let mut value = trim_whitespace(&v0);
                    if v0.len() != 0 && value.len() == 0 {
                        value = GoBytes::from_static(SPACE_BYTES);
                    }
                    self.wrg(&value);
                    semicolon_queued = true;
                }
                CommentGrammar => {
                    if 5 < data.len() && data.at(1) == b'*' && data.at(2) == b'!' {
                        self.wrg(&data.slice_to(3));
                        let comment = trim_whitespace(&replace_multiple_whitespace(
                            data.slice(3, data.len() - 2),
                        ));
                        self.wrg(&comment);
                        self.wrg(&data.slice_from(data.len() - 2));
                    }
                }
                _ => {
                    self.wrg(&data);
                }
            }
        }
    }

    // Go: css/css.go:cssMinifier.minifySelectors
    fn minify_selectors(&mut self, _property: &GoBytes) {
        let mut in_attr = false;
        let mut is_class = false;
        let values = self.p.values().to_vec();
        for val in &values {
            if !in_attr {
                if val.token_type == IdentToken {
                    if !is_class {
                        to_lower(val.data.clone());
                    }
                    is_class = false;
                } else if val.token_type == DelimToken && val.data.at(0) == b'.' {
                    is_class = true;
                } else if val.token_type == LeftBracketToken {
                    in_attr = true;
                }
            } else {
                if val.token_type == StringToken && val.data.len() > 2 {
                    let s = val.data.slice(1, val.data.len() - 1);
                    if is_ident(&s) {
                        self.wrg(&s);
                        continue;
                    }
                } else if val.token_type == RightBracketToken {
                    in_attr = false;
                } else if val.token_type == IdentToken
                    && val.data.len() == 1
                    && (val.data.at(0) == b'i' || val.data.at(0) == b'I')
                {
                    self.wr(SPACE_BYTES);
                }
            }
            self.wrg(&val.data);
        }
    }

    // Go: css/css.go:cssMinifier.parseFunction
    ///
    /// Go recurses into nested functions (`c.parseFunction(values[i:])`);
    /// this port keeps one frame per open function on an explicit stack so
    /// that deep nesting cannot overflow the thread stack (see [`Token`]).
    /// `i` indexes `values` in every frame: a Go callee gets `values[i:]`
    /// (its index is ours minus the position of its function token), and
    /// the caller's `i += di - 1` followed by the loop's `i++` lands exactly
    /// on the callee's final `i`, so one shared `i` serves all frames.
    fn parse_function(&mut self, values: &[tdewolff_parse::css::Token]) -> (Vec<Token>, usize) {
        struct Frame {
            args: Vec<Token>,
            level: i64,
            data: GoBytes, // the function token's data (unused for the outermost frame)
        }
        let mut stack: Vec<Frame> = Vec::new();
        let mut f = Frame {
            args: Vec::new(),
            level: 0,
            data: GoBytes::nil(),
        };
        let mut i = 1;
        loop {
            while i < values.len() {
                let tt = values[i].token_type;
                let data = values[i].data.clone();
                if tt == LeftParenthesisToken {
                    f.level += 1;
                } else if tt == RightParenthesisToken {
                    if f.level == 0 {
                        i += 1;
                        break;
                    }
                    f.level -= 1;
                }
                if tt == FunctionToken {
                    // subArgs, di := c.parseFunction(values[i:])
                    let callee = Frame {
                        args: Vec::new(),
                        level: 0,
                        data,
                    };
                    stack.push(std::mem::replace(&mut f, callee));
                } else {
                    let mut h = Hash(0);
                    if tt == IdentToken {
                        h = to_hash_lower(&data); // TODO: use ToHashFold
                    }
                    f.args.push(Token::new(tt, data, Hash(0), h));
                }
                i += 1;
            }
            // return args, i
            let Some(caller) = stack.pop() else {
                return (f.args, i);
            };
            let callee = std::mem::replace(&mut f, caller);
            let data = callee.data;
            let h = to_hash_lower(&data.slice_to(data.len() - 1)); // TODO: use ToHashFold
            f.args.push(Token {
                token_type: FunctionToken,
                data,
                args: callee.args,
                fun: h,
                ident: Hash(0),
            });
            // i += di - 1; i++ (the caller continues at the callee's final i)
        }
    }

    // Go: css/css.go:cssMinifier.parseDeclaration
    /// Returns `None` (Go nil) when the value is not a simple list of values
    /// separated by whitespace or commas.
    fn parse_declaration(&mut self, values: &[tdewolff_parse::css::Token]) -> Option<Vec<Token>> {
        // Check if this is a simple list of values separated by whitespace or commas, otherwise we'll not be processing
        let mut prev_sep = true;
        let mut tokens: Vec<Token> = Vec::new();
        let mut i = 0;
        while i < values.len() {
            let tt = values[i].token_type;
            let data = values[i].data.clone();
            if tt == LeftParenthesisToken
                || tt == LeftBraceToken
                || tt == LeftBracketToken
                || tt == RightParenthesisToken
                || tt == RightBraceToken
                || tt == RightBracketToken
            {
                return Option::None;
            }

            if !prev_sep
                && tt != WhitespaceToken
                && tt != CommaToken
                && (tt != DelimToken || values[i].data.at(0) != b'/')
            {
                return Option::None;
            }

            if tt == WhitespaceToken
                || tt == CommaToken
                || tt == DelimToken && values[i].data.at(0) == b'/'
            {
                if tt != WhitespaceToken {
                    tokens.push(Token::new(tt, data, Hash(0), Hash(0)));
                }
                prev_sep = true;
            } else if tt == FunctionToken {
                let (args, di) = self.parse_function(&values[i..]);
                let h = to_hash_lower(&data.slice_to(data.len() - 1)); // TODO: use ToHashFold
                tokens.push(Token {
                    token_type: tt,
                    data,
                    args,
                    fun: h,
                    ident: Hash(0),
                });
                prev_sep = true;
                i += di - 1;
            } else {
                let mut h = Hash(0);
                if tt == IdentToken {
                    h = to_hash_lower(&data); // TODO: use ToHashFold
                }
                tokens.push(Token::new(tt, data, Hash(0), h));
                prev_sep = tt == URLToken;
            }
            i += 1;
        }
        // `tokens := c.tokenBuffer[:0]` is nil until a declaration appended
        // at least one token (then `c.tokenBuffer = tokens` keeps it non-nil).
        if tokens.is_empty() && self.token_buffer_nil {
            return Option::None;
        }
        if !tokens.is_empty() {
            self.token_buffer_nil = false;
        }
        Some(tokens) // update buffer size for memory reuse
    }

    // Go: css/css.go:cssMinifier.minifyDeclaration
    fn minify_declaration(
        &mut self,
        property: &GoBytes,
        mut components: Vec<tdewolff_parse::css::Token>,
    ) {
        self.wrg(property);
        self.wr(COLON_BYTES);

        if components.len() == 0 {
            return;
        }

        // Strip !important from the component list, this will be added later separately
        let mut important = false;
        let n = components.len();
        if 2 < n
            && components[n - 2].token_type == DelimToken
            && components[n - 2].data.at(0) == b'!'
            && to_hash(&components[n - 1].data) == Important
        {
            components.truncate(n - 2);
            important = true;
        }

        let prop = to_hash(property);
        let values = self.parse_declaration(&components);

        // Do not process complex values (eg. containing blocks or is not alternated between whitespace/commas and flat values
        let Some(values) = values else {
            if prop == Filter && components.len() == 11 {
                if components[0].data.equal(b"progid")
                    && components[1].token_type == tdewolff_parse::css::ColonToken
                    && components[2].data.equal(b"DXImageTransform")
                    && components[3].data.at(0) == b'.'
                    && components[4].data.equal(b"Microsoft")
                    && components[5].data.at(0) == b'.'
                    && components[6].data.equal(b"Alpha(")
                    && to_lower(components[7].data.clone()).equal(b"opacity")
                    && components[8].data.at(0) == b'='
                    && components[10].data.at(0) == b')'
                {
                    components.drain(..6);
                    components[0].data = GoBytes::from_slice(b"alpha(");
                }
            }

            for component in &components {
                self.wrg(&component.data);
            }
            if important {
                if self.o.keep_css2 {
                    self.wr(SPACE_BYTES);
                }
                self.wr(IMPORTANT_BYTES);
            }
            return;
        };

        let mut values = self.minify_tokens(prop, Hash(0), values);
        if 0 < values.len() {
            values = self.minify_property(prop, values);
        }
        self.write_declaration(&values, important);
    }

    // Go: css/css.go:cssMinifier.writeFunction
    ///
    /// Go recurses into nested functions; an explicit stack of argument
    /// iterators writes the same bytes without recursion (see [`Token`]).
    fn write_function(&mut self, args: &[Token]) {
        let mut stack = vec![args.iter()];
        while let Some(it) = stack.last_mut() {
            match it.next() {
                Some(arg) => {
                    self.wrg(&arg.data);
                    if arg.token_type == FunctionToken {
                        stack.push(arg.args.iter()); // c.writeFunction(arg.Args)
                    }
                }
                Option::None => {
                    stack.pop();
                    if !stack.is_empty() {
                        self.wr(RIGHT_PAREN_BYTES); // after the nested c.writeFunction
                    }
                }
            }
        }
    }

    // Go: css/css.go:cssMinifier.writeDeclaration
    fn write_declaration(&mut self, values: &[Token], important: bool) {
        let mut prev_sep = true;
        for value in values {
            if !prev_sep
                && value.token_type != CommaToken
                && (value.token_type != DelimToken || value.data.at(0) != b'/')
            {
                self.wr(SPACE_BYTES);
            }

            self.wrg(&value.data);
            if value.token_type == FunctionToken {
                self.write_function(&value.args);
                self.wr(RIGHT_PAREN_BYTES);
            }

            if value.token_type == CommaToken
                || value.token_type == DelimToken && value.data.at(0) == b'/'
                || value.token_type == FunctionToken
                || value.token_type == URLToken
            {
                prev_sep = true;
            } else {
                prev_sep = false;
            }
        }

        if important {
            if self.o.keep_css2 {
                self.wr(SPACE_BYTES);
            }
            self.wr(IMPORTANT_BYTES);
        }
    }

    // Go: css/css.go:cssMinifier.minifyTokens
    fn minify_tokens(&mut self, prop: Hash, fun: Hash, mut values: Vec<Token>) -> Vec<Token> {
        if 100 < self.tokens_level + 1 {
            return values;
        }
        self.tokens_level += 1;

        for i in 0..values.len() {
            let tt = values[i].token_type;
            'sw: {
                match tt {
                    NumberToken => {
                        if prop == Z_Index
                            || prop == Counter_Increment
                            || prop == Counter_Reset
                            || prop == Orphans
                            || prop == Widows
                        {
                            break 'sw; // integers
                        }
                        if self.o.keep_css2 {
                            values[i].data = decimal(values[i].data.clone(), self.o.precision); // don't use exponents
                        } else {
                            values[i].data = number(values[i].data.clone(), self.o.precision);
                        }
                    }
                    PercentageToken => {
                        let n = values[i].data.len() - 1;
                        if self.o.keep_css2 {
                            values[i].data = decimal(values[i].data.slice_to(n), self.o.precision); // don't use exponents
                        } else {
                            values[i].data = number(values[i].data.slice_to(n), self.o.precision);
                        }
                        values[i].data = values[i].data.append_byte(b'%');
                    }
                    DimensionToken => {
                        let (v, dim) = self.minify_dimension(values[i].clone());
                        values[i] = v;
                        if 1 < values[i].data.len()
                            && values[i].data.at(0) == b'0'
                            && optional_zero_dimension(&dim.to_vec())
                            && prop != Flex
                            && fun == Hash(0)
                        {
                            // cut dimension for zero value, TODO: don't hardcode check for Flex and remove the dimension in minifyDimension
                            values[i].data = values[i].data.slice_to(1);
                        }
                    }
                    StringToken => {
                        values[i].data = remove_markup_newlines(values[i].data.clone());
                    }
                    URLToken => {
                        if 10 < values[i].data.len() {
                            let d = values[i].data.clone();
                            let mut uri = trim_whitespace(&d.slice(4, d.len() - 1));
                            let mut delim = b'"';
                            if 1 < uri.len() && (uri.at(0) == b'\'' || uri.at(0) == b'"') {
                                delim = uri.at(0);
                                uri = remove_markup_newlines(uri);
                                uri = uri.slice(1, uri.len() - 1);
                            }
                            if 4 < uri.len() && equal_fold(&uri.slice_to(5), DATA_SCHEME_BYTES) {
                                uri = data_uri(self.m, uri);
                            }
                            if is_url_unquoted(&uri) {
                                values[i].data = GoBytes::from_static(URL_BYTES)
                                    .append_bytes(&uri)
                                    .append_byte(b')');
                            } else {
                                values[i].data = GoBytes::from_static(URL_BYTES)
                                    .append_byte(delim)
                                    .append_bytes(&uri)
                                    .append(&[delim, b')']);
                            }
                        }
                    }
                    FunctionToken => {
                        let args = std::mem::take(&mut values[i].args);
                        values[i].args = self.minify_tokens(prop, values[i].fun, args);

                        let fun = values[i].fun;
                        if fun == Rgb || fun == Rgba || fun == Hsl || fun == Hsla {
                            let mut valid = true;
                            let mut vals: Vec<f64> = Vec::new();
                            for (k, arg) in values[i].args.iter().enumerate() {
                                let numeric = arg.token_type == NumberToken
                                    || arg.token_type == PercentageToken;
                                let separator = arg.token_type == CommaToken
                                    || k != 5 && arg.token_type == WhitespaceToken
                                    || k == 5
                                        && arg.token_type == DelimToken
                                        && arg.data.at(0) == b'/';
                                if k % 2 == 0 && !numeric || k % 2 == 1 && !separator {
                                    valid = false;
                                    break;
                                } else if numeric {
                                    let mut d: f64;
                                    if arg.token_type == PercentageToken {
                                        let s = arg.data.slice_to(arg.data.len() - 1).to_vec();
                                        match go_strconv::parse_float(&s, 32) {
                                            // can overflow
                                            Ok(v) => d = v,
                                            Err(_) => {
                                                valid = false;
                                                break;
                                            }
                                        }
                                        d /= 100.0;
                                        if d < EPSILON {
                                            d = 0.0;
                                        } else if 1.0 - EPSILON < d {
                                            d = 1.0;
                                        }
                                    } else {
                                        match go_strconv::parse_float(arg.data.to_vec(), 32) {
                                            // can overflow
                                            Ok(v) => d = v,
                                            Err(_) => {
                                                valid = false;
                                                break;
                                            }
                                        }
                                    }
                                    vals.push(d);
                                }
                            }
                            if !valid {
                                break 'sw;
                            }

                            let mut a = 1.0;
                            if vals.len() == 4 {
                                if vals[0] < EPSILON
                                    && vals[1] < EPSILON
                                    && vals[2] < EPSILON
                                    && vals[3] < EPSILON
                                {
                                    values[i] = Token::new(
                                        IdentToken,
                                        gb(TRANSPARENT_BYTES),
                                        Hash(0),
                                        Transparent,
                                    );
                                    break 'sw;
                                } else if 1.0 - EPSILON < vals[3] {
                                    vals.truncate(3);
                                    let n = values[i].args.len();
                                    values[i].args.truncate(n - 2);
                                    if fun == Rgba || fun == Hsla {
                                        let d = values[i].data.slice_to(values[i].data.len() - 1);
                                        d.set(d.len() - 1, b'(');
                                        values[i].data = d;
                                    }
                                } else {
                                    a = vals[3];
                                }
                            }

                            if a == 1.0 && (vals.len() == 3 || vals.len() == 4) {
                                // only minify color if fully opaque
                                if fun == Rgb || fun == Rgba {
                                    for j in 0..3 {
                                        if values[i].args[j * 2].token_type == NumberToken {
                                            vals[j] /= 255.0;
                                            if vals[j] < EPSILON {
                                                vals[j] = 0.0;
                                            } else if 1.0 - EPSILON < vals[j] {
                                                vals[j] = 1.0;
                                            }
                                        }
                                    }
                                    values[i] = rgb_to_token(vals[0], vals[1], vals[2]);
                                    break 'sw;
                                } else if fun == Hsl
                                    || fun == Hsla
                                        && values[i].args[0].token_type == NumberToken
                                        && values[i].args[2].token_type == PercentageToken
                                        && values[i].args[4].token_type == PercentageToken
                                {
                                    vals[0] /= 360.0;
                                    vals[0] = go_modf_frac(vals[0]);
                                    if vals[0] < 0.0 {
                                        vals[0] = 1.0 + vals[0];
                                    }
                                    let (r, g, b) =
                                        tdewolff_parse::css::hsl2rgb(vals[0], vals[1], vals[2]);
                                    values[i] = rgb_to_token(r, g, b);
                                    break 'sw;
                                }
                            } else if vals.len() == 4 {
                                let a6 = values[i].args[6].clone();
                                values[i].args[6] = minify_number_percentage(a6);
                            }

                            if 3 <= vals.len() && (fun == Rgb || fun == Rgba) {
                                // 0%, 20%, 40%, 60%, 80% and 100% can be represented exactly as, 51, 102, 153, 204, and 255 respectively
                                let mut remove_percentage = true;
                                for j in 0..3 {
                                    if values[i].args[j * 2].token_type != PercentageToken
                                        || 2.0 * EPSILON <= (vals[j] + EPSILON) % 0.2
                                    {
                                        remove_percentage = false;
                                        break;
                                    }
                                }
                                if remove_percentage {
                                    for j in 0..3 {
                                        let arg = &mut values[i].args[j * 2];
                                        arg.token_type = NumberToken;
                                        if vals[j] < EPSILON {
                                            arg.data = gb(ZERO_BYTES);
                                        } else if (vals[j] - 0.2).abs() < EPSILON {
                                            arg.data = GoBytes::from_slice(b"51");
                                        } else if (vals[j] - 0.4).abs() < EPSILON {
                                            arg.data = GoBytes::from_slice(b"102");
                                        } else if (vals[j] - 0.6).abs() < EPSILON {
                                            arg.data = GoBytes::from_slice(b"153");
                                        } else if (vals[j] - 0.8).abs() < EPSILON {
                                            arg.data = GoBytes::from_slice(b"204");
                                        } else if (vals[j] - 1.0).abs() < EPSILON {
                                            arg.data = GoBytes::from_slice(b"255");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        self.tokens_level -= 1;
        values
    }

    // Go: css/css.go:cssMinifier.minifyProperty
    fn minify_property(&mut self, prop: Hash, mut values: Vec<Token>) -> Vec<Token> {
        // limit maximum to prevent slow recursions (e.g. for background's append)
        if 100 < values.len() {
            return values;
        }

        match prop {
            Font => {
                if values.len() > 1 {
                    // must contain atleast font-size and font-family
                    // the font-families are separated by commas and are at the end of font
                    // get index for last token before font family names
                    let mut i: isize = values.len() as isize - 1;
                    for j in 2..values.len() {
                        if values[j].token_type == CommaToken {
                            i = j as isize - 1; // identifier before first comma is a font-family
                            break;
                        }
                    }
                    i -= 1;

                    // advance i while still at font-families when they contain spaces but no quotes
                    while i > 0 {
                        // i cannot be 0, font-family must be prepended by font-size
                        let iu = i as usize;
                        let h = values[iu].ident;
                        if values[iu - 1].token_type == DelimToken
                            && values[iu - 1].data.at(0) == b'/'
                        {
                            break;
                        } else if values[iu].token_type != IdentToken
                            && values[iu].token_type != StringToken
                        {
                            break;
                        } else if h == Xx_Small
                            || h == X_Small
                            || h == Small
                            || h == Medium
                            || h == Large
                            || h == X_Large
                            || h == Xx_Large
                            || h == Smaller
                            || h == Larger
                            || h == Inherit
                            || h == Initial
                            || h == Unset
                        {
                            // inherit, initial and unset are followed by an IdentToken/StringToken, so must be for font-size
                            break;
                        }
                        i -= 1;
                    }

                    // font-family minified in place
                    let iu = i as usize;
                    let tail = values.split_off(iu + 1);
                    let tail = self.minify_property(Font_Family, tail);
                    values.extend(tail);

                    // fix for IE9, IE10, IE11: font name starting with `-` is not recognized
                    if values[iu + 1].data.at(0) == b'-' {
                        let d = &values[iu + 1].data;
                        let v = GoBytes::make(d.len() + 2, d.len() + 2);
                        v.set(0, b'\'');
                        v.slice_from(1).copy_from(d);
                        v.set(v.len() - 1, b'\'');
                        values[iu + 1].data = v;
                    }

                    if i > 0 {
                        // line-height
                        if i > 1
                            && values[iu - 1].token_type == DelimToken
                            && values[iu - 1].data.at(0) == b'/'
                        {
                            if values[iu].ident == Normal {
                                values.drain(iu - 1..iu + 1);
                            }
                            i -= 2;
                        }

                        // font-size
                        i -= 1;

                        while i > -1 {
                            let iu = i as usize;
                            if values[iu].ident == Normal {
                                values.remove(iu);
                            } else if values[iu].ident == Bold {
                                values[iu].token_type = NumberToken;
                                values[iu].data = gb(N700_BYTES);
                            } else if values[iu].token_type == NumberToken
                                && values[iu].data.equal(N400_BYTES)
                            {
                                values.remove(iu);
                            }
                            i -= 1;
                        }
                    }
                }
            }
            Font_Family => {
                for i in 0..values.len() {
                    let value = values[i].clone();
                    if value.token_type == StringToken && 2 < value.data.len() {
                        let mut unquote = true;
                        to_lower(value.data.clone());
                        let s = value.data.slice(1, value.data.len() - 1);
                        if 0 < s.len() {
                            for split in go_bytes_split(&s, b' ') {
                                // if len is zero, it contains two consecutive spaces
                                if split.len() == 0 || !is_ident(&split) {
                                    unquote = false;
                                    break;
                                }
                            }
                        }
                        if unquote {
                            values[i].data = s;
                        }
                    }
                }
            }
            Font_Weight => {
                if values[0].ident == Normal {
                    values[0].token_type = NumberToken;
                    values[0].data = gb(N400_BYTES);
                } else if values[0].ident == Bold {
                    values[0].token_type = NumberToken;
                    values[0].data = gb(N700_BYTES);
                }
            }
            Url => {
                for i in 0..values.len() {
                    if values[i].token_type == FunctionToken && values[i].args.len() == 1 {
                        let fun = values[i].fun;
                        let mut data = values[i].args[0].data.clone();
                        if fun == Local && (data.at(0) == b'\'' || data.at(0) == b'"') {
                            if is_url_unquoted(&data.slice(1, data.len() - 1)) {
                                data = data.slice(1, data.len() - 1);
                            }
                            values[i].args[0].data = data;
                        }
                    }
                }
            }
            Margin | Padding | Border_Width => match values.len() {
                2 => {
                    if values[0].equal(&values[1]) {
                        values.truncate(1);
                    }
                }
                3 => {
                    if values[0].equal(&values[1]) && values[0].equal(&values[2]) {
                        values.truncate(1);
                    } else if values[0].equal(&values[2]) {
                        values.truncate(2);
                    }
                }
                4 => {
                    if values[0].equal(&values[1])
                        && values[0].equal(&values[2])
                        && values[0].equal(&values[3])
                    {
                        values.truncate(1);
                    } else if values[0].equal(&values[2]) && values[1].equal(&values[3]) {
                        values.truncate(2);
                    } else if values[1].equal(&values[3]) {
                        values.truncate(3);
                    }
                }
                _ => {}
            },
            Border | Border_Bottom | Border_Left | Border_Right | Border_Top => {
                let mut i: isize = 0;
                while i < values.len() as isize {
                    let iu = i as usize;
                    if values[iu].ident == None
                        || values[iu].ident == Currentcolor
                        || values[iu].ident == Medium
                    {
                        values.remove(iu);
                        i -= 1;
                    } else {
                        values[iu] = minify_color(values[iu].clone());
                    }
                    i += 1;
                }
                if values.len() == 0 {
                    values = vec![Token::new(IdentToken, gb(NONE_BYTES), Hash(0), None)];
                }
            }
            Outline => {
                let mut i: isize = 0;
                while i < values.len() as isize {
                    let iu = i as usize;
                    if values[iu].ident == Invert
                        || values[iu].ident == None
                        || values[iu].ident == Medium
                    {
                        values.remove(iu);
                        i -= 1;
                    } else {
                        values[iu] = minify_color(values[iu].clone());
                    }
                    i += 1;
                }
                if values.len() == 0 {
                    values = vec![Token::new(IdentToken, gb(NONE_BYTES), Hash(0), None)];
                }
            }
            Background => {
                values = self.minify_background(values);
            }
            Background_Size => {
                let mut start = 0usize;
                let mut end = 0usize;
                while end <= values.len() {
                    // loop over comma-separated lists
                    if end != values.len() && values[end].token_type != CommaToken {
                        end += 1;
                        continue;
                    } else if start == end {
                        start += 1;
                        end += 1;
                        continue;
                    }

                    if end - start == 2 && values[start + 1].ident == Auto {
                        values.remove(start + 1);
                        end -= 1;
                    }
                    start = end + 1;
                    end += 1;
                }
            }
            Background_Repeat => {
                let mut start = 0usize;
                let mut end = 0usize;
                while end <= values.len() {
                    // loop over comma-separated lists
                    if end != values.len() && values[end].token_type != CommaToken {
                        end += 1;
                        continue;
                    } else if start == end {
                        start += 1;
                        end += 1;
                        continue;
                    }

                    if end - start == 2
                        && values[start].token_type == IdentToken
                        && values[start + 1].token_type == IdentToken
                    {
                        if values[start].ident == values[start + 1].ident {
                            values.remove(start + 1);
                            end -= 1;
                        } else if values[start].ident == Repeat
                            && values[start + 1].ident == No_Repeat
                        {
                            values[start].data = gb(REPEAT_X_BYTES);
                            values[start].ident = Repeat_X;
                            values.remove(start + 1);
                            end -= 1;
                        } else if values[start].ident == No_Repeat
                            && values[start + 1].ident == Repeat
                        {
                            values[start].data = gb(REPEAT_Y_BYTES);
                            values[start].ident = Repeat_Y;
                            values.remove(start + 1);
                            end -= 1;
                        }
                    }
                    start = end + 1;
                    end += 1;
                }
            }
            Background_Position => {
                values = minify_background_position(values);
            }
            Box_Shadow => {
                let mut start = 0usize;
                let mut end = 0usize;
                while end <= values.len() {
                    // loop over comma-separated lists
                    if end != values.len() && values[end].token_type != CommaToken {
                        end += 1;
                        continue;
                    } else if start == end {
                        start += 1;
                        end += 1;
                        continue;
                    }

                    if end - start == 1 && values[start].ident == Initial {
                        values[start].ident = None;
                        values[start].data = gb(NONE_BYTES);
                    } else {
                        let mut numbers: Vec<usize> = Vec::new();
                        for i in start..end {
                            if values[i].is_length() {
                                numbers.push(i);
                            }
                        }
                        if numbers.len() == 4 && values[numbers[3]].is_zero() {
                            values.remove(numbers[3]);
                            numbers.truncate(3);
                            end -= 1;
                        }
                        if numbers.len() == 3 && values[numbers[2]].is_zero() {
                            values.remove(numbers[2]);
                            end -= 1;
                        }
                    }
                    start = end + 1;
                    end += 1;
                }
            }
            Ms_Filter => {
                let alpha: &[u8] = b"progid:DXImageTransform.Microsoft.Alpha(Opacity=";
                let d = values[0].data.clone();
                if values[0].token_type == StringToken
                    && 2 < d.len()
                    && d.slice(1, d.len() - 1).has_prefix(alpha)
                {
                    values[0].data = GoBytes::nil()
                        .append(&[d.at(0)])
                        .append(b"alpha(opacity=")
                        .append_bytes(&d.slice_from(1 + alpha.len()));
                }
            }
            Color => {
                values[0] = minify_color(values[0].clone());
            }
            Background_Color => {
                values[0] = minify_color(values[0].clone());
                if !self.o.keep_css2 {
                    if values[0].ident == Transparent {
                        values[0].data = gb(INITIAL_BYTES);
                        values[0].ident = Initial;
                    }
                }
            }
            Border_Color => {
                let mut same_values = true;
                for i in 0..values.len() {
                    if values[i].ident == Currentcolor {
                        values[i].data = gb(INITIAL_BYTES);
                        values[i].ident = Initial;
                    } else {
                        values[i] = minify_color(values[i].clone());
                    }
                    if 0 < i && same_values && !values[0].equal(&values[i]) {
                        same_values = false;
                    }
                }
                if same_values {
                    values.truncate(1);
                }
            }
            Border_Left_Color
            | Border_Right_Color
            | Border_Top_Color
            | Border_Bottom_Color
            | Text_Decoration_Color
            | Text_Emphasis_Color => {
                if values[0].ident == Currentcolor {
                    values[0].data = gb(INITIAL_BYTES);
                    values[0].ident = Initial;
                } else {
                    values[0] = minify_color(values[0].clone());
                }
            }
            Caret_Color | Outline_Color | Fill | Stroke => {
                values[0] = minify_color(values[0].clone());
            }
            Column_Rule => {
                let mut i: isize = 0;
                while i < values.len() as isize {
                    let iu = i as usize;
                    if values[iu].ident == Currentcolor
                        || values[iu].ident == None
                        || values[iu].ident == Medium
                    {
                        values.remove(iu);
                        i -= 1;
                    } else {
                        values[iu] = minify_color(values[iu].clone());
                    }
                    i += 1;
                }
                if values.len() == 0 {
                    values = vec![Token::new(IdentToken, gb(NONE_BYTES), Hash(0), None)];
                }
            }
            Text_Shadow => {
                // TODO: minify better (can be comma separated list)
                for i in 0..values.len() {
                    values[i] = minify_color(values[i].clone());
                }
            }
            Text_Decoration => {
                let mut i: isize = 0;
                while i < values.len() as isize {
                    let iu = i as usize;
                    if values[iu].ident == Currentcolor
                        || values[iu].ident == None
                        || values[iu].ident == Solid
                    {
                        values.remove(iu);
                        i -= 1;
                    } else {
                        values[iu] = minify_color(values[iu].clone());
                    }
                    i += 1;
                }
                if values.len() == 0 {
                    values = vec![Token::new(IdentToken, gb(NONE_BYTES), Hash(0), None)];
                }
            }
            Text_Emphasis => {
                let mut i: isize = 0;
                while i < values.len() as isize {
                    let iu = i as usize;
                    if values[iu].ident == Currentcolor || values[iu].ident == None {
                        values.remove(iu);
                        i -= 1;
                    } else {
                        values[iu] = minify_color(values[iu].clone());
                    }
                    i += 1;
                }
                if values.len() == 0 {
                    values = vec![Token::new(IdentToken, gb(NONE_BYTES), Hash(0), None)];
                }
            }
            Flex => {
                if values.len() == 2 && values[0].token_type == NumberToken {
                    if values[1].token_type != NumberToken && values[1].is_zero() {
                        values.truncate(1); // remove <flex-basis> if it is zero
                    }
                } else if values.len() == 3
                    && values[0].token_type == NumberToken
                    && values[1].token_type == NumberToken
                {
                    if values[0].data.len() == 1 && values[1].data.len() == 1 {
                        if values[2].ident == Auto {
                            if values[0].data.at(0) == b'0' && values[1].data.at(0) == b'1' {
                                values.truncate(1);
                                values[0].token_type = IdentToken;
                                values[0].data = gb(INITIAL_BYTES);
                                values[0].ident = Initial;
                            } else if values[0].data.at(0) == b'1' && values[1].data.at(0) == b'1' {
                                values.truncate(1);
                                values[0].token_type = IdentToken;
                                values[0].data = gb(AUTO_BYTES);
                                values[0].ident = Auto;
                            } else if values[0].data.at(0) == b'0' && values[1].data.at(0) == b'0' {
                                values.truncate(1);
                                values[0].token_type = IdentToken;
                                values[0].data = gb(NONE_BYTES);
                                values[0].ident = None;
                            }
                        } else if values[1].data.at(0) == b'1' && values[2].is_zero() {
                            values.truncate(1); // remove <flex-shrink> and <flex-basis> if they are 1 and 0 respectively
                        } else if values[2].is_zero() {
                            values.truncate(2); // remove auto to write 2-value syntax of <flex-grow> <flex-shrink>
                        } else {
                            values[2] = minify_length_percentage(values[2].clone());
                        }
                    }
                }
            }
            Flex_Basis => {
                if values[0].ident == Initial {
                    values[0].data = gb(AUTO_BYTES);
                    values[0].ident = Auto;
                } else {
                    values[0] = minify_length_percentage(values[0].clone());
                }
            }
            Order | Flex_Grow => {
                if values[0].ident == Initial {
                    values[0].token_type = NumberToken;
                    values[0].data = gb(ZERO_BYTES);
                    values[0].ident = Hash(0);
                }
            }
            Flex_Shrink => {
                if values[0].ident == Initial {
                    values[0].token_type = NumberToken;
                    values[0].data = gb(ONE_BYTES);
                    values[0].ident = Hash(0);
                }
            }
            Unicode_Range => {
                let mut ranges: Vec<[i64; 2]> = Vec::new();
                for value in &values {
                    if value.token_type == CommaToken {
                        continue;
                    } else if value.token_type != UnicodeRangeToken {
                        return values;
                    }

                    let d = &value.data;
                    let mut i = 2usize;
                    let mut i_wildcard = 0usize;
                    let mut start: i64 = 0;
                    while i < d.len() && d.at(i) != b'-' {
                        start = start.wrapping_mul(16);
                        let c = d.at(i);
                        if b'0' <= c && c <= b'9' {
                            start = start.wrapping_add((c - b'0') as i64);
                        } else if b'a' <= c | 32 && c | 32 <= b'f' {
                            start = start.wrapping_add(((c | 32) - b'a') as i64 + 10);
                        } else if i_wildcard == 0 && c == b'?' {
                            i_wildcard = i;
                        }
                        i += 1;
                    }
                    let mut end = start;
                    if i_wildcard != 0 {
                        end = start
                            .wrapping_add(go_int_pow16((d.len() - i_wildcard) as i64))
                            .wrapping_sub(1);
                    } else if i < d.len() && d.at(i) == b'-' {
                        i += 1;
                        end = 0;
                        while i < d.len() {
                            end = end.wrapping_mul(16);
                            let c = d.at(i);
                            if b'0' <= c && c <= b'9' {
                                end = end.wrapping_add((c - b'0') as i64);
                            } else if b'a' <= c | 32 && c | 32 <= b'f' {
                                end = end.wrapping_add(((c | 32) - b'a') as i64 + 10);
                            }
                            i += 1;
                        }
                        if end <= start {
                            end = start;
                        }
                    }
                    ranges.push([start, end]);
                }

                // sort and remove overlapping ranges
                go_sort::sort_by(&mut ranges, |a, b| a[0] < b[0]);
                let mut i = 0usize;
                while ranges.len() > 0 && i < ranges.len() - 1 {
                    if ranges[i + 1][1] <= ranges[i][1] {
                        // next range is fully contained in the current range
                        ranges.remove(i + 1);
                    } else if ranges[i + 1][0] <= ranges[i][1].wrapping_add(1) {
                        // next range is partially covering the current range
                        ranges[i][1] = ranges[i + 1][1];
                        ranges.remove(i + 1);
                    }
                    i += 1;
                }

                values.clear();
                for (i, ran) in ranges.iter().enumerate() {
                    if i != 0 {
                        values.push(Token::new(CommaToken, gb(COMMA_BYTES), Hash(0), None));
                    }
                    if ran[0] == ran[1] {
                        let urange = format!("U+{}", go_hex_upper(ran[0])).into_bytes();
                        values.push(Token::new(
                            UnicodeRangeToken,
                            GoBytes::from_vec(urange),
                            Hash(0),
                            None,
                        ));
                    } else if ran[0] == 0 && ran[1] == 0x10FFFF {
                        values.push(Token::new(IdentToken, gb(INITIAL_BYTES), Hash(0), None));
                    } else {
                        let mut k = 0;
                        while k < 6
                            && (ran[0] >> (k * 4)) & 0xF == 0
                            && (ran[1] >> (k * 4)) & 0xF == 0xF
                        {
                            k += 1;
                        }
                        let mut wildcards = k;
                        while k < 6 {
                            if (ran[0] >> (k * 4)) & 0xF != (ran[1] >> (k * 4)) & 0xF {
                                wildcards = 0;
                                break;
                            }
                            k += 1;
                        }
                        let urange = if wildcards != 0 {
                            if ran[0] >> (wildcards * 4) == 0 {
                                format!("U+{}", "?".repeat(wildcards))
                            } else {
                                format!(
                                    "U+{}{}",
                                    go_hex_upper(ran[0] >> (wildcards * 4)),
                                    "?".repeat(wildcards)
                                )
                            }
                        } else {
                            format!("U+{}-{}", go_hex_upper(ran[0]), go_hex_upper(ran[1]))
                        };
                        values.push(Token::new(
                            UnicodeRangeToken,
                            GoBytes::from_vec(urange.into_bytes()),
                            Hash(0),
                            None,
                        ));
                    }
                }
            }
            _ => {}
        }
        values
    }

    // Go: css/css.go:cssMinifier.minifyProperty (case Background)
    fn minify_background(&mut self, mut values: Vec<Token>) -> Vec<Token> {
        let mut start: isize = 0;
        let mut end: isize = 0;
        while end <= values.len() as isize {
            // loop over comma-separated lists
            if end != values.len() as isize && values[end as usize].token_type != CommaToken {
                end += 1;
                continue;
            } else if start == end {
                start += 1;
                end += 1;
                continue;
            }

            // minify background-size and lowercase all identifiers
            let mut i = start;
            while i < end {
                let iu = i as usize;
                if values[iu].token_type == DelimToken && values[iu].data.at(0) == b'/' {
                    // background-size consists of either [<length-percentage> | auto | cover | contain] or [<length-percentage> | auto]{2}
                    // we can only minify the latter
                    if i + 1 < end
                        && (values[iu + 1].token_type == NumberToken
                            || values[iu + 1].is_length_percentage()
                            || values[iu + 1].ident == Auto)
                    {
                        if i + 2 < end
                            && (values[iu + 2].token_type == NumberToken
                                || values[iu + 2].is_length_percentage()
                                || values[iu + 2].ident == Auto)
                        {
                            let window = values[iu + 1..iu + 3].to_vec();
                            let size_values = self.minify_property(Background_Size, window);
                            if size_values.len() == 1 && size_values[0].ident == Auto {
                                // remove background-size if it is '/ auto' after minifying the property
                                values.drain(iu..iu + 3);
                                end -= 3;
                                i -= 1;
                            } else {
                                let n = size_values.len() as isize;
                                values.splice(iu + 1..iu + 3, size_values);
                                end -= 2 - n;
                                i += n - 1;
                            }
                        } else if values[iu + 1].ident == Auto {
                            // remove background-size if it is '/ auto'
                            values.drain(iu..iu + 2);
                            end -= 2;
                            i -= 1;
                        }
                    }
                }
                i += 1;
            }

            // minify all other values
            let mut i_padding_box: isize = -1; // position of background-origin that is padding-box
            let mut i = start;
            while i < end {
                let iu = i as usize;
                let h = values[iu].ident;
                values[iu] = minify_color(values[iu].clone());
                if values[iu].token_type == IdentToken {
                    if i + 1 < end
                        && values[iu + 1].token_type == IdentToken
                        && (h == Space || h == Round || h == Repeat || h == No_Repeat)
                    {
                        let h2 = values[iu + 1].ident;
                        if h2 == Space || h2 == Round || h2 == Repeat || h2 == No_Repeat {
                            let window = values[iu..iu + 2].to_vec();
                            let repeat_values = self.minify_property(Background_Repeat, window);
                            if repeat_values.len() == 1 && repeat_values[0].ident == Repeat {
                                values.drain(iu..iu + 2);
                                end -= 2;
                                i -= 1;
                            } else {
                                let n = repeat_values.len() as isize;
                                values.splice(iu..iu + 2, repeat_values);
                                end -= 2 - n;
                                i += n - 1;
                            }
                            i += 1;
                            continue;
                        }
                    } else if h == None || h == Scroll || h == Transparent {
                        values.remove(iu);
                        end -= 1;
                        i -= 1;
                        i += 1;
                        continue;
                    } else if h == Border_Box || h == Padding_Box {
                        if i_padding_box == -1 && h == Padding_Box {
                            // background-origin
                            i_padding_box = i;
                        } else if i_padding_box != -1 && h == Border_Box {
                            // background-clip
                            values.remove(iu);
                            values.remove(i_padding_box as usize);
                            end -= 2;
                            i -= 2;
                        }
                        i += 1;
                        continue;
                    }
                } else if values[iu].token_type == HashToken && values[iu].data.equal(BLACK_BYTES) {
                    values.remove(iu);
                    end -= 1;
                    i -= 1;
                    i += 1;
                    continue;
                } else if values[iu].token_type == FunctionToken && values[iu].data.equal(VAR_BYTES)
                {
                    i += 1;
                    continue;
                }

                // further minify background-position and background-size combination
                if values[iu].token_type == NumberToken
                    || values[iu].is_length_percentage()
                    || h == Left
                    || h == Right
                    || h == Top
                    || h == Bottom
                    || h == Center
                {
                    let mut j = iu + 1;
                    while j < values.len() {
                        let h = values[j].ident;
                        if h == Left || h == Right || h == Top || h == Bottom || h == Center {
                            j += 1;
                            continue;
                        } else if values[j].token_type == NumberToken
                            || values[j].is_length_percentage()
                        {
                            j += 1;
                            continue;
                        }
                        break;
                    }

                    let window = values[iu..j].to_vec();
                    let position_values = self.minify_property(Background_Position, window);
                    let has_size = j < values.len()
                        && values[j].token_type == DelimToken
                        && values[j].data.at(0) == b'/';
                    if !has_size
                        && position_values.len() == 2
                        && position_values[0].is_zero()
                        && position_values[1].is_zero()
                    {
                        if end - start == 2 {
                            values[iu] = Token::new(NumberToken, gb(ZERO_BYTES), Hash(0), Hash(0));
                            values[iu + 1] =
                                Token::new(NumberToken, gb(ZERO_BYTES), Hash(0), Hash(0));
                            i += 1;
                        } else {
                            values.drain(iu..j);
                            end -= (j - iu) as isize;
                            i -= 1;
                        }
                    } else {
                        let n = position_values.len() as isize;
                        if position_values.len() == j - iu {
                            for (k, position_value) in position_values.into_iter().enumerate() {
                                values[iu + k] = position_value;
                            }
                        } else {
                            values.splice(iu..j, position_values);
                            end -= (j - iu) as isize - n;
                        }
                        i += n - 1;
                    }
                }
                i += 1;
            }

            if end - start == 0 {
                let s = start as usize;
                values.splice(
                    s..end as usize,
                    [
                        Token::new(NumberToken, gb(ZERO_BYTES), Hash(0), Hash(0)),
                        Token::new(NumberToken, gb(ZERO_BYTES), Hash(0), Hash(0)),
                    ],
                );
                end += 2;
            }
            start = end + 1;
            end += 1;
        }
        values
    }

    // Go: css/css.go:cssMinifier.minifyDimension
    fn minify_dimension(&mut self, mut value: Token) -> (Token, GoBytes) {
        // TODO: add check for zero value
        let mut dim = GoBytes::nil();
        if value.token_type == DimensionToken {
            let mut n = value.data.len();
            while 0 < n {
                let c = value.data.at(n - 1);
                let lower = c.is_ascii_lowercase();
                let upper = c.is_ascii_uppercase();
                if !lower && !upper {
                    break;
                } else if upper {
                    value.data.set(n - 1, c + (b'a' - b'A'));
                }
                n -= 1;
            }

            let mut num = value.data.slice_to(n);
            if self.o.keep_css2 {
                num = decimal(num, self.o.precision); // don't use exponents
            } else {
                num = number(num, self.o.precision);
            }
            dim = value.data.slice_from(n);
            value.data = num.append_bytes(&dim);
        }
        (value, dim)
    }
}

// Go: css/css.go:cssMinifier.minifyProperty (case Background_Position)
fn minify_background_position(mut values: Vec<Token>) -> Vec<Token> {
    let mut start = 0usize;
    let mut end = 0usize;
    'outer: while end <= values.len() {
        // loop over comma-separated lists
        if end != values.len() && values[end].token_type != CommaToken {
            end += 1;
            continue;
        } else if start == end {
            start += 1;
            end += 1;
            continue;
        }

        if end - start == 3 || end - start == 4 {
            // remove zero offsets
            for i in [end - start - 1, start + 1] {
                if 2 < end - start && values[i].is_zero() {
                    values.remove(i);
                    end -= 1;
                }
            }

            let mut j = start + 1; // position of second set of horizontal/vertical values
            if 2 < end - start && values[start + 2].token_type == IdentToken {
                j = start + 2;
            }

            let mut b = GoBytes::make(0, 4);
            let mut offsets: Vec<Token> = vec![Token::default(), Token::default()];
            for i in [j, start] {
                if i + 1 < end && i + 1 != j {
                    if values[i + 1].token_type == PercentageToken {
                        // change right or bottom with percentage offset to left or top respectively
                        if values[i].ident == Right || values[i].ident == Bottom {
                            let d = values[i + 1].data.clone();
                            let (n, _) =
                                tdewolff_parse::strconv::parse_int(&d.slice_to(d.len() - 1));
                            b = b
                                .slice_to(0)
                                .append(100i64.wrapping_sub(n).to_string().as_bytes());
                            b = b.append_byte(b'%');
                            values[i + 1].data = b.clone();
                            if values[i].ident == Right {
                                values[i].data = gb(LEFT_BYTES);
                                values[i].ident = Left;
                            } else {
                                values[i].data = gb(TOP_BYTES);
                                values[i].ident = Top;
                            }
                        }
                    }
                    if values[i].ident == Left {
                        offsets[0] = values[i + 1].clone();
                    } else if values[i].ident == Top {
                        offsets[1] = values[i + 1].clone();
                    }
                } else if values[i].ident == Left {
                    offsets[0] = Token::new(NumberToken, gb(ZERO_BYTES), Hash(0), Hash(0));
                } else if values[i].ident == Top {
                    offsets[1] = Token::new(NumberToken, gb(ZERO_BYTES), Hash(0), Hash(0));
                } else if values[i].ident == Right {
                    offsets[0] = Token::new(PercentageToken, gb(N100P_BYTES), Hash(0), Hash(0));
                    values[i].ident = Left;
                } else if values[i].ident == Bottom {
                    offsets[1] = Token::new(PercentageToken, gb(N100P_BYTES), Hash(0), Hash(0));
                    values[i].ident = Top;
                }
            }

            if values[start].ident == Center || values[j].ident == Center {
                if values[start].ident == Left || values[j].ident == Left {
                    offsets.truncate(1);
                } else if values[start].ident == Top || values[j].ident == Top {
                    offsets[0] = Token::new(NumberToken, gb(N50P_BYTES), Hash(0), Hash(0));
                }
            }

            if !offsets[0].data.is_nil() && (offsets.len() == 1 || !offsets[1].data.is_nil()) {
                let k = offsets.len();
                values.splice(start..end, offsets);
                end -= end - start - k;
            }
        }
        // removing zero offsets in the previous loop might make it eligible for the next loop
        if end - start == 1 || end - start == 2 {
            if end - start == 1 && (values[start].ident == Top || values[start].ident == Bottom) {
                // we can't make this smaller, and converting to a number will break it
                // (https://github.com/tdewolff/minify/issues/221#issuecomment-415419918)
                break 'outer;
            }

            if end - start == 2
                && (values[start].ident == Top
                    || values[start].ident == Bottom
                    || values[start + 1].ident == Left
                    || values[start + 1].ident == Right)
            {
                // if it's a vertical position keyword, swap it with the next element
                // since otherwise converted number positions won't be valid anymore
                // (https://github.com/tdewolff/minify/issues/221#issue-353067229)
                values.swap(start, start + 1);
            }

            // transform keywords to lengths|percentages
            let mut i = start;
            while i < end {
                if values[i].token_type == IdentToken {
                    if values[i].ident == Left || values[i].ident == Top {
                        values[i].token_type = NumberToken;
                        values[i].data = gb(ZERO_BYTES);
                        values[i].ident = Hash(0);
                    } else if values[i].ident == Right || values[i].ident == Bottom {
                        values[i].token_type = PercentageToken;
                        values[i].data = gb(N100P_BYTES);
                        values[i].ident = Hash(0);
                    } else if values[i].ident == Center {
                        if i == start {
                            values[i].token_type = PercentageToken;
                            values[i].data = gb(N50P_BYTES);
                            values[i].ident = Hash(0);
                        } else {
                            values.remove(start + 1);
                            end -= 1;
                        }
                    }
                } else if i == start + 1
                    && values[i].token_type == PercentageToken
                    && values[i].data.equal(N50P_BYTES)
                {
                    values.remove(start + 1);
                    end -= 1;
                } else if values[i].token_type == PercentageToken && values[i].data.at(0) == b'0' {
                    values[i].token_type = NumberToken;
                    values[i].data = gb(ZERO_BYTES);
                    values[i].ident = Hash(0);
                }
                i += 1;
            }
        }
        start = end + 1;
        end += 1;
    }
    values
}

// Go: css/css.go:minifyColor
fn minify_color(mut value: Token) -> Token {
    let mut data = value.data.clone();
    if value.token_type == IdentToken {
        if let Some(hex_value) = shorten_color_name(value.ident) {
            value.token_type = HashToken;
            value.data = gb(hex_value);
        }
    } else if value.token_type == HashToken {
        to_lower(data.slice_from(1));
        if data.len() == 9 && data.at(7) == data.at(8) {
            if data.at(7) == b'f' {
                data = data.slice_to(7);
            } else if data.at(7) == b'0' {
                data = gb(BLACK_BYTES);
            }
        }
        if let Some(ident) = shorten_color_hex(&data.to_vec()) {
            value.token_type = IdentToken;
            data = gb(ident);
        } else if data.len() == 7
            && data.at(1) == data.at(2)
            && data.at(3) == data.at(4)
            && data.at(5) == data.at(6)
        {
            value.token_type = HashToken;
            data.set(2, data.at(3));
            data.set(3, data.at(5));
            data = data.slice_to(4);
        } else if data.len() == 9
            && data.at(1) == data.at(2)
            && data.at(3) == data.at(4)
            && data.at(5) == data.at(6)
            && data.at(7) == data.at(8)
        {
            // from working draft Color Module Level 4
            value.token_type = HashToken;
            data.set(2, data.at(3));
            data.set(3, data.at(5));
            data.set(4, data.at(7));
            data = data.slice_to(5);
        }
        value.data = data;
    }
    value
}

// Go: css/css.go:minifyNumberPercentage
fn minify_number_percentage(mut value: Token) -> Token {
    // assumes input already minified
    let d = value.data.clone();
    if value.token_type == PercentageToken && d.len() == 3 && d.at(d.len() - 2) == b'0' {
        d.set(1, d.at(0));
        d.set(0, b'.');
        value.data = d.slice_to(2);
        value.token_type = NumberToken;
    } else if value.token_type == NumberToken && 2 < d.len() && d.at(0) == b'.' && d.at(1) == b'0' {
        if d.at(2) == b'0' {
            d.set(0, b'.');
            d.slice_from(1).copy_from(&d.slice_from(3));
            d.set(d.len() - 2, b'%');
            value.data = d.slice_to(d.len() - 1);
            value.token_type = PercentageToken;
        } else if d.len() == 3 {
            d.set(0, d.at(2));
            d.set(1, b'%');
            value.data = d.slice_to(2);
            value.token_type = PercentageToken;
        }
    }
    value
}

// Go: css/css.go:minifyLengthPercentage
fn minify_length_percentage(mut value: Token) -> Token {
    if value.token_type != NumberToken && value.is_zero() {
        value.token_type = NumberToken;
        value.data = value.data.slice_to(1); // remove dimension for zero value
    }
    value
}
