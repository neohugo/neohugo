//! Differential tests against fixtures produced by the Go oracle
//! (tools/go-oracle/go-yaml): every case is decoded with yaml.v2
//! (`interface{}` and `map[string]interface{}` targets) and with neohugo's
//! metadecoders (`UnmarshalToMap`, `Unmarshal`), and the canonical dumps or
//! error messages must be identical.
//!
//! Set `GO_YAML_FIXTURE_DIR` to a directory of uncompressed `*.fixture`
//! files (the full corpora in the scratch area) to run them as well.

use std::io::Read;

fn unhex(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let v = |c: u8| match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("bad hex"),
    };
    b.chunks(2).map(|p| v(p[0]) << 4 | v(p[1])).collect()
}

fn esc(s: &[u8]) -> String {
    go_yaml::dump_escape(s)
}

pub fn run_case(data: &[u8]) -> [String; 4] {
    let r1 = match go_yaml::unmarshal(data) {
        Ok(v) => format!("ok {}", go_yaml::dump(&v)),
        Err(e) => format!("err {}", esc(e.message_bytes())),
    };
    let r2 = match go_yaml::unmarshal_str_map(data) {
        Ok(m) => format!("ok {}", go_yaml::dump_str_map(&m)),
        Err(e) => format!("err {}", esc(e.message_bytes())),
    };
    let r3 = match go_yaml::metadecoders::unmarshal_to_map(data) {
        Ok(m) => format!("ok {}", go_yaml::metadecoders::dump_map(&m)),
        Err(e) => format!("err {}", esc(&e.message_bytes())),
    };
    let r4 = match go_yaml::metadecoders::unmarshal(data) {
        Ok(v) => format!("ok {}", go_yaml::metadecoders::dump_value(&v)),
        Err(e) => format!("err {}", esc(&e.message_bytes())),
    };
    [r1, r2, r3, r4]
}

/// Mirror of the oracle's `collides`: does stringifyMapKeys map two keys of
/// one map to the same string (Go's result is then nondeterministic)?
fn collides(v: &go_yaml::Yaml) -> bool {
    match v {
        go_yaml::Yaml::Seq(items) => items.iter().any(collides),
        go_yaml::Yaml::Map(m) => {
            let mut seen = std::collections::HashSet::new();
            for (k, e) in m.iter() {
                if !seen.insert(go_yaml::metadecoders::cast_to_string(k)) || collides(e) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Known gap: in "invalid map key: map[...]{...}" messages Go orders keys
/// of different dynamic types by type-descriptor address (fmtsort), which
/// depends on the binary's layout. Accept a permutation of the same text.
fn known_map_key_order_gap(rust: &str, go: &str) -> bool {
    let p = "invalid map key: ";
    if !rust.contains(p) || !go.contains(p) || rust.len() != go.len() {
        return false;
    }
    let mut a: Vec<u8> = rust.bytes().collect();
    let mut b: Vec<u8> = go.bytes().collect();
    a.sort_unstable();
    b.sort_unstable();
    a == b
}

/// Handle Go "nondet" results: Rust must succeed and see a collision.
fn nondet_ok(data: &[u8], which: usize) -> bool {
    match which {
        2 => match go_yaml::unmarshal_str_map(data) {
            Ok(Some(m)) => m.iter().any(|(_, v)| collides(v)),
            _ => false,
        },
        3 => match go_yaml::unmarshal(data) {
            Ok(v) => collides(&v),
            _ => false,
        },
        _ => false,
    }
}

fn check_fixture_text(label: &str, text: &str) -> (usize, usize) {
    let mut n = 0;
    let mut bad = 0;
    for line in text.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != 6 {
            continue;
        }
        n += 1;
        let data = unhex(cols[1]);
        let got = run_case(&data);
        for i in 0..4 {
            if cols[2 + i] == "nondet" && nondet_ok(&data, i) {
                continue;
            }
            if got[i] != cols[2 + i] && !known_map_key_order_gap(&got[i], cols[2 + i]) {
                bad += 1;
                if bad <= 20 {
                    let show = |s: &str| {
                        if s.len() > 400 {
                            format!("{}...", &s[..400])
                        } else {
                            s.to_string()
                        }
                    };
                    eprintln!(
                        "[{label}] MISMATCH {} r{} input={:?}\n  go:   {}\n  rust: {}",
                        cols[0],
                        i + 1,
                        String::from_utf8_lossy(&data[..data.len().min(300)]),
                        show(cols[2 + i]),
                        show(&got[i])
                    );
                }
                break;
            }
        }
    }
    eprintln!("[{label}] {n} cases, {bad} mismatching");
    (n, bad)
}

fn gz_fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    s
}

/// Run on a big-stack thread: the dumps and `go_value::Value` drops are
/// recursive, and some cases nest 10000 levels deep.
fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

fn run_gz(name: &'static str) {
    let text = gz_fixture(name);
    let (n, bad) = big_stack(move || check_fixture_text(name, &text));
    assert!(n > 0, "{name}: no cases");
    assert_eq!(bad, 0, "{name}: {bad} of {n} cases differ from Go");
}

#[test]
fn yamlv2_test_tables() {
    run_gz("yamlv2-tests.fixture.gz");
}

#[test]
fn seeksnack_front_matter() {
    run_gz("seeksnack-fm.fixture.gz");
}

#[test]
fn adversarial() {
    run_gz("adversarial.fixture.gz");
}

#[test]
fn fuzz_subset() {
    run_gz("fuzz.fixture.gz");
}

/// Second-round (independent verifier) corpora: `go-yaml verify` generators
/// (colliding map keys incl. NaN/-0 keys in maps of > 8 entries, merges,
/// anchors, random numerals, float keys, timestamps, Unicode, block
/// scalars, token soup, UTF-16/UTF-8 buffer boundaries) plus regression
/// cases.
#[test]
fn verify_corpora() {
    run_gz("verify.fixture.gz");
}

#[test]
fn full_corpora_from_env() {
    let Ok(dir) = std::env::var("GO_YAML_FIXTURE_DIR") else {
        return;
    };
    let mut total_bad = 0;
    let mut total = 0;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for p in entries {
        if p.extension().and_then(|x| x.to_str()) != Some("fixture") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        let label = p.display().to_string();
        let (n, bad) = big_stack(move || check_fixture_text(&label, &text));
        total += n;
        total_bad += bad;
    }
    eprintln!("full corpora: {total} cases, {total_bad} mismatching");
    assert_eq!(total_bad, 0);
}
