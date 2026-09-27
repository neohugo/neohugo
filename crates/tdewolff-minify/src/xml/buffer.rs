//! Go: minify/xml/buffer.go

use tdewolff_parse::GoBytes;
use tdewolff_parse::xml::{AttributeToken, ErrorToken, Lexer, TokenType};

use crate::tokbuf::GoTokenBuf;

/// Go: xml.Token — a single token unit with an attribute value (if given).
#[derive(Clone, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub data: GoBytes,
    pub text: GoBytes,
    pub attr_val: GoBytes,
}

impl Default for Token {
    fn default() -> Token {
        Token {
            token_type: ErrorToken,
            data: GoBytes::nil(),
            text: GoBytes::nil(),
            attr_val: GoBytes::nil(),
        }
    }
}

/// Go: xml.TokenBuffer — a buffer that allows for token look-ahead.
pub struct TokenBuffer {
    l: Lexer,
    b: GoTokenBuf<Token>,
}

// Go: xml/buffer.go:TokenBuffer.read
fn read(l: &mut Lexer, t: &mut Token) {
    (t.token_type, t.data) = l.next();
    t.text = l.text();
    if t.token_type == AttributeToken {
        t.attr_val = l.attr_val();
    } else {
        t.attr_val = GoBytes::nil();
    }
}

impl TokenBuffer {
    // Go: xml/buffer.go:NewTokenBuffer
    /// Returns a new TokenBuffer.
    pub fn new(l: Lexer) -> TokenBuffer {
        TokenBuffer {
            l,
            b: GoTokenBuf::new(),
        }
    }

    /// The lexer (Go keeps its own `l` pointer next to the buffer).
    pub fn lexer(&self) -> &Lexer {
        &self.l
    }

    /// `(z.pos, len(z.buf))` (used by the ported upstream buffer tests).
    #[doc(hidden)]
    pub fn pos_len(&self) -> (usize, usize) {
        (self.b.pos, self.b.len)
    }

    /// The buffer slot `Peek(pos)` returns (Go compares the `*Token`s).
    #[doc(hidden)]
    pub fn peek_index(&mut self, pos: usize) -> usize {
        let l = &mut self.l;
        self.b
            .peek(pos, |t| t.token_type == ErrorToken, |t| read(l, t))
    }

    // Go: xml/buffer.go:TokenBuffer.Peek
    /// Returns the ith element and possibly does an allocation. Peeking past
    /// an error returns the error token.
    pub fn peek(&mut self, pos: usize) -> &Token {
        let l = &mut self.l;
        let i = self
            .b
            .peek(pos, |t| t.token_type == ErrorToken, |t| read(l, t));
        &self.b.buf[i]
    }

    // Go: xml/buffer.go:TokenBuffer.Shift
    /// Returns the first element and advances position.
    pub fn shift(&mut self) -> &mut Token {
        let l = &mut self.l;
        let i = self.b.shift(|t| read(l, t));
        &mut self.b.buf[i]
    }
}
