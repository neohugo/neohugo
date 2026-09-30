//! HTML through minify-html (inline CSS through lightningcss, inline JS through oxc).

use crate::MinifyError;
use crate::options::{HtmlComments, HtmlOptions, TemplateSyntax};

pub(crate) fn minify(
    o: &HtmlOptions,
    css: bool,
    js: bool,
    input: &str,
) -> Result<String, MinifyError> {
    let mut cfg = minify_html::Cfg {
        keep_comments: o.comments == HtmlComments::KeepAll,
        keep_ssi_comments: o.comments != HtmlComments::Remove,
        keep_closing_tags: o.keep_end_tags,
        keep_html_and_head_opening_tags: o.keep_document_tags,
        keep_input_type_text_attr: o.keep_default_attr_vals,
        preserve_brace_template_syntax: o.templates == TemplateSyntax::Braces,
        preserve_chevron_percent_template_syntax: o.templates == TemplateSyntax::ChevronPercent,
        minify_css: css,
        minify_js: js,
        // Spec-compliant output only: no `<!doctypehtml>`, no unquoted values with `"'=<>` or
        // backticks, no attributes run together.
        ..minify_html::Cfg::default()
    };
    let mut out = minify_html::minify(input.as_bytes(), &cfg);
    // A second pass (inline CSS and JS are done) makes the result idempotent where minify-html
    // decides on the tokens it has not yet removed: it collapses whitespace before it drops
    // comments (`a <!-- c --> b` leaves two spaces), and it omits an end tag by the next
    // sibling as written (`</tfoot>` before an omitted `</tbody>`).
    let dropped_comments = o.comments != HtmlComments::KeepAll && input.contains("<!--");
    if dropped_comments || !o.keep_end_tags {
        cfg.minify_css = false;
        cfg.minify_js = false;
        out = minify_html::minify(&out, &cfg);
    }
    String::from_utf8(out).map_err(|_| MinifyError::HtmlEncoding)
}
