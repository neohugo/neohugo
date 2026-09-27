//! Go: minify/json/json.go — minifies JSON following the specifications at
//! <http://json.org/>.

use tdewolff_parse::json::{
    ArrayState, EndArrayGrammar, EndObjectGrammar, ErrorGrammar, ObjectKeyState, ObjectValueState,
    Parser, StartArrayGrammar, StartObjectGrammar,
};
use tdewolff_parse::{GoError, GoReader, Input, Params, is_eof};

use crate::common::number;
use crate::{M, RestoreGuard, Writer};

static COMMA_BYTES: &[u8] = b",";
static COLON_BYTES: &[u8] = b":";
static ZERO_BYTES: &[u8] = b"0";
static MINUS_ZERO_BYTES: &[u8] = b"-0";

/// Go: json.Minifier — a JSON minifier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Minifier {
    /// number of significant digits
    pub precision: i64,
    /// prevent numbers from being minified
    pub keep_numbers: bool,
}

// Go: json/json.go:Minify
/// Minifies JSON data with the default options, it reads from r and writes
/// to w.
pub fn minify(
    m: &M,
    w: &mut dyn Writer,
    r: &mut dyn GoReader,
    params: Option<&Params>,
) -> Result<(), GoError> {
    crate::Minifier::minify(&Minifier::default(), m, w, r, params)
}

impl crate::Minifier for Minifier {
    // Go: json/json.go:Minifier.Minify
    /// Minifies JSON data, it reads from r and writes to w.
    fn minify(
        &self,
        _m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        _params: Option<&Params>,
    ) -> Result<(), GoError> {
        let o = self;
        let mut skip_comma = true;

        let z = Input::new(Some(r));
        let _restore = RestoreGuard(z.clone());

        let mut p = Parser::new(z);
        loop {
            let state = p.state();
            let (gt, mut text) = p.next();
            if gt == ErrorGrammar {
                w.write(&[])?;
                let err = p.err();
                if !is_eof(&err) {
                    return match err {
                        Some(e) => Err(e),
                        None => Ok(()),
                    };
                }
                return Ok(());
            }

            if !skip_comma && gt != EndObjectGrammar && gt != EndArrayGrammar {
                if state == ObjectKeyState || state == ArrayState {
                    let _ = w.write(COMMA_BYTES);
                } else if state == ObjectValueState {
                    let _ = w.write(COLON_BYTES);
                }
            }
            skip_comma = gt == StartObjectGrammar || gt == StartArrayGrammar;

            if !o.keep_numbers
                && 0 < text.len()
                && (text.at(0).is_ascii_digit() || text.at(0) == b'-')
            {
                text = number(text, o.precision);
                if text.at(0) == b'.' {
                    let _ = w.write(ZERO_BYTES);
                } else if 1 < text.len() && text.at(0) == b'-' && text.at(1) == b'.' {
                    text = text.slice_from(1);
                    let _ = w.write(MINUS_ZERO_BYTES);
                }
            }
            let _ = w.write_go(&text);
        }
    }
}
