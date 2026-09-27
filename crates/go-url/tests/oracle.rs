//! Differential tests against fixtures produced by tools/go-oracle/go-url.

use go_url::{Url, Userinfo};

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

fn b2s(b: bool) -> Vec<u8> {
    if b { b"1".to_vec() } else { b"0".to_vec() }
}

fn err_str<T>(r: &Result<T, go_url::Error>) -> Vec<u8> {
    match r {
        Ok(_) => b"<nil>".to_vec(),
        Err(e) => e.to_string().into_bytes(),
    }
}

fn struct_fields(u: &Url) -> Vec<Vec<u8>> {
    let (uf, un, pw): (&[u8], Vec<u8>, Vec<u8>) = match &u.user {
        None => (b"nil", vec![], vec![]),
        Some(ui) => {
            let (p, set) = ui.password();
            (
                if set { b"userpass" } else { b"user" },
                ui.username().to_vec(),
                p.to_vec(),
            )
        }
    };
    vec![
        u.scheme.clone(),
        u.opaque.clone(),
        uf.to_vec(),
        un,
        pw,
        u.host.clone(),
        u.path.clone(),
        u.raw_path.clone(),
        b2s(u.omit_host),
        b2s(u.force_query),
        u.raw_query.clone(),
        u.fragment.clone(),
        u.raw_fragment.clone(),
    ]
}

fn dump(u: &Url) -> Vec<Vec<u8>> {
    let mut out = struct_fields(u);
    let us = match &u.user {
        None => vec![],
        Some(ui) => ui.string(),
    };
    out.extend([
        u.string(),
        u.escaped_path(),
        u.escaped_fragment(),
        u.redacted(),
        u.request_uri(),
        u.hostname().to_vec(),
        u.port().to_vec(),
        b2s(u.is_abs()),
        u.query().encode(),
        us,
        u.marshal_binary(),
    ]);
    out
}

fn parse_res(r: Result<Url, go_url::Error>) -> Vec<Vec<u8>> {
    match r {
        Err(e) => vec![b"ERR".to_vec(), e.to_string().into_bytes()],
        Ok(u) => {
            let mut v = vec![b"OK".to_vec()];
            v.extend(dump(&u));
            v
        }
    }
}

fn url_from_fields(f: &[Vec<u8>]) -> Url {
    let user: Option<Userinfo> = match f[2].as_slice() {
        b"nil" => None,
        b"user" => Some(go_url::user(&f[3])),
        b"userpass" => Some(go_url::user_password(&f[3], &f[4])),
        other => panic!("bad user flag {other:?}"),
    };
    Url {
        scheme: f[0].clone(),
        opaque: f[1].clone(),
        user,
        host: f[5].clone(),
        path: f[6].clone(),
        raw_path: f[7].clone(),
        omit_host: f[8] == b"1",
        force_query: f[9] == b"1",
        raw_query: f[10].clone(),
        fragment: f[11].clone(),
        raw_fragment: f[12].clone(),
    }
}

fn eval(op: &str, args: &[Vec<u8>]) -> Vec<Vec<u8>> {
    match op {
        "Table" => go_url::encoding_table()
            .iter()
            .map(|v| v.to_string().into_bytes())
            .collect(),
        "Parse" => parse_res(go_url::parse(&args[0])),
        "ParseRequestURI" => parse_res(go_url::parse_request_uri(&args[0])),
        "QueryEscape" => vec![go_url::query_escape(&args[0])],
        "PathEscape" => vec![go_url::path_escape(&args[0])],
        "QueryUnescape" => {
            let r = go_url::query_unescape(&args[0]);
            vec![r.clone().unwrap_or_default(), err_str(&r)]
        }
        "PathUnescape" => {
            let r = go_url::path_unescape(&args[0]);
            vec![r.clone().unwrap_or_default(), err_str(&r)]
        }
        "ParseQuery" => {
            let (v, err) = go_url::parse_query(&args[0]);
            let mut res = vec![match err {
                None => b"<nil>".to_vec(),
                Some(e) => e.to_string().into_bytes(),
            }];
            res.push(v.encode());
            res.push(v.0.len().to_string().into_bytes());
            for (k, vs) in &v.0 {
                res.push(k.clone());
                res.push(vs.len().to_string().into_bytes());
                res.extend(vs.iter().cloned());
            }
            res
        }
        "Resolve" => {
            // The oracle only emits bases that Go parses.
            match go_url::parse(&args[0]) {
                Ok(base) => parse_res(base.parse(&args[1])),
                Err(e) => vec![b"BASE-ERR".to_vec(), e.to_string().into_bytes()],
            }
        }
        "JoinPath" => {
            let elems = &args[1..];
            let r = go_url::join_path(&args[0], elems);
            let mut res = vec![r.clone().unwrap_or_default(), err_str(&r)];
            if let Ok(bu) = go_url::parse(&args[0]) {
                res.extend(dump(&bu.join_path(elems)));
            }
            res
        }
        "Struct" => {
            let u = url_from_fields(&args[0..13]);
            let r = url_from_fields(&args[13..26]);
            let elems = &args[26..];
            let mut res = dump(&u);
            res.extend(dump(&u.resolve_reference(&r)));
            res.extend(dump(&u.join_path(elems)));
            res
        }
        _ => panic!("unknown op {op}"),
    }
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
            if failures <= 15 {
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
        "/tests/fixtures/url.txt"
    ));
    assert!(n > 10000, "fixture too small: {n}");
}

/// URLs extracted from the golden seeksnack build (href/src/content/loc
/// values), run through Parse/ParseRequestURI/escapes/ParseQuery and resolved
/// against a site base URL.
#[test]
fn oracle_golden_sample() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/golden-sample.txt"
    ));
    assert!(n > 2000, "fixture too small: {n}");
}

/// Adversarial sample (`go-url -adv 1000 -seed 7 -every 6`): URLs built from
/// %, #, ?, Thai, spaces, ';', '@', invalid UTF-8 and piecewise IPv6
/// literals, String() round trips, odd bases, JoinPath elements, URL structs
/// with disagreeing RawPath/RawFragment, every byte / every %XX in every
/// component, and ParseQuery around the 10000-parameter GODEBUG limit.
#[test]
fn oracle_adversarial() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/adversarial.txt"
    ));
    assert!(n > 8000, "fixture too small: {n}");
}

/// Large scratch corpus: GO_URL_CORPUS=<file> cargo test --release -- --ignored
#[test]
#[ignore]
fn oracle_corpus() {
    let path = std::env::var("GO_URL_CORPUS").expect("set GO_URL_CORPUS");
    let n = run_file(&path);
    eprintln!("{n} cases OK");
}

// A few hand-checked cases from Go's url_test.go.
#[test]
fn go_url_tests() {
    let u = go_url::parse("http://www.google.com/?q=go+language").unwrap();
    assert_eq!(u.raw_query, b"q=go+language");
    assert_eq!(u.string(), b"http://www.google.com/?q=go+language");
    let u = go_url::parse("http://www.google.com/a%20b").unwrap();
    assert_eq!(u.path, b"/a b");
    assert_eq!(u.string(), b"http://www.google.com/a%20b");
    let e = go_url::parse("http://[::1]:namedport").unwrap_err();
    assert_eq!(
        e.to_string(),
        "parse \"http://[::1]:namedport\": invalid port \":namedport\" after host"
    );
    let e = go_url::parse("%%").unwrap_err();
    assert_eq!(e.to_string(), "parse \"%%\": invalid URL escape \"%%\"");
    assert_eq!(go_url::query_escape("a b/c"), b"a+b%2Fc");
    assert_eq!(go_url::path_escape("a b/c"), b"a%20b%2Fc");
    let base = go_url::parse("http://a/b/c/d;p?q").unwrap();
    assert_eq!(base.parse("../../g").unwrap().string(), b"http://a/g");
    assert_eq!(
        go_url::join_path("https://go.googlesource.com/a/b/c", &["../../../go"]).unwrap(),
        b"https://go.googlesource.com/go"
    );
}
