//! Port of `resources/images/color.go`.
//!
//! Owner: Wave B task T10 (images).

use std::borrow::Cow;
use std::sync::Arc;

use go_image::color::{self, Color as GoColor, Palette};
use go_value::{HostCtx, Object, Value};
use nh_common::{Error, Result};

/// Go: `images.colorGoProvider` — a value that carries a Go `color.Color` (implemented by
/// [`Color`]).
pub trait ColorGoProvider {
    fn color_go(&self) -> GoColor;
}

/// Go: `images.Color` (what `.Colors` returns and what the text/padding/dither options accept).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// The color.
    color: GoColor,
    /// The color prefixed with a #.
    hex: HexString,
    /// The relative luminance of the color.
    luminance: f64,
}

/// `#rrggbb` or `#rrggbbaa` (ASCII, at most 9 bytes).
#[derive(Clone, Copy, PartialEq, Eq)]
struct HexString {
    buf: [u8; 9],
    len: u8,
}

impl HexString {
    fn as_str(&self) -> &str {
        // Only ASCII hex digits and '#' are ever stored.
        std::str::from_utf8(&self.buf[..self.len as usize]).unwrap_or("")
    }
}

impl std::fmt::Debug for HexString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Color {
    /// Luminance as defined by w3.org.
    // Go: resources/images/color.go:Luminance
    pub fn luminance(&self) -> f64 {
        self.luminance
    }

    /// ColorGo returns the color as a color.Color.
    // Go: resources/images/color.go:ColorGo
    pub fn color_go(&self) -> GoColor {
        self.color
    }

    /// ColorHex returns the color as a hex string prefixed with a #.
    // Go: resources/images/color.go:ColorHex
    pub fn color_hex(&self) -> &str {
        self.hex.as_str()
    }

    /// String returns the color as a hex string prefixed with a #.
    // Go: resources/images/color.go:String
    pub fn string(&self) -> &str {
        self.hex.as_str()
    }

    /// For hashstructure (`Hashable`): FNV-1a 64 of the hex string.
    // Go: resources/images/color.go:Hash
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in self.hex.as_str().as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    // Go: resources/images/color.go:init
    fn init(&mut self) {
        let hex = color_go_to_hex_string(self.color);
        let mut buf = [0u8; 9];
        buf[..hex.len()].copy_from_slice(hex.as_bytes());
        self.hex = HexString {
            buf,
            len: hex.len() as u8,
        };
        let (r, g, b, _) = self.color.rgba();
        // 0.2126*toSRGB(r) + 0.7152*toSRGB(g) + 0.0722*toSRGB(b): arm64 fuses both additions
        // (FMADDD; the red product is rounded), see PORTING.md "FMA sites".
        let rr = 0.2126 * self.to_srgb(r as u8);
        let s = self.to_srgb(g as u8).mul_add(0.7152, rr);
        self.luminance = self.to_srgb(b as u8).mul_add(0.0722, s);
    }

    // Go: resources/images/color.go:toSRGB
    fn to_srgb(self, i: u8) -> f64 {
        let v = i as f64 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            gift::gomath::pow((v + 0.055) / 1.055, 2.4)
        }
    }
}

impl ColorGoProvider for Color {
    fn color_go(&self) -> GoColor {
        self.color
    }
}

impl Object for Color {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("images.Color")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(
            name,
            "Luminance" | "ColorGo" | "ColorHex" | "String" | "Hash"
        )
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if !self.has_method(name) {
            return None;
        }
        if !args.is_empty() {
            return Some(Err(go_value::Error::new(format!(
                "wrong number of args for {name}: want 0 got {}",
                args.len()
            ))));
        }
        Some(Ok(match name {
            "Luminance" => Value::float64(self.luminance),
            "ColorHex" | "String" => Value::string(self.hex.as_str()),
            "Hash" => Value::Uint(self.hash(), go_value::UintKind::Uint64),
            // ColorGo returns a color.Color (internal use only): exposed as Go's `%v` of it.
            _ => Value::string(go_color_v(self.color)),
        }))
    }
    fn go_string(&self) -> Option<go_value::GoString> {
        Some(self.hex.as_str().into())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go's `%v` of a `color.Color` value (`{1 2 3 255}`, `{65535}`).
pub fn go_color_v(c: GoColor) -> String {
    match c {
        GoColor::RGBA(x) => format!("{{{} {} {} {}}}", x.r, x.g, x.b, x.a),
        GoColor::RGBA64(x) => format!("{{{} {} {} {}}}", x.r, x.g, x.b, x.a),
        GoColor::NRGBA(x) => format!("{{{} {} {} {}}}", x.r, x.g, x.b, x.a),
        GoColor::NRGBA64(x) => format!("{{{} {} {} {}}}", x.r, x.g, x.b, x.a),
        GoColor::Alpha(x) => format!("{{{}}}", x.a),
        GoColor::Alpha16(x) => format!("{{{}}}", x.a),
        GoColor::Gray(x) => format!("{{{}}}", x.y),
        GoColor::Gray16(x) => format!("{{{}}}", x.y),
        GoColor::YCbCr(x) => format!("{{{} {} {}}}", x.y, x.cb, x.cr),
        GoColor::NYCbCrA(x) => format!(
            "{{{{{} {} {}}} {}}}",
            x.ycbcr.y, x.ycbcr.cb, x.ycbcr.cr, x.a
        ),
        GoColor::CMYK(x) => format!("{{{} {} {} {}}}", x.c, x.m, x.y, x.k),
    }
}

/// AddColorToPalette adds c as the first color in p if not already there.
// Go: resources/images/color.go:AddColorToPalette
pub fn add_color_to_palette(c: GoColor, p: Palette) -> Palette {
    let found = p.0.contains(&c);
    if !found {
        let mut v = Vec::with_capacity(p.0.len() + 1);
        v.push(c);
        v.extend(p.0);
        return Palette(v);
    }
    p
}

/// ReplaceColorInPalette will replace the color in palette p closest to c in Euclidean
/// R,G,B,A space with c.
// Go: resources/images/color.go:ReplaceColorInPalette
pub fn replace_color_in_palette(c: GoColor, p: &mut Palette) {
    let i = p.index(c);
    p.0[i] = c;
}

/// ColorGoToHexString converts a color.Color to a hex string.
// Go: resources/images/color.go:ColorGoToHexString
pub fn color_go_to_hex_string(c: GoColor) -> String {
    let (r, g, b, a) = c.rgba();
    let rgba = color::RGBA {
        r: r as u8,
        g: g as u8,
        b: b as u8,
        a: a as u8,
    };
    if rgba.a == 0xff {
        return format!("#{:02x}{:02x}{:02x}", rgba.r, rgba.g, rgba.b);
    }
    format!("#{:02x}{:02x}{:02x}{:02x}", rgba.r, rgba.g, rgba.b, rgba.a)
}

/// ColorGoToColor converts a color.Color to a Color.
// Go: resources/images/color.go:ColorGoToColor
pub fn color_go_to_color(c: GoColor) -> Color {
    let mut cc = Color {
        color: c,
        hex: HexString {
            buf: [0; 9],
            len: 0,
        },
        luminance: 0.0,
    };
    cc.init();
    cc
}

/// Go panics on an invalid color; the port returns the error.
// Go: resources/images/color.go:hexStringToColor
pub fn hex_string_to_color(s: &[u8]) -> Result<Color> {
    let c = hex_string_to_color_go(s)?;
    Ok(color_go_to_color(c))
}

/// HexStringsToColors converts a slice of hex strings to a slice of Colors.
// Go: resources/images/color.go:HexStringsToColors
pub fn hex_strings_to_colors(s: &[&[u8]]) -> Result<Vec<Color>> {
    let mut colors = Vec::new();
    for v in s {
        colors.push(hex_string_to_color(v)?);
    }
    Ok(colors)
}

/// Go: `toColorGo(v)` -> (color, ok, err): a [`Color`] object gives its color; anything
/// `hstrings.ToString` accepts is parsed as a hex string.
// Go: resources/images/color.go:toColorGo
pub fn to_color_go(v: &Value) -> Result<Option<GoColor>> {
    if let Some(c) = v.downcast::<Color>() {
        return Ok(Some(c.color_go()));
    }
    let Some(s) = nh_common::hstrings::to_string(v) else {
        return Ok(None);
    };
    let c = hex_string_to_color_go(&s)?;
    Ok(Some(c))
}

/// Go: `hexStringToColorGo(s)`: `#`-less (or `#`-prefixed) 3, 4, 6 or 8 digit hex colours;
/// `ffffff` and `000000` are `color.White`/`color.Black` (`color.Gray16`), everything else a
/// `color.RGBA`.
// Go: resources/images/color.go:hexStringToColorGo
pub fn hex_string_to_color_go(s: &[u8]) -> Result<GoColor> {
    let s = go_unicode::strings::trim_prefix(s, b"#");

    if s.len() != 3 && s.len() != 4 && s.len() != 6 && s.len() != 8 {
        return Err(Error::new(format!(
            "invalid color code: {}",
            go_strconv::quote(s)
        )));
    }

    let mut s: Vec<u8> = go_unicode::strings::to_lower(s).into_owned();

    if s.len() == 3 || s.len() == 4 {
        // for _, r := range s { v += string(r) + string(r) }
        let mut v = Vec::new();
        let mut i = 0;
        while i < s.len() {
            let (r, size) = go_unicode::utf8::decode_rune(&s[i..]);
            let mut enc = [0u8; 4];
            let n = go_unicode::utf8::encode_rune(&mut enc, r);
            v.extend_from_slice(&enc[..n]);
            v.extend_from_slice(&enc[..n]);
            i += size;
        }
        s = v;
    }

    // Standard colors.
    if s == b"ffffff" {
        return Ok(GoColor::Gray16(color::WHITE));
    }

    if s == b"000000" {
        return Ok(GoColor::Gray16(color::BLACK));
    }

    // Set Alfa to white.
    if s.len() == 6 {
        s.extend_from_slice(b"ff");
    }

    let b = hex_decode_string(&s)?;

    Ok(GoColor::RGBA(color::RGBA {
        r: b[0],
        g: b[1],
        b: b[2],
        a: b[3],
    }))
}

/// Go `encoding/hex.DecodeString` (errors: `InvalidByteError`, `ErrLength`).
fn hex_decode_string(src: &[u8]) -> Result<Vec<u8>> {
    fn from_hex_char(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(src.len() / 2);
    let mut j = 1;
    while j < src.len() {
        let p = src[j - 1];
        let q = src[j];
        let Some(a) = from_hex_char(p) else {
            return Err(invalid_byte_error(p));
        };
        let Some(b) = from_hex_char(q) else {
            return Err(invalid_byte_error(q));
        };
        out.push((a << 4) | b);
        j += 2;
    }
    if src.len() % 2 == 1 {
        // Check for invalid char before reporting bad length, since the invalid char (if
        // present) is an earlier problem.
        if from_hex_char(src[j - 1]).is_none() {
            return Err(invalid_byte_error(src[j - 1]));
        }
        return Err(Error::new("encoding/hex: odd length hex string"));
    }
    Ok(out)
}

/// Go `hex.InvalidByteError.Error()`: `encoding/hex: invalid byte: %#U`.
fn invalid_byte_error(b: u8) -> Error {
    let r = b as i32;
    let mut s = format!("encoding/hex: invalid byte: U+{:04X}", r);
    if go_strconv::is_print(r)
        && let Some(c) = char::from_u32(r as u32)
    {
        s.push_str(&format!(" '{c}'"));
    }
    Error::new(s)
}

/// The shared `Arc` form used in filter options.
pub fn color_value(c: Color) -> Value {
    Value::Object(Arc::new(c))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/color.go (202 lines; 1/15 funcs executed)
//   types: colorGoProvider, Color
// OK L45-47: (c Color) Luminance() float64
// OK L51-53: (c Color) ColorGo() color.Color
// OK L56-58: (c Color) ColorHex() string
// OK L61-63: (c Color) String() string
// OK L68-72: (c Color) Hash() (uint64, error)
// OK L74-79: (c *Color) init() error
// OK L81-88: (c Color) toSRGB(i uint8) float64
// OK L93-104: AddColorToPalette(c color.Color, p color.Palette) color.Palette
// OK L108-110: ReplaceColorInPalette(c color.Color, p color.Palette)
// OK L113-120: ColorGoToHexString(c color.Color) string
// OK L123-129: ColorGoToColor(c color.Color) Color
// OK L131-137: hexStringToColor(s string) Color
// OK L140-146: HexStringsToColors(s ...string) []Color
// OK L148-163: toColorGo(v any) (color.Color, bool, error)
// OK L165-202: hexStringToColorGo(s string) (color.Color, error)
// ---------------------------------------------------------------------------
