//! Port of gift v1.2.1 `rank.go`: Median, Minimum and Maximum.

use std::sync::Arc;

use go_image::{Image, Rectangle, draw, rect};

use crate::gift::{DEFAULT_OPTIONS, Filter, Options};
use crate::pixels::{Pixel, PixelGetter, PixelSetter};
use crate::utils::{copyimage, gen_disk, is_opaque, maxf32, minf32, parallelize, sort};

/// Go: rank.go:rankMode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RankMode {
    Median = 0,
    Min = 1,
    Max = 2,
}

/// Go: rank.go:rankFilter
#[derive(Clone, Copy, Debug)]
pub struct RankFilter {
    pub ksize: i64,
    pub disk: bool,
    pub mode: RankMode,
}

impl Filter for RankFilter {
    // Go: rank.go:(*rankFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: rank.go:(*rankFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        let dstb = dst.bounds();

        if srcb.dx() <= 0 || srcb.dy() <= 0 {
            return;
        }

        let mut ksize = self.ksize;
        if ksize % 2 == 0 {
            ksize -= 1;
        }

        if ksize <= 1 {
            copyimage(dst, src, Some(options));
            return;
        }
        let kradius = ksize / 2;

        let opaque = is_opaque(src);

        let mut disk: Vec<f32> = Vec::new();
        if self.disk {
            disk = gen_disk(ksize);
        }

        let pix_getter = PixelGetter::new(src);
        let mut pix_setter = PixelSetter::new(dst);
        let kk = (ksize * ksize) as usize;

        parallelize(
            options.parallelization,
            srcb.min.y,
            srcb.max.y,
            |start, stop| {
                let mut pxbuf: Vec<Pixel> = Vec::new();

                let (mut rbuf, mut gbuf, mut bbuf, mut abuf) =
                    (Vec::new(), Vec::new(), Vec::new(), Vec::new());
                if self.mode == RankMode::Median {
                    rbuf = Vec::with_capacity(kk);
                    gbuf = Vec::with_capacity(kk);
                    bbuf = Vec::with_capacity(kk);
                    if !opaque {
                        abuf = Vec::with_capacity(kk);
                    }
                }

                for y in start..stop {
                    // Init buffer.
                    pxbuf.clear();
                    for i in srcb.min.x - kradius..=srcb.min.x + kradius {
                        for j in y - kradius..=y + kradius {
                            let (mut kx, mut ky) = (i, j);
                            if kx < srcb.min.x {
                                kx = srcb.min.x;
                            } else if kx > srcb.max.x - 1 {
                                kx = srcb.max.x - 1;
                            }
                            if ky < srcb.min.y {
                                ky = srcb.min.y;
                            } else if ky > srcb.max.y - 1 {
                                ky = srcb.max.y - 1;
                            }
                            pxbuf.push(pix_getter.get_pixel(kx, ky));
                        }
                    }

                    for x in srcb.min.x..srcb.max.x {
                        let (mut r, mut g, mut b, mut a) = (0f32, 0f32, 0f32, 0f32);
                        if self.mode == RankMode::Median {
                            rbuf.clear();
                            gbuf.clear();
                            bbuf.clear();
                            if !opaque {
                                abuf.clear();
                            }
                        } else if self.mode == RankMode::Min {
                            (r, g, b, a) = (1.0, 1.0, 1.0, 1.0);
                        } else if self.mode == RankMode::Max {
                            (r, g, b, a) = (0.0, 0.0, 0.0, 0.0);
                        }

                        let mut sz = 0usize;
                        for i in 0..ksize {
                            for j in 0..ksize {
                                if self.disk && disk[(i * ksize + j) as usize] == 0.0 {
                                    continue;
                                }

                                let px = pxbuf[(i * ksize + j) as usize];
                                if self.mode == RankMode::Median {
                                    rbuf.push(px.r);
                                    gbuf.push(px.g);
                                    bbuf.push(px.b);
                                    if !opaque {
                                        abuf.push(px.a);
                                    }
                                } else if self.mode == RankMode::Min {
                                    r = minf32(r, px.r);
                                    g = minf32(g, px.g);
                                    b = minf32(b, px.b);
                                    if !opaque {
                                        a = minf32(a, px.a);
                                    }
                                } else if self.mode == RankMode::Max {
                                    r = maxf32(r, px.r);
                                    g = maxf32(g, px.g);
                                    b = maxf32(b, px.b);
                                    if !opaque {
                                        a = maxf32(a, px.a);
                                    }
                                }
                                sz += 1;
                            }
                        }

                        if self.mode == RankMode::Median {
                            sort(&mut rbuf);
                            sort(&mut gbuf);
                            sort(&mut bbuf);
                            if !opaque {
                                sort(&mut abuf);
                            }

                            let idx = sz / 2;
                            (r, g, b) = (rbuf[idx], gbuf[idx], bbuf[idx]);
                            if !opaque {
                                a = abuf[idx];
                            }
                        }

                        if opaque {
                            a = 1.0;
                        }

                        pix_setter.set_pixel(
                            dstb.min.x + x - srcb.min.x,
                            dstb.min.y + y - srcb.min.y,
                            Pixel::new(r, g, b, a),
                        );

                        // Rotate buffer columns.
                        if x < srcb.max.x - 1 {
                            pxbuf.drain(0..ksize as usize);
                            let mut kx = x + 1 + kradius;
                            if kx > srcb.max.x - 1 {
                                kx = srcb.max.x - 1;
                            }
                            for j in y - kradius..=y + kradius {
                                let mut ky = j;
                                if ky < srcb.min.y {
                                    ky = srcb.min.y;
                                } else if ky > srcb.max.y - 1 {
                                    ky = srcb.max.y - 1;
                                }
                                pxbuf.push(pix_getter.get_pixel(kx, ky));
                            }
                        }
                    }
                }
            },
        );
    }
}

/// Median creates a median image filter.
///
/// Go: rank.go:Median
pub fn median(ksize: i64, disk: bool) -> Arc<dyn Filter> {
    Arc::new(RankFilter {
        ksize,
        disk,
        mode: RankMode::Median,
    })
}

/// Minimum creates a local minimum image filter.
///
/// Go: rank.go:Minimum
pub fn minimum(ksize: i64, disk: bool) -> Arc<dyn Filter> {
    Arc::new(RankFilter {
        ksize,
        disk,
        mode: RankMode::Min,
    })
}

/// Maximum creates a local maximum image filter.
///
/// Go: rank.go:Maximum
pub fn maximum(ksize: i64, disk: bool) -> Arc<dyn Filter> {
    Arc::new(RankFilter {
        ksize,
        disk,
        mode: RankMode::Max,
    })
}
