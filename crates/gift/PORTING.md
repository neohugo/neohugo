# gift — porting notes

Byte-exact port of `github.com/disintegration/gift` v1.2.1 (every filter,
`GIFT.Draw`/`DrawAt`, the pixel getters and setters) plus the eleven extra
resampling filters of neohugo `resources/images/resampling.go` and the
resampling-name map of `resources/images/config.go`. The floating-point
results are those of the Go code as compiled by **go1.27.1 for arm64** (the
golden build ran on darwin/arm64): every `x*y ± z` that the arm64 compiler
fuses into an FMA instruction is written with `mul_add`, every other
operation is plain IEEE arithmetic, and the Go `math` functions gift calls
are ported with their own arm64 code (`src/gomath.rs`).

Hugo's own `gift.Filter` implementations (`resources/images/filters.go`,
`overlay.go`, `mask.go`, `opacity.go`, `padding.go`, `text.go`, `dither.go`,
`auto_orient.go`, `process.go`) are not in this crate. They belong to
nh-images (WAVE_B_PLAN task T10) and are built on this crate's `Filter`
trait, `GIFT::draw` and `GIFT::draw_at`. The tests carry a copy of
`overlay.go:overlayFilter` (`tests/common/mod.rs:OverlayFilter`) because the
watermark overlay is the `DrawAt(OverOperator)` path of the golden build.

## Go file → Rust module map

| Go file | Rust module |
|---|---|
| `gift.go` (Filter, Options, GIFT, New, Bounds, Draw, DrawAt, Operator, getSubImage) | `src/gift.rs` |
| `pixels.go` (pixel, pixelGetter/getPixel/Row/Column, pixelFromColor, convertPalette, getPaletteIndex, f32u8, f32u16, clampi32, pixelSetter/setPixel/Row/Column) | `src/pixels.rs` |
| `resize.go` (Resampling, bcspline, sinc, resamp, the five filters, prepareResampWeights, resizeLine, resizeHorizontal/Vertical/Nearest, Resize, ResizeToFit, ResizeToFill) | `src/resize.rs` |
| `colors.go` (prepareLut, getFromLut, colorchanFilter, Invert, ColorspaceSRGBToLinear/LinearToSRGB, Gamma, Sigmoid, Contrast, Brightness, colorFilter, Grayscale, Sepia, convertHSLToRGB, convertRGBToHSL, normalizeHue, Hue, Saturation, Colorize, ColorBalance, Threshold, ColorFunc) | `src/colors.rs` |
| `convolution.go` (prepareConvolutionWeights(1d), Convolution, convolveLine, convolve1dh/v, GaussianBlur, UnsharpMask, Mean, Sobel) | `src/convolution.rs` |
| `transform.go` (transformFilter + Rotate90…Transverse, Interpolation, rotatePoint, calcRotatedSize, Rotate, interpolateCubic/Linear/Nearest, Crop, Anchor, anchorPt, CropToSize) | `src/transform.rs` |
| `rank.go` (Median, Minimum, Maximum) | `src/rank.rs` |
| `effects.go` (Pixelate) | `src/effects.rs` |
| `utils.go` (parallelize, splitRange, absf32, minf32, maxf32, powf32, logf32, expf32, sincosf32, floorf32, sqrtf32, minint, maxint, sort, createTempImage, isOpaque, genDisk, copyimage, copyimageFilter) | `src/utils.rs` (minf32/maxf32 in `src/pixels.rs`) |
| neohugo `resources/images/resampling.go` (Hermite … Cosine; its own copies of resamp, bcspline, absf32, sinc) + `config.go:imageFilters` | `src/hugo_resampling.rs` (`image_filter(name)`) |
| go1.27.1 `math`: `exp_arm64.s` (archExp), `log.go`, `pow.go`, `sin.go`, `sincos.go`, `trig_reduce.go`, `frexp.go`, `ldexp.go`, `modf.go`, `bits.go`, `dim_arm64.s` (archMax) | `src/gomath.rs` |
| `*_test.go` (all 61 test functions) | `src/go_tests/*_test.rs`, `src/utils.rs` (utils_test.go), `tests/go_golden.rs` (TestGolden) |

Every ported function carries a `// Go: <file>:<Func>` comment. Hugo's
`bcspline`, `absf32` and `sinc` are not duplicated: their arm64 machine code
is identical to gift's (checked below), so `hugo_resampling` calls gift's.

## Public API

Go `int` is `i64`. Constructors keep Go's names in snake_case and return
`Arc<dyn Filter>`: `resize`, `resize_to_fit`, `resize_to_fill`, `crop`,
`crop_to_size`, `rotate`, `rotate90`, `rotate180`, `rotate270`,
`flip_horizontal`, `flip_vertical`, `transpose`, `transverse`, `invert`,
`colorspace_srgb_to_linear`, `colorspace_linear_to_srgb`, `gamma`,
`sigmoid_filter` (Go `Sigmoid`), `contrast`, `brightness`, `grayscale`,
`sepia`, `hue`, `saturation`, `colorize`, `color_balance`, `threshold`,
`color_func`, `convolution`, `gaussian_blur`, `unsharp_mask`, `mean`,
`sobel`, `median`, `minimum`, `maximum`, `pixelate`.

* `trait Filter: Any + Send + Sync { fn draw(&self, dst: &mut dyn draw::Image,
  src: &dyn Image, options: Option<&Options>); fn bounds(&self, Rectangle) ->
  Rectangle }` (`None` = Go's nil `*Options`). The concrete filter structs
  (`ResizeFilter`, `ColorFilter`, …) are public and can be downcast.
* `GIFT { filters: Vec<Arc<dyn Filter>>, options: Options }`, `new(filters)`,
  `bounds`, `draw`, `draw_at(dst, src, pt, op)`, `add`, `empty`,
  `set_parallelization`, `parallelization`; `Options { parallelization }`,
  `DEFAULT_OPTIONS`; `Operator::{CopyOperator, OverOperator}`
  (`COPY_OPERATOR`, `OVER_OPERATOR`).
* `trait Resampling { support, kernel }`, `Resamp { name, support, kernel }`
  (`Display` = Go `String()`), the statics `NEAREST_NEIGHBOR_RESAMPLING`,
  `BOX_RESAMPLING`, `LINEAR_RESAMPLING`, `CUBIC_RESAMPLING`,
  `LANCZOS_RESAMPLING`, `bcspline`, `sinc`; `hugo_resampling::*_RESAMPLING`
  and `hugo_resampling::image_filter("box" | "lanczos" | "hermite" | …)`.
* `Anchor(i64)` and `Interpolation(i64)` are newtypes over Go's `int`
  (unknown values behave like `CenterAnchor` / `NearestNeighborInterpolation`,
  as in Go), with the `*_ANCHOR` / `*_INTERPOLATION` constants.
* `gomath::{exp, log, pow, sin, cos, sincos, frexp, ldexp, modf, max, …}`.

## FMA sites (go1.27.1, arm64)

**How they were verified (re-done for this document).** The oracle was
cross-compiled for arm64 on linux/amd64 and disassembled:

```sh
# zig 0.16 (pip download ziglang) is the C cross-compiler for the cgo deps
printf '#!/bin/sh\nexec /path/to/zig cc -target aarch64-linux-musl "$@"\n' > zcc; chmod +x zcc
GOTOOLCHAIN=go1.27.1 GOARCH=arm64 CGO_ENABLED=1 CC=$PWD/zcc \
  go build -ldflags '-linkmode external -extldflags -static' -o oracle_arm64 ./tools/go-oracle/gift
go tool objdump -s 'github.com/disintegration/gift' oracle_arm64   # 119 fused instructions
go tool objdump -s 'neohugo/resources/images\.' oracle_arm64      #  15
go tool objdump -s '^math\.' oracle_arm64                          #  68 (67 reachable from gift)
```

The fusion rules are per architecture, not per OS (the linux/arm64 oracle
reproduces every checked-in darwin/arm64 fixture exactly, see
Verification). For every site the operands were read from the disassembly
(Go's ARM64 syntax: `FMADDS Fm, Fa, Fn, Fd` is `Fd = Fa + Fn*Fm`, `FMSUBS` is
`Fa - Fn*Fm`, `FNMSUBS` is `Fn*Fm - Fa`), with the constant loads resolved
through the binary's `$f32.*`/`$f64.*` symbols, to see *which* product of a
sum is fused: that choice differs between sites that look alike (Grayscale
vs. the Gray setter, Sobel's red vs. green/blue, DrawAt's red vs.
green/blue, rotatePoint's four inlined copies). How many of these sites
the tests alone would catch is measured under "Mutation testing" below.

### gift (119 instructions)

| Go site | arm64 | fused expression (Rust) | Rust location |
|---|---|---|---|
| `pixels.go:152,154,156` getPaletteIndex `dcur += d*d` | 3 × FMADDS | `d.mul_add(d, dcur)` (the first `d*d` is rounded) | `pixels.rs:get_palette_index` |
| `pixels.go:296` `f32u8(v*0xff)` inlined in setPixel: `int64(v*255 + 0.5)` | FMADDS: NRGBA ×4, RGBA ×4, Gray ×1 | `v.mul_add(255, 0.5)` | `pixels.rs:f32u8_mul` |
| `pixels.go:307` `f32u16(v*0xffff)` | FMADDS: NRGBA64 ×4, RGBA64 ×4, generic ×4, Gray16 ×1 | `v.mul_add(65535, 0.5)` | `pixels.rs:f32u16_mul` |
| `pixels.go:430-435 / 438-442` RGBA/RGBA64 setters | (the `f32u8`/`f32u16` rows above) | `fa := px.a*255` is rounded and used for r,g,b (`px.c.mul_add(fa, 0.5)`); the alpha is `px.a.mul_add(255, 0.5)`, fused from `px.a`, not from `fa` | `pixels.rs:PixelSetter::set_pixel` |
| `pixels.go:455, 459` Gray/Gray16 setter `(0.299*r + 0.587*g + 0.114*b) * a` | 2 × FMADDS each | `b.mul_add(0.114, r.mul_add(0.299, 0.587*g)) * a` (green rounded) | `pixels.rs:gray_luma` |
| `colors.go:20` getFromLut `u*float32(len-1) + 0.5` (inlined, r,g,b) | 3 × FMADDS | `u.mul_add(n-1, 0.5)` | `colors.rs:get_from_lut` |
| `colors.go:114` LinearToSRGB `1.055*math.Pow(x, 1/2.4) - 0.055` | FNMSUBD | `pow.mul_add(1.055, -0.055)` (f64) | `colors.rs:colorspace_linear_to_srgb` |
| `colors.go:166` Sigmoid (factor < 0) `(sig1-sig0)*x + sig0` | FMADDS | `(sig1-sig0).mul_add(x, sig0)` | `colors.rs:sigmoid_filter` |
| `colors.go:187, 189` Contrast `0.5 + (x-0.5)*p`, `0.5 + (x-0.5)*(1/(2-p))` | 2 × FMADDS | `(x-0.5).mul_add(p, 0.5)` | `colors.rs:contrast` |
| `colors.go:252` Grayscale `0.299*r + 0.587*g + 0.114*b` | 2 × FMADDS | `b.mul_add(0.114, g.mul_add(0.587, 0.299*r))` (red rounded — not the setter's order) | `colors.rs:luma` |
| `colors.go:475` Threshold (same expression) | 2 × FMADDS | same as Grayscale | `colors.rs:luma` |
| `colors.go:271, 275, 279` Sepia `1 - 0.607*adj` etc. | 3 × FMSUBS | `(-adj).mul_add(0.607, 1)` | `colors.rs:sepia` |
| `colors.go:282-284` Sepia matrix `r*rr + g*rg + b*rb` | 6 × FMADDS | `b.mul_add(rb, r.mul_add(rr, g*rg))` (green rounded) | `colors.rs:sepia` |
| `colors.go:303` hueToRGB `p + (q-p)*6*t` (inlined 3×) | 3 × FMADDS | `((q-p)*6).mul_add(t, p)` | `colors.rs:convert_hsl_to_rgb` |
| `colors.go:309` hueToRGB `p + (q-p)*(2/3.0-t)*6` (3×) | 3 × FMADDS | `((q-p)*(2/3-t)).mul_add(6, p)` | `colors.rs:convert_hsl_to_rgb` |
| `colors.go:318` `q = l + s - l*s` | FMSUBS | `(-l).mul_add(s, l+s)` | `colors.rs:convert_hsl_to_rgb` |
| `colors.go:435-437` Colorize `px.c += (c - px.c)*p` | 3 × FMADDS | `(c-px.c).mul_add(p, px.c)` | `colors.rs:colorize` |
| `convolution.go:139-143` convolutionFilter `r += px.r*w.weight` (+a if alpha) | 4 × FMADDS | `px.c.mul_add(w, acc)` | `convolution.rs:ConvolutionFilter::draw` |
| `convolution.go:259-263` convolveLine | 4 × FMADDS | `wa := c.a*w` rounded; `c.r.mul_add(wa, r)` ×3; `a = c.a.mul_add(w, a)` from the *unrounded* `c.a*w` | `convolution.rs:convolve_line` |
| `convolution.go:406-408` unsharp `orig + dif` | 4 × FMADDS (r,g,b,a) | `dif` rounded for the threshold test; the result is `(orig-blurred).mul_add(amount, orig)` | `convolution.rs:unsharp` |
| `convolution.go:563-565` Sobel `sqrt(h*h + v*v)` | 3 × FMADDS | red `v.r.mul_add(v.r, h.r*h.r)`; green/blue `h.mul_add(h, v*v)` | `convolution.rs:HvConvolutionFilter::draw` |
| `gift.go:156-157` DrawAt Over `c0 := (1-c1)*px0.a; cs := c0 + c1` | FMADDS | `c0` rounded (then divided); `cs = px0.a.mul_add(1-c1, c1)` | `gift.rs:over` |
| `gift.go:160` `r := px0.r*c0 + px1.r*c1` | FMADDS | `px1.r.mul_add(c1, px0.r*c0)` | `gift.rs:over` |
| `gift.go:161-162` `g`, `b` (same shape) | 2 × FMADDS | `px0.g.mul_add(c0, px1.g*c1)` — the *other* product | `gift.rs:over` |
| `gift.go:163` `a := px0.a + px1.a*(1-px0.a)` | FMADDS | `px1.a.mul_add(1-px0.a, px0.a)` | `gift.rs:over` |
| `resize.go:20` bcspline, `x < 1` | 2 × FMSUBS, 3 × FMADDS | `k3 = (-c).mul_add(6, (-b).mul_add(9, 12))`, `k2 = c.mul_add(6, b.mul_add(12, -18))`, `(k3*x*x).mul_add(x, k2*x*x)` | `resize.rs:bcspline` |
| `resize.go:23` bcspline, `1 ≤ x < 2` | 2 × FMSUBS, 4 × FMADDS | `k3 = (-c).mul_add(6, -b)`, `k2 = b.mul_add(6, 30c)`, `s1 = t3.mul_add(x, t2)`, `k1 = (-c).mul_add(48, -12b)`, `s2 = k1.mul_add(x, s1)`, `k0 = b.mul_add(8, 24c)` | `resize.rs:bcspline` |
| `resize.go:85` `center := (float32(i)+0.5)*delta - 0.5` | FNMSUBS | `(i+0.5).mul_add(delta, -0.5)` | `resize.rs:prepare_resamp_weights` |
| `resize.go:125-129` resizeLine | 4 × FMADDS | as convolveLine | `resize.rs:resize_line` |
| `transform.go:150-151` `float32(w)/2 - 0.5` | 2 × FNMSUBS | `w.mul_add(0.5, -0.5)` (exact either way) | `transform.rs:calc_rotated_size` |
| `transform.go:140-141` rotatePoint inlined 4× in calcRotatedSize | 4 × FMSUBS, 4 × FMADDS | `newx = (-y).mul_add(asin, x*acos)` for all four; `newy = x.mul_add(asin, y*acos)` for points 1-2, `y.mul_add(acos, x*asin)` for points 3-4 | `transform.rs:calc_rotated_size` |
| `transform.go:200-203` rotateFilter offsets `float32(n)/2 - 0.5` | 4 × FNMSUBS | `n.mul_add(0.5, -0.5)` | `transform.rs:RotateFilter::draw` |
| `transform.go:140-141` rotatePoint inlined in rotateFilter.Draw.func1 | FMSUBS + FMADDS | `xf = (-yy).mul_add(asin, xx*acos)`, `yf = yy.mul_add(acos, xx*asin)` | `transform.rs:RotateFilter::draw` |
| `transform.go:280-284` interpolateCubic accumulation | 4 × FMADDS | as resizeLine (`cfs` products are plain multiplications) | `transform.rs:interpolate_cubic` |
| `transform.go:324-328` interpolateLinear accumulation | 4 × FMADDS | as resizeLine | `transform.rs:interpolate_linear` |

Checked and **not** fused (explicit conversion, division or no add):
`gaussianBlurKernel` (float64 of rounded float32 products), `sum += 2*f`
(compiled as `f+f`), `resizeNearest`'s `(x+0.5)*dx`, the Resize/ResizeToFit/
ResizeToFill bounds arithmetic (float64, add after a division), the
`interpolateCubic`/`Linear` coefficient products, `sigmoid`, ColorBalance,
Pixelate, convertRGBToHSL (additions after divisions),
`prepareResampWeights`'s `radius` (`FMULS` + `FRINTPS`) and weight sums.

### neohugo `resources/images/resampling.go` (15 instructions)

| Go site | arm64 | Rust (`src/hugo_resampling.rs`) |
|---|---|---|
| `:99` Hann `0.5 + 0.5*math.Cos(…)` | FMADDD | `cos.mul_add(0.5, 0.5)` |
| `:111` Hamming `0.54 + 0.46*math.Cos(…)` | FMADDD | `cos.mul_add(0.46, 0.54)` |
| `:124` Blackman `0.42 - 0.5*cos(A) + 0.08*cos(B)` | FMSUBD, FMADDD | `cos_b.mul_add(0.08, (-cos_a).mul_add(0.5, 0.42))` |
| `:194, :197` Hugo's own `bcspline` copy | 11 (as gift's) | calls `resize::bcspline`: the float instruction sequences of the two copies are identical (compared after resolving constants) |

Gaussian (`math.Exp(float64(-2*x*x))`), Bartlett, Welch and Cosine have no
fused site.

### Go `math` (68 instructions; `src/gomath.rs`)

| Go site | arm64 | Rust |
|---|---|---|
| `exp_arm64.s:46-68` archExp (hand-written assembly) | FNMSUBD, FMADDD, FMSUBD, 4 × FMADDD, FMSUBD | `gomath::exp`, instruction by instruction |
| `log.go:124-128` `t1`, `t2`, `R`, `hfsq`, the final combination | 10 | `gomath::log` |
| `pow.go:153` `x1 += x1` right after `x1 *= x1` | FMADDD | `gomath::pow`: `x1 = x0.mul_add(x0, x1)` |
| `sin.go:155` (cos) `z = ((x - y*PI4A) - y*PI4B) - y*PI4C` | 3 × FMSUBD | `gomath::reduce_small` |
| `sin.go:168, 170` (cos) the two polynomials | 13 | `gomath::sin_poly`, `cos_poly` |
| `sin.go:227, 236, 238` (sin) | 16 | `gomath::sin` |
| `sincos.go:50, 61, 62` | 16 | `gomath::sincos` |
| `log10.go:36` log2 | FMADDD | not reachable from gift (not ported) |

`trigReduce`, `Frexp`, `Ldexp`, `Modf`, `archMax` have no fused site;
`Sqrt`, `Floor`, `Ceil`, `Trunc` are exact intrinsics. On **amd64** Go uses
assembly for `Exp` and `Log` (`exp_amd64.s`, `log_amd64.s`) besides not
fusing, so amd64 differs in the math functions too.

## Deliberate deviations (none changes output bytes)

1. **Parallelism.** `parallelize` splits the range exactly like Go
   (`GOMAXPROCS` parts via `splitRange`) but runs the parts in order on the
   calling thread. Every gift caller writes a disjoint set of destination
   pixels per part and, except `DrawAt(OverOperator)`, reads only the
   source; the Over blend reads and writes the same destination pixel, never
   another part's. The result therefore cannot depend on scheduling (the
   first session also reproduced the golden output with `GOMAXPROCS=1`).
2. **Getter/setter aliasing.** Go keeps a pixel getter and a pixel setter on
   the same destination in `DrawAt(OverOperator)`; the port reads the
   destination through the setter (`PixelSetter::get_pixel`) with the
   getter's converted palette.
3. **Sub-images.** Go's `getSubImage` returns a sub-image aliasing the
   destination's `Pix`. `gift::get_sub_image(img, pt, f)` runs `f` on that
   sub-image through go-image's `with_sub_image_mut` (the writes land in the
   parent) and returns whether Go's `getSubImage` succeeds. Drawing an image
   onto itself (`dst == src`) cannot be expressed in safe Rust.
4. **Default options.** `Options::default()` and `GIFT::default()` use Go's
   `defaultOptions` (`Parallelization: true`); Go's zero value `GIFT{}` has
   `false`. Construct `Options { parallelization: false }` for the zero value
   (the TestGIFT port does). Parallelization has no effect on output (1).
5. **Trait objects.** Filters are `Arc<dyn Filter>` (shared, immutable);
   `Resampling` is `&'static dyn Resampling`; `draw` takes
   `Option<&Options>` (`None` is nil). `convolve_1dh`/`convolve_1dv` take
   `&Options` (Go would dereference a nil pointer for a non-empty kernel).
6. **ColorFunc callbacks** are compiled by rustc, which never fuses. A port
   of a Go callback must add `mul_add` wherever the arm64 Go compiler fuses
   (none of the oracle's or the Go tests' callbacks has a fusable site).
7. **Integer conversions.** `f32 as i64` saturates and maps NaN to 0, which
   is arm64's `FCVTZS` (Go on amd64 returns `0x8000000000000000` instead;
   for the setters' clamping the result is the same, e.g. the NaN pixels of
   a transparent-over-transparent Over blend).
8. **Panics are kept.** Where Go panics (a LUT index out of range for
   channel values > 1, i.e. invalid premultiplied RGBA/RGBA64/Paletted input
   through a LUT filter; out-of-range palette indexes), the port panics too.

## Tests and verification

`cargo test` (offline, no Go): 76 tests.

| test | what |
|---|---|
| `src/go_tests/*_test.rs` (54) + `src/utils.rs` (6) | ports of all 61 Go test functions of gift v1.2.1 except TestGolden: tables converted mechanically from the Go sources (same order, same rows), loops by hand |
| `tests/go_golden.rs` | TestGolden: 28 filters on `testdata/src.png` vs. the amd64-generated `testdata/dst_*.png` with Go's non-amd64 tolerance (±1), plus the exact arm64 result (bytes differing from each golden and SHA-256), see below |
| `tests/synth.rs` | `synth 8000 1`: random 0–3-filter chains (every filter, every resampling kernel incl. Hugo's, both operators of DrawAt) over generated images of 18 kinds (every standard type, all six YCbCr ratios, NYCbCrA 4:4:4/4:2:0, paletted; invalid premultiplied colours, odd and negative origins) into the 10 settable destination types |
| `tests/real.rs` | 1152 ops on 9 real images (crops of seeksnack JPEGs with 4:4:4/4:2:0/4:2:2 chroma, the NRGBA watermark, RGB/paletted/RGBA PNGs, gift's `src.png`): the Hugo pipeline (box resizes, same-size copy, watermark overlays), two resizes per resampling filter, ResizeToFill/Fit and every other gift filter |
| `tests/setter.rs` | every setter over 65536 adversarial float32 values (rounding boundaries ±4 ulps, out of range, NaN, ±Inf, denormals) + large DrawAt Over cases over every destination type |
| `tests/math.rs`, `tests/digests.rs` (`math_digest_fixture`) | `gomath` against Go: 20000 vectors of Exp/Log/Pow/Sin/Cos/Sincos (specials, random bits, ranges) + dense sweeps (92 FNV digests of up to 2^20 results each) |
| `tests/kernels.rs`, `digests.rs` (`kernel_digest_fixture`) | all 16 kernels over 20000 inputs + every 64th float32 in [0, 4.5] (and a negative sweep) |
| `digests.rs` (`weights`, `rotbounds`) | 1-row/1-column resizes over 3901 size pairs × 16 kernels; Rotate bounds over 2221 angles × ~1400 sizes |
| `tests/site.rs` (`GIFT_SITE_DIR`) | the full seeksnack corpus, out of the repo (below) |
| `*_big` (`GIFT_SYNTH_BIG`, `GIFT_MATHDIGEST_BIG`, `GIFT_KERNELDIGEST_BIG`) | larger oracle runs, out of the repo |

**Go unit tests and FMA.** gift's own tests were run with go1.27.1 on
linux/amd64 and, built with `GOARCH=arm64 go test -c`, on linux/arm64 under
`qemu-aarch64-static`: all pass on both. So every exact-pixel table in
them is platform-independent (the paths go through fused sites — every
setter is fused — but the expected bytes are the same with and without
fusion) and the Rust port must, and does, match them exactly. The only
FMA-dependent Go test is TestGolden, whose goldens were generated on amd64
(Go allows ±1 elsewhere). Against them, go1.27.1 on arm64 differs in 4 of
28 images — contrast_increase (234 bytes), hue_rotate (34),
saturation_decrease (57), saturation_increase (52), each by 1 — and amd64
in none. The port reproduces the arm64 counts and the SHA-256 of all 28
arm64 results (taken from the qemu run of a scratch copy of TestGolden).

**linux/arm64 = darwin/arm64.** Every checked-in fixture came from
darwin/arm64. Regenerated in this session with the linux/arm64 oracle under
qemu, all are identical: synth 8000/8000, math 20000/20000, kernels
20016/20016, setter 60/60, real 1152/1152 (from the dumps, `realops`),
weights 17/17 and kerneldigest 288/288 and mathdigest 92/92 and rotbounds
3/3 chunk digests. The native amd64 oracle differs from them in 1569 synth
cases, 2068 math rows, 4033 kernel rows, 23 setter cases, 131 real ops, 15
weights, 3 rotbounds, 11 kerneldigest and 58 mathdigest chunks — which is
why float-sensitive fixtures must never be generated by amd64 Go.

**Out-of-repo runs** (all 0 mismatches):

* the seeksnack corpus (first session, darwin/arm64 oracle `site`): all
  603 JPEG/PNG images under `content/` and `assets/`, decoded by Go and
  dumped raw (~290 MB), with the Hugo pipeline ops (box resizes to
  600x480, 300x240, 600x200, 128x128, 32x32, same-size copies, watermark
  overlays at the original size and after 600x480/300x240 resizes) plus two
  resizes per resampling filter, and the full filter table for every 8th
  image; together with the 1461-file reproduction in
  `docs/rust-port/specs/images.md` (11 of which depend on FMA);
* from the linux/arm64 oracle under qemu (this session): `synth 300000
  12345` (300,000 random filter chains, `GIFT_SYNTH_BIG`), `kerneldigest 3`
  (every 3rd float32 in [0, 4.5] through all 16 kernels: 5856 chunk
  digests, `GIFT_KERNELDIGEST_BIG`) and `mathdigest 64` (1412 chunk
  digests of the Exp/Log/Pow/Sin/Cos/Sincos sweeps, `GIFT_MATHDIGEST_BIG`).

### Mutation testing of the FMA sites

Each of the 125 `mul_add` calls in `src/` (90 in gift + Hugo's kernels, 35
in `gomath`) was replaced, one at a time, by the unfused `x*y + z`, and the
checked-in suite was run (then, for survivors, the out-of-repo arm64 runs).
The disassembly above is the evidence that every site is fused; this
measures how much of it the tests would catch on their own.

* **77 killed by the checked-in tests**, among them everything on the
  golden build's path except the setters' `+ 0.5` (below): resizeLine, the
  resample `center`, DrawAt Over (all five), and further the Gray setter
  luma, rotatePoint, the interpolations, convolution, unsharp, Sepia, HSL,
  Sigmoid, Colorize r/g, bcspline's `x` terms and its `-b-6c`, the kernels'
  Blackman `0.08*cos` term, exp's outer terms, log's `R`/`hfsq`
  combination, the sin/cos outer terms and `y*PI4C`.
* **9 more killed only by the out-of-repo arm64 runs.** synth300k:
  getPaletteIndex's first add (10 of 300,000 cases), the `f32u8` setter
  rounding (2 cases; it matters only for products within ~2^-25 below 0.5,
  where the unfused sum rounds up to 1), Contrast for 0 ≤ p ≤ 1 (23),
  Grayscale/Threshold's blue term (16), Colorize's blue channel (30),
  Sobel green/blue (9, 8). mathdigest64: log's outer `t1` term and the
  third sin coefficient term. The synth lines that kill them exist
  (from the linux/arm64 oracle); they are not checked in because new
  fixtures must be platform-independent and these differ on amd64 by
  construction.
* **21 equivalent (no input can tell them apart):** the six
  `float32(n)/2 - 0.5` offsets and Hann's `0.5*cos + 0.5` / Blackman's
  `0.5*cos` (exact products); bcspline's seven coefficient sites (for the
  four (B, C) pairs gift and Hugo use, the unfused coefficient rounds to the
  same float32 — any change would alter every kernel value);
  `x - k*Ln2Hi` in exp and `k*Ln2Hi - …` in log, `y*PI4A` and `y*PI4B` in
  the sin/cos reduction (constants with trailing zero bits: exact
  products), cos's `1 - 0.5*zz`, pow's `x1 += x1` (`fma(x, x, RN(x²)) =
  2·RN(x²)`).
* **18 not observed.** The `f32u16` setter and getFromLut's index (the same
  0/1 boundary as `f32u8`, never hit; the first session's experiment found
  that the setter fusion changes none of the 1461 golden images either); ColorspaceLinearToSRGB's float64
  `FNMSUBD` (checked exhaustively: 2 of the 1.14e9 positive float32 inputs
  above 0.0031308 give a different float32, both > 2e9, outside any
  channel range); Hamming's window (exhaustively, 2 of the 1.08e9 float32
  in [0, 3) change the window by one ulp); exp's `Log2e*x ± 0.5` and the
  inner polynomial terms of exp (3), log (4), sin (2) and cos (3), whose
  rounding differences are absorbed by the outer terms in all 1.48e9
  mathdigest64 values.

### Regenerating fixtures

Float-sensitive fixtures come from an **arm64** oracle. On an amd64
machine, build it as above (zig + `-extldflags -static`) and run it under
`qemu-aarch64-static`; linux/arm64 reproduces the darwin/arm64 fixtures
(previous section). From the repository root, with `O="qemu-aarch64-static
oracle_arm64"` and `F=crates/gift/tests/fixtures`:

```sh
$O synth 8000 1 | gzip -9 > $F/synth.tsv.gz
$O math 20000 | gzip -9 > $F/math.tsv.gz
$O kernels 20000 | gzip -9 > $F/kernels.tsv.gz
$O setter | gzip -9 > $F/setter.tsv.gz
$O weights > $F/weights.tsv
$O rotbounds > $F/rotbounds.tsv
$O kerneldigest 64 > $F/kerneldigest.tsv
$O mathdigest 4 > $F/mathdigest.tsv
$O realops $F/real | gzip -9 > $F/real.tsv.gz        # from the checked-in dumps
$O realfix <seeksnack> <gift>/testdata $F/real | gzip -9 > $F/real.tsv.gz   # needs the private site
$O site <seeksnack> <outDir>                          # then GIFT_SITE_DIR=<outDir>
```

Platform-independent (integer-only decoding; any architecture):

```sh
go run ./tools/go-oracle/gift gotestdata "$(go env GOMODCACHE)/github.com/disintegration/gift@v1.2.1/testdata" $F/gotestdata
```

`gotestdata/dst_*.gz` were generated on amd64 and on arm64 (qemu) with
identical content (and identical gzip bytes). The gzip bytes of a dump are
not guaranteed to be platform-independent (compress/flate chooses block
types with float estimates), only their content; the Rust reader
decompresses. `gotestdata/LICENSE-gift` is gift's MIT license (the goldens
are its test data); `src.png` is `real/giftsrc.gz`.

## Changes in this pass

* Ported all of gift's Go tests (`src/go_tests/`, `tests/go_golden.rs`);
  `get_sub_image` split out of `DrawAt` so TestSubImage can test it;
  `prepare_lut`/`get_from_lut` and the getter/setter image references made
  `pub(crate)` for the white-box tests.
* Oracle: the synth generator picked `colorfunc(r.intn(len(colorFuncs)))`;
  after extra.go appended a fourth callback, `synth 8000 1` no longer
  reproduced the checked-in `synth.tsv` (the colorfunc rows changed). It now
  draws from the three synth callbacks (`numSynthColorFuncs`), and
  regenerates the fixture exactly. Added `loadDump` (the Rust reader's
  counterpart, referenced by `tests/common/mod.rs` but missing), `realops`
  and `gotestdata`.
* No output bug was found in the port: all Go tests passed on the first
  run, and so did the arm64 synth300k, kerneldigest 3 and mathdigest 64
  runs. Added the FMA mutation testing and the exhaustive float32 checks
  above.
* Two clippy findings (`needless_late_init` in convertRGBToHSL,
  `redundant_closure` in parallelize) fixed without behaviour change.

## Known gaps

* The seeksnack site corpus was not re-run in this session (the site repo
  was not attached); its 603-image result is the first session's.
* No independent red-team pass yet (this pass added the qemu-based arm64
  regeneration, the large arm64 runs and the mutation testing, but no new
  adversarial generators).
* 9 FMA sites are caught only by out-of-repo runs and 18 by none (see
  "Mutation testing"); their fusion rests on the disassembly. A small
  arm64-generated witness fixture (the ~100 synth300k lines that kill the
  9) would close the first group if FMA-dependent fixtures produced by
  the linux/arm64 oracle under qemu are accepted.
* The getPaletteIndex sites only matter for Paletted destinations (GIF,
  not used by seeksnack).
* `BenchmarkFilter` is not ported (performance only). The sequential
  `parallelize` makes large filters slower than Go on multi-core machines.
* Hugo's filters (`filters.go` and friends) are nh-images' job (see
  above); all 16 resampling kernels are verified here, so nh-images need
  not restrict itself to box resampling.
* `log2` and the other `math` functions gift does not call are not ported.
