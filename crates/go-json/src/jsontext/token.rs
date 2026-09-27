//! Port of the parts of `encoding/json/jsontext/token.go` (go1.27.1) that
//! the v1 API reaches.
//!
//! A Go `Token` either points into a decoder buffer (`raw`) or holds a
//! string or number value. Here a decoded string or number token carries a
//! copy of its raw bytes ([`Token::Raw`]).

use super::state::{Kind, normalize};
use crate::goerr::Err;
use crate::jsonflags::Flags;
use crate::jsonwire;

// Go: token.go:Token
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    /// `Null` (rawToken("null"))
    Null,
    /// `False`
    False,
    /// `True`
    True,
    /// `BeginObject`
    BeginObject,
    /// `EndObject`
    EndObject,
    /// `BeginArray`
    BeginArray,
    /// `EndArray`
    EndArray,
    /// A raw JSON string or number: a token read by a Decoder, or
    /// `zeroString` / `zeroNumber`.
    Raw(Vec<u8>),
    /// `String(s)` for a non-empty s (Go's `str` field).
    Str(Vec<u8>),
    /// `Float(n)`: Go `str == "f"`, `num` = float64 bits (non-zero).
    Float(u64),
}

impl Token {
    // Go: token.go:String (the constructor)
    /// String constructs a Token representing a JSON string.
    /// The provided string should contain valid UTF-8, otherwise invalid characters
    /// may be mangled as the Unicode replacement character.
    pub(crate) fn string(s: &[u8]) -> Token {
        if s.is_empty() {
            return Token::Raw(b"\"\"".to_vec()); // zeroString
        }
        Token::Str(s.to_vec())
    }

    // Go: token.go:Float
    /// Float constructs a Token representing a JSON number.
    /// The values NaN, +Inf, and -Inf will be represented
    /// as a JSON string with the values "NaN", "Infinity", and "-Infinity".
    pub(crate) fn float(n: f64) -> Token {
        if n.to_bits() == 0 {
            Token::Raw(b"0".to_vec()) // zeroNumber
        } else if n.is_nan() {
            Token::Str(b"NaN".to_vec())
        } else if n == f64::INFINITY {
            Token::Str(b"Infinity".to_vec())
        } else if n == f64::NEG_INFINITY {
            Token::Str(b"-Infinity".to_vec())
        } else {
            Token::Float(n.to_bits())
        }
    }

    // Go: token.go:Bool (the constructor)
    pub(crate) fn bool(b: bool) -> Token {
        if b { Token::True } else { Token::False }
    }

    // Go: token.go:Token.Kind
    /// Kind returns the token kind.
    pub(crate) fn kind(&self) -> Kind {
        match self {
            Token::Null => b'n',
            Token::False => b'f',
            Token::True => b't',
            Token::BeginObject => b'{',
            Token::EndObject => b'}',
            Token::BeginArray => b'[',
            Token::EndArray => b']',
            Token::Raw(b) => normalize(b[0]),
            Token::Str(_) => b'"',
            Token::Float(_) => b'0',
        }
    }

    // Go: token.go:Token.Bool
    pub(crate) fn as_bool(&self) -> bool {
        match self {
            Token::True => true,
            Token::False => false,
            _ => panic!(
                "invalid JSON token kind: {}",
                super::state::kind_string(self.kind())
            ),
        }
    }

    // Go: token.go:Token.String / Token.string
    /// String returns the unescaped string value for a JSON string.
    /// For other JSON kinds, this returns the raw JSON representation.
    pub(crate) fn to_go_string(&self) -> Vec<u8> {
        match self {
            Token::Raw(buf) => {
                if buf[0] == b'"' {
                    let is_verbatim = jsonwire::consume_simple_string(buf) == buf.len();
                    return jsonwire::unquote_may_copy(buf, is_verbatim);
                }
                buf.clone()
            }
            Token::Str(s) => s.clone(),
            Token::Float(bits) => {
                let mut b = Vec::new();
                jsonwire::append_float(&mut b, f64::from_bits(*bits), 64);
                b
            }
            Token::Null => b"null".to_vec(),
            Token::False => b"false".to_vec(),
            Token::True => b"true".to_vec(),
            Token::BeginObject => b"{".to_vec(),
            Token::EndObject => b"}".to_vec(),
            Token::BeginArray => b"[".to_vec(),
            Token::EndArray => b"]".to_vec(),
        }
    }

    // Go: token.go:Token.appendString
    pub(crate) fn append_string(&self, dst: &mut Vec<u8>, flags: &Flags) -> Option<Err> {
        match self {
            Token::Raw(buf) if buf[0] == b'"' => {
                if jsonwire::consume_simple_string(buf) == buf.len() {
                    dst.extend_from_slice(buf);
                    return None;
                }
                let (_, err) = jsonwire::reformat_string(dst, buf, flags);
                err
            }
            Token::Str(s) => jsonwire::append_quote(dst, s, flags),
            _ => panic!(
                "invalid JSON token kind: {}",
                super::state::kind_string(self.kind())
            ),
        }
    }

    // Go: token.go:Token.appendNumber
    pub(crate) fn append_number(&self, dst: &mut Vec<u8>, flags: &Flags) -> Option<Err> {
        match self {
            Token::Raw(buf) if normalize(buf[0]) == b'0' => {
                let (_, err) = jsonwire::reformat_number(dst, buf, flags);
                err
            }
            Token::Float(bits) => {
                jsonwire::append_float(dst, f64::from_bits(*bits), 64);
                None
            }
            _ => panic!(
                "invalid JSON token kind: {}",
                super::state::kind_string(self.kind())
            ),
        }
    }

    // Go: token.go:Token.Float (the accessor, 64-bit)
    /// Float returns the floating-point value for a JSON number.
    /// (Only the raw-number case is reachable from the v1 API.) The error
    /// is Go's `*numError`, only ever for ErrRange.
    pub(crate) fn float_value(&self) -> (f64, bool) {
        match self {
            Token::Raw(buf) if normalize(buf[0]) == b'0' => {
                let (fv, err) = go_strconv::internal::parse_float(buf, 64);
                (fv, err.is_none())
            }
            Token::Float(bits) => (f64::from_bits(*bits), true),
            _ => panic!(
                "invalid JSON token kind: {}",
                super::state::kind_string(self.kind())
            ),
        }
    }
}
