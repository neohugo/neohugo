//! Port of neohugo `resources/images/resampling.go`: the extra resampling
//! filters Hugo registers next to gift's (Hermite, MitchellNetravali,
//! CatmullRom, BSpline, Gaussian, Hann, Hamming, Blackman, Bartlett, Welch,
//! Cosine). They plug into gift through the [`Resampling`] trait.
//!
//! Hugo compiles its own copies of `bcspline`, `absf32` and `sinc`; their
//! machine code is identical to gift's (checked with `go tool objdump`), so
//! the gift functions are reused. The window functions' fused sites (from the
//! objdump of the golden binary): Hann `0.5+0.5*cos` and Hamming
//! `0.54+0.46*cos` are FMADDD; Blackman is `(0.42 - 0.5*cosA) + 0.08*cosB`
//! with both products fused.

use crate::gomath;
use crate::resize::{Resamp, Resampling, bcspline, sinc};

use std::f64::consts::PI;

/// Go: resampling.go:absf32
#[inline]
fn absf32(x: f32) -> f32 {
    if x < 0.0 {
        return -x;
    }
    x
}

// float32(1.0/3.0)
const ONE_THIRD: f32 = f32::from_bits(0x3eaaaaab);

fn hermite_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 1.0 {
        return bcspline(x, 0.0, 0.0);
    }
    0.0
}

fn mitchell_netravali_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 2.0 {
        return bcspline(x, ONE_THIRD, ONE_THIRD);
    }
    0.0
}

fn catmull_rom_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 2.0 {
        return bcspline(x, 0.0, 0.5);
    }
    0.0
}

fn bspline_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 2.0 {
        return bcspline(x, 1.0, 0.0);
    }
    0.0
}

fn gaussian_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 2.0 {
        return gomath::exp((-2.0 * x * x) as f64) as f32;
    }
    0.0
}

fn hann_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 3.0 {
        let c = gomath::cos(PI * x as f64 / 3.0);
        return sinc(x) * c.mul_add(0.5, 0.5) as f32;
    }
    0.0
}

fn hamming_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 3.0 {
        let c = gomath::cos(PI * x as f64 / 3.0);
        return sinc(x) * c.mul_add(0.46, 0.54) as f32;
    }
    0.0
}

fn blackman_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 3.0 {
        let s = sinc(x);
        let ca = gomath::cos(PI * x as f64 / 3.0 + PI);
        let t = (-ca).mul_add(0.5, 0.42);
        let cb = gomath::cos(2.0 * PI * x as f64 / 3.0);
        return s * cb.mul_add(0.08, t) as f32;
    }
    0.0
}

fn bartlett_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 3.0 {
        return sinc(x) * (3.0 - x) / 3.0;
    }
    0.0
}

fn welch_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 3.0 {
        return sinc(x) * (1.0 - (x * x / 9.0));
    }
    0.0
}

fn cosine_kernel(x: f32) -> f32 {
    let x = absf32(x);
    if x < 3.0 {
        return sinc(x) * gomath::cos((PI / 2.0) * (x as f64 / 3.0)) as f32;
    }
    0.0
}

/// Hermite cubic spline filter (BC-spline; B=0; C=0).
pub static HERMITE_RESAMPLING: Resamp = Resamp {
    name: "Hermite",
    support: 1.0,
    kernel: hermite_kernel,
};

/// Mitchell-Netravali cubic filter (BC-spline; B=1/3; C=1/3).
pub static MITCHELL_NETRAVALI_RESAMPLING: Resamp = Resamp {
    name: "MitchellNetravali",
    support: 2.0,
    kernel: mitchell_netravali_kernel,
};

/// Catmull-Rom - sharp cubic filter (BC-spline; B=0; C=0.5).
pub static CATMULL_ROM_RESAMPLING: Resamp = Resamp {
    name: "CatmullRomResampling",
    support: 2.0,
    kernel: catmull_rom_kernel,
};

/// BSpline is a smooth cubic filter (BC-spline; B=1; C=0).
pub static BSPLINE_RESAMPLING: Resamp = Resamp {
    name: "BSplineResampling",
    support: 2.0,
    kernel: bspline_kernel,
};

/// Gaussian blurring filter.
pub static GAUSSIAN_RESAMPLING: Resamp = Resamp {
    name: "GaussianResampling",
    support: 2.0,
    kernel: gaussian_kernel,
};

/// Hann-windowed sinc filter (3 lobes).
pub static HANN_RESAMPLING: Resamp = Resamp {
    name: "HannResampling",
    support: 3.0,
    kernel: hann_kernel,
};

/// Hamming-windowed sinc filter (3 lobes).
pub static HAMMING_RESAMPLING: Resamp = Resamp {
    name: "HammingResampling",
    support: 3.0,
    kernel: hamming_kernel,
};

/// Blackman-windowed sinc filter (3 lobes).
pub static BLACKMAN_RESAMPLING: Resamp = Resamp {
    name: "BlackmanResampling",
    support: 3.0,
    kernel: blackman_kernel,
};

/// Bartlett-windowed sinc filter (3 lobes).
pub static BARTLETT_RESAMPLING: Resamp = Resamp {
    name: "BartlettResampling",
    support: 3.0,
    kernel: bartlett_kernel,
};

/// Welch-windowed sinc filter (parabolic window, 3 lobes).
pub static WELCH_RESAMPLING: Resamp = Resamp {
    name: "WelchResampling",
    support: 3.0,
    kernel: welch_kernel,
};

/// Cosine-windowed sinc filter (3 lobes).
pub static COSINE_RESAMPLING: Resamp = Resamp {
    name: "CosineResampling",
    support: 3.0,
    kernel: cosine_kernel,
};

/// neohugo `resources/images/config.go:imageFilters`: the resampling filter
/// for a lower-cased config/spec name (`"box"`, `"lanczos"`, `"hermite"`...).
pub fn image_filter(name: &str) -> Option<&'static dyn Resampling> {
    use crate::resize::{
        BOX_RESAMPLING, LANCZOS_RESAMPLING, LINEAR_RESAMPLING, NEAREST_NEIGHBOR_RESAMPLING,
    };
    Some(match name {
        "nearestneighbor" => &NEAREST_NEIGHBOR_RESAMPLING,
        "box" => &BOX_RESAMPLING,
        "linear" => &LINEAR_RESAMPLING,
        "hermite" => &HERMITE_RESAMPLING,
        "mitchellnetravali" => &MITCHELL_NETRAVALI_RESAMPLING,
        "catmullrom" => &CATMULL_ROM_RESAMPLING,
        "bspline" => &BSPLINE_RESAMPLING,
        "gaussian" => &GAUSSIAN_RESAMPLING,
        "lanczos" => &LANCZOS_RESAMPLING,
        "hann" => &HANN_RESAMPLING,
        "hamming" => &HAMMING_RESAMPLING,
        "blackman" => &BLACKMAN_RESAMPLING,
        "bartlett" => &BARTLETT_RESAMPLING,
        "welch" => &WELCH_RESAMPLING,
        "cosine" => &COSINE_RESAMPLING,
        _ => return None,
    })
}
