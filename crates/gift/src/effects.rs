//! Port of gift v1.2.1 `effects.go`: Pixelate.

use std::sync::Arc;

use go_image::{Image, Rectangle, draw, rect};

use crate::gift::{DEFAULT_OPTIONS, Filter, Options};
use crate::pixels::{Pixel, PixelGetter, PixelSetter};
use crate::utils::{copyimage, parallelize};

/// Go: effects.go:pixelateFilter
#[derive(Clone, Copy, Debug)]
pub struct PixelateFilter {
    pub size: i64,
}

impl Filter for PixelateFilter {
    // Go: effects.go:(*pixelateFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: effects.go:(*pixelateFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let block_size = self.size;
        if block_size <= 1 {
            copyimage(dst, src, Some(options));
            return;
        }

        let srcb = src.bounds();
        let dstb = dst.bounds();

        let mut num_blocks_x = srcb.dx() / block_size;
        if srcb.dx() % block_size > 0 {
            num_blocks_x += 1;
        }
        let mut num_blocks_y = srcb.dy() / block_size;
        if srcb.dy() % block_size > 0 {
            num_blocks_y += 1;
        }

        let pix_getter = PixelGetter::new(src);
        let mut pix_setter = PixelSetter::new(dst);

        parallelize(options.parallelization, 0, num_blocks_y, |start, stop| {
            for by in start..stop {
                for bx in 0..num_blocks_x {
                    // Calculate the block bounds.
                    let bb = rect(
                        bx * block_size,
                        by * block_size,
                        (bx + 1) * block_size,
                        (by + 1) * block_size,
                    );
                    let bb_src = bb.add(srcb.min).intersect(srcb);
                    let bb_dst = bb_src.sub(srcb.min).add(dstb.min).intersect(dstb);

                    // Calculate the average color of the block.
                    let (mut r, mut g, mut b, mut a) = (0f32, 0f32, 0f32, 0f32);
                    let mut cnt = 0f32;
                    for y in bb_src.min.y..bb_src.max.y {
                        for x in bb_src.min.x..bb_src.max.x {
                            let px = pix_getter.get_pixel(x, y);
                            r += px.r;
                            g += px.g;
                            b += px.b;
                            a += px.a;
                            cnt += 1.0;
                        }
                    }
                    if cnt > 0.0 {
                        r /= cnt;
                        g /= cnt;
                        b /= cnt;
                        a /= cnt;
                    }

                    // Set the calculated color for all pixels in the block.
                    for y in bb_dst.min.y..bb_dst.max.y {
                        for x in bb_dst.min.x..bb_dst.max.x {
                            pix_setter.set_pixel(x, y, Pixel::new(r, g, b, a));
                        }
                    }
                }
            }
        });
    }
}

/// Pixelate creates a filter that applies a pixelation effect to an image.
///
/// Go: effects.go:Pixelate
pub fn pixelate(size: i64) -> Arc<dyn Filter> {
    Arc::new(PixelateFilter { size })
}
