//! Differential tests against fixtures produced by tools/go-oracle/go-path.

use go_path::{filepath, path};

/// Decodes the oracle's field encoding: printable ASCII verbatim, `\xHH` otherwise.
fn unesc(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            assert_eq!(b[i + 1], b'x', "bad escape in fixture");
            let h = std::str::from_utf8(&b[i + 2..i + 4]).unwrap();
            out.push(u8::from_str_radix(h, 16).unwrap());
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

fn b(v: bool) -> Vec<u8> {
    if v {
        b"true".to_vec()
    } else {
        b"false".to_vec()
    }
}

fn err_str<E: std::fmt::Display>(r: &Result<impl Sized, E>) -> Vec<u8> {
    match r {
        Ok(_) => b"<nil>".to_vec(),
        Err(e) => e.to_string().into_bytes(),
    }
}

/// Runs one op; also cross-checks the `&str` forms when inputs are UTF-8.
fn eval(op: &str, args: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let a0 = || args[0].as_slice();
    let a1 = || args[1].as_slice();
    let s0 = std::str::from_utf8(args.first().map(|v| v.as_slice()).unwrap_or(b"")).ok();
    let res: Vec<Vec<u8>> = match op {
        "path.Clean" => {
            let r = path::clean_bytes(a0());
            if let Some(s) = s0 {
                assert_eq!(path::clean(s).as_bytes(), &r[..]);
            }
            vec![r]
        }
        "path.Split" => {
            let (d, f) = path::split_bytes(a0());
            if let Some(s) = s0 {
                let (ds, fs) = path::split(s);
                assert_eq!((ds.as_bytes(), fs.as_bytes()), (d, f));
            }
            vec![d.to_vec(), f.to_vec()]
        }
        "path.Ext" => {
            let r = path::ext_bytes(a0()).to_vec();
            if let Some(s) = s0 {
                assert_eq!(path::ext(s).as_bytes(), &r[..]);
            }
            vec![r]
        }
        "path.Base" => {
            let r = path::base_bytes(a0()).to_vec();
            if let Some(s) = s0 {
                assert_eq!(path::base(s).as_bytes(), &r[..]);
            }
            vec![r]
        }
        "path.Dir" => {
            let r = path::dir_bytes(a0());
            if let Some(s) = s0 {
                assert_eq!(path::dir(s).as_bytes(), &r[..]);
            }
            vec![r]
        }
        "path.IsAbs" => vec![b(path::is_abs_bytes(a0()))],
        "path.Join" => {
            let r = path::join_bytes(args);
            if let Ok(strs) = args
                .iter()
                .map(|a| std::str::from_utf8(a))
                .collect::<Result<Vec<_>, _>>()
            {
                assert_eq!(path::join(&strs).as_bytes(), &r[..]);
            }
            vec![r]
        }
        "path.Match" => {
            let r = path::match_bytes(a0(), a1());
            vec![b(*r.as_ref().unwrap_or(&false)), err_str(&r)]
        }
        "filepath.Clean" => {
            let r = filepath::clean_bytes(a0());
            if let Some(s) = s0 {
                assert_eq!(filepath::clean(s).as_bytes(), &r[..]);
            }
            vec![r]
        }
        "filepath.Split" => {
            let (d, f) = filepath::split_bytes(a0());
            if let Some(s) = s0 {
                let (ds, fs) = filepath::split(s);
                assert_eq!((ds.as_bytes(), fs.as_bytes()), (d, f));
            }
            vec![d.to_vec(), f.to_vec()]
        }
        "filepath.Ext" => vec![filepath::ext_bytes(a0()).to_vec()],
        "filepath.Base" => {
            let r = filepath::base_bytes(a0()).to_vec();
            if let Some(s) = s0 {
                assert_eq!(filepath::base(s).as_bytes(), &r[..]);
            }
            vec![r]
        }
        "filepath.Dir" => {
            let r = filepath::dir_bytes(a0());
            if let Some(s) = s0 {
                assert_eq!(filepath::dir(s).as_bytes(), &r[..]);
            }
            vec![r]
        }
        "filepath.IsAbs" => vec![b(filepath::is_abs_bytes(a0()))],
        "filepath.IsLocal" => vec![b(filepath::is_local_bytes(a0()))],
        "filepath.ToSlash" => vec![filepath::to_slash_bytes(a0())],
        "filepath.FromSlash" => vec![filepath::from_slash_bytes(a0())],
        "filepath.VolumeName" => vec![filepath::volume_name_bytes(a0())],
        "filepath.SplitList" => {
            let r: Vec<Vec<u8>> = filepath::split_list_bytes(a0())
                .into_iter()
                .map(|x| x.to_vec())
                .collect();
            if let Some(s) = s0 {
                let rs: Vec<Vec<u8>> = filepath::split_list(s)
                    .into_iter()
                    .map(|x| x.as_bytes().to_vec())
                    .collect();
                assert_eq!(rs, r);
            }
            r
        }
        "filepath.Join" => vec![filepath::join_bytes(args)],
        "filepath.Match" => {
            let r = filepath::match_bytes(a0(), a1());
            vec![b(*r.as_ref().unwrap_or(&false)), err_str(&r)]
        }
        "filepath.Rel" => {
            let r = filepath::rel_bytes(a0(), a1());
            let v = match &r {
                Ok(v) => v.clone(),
                Err(_) => Vec::new(),
            };
            let e = match &r {
                Ok(_) => b"<nil>".to_vec(),
                Err(e) => e.msg.clone(),
            };
            vec![v, e]
        }
        _ => panic!("unknown op {op}"),
    };
    res
}

fn run_file(path: &str) -> usize {
    let data = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let data = String::from_utf8(data).expect("fixture is ASCII");
    let mut failures = 0;
    let mut n = 0;
    for (lineno, line) in data.lines().enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        let op = f[0];
        let nargs: usize = f[1].parse().unwrap();
        let args: Vec<Vec<u8>> = f[2..2 + nargs].iter().map(|s| unesc(s)).collect();
        let want: Vec<Vec<u8>> = f[2 + nargs..].iter().map(|s| unesc(s)).collect();
        let got = eval(op, &args);
        if got != want {
            failures += 1;
            if failures <= 20 {
                let show = |v: &Vec<Vec<u8>>| {
                    v.iter()
                        .map(|x| String::from_utf8_lossy(x).into_owned())
                        .collect::<Vec<_>>()
                };
                eprintln!(
                    "line {}: {op}{:?}\n  want {:?}\n  got  {:?}",
                    lineno + 1,
                    show(&args),
                    show(&want),
                    show(&got)
                );
            }
        }
        n += 1;
    }
    assert_eq!(failures, 0, "{failures} mismatches out of {n}");
    n
}

#[test]
fn oracle_fixture() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/path.txt"
    ));
    assert!(n > 40000, "fixture too small: {n}");
}

/// File paths of the golden seeksnack build output and site sources.
#[test]
fn oracle_golden_sample() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/golden-sample.txt"
    ));
    assert!(n > 5000, "fixture too small: {n}");
}

/// Adversarial sample (`go-path -adv 3000 -seed 5 -every 16`): dot and slash
/// runs, Thai and invalid UTF-8 segments, Rel between lexical variants, glob
/// patterns with multi-byte / malformed classes and escapes, every UTF-8
/// boundary sequence against '?', '*' and classes.
#[test]
fn oracle_adversarial() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/adversarial.txt"
    ));
    assert!(n > 15000, "fixture too small: {n}");
}

/// Large scratch corpus: GO_PATH_CORPUS=<file> cargo test --release -- --ignored
#[test]
#[ignore]
fn oracle_corpus() {
    let path = std::env::var("GO_PATH_CORPUS").expect("set GO_PATH_CORPUS");
    let n = run_file(&path);
    eprintln!("{n} cases OK");
}

// Go: path/path_test.go:cleantests (subset)
#[test]
fn go_clean_tests() {
    let tests = [
        ("", "."),
        ("abc", "abc"),
        ("abc/def", "abc/def"),
        ("a/b/c", "a/b/c"),
        (".", "."),
        ("..", ".."),
        ("../..", "../.."),
        ("../../abc", "../../abc"),
        ("/abc", "/abc"),
        ("/", "/"),
        ("abc/", "abc"),
        ("abc/def/", "abc/def"),
        ("/abc/", "/abc"),
        ("//abc//def//ghi", "/abc/def/ghi"),
        ("abc/./def", "abc/def"),
        ("abc/def/..", "abc"),
        ("abc/def/../../..", ".."),
        ("/abc/def/../../..", "/"),
        ("abc/def/../ghi/../jkl", "abc/jkl"),
        ("abc/./../def", "def"),
        ("/../abc", "/abc"),
    ];
    for (p, want) in tests {
        assert_eq!(path::clean(p), want, "path.Clean({p:?})");
        assert_eq!(filepath::clean(p), want, "filepath.Clean({p:?})");
    }
}

#[test]
fn go_rel_tests() {
    assert_eq!(filepath::rel("/a/b", "/a/b/c/d").unwrap(), "c/d");
    assert_eq!(filepath::rel("a/b", "a/c").unwrap(), "../c");
    assert_eq!(
        filepath::rel("/a", "b").unwrap_err().to_string(),
        "Rel: can't make b relative to /a"
    );
    assert_eq!(path::r#match("a*/b", "abc/b"), Ok(true));
    assert_eq!(
        path::r#match("[", "a").unwrap_err().to_string(),
        "syntax error in pattern"
    );
}
