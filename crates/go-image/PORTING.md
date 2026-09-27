# go-image — porting notes

Byte-exact port of the go1.27.1 standard library packages `image`,
`image/color`, `image/color/palette`, `image/draw`, `image/jpeg` and
`image/internal/imageutil`. `image/png` is **not** in this crate (the
go-png crate builds on it).

## Go file → Rust module map

| Go file (go1.27.1 `$GOROOT/src/image/…`) | Rust module |
|---|---|
| `geom.go` (Point, Rectangle, Rect, mul3NonNeg, add2NonNeg) | `src/geom.rs` |
| `image.go` (Image/RGBA64Image/PalettedImage, RGBA, RGBA64, NRGBA, NRGBA64, Alpha, Alpha16, Gray, Gray16, CMYK, Paletted) | `src/image.rs` |
| `ycbcr.go` (YCbCrSubsampleRatio, YCbCr, NYCbCrA) | `src/ycbcr.rs` |
| `names.go` (Uniform, Black/White/Transparent/Opaque) | `src/names.rs` |
| `format.go` (RegisterFormat, Decode, DecodeConfig, ErrFormat) + `image.Config` | `src/format.rs` |
| `color/color.go` | `src/color/mod.rs` |
| `color/ycbcr.go` | `src/color/ycbcr.rs` |
| `color/palette/palette.go` (generated tables) | `src/color/palette.rs` (tables converted mechanically) |
| `draw/draw.go` | `src/draw.rs` |
| `internal/imageutil/impl.go` (DrawYCbCr) | `src/imageutil.rs` |
| `jpeg/reader.go` | `src/jpeg/mod.rs` |
| `jpeg/scan.go` | `src/jpeg/scan.rs` |
| `jpeg/huffman.go` | `src/jpeg/huffman.rs` |
| `jpeg/dct.go` (the go1.26+ integer Loeffler FDCT/IDCT) | `src/jpeg/dct.rs` |
| `jpeg/writer.go` | `src/jpeg/writer.rs` |

Every ported function carries a `// Go: <path>:<Func>` comment.

## Public API (for downstream crates: go-png, gift, nh-* image layer)

Crate root (`go_image::…`, Go package `image`):

* `Point { x, y }`, `Rectangle { min, max }` (Go `int` = `i64`), `pt`, `rect`,
  `ZP`, `ZR`, all geometry methods (`add`, `sub`, `intersect`, `union`,
  `in_`, `eq`, `overlaps`, `inset`, `canon`, `dx`, `dy`, `size`, `mod_`…);
  `Display` gives Go's `String()` (`"(3,4)-(6,5)"`).
* Trait `Image` (Go `image.Image`): `color_model() -> color::Model`,
  `bounds()`, `at(x, y) -> color::Color`, plus the interface-assertion hooks
  `as_any()` / `into_any()` (Go type switches), `as_rgba64_image()`
  (`img.(image.RGBA64Image)`), `as_paletted_image()`
  (`img.(image.PalettedImage)`), `try_opaque()`
  (`img.(interface{ Opaque() bool })`). Helpers on `dyn Image`:
  `downcast_ref::<T>()`, `is::<T>()`, `downcast::<T>()` (owned).
* Traits `RGBA64Image` (`rgba64_at`) and `PalettedImage` (`color_index_at`).
* Concrete images with public fields `pix: Vec<u8>`, `stride: i64`,
  `rect: Rectangle` (and `palette` for `Paletted`; `y/cb/cr/y_stride/c_stride/
  subsample_ratio/rect` for `YCbCr`; `ycbcr/a/a_stride` for `NYCbCrA`, which
  also derefs to its `YCbCr`): `RGBA`, `RGBA64`, `NRGBA`, `NRGBA64`, `Alpha`,
  `Alpha16`, `Gray`, `Gray16`, `CMYK`, `Paletted`, `YCbCr`, `NYCbCrA`,
  `Uniform { c }`, and `Rectangle` itself. Constructors are `T::new(r)`
  (`NewT`), `Paletted::new(r, palette)`, `YCbCr::new(r, ratio)`. Inherent
  methods mirror Go (`rgba_at`, `nrgba_at`, `gray_at`, `ycbcr_at`,
  `pix_offset`, `y_offset`, `c_offset`, `set`, `set_rgba64`, `set_rgba`,
  `set_nrgba`, …, `sub_image`, `opaque`) so no trait import is needed for
  concrete types.
* `YCbCrSubsampleRatio::{Ratio444, Ratio422, Ratio420, Ratio440, Ratio411,
  Ratio410}` (discriminants = Go constants; `Display` = Go `String()`).
* `BLACK`, `WHITE`, `TRANSPARENT`, `OPAQUE` (`image.Black`… as `Uniform`s).
* Registry: `register_format`, `decode(&mut dyn Read) -> (Box<dyn Image>,
  String)`, `decode_config`, `Config`, `ErrFormat`. Formats must register
  explicitly (no Go `init`): `go_image::jpeg::register()`.

`go_image::color` (Go `image/color`):

* Colour structs `RGBA`, `RGBA64`, `NRGBA`, `NRGBA64`, `Alpha`, `Alpha16`,
  `Gray`, `Gray16`, `YCbCr`, `NYCbCrA { ycbcr, a }`, `CMYK`, each with
  `rgba() -> (u32, u32, u32, u32)`.
* `Color` enum over those (Go's `color.Color` interface), `Copy`, with
  `rgba()` and `From<T>` for each struct.
* `Model` enum (`RGBA`, …, `CMYK`, `Palette(Palette)`, `Uniform(Color)`,
  `Func(fn(Color) -> Color)`) with `convert`; model functions `rgba_model`,
  `nrgba_model`, `gray_model`, `ycbcr_model`, `cmyk_model`, …; consts
  `RGBA_MODEL`, `NRGBA_MODEL`, ….
* `Palette(pub Vec<Color>)` (derefs to `Vec<Color>`) with `index` and
  `convert` (`None` = Go nil).
* `rgb_to_ycbcr`, `ycbcr_to_rgb`, `rgb_to_cmyk`, `cmyk_to_rgb`, `BLACK`,
  `WHITE`, `TRANSPARENT`, `OPAQUE`.
* `color::palette::{plan9(), web_safe(), PLAN9_RGBA, WEB_SAFE_RGBA}`.

`go_image::draw` (Go `image/draw`):

* Traits `draw::Image` (`set`, `as_any_mut`, `as_draw_rgba64_image`, and
  `<dyn draw::Image>::downcast_mut`) and `draw::RGBA64Image` (`set_rgba64`).
  All settable standard images implement both.
* `draw(dst, r, src, sp, op)`, `draw_mask(dst, r, src, sp, mask, mp, op)`,
  `Op::{Over, Src}`, trait `Drawer` (implemented by `Op` and
  `FloydSteinberg`), `FLOYD_STEINBERG`, trait `Quantizer`.

`go_image::jpeg` (Go `image/jpeg`):

* `decode(&mut impl Read) -> Result<Box<dyn Image>, Error>` returns a
  `Gray`, `YCbCr` (exact subsample ratio, un-upsampled chroma, full MCU-padded
  buffers exactly as Go), `RGBA` (RGB JPEGs) or `CMYK`.
* `decode_config`, `encode(&mut impl Write, &dyn Image, Option<&Options>)`,
  `Options { quality }`, `DEFAULT_QUALITY`, `register()`.
* `Error` (`Display` equals Go's `Error()` strings: `invalid JPEG format: …`,
  `unsupported JPEG feature: …`, `unexpected EOF`, …).

## Deliberate deviations

1. **SubImage aliasing.** Go's `SubImage` returns a view sharing `Pix`.
   `sub_image(&self, r)` returns an owned image whose `pix` is a *copy of
   Go's `Pix[i:]`* (same contents, length and stride), so reads are
   identical but writes do not propagate to the parent. `into_sub_image(self,
   r)` consumes the parent (no copy when `r.Min` is the parent's `Min`; the
   JPEG decoder relies on this to keep writing into the MCU-padded buffer
   exactly like Go). `with_sub_image_mut(&mut self, r, f)` emulates one
   aliasing mutable view (splits off Go's `Pix[i:]`, runs `f`, re-appends) —
   use it where Go code draws into a sub-image of a destination (e.g. gift's
   `DrawAt`).
2. **dst == src.** `draw_mask(dst: &mut dyn draw::Image, src: &dyn Image, …)`
   cannot alias in safe Rust, so Go's `processBackward`/`dst == src` checks
   are kept as address comparisons that are always false. Drawing an image
   onto itself must clone the source first (which gives Go's result).
3. **nil colours.** `Paletted::at` on an empty palette returns
   `Color::RGBA64(zero)` (Go returns nil); `Model::Palette(empty).convert`
   panics (Go returns nil, and any use of it panics). `Palette::convert`
   returns `Option<Color>` to expose the nil case.
4. **Closed colour set.** `Color` is an enum of the standard colour types; a
   custom Go `color.Color` implementation cannot be expressed. Custom *image*
   types are supported through the traits (the tests implement Go's
   `slowerRGBA`/`slowestRGBA`/`embeddedPaletted` and wrapper types).
5. **Subsample ratio values.** Go's `YCbCrSubsampleRatio` is an `int`; an
   out-of-range value (→ `"YCbCrSubsampleRatioUnknown"`, 4:4:4 offsets) cannot
   be represented by the Rust enum.
6. **Readers/writers.** The JPEG decoder keeps Go's 4096-byte buffer and
   byte-stuffing/unread logic but reads from `std::io::Read` (`Ok(0)` = EOF →
   `unexpected EOF`; `Interrupted` is retried). The encoder writes through an
   internal 4096-byte buffer (Go wraps non-`Flush` writers in a
   `bufio.Writer`); the byte stream is identical, only the chunking of
   `write` calls may differ. `decode`/`decode_config` (Go `image.Decode`/
   `image.DecodeConfig`) wrap the reader in a port of `bufio.Reader`
   (4096-byte buffer, Go's `Peek`/`Read`/`fill` logic) exactly as Go's
   `asReader` does, so the registered decoder sees Go's read chunking. (Go's
   JPEG decoder result is in fact chunking-independent: verified in Go over
   200 000 mutated inputs with five different reader chunkings, and in the
   port by `jpeg_corpus::corpus_chunked_reads`.)
7. **Format registry** requires explicit registration (`jpeg::register()`)
   instead of package `init`. `decode` returns only the error on failure
   (Go also returns the format name).
8. **Panics** are kept where Go panics on programmer errors (`NewT` with
   negative/huge rectangles, out-of-range indexes such as a palette index
   beyond the palette). Decoder/encoder input errors are `Result`s; the
   decoder never panics on the 100k-mutation fuzz corpus.

## Integer semantics

Go's wrapping `int32`/`uint32`/`uint16` arithmetic is reproduced with
`wrapping_*` everywhere it can overflow (DCT on corrupt coefficients,
`draw` blending with invalid premultiplied colours, `FALLBACK1.17`'s
`uint16 + uint16`, Floyd-Steinberg error sums, geometry), and Go's shift
semantics (shift counts ≥ width) with `go_shl_u32`/`go_shr_u32`/
`go_shr_i32`/`go_shl_i32` in the Huffman bit reader and the encoder's
`emit`.

## FMA sites

None. These packages contain no floating-point arithmetic in go1.27.1 (the
images spec confirmed with `go tool objdump` of the golden binary: no float
instructions in `image`, `image/color`, `image/draw`, `image/jpeg`); the new
DCT is pure integer fixed-point. This port contains no `f32`/`f64` code.

## Tests / parity evidence

Oracle: `tools/go-oracle/go-image` (package main in the neohugo module;
`dct_copy.go` is a verbatim copy of go1.27.1 `image/jpeg/dct.go` so the
oracle can call the unexported `fdct`/`idct`). Random inputs come from a
splitmix64 generator re-implemented call-for-call in `tests/common/mod.rs`.

| test | oracle command | coverage |
|---|---|---|
| `jpeg_decode::decode_seeksnack_corpus` | `decode-files` | all 538 seeksnack JPEGs (content + assets; 513 YCbCr 4:4:4, 24 4:2:0, 1 4:2:2; progressive, baseline, DRI, Adobe APP14): DecodeConfig, type/rect/strides/ratio/full-buffer SHA-256, q75 re-encode of the decoded YCbCr, `draw.Draw(RGBA, Src)` pixels and its q75 encoding. Skipped if the corpus is absent (`GO_IMAGE_SITE_ROOT` overrides the path). |
| `jpeg_decode::decode_checked_in_fixtures` | `decode-files` | 30 Go `image/testdata` JPEGs (CMYK, RGB, gray, all ratios, flex 221212/121121/211211/221122/222112, progressive, truncated, restart intervals) + 10 site samples |
| `jpeg_synth` | `synth 300` (+ `GO_IMAGE_SYNTH_BIG` for 2000) | 17 image kinds (every standard type incl. all YCbCr ratios, NYCbCrA, Paletted), random/smooth content, sizes 1–300, non-zero origins, encoded at **every quality 1..100**, nil/−5/200 options, q75 round-trip decode |
| `jpeg_fuzz` | `fuzz 5000` (+ `GO_IMAGE_FUZZ_BIG` for 100 000) | mutated JPEGs: config result, exact error string or full decoded digest |
| `draw_ops` | `draw 12000` (+ `GO_IMAGE_DRAW_BIG` for 200 000) | random `Draw`/`DrawMask`/`FloydSteinberg.Draw` over 10 dst types + a non-RGBA64 wrapper (FALLBACK1.0), 19 src kinds incl. Uniform, Rectangle, non-RGBA64 wrapper, 8 mask kinds, both ops, clipping |
| `color_space` | `color` | RGBToYCbCr, YCbCrToRGB, YCbCr.RGBA, RGBToCMYK over all 2^24 inputs; CMYKToRGB/CMYK.RGBA over 2^24 × 6 K values; NYCbCrA.RGBA; all 11 models over exhaustive (c, a) inputs and 200k random colours of every type; gray/YCbCr models over the RGB cube; Plan9/WebSafe/random palette Index/Convert |
| `jpeg::dct::tests` | `dct 5000` | fdct/idct on pixel, sparse, large and arbitrary-int32 (wrapping) blocks |
| `jpeg_corpus::corpus_checked_in` | `record`, `encsweep` over `tests/fixtures/corpus/{gen,mut}.bin` | corpus-based records (inputs stored in the corpus, so Rust does not re-implement the generators): `jpeg.DecodeConfig`, `image.DecodeConfig` (registry + bufio), `jpeg.Decode`, `image.Decode`, encodings of the decoded image at q1/50/75/100, `draw.Draw` into RGBA (DrawYCbCr/drawGray/drawCMYK/copy fast paths, drawRGBA for 4:1:1/4:1:0), its q75 encoding, `DrawMask` Over with an Alpha mask onto a patterned RGBA (drawGrayMaskOver/drawRGBA64ImageMaskOver/drawRGBAMaskOver), `draw.Draw` into NRGBA (FALLBACK1.17), a sub-image with odd offsets (bounds, q75 encoding, RGBA conversion), FloydSteinberg onto Plan9. `gen.bin`: 260 small files stratified over output type × YCbCr ratio × progressive × restart × error class, from cjpeg/jpegtran/ImageMagick (`gen_libjpeg.py`, see below) and `mkjpeg` (random-structure baseline/extended writer: 1/3/4 components, arbitrary sampling factors incl. flex and unsupported ones, 8/16-bit and zero quant tables, table ids 0..3, permuted Huffman tables, component ids incl. 'R','G','B', JFIF/Adobe markers, restart intervals with junk/wrong/missing RST markers, interleaved and non-interleaved scans, re-scans, missing EOI, trailing data). `gen.enc.tsv`: every decodable one encoded at every quality 1..100. `mut.bin`: 1000 structure-aware mutations (header bytes set to special values/flipped, marker pairs inserted, truncation, deletion, segment duplication/removal, marker-code and length changes) |
| `jpeg_corpus::corpus_chunked_reads` | — | decoding with 1/2/3/7/4093/4094/4095-byte reads gives the same result as one-shot reads |
| `jpeg_corpus::corpus_big` | `GO_IMAGE_CORPUS_BIG=<dir>` | every `<name>.bin` + `<name>.tsv` (+ `<name>.enc.tsv`) in the directory |
| `draw_ops2` | `draw2 0 8000` (+ `GO_IMAGE_DRAW2_BIG` for 2 000 000) | like `draw_ops`, but destinations, sources and masks are sub-images one time in three (Pix not starting at Rect.Min, Stride larger than the row, odd chroma offsets, empty sub-images). `draw2_aliased_sub_image_dst` replays the rows with a sub-image destination through `with_sub_image_mut` on the parent (the aliasing emulation gift's DrawAt uses) and checks Go's `sub.Pix` against the parent's tail |
| `go_*_tests.rs` + unit tests | — | ports of Go's own tests: TestRectangle, TestImage, TestNewXxxBadRectangle, Test16BitsPerColorChannel, TestRGBA64Image, TestYCbCr, colour consistency tests, TestPalette, TestSqDiff (color and draw), TestDraw (incl. slower/slowest RGBA), TestNonZeroSrcPt, TestFill, TestDrawSrcNonpremultiplied, TestFloydSteinbergCheckerboard, TestPaletted, TestDecodeProgressive, TestDecodeEOF (as short reads), TestTruncatedSOSDataDoesntPanic, TestLargeImageWithShortData, TestPaddedRSTMarker, TestExtraneousData, TestIssue56724, TestIssue78368, TestBadRestartMarker, TestDecodeFlexSubsampling, TestZigUnzig, TestUnscaledQuant, TestWriteGrayscale, TestEncodeYCbCr, TestClip (`draw::tests::test_clip`) |

Results at the time of writing (go1.27.1 oracle, darwin/arm64): every
checked-in fixture matches, plus the out-of-repo corpora — 538/538
seeksnack JPEGs (all six fields), 2000 synthetic images × 100 qualities
(200 000 encodings) + round trips, 200 000 random draw operations and
100 000 mutated JPEGs — with zero differences. Performance of the release
build is on par with Go for decode, DrawYCbCr conversion and q75 encode
(`examples/jpegbench.rs`, `decodeloop.rs`, `encodeloop.rs`).

Regenerating fixtures (from the repo root):

```sh
go build -o /tmp/oracle ./tools/go-oracle/go-image
cd crates/go-image/tests/fixtures
/tmp/oracle synth 300 > synth.tsv
/tmp/oracle draw 12000 > draw.tsv
/tmp/oracle dct 5000 > dct.tsv
/tmp/oracle color > color.tsv
ls gotestdata/*.jpeg site/*.jpg > /tmp/list && /tmp/oracle decode-files . /tmp/list > fixtures-decode.tsv
/tmp/oracle fuzz 5000 $(ls gotestdata/*.jpeg site/content_cookies_alices-pineapple-pastry_600x200.jpg \
    site/content_potato-crisps_pringles-paprika_pringles-paprika.jpg \
    site/content_pretzels_combos-pizzeria-pretzel_combos-pipr.jpg) > fuzz.tsv
# site-decode.tsv: decode-files <pristine-seeksnack> <sorted list of content/**/*.jpg assets/**/*.jpg>
```

Corpus fixtures (adversarial verification pass; `W` is a scratch dir):

```sh
go build -o $W/oracle ./tools/go-oracle/go-image
cd $W
$W/oracle mkjpeg 0 3000 mk                        # random-structure JPEGs
python3 <repo>/tools/go-oracle/go-image/gen_libjpeg.py lj1 1000 1 @extra.list   # cjpeg/jpegtran/magick
# (extra.list: files for jpegtran to transform, e.g. every 10th mk file + fixtures)
ls mk > mk.list && $W/oracle pack mk.bin mk mk.list && $W/oracle record mk.bin > mk.tsv
ls lj1/*.jpg > lj.list && $W/oracle pack lj.bin . lj.list && $W/oracle record lj.bin > lj.tsv
python3 <repo>/tools/go-oracle/go-image/select_fixtures.py lj.tsv mk.tsv mk > sel.list
C=<repo>/crates/go-image/tests/fixtures/corpus
$W/oracle pack $C/gen.bin . sel.list    # (mk entries listed as mk/<name>)
$W/oracle record $C/gen.bin > $C/gen.tsv
$W/oracle encsweep $C/gen.bin > $C/gen.enc.tsv
$W/oracle mutate 1000 5000000 $C/mut.bin $C/gen.bin && $W/oracle record $C/mut.bin > $C/mut.tsv
$W/oracle draw2 0 8000 > <repo>/crates/go-image/tests/fixtures/draw2.tsv
```

### Adversarial verification pass (independent verifier)

Method: side-by-side review of every Rust function against go1.27.1
(`reader.go`, `scan.go`, `huffman.go`, `dct.go`, `writer.go`, `image.go`,
`ycbcr.go`, `geom.go`, `names.go`, `format.go`, `color/*.go`,
`draw/draw.go`, `internal/imageutil/impl.go`), plus new generators and
differential corpora (above). `go tool objdump` of the oracle confirms zero
floating-point instructions in the 233 functions of `image`,
`image/color`, `image/draw`, `image/jpeg` and `image/internal/imageutil`
(no FMA sites possible).

Out-of-repo results (zero differences in every case):

| corpus | entries |
|---|---|
| `mkjpeg` random-structure JPEGs (`record`) | 3 000 |
| cjpeg/jpegtran/ImageMagick JPEGs: progressive, custom scan scripts with successive approximation, restart intervals (rows and blocks), optimized Huffman tables, arbitrary `-sample` factors, `-rgb`, grayscale, CMYK/Adobe, crops/rotations to odd sizes, arithmetic and 12-bit (error paths), transforms of seeksnack images (`record`) | 4 100 |
| structure-aware mutations of the above (`record`) | 120 000 |
| all 538 seeksnack JPEGs through `record` (adds `image.Decode`, mask/NRGBA/sub-image paths; 12 > 4 MP skipped there, covered by `site-decode.tsv`) + 3 000 mutations of them | 3 538 |
| every decodable image of the two generated corpora encoded at q1..100 (`encsweep`) | 5 581 × 100 |
| `draw2` (sub-image destinations/sources/masks), both owned-copy and `with_sub_image_mut` aliasing modes | 2 000 000 |
| `draw` (original generator, 5× the previous run) | 1 000 000 |
| Go-only check that `jpeg.Decode` results do not depend on reader chunking (bytes.Reader, image.Decode/bufio, OneByteReader, HalfReader, fixed chunks) | 200 000 |

Findings: no output discrepancy was found in the decoder, encoder, colour
conversions or draw paths. Changes made: `image.Decode`/`DecodeConfig` now
use a faithful `bufio.Reader` port instead of an ad-hoc peek buffer
(deviation 6; behaviour-preserving because Go's decoder is
chunking-independent), Go's `TestClip` was ported, and the corpus,
chunked-read and sub-image draw tests above were added.

Go quirk reproduced (not a port bug): Go's decoder counts restart intervals
in non-interleaved progressive scans per `mxx*myy` MCU of `h*v` blocks
rather than per block, so some valid libjpeg files (progressive, sampled
luma, `-restart`) fail in Go with `missing 0xff00 sequence`; the port
returns the same error.

### Linux x86_64 verification pass

Platform check on linux/amd64 (go1.27.1 via `GOTOOLCHAIN=go1.27.1`, rustc
1.94.1, libjpeg-turbo 2.1.5 `cjpeg`/`jpegtran`, ImageMagick 6.9.12
`convert`). Results:

* `cargo test --release`: all 52 tests pass (no test is `#[ignore]`d; the
  `*_big` tests and `decode_seeksnack_corpus` return early without their
  environment variable / corpus).
* An oracle built on linux/amd64 regenerates every checked-in Go-generated
  fixture byte-for-byte (`synth.tsv`, `draw.tsv`, `draw2.tsv`, `dct.tsv`,
  `color.tsv`, `fixtures-decode.tsv`, `fuzz.tsv`, `corpus/gen.tsv`,
  `corpus/gen.enc.tsv`, `corpus/mut.bin`, `corpus/mut.tsv`), and `mkjpeg 0
  3000` reproduces all 103 `mk/*` entries of `corpus/gen.bin`. The `lj*/*`
  entries of `gen.bin` are *inputs* made by external tools and are not
  reproducible: the `@extra.list` passed to `gen_libjpeg.py` was not
  recorded (its length changes the Python RNG stream at the first jpegtran-of-
  an-extra-file item), and the tools differ (libjpeg-turbo 2.1.5 has no
  `-precision 12`, so e.g. `lj4/lj-4-000006.jpg`, a 12-bit file, cannot be
  produced; ImageMagick 6 vs 7). Entries made before the streams diverge
  with plain cjpeg/jpegtran are byte-identical (e.g. `lj2/lj-2-000026.jpg`,
  `lj3/lj-3-000032.jpg`, `lj3/lj-3-000042.jpg`). Only the checked-in
  `gen.bin` matters for parity; everything the oracle computes from it is
  identical.
* `go tool objdump` of the amd64 oracle: no floating-point instructions in
  the 233 functions of `image`, `image/color`, `image/color/palette`,
  `image/draw`, `image/jpeg`, `image/internal/imageutil` (as on arm64), so
  nothing here can differ between architectures.
* Oracle: `GO_IMAGE_SEED0` starts `synth`/`draw`/`fuzz`/`dct` at a fresh
  seed (rows carry their seed, so the Rust tests replay any range);
  `dct_matches_go_big` (`GO_IMAGE_DCT_BIG`) checks a large `dct` output;
  `gen_libjpeg.py` falls back to `convert` when `magick` is absent.

Out-of-repo differential runs, zero differences in every case:

| run | entries |
|---|---|
| `synth 2000` (seeds 0..) / `synth 3000` at seed 10 000 000 | 2 000 / 3 000 images × 100 qualities |
| `fuzz 100000` / `fuzz 200000` at seed 10 000 000 | 100 000 / 200 000 |
| `draw 200000` / `draw 1200000` at seed 10 000 000 | 200 000 / 1 200 000 |
| `draw2 0 2000000` / `draw2 10000000 2000000` (owned + aliased) | 2 000 000 / 2 000 000 |
| `dct 1000000` at seed 10 000 000 | 1 000 000 blocks |
| `mkjpeg 0 3000` + `mkjpeg 100000 10000` (`record`, `encsweep`) | 13 000 (10 121 sweeps) |
| `gen_libjpeg.py` seeds 1–4 and 11–18 × 1000 (`record`, `encsweep`) | 13 166 (9 959 sweeps) |
| `mutate` of those (seed0 0 and 20 000 000) | 120 000 + 200 000 |
| larger real-tool images (1–1599 px per side, mean 0.29 MP, max 2.4 MP; ImageMagick plasma/gradient/noise/pattern content encoded by cjpeg/`convert`/jpegtran with random quality, sampling incl. 4:1:1/3x1/2x3, progressive, restart, optimize, CMYK, crop/rotate) + 20 000 mutations (`record`, `encsweep`) | 1 600 (1 326 sweeps) + 20 000 |

`gotestdata/` holds Go's BSD-licensed `image/testdata/*.jpeg` (see
`LICENSE-go`); `padded-rst-marker.b64` is the image from Go's
TestPaddedRSTMarker.

## Known gaps

* `image/png`, `image/gif` are not ported here (go-png is a separate crate;
  GIF is not used by seeksnack).
* Aliasing sub-images / `dst == src` drawing (see deviations 1–2); Go's
  TestDrawOverlap is therefore not ported.
* Go tests not ported: TestWriter and image TestDecode (need `image/png`),
  TestDCT (compares against a float reference with a tolerance; the exact
  `dct` differential test is stronger), TestYCbCrSlicesDontOverlap (the port
  allocates Y/Cb/Cr as separate vectors), FuzzDecode (covered by the fuzz and
  mutation corpora).
* In debug builds (`overflow-checks`), plain `+`/`-` on coordinates in the
  draw loops (`sp.x + x0 - r.min.x` etc.) would panic on i64 overflow where
  Go wraps. This needs points near ±2^63 and was not observed in any
  corpus (clipping normally empties such rectangles first); release builds
  wrap like Go.
* The draw fixtures only use YCbCr/NYCbCrA with non-negative origins because
  Go's chroma offset arithmetic panics for some negative origins (the port
  reproduces the same arithmetic, so it panics in the same places).
