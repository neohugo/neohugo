//! Differential test: random draw.Draw / draw.DrawMask / FloydSteinberg
//! operations where destinations, sources and masks are sometimes
//! sub-images (Pix not starting at Rect.Min, Stride larger than the row,
//! odd YCbCr chroma offsets, empty sub-images), against the Go oracle
//! (`go-image draw2`). The destination digest is the sub-image's Pix, which
//! in Go aliases the parent's tail and in the port is an owned copy of it.

mod common;

use common::*;
use go_image::color::{Color, Model};
use go_image::draw::{self, Drawer, FLOYD_STEINBERG, Op};
use go_image::{Image, Rectangle, Uniform, pt, rect};
use std::any::Any;

/// Go oracle's slowDst.
struct SlowDst(Gen);

impl Image for SlowDst {
    fn color_model(&self) -> Model {
        self.0.as_image().color_model()
    }
    fn bounds(&self) -> Rectangle {
        self.0.as_image().bounds()
    }
    fn at(&self, x: i64, y: i64) -> Color {
        self.0.as_image().at(x, y)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl draw::Image for SlowDst {
    fn set(&mut self, x: i64, y: i64, c: Color) {
        self.0.as_draw_image().set(x, y, c)
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Go oracle's slowSrc.
struct SlowSrc(Gen);

impl Image for SlowSrc {
    fn color_model(&self) -> Model {
        self.0.as_image().color_model()
    }
    fn bounds(&self) -> Rectangle {
        self.0.as_image().bounds()
    }
    fn at(&self, x: i64, y: i64) -> Color {
        self.0.as_image().at(x, y)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

enum Dst {
    Gen(Gen),
    Slow(SlowDst),
}

impl Dst {
    fn as_image(&self) -> &dyn Image {
        match self {
            Dst::Gen(g) => g.as_image(),
            Dst::Slow(s) => s,
        }
    }
    fn as_draw_image(&mut self) -> &mut dyn draw::Image {
        match self {
            Dst::Gen(g) => g.as_draw_image(),
            Dst::Slow(s) => s,
        }
    }
    fn go_type(&self) -> &'static str {
        match self {
            Dst::Gen(g) => g.go_type(),
            Dst::Slow(_) => "*main.slowDst",
        }
    }
    fn pix(&self) -> &[u8] {
        match self {
            Dst::Gen(g) => g.pix(),
            Dst::Slow(s) => s.0.pix(),
        }
    }
}

enum Src {
    Gen(Gen),
    Uniform(Uniform),
    Rect(Rectangle),
    Slow(SlowSrc),
}

impl Src {
    fn as_image(&self) -> &dyn Image {
        match self {
            Src::Gen(g) => g.as_image(),
            Src::Uniform(u) => u,
            Src::Rect(r) => r,
            Src::Slow(s) => s,
        }
    }
    fn go_type(&self) -> &'static str {
        match self {
            Src::Gen(g) => g.go_type(),
            Src::Uniform(_) => "*image.Uniform",
            Src::Rect(_) => "image.Rectangle",
            Src::Slow(_) => "*main.slowSrc",
        }
    }
}

/// Go: `m.(subImager).SubImage(r)` for the generated image types.
fn gen_sub(g: &Gen, r: Rectangle) -> Gen {
    match g {
        Gen::RGBA(m) => Gen::RGBA(m.sub_image(r)),
        Gen::NRGBA(m) => Gen::NRGBA(m.sub_image(r)),
        Gen::Gray(m) => Gen::Gray(m.sub_image(r)),
        Gen::YCbCr(m) => Gen::YCbCr(m.sub_image(r)),
        Gen::CMYK(m) => Gen::CMYK(m.sub_image(r)),
        Gen::RGBA64(m) => Gen::RGBA64(m.sub_image(r)),
        Gen::NRGBA64(m) => Gen::NRGBA64(m.sub_image(r)),
        Gen::Paletted(m) => Gen::Paletted(m.sub_image(r)),
        Gen::Gray16(m) => Gen::Gray16(m.sub_image(r)),
        Gen::Alpha(m) => Gen::Alpha(m.sub_image(r)),
        Gen::NYCbCrA(m) => Gen::NYCbCrA(m.sub_image(r)),
        Gen::Alpha16(m) => Gen::Alpha16(m.sub_image(r)),
    }
}

// Go: maybeSub (the random choice and rectangle only).
fn maybe_sub_rect(r: &mut Rng, g: &Gen) -> Option<Rectangle> {
    if r.intn(3) != 0 {
        return None;
    }
    let b = g.as_image().bounds();
    let x0 = b.min.x - 1 + r.intn(b.dx() + 2);
    let y0 = b.min.y - 1 + r.intn(b.dy() + 2);
    let x1 = x0 + r.intn(b.dx() + 2);
    let y1 = y0 + r.intn(b.dy() + 2);
    Some(rect(x0, y0, x1, y1))
}

// Go: maybeSub
fn maybe_sub(r: &mut Rng, g: Gen) -> Gen {
    match maybe_sub_rect(r, &g) {
        None => g,
        Some(sr) => gen_sub(&g, sr),
    }
}

const DST2_KINDS: [i64; 12] = [0, 1, 11, 10, 12, 2, 13, 14, 16, 9, 0, 12];

// Go: dstImage2
fn dst_image2(r: &mut Rng) -> Dst {
    let rr = rand_rect(r, 24, 8);
    let k = r.intn(DST2_KINDS.len() as i64);
    let g = gen_image(r, DST2_KINDS[k as usize], rr);
    let m = maybe_sub(r, g);
    if k >= 10 {
        Dst::Slow(SlowDst(m))
    } else {
        Dst::Gen(m)
    }
}

// Go: srcImage2
fn src_image2(r: &mut Rng) -> Src {
    let k = r.intn(NUM_KINDS + 3);
    if k == NUM_KINDS {
        return Src::Uniform(Uniform::new(rand_color(r)));
    }
    if k == NUM_KINDS + 1 {
        return Src::Rect(rand_rect(r, 30, 10));
    }
    if k == NUM_KINDS + 2 {
        let k2 = r.intn(NUM_KINDS);
        let rr = rand_rect(r, 30, 10);
        let g = gen_image(r, k2, rr);
        return Src::Slow(SlowSrc(maybe_sub(r, g)));
    }
    let rr = rand_rect(r, 30, 10);
    let g = gen_image(r, k, rr);
    Src::Gen(maybe_sub(r, g))
}

// Go: maskImage2
fn mask_image2(r: &mut Rng) -> Option<Src> {
    let sub = |r: &mut Rng, kind: i64| -> Gen {
        let rr = rand_rect(r, 30, 10);
        let g = gen_image(r, kind, rr);
        maybe_sub(r, g)
    };
    match r.intn(8) {
        0 | 1 => None,
        2 => Some(Src::Gen(sub(r, 14))),
        3 => Some(Src::Gen(sub(r, 16))),
        4 => Some(Src::Uniform(Uniform::new(rand_color(r)))),
        5 => Some(Src::Gen(sub(r, 0))),
        6 => Some(Src::Slow(SlowSrc(sub(r, 14)))),
        _ => {
            let k = r.intn(NUM_KINDS);
            Some(Src::Gen(sub(r, k)))
        }
    }
}

fn run(rows: &[Vec<String>]) -> usize {
    let mut failures = 0;
    for want in rows {
        let seed: u64 = want[0].parse().unwrap();
        let mut r = Rng::new(seed.wrapping_mul(104729).wrapping_add(11));
        let mut dst = dst_image2(&mut r);
        let src = src_image2(&mut r);
        let mask = mask_image2(&mut r);
        let db = dst.as_image().bounds();
        let a = r.intn(12);
        let b = r.intn(12);
        let c = r.intn(12);
        let d = r.intn(12);
        let rr = rect(
            db.min.x - 4 + a,
            db.min.y - 4 + b,
            db.max.x - 6 + c,
            db.max.y - 6 + d,
        );
        let spx = r.intn(41) - 20;
        let spy = r.intn(41) - 20;
        let sp = pt(spx, spy);
        let mpx = r.intn(41) - 20;
        let mpy = r.intn(41) - 20;
        let mp = pt(mpx, mpy);
        let op = if r.intn(2) == 0 { Op::Over } else { Op::Src };
        let how = r.intn(5);
        match how {
            0 => FLOYD_STEINBERG.draw(dst.as_draw_image(), rr, src.as_image(), sp),
            1 => draw::draw(dst.as_draw_image(), rr, src.as_image(), sp, op),
            _ => draw::draw_mask(
                dst.as_draw_image(),
                rr,
                src.as_image(),
                sp,
                mask.as_ref().map(|m| m.as_image()),
                mp,
                op,
            ),
        }
        let got = vec![
            seed.to_string(),
            dst.go_type().to_string(),
            src.go_type().to_string(),
            rect_str(dst.as_image().bounds()),
            how.to_string(),
            sha16(dst.pix()),
        ];
        if &got != want {
            failures += 1;
            if failures < 20 {
                eprintln!("MISMATCH got {:?}\n         want {:?}", got, want);
            }
        }
    }
    failures
}

/// Draws into Go's aliasing sub-image of `parent` using the port's
/// `with_sub_image_mut` (as gift's DrawAt does) and returns the sub-image's
/// bounds and Go's `sub.Pix` (the parent's tail after the draw), or None for
/// the types without `with_sub_image_mut`.
fn draw_aliased(
    parent: &mut Gen,
    sub_r: Rectangle,
    slow: bool,
    f: &dyn Fn(&mut dyn draw::Image),
) -> Option<(Rectangle, Vec<u8>)> {
    macro_rules! case {
        ($m:expr, $variant:ident) => {{
            let rr = sub_r.intersect($m.rect);
            $m.with_sub_image_mut(sub_r, |sub| {
                if slow {
                    let owned = std::mem::take(sub);
                    let mut sd = SlowDst(Gen::$variant(owned));
                    f(&mut sd);
                    *sub = match sd.0 {
                        Gen::$variant(m) => m,
                        _ => unreachable!(),
                    };
                } else {
                    f(sub);
                }
            });
            let tail = if rr.empty() {
                Vec::new()
            } else {
                $m.pix[$m.pix_offset(rr.min.x, rr.min.y) as usize..].to_vec()
            };
            Some((rr, tail))
        }};
    }
    match parent {
        Gen::RGBA(m) => case!(m, RGBA),
        Gen::NRGBA(m) => case!(m, NRGBA),
        Gen::Gray(m) => case!(m, Gray),
        Gen::CMYK(m) => case!(m, CMYK),
        Gen::RGBA64(m) => case!(m, RGBA64),
        Gen::NRGBA64(m) => case!(m, NRGBA64),
        Gen::Gray16(m) => case!(m, Gray16),
        Gen::Alpha(m) => case!(m, Alpha),
        Gen::Alpha16(m) => case!(m, Alpha16),
        Gen::Paletted(_) | Gen::YCbCr(_) | Gen::NYCbCrA(_) => None,
    }
}

/// Replays the draw2 rows whose destination is a sub-image, drawing through
/// `with_sub_image_mut` on the parent instead of an owned sub-image copy.
fn run_aliased(rows: &[Vec<String>]) -> (usize, usize) {
    let (mut failures, mut checked) = (0, 0);
    for want in rows {
        let seed: u64 = want[0].parse().unwrap();
        let mut r = Rng::new(seed.wrapping_mul(104729).wrapping_add(11));
        // dstImage2, keeping the parent.
        let rr0 = rand_rect(&mut r, 24, 8);
        let k = r.intn(DST2_KINDS.len() as i64);
        let mut parent = gen_image(&mut r, DST2_KINDS[k as usize], rr0);
        let sub_r = maybe_sub_rect(&mut r, &parent);
        let slow = k >= 10;
        let src = src_image2(&mut r);
        let mask = mask_image2(&mut r);
        let db = match sub_r {
            Some(sr) => sr.intersect(parent.as_image().bounds()),
            None => parent.as_image().bounds(),
        };
        let a = r.intn(12);
        let b = r.intn(12);
        let c = r.intn(12);
        let d = r.intn(12);
        let rr = rect(
            db.min.x - 4 + a,
            db.min.y - 4 + b,
            db.max.x - 6 + c,
            db.max.y - 6 + d,
        );
        let spx = r.intn(41) - 20;
        let spy = r.intn(41) - 20;
        let sp = pt(spx, spy);
        let mpx = r.intn(41) - 20;
        let mpy = r.intn(41) - 20;
        let mp = pt(mpx, mpy);
        let op = if r.intn(2) == 0 { Op::Over } else { Op::Src };
        let how = r.intn(5);
        let Some(sub_r) = sub_r else {
            continue;
        };
        let go_type = if slow {
            "*main.slowDst"
        } else {
            parent.go_type()
        };
        let f = |dst: &mut dyn draw::Image| match how {
            0 => FLOYD_STEINBERG.draw(dst, rr, src.as_image(), sp),
            1 => draw::draw(dst, rr, src.as_image(), sp, op),
            _ => draw::draw_mask(
                dst,
                rr,
                src.as_image(),
                sp,
                mask.as_ref().map(|m| m.as_image()),
                mp,
                op,
            ),
        };
        let Some((sb, tail)) = draw_aliased(&mut parent, sub_r, slow, &f) else {
            continue;
        };
        checked += 1;
        let got = vec![
            seed.to_string(),
            go_type.to_string(),
            src.go_type().to_string(),
            rect_str(sb),
            how.to_string(),
            sha16(&tail),
        ];
        if &got != want {
            failures += 1;
            if failures < 20 {
                eprintln!("ALIASED MISMATCH got {:?}\n         want {:?}", got, want);
            }
        }
    }
    (failures, checked)
}

#[test]
fn draw2_aliased_sub_image_dst() {
    let rows = read_tsv("draw2.tsv");
    let (failures, checked) = run_aliased(&rows);
    assert!(checked > 1500, "only {} aliased rows", checked);
    assert_eq!(
        failures, 0,
        "{} of {} aliased draw ops differ",
        failures, checked
    );
}

#[test]
fn draw2_random_ops() {
    let rows = read_tsv("draw2.tsv");
    assert_eq!(rows.len(), 8000);
    let failures = run(&rows);
    assert_eq!(
        failures,
        0,
        "{} of {} draw ops differ",
        failures,
        rows.len()
    );
}

/// Larger corpus kept outside the repository (GO_IMAGE_DRAW2_BIG=<path to
/// `go-image draw2 0 2000000` output>).
#[test]
fn draw2_random_ops_big() {
    let Ok(path) = std::env::var("GO_IMAGE_DRAW2_BIG") else {
        return;
    };
    let rows = parse_tsv(&std::fs::read_to_string(path).unwrap());
    let (afail, achecked) = run_aliased(&rows);
    assert_eq!(
        afail, 0,
        "{} of {} aliased draw ops differ",
        afail, achecked
    );
    let failures = run(&rows);
    assert_eq!(
        failures,
        0,
        "{} of {} draw ops differ",
        failures,
        rows.len()
    );
}
