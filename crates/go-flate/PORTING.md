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

Tests: `redteam.rs::fma_sensitive_blocks` (16 inputs whose output depends on
these fusions; unfusing any one of the three `token.rs` sites changes at
least 5 of them). See "FMA sensitivity" below.

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

## Go behaviour reproduced on purpose

* **Dictionary bytes in a stored first block (levels 7-9).** `NewWriterDict`
  calls `fillWindow`, which puts the dictionary at `window[0:n]` and sets
  `s.index = n` but leaves `d.blockStart` at 0. When the first block then
  falls back to a stored block (incompressible data and a small dictionary),
  `writeBlock` passes `window[blockStart:index]` and the stored block holds
  dictionary + data, so the stream inflates to more bytes than were written
  (go1.27.1, `deflate.go:162-169` and `:203-253`). The port does the same;
  `redteam.rs::go_quirk_dictionary_in_stored_block` pins the bytes and the
  inflated result.

## Tests and parity evidence

Every expected value comes from the Go oracle (`tools/go-oracle/go-flate`,
built with go1.27.1) or from Go's own test goldens; `cargo test` needs no Go
toolchain. Default `cargo test -j 2` runs everything below that is not marked
*ignored* (about 1 minute; `[profile.test]` uses `opt-level = 2`).

**Case lines.** Most compressor fixtures share the format
`id wrapper level data dict ops outlen outfnv wlogfnv results` (see the
header of `main.go`): `ops` is a program such as `W100,F,W5x20,C,R,W0,C`
(Write n bytes / n bytes k times, Flush, Close, Reset to the same sink);
`wrapper` may carry a failing sink (`!cK`: the K-th Write call fails, `!pK`:
every call from the K-th fails, `!bN`: at most N bytes are accepted). Each
case compares the output length and FNV-1a 64, the FNV of the sequence of
underlying Write-call sizes (the IDAT chunking contract) and the per-op
`o`/`e` result. Data specs: `gen:KIND:SIZE:SEED` (16 generators, Go
`genData` = Rust `gen_data`, pinned by `genhash.txt`), `+`-joined
concatenations of them with `/POS=VAL` byte assignments, `go:...` (the
inputs of Go's own tests), `file:NAME`.

### Checked in

Compressor (all fixtures from the linux/arm64 oracle or byte-identical to
it, see "Regenerating fixtures"):

| test | fixture (`tests/fixtures/`) | cases | what |
|---|---|---|---|
| `oracle_cases.rs::oracle_cases_small` | `cases_small.txt` | 6,196 | `cases -profile small`: every level (-2…9) × 10 generators × 43 sizes (0…131,071, one Write); 200,000/300,000-byte inputs; fixed chunkings 1…100,000; 700 random Write/Flush/Reset/Close programs (flate and zlib, nil / empty / generated dictionaries incl. > 32 KiB, use after Close); zlib header per level |
| `oracle_cases.rs::oracle_cases_files` | `cases_files.txt` | 480 | `files -programs 3` over the 8 files in `files/` (Go testdata, seeksnack HTML/XML/CSS/PNG/JPEG) |
| `fuzz_cases.rs::fuzz_cases` | `fuzz_cases.txt` | 1,500 | `fuzz -n 1500 -seed 1 -max 150000`: generators 10-15 (Fibonacci-skewed symbols, exact periods around 32 KiB, sparse, counters, block-boundary segments, filtered PNG rows), sizes at block/window boundaries, flush-heavy / boundary / lifecycle programs, failing sinks |
| `fuzz_cases.rs::go_test_programs` | `gotests.txt` | 3,948 | `gotests`: the programs of Go's `TestBestSpeed` (every level, ± Flush), `TestBestSpeedMaxMatchOffset`, `TestWriterReset`, `TestWriteError`, `TestWriterPersistentWriteError`, `TestDeflateFast_Reset`, `TestDeterministic`, `TestRegression2508` (128 MiB), `TestMaxStackSize`, `TestVeryLongSparseChunk` (64 MiB), byte-exact |
| `fuzz_cases.rs::gen_data_matches_oracle` | `genhash.txt` | 288 | `genhash`: generator parity |
| `long_streams.rs::long_streams_random` | `longstreams_random.txt` | 7 × 64 MiB | levels 7, 8, 9, 1, 6, -2, 0 on fresh random bytes (uint16 `literalCounter` wrap, `skipLiterals`) |
| `redteam.rs::sweep_flush_positions` | `redteam/sweep_flushpos.txt` | 11,976 | `enc-sweep -part flushpos -seed 1`: Flush after every prefix of 300-byte inputs (the ≤ 32 stored / < 128 Huffman-only / fast / lazy sync paths), two Flushes in 3,000 bytes (flate + zlib), Flush at 32 KiB / 64 KiB boundaries ± 2 of 140,000-byte inputs; every level |
| `redteam.rs::sweep_close_positions` | `redteam/sweep_closepos.txt` | 7,224 | `-part closepos`: Close after every prefix, then Write / Flush / Close on the closed writer, Reset, the rest; flate + zlib |
| `redteam.rs::sweep_dictionaries` | `redteam/sweep_dicts.txt` | 2,016 | `-part dicts`: 42 dictionary sizes (0 … 100,000, around 4, 258, 32 KiB, 64 KiB) × every level; unrelated, same-vocabulary and prefix-of-data dictionaries; Flush, Close + Reset (dictionary re-applied), chunked writes |
| `redteam.rs::sweep_writer_errors` | `redteam/sweep_errsweep.txt` | 2,548 | `-part errsweep`: 60 programs (every level, flate / zlib, ± dictionary, Write / Flush / Close / Reset, Write and Flush after a failed Close), each with the sink failing at every Write call once (`!c`) and persistently (`!p`), and at 12 byte limits (`!b`) |
| `redteam.rs::sweep_sizes` | `redteam/sweep_sizes.txt` | 756 | `-part sizes`: k·32,768, k·65,535, k·65,536 ± 1 (k = 1, 2, 3, 5, 8) as one Write, window-sized Writes or small Writes; 1-, 2-, 3-, … 257-byte Writes against one Write |
| `redteam.rs::fma_sensitive_blocks` | `redteam/fma_cases.txt` | 16 | `specs < fma_cases.spec`: see "FMA sensitivity" |
| `redteam.rs::go_quirk_dictionary_in_stored_block` | `redteam/go_quirks.txt` | 9 | `specs < go_quirks.spec`, plus inflating case 0 back to dictionary + data |
| `redteam.rs::invalid_level_errors` | | 20 | error strings of `NewWriter`, `NewWriterDict`, `zlib.NewWriterLevel`, `NewWriterLevelDict` for levels -3, 10, 100 and the i32 extremes, as printed by go1.27.1 |
| `png_streams.rs::golden_site_pngs` | `png/` | 11 | the Go-encoded PNGs of the golden seeksnack build: IDAT inflated with our zlib reader, filtered rows recompressed with one Write per row through a `bufio.Writer(1<<15)` port: zlib bytes **and** IDAT chunk sizes identical |
| `go_unit.rs` (12 tests) | `go_testdata/` | | Go's `TestDeflate`, `TestBlockHuff` (9 goldens), `TestWriteBlock`, `TestWriteBlockDynamic`, `TestWriteBlockDynamicSync` (`*.expect*` goldens, with and without input, after reset, EOF marker), `TestWriterClose`, invalid levels, zlib header bytes per level, `TestWriterPersistentWrite/Flush/CloseError`, `TestWriteError` |

Decompressor and checksums:

| test | fixture | cases | what |
|---|---|---|---|
| `inflate_cases.rs::inflate_oracle_cases` | `inflate_cases.txt` | 3,000 | `inflate-cases -n 3000` (seed 99): Go-compressed streams (all levels, dictionaries incl. wrong / missing ones) truncated, bit-flipped or extended; output, per-Read `(n, err)` log, input bytes consumed, final Read / Close errors |
| `inflate_cases.rs` (5 tests) | | | Go's `TestStreams` (28 vectors), `TestTruncatedStreams`, `TestReaderTruncated`, `TestReset`, `TestResetDict`, zlib `TestDecompressor` |
| `inflate_gen.rs::inflate_gen_cases` | `inflate_gen.{bin,txt}` | 600 | `inflate-gen -n 600 -seed 11 -maxblock 2000 -maxdict 3000`: random valid streams Go's encoder never writes (random prefix codes up to 15 bits, degenerate / empty distance trees, RLE and plain code lengths, length 258 as code 284 + 31, distances at the window / dictionary limit), optionally mutated |
| `redteam.rs::inflate_exhaustive_mutations` | `redteam/inflate_exh.{bin,txt}` | 120 bases, 110,289 decodes | `inflate-exh -n 120 -seed 1 -maxlen 300`: every truncation and every single-bit flip of Go-compressed, random valid and hand-made defective streams (distances beyond the history, distance codes 30/31, literal codes 286/287, HLIT/HDIST out of range, missing EOB code, incomplete / oversubscribed / empty / single-code trees, 15-bit codes, block type 3, stored LEN/NLEN mismatches, missing final block, sync markers, 36 KB of output through the 32 KiB window, trailing garbage), flate and zlib, ± dictionary, with per-call Read sizes 0, 1, 7, …, 70,000 from a seeded schedule; one hash over all decodes of a base |
| `redteam.rs::inflate_reset_chains` | `redteam/inflate_reset.{bin,txt}` | 300 chains | `inflate-reset -n 300 -seed 1`: one flate / zlib reader reused through `Reset` over 2-7 streams (valid, mutated, wrong dictionaries, zlib header errors), stopping after 1-5 Reads or at the first error |
| `redteam.rs::zlib_every_header` | `redteam/zlib_hdr.txt` | 393,216 decodes | `zlib-hdr`: all 65,536 two-byte headers × dictionary ID (right, `1`, truncated) × reader dictionary (none, "dictionary") |
| `redteam.rs::adler32_boundaries` | `redteam/adler.txt` | 2,544 lengths | `adler`: Adler-32 of 0xff… and random data for lengths 0-300, k·5,552 ± 5 (k ≤ 200) and 2^s ± 1 (s ≤ 22), one-shot and written in random pieces |

Unit tests in `src/`: `TestBulkHash4`, `bestSpeedMatch`, shift offsets,
`TestDictDecoder`, `reverseBits`, `TestIssue5915/5962/6255`,
`TestInvalidBits`, `TestInvalidEncoding`, `TestReaderEarlyEOF`,
`TestNlitOutOfRange`, the `mFastLog2` float32 constants, Adler-32 goldens.

### Out-of-repo corpora (`--ignored`)

The ignored tests read large corpora from environment variables:
`GO_FLATE_CASES` (+ `GO_FLATE_FILES`: directory of the `file:` inputs)
for `oracle_cases.rs::big_corpus`, `GO_FLATE_FUZZ=a.txt:b.txt` for
`fuzz_cases.rs::fuzz_corpus`, `GO_FLATE_REDTEAM_CASES=a.txt:b.txt` for
`redteam.rs::redteam_case_corpus` (any case-line file),
`GO_FLATE_INFLATE_GEN=a.bin:a.txt,…`, `GO_FLATE_EXH=a.bin:a.txt,…`,
`GO_FLATE_RESET=a.bin:a.txt,…`, `GO_FLATE_PNG_DIR`; `long_streams.rs` needs
none. `GO_FLATE_EXH_DETAIL=<base>` prints every decode of one base (compare
with `oracle inflate-exh … -detail <base>`). Run them with
`cargo test --release --test <file> -- --ignored`.

First port (darwin/arm64 Go): `cases -profile big` 15,516 cases;
`files` over 283 real files (seeksnack output, Go testdata) 13,584 cases;
2,724 Go-written PNGs (201 seeksnack images, PngSuite `basn*`, image/png
bench images × 4 compression levels × 3 colour models) with identical zlib
streams and IDAT chunk boundaries; `longstreams.txt` (levels 1-6: 2.5 GB
through one writer, 70,000 Write/Close/Reset cycles; `fastGen.cur` table
shift / clear) 12/12. First red-team: `longstreams_lazy.txt` (levels 7-9,
600 MiB, the `hashOffset > maxHashOffset` rebase twice) 3/3; fuzz seed 1
checked in (the handoff listed seeds 4-11 as not yet checked; no result
is recorded for seeds 2-3).

Second red-team (linux/amd64 host; "arm64" = linux/arm64 Go under
`qemu-aarch64-static`), 0 differences in every run:

| corpus | seeds | cases | oracle |
|---|---|---|---|
| `fuzz` (n = 5,000, max = 400,000) | 4-11 | 40,000 | arm64; the amd64 oracle writes identical lines for all 40,000 |
| `fuzz` (n = 5,000, max = 400,000) | 12-15 | 20,000 | arm64 |
| `enc-sweep`, all five parts | 1 (checked in), 2, 3 | 73,486 | arm64 |
| `enc-sweep`, all five parts | 2-9 | 195,756 | amd64 |
| `inflate-exh` (maxlen 400) | 101 (n = 300), 102-105 (n = 3,000) | 12,300 bases, 12,351,957 decodes | amd64 (decoding is float-free; the base streams are stored in the corpus) |
| `inflate-reset` | 101 (n = 300), 102-105 (n = 3,000) | 12,300 chains | amd64 |
| `inflate-gen` (maxblock 8,000, maxdict 5,000) | 201, 202 | 40,000 streams | amd64 |
| every checked-in fixture except the slow `longstreams.txt` / `longstreams_lazy.txt`, regenerated with both oracles | | | arm64: byte-identical; amd64: identical except `fma_cases.txt` (and the architecture in the new files' header lines) |

### FMA sensitivity

`EstimatedBits` (`token.go:203`) is the only float computation whose result
reaches the output: in `writeBlockDynamic`, when a previous dynamic table is
still open, it estimates the cost of a new table and the block reuses the
old table or starts a new one. (`writeBlockHuff`'s `abs` sum is exact with or
without FMA: `diff` has at most 24 significant bits and the sum stays below
2^33 in steps of 2^-16.) Before this pass no fixture depended on it: all
fixtures regenerated with amd64 Go (no fusion) were byte-identical, and so
were fuzz seeds 4-11, so replacing the three `mul_add`s in `token.rs` by
plain arithmetic would have passed every test. The go-png pass found no
arm64/amd64 difference in 2,000 large images either.

A search in a scratch copy of the crate, instrumented to evaluate both the
fused and the unfused estimate at every reuse decision:

* random inputs (20,000 cases, 8.7 GB, levels 1-9): 25,271 decisions, 138
  with a different estimate, **0** with a different decision (a flip needs
  the estimate to differ *and* the reuse / new-table gap to be exactly at
  the boundary; the targeted search below needed about 70,000
  evaluations per flip);
* targeted: two 65,535-byte blocks, block 1 mixing generators X and Y,
  block 2 = m bytes of Y + 65,535 - m bytes of X; bisecting m to where the
  gap changes sign and scanning ±300 around it. Levels 1-6: more than 1.1 M
  evaluations found 16 distinct flipping inputs (some searches unfused only
  one of the three FMA sites); levels 7-9: 87,674 evaluations, 70,558
  decisions, 0 flips (their blocks end after 32,768 tokens, which this
  construction does not control; the decision code is shared).

All 16 are checked in (`fma_cases.spec` → `fma_cases.txt`, arm64 oracle).
12 of them are written differently by linux/amd64 Go; the Rust port matches
linux/arm64 in all 16. Mutation check: unfusing only mFastLog2's FMADDS
changes 7 of the 16 outputs, only its FNMSUBS 5, only the shannon FMADDS 9,
all three 12.

### Regenerating fixtures

Compression output is float-sensitive (above), so fixtures must come from an
arm64 build. linux/arm64 reproduces every darwin/arm64 fixture byte for
byte; on an x86_64 host run it under qemu-user. The decompressor-only
corpora (`inflate-exh`, `inflate-reset`, `inflate-gen`, `zlib-hdr`,
`adler`) may use amd64 because their inputs are stored in the corpus, but the
checked-in ones come from arm64 too.

```sh
export GOTOOLCHAIN=go1.27.1
S=/tmp/go-flate-oracle; mkdir -p $S
GOARCH=arm64 CGO_ENABLED=0 go build -o $S/oracle ./tools/go-oracle/go-flate
O="qemu-aarch64-static $S/oracle"        # on an arm64 host: O=$S/oracle
F=crates/go-flate/tests/fixtures; R=$F/redteam

$O genhash > $F/genhash.txt
$O cases -profile small > $F/cases_small.txt
(cd $F/files && $O files -programs 3 e.txt f0011_seeksnack.css f0025_logo.png \
   f0160_index.html f0199_index.xml f0216_mildy_strawberry_hu_773457e87f21d02e.jpg \
   gettysburg.txt Isaac.Newton-Opticks.txt) > $F/cases_files.txt   # this file order
$O fuzz -n 1500 -seed 1 -max 150000 > $F/fuzz_cases.txt
$O gotests $F/files/Isaac.Newton-Opticks.txt > $F/gotests.txt
$O inflate-cases -n 3000 > $F/inflate_cases.txt
$O inflate-gen -n 600 -seed 11 -maxblock 2000 -maxdict 3000 -bin $F/inflate_gen.bin > $F/inflate_gen.txt
# long streams: the oracle prints a shorter header line; the rest must match.
$O longstreams -levels 7,8,9,1,6,-2,0 -noresets -variant random -total 67108864   # longstreams_random.txt
$O longstreams -levels 7,8,9 -noresets -total 629145600                            # longstreams_lazy.txt (slow)
$O longstreams                                                                     # longstreams.txt (slow)

for p in flushpos closepos dicts errsweep sizes; do
  $O enc-sweep -part $p -seed 1 > $R/sweep_$p.txt
done
$O inflate-exh -n 120 -seed 1 -maxlen 300 -bin $R/inflate_exh.bin > $R/inflate_exh.txt
$O inflate-reset -n 300 -seed 1 -bin $R/inflate_reset.bin > $R/inflate_reset.txt
$O zlib-hdr > $R/zlib_hdr.txt
$O adler > $R/adler.txt
$O specs -note "FMA-sensitive: EstimatedBits fused (arm64) vs unfused (amd64) choose different blocks" \
  < $R/fma_cases.spec > $R/fma_cases.txt
$O specs -note "Go quirks reproduced on purpose" < $R/go_quirks.spec > $R/go_quirks.txt

$O png -out $S/pngs IMAGE...          # PNG corpus for png_streams.rs::png_corpus
```

Larger out-of-repo runs use other seeds / sizes of the same modes
(`fuzz -seed N`, `enc-sweep -seed N`, `inflate-exh -n 3000 -seed N -maxlen 400
-bin …`, `inflate-reset -n 3000 -seed N -bin …`) and the ignored tests
above. The golden PNGs (`png/`) and `go_testdata/` are copied, not
generated.

Speed: the Rust port runs the long-stream workload (15 GB) in 70 s vs Go's
139 s CPU time (release, darwin/arm64).

## Known gaps

* `compress/gzip` is not ported (not used by the seeksnack build path).
* The zlib/flate readers do not wrap a non-`BufRead` reader automatically
  (Rust has no dynamic `io.ByteReader` check); see the reader contract above.
