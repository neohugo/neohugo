# neohugo-images

Image processing for neohugo (REWRITE_PLAN.md §2.1, §2.4, tasks T41 and T72a): processing
specs, operations and filters (text and dithering included), codecs, the deferred image queue and
its file cache, EXIF metadata, and QR code images. Hugo's semantics are the reference; Go's pixel
arithmetic is not reproduced (the goal is PSNR ≥ 30 dB against Go-processed images and exact
result sizes, not byte parity) — except for QR codes, whose PNG bytes equal Go's, and the JPEG
encoder, a port of Go's (bytes equal Go's for the same pixels but for rare DCT roundings).

## API

| Item | What it is |
|---|---|
| `ImageSpec` | A processing spec: `"600x400 webp q75 Lanczos Center #fff r90"` (`FromStr`) or typed kwargs (`Deserialize` from a map: `action`, `width`, `height`, `format`, `quality`, `hint`, `filter`, `anchor`, `rotate`, `background`). `resolve` fills the unset options from `Imaging` → `ResolvedSpec`. |
| `Action`, `Anchor`, `Hint`, `Resample`, `ImageFormat`, `Color` | The typed options (case-insensitive names). |
| `ImageFilter` | Every Hugo filter, deserialized from template maps (`{"op": "overlay", "image": …, "x": 10, "y": 10}`), including `Process { spec }`, `Padding(PaddingSpec)`, `Overlay`, `Mask`, `Opacity`, `AutoOrient`, `Text(TextSpec)`, `Dither(DitherSpec)` and the colour filters. |
| `TextSpec`, `AlignX`, `AlignY`, `FontInput`, `FontId` | `images.Text`: `text`, `color` (white), `size` (20), `x`/`y` (10), `alignx`/`aligny`, `linespacing` (2), `font` (Go Regular, else a font file or bytes registered with `ImageQueue::add_font`; an integer id in maps). Hugo's option names in any case, plus `line_spacing`, `align_x`, `align_y`; numbers may be strings, `x`/`y`/`linespacing` truncate like `cast.ToInt`. |
| `DitherSpec`, `DitherMethod` | `images.Dither`: `colors` (≥ 2; black and white), `method` (Hugo's 29 names, `floydsteinberg`), `serpentine` (true), `strength` (1.0). |
| `qr_png`, `qr_modules`, `QrLevel`, `QrModules` | `images.QR`: the PNG of a QR code, byte-identical to Hugo's (`rsc.io/qr`); the symbol's modules. Naming and publishing are `neohugo-resources`' (`ResourceStore::qr_code`). |
| `ImageInput` | What an operation or an overlay/mask reads: `File(path)` or `Op(ImageOpId)` (a string or an integer in template maps). |
| `Imaging` | `[imaging]` typed: `Imaging::from_config(&neohugo_config::ImagingConfig)`. |
| `ImageQueue` | `new(imaging, cache)`; `enqueue(input, spec, filters) -> Enqueued` (metadata only: name, size, format, at once); `process(&BTreeMap<OutputPath, ImageOpId>, &dyn Sink)` (rayon, build phase E6); `encoded(id)` (on demand); `get`, `len`; `add_font(bytes) -> FontId` (fonts of text filters, validated, by content). |
| `Enqueued` | `id`, `file_name` (`<stem>_hu_<16 hex>.<ext>`), `width`, `height`, `format`. |
| `ImageCache` | `[caches.images]`: `ImageCache::from_config(&FileCache)`; entries are read while younger than `maxAge` (never with `maxAge = 0` / `--ignoreCache`). |
| `exif::read`, `exif::orientation`, `Exif` | `.Exif`: date (`DateTimeOriginal`, wall clock), position, tags filtered by `[imaging.exif]`. |
| `resize_size`, `fit_size`, `cover_size`, `crop_rect`, `smart_region`, `rotated_size` | The size maths, public for callers that need a size without queueing. |
| `jpeg::encode`, `jpeg::Pixels`, `jpeg::YCbCr` | Go's `jpeg.Encode`: RGB(A), grey or planar YCbCr pixels at a quality (1–100, clamped) → baseline JPEG, 4:2:0 or one grey component. |

## Pipeline

`enqueue` probes the source (header only; content hash, EXIF orientation), plans the spec and
the filters into concrete steps with every intermediate size, and hashes the source content
plus the plan (xxh3) into the digits of the file name, `<stem>_hu_<digits>.<ext>`, where stem
and extension are the source's own (as in Go). The operation id hashes the digits *and* the
name: identical bytes under two names (the same photo in two bundles) are two operations, each
named after its own source whatever the render order, that share one processed result.
Enqueueing the same thing twice returns the same id; any change of the source, the spec, the
filters or the `[imaging]` defaults gives other digits. A chain (`B.Resize` of a filtered `B`) takes an `Op` input and is
decoded from its parent's encoded bytes, as in Hugo.

`process`/`encoded` decode with `image`, run the steps and encode:

* resizing: `fast_image_resize`, alpha-premultiplied (no dark fringes), with Hugo's fifteen
  kernels — Box, Linear (bilinear), Lanczos (Lanczos3) are the crate's own; CatmullRom,
  MitchellNetravali, Hermite, BSpline, Gaussian, Hann, Hamming, Blackman, Bartlett, Welch and
  Cosine are custom kernels with Hugo's definitions and supports; NearestNeighbor is nearest;
* Gaussian blur and unsharp mask: `imageproc` separable filter (radius ⌈3σ⌉), premultiplied;
* rotation by multiples of 90° and EXIF orientation: `image`; other angles: nearest neighbour
  onto a transparent canvas of Hugo's size;
* colour filters (brightness, contrast, gamma, invert, sigmoid by lookup table; grayscale,
  sepia, hue, saturation, colorize, color balance on HSL/RGB), compositing (overlay, mask,
  opacity, padding) and pixelate: here;
* text (`text.rs`, `font.rs`): Hugo's `text.go` layout — word wrapping at
  `ceil(line) + ceil(word) >= available` (an over-long first word leaves an empty line, as in
  Go), alignment, `font_height` = ⌈ascent⌉ — over a face that measures like
  `golang.org/x/image/font/opentype` (26.6 fixed point, `hhea` ascent, `hmtx` advances, GPOS
  `kern`-feature pair adjustments or the `kern` table, with x/image's quirk of applying the value
  in font units as 26.6); glyphs rasterised with `ab_glyph` at the pen's sub-pixel position,
  coverage quantised like `x/image/vector` and composited source-over. The font is part of the
  plan by its content hash (`FontId`); Go Regular (`THIRD_PARTY/gofont`) is embedded;
* dithering (`dither.rs`; its documentation names the source of each kernel and matrix): the
  published error-diffusion kernels in linear RGB with luminance-weighted nearest colours,
  serpentine rows (even rows right to left), `strength` scaling the kernel; ordered dithering as
  a threshold offset `65535·strength·(cell/max − 0.50000006)`; every result pixel is a palette
  colour with the source's alpha (transparent pixels untouched);
* JPEG: `jpeg.rs`, a port of Go's `image/jpeg` writer (`writer.go`): libjpeg's quality
  scaling of the K.1 quantisation tables, `color.RGBToYCbCr`, 4:2:0 chroma as the rounded mean
  of 2×2 samples (one component for a greyscale source whose result is still grey, like Go's
  `*image.Gray`), the K.3 Huffman tables, no JFIF/EXIF segment. The forward DCT is the exactly
  rounded one: Go 1.26 replaced its IJG-derived DCT, and the new one (source not available
  offline; the oracles ran Go 1.27.1) equals exact rounding but for a rare coefficient;
* PNG, GIF, TIFF and BMP: `image`; WebP: libwebp through `webp`, lossy at the quality,
  with the hint's libwebp preset (photo by default: sns 80, filter 30/3, dithering) and sharp
  YUV; transparent results keep their alpha (`VP8X` + `ALPH`).

Transparent results are flattened onto the explicit background (`#rrggbb`), or onto
`[imaging] bgColor` when the target format has no transparency (JPEG). PNG is written as RGBA
when a pixel is transparent, else RGB (grey for greyscale sources).

Names of text and dither results follow the rule of every filter: `<stem>_hu_<digits>.<ext>`,
the digits hashing the source and the planned steps, which hold every option of the filter
(the font by its content). Go's digits hash the filter's options too
(`filterOpts{Version, Vals: [text, options]}` with the font replaced by its key;
`[ditherOptions]`), so in both equal options give equal names and any option change gives other
digits; the digits themselves differ (xxh3 of the plan, see "Accepted deviations").

QR codes (`qr.rs`): the symbol is built with the `qrcode` crate under `rsc.io/qr`'s choices —
one segment in the smallest mode (numeric, alphanumeric, else bytes), the smallest version that
holds it, always mask pattern 0 — and written by a port of `rsc.io/qr`'s PNG writer (1-bit
grey, a `tEXt` chunk, one fixed-Huffman deflate block). Module layout and bytes equal Go's: the
tests compare against Hugo's golden QR images and `TestQR`'s content hashes.

## Tests (`cargo test -p neohugo-images`)

* `spec`: the grammar against the 2376 `DecodeImageConfig` cases of
  `oracle/images/config` (all match, bar the documented rules below), `[imaging]` decoding,
  colours, formats, typed kwargs.
* `process`: result sizes against `oracle/images/process` — every spec, filter chain and
  seeksnack template chain the oracle ran: **13087/13089**, the two differences being corrupt
  PNGs (below). 80 sources are Go's own image test data (Go 1.24.7's, in
  `testdata/upstream/goroot/src/image/`); five more that only a newer Go has are not in the
  repository, so their cases are not compared. The small synthetic results are processed too
  (4100), and their pixels have the planned size.
* `psnr`: the 20 Go-processed images of `golden/images/manifest.json` (T01) and 45 of Hugo's
  own golden images (`testdata/upstream/resources/images/testdata/images_golden`), all
  ≥ 30 dB: JPEG results reach 40–57 dB since the encoder is Go's, within 1.5 % of Go's sizes (the table printed with
  `--nocapture` gives both sizes); the text goldens 47–50 dB (glyph placement is pixel-exact, a
  one-pixel shift gives 28.5 dB) and `dither-default` 33.8 dB through the low-pass below.
  Also the small oracle outputs stored in full. A recipe with a `dither` filter is compared after a 7×7 box blur of
  both images: error diffusion is chaotic in its input (the resized source already differs from
  Go's by a few levels), so its pattern cannot match pixel for pixel; the blur compares the
  local tone (the unblurred PSNR is 7.8 dB, the mean differs by 0.1 level).
* `jpeg`: the encoder's bytes against Go's (SHA-256 of `oracle/images/process`) for every case
  whose pixels are known exactly: the synthetic YCbCr JPEG sources (4:4:4 and 4:2:0 planes,
  1×1 to 2048×1536, q90), `d:jpg` of the synthetic grey/RGB PNG sources, and the uniform
  results of the 1×1 sources at q1/q75/q90/q100: 33 of 41 equal; the others (the 640×480 and
  larger images, two 33×17) are listed in `expected_diffs.toml [jpeg]` and within 0.1 % of
  Go's size. Also the frame layout, decoding, quality clamping, greyscale.
* `qr`: `qr_png` equals Hugo's golden QR images byte for byte
  (`testdata/upstream/tpl/images/testdata/images_golden/funcs`) and the XXH64 content
  hashes of Hugo's `TestQR`.
* `text_dither`: text placement per option (colour, size, alignment, line spacing, wrapping),
  fonts from files and registered bytes (by content; unknown ids and non-fonts are errors);
  every dither method and palette gives palette colours only, with the source alpha; options
  change results deterministically, ordered patterns tile, linear luminance is kept; names of
  both filters per option.
* `filters`: every variant from template maps on opaque, transparent and JPEG inputs; pixels of
  the geometry and compositing filters; alpha edges.
* `queue`: names, identity, chains, publishing only wanted results, the cache, WebP, all formats.
* `exif`: date and position against `oracle/images/exif` (every real image matches).

The T01 manifest format (`testdata/golden/images/manifest.json`):

```json
[{"golden": "a_hu_x.webp", "source": "path/from/repo/root.jpg",
  "imaging": {"resampleFilter": "box"},
  "steps": [{"spec": "resize 600x480"},
            {"filters": [{"op": "overlay", "image": "path/from/repo/root.png", "x": 0, "y": 0}]},
            {"spec": "resize 600x480 webp"}]}]
```

Go-processed text and dither results enter the same way: a `{"filters": [{"op": "text", …,
"font": "docs/assets/opengraph/mulish-black.ttf"}]}` step (the `font` path is rooted like
`image`), or `{"op": "dither", …}` (compared through the low-pass automatically).

## Accepted deviations

Also in `expected_diffs.toml`, which the tests read.

* **Pixels**: not byte-identical to Go (other resampling code, other PNG/WebP encoders:
  libwebp 1.4+ instead of 1.3.2). Measured by PSNR.
* **JPEG**: Go's writer, with an exactly rounded DCT for Go 1.26+'s fixed-point one: a rare
  coefficient near a rounding boundary rounds the other way (`expected_diffs.toml [jpeg]`).
  A 16-bit greyscale source is written with one component (Go: three, its result is not an
  `*image.Gray`); a JPEG source processed without any change is re-encoded from RGB (Go
  encodes its decoded YCbCr planes directly).
* **Names**: `<stem>_hu_<xxh3>.<ext>`, not Hugo's hashes; they depend on content and plan only
  (Hugo's warm/cold-cache naming difference does not exist here).
* **Smart anchor**: the region *size* of Hugo's smart crop is reproduced exactly (so sizes
  match); its *position* is the centre, not content-aware (smartcrop is a COULD feature, T72).
* **Spec grammar**: unknown tokens (`bogus`, `1234`) and negative sizes are errors (Hugo ignores
  them or fails later); extensions are case-insensitive and the dot is optional; `#rgba` and
  `#rrggbbaa` are CSS (non-premultiplied) colours.
* **`[imaging]`**: an unknown `hint` is an error, and the configured hint is honoured (Hugo
  ignores it); an invalid EXIF field pattern is a configuration error.
* **Opacity** is clamped to 0…1 (Go wraps `uint8(o*255)` above 1).
* **Paletted PNG** sources are written as true-colour PNG (Hugo re-quantises to the source
  palette with Floyd–Steinberg).
* **Animated GIF** sources are processed as their first frame.
* **Corrupt PNGs**: the `png` crate decodes a file without IEND and ignores a bad zlib checksum.
* **EXIF**: tag names follow exiftool (`ModifyDate`, `ISO`, `LensInfo`, …) as Hugo's do, but
  values are simplified (rationals as `"n/d"` strings, several numbers as a space-separated
  string); damaged blocks are rejected as a whole; GPS positions stored as text are not read.
* The oracle's alpha → JPEG outputs are skipped in the interim PSNR check: the oracle calls
  `EncodeTo` directly and bypasses Hugo's flattening, so its transparent pixels are black.
* **Text**: glyph coverage is `ab_glyph`'s (font-rs lineage, like `x/image/vector`; outlines are
  not rounded to 1/64 px first) and compositing is floating point, not Go's 16-bit integer
  arithmetic: edge pixels differ slightly (PSNR). Unknown options, an invalid colour, a size
  that is not positive (or above 10 000 px), `x`/`y`/`linespacing` beyond ±2²⁴ px and a font
  that is not a resource are errors (Go ignores the first two, fails later, wraps its integers
  or panics on the others); `alignx`/`aligny` ignore case (Go: lower case only);
  a font is identified by its content (Go: by its resource key). GPOS subtables `x/image`
  cannot parse make Go refuse the font; here they only give no kerning.
* **Dither**: the pixel pattern of error diffusion is not Go's (compared through a low-pass,
  above). Palette colours are used as written, their alpha ignored (Go too). Steven Pigeon's
  kernel could not be consulted offline: a stand-in with its reach is used. The tables of
  Ulichney's and Lau & Arce's books and of the web page the library cites could not be
  consulted either: those eleven ordered matrices are constructed with the documented size,
  grey levels and dot shape (`clustereddot4x4`, `clustereddotdiagonal8x8` and
  `vertical5x3`/`horizontal3x5` are the caca.zoy.org tables; the construction reproduces the
  published 4×4). Recorded in `expected_diffs.toml`.
* **QR codes**: none in the output (bytes equal); a level's name may be in any case and is
  hashed lower-cased (Go accepts lower case only).

## Not ported yet

* `.Colors` (`image_colors`): planned with `color_quant`/`kmeans_colors`, kept in Cargo.toml.
* Content-aware smart crop: COULD (T72).
