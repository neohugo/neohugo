//! Port of gift v1.2.1 `gift.go`: the `Filter` interface, `Options`, `GIFT`
//! (filter lists with `Bounds`, `Draw` and `DrawAt`).

use std::any::Any;
use std::sync::Arc;

use go_image::{Gray, Gray16, Image, NRGBA, NRGBA64, Point, RGBA, RGBA64, Rectangle, draw};

use crate::pixels::{Pixel, PixelGetter, PixelSetter};
use crate::utils::{copyimage, create_temp_image, parallelize};

/// Filter is an image processing filter.
///
/// Go: gift.go:Filter. `Any` is a supertrait so callers can downcast a
/// `&dyn Filter` to a concrete filter type (Go type switches / reflection).
pub trait Filter: Any + Send + Sync {
    /// Draw applies the filter to the src image and outputs the result to the
    /// dst image. `None` is Go's nil `*Options` (the default options).
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>);
    /// Bounds calculates the appropriate bounds of an image after applying
    /// the filter.
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle;
}

/// Options is the parameters passed to image processing filters.
///
/// Go: gift.go:Options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Options {
    pub parallelization: bool,
}

/// Go: gift.go:defaultOptions
pub const DEFAULT_OPTIONS: Options = Options {
    parallelization: true,
};

impl Default for Options {
    fn default() -> Options {
        DEFAULT_OPTIONS
    }
}

/// GIFT is a list of image processing filters.
///
/// Go: gift.go:GIFT
#[derive(Clone, Default)]
pub struct GIFT {
    pub filters: Vec<Arc<dyn Filter>>,
    pub options: Options,
}

/// Operator is an image composition operator.
///
/// Go: gift.go:Operator
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operator {
    CopyOperator = 0,
    OverOperator = 1,
}

/// Go: gift.go:CopyOperator
pub const COPY_OPERATOR: Operator = Operator::CopyOperator;
/// Go: gift.go:OverOperator
pub const OVER_OPERATOR: Operator = Operator::OverOperator;

/// New creates a new filter list and initializes it with the given slice of
/// filters.
///
/// Go: gift.go:New
pub fn new(filters: Vec<Arc<dyn Filter>>) -> GIFT {
    GIFT {
        filters,
        options: DEFAULT_OPTIONS,
    }
}

impl GIFT {
    /// Go: gift.go:New
    pub fn new(filters: Vec<Arc<dyn Filter>>) -> GIFT {
        new(filters)
    }

    /// SetParallelization enables or disables the image processing
    /// parallelization.
    ///
    /// Go: gift.go:(*GIFT).SetParallelization
    pub fn set_parallelization(&mut self, is_enabled: bool) {
        self.options.parallelization = is_enabled;
    }

    /// Go: gift.go:(*GIFT).Parallelization
    pub fn parallelization(&self) -> bool {
        self.options.parallelization
    }

    /// Add appends the given filters to the list of filters.
    ///
    /// Go: gift.go:(*GIFT).Add
    pub fn add(&mut self, filters: impl IntoIterator<Item = Arc<dyn Filter>>) {
        self.filters.extend(filters);
    }

    /// Empty removes all the filters from the list.
    ///
    /// Go: gift.go:(*GIFT).Empty
    pub fn empty(&mut self) {
        self.filters = Vec::new();
    }

    /// Bounds calculates the appropriate bounds for the result image after
    /// applying all the added filters.
    ///
    /// Go: gift.go:(*GIFT).Bounds
    pub fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        let mut b = src_bounds;
        for f in &self.filters {
            b = f.bounds(b);
        }
        b
    }

    /// Draw applies all the added filters to the src image and outputs the
    /// result to the dst image.
    ///
    /// Go: gift.go:(*GIFT).Draw
    pub fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image) {
        if self.filters.is_empty() {
            copyimage(dst, src, Some(&self.options));
            return;
        }

        let (first, last) = (0, self.filters.len() - 1);
        let mut tmp_out: Option<NRGBA64> = None;

        for (i, f) in self.filters.iter().enumerate() {
            let prev = tmp_out.take();
            let tmp_in: &dyn Image = if i == first {
                src
            } else {
                prev.as_ref().expect("temporary image")
            };

            if i == last {
                f.draw(dst, tmp_in, Some(&self.options));
            } else {
                let mut out = create_temp_image(f.bounds(tmp_in.bounds()));
                f.draw(&mut out, tmp_in, Some(&self.options));
                tmp_out = Some(out);
            }
        }
    }

    /// DrawAt applies all the added filters to the src image and outputs the
    /// result to the dst image at the specified position pt using the
    /// specified composition operator op.
    ///
    /// Go: gift.go:(*GIFT).DrawAt
    pub fn draw_at(&self, dst: &mut dyn draw::Image, src: &dyn Image, pt: Point, op: Operator) {
        match op {
            Operator::OverOperator => {
                let mut tb = self.bounds(src.bounds());
                tb = tb.sub(tb.min).add(pt);
                let mut tmp = create_temp_image(tb);
                self.draw(&mut tmp, src);
                // pixGetterDst := newPixelGetter(dst): only its converted
                // palette is needed; the pixels are read through the setter.
                let dst_palette: Vec<Pixel> = PixelGetter::new(&*dst).palette().to_vec();
                let pix_getter_tmp = PixelGetter::new(&tmp);
                let ib = tb.intersect(dst.bounds());
                let mut pix_setter_dst = PixelSetter::new(dst);
                parallelize(
                    self.options.parallelization,
                    ib.min.y,
                    ib.max.y,
                    |start, stop| {
                        for y in start..stop {
                            for x in ib.min.x..ib.max.x {
                                let px0 = pix_setter_dst.get_pixel(&dst_palette, x, y);
                                let px1 = pix_getter_tmp.get_pixel(x, y);
                                pix_setter_dst.set_pixel(x, y, over(px0, px1));
                            }
                        }
                    },
                );
            }
            Operator::CopyOperator => {
                if pt.eq(dst.bounds().min) {
                    self.draw(dst, src);
                    return;
                }
                if self.draw_sub_image(dst, pt, src) {
                    return;
                }
                let mut tb = self.bounds(src.bounds());
                tb = tb.sub(tb.min).add(pt);
                let mut tmp = create_temp_image(tb);
                self.draw(&mut tmp, src);
                let pix_getter = PixelGetter::new(&tmp);
                let ib = tb.intersect(dst.bounds());
                let mut pix_setter = PixelSetter::new(dst);
                parallelize(
                    self.options.parallelization,
                    ib.min.y,
                    ib.max.y,
                    |start, stop| {
                        for y in start..stop {
                            for x in ib.min.x..ib.max.x {
                                pix_setter.set_pixel(x, y, pix_getter.get_pixel(x, y));
                            }
                        }
                    },
                );
            }
        }
    }

    /// `if subimg, ok := getSubImage(dst, pt); ok { g.Draw(subimg, src) }`:
    /// draws into Go's aliasing sub-image of dst (see go-image
    /// `with_sub_image_mut`) and reports whether getSubImage succeeded.
    fn draw_sub_image(&self, dst: &mut dyn draw::Image, pt: Point, src: &dyn Image) -> bool {
        // Go: gift.go:getSubImage
        if !pt.in_(dst.bounds()) {
            return false;
        }
        let r = Rectangle {
            min: pt,
            max: dst.bounds().max,
        };
        if let Some(img) = dst.downcast_mut::<Gray>() {
            img.with_sub_image_mut(r, |sub| self.draw(sub, src));
        } else if let Some(img) = dst.downcast_mut::<Gray16>() {
            img.with_sub_image_mut(r, |sub| self.draw(sub, src));
        } else if let Some(img) = dst.downcast_mut::<RGBA>() {
            img.with_sub_image_mut(r, |sub| self.draw(sub, src));
        } else if let Some(img) = dst.downcast_mut::<RGBA64>() {
            img.with_sub_image_mut(r, |sub| self.draw(sub, src));
        } else if let Some(img) = dst.downcast_mut::<NRGBA>() {
            img.with_sub_image_mut(r, |sub| self.draw(sub, src));
        } else if let Some(img) = dst.downcast_mut::<NRGBA64>() {
            img.with_sub_image_mut(r, |sub| self.draw(sub, src));
        } else {
            return false;
        }
        true
    }
}

/// The OverOperator blend of `(*GIFT).DrawAt` (gift.go:155-164), with the
/// arm64 fusion: `cs = c0 + c1` uses the unrounded `(1-c1)*px0.a`; red fuses
/// `px1.r*c1`, green and blue fuse `px0.*c0`; alpha is fused.
#[inline]
fn over(px0: Pixel, px1: Pixel) -> Pixel {
    let c1 = px1.a;
    let omc = 1.0 - c1;
    let mut c0 = omc * px0.a;
    let cs = px0.a.mul_add(omc, c1);
    c0 /= cs;
    let c1 = c1 / cs;
    let r = px1.r.mul_add(c1, px0.r * c0);
    let g = px0.g.mul_add(c0, px1.g * c1);
    let b = px0.b.mul_add(c0, px1.b * c1);
    let a = px1.a.mul_add(1.0 - px0.a, px0.a);
    Pixel::new(r, g, b, a)
}
