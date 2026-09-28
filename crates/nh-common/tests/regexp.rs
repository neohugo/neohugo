//! Differential tests of `nh_common::goregexp` (Go's `regexp` and `regexp/syntax`) against the
//! fixtures of `tools/go-oracle/nh-common/regexp` (go1.27.1).

use std::io::Read;

use nh_common::goregexp::syntax::{self, Flags};
use nh_common::goregexp::{Match, Regexp};
use serde_json::{Value as J, json};

fn fixture(name: &str) -> J {
    read_fixture(&format!(
        "{}/tests/fixtures/regexp/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn read_fixture(path: &str) -> J {
    let raw = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut s = String::new();
    flate2::read::GzDecoder::new(&raw[..])
        .read_to_string(&mut s)
        .unwrap();
    let j: J = serde_json::from_str(&s).unwrap();
    assert_eq!(j["go"], "go1.27.1");
    j
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// Decodes the oracle's byte string (a JSON string, or {"x": hex}).
fn db(j: &J) -> Vec<u8> {
    match j {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => hex(o["x"].as_str().unwrap()),
        other => panic!("not a byte string: {other}"),
    }
}

/// The oracle's `bs`.
fn bs(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => {
            let h: String = b.iter().map(|c| format!("{c:02x}")).collect();
            json!({ "x": h })
        }
    }
}

/// FNV-1a 64 (the oracle's checksum).
fn fnv(b: &[u8]) -> String {
    let mut h: u64 = 14695981039346656037;
    for &c in b {
        h ^= u64::from(c);
        h = h.wrapping_mul(1099511628211);
    }
    format!("{h:x}")
}

/// The oracle's `big`: long strings by length and checksum.
fn big(b: &[u8]) -> J {
    if b.len() > 4096 {
        return json!({ "len": b.len(), "ck": fnv(b) });
    }
    bs(b)
}

fn ints_text<'a>(ms: impl IntoIterator<Item = &'a [isize]>) -> Vec<u8> {
    let mut b = Vec::new();
    for m in ms {
        for v in m {
            b.extend_from_slice(v.to_string().as_bytes());
            b.push(b',');
        }
        b.push(b';');
    }
    b
}

fn opt_ints(m: Option<Match>) -> J {
    match m {
        None => J::Null,
        Some(m) => json!(m),
    }
}

fn all_ints(ms: Vec<Match>) -> J {
    if ms.is_empty() {
        return J::Null;
    }
    json!(ms)
}

fn all_pairs(ms: Vec<[usize; 2]>) -> J {
    if ms.is_empty() {
        return J::Null;
    }
    json!(ms)
}

fn list(v: Vec<&[u8]>) -> J {
    J::Array(v.into_iter().map(bs).collect())
}

fn list_or_null(v: Vec<&[u8]>) -> J {
    if v.is_empty() {
        return J::Null;
    }
    list(v)
}

fn split(re: &Regexp, s: &[u8], n: isize) -> J {
    if n == 0 {
        // Go returns nil.
        assert!(re.split(s, n).is_empty());
        return J::Null;
    }
    list(re.split(s, n))
}

/// The oracle's `header`: the compile error or the regexp's metadata.
fn header(c: &J) -> (J, Option<Regexp>) {
    let p = db(&c["p"]);
    let longest = c.get("longest").is_some();
    let mut h = json!({ "p": c["p"].clone() });
    if longest {
        h["longest"] = J::Bool(true);
    }
    match Regexp::compile_bytes(&p) {
        Err(e) => {
            h["err"] = big(e.as_bytes());
            (h, None)
        }
        Ok(mut re) => {
            if longest {
                re.longest();
            }
            let (prefix, complete) = re.literal_prefix();
            h["n"] = json!(re.num_subexp());
            h["names"] = json!(re.subexp_names());
            h["prefix"] = json!([bs(prefix), complete]);
            (h, Some(re))
        }
    }
}

/// Compares the header fields of a case.
fn check_header(c: &J, h: &J, bad: &mut Vec<String>) -> bool {
    for k in ["err", "n", "names", "prefix"] {
        if c.get(k) != h.get(k) {
            bad.push(format!(
                "{}: {k}: want {:?} got {:?}",
                c["p"],
                c.get(k),
                h.get(k)
            ));
            return false;
        }
    }
    true
}

/// The oracle's `full`: every operation over one input.
fn full(re: &Regexp, s: &[u8], templates: &[Vec<u8>]) -> J {
    let repl: Vec<J> = templates
        .iter()
        .map(|t| bs(&re.replace_all(s, t)))
        .collect();
    let fas3: Vec<J> = re
        .find_all_submatch(s, 3)
        .into_iter()
        .map(|sub| J::Array(sub.into_iter().map(|x| bs(x.unwrap_or(b""))).collect()))
        .collect();
    json!([
        re.match_string(s),
        opt_ints(re.find_submatch_index(s)),
        all_ints(re.find_all_submatch_index(s, -1)),
        all_pairs(re.find_all_index(s, -1)),
        list_or_null(re.find_all(s, 2)),
        if fas3.is_empty() {
            J::Null
        } else {
            J::Array(fas3)
        },
        bs(re.find(s).unwrap_or(b"")),
        match re.find_submatch(s) {
            None => J::Null,
            Some(sub) => J::Array(sub.into_iter().map(|x| bs(x.unwrap_or(b""))).collect()),
        },
        repl,
        bs(&re.replace_all_literal(s, b"<$1>")),
        bs(&re.replace_all_func(s, |m| format!("({})", m.len()).into_bytes())),
        split(re, s, -1),
        split(re, s, 2),
        split(re, s, 0),
        all_pairs(re.find_all_index(s, 1)),
    ])
}

fn inputs(j: &J, key: &str) -> Vec<Vec<u8>> {
    j[key].as_array().unwrap().iter().map(db).collect()
}

fn report(name: &str, n: usize, bad: &[String]) {
    eprintln!("{name}: {n} checks, {} mismatches", bad.len());
    assert!(
        bad.is_empty(),
        "{name}: {} of {n} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
}

fn run_matrix(name: &str) {
    let fx = fixture(name);
    let ins = inputs(&fx, "inputs");
    let templates = inputs(&fx, "templates");
    let mut bad = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        n += 1;
        let (h, re) = header(c);
        if !check_header(c, &h, &mut bad) {
            continue;
        }
        let Some(re) = re else { continue };
        let own_input = c.get("in").map(db);
        let ins: Vec<&[u8]> = match &own_input {
            Some(i) => vec![&i[..]],
            None => ins.iter().map(|i| &i[..]).collect(),
        };
        let want = c["r"].as_array().unwrap();
        assert_eq!(want.len(), ins.len());
        for (s, w) in ins.iter().zip(want) {
            n += 1;
            let got = full(&re, s, &templates);
            if &got != w {
                // Name the first differing operation.
                let (ga, wa) = (got.as_array().unwrap(), w.as_array().unwrap());
                let k = (0..ga.len()).find(|&k| ga[k] != wa[k]).unwrap_or(0);
                bad.push(format!(
                    "{} (longest {}) on {}: op {k}: want {} got {}",
                    c["p"],
                    c.get("longest").is_some(),
                    bs(s),
                    wa[k],
                    ga[k]
                ));
            }
        }
    }
    report(name, n, &bad);
}

#[test]
fn matrix() {
    run_matrix("matrix.json.gz");
}

#[test]
fn hugo_patterns() {
    run_matrix("hugo.json.gz");
}

#[test]
fn long_inputs() {
    let fx = fixture("long.json.gz");
    let ins = inputs(&fx, "inputs");
    let mut bad = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        n += 1;
        let (h, re) = header(c);
        if !check_header(c, &h, &mut bad) {
            continue;
        }
        let Some(re) = re else { continue };
        for (s, w) in ins.iter().zip(c["r"].as_array().unwrap()) {
            n += 1;
            let all = re.find_all_submatch_index(s, -1);
            let first100 = re.find_all_index(s, 100);
            let got = json!([
                re.match_string(s),
                opt_ints(re.find_submatch_index(s)),
                all.len(),
                fnv(&ints_text(all.iter().map(|m| &m[..]))),
                fnv(&ints_text(
                    first100
                        .iter()
                        .map(|m| [m[0] as isize, m[1] as isize])
                        .collect::<Vec<_>>()
                        .iter()
                        .map(|m| &m[..])
                )),
                fnv(&re.replace_all(s, b"<$1|$0>")),
                re.split(s, -1).len(),
            ]);
            if &got != w {
                bad.push(format!(
                    "{} (longest {}) on input of {} bytes: want {w} got {got}",
                    c["p"],
                    c.get("longest").is_some(),
                    s.len()
                ));
            }
        }
    }
    report("long", n, &bad);
}

#[test]
fn re2_search() {
    let fx = fixture("re2search.json.gz");
    let mut bad = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        n += 1;
        let (h, re) = header(c);
        if !check_header(c, &h, &mut bad) {
            continue;
        }
        let Some(re) = re else { continue };
        for w in c["r"].as_array().unwrap() {
            n += 1;
            let s = db(&w[0]);
            let got = json!([
                w[0].clone(),
                re.match_string(&s),
                opt_ints(re.find_submatch_index(&s)),
                all_ints(re.find_all_submatch_index(&s, -1)),
                bs(&re.replace_all(&s, b"<$1>")),
            ]);
            if &got != w {
                bad.push(format!(
                    "{} (longest {}): want {w} got {got}",
                    c["p"],
                    c.get("longest").is_some()
                ));
            }
        }
    }
    report("re2-search", n, &bad);
}

/// The checked-in fuzz fixture, and the out-of-repo ones named by `GOREGEXP_BIG` (paths
/// separated by `:`, written by the oracle's `-big` mode).
#[test]
fn fuzz() {
    run_fuzz("fuzz", &fixture("fuzz.json.gz"));
    if let Ok(paths) = std::env::var("GOREGEXP_BIG") {
        for p in paths.split(':').filter(|p| !p.is_empty()) {
            run_fuzz(p, &read_fixture(p));
        }
    }
}

fn run_fuzz(name: &str, fx: &J) {
    let mut bad = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        n += 1;
        let (h, re) = header(c);
        if !check_header(c, &h, &mut bad) {
            continue;
        }
        let (Some(re), Some(r)) = (re, c.get("r")) else {
            continue;
        };
        for w in r.as_array().unwrap() {
            n += 1;
            let s = db(&w[0]);
            let got = json!([
                w[0].clone(),
                all_ints(re.find_all_submatch_index(&s, -1)),
                bs(&re.replace_all(&s, b"<$1>")),
                split(&re, &s, -1),
            ]);
            if &got != w {
                bad.push(format!(
                    "{} (longest {}): want {w} got {got}",
                    c["p"],
                    c.get("longest").is_some()
                ));
            }
        }
    }
    report(name, n, &bad);
}

fn syntax_mode(p: &[u8], flags: Flags) -> J {
    match syntax::parse(p, flags) {
        Err(e) => json!({ "err": big(&e.error()) }),
        Ok(re) => {
            let s = re.simplify();
            let prog = syntax::compile(&s);
            json!({
                "re": big(re.string().as_bytes()),
                "simp": big(s.string().as_bytes()),
                "prog": big(prog.string().as_bytes()),
                "cap": re.max_cap(),
                "names": re.cap_names(),
            })
        }
    }
}

#[test]
fn syntax_dump() {
    let fx = fixture("syntax.json.gz");
    let mut bad = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        let p = db(&c["p"]);
        for (mode, flags) in [
            ("perl", syntax::PERL),
            ("posix", syntax::POSIX),
            ("lit", syntax::PERL | syntax::LITERAL),
        ] {
            n += 1;
            let got = syntax_mode(&p, flags);
            if got != c[mode] {
                bad.push(format!(
                    "{} ({mode}):\n  want {}\n  got  {}",
                    c["p"], c[mode], got
                ));
            }
        }
        n += 1;
        let got = match Regexp::compile_posix_bytes(&p) {
            Ok(_) => J::Null,
            Err(e) => big(e.as_bytes()),
        };
        if got != c["cposix"] {
            bad.push(format!(
                "{} (CompilePOSIX): want {} got {got}",
                c["p"], c["cposix"]
            ));
        }
    }
    report("syntax", n, &bad);
}

#[test]
fn unicode_tables() {
    let fx = fixture("tables.json.gz");
    let mut bad = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        n += 1;
        let p = c["p"].as_str().unwrap();
        let got = if p == "single-folds" {
            let mut single = String::new();
            for r in 0..=0x1ffff_i32 {
                if go_unicode::simple_fold(r) == r && r > 0x80 {
                    continue;
                }
                let re =
                    syntax::parse(format!(r"(?i)\x{{{r:x}}}").as_bytes(), syntax::PERL).unwrap();
                let prog = syntax::compile(&re.simplify());
                single.push_str(&re.string());
                single.push_str(&prog.string());
            }
            json!({ "p": p, "len": single.len(), "ck": fnv(single.as_bytes()) })
        } else if p.starts_with("(?i)[") && p.contains(r"\x{") {
            let re = syntax::parse(p.as_bytes(), syntax::PERL).unwrap();
            let s = re.string();
            json!({ "p": p, "len": s.len(), "ck": fnv(s.as_bytes()) })
        } else {
            match syntax::parse(p.as_bytes(), syntax::PERL) {
                Err(e) => json!({ "p": p, "err": e.to_string() }),
                Ok(re) => {
                    let ps = syntax::compile(&re.simplify()).string();
                    json!({ "p": p, "len": ps.len(), "ck": fnv(ps.as_bytes()) })
                }
            }
        };
        if &got != c {
            bad.push(format!("want {c} got {got}"));
        }
    }
    report("tables", n, &bad);
}
