# go-flate — porting notes

Byte-exact port of **go1.27.1** `compress/flate` (the new klauspost-derived
compressor *and* the inflater), `compress/zlib` and the parts of
`hash/adler32` that zlib uses. No dependencies.

## Go file → Rust module map

| Go (go1.27.1 `$GOROOT/src/...`)            | Rust                              |
|--------------------------------------------|-----------------------------------|
| `compress/flate/deflate.go`                | `src/flate/deflate.rs` (`Compressor`, `Writer`, `new_writer`, `new_writer_dict`) |
| `compress/flate/deflatefast.go`            | `src/flate/deflatefast.rs` (`FastEnc` enum = Go `fastEnc` interface, `FastGen`, `hash_len`, `match_len`, …) |
| `compress/flate/load_store.go`             | `src/flate/deflatefast.rs` (`load_le8/32/64`, `store_le64`) |
| `compress/flate/regmask_other.go`          | `src/flate/huffman_bit_writer.rs` (`shl64`: arm64 `reg8SizeMask64 = 0xff`, i.e. Go shift semantics) |
| `compress/flate/level1.go` … `level6.go`   | `src/flate/level1.rs` … `level6.rs` |
| `compress/flate/huffman_bit_writer.go`     | `src/flate/huffman_bit_writer.rs` |
| `compress/flate/huffman_code.go`           | `src/flate/huffman_code.rs`       |
| `compress/flate/token.go`                  | `src/flate/token.rs`              |
| `compress/flate/inflate.go`                | `src/flate/inflate.rs` (`Decompressor`, `new_reader`, `new_reader_dict`) |
| `compress/flate/dict_decoder.go`           | `src/flate/dict_decoder.rs`       |
| `compress/zlib/writer.go`                  | `src/zlib/writer.rs`              |
| `compress/zlib/reader.go`                  | `src/zlib/reader.rs`              |
| `hash/adler32/adler32.go`                  | `src/adler32.rs`                  |
| Go error values (`io.EOF`, `CorruptInputError`, …) | `src/error.rs` (`Error`)  |

Every ported function carries a `// Go: <path>:<Func>` marker.

## Public API (for downstream crates, e.g. go-image's PNG encoder/decoder)

```rust
use go_flate::{flate, zlib, adler32, Error};

// Compression (Go: flate.NewWriter / NewWriterDict, zlib.NewWriter*)
let mut w = flate::new_writer(sink, level)?;            // level: -2..=9, -1 = default (6)
let mut w = flate::new_writer_dict(sink, level, dict)?;
w.write(&buf)?;  w.flush()?;  w.close()?;  let old = w.reset(new_sink);
w.get_ref(); w.get_mut(); w.into_inner();

let mut z = zlib::new_writer(sink);                      // DefaultCompression
let mut z = zlib::new_writer_level(sink, level)?;
let mut z = zlib::new_writer_level_dict(sink, level, Some(dict))?;  // None = Go nil dict
z.write(..)?; z.flush()?; z.close()?; z.reset(sink); z.into_inner();

// Decompression (Go: flate.NewReader / zlib.NewReader). R: std::io::BufRead
let mut f = flate::new_reader(r);  let mut f = flate::new_reader_dict(r, dict);
let (n, err) = f.read(&mut buf);   // Go-style (n, error); Error::Eof at end
f.close()?;  f.reset(r, dict);
let mut zr = zlib::new_reader(r)?;  let mut zr = zlib::new_reader_dict(r, Some(dict))?;
let (n, err) = zr.read(&mut buf);  zr.close()?;  zr.reset(r, dict)?;

adler32::checksum(&data); adler32::Digest::new().write(..).sum32();
```

Constants: `flate::{NO_COMPRESSION, BEST_SPEED, BEST_COMPRESSION, DEFAULT_COMPRESSION,
HUFFMAN_ONLY}` and the same names in `zlib`.

All writers/readers also implement `std::io::Write` / `std::io::Read`
(`Write::flush` is Go's `Flush`, i.e. it emits a sync marker; `Read` maps
`Error::Eof` to `Ok(0)` and returns data before a pending error).

### Contract with the underlying writer (important for PNG IDAT chunking)

Every Go `w.Write(p)` call on the underlying `io.Writer` becomes exactly one
`write_all(p)` call on the Rust `W` (a zero-length Go write becomes one
`write(&[])` call). The sequence and sizes of write calls are therefore
identical to Go's; this is verified by the oracle (`wlogfnv` column) and by
the PNG test, where a port of `bufio.Writer` (size 1<<15) placed between the
zlib writer and the IDAT sink reproduces Go's IDAT chunk boundaries. A
downstream `bufio.Writer` port must implement `std::io::Write::write` like Go
(always consume the whole slice).

### Contract with the underlying reader

Go's `flate.Reader` (`io.Reader` + `io.ByteReader`) is `std::io::BufRead`.
When Go is handed a plain `io.Reader`, `makeReader` wraps it in
`bufio.NewReader` (4096 bytes): callers must do the same with
`std::io::BufReader::with_capacity(4096, r)` to reproduce Go's read pattern on
the underlying reader (e.g. image/png passes its chunk reader, which is not a
ByteReader). A `&[u8]` / `&mut &[u8]` behaves like Go's `*bytes.Reader`.

## FMA sites (darwin/arm64, verified with `go tool objdump`)

Checked in the golden binary's disassembly
(`$S/work/images/flate.objdump`) and in the oracle binary built with
go1.27.1 (identical instruction sequences). These are the only fused
instructions in `compress/flate`.

| Go source | Instruction | Rust |
|---|---|---|
| `token.go:195` `mFastLog2`: `(-0.34484843)*uval+2.02466578` | `FMADDS` | `(-0.34484843_f32).mul_add(uval, 2.02466578)` |
| `token.go:195` `(...)*uval - 0.67487759` | `FNMSUBS` (= `x*y - z`) | `t.mul_add(uval, -0.67487759)` |
| `token.go:195` `log2 += ...` | plain `FADDS` | `log2 += t` |
| `token.go:213/221/232` `shannon += min(15, max(1, -mFastLog2(n*invTotal))) * n` | `FMADDS` (`n*invTotal` is a separate, rounded `FMULS`) | `clamped.mul_add(n, shannon)` |
| `token.go:217` `shannon += 15` | plain `FADDS` | `shannon += 15.0` |
| `huffman_bit_writer.go:993` `diff := float64(v) - avg` (`avg = len/256` compiled to `len * (1/256)`) | `FMSUBD` | `(-len).mul_add(1.0/256.0, v)` (exact either way) |
| `huffman_bit_writer.go:994` `abs += diff * diff` | `FMADDD` | `diff.mul_add(diff, abs)` |

The float32 constants are asserted against the binary's `$f32.beb08ff9`,
`$f32.40019420`, `$f32.3f2cc4c7` symbols (`token.rs` tests). Operand order
was checked from the `FMOVS` constant addresses (`F6 = 2.02466578`,
`F7 = -0.34484843` in `FMADDS F5, F6, F7, F8`, i.e. `F8 = F6 + F7*F5`).

## Deliberate deviations (none affect output bytes)

* **Slice capacity emulation.** Go's `huffmanEncoder.codes` has `len == size`
  and `cap == next power of two`, and Go code reads/writes past `len`
  through reslicing (`h.codes[:len(freq)]`, `lenCodes[lengthCodesStart:][:32]`,
  `offCodes[:32]`). The Rust `codes` vector has the full capacity length;
  stale entries beyond `len(freq)` are kept exactly like Go (they matter
  after the `literalEncoding`/`tmpLitEncoding` swap in `writeBlockHuff`).
  `fastGen.hist` capacity is tracked in `hist_cap` (0 or `allocHistory`),
  because `addBlock`'s move-down decision depends on `cap(e.hist)`.
* **Interfaces / function pointers → enums.** `fastEnc` → `FastEnc`;
  `compressor.fill/step` → `Mode`; `decompressor.step` → `Step`;
  `f.hl`/`f.hd` pointers → `Hl` + `hd_fixed`.
* **Ownership of the writer.** Go's `close()` does `d.w.reset(nil)`; the Rust
  port keeps `W` (never written again because `d.err = errWriterClosed`) so
  callers can retrieve it with `into_inner`. `reset` returns the previous
  `W`. zlib's `z.w` and the compressor share one `io.Writer` in Go; in Rust
  the compressor owns it once created (`Sink` enum).
* **Errors** are values of `Error` (Clone, sticky); `Display` reproduces
  Go's `Error()` strings. `Write` returns `Err(e)` where Go returns `(0, e)`.
  Readers expose Go's `(n, err)` via inherent `read`.
* **`advancedState` borrow split.** `deflateLazy` temporarily moves the
  state out of the compressor (`deflate_lazy` → `deflate_lazy_inner`) and
  passes it explicitly to `findMatch`, `tryBetterMatchAtEnd`,
  `skipLiterals`; semantics unchanged.
* **Sorting.** `slices.SortFunc` in `huffman_code.go` sorts keys that are
  unique (literal values are unique), so `sort_unstable_by_key` yields Go's
  order.
* `readFlush` returns an index range instead of a subslice.
* Out-of-contract Go behaviours (panics on impossible states) are Rust
  panics at the same places (index out of range, `"invalid level
  specified"`, `"leafCounts[maxBits][maxBits] != n"`).
* The deprecated `flate.ReadError`/`WriteError` types (never returned) and the
  `Resetter` interfaces (inherent `reset` methods instead) are not ported.
  `writeBlockSkip` (dead code in Go) is ported but unused.

## Tests and parity evidence

All from Go oracles / Go's own goldens; `cargo test` needs no Go toolchain.

* `tests/oracle_cases.rs::oracle_cases_small` — 6 196 cases from
  `tools/go-oracle/go-flate cases -profile small`: every level (-2…9) × 10
  data generators × 43 sizes (0 … 131 071, single write), inputs up to
  300 000 bytes, fixed chunkings (1 … 100 000 bytes), 700 random
  write/Flush/Reset/Close programs (flate and zlib, nil/empty/generated
  dictionaries incl. > 32 KiB, use-after-close). Each case compares output
  length + FNV-64, the FNV-64 of the underlying Write-call sizes and the
  per-op ok/error string.
* `tests/oracle_cases.rs::oracle_cases_files` — 480 cases on real files
  (Go testdata `e.txt`, `gettysburg.txt`, `Isaac.Newton-Opticks.txt`, a
  seeksnack HTML page, XML, CSS, PNG, JPEG).
* `tests/png_streams.rs::golden_site_pngs` — the 11 Go-encoded PNGs of the
  golden seeksnack build: IDAT inflated with our zlib reader, filtered rows
  recompressed with one `Write` per row through a `bufio.Writer(1<<15)`
  port: zlib bytes **and** IDAT chunk sizes identical.
* `tests/inflate_cases.rs::inflate_oracle_cases` — 3 000 streams (flate/zlib,
  all levels, dictionaries incl. wrong/missing ones) truncated, bit-flipped
  or extended; compares output, the per-`Read` `(n, err)` log, input bytes
  consumed and the final `Read`/`Close` error strings (e.g.
  `flate: corrupt input before offset 112`).
* `tests/go_unit.rs` — Go's `TestDeflate`, `TestBlockHuff` (9 goldens),
  `TestWriteBlock`, `TestWriteBlockDynamic`, `TestWriteBlockDynamicSync`
  (`testdata/*.expect*` goldens, with and without input, after reset, EOF
  marker), `TestWriterClose`, zlib header bytes per level.
* `tests/inflate_cases.rs` — Go's `TestStreams` (28 vectors),
  `TestTruncatedStreams`, `TestReaderTruncated`, `TestReset`,
  `TestResetDict`, zlib `TestDecompressor` table.
* Unit tests: `TestBulkHash4`, `reverseBitsTests`, `TestIssue5915/5962/6255`,
  `TestInvalidBits`, `TestInvalidEncoding`, adler32 goldens.

Large corpora (outside the repo, `$S/work/go-flate/`), run with
`--ignored`:

* `cases_big.txt` — 15 516 cases (profile `big`: 77 sizes up to 262 144
  single-write, 300 000 … 4 MiB inputs for all levels, 6 000 random
  programs): **0 failures**.
* `cases_files_big.txt` — 13 584 cases over 283 real files (seeksnack
  golden output: HTML, XML, JS, CSS, JSON, fonts, ICO, PNG, JPEG, WebP; Go
  testdata): **0 failures**.
* `pngs/` — 2 724 PNGs (2.5 GB) written by Go's image/png from 201
  seeksnack source images, the PngSuite `basn*` images and the image/png
  bench images, at all 4 `png.CompressionLevel`s × 3 colour models
  (original, NRGBA, Gray): zlib stream **and** IDAT chunk boundaries
  identical for **all 2 724** (`png_streams.rs::png_corpus`).
* `tests/long_streams.rs` (`--ignored`, expected values checked in as
  `tests/fixtures/longstreams.txt`) — for levels 1-6: 2.5 GB through one
  writer (with occasional Flush) and 70 000 Write/Close/Reset cycles, which
  push `fastGen.cur` past `bufferReset` and exercise the table-shift and
  table-clear paths: **12/12 identical** (output hash, Write-call log).

Regenerate fixtures with:

```sh
go build -o $S/work/go-flate/oracle ./tools/go-oracle/go-flate
$S/work/go-flate/oracle cases -profile small > crates/go-flate/tests/fixtures/cases_small.txt
$S/work/go-flate/oracle files -programs 3 crates/go-flate/tests/fixtures/files/* > crates/go-flate/tests/fixtures/cases_files.txt
$S/work/go-flate/oracle inflate-cases -n 3000 > crates/go-flate/tests/fixtures/inflate_cases.txt
$S/work/go-flate/oracle longstreams > crates/go-flate/tests/fixtures/longstreams.txt   # ~2 min
$S/work/go-flate/oracle png -out $S/work/go-flate/pngs IMAGE...                    # PNG corpus
```

Speed: the Rust port runs the long-stream workload (15 GB) in 70 s vs Go's
139 s CPU time (release, darwin/arm64).

## Known gaps

* `compress/gzip` is not ported (not used by the seeksnack build path).
* The zlib/flate readers do not wrap a non-`BufRead` reader automatically
  (Rust has no dynamic `io.ByteReader` check); see the reader contract above.
