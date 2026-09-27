//! Go: minify/css/util.go

use tdewolff_parse::css::{HashToken, IdentToken};
use tdewolff_parse::{GoBytes, to_lower};

use super::Token;
use super::hash::Hash;
use super::table::shorten_color_hex;

// Go: css/util.go:removeMarkupNewlines
/// Removes any `\\\r\n`, `\\\r`, `\\\n` (in place).
pub(crate) fn remove_markup_newlines(mut data: GoBytes) -> GoBytes {
    let n = data.len() as isize;
    let mut i: isize = 1;
    while i < n - 2 {
        let iu = i as usize;
        if data.at(iu) == b'\\' && (data.at(iu + 1) == b'\n' || data.at(iu + 1) == b'\r') {
            // encountered first replacee, now start to move bytes to the front
            let mut j = i + 2;
            if data.at(iu + 1) == b'\r' && n > i + 2 && data.at(iu + 2) == b'\n' {
                j += 1;
            }
            while j < n {
                let ju = j as usize;
                if data.at(ju) == b'\\'
                    && n > j + 1
                    && (data.at(ju + 1) == b'\n' || data.at(ju + 1) == b'\r')
                {
                    if data.at(ju + 1) == b'\r' && n > j + 2 && data.at(ju + 2) == b'\n' {
                        j += 1;
                    }
                    j += 1;
                } else {
                    data.set(i as usize, data.at(ju));
                    i += 1;
                }
                j += 1;
            }
            data = data.slice_to(i as usize);
            break;
        }
        i += 1;
    }
    data
}

const HEX_LOWER: &[u8; 16] = b"0123456789abcdef";

// Go: css/util.go:rgbToToken
/// Converts r, g, b in the interval [0.0, 1.0] to a (shortest) hash or
/// color name token.
pub(crate) fn rgb_to_token(r: f64, g: f64, b: f64) -> Token {
    // r, g, b are in interval [0.0, 1.0]
    // FMA: `(r * 255.0) + 0.5` is FMADDD on arm64 (util.go:39), then
    // FCVTZSDW (float64 -> int32, saturating) and the low byte is kept.
    let conv = |x: f64| -> u8 { (x.mul_add(255.0, 0.5) as i32) as u8 };
    let rgb = [conv(r), conv(g), conv(b)];

    let val = GoBytes::make(7, 7);
    val.set(0, b'#');
    for (k, &c) in rgb.iter().enumerate() {
        // hex.Encode (lowercase)
        val.set(1 + 2 * k, HEX_LOWER[(c >> 4) as usize]);
        val.set(2 + 2 * k, HEX_LOWER[(c & 0x0f) as usize]);
    }
    let val = to_lower(val);
    if let Some(s) = shorten_color_hex(&val.slice_to(7).to_vec()) {
        return Token::new(IdentToken, GoBytes::from_slice(s), Hash(0), Hash(0));
    }
    let val = if val.at(1) == val.at(2) && val.at(3) == val.at(4) && val.at(5) == val.at(6) {
        val.set(2, val.at(3));
        val.set(3, val.at(5));
        val.slice_to(4)
    } else {
        val.slice_to(7)
    };
    Token::new(HashToken, val, Hash(0), Hash(0))
}
