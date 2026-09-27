//! Go: parse/json/parse.go — a JSON parser following the specifications at
//! <http://json.org/>.

use crate::error::{Error, GoError, new_error_lexer, new_error_lexer_err};
use crate::gobytes::GoBytes;
use crate::input::Input;

/// Go: json.GrammarType — the type of grammar.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GrammarType {
    ErrorGrammar = 0, // extra grammar when errors occur
    WhitespaceGrammar,
    LiteralGrammar,
    NumberGrammar,
    StringGrammar,
    StartObjectGrammar, // {
    EndObjectGrammar,   // }
    StartArrayGrammar,  // [
    EndArrayGrammar,    // ]
}

pub use GrammarType::*;

impl GrammarType {
    // Go: parse/json/parse.go:GrammarType.String
    /// Returns the string representation of a GrammarType.
    pub fn string(self) -> String {
        grammar_type_string(self as u32)
    }
}

/// `GrammarType(n).String()` for any integer value.
pub fn grammar_type_string(n: u32) -> String {
    match n {
        0 => "Error",
        1 => "Whitespace",
        2 => "Literal",
        3 => "Number",
        4 => "String",
        5 => "StartObject",
        6 => "EndObject",
        7 => "StartArray",
        8 => "EndArray",
        _ => return format!("Invalid({})", n),
    }
    .to_string()
}

/// Go: json.State — the state the parser is in.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    ValueState = 0, // extra token when errors occur
    ObjectKeyState,
    ObjectValueState,
    ArrayState,
}

pub use State::*;

impl State {
    // Go: parse/json/parse.go:State.String
    /// Returns the string representation of a State.
    pub fn string(self) -> String {
        state_string(self as u32)
    }
}

/// `State(n).String()` for any integer value.
pub fn state_string(n: u32) -> String {
    match n {
        0 => "Value",
        1 => "ObjectKey",
        2 => "ObjectValue",
        3 => "Array",
        _ => return format!("Invalid({})", n),
    }
    .to_string()
}

/// Go: json.Parser — the state for the parser.
pub struct Parser {
    r: Input,
    state: Vec<State>,
    err: Option<GoError>,

    need_comma: bool,
}

impl Parser {
    // Go: parse/json/parse.go:NewParser
    /// Returns a new Parser for a given Input.
    pub fn new(r: Input) -> Parser {
        Parser {
            r,
            state: vec![ValueState],
            err: None,
            need_comma: false,
        }
    }

    /// The underlying `*parse.Input` (shared handle).
    pub fn input(&self) -> &Input {
        &self.r
    }

    // Go: parse/json/parse.go:Parser.Err
    /// Returns the error encountered during tokenization, this is often
    /// io.EOF but also other errors can be returned.
    pub fn err(&self) -> Option<GoError> {
        if self.err.is_some() {
            return self.err.clone();
        }
        self.r.err()
    }

    // Go: parse/json/parse.go:Parser.State
    /// Returns the state the parser is currently in (ie. which token is
    /// expected).
    pub fn state(&self) -> State {
        *self.state.last().expect("state")
    }

    fn set_err(&mut self, msg: &str) {
        self.err = Some(new_error_lexer_err(&self.r, msg));
    }

    // Go: parse/json/parse.go:Parser.Next
    /// Returns the next Grammar. It returns ErrorGrammar when an error was
    /// encountered. Using Err() one can retrieve the error message.
    pub fn next(&mut self) -> (GrammarType, GoBytes) {
        self.move_whitespace();
        let mut c = self.r.peek(0);
        let state = *self.state.last().expect("state");
        if c == b',' {
            if state != ArrayState && state != ObjectKeyState {
                self.set_err("unexpected comma character");
                return (ErrorGrammar, GoBytes::nil());
            }
            self.r.move_(1);
            self.move_whitespace();
            self.need_comma = false;
            c = self.r.peek(0);
        }
        self.r.skip();

        if self.need_comma && c != b'}' && c != b']' && c != 0 {
            self.set_err("expected comma character or an array or object ending");
            return (ErrorGrammar, GoBytes::nil());
        } else if c == b'{' {
            self.state.push(ObjectKeyState);
            self.r.move_(1);
            return (StartObjectGrammar, self.r.shift());
        } else if c == b'}' {
            if state != ObjectKeyState {
                self.set_err("unexpected right brace character");
                return (ErrorGrammar, GoBytes::nil());
            }
            self.need_comma = true;
            self.state.pop();
            let last = self.state.len() - 1;
            if self.state[last] == ObjectValueState {
                self.state[last] = ObjectKeyState;
            }
            self.r.move_(1);
            return (EndObjectGrammar, self.r.shift());
        } else if c == b'[' {
            self.state.push(ArrayState);
            self.r.move_(1);
            return (StartArrayGrammar, self.r.shift());
        } else if c == b']' {
            self.need_comma = true;
            if state != ArrayState {
                self.set_err("unexpected right bracket character");
                return (ErrorGrammar, GoBytes::nil());
            }
            self.state.pop();
            let last = self.state.len() - 1;
            if self.state[last] == ObjectValueState {
                self.state[last] = ObjectKeyState;
            }
            self.r.move_(1);
            return (EndArrayGrammar, self.r.shift());
        } else if state == ObjectKeyState {
            if c != b'"' || !self.consume_string_token() {
                self.set_err("expected object key to be a quoted string");
                return (ErrorGrammar, GoBytes::nil());
            }
            let n = self.r.pos();
            self.move_whitespace();
            let c = self.r.peek(0);
            if c != b':' {
                self.set_err("expected colon character after object key");
                return (ErrorGrammar, GoBytes::nil());
            }
            self.r.move_(1);
            let last = self.state.len() - 1;
            self.state[last] = ObjectValueState;
            return (StringGrammar, self.r.shift().slice_to(n));
        } else {
            self.need_comma = true;
            if state == ObjectValueState {
                let last = self.state.len() - 1;
                self.state[last] = ObjectKeyState;
            }
            if c == b'"' && self.consume_string_token() {
                return (StringGrammar, self.r.shift());
            } else if self.consume_number_token() {
                return (NumberGrammar, self.r.shift());
            } else if self.consume_literal_token() {
                return (LiteralGrammar, self.r.shift());
            }
            let c = self.r.peek(0); // pick up movement from consumeStringToken to detect NULL or EOF
            if c == 0 && !self.r.has_err() {
                self.set_err("unexpected NULL character");
                return (ErrorGrammar, GoBytes::nil());
            } else if c == 0 {
                // EOF
                return (ErrorGrammar, GoBytes::nil());
            }
        }
        // fmt.Sprintf("unexpected character '%c'", c) on a byte: the rune U+00XX
        let mut msg = b"unexpected character '".to_vec();
        crate::utf8::append_rune(&mut msg, c as i32);
        msg.push(b'\'');
        let e: Error = new_error_lexer(&self.r, msg);
        self.err = Some(GoError::Parse(Box::new(e)));
        (ErrorGrammar, GoBytes::nil())
    }

    ////////////////////////////////////////////////////////////////

    // The following functions follow the specifications at http://json.org/

    // Go: parse/json/parse.go:Parser.moveWhitespace
    fn move_whitespace(&mut self) {
        loop {
            let c = self.r.peek(0);
            if c != b' ' && c != b'\n' && c != b'\r' && c != b'\t' {
                break;
            }
            self.r.move_(1);
        }
    }

    // Go: parse/json/parse.go:Parser.consumeLiteralToken
    fn consume_literal_token(&mut self) -> bool {
        let c = self.r.peek(0);
        if c == b't' && self.r.peek(1) == b'r' && self.r.peek(2) == b'u' && self.r.peek(3) == b'e' {
            self.r.move_(4);
            return true;
        } else if c == b'f'
            && self.r.peek(1) == b'a'
            && self.r.peek(2) == b'l'
            && self.r.peek(3) == b's'
            && self.r.peek(4) == b'e'
        {
            self.r.move_(5);
            return true;
        } else if c == b'n'
            && self.r.peek(1) == b'u'
            && self.r.peek(2) == b'l'
            && self.r.peek(3) == b'l'
        {
            self.r.move_(4);
            return true;
        }
        false
    }

    // Go: parse/json/parse.go:Parser.consumeNumberToken
    fn consume_number_token(&mut self) -> bool {
        let mut mark = self.r.pos();
        if self.r.peek(0) == b'-' {
            self.r.move_(1);
        }
        let c = self.r.peek(0);
        if (b'1'..=b'9').contains(&c) {
            self.r.move_(1);
            loop {
                let c = self.r.peek(0);
                if !c.is_ascii_digit() {
                    break;
                }
                self.r.move_(1);
            }
        } else if c != b'0' {
            self.r.rewind(mark);
            return false;
        } else {
            self.r.move_(1); // 0
        }
        if self.r.peek(0) == b'.' {
            self.r.move_(1);
            if !self.r.peek(0).is_ascii_digit() {
                self.r.move_(-1);
                return true;
            }
            loop {
                if !self.r.peek(0).is_ascii_digit() {
                    break;
                }
                self.r.move_(1);
            }
        }
        mark = self.r.pos();
        let c = self.r.peek(0);
        if c == b'e' || c == b'E' {
            self.r.move_(1);
            let c = self.r.peek(0);
            if c == b'+' || c == b'-' {
                self.r.move_(1);
            }
            if !self.r.peek(0).is_ascii_digit() {
                self.r.rewind(mark);
                return true;
            }
            loop {
                if !self.r.peek(0).is_ascii_digit() {
                    break;
                }
                self.r.move_(1);
            }
        }
        true
    }

    // Go: parse/json/parse.go:Parser.consumeStringToken
    fn consume_string_token(&mut self) -> bool {
        // assume to be on "
        self.r.move_(1);
        loop {
            let c = self.r.peek(0);
            if c == b'"' {
                let mut escaped = false;
                let lexeme = self.r.lexeme();
                let mut i = self.r.pos() as isize - 1;
                while i >= 0 {
                    if lexeme.at(i as usize) == b'\\' {
                        escaped = !escaped;
                    } else {
                        break;
                    }
                    i -= 1;
                }
                if !escaped {
                    self.r.move_(1);
                    break;
                }
            } else if c == 0 {
                return false;
            }
            self.r.move_(1);
        }
        true
    }
}
