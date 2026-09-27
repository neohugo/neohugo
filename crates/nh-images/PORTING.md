# nh-images — porting notes

neohugo resources/images/** (image config, spec parsing, processing, filters, encoders, exif stub).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `config` | `resources/images/config.go` | T10 images |  |
| `image` | `resources/images/image.go` | T10 images |  |
| `filters` | `resources/images/filters.go` | T10 images |  |
| `overlay` | `resources/images/overlay.go` | T10 images |  |
| `process` | `resources/images/process.go` | T10 images |  |
| `resampling` | `resources/images/resampling.go` | T10 images | only box is exercised; other kernels have their own FMA sites (verify with objdump before porting) |
| `color` | `resources/images/color.go` | T10 images |  |
| `image_resource` | `resources/images/image_resource.go` | T10 images |  |
| `auto_orient` | `resources/images/auto_orient.go` | T10 images | STUB |
| `dither` | `resources/images/dither.go` | T10 images | STUB |
| `mask` | `resources/images/mask.go` | T10 images | STUB |
| `opacity` | `resources/images/opacity.go` | T10 images | STUB |
| `padding` | `resources/images/padding.go` | T10 images | STUB |
| `smartcrop` | `resources/images/smartcrop.go` | T10 images | STUB (smartcrop not exercised) |
| `text` | `resources/images/text.go` | T10 images | STUB |
| `exif` | `resources/images/exif/exif.go` | T10 images | MINIMAL (excludeFields='.*' => nothing decoded) |
| `webp` | `resources/images/webp/webp.go` | T10 images |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-resource
- Wave A (to add when available): gift, go-image, go-png, go-flate, libwebp-sys, go-hashstructure
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
