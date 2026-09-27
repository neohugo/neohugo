//! Go: minify/html/html.go — minifies HTML5 following the specifications at
//! <http://www.w3.org/TR/html5/syntax.html>.

pub mod buffer;
mod entities;
pub mod hash;
pub mod table;

pub use buffer::{Token, TokenBuffer};
pub use hash::*;
pub use table::{Traits, attr_map, js_mimetypes, tag_map};

use table::{
    blockTag, booleanAttr, keepPTag, normalTag, objectTag, omitPTag, rawTag, trimAttr, urlAttr,
};

use tdewolff_parse::html::{
    AttributeToken, CommentToken, DoctypeToken, EndTagToken, ErrorToken, Lexer, MathToken,
    StartTagToken, SvgToken, TemplateToken, TextToken, escape_attr_val,
};
use tdewolff_parse::{
    EntityMap, GoBytes, GoError, GoReader, Input, NilMap, Params, RevEntityMap, SubView,
    equal_fold, is_all_whitespace, is_eof, is_whitespace, replace_entities,
    replace_multiple_whitespace_and_entities, to_lower, trim_whitespace,
};

use crate::common::{data_uri, mediatype, number, update_error_position};
use crate::{M, RestoreGuard, Writer, is_err_not_exist, params1};

static GT_BYTES: &[u8] = b">";
static IS_BYTES: &[u8] = b"=";
static SPACE_BYTES: &[u8] = b" ";
static DOCTYPE_BYTES: &[u8] = b"<!doctype html>";
static JS_MIME_BYTES: &[u8] = b"application/javascript";
static CSS_MIME_BYTES: &[u8] = b"text/css";
static HTML_MIME_BYTES: &[u8] = b"text/html";
static SVG_MIME_BYTES: &[u8] = b"image/svg+xml";
static FORM_MIME_BYTES: &[u8] = b"application/x-www-form-urlencoded";
static MATH_MIME_BYTES: &[u8] = b"application/mathml+xml";
static DATA_SCHEME_BYTES: &[u8] = b"data:";
static JS_SCHEME_BYTES: &[u8] = b"javascript:";
static HTTP_BYTES: &[u8] = b"http";
static RADIO_BYTES: &[u8] = b"radio";
static ON_BYTES: &[u8] = b"on";
static TEXT_BYTES: &[u8] = b"text";
static SUBMIT_BYTES: &[u8] = b"submit";
static ALL_BYTES: &[u8] = b"all";
static RECT_BYTES: &[u8] = b"rect";
static GET_BYTES: &[u8] = b"get";
static ONE_BYTES: &[u8] = b"one";

/// Go: html.GoTemplateDelims
pub const GoTemplateDelims: [&str; 2] = ["{{", "}}"];
/// Go: html.HandlebarsTemplateDelims
pub const HandlebarsTemplateDelims: [&str; 2] = ["{{", "}}"];
/// Go: html.MustacheTemplateDelims
pub const MustacheTemplateDelims: [&str; 2] = ["{{", "}}"];
/// Go: html.EJSTemplateDelims
pub const EJSTemplateDelims: [&str; 2] = ["<%", "%>"];
/// Go: html.ASPTemplateDelims
pub const ASPTemplateDelims: [&str; 2] = ["<%", "%>"];
/// Go: html.PHPTemplateDelims
pub const PHPTemplateDelims: [&str; 2] = ["<?", "?>"];

/// Go: html.EntitiesMap — all named character entities, mapped to their
/// shortest equivalent.
pub struct EntitiesMapT;

/// Go: html.EntitiesMap
pub static EntitiesMap: EntitiesMapT = EntitiesMapT;

impl EntitiesMapT {
    /// `EntitiesMap[name]`
    pub fn get(&self, name: &[u8]) -> Option<&'static [u8]> {
        entities::ENTITIES
            .binary_search_by(|(k, _)| (*k).cmp(name))
            .ok()
            .map(|i| entities::ENTITIES[i].1)
    }

    /// `len(EntitiesMap)`
    pub fn len(&self) -> usize {
        entities::ENTITIES.len()
    }

    /// `len(EntitiesMap) == 0`
    pub fn is_empty(&self) -> bool {
        entities::ENTITIES.is_empty()
    }

    /// All `(name, replacement)` pairs, sorted by name.
    pub fn entries(&self) -> &'static [(&'static [u8], &'static [u8])] {
        &entities::ENTITIES
    }
}

impl EntityMap for EntitiesMapT {
    fn lookup_entity(&self, name: &[u8]) -> Option<&[u8]> {
        self.get(name)
    }
}

/// Go: html.TextRevEntitiesMap — a map of escapes.
pub struct TextRevEntitiesMapT;

/// Go: html.TextRevEntitiesMap
pub static TextRevEntitiesMap: TextRevEntitiesMapT = TextRevEntitiesMapT;

impl RevEntityMap for TextRevEntitiesMapT {
    fn lookup_rev(&self, c: u8) -> Option<&[u8]> {
        match c {
            b'<' => Some(b"&lt;"),
            _ => Option::None,
        }
    }
}

/// Go: html.Minifier — an HTML minifier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Minifier {
    pub keep_comments: bool,
    /// Deprecated upstream: prints a warning to stdout and acts as
    /// `keep_special_comments`.
    pub keep_conditional_comments: bool,
    pub keep_special_comments: bool,
    pub keep_default_attr_vals: bool,
    pub keep_document_tags: bool,
    pub keep_end_tags: bool,
    pub keep_quotes: bool,
    pub keep_whitespace: bool,
    /// Go `[2]string`; `["", ""]` disables template detection.
    pub template_delims: [String; 2],
}

// Go: html/html.go:Minify
/// Minifies HTML data with the default options, it reads from r and writes
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
    fn minify(
        &self,
        m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        params: Option<&Params>,
    ) -> Result<(), GoError> {
        let mut o = self.clone();
        if o.keep_conditional_comments {
            println!("DEPRECATED: KeepConditionalComments is replaced by KeepSpecialComments");
            o.keep_special_comments = true;
            o.keep_conditional_comments = false; // omit next warning
        }
        o.minify_html(m, w, r, params)
    }
}

impl Minifier {
    // Go: html/html.go:Minifier.Minify
    /// Minifies HTML data, it reads from r and writes to w.
    fn minify_html(
        &self,
        m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        _params: Option<&Params>,
    ) -> Result<(), GoError> {
        let o = self;
        let mut raw_tag_hash = Hash(0);
        let mut raw_tag_mediatype = GoBytes::nil();

        let mut omit_space = true; // if true the next leading space is omitted
        let mut in_pre = false;

        let mut attr_minify_buffer = tdewolff_parse::buffer::Writer::new(GoBytes::make(0, 64));
        let mut attr_byte_buffer = GoBytes::make(0, 64);
        let inline_params = params1(b"inline", b"1");

        let z = Input::new(Some(r));
        let _restore = RestoreGuard(z.clone());

        let l = Lexer::new_template(
            z.clone(),
            [o.template_delims[0].as_str(), o.template_delims[1].as_str()],
        );
        let mut tb = TokenBuffer::new(z.clone(), l);
        loop {
            let mut t = tb.shift().clone();
            'sw: {
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
                    DoctypeToken => {
                        let _ = w.write(DOCTYPE_BYTES);
                    }
                    CommentToken => {
                        if o.keep_comments {
                            let _ = w.write_go(&t.data);
                        } else if o.keep_special_comments {
                            if 6 < t.text.len()
                                && (t.text.has_prefix(b"[if ")
                                    || t.text.has_suffix(b"[endif]")
                                    || t.text.has_suffix(b"[endif]--"))
                            {
                                // [if ...] is always 7 or more characters, [endif] is only encountered for downlevel-revealed
                                // see https://msdn.microsoft.com/en-us/library/ms537512(v=vs.85).aspx#syntax
                                if t.data.has_prefix(b"<!--[if ")
                                    && t.data.has_suffix(b"<![endif]-->")
                                {
                                    // downlevel-hidden
                                    let begin = (t.data.index_byte(b'>') + 1) as usize;
                                    let end = t.data.len() - b"<![endif]-->".len();
                                    if begin < end {
                                        let _ = w.write_go(&t.data.slice_to(begin));
                                        let mut rd = tdewolff_parse::buffer::Reader::new(
                                            t.data.slice(begin, end),
                                        );
                                        if let Err(err) = o.minify_html(m, w, &mut rd, Option::None)
                                        {
                                            return Err(update_error_position(err, &z, t.offset));
                                        }
                                        let _ = w.write_go(&t.data.slice_from(end));
                                    } else {
                                        let _ = w.write_go(&t.data); // malformed
                                    }
                                } else {
                                    let _ = w.write_go(&t.data); // downlevel-revealed or short downlevel-hidden
                                }
                            } else if 1 < t.text.len() && t.text.at(0) == b'#' {
                                // SSI tags
                                let _ = w.write_go(&t.data);
                            }
                        }
                    }
                    SvgToken => {
                        let mut rd = tdewolff_parse::buffer::Reader::new(t.data.clone());
                        if let Err(err) =
                            m.minify_mimetype(SVG_MIME_BYTES, w, &mut rd, Some(&inline_params))
                        {
                            if !is_err_not_exist(&err) {
                                return Err(update_error_position(err, &z, t.offset));
                            }
                            let _ = w.write_go(&t.data);
                        }
                        omit_space = false;
                    }
                    MathToken => {
                        let mut rd = tdewolff_parse::buffer::Reader::new(t.data.clone());
                        if let Err(err) =
                            m.minify_mimetype(MATH_MIME_BYTES, w, &mut rd, Option::None)
                        {
                            if !is_err_not_exist(&err) {
                                return Err(update_error_position(err, &z, t.offset));
                            }
                            let _ = w.write_go(&t.data);
                        }
                        omit_space = false;
                    }
                    TemplateToken => {
                        let _ = w.write_go(&t.data);
                        omit_space = false;
                    }
                    TextToken => {
                        if raw_tag_hash != Hash(0) && !t.has_template {
                            if raw_tag_hash == Style
                                || raw_tag_hash == Script
                                || raw_tag_hash == Iframe
                            {
                                let mut mimetype = GoBytes::nil();
                                let mut params: Option<Params> = Option::None;
                                if raw_tag_hash == Iframe {
                                    mimetype = GoBytes::from_static(HTML_MIME_BYTES);
                                } else if 0 < raw_tag_mediatype.len() {
                                    (mimetype, params) =
                                        tdewolff_parse::mediatype(&raw_tag_mediatype);
                                } else if raw_tag_hash == Script {
                                    mimetype = GoBytes::from_static(JS_MIME_BYTES);
                                } else if raw_tag_hash == Style {
                                    mimetype = GoBytes::from_static(CSS_MIME_BYTES);
                                }
                                let mut rd = tdewolff_parse::buffer::Reader::new(t.data.clone());
                                if let Err(err) = m.minify_mimetype(
                                    &mimetype.to_vec(),
                                    w,
                                    &mut rd,
                                    params.as_ref(),
                                ) {
                                    if !is_err_not_exist(&err) {
                                        return Err(update_error_position(err, &z, t.offset));
                                    }
                                    let _ = w.write_go(&t.data);
                                }
                            } else {
                                let _ = w.write_go(&t.data);
                            }
                        } else if in_pre {
                            let _ = w.write_go(&t.data);
                            // omitSpace = true after block element
                        } else {
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
                                    // trim if EOF, text token with leading whitespace or block token
                                    if next.token_type == ErrorToken {
                                        t.data = t.data.slice_to(t.data.len() - 1);
                                        omit_space = false;
                                        break;
                                    } else if next.token_type == TextToken
                                        && !is_all_whitespace(&next.data)
                                        || next.token_type == TemplateToken
                                    {
                                        // stop looking when text encountered
                                        break;
                                    } else if next.token_type == StartTagToken
                                        || next.token_type == EndTagToken
                                        || next.token_type == SvgToken
                                        || next.token_type == MathToken
                                    {
                                        if o.keep_whitespace {
                                            break;
                                        }
                                        // remove when followed by a block tag
                                        if next.traits & blockTag != 0 {
                                            t.data = t.data.slice_to(t.data.len() - 1);
                                            omit_space = false;
                                            break;
                                        } else if next.token_type == StartTagToken
                                            || next.token_type == SvgToken
                                            || next.token_type == MathToken
                                        {
                                            break;
                                        }
                                    }
                                    i += 1;
                                }
                            }
                            let _ = w.write_go(&t.data);
                        }
                    }
                    StartTagToken | EndTagToken => {
                        raw_tag_hash = Hash(0);
                        let mut has_attributes = false;
                        if t.token_type == StartTagToken {
                            if tb.peek(0).token_type == AttributeToken {
                                has_attributes = true;
                            }
                            if t.traits & rawTag != 0 {
                                // ignore empty script and style tags
                                if !has_attributes && (t.hash == Script || t.hash == Style) {
                                    if tb.peek(1).token_type == EndTagToken {
                                        tb.shift();
                                        tb.shift();
                                        break 'sw;
                                    }
                                }
                                raw_tag_hash = t.hash;
                                raw_tag_mediatype = GoBytes::nil();

                                // do not minify content of <style amp-boilerplate>
                                if has_attributes && t.hash == Style {
                                    let attrs = tb.attributes(&[Amp_Boilerplate]);
                                    if attrs[0].is_some() {
                                        raw_tag_hash = Hash(0);
                                    }
                                }
                            }
                        } else if t.hash == Template {
                            omit_space = true; // EndTagToken
                        }

                        if t.hash == Pre {
                            in_pre = t.token_type == StartTagToken;
                        }

                        // remove superfluous tags, except for html, head and body tags when KeepDocumentTags is set
                        if !has_attributes
                            && (!o.keep_document_tags
                                && (t.hash == Html || t.hash == Head || t.hash == Body)
                                || t.hash == Colgroup)
                        {
                            break 'sw;
                        } else if t.token_type == EndTagToken {
                            let mut omit_end_tag = false;
                            if !o.keep_end_tags {
                                if t.hash == Thead
                                    || t.hash == Tbody
                                    || t.hash == Tfoot
                                    || t.hash == Tr
                                    || t.hash == Th
                                    || t.hash == Td
                                    || t.hash == Option
                                    || t.hash == Dd
                                    || t.hash == Dt
                                    || t.hash == Li
                                    || t.hash == Rb
                                    || t.hash == Rt
                                    || t.hash == Rtc
                                    || t.hash == Rp
                                {
                                    omit_end_tag = true; // omit end tags
                                } else if t.hash == P {
                                    let mut i = 0;
                                    loop {
                                        let next = tb.peek(i);
                                        i += 1;
                                        // continue if text token is empty or whitespace
                                        if next.token_type == TextToken
                                            && is_all_whitespace(&next.data)
                                        {
                                            continue;
                                        }
                                        if next.token_type == ErrorToken
                                            || next.token_type == EndTagToken
                                                && next.traits & keepPTag == 0
                                            || next.token_type == StartTagToken
                                                && next.traits & omitPTag != 0
                                        {
                                            omit_end_tag = true; // omit p end tag
                                        }
                                        break;
                                    }
                                } else if t.hash == Optgroup {
                                    let mut i = 0;
                                    loop {
                                        let next = tb.peek(i);
                                        i += 1;
                                        // continue if text token
                                        if next.token_type == TextToken {
                                            continue;
                                        }
                                        if next.token_type == ErrorToken || next.hash != Option {
                                            omit_end_tag = true; // omit optgroup end tag
                                        }
                                        break;
                                    }
                                }
                            }

                            if !omit_end_tag {
                                if o.keep_whitespace || t.traits & objectTag != 0 {
                                    omit_space = false;
                                } else if t.traits & blockTag != 0 {
                                    omit_space = true; // omit spaces after block elements
                                }

                                if 3 + t.text.len() < t.data.len() {
                                    t.data.set(2 + t.text.len(), b'>');
                                    t.data = t.data.slice_to(3 + t.text.len());
                                }
                                let _ = w.write_go(&t.data);
                            }

                            // skip text in select and optgroup tags
                            if t.hash == Option || t.hash == Optgroup {
                                let next = tb.peek(0);
                                if next.token_type == TextToken && !next.has_template {
                                    tb.shift();
                                }
                            }
                            break 'sw;
                        }

                        if o.keep_whitespace || t.traits & objectTag != 0 {
                            omit_space = false;
                        } else if t.traits & blockTag != 0 {
                            omit_space = true; // omit spaces after block elements
                        }

                        let _ = w.write_go(&t.data);

                        if has_attributes {
                            if t.hash == Meta {
                                let attrs = tb.attributes(&[Content, Http_Equiv, Charset, Name]);
                                if let Some(content) = attrs[0] {
                                    if let Some(http_equiv) = attrs[1] {
                                        let v = trim_whitespace(&tb.token_mut(http_equiv).attr_val);
                                        tb.token_mut(http_equiv).attr_val = v.clone();
                                        if attrs[2].is_none() && equal_fold(&v, b"content-type") {
                                            let cv =
                                                mediatype(tb.token_mut(content).attr_val.clone());
                                            tb.token_mut(content).attr_val = cv.clone();
                                            if cv.equal(b"text/html;charset=utf-8") {
                                                tb.token_mut(http_equiv).text = GoBytes::nil();
                                                let c = tb.token_mut(content);
                                                c.text = GoBytes::from_slice(b"charset");
                                                c.hash = Charset;
                                                c.attr_val = GoBytes::from_slice(b"utf-8");
                                            }
                                        }
                                    }
                                    if let Some(name) = attrs[3] {
                                        let nv = trim_whitespace(&tb.token_mut(name).attr_val);
                                        tb.token_mut(name).attr_val = nv.clone();
                                        if equal_fold(&nv, b"keywords") {
                                            let c = tb.token_mut(content);
                                            c.attr_val =
                                                go_bytes_replace_all(&c.attr_val, b", ", b",");
                                        } else if equal_fold(&nv, b"viewport") {
                                            let c = tb.token_mut(content);
                                            c.attr_val =
                                                go_bytes_replace_all(&c.attr_val, b" ", b"");
                                            let mut i = 0usize;
                                            while i < c.attr_val.len() {
                                                if c.attr_val.at(i) == b'='
                                                    && i + 2 < c.attr_val.len()
                                                {
                                                    i += 1;
                                                    let n = tdewolff_parse::number(&SubView::new(
                                                        &c.attr_val,
                                                        i,
                                                    ));
                                                    if 0 < n {
                                                        let min_num =
                                                            number(c.attr_val.slice(i, i + n), -1);
                                                        if min_num.len() < n {
                                                            c.attr_val
                                                                .slice(i, i + min_num.len())
                                                                .copy_from(&min_num);
                                                            c.attr_val
                                                                .slice_from(i + min_num.len())
                                                                .copy_from(
                                                                    &c.attr_val.slice_from(i + n),
                                                                );
                                                            c.attr_val = c.attr_val.slice_to(
                                                                c.attr_val.len() + min_num.len()
                                                                    - n,
                                                            );
                                                        }
                                                        i += min_num.len();
                                                    }
                                                    i = i.wrapping_sub(1); // mitigate for-loop increase
                                                }
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                            } else if t.hash == Script {
                                let attrs = tb.attributes(&[Src, Charset]);
                                if attrs[0].is_some() && attrs[1].is_some() {
                                    tb.token_mut(attrs[1].unwrap()).text = GoBytes::nil();
                                }
                            } else if t.hash == Input {
                                let attrs = tb.attributes(&[Type, Value]);
                                if let (Some(ti), Some(vi)) = (attrs[0], attrs[1]) {
                                    let is_radio =
                                        equal_fold(&tb.token_mut(ti).attr_val, RADIO_BYTES);
                                    let value = tb.token_mut(vi);
                                    if !is_radio && value.attr_val.len() == 0 {
                                        value.text = GoBytes::nil();
                                    } else if is_radio && equal_fold(&value.attr_val, ON_BYTES) {
                                        value.text = GoBytes::nil();
                                    }
                                }
                            } else if t.hash == A {
                                let attrs = tb.attributes(&[Id, Name]);
                                if let (Some(id), Some(name)) = (attrs[0], attrs[1]) {
                                    let idv = tb.token_mut(id).attr_val.clone();
                                    let nm = tb.token_mut(name);
                                    if idv == nm.attr_val {
                                        nm.text = GoBytes::nil();
                                    }
                                }
                            }

                            // write attributes
                            loop {
                                let attr = tb.shift().clone();
                                if attr.token_type != AttributeToken {
                                    break;
                                } else if attr.text.is_nil() {
                                    continue; // removed attribute
                                } else if attr.has_template {
                                    let _ = w.write_go(&attr.data);
                                    continue; // don't minify attributes that contain templates
                                }

                                let mut val = attr.attr_val.clone();
                                if attr.traits & trimAttr != 0 {
                                    val = replace_multiple_whitespace_and_entities(
                                        val,
                                        &EntitiesMap,
                                        &NilMap,
                                    );
                                    val = trim_whitespace(&val);
                                } else {
                                    val = replace_entities(val, &EntitiesMap, &NilMap);
                                }
                                if t.traits != 0 {
                                    if val.len() == 0
                                        && (attr.hash == Class
                                            || attr.hash == Dir
                                            || attr.hash == Id
                                            || attr.hash == Name
                                            || attr.hash == Action && t.hash == Form)
                                    {
                                        continue; // omit empty attribute values
                                    }
                                    if raw_tag_hash != Hash(0) && attr.hash == Type {
                                        raw_tag_mediatype = tdewolff_parse::copy(&val);
                                    }

                                    if attr.hash == Enctype
                                        || attr.hash == Formenctype
                                        || attr.hash == Accept
                                        || attr.hash == Type
                                            && (t.hash == A
                                                || t.hash == Link
                                                || t.hash == Embed
                                                || t.hash == Object
                                                || t.hash == Source
                                                || t.hash == Script)
                                    {
                                        val = mediatype(val);
                                    }

                                    // default attribute values can be omitted
                                    if !o.keep_default_attr_vals
                                        && (attr.hash == Type
                                            && (t.hash == Script
                                                && js_mimetypes(
                                                    &to_lower(tdewolff_parse::copy(&val)).to_vec(),
                                                )
                                                || t.hash == Style
                                                    && equal_fold(&val, CSS_MIME_BYTES)
                                                || t.hash == Link
                                                    && equal_fold(&val, CSS_MIME_BYTES)
                                                || t.hash == Input && equal_fold(&val, TEXT_BYTES)
                                                || t.hash == Button
                                                    && equal_fold(&val, SUBMIT_BYTES))
                                            || attr.hash == Method && equal_fold(&val, GET_BYTES)
                                            || attr.hash == Enctype
                                                && equal_fold(&val, FORM_MIME_BYTES)
                                            || attr.hash == Colspan && val.equal(ONE_BYTES)
                                            || attr.hash == Rowspan && val.equal(ONE_BYTES)
                                            || attr.hash == Shape && equal_fold(&val, RECT_BYTES)
                                            || attr.hash == Span && val.equal(ONE_BYTES)
                                            || attr.hash == Media
                                                && t.hash == Style
                                                && equal_fold(&val, ALL_BYTES))
                                    {
                                        continue;
                                    }

                                    if attr.hash == Style {
                                        // CSS minifier for attribute inline code
                                        val = trim_whitespace(&val);
                                        attr_minify_buffer.reset();
                                        let mut rd =
                                            tdewolff_parse::buffer::Reader::new(val.clone());
                                        match m.minify_mimetype(
                                            CSS_MIME_BYTES,
                                            &mut attr_minify_buffer,
                                            &mut rd,
                                            Some(&inline_params),
                                        ) {
                                            Ok(()) => val = attr_minify_buffer.bytes(),
                                            Err(err) => {
                                                if !is_err_not_exist(&err) {
                                                    return Err(update_error_position(
                                                        err,
                                                        &z,
                                                        attr.offset,
                                                    ));
                                                }
                                            }
                                        }
                                        if val.len() == 0 {
                                            continue;
                                        }
                                    } else if 2 < attr.text.len()
                                        && attr.text.at(0) == b'o'
                                        && attr.text.at(1) == b'n'
                                    {
                                        // JS minifier for attribute inline code
                                        val = trim_whitespace(&val);
                                        if 11 <= val.len()
                                            && equal_fold(&val.slice_to(11), JS_SCHEME_BYTES)
                                        {
                                            val = val.slice_from(11);
                                        }
                                        attr_minify_buffer.reset();
                                        let mut rd =
                                            tdewolff_parse::buffer::Reader::new(val.clone());
                                        match m.minify_mimetype(
                                            JS_MIME_BYTES,
                                            &mut attr_minify_buffer,
                                            &mut rd,
                                            Some(&inline_params),
                                        ) {
                                            Ok(()) => val = attr_minify_buffer.bytes(),
                                            Err(err) => {
                                                if !is_err_not_exist(&err) {
                                                    return Err(update_error_position(
                                                        err,
                                                        &z,
                                                        attr.offset,
                                                    ));
                                                }
                                            }
                                        }
                                        if val.len() == 0 {
                                            continue;
                                        }
                                    } else if attr.traits & urlAttr != 0 {
                                        // anchors are already handled
                                        val = trim_whitespace(&val);
                                        if 5 < val.len() {
                                            if equal_fold(&val.slice_to(4), HTTP_BYTES) {
                                                if val.at(4) == b':' {
                                                    if m.url
                                                        .as_ref()
                                                        .is_some_and(|u| u.scheme == "http")
                                                    {
                                                        val = val.slice_from(5);
                                                    } else {
                                                        to_lower(val.slice_to(4));
                                                    }
                                                } else if (val.at(4) == b's' || val.at(4) == b'S')
                                                    && val.at(5) == b':'
                                                {
                                                    if m.url
                                                        .as_ref()
                                                        .is_some_and(|u| u.scheme == "https")
                                                    {
                                                        val = val.slice_from(6);
                                                    } else {
                                                        to_lower(val.slice_to(5));
                                                    }
                                                }
                                            } else if equal_fold(
                                                &val.slice_to(5),
                                                DATA_SCHEME_BYTES,
                                            ) {
                                                val = data_uri(m, val);
                                            }
                                        }
                                    }
                                }

                                let _ = w.write(SPACE_BYTES);
                                let _ = w.write_go(&attr.text);
                                if 0 < val.len() && attr.traits & booleanAttr == 0 {
                                    let _ = w.write(IS_BYTES);

                                    // use double quotes for RDFa attributes
                                    let is_xml = attr.hash == Vocab
                                        || attr.hash == Typeof
                                        || attr.hash == Property
                                        || attr.hash == Resource
                                        || attr.hash == Prefix
                                        || attr.hash == Content
                                        || attr.hash == About
                                        || attr.hash == Rev
                                        || attr.hash == Datatype
                                        || attr.hash == Inlist;

                                    // no quotes if possible, else prefer single or double depending on which occurs more often in value
                                    let mut quote = 0u8;

                                    let n = attr.data.len();
                                    if 0 < n
                                        && (attr.data.at(n - 1) == b'\''
                                            || attr.data.at(n - 1) == b'"')
                                    {
                                        quote = attr.data.at(n - 1);
                                    }
                                    val = escape_attr_val(
                                        &mut attr_byte_buffer,
                                        &val,
                                        quote,
                                        o.keep_quotes || is_xml,
                                    );
                                    let _ = w.write_go(&val);
                                }
                            }
                        } else {
                            let _ = tb.shift(); // StartTagClose
                        }
                        let _ = w.write(GT_BYTES);

                        // skip text in select and optgroup tags
                        if t.hash == Select || t.hash == Optgroup {
                            let next = tb.peek(0);
                            if next.token_type == TextToken && !next.has_template {
                                tb.shift();
                            }
                        }

                        // keep space after phrasing tags (<i>, <span>, ...) FontAwesome etc.
                        if t.token_type == StartTagToken && t.traits == normalTag {
                            let next = tb.peek(0);
                            if next.hash == t.hash && next.token_type == EndTagToken {
                                omit_space = false;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Go `bytes.ReplaceAll(s, old, new)`: always returns a fresh copy
/// (`append([]byte(nil), s...)` when there is nothing to replace).
fn go_bytes_replace_all(s: &GoBytes, old: &[u8], new: &[u8]) -> GoBytes {
    let v = s.to_vec();
    // Compute number of replacements.
    let mut m = 0usize;
    if !old.is_empty() {
        let mut k = 0;
        while k + old.len() <= v.len() {
            if &v[k..k + old.len()] == old {
                m += 1;
                k += old.len();
            } else {
                k += 1;
            }
        }
    }
    if m == 0 {
        // Just return a copy.
        return GoBytes::nil().append(&v);
    }
    // Apply replacements to buffer.
    let n = v.len() + m * new.len() - m * old.len();
    let mut t = Vec::with_capacity(n);
    let mut start = 0;
    let mut k = 0;
    let mut done = 0;
    while done < m {
        if &v[k..k + old.len()] == old {
            t.extend_from_slice(&v[start..k]);
            t.extend_from_slice(new);
            k += old.len();
            start = k;
            done += 1;
        } else {
            k += 1;
        }
    }
    t.extend_from_slice(&v[start..]);
    GoBytes::from_slice_cap(&t, n)
}
