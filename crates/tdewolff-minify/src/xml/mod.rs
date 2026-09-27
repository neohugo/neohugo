//! Go: minify/xml/xml.go — minifies XML1.0 following the specifications at
//! <http://www.w3.org/TR/xml/>.

pub mod buffer;
pub mod table;

pub use buffer::{Token, TokenBuffer};
pub use table::{EntitiesMap, TextRevEntitiesMap};

use tdewolff_parse::xml::{
    AttributeToken, CDATAToken, DOCTYPEToken, EndTagToken, ErrorToken, Lexer, StartTagClosePIToken,
    StartTagCloseToken, StartTagCloseVoidToken, StartTagPIToken, StartTagToken, TextToken,
    escape_attr_val, escape_cdata_val,
};
use tdewolff_parse::{
    GoBytes, GoError, GoReader, Input, NilMap, Params, is_all_whitespace, is_eof, is_whitespace,
    replace_entities, replace_multiple_whitespace_and_entities,
};

use crate::{M, RestoreGuard, Writer};

static IS_BYTES: &[u8] = b"=";
static SPACE_BYTES: &[u8] = b" ";
static VOID_BYTES: &[u8] = b"/>";

/// Go: xml.Minifier — an XML minifier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Minifier {
    pub keep_whitespace: bool,
}

// Go: xml/xml.go:Minify
/// Minifies XML data with the default options, it reads from r and writes
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
    // Go: xml/xml.go:Minifier.Minify
    /// Minifies XML data, it reads from r and writes to w.
    fn minify(
        &self,
        _m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        _params: Option<&Params>,
    ) -> Result<(), GoError> {
        let o = self;
        let mut omit_space = true; // on true the next text token must not start with a space

        let mut attr_byte_buffer = GoBytes::make(0, 64);

        let z = Input::new(Some(r));
        let _restore = RestoreGuard(z.clone());

        let l = Lexer::new(z);
        let mut tb = TokenBuffer::new(l);
        loop {
            let mut t = tb.shift().clone();
            if t.token_type == CDATAToken {
                // convert CDATA to regular text if smaller
                if t.text.len() == 0 {
                    continue;
                } else {
                    let (text, use_text) = escape_cdata_val(&mut attr_byte_buffer, &t.text);
                    if use_text {
                        t.data = text;
                    }
                }
            }
            match t.token_type {
                ErrorToken => {
                    w.write(&[])?;
                    let err = tb.lexer().err();
                    if is_eof(&err) {
                        return Ok(());
                    }
                    return match err {
                        Some(e) => Err(e),
                        None => Ok(()),
                    };
                }
                DOCTYPEToken => {
                    let _ = w.write_go(&t.data);
                }
                CDATAToken => {
                    let _ = w.write_go(&t.data);
                    if t.text.len() > 0 && is_whitespace(t.text.at(t.text.len() - 1)) {
                        omit_space = true;
                    }
                }
                TextToken => {
                    t.data = replace_multiple_whitespace_and_entities(
                        t.data,
                        &EntitiesMap,
                        &TextRevEntitiesMap,
                    );

                    // whitespace removal; trim left
                    if omit_space && is_whitespace(t.data.at(0)) {
                        t.data = t.data.slice_from(1);
                    }

                    // whitespace removal; trim right
                    omit_space = false;
                    if t.data.len() == 0 {
                        omit_space = true;
                    } else if is_whitespace(t.data.at(t.data.len() - 1)) {
                        omit_space = true;
                        let mut i = 0;
                        loop {
                            let next = tb.peek(i);
                            // trim if EOF, text token with whitespace begin or block token
                            if next.token_type == ErrorToken {
                                t.data = t.data.slice_to(t.data.len() - 1);
                                omit_space = false;
                                break;
                            } else if next.token_type == TextToken {
                                // this only happens when a comment, doctype, cdata startpi tag was in between
                                // remove if the text token starts with a whitespace
                                if next.data.len() > 0 && is_whitespace(next.data.at(0)) {
                                    t.data = t.data.slice_to(t.data.len() - 1);
                                    omit_space = false;
                                }
                                break;
                            } else if next.token_type == CDATAToken {
                                if next.text.len() > 0 && is_whitespace(next.text.at(0)) {
                                    t.data = t.data.slice_to(t.data.len() - 1);
                                    omit_space = false;
                                }
                                break;
                            } else if next.token_type == StartTagToken
                                || next.token_type == EndTagToken
                            {
                                if !o.keep_whitespace {
                                    t.data = t.data.slice_to(t.data.len() - 1);
                                    omit_space = false;
                                }
                                break;
                            }
                            i += 1;
                        }
                    }
                    let _ = w.write_go(&t.data);
                }
                StartTagToken => {
                    let _ = w.write_go(&t.data);
                    if o.keep_whitespace {
                        omit_space = false;
                    }
                }
                StartTagPIToken => {
                    let _ = w.write_go(&t.data);
                }
                AttributeToken => {
                    let _ = w.write(SPACE_BYTES);
                    let _ = w.write_go(&t.text);
                    let _ = w.write(IS_BYTES);

                    if t.attr_val.len() < 2
                        || t.attr_val.at(0) != b'"'
                        || t.attr_val.at(t.attr_val.len() - 1) != b'"'
                    {
                        let _ = w.write_go(&t.attr_val);
                    } else {
                        let mut val = t.attr_val.slice(1, t.attr_val.len() - 1);
                        val = replace_entities(val, &EntitiesMap, &NilMap);
                        val = escape_attr_val(&mut attr_byte_buffer, &val); // prefer single or double quotes depending on what occurs more often in value
                        let _ = w.write_go(&val);
                    }
                }
                StartTagCloseToken => {
                    let mut next = tb.peek(0).token_type;
                    let mut skip_extra = false;
                    if next == TextToken && is_all_whitespace(&tb.peek(0).data) {
                        next = tb.peek(1).token_type;
                        skip_extra = true;
                    }
                    if next == EndTagToken {
                        // collapse empty tags to single void tag
                        tb.shift();
                        if skip_extra {
                            tb.shift();
                        }
                        let _ = w.write(VOID_BYTES);
                    } else {
                        let _ = w.write_go(&t.data);
                    }
                }
                StartTagCloseVoidToken => {
                    let _ = w.write_go(&t.data);
                }
                StartTagClosePIToken => {
                    let _ = w.write_go(&t.data);
                }
                EndTagToken => {
                    if t.data.len() > 3 + t.text.len() {
                        t.data.set(2 + t.text.len(), b'>');
                        t.data = t.data.slice_to(3 + t.text.len());
                    }
                    let _ = w.write_go(&t.data);
                    if o.keep_whitespace {
                        omit_space = false;
                    }
                }
                _ => {}
            }
        }
    }
}
