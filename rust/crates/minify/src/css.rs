//! Stand-alone CSS through lightningcss.

use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::targets::{Features, Targets};

use crate::MinifyError;
use crate::options::CssOptions;

/// lightningcss can reorder `!important` declarations it keeps unparsed differently when it
/// minifies its own output; passes repeat (at most three) until the output is stable.
pub(crate) fn minify(o: &CssOptions, input: &str) -> Result<String, MinifyError> {
    let mut out = pass(o, input)?;
    for _ in 0..2 {
        let again = pass(o, &out)?;
        if again == out {
            break;
        }
        out = again;
    }
    Ok(out)
}

fn pass(o: &CssOptions, input: &str) -> Result<String, MinifyError> {
    let targets = Targets {
        browsers: None,
        include: if o.keep_css2 {
            Features::HexAlphaColors | Features::SpaceSeparatedColorNotation
        } else {
            Features::empty()
        },
        exclude: Features::empty(),
    };
    let err = |e: &dyn std::fmt::Display| MinifyError::Css(e.to_string());
    let mut sheet = StyleSheet::parse(input, ParserOptions::default()).map_err(|e| err(&e))?;
    sheet
        .minify(MinifyOptions {
            targets,
            ..MinifyOptions::default()
        })
        .map_err(|e| err(&e))?;
    let printed = sheet
        .to_css(PrinterOptions {
            minify: true,
            targets,
            ..PrinterOptions::default()
        })
        .map_err(|e| err(&e))?;
    Ok(printed.code)
}
