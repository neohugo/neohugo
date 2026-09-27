//! Shared test harness: the oracle's deterministic data generator and the
//! case runner. `gen_data` must stay in sync with `genData` in
//! tools/go-oracle/go-flate/main.go.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Write;

use go_flate::{Error, flate, zlib};

/// splitmix64, identical to the oracle's `rng`.
pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(s: u64) -> Rng {
        Rng { s }
    }
    pub fn next(&mut self) -> u64 {
        self.s = self.s.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn intn(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const WORDS: &[&str] = &[
    "the",
    "of",
    "and",
    "to",
    "in",
    "is",
    "you",
    "that",
    "it",
    "he",
    "was",
    "for",
    "on",
    "are",
    "as",
    "with",
    "his",
    "they",
    "at",
    "be",
    "this",
    "have",
    "from",
    "or",
    "one",
    "had",
    "by",
    "word",
    "but",
    "not",
    "what",
    "all",
    "were",
    "we",
    "when",
    "your",
    "can",
    "said",
    "there",
    "use",
    "an",
    "each",
    "which",
    "she",
    "do",
    "how",
    "their",
    "if",
    "will",
    "up",
    "compression",
    "snack",
    "company",
    "Thailand",
    "<div class=\"card\">",
    "</div>",
    "{{ .Title }}",
    "https://example.com/",
    "\t",
    "0123456789",
    "ประเทศไทย",
    "日本語",
    "naïve",
    "café",
];

pub fn gen_data(kind: usize, size: usize, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed ^ (kind as u64) << 56);
    let mut out: Vec<u8> = Vec::with_capacity(size + 64);
    match kind {
        0 => out.resize(size, 0),
        1 => {
            while out.len() < size {
                out.push(r.next() as u8);
            }
        }
        2 => {
            let k = 2 + r.intn(14);
            let base = r.next() as u8;
            while out.len() < size {
                out.push(base.wrapping_add(r.intn(k) as u8));
            }
        }
        3 => {
            while out.len() < size {
                out.extend_from_slice(WORDS[r.intn(WORDS.len())].as_bytes());
                match r.intn(12) {
                    0 => out.extend_from_slice(b".\n"),
                    1 => out.extend_from_slice(b", "),
                    _ => out.push(b' '),
                }
            }
        }
        4 => {
            let l = 1 + r.intn(64);
            let pat: Vec<u8> = (0..l).map(|_| r.next() as u8).collect();
            let mut i = 0;
            while out.len() < size {
                let mut b = pat[i % l];
                if r.intn(64) == 0 {
                    b = r.next() as u8;
                }
                out.push(b);
                i += 1;
            }
        }
        5 => {
            while out.len() < size {
                let b = r.next() as u8;
                let n = 1 + r.intn(300);
                for _ in 0..n {
                    out.push(b);
                }
            }
        }
        6 => {
            while out.len() < size {
                let mut k = r.intn(9);
                if k == 6 {
                    k = 9;
                }
                let n = 1 + r.intn(20000);
                let seed = r.next();
                out.extend_from_slice(&gen_data(k, n, seed));
            }
        }
        7 => {
            while out.len() < size {
                let mut v = 0usize;
                while r.intn(3) != 0 && v < 255 {
                    v += 1;
                }
                out.push(v as u8);
            }
        }
        8 => {
            let row_len = 1 + 3 * (1 + r.intn(400));
            while out.len() < size {
                out.push(r.intn(5) as u8);
                let mut j = 1;
                while j < row_len && out.len() < size {
                    if r.intn(16) == 0 {
                        out.push(r.next() as u8);
                    } else {
                        out.push((r.intn(7) as i64 - 3) as u8);
                    }
                    j += 1;
                }
            }
        }
        9 => {
            let clen = 1 + r.intn(40000);
            let chunk: Vec<u8> = (0..clen).map(|_| r.next() as u8).collect();
            while out.len() < size {
                match r.intn(3) {
                    0 => out.extend_from_slice(&chunk),
                    1 => {
                        let n = 1 + r.intn(40000);
                        for _ in 0..n {
                            out.push(r.next() as u8);
                        }
                    }
                    _ => {
                        let start = r.intn(chunk.len());
                        out.extend_from_slice(&chunk[start..]);
                    }
                }
            }
        }
        _ => gen_data_ext(&mut r, kind, size, &mut out),
    }
    out.truncate(size);
    out
}

/// Data kinds >= 10 (Go: `genDataExt` in tools/go-oracle/go-flate/fuzz.go).
fn gen_data_ext(r: &mut Rng, kind: usize, size: usize, out: &mut Vec<u8>) {
    match kind {
        10 => {
            // Fibonacci-skewed symbol counts, shuffled: deep Huffman trees
            let nsym = 3 + r.intn(30);
            let syms: Vec<u8> = (0..nsym).map(|_| r.next() as u8).collect();
            let mut fib = vec![0u64; nsym];
            let mut sum = 0u64;
            let (mut a, mut b) = (1u64, 1u64);
            for f in fib.iter_mut() {
                *f = a;
                sum += a;
                (a, b) = (b, a + b);
            }
            for i in 0..nsym {
                let n = (fib[i] * size as u64 / sum) as usize;
                for _ in 0..n {
                    out.push(syms[i]);
                }
            }
            while out.len() < size {
                out.push(syms[nsym - 1]);
            }
            let mut i = out.len();
            while i > 1 {
                i -= 1;
                let j = r.intn(i + 1);
                out.swap(i, j);
            }
        }
        11 => {
            // exact period repeats, optional rare mutations
            const PERIODS: [usize; 21] = [
                1, 2, 3, 4, 5, 7, 8, 16, 255, 256, 257, 258, 259, 1000, 32766, 32767, 32768, 32769,
                32770, 65535, 65536,
            ];
            let p = PERIODS[r.intn(PERIODS.len())];
            let pat: Vec<u8> = (0..p).map(|_| r.next() as u8).collect();
            let m = r.intn(4);
            let mut i = 0;
            while out.len() < size {
                let mut c = pat[i % p];
                if m > 0 && r.intn(1 << (6 + 2 * m)) == 0 {
                    c = r.next() as u8;
                }
                out.push(c);
                i += 1;
            }
        }
        12 => {
            // sparse: zero runs with isolated random bytes
            const GAPS: [usize; 4] = [4, 64, 2000, 70000];
            while out.len() < size {
                let g = GAPS[r.intn(GAPS.len())];
                let gap = r.intn(1 + g);
                let mut j = 0;
                while j < gap && out.len() < size {
                    out.push(0);
                    j += 1;
                }
                out.push(r.next() as u8);
            }
        }
        13 => {
            // counters / binary tables
            let mode = r.intn(4);
            let k = 1 + r.intn(7) as u32;
            let mut i: u32 = 0;
            while out.len() < size {
                match mode {
                    0 => out.push(i.wrapping_mul(k) as u8),
                    1 => {
                        let v = i.wrapping_mul(k) as u16;
                        out.extend_from_slice(&v.to_le_bytes());
                    }
                    2 => {
                        let v = i.wrapping_mul(k).wrapping_mul(2654435761);
                        out.extend_from_slice(&v.to_le_bytes());
                    }
                    _ => {
                        let v = i.wrapping_mul(k);
                        out.extend_from_slice(&v.to_le_bytes());
                    }
                }
                i = i.wrapping_add(1);
            }
        }
        14 => {
            // segments sized around block / window boundaries
            const KINDS: [usize; 6] = [0, 1, 3, 5, 11, 12];
            const LENS: [i64; 6] = [65535, 65536, 32768, 65274, 128, 32];
            while out.len() < size {
                let k = KINDS[r.intn(KINDS.len())];
                let mut n = LENS[r.intn(LENS.len())] + r.intn(9) as i64 - 4;
                if n < 1 {
                    n = 1;
                }
                let seed = r.next();
                out.extend_from_slice(&gen_data(k, n as usize, seed));
            }
        }
        15 => {
            // PNG filtered scanlines of a synthetic image
            const BPPS: [usize; 6] = [1, 2, 3, 4, 6, 8];
            let bpp = BPPS[r.intn(BPPS.len())];
            let width = 1 + r.intn(700);
            let row_len = width * bpp;
            let mut prev = vec![0u8; row_len];
            let mut cur = vec![0u8; row_len];
            let noise = r.intn(4);
            let fx = 1 + r.intn(5);
            let fy = 1 + r.intn(5);
            let flat = r.intn(3) == 0;
            let mut y = 0usize;
            while out.len() < size {
                for x in 0..width {
                    for c in 0..bpp {
                        let mut v = x * fx + y * fy + c * 37;
                        if flat {
                            v = (x / 64) * fx + (y / 64) * fy + c;
                        }
                        if noise > 0 {
                            v += r.intn(1 << noise);
                        }
                        cur[x * bpp + c] = v as u8;
                    }
                }
                let ft = r.intn(5);
                out.push(ft as u8);
                for i in 0..row_len {
                    let (mut a, mut c) = (0u8, 0u8);
                    if i >= bpp {
                        a = cur[i - bpp];
                        c = prev[i - bpp];
                    }
                    let b = prev[i];
                    let f = match ft {
                        0 => cur[i],
                        1 => cur[i].wrapping_sub(a),
                        2 => cur[i].wrapping_sub(b),
                        3 => cur[i].wrapping_sub(((a as usize + b as usize) / 2) as u8),
                        _ => cur[i].wrapping_sub(paeth(a, b, c)),
                    };
                    out.push(f);
                }
                std::mem::swap(&mut prev, &mut cur);
                y += 1;
            }
        }
        _ => panic!("bad kind"),
    }
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let (pa, pb, pc) = (
        (p - a as i32).abs(),
        (p - b as i32).abs(),
        (p - c as i32).abs(),
    );
    if pa <= pb && pa <= pc {
        return a;
    }
    if pb <= pc {
        return b;
    }
    c
}

/// The oracle's sink: all bytes plus the length of every Write call.
/// `mode` b: at most `limit` bytes are accepted in total (a Go partial
/// Write + error becomes a short `Ok` followed by an `Err` on the retry of
/// `write_all`); `mode` c: the `limit`-th call fails, later calls succeed;
/// `mode` p: every call from the `limit`-th on fails.
#[derive(Default)]
pub struct Recorder {
    pub buf: Vec<u8>,
    pub wlog: Vec<usize>,
    pub mode: u8,
    pub limit: usize,
    pub calls: usize,
}

impl Write for Recorder {
    fn write(&mut self, p: &[u8]) -> std::io::Result<usize> {
        self.wlog.push(p.len());
        self.calls += 1;
        match self.mode {
            b'b' => {
                let room = self.limit - self.buf.len();
                if p.len() > room {
                    self.buf.extend_from_slice(&p[..room]);
                    if room > 0 {
                        return Ok(room);
                    }
                    return Err(std::io::Error::other("oracle: sink failure"));
                }
            }
            b'c' => {
                if self.calls == self.limit {
                    return Err(std::io::Error::other("oracle: sink failure"));
                }
            }
            b'p' => {
                if self.calls >= self.limit {
                    return Err(std::io::Error::other("oracle: sink failure"));
                }
            }
            _ => {}
        }
        self.buf.extend_from_slice(p);
        Ok(p.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn fnv64(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn wlog_hash(wlog: &[usize]) -> u64 {
    let mut bytes = Vec::with_capacity(wlog.len() * 8);
    for &n in wlog {
        bytes.extend_from_slice(&(n as u64).to_le_bytes());
    }
    fnv64(&bytes)
}

pub enum DataSpec {
    None,
    Empty,
    Bytes(Vec<u8>),
}

pub fn resolve(spec: &str, files: &HashMap<String, Vec<u8>>) -> DataSpec {
    if spec == "none" {
        return DataSpec::None;
    }
    if spec == "empty" {
        return DataSpec::Empty;
    }
    if let Some(rest) = spec.strip_prefix("gen:") {
        let p: Vec<&str> = rest.split(':').collect();
        return DataSpec::Bytes(gen_data(
            p[0].parse().unwrap(),
            p[1].parse().unwrap(),
            p[2].parse().unwrap(),
        ));
    }
    if spec.starts_with("go:") {
        return DataSpec::Bytes(go_test_data(spec));
    }
    if let Some(name) = spec.strip_prefix("file:") {
        return DataSpec::Bytes(
            files
                .get(name)
                .unwrap_or_else(|| panic!("missing file {name}"))
                .clone(),
        );
    }
    panic!("bad spec {spec}")
}

impl DataSpec {
    pub fn bytes(&self) -> &[u8] {
        match self {
            DataSpec::None | DataSpec::Empty => &[],
            DataSpec::Bytes(b) => b,
        }
    }
}

pub struct Case {
    pub line: String,
    pub id: String,
    pub wrapper: String,
    pub level: i32,
    pub data: String,
    pub dict: String,
    pub ops: Vec<String>,
    pub outlen: usize,
    pub outfnv: u64,
    pub wlogfnv: u64,
    pub results: String,
}

pub fn parse_cases(text: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(f.len(), 10, "bad line {line}");
        cases.push(Case {
            line: line.to_string(),
            id: f[0].to_string(),
            wrapper: f[1].to_string(),
            level: f[2].parse().unwrap(),
            data: f[3].to_string(),
            dict: f[4].to_string(),
            ops: f[5].split(',').map(|s| s.to_string()).collect(),
            outlen: f[6].parse().unwrap(),
            outfnv: u64::from_str_radix(f[7], 16).unwrap(),
            wlogfnv: u64::from_str_radix(f[8], 16).unwrap(),
            results: f[9].to_string(),
        });
    }
    cases
}

#[allow(clippy::large_enum_variant)]
enum Comp {
    Flate(flate::Writer<Recorder>),
    Zlib(zlib::Writer<Recorder>),
}

pub struct RunResult {
    pub out: Vec<u8>,
    pub wlog: Vec<usize>,
    pub results: String,
}

pub fn run_case(c: &Case, files: &HashMap<String, Vec<u8>>) -> RunResult {
    let data = resolve(&c.data, files);
    let data = data.bytes();
    let dict = resolve(&c.dict, files);
    let mut rec = Recorder::default();
    let mut wrapper = c.wrapper.as_str();
    if let Some(i) = wrapper.find('!') {
        rec.mode = wrapper.as_bytes()[i + 1];
        rec.limit = wrapper[i + 2..].parse().unwrap();
        wrapper = &wrapper[..i];
    }
    let mut w = match wrapper {
        "flate" => Comp::Flate(match &dict {
            DataSpec::None => flate::new_writer(rec, c.level).unwrap(),
            d => flate::new_writer_dict(rec, c.level, d.bytes()).unwrap(),
        }),
        "zlib" => Comp::Zlib(
            zlib::new_writer_level_dict(
                rec,
                c.level,
                match &dict {
                    DataSpec::None => None,
                    d => Some(d.bytes()),
                },
            )
            .unwrap(),
        ),
        _ => panic!("bad wrapper"),
    };
    let mut pos = 0;
    let mut results = String::new();
    for op in &c.ops {
        let r: Result<(), Error> = match op.as_bytes()[0] {
            b'W' => {
                // W<n> writes the next n bytes; W<n>x<k> does that k times,
                // stopping at the first error (like io.Copy).
                let (n, k) = match op[1..].split_once('x') {
                    Some((n, k)) => (n.parse::<usize>().unwrap(), k.parse::<usize>().unwrap()),
                    None => (op[1..].parse::<usize>().unwrap(), 1),
                };
                let mut r = Ok(());
                for _ in 0..k {
                    let chunk = &data[pos..pos + n];
                    pos += n;
                    r = match &mut w {
                        Comp::Flate(f) => f.write(chunk).map(|_| ()),
                        Comp::Zlib(z) => z.write(chunk).map(|_| ()),
                    };
                    if r.is_err() {
                        break;
                    }
                }
                r
            }
            b'F' => match &mut w {
                Comp::Flate(f) => f.flush(),
                Comp::Zlib(z) => z.flush(),
            },
            b'C' => match &mut w {
                Comp::Flate(f) => f.close(),
                Comp::Zlib(z) => z.close(),
            },
            b'R' => {
                // Reset to the same sink.
                match &mut w {
                    Comp::Flate(f) => {
                        let rec = std::mem::take(f.get_mut());
                        f.reset(rec);
                    }
                    Comp::Zlib(z) => {
                        let rec = std::mem::take(z.get_mut());
                        z.reset(rec);
                    }
                }
                Ok(())
            }
            _ => panic!("bad op {op}"),
        };
        results.push(if r.is_ok() { 'o' } else { 'e' });
    }
    let rec = match w {
        Comp::Flate(f) => f.into_inner(),
        Comp::Zlib(z) => z.into_inner(),
    };
    RunResult {
        out: rec.buf,
        wlog: rec.wlog,
        results,
    }
}

/// Runs every case and returns descriptions of the failures.
pub fn check_cases(cases: &[Case], files: &HashMap<String, Vec<u8>>) -> Vec<String> {
    let mut failures = Vec::new();
    for c in cases {
        let res = run_case(c, files);
        let got_fnv = fnv64(&res.out);
        let got_wlog = wlog_hash(&res.wlog);
        // A Go partial Write is two Rust write calls (see Recorder).
        let wlog_ok = got_wlog == c.wlogfnv || c.wrapper.contains("!b");
        if res.out.len() != c.outlen || got_fnv != c.outfnv || !wlog_ok || res.results != c.results
        {
            failures.push(format!(
                "case {}: got len={} fnv={:016x} wlog={:016x} results={} ; want {}",
                c.id,
                res.out.len(),
                got_fnv,
                got_wlog,
                res.results,
                c.line.chars().take(300).collect::<String>()
            ));
        }
    }
    failures
}

/// TestBestSpeed's testCases (tc[0] is replaced by firstN).
const BEST_SPEED_CASES: &[&[usize]] = &[
    &[65536, 0],
    &[65536, 1],
    &[65536, 1, 256],
    &[65536, 1, 65536],
    &[65536, 14],
    &[65536, 15],
    &[65536, 16],
    &[65536, 16, 256],
    &[65536, 16, 65536],
    &[65536, 127],
    &[65536, 128],
    &[65536, 128, 256],
    &[65536, 128, 65536],
    &[65536, 129],
    &[65536, 65536, 256],
    &[65536, 65536, 65536],
];

/// Inputs of Go's own flate tests (Go: `goTestData` in
/// tools/go-oracle/go-flate/gotests.go).
pub fn go_test_data(spec: &str) -> Vec<u8> {
    let p: Vec<&str> = spec.split(':').collect();
    let arg = |i: usize| -> i64 { p[i].parse().unwrap() };
    let mut out = Vec::new();
    match p[1] {
        "abcseq" => {
            let abc: Vec<u8> = (0..128u8).collect();
            let abcabc = abc.repeat(131072 / abc.len());
            let mut tc = BEST_SPEED_CASES[arg(2) as usize].to_vec();
            tc[0] = arg(3) as usize;
            for n in tc {
                out.extend_from_slice(&abcabc[..n]);
            }
        }
        "maxoff" => {
            const ABC: &[u8] = b"abcdefgh";
            const XYZ: &[u8] = b"stuvwxyz";
            let (match_before, extra, offset_adj) = (arg(2) == 1, arg(3), arg(4));
            let offset = ((1i64 << 15) + offset_adj) as usize;
            out = vec![0u8; offset + ABC.len() + extra as usize];
            out[..ABC.len()].copy_from_slice(ABC);
            if !match_before {
                out[offset - XYZ.len()..offset].copy_from_slice(XYZ);
            }
            out[offset..offset + ABC.len()].copy_from_slice(ABC);
        }
        "hello" => out = b"hello world - how are you doing?".repeat(arg(2) as usize),
        "world" => out = b"we are the world - how are you?".repeat(3),
        "asd" => {
            for i in 0..arg(2) {
                out.extend_from_slice(format!("asdasfasf{i}{i}fghfgujyut{i}yutyu\n").as_bytes());
            }
        }
        "asdf" => {
            let mut b = Vec::new();
            for i in 0..arg(2) {
                b.extend_from_slice(
                    format!("asdfasdfasdfasdf{i}{i}fghfgujyut{i}yutyu\n").as_bytes(),
                );
            }
            out = b.repeat(arg(3) as usize);
        }
        "det" => {
            let mut r = Rng::new(1);
            out = (0..arg(2)).map(|_| (r.next() & 7) as u8).collect();
        }
        "zeros" => out = vec![0u8; arg(2) as usize],
        "sparse" => {
            let l = arg(2) as usize;
            out = vec![0u8; l];
            for b in out[l.saturating_sub(1 << 16)..].iter_mut() {
                *b = 1;
            }
        }
        _ => panic!("bad go: spec {spec}"),
    }
    out
}
