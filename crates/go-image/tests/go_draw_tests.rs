//! Ports of go1.27.1 `image/draw/draw_test.go` (TestDraw with the
//! slowerRGBA/slowestRGBA types, TestNonZeroSrcPt, TestFill,
//! TestDrawSrcNonpremultiplied, TestFloydSteinbergCheckerboard, TestPaletted).
//! TestPaletted decodes testdata/video-001.jpeg instead of video-001.png (no
//! PNG decoder in this crate).

use std::any::Any;

use go_image::color::{self, Color, Model, Palette};
use go_image::draw::{self, Drawer, FLOYD_STEINBERG, Op};
use go_image::{
    Alpha, CMYK, Gray, Image, NRGBA, NRGBA64, Paletted, Point, RGBA, RGBA64Image, Rectangle,
    Uniform, YCbCr, YCbCrSubsampleRatio, pt, rect,
};

/// slowestRGBA: a draw.Image like image.RGBA that does not implement
/// draw.RGBA64Image (but does implement image.RGBA64Image, as in Go).
#[derive(Clone)]
struct SlowestRGBA {
    pix: Vec<u8>,
    stride: i64,
    rect: Rectangle,
}

/// slowerRGBA: like slowestRGBA but also implements draw.RGBA64Image.
#[derive(Clone)]
struct SlowerRGBA {
    pix: Vec<u8>,
    stride: i64,
    rect: Rectangle,
}

macro_rules! slow_impl {
    ($t:ident) => {
        impl $t {
            fn pix_offset(&self, x: i64, y: i64) -> i64 {
                (y - self.rect.min.y) * self.stride + (x - self.rect.min.x) * 4
            }
            fn rgba64_at_(&self, x: i64, y: i64) -> color::RGBA64 {
                if !pt(x, y).in_(self.rect) {
                    return color::RGBA64::default();
                }
                let i = self.pix_offset(x, y) as usize;
                let s = &self.pix[i..i + 4];
                let (r, g, b, a) = (s[0] as u16, s[1] as u16, s[2] as u16, s[3] as u16);
                color::RGBA64 {
                    r: (r << 8) | r,
                    g: (g << 8) | g,
                    b: (b << 8) | b,
                    a: (a << 8) | a,
                }
            }
        }
        impl Image for $t {
            fn color_model(&self) -> Model {
                Model::RGBA
            }
            fn bounds(&self) -> Rectangle {
                self.rect
            }
            fn at(&self, x: i64, y: i64) -> Color {
                Color::RGBA64(self.rgba64_at_(x, y))
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn into_any(self: Box<Self>) -> Box<dyn Any> {
                self
            }
            fn as_rgba64_image(&self) -> Option<&dyn RGBA64Image> {
                Some(self)
            }
        }
        impl RGBA64Image for $t {
            fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
                self.rgba64_at_(x, y)
            }
        }
    };
}
slow_impl!(SlowestRGBA);
slow_impl!(SlowerRGBA);

fn set_rgba_model(pix: &mut [u8], i: usize, c: Color) {
    let Color::RGBA(c1) = color::rgba_model(c) else {
        unreachable!()
    };
    pix[i] = c1.r;
    pix[i + 1] = c1.g;
    pix[i + 2] = c1.b;
    pix[i + 3] = c1.a;
}

impl draw::Image for SlowestRGBA {
    fn set(&mut self, x: i64, y: i64, c: Color) {
        if !pt(x, y).in_(self.rect) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        set_rgba_model(&mut self.pix, i, c);
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl draw::Image for SlowerRGBA {
    fn set(&mut self, x: i64, y: i64, c: Color) {
        if !pt(x, y).in_(self.rect) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        set_rgba_model(&mut self.pix, i, c);
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn as_draw_rgba64_image(&mut self) -> Option<&mut dyn draw::RGBA64Image> {
        Some(self)
    }
}

impl draw::RGBA64Image for SlowerRGBA {
    fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !pt(x, y).in_(self.rect) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 4];
        s[0] = (c.r >> 8) as u8;
        s[1] = (c.g >> 8) as u8;
        s[2] = (c.b >> 8) as u8;
        s[3] = (c.a >> 8) as u8;
    }
}

fn to_rgba(m: &dyn Image) -> RGBA {
    if let Some(r) = m.downcast_ref::<RGBA>() {
        return r.clone();
    }
    let mut rgba = RGBA::new(m.bounds());
    let b = rgba.bounds();
    draw::draw(&mut rgba, b, m, m.bounds().min, Op::Src);
    rgba
}

fn convert_to_slowest(m: &dyn Image) -> SlowestRGBA {
    let r = to_rgba(m);
    SlowestRGBA {
        pix: r.pix,
        stride: r.stride,
        rect: r.rect,
    }
}

fn convert_to_slower(m: &dyn Image) -> SlowerRGBA {
    let r = to_rgba(m);
    SlowerRGBA {
        pix: r.pix,
        stride: r.stride,
        rect: r.rect,
    }
}

fn eq(c0: Color, c1: Color) -> bool {
    c0.rgba() == c1.rgba()
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::RGBA(color::RGBA { r, g, b, a })
}

fn fill_blue(alpha: u8) -> Box<dyn Image> {
    Box::new(Uniform::new(rgba(0, 0, alpha, alpha)))
}

fn fill_alpha(alpha: u8) -> Box<dyn Image> {
    Box::new(Uniform::new(Color::Alpha(color::Alpha { a: alpha })))
}

fn vgrad_green(alpha: i64) -> Box<dyn Image> {
    let mut m = RGBA::new(rect(0, 0, 16, 16));
    for y in 0..16 {
        for x in 0..16 {
            m.set(x, y, rgba(0, (y * alpha / 15) as u8, 0, alpha as u8));
        }
    }
    Box::new(m)
}

fn vgrad_alpha(alpha: i64) -> Box<dyn Image> {
    let mut m = Alpha::new(rect(0, 0, 16, 16));
    for y in 0..16 {
        for x in 0..16 {
            m.set(
                x,
                y,
                Color::Alpha(color::Alpha {
                    a: (y * alpha / 15) as u8,
                }),
            );
        }
    }
    Box::new(m)
}

fn vgrad_green_nrgba(alpha: i64) -> Box<dyn Image> {
    let mut m = NRGBA::new(rect(0, 0, 16, 16));
    for y in 0..16 {
        for x in 0..16 {
            m.set(x, y, rgba(0, (y * 0x11) as u8, 0, alpha as u8));
        }
    }
    Box::new(m)
}

fn vgrad_cr() -> Box<dyn Image> {
    let mut m = YCbCr {
        y: vec![0; 256],
        cb: vec![0; 256],
        cr: vec![0; 256],
        y_stride: 16,
        c_stride: 16,
        subsample_ratio: YCbCrSubsampleRatio::Ratio444,
        rect: rect(0, 0, 16, 16),
    };
    for y in 0..16 {
        for x in 0..16 {
            m.cr[(y * m.c_stride + x) as usize] = (y * 0x11) as u8;
        }
    }
    Box::new(m)
}

fn vgrad_gray() -> Box<dyn Image> {
    let mut m = Gray::new(rect(0, 0, 16, 16));
    for y in 0..16 {
        for x in 0..16 {
            m.set(
                x,
                y,
                Color::Gray(color::Gray {
                    y: (y * 0x11) as u8,
                }),
            );
        }
    }
    Box::new(m)
}

fn vgrad_magenta() -> Box<dyn Image> {
    let mut m = CMYK::new(rect(0, 0, 16, 16));
    for y in 0..16 {
        for x in 0..16 {
            m.set(
                x,
                y,
                Color::CMYK(color::CMYK {
                    c: 0,
                    m: (y * 0x11) as u8,
                    y: 0,
                    k: 0x3f,
                }),
            );
        }
    }
    Box::new(m)
}

fn hgrad_red(alpha: i64) -> RGBA {
    let mut m = RGBA::new(rect(0, 0, 16, 16));
    for y in 0..16 {
        for x in 0..16 {
            m.set(x, y, rgba((x * alpha / 15) as u8, 0, 0, alpha as u8));
        }
    }
    m
}

struct DrawTest {
    desc: &'static str,
    src: Box<dyn Image>,
    mask: Option<Box<dyn Image>>,
    op: Op,
    expected: Color,
}

fn draw_tests() -> Vec<DrawTest> {
    use Op::{Over, Src};
    let t = |desc, src, mask, op, expected| DrawTest {
        desc,
        src,
        mask,
        op,
        expected,
    };
    let slower = |m: Box<dyn Image>| -> Box<dyn Image> { Box::new(convert_to_slower(m.as_ref())) };
    let slowest =
        |m: Box<dyn Image>| -> Box<dyn Image> { Box::new(convert_to_slowest(m.as_ref())) };
    vec![
        t(
            "nop",
            vgrad_green(255),
            Some(fill_alpha(0)),
            Over,
            rgba(136, 0, 0, 255),
        ),
        t(
            "clear",
            vgrad_green(255),
            Some(fill_alpha(0)),
            Src,
            rgba(0, 0, 0, 0),
        ),
        t(
            "fill",
            fill_blue(90),
            Some(fill_alpha(255)),
            Over,
            rgba(88, 0, 90, 255),
        ),
        t(
            "fillSrc",
            fill_blue(90),
            Some(fill_alpha(255)),
            Src,
            rgba(0, 0, 90, 90),
        ),
        t(
            "fillAlpha",
            fill_blue(90),
            Some(fill_alpha(192)),
            Over,
            rgba(100, 0, 68, 255),
        ),
        t(
            "fillAlphaSrc",
            fill_blue(90),
            Some(fill_alpha(192)),
            Src,
            rgba(0, 0, 68, 68),
        ),
        t("fillNil", fill_blue(90), None, Over, rgba(88, 0, 90, 255)),
        t("fillNilSrc", fill_blue(90), None, Src, rgba(0, 0, 90, 90)),
        t(
            "copy",
            vgrad_green(90),
            Some(fill_alpha(255)),
            Over,
            rgba(88, 48, 0, 255),
        ),
        t(
            "copySrc",
            vgrad_green(90),
            Some(fill_alpha(255)),
            Src,
            rgba(0, 48, 0, 90),
        ),
        t(
            "copyAlpha",
            vgrad_green(90),
            Some(fill_alpha(192)),
            Over,
            rgba(100, 36, 0, 255),
        ),
        t(
            "copyAlphaSrc",
            vgrad_green(90),
            Some(fill_alpha(192)),
            Src,
            rgba(0, 36, 0, 68),
        ),
        t("copyNil", vgrad_green(90), None, Over, rgba(88, 48, 0, 255)),
        t("copyNilSrc", vgrad_green(90), None, Src, rgba(0, 48, 0, 90)),
        t(
            "nrgba",
            vgrad_green_nrgba(90),
            Some(fill_alpha(255)),
            Over,
            rgba(88, 46, 0, 255),
        ),
        t(
            "nrgbaSrc",
            vgrad_green_nrgba(90),
            Some(fill_alpha(255)),
            Src,
            rgba(0, 46, 0, 90),
        ),
        t(
            "nrgbaAlpha",
            vgrad_green_nrgba(90),
            Some(fill_alpha(192)),
            Over,
            rgba(100, 34, 0, 255),
        ),
        t(
            "nrgbaAlphaSrc",
            vgrad_green_nrgba(90),
            Some(fill_alpha(192)),
            Src,
            rgba(0, 34, 0, 68),
        ),
        t(
            "nrgbaNil",
            vgrad_green_nrgba(90),
            None,
            Over,
            rgba(88, 46, 0, 255),
        ),
        t(
            "nrgbaNilSrc",
            vgrad_green_nrgba(90),
            None,
            Src,
            rgba(0, 46, 0, 90),
        ),
        t(
            "ycbcr",
            vgrad_cr(),
            Some(fill_alpha(255)),
            Over,
            rgba(11, 38, 0, 255),
        ),
        t(
            "ycbcrSrc",
            vgrad_cr(),
            Some(fill_alpha(255)),
            Src,
            rgba(11, 38, 0, 255),
        ),
        t(
            "ycbcrAlpha",
            vgrad_cr(),
            Some(fill_alpha(192)),
            Over,
            rgba(42, 28, 0, 255),
        ),
        t(
            "ycbcrAlphaSrc",
            vgrad_cr(),
            Some(fill_alpha(192)),
            Src,
            rgba(8, 28, 0, 192),
        ),
        t("ycbcrNil", vgrad_cr(), None, Over, rgba(11, 38, 0, 255)),
        t("ycbcrNilSrc", vgrad_cr(), None, Src, rgba(11, 38, 0, 255)),
        t(
            "gray",
            vgrad_gray(),
            Some(fill_alpha(255)),
            Over,
            rgba(136, 136, 136, 255),
        ),
        t(
            "graySrc",
            vgrad_gray(),
            Some(fill_alpha(255)),
            Src,
            rgba(136, 136, 136, 255),
        ),
        t(
            "grayAlpha",
            vgrad_gray(),
            Some(fill_alpha(192)),
            Over,
            rgba(136, 102, 102, 255),
        ),
        t(
            "grayAlphaSrc",
            vgrad_gray(),
            Some(fill_alpha(192)),
            Src,
            rgba(102, 102, 102, 192),
        ),
        t(
            "grayNil",
            vgrad_gray(),
            None,
            Over,
            rgba(136, 136, 136, 255),
        ),
        t(
            "grayNilSrc",
            vgrad_gray(),
            None,
            Src,
            rgba(136, 136, 136, 255),
        ),
        t(
            "graySlower",
            slower(vgrad_gray()),
            Some(fill_alpha(255)),
            Over,
            rgba(136, 136, 136, 255),
        ),
        t(
            "graySrcSlower",
            slower(vgrad_gray()),
            Some(fill_alpha(255)),
            Src,
            rgba(136, 136, 136, 255),
        ),
        t(
            "grayAlphaSlower",
            slower(vgrad_gray()),
            Some(fill_alpha(192)),
            Over,
            rgba(136, 102, 102, 255),
        ),
        t(
            "grayAlphaSrcSlower",
            slower(vgrad_gray()),
            Some(fill_alpha(192)),
            Src,
            rgba(102, 102, 102, 192),
        ),
        t(
            "grayNilSlower",
            slower(vgrad_gray()),
            None,
            Over,
            rgba(136, 136, 136, 255),
        ),
        t(
            "grayNilSrcSlower",
            slower(vgrad_gray()),
            None,
            Src,
            rgba(136, 136, 136, 255),
        ),
        t(
            "graySlowest",
            slowest(vgrad_gray()),
            Some(fill_alpha(255)),
            Over,
            rgba(136, 136, 136, 255),
        ),
        t(
            "graySrcSlowest",
            slowest(vgrad_gray()),
            Some(fill_alpha(255)),
            Src,
            rgba(136, 136, 136, 255),
        ),
        t(
            "grayAlphaSlowest",
            slowest(vgrad_gray()),
            Some(fill_alpha(192)),
            Over,
            rgba(136, 102, 102, 255),
        ),
        t(
            "grayAlphaSrcSlowest",
            slowest(vgrad_gray()),
            Some(fill_alpha(192)),
            Src,
            rgba(102, 102, 102, 192),
        ),
        t(
            "grayNilSlowest",
            slowest(vgrad_gray()),
            None,
            Over,
            rgba(136, 136, 136, 255),
        ),
        t(
            "grayNilSrcSlowest",
            slowest(vgrad_gray()),
            None,
            Src,
            rgba(136, 136, 136, 255),
        ),
        t(
            "cmyk",
            vgrad_magenta(),
            Some(fill_alpha(255)),
            Over,
            rgba(192, 89, 192, 255),
        ),
        t(
            "cmykSrc",
            vgrad_magenta(),
            Some(fill_alpha(255)),
            Src,
            rgba(192, 89, 192, 255),
        ),
        t(
            "cmykAlpha",
            vgrad_magenta(),
            Some(fill_alpha(192)),
            Over,
            rgba(178, 67, 145, 255),
        ),
        t(
            "cmykAlphaSrc",
            vgrad_magenta(),
            Some(fill_alpha(192)),
            Src,
            rgba(145, 67, 145, 192),
        ),
        t(
            "cmykNil",
            vgrad_magenta(),
            None,
            Over,
            rgba(192, 89, 192, 255),
        ),
        t(
            "cmykNilSrc",
            vgrad_magenta(),
            None,
            Src,
            rgba(192, 89, 192, 255),
        ),
        t(
            "generic",
            fill_blue(255),
            Some(vgrad_alpha(192)),
            Over,
            rgba(81, 0, 102, 255),
        ),
        t(
            "genericSrc",
            fill_blue(255),
            Some(vgrad_alpha(192)),
            Src,
            rgba(0, 0, 102, 102),
        ),
        t(
            "genericSlower",
            fill_blue(255),
            Some(slower(vgrad_alpha(192))),
            Over,
            rgba(81, 0, 102, 255),
        ),
        t(
            "genericSrcSlower",
            fill_blue(255),
            Some(slower(vgrad_alpha(192))),
            Src,
            rgba(0, 0, 102, 102),
        ),
        t(
            "genericSlowest",
            fill_blue(255),
            Some(slowest(vgrad_alpha(192))),
            Over,
            rgba(81, 0, 102, 255),
        ),
        t(
            "genericSrcSlowest",
            fill_blue(255),
            Some(slowest(vgrad_alpha(192))),
            Src,
            rgba(0, 0, 102, 102),
        ),
        t(
            "rgbaVariableMaskOver",
            vgrad_green(90),
            Some(vgrad_alpha(192)),
            Over,
            rgba(117, 19, 0, 255),
        ),
        t(
            "grayVariableMaskOver",
            vgrad_gray(),
            Some(vgrad_alpha(192)),
            Over,
            rgba(136, 54, 54, 255),
        ),
    ]
}

// Go: makeGolden
fn make_golden(
    dst: &dyn Image,
    r: Rectangle,
    src: &dyn Image,
    sp: Point,
    mask: Option<&dyn Image>,
    mp: Point,
    op: Op,
) -> RGBA {
    let b = dst.bounds();
    let sb = src.bounds();
    let mb = match mask {
        Some(m) => m.bounds(),
        None => rect(-1_000_000_000, -1_000_000_000, 1_000_000_000, 1_000_000_000),
    };
    let mut golden = RGBA::new(rect(0, 0, b.max.x, b.max.y));
    for y in r.min.y..r.max.y {
        let sy = y + sp.y - r.min.y;
        let my = y + mp.y - r.min.y;
        for x in r.min.x..r.max.x {
            if !pt(x, y).in_(b) {
                continue;
            }
            let sx = x + sp.x - r.min.x;
            if !pt(sx, sy).in_(sb) {
                continue;
            }
            let mx = x + mp.x - r.min.x;
            if !pt(mx, my).in_(mb) {
                continue;
            }
            const M: u32 = (1 << 16) - 1;
            let (mut dr, mut dg, mut db, mut da) = (0, 0, 0, 0);
            if op == Op::Over {
                (dr, dg, db, da) = dst.at(x, y).rgba();
            }
            let (sr, sg, sb_, sa) = src.at(sx, sy).rgba();
            let mut ma = M;
            if let Some(mask) = mask {
                (_, _, _, ma) = mask.at(mx, my).rgba();
            }
            let a = M - (sa * ma / M);
            golden.set(
                x,
                y,
                Color::RGBA64(color::RGBA64 {
                    r: ((dr * a + sr * ma) / M) as u16,
                    g: ((dg * a + sg * ma) / M) as u16,
                    b: ((db * a + sb_ * ma) / M) as u16,
                    a: ((da * a + sa * ma) / M) as u16,
                }),
            );
        }
    }
    golden.sub_image(b)
}

// Go: TestDraw
#[test]
fn draw_table() {
    let rr = [
        rect(0, 0, 0, 0),
        rect(0, 0, 16, 16),
        rect(3, 5, 12, 10),
        rect(0, 0, 9, 9),
        rect(8, 8, 16, 16),
        rect(8, 0, 9, 16),
        rect(0, 8, 16, 9),
        rect(8, 8, 9, 9),
        rect(8, 8, 8, 8),
    ];
    let tests = draw_tests();
    for r in rr {
        for test in &tests {
            for i in 0..3 {
                let base = hgrad_red(255).sub_image(r);
                let mut dst: Box<dyn draw::Image> = match i {
                    0 => Box::new(base),
                    1 => Box::new(convert_to_slower(&base)),
                    _ => Box::new(convert_to_slowest(&base)),
                };
                let golden = make_golden(
                    dst.as_ref(),
                    rect(0, 0, 16, 16),
                    test.src.as_ref(),
                    Point::default(),
                    test.mask.as_deref(),
                    Point::default(),
                    test.op,
                );
                let b = dst.bounds();
                assert!(b.eq(golden.bounds()), "{} bounds", test.desc);
                draw::draw_mask(
                    dst.as_mut(),
                    rect(0, 0, 16, 16),
                    test.src.as_ref(),
                    Point::default(),
                    test.mask.as_deref(),
                    Point::default(),
                    test.op,
                );
                if pt(8, 8).in_(r) {
                    assert!(
                        eq(dst.at(8, 8), test.expected),
                        "draw {} {} (i={}): at (8, 8) {:?} versus {:?}",
                        r,
                        test.desc,
                        i,
                        dst.at(8, 8),
                        test.expected
                    );
                }
                for y in b.min.y..b.max.y {
                    for x in b.min.x..b.max.x {
                        assert!(
                            eq(dst.at(x, y), golden.at(x, y)),
                            "draw {} {} (i={}): at ({}, {})",
                            r,
                            test.desc,
                            i,
                            x,
                            y
                        );
                    }
                }
            }
        }
    }
}

// Go: TestNonZeroSrcPt
#[test]
fn non_zero_src_pt() {
    let mut a = RGBA::new(rect(0, 0, 1, 1));
    let mut b = RGBA::new(rect(0, 0, 2, 2));
    b.set(0, 0, rgba(0, 0, 0, 5));
    b.set(1, 0, rgba(0, 0, 5, 5));
    b.set(0, 1, rgba(0, 5, 0, 5));
    b.set(1, 1, rgba(5, 0, 0, 5));
    draw::draw(&mut a, rect(0, 0, 1, 1), &b, pt(1, 1), Op::Over);
    assert!(eq(rgba(5, 0, 0, 5), a.at(0, 0)));
}

// Go: TestFill
#[test]
fn fill() {
    let rr = [
        rect(0, 0, 0, 0),
        rect(0, 0, 40, 30),
        rect(10, 0, 40, 30),
        rect(0, 20, 40, 30),
        rect(10, 20, 40, 30),
        rect(10, 20, 15, 25),
        rect(10, 0, 35, 30),
        rect(0, 15, 40, 16),
        rect(24, 24, 25, 25),
        rect(23, 23, 26, 26),
        rect(22, 22, 27, 27),
        rect(21, 21, 28, 28),
        rect(20, 20, 29, 29),
    ];
    for r in rr {
        let mut m = RGBA::new(rect(0, 0, 40, 30)).sub_image(r);
        let b = m.bounds();
        let check = |m: &RGBA, c: Color| {
            for y in b.min.y..b.max.y {
                for x in b.min.x..b.max.x {
                    assert!(eq(c, m.at(x, y)), "fill at ({}, {}) bounds {}", x, y, r);
                }
            }
        };
        let c = rgba(11, 0, 0, 255);
        let src = Uniform::new(c);
        for y in b.min.y..b.max.y {
            for x in b.min.x..b.max.x {
                draw::draw_mask(
                    &mut m,
                    rect(x, y, x + 1, y + 1),
                    &src,
                    Point::default(),
                    None,
                    Point::default(),
                    Op::Src,
                );
            }
        }
        check(&m, c);
        let c = rgba(0, 22, 0, 255);
        let src = Uniform::new(c);
        for y in b.min.y..b.max.y {
            draw::draw_mask(
                &mut m,
                rect(b.min.x, y, b.max.x, y + 1),
                &src,
                Point::default(),
                None,
                Point::default(),
                Op::Src,
            );
        }
        check(&m, c);
        let c = rgba(0, 0, 33, 255);
        let src = Uniform::new(c);
        for x in b.min.x..b.max.x {
            draw::draw_mask(
                &mut m,
                rect(x, b.min.y, x + 1, b.max.y),
                &src,
                Point::default(),
                None,
                Point::default(),
                Op::Src,
            );
        }
        check(&m, c);
        let c = rgba(44, 55, 66, 77);
        let src = Uniform::new(c);
        draw::draw_mask(
            &mut m,
            b,
            &src,
            Point::default(),
            None,
            Point::default(),
            Op::Src,
        );
        check(&m, c);
    }
}

// Go: TestDrawSrcNonpremultiplied
#[test]
fn draw_src_nonpremultiplied() {
    let nrgba = |r, g, b, a| color::NRGBA { r, g, b, a };
    let opaque_gray = nrgba(0x99, 0x99, 0x99, 0xff);
    let transparent_blue = nrgba(0x00, 0x00, 0xff, 0x00);
    let transparent_green = nrgba(0x00, 0xff, 0x00, 0x00);
    let transparent_red = nrgba(0xff, 0x00, 0x00, 0x00);
    let opaque_gray64 = color::NRGBA64 {
        r: 0x9999,
        g: 0x9999,
        b: 0x9999,
        a: 0xffff,
    };
    let transparent_purple64 = color::NRGBA64 {
        r: 0xfedc,
        g: 0x0000,
        b: 0x7654,
        a: 0x0000,
    };
    {
        let mut dst = NRGBA::new(rect(0, 10, 3, 11));
        dst.set_nrgba(0, 10, opaque_gray);
        let mut src = NRGBA::new(rect(1, 20, 4, 21));
        src.set_nrgba(1, 20, transparent_blue);
        src.set_nrgba(2, 20, transparent_green);
        src.set_nrgba(3, 20, transparent_red);
        draw::draw(&mut dst, rect(1, 10, 3, 11), &src, pt(1, 20), Op::Src);
        assert_eq!(dst.at(0, 10), Color::NRGBA(opaque_gray));
        assert_eq!(dst.at(1, 10), Color::NRGBA(transparent_blue));
        assert_eq!(dst.at(2, 10), Color::NRGBA(transparent_green));
    }
    {
        let mut dst = NRGBA64::new(rect(0, 0, 1, 1));
        dst.set_nrgba64(0, 0, opaque_gray64);
        let mut src = NRGBA64::new(rect(0, 0, 1, 1));
        src.set_nrgba64(0, 0, transparent_purple64);
        let b = dst.bounds();
        draw::draw(&mut dst, b, &src, pt(0, 0), Op::Src);
        assert_eq!(dst.at(0, 0), Color::NRGBA64(transparent_purple64));
    }
}

// Go: TestFloydSteinbergCheckerboard
#[test]
fn floyd_steinberg_checkerboard() {
    let b = rect(0, 0, 640, 480);
    let src = Uniform::new(Color::Gray16(color::Gray16 { y: 0x7fff }));
    let mut dst = Paletted::new(
        b,
        Palette(vec![
            Color::Gray16(color::BLACK),
            Color::Gray16(color::WHITE),
        ]),
    );
    FLOYD_STEINBERG.draw(&mut dst, b, &src, Point::default());
    for y in b.min.y..b.max.y {
        for x in b.min.x..b.max.x {
            let got = dst.pix[dst.pix_offset(x, y) as usize];
            assert_eq!(got, ((x + y) % 2) as u8, "at ({}, {})", x, y);
        }
    }
}

/// embeddedPaletted: behaves like *image.Paletted but is a different type.
struct EmbeddedPaletted(Paletted);

impl Image for EmbeddedPaletted {
    fn color_model(&self) -> Model {
        self.0.color_model()
    }
    fn bounds(&self) -> Rectangle {
        self.0.bounds()
    }
    fn at(&self, x: i64, y: i64) -> Color {
        self.0.at(x, y)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn as_rgba64_image(&self) -> Option<&dyn RGBA64Image> {
        Some(self)
    }
}

impl RGBA64Image for EmbeddedPaletted {
    fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        self.0.rgba64_at(x, y)
    }
}

impl draw::Image for EmbeddedPaletted {
    fn set(&mut self, x: i64, y: i64, c: Color) {
        self.0.set(x, y, c)
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn as_draw_rgba64_image(&mut self) -> Option<&mut dyn draw::RGBA64Image> {
        Some(self)
    }
}

impl draw::RGBA64Image for EmbeddedPaletted {
    fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        self.0.set_rgba64(x, y, c)
    }
}

// Go: TestPaletted
#[test]
fn paletted_matches_embedded() {
    let data = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/gotestdata/video-001.jpeg"),
    )
    .unwrap();
    let video001 = go_image::jpeg::decode(&mut &data[..]).unwrap();
    let b = video001.bounds();
    let cga = Palette(vec![
        rgba(0x00, 0x00, 0x00, 0xff),
        rgba(0x55, 0xff, 0xff, 0xff),
        rgba(0xff, 0x55, 0xff, 0xff),
        rgba(0xff, 0xff, 0xff, 0xff),
    ]);
    let uniform = Uniform::new(rgba(0xff, 0x7f, 0xff, 0xff));
    let drawers: [&dyn Drawer; 2] = [&Op::Src, &FLOYD_STEINBERG];
    let sources: [&dyn Image; 2] = [&uniform, video001.as_ref()];
    for d in drawers {
        for src in sources {
            let mut dst0 = Paletted::new(b, cga.clone());
            let mut dst1 = EmbeddedPaletted(Paletted::new(b, cga.clone()));
            d.draw(&mut dst0, b, src, Point::default());
            d.draw(&mut dst1, b, src, Point::default());
            for y in b.min.y..b.max.y {
                for x in b.min.x..b.max.x {
                    assert!(eq(dst0.at(x, y), dst1.at(x, y)), "at ({}, {})", x, y);
                }
            }
        }
    }
}
