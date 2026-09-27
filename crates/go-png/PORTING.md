# go-png — porting notes

Byte-exact port of the **go1.27.1** `image/png` package (decoder and
encoder) on top of `crates/go-image` (image types, colour models, the
`image.Decode` registry) and `crates/go-flate` (`compress/zlib` and the
klauspost-derived `compress/flate` of go1.27). neohugo uses it for every PNG
it reads (`image.Decode`) and writes
(`png.Encoder{CompressionLevel: png.DefaultCompression}` in
`resources/images/image.go`).

## Go file → Rust module map

| Go (go1.27.1 `$GOROOT/src/image/png/…`) | Rust |
|---|---|
| `reader.go` constants (`ct*`, `cb*`, `ft*`, `it*`, `ds*`), `interlacing`, `pngHeader`, `FormatError`/`UnsupportedError`/`chunkOrderError` | `src/reader.rs` (constants), `src/error.rs` (`Error`) |
| `reader.go` `decoder`, `parseIHDR`, `parsePLTE`, `parsetRNS`, `Read`, `decode`, `readImagePass`, `mergePassInto`, `parseIDAT`, `parseIEND`, `parseChunk`, `verifyChecksum`, `checkHeader`, `Decode`, `DecodeConfig`, `init` | `src/reader.rs` (`Decoder`, `ChunkIo` = the fields `(*decoder).Read` shares with the chunk parser, `IdatReader` = the decoder as the zlib reader's `io.Reader`, `read_image_pass`, `merge_pass_into`, `decode`, `decode_config`, `register`) |
| `writer.go` `Encoder`, `EncoderBufferPool`, `EncoderBuffer`/`encoder`, `CompressionLevel` + constants, `opaque`, `abs8`, `writeChunk`, `writeIHDR`, `writePLTEAndTRNS`, `(*encoder).Write`, `filter`, `writeImage`, `writeIDATs`, `levelToZlib`, `writeIEND`, `Encode`, `(*Encoder).Encode` | `src/writer.rs` (`EncState` = an `*encoder` during one `Encode`; `IdatSink` + `drain` = `(*encoder).Write`, see deviation 2) |
| `paeth.go` `intSize`, `abs`, `paeth`, `filterPaeth` | `src/paeth.rs` |
| `bufio` (go1.27.1): `Reader.fill/readErr/Read/ReadByte`, `Writer.Write/Flush/Reset/Available/Buffered` | `src/bufio.rs` (the parts `image/png` depends on, see below) |
| `paeth_test.go` (`TestPaeth`, `TestPaethDecode`) | `src/paeth.rs` unit tests |
| `reader_test.go`, `fuzz_test.go` (seed corpus) | `tests/go_reader_tests.rs` |
| `writer_test.go` (+ `BenchmarkEncodeGrayWithBufferPool` as a test) | `tests/go_writer_tests.rs` |

Every ported function carries a `// Go: <path>:<Func>` comment.

### Why `bufio` is ported here

* **Decoder.** `zlib.NewReader(d)` wraps the decoder (an `io.Reader` without
  `ReadByte`) in `bufio.NewReader` (4096 bytes). How much IDAT data that
  buffer has pulled when the zlib stream ends decides `too much pixel data`,
  which chunk header/CRC errors are reported, and how an error of the
  caller's reader surfaces (as is, or wrapped as
  `png: invalid format: <err>` when the zlib reader meets it while reading
  the Adler-32 trailer). The result therefore depends on the caller's read
  chunking — Go's own result differs between `bytes.Reader`,
  `iotest.OneByteReader` and `iotest.HalfReader` for ~2 % of corrupt
  inputs, and the port reproduces all three (`record` fixtures).
* **Encoder.** The zlib stream goes through `bufio.NewWriterSize(e, 1<<15)`
  whose every `Write` on the encoder becomes one IDAT chunk, so the large
  write bypass and `Flush` decide the IDAT chunk boundaries (32768 + rest in
  the golden build).

## Public API

```rust
use go_png::{decode, decode_config, register, encode, Encoder, CompressionLevel,
             EncoderBuffer, EncoderBufferPool, Error, PNG_HEADER,
             DEFAULT_COMPRESSION, NO_COMPRESSION, BEST_SPEED, BEST_COMPRESSION};

let img: Box<dyn go_image::Image> = decode(&mut reader)?;    // png.Decode
let cfg: go_image::Config = decode_config(&mut reader)?;     // png.DecodeConfig
register();                // image.RegisterFormat("png", …) for go_image::decode
encode(&mut writer, &*img)?;                                 // png.Encode
Encoder { compression_level: DEFAULT_COMPRESSION, buffer_pool: None }
    .encode(&mut writer, &*img)?;                            // (*png.Encoder).Encode
```

* `decode`/`decode_config` take `&mut dyn std::io::Read`; `encode` takes
  `&mut dyn std::io::Write` and a `&dyn go_image::Image`.
* `CompressionLevel(pub i64)` with Go's four constants; positive values mean
  `DefaultCompression`, as in Go.
* `EncoderBufferPool` is a `Send + Sync` trait (`get() -> Option<Box<EncoderBuffer>>`,
  `None` = Go's nil; `put(Box<EncoderBuffer>)`), held as
  `Option<Arc<dyn EncoderBufferPool>>`.
* `Error` (`Display` = Go's `Error()` strings): `Format` (`png: invalid
  format: …`), `Unsupported` (`png: unsupported feature: …`), `Eof`,
  `UnexpectedEof`, `NoProgress`, `Flate(go_flate::Error)` (`zlib: invalid
  header`, `flate: corrupt input before offset N`, …) and `Io` (the caller's
  reader/writer error, unchanged).

### Decoded image types (Go `readImagePass`)

| PNG | Go / Rust type |
|---|---|
| gray 1/2/4/8 | `Gray` (`NRGBA` with a tRNS chunk) |
| gray 16 | `Gray16` (`NRGBA64` with tRNS) |
| gray+alpha 8 / 16 | `NRGBA` / `NRGBA64` |
| truecolor 8 | `RGBA` (`NRGBA` with tRNS) |
| truecolor 16 | `RGBA64` (`NRGBA64` with tRNS) |
| truecolor+alpha 8 / 16 | `NRGBA` / `NRGBA64` |
| paletted 1/2/4/8 | `Paletted`; entries are `color::RGBA{…, 0xff}`, the first `len(tRNS)` become `color::NRGBA`; pixel indices ≥ the PLTE length extend the palette with opaque black (per pass, merged by `mergePassInto`); `decode_config` reports the PLTE (+tRNS) palette only |

Rects start at (0,0); strides are the minimal ones (`NewXxx`).

## Deliberate deviations (none changes a byte or an error string)

1. **Readers and writers.** Rust readers signal EOF with `Ok(0)`, so a Go
   reader returning `(0, nil)` (`io.ErrNoProgress` after 100 tries in
   `bufio.fill`) cannot be expressed; in `bufio.fill` the 100-try limit
   applies to `ErrorKind::Interrupted` instead, which is retried without
   limit everywhere else. Every Go
   `w.Write(p)` is one `write_all(p)` (a zero-length Go write is one
   `write(&[])`), so the sequence of writes is Go's; a Rust writer returning
   `Ok(0)` fails with `WriteZero` where Go (whose `io.Writer` contract forbids
   short writes without an error) would carry on.
2. **The IDAT sink.** In Go the `*encoder` itself is the `io.Writer` behind
   the `bufio.Writer`, and its `Write` writes an IDAT chunk synchronously.
   The pooled zlib writer cannot hold the caller's borrowed writer, so the
   port records the `bufio.Writer`'s writes in an `IdatSink` and turns each
   into an IDAT chunk (`writeChunk`, same three `Write`s) as soon as the
   zlib/bufio call that produced it returns. The caller's writer sees the
   same writes in the same order, the first failing write stops everything
   exactly like Go's `e.err`, and `Encode` returns the same error. Only
   unobservable internal state differs after a write error (Go's flate
   compressor records the error; in the port the sink and the
   `bufio.Writer` do); `Reset` clears both on the next pooled `Encode`
   (`pool` fixtures include failing writes).
3. **`e.bw` lives inside `e.zw`.** Go keeps the `bufio.Writer` and the zlib
   writer as separate `EncoderBuffer` fields; the port keeps the
   `bufio.Writer` as the zlib writer's `W` and takes it out/back in
   `writeIDATs`. Same buffer (1<<15), same `Reset`/re-creation rules.
4. **Row buffers.** Go copies the `cr` array of slice headers and swaps
   `pr`/`cr[0]` locally, so the `EncoderBuffer` keeps the original headers;
   the port stores the swapped buffers back. Their contents are never read
   before being rewritten (`pr` is cleared, `cr[0]` fully written, a filter
   buffer is only chosen after being fully written), so output is the same.
5. **nil palettes.** `go_image::color::Palette` cannot distinguish Go's nil
   palette from an empty one. For an `*image.Paletted` with a nil palette Go's
   encoder panics (nil colour from `At`); the port returns
   `png: invalid format: bad palette length: 0`, Go's result for a non-nil
   empty palette.
6. **64-bit `int`.** The branches that only fire with a 32-bit `int`
   (`nPixels64 != int64(nPixels)`, `rowSize != int64(int(rowSize))`,
   `int(d.idatLength) < 0`) are constant-false and left as comments.
7. **Types.** `readImagePass` returns a private enum (`PassImage`) instead of
   `image.Image`; `mergePassInto`'s `*image.Alpha`/`*image.Alpha16` cases
   (never produced) are omitted. `decode` returns `Box<dyn Image>`
   (`downcast_ref::<T>()` for the concrete type). The impossible
   `d.palette[i].(color.RGBA)` assertion failure is an `unreachable!`.
8. **Registration** is explicit (`register()`, idempotent) instead of the
   package `init`.
9. **`bufio`**: only the methods `image/png` uses are ported;
   `lastByte`/`lastRuneSize` (for `Unread*`) are not tracked.

## FMA and platform dependence

`image/png` contains no floating-point code (`go tool objdump` of the arm64
oracle binary: no float instructions in `image/png`). The encoder's output
does depend on the float code of the go1.27 flate compressor
(`EstimatedBits` for multi-block Huffman-table reuse, the `writeBlockHuff`
incompressibility check); go-flate replicates the darwin/arm64 FMA
fusions (see `crates/go-flate/PORTING.md`).

All checked-in fixtures were generated by a **linux/arm64** build of the
oracle run under `qemu-aarch64` (`GOTOOLCHAIN=go1.27.1 GOOS=linux
GOARCH=arm64`), whose flate code has the same `FMADDS`/`FNMSUBS`/
`FMSUBD`/`FMADDD` instructions at the same sites as the golden darwin/arm64
binary (`go tool objdump` of the oracle vs the list in go-flate's
PORTING.md: `token.go:195/213/232`, `huffman_bit_writer.go:993/994`; qemu
implements fused multiply-add exactly). Every fixture and every
out-of-repo corpus below was also generated with the linux/amd64 oracle for
comparison: **the outputs are identical** on this data, i.e. none of these
encodings hits a Huffman-table decision the FMA changes (the
`writeBlockHuff` check cannot differ at all: for flate block sizes every
`diff*diff` is exact in float64, so fused and unfused sums round the same).
A dedicated search
(`fmasearch`, below) found no such encoding either. The golden-site test
compares against the golden darwin/arm64 files themselves, independently of
the oracle. Do not regenerate the encoder fixtures on amd64 without
comparing against an arm64 run.

## Tests and parity evidence

`cargo test` (offline, no Go toolchain) runs 35 tests (7 more are
`#[ignore]`d big-corpus runs). The differential tests compare **217,650**
oracle values with 0 differences.

| test | oracle command | coverage |
|---|---|---|
| `oracle_encode::synth` | `synth 0 2000 200` | 16 image kinds — RGBA/RGBA64 with valid and invalid premultiplied alpha, NRGBA, NRGBA64, Alpha, Alpha16, Gray, Gray16, CMYK, Paletted (1–266 entries incl. 0 and >256, every colour type as entries, out-of-range indices), YCbCr and NYCbCrA (all ratios), sub-images with odd offsets, `image.Rectangle`, an image with no optional methods (generic `At` paths, pixel-scan `opaque`), a `PalettedImage` that is not `*image.Paletted` — sizes 0…200, non-zero origins; encoded at Default/No/BestSpeed/BestCompression/7, round-trip decode, failing writer (16,000 checks) |
| `oracle_encode::filters` | `filters 0 4000` | filter-heuristic edge cases: few-valued rows (incl. 127/128/129 around `abs8`'s sign), repeated/constant/ramp/delta rows, every bytes-per-pixel (1,2,3,4,6,8), widths 1…40; per-row filter types + encodings at 4 levels (24,000) |
| `oracle_encode::pool` | `pool 0 400 120` | 400 sequences of 2–7 encodes through one `Encoder` whose `BufferPool` returns the same `EncoderBuffer` (zlib writer reuse/re-creation across levels, stale row buffers, failing writers mid-sequence) (1,760) |
| `oracle_encode::gotests` | `gotests` | the images of Go's `TestWriterPaletted`, `TestWriterLevels`, `TestSubImage`, `TestWriteRGBA`, the six encode benchmarks and `ExampleEncode` (several 640x480, multi-block streams), palette-length and image-size errors, at 4 levels (120) |
| `oracle_decode::files` | `files . <list>` | 75 files: Go's image/png testdata incl. PngSuite (every colour type, bit depth, interlacing, tRNS), image/testdata PNGs, the ExampleDecode gopher, the 11 golden seeksnack PNGs, 9 repository PNGs (1-bit QR, paletted favicons, RGB/RGBA icons, Hugo test images): DecodeConfig, Decode, image.Decode, encodings at 4 levels, of NRGBA/RGBA/Gray/WebSafe/Plan9[:16]/{Black,Transparent} conversions (go-image `draw`), of an odd sub-image, failing writer (1,152) |
| `oracle_decode::truncated` | `trunc . <list>` | prefixes of 65 fixture files (every prefix of files ≤ 1200 bytes; otherwise the first and last 300 and ~300 in between): DecodeConfig and Decode error strings (85,180) |
| `oracle_decode::reader_errors` | `errread . <list>` | Decode through a reader that fails after k bytes, same k as `trunc` (42,590) |
| `oracle_decode::generated_corpus` | `mkpng 0 3000` + `record` | 3,000 generated PNGs (`gen.bin.gz`): every valid colour type/bit depth plus invalid combinations, Adam7, random filters and bad filter types, PLTE/tRNS variants (too long, odd length, misplaced, repeated, on non-paletted images), gAMA/tEXt chunks, IDAT split at random points, leading/trailing/empty IDATs, truncated or extended zlib data, bad Adler-32, bad CRCs, missing/odd IEND, trailing garbage; DecodeConfig, Decode, image.Decode, Decode via `iotest.OneByteReader` and `iotest.HalfReader`, default re-encoding (16,726) |
| `oracle_decode::mutated_corpus` | `mutate 0 6000` + `record` | 6,000 structure-aware mutations of `gen.bin` (bit flips, truncation, chunk data/length/order changes incl. huge lengths, chunk insertion/deletion/duplication, IHDR field changes, IDAT corruption with fixed CRCs), rebuilt in Rust by a port of the mutator (30,122) |
| `golden_site` | — | the 11 PNGs of the golden seeksnack build (darwin/arm64) decode and re-encode (DefaultCompression) to **identical bytes**: 128x128…600x480, multi-block streams (float `EstimatedBits` path), IDAT chunks of 32768 + rest |
| `go_reader_tests` | — | Go's `TestReader` (PngSuite vs `.sng` dumps via a port of the test's `sng` writer), `TestReaderError`, `TestPalettedDecodeConfig`, `TestInterlaced`, `TestIncompleteIDATOnRowBoundary`, `TestTrailingIDATChunks`, `TestMultipletRNSChunks`, `TestUnknownChunkLengthUnderflow`, `TestPaletted8OutOfRangePixel`, `TestGray8Transparent`, `TestDimensionOverflow` (incl. the 512 MB `Decode` case), `TestDecodePalettedWithTransparency`, `FuzzDecode` over its seeds; plus an interlaced paletted image with out-of-range indices (palette extension across passes) |
| `go_writer_tests` | — | `TestWriter`, `TestWriterPaletted`, `TestWriterLevels`, `TestSubImage`, `TestWriteRGBA`, `BenchmarkEncodeGrayWithBufferPool` as a byte-identity test |
| unit tests | — | `TestPaeth`, `TestPaethDecode` (Go's math/rand replaced by xorshift: the test compares two implementations), `abs`, `bufio` reader/writer |

Source coverage of these tests (llvm-cov, `-C instrument-coverage`):
reader 97.0 %, writer 96.1 %, paeth 100 %, bufio 87.1 % of lines; the rest
are `unreachable!` arms, `bufio` paths the decoder/encoder never take (empty
reads, `Flush` failures of the IDAT sink, which never fails, see deviation
2), `NoProgress` and the 32-bit-only `chunk is too large` error.

Mutation checks (to confirm the fixtures bite): flipping the Sub filter's
tie-break (`<` → `<=`) fails 1,513 checks of the decode suites alone (their
re-encodings); dropping the `d.idatLength != 0` part of `too much pixel
data` fails 386.

### Out-of-repo corpora (`#[ignore]`d tests, `GO_PNG_BIG=<dir>`)

Generated with the arm64 oracle (qemu) and the amd64 oracle; the two
outputs were identical for every corpus. Put the files (names below, plain
TSV) in one directory and run
`GO_PNG_BIG=<dir> cargo test -- --ignored`. All ran with **0
differences** (1,557,467 checks):

| test | oracle command | checks |
|---|---|---|
| `synth_big` | `synth 0 20000 400` → `synth.tsv` | 160,000 |
| `filters_big` | `filters 0 50000` → `filters.tsv` | 300,000 |
| `pool_big` | `pool 0 3000 200` → `pool.tsv` | 13,490 |
| `corpus_big` | `mkpng 0 30000 gen.bin` + `record` → `gen.tsv`; `mutate 0 100000 gen.bin mut.bin` + `record` → `mut.tsv` | 168,298 + 502,928 |
| `files_big` | `files <repo> <list>` → `files.tsv` (+ `files.root` = repo path): the 95 other PNGs tracked in this repository (docs screenshots up to 3000x1500 and 1.5 MB, an interlaced 1500x750 RGB, paletted/RGB/RGBA/1-bit) plus the 75 fixture PNGs | 2,672 |
| `truncated_big` | `trunc <repo> <list>` → `trunc.tsv` (decode-only; amd64) | 273,386 |
| `reader_errors_big` | `errread <repo> <list>` → `errread.tsv` (decode-only; amd64) | 136,693 |

`fmasearch 0 2000` (Go only): 2,000 images of 150–650 px per side (every
`genBase` kind, multi-block streams) at the four levels gave identical bytes
on arm64 and amd64, i.e. no encoding found whose bytes depend on the flate
FMA sites.

## Regenerating fixtures

From the repository root, with `qemu-user-static` installed (Linux x86_64;
on an arm64 host drop the qemu wrapper):

```sh
export GOTOOLCHAIN=go1.27.1
GOOS=linux GOARCH=arm64 go build -o /tmp/png-arm64 ./tools/go-oracle/go-png
go build -o /tmp/png-amd64 ./tools/go-oracle/go-png      # for comparison only
O="qemu-aarch64-static /tmp/png-arm64"
cd crates/go-png/tests/fixtures
find gotestdata golden repo -name '*.png' | LC_ALL=C sort > /tmp/files.list
ls gotestdata/*.png gotestdata/pngsuite/*.png gotestdata/image/*.png golden/*.png repo/*.png \
  | grep -v -E 'bench|video-001\.(221212|cmyk|progressive|rgb)' > /tmp/trunc.list
$O files . /tmp/files.list      | gzip -9n > files.tsv.gz
$O synth 0 2000 200             | gzip -9n > synth.tsv.gz
$O filters 0 4000               | gzip -9n > filters.tsv.gz
$O pool 0 400 120               | gzip -9n > pool.tsv.gz
$O gotests                      > gotests.tsv
$O trunc . /tmp/trunc.list      | gzip -9n > trunc.tsv.gz
$O errread . /tmp/trunc.list    | gzip -9n > errread.tsv.gz
$O mkpng 0 3000 /tmp/gen.bin && gzip -9nc /tmp/gen.bin > gen.bin.gz
$O record /tmp/gen.bin          | gzip -9n > gen.tsv.gz
$O mutate 0 6000 /tmp/gen.bin /tmp/mut.bin && $O record /tmp/mut.bin | gzip -9n > mut.tsv.gz
```

(`mkpng`/`mutate` write inputs, not outputs, and are platform-independent.)
Then run the same commands with `/tmp/png-amd64` and compare: a difference
there is an FMA-sensitive encoding, and the arm64 output is the one to keep.
The out-of-repo corpora are the commands of the table above with outputs
written uncompressed into `$GO_PNG_BIG`; `fmasearch <seed0> <n>` is only
useful as an arm64-vs-amd64 diff.

Fixture provenance: `gotestdata/` is Go's `image/png/testdata` (incl.
PngSuite, see `pngsuite/README`), `image/testdata/*.png` and the
`example_test.go` gopher (BSD, `gotestdata/LICENSE-go`); `golden/` holds
the 11 PNGs of the golden build (same files as
`crates/go-flate/tests/fixtures/png/`); `repo/` holds PNGs copied from this
repository (`docs/static`, `resources/testdata`, `tpl/images/testdata`,
`resources/images/testdata`).

## Known gaps

* The seeksnack **source** PNGs (private repository) are not in the corpus;
  the 11 golden outputs are, and round-trip exactly.
* Custom `color.Color` implementations cannot be expressed (go-image's
  `Color` is a closed enum), so palettes/images with exotic colour types
  are covered only for the standard types.
* A Go reader that returns data together with an error, or `(0, nil)`, has
  no `std::io::Read` equivalent (deviation 1).
* None of the fixtures, out-of-repo corpora or golden files contains an
  encoding whose bytes differ between the FMA (arm64) and non-FMA (amd64)
  flate paths, so this crate's tests cannot tell a correct FMA port from a
  non-FMA one; that is covered by go-flate's own tests (see its
  PORTING.md).
