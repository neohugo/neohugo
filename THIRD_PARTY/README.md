# Third-party material cargo cannot see

`tools/neohugo/licence-check.sh` checks every crate in the dependency graph. The material below
is not a crate licence of its own (data bundled inside a crate, files we embed), so its licence
texts and sources are kept here (REWRITE_PLAN.md §5). Each entry names the task that adds it.

| Directory | Material | Licence | Added by |
|---|---|---|---|
| `hugo/` | Hugo (gohugoio/hugo, forked as neohugo): the embedded templates rewritten in Tera (`crates/layouts/embedded/`) derive from Hugo's `tpl/tplimpl/embedded/templates` (this repository at commit `44529028`) | Apache-2.0 (`hugo/LICENSE`) | T00 (licence), T32 (templates) |
| `two-face/` | Syntax definitions and themes bundled in the two-face 0.5.2 dump (bat's assets), each with its own licence; `ACKNOWLEDGEMENTS.md` is two-face's listing | per asset (MIT, Apache-2.0, BSD, …; `two-face/ACKNOWLEDGEMENTS.md`) | T25 |
| `cldr/` | Unicode CLDR 48.2.1 data compiled into the ICU4X 2.3 data crates (collation, plurals, numbers, dates) used by `crates/locale` | Unicode-3.0 (`cldr/LICENSE`) | T12 |
| `emoji/` | Emoji data compiled in by the `emojis` crate 0.8.2 (used by `emojify` and comrak's `shortcodes`): Unicode 17.0 emoji data and GitHub gemoji v4.1.0 short codes | Unicode-3.0 (`emoji/LICENSE-UNICODE`), MIT (`emoji/LICENSE-GEMOJI`) | T22 (entry), T70 (directory) |
| `livereload/` | `livereload.min.js` served by `neohugo server` (`crates/serve/assets/`): livereload-js 4.0.2 with core-js 2.6.12 modules, and Hugo's LiveReload plugin, as Hugo bundles them | MIT (`livereload/LICENSE`: livereload-js, core-js); Apache-2.0 (the plugin, `hugo/LICENSE`) | T71 |
| `flect/` | Inflection word lists and rules from gobuffalo/flect v1.0.3, transcribed into `crates/base/src/inflect.rs` | MIT (`flect/LICENSE`) | T10 |
| `prose/` | Title-case word lists and rules from jdkato/prose v1.2.1, transcribed into `crates/base/src/title.rs` | MIT (`prose/LICENSE`) | T10 |
| `gofont/` | `Go-Regular.ttf`, the default font of the text filter (the font Hugo embeds, `golang.org/x/image/font/gofont` v0.28.0), embedded by `crates/images/src/font.rs` | BSD-3-Clause, Bigelow & Holmes (`gofont/LICENSE`, the `ttfs/README` that ships with the font) | T72a |
| `x-image/` | golang.org/x/image v0.28.0 (`font/sfnt`, `font/opentype`, `vector`): the 26.6 metrics, the GPOS/`kern` kerning subset and its quirks, and the coverage quantisation that `crates/images/src/font.rs` reproduces | BSD-3-Clause (`x-image/LICENSE`) | T72a |
| `rsc-qr/` | rsc.io/qr v0.2.0: the PNG writer (1-bit, one fixed-Huffman deflate block) rewritten in `crates/images/src/qr.rs`, and the encoder choices (mode, version, mask 0) it reproduces with the `qrcode` crate | BSD-3-Clause (`rsc-qr/LICENSE`) | T72a |
| `hashstructure/` | gohugoio/hashstructure v0.5.0: the structure-hash rules of Hugo's `hashing.HashString`, re-implemented in `crates/resources/src/gohash.rs` (GetRemote cache names, QR code names) | MIT (`hashstructure/LICENSE`) | T40, T72a |
| `go/` | The Go standard library (Go 1.24.7 sources, `src/image/jpeg/writer.go`): the JPEG writer ported to `crates/images/src/jpeg.rs` (tables, colour conversion, 4:2:0 subsampling, quantisation, Huffman coding, markers). Its forward DCT (`fdct.go`, from the IJG's `jfdctint.c`) is not ported: the DCT is computed exactly, as Go 1.26+ approximates it, so no IJG code is used | BSD-3-Clause (`go/LICENSE`, The Go Authors) | F9 |
