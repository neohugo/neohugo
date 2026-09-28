//! `glob`: the differential tests against `tools/go-oracle/nh-common/glob` (gobwas/glob compile
//! trees, a pattern × input match matrix, hugofs/glob `GetGlob`, `FilenameFilter` decisions and
//! the path helpers), the Go tests of hugofs/glob (glob_test.go, filename_filter_test.go) and
//! gobwas/glob's own glob_test.go.

mod t02support;

use std::collections::HashMap;
use std::sync::Arc;

use nh_common::glob::filename_filter::{self as ff, FilenameFilter};
use nh_common::glob::glob::{
    filter_glob_parts, get_glob, has_glob_char, normalize_path, normalize_path_no_lower,
    resolve_root_dir,
};
use nh_common::glob::gobwas::{self, Rune};
use serde_json::Value as J;
use t02support::*;

fn seps(f: &J) -> Vec<Vec<Rune>> {
    f["separators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().chars().map(|c| c as Rune).collect())
        .collect()
}

#[test]
fn gobwas_compile_matches_go() {
    let f = fixture("glob/compile.json.gz");
    let seps = seps(&f);
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() > 1000);
    let mut bad = Vec::new();
    let mut checks = 0;
    for c in cases {
        let pattern = bytes(&c["pattern"]);
        for (sep, want) in seps.iter().zip(c["compile"].as_array().unwrap()) {
            checks += 1;
            let got = match catch(|| gobwas::compile(&pattern, sep)) {
                Ok(Ok(m)) => serde_json::json!({ "ok": m.string() }),
                Ok(Err(e)) => serde_json::json!({ "err": e }),
                Err(p) => serde_json::json!({ "panic": p }),
            };
            if &got != want {
                bad.push(format!(
                    "{:?} seps {:?}: want {want} got {got}",
                    String::from_utf8_lossy(&pattern),
                    sep
                ));
            }
        }
        // hugofs/glob.GetGlob
        if let Ok(p) = std::str::from_utf8(&pattern) {
            checks += 1;
            let got = match get_glob(p) {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(e) => serde_json::json!({ "err": e.message() }),
            };
            if got != c["getGlob"] {
                bad.push(format!("GetGlob({p:?}): want {} got {got}", c["getGlob"]));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
    eprintln!("compile: {} patterns, {checks} checks", cases.len());
}

fn row(m: impl Fn(&[u8]) -> bool, inputs: &[Vec<u8>]) -> String {
    inputs
        .iter()
        .map(|i| match catch(|| m(i)) {
            Ok(true) => '1',
            Ok(false) => '0',
            Err(_) => 'p',
        })
        .collect()
}

fn diff_row(want: &str, got: &str, inputs: &[Vec<u8>]) -> String {
    want.chars()
        .zip(got.chars())
        .zip(inputs)
        .filter(|((w, g), _)| w != g)
        .map(|((w, g), i)| format!("{:?} want {w} got {g}", String::from_utf8_lossy(i)))
        .take(5)
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn gobwas_match_matches_go() {
    let f = fixture("glob/match.json.gz");
    let seps = seps(&f);
    let inputs: Vec<Vec<u8>> = f["inputs"].as_array().unwrap().iter().map(bytes).collect();
    let cases = f["cases"].as_array().unwrap();
    let mut bad = Vec::new();
    let mut checks = 0;
    let mut panics = 0;
    for c in cases {
        let pattern = bytes(&c["pattern"]);
        for (sep, want) in seps.iter().zip(c["raw"].as_array().unwrap()) {
            let Some(want) = want.as_str() else { continue };
            let m = gobwas::compile(&pattern, sep).expect("Go compiled it");
            let got = row(|s| m.is_match(s), &inputs);
            checks += inputs.len();
            panics += want.matches('p').count();
            if got != want {
                bad.push(format!(
                    "{:?} seps {sep:?} ({}): {}",
                    String::from_utf8_lossy(&pattern),
                    m.string(),
                    diff_row(want, &got, &inputs)
                ));
            }
        }
        if let (Some(want), Ok(p)) = (c["hugo"].as_str(), std::str::from_utf8(&pattern)) {
            let g = get_glob(p).expect("Go compiled it");
            let got = row(|s| g.matches_bytes(s), &inputs);
            checks += inputs.len();
            if got != want {
                bad.push(format!("GetGlob({p:?}): {}", diff_row(want, &got, &inputs)));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
    eprintln!(
        "match: {} patterns × {} inputs, {checks} checks ({panics} Go panics reproduced)",
        cases.len(),
        inputs.len()
    );
}

fn string_list(v: &J) -> Option<Vec<String>> {
    v.as_array()
        .map(|a| a.iter().map(|s| s.as_str().unwrap().to_string()).collect())
}

#[test]
fn filename_filter_matches_go() {
    let f = fixture("glob/filter.json.gz");
    let cases = f["cases"].as_array().unwrap();
    let names: Vec<(Vec<u8>, bool)> = cases
        .iter()
        .find_map(|c| c.get("names"))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (bytes(&n[0]), n[1].as_bool().unwrap()))
        .collect();

    let mut built: HashMap<String, Option<FilenameFilter>> = HashMap::new();
    let mut bad = Vec::new();
    let mut checks = 0;
    for c in cases {
        let Some(cfg) = c.get("config") else { continue };
        let name = cfg["name"].as_str().unwrap().to_string();
        let inclusions = string_list(&cfg["inclusions"]);
        let exclusions = string_list(&cfg["exclusions"]);
        let func_eq = cfg["funcEq"].as_str().unwrap().to_string();
        let func_suffix = cfg["funcSuffix"].as_str().unwrap().to_string();
        let append_to = cfg["appendTo"].as_str().unwrap();

        let mut filter = if !func_eq.is_empty() {
            Ok(Some(FilenameFilter::new_for_inclusion_func(move |s| {
                s == func_eq
            })))
        } else if !func_suffix.is_empty() {
            Ok(Some(FilenameFilter::new_for_inclusion_func(move |s| {
                s.ends_with(func_suffix.as_str())
            })))
        } else {
            FilenameFilter::new_opt(inclusions.as_deref(), exclusions.as_deref())
        };
        if !append_to.is_empty() {
            filter = filter.map(|f| ff::append(built[append_to].as_ref(), f));
        }

        match filter {
            Err(e) => {
                checks += 1;
                if c["err"].as_str() != Some(e.message()) {
                    bad.push(format!("{name}: want {} got error {e}", c["err"]));
                }
            }
            Ok(filter) => {
                if c.get("err").is_some() {
                    bad.push(format!("{name}: want error {}", c["err"]));
                    continue;
                }
                if filter.is_none() != c["isNil"].as_bool().unwrap() {
                    bad.push(format!("{name}: isNil want {}", c["isNil"]));
                }
                let want = c["match"].as_str().unwrap();
                for ((n, is_dir), w) in names.iter().zip(want.chars()) {
                    let Ok(n) = std::str::from_utf8(n) else {
                        continue;
                    };
                    checks += 1;
                    let got = if ff::match_opt(filter.as_ref(), n, *is_dir) {
                        '1'
                    } else {
                        '0'
                    };
                    if got != w {
                        bad.push(format!("{name}: Match({n:?}, {is_dir}) want {w} got {got}"));
                    }
                }
                built.insert(name, filter);
            }
        }
    }

    // The hugofs/glob path helpers.
    let helpers = cases
        .iter()
        .find_map(|c| c.get("helpers"))
        .unwrap()
        .as_array()
        .unwrap();
    for h in helpers {
        let Ok(s) = String::from_utf8(bytes(&h["in"])) else {
            continue;
        };
        let parts: Vec<String> = s.split('/').map(str::to_string).collect();
        let got = serde_json::json!({
            "in": s,
            "NormalizePath": normalize_path(&s),
            "NormalizePathNoLower": normalize_path_no_lower(&s),
            "ResolveRootDir": resolve_root_dir(&s),
            "HasGlobChar": has_glob_char(&s),
            "FilterGlobParts": filter_glob_parts(parts),
            "QuoteMeta": enc(&gobwas::quote_meta(s.as_bytes())),
            "QuoteMetaCompileMatch": gobwas::compile(&gobwas::quote_meta(s.as_bytes()), &[])
                .is_ok_and(|m| m.is_match(s.as_bytes())),
        });
        checks += 7;
        if &got != h {
            bad.push(format!("helpers {s:?}: want {h} got {got}"));
        }
    }

    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
    eprintln!("filter: {} names, {checks} checks", names.len());
}

// ---------------------------------------------------------------------------
// Go: hugofs/glob/glob_test.go

// Go: hugofs/glob/glob_test.go:TestResolveRootDir
#[test]
fn go_test_resolve_root_dir() {
    for (input, expected) in [
        ("data/foo.json", "data"),
        ("a/b/**/foo.json", "a/b"),
        ("dat?a/foo.json", ""),
        ("a/b[a-c]/foo.json", "a"),
    ] {
        assert_eq!(resolve_root_dir(input), expected);
    }
}

// Go: hugofs/glob/glob_test.go:TestFilterGlobParts
#[test]
fn go_test_filter_glob_parts() {
    assert_eq!(
        filter_glob_parts(vec!["a".into(), "*".into(), "c".into()]),
        vec!["a".to_string(), "c".to_string()]
    );
}

// Go: hugofs/glob/glob_test.go:TestNormalizePath
#[test]
fn go_test_normalize_path() {
    for (input, expected) in [
        ("data/FOO.json", "data/foo.json"),
        ("/data/FOO.json", "data/foo.json"),
        ("./FOO.json", "foo.json"),
        ("//", ""),
    ] {
        assert_eq!(normalize_path(input), expected);
    }
}

// Go: hugofs/glob/glob_test.go:TestGetGlob
#[test]
fn go_test_get_glob() {
    let g = get_glob("**.JSON").unwrap();
    assert!(g.matches("data/my.jSon"));
}

// Go: hugofs/glob/filename_filter_test.go:TestFilenameFilter
#[test]
fn go_test_filename_filter() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();

    let f = FilenameFilter::new(&s(&["/a/b/c/foo.json"]), &s(&["**.json"]))
        .unwrap()
        .unwrap();
    assert!(!f.matches("/data/my.json", false));
    assert!(f.matches("/a/b/c/foo.json", false));
    assert!(!f.matches("/a/b/c/foo.bar", false));
    assert!(f.matches("/a/b/c", true));
    assert!(f.matches("/a/b", true));
    assert!(f.matches("/a", true));
    assert!(f.matches("/", true));
    assert!(f.matches("", true));

    let f = FilenameFilter::new(&s(&["/a/**/foo.json"]), &s(&["**.json"]))
        .unwrap()
        .unwrap();
    assert!(!f.matches("/data/my.json", false));
    assert!(f.matches("/a/b/c/d/e/foo.json", false));
    assert!(f.matches("/a/b/c", true));
    assert!(f.matches("/a/b/", true));
    assert!(f.matches("/", true));
    assert!(!f.matches("/b", true));

    let f = FilenameFilter::new(&s(&["/**/Foo.json"]), &[])
        .unwrap()
        .unwrap();
    assert!(f.matches("/a/b/c/d/e/foo.json", false));
    assert!(f.matches("/a/b/c/d/e/FOO.json", false));

    let nop = FilenameFilter::new(&[], &[]).unwrap();
    assert!(nop.is_none());
    assert!(ff::match_opt(nop.as_ref(), "ab.txt", false));

    let f = FilenameFilter::new(&s(&["**.json", "**.jpg"]), &[])
        .unwrap()
        .unwrap();
    assert!(f.matches("ab.json", false));
    assert!(f.matches("ab.jpg", false));
    assert!(!f.matches("ab.gif", false));

    let f = FilenameFilter::new(&[], &s(&["**.json", "**.jpg"]))
        .unwrap()
        .unwrap();
    assert!(!f.matches("ab.json", false));
    assert!(!f.matches("ab.jpg", false));
    assert!(f.matches("ab.gif", false));

    assert!(ff::match_opt(None, "ab.gif", false));

    let f = FilenameFilter::new_for_inclusion_func(|s| s.ends_with(".json"));
    assert!(f.matches("ab.json", false));
    assert!(!f.matches("ab.bson", false));

    // Append on a nil filter returns the other.
    let other = FilenameFilter::new_for_inclusion_func(|s| s == "/x");
    let chained = ff::append(None, Some(other)).unwrap();
    assert!(chained.matches("x", false));
    let _ = Arc::new(chained);
}

// ---------------------------------------------------------------------------
// Go: github.com/gobwas/glob@v0.2.3 glob_test.go

const PATTERN_ALL: &str = "[a-z][!a-x]*cat*[h][!b]*eyes*";
const FIXTURE_ALL_MATCH: &str = "my cat has very bright eyes";
const FIXTURE_ALL_MISMATCH: &str = "my dog has very bright eyes";
const PATTERN_PLAIN: &str = "google.com";
const FIXTURE_PLAIN_MATCH: &str = "google.com";
const FIXTURE_PLAIN_MISMATCH: &str = "gobwas.com";
const PATTERN_MULTIPLE: &str = "https://*.google.*";
const FIXTURE_MULTIPLE_MATCH: &str = "https://account.google.com";
const FIXTURE_MULTIPLE_MISMATCH: &str = "https://google.com";
const PATTERN_ALTERNATIVES: &str = "{https://*.google.*,*yandex.*,*yahoo.*,*mail.ru}";
const FIXTURE_ALTERNATIVES_MATCH: &str = "http://yahoo.com";
const FIXTURE_ALTERNATIVES_MISMATCH: &str = "http://google.com";
const PATTERN_ALTERNATIVES_SUFFIX: &str = "{https://*gobwas.com,http://exclude.gobwas.com}";
const FIXTURE_ALTERNATIVES_SUFFIX_FIRST_MATCH: &str = "https://safe.gobwas.com";
const FIXTURE_ALTERNATIVES_SUFFIX_FIRST_MISMATCH: &str = "http://safe.gobwas.com";
const FIXTURE_ALTERNATIVES_SUFFIX_SECOND: &str = "http://exclude.gobwas.com";
const PATTERN_PREFIX: &str = "abc*";
const PATTERN_SUFFIX: &str = "*def";
const PATTERN_PREFIX_SUFFIX: &str = "ab*ef";
const FIXTURE_PREFIX_SUFFIX_MATCH: &str = "abcdef";
const FIXTURE_PREFIX_SUFFIX_MISMATCH: &str = "af";
const PATTERN_ALTERNATIVES_COMBINE_LITE: &str = "{abc*def,abc?def,abc[zte]def}";
const FIXTURE_ALTERNATIVES_COMBINE_LITE: &str = "abczdef";
const PATTERN_ALTERNATIVES_COMBINE_HARD: &str = "{abc*[a-c]def,abc?[d-g]def,abc[zte]?def}";
const FIXTURE_ALTERNATIVES_COMBINE_HARD: &str = "abczqdef";

// Go: github.com/gobwas/glob glob_test.go:TestGlob
#[test]
fn gobwas_test_glob() {
    let tests: Vec<(bool, &str, &str, &[char])> = vec![
        (true, "* ?at * eyes", "my cat has very bright eyes", &[]),
        (true, "", "", &[]),
        (false, "", "b", &[]),
        (true, "*ä", "åä", &[]),
        (true, "abc", "abc", &[]),
        (true, "a*c", "abc", &[]),
        (true, "a*c", "a12345c", &[]),
        (true, "a?c", "a1c", &[]),
        (true, "a.b", "a.b", &['.']),
        (true, "a.*", "a.b", &['.']),
        (true, "a.**", "a.b.c", &['.']),
        (true, "a.?.c", "a.b.c", &['.']),
        (true, "a.?.?", "a.b.c", &['.']),
        (true, "?at", "cat", &[]),
        (true, "?at", "fat", &[]),
        (true, "*", "abc", &[]),
        (true, r"\*", "*", &[]),
        (true, "**", "a.b.c", &['.']),
        (false, "?at", "at", &[]),
        (false, "?at", "fat", &['f']),
        (false, "a.*", "a.b.c", &['.']),
        (false, "a.?.c", "a.bb.c", &['.']),
        (false, "*", "a.b.c", &['.']),
        (true, "*test", "this is a test", &[]),
        (true, "this*", "this is a test", &[]),
        (true, "*is *", "this is a test", &[]),
        (true, "*is*a*", "this is a test", &[]),
        (true, "**test**", "this is a test", &[]),
        (true, "**is**a***test*", "this is a test", &[]),
        (false, "*is", "this is a test", &[]),
        (false, "*no*", "this is a test", &[]),
        (true, "[!a]*", "this is a test3", &[]),
        (true, "*abc", "abcabc", &[]),
        (true, "**abc", "abcabc", &[]),
        (true, "???", "abc", &[]),
        (true, "?*?", "abc", &[]),
        (true, "?*?", "ac", &[]),
        (false, "sta", "stagnation", &[]),
        (true, "sta*", "stagnation", &[]),
        (false, "sta?", "stagnation", &[]),
        (false, "sta?n", "stagnation", &[]),
        (true, "{abc,def}ghi", "defghi", &[]),
        (true, "{abc,abcd}a", "abcda", &[]),
        (true, "{a,ab}{bc,f}", "abc", &[]),
        (true, "{*,**}{a,b}", "ab", &[]),
        (false, "{*,**}{a,b}", "ac", &[]),
        (true, "/{rate,[a-z][a-z][a-z]}*", "/rate", &[]),
        (true, "/{rate,[0-9][0-9][0-9]}*", "/rate", &[]),
        (true, "/{rate,[a-z][a-z][a-z]}*", "/usd", &[]),
        (true, "{*.google.*,*.yandex.*}", "www.google.com", &['.']),
        (true, "{*.google.*,*.yandex.*}", "www.yandex.com", &['.']),
        (false, "{*.google.*,*.yandex.*}", "yandex.com", &['.']),
        (false, "{*.google.*,*.yandex.*}", "google.com", &['.']),
        (true, "{*.google.*,yandex.*}", "www.google.com", &['.']),
        (true, "{*.google.*,yandex.*}", "yandex.com", &['.']),
        (false, "{*.google.*,yandex.*}", "www.yandex.com", &['.']),
        (false, "{*.google.*,yandex.*}", "google.com", &['.']),
        (true, PATTERN_ALL, FIXTURE_ALL_MATCH, &[]),
        (false, PATTERN_ALL, FIXTURE_ALL_MISMATCH, &[]),
        (true, PATTERN_PLAIN, FIXTURE_PLAIN_MATCH, &[]),
        (false, PATTERN_PLAIN, FIXTURE_PLAIN_MISMATCH, &[]),
        (true, PATTERN_MULTIPLE, FIXTURE_MULTIPLE_MATCH, &[]),
        (false, PATTERN_MULTIPLE, FIXTURE_MULTIPLE_MISMATCH, &[]),
        (true, PATTERN_ALTERNATIVES, FIXTURE_ALTERNATIVES_MATCH, &[]),
        (
            false,
            PATTERN_ALTERNATIVES,
            FIXTURE_ALTERNATIVES_MISMATCH,
            &[],
        ),
        (
            true,
            PATTERN_ALTERNATIVES_SUFFIX,
            FIXTURE_ALTERNATIVES_SUFFIX_FIRST_MATCH,
            &[],
        ),
        (
            false,
            PATTERN_ALTERNATIVES_SUFFIX,
            FIXTURE_ALTERNATIVES_SUFFIX_FIRST_MISMATCH,
            &[],
        ),
        (
            true,
            PATTERN_ALTERNATIVES_SUFFIX,
            FIXTURE_ALTERNATIVES_SUFFIX_SECOND,
            &[],
        ),
        (
            true,
            PATTERN_ALTERNATIVES_COMBINE_HARD,
            FIXTURE_ALTERNATIVES_COMBINE_HARD,
            &[],
        ),
        (
            true,
            PATTERN_ALTERNATIVES_COMBINE_LITE,
            FIXTURE_ALTERNATIVES_COMBINE_LITE,
            &[],
        ),
        (true, PATTERN_PREFIX, FIXTURE_PREFIX_SUFFIX_MATCH, &[]),
        (false, PATTERN_PREFIX, FIXTURE_PREFIX_SUFFIX_MISMATCH, &[]),
        (true, PATTERN_SUFFIX, FIXTURE_PREFIX_SUFFIX_MATCH, &[]),
        (false, PATTERN_SUFFIX, FIXTURE_PREFIX_SUFFIX_MISMATCH, &[]),
        (
            true,
            PATTERN_PREFIX_SUFFIX,
            FIXTURE_PREFIX_SUFFIX_MATCH,
            &[],
        ),
        (
            false,
            PATTERN_PREFIX_SUFFIX,
            FIXTURE_PREFIX_SUFFIX_MISMATCH,
            &[],
        ),
    ];
    for (should, pattern, input, delimiters) in tests {
        let seps: Vec<Rune> = delimiters.iter().map(|c| *c as Rune).collect();
        let g = gobwas::compile(pattern.as_bytes(), &seps).unwrap();
        assert_eq!(
            g.is_match(input.as_bytes()),
            should,
            "pattern {pattern:?} matching {input:?}\n{}",
            g.string()
        );
    }
}

// Go: github.com/gobwas/glob glob_test.go:TestQuoteMeta
#[test]
fn gobwas_test_quote_meta() {
    for (input, out) in [
        ("[foo*]", r"\[foo\*\]"),
        ("{foo*}", r"\{foo\*\}"),
        (r"*?\[]{}", r"\*\?\\\[\]\{\}"),
        (r"some text and *?\[]{}", r"some text and \*\?\\\[\]\{\}"),
    ] {
        let act = gobwas::quote_meta(input.as_bytes());
        assert_eq!(act, out.as_bytes());
        assert!(gobwas::compile(&act, &[]).is_ok());
    }
}

/// Two Go panics are reproduced on purpose: `Row` slicing past its input.
#[test]
fn gobwas_row_panic() {
    let g = gobwas::compile(b"a{,}", &[]).unwrap();
    assert!(catch(|| g.is_match(b"a")).is_err());
}
