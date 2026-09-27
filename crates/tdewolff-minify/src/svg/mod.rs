//! Go: minify/svg/svg.go — minifies SVG1.1 following the specifications at
//! <http://www.w3.org/TR/SVG11/>.

pub mod buffer;
pub mod hash;
pub mod pathdata;
pub mod table;

pub use buffer::{Token, TokenBuffer};
pub use hash::*;
pub use pathdata::{PathData, PathDataState};
pub use table::color_attr_map;

use tdewolff_parse::xml::{
    AttributeToken, CDATAToken, CommentToken, DOCTYPEToken, EndTagToken, ErrorToken, Lexer,
    StartTagClosePIToken, StartTagCloseToken, StartTagCloseVoidToken, StartTagPIToken,
    StartTagToken, TextToken, escape_attr_val, escape_cdata_val,
};
use tdewolff_parse::{
    GoBytes, GoError, GoReader, Input, NilMap, Params, dimension, equal_fold, is_all_whitespace,
    is_eof, replace_multiple_whitespace, replace_multiple_whitespace_and_entities, to_lower,
    trim_whitespace,
};

use crate::common::{mediatype, number, update_error_position};
use crate::{M, RestoreGuard, Writer, is_err_not_exist, param_is, params1};

static VOID_BYTES: &[u8] = b"/>";
static IS_BYTES: &[u8] = b"=";
static SPACE_BYTES: &[u8] = b" ";
static CDATA_END_BYTES: &[u8] = b"]]>";
static ZERO_BYTES: &[u8] = b"0";
static CSS_MIME_BYTES: &[u8] = b"text/css";
static NONE_BYTES: &[u8] = b"none";
static URL_BYTES: &[u8] = b"url(";

/// Go: svg.Minifier — an SVG minifier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Minifier {
    pub keep_comments: bool,
    /// number of significant digits
    pub precision: i64,
    pub inline: bool,
}

// Go: svg/svg.go:Minify
/// Minifies SVG data with the default options, it reads from r and writes
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
    // Go: svg/svg.go:Minifier.Minify
    /// Minifies SVG data, it reads from r and writes to w.
    fn minify(
        &self,
        m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        params: Option<&Params>,
    ) -> Result<(), GoError> {
        let mut o = self.clone();

        let mut new_precision = o.precision;
        if new_precision <= 0 || 15 < new_precision {
            new_precision = 15; // minimum number of digits a double can represent exactly
        }
        if !o.inline {
            o.inline = param_is(params, b"inline", b"1");
        }

        // namespaces to keep
        let namespaces: [Hash; 1] = [Xlink];

        let mut tag = Hash(0);
        let mut default_style_type: GoBytes = GoBytes::from_static(CSS_MIME_BYTES);
        let default_style_params: Option<&Params> = Option::None;
        let default_inline_style_params = params1(b"inline", b"1");

        let mut p = PathData::with_precisions(o.precision, new_precision);
        let mut minify_buffer = tdewolff_parse::buffer::Writer::new(GoBytes::make(0, 64));
        let mut attr_byte_buffer = GoBytes::make(0, 64);

        let z = Input::new(Some(r));
        let _restore = RestoreGuard(z.clone());

        let l = Lexer::new(z.clone());
        let mut tb = TokenBuffer::new(z.clone(), l);
        loop {
            let mut t = tb.shift().clone();
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
                CommentToken => {
                    if o.keep_comments {
                        let _ = w.write_go(&t.data);
                    }
                }
                DOCTYPEToken => {
                    if t.text.len() > 0 && t.text.at(t.text.len() - 1) == b']' {
                        let _ = w.write_go(&t.data);
                    }
                }
                TextToken => {
                    t.data = replace_multiple_whitespace_and_entities(
                        t.data,
                        &crate::xml::EntitiesMap,
                        &NilMap,
                    );
                    t.data = trim_whitespace(&t.data);

                    if tag == Style && t.data.len() > 0 {
                        let mut rd = tdewolff_parse::buffer::Reader::new(t.data.clone());
                        if let Err(err) = m.minify_mimetype(
                            &default_style_type.to_vec(),
                            w,
                            &mut rd,
                            default_style_params,
                        ) {
                            if !is_err_not_exist(&err) {
                                return Err(update_error_position(err, &z, t.offset));
                            }
                            let _ = w.write_go(&t.data);
                        }
                    } else {
                        let _ = w.write_go(&t.data);
                    }
                }
                CDATAToken => {
                    if tag == Style {
                        minify_buffer.reset();
                        let mut rd = tdewolff_parse::buffer::Reader::new(t.text.clone());
                        match m.minify_mimetype(
                            &default_style_type.to_vec(),
                            &mut minify_buffer,
                            &mut rd,
                            default_style_params,
                        ) {
                            Ok(()) => {
                                t.data = t.data.slice_to(9).append_bytes(&minify_buffer.bytes());
                                t.text = t.data.slice_from(9);
                                t.data = t.data.append(CDATA_END_BYTES);
                            }
                            Err(err) => {
                                if !is_err_not_exist(&err) {
                                    return Err(update_error_position(err, &z, t.offset));
                                }
                            }
                        }
                    }
                    let (text, use_text) = escape_cdata_val(&mut attr_byte_buffer, &t.text);
                    t.text = text;
                    if use_text {
                        t.text = replace_multiple_whitespace(t.text);
                        t.text = trim_whitespace(&t.text);
                        let _ = w.write_go(&t.text);
                    } else {
                        let _ = w.write_go(&t.data);
                    }
                }
                StartTagPIToken => loop {
                    let tt = tb.shift().token_type;
                    if tt == StartTagClosePIToken || tt == ErrorToken {
                        break;
                    }
                },
                StartTagToken => {
                    tag = t.hash;
                    if tag == Metadata {
                        t.data = GoBytes::nil();
                    } else {
                        let colon = t.text.index_byte(b':');
                        if colon != -1 {
                            let prefix = to_hash(&t.text.slice_to(colon as usize));
                            if prefix == Svg {
                                t.data = t.data.slice_to(1).append_bytes(&t.data.slice_from(5));
                            } else {
                                // skip attributes in namespace (eg. inkscape or sodipodi)
                                let mut keep = false;
                                for ns in namespaces {
                                    if prefix == ns {
                                        keep = true;
                                        break;
                                    }
                                }
                                if !keep {
                                    t.data = GoBytes::nil();
                                }
                            }
                        } else if tag == Defs && tb.peek(1).token_type == StartTagCloseVoidToken {
                            // skip empty tags
                            t.data = GoBytes::nil();
                        }
                    }

                    if t.data.is_nil() {
                        skip_tag(&mut tb);
                    } else {
                        let _ = w.write_go(&t.data);
                    }
                }
                AttributeToken => {
                    if t.text.is_nil() {
                        // data is nil when attribute has been removed
                        continue;
                    }

                    let attr = t.hash;
                    let mut val = t.attr_val.clone();
                    let (n, mm) = dimension(&val);
                    if n + mm == val.len() && attr != Version {
                        // TODO: inefficient, temporary measure
                        (val, _) = shorten_dimension(&o, val);
                    }
                    if attr == Xml_Space && val.equal(b"preserve")
                        || tag == Svg
                            && (o.inline && attr == Xmlns
                                || attr == Version && val.equal(b"1.1")
                                || attr == X && val.equal(ZERO_BYTES)
                                || attr == Y && val.equal(ZERO_BYTES)
                                || attr == PreserveAspectRatio && val.equal(b"xMidYMid meet")
                                || attr == BaseProfile && val.equal(NONE_BYTES)
                                || attr == ContentScriptType
                                    && val.equal(b"application/ecmascript")
                                || attr == ContentStyleType && val.equal(CSS_MIME_BYTES))
                        || tag == Style && attr == Type && val.equal(CSS_MIME_BYTES)
                    {
                        continue;
                    }

                    // skip attributes in namespace (eg. inkscape or sodipodi)
                    let colon = t.text.index_byte(b':');
                    if colon != -1 {
                        let colon = colon as usize;
                        let mut keep = false;
                        let prefix = to_hash(&t.text.slice_to(colon));
                        let name = to_hash(&t.text.slice_from(colon + 1));
                        for ns in namespaces {
                            if prefix == ns || tag == Svg && prefix == Xmlns && name == ns {
                                keep = true;
                                break;
                            }
                        }
                        if !keep {
                            continue;
                        }
                    }

                    let _ = w.write(SPACE_BYTES);
                    let _ = w.write_go(&t.text);
                    let _ = w.write(IS_BYTES);

                    if tag == Svg && attr == ContentStyleType {
                        val = mediatype(val);
                        default_style_type = val.clone();
                    } else if attr == Style {
                        minify_buffer.reset();
                        let mut rd = tdewolff_parse::buffer::Reader::new(val.clone());
                        match m.minify_mimetype(
                            &default_style_type.to_vec(),
                            &mut minify_buffer,
                            &mut rd,
                            Some(&default_inline_style_params),
                        ) {
                            Ok(()) => val = minify_buffer.bytes(),
                            Err(err) => {
                                if !is_err_not_exist(&err) {
                                    return Err(update_error_position(err, &z, t.offset));
                                }
                            }
                        }
                    } else if attr == D {
                        val = p.shorten_path_data(val);
                    } else if attr == ViewBox {
                        let mut j = 0usize;
                        let mut new_val = val.slice_to(0);
                        for i in 0..4 {
                            if i != 0 {
                                if j >= val.len() || val.at(j) != b' ' && val.at(j) != b',' {
                                    new_val = new_val.append_bytes(&val.slice_from(j));
                                    break;
                                }
                                new_val = new_val.append_byte(b' ');
                                j += 1;
                            }
                            let (dim, n) = shorten_dimension(&o, val.slice_from(j));
                            if n > 0 {
                                new_val = new_val.append_bytes(&dim);
                                j += n;
                            } else {
                                new_val = new_val.append_bytes(&val.slice_from(j));
                                break;
                            }
                        }
                        val = new_val;
                    } else if color_attr_map(attr)
                        && val.len() > 0
                        && (val.len() < 5 || !equal_fold(&val.slice_to(4), URL_BYTES))
                    {
                        //parse.ToLower(val)
                        if val.at(0) == b'#' {
                            if let Some(name) = crate::css::shorten_color_hex(&val.to_vec()) {
                                val = GoBytes::from_slice(name);
                            } else if val.len() == 7
                                && val.at(1) == val.at(2)
                                && val.at(3) == val.at(4)
                                && val.at(5) == val.at(6)
                            {
                                val.set(2, val.at(3));
                                val.set(3, val.at(5));
                                val = val.slice_to(4);
                            }
                        } else if let Some(hex) =
                            crate::css::shorten_color_name(crate::css::to_hash(&val))
                        {
                            val = GoBytes::from_slice(hex);
                            // } else if len(val) > 5 && bytes.Equal(val[:4], []byte("rgb(")) && val[len(val)-1] == ')' {
                            // TODO: handle rgb(x, y, z) and hsl(x, y, z)
                        }
                    }

                    // prefer single or double quotes depending on what occurs more often in value
                    val = escape_attr_val(&mut attr_byte_buffer, &val);
                    let _ = w.write_go(&val);
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

                    if tag == ForeignObject {
                        print_tag(w, &mut tb, tag);
                    }
                }
                StartTagCloseVoidToken => {
                    tag = Hash(0);
                    let _ = w.write_go(&t.data);
                }
                EndTagToken => {
                    tag = Hash(0);
                    if t.data.len() > 3 + t.text.len() {
                        t.data.set(2 + t.text.len(), b'>');
                        t.data = t.data.slice_to(3 + t.text.len());
                    }
                    let _ = w.write_go(&t.data);
                }
                _ => {}
            }
        }
    }
}

// Go: svg/svg.go:Minifier.shortenDimension
fn shorten_dimension(o: &Minifier, b: GoBytes) -> (GoBytes, usize) {
    let (n, m) = dimension(&b);
    if n > 0 {
        let mut unit = b.slice(n, n + m);
        let mut b = number(b.slice_to(n), o.precision);
        if b.len() != 1 || b.at(0) != b'0' {
            if m == 2 && unit.at(0) == b'p' && unit.at(1) == b'x' {
                unit = GoBytes::nil();
            } else if m > 1 {
                // only percentage is length 1
                to_lower(unit.clone());
            }
            b = b.append_bytes(&unit);
        }
        return (b, n + m);
    }
    (b, 0)
}

// Go: svg/svg.go:printTag
fn print_tag(w: &mut dyn Writer, tb: &mut TokenBuffer, tag: Hash) {
    let mut level = 0;
    let mut in_start_tag = false;
    loop {
        let t = tb.peek(0).clone();
        match t.token_type {
            ErrorToken => return,
            StartTagToken => {
                in_start_tag = t.hash == tag;
                if t.hash == tag {
                    level += 1;
                }
            }
            StartTagCloseVoidToken => {
                if in_start_tag {
                    if level == 0 {
                        return;
                    }
                    level -= 1;
                }
            }
            EndTagToken => {
                if t.hash == tag {
                    if level == 0 {
                        return;
                    }
                    level -= 1;
                }
            }
            _ => {}
        }
        let _ = w.write_go(&t.data);
        tb.shift();
    }
}

// Go: svg/svg.go:skipTag
fn skip_tag(tb: &mut TokenBuffer) {
    let mut level = 0;
    loop {
        let tt = tb.shift().token_type;
        if tt == ErrorToken {
            break;
        } else if tt == EndTagToken || tt == StartTagCloseVoidToken {
            if level == 0 {
                break;
            }
            level -= 1;
        } else if tt == StartTagToken {
            level += 1;
        }
    }
}
