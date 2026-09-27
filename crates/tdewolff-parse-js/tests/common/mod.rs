//! Shared helpers for the differential tests: fixture record loading, the
//! FNV-1a digest and the per-mode dumps (mirroring
//! tools/go-oracle/tdewolff-parse-js/{main,fixtures}.go).
#![allow(dead_code)]

use std::io::Read;
use std::path::PathBuf;

use tdewolff_parse_js::Options;
use tdewolff_parse_js::dump::{js_dump, json_dump, lex_dump, parse_dump, string_dump};

/// The compared serializations, in record order (Go: `modes`).
pub const MODES: [&str; 8] = [
    "lex",
    "lexre",
    "parse",
    "parsew2f",
    "parseinline",
    "string",
    "js",
    "json",
];

// Go: main.go:runMode
pub fn run_mode(mode: &str, src: &[u8]) -> Vec<u8> {
    match mode {
        "lex" => lex_dump(src, false),
        "lexre" => lex_dump(src, true),
        "parse" => parse_dump(src, Options::default()),
        "parsew2f" => parse_dump(
            src,
            Options {
                while_to_for: true,
                inline: false,
            },
        ),
        "parseinline" => parse_dump(
            src,
            Options {
                while_to_for: true,
                inline: true,
            },
        ),
        "string" => {
            if 1 << 20 < src.len() {
                return b"SKIP\n".to_vec();
            }
            string_dump(src, Options::default())
        }
        "js" => {
            if 1 << 20 < src.len() {
                return b"SKIP\n".to_vec();
            }
            js_dump(src, Options::default())
        }
        "json" => json_dump(src, Options::default()),
        _ => panic!("unknown mode {}", mode),
    }
}

pub fn fixtures_dir() -> PathBuf {
    if let Ok(d) = std::env::var("TDEWOLFF_PARSE_JS_FIXTURES") {
        return PathBuf::from(d);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Reads a gzip-compressed record file: `#rec N\n` then N fields of
/// `<len>\n<bytes>\n`.
pub fn read_records(name: &str) -> Vec<Vec<Vec<u8>>> {
    let p = fixtures_dir().join(name);
    let f = std::fs::File::open(&p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(f)
        .read_to_end(&mut data)
        .unwrap();
    let mut recs = Vec::new();
    let mut pos = 0;
    let line = |pos: &mut usize| -> Vec<u8> {
        let start = *pos;
        while data[*pos] != b'\n' {
            *pos += 1;
        }
        let l = data[start..*pos].to_vec();
        *pos += 1;
        l
    };
    while pos < data.len() {
        let hdr = line(&mut pos);
        let hdr = String::from_utf8(hdr).unwrap();
        let n: usize = hdr.strip_prefix("#rec ").unwrap().parse().unwrap();
        let mut fields = Vec::with_capacity(n);
        for _ in 0..n {
            let len: usize = String::from_utf8(line(&mut pos)).unwrap().parse().unwrap();
            fields.push(data[pos..pos + len].to_vec());
            pos += len;
            assert_eq!(data[pos], b'\n');
            pos += 1;
        }
        recs.push(fields);
    }
    recs
}

/// Go `hash/fnv` New64a, as 16 lowercase hex digits.
pub fn digest(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}

/// First differing line, for diagnostics.
pub fn first_diff(a: &[u8], b: &[u8]) -> String {
    let al: Vec<&[u8]> = a.split(|&c| c == b'\n').collect();
    let bl: Vec<&[u8]> = b.split(|&c| c == b'\n').collect();
    for i in 0..al.len().max(bl.len()) {
        let x = al.get(i).copied().unwrap_or(b"<none>");
        let y = bl.get(i).copied().unwrap_or(b"<none>");
        if x != y {
            let cut = |s: &[u8]| {
                let s = String::from_utf8_lossy(s).into_owned();
                if s.len() > 600 {
                    format!(
                        "{}...",
                        &s[..s.char_indices().nth(600).map(|x| x.0).unwrap_or(s.len())]
                    )
                } else {
                    s
                }
            };
            return format!("line {}:\n  rust: {}\n  go:   {}", i + 1, cut(x), cut(y));
        }
    }
    "identical".to_string()
}

/// Runs `f` on a thread with a large stack (the parser is recursive).
pub fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

/// Parses an alphabet argument of the oracle's `enumerate`: hex bytes, or
/// comma-separated hex tokens.
pub fn parse_alphabet(s: &str) -> Vec<Vec<u8>> {
    let unhex = |s: &str| -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    };
    if s.contains(',') {
        s.split(',').map(unhex).collect()
    } else {
        unhex(s).into_iter().map(|c| vec![c]).collect()
    }
}

/// Checks an `enumerate` file of the Go oracle (header
/// `# enumerate ALPHAHEX MAXLEN`, then one combined digest per input):
/// enumerates every sequence of 1..MAXLEN symbols in the same order
/// (length-major, lexicographic) and returns the number of inputs and the
/// differing ones. Call on a big stack.
pub fn check_enumerate(path: &std::path::Path) -> (usize, Vec<Vec<u8>>) {
    use std::io::BufRead;
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
    let mut lines = std::io::BufReader::new(flate2::read::GzDecoder::new(f)).lines();
    let head = lines.next().unwrap().unwrap();
    let parts: Vec<&str> = head.split(' ').collect();
    assert_eq!(parts[..2], ["#", "enumerate"], "not an enumerate file");
    let alpha = parse_alphabet(parts[2]);
    let max_len: usize = parts[3].parse().unwrap();
    let (mut n, mut bad) = (0usize, Vec::new());
    for l in 1..=max_len {
        let mut idx = vec![0usize; l];
        loop {
            let src: Vec<u8> = idx.iter().flat_map(|&k| alpha[k].clone()).collect();
            let mut all = Vec::new();
            for mode in MODES {
                all.extend_from_slice(digest(&run_mode(mode, &src)).as_bytes());
            }
            let want = lines.next().expect("short enumerate file").unwrap();
            n += 1;
            if digest(&all) != want {
                bad.push(src);
            }
            // next index vector
            let mut i = l as isize - 1;
            while 0 <= i {
                idx[i as usize] += 1;
                if idx[i as usize] < alpha.len() {
                    break;
                }
                idx[i as usize] = 0;
                i -= 1;
            }
            if i < 0 {
                break;
            }
        }
    }
    assert!(lines.next().is_none(), "long enumerate file");
    (n, bad)
}
