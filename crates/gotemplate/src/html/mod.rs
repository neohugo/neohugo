//! Go: tpl/internal/go_templates/htmltemplate — contextual autoescaping
//! (go1.24.0 fork + Hugo's `hugo_template.go`).
//!
//! | Go file | Rust module |
//! |---|---|
//! | `attr.go` | `attr` |
//! | `content.go`, `hugo_template.go:indirect`, `escape.go:evalArgs` | `content` |
//! | `context.go` + the `*_string.go` stringers | `context` |
//! | `css.go` | `css` |
//! | `error.go` | `error` |
//! | `html.go` | `html` |
//! | `js.go` (+ `escape.go` `specialScriptTagRE`) | `js` |
//! | `transition.go` (+ `escape.go` `delimEnds`) | `transition` |
//! | `url.go` | `url` |
//! | `escape.go` (the escaper) | `escape` |
//! | `template.go` + `hugo_template.go` (`Prepare`, `All`, `CloneShallow`) | `template` |
//! | `escape.go:funcMap`, `escape.go:filterFailsafe`, `hugo_template.go:StripTags`, `GoFuncs` | this module |

pub(crate) mod attr;
pub(crate) mod content;
pub(crate) mod context;
pub(crate) mod css;
pub mod error;
mod escape;
#[allow(clippy::module_inception)]
pub(crate) mod html;
pub(crate) mod js;
mod template;
pub(crate) mod transition;
pub(crate) mod url;

#[cfg(test)]
mod tests;

use go_value::Value;

pub use error::{ErrNode, Error, ErrorCode};
pub use template::{ErrorBox, Template};

// Go: escape.go:filterFailsafe
/// filterFailsafe is an innocuous word that is emitted in place of unsafe
/// values by sanitizer functions. It is not a keyword in any programming
/// language, contains no special characters, is not empty, and when it
/// appears in output it is distinct enough that a developer can find the
/// source of the problem via a search engine.
pub(crate) const FILTER_FAILSAFE: &str = "ZgotmplZ";

/// An escaper function (Go: `func(args ...any) string`).
pub(crate) type EscFunc = fn(&[Value]) -> Vec<u8>;

// Go: escape.go:funcMap (key order as in the Go source)
/// The names of the escaper functions the escaper inserts into pipelines
/// (Hugo exports the map as `htmltemplate.GoFuncs`).
pub(crate) const ESC_FUNC_NAMES: &[&str] = &[
    "_html_template_attrescaper",
    "_html_template_commentescaper",
    "_html_template_cssescaper",
    "_html_template_cssvaluefilter",
    "_html_template_htmlnamefilter",
    "_html_template_htmlescaper",
    "_html_template_jsregexpescaper",
    "_html_template_jsstrescaper",
    "_html_template_jstmpllitescaper",
    "_html_template_jsvalescaper",
    "_html_template_nospaceescaper",
    "_html_template_rcdataescaper",
    "_html_template_srcsetescaper",
    "_html_template_urlescaper",
    "_html_template_urlfilter",
    "_html_template_urlnormalizer",
    "_eval_args_",
];

// Go: escape.go:funcMap
/// funcMap maps command names to functions that render their inputs safe.
pub(crate) fn esc_func(name: &str) -> Option<EscFunc> {
    Some(match name {
        "_html_template_attrescaper" => html::attr_escaper,
        "_html_template_commentescaper" => html::comment_escaper,
        "_html_template_cssescaper" => css::css_escaper,
        "_html_template_cssvaluefilter" => css::css_value_filter,
        "_html_template_htmlnamefilter" => html::html_name_filter,
        "_html_template_htmlescaper" => html::html_escaper,
        "_html_template_jsregexpescaper" => js::js_regexp_escaper,
        "_html_template_jsstrescaper" => js::js_str_escaper,
        "_html_template_jstmpllitescaper" => js::js_tmpl_lit_escaper,
        "_html_template_jsvalescaper" => js::js_val_escaper,
        "_html_template_nospaceescaper" => html::html_nospace_escaper,
        "_html_template_rcdataescaper" => html::rcdata_escaper,
        "_html_template_srcsetescaper" => url::srcset_filter_and_escaper,
        "_html_template_urlescaper" => url::url_escaper,
        "_html_template_urlfilter" => url::url_filter,
        "_html_template_urlnormalizer" => url::url_normalizer,
        "_eval_args_" => content::eval_args,
        _ => return None,
    })
}

// Go: htmltemplate/hugo_template.go:StripTags
/// Hugo's exported `stripTags` (see <https://github.com/golang/go/issues/5884>):
/// the text content of an HTML snippet, without entity decoding.
pub fn strip_tags(html: &[u8]) -> Vec<u8> {
    html::strip_tags(html)
}

// Go: htmltemplate/hugo_template.go:GoFuncs
/// The escaper functions as template functions (Go `htmltemplate.GoFuncs`,
/// which Hugo merges into its function lookup), in Go's `funcMap` order.
pub fn go_funcs() -> Vec<(&'static str, crate::text::Func)> {
    ESC_FUNC_NAMES
        .iter()
        .map(|&name| {
            let f = esc_func(name).expect("escaper func");
            let func: crate::text::Func =
                std::sync::Arc::new(move |_ctx: go_value::HostCtx<'_>, args: &[Value]| {
                    Ok(Value::string(f(args)))
                });
            (name, func)
        })
        .collect()
}
