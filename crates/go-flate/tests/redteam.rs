//! Second red-team pass (tools/go-oracle/go-flate/redteam.go):
//!
//! * compressor: Flush / Close at every position, every dictionary size
//!   class, the underlying writer failing at every Write call and byte
//!   count, exact 32 KiB / 64 KiB multiples and byte-at-a-time writes
//!   (`enc-sweep`), FMA-sensitive block decisions and regression cases
//!   (`specs`);
//! * decompressor: every truncation and single-bit flip of small streams
//!   with random per-call Read sizes (`inflate-exh`), reader reuse through
//!   `Reset` (`inflate-reset`), every 2-byte zlib header (`zlib-hdr`);
//! * Adler-32 at the NMAX boundaries (`adler`).
//!
//! `read_size`, `decode_sched` and the case enumeration mirror the oracle.

mod common;

use std::cell::Cell;
use std::collections::HashMap;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use common::{Rng, check_cases, fnv64, gen_data, parse_cases};
use go_flate::{Error, adler32, flate, zlib};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/redteam")
        .join(name)
}

fn err_string(e: &Option<Error>) -> String {
    match e {
        None => "<nil>".to_string(),
        Some(e) => e.to_string(),
    }
}

/// Streaming FNV-1a 64.
struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf29ce484222325)
    }
    fn write(&mut self, b: &[u8]) {
        for &c in b {
            self.0 ^= c as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}

// ---- case-line fixtures (compressor) ----

fn check_case_file(path: &Path, min_cases: usize) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let cases = parse_cases(&text);
    assert!(
        cases.len() >= min_cases,
        "{}: {} cases",
        path.display(),
        cases.len()
    );
    let failures = check_cases(&cases, &HashMap::new());
    for f in failures.iter().take(40) {
        eprintln!("{f}");
    }
    assert!(
        failures.is_empty(),
        "{}: {} of {} cases differ from Go",
        path.display(),
        failures.len(),
        cases.len()
    );
}

/// `oracle enc-sweep -part flushpos` (arm64): Flush after every prefix of
/// 300-byte inputs, two Flushes in 3000 bytes, Flush near the 32 KiB /
/// 64 KiB boundaries of 140000-byte inputs, every level.
#[test]
fn sweep_flush_positions() {
    check_case_file(&fixture_path("sweep_flushpos.txt"), 11900);
}

/// `oracle enc-sweep -part closepos` (arm64): Close after every prefix,
/// then Write / Flush / Close on the closed writer, Reset, the rest.
#[test]
fn sweep_close_positions() {
    check_case_file(&fixture_path("sweep_closepos.txt"), 7200);
}

/// `oracle enc-sweep -part dicts` (arm64): every dictionary size class
/// (0 … 100000) × level, unrelated / related / prefix-of-data dictionaries,
/// with Flush, Reset and chunked writes.
#[test]
fn sweep_dictionaries() {
    check_case_file(&fixture_path("sweep_dicts.txt"), 2000);
}

/// `oracle enc-sweep -part errsweep` (arm64): the underlying writer fails
/// at every Write call (once / persistently) and at byte limits, in
/// programs with Flush, Close, Write / Flush after a failed Close, Reset.
#[test]
fn sweep_writer_errors() {
    check_case_file(&fixture_path("sweep_errsweep.txt"), 2500);
}

/// `oracle enc-sweep -part sizes` (arm64): sizes k·32768 / k·65535 /
/// k·65536 ± 1, one Write, window-sized Writes, small Writes; byte-at-a-time.
#[test]
fn sweep_sizes() {
    check_case_file(&fixture_path("sweep_sizes.txt"), 700);
}

/// `oracle specs` (arm64) over `fma_cases.spec`: inputs whose table
/// reuse decision depends on the fused multiply-adds of `EstimatedBits`.
/// 12 of the 16 are written differently by linux/amd64 Go (nothing fused);
/// unfusing any one of the three arm64 FMA sites (`token.rs`) changes 5 to
/// 9 of them.
#[test]
fn fma_sensitive_blocks() {
    check_case_file(&fixture_path("fma_cases.txt"), 16);
}

/// Go 1.27.1 quirk reproduced on purpose (`oracle specs` over
/// `go_quirks.spec`, arm64): `NewWriterDict` at levels 7-9 leaves
/// `blockStart` at 0 after `fillWindow`, so a stored first block also
/// stores the dictionary and the stream inflates to dictionary + data.
#[test]
fn go_quirk_dictionary_in_stored_block() {
    let path = fixture_path("go_quirks.txt");
    check_case_file(&path, 9);
    let cases = parse_cases(&std::fs::read_to_string(&path).unwrap());
    let c = &cases[0];
    let files = HashMap::new();
    let out = common::run_case(c, &files).out;
    let data = common::resolve(&c.data, &files);
    let dict = common::resolve(&c.dict, &files);
    let mut dec = flate::new_reader_dict(&out[..], dict.bytes());
    let mut got = Vec::new();
    dec.read_to_end(&mut got).unwrap();
    let mut want = dict.bytes().to_vec();
    want.extend_from_slice(data.bytes());
    assert_eq!(got, want, "Go's stored block holds dict+data");
}

/// Constructor errors for invalid levels, as printed by go1.27.1
/// (`flate.NewWriter`, `NewWriterDict`, `zlib.NewWriterLevel`,
/// `NewWriterLevelDict` with a dictionary), including the i32 extremes.
#[test]
fn invalid_level_errors() {
    for level in [-3, 10, 100, i32::MIN, i32::MAX] {
        let want = format!("flate: invalid compression level {level}: want value in range [-2, 9]");
        let e = flate::new_writer(Vec::new(), level).err().unwrap();
        assert_eq!(e.to_string(), want);
        let e = flate::new_writer_dict(Vec::new(), level, b"x")
            .err()
            .unwrap();
        assert_eq!(e.to_string(), want);
        let want = format!("zlib: invalid compression level: {level}");
        let e = zlib::new_writer_level(Vec::new(), level).err().unwrap();
        assert_eq!(e.to_string(), want);
        let e = zlib::new_writer_level_dict(Vec::new(), level, Some(b"x"))
            .err()
            .unwrap();
        assert_eq!(e.to_string(), want);
    }
}

/// Large case files outside the repository (`GO_FLATE_REDTEAM_CASES=a.txt:b.txt`).
#[test]
#[ignore]
fn redteam_case_corpus() {
    let Ok(paths) = std::env::var("GO_FLATE_REDTEAM_CASES") else {
        eprintln!("GO_FLATE_REDTEAM_CASES not set; skipping");
        return;
    };
    for p in paths.split(':') {
        check_case_file(Path::new(p), 1);
        eprintln!("{p}: ok");
    }
}

// ---- decoding with a Read size schedule ----

/// Go: `readSize` in redteam.go.
fn read_size(r: &mut Rng, mode: u8) -> usize {
    match mode {
        0 => 1,
        1 => 7,
        2 => 4096,
        3 => 70000,
        _ => match r.intn(8) {
            0 => 0,
            1 => 1,
            2 => 1 + r.intn(16),
            3 => 1 + r.intn(300),
            4 => [4095, 4096, 4097, 32767, 32768, 32769][r.intn(6)],
            5 => 1 + r.intn(70000),
            _ => 1 + r.intn(3000),
        },
    }
}

const MAX_READ_CALLS: usize = 1 << 16;

/// Appends "<n>:<err>;" to the call log.
fn log_call(h: &mut Fnv, n: usize, err: &Option<Error>) {
    h.write(format!("{}:{};", n, err_string(err)).as_bytes());
}

/// Go: `decodeSched`.
fn decode_sched(
    zl: bool,
    stream: &[u8],
    dict: &[u8],
    mode: u8,
    seed: u64,
) -> (Vec<u8>, u64, usize, String) {
    let mut cur: &[u8] = stream;
    let mut out = Vec::new();
    let mut h = Fnv::new();
    let mut buf = vec![0u8; 70000];
    let mut r = Rng::new(seed);
    let result;
    {
        enum Rd<'a, 'b> {
            F(flate::Decompressor<&'a mut &'b [u8]>),
            Z(zlib::Reader<&'a mut &'b [u8]>),
        }
        let mut rd = if !zl {
            Rd::F(flate::new_reader_dict(&mut cur, dict))
        } else {
            match zlib::new_reader_dict(&mut cur, Some(dict)) {
                Ok(z) => Rd::Z(z),
                Err(e) => {
                    let consumed = stream.len() - cur.len();
                    return (Vec::new(), 0, consumed, format!("new: {e}"));
                }
            }
        };
        let mut last = "<none>".to_string();
        for _ in 0..MAX_READ_CALLS {
            let size = read_size(&mut r, mode);
            let (n, err) = match &mut rd {
                Rd::F(f) => f.read(&mut buf[..size]),
                Rd::Z(z) => z.read(&mut buf[..size]),
            };
            out.extend_from_slice(&buf[..n]);
            log_call(&mut h, n, &err);
            if err.is_some() {
                last = err_string(&err);
                break;
            }
        }
        let close = match &mut rd {
            Rd::F(f) => f.close().err(),
            Rd::Z(z) => z.close().err(),
        };
        result = format!("{} | close: {}", last, err_string(&close));
    }
    let consumed = stream.len() - cur.len();
    (out, h.0, consumed, result)
}

fn take<'a>(bin: &mut &'a [u8]) -> &'a [u8] {
    let n = u32::from_le_bytes(bin[..4].try_into().unwrap()) as usize;
    let b = &bin[4..4 + n];
    *bin = &bin[4 + n..];
    b
}

fn take_u64(bin: &mut &[u8]) -> u64 {
    let v = u64::from_le_bytes(bin[..8].try_into().unwrap());
    *bin = &bin[8..];
    v
}

fn take_u8(bin: &mut &[u8]) -> u8 {
    let v = bin[0];
    *bin = &bin[1..];
    v
}

// ---- inflate-exh ----

/// Go: `exhCases`.
fn exh_cases(l: usize) -> usize {
    1 + l + 8 * l
}

/// Go: `exhStream`.
fn exh_stream(s: &[u8], k: usize) -> std::borrow::Cow<'_, [u8]> {
    let l = s.len();
    if k == 0 {
        s.into()
    } else if k <= l {
        s[..k - 1].into()
    } else {
        let k = k - (l + 1);
        let mut m = s.to_vec();
        m[k / 8] ^= 1 << (k % 8);
        m.into()
    }
}

fn exh_record(zl: bool, stream: &[u8], dict: &[u8], mode: u8, seed: u64, k: usize) -> String {
    let s = exh_stream(stream, k);
    let seed = seed.wrapping_add((k as u64).wrapping_mul(0x9e3779b97f4a7c15));
    let (out, log, consumed, err) = decode_sched(zl, &s, dict, mode, seed);
    format!(
        "{}:{}:{:016x}:{:016x}:{}:{}\n",
        k,
        out.len(),
        fnv64(&out),
        log,
        consumed,
        err
    )
}

fn check_exh(bin_path: &Path, txt_path: &Path) -> (usize, usize) {
    let bin_all = std::fs::read(bin_path).unwrap();
    let txt = std::fs::read_to_string(txt_path).unwrap();
    let mut bin: &[u8] = &bin_all;
    let detail: Option<usize> = std::env::var("GO_FLATE_EXH_DETAIL")
        .ok()
        .map(|s| s.parse().unwrap());
    let mut failures = Vec::new();
    let (mut bases, mut total) = (0, 0);
    for line in txt.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(' ').collect();
        let w = take_u8(&mut bin);
        let mode = take_u8(&mut bin);
        let seed = take_u64(&mut bin);
        let dict = take(&mut bin);
        let stream = take(&mut bin);
        assert_eq!(f[1].as_bytes()[0], w, "corpus out of sync at {line}");
        let idx: usize = f[0].parse().unwrap();
        let nc = exh_cases(stream.len());
        assert_eq!(nc.to_string(), f[6], "case count at {line}");
        let mut h = Fnv::new();
        for k in 0..nc {
            let rec = exh_record(w == b'z', stream, dict, mode, seed, k);
            if detail == Some(idx) {
                print!("{rec}");
            }
            h.write(rec.as_bytes());
        }
        bases += 1;
        total += nc;
        if format!("{:016x}", h.0) != f[7] {
            failures.push(format!("{line}: got {:016x}", h.0));
        }
    }
    assert!(bin.is_empty(), "trailing corpus data");
    for f in failures.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(
        failures.is_empty(),
        "{} of {bases} inflate-exh base streams differ (GO_FLATE_EXH_DETAIL=<id> prints the cases; oracle: -detail <id>)",
        failures.len()
    );
    (bases, total)
}

/// `oracle inflate-exh -n 120 -seed 1 -maxlen 300`: every truncation and
/// single-bit flip of 120 base streams (Go-compressed, random valid,
/// structural defects), random per-call Read sizes; one hash per base.
#[test]
fn inflate_exhaustive_mutations() {
    let (bases, total) = check_exh(
        &fixture_path("inflate_exh.bin"),
        &fixture_path("inflate_exh.txt"),
    );
    eprintln!("{bases} bases, {total} cases");
    assert!(bases >= 120);
}

/// Large corpora: `GO_FLATE_EXH=a.bin:a.txt,b.bin:b.txt`.
#[test]
#[ignore]
fn inflate_exhaustive_corpus() {
    let Ok(spec) = std::env::var("GO_FLATE_EXH") else {
        eprintln!("GO_FLATE_EXH not set; skipping");
        return;
    };
    for pair in spec.split(',') {
        let (b, t) = pair.split_once(':').unwrap();
        let (bases, total) = check_exh(Path::new(b), Path::new(t));
        eprintln!("{t}: {bases} bases, {total} cases ok");
    }
}

// ---- inflate-reset ----

/// A byte source that shares its read position (Go: `*bytes.Reader`, whose
/// `Len` the oracle reads after each segment).
struct Src<'a> {
    data: &'a [u8],
    pos: Rc<Cell<usize>>,
}

impl Read for Src<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let rest = &self.data[self.pos.get()..];
        let n = std::cmp::min(rest.len(), buf.len());
        buf[..n].copy_from_slice(&rest[..n]);
        self.pos.set(self.pos.get() + n);
        Ok(n)
    }
}

impl BufRead for Src<'_> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(&self.data[self.pos.get()..])
    }
    fn consume(&mut self, n: usize) {
        self.pos.set(self.pos.get() + n);
    }
}

struct Seg<'a> {
    stream: &'a [u8],
    dict: &'a [u8],
    max_reads: usize,
    mode: u8,
    seed: u64,
}

/// Go: `runResetChain`.
fn run_reset_chain(zl: bool, segs: &[Seg]) -> String {
    enum Rd<'a> {
        F(flate::Decompressor<Src<'a>>),
        Z(zlib::Reader<Src<'a>>),
    }
    let mut sb = String::new();
    let mut fr: Option<Rd> = None;
    let mut buf = vec![0u8; 70000];
    for (i, s) in segs.iter().enumerate() {
        let pos = Rc::new(Cell::new(0));
        let src = Src {
            data: s.stream,
            pos: pos.clone(),
        };
        let consumed = || pos.get();
        let mut reset_err = "-".to_string();
        match &mut fr {
            Some(Rd::F(f)) => {
                // Go: flate's Reset always returns nil.
                f.reset(src, s.dict);
                reset_err = "<nil>".to_string();
            }
            Some(Rd::Z(z)) => {
                reset_err = err_string(&z.reset(src, Some(s.dict)).err());
            }
            None if !zl => fr = Some(Rd::F(flate::new_reader_dict(src, s.dict))),
            None => match zlib::new_reader_dict(src, Some(s.dict)) {
                Ok(z) => fr = Some(Rd::Z(z)),
                Err(e) => {
                    sb.push_str(&format!("{i} new:{e} consumed={};", consumed()));
                    continue;
                }
            },
        }
        let rd = fr.as_mut().unwrap();
        let mut r = Rng::new(s.seed);
        let mut h = Fnv::new();
        let mut out = Vec::new();
        let mut reads = 0;
        while reads < MAX_READ_CALLS {
            if s.max_reads > 0 && reads == s.max_reads {
                break;
            }
            let size = read_size(&mut r, s.mode);
            let (n, err) = match rd {
                Rd::F(f) => f.read(&mut buf[..size]),
                Rd::Z(z) => z.read(&mut buf[..size]),
            };
            out.extend_from_slice(&buf[..n]);
            log_call(&mut h, n, &err);
            if err.is_some() {
                break;
            }
            reads += 1;
        }
        let close = match rd {
            Rd::F(f) => f.close().err(),
            Rd::Z(z) => z.close().err(),
        };
        sb.push_str(&format!(
            "{i} reset={reset_err} reads={reads} out={}:{:016x} log={:016x} consumed={} close={};",
            out.len(),
            fnv64(&out),
            h.0,
            consumed(),
            err_string(&close)
        ));
    }
    sb
}

fn check_reset(bin_path: &Path, txt_path: &Path) -> usize {
    let bin_all = std::fs::read(bin_path).unwrap();
    let txt = std::fs::read_to_string(txt_path).unwrap();
    let mut bin: &[u8] = &bin_all;
    let mut failures = Vec::new();
    let mut n = 0;
    for line in txt.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.splitn(4, ' ').collect();
        let w = take_u8(&mut bin);
        let nseg = take_u8(&mut bin) as usize;
        assert_eq!(f[1].as_bytes()[0], w, "corpus out of sync at {line}");
        let mut segs = Vec::new();
        for _ in 0..nseg {
            let max_reads = u32::from_le_bytes(bin[..4].try_into().unwrap()) as usize;
            bin = &bin[4..];
            let mode = take_u8(&mut bin);
            let seed = take_u64(&mut bin);
            let dict = take(&mut bin);
            let stream = take(&mut bin);
            segs.push(Seg {
                stream,
                dict,
                max_reads,
                mode,
                seed,
            });
        }
        let rec = run_reset_chain(w == b'z', &segs);
        // The oracle prints the record (-detail) or its hash.
        let want = f[3];
        let got = if want.len() == 16 && !want.contains(' ') {
            format!("{:016x}", fnv64(rec.as_bytes()))
        } else {
            rec.clone()
        };
        if got != want {
            failures.push(format!("{} {}: got {rec}", f[0], f[1]));
        }
        n += 1;
    }
    assert!(bin.is_empty(), "trailing corpus data");
    for f in failures.iter().take(20) {
        eprintln!("{f}");
    }
    assert!(
        failures.is_empty(),
        "{} of {n} Reset chains differ",
        failures.len()
    );
    n
}

/// `oracle inflate-reset -n 300 -seed 1`: one flate / zlib reader reused
/// through Reset over chains of 2-7 streams (valid, mutated, wrong
/// dictionaries, header errors), partial reads before the Reset.
#[test]
fn inflate_reset_chains() {
    let n = check_reset(
        &fixture_path("inflate_reset.bin"),
        &fixture_path("inflate_reset.txt"),
    );
    assert!(n >= 300);
}

/// Large corpora: `GO_FLATE_RESET=a.bin:a.txt,...`.
#[test]
#[ignore]
fn inflate_reset_corpus() {
    let Ok(spec) = std::env::var("GO_FLATE_RESET") else {
        eprintln!("GO_FLATE_RESET not set; skipping");
        return;
    };
    for pair in spec.split(',') {
        let (b, t) = pair.split_once(':').unwrap();
        let n = check_reset(Path::new(b), Path::new(t));
        eprintln!("{t}: {n} chains ok");
    }
}

// ---- zlib-hdr ----

/// Go: `zlibHdrBody` (a stored block with "hello" and its Adler-32).
const ZLIB_HDR_BODY: [u8; 14] = [
    0x01, 0x05, 0x00, 0xfa, 0xff, b'h', b'e', b'l', b'l', b'o', 0x06, 0x2c, 0x02, 0x15,
];

/// Every 2-byte zlib header × dictionary ID (right, 1, truncated) × reader
/// dictionary (none, "dictionary"): `oracle zlib-hdr`.
#[test]
fn zlib_every_header() {
    let text = std::fs::read_to_string(fixture_path("zlib_hdr.txt")).unwrap();
    let dict: &[u8] = b"dictionary";
    let ids: [Vec<u8>; 3] = [
        adler32::checksum(dict).to_be_bytes().to_vec(),
        1u32.to_be_bytes().to_vec(),
        vec![0, 0],
    ];
    let dicts: [&[u8]; 2] = [&[], dict];
    let mut n = 0;
    let mut failures = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let (b0, want) = line.split_once(' ').unwrap();
        let b0: usize = b0.parse().unwrap();
        let mut h = Fnv::new();
        for (vi, id) in ids.iter().enumerate() {
            for (di, d) in dicts.iter().enumerate() {
                for b1 in 0..256usize {
                    let hdr = b0 << 8 | b1;
                    let mut s = vec![b0 as u8, b1 as u8];
                    if hdr & 0x20 != 0 {
                        s.extend_from_slice(id);
                    }
                    s.extend_from_slice(&ZLIB_HDR_BODY);
                    let mode = ((b1 + vi + di) % 5) as u8;
                    let (out, log, consumed, err) = decode_sched(true, &s, d, mode, hdr as u64);
                    let rec = format!(
                        "{vi}.{di}.{hdr}:{}:{:016x}:{:016x}:{consumed}:{err}\n",
                        out.len(),
                        fnv64(&out),
                        log
                    );
                    h.write(rec.as_bytes());
                }
            }
        }
        if format!("{:016x}", h.0) != want {
            failures.push(b0);
        }
        n += 1;
    }
    assert_eq!(n, 256);
    assert!(
        failures.is_empty(),
        "zlib headers differ for first bytes {failures:?} (oracle zlib-hdr -detail <b0>)"
    );
}

// ---- adler ----

/// Go: `adlerLengths`.
fn adler_lengths() -> Vec<usize> {
    let mut ls: Vec<usize> = (0..=300).collect();
    for k in 1..=200usize {
        for d in -5i64..=5 {
            ls.push((5552 * k as i64 + d) as usize);
        }
    }
    for s in 9..=22u32 {
        for d in -1i64..=1 {
            ls.push(((1i64 << s) + d) as usize);
        }
    }
    ls
}

/// Adler-32 of all-0xff and random data at every length 0-300, around
/// every multiple of NMAX (5552) up to 200·5552 and around powers of two,
/// one-shot and written in random pieces: `oracle adler`.
#[test]
fn adler32_boundaries() {
    let text = std::fs::read_to_string(fixture_path("adler.txt")).unwrap();
    let ff = vec![0xffu8; (1 << 22) + 1];
    let rnd = gen_data(1, (1 << 22) + 1, 5);
    let mut r = Rng::new(3);
    let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
    let ls = adler_lengths();
    assert_eq!(lines.len(), ls.len());
    for (line, &l) in lines.iter().zip(ls.iter()) {
        let mut d = adler32::Digest::new();
        let mut pos = 0;
        while pos < l {
            // Go evaluates the slice literal (with its r.intn) before the index.
            let rand_piece = 1 + r.intn(20000);
            let c = [1, 3, 4, 5551, 5552, 5553, rand_piece][r.intn(7)];
            let c = std::cmp::min(l - pos, c);
            d.write(&rnd[pos..pos + c]);
            pos += c;
        }
        let got = format!(
            "{} {:08x} {:08x} {:08x}",
            l,
            adler32::checksum(&ff[..l]),
            adler32::checksum(&rnd[..l]),
            d.sum32()
        );
        assert_eq!(&got, line);
    }
}
