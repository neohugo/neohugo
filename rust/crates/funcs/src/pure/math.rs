//! `to_math` (feature `math`): LaTeX to MathML Core with pulldown-latex.

use std::borrow::Cow;
use std::sync::Arc;

use neohugo_base::diag::Diagnostic;
use pulldown_latex::config::DisplayMode;
use pulldown_latex::{Parser, RenderConfig, Storage, push_mathml};
use tera::Value;

use super::value::text;
use super::{PureEnv, Registrar};

/// `to_math(display=?, optional=?)`. A construct pulldown-latex cannot parse (invalid LaTeX,
/// or a KaTeX extension such as mhchem's `\ce`) is an error, like Go's `transform.ToMath`;
/// with `optional=true` (Go's `try`) it is a warning instead (id `to_math`, as `get_remote`),
/// and the formula renders with that construct as an in-place `<merror>`.
pub(super) fn register(r: &mut Registrar<'_>, env: &Arc<PureEnv>) {
    let env = Arc::clone(env);
    r.filter("to_math", move |v, kw, _| {
        let display = kw.get::<bool>("display")?.unwrap_or(false);
        let optional = kw.get::<bool>("optional")?.unwrap_or(false);
        let tex = text(&v, "to_math")?;
        let (mathml, errors) = to_math(&tex, display)?;
        if !errors.is_empty() {
            let message = format!("to_math: `{}`: {}", tex.trim(), errors.join("; "));
            if !optional {
                return Err(tera::Error::message(message));
            }
            env.diagnostics
                .push(Diagnostic::warning(message).with_id("to_math"));
        }
        Ok(Value::safe_string(&mathml))
    });
}

/// The MathML of `tex` (a `<math>` element, `display="block"` when `display`) and the parse
/// errors, each rendered in place as `<merror>`.
///
/// # Errors
/// Only an I/O failure of the renderer (writing to a string does not fail).
pub(super) fn to_math(tex: &str, display: bool) -> tera::TeraResult<(String, Vec<String>)> {
    let tex = rewrite_over(tex);
    let storage = Storage::new();
    let mut errors = Vec::new();
    let parser = Parser::new(&tex, &storage).inspect(|e| {
        if let Err(e) = e {
            // the kind only: `Display` appends a multi-line excerpt of the source
            let kind =
                std::error::Error::source(e).map_or_else(|| e.to_string(), ToString::to_string);
            errors.push(kind);
        }
    });
    let config = RenderConfig {
        display_mode: if display {
            DisplayMode::Block
        } else {
            DisplayMode::Inline
        },
        ..RenderConfig::default()
    };
    let mut out = String::new();
    push_mathml(&mut out, parser, config).map_err(|e| tera::Error::chain("to_math", e))?;
    // pulldown-latex 0.8 escapes the `&nbsp;` it writes for `~` and `\ `
    Ok((out.replace("&amp;nbsp;", "\u{a0}"), errors))
}

/// TeX's infix fraction: `{a \over b}` becomes `{\frac{a}{b}}` (in every group, and at the top
/// level), which pulldown-latex understands.
fn rewrite_over(tex: &str) -> Cow<'_, str> {
    if !tex.contains("\\over") {
        return Cow::Borrowed(tex);
    }
    let (out, _) = group(tex, 0);
    Cow::Owned(out)
}

/// Rewrites the group starting at byte `start` up to its closing `}` (or the end); returns the
/// rewritten content and the position after it.
fn group(tex: &str, start: usize) -> (String, usize) {
    let bytes = tex.as_bytes();
    let mut out = String::new();
    let mut split: Option<usize> = None;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                let rest = &tex[i + 1..];
                let word_len = rest
                    .find(|c: char| !c.is_ascii_alphabetic())
                    .unwrap_or(rest.len());
                if split.is_none() && &rest[..word_len] == "over" {
                    split = Some(out.len());
                    i += 1 + word_len;
                    continue;
                }
                // a command, or an escaped character
                let len = if word_len > 0 {
                    word_len
                } else {
                    rest.chars().next().map_or(0, char::len_utf8)
                };
                out.push_str(&tex[i..=i + len]);
                i += 1 + len;
            }
            b'{' => {
                let (inner, next) = group(tex, i + 1);
                out.push('{');
                out.push_str(&inner);
                out.push('}');
                i = next;
            }
            b'}' => {
                i += 1;
                break;
            }
            _ => {
                let c = tex[i..].chars().next().unwrap_or_default();
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    let out = match split {
        Some(at) => format!("\\frac{{{}}}{{{}}}", out[..at].trim(), out[at..].trim()),
        None => out,
    };
    (out, i)
}
