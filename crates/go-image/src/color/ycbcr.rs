//! Port of Go 1.27.1 `image/color/ycbcr.go`.

use super::Color;

/// RGBToYCbCr converts an RGB triple to a Y'CbCr triple.
///
/// Go: image/color/ycbcr.go:RGBToYCbCr
pub fn rgb_to_ycbcr(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    // The JFIF specification says:
    //	Y' =  0.2990*R + 0.5870*G + 0.1140*B
    //	Cb = -0.1687*R - 0.3313*G + 0.5000*B + 128
    //	Cr =  0.5000*R - 0.4187*G - 0.0813*B + 128
    // https://www.w3.org/Graphics/JPEG/jfif3.pdf says Y but means Y'.

    let r1 = r as i32;
    let g1 = g as i32;
    let b1 = b as i32;

    // yy is in range [0,0xff].
    //
    // Note that 19595 + 38470 + 7471 equals 65536.
    let yy = (19595 * r1 + 38470 * g1 + 7471 * b1 + (1 << 15)) >> 16;

    // The bit twiddling below is equivalent to
    //
    // cb := (-11056*r1 - 21712*g1 + 32768*b1 + 257<<15) >> 16
    // if cb < 0 {
    //     cb = 0
    // } else if cb > 0xff {
    //     cb = ^int32(0)
    // }
    //
    // but uses fewer branches and is faster.
    // Note that the uint8 type conversion in the return
    // statement will convert ^int32(0) to 0xff.
    // The code below to compute cr uses a similar pattern.
    //
    // Note that -11056 - 21712 + 32768 equals 0.
    let mut cb = -11056 * r1 - 21712 * g1 + 32768 * b1 + (257 << 15);
    if (cb as u32) & 0xff000000 == 0 {
        cb >>= 16;
    } else {
        cb = !(cb >> 31);
    }

    // Note that 32768 - 27440 - 5328 equals 0.
    let mut cr = 32768 * r1 - 27440 * g1 - 5328 * b1 + (257 << 15);
    if (cr as u32) & 0xff000000 == 0 {
        cr >>= 16;
    } else {
        cr = !(cr >> 31);
    }

    (yy as u8, cb as u8, cr as u8)
}

/// YCbCrToRGB converts a Y'CbCr triple to an RGB triple.
///
/// Go: image/color/ycbcr.go:YCbCrToRGB
pub fn ycbcr_to_rgb(y: u8, cb: u8, cr: u8) -> (u8, u8, u8) {
    // See the Go source for the derivation of the constants and of the
    // rounding adjustment YY1 = Y' * 0x10101.
    let yy1 = (y as i32) * 0x10101;
    let cb1 = (cb as i32) - 128;
    let cr1 = (cr as i32) - 128;

    // The bit twiddling below is equivalent to
    //
    // r := (yy1 + 91881*cr1) >> 16
    // if r < 0 {
    //     r = 0
    // } else if r > 0xff {
    //     r = ^int32(0)
    // }
    //
    // but uses fewer branches and is faster.
    // Note that the uint8 type conversion in the return
    // statement will convert ^int32(0) to 0xff.
    // The code below to compute g and b uses a similar pattern.
    let mut r = yy1 + 91881 * cr1;
    if (r as u32) & 0xff000000 == 0 {
        r >>= 16;
    } else {
        r = !(r >> 31);
    }

    let mut g = yy1 - 22554 * cb1 - 46802 * cr1;
    if (g as u32) & 0xff000000 == 0 {
        g >>= 16;
    } else {
        g = !(g >> 31);
    }

    let mut b = yy1 + 116130 * cb1;
    if (b as u32) & 0xff000000 == 0 {
        b >>= 16;
    } else {
        b = !(b >> 31);
    }

    (r as u8, g as u8, b as u8)
}

/// YCbCr represents a fully opaque 24-bit Y'CbCr color, having 8 bits each
/// for one luma and two chroma components.
///
/// Go: image/color/ycbcr.go:YCbCr
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct YCbCr {
    pub y: u8,
    pub cb: u8,
    pub cr: u8,
}

impl YCbCr {
    /// Go: image/color/ycbcr.go:YCbCr.RGBA — a copy of YCbCrToRGB returning
    /// 16-bit values.
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        let yy1 = (self.y as i32) * 0x10101;
        let cb1 = (self.cb as i32) - 128;
        let cr1 = (self.cr as i32) - 128;

        // The bit twiddling below is equivalent to
        //
        // r := (yy1 + 91881*cr1) >> 8
        // if r < 0 {
        //     r = 0
        // } else if r > 0xff {
        //     r = 0xffff
        // }
        //
        // but uses fewer branches and is faster.
        // The code below to compute g and b uses a similar pattern.
        let mut r = yy1 + 91881 * cr1;
        if (r as u32) & 0xff000000 == 0 {
            r >>= 8;
        } else {
            r = !(r >> 31) & 0xffff;
        }

        let mut g = yy1 - 22554 * cb1 - 46802 * cr1;
        if (g as u32) & 0xff000000 == 0 {
            g >>= 8;
        } else {
            g = !(g >> 31) & 0xffff;
        }

        let mut b = yy1 + 116130 * cb1;
        if (b as u32) & 0xff000000 == 0 {
            b >>= 8;
        } else {
            b = !(b >> 31) & 0xffff;
        }

        (r as u32, g as u32, b as u32, 0xffff)
    }
}

// Go: image/color/ycbcr.go:yCbCrModel
pub fn ycbcr_model(c: Color) -> Color {
    if let Color::YCbCr(_) = c {
        return c;
    }
    let (r, g, b, _) = c.rgba();
    let (y, u, v) = rgb_to_ycbcr((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8);
    Color::YCbCr(YCbCr { y, cb: u, cr: v })
}

/// NYCbCrA represents a non-alpha-premultiplied Y'CbCr-with-alpha color,
/// having 8 bits each for one luma, two chroma and one alpha component.
///
/// Go: image/color/ycbcr.go:NYCbCrA (the embedded YCbCr is the `ycbcr` field).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NYCbCrA {
    pub ycbcr: YCbCr,
    pub a: u8,
}

impl NYCbCrA {
    // Go: image/color/ycbcr.go:NYCbCrA.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        // The first part of this method is the same as YCbCr.RGBA.
        let yy1 = (self.ycbcr.y as i32) * 0x10101;
        let cb1 = (self.ycbcr.cb as i32) - 128;
        let cr1 = (self.ycbcr.cr as i32) - 128;

        let mut r = yy1 + 91881 * cr1;
        if (r as u32) & 0xff000000 == 0 {
            r >>= 8;
        } else {
            r = !(r >> 31) & 0xffff;
        }

        let mut g = yy1 - 22554 * cb1 - 46802 * cr1;
        if (g as u32) & 0xff000000 == 0 {
            g >>= 8;
        } else {
            g = !(g >> 31) & 0xffff;
        }

        let mut b = yy1 + 116130 * cb1;
        if (b as u32) & 0xff000000 == 0 {
            b >>= 8;
        } else {
            b = !(b >> 31) & 0xffff;
        }

        // The second part of this method applies the alpha.
        let a = (self.a as u32) * 0x101;
        (
            (r as u32) * a / 0xffff,
            (g as u32) * a / 0xffff,
            (b as u32) * a / 0xffff,
            a,
        )
    }
}

// Go: image/color/ycbcr.go:nYCbCrAModel
pub fn n_ycbcr_a_model(c: Color) -> Color {
    match c {
        Color::NYCbCrA(_) => return c,
        Color::YCbCr(c) => return Color::NYCbCrA(NYCbCrA { ycbcr: c, a: 0xff }),
        _ => {}
    }
    let (mut r, mut g, mut b, a) = c.rgba();

    // Convert from alpha-premultiplied to non-alpha-premultiplied.
    if a != 0 {
        r = r.wrapping_mul(0xffff) / a;
        g = g.wrapping_mul(0xffff) / a;
        b = b.wrapping_mul(0xffff) / a;
    }

    let (y, u, v) = rgb_to_ycbcr((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8);
    Color::NYCbCrA(NYCbCrA {
        ycbcr: YCbCr { y, cb: u, cr: v },
        a: (a >> 8) as u8,
    })
}

/// RGBToCMYK converts an RGB triple to a CMYK quadruple.
///
/// Go: image/color/ycbcr.go:RGBToCMYK
pub fn rgb_to_cmyk(r: u8, g: u8, b: u8) -> (u8, u8, u8, u8) {
    let rr = r as u32;
    let gg = g as u32;
    let bb = b as u32;
    let mut w = rr;
    if w < gg {
        w = gg;
    }
    if w < bb {
        w = bb;
    }
    if w == 0 {
        return (0, 0, 0, 0xff);
    }
    let c = (w - rr) * 0xff / w;
    let m = (w - gg) * 0xff / w;
    let y = (w - bb) * 0xff / w;
    (c as u8, m as u8, y as u8, (0xff - w) as u8)
}

/// CMYKToRGB converts a CMYK quadruple to an RGB triple.
///
/// Go: image/color/ycbcr.go:CMYKToRGB
pub fn cmyk_to_rgb(c: u8, m: u8, y: u8, k: u8) -> (u8, u8, u8) {
    let w = 0xffff - (k as u32) * 0x101;
    let r = (0xffff - (c as u32) * 0x101) * w / 0xffff;
    let g = (0xffff - (m as u32) * 0x101) * w / 0xffff;
    let b = (0xffff - (y as u32) * 0x101) * w / 0xffff;
    ((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8)
}

/// CMYK represents a fully opaque CMYK color, having 8 bits for each of cyan,
/// magenta, yellow and black.
///
/// Go: image/color/ycbcr.go:CMYK
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CMYK {
    pub c: u8,
    pub m: u8,
    pub y: u8,
    pub k: u8,
}

impl CMYK {
    // Go: image/color/ycbcr.go:CMYK.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        // This code is a copy of the CMYKToRGB function above, except that it
        // returns values in the range [0, 0xffff] instead of [0, 0xff].

        let w = 0xffff - (self.k as u32) * 0x101;
        let r = (0xffff - (self.c as u32) * 0x101) * w / 0xffff;
        let g = (0xffff - (self.m as u32) * 0x101) * w / 0xffff;
        let b = (0xffff - (self.y as u32) * 0x101) * w / 0xffff;
        (r, g, b, 0xffff)
    }
}

// Go: image/color/ycbcr.go:cmykModel
pub fn cmyk_model(c: Color) -> Color {
    if let Color::CMYK(_) = c {
        return c;
    }
    let (r, g, b, _) = c.rgba();
    let (cc, mm, yy, kk) = rgb_to_cmyk((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8);
    Color::CMYK(CMYK {
        c: cc,
        m: mm,
        y: yy,
        k: kk,
    })
}
