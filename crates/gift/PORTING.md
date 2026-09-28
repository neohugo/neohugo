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

`zig cc -O2` (cgo's default `CGO_CFLAGS` is `-g -O2`) defines `NDEBUG`,
which cgo's own clang/gcc does not, so C `assert`s compile away. The gift
oracle links C only through gowebp's libwebp (pulled in by
`neohugo/resources/images`), which no gift command calls, so it does not
affect any gift output. The fixtures above were generated with the plain
wrapper. For a cgo oracle whose output runs through C code with asserts,
add `-UNDEBUG` to the wrapper (`exec zig cc -target aarch64-linux-musl
"$@" -UNDEBUG`).

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

## Deliberate deviations (none changes output bytes but the nil-colour case, 9)

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
   a transparent-over-transparent Over blend). Go's `int` arithmetic wraps,
   and gift relies on it for coordinates near the ends of the int range
   (`dstb.Min.X+x-srcb.Min.X` of two far origins, `Min.X-kradius`,
   `dstb.Min.Y+h` of resizeNearest, anchorPt's `b.Max.X-w` for a huge
   CropToSize): those sites use `wrapping_*` (`utils::add_sub`,
   `utils::add_sub_1`), so builds with overflow checks (the crate's own test
   profile) behave like Go instead of panicking. (Release builds wrapped
   already.)
8. **Panics are kept.** Where Go panics (a LUT index out of range for
   channel values > 1, i.e. invalid premultiplied RGBA/RGBA64/Paletted input
   through a LUT filter; out-of-range palette indexes; negative-origin
   YCbCr chroma offsets; `image.Paletted.Opaque` of a palette with more than
   256 entries, reached through the rank filters' `isOpaque`; a rank filter
   whose `Min.X-kradius` wraps; a blur sigma whose kernel size overflows),
   the port panics too. The red-team fixture checks this for 127 such
   cases.
9. **nil colours (known divergence).** A `Paletted` image with an *empty*
   palette read through gift's generic path (a type-hiding wrapper, i.e.
   `Paletted.At`) returns a nil `color.Color` in Go, and gift's
   `pixelFromColor(nil)` panics (nil dereference). go-image's closed
   `Color` enum cannot express nil and returns transparent black
   (go-image deviation 3), so the port does not panic there. Hugo's
   decoders never produce an empty palette; the red-team generators avoid
   this one combination.

## Tests and verification

`cargo test` (offline, no Go): 81 tests.

| test | what |
|---|---|
| `src/go_tests/*_test.rs` (54) + `src/utils.rs` (6) | ports of all 61 Go test functions of gift v1.2.1 except TestGolden: tables converted mechanically from the Go sources (same order, same rows), loops by hand |
| `tests/go_golden.rs` | TestGolden: 28 filters on `testdata/src.png` vs. the amd64-generated `testdata/dst_*.png` with Go's non-amd64 tolerance (±1), plus the exact arm64 result (bytes differing from each golden and SHA-256), see below |
| `tests/synth.rs` | `synth 8000 1`: random 0–3-filter chains (every filter, every resampling kernel incl. Hugo's, both operators of DrawAt) over generated images of 18 kinds (every standard type, all six YCbCr ratios, NYCbCrA 4:4:4/4:2:0, paletted; invalid premultiplied colours, odd and negative origins) into the 10 settable destination types |
| `tests/real.rs` | 1152 ops on 9 real images (crops of seeksnack JPEGs with 4:4:4/4:2:0/4:2:2 chroma, the NRGBA watermark, RGB/paletted/RGBA PNGs, gift's `src.png`): the Hugo pipeline (box resizes, same-size copy, watermark overlays), two resizes per resampling filter, ResizeToFill/Fit and every other gift filter |
| `tests/setter.rs` | every setter over 65536 adversarial float32 values (rounding boundaries ±4 ulps, out of range, NaN, ±Inf, denormals) + large DrawAt Over cases over every destination type |
| `tests/math.rs`, `tests/digests.rs` (`math_digest_fixture`) | `gomath` against Go: 20000 vectors of Exp/Log/Pow/Sin/Cos/Sincos (specials, random bits, ranges) + dense sweeps (92 FNV digests of up to 2^20 results each); `math_fma_witnesses`: `mathwitness.tsv`, 7 inputs where an unfused Log (`t1` outer term) or Sin (third coefficient) gives a different float64 |
| `tests/redteam.rs` (`redteam_fixture`) | `redteam.tsv.gz`, 15932 red-team cases (below): every image kind incl. sub-images, type-hiding wrappers, all YCbCr/NYCbCrA ratios, palettes of every colour kind and > 256 entries, far origins; extreme parameters (NaN, ±Inf, huge, denormal, -0); chains of up to 6 filters; resampling extremes; DrawAt into every destination kind; Hugo-shaped pipelines; 5000 `Bounds` cases; 127 cases where Go panics (expected `panic`); and the 81 FMA-witness cases |
| `tests/redteam.rs` (`regression_*`) | `regressions.tsv`: the two red-team bugs, one case per overflowing site |
| `tests/redteam.rs` (`redteam_big`, `GIFT_REDTEAM_BIG`) | red-team corpora, out of the repo |
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
3/3 chunk digests (and again with the red-team pass's oracle build). The
red-team fixtures (`redteam.tsv.gz`, `regressions.tsv`, `mathwitness.tsv`)
come from that linux/arm64 oracle. The native amd64 oracle differs from
the darwin/arm64 fixtures in 1569 synth cases, 2068 math rows, 4033 kernel
rows, 23 setter cases, 131 real ops, 15 weights, 3 rotbounds, 11
kerneldigest and 58 mathdigest chunks — which is why float-sensitive
fixtures must never be generated by amd64 Go.

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
checked-in suite was run. The disassembly above is the evidence that every
site is fused; this measures how much of it the tests would catch on their
own. (Site numbers below are the order of the `.mul_add(` calls in
`pixels.rs, colors.rs, convolution.rs, gift.rs, resize.rs, transform.rs,
hugo_resampling.rs, gomath.rs`; the red-team pass re-ran all 48 sites the
checked-in suite did not kill before against its new tests.)

* **88 killed by the checked-in tests** (77 before the red-team pass),
  among them everything on the golden build's path: resizeLine, the
  resample `center`, DrawAt Over (all five), the setters' `+ 0.5`, and
  further the Gray setter luma, rotatePoint, the interpolations,
  convolution, unsharp, Sepia, HSL, Sigmoid, Colorize, bcspline's `x` terms
  and its `-b-6c`, the kernels' Blackman `0.08*cos` term, exp's outer
  terms, log's `R`/`hfsq` combination, the sin/cos outer terms and
  `y*PI4C`. The 11 added by the red-team pass, all from linux/arm64 (qemu)
  output:
  * in `redteam.tsv.gz`, 25 lines of the arm64 `synth 300000 12345` run
    (the smallest of the lines that kill each site): getPaletteIndex's
    first add (site 0), the `f32u8` setter rounding (3), Contrast for
    0 ≤ p ≤ 1 (10), Grayscale/Threshold's blue term (12), Colorize's blue
    channel (28), Sobel green and blue (39, 40);
  * the `witness()` cases (`gift rtwitness`): float32 pixels found by an
    exact search for which `f32u8(r*fa)` of the RGBA setter and
    `f32u16(r*fa)` of the RGBA64 setter truncate differently fused and
    unfused (`fa = a*255` or `a*65535`, rounded; e.g. RGBA64
    r = `0x391fc3d7`, a = `0x3d4d1ab7`: 0 fused, 1 unfused). This kills the
    `f32u16` site (4) and kills `f32u8` (3) a second time. The single-variable products
    (`v*255`, `v*65535` of NRGBA/NRGBA64/Gray/generic setters) have no
    float32 witness: their one candidate near 0.5 misses the window;
  * Hamming's window (87): the exhaustive float32 sweep finds 2 of the
    1,077,936,128 inputs in [0, 3) where the kernel value changes
    (0.053204764, 0.20666514); a search of every size pair up to 600×600
    finds that resizing a width of 423 to 364 evaluates the kernel at
    exactly −0.053204764 (i = 259, j = 301), and 423→364 Hamming resizes
    of random NRGBA64/NRGBA rows kill the mutant;
  * `mathwitness.tsv` (`gift mathin`): log's outer `t1` term (98, 4 inputs
    from the mathdigest64 sweep) and the third sin coefficient term (111,
    3 inputs).
* **25 equivalent (no input can tell them apart, or none that gift can
  produce):** the six `float32(n)/2 - 0.5` offsets and Hann's
  `0.5*cos + 0.5` / Blackman's `0.5*cos` (exact products); bcspline's seven
  coefficient sites (for the four (B, C) pairs gift and Hugo use, the
  unfused coefficient rounds to the same float32 — any change would alter
  every kernel value); `x - k*Ln2Hi` in exp and `k*Ln2Hi - …` in log,
  `y*PI4A` and `y*PI4B` in the sin/cos reduction (constants with trailing
  zero bits: exact products), cos's `1 - 0.5*zz`, pow's `x1 += x1`
  (`fma(x, x, RN(x²)) = 2·RN(x²)`); and, shown in the red-team pass:
  * getFromLut's index (7): fused and unfused `int(u*N + 0.5)` differ only
    for `u*N` within (2^-25, 1.5·2^-25) below 0.5 (at every other n + 0.5
    the unfused sum is exact). Of all getter values, only YCbCr's
    `float32(V)*inv` can come near that window (NRGBA/Gray give k/255,
    RGBA r/a ≥ 1/255, the 16-bit types k/65535 or r/a ≥ 1/65535), and its
    only candidate, V = 50000 (e.g. Y=222, Cb=131), lies exactly on the tie
    2^-25 below 0.5, where both round the same;
  * ColorspaceLinearToSRGB's float64 `FNMSUBD` (8): all 204,656,868
    float32 inputs in (0.0031308, 65535] (the largest channel value a
    getter can return is 65535, an RGBA64 r/a with a = 1) give the same
    float32;
  * exp's `Log2e*x ∓ 0.5` (90, 91): fused and unfused `int(k)` can only
    differ where `Log2e*x ± 0.5` crosses ±1 (the one place where
    `RN(Log2e*x) ± 0.5` is inexact); a scan of ±20,000 ulps around every
    boundary `(n ± 0.5)/Log2e`, n = −1100…1100 (176M inputs) finds no
    input where `k` differs.
* **12 not observed:** the inner polynomial terms of exp (93–95), log
  (99–102), sin (112, 113) and cos (118–120). Their rounding differences
  are scaled by powers of `r²`/`s⁴`/`z²` before the outer terms and are
  absorbed by the final rounding (an estimate for exp's P3 term gives
  ~1e-14 per input); none differs in the 1.48e9 mathdigest64 values or in
  a targeted native search (3M inputs per function per site: exp near
  its reduction boundaries, log near 1 and over all exponents, sin/cos up
  to 1e15, pow over gift's channel values).

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

Red-team fixtures (linux/arm64 under qemu; `rtrun.py` is the crash-tolerant
driver, see "Red-team pass"). Each regenerates from itself, so the check is
that the output is identical:

```sh
R=tools/go-oracle/gift/rtrun.py
python3 $R "$O" $F/redteam.tsv.gz /tmp/rt.tsv && zcat $F/redteam.tsv.gz | cmp - /tmp/rt.tsv
python3 $R "$O" $F/regressions.tsv /tmp/rg.tsv && cmp $F/regressions.tsv /tmp/rg.tsv
$O mathin $F/mathwitness.tsv | cmp - $F/mathwitness.tsv
$O rtwitness                  # the witness() lines inside redteam.tsv.gz
```

`redteam.tsv.gz` was assembled (then `gzip -9`) from these arm64 runs, in
this order, dropping repeated cases: the 25 synth300k FMA-killer lines
(`synth 300000 12345`, indices 10908 … 284468, see "Mutation testing"),
`rtwitness`, 32 423→364 Hamming resizes (`rtrun` of hand-written lines),
`rtbounds 2000 1`, the first 3000 lines of `rtbounds 1000000 701`,
`rtgen far 3000 1` (all 3000) and case 22046 of `rtgen far 50000 601`, the
first 2500 / 2500 / 1500 / 200 lines of `synth2 100000 101`,
`params 100000 201`, `drawat 100000 301`, `hugo 20000 401`, the 150
resample cases with the smallest source of `resample 5000 502`, and the
first 1000 of `panic 20000 802` (`rtgen` + `rtrun.py` for the draw modes;
`rtgen <mode> <n> <seed>` regenerates the case lines of each run).
`regressions.tsv` is one line per site the red-team fixes touched, from
the same far and bounds runs.

## Red-team pass (independent)

An adversarial pass that extended the oracle with new generators
(`tools/go-oracle/gift/redteam.go`) and replayed their output through the
port. Every expected value came from the **linux/arm64** oracle under
`qemu-aarch64-static` (built as in "FMA sites"), which first reproduced
every checked-in fixture byte for byte.

**Generators** (`gift rt <mode> <n> <seed>`, `gift rtbounds`). The Rust
side only parses their lines (`tests/common/mod.rs`: `gen_image_x`,
`parse_filter_x`); image specs gain `w.<typ>` (a type-hiding wrapper: the
generic getter/setter and no `Opaque`), `<typ>@x0/y0/x1/y1` (a `SubImage`
of a larger parent, sometimes partly outside it), `nycbcra422/440/411/410`,
`palettedx` (palettes of every colour kind incl. Gray16, Alpha16, CMYK,
YCbCr, NYCbCrA, up to 300 entries), `palettedoor` (indexes beyond the
palette), `palettedempty`, `rectimg` (`image.Rectangle` as an image);
rotate background colours of every kind; `witness()` and `overlayg(…)`
(Hugo's overlayFilter over a generated image).

| mode | what |
|---|---|
| `synth2` | 0–6 filter chains, every filter with parameters at and beyond their bounds (NaN, ±Inf, ±MaxFloat32, denormals, −0, 1e10, 16777217, huge ints for Crop/CropToSize/Pixelate, anchors −1…10, interpolations −1…3, convolution kernels of 0–51 entries with NaN/Inf weights), all 16 kernels, every source kind above (origins up to ±1000 and ±2^40) into every destination kind, Draw and DrawAt |
| `params` | one or two filters with extreme parameters over ≤ 12×12 images |
| `resample` | Resize/ResizeToFit/ResizeToFill (every anchor)/Resize+CropToSize, all 16 kernels: 1→3000 and 20000→1 strips both ways, prime sizes, strips to strips, one side 0, kernel support wider than the source |
| `drawat` | DrawAt, both operators, every destination kind (sub-images, wrappers, palettes > 256), points inside, outside, at Min and ±1000 away |
| `hugo` | the images Hugo's decoders return, Hugo's filters with template-like parameters and every resampling name, doFilter's destination rule, overlays |
| `far` | sources, destinations and DrawAt points near ±2^62, ±2^63 |
| `panic` | out-of-range palette indexes, empty palettes, negative-origin YCbCr, LUT filters over invalid premultiplied input, blur sigmas whose kernel size overflows |
| `rtbounds` | `GIFT.Bounds` of 1–4 filter chains with huge/negative ints and angles over rects with coordinates up to ±2^63 |

A Go panic inside gift's `parallelize` goroutines cannot be recovered and
runs the goroutine's `defer wg.Done()` first, so the main goroutine could
print a partial digest before the process dies. Cases are therefore run
by `rtrun.py`, which drives `gift rtrun` with `GOMAXPROCS=1` (the panicking
goroutine then reaches the exit without being descheduled), records a
crashing case as `panic` and restarts after it; results were checked to be
deterministic across repeated runs.

**Runs (all on the arm64 oracle; results after the fixes below).**

| mode | seeds × cases | cases | Go panics (expected `panic`) |
|---|---|---|---|
| `synth2` | 101–106 × 100,000 | 600,000 | 19 |
| `params` | 201–203 × 100,000 | 300,000 | 13 |
| `drawat` | 301–303 × 100,000 | 300,000 | 3 |
| `far` | 601 × 50,000, 602 × 100,000 | 150,000 | 27 |
| `hugo` | 401–403 × 20,000 | 60,000 | 0 |
| `resample` | 501 (first 3,133; stopped to bound ResizeToFill's temporary), 502, 503 × 5,000 | 13,133 | 0 |
| `panic` | 801 (first 1,760), 802 (first 1,643), 803 × 3,000 | 6,403 | 817 |
| `rtbounds` | 701, 702 × 1,000,000 | 2,000,000 | — |
| `synth` (the original generator) | 12345 (the previous pass's arm64 corpus, regenerated), 777 × 300,000 | 600,000 | — |
| smoke runs (seed 1, earlier generator versions) | 2,000 each of synth2/params/drawat/hugo/resample/rtbounds, 3,000 far, 500 panic | 15,500 | 60 |

That is about 2.04M draw cases and 2.0M `Bounds` cases, with **0
differences** from arm64 Go after the two fixes below (every Go panic is
matched by a Rust panic), except 4 smoke `panic` cases that hit the
nil-colour divergence (deviation 9), which the generators avoid since.
Out of the repo, `GIFT_REDTEAM_BIG` replays the gzipped outputs
(`GIFT_SYNTH_BIG` for `synth`).

**Bugs found and fixed.** Both are integer overflows that Go wraps and the
port turned into panics under overflow checks (the crate's `[profile.test]`
and any debug build); release builds wrapped already and produced Go's
bytes. No output difference was found in release semantics.

1. `transform.go:anchorPt` (CropToSize, ResizeToFill): `b.Max.X - w`,
   `b.Min.X + (b.Dx()-w)/2` for a huge width/height or a source rect whose
   `Dx` overflows (e.g. `rtbounds` case 412: CropToSize(2^62, 2^40,
   Bottom) over (MinInt64,861)-(2,1279) gives Go `0,0,1375,398` after
   later filters; the port panicked). Fix: `wrapping_*` in
   `transform.rs:anchor_pt`. Test: `regression_crop_to_size_anchor_wraps`.
2. Coordinate arithmetic near the ends of the int range (images and DrawAt
   points at ±2^62…±2^63, e.g. a DrawAt into an image at x ≈ MaxInt64−40):
   `dstb.Min.X+x-srcb.Min.X` of copyimage, colorchanFilter, colorFilter,
   convolutionFilter, convolve1dh/1dv, unsharp, Sobel, rankFilter, crop,
   resizeHorizontal/Vertical; `dstb.Min.X+srcb.Max.X-srcx-1` of the
   transforms; convolution's `starty+i-kcenter`, `x+w.u`,
   `y+ksize/2+1`; rank's `Min.X±kradius`, `x+1+kradius`; resizeNearest's
   `dstb.Min.Y+h` loop bound and `dsty-dstb.Min.Y`; rotate's
   `dstb.Min.X+x`; interpolateCubic/Linear's `Min.X-1` and `x0+j-1`;
   splitRange; setPixelRow/Column's `x++`. 813 of the first 3000 `far`
   cases panicked. Fix: `wrapping_*` in Go's evaluation order
   (`utils::add_sub`, `utils::add_sub_1`). Test:
   `regression_far_origin_coordinates_wrap` (one case per site).

**Go panics reproduced** (127 in the fixture; the port panics in the same
cases): out-of-range palette indexes, negative-origin YCbCr chroma
offsets, the `image.Paletted.Opaque` index panic for palettes with more
than 256 entries (via the rank filters' `isOpaque`; go-image reproduces
it), rank filters whose `Min.X-kradius` wraps (the init loop does not run
and `pxbuf[...]` panics), LUT indexes of invalid premultiplied input and
`makeslice` for sigmas of ±Inf/1e20/MaxFloat32.

**Left unfixed:** the nil-colour case (deviation 9). Also theoretical and
not tested: a rank filter whose `Min.X+kradius` is exactly MaxInt64 loops
forever in Go (`i <= MaxInt64`); the port's inclusive range terminates.

**Other checks.** The fused-instruction inventory of the arm64 build
matches the tables above exactly (gift 119, resampling.go 15, math 68, same
source lines). The FMA witness searches of "Mutation testing" (exact
float32 setter/LUT search, exhaustive float32 Hamming and LinearToSRGB
sweeps, the resize size-pair search, the exp boundary scan and the gomath
mutant searches) are scratch programs over the crate, not checked in;
their witnesses are.

## Changes in the verification pass (before the red-team)

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
* 12 FMA sites (the inner polynomial terms of exp, log, sin and cos) are
  observed by no test; their fusion rests on the disassembly, and their
  effect is below every output rounding found so far (see "Mutation
  testing").
* nil colours (deviation 9) and other custom Go `color.Color`
  implementations cannot be expressed with go-image's closed `Color` set.
* The wrapping audit covers gift; go-image's own draw loops keep their
  documented debug-build overflow gap (go-image PORTING.md).
* The getPaletteIndex sites only matter for Paletted destinations (GIF,
  not used by seeksnack).
* `BenchmarkFilter` is not ported (performance only). The sequential
  `parallelize` makes large filters slower than Go on multi-core machines.
* Hugo's filters (`filters.go` and friends) are nh-images' job (see
  above); all 16 resampling kernels are verified here, so nh-images need
  not restrict itself to box resampling.
* `log2` and the other `math` functions gift does not call are not ported.
