# neohugo-images

Image processing for neohugo (REWRITE_PLAN.md §2.1, §2.4, task T41): processing specs,
operations and filters, codecs, the deferred image queue and its file cache, and EXIF metadata.
Hugo's semantics are the reference; Go's pixel arithmetic is not reproduced (the goal is
PSNR ≥ 30 dB against Go-processed images and exact result sizes, not byte parity).

## API

| Item | What it is |
|---|---|
| `ImageSpec` | A processing spec: `"600x400 webp q75 Lanczos Center #fff r90"` (`FromStr`) or typed kwargs (`Deserialize` from a map: `action`, `width`, `height`, `format`, `quality`, `hint`, `filter`, `anchor`, `rotate`, `background`). `resolve` fills the unset options from `Imaging` → `ResolvedSpec`. |
| `Action`, `Anchor`, `Hint`, `Resample`, `ImageFormat`, `Color` | The typed options (case-insensitive names). |
| `ImageFilter` | Every Hugo filter except Text and Dither (COULD, T72), deserialized from template maps (`{"op": "overlay", "image": …, "x": 10, "y": 10}`), including `Process { spec }`, `Padding(PaddingSpec)`, `Overlay`, `Mask`, `Opacity`, `AutoOrient` and the colour filters. |
| `ImageInput` | What an operation or an overlay/mask reads: `File(path)` or `Op(ImageOpId)` (a string or an integer in template maps). |
| `Imaging` | `[imaging]` typed: `Imaging::from_config(&neohugo_config::ImagingConfig)`. |
| `ImageQueue` | `new(imaging, cache)`; `enqueue(input, spec, filters) -> Enqueued` (metadata only: name, size, format, at once); `process(&BTreeMap<OutputPath, ImageOpId>, &dyn Sink)` (rayon, build phase E6); `encoded(id)` (on demand); `get`, `len`. |
| `Enqueued` | `id`, `file_name` (`<stem>_hu_<16 hex>.<ext>`), `width`, `height`, `format`. |
| `ImageCache` | `[caches.images]`: `ImageCache::from_config(&FileCache)`; entries are read while younger than `maxAge` (never with `maxAge = 0` / `--ignoreCache`). |
| `exif::read`, `exif::orientation`, `Exif` | `.Exif`: date (`DateTimeOriginal`, wall clock), position, tags filtered by `[imaging.exif]`. |
| `resize_size`, `fit_size`, `cover_size`, `crop_rect`, `smart_region`, `rotated_size` | The size maths, public for callers that need a size without queueing. |

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
* JPEG, PNG, GIF, TIFF and BMP: `image`; WebP: libwebp through `webp`, lossy at the quality,
  with the hint's libwebp preset (photo by default: sns 80, filter 30/3, dithering) and sharp
  YUV; transparent results keep their alpha (`VP8X` + `ALPH`).

Transparent results are flattened onto the explicit background (`#rrggbb`), or onto
`[imaging] bgColor` when the target format has no transparency (JPEG). PNG is written as RGBA
when a pixel is transparent, else RGB (grey for greyscale sources).

## Tests (`cargo test -p neohugo-images`)

* `spec`: the grammar against the 2376 `DecodeImageConfig` cases of
  `oracle/images/config` (all match, bar the documented rules below), `[imaging]` decoding,
  colours, formats, typed kwargs.
* `process`: result sizes against `oracle/images/process` — every spec, filter chain and
  seeksnack template chain the oracle ran: **11046/11046** with the sources in this repository;
  **13087/13089** with Go's image test data (`NEOHUGO_GOROOT=/usr/local/go…`, 85 more sources),
  the two differences being corrupt PNGs (below). The small synthetic results are processed too
  (4100), and their pixels have the planned size.
* `psnr`: `golden/images/manifest.json` (T01) when present; interim: 38 of Hugo's own golden
  images (`resources/images/testdata/images_golden`), all ≥ 30 dB but one documented case, and
  the small oracle outputs stored in full.
* `filters`: every variant from template maps on opaque, transparent and JPEG inputs; pixels of
  the geometry and compositing filters; alpha edges.
* `queue`: names, identity, chains, publishing only wanted results, the cache, WebP, all formats.
* `exif`: date and position against `oracle/images/exif` (every real image matches).

The T01 manifest format (`rust/testdata/golden/images/manifest.json`):

```json
[{"golden": "a_hu_x.webp", "source": "path/from/repo/root.jpg",
  "imaging": {"resampleFilter": "box"},
  "steps": [{"spec": "resize 600x480"},
            {"filters": [{"op": "overlay", "image": "path/from/repo/root.png", "x": 0, "y": 0}]},
            {"spec": "resize 600x480 webp"}]}]
```

## Accepted deviations

Also in `expected_diffs.toml`, which the tests read.

* **Pixels**: not byte-identical to Go (other resampling code, other JPEG/PNG/WebP encoders:
  `image`'s JPEG writes 4:2:2 chroma, Go 4:2:0; libwebp 1.4+ instead of 1.3.2). Measured by PSNR.
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

## Not ported yet

* `.Colors` (`image_colors`): planned with `color_quant`/`kmeans_colors`, kept in Cargo.toml.
* Text, Dither, QR codes and content-aware smart crop: COULD (T72).
