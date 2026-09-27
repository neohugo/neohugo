//! More than 2 GiB through one writer (and 70 000 Resets) to exercise the
//! `fastGen.cur >= bufferReset` table-shift / table-clear paths of the level
//! 1-6 encoders. Expected values: `tools/go-oracle/go-flate longstreams`.
//! Slow; run with
//! `cargo test --release --test long_streams -- --ignored`.

mod common;

use std::io::Write;
use std::path::Path;

use common::{Rng, gen_data};
use go_flate::flate;

/// Hashes everything written (FNV-1a 64) and the Write sizes.
struct HashWriter {
    n: i64,
    h: u64,
    wlog: u64,
    nw: i64,
}

impl HashWriter {
    fn new() -> HashWriter {
        HashWriter {
            n: 0,
            h: 0xcbf29ce484222325,
            wlog: 0xcbf29ce484222325,
            nw: 0,
        }
    }
}

impl Write for HashWriter {
    fn write(&mut self, p: &[u8]) -> std::io::Result<usize> {
        for &c in p {
            self.h ^= c as u64;
            self.h = self.h.wrapping_mul(0x100000001b3);
        }
        for c in (p.len() as u64).to_le_bytes() {
            self.wlog ^= c as u64;
            self.wlog = self.wlog.wrapping_mul(0x100000001b3);
        }
        self.n += p.len() as i64;
        self.nw += 1;
        Ok(p.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn long_stream(level: i32, variant: &str, total: i64) -> String {
    let mut w = flate::new_writer(HashWriter::new(), level).unwrap();
    let pool: Vec<Vec<u8>> = (0..16)
        .map(|i| gen_data([3, 6, 8, 9][i % 4], (1 << 20) + i * 977, i as u64))
        .collect();
    // Go: uint64(level)*1000 + 17 (wraps for negative levels).
    let mut r = Rng::new((level as i64 as u64).wrapping_mul(1000).wrapping_add(17));
    let mut written: i64 = 0;
    match variant {
        "stream" => {
            while written < total {
                let b = &pool[r.intn(pool.len())];
                let n = 1 + r.intn(b.len());
                w.write(&b[..n]).unwrap();
                written += n as i64;
                if r.intn(500) == 0 {
                    w.flush().unwrap();
                }
            }
        }
        "random" => {
            // Fresh incompressible bytes (Go: longStream "random").
            let mut src = Rng::new(99);
            let mut buf = vec![0u8; 1 << 20];
            while written < total {
                let n = 1 + r.intn(buf.len());
                let mut i = 0;
                while i < n {
                    let v = src.next();
                    let mut j = 0;
                    while j < 8 && i + j < n {
                        buf[i + j] = (v >> (8 * j)) as u8;
                        j += 1;
                    }
                    i += 8;
                }
                w.write(&buf[..n]).unwrap();
                written += n as i64;
                if r.intn(500) == 0 {
                    w.flush().unwrap();
                }
            }
        }
        "resets" => {
            for _ in 0..total {
                let b = &pool[r.intn(pool.len())];
                let mut n = r.intn(200);
                if r.intn(50) == 0 {
                    n = r.intn(70000);
                }
                w.write(&b[..n]).unwrap();
                if r.intn(2) == 0 {
                    w.close().unwrap();
                }
                let hw = std::mem::replace(w.get_mut(), HashWriter::new());
                w.reset(hw);
            }
        }
        _ => panic!("bad variant"),
    }
    w.close().unwrap();
    let hw = w.into_inner();
    format!(
        "{} {} {} {} {:016x} {:016x} {}",
        level, variant, total, hw.n, hw.h, hw.wlog, hw.nw
    )
}

#[test]
#[ignore]
fn long_streams_wraparound() {
    check_file("longstreams.txt");
}

/// Levels 7-9: 600 MiB through one writer pushes `advancedState.hashOffset`
/// past `maxHashOffset` twice (the hashHead/hashPrev rebase in fillDeflate).
/// `oracle longstreams -levels 7,8,9 -noresets -total 629145600`.
#[test]
#[ignore]
fn long_streams_hash_offset_rebase() {
    check_file("longstreams_lazy.txt");
}

/// 64 MiB of fresh random bytes at levels 7, 8, 9 (long literal runs wrap
/// the uint16 `literalCounter` and drive `skipLiterals`), 1, 6, -2, 0.
/// `oracle longstreams -levels 7,8,9,1,6,-2,0 -noresets -variant random -total 67108864`.
#[test]
fn long_streams_random() {
    check_file("longstreams_random.txt");
}

fn check_file(name: &str) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let text = std::fs::read_to_string(p).unwrap();
    let mut failures = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(' ').collect();
        let got = long_stream(f[0].parse().unwrap(), f[1], f[2].parse().unwrap());
        eprintln!("got  {got}\nwant {line}");
        if got != line {
            failures.push(line.to_string());
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
