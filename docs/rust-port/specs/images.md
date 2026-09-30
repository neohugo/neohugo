# Image processing: byte-parity spec for the Rust port (agent: images)

> **Byte-parity sections obsolete.** This spec is research for the old byte-for-byte port
> (the `crates/` tree deleted in T00 of [`REWRITE_PLAN.md`](../REWRITE_PLAN.md); recoverable with
> `git show go-parity-final:crates/<path>`, commit `be02933a`, local tag). The Rust rewrite in
> `rust/` compares structurally (REWRITE_PLAN.md §7), so every byte-parity target, golden-byte
> count and "reproduce Go's bytes" rule below is obsolete. The Hugo semantics it documents (Go
> file and line references, parity traps) remain a reference; scratch paths (`/Users/…`,
> `/private/tmp/…`, `$W`, `$SP`, `golden/run1`) no longer exist. Current state:
> [`HANDOFF.md`](../HANDOFF.md).

Scope: everything needed so that a Rust port of neohugo produces the processed images of the
seeksnack golden build (`golden/run1`) byte for byte, with identical file names. This covers
1169 jpg, 807 webp and 35 png files, of which 1461 are processed (`*_hu_*`) and 552 are copies.

All paths below are relative to:
* neohugo repo: `/Users/blackb1rd/git/github/org/neohugo`
* Go stdlib: `$(go env GOROOT)/src` = `/opt/homebrew/Cellar/go/1.27.1/libexec/src` (**go1.27.1**; this matters, see §6)
* module cache: `/Users/blackb1rd/go/pkg/mod`
* scratch: `$S = /private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad`

---------------------------------------------------------------------------------------------------

## 0. Key findings

1. **The spec is complete and proven.** A standalone Go program that uses only `gift`, `image/jpeg`,
   `image/png`, `image/draw`, `gowebp`, `xxhash` and `hashstructure` (no neohugo code) reproduces
   **all 1461 processed images in the golden output, with matching names and identical bytes**
   (`$S/work/images/repro/full`, 9 s). A second program replaces `gift` with my own
   explicit-FMA reimplementation (`$S/work/images/repro/fullref/giftref.go`). It reproduces all
   1461 files on **both arm64 and amd64** (amd64 run under Rosetta). That shows the numeric recipe
   below is exact and does not depend on the platform.
2. **The golden output depends on arm64 FMA fusion.** The Go compiler on darwin/arm64 turns
   `x*y + z` float32 expressions in `gift` into `FMADDS`/`FNMSUBS` instructions. The same
   pipeline built for amd64 (no fusion) produces **11 of 1461 files with different bytes**. A Rust
   port has to call `f32::mul_add` at exactly the fused sites listed in §5 and nowhere else.
   The two sites that matter for this site are the resample-weight `center` computation (5 files)
   and the `resizeLine` accumulation (6 files). The other sites have to be fused too, for general
   correctness.
3. **Names depend on whether the image cache was cold or warm (neohugo bug).** When an image is
   processed in this build (cache miss), its `Key()` includes `_<decimal xxhash>`. When it is read
   from `resources/_gen/images` (cache hit), the suffix is missing.
   `resources/image_cache.go:67` calls `setSourceFilenameIsHash(true)` only in the *read* path
   (the create path, lines 78-93, never calls it). The watermark `Key()` feeds the Overlay filter
   hash, so **every filtered image and all of its descendants gets a different name on a warm
   build**. I measured 4689 differing paths between warm and cold builds. The golden output is a
   **cold** build. Two ways to get cold-equivalent output: run `--ignoreCache` (I verified it
   matches the golden output even with a populated cache), or delete `resources/_gen/images`. The
   Rust port must always use **cold semantics**. Note that `$S/sites/seeksnack/resources/_gen` is
   now populated, so a new Go golden built there without `--ignoreCache` will differ.
4. **The Go version matters.** go1.27.1 ships a new integer Loeffler DCT in `image/jpeg/dct.go` and
   a new klauspost-derived `compress/flate` (`level1.go`…`level6.go`, and `EstimatedBits` with
   float32 math plus FMA). JPEG and PNG bytes are specific to go1.27.x. Port the go1.27.1 sources,
   not older Go sources and not any Rust crate.
5. **The imaging config hash includes `_merge` keys.** Every processed file name hashes the
   `[imaging]` config map, which is **4bf645f71319dd1d**. The config loader injects
   `"_merge":"none"` into every nested Params map (`config/defaultConfigProvider.go:319-335`), and
   those keys are part of the hashed map.
6. **Only three encoders and two decoders are used:** JPEG q75 4:2:0 (Go writer), PNG
   `DefaultCompression` (Go writer and Go 1.27 flate level 6), WebP lossy q75, preset *photo*,
   `use_sharp_yuv=1` (libwebp **1.3.2** via cgo). The decoders are Go `image/jpeg` and
   `image/png`. GIF, TIFF, BMP, EXIF, smartcrop, Fill, Fit, Crop and the other filters are **not
   exercised**. Resampling is always `box`.

---------------------------------------------------------------------------------------------------

## 1. Evidence and reproduction programs

| Program | What it proves |
|---|---|
| `$S/work/images/repro/main.go` + `hashes.go` (`./repro`) | 18 hand-picked chains (single.html, render-image, list 300x240, index.json 600x480 webp, RGBA PNG, **paletted PNG with Floyd-Steinberg**, favicons): all OK. Prints sha256 values. |
| `$S/work/images/repro/full/` (`./fullcheck`, `NAMES_ONLY=1` prints the chain histogram) | For every golden `_hu_` file it finds the chain by name (names are computed without pixels), then reproduces the bytes: **1461/1461 names, 1461/1461 bytes**. Also identical with `GOMAXPROCS=1`, so gift parallelism has no effect. The amd64 build (`fullcheck-amd64`, log `amd64.log`) has **11 differences** (FMA). |
| `$S/work/images/repro/fullref/` (`giftref.go`) | Reference reimplementation of the gift subset with explicit `fmaf` and `float32()` rounding. **1461/1461 on arm64 and on amd64 (Rosetta)**. This is the blueprint for the Rust code. |
| `$S/work/images/repro/fmaexp/` (`FMA_OFF=set,center,line,over ./fmaexp`) | Disables one FMA site at a time. Results: center 5 diffs, line 6, set 0, over 0, all 11. |
| `$S/work/images/repro/vectors/` | Hash test vectors (§4.5). |
| `$S/work/images/repro/cand/` | Search for the imaging-config SourceHash (found by matching `mstile-70x70_hu_80634bc5fec9785.png`). |
| `$S/work/images/inventory`, `$S/work/images/sof` | Decoded Go image types and SOF markers of all site images. |
| `$S/work/images/{gift,image,hugoimages,flate,gowebp}.objdump` | `go tool objdump` of the golden binary `$S/bin/neohugo-go` (go1.27.1 darwin/arm64). |
| `$S/work/images/site1`, `out-cold`, `out-warm`, `out-ignorecache` | Cold build = golden (except the 8-9 known RSS collisions). Warm build: 4689 diffs. `--ignoreCache` on a warm cache: only the 7 RSS diffs. |

All `repro` programs build offline with
`GOFLAGS=-mod=mod GOPROXY=off go build` (the go.sum is copied from neohugo). The dependency versions
come from neohugo `go.mod`: gift v1.2.1, gowebp v0.3.0, xxhash/v2 v2.3.0, hashstructure v0.5.0.

---------------------------------------------------------------------------------------------------

## 2. What seeksnack actually does (template chains)

Each template calls the resolved resource's `.Resize` or `images.Filter`. `wm` is
`resources.Get "images/watermark.png"` (600x480, 8-bit RGBA PNG).

| Template (layouts/…) | Chain | Published? |
|---|---|---|
| `_default/single.html:47-57` (`.Params.Image`, only if ext .jpg/.png) | A=`img.Resize "600x480"`; W=`wm.Resize "600x480"`; B=`A \| images.Filter (images.Overlay W 0 0)`; C=`B.Resize "600x480 webp"` | B (jpeg/png srcset + img src), C |
| `_default/_markup/render-image.html` (every markdown image) | W=`wm.Resize "<W>x<H>"` using the **original's** size; B=`img \| Filter(Overlay W 0 0)` (no resize); C=`B.Resize "<W>x<H> webp"` | B, C |
| `_default/list.html:34-39`, `taxonomy/list.html:39-44`, `term/term.html:127-132`, `partials/related.html:12-17`, `index.html:48-53` | A=`preview.Resize "300x240"`; W=`wm.Resize "300x240"`; B=Filter(Overlay); C=`B.Resize "300x240 webp"` | B, C |
| `_default/index.json:3-10` | same B (300x240 filtered) → `B.Resize "600x480 webp"` (upscale) | C only (B is published by the list pages) |
| `term/term.html:30-37` (term with `.Params.Image`) | A=`img.Resize "600x480"`; W computed, filter **commented out**; C=`A.Resize "600x480 webp"` | A, C |
| `partials/carousel.html:24-29` | A=Resize "600x200", W, B=Filter, C=`B.Resize "600x200 webp"` | **only B** (C is processed and cached but never published) |
| `partials/footer.html:11-12` | A=`mstile-70x70.png.Resize "128x128"`, C=`A.Resize "128x128 webp"` | A, C (+ original via head.html) |
| `partials/header.html:13` | C=`favicon-32x32.png.Resize "32x32 webp"` (directly on the original) | C |

Golden census (from `NAMES_ONLY=1 ./fullcheck`):

```
  156  .jpg Resize 300x240 -> Filter(Overlay wm300x240)
  156  .jpg Resize 300x240 -> Filter(Overlay) -> Resize 300x240 webp
  156  .jpg Resize 300x240 -> Filter(Overlay) -> Resize 600x480 webp (index.json)
    5  .jpg Resize 600x200 -> Filter(Overlay wm600x200)
    2  .jpg Resize 600x480                                   (term.html, jpg logo)
  154  .jpg Resize 600x480 -> Filter(Overlay wm600x480)
  154  .jpg Resize 600x480 -> Filter(Overlay) -> Resize 600x480 webp
    2  .jpg Resize 600x480 -> Resize 600x480 webp
  326  .jpg render-image: Filter(Overlay wm<WxH of original>)
  326  .jpg render-image: Filter(Overlay) -> Resize <WxH> webp
    1  .png Resize 128x128 ; 1 .png Resize 128x128 -> 128x128 webp
    5  .png Resize 300x240 -> Filter ; 5 -> 300x240 webp ; 1 -> 600x480 webp (index.json)
    1  .png Resize 32x32 webp (direct)
    5  .png Resize 600x480 ; 5 .png Resize 600x480 -> 600x480 webp
 = 1461
```

Unprocessed originals: all 547 bundle and asset images that were checked are byte-identical copies
of their sources (`cmp`). Five more were not found by my path-mapping script. Their URLs drop
characters such as `&`, but they are copies too. The copies are published because page resources
are published and `head.html` calls `.Permalink` on the favicons. Processed intermediates such as
A (when only B is used) and the resized watermarks are **never published**. They live only in the
file cache.

Site images (`$S/work/images/inventory`): content JPEGs decode as `*image.YCbCr`. 491 are
progressive (SOF2) 4:4:4, 16 are baseline 4:4:4, and 19 are 4:2:0. Some have Adobe APP14 or DRI
(restart intervals), but all have JFIF APP0 or no RGB transform, so none decode as RGB, CMYK or
Gray. Content PNGs are frito-lay (RGBA→`*image.NRGBA`), cpram and ja-yubari (RGBA→NRGBA),
classic-foods-inc (**RGB→`*image.RGBA`**) and berli-jucker (**8-bit palette, 115 colours + tRNS →
`*image.Paletted`**). `assets/images/companies/kee-wee-hup-kee.gif` is **never referenced**, so no
GIF code is needed.

EXIF: `[imaging.exif] excludeFields='.*'` changes the config hash only (§4.3). No template calls
`.Exif` or `.Colors` (grep is empty, and there are no `*.json` meta files in `resources/_gen/images`),
so EXIF is never decoded.

---------------------------------------------------------------------------------------------------

## 3. Processing semantics (neohugo code path)

### 3.1 Entry points (`resources/image.go`)
* `Resize(spec)` → `processActionSpec("resize", spec)` (l.218-220, 295-298). The options are
  `append([]string{action}, strings.Fields(strings.ToLower(spec))...)`, for example
  `["resize","600x480","webp"]`.
* `processOptions` (l.300-326) → `images.DecodeImageConfig(options, i.Proc.Cfg, i.Format)`, then
  `doWithImageConfig(conf, src -> Proc.ApplyFiltersFromConfig(src, conf))`.
* `Filter(filters...)` (l.241-293):
  `gfilters = images.ToFilters(f)` gives one `images.filter{Options, gift.Filter}` per filter.
  `confMain = DecodeImageConfig(options-from-ImageProcessSpecProviders (none for Overlay → []), …)`.
  `confMain.Action = "filter"`; **`confMain.Key = hashing.HashString(gfilters)`** (a decimal
  string). At draw time it calls `Proc.Filter(src, overlayFilter)` → `doFilter(src, 0, filters…)`.
* `doWithImageConfig` (l.337-397) → `ImageCache.getOrCreate`. On a miss:
  `src = i.DecodeImage()` (decodes the **encoded bytes** of the parent, which for an intermediate
  is the cache file). Then `converted = f(src)`, post-processing (§3.6), and
  `ci = i.clone(converted)`. `targetPath = relTargetPathFromConfig(conf, SourceHash)` (§4). Finally
  `ci.Format = conf.TargetFormat`, and the image is encoded to the file cache with
  `Image.EncodeTo(conf, converted, w)` (`resources/images/image.go:66-115`).
* `imageProcSem` has a single worker (l.333). Processing is serial, which has no effect on the bytes.

### 3.2 Spec parsing: `DecodeImageConfig` (`resources/images/config.go:213-344`)
* Starts from `GetDefaultImageConfig` (`image.go:333-342`): `Anchor=-1`,
  `Hint=defaults.Config.Hint` (**always 0**, because `ImagingConfigInternal.Hint` is never set in
  `DecodeConfig`), and `Quality=defaults.Config.Imaging.Quality` (75).
* Each option is trimmed and lower-cased, and empty options are dropped. Precedence order: action
  (`resize|crop|fit|fill`) → anchor name → resample filter name → hint (`picture|photo|drawing|icon|text`)
  → `#hex` bgcolor → `q<N>` quality (1..100) → `r<N>` rotate → contains `x` → `W x H` (split on
  `x`: `"600x480"`→(600,480), `"x480"`→(0,480), `"600x"`→(600,0), more than two parts is an error)
  → `ImageFormatFromExt("."+part)` gives the target format (`jpg|jpeg|jpe|jif|jfif|png|tif|tiff|bmp|gif|webp`).
* Validation: resize needs W or H. crop, fill and fit need both. No action means no W or H allowed.
* With an action and no filter set, `Filter = defaults.Config.ResampleFilter`, which is **box**
  because `defaultResampleFilter = "box"` (l.137). `Hint==0` becomes
  `EncodingPresetPhoto` (2). With an action and `Anchor==-1`, `Anchor = defaults.Anchor = smart (1000)`,
  which does not matter for resize. TargetFormat defaults to the source format. `Quality<=0` with a
  target of JPEG or WEBP becomes 75. A bgcolor is set only when source→target goes from
  transparency-capable to JPEG.
* Version suffixes: `mainImageVersionNumber=0`, `imageFormatsVersions` all 0 and
  `smartCropVersionNumber=0` (`smartcrop.go:31`), so **nothing is appended**.
* **`c.Key = hashing.HashStringHex(options)`**, where options is the cleaned `[]string` including
  the action (§4.2).
* **Both W and H given → exact size, aspect ignored.** Only `ResizeToFit` and `Fill` preserve
  aspect. Proof: every 640x480 source resized with "600x480" produces 600x480.

### 3.3 Filters built (`image.go:210-256`, `258-274`)
`resize` becomes `gift.Resize(W, H, gift.BoxResampling)`. `r<N>` would prepend `gift.Rotate` (not
used). An empty filter list returns the source unchanged. That path is not used, because every
call has an action or a filter.

### 3.4 `doFilter` destination type (`image.go:290-331`)
`bounds = gift.New(filters…).Bounds(src.Bounds())`, then the destination type is chosen by
**source type**: `*image.RGBA`→`NewRGBA`, `*image.NRGBA`→`NewNRGBA`,
`*image.Gray`→`NewGray`, **anything else (YCbCr, Paletted, CMYK, NRGBA64, …) → `NewNRGBA`**.
Then `g.Draw(dst, src)`. So JPEG sources produce NRGBA, RGB PNGs produce RGBA, RGBA PNGs produce
NRGBA and paletted PNGs produce NRGBA.

### 3.5 gift semantics used (disintegration/gift v1.2.1)
* `GIFT.Draw` (`gift.go:101-126`): with no filters it calls `copyimage(dst, src)`. With filters it
  chains them through temporary images `createTempImage` = **`image.NewNRGBA64`** (`utils.go:157-159`).
* `resizeFilter.Bounds` (`resize.go:209-228`): both dimensions given → `Rect(0,0,w,h)`. With w=0,
  the width is `int(max(1, floor(h*srcw/srch + 0.5)))` in float64 (unused).
* `resizeFilter.Draw` (`resize.go:230-265`):
  1. Same size as the source → `copyimage` (**always re-encoded**, never skipped. Examples: watermark
     600x480→600x480, favicons, the render-image webp step and every `B.Resize "<same> webp"`).
  2. Support ≤ 0 (NearestNeighbor) → `resizeNearest` (unused).
  3. Only the height changes → `resizeVertical(dst, src)`. Only the width changes →
     `resizeHorizontal(dst, src)`.
  4. Both change → `tmp = NewNRGBA64(Rect(0,0,w,srcH))`, `resizeHorizontal(tmp, src)`, then
     `resizeVertical(dst, tmp)`.
* Box kernel (`resize.go:403-416`): support 0.5, `|x| <= 0.5 → 1` else 0.
* `prepareResampWeights` (`resize.go:73-118`) and `resizeLine` (`resize.go:120-138`) are
  reproduced exactly in §5.
* `parallelize` splits rows or columns across GOMAXPROCS goroutines. Each output line is
  independent, so there is **no effect on bytes** (verified with GOMAXPROCS=1). The Rust port can
  run serially or with rayon.
* Overlay (`resources/images/overlay.go:31-43`): `Bounds = Rect(0,0,Dx,Dy)`. `Draw` does:
  `overlaySrc = f.src.DecodeImage()`, which decodes the resized watermark's **PNG bytes** from the
  cache and gives NRGBA. Then `gift.New().Draw(dst, src)` (copyimage), then
  `gift.New().DrawAt(dst, overlaySrc, image.Pt(x,y), gift.OverOperator)`.
  `DrawAt/Over` (`gift.go:139-167`): `tb = Bounds(src).Sub(Min).Add(pt)`,
  `tmp = NewNRGBA64(tb)`, `copyimage(tmp, overlay)`, then for each (x,y) in `tb ∩ dst.Bounds()`
  it blends dst and tmp as in §5.4 and writes the result back to dst through the setter.

### 3.6 Post-processing after `f(src)` (`resources/image.go:354-383`)
* `hasAlpha = !IsOpaque(converted)`, where `IsOpaque` uses `Opaque()` (the NRGBA/RGBA alpha scan).
  If (bgColor set and hasAlpha) or (target does not support transparency, i.e. JPEG, and hasAlpha),
  it runs `tmp=NewRGBA; draw.Draw(tmp, Uniform(bg), Src); draw.Draw(tmp, converted, Over)`. The
  default bg is `color.White`. **This is never hit on seeksnack**: every JPEG target has an opaque
  source.
* **Target PNG with a `*image.Paletted` source** (the berli-jucker logo): `palette = src.Palette`
  (plus the bg colour when bgColor is non-nil, which is not the case here), then
  `tmp = image.NewPaletted(bounds, palette)` and
  **`draw.FloydSteinberg.Draw(tmp, tmp.Bounds(), converted, converted.Bounds().Min)`**. Note that
  the *filtered* PNG (B) also decodes its parent A as Paletted, so it is re-paletted too. See §6.3.

### 3.7 Encoding (`resources/images/image.go:66-115`)
* JPEG: if `converted` is `*image.NRGBA` **and `Opaque()`**, the same Pix/Stride/Rect is
  reinterpreted as `*image.RGBA` (this gives the fast `rgbaToYCbCr` path).
  `jpeg.Encode(w, img, &jpeg.Options{Quality: 75})` (§6.1).
* PNG: `png.Encoder{CompressionLevel: png.DefaultCompression}.Encode` (§6.2).
* WEBP: `libwebp.Encode(w, img, EncodingOptions{Quality: 75, EncodingPreset: 2 /*photo*/, UseSharpYuv: true})` (§6.4).
* GIF, TIFF and BMP exist but are unused.

### 3.8 Decoding and image-type mapping
`DecodeImage` (`resources/image.go:410-426`) calls `image.Decode` (GIF uses `gif.DecodeAll`). The
Go type returned drives both `doFilter` (§3.4) and gift's getters (§5.1), so a Rust decoder has to
produce the **same variant**:
* JPEG (Go 1.27 `image/jpeg`): 3 components → `YCbCr` with a subsample ratio taken from the
  sampling factors (444/422/420/440/411/410, or "flex" mode for non-standard factors). The chroma
  planes are **not upsampled**, and gift takes chroma by nearest index (§5.1). 1 component → Gray.
  4 components → CMYK. Adobe transform=0 without JFIF, or component ids R,G,B → RGBA.
* PNG (Go `image/png/reader.go:438-492`): G8 → `Gray` (NRGBA when tRNS is present). GA8 → NRGBA.
  **TC8 → `RGBA`** (NRGBA when tRNS is present). **TCA8 → `NRGBA`**. P1/2/4/8 → `Paletted`.
  16-bit → Gray16/RGBA64/NRGBA64.
  Palette entries are `color.RGBA{r,g,b,0xff}`. Entries covered by tRNS become
  `color.NRGBA{r,g,b,a}` (l.299-316). If a pixel index is ≥ the palette length, the palette is
  **extended** with opaque black (l.705-716). The palette length matters for the PNG writer's bit
  depth and for PLTE and tRNS.

### 3.9 Cache, identity and publishing
* `ImageCache.getOrCreate` (`resources/image_cache.go:36-113`): `memKey = relTargetPath`, stored in
  the dynacache partition `/imgs`. On a miss it calls `fcache.ReadOrCreate(relTargetPath, read, create)`.
  * **read** (a cache file exists): clones the parent with its source set to the cache file,
    **`setSourceFilenameIsHash(true)`** (l.67), and `InitConfig` from the file.
  * **create**: runs `createImage()`, sets the target path, sets the source to the cache file and
    encodes. It does **not** call `setSourceFilenameIsHash`, so it inherits `false` from the parent.
* `genericResource.Key()` (`resources/resource.go:447-465`): `RelPermalink()` (minus the baseURL
  path, plus the language prefix for multihost). For images `includeHashInKey` is true
  (`resource_spec.go:195`), so while `!sourceFilenameIsHash` it appends `fmt.Sprintf("_%d", l.hash())`.
  The effect: **cold key = `/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147`, warm
  key = `/images/watermark_hu_3bf49ff914f6e68c.png`**. `RelPermalink` is `paths.PathEscape`d.
* `hash()` is the xxhash64 of the **original** source bytes. The `*resourceHash` pointer is shared
  by `clone()` (`resource.go:638-642`), so every processed descendant reports its root's hash
  (`resourceHash.init` l.663-688, `hashing.XXHashFromReader`).
* `filecache` (`cache/filecache/filecache.go:149-184, 332-348`): `maxAge==0` (`--ignoreCache`)
  means reads are never served, which gives cold semantics. The images cache is
  `:resourceDir/_gen/images` with `maxAge -1`.
* Hidden nondeterminism risk in Go: if dynacache evicted the watermark entry during a build (memory
  pressure), later filters would go through the *read* path and get new names. This did not happen
  in run1, run2 or nominify, which are identical.
* **Publishing**: `resourceAdapter.RelPermalink()`/`Permalink()` call `init(true, …)` →
  `publish()` → `genericResource.Publish()` (`resource.go:503-537`). That copies the cache file to
  the publish dir (skipped only when `sourceFilenameIsHash` is set and the target exists). **Only
  images whose RelPermalink or Permalink is evaluated are published.** `Key()` uses the
  genericResource's RelPermalink directly and does not publish.
* Processed images go in the **same directory as the source resource's target** (the page bundle
  dir, lower-cased, e.g. `companies/berli-jucker-foods-ltd.berli-jucker-plc/`, or `images/favicon/`
  for assets). Nothing is published under `/th/`: the TH pages reference the same URLs, the
  mcache is shared, and the site is not multihost.

---------------------------------------------------------------------------------------------------

## 4. Output file names

### 4.1 Formula (`resources/image.go:459-480`, `relTargetPathFromConfig`)
```
p1, p2 := paths.FileAndExt(sourceResourcePaths.File)     // "collon_strawberry_package", ".jpg"
if conf.TargetFormat != i.Format { p2 = TargetFormat.DefaultExtension() }   // ".webp" ".png" ".jpg"
huIdx := strings.LastIndex(p1, "_hu_")
incomingID := ""; if huIdx > -1 { incomingID = p1[huIdx+4:]; p1 = p1[:huIdx] }
hash := hashing.HashStringHex(incomingID, i.hash() /*uint64 xxhash of ROOT source*/, conf.Key, imagingCfg.SourceHash)
File = p1 + "_hu_" + hash + p2
```
When the format is unchanged the original extension spelling is kept (for example `.jpeg`
stays `.jpeg`). `HashStringHex` is `strconv.FormatUint(h, 16)`: **lower-case and not zero-padded**,
so names like `mstile-70x70_hu_80634bc5fec9785.png` have 15 hex digits.

### 4.2 `common/hashing` + `gohugoio/hashstructure@v0.5.0` (port both, ~250 lines)
* `HashUint64(vs...)` (`hashing.go:114-131`): one argument → `toHashable(v)`. Several arguments →
  `[]any{toHashable(v)...}`. Then `hashstructure.Hash(o, {Hasher: xxhash.New()})`. `toHashable`
  replaces `Key() string` types with `Key()` and IdentityProviders with `GetIdentity()`. Neither
  case applies here.
* `HashString` returns a **decimal** string (`FormatUint(h,10)`, used for the filter Key).
  `HashStringHex` returns a hex string.
* hashstructure (`hashstructure.go`), with `H(bytes)` = xxhash64 seed 0 after `Reset`:
  * top-level `string` fast path: `H(s)`. A nested string also gives `H(s)`.
  * Kind `Int`: `H(le64(int64))`. Int8/16/32: `binary.Write` of the fixed size (1, 2 or 4 bytes LE).
    Uint64/Uint: `H(le64)`. Float32/64: `H(le IEEE bits)`. Bool: `H([0|1])` (int8).
  * `hashUpdateOrdered(a,b) = H(le64(a) ‖ le64(b))`. `hashUpdateUnordered(a,b) = a ^ b`.
    `hashFinishUnordered(a) = H(le64(a))`.
  * Slice (not a set): `h=0; for e: h = ordered(h, hash(e))`. **An empty slice gives 0.**
  * Map: `h=0; for (k,v): h ^= ordered(hash(k), hash(v)); h = finish(h)`, so iteration order does
    not matter.
  * Struct: `h = H(TypeName)` (bare name, no package). For each **exported** field that is not
    tagged ignore: `h ^= ordered(H(FieldName), hash(value)); h = finish(h)`. finish runs after each
    included field, not after skipped ones. Interfaces and pointers are dereferenced first.
* Test vectors: `hash(int 0)=34c96acdcadb1bbb`, `hash("x")=5c80c09683041123`,
  `hash([]string{})=0`, `hash(false)=e934a84adb052768`, `xxhash("")=ef46db3751d8e999`.

### 4.3 Imaging config SourceHash
`config/namespace.go:22-48`: `SourceHash = HashStringHex(configSource)`, computed **before**
defaults are merged. `configSource = p.p.GetStringMap("imaging")`
(`config/allconfig/alldecoders.go:78-85`). This is the lower-cased `maps.Params` **including the
`_merge` keys** that `SetDefaultMergeStrategy` adds (`config/defaultConfigProvider.go:263-335`: the
default strategy for anything that is not params, menus, outputformats or mediatypes is `"none"`,
inherited downward):
```
{"_merge":"none","exif":{"_merge":"none","disabledate":false,"disablelatlong":false,"excludefields":".*","includefields":""}}
→ 4bf645f71319dd1d
```
Rejected candidates: without `_merge` → d18d537662de763a; with defaults merged → 4d94cd2d019ccef3.
Only the value above reproduces the golden names. **Any change to `[imaging]` renames every
processed image.**

### 4.4 Keys
* Resize: `Key = HashStringHex([]string{"resize","600x480"})` (with `"webp"` appended when given).
* Filter(Overlay): `Key = HashString([]gift.Filter{ images.filter{ Options: filterOpts{Version:0, Vals: []any{wm.Key(), x, y}}, Filter: overlayFilter{…} } })`
  (`resources/images/filters.go:49-54, 388-404`). Following §4.2, the struct names are `filter`,
  `filterOpts` and `overlayFilter`. The fields are `Options`, `Filter` (the embedded interface
  field is named after its type) and `Version`, `Vals`. `overlayFilter` has only unexported fields,
  so its hash is `H("overlayFilter")`. `x` and `y` come from template literals `0` and arrive as Go
  `int`, i.e. 8-byte int64. A float literal would change the hash.

### 4.5 Worked vectors (`$S/work/images/repro/vectors`)
```
imagingCfgSourceHash                 = 4bf645f71319dd1d
key[resize 600x480]                  = 7bfd4638d4eb3be2
key[resize 300x240]                  = a08a22b9e9d91e29
key[resize 600x480 webp]             = 590f9512b18cac1e
key[resize 640x480 webp]             = 73816495dd661eee
xxhash64(watermark.png)              = 6519743917224815147 (0x5a7ac88cb4b2ea2b)
watermark 600x480                    = watermark_hu_3bf49ff914f6e68c.png
wm.Key() (cold)                      = /images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147
filterKey(Overlay wm600x480, 0, 0)   = 1682858112077426900
xxhash64(collon_strawberry_package.jpg) = 41b9c8214be9d5d7
A = 143d7e1f185771c7 (unpublished)  B = c8f2bc05dec496d8 (.jpg)  C = 1e78ebf3348c6618 (.webp)
```
The B and C names match the golden output.

---------------------------------------------------------------------------------------------------

## 5. Numerics: exact gift float math, including the arm64 FMA sites

The Go compiler (gc, arm64) rewrites `(FADDS a (FMULS x y))` → `FMADDS` and `(FSUBS …)` →
`FMSUBS`/`FNMSUBS` whenever an add or sub has a multiply as an operand. That holds **even across
statements and even when the product has other uses**. An explicit `float32(x*y)` conversion
prevents it. amd64 (GOAMD64=v1) never fuses. Every site below was read from
`$S/work/images/gift.objdump`. Rust must use `a.mul_add(b, c)` (`f32`, single rounding) exactly
where fused and plain `*`/`+` elsewhere. Rust/LLVM never contracts implicitly. Go's ARM64
operand forms: `FMADDS Fm,Fa,Fn,Fd` means `Fd = Fa + Fn*Fm`. `FNMSUBS Fm,Fa,Fn,Fd` means
`Fd = Fn*Fm − Fa`.

Constants: `qf8 = f32(1/255)`, `qf16 = f32(1/65535)`, and for YCbCr `inv = f32(1/25500000)`.
These are Go untyped constants rounded once. In Rust, `1.0f32/255.0` and similar divisions give
the same bits because the divisors are exactly representable. Emitting the bits from Go is the
safest option.

### 5.1 Getters (`pixels.go:168-279`, no FMA)
* NRGBA: `v = f32(u8) * qf8` for all four channels.
* NRGBA64: `f32(u16) * qf16`.
* RGBA: if a=255, `(f32(c)*qf8, …, 1)`. If a=0, `(0,0,0,0)`. Otherwise `q = 1/f32(a8)`,
  `rgb = f32(c)*q`, `a = f32(a8)*qf8`. The result is un-premultiplied: note that r uses `q`, not `qf8`.
* YCbCr: `iy = (y-MinY)*YStride + (x-MinX)`. `ic` depends on the ratio: 444 `(y-MinY)*CStride+(x-MinX)`;
  422 `(y-MinY)*CStride+(x/2-MinX/2)`; 420 `(y/2-MinY/2)*CStride+(x/2-MinX/2)`;
  440 `(y/2-MinY/2)*CStride+(x-MinX)`; otherwise `COffset`. Then (int32):
  `y1=Y*100000; cb1=Cb-128; cr1=Cr-128; r1=y1+140200*cr1; g1=y1-34414*cb1-71414*cr1; b1=y1+177200*cb1`,
  `clamp(v,0,25500000)` (note `clampi32` returns 0 when `v<=min`), and `f32(clamped)*inv`
  (int→f32 rounds to nearest-even for values above 2^24). Alpha = 1.
* Paletted: `palette[k]` precomputed with `pixelFromColor(c.RGBA())`: a16==0 → 0. a16==0xffff →
  `f32(r16)*qf16`. Otherwise `q=1/f32(a16)`, `rgb=f32(r16)*q`, `a=f32(a16)*qf16`.
* Other types: `pixelFromColor(At(x,y))`.

### 5.2 Setters (`pixels.go:402-481`)
`f32u8(v)`: `x = int64(v)` (truncates toward zero), then clamp to [0,255]. `f32u16` is the same
with [0,65535].
* NRGBA: `Pix[i+k] = f32u8(mul_add(p.k, 255, 0.5))` for r, g, b and a. **Fused** (line 296).
* NRGBA64 and generic: `f32u16(mul_add(p.k, 65535, 0.5))`, big-endian bytes. **Fused** (line 307).
* RGBA: `fa = p.a*255` (rounded). r,g,b: `f32u8(mul_add(p.c, fa, 0.5))`. **alpha:
  `f32u8(mul_add(p.a, 255, 0.5))`**. The alpha is fused from p.a, not taken from fa.
* Gray (unused here): `t = mul_add(p.r, 0.299, 0.587*p.g)`, `t = mul_add(p.b, 0.114, t)`,
  `t = t*p.a`, then `f32u8(mul_add(t, 255, 0.5))`.
* Paletted (unused here): clamp to [0,1], then `getPaletteIndex` (it has FMA at l.152-156, only
  used for GIF).
* A setter ignores points outside its bounds.

### 5.3 Resize
```
prepareResampWeights(dstSize, srcSize):          // resize.go:73-118, Box: support=0.5
  delta  = f32(srcSize)/f32(dstSize)
  scale  = max(delta, 1)
  radius = ceil(scale*0.5)                          // f32 mul then ceil
  for i in 0..dstSize:
    center = mul_add(f32(i)+0.5, delta, -0.5)       // FNMSUBS: single rounding  <-- matters (5 files)
    left   = max(0, int(ceil(f64(center - radius))))
    right  = min(srcSize-1, int(floor(f64(center + radius))))
    for j in left..=right: w = kernel((f32(j)-center)/scale); if w != 0 { push(j,w); sum += w }
    for each: w /= sum
resizeLine(dst, src, weights):                     // resize.go:120-138
  for i: r=g=b=a=0
    for (j,w): c=src[j]; wa = c.a*w                // rounded
               r = mul_add(c.r, wa, r)             // FMADDS          <-- matters (6 files)
               g = mul_add(c.g, wa, g); b = mul_add(c.b, wa, b)
               a = mul_add(c.a, w, a)              // fused from the UNROUNDED c.a*w
    if a != 0 { r/=a; g/=a; b/=a }
    dst[i] = (r,g,b,a)
```
`resizeHorizontal`: for each source row, get the row through the getter, run `resizeLine`, and set
row `dst.Min.Y + (y - src.Min.Y)`. `resizeVertical` does the same by columns. Setters apply §5.2
to the destination type (NRGBA64 for the temporary image, then NRGBA or RGBA).

### 5.4 DrawAt with OverOperator (`gift.go:150-166`)
`px0` = dst pixel, `px1` = tmp (overlay) pixel:
```
c1  = px1.a
omc = 1 - c1
c0  = omc*px0.a                        // rounded (kept for later)
cs  = mul_add(px0.a, omc, c1)          // (1-c1)*px0.a + c1, fused
c0 /= cs ; c1 /= cs
r   = mul_add(px1.r, c1, px0.r*c0)     // r: fuses px1.r*c1 (px0.r*c0 rounded)
g   = mul_add(px0.g, c0, px1.g*c1)     // g,b: fuse px0.*c0 (px1.*c1 rounded)   <-- asymmetric!
b   = mul_add(px0.b, c0, px1.b*c1)
a   = mul_add(px1.a, 1 - px0.a, px0.a)
set(dst, x, y, (r,g,b,a))
```
If `cs == 0` (both pixels transparent) you get NaN. That does not happen with an opaque
destination, which is the case for every JPEG source. For transparent PNG destinations (frito-lay,
cpram) `cs` can be 0 → 0/0 = NaN. The NRGBA setter then computes `int64(NaN)`. On arm64 `FCVTZS`
gives 0, and Rust `as i64` also gives 0. With px0.a=0 and px1.a=0 you get cs=0. Those pixels
reproduced correctly with the reference port on both archs. Go amd64 `CVTTSS2SQ` returns
0x8000000000000000, which is negative and clamps to 0, so the result is 0 either way.

### 5.5 Also in Go 1.27 `compress/flate` (used by PNG) — see §6.2
`token.go:189-238` `mFastLog2` and `EstimatedBits`. `huffman_bit_writer.go:993-994`.

### 5.6 No float code elsewhere
`go tool objdump` shows **no float instructions** in `image/jpeg`, `image/png`, `image/draw`,
`image/color`, `image` or `compress/zlib` in go1.27.1. They are pure integer code, so their
behaviour is platform independent.

---------------------------------------------------------------------------------------------------

## 6. Encoders and decoders

### 6.1 JPEG (go1.27.1 `image/jpeg`, **version specific**)
* go1.26+ replaced `fdct.go`/`idct.go` with **`dct.go` (521 lines): a Loeffler 11-multiply
  integer DCT** used by both FDCT and IDCT (`fdct` l.153, `fdctCols` 161, `fdctRows` 243, `idct`
  354, `idctRows` 362, `idctCols` 449, `dctBox` 76). Porting pre-1.26 Go code, libjpeg or any Rust
  crate **will not match**.
* Writer (`writer.go`, 641 lines):
  * Markers: SOI, one DQT (length 132, both tables), SOF0 (17), one DHT (418, the four standard
    tables), SOS (12), EOI. **There is no APP0/JFIF.** I verified this on the golden output.
  * Quality → scale: `q<50 ? 5000/q : 200-2q`, so q75 gives 50. Each table entry is
    `(unscaled*scale+50)/100` clamped to [1,255]. The tables are stored in zig-zag order.
  * 4:2:0 on `*image.RGBA`: `rgbaToYCbCr` per 8x8 block, clamping edge pixels to the last row or
    column. `color.RGBToYCbCr` is integer: `yy=(19595r+38470g+7471b+1<<15)>>16`, `cb=-11056r-21712g+32768b+257<<15`
    with the bit-twiddling clamp, and `cr=32768r-27440g-5328b+257<<15`. MCU = 16x16: 4 Y blocks,
    then `scale()` averages 2x2 as `(sum+2)>>2` for Cb and Cr.
  * `writeBlock`: `fdct(b)`, `dc = div(b[0], 8*quant[0])`, AC `div(b[unzig[z]], 8*quant[z])`,
    where `div` rounds half away from zero. Standard Huffman RLE with 0xF0 ZRL and EOB. Final pad
    `emit(0x7f,7)`.
  * Gray → 1 component, `*image.YCbCr` → `yCbCrToYCbCr`, anything else → generic `toYCbCr` via
    `At().RGBA()>>8`. Only the RGBA path is used here.
* Reader (`reader.go` 837, `scan.go` 580, `huffman.go` 264, plus `idct` from dct.go): baseline and
  **progressive** (491 site images), restart intervals, Adobe APP14, "flex" non-standard subsampling
  (new in 1.27), SubImage to the true size. It outputs subsampled planes and does **no chroma
  upsampling**.

### 6.2 PNG (go1.27.1 `image/png/writer.go` 668 + `paeth.go` 71 + `compress/zlib` + **new `compress/flate`**)
* Colour type (`Encode` l.598-667): Paletted → cbP1/P2/P4/P8 by palette length (≤2, ≤4, ≤16,
  otherwise P8). Gray → G8. RGBA, NRGBA or Alpha model → **TC8 if `opaque(m)`, else TCA8**. Other
  types → TC16/TCA16. Examples in the golden output: opaque NRGBA favicon → ct2, frito-lay → ct6,
  berli-jucker → ct3 bd8 with PLTE 345 and tRNS 1.
* `writePLTEAndTRNS` (l.169-189): writes `NRGBAModel.Convert` of each entry. tRNS is written only
  if some alpha is not 0xff, truncated to the last non-opaque index + 1.
* Rows: `cr[0]` holds the raw row. TC8 copies RGB from RGBA or NRGBA Pix. TCA8 copies NRGBA Pix
  directly. RGBA with partial alpha is un-premultiplied as `c*0x101*0xffff/(a*0x101)>>8`.
  **Filter heuristic** `filter()` (l.207-296) tries Up, Paeth (with early break), None, Sub, then
  Average, keeps the minimum sum of |int8|, and breaks ties in favour of the earlier try. Paletted
  images use **no filtering** (ftNone). `pr` starts zeroed.
* Each row is written separately with `zw.Write(cr[f])`. The zlib stream goes into a
  **`bufio.Writer` of size 1<<15** whose sink is `encoder.Write` → `writeChunk("IDAT")`. As a
  result IDAT chunks are 32768 bytes plus a remainder (golden: `IDAT:32768 IDAT:2873`). Port
  bufio's large-write bypass rule faithfully. CRC32 is IEEE over type and data.
* zlib header for level 6: `0x78 0x9c`. Adler-32 is written big-endian at the end.
* **compress/flate (go1.27.1 is klauspost-derived)**: `DefaultCompression` maps to level 6, with
  `d.w.logNewTablePenalty=7`, `d.fast=newFastEnc(6)` (`fastEncL6`, `level6.go` 297 lines,
  `deflatefast.go` 201 lines), `window=maxStoreBlockSize(65535)`, and `fillBlock`/`deflateFast`
  (`deflate.go:617-676`). A block is written when the window is full or on sync.
  * `tokens.n==0` → stored block. More than `windowEnd - windowEnd>>4` tokens →
    **`writeBlockHuff`**. Otherwise **`writeBlockDynamic`**. When `windowEnd < 128` at sync:
    ≤32 bytes is stored, otherwise Huffman-only.
  * `huffman_bit_writer.go` (1116), `huffman_code.go` (417), `token.go` (309) and `load_store.go`
    (43) are all needed.
  * **Float plus FMA inside flate** (arm64 disassembly, `flate.objdump`):
    * `mFastLog2(v)`: `log2 = f32(((bits>>23)&255) - 128)`; `u = frombits((bits & -0x7f800001) + 127<<23)`;
      `t = mul_add(-0.34484843, u, 2.02466578)`; `t = mul_add(t, u, -0.67487759)`; `log2 = log2 + t`
      (plain add).
    * `EstimatedBits`: `inv = 1/f32(total)`; `shannon = mul_add(min(15, max(1, -mFastLog2(f32(v)*inv))), f32(v), shannon)`
      (the product `f32(v)*inv` is rounded); `shannon += 15` (plain); returns `int(shannon) + bits`.
      It is used by `writeBlockDynamic` (l.676) to choose between reusing the previous Huffman
      table and writing a new one, which applies to multi-block streams (PNG input > 64 KiB, i.e.
      all 600x480 PNGs).
    * `writeBlockHuff` l.989-994: `diff = f64(v) - len/256` (exact), `abs = mul_add(diff, diff, abs)`
      in f64, then compared with `2*len`.
  * The Adler-32 and CRC-32 checksums can use crates.

### 6.3 Floyd-Steinberg (`image/draw/draw.go:956-1084`, `clamp` l.934, `sqDiff` l.948)
The destination palette becomes `[][4]int32` from `col.RGBA()`. That means `color.RGBA` entries are
`c*0x101`, and `color.NRGBA` entries are premultiplied as `(c*0x101*a)/0xff` (`color.go`). Source
pixels come from `NRGBAAt(x,y).RGBA()`, which premultiplies. The error arrays have length
`Dx+2` and use **int32 division by 16 (truncating)**. `clamp` limits values to [0,0xffff]. The
nearest colour minimises `Σ sqDiff`, where `sqDiff(x,y) = (uint32(x-y)*uint32(x-y))>>2` with
**wrapping uint32 arithmetic**. The first minimum wins, with an early exit at 0. Error weights are
3/5/1 (next row x-1, x, x+1) and 7 (current row x+1). Buffers are swapped and cleared per row.

### 6.4 WebP: gowebp v0.3.0 → **libwebp 1.3.2** (`libwebp_src/ChangeLog`: "bump version to 1.3.2")
* Go side (`gowebp@v0.3.0/internal/libwebp/a__encoder.go`): `*image.RGBA` or `*image.NRGBA` →
  `encodeNRGBA(config, &Pix[0], bounds.Max.X, bounds.Max.Y, Stride)`. `*image.Gray` → `encodeGray`
  with the chroma planes memset to 128. Anything else → `draw.Draw` into a new NRGBA first.
  RGBA Pix is imported **as if it were non-premultiplied** (a bug that cannot be seen on opaque
  images).
* C side: `WebPPictureInit`, `pic.use_argb=1`, `WebPMemoryWriter`,
  **`WebPPictureImportRGBA(&pic, rgba, stride)`**, `WebPEncode`.
* Config: `WebPConfigPreset(cfg, 2 /*PHOTO*/, 75.0)`. The inline helper calls
  `WebPConfigInitInternal`, which gives quality 75, method 4, sns 80, filter_strength 30,
  sharpness 3, filter_type 1, segments 4, pass 1, `preprocessing |= 2`, alpha_compression 1,
  alpha_filtering 1, alpha_quality 100, lossless 0, exact 0 and thread_level 0
  (`libwebp_src/src/enc/config_enc.c:24-90`). Then `cfg.use_sharp_yuv = 1`, then
  `WebPValidateConfig`. Because sharp YUV is set, the dithering from preprocessing bit 2 is
  bypassed and `WebPPictureSharpARGBToYUVA` is used. Quality 0 would switch to the lossless preset
  6, but that is not used.
* Output in the golden run: 798 plain `RIFF/WEBP/VP8`, and 9 `VP8X+ALPH+VP8` from PNG sources
  with alpha.
* **C build to replicate** (`go build -x`):
  `cc -I <pkgdir> -fPIC -arch arm64 -pthread -fno-common -O2 -g -I<gowebp>/libwebp_src -c <file>.c`.
  `CGO_CFLAGS` is empty in the binary's build info, which means cgo's default `-O2 -g`. There are
  no `-D` defines, so `WEBP_USE_THREAD` and `HAVE_CONFIG_H` are both unset. The compiler is Apple
  clang 21 (default `-ffp-contract=on`), and NEON is selected through `__ARM_NEON`. There are 122
  stub `.c` files, each doing `#include "../../libwebp_src/…"`:
  * `sharpyuv/`: sharpyuv.c, sharpyuv_cpu.c, sharpyuv_csp.c, sharpyuv_dsp.c, sharpyuv_gamma.c,
    sharpyuv_neon.c, sharpyuv_sse2.c
  * `src/enc/`: alpha_enc, analysis_enc, backward_references_cost_enc, backward_references_enc,
    config_enc, cost_enc, filter_enc, frame_enc, histogram_enc, iterator_enc, near_lossless_enc,
    picture_csp_enc, picture_enc, picture_psnr_enc, picture_rescale_enc, picture_tools_enc,
    predictor_enc, quant_enc, syntax_enc, token_enc, tree_enc, vp8l_enc, webp_enc
  * `src/dsp/`: alpha_processing*, cost*, cpu, dec*, dec_clip_tables, enc*, filters*,
    lossless*, lossless_enc*, rescaler*, ssim*, upsampling*, yuv* (the _neon, _sse2, _sse41,
    _mips*, _msa variants all compile, and the non-matching ones compile to empty units)
  * `src/utils/`: bit_reader_utils, bit_writer_utils, color_cache_utils, filters_utils,
    huffman_encode_utils, huffman_utils, quant_levels_dec_utils, quant_levels_utils, random_utils,
    rescaler_utils, thread_utils, utils
  * `src/dec/*`, `src/demux/*`, `src/mux/*`: not needed for encoding, but gowebp compiles them.
  * Total is about 60k lines of C. **Do not port it. Compile it with the `cc` crate** from a vendored
    copy of `gowebp@v0.3.0/libwebp_src`.
* Cross-arch evidence: the amd64 build (SSE2/SSE4.1 paths, no FP contraction) produced identical
  WebP bytes for every file whose input image was identical. Only webps downstream of the 11
  FMA-affected JPEG/PNG files differed, and with the explicit-FMA port **all 807** matched. So
  libwebp 1.3.2 is robust across SIMD paths for this data.

---------------------------------------------------------------------------------------------------

## 7. What to port line by line (Go sources and line counts)

| Component | Source | Lines (total → needed) |
|---|---|---|
| gift subset | `gift@v1.2.1/pixels.go` 493 → ~330 (getters, setters, f32u8/16, pixelFromColor); `resize.go` 462 → ~280 (weights, resizeLine, H/V, resizeFilter, all resamp kernels); `gift.go` 215 → ~120 (Draw chain, DrawAt Over and Copy); `utils.go` 226 → ~40 (copyimage, createTempImage) | ~770 |
| image model | `image/image.go` (NRGBA, NRGBA64, RGBA, Gray, Paletted, Opaque, PixOffset), `image/ycbcr.go` 329 (YOffset, COffset, subsample ratios), `image/geom.go` (Rect ops), `image/color/color.go` 347 (RGBA() of RGBA/NRGBA/NRGBA64/Gray, NRGBAModel), `image/color/ycbcr.go` 373 (RGBToYCbCr; YCbCr.RGBA for generic paths) | ~900 needed |
| JPEG | go1.27.1 `image/jpeg`: reader.go 837, scan.go 580, huffman.go 264, **dct.go 521**, writer.go 641 | 2843 |
| PNG writer | `image/png/writer.go` 668, `paeth.go` 71 | 739 |
| PNG reader | `image/png/reader.go` 1054. Optional: the `png` crate is fine if the Go type mapping of §3.8 is reproduced | 0-1054 |
| zlib + flate encoder | `compress/zlib/writer.go` 205; go1.27.1 `compress/flate`: deflate.go 893 (level 1-6 and store/huff paths, NewWriter, write, close, sync; ~450), deflatefast.go 201, **level6.go 297**, huffman_bit_writer.go 1116, huffman_code.go 417, token.go 309, load_store.go 43 (level1-5.go only needed for other levels) | ~2800 |
| bufio.Writer | `bufio/bufio.go` Write/Flush semantics (large-write bypass) | ~60 |
| Floyd-Steinberg | `image/draw/draw.go` 934-1084 (clamp, sqDiff, drawPaletted) and `clip` | ~170 |
| hashing | `common/hashing/hashing.go` 194 → ~60; `gohugoio/hashstructure@v0.5.0/hashstructure.go` 488 → ~250 | ~310 |
| neohugo image layer | `resources/images/config.go` 490 (DecodeConfig 161-211, DecodeImageConfig 213-344, init 448-468), `resources/images/image.go` 475 (EncodeTo 66-115, FiltersFromConfig 210-256, doFilter 290-331, IsOpaque), `overlay.go` 43, `filters.go` (Overlay 49-54, filter types 388-404), `resources/image.go` 480 (Filter, processOptions, doWithImageConfig, DecodeImage, relTargetPathFromConfig), `resources/image_cache.go` 125, `resources/resource.go` (Key 447-465, Publish 503-537, hash 657-688), `config/namespace.go` 22-48 | ~1100 needed |
| libwebp 1.3.2 | compiled from C, ~60k lines | 0 ported |
| Not needed for seeksnack | GIF (`image/gif` 1125), TIFF, BMP, EXIF (`resources/images/exif`, bep/imagemeta), smartcrop, dither, text, mask, padding, opacity, auto_orient, `resampling.go` (non-box kernels, which contain FMA in bcspline), color extraction | defer |

Estimated total to port: **~10k lines of Go**, plus the C build of libwebp.

---------------------------------------------------------------------------------------------------

## 8. Rust crates

**Safe (deterministic, spec-defined output):**
* `xxhash-rust` (feature xxh64, seed 0) or `twox-hash`: xxhash64 for source hashes and for
  hashstructure.
* `crc32fast` (IEEE) for PNG chunk CRCs. `adler`/`adler32` for the zlib trailer.
* `cc` to compile vendored libwebp 1.3.2 with `-O2`, no defines, the same file list, and the same
  clang. Compiling with `cargo build --release` flags (-O3) should not change results because there
  is no `-ffast-math`, but pinning `-O2` is safest.
* `png` crate (decode only): lossless, so the pixel values are exact. You must map it to Go image
  kinds yourself (TC8 → RGBA, TCA8 → NRGBA, Paletted with Go palette semantics: tRNS gives
  NRGBA entries, out-of-range indices extend the palette with opaque black, 16-bit gives
  RGBA64/NRGBA64). Turn off any gamma, sRGB or expansion transforms (use `Transformations::IDENTITY`).
* `rayon` for parallel image processing. The output does not depend on order, provided keys use
  cold semantics.

**NOT safe (different bytes):**
* `image`'s `imageops::resize`, `fast_image_resize`, `resize`, `imageproc`: different kernels,
  fixed-point u8 or u16 math, different edge handling, no FMA matching.
* `jpeg-decoder`, `zune-jpeg`, `mozjpeg`, `turbojpeg`, `image`'s JPEG decoder: different IDCT
  (stb, AAN or jsimd), chroma upsampling and colour conversion. Go gives YCbCr planes, uses its own
  Loeffler IDCT, and the gift float conversion does nearest-neighbour chroma.
* `jpeg-encoder`, `image::codecs::jpeg`, `mozjpeg`: different FDCT and quantisation rounding, and
  they write a JFIF APP0 (Go writes none), use optimised or different Huffman tables, and have
  different subsampling averaging.
* `png` crate (encode), `flate2`, `miniz_oxide`, `zlib-rs`, `libdeflater`, `zopfli`: different
  deflate token streams and block splitting, different filter heuristics, different IDAT chunking.
* `webp` / `libwebp-sys` / `image-webp`: `libwebp-sys` bundles a different libwebp version
  (1.4/1.5 at the time of writing) and `image-webp` is a pure-Rust encoder (lossless only). Both
  give different bytes. Only a vendored 1.3.2 with the same config is safe.
* `gif` crate encoder: not needed. If GIF is ever needed, port Go `image/gif` (its LZW and
  palette quantisation differ).

---------------------------------------------------------------------------------------------------

## 9. Parity risks, ordered by impact

1. **Cold versus warm cache naming** (§0.3, §3.9). Always emulate cold: `Key()` of every processed
   image created in this run includes `_<decimal root xxhash>`. The Rust port must not "fix" the
   read path either, because the golden output is cold. Keep the Rust cache separate from Go's
   `resources/_gen/images`, or ignore it, and regenerate the golden with `--ignoreCache` or an
   empty `_gen/images`.
2. **FMA placement** (§5). About 0.75% of files (11) change if any of the `center` or `resizeLine`
   fusions is missed. The setter and Over fusions did not matter for this data, but they are
   still required. Validate with the `fmaexp` and `fullref` harnesses on amd64 and arm64.
3. **Go version coupling.** go1.27's new jpeg DCT and new flate change every JPEG and PNG byte
   compared with Go ≤1.25. The golden binary was built with go1.27.1, so porting 1.27.1 sources is
   mandatory. If the golden is ever rebuilt with another Go version, re-verify.
4. **flate `EstimatedBits` and `writeBlockHuff` floats** (§6.2). The fused forms are needed for
   table-reuse decisions on large PNGs. They did not flip on this data (the amd64 port matched),
   but they could on others.
5. **Imaging config hash** (§4.3): the `_merge:"none"` keys are part of it. Any config-loader
   difference in the Rust port (key case, `_merge` injection, value types such as bool versus
   string) renames every image. Coordinate with the config agent.
6. **hashstructure type fidelity**: template int versus float literals, struct type names, and the
   rule that `hashFinishUnordered` runs after each included field. Getting any of these wrong
   renames every filtered image. Use the §4.5 vectors as unit tests.
7. **Go image-type mapping in decoders** (RGB PNG → RGBA gives the premultiplied getter and setter
   path and an RGBA destination; RGBA PNG → NRGBA; JPEG → YCbCr with the exact subsample ratio;
   palette length and entry kinds). A wrong type changes both the gift arithmetic and the PNG
   colour type.
8. **libwebp build**: use exactly 1.3.2, preset 2, `use_sharp_yuv=1`, ImportRGBA. Watch for
   `libm pow/log` in sharpyuv gamma tables and VP8L cost estimation when building on Linux (glibc
   and Apple libm can differ in the last ulp). This is untested; macOS arm64 and x86_64 agreed.
   `-ffp-contract` behaviour of the C compiler matters as well: keep clang with its default
   (`on`). gcc defaults to `fast` for non-ISO modes, so pass `-ffp-contract=on` or `off`
   explicitly and verify.
9. **Publishing rules**: publish only what RelPermalink or Permalink touches (the carousel webp and
   the resized watermarks must not appear). Always re-encode, even for same-size resizes.
10. **Overlay semantics**: the overlay source is decoded from the watermark's *encoded PNG*, and
    each filter source is decoded from its parent's *encoded* bytes (JPEG round trip). Keeping
    in-memory pixels instead gives different results.
11. **NaN handling in Over for transparent-on-transparent pixels** (§5.4): f32→int conversion must
    produce 0 for NaN (Rust `as` does).
12. **Go-side dynacache eviction** (a theoretical nondeterminism in the reference implementation):
    do not emulate it.

---------------------------------------------------------------------------------------------------

## 10. Open questions

* Should the Rust port reproduce the warm-cache naming bug when a Go-populated
  `resources/_gen/images` exists? The recommendation is no: the golden is cold, so be cold always.
  This needs sign-off from the orchestrator. The golden-regeneration procedure should add
  `--ignoreCache`.
* Should the port generalise beyond seeksnack now (Fill/Fit/Crop/smartcrop, other resampling
  kernels, GIF, EXIF) or defer? Each has its own FMA sites that must be taken from disassembly
  first (for example `resources/images/resampling.go:99,111,124,194,197` and `bcspline`).
* Linux build of libwebp (glibc libm) has not been verified against the macOS golden.
