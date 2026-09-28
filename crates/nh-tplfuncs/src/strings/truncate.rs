//! Port of `tpl/strings/truncate.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::LazyLock;

use go_unicode::utf8;
use go_value::{HostCtx, SafeKind, Value};
use nh_common::cast::caste;
use nh_common::goregexp::Regexp;
use nh_common::object::{GoResult, args};

use super::strings::{Namespace, gerr};

static TAG_RE: LazyLock<Regexp> =
    LazyLock::new(|| Regexp::must_compile(r"^<(/)?([^ ]+?)(?:(\s*/)| .*?)?>"));

// Go: tpl/strings/truncate.go:htmlSinglets
fn html_singlet(name: &[u8]) -> bool {
    matches!(
        name,
        b"br" | b"col" | b"link" | b"base" | b"img" | b"param" | b"area" | b"hr" | b"input"
    )
}

/// Go: `htmlTag`.
#[derive(Clone)]
struct HtmlTag {
    name: Vec<u8>,
    pos: usize,
    open_tag: bool,
}

fn html_escape(b: &[u8]) -> Vec<u8> {
    go_html::escape_string_bytes(b)
}

impl Namespace {
    /// Truncate truncates the string in s to the specified length.
    // Go: tpl/strings/truncate.go:Truncate
    pub fn truncate(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Truncate")?;
        let length = caste::to_int_e(&a[0])?;
        let options = &a[1..];
        let text_param: &Value;
        let ellipsis: Vec<u8>;

        match options.len() {
            0 => return Err(gerr("truncate requires a length and a string")),
            1 => {
                text_param = &options[0];
                ellipsis = " …".as_bytes().to_vec();
            }
            2 => {
                text_param = &options[1];
                let e = caste::to_string_e(&options[0])
                    .map_err(|_| gerr("ellipsis must be a string"))?;
                ellipsis = match &options[0] {
                    Value::Safe(SafeKind::Html, _) => e.to_vec(),
                    _ => html_escape(e.as_bytes()),
                };
            }
            _ => return Err(gerr("too many arguments passed to truncate")),
        }
        let text = caste::to_string_e(text_param).map_err(|_| gerr("text must be a string"))?;
        let text = text.as_bytes();

        let is_html = matches!(text_param, Value::Safe(SafeKind::Html, _));

        if (utf8::rune_count_in_string(text) as i64) <= length {
            if is_html {
                return Ok(Value::html(text.to_vec()));
            }
            return Ok(Value::html(html_escape(text)));
        }

        let mut tags: Vec<HtmlTag> = Vec::new();
        let (mut last_word_index, mut last_non_space, mut current_len, mut next_tag) =
            (0usize, 0usize, 0i64, 0usize);

        let mut i = 0usize;
        while i < text.len() {
            let (r, size) = utf8::decode_rune(&text[i..]);
            let at = i;
            i += size;
            if at < next_tag {
                continue;
            }

            if is_html {
                // Make sure we keep tagname of HTML tags
                let slice = &text[at..];
                if let Some(m) = TAG_RE.find_submatch_index(slice)
                    && m[0] == 0
                {
                    next_tag = at + m[1] as usize;
                    let fields = go_unicode::strings::fields(&slice[m[4] as usize..m[5] as usize]);
                    let Some(first) = fields.first() else {
                        // Go: `strings.Fields(...)[0]` panics on an all-space tag name.
                        return Err(gerr("runtime error: index out of range [0] with length 0"));
                    };
                    let tagname = first.to_vec();
                    last_word_index = last_non_space;
                    let singlet = html_singlet(&tagname);
                    if !singlet && m[6] == -1 {
                        tags.push(HtmlTag {
                            name: tagname,
                            pos: at,
                            open_tag: m[2] == -1,
                        });
                    }

                    continue;
                }
            }

            current_len += 1;
            if go_unicode::is_space(r) {
                last_word_index = last_non_space;
            } else if go_unicode::r#in(
                r,
                &[
                    go_unicode::tables::HAN,
                    go_unicode::tables::HANGUL,
                    go_unicode::tables::HIRAGANA,
                    go_unicode::tables::KATAKANA,
                ],
            ) {
                last_word_index = at;
            } else {
                // Go: `i + utf8.RuneLen(r)`; RuneLen(RuneError) is 3 even for an invalid byte.
                last_non_space = at + utf8::rune_len(r) as usize;
            }

            if current_len > length {
                let end_text_pos = if last_word_index == 0 {
                    at
                } else {
                    last_word_index
                };
                // Go's `text[0:endTextPos]` panics when a rune length of 3 for an invalid
                // byte pushed the index past the end.
                if end_text_pos > text.len() {
                    return Err(gerr(format!(
                        "runtime error: slice bounds out of range [:{end_text_pos}] with length {}",
                        text.len()
                    )));
                }
                let mut out = text[0..end_text_pos].to_vec();
                if is_html {
                    out.extend_from_slice(&ellipsis);
                    // Close out any open HTML tags
                    let mut current_tag: Option<HtmlTag> = None;
                    for tag in tags.iter().rev() {
                        if tag.pos >= end_text_pos || current_tag.is_some() {
                            if current_tag.as_ref().is_some_and(|c| c.name == tag.name) {
                                current_tag = None;
                            }
                            continue;
                        }

                        if tag.open_tag {
                            out.extend_from_slice(b"</");
                            out.extend_from_slice(&tag.name);
                            out.push(b'>');
                        } else {
                            current_tag = Some(tag.clone());
                        }
                    }

                    return Ok(Value::html(out));
                }
                let mut out = html_escape(&out);
                out.extend_from_slice(&ellipsis);
                return Ok(Value::html(out));
            }
        }

        if is_html {
            return Ok(Value::html(text.to_vec()));
        }
        Ok(Value::html(html_escape(text)))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/strings/truncate.go (158 lines; 0/1 funcs executed)
//   types: htmlTag
// OK L44-158: (ns *Namespace) Truncate(s any, options ...any) (template.HTML, error)
// ---------------------------------------------------------------------------
