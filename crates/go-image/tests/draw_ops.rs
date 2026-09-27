//! Differential test: random draw.Draw / draw.DrawMask / FloydSteinberg
//! operations over every destination, source and mask image type (including
//! invalid premultiplied colours that exercise Go's wrapping arithmetic),
//! against the Go oracle (`go-image draw`).

mod common;

use common::*;
use go_image::color::{Color, Model};
use go_image::draw::{self, Drawer, FLOYD_STEINBERG, Op};
use go_image::{Image, Rectangle, Uniform, pt, rect};
use std::any::Any;

/// Go oracle's slowDst: a draw.Image implementing neither image.RGBA64Image
/// nor draw.RGBA64Image (forces DrawMask's FALLBACK1.0).
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

/// Go oracle's slowSrc: an image.Image that is not an image.RGBA64Image.
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

// Go: dstImage
fn dst_image(r: &mut Rng) -> Dst {
    let rr = rand_rect(r, 24, 8);
    let (kind, slow) = match r.intn(12) {
        0 => (0, false),
        1 => (1, false),
        2 => (11, false),
        3 => (10, false),
        4 => (12, false),
        5 => (2, false),
        6 => (13, false),
        7 => (14, false),
        8 => (16, false),
        9 => (9, false),
        10 => (0, true),
        _ => (12, true),
    };
    let g = gen_image(r, kind, rr);
    if slow {
        Dst::Slow(SlowDst(g))
    } else {
        Dst::Gen(g)
    }
}

// Go: srcImage
fn src_image(r: &mut Rng) -> Src {
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
        return Src::Slow(SlowSrc(gen_image(r, k2, rr)));
    }
    let rr = rand_rect(r, 30, 10);
    Src::Gen(gen_image(r, k, rr))
}

// Go: maskImage
fn mask_image(r: &mut Rng) -> Option<Src> {
    match r.intn(8) {
        0 | 1 => None,
        2 => {
            let rr = rand_rect(r, 30, 10);
            Some(Src::Gen(gen_image(r, 14, rr)))
        }
        3 => {
            let rr = rand_rect(r, 30, 10);
            Some(Src::Gen(gen_image(r, 16, rr)))
        }
        4 => Some(Src::Uniform(Uniform::new(rand_color(r)))),
        5 => {
            let rr = rand_rect(r, 30, 10);
            Some(Src::Gen(gen_image(r, 0, rr)))
        }
        6 => {
            let rr = rand_rect(r, 30, 10);
            Some(Src::Slow(SlowSrc(gen_image(r, 14, rr))))
        }
        _ => {
            let k = r.intn(NUM_KINDS);
            let rr = rand_rect(r, 30, 10);
            Some(Src::Gen(gen_image(r, k, rr)))
        }
    }
}

fn run(rows: &[Vec<String>]) -> usize {
    let mut failures = 0;
    for want in rows {
        let seed: u64 = want[0].parse().unwrap();
        let mut r = Rng::new(seed.wrapping_mul(7919).wrapping_add(3));
        let mut dst = dst_image(&mut r);
        let src = src_image(&mut r);
        let mask = mask_image(&mut r);
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

#[test]
fn draw_random_ops() {
    let rows = read_tsv("draw.tsv");
    assert_eq!(rows.len(), 12000);
    let failures = run(&rows);
    assert_eq!(
        failures,
        0,
        "{} of {} draw ops differ",
        failures,
        rows.len()
    );
}

/// Larger corpus kept outside the repository (GO_IMAGE_DRAW_BIG=<path to
/// `go-image draw 200000` output>).
#[test]
fn draw_random_ops_big() {
    let Ok(path) = std::env::var("GO_IMAGE_DRAW_BIG") else {
        return;
    };
    let rows = parse_tsv(&std::fs::read_to_string(path).unwrap());
    let failures = run(&rows);
    assert_eq!(
        failures,
        0,
        "{} of {} draw ops differ",
        failures,
        rows.len()
    );
}
