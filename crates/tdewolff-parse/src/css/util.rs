//! Go: parse/css/util.go

use crate::css::lex::Lexer;
use crate::gobytes::GoBytes;
use crate::input::Input;

// Go: parse/css/util.go:IsIdent
/// Returns true if the bytes are a valid identifier.
pub fn is_ident(b: &GoBytes) -> bool {
    let mut l = Lexer::new(Input::new_bytes(b.clone()));
    l.consume_ident_token();
    l.r.restore();
    l.r.pos() == b.len()
}

// Go: parse/css/util.go:IsURLUnquoted
/// Returns true if the bytes are a valid unquoted URL.
pub fn is_url_unquoted(b: &GoBytes) -> bool {
    let mut l = Lexer::new(Input::new_bytes(b.clone()));
    l.consume_unquoted_url();
    l.r.restore();
    l.r.pos() == b.len()
}

// Go: parse/css/util.go:HSL2RGB
/// Converts HSL to RGB with all of range [0,1], from
/// <http://www.w3.org/TR/css3-color/#hsl-color>.
pub fn hsl2rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let mut m2 = l * (s + 1.0);
    if l > 0.5 {
        // FMA: `l + s - l*s` is FMSUBD on arm64 (util.go:26)
        m2 = (-l).mul_add(s, l + s);
    }
    let m1 = l * 2.0 - m2; // `l*2` is compiled to `l+l`, no fusion (util.go:28)
    (
        hue2rgb(m1, m2, h + 1.0 / 3.0),
        hue2rgb(m1, m2, h),
        hue2rgb(m1, m2, h - 1.0 / 3.0),
    )
}

// Go: parse/css/util.go:hue2rgb (inlined into HSL2RGB by the Go compiler)
fn hue2rgb(m1: f64, m2: f64, mut h: f64) -> f64 {
    if h < 0.0 {
        h += 1.0;
    }
    if h > 1.0 {
        h -= 1.0;
    }
    if h * 6.0 < 1.0 {
        // FMA: `m1 + (m2-m1)*h*6.0` is FMADDD on arm64 (util.go:40)
        return ((m2 - m1) * h).mul_add(6.0, m1);
    } else if h * 2.0 < 1.0 {
        return m2;
    } else if h * 3.0 < 2.0 {
        // FMA: `m1 + (m2-m1)*(2.0/3.0-h)*6.0` is FMADDD on arm64 (util.go:44)
        return ((m2 - m1) * (2.0 / 3.0 - h)).mul_add(6.0, m1);
    }
    m1
}
