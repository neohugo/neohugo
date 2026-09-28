//! Port of gift v1.2.1 `utils.go`.

use std::sync::Arc;

use go_image::{Image, NRGBA64, Rectangle, draw, rect};

use crate::gift::{DEFAULT_OPTIONS, Filter, Options};
use crate::gomath;
use crate::pixels::{PixelGetter, PixelSetter};

pub use crate::pixels::{maxf32, minf32};

/// The number of parallel parts `parallelize` splits work into when
/// parallelization is enabled (Go: `runtime.GOMAXPROCS(0)`).
fn gomaxprocs() -> i64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1)
}

/// Go: utils.go:parallelize.
///
/// Go runs every part in its own goroutine. Every gift caller writes a
/// disjoint set of destination pixels per part and reads only source pixels,
/// so the result does not depend on scheduling; the port runs the parts in
/// order on the calling thread (see PORTING.md).
pub(crate) fn parallelize(enabled: bool, start: i64, stop: i64, f: impl FnMut(i64, i64)) {
    let mut procs = 1;
    if enabled {
        procs = gomaxprocs();
    }
    split_range(start, stop, procs, f);
}

/// Go: utils.go:splitRange. Go's `int` arithmetic wraps (e.g. a stop of
/// `dstb.Min.Y+h` beyond the int range, resize.go:resizeNearest), so the
/// arithmetic is `wrapping_*`; the parts are then empty ranges, as in Go.
pub(crate) fn split_range(start: i64, stop: i64, n: i64, mut f: impl FnMut(i64, i64)) {
    let count = stop.wrapping_sub(start);
    if count < 1 {
        return;
    }

    let mut n = n;
    if n < 1 {
        n = 1;
    }
    if n > count {
        n = count;
    }

    let div = count / n;
    let m = count % n;

    for i in 0..n {
        f(
            start
                .wrapping_add(i.wrapping_mul(div))
                .wrapping_add(minint(i, m)),
            start
                .wrapping_add((i + 1).wrapping_mul(div))
                .wrapping_add(minint(i + 1, m)),
        );
    }
}

/// Go's `a + b - c` on `int` (wrapping, evaluated left to right): the
/// destination coordinate `dstb.Min.X + x - srcb.Min.X` of most filters.
#[inline]
pub(crate) fn add_sub(a: i64, b: i64, c: i64) -> i64 {
    a.wrapping_add(b).wrapping_sub(c)
}

/// Go's `a + b - c - 1` on `int` (wrapping, left to right): the mirrored
/// coordinates of transform.go (`dstb.Min.X + srcb.Max.X - srcx - 1`).
#[inline]
pub(crate) fn add_sub_1(a: i64, b: i64, c: i64) -> i64 {
    a.wrapping_add(b).wrapping_sub(c).wrapping_sub(1)
}

/// Go: utils.go:absf32
#[inline]
pub fn absf32(x: f32) -> f32 {
    if x < 0.0 {
        return -x;
    }
    x
}

/// Go: utils.go:powf32
#[inline]
pub fn powf32(x: f32, y: f32) -> f32 {
    gomath::pow(x as f64, y as f64) as f32
}

/// Go: utils.go:logf32
#[inline]
pub fn logf32(x: f32) -> f32 {
    gomath::log(x as f64) as f32
}

/// Go: utils.go:expf32
#[inline]
pub fn expf32(x: f32) -> f32 {
    gomath::exp(x as f64) as f32
}

/// Go: utils.go:sincosf32
#[inline]
pub fn sincosf32(a: f32) -> (f32, f32) {
    let (sin, cos) = gomath::sincos(std::f64::consts::PI * a as f64 / 180.0);
    (sin as f32, cos as f32)
}

/// Go: utils.go:floorf32
#[inline]
pub fn floorf32(x: f32) -> f32 {
    (x as f64).floor() as f32
}

/// Go: utils.go:sqrtf32
#[inline]
pub fn sqrtf32(x: f32) -> f32 {
    (x as f64).sqrt() as f32
}

/// Go: utils.go:minint
#[inline]
pub fn minint(x: i64, y: i64) -> i64 {
    if x < y {
        return x;
    }
    y
}

/// Go: utils.go:maxint
#[inline]
pub fn maxint(x: i64, y: i64) -> i64 {
    if x > y {
        return x;
    }
    y
}

/// Go: utils.go:sort (insertion sort for n <= 20, else Hoare quicksort).
pub fn sort(data: &mut [f32]) {
    let n = data.len();

    if n < 2 {
        return;
    }

    if n <= 20 {
        for i in 1..n {
            let x = data[i];
            let mut j = i as i64 - 1;
            while j >= 0 && data[j as usize] > x {
                data[(j + 1) as usize] = data[j as usize];
                j -= 1;
            }
            data[(j + 1) as usize] = x;
        }
        return;
    }

    let mut i: i64 = 0;
    let mut j: i64 = n as i64 - 1;
    let x = data[n / 2];
    while i <= j {
        while data[i as usize] < x {
            i += 1;
        }
        while data[j as usize] > x {
            j -= 1;
        }
        if i <= j {
            data.swap(i as usize, j as usize);
            i += 1;
            j -= 1;
        }
    }
    if j > 0 {
        sort(&mut data[..(j + 1) as usize]);
    }
    if i < n as i64 - 1 {
        sort(&mut data[i as usize..]);
    }
}

/// Go: utils.go:createTempImage
#[inline]
pub(crate) fn create_temp_image(r: Rectangle) -> NRGBA64 {
    NRGBA64::new(r)
}

/// Go: utils.go:isOpaque
#[inline]
pub(crate) fn is_opaque(img: &dyn Image) -> bool {
    img.try_opaque().unwrap_or(false)
}

/// Go: utils.go:genDisk
pub(crate) fn gen_disk(ksize: i64) -> Vec<f32> {
    let mut ksize = ksize;
    if ksize % 2 == 0 {
        ksize -= 1;
    }
    if ksize < 1 {
        return Vec::new();
    }
    let mut disk = vec![0f32; (ksize * ksize) as usize];
    let kcenter = ksize / 2;
    for i in 0..ksize {
        for j in 0..ksize {
            let x = kcenter - i;
            let y = kcenter - j;
            let r = ((x * x + y * y) as f64).sqrt();
            if r <= (ksize / 2) as f64 {
                disk[(j * ksize + i) as usize] = 1.0;
            }
        }
    }
    disk
}

/// Go: utils.go:copyimage
pub(crate) fn copyimage(dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
    let options = options.unwrap_or(&DEFAULT_OPTIONS);

    let srcb = src.bounds();
    let dstb = dst.bounds();
    let pix_getter = PixelGetter::new(src);
    let mut pix_setter = PixelSetter::new(dst);

    parallelize(
        options.parallelization,
        srcb.min.y,
        srcb.max.y,
        |start, stop| {
            for srcy in start..stop {
                for srcx in srcb.min.x..srcb.max.x {
                    let dstx = add_sub(dstb.min.x, srcx, srcb.min.x);
                    let dsty = add_sub(dstb.min.y, srcy, srcb.min.y);
                    pix_setter.set_pixel(dstx, dsty, pix_getter.get_pixel(srcx, srcy));
                }
            }
        },
    );
}

/// Go: utils.go:copyimageFilter
#[derive(Clone, Copy, Debug, Default)]
pub struct CopyimageFilter;

impl Filter for CopyimageFilter {
    // Go: utils.go:(*copyimageFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: utils.go:(*copyimageFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        copyimage(dst, src, options);
    }
}

/// A `&copyimageFilter{}` as an `Arc<dyn Filter>`.
pub(crate) fn copyimage_filter() -> Arc<dyn Filter> {
    Arc::new(CopyimageFilter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use go_image::color::{self, Color};
    use go_image::{
        Alpha, Gray, Gray16, NRGBA, Paletted, RGBA, RGBA64, YCbCr, YCbCrSubsampleRatio,
    };

    // Go: utils_test.go:TestParallelize (GOMAXPROCS cannot be varied; the
    // part count is exercised through split_range below).
    #[test]
    fn test_parallelize() {
        for e in [true, false] {
            for n in [0i64, 1, 5, 10, 50, 100, 500, 1000, 5000] {
                let mut data = vec![0; n as usize];
                parallelize(e, 0, n, |start, stop| {
                    for i in start..stop {
                        data[i as usize] += 1;
                    }
                });
                assert!(data.iter().all(|&d| d == 1), "e={e} n={n}");
            }
        }
    }

    // Go: utils_test.go:TestSplitRange
    #[test]
    fn test_split_range() {
        for count in 0i64..100 {
            for procs in 0i64..100 {
                let start = -55;
                let mut parts = Vec::new();
                split_range(start, start + count, procs, |a, b| parts.push((a, b)));

                let mut want_len = procs;
                if want_len < 1 {
                    want_len = 1;
                }
                if want_len > count {
                    want_len = count;
                }
                assert_eq!(parts.len() as i64, want_len, "count={count} procs={procs}");

                let mut data = vec![0; count as usize];
                for (a, b) in parts {
                    for i in a..b {
                        data[(i - start) as usize] += 1;
                    }
                }
                assert!(data.iter().all(|&d| d == 1), "count={count} procs={procs}");
            }
        }
    }

    // Go: utils_test.go:TestTempImageCopy
    #[test]
    fn test_temp_image_copy() {
        let mut tmp1 = create_temp_image(rect(-1, -2, 1, 2));
        assert!(tmp1.bounds().eq(rect(-1, -2, 1, 2)));
        let tmp2 = create_temp_image(rect(-3, -4, 3, 4));
        assert!(tmp2.bounds().eq(rect(-3, -4, 3, 4)));
        copyimage(&mut tmp1, &tmp2, None);
    }

    // Go: utils_test.go:TestSort
    #[test]
    fn test_sort() {
        let test_data: Vec<(Vec<f32>, Vec<f32>)> = vec![
            (vec![], vec![]),
            (vec![0.1], vec![0.1]),
            (
                vec![0.4, 0.2, 0.5, -0.5, 0.3, 0.0, 0.1],
                vec![-0.5, 0.0, 0.1, 0.2, 0.3, 0.4, 0.5],
            ),
            (
                vec![-10.0, 10.0, -20.0, 20.0, -30.0, 30.0],
                vec![-30.0, -20.0, -10.0, 10.0, 20.0, 30.0],
            ),
            (
                vec![
                    0.60, 0.94, 0.66, 0.44, 0.42, 0.69, 0.07, 0.16, 0.10, 0.30, 0.52, 0.81, 0.21,
                    0.38, 0.32, 0.47, 0.28, 0.29, 0.68, 0.22, 0.20, 0.36, 0.57, 0.86, 0.29, 0.30,
                    0.75, 0.21, 0.87, 0.70,
                ],
                vec![
                    0.07, 0.10, 0.16, 0.20, 0.21, 0.21, 0.22, 0.28, 0.29, 0.29, 0.30, 0.30, 0.32,
                    0.36, 0.38, 0.42, 0.44, 0.47, 0.52, 0.57, 0.60, 0.66, 0.68, 0.69, 0.70, 0.75,
                    0.81, 0.86, 0.87, 0.94,
                ],
            ),
        ];
        for (mut a, b) in test_data {
            sort(&mut a);
            assert_eq!(a, b);
        }
    }

    // Go: utils_test.go:TestDisk
    #[test]
    fn test_disk() {
        let d3: &[f32] = &[0., 1., 0., 1., 1., 1., 0., 1., 0.];
        let d5: &[f32] = &[
            0., 0., 1., 0., 0., 0., 1., 1., 1., 0., 1., 1., 1., 1., 1., 0., 1., 1., 1., 0., 0., 0.,
            1., 0., 0.,
        ];
        let d7: &[f32] = &[
            0., 0., 0., 1., 0., 0., 0., 0., 1., 1., 1., 1., 1., 0., 0., 1., 1., 1., 1., 1., 0., 1.,
            1., 1., 1., 1., 1., 1., 0., 1., 1., 1., 1., 1., 0., 0., 1., 1., 1., 1., 1., 0., 0., 0.,
            0., 1., 0., 0., 0.,
        ];
        let test_data: &[(i64, &[f32])] = &[
            (-5, &[]),
            (0, &[]),
            (1, &[1.0]),
            (2, &[1.0]),
            (3, d3),
            (4, d3),
            (5, d5),
            (6, d5),
            (7, d7),
        ];
        for (ksize, k) in test_data {
            let disk = gen_disk(*ksize);
            assert_eq!(&disk[..], *k, "gen disk failed: {ksize}");
        }
    }

    /// Go: utils_test.go:customImage
    struct CustomImage;

    impl Image for CustomImage {
        fn color_model(&self) -> color::Model {
            color::Model::Gray
        }
        fn bounds(&self) -> Rectangle {
            Rectangle::default()
        }
        fn at(&self, _x: i64, _y: i64) -> Color {
            Color::Gray(color::Gray { y: 0 })
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }
    }

    // Go: utils_test.go:TestIsOpaque
    #[test]
    fn test_is_opaque() {
        let r = rect(0, 0, 1, 1);
        let black = Color::NRGBA(color::NRGBA {
            r: 0,
            g: 0,
            b: 0,
            a: 0xff,
        });
        let mut img1 = NRGBA::new(r);
        img1.set(0, 0, black);
        let mut img2 = NRGBA64::new(r);
        img2.set(0, 0, black);
        let mut img3 = RGBA::new(r);
        img3.set(0, 0, black);
        let mut img4 = RGBA64::new(r);
        img4.set(0, 0, black);
        let mut imgp1 = Paletted::new(r, vec![black].into());
        imgp1.set_color_index(0, 0, 0);
        let mut imgp2 = Paletted::new(
            r,
            vec![Color::NRGBA(color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 0xfe,
            })]
            .into(),
        );
        imgp2.set_color_index(0, 0, 0);
        let test_data: Vec<(Box<dyn Image>, bool)> = vec![
            (Box::new(CustomImage), false),
            (Box::new(NRGBA::new(r)), false),
            (Box::new(NRGBA64::new(r)), false),
            (Box::new(RGBA::new(r)), false),
            (Box::new(RGBA64::new(r)), false),
            (Box::new(Gray::new(r)), true),
            (Box::new(Gray16::new(r)), true),
            (Box::new(YCbCr::new(r, YCbCrSubsampleRatio::Ratio444)), true),
            (Box::new(Alpha::new(r)), false),
            (Box::new(img1), true),
            (Box::new(img2), true),
            (Box::new(img3), true),
            (Box::new(img4), true),
            (Box::new(imgp1), true),
            (Box::new(imgp2), false),
        ];
        for (i, (img, opaque)) in test_data.iter().enumerate() {
            assert_eq!(is_opaque(img.as_ref()), *opaque, "case {i}");
        }
    }
}
