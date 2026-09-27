//! Decompressor parity: streams built from generated data (then truncated,
//! corrupted or extended) are read back with our flate / zlib readers and
//! must produce Go's output, Read-call sequence, input consumption and
//! error values (tools/go-oracle/go-flate `inflate-cases`).

mod common;

use std::collections::HashMap;
use std::path::Path;

use common::{DataSpec, fnv64, resolve};
use go_flate::{Error, flate, zlib};

fn err_string(e: &Option<Error>) -> String {
    match e {
        None => "<nil>".to_string(),
        Some(e) => e.to_string(),
    }
}

struct Stream {
    wrapper: String,
    level: i32,
    data: String,
    flush_at: i64,
    dict_w: String,
    mutations: Vec<String>,
}

fn build_stream(c: &Stream) -> Vec<u8> {
    let files = HashMap::new();
    let data = resolve(&c.data, &files);
    let data = data.bytes();
    let dict_w = resolve(&c.dict_w, &files);
    let mut out: Vec<u8> = Vec::new();
    {
        let parts: Vec<&[u8]> = if c.flush_at >= 0 {
            let (a, b) = data.split_at(c.flush_at as usize);
            vec![a, b]
        } else {
            vec![data]
        };
        if c.wrapper == "flate" {
            let mut w = match &dict_w {
                DataSpec::None => flate::new_writer(&mut out, c.level).unwrap(),
                d => flate::new_writer_dict(&mut out, c.level, d.bytes()).unwrap(),
            };
            w.write(parts[0]).unwrap();
            if parts.len() > 1 {
                w.flush().unwrap();
                w.write(parts[1]).unwrap();
            }
            w.close().unwrap();
        } else {
            let d = match &dict_w {
                DataSpec::None => None,
                d => Some(d.bytes()),
            };
            let mut w = zlib::new_writer_level_dict(&mut out, c.level, d).unwrap();
            w.write(parts[0]).unwrap();
            if parts.len() > 1 {
                w.flush().unwrap();
                w.write(parts[1]).unwrap();
            }
            w.close().unwrap();
        }
    }
    for m in &c.mutations {
        match m.as_bytes()[0] {
            b't' => out.truncate(m[1..].parse().unwrap()),
            b'x' => {
                let (p, v) = m[1..].split_once(':').unwrap();
                let pos: usize = p.parse().unwrap();
                out[pos] ^= v.parse::<u8>().unwrap();
            }
            b'a' => {
                let n: usize = m[1..].parse().unwrap();
                out.extend_from_slice(&common::gen_data(1, n, n as u64));
            }
            _ => panic!("bad mutation"),
        }
    }
    out
}

enum Rd<'a, 'b> {
    Flate(flate::Decompressor<&'a mut &'b [u8]>),
    Zlib(zlib::Reader<&'a mut &'b [u8]>),
}

/// Returns (output, call log fnv, consumed, error string).
fn run_inflate(c: &Stream, dict_r: &str, read_size: usize) -> (Vec<u8>, u64, usize, String) {
    let stream = build_stream(c);
    let files = HashMap::new();
    let dict = resolve(dict_r, &files);
    // Go reads from a *bytes.Reader (an io.ByteReader, used directly).
    let mut cur: &[u8] = &stream[..];
    let mut log = String::new();
    let mut out = Vec::new();
    let result;
    {
        let mut rd = if c.wrapper == "flate" {
            Rd::Flate(match &dict {
                DataSpec::None => flate::new_reader(&mut cur),
                d => flate::new_reader_dict(&mut cur, d.bytes()),
            })
        } else {
            let d = match &dict {
                DataSpec::None => None,
                d => Some(d.bytes()),
            };
            match zlib::new_reader_dict(&mut cur, d) {
                Ok(z) => Rd::Zlib(z),
                Err(e) => {
                    let remaining = cur.len();
                    return (Vec::new(), 0, stream.len() - remaining, format!("new: {e}"));
                }
            }
        };
        let mut buf = vec![0u8; read_size];
        let mut last: Option<Error> = None;
        for _ in 0..(1 << 20) {
            let (n, err) = match &mut rd {
                Rd::Flate(f) => f.read(&mut buf),
                Rd::Zlib(z) => z.read(&mut buf),
            };
            out.extend_from_slice(&buf[..n]);
            log.push_str(&format!("{}:{};", n, err_string(&err)));
            if err.is_some() {
                last = err;
                break;
            }
        }
        let close = match &mut rd {
            Rd::Flate(f) => f.close().err(),
            Rd::Zlib(z) => z.close().err(),
        };
        result = format!("{} | close: {}", err_string(&last), err_string(&close));
    }
    let remaining = cur.len();
    (out, fnv64(log.as_bytes()), stream.len() - remaining, result)
}

#[test]
fn inflate_oracle_cases() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/inflate_cases.txt");
    let text = std::fs::read_to_string(p).unwrap();
    let mut failures = Vec::new();
    let mut total = 0;
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        total += 1;
        let f: Vec<&str> = line.splitn(15, ' ').collect();
        let c = Stream {
            wrapper: f[1].to_string(),
            level: f[2].parse().unwrap(),
            data: f[3].to_string(),
            flush_at: f[4].parse().unwrap(),
            dict_w: f[5].to_string(),
            mutations: if f[7] == "-" {
                Vec::new()
            } else {
                f[7].split(',').map(|s| s.to_string()).collect()
            },
        };
        let dict_r = f[6];
        let read_size: usize = f[8].parse().unwrap();
        let want_len: usize = f[9].parse().unwrap();
        let want_fnv = u64::from_str_radix(f[10], 16).unwrap();
        let want_log = u64::from_str_radix(f[11], 16).unwrap();
        let want_consumed: usize = f[12].parse().unwrap();
        let want_err = f[13..].join(" ");
        let (out, log, consumed, err) = run_inflate(&c, dict_r, read_size);
        let log_ok = want_err.starts_with("new: ") || log == want_log;
        if out.len() != want_len
            || fnv64(&out) != want_fnv
            || !log_ok
            || consumed != want_consumed
            || err != want_err
        {
            failures.push(format!(
                "{line}\n   got len={} fnv={:016x} log={:016x} consumed={} err={err}",
                out.len(),
                fnv64(&out),
                log,
                consumed
            ));
        }
    }
    for f in failures.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(
        failures.is_empty(),
        "{} of {total} inflate cases differ",
        failures.len()
    );
    assert!(total >= 3000);
}

// Go: compress/flate/flate_test.go:TestStreams
#[test]
fn go_test_streams() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "degenerate HCLenTree",
            "05e001000000000010000000000000000000000000000000000000000000000000000000000000000004",
            "fail",
        ),
        (
            "complete HCLenTree, empty HLitTree, empty HDistTree",
            "05e001040000000000000000000000000000000000000000000000000000000000000000000000000010",
            "fail",
        ),
        (
            "empty HCLenTree",
            "05e001000000000000000000000000000000000000000000000000000000000000000000000000000010",
            "fail",
        ),
        (
            "complete HCLenTree, complete HLitTree, empty HDistTree, use missing HDist symbol",
            "000100feff000de00104000000001000000000000000000000000000000000000000000000000000000000000000002c",
            "fail",
        ),
        (
            "complete HCLenTree, complete HLitTree, degenerate HDistTree, use missing HDist symbol",
            "000100feff000de001000000000000000000000000000000000000000000000000000000000000000610000000004070",
            "fail",
        ),
        (
            "complete HCLenTree, empty HLitTree, empty HDistTree",
            "05e00104000000001004000000000000000000000000000000000000000000000000000000000000000000000008",
            "fail",
        ),
        (
            "complete HCLenTree, empty HLitTree, degenerate HDistTree",
            "05e00104000000001004000000000000000000000000000000000000000000000000000000000000000800000008",
            "fail",
        ),
        (
            "complete HCLenTree, degenerate HLitTree, degenerate HDistTree, use missing HLit symbol",
            "05e00104000000001000000000000000000000000000000000000000000000000000000000000000001c",
            "fail",
        ),
        (
            "complete HCLenTree, complete HLitTree, too large HDistTree",
            "edff870500000000200400000000000000000000000000000000000000000000000000000000000000080000000000000004",
            "fail",
        ),
        (
            "complete HCLenTree, complete HLitTree, empty HDistTree, excessive repeater code",
            "edfd870500000000200400000000000000000000000000000000000000000000000000000000000000e8b100",
            "fail",
        ),
        (
            "complete HCLenTree, complete HLitTree, empty HDistTree of normal length 30",
            "05fd01240000000000f8ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff07000000fe01",
            "",
        ),
        (
            "complete HCLenTree, complete HLitTree, empty HDistTree of excessive length 31",
            "05fe01240000000000f8ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff07000000fc03",
            "fail",
        ),
        (
            "complete HCLenTree, over-subscribed HLitTree, empty HDistTree",
            "05e001240000000000fcffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff07f00f",
            "fail",
        ),
        (
            "complete HCLenTree, under-subscribed HLitTree, empty HDistTree",
            "05e001240000000000fcfffffffffffffffffffffffffffffffffffffffffffffffffffffcffffffff07f00f",
            "fail",
        ),
        (
            "complete HCLenTree, complete HLitTree with single code, empty HDistTree",
            "05e001240000000000f8ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff07f00f",
            "01",
        ),
        (
            "complete HCLenTree, complete HLitTree with multiple codes, empty HDistTree",
            "05e301240000000000f8ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff07807f",
            "01",
        ),
        (
            "complete HCLenTree, complete HLitTree, degenerate HDistTree, use valid HDist symbol",
            "000100feff000de00104000000001000000000000000000000000000000000000000000000000000000000000000003c",
            "00000000",
        ),
        (
            "complete HCLenTree, degenerate HLitTree, degenerate HDistTree",
            "05e00104000000001000000000000000000000000000000000000000000000000000000000000000000c",
            "",
        ),
        (
            "complete HCLenTree, degenerate HLitTree, empty HDistTree",
            "05e001040000000010000000000000000000000000000000000000000000000000000000000000000004",
            "",
        ),
        (
            "complete HCLenTree, complete HLitTree, empty HDistTree, spanning repeater code",
            "edfd870500000000200400000000000000000000000000000000000000000000000000000000000000e8b000",
            "",
        ),
        (
            "complete HCLenTree with length codes, complete HLitTree, empty HDistTree",
            "ede00104000000001000000000000000000000000000000000000000000000000000000000000000000400004000",
            "",
        ),
        (
            "complete HCLenTree, complete HLitTree, degenerate HDistTree, use valid HLit symbol 284 with count 31",
            "000100feff00ede0010400000000100000000000000000000000000000000000000000000000000000000000000000040000407f00",
            "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
        (
            "complete HCLenTree, complete HLitTree, degenerate HDistTree, use valid HLit and HDist symbols",
            "0cc2010d00000082b0ac4aff0eb07d27060000ffff",
            "616263616263",
        ),
        ("fixed block, use reserved symbol 287", "33180700", "fail"),
        ("raw block", "010100feff11", "11"),
        (
            "issue 10426 - over-subscribed HCLenTree causes a hang",
            "344c4a4e494d4b070000ff2e2eff2e2e2e2e2eff",
            "fail",
        ),
        (
            "issue 11030 - empty HDistTree unexpectedly leads to error",
            "05c0070600000080400fff37a0ca",
            "",
        ),
        (
            "issue 11033 - empty HDistTree unexpectedly leads to error",
            "050fb109c020cca5d017dcbca044881ee1034ec149c8980bbc413c2ab35be9dcb1473449922449922411202306ee97b0383a521b4ffdcf3217f9f7d3adb701",
            "3130303634342068652e706870005d05355f7ed957ff084a90925d19e3ebc6d0c6d7",
        ),
    ];
    for (i, (desc, stream, want)) in cases.iter().enumerate() {
        let data = hex(stream);
        let mut f = flate::new_reader(&data[..]);
        let (out, err) = read_all(|b| f.read(b));
        if *want == "fail" {
            assert!(err.is_some(), "#{i} ({desc}): got nil error, want non-nil");
        } else {
            assert!(err.is_none(), "#{i} ({desc}): {err:?}");
            assert_eq!(to_hex(&out), *want, "#{i} ({desc})");
        }
    }
}

// Go: compress/flate/flate_test.go:TestTruncatedStreams
#[test]
fn go_test_truncated_streams() {
    let data: &[u8] = b"\x00\x0c\x00\xf3\xffhello, world\x01\x00\x00\xff\xff";
    for i in 0..data.len() - 1 {
        let mut f = flate::new_reader(&data[..i]);
        let (_, err) = read_all(|b| f.read(b));
        assert!(matches!(err, Some(Error::UnexpectedEof)), "{i}: {err:?}");
    }
}

// Go: compress/flate/inflate_test.go:TestReaderTruncated
#[test]
fn go_test_reader_truncated() {
    let vectors: &[(&[u8], &[u8])] = &[
        (b"\x00", b""),
        (b"\x00\x0c", b""),
        (b"\x00\x0c\x00", b""),
        (b"\x00\x0c\x00\xf3\xff", b""),
        (b"\x00\x0c\x00\xf3\xffhello", b"hello"),
        (b"\x00\x0c\x00\xf3\xffhello, world", b"hello, world"),
        (b"\x02", b""),
        (b"\xf2H\xcd", b"He"),
        (
            "\u{f2}H\u{0359}0a\u{0084}\t".as_bytes(),
            b"Hel\x90\x90\x90\x90\x90",
        ),
        (
            "\u{f2}H\u{0359}0a\u{0084}\t\x00".as_bytes(),
            b"Hel\x90\x90\x90\x90\x90",
        ),
    ];
    for (i, (input, output)) in vectors.iter().enumerate() {
        // Go string literals: "\xf2H͙0a\u0084\t" — \xf2 is a raw byte, the
        // rest is UTF-8. Rebuild the exact bytes.
        let input: Vec<u8> = if i >= 8 {
            let mut v = vec![0xf2u8, b'H'];
            v.extend_from_slice("\u{0359}0a\u{0084}\t".as_bytes());
            if i == 9 {
                v.push(0);
            }
            v
        } else {
            input.to_vec()
        };
        let mut f = flate::new_reader(&input[..]);
        let (out, err) = read_all(|b| f.read(b));
        assert!(
            matches!(err, Some(Error::UnexpectedEof)),
            "test {i}: {err:?}"
        );
        assert_eq!(&out[..], *output, "test {i}");
    }
}

// Go: compress/flate/inflate_test.go:TestReset / TestResetDict
#[test]
fn go_test_reset_and_reset_dict() {
    let ss = [
        "lorem ipsum izzle fo rizzle",
        "the quick brown fox jumped over",
    ];
    let mut deflated: Vec<Vec<u8>> = Vec::new();
    for s in ss {
        let mut w = flate::new_writer(Vec::new(), 1).unwrap();
        w.write(s.as_bytes()).unwrap();
        w.close().unwrap();
        deflated.push(w.into_inner());
    }
    let mut f = flate::new_reader(&deflated[0][..]);
    let (a, _) = read_all(|b| f.read(b));
    f.reset(&deflated[1][..], &[]);
    let (b, _) = read_all(|b| f.read(b));
    assert!(f.close().is_ok());
    assert_eq!(a, ss[0].as_bytes());
    assert_eq!(b, ss[1].as_bytes());

    let dict = b"the lorem fox";
    let mut deflated: Vec<Vec<u8>> = Vec::new();
    for s in ss {
        let mut w = flate::new_writer_dict(Vec::new(), flate::DEFAULT_COMPRESSION, dict).unwrap();
        w.write(s.as_bytes()).unwrap();
        w.close().unwrap();
        deflated.push(w.into_inner());
    }
    let empty: &[u8] = &[];
    let mut f = flate::new_reader(empty);
    for (i, s) in ss.iter().enumerate() {
        f.reset(&deflated[i][..], dict);
        let (out, _) = read_all(|b| f.read(b));
        assert_eq!(out, s.as_bytes());
    }
}

// Go: compress/zlib/reader_test.go:zlibTests
#[test]
fn go_test_zlib_decompressor() {
    type T = (
        &'static str,
        &'static [u8],
        &'static [u8],
        Option<&'static [u8]>,
        Option<&'static str>,
    );
    let tests: &[T] = &[
        ("truncated empty", b"", &[], None, Some("unexpected EOF")),
        (
            "truncated dict",
            b"",
            &[0x78, 0xbb],
            Some(&[0x00]),
            Some("unexpected EOF"),
        ),
        (
            "truncated checksum",
            b"",
            &[
                0x78, 0xbb, 0x00, 0x01, 0x00, 0x01, 0xca, 0x48, 0xcd, 0xc9, 0xc9, 0xd7, 0x51, 0x28,
                0xcf, 0x2f, 0xca, 0x49, 0x01, 0x04, 0x00, 0x00, 0xff, 0xff,
            ],
            Some(&[0x00]),
            Some("unexpected EOF"),
        ),
        (
            "empty",
            b"",
            &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
            None,
            None,
        ),
        (
            "goodbye",
            b"goodbye, world",
            &[
                0x78, 0x9c, 0x4b, 0xcf, 0xcf, 0x4f, 0x49, 0xaa, 0x4c, 0xd5, 0x51, 0x28, 0xcf, 0x2f,
                0xca, 0x49, 0x01, 0x00, 0x28, 0xa5, 0x05, 0x5e,
            ],
            None,
            None,
        ),
        (
            "bad header (CINFO)",
            b"",
            &[0x88, 0x98, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
            None,
            Some("zlib: invalid header"),
        ),
        (
            "bad header (FCHECK)",
            b"",
            &[0x78, 0x9f, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
            None,
            Some("zlib: invalid header"),
        ),
        (
            "bad checksum",
            b"",
            &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0xff],
            None,
            Some("zlib: invalid checksum"),
        ),
        (
            "not enough data",
            b"",
            &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00],
            None,
            Some("unexpected EOF"),
        ),
        (
            "excess data is silently ignored",
            b"",
            &[
                0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01, 0x78, 0x9c, 0xff,
            ],
            None,
            None,
        ),
        (
            "dictionary",
            b"Hello, World!\n",
            &[
                0x78, 0xbb, 0x1c, 0x32, 0x04, 0x27, 0xf3, 0x00, 0xb1, 0x75, 0x20, 0x1c, 0x45, 0x2e,
                0x00, 0x24, 0x12, 0x04, 0x74,
            ],
            Some(&[
                0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64, 0x0a,
            ]),
            None,
        ),
        (
            "wrong dictionary",
            b"",
            &[
                0x78, 0xbb, 0x1c, 0x32, 0x04, 0x27, 0xf3, 0x00, 0xb1, 0x75, 0x20, 0x1c, 0x45, 0x2e,
                0x00, 0x24, 0x12, 0x04, 0x74,
            ],
            Some(&[0x48, 0x65, 0x6c, 0x6c]),
            Some("zlib: invalid dictionary"),
        ),
        (
            "truncated zlib stream amid raw-block",
            b"hello",
            &[
                0x78, 0x9c, 0x00, 0x0c, 0x00, 0xf3, 0xff, 0x68, 0x65, 0x6c, 0x6c, 0x6f,
            ],
            None,
            Some("unexpected EOF"),
        ),
        (
            "truncated zlib stream amid fixed-block",
            b"He",
            &[0x78, 0x9c, 0xf2, 0x48, 0xcd],
            None,
            Some("unexpected EOF"),
        ),
    ];
    for (desc, raw, compressed, dict, want_err) in tests {
        // Go: zlib.NewReaderDict(bytes.NewReader(tt.compressed), tt.dict)
        let z = zlib::new_reader_dict(*compressed, *dict);
        let mut z = match z {
            Ok(z) => z,
            Err(e) => {
                assert_eq!(Some(e.to_string().as_str()), *want_err, "{desc}: NewReader");
                continue;
            }
        };
        let (out, err) = read_all(|b| z.read(b));
        if let Some(err) = err {
            assert_eq!(Some(err.to_string().as_str()), *want_err, "{desc}: io.Copy");
            continue;
        }
        assert_eq!(&out[..], *raw, "{desc}");
        // Check for sticky errors.
        let (n, err) = z.read(&mut [0u8]);
        assert!(n == 0 && matches!(err, Some(Error::Eof)), "{desc}: sticky");
        assert!(z.close().is_ok(), "{desc}: close");
    }
}

/// Go's io.ReadAll over a Go-style reader: EOF is success.
fn read_all(mut read: impl FnMut(&mut [u8]) -> (usize, Option<Error>)) -> (Vec<u8>, Option<Error>) {
    let mut out = Vec::new();
    let mut buf = vec![0u8; 512];
    loop {
        let (n, err) = read(&mut buf);
        out.extend_from_slice(&buf[..n]);
        match err {
            None => {}
            Some(Error::Eof) => return (out, None),
            Some(e) => return (out, Some(e)),
        }
    }
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
