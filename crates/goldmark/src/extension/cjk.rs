// Go: github.com/yuin/goldmark@v1.7.12/extension/cjk.go

use crate::parser;
use crate::renderer::html;
use crate::{Extender, Markdown};

/// A EastAsianLineBreaks is a style of east asian line breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EastAsianLineBreaks {
    /// EastAsianLineBreaksNone renders line breaks as it is.
    None = 0,
    /// EastAsianLineBreaksSimple is a style where soft line breaks are ignored
    /// if both sides of the break are east asian wide characters.
    Simple = 1,
    /// EastAsianLineBreaksCSS3Draft is a style where soft line breaks are ignored
    /// even if only one side of the break is an east asian wide character.
    Css3Draft = 2,
}

impl EastAsianLineBreaks {
    /// Go: `html.EastAsianLineBreaks(e.EastAsianLineBreaks)` (same values).
    fn to_html(self) -> html::EastAsianLineBreaks {
        match self {
            EastAsianLineBreaks::None => html::EastAsianLineBreaks::None,
            EastAsianLineBreaks::Simple => html::EastAsianLineBreaks::Simple,
            EastAsianLineBreaks::Css3Draft => html::EastAsianLineBreaks::Css3Draft,
        }
    }
}

/// A CJKOption sets options for CJK support mostly for HTML based renderers.
pub type CJKOption = Box<dyn Fn(&mut CjkExt) + Send + Sync>;

// Go: extension/cjk.go:WithEastAsianLineBreaks
/// WithEastAsianLineBreaks is a functional option that indicates whether softline breaks
/// between east asian wide characters should be ignored.
/// style defauts to [EastAsianLineBreaks::Simple] (pass an empty slice).
pub fn with_east_asian_line_breaks(style: &[EastAsianLineBreaks]) -> CJKOption {
    let style = style.to_vec();
    Box::new(move |c: &mut CjkExt| {
        if style.is_empty() {
            c.east_asian_line_breaks = EastAsianLineBreaks::Simple;
            return;
        }
        c.east_asian_line_breaks = style[0];
    })
}

// Go: extension/cjk.go:WithEscapedSpace
/// WithEscapedSpace is a functional option that indicates that a '\' escaped half-space(0x20) should not be rendered.
pub fn with_escaped_space() -> CJKOption {
    Box::new(|c: &mut CjkExt| c.escaped_space = true)
}

/// Go: `type cjk struct`.
pub struct CjkExt {
    pub east_asian_line_breaks: EastAsianLineBreaks,
    pub escaped_space: bool,
}

// Go: extension/cjk.go:CJK
/// CJK is a goldmark extension that provides functionalities for CJK languages.
pub fn cjk() -> Box<dyn Extender> {
    new_cjk(vec![with_east_asian_line_breaks(&[]), with_escaped_space()])
}

// Go: extension/cjk.go:NewCJK
/// NewCJK returns a new extension with given options.
pub fn new_cjk(opts: Vec<CJKOption>) -> Box<dyn Extender> {
    let mut e = CjkExt {
        east_asian_line_breaks: EastAsianLineBreaks::None,
        escaped_space: false,
    };
    for opt in opts {
        opt(&mut e);
    }
    Box::new(e)
}

impl Extender for CjkExt {
    // Go: extension/cjk.go:cjk.Extend
    fn extend(&self, m: &mut Markdown) {
        m.renderer()
            .add_options(vec![Box::new(html::with_east_asian_line_breaks(
                self.east_asian_line_breaks.to_html(),
            ))]);
        if self.escaped_space {
            m.renderer()
                .add_options(vec![Box::new(html::with_writer(html::new_writer(vec![
                    html::with_escaped_space(),
                ])))]);
            m.parser().add_options(vec![parser::with_escaped_space()]);
        }
    }
}
