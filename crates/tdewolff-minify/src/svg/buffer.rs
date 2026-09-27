//! Go: minify/svg/buffer.go

use tdewolff_parse::xml::{
    AttributeToken, EndTagToken, ErrorToken, Lexer, StartTagToken, TokenType,
};
use tdewolff_parse::{
    GoBytes, Input, NilMap, replace_multiple_whitespace_and_entities, trim_whitespace,
};

use super::hash::{Hash, to_hash};
use crate::tokbuf::GoTokenBuf;

/// Go: svg.Token — a single token unit with an attribute value (if given)
/// and hash of the data.
#[derive(Clone, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub hash: Hash,
    pub data: GoBytes,
    pub text: GoBytes,
    pub attr_val: GoBytes,
    pub offset: usize,
}

impl Default for Token {
    fn default() -> Token {
        Token {
            token_type: ErrorToken,
            hash: Hash(0),
            data: GoBytes::nil(),
            text: GoBytes::nil(),
            attr_val: GoBytes::nil(),
            offset: 0,
        }
    }
}

/// Go: svg.TokenBuffer — a buffer that allows for token look-ahead.
pub struct TokenBuffer {
    r: Input,
    l: Lexer,
    b: GoTokenBuf<Token>,
}

// Go: svg/buffer.go:TokenBuffer.read
fn read(r: &Input, l: &mut Lexer, t: &mut Token) {
    t.offset = r.offset();
    (t.token_type, t.data) = l.next();
    t.text = l.text();
    if t.token_type == AttributeToken {
        t.offset += 1 + t.text.len() + 1;
        t.attr_val = l.attr_val();
        if t.attr_val.len() > 1 && (t.attr_val.at(0) == b'"' || t.attr_val.at(0) == b'\'') {
            t.offset += 1;
            t.attr_val = t.attr_val.slice(1, t.attr_val.len() - 1); // quotes will be readded in attribute loop if necessary
            t.attr_val = replace_multiple_whitespace_and_entities(
                t.attr_val.clone(),
                &crate::xml::EntitiesMap,
                &NilMap,
            );
            t.attr_val = trim_whitespace(&t.attr_val);
        }
        t.hash = to_hash(&t.text);
    } else if t.token_type == StartTagToken || t.token_type == EndTagToken {
        t.attr_val = GoBytes::nil();
        t.hash = to_hash(&t.text);
    } else {
        t.attr_val = GoBytes::nil();
        t.hash = Hash(0);
    }
}

impl TokenBuffer {
    // Go: svg/buffer.go:NewTokenBuffer
    /// Returns a new TokenBuffer.
    pub fn new(r: Input, l: Lexer) -> TokenBuffer {
        TokenBuffer {
            r,
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
        let (r, l) = (&self.r, &mut self.l);
        self.b
            .peek(pos, |t| t.token_type == ErrorToken, |t| read(r, l, t))
    }

    // Go: svg/buffer.go:TokenBuffer.Peek
    /// Returns the ith element and possibly does an allocation. Peeking past
    /// an error returns the error token.
    pub fn peek(&mut self, pos: usize) -> &Token {
        let (r, l) = (&self.r, &mut self.l);
        let i = self
            .b
            .peek(pos, |t| t.token_type == ErrorToken, |t| read(r, l, t));
        &self.b.buf[i]
    }

    // Go: svg/buffer.go:TokenBuffer.Shift
    /// Returns the first element and advances position.
    pub fn shift(&mut self) -> &mut Token {
        let (r, l) = (&self.r, &mut self.l);
        let i = self.b.shift(|t| read(r, l, t));
        &mut self.b.buf[i]
    }

    // Go: svg/buffer.go:TokenBuffer.Attributes
    /// Extracts the given attribute hashes from a tag. It returns in the same
    /// order the indices (see [`TokenBuffer::token_mut`]) of the requested
    /// tokens, or `None`.
    pub fn attributes(&mut self, hashes: &[Hash]) -> Vec<Option<usize>> {
        let mut n = 0;
        loop {
            if self.peek(n).token_type != AttributeToken {
                break;
            }
            n += 1;
        }
        let mut attr_buffer = vec![Option::None; hashes.len()];
        for i in self.b.pos..self.b.pos + n {
            let attr = &self.b.buf[i];
            for (j, &hash) in hashes.iter().enumerate() {
                if hash == attr.hash {
                    attr_buffer[j] = Some(i);
                }
            }
        }
        attr_buffer
    }

    /// The buffered token at an index returned by [`TokenBuffer::attributes`]
    /// (Go dereferences the returned `*Token`).
    pub fn token_mut(&mut self, i: usize) -> &mut Token {
        &mut self.b.buf[i]
    }
}
