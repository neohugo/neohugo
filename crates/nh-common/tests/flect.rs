//! Differential tests for `nh_common::flect` (gobuffalo/flect) against
//! `tests/fixtures/flect/flect.json`, written by `tools/go-oracle/nh-common/flect`, plus flect's
//! own `*_test.go` tables.

use nh_common::flect;
use serde_json::Value as J;

fn fixture() -> J {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/flect/flect.json"
    );
    let b = std::fs::read(path).expect("read fixture");
    serde_json::from_slice(&b).expect("parse fixture")
}

/// A fixture string: a JSON string, or `{"hex": ...}` for invalid UTF-8.
fn bytes(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => hex(o["hex"].as_str().expect("hex")),
        other => panic!("not a string: {other}"),
    }
}

/// A fixture result: a string, or `{"panic": msg}`.
fn result(v: &J) -> Result<Vec<u8>, String> {
    match v {
        J::Object(o) if o.contains_key("panic") => Err(o["panic"].as_str().unwrap().to_string()),
        _ => Ok(bytes(v)),
    }
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn show(r: &Result<Vec<u8>, String>) -> String {
    match r {
        Ok(b) => format!("{:?}", String::from_utf8_lossy(b)),
        Err(e) => format!("panic({e})"),
    }
}

/// neohugo's `tpl/inflect` `Humanize` for a string argument (the composition T19 ports).
fn inflect_humanize(word: &[u8]) -> Result<Vec<u8>, String> {
    if word.is_empty() {
        return Ok(Vec::new());
    }
    if go_strconv::atoi(word).is_ok() {
        return Ok(flect::ordinalize_bytes(word));
    }
    let s = flect::try_humanize_bytes(word).map_err(|e| e.message().to_string())?;
    flect::try_humanize_bytes(&go_unicode::strings::to_lower(&s))
        .map_err(|e| e.message().to_string())
}

#[test]
fn corpus_matches_go() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() > 3000, "corpus too small: {}", cases.len());

    let mut bad = Vec::new();
    for c in cases {
        let input = bytes(&c["in"]);
        let s = String::from_utf8(input.clone()).ok();
        let checks: Vec<(&str, Result<Vec<u8>, String>)> = vec![
            ("pluralize", Ok(flect::pluralize_bytes(&input))),
            ("singularize", Ok(flect::singularize_bytes(&input))),
            (
                "humanize",
                flect::try_humanize_bytes(&input).map_err(|e| e.message().to_string()),
            ),
            ("titleize", Ok(flect::titleize_bytes(&input))),
            ("ordinalize", Ok(flect::ordinalize_bytes(&input))),
            ("inflect_humanize", inflect_humanize(&input)),
        ];
        let mut checks = checks;
        if let Some(s) = &s {
            // The &str wrappers agree with the byte forms.
            checks.push(("pluralize", Ok(flect::pluralize(s).into_bytes())));
            checks.push(("singularize", Ok(flect::singularize(s).into_bytes())));
            checks.push(("titleize", Ok(flect::titleize(s).into_bytes())));
            checks.push(("ordinalize", Ok(flect::ordinalize(s).into_bytes())));
            checks.push((
                "humanize",
                flect::try_humanize(s)
                    .map(String::into_bytes)
                    .map_err(|e| e.message().to_string()),
            ));
        }
        for (name, got) in checks {
            let want = result(&c[name]);
            if got != want {
                bad.push(format!(
                    "{name}({:?}): got {} want {}",
                    String::from_utf8_lossy(&input),
                    show(&got),
                    show(&want)
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
}

/// flect loads `inflections.json`/`acronyms.json` in `init()`; the oracle ran itself in a
/// directory holding each configuration.
#[test]
fn custom_data_matches_go() {
    let f = fixture();
    let configs = f["custom"].as_array().unwrap();
    assert!(configs.len() > 10);
    let mut bad = Vec::new();
    for cfg in configs {
        let name = cfg["name"].as_str().unwrap();
        let infl = cfg["inflections"].as_str().map(|s| s.as_bytes().to_vec());
        let acr = cfg["acronyms"].as_str().map(|s| s.as_bytes().to_vec());
        let run = |op: &str, input: &[u8]| {
            flect::with_custom_data_for_tests(infl.as_deref(), acr.as_deref(), op, input)
        };

        if let Some(msg) = cfg.get("init_panic").and_then(J::as_str) {
            let r = std::panic::catch_unwind(|| run("pluralize", b"x"));
            let got = match r {
                Ok(_) => "no panic".to_string(),
                Err(p) => p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default(),
            };
            if got != msg {
                bad.push(format!("{name}: init panic {got:?} want {msg:?}"));
            }
            continue;
        }

        let want_printed: Vec<String> = cfg["printed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect();
        let (_, printed) = run("pluralize", b"");
        if printed != want_printed {
            bad.push(format!("{name}: printed {printed:?} want {want_printed:?}"));
        }

        for (key, want) in cfg["results"].as_object().unwrap() {
            let (op, input) = key.split_once(':').unwrap();
            let (got, _) = run(op, input.as_bytes());
            let want = result(want);
            if got != want {
                bad.push(format!(
                    "{name}: {op}({input:?}): got {} want {}",
                    show(&got),
                    show(&want)
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
}

/// CM §0.10 acceptance.
#[test]
fn section_names() {
    assert_eq!(flect::pluralize("biscuit-roll"), "biscuit-rolls");
    assert_eq!(flect::pluralize("candy"), "candies");
    assert_eq!(flect::pluralize("jelly"), "jellies");
    assert_eq!(flect::pluralize("pastry"), "pastries");
    assert_eq!(flect::pluralize("popcorn"), "popcorns");
    assert_eq!(flect::pluralize("almonds"), "almonds");
    assert_eq!(flect::pluralize("corn-chips"), "corn-chips");
    assert_eq!(flect::humanize("potato-chips"), "Potato chips");
    assert_eq!(flect::humanize("ญี่ปุ่น"), "ญ ป น");
    assert_eq!(flect::humanize("เกาหลี"), "เกาหล");
    assert_eq!(flect::ordinalize("3"), "3rd");
}

// ---------------------------------------------------------------------------
// flect's own tests (flect_test.go, *_test.go); the tables are transcribed in
// tests/fixtures/flect/upstream_tables.rs.

include!("fixtures/flect/upstream_tables.rs");

/// pluralize_test.go `Test_Pluralize`.
#[test]
fn upstream_pluralize() {
    for &(singular, plural, _, do_pluralize) in SINGLE_PLURAL_ASSERTIONS {
        if do_pluralize {
            assert_eq!(flect::pluralize(singular), plural, "pluralize {singular}");
            assert_eq!(flect::pluralize(plural), plural, "pluralize {plural}");
        }
    }
}

/// pluralize_test.go `Test_PluralizeWithSize`.
#[test]
fn upstream_pluralize_with_size() {
    for &(singular, plural, do_singularize, do_pluralize) in SINGLE_PLURAL_ASSERTIONS {
        if do_singularize {
            for (s, n) in [(singular, -1), (plural, -1), (singular, 1), (plural, 1)] {
                assert_eq!(
                    flect::pluralize_with_size(s, n),
                    singular,
                    "pluralize {n} {s}"
                );
            }
        }
        if do_pluralize {
            for (s, n) in [
                (singular, -2),
                (plural, -2),
                (singular, 0),
                (plural, 0),
                (singular, 2),
                (plural, 2),
            ] {
                assert_eq!(
                    flect::pluralize_with_size(s, n),
                    plural,
                    "pluralize {n} {s}"
                );
            }
        }
    }
}

/// singularize_test.go `Test_Singularize`.
#[test]
fn upstream_singularize() {
    for &(singular, plural, do_singularize, _) in SINGLE_PLURAL_ASSERTIONS {
        if do_singularize {
            assert_eq!(flect::singularize(plural), singular, "singularize {plural}");
            assert_eq!(
                flect::singularize(singular),
                singular,
                "singularize {singular}"
            );
        }
    }
}

/// singularize_test.go `Test_SingularizeWithSize`.
#[test]
fn upstream_singularize_with_size() {
    for &(singular, plural, do_singularize, do_pluralize) in SINGLE_PLURAL_ASSERTIONS {
        if do_singularize {
            for (s, n) in [(plural, -1), (singular, -1), (plural, 1), (singular, 1)] {
                assert_eq!(
                    flect::singularize_with_size(s, n),
                    singular,
                    "singularize {n} {s}"
                );
            }
        }
        if do_pluralize {
            for (s, n) in [
                (plural, -2),
                (singular, -2),
                (plural, 0),
                (singular, 0),
                (plural, 2),
                (singular, 2),
            ] {
                assert_eq!(
                    flect::singularize_with_size(s, n),
                    plural,
                    "singularize {n} {s}"
                );
            }
        }
    }
}

/// humanize_test.go, titleize_test.go, capitalize_test.go, ordinalize_test.go: `f(act) == exp`
/// and `f(exp) == exp`.
#[test]
fn upstream_act_exp_tables() {
    type Table = &'static [(&'static str, &'static str)];
    type Case = (&'static str, fn(&str) -> String, Table);
    let tables: [Case; 4] = [
        ("Humanize", flect::humanize, HUMANIZE),
        ("Titleize", flect::titleize, TITLEIZE),
        ("Capitalize", flect::capitalize, CAPITALIZE),
        ("Ordinalize", flect::ordinalize, ORDINALIZE),
    ];
    for (name, f, table) in tables {
        assert!(!table.is_empty(), "{name}");
        for &(act, exp) in table {
            assert_eq!(f(act), exp, "{name}({act:?})");
            assert_eq!(f(exp), exp, "{name}({exp:?})");
        }
    }
}

/// ident_test.go `Test_New`.
#[test]
fn upstream_ident_new() {
    for &(original, parts) in IDENT_NEW {
        let i = flect::Ident::new(original.as_bytes());
        assert_eq!(i.original, original.as_bytes(), "{original:?}");
        let got: Vec<&[u8]> = i.parts.iter().map(|p| p.as_slice()).collect();
        let want: Vec<&[u8]> = parts.iter().map(|p| p.as_bytes()).collect();
        assert_eq!(got, want, "{original:?}");
    }
}

/// flect_test.go `Test_LoadInflections`, `Test_LoadInflectionsWrongSingular`,
/// `Test_LoadInflectionsWrongPlural`, `Test_LoadAcronyms` (on fresh package state).
#[test]
fn upstream_load_custom_data() {
    let infl = br#"{"baby":"bebe","xyz":"zyx"}"#;
    for (k, v) in [("baby", "bebe"), ("xyz", "zyx")] {
        let run = |op: &str, s: &str| {
            flect::with_custom_data_for_tests(Some(infl), None, op, s.as_bytes()).0
        };
        assert_eq!(run("pluralize", k), Ok(v.as_bytes().to_vec()));
        assert_eq!(run("pluralize", v), Ok(v.as_bytes().to_vec()));
        assert_eq!(run("singularize", k), Ok(k.as_bytes().to_vec()));
        assert_eq!(run("singularize", v), Ok(k.as_bytes().to_vec()));
    }
    for bad in [
        &br#"{"a file":"files"}"#[..],
        br#"{"beatle":"the beatles"}"#,
    ] {
        let (_, printed) = flect::with_custom_data_for_tests(Some(bad), None, "pluralize", b"x");
        assert_eq!(printed, ["inflection elements should be a single word"]);
    }
    let acr = br#"["ACC","TLC","LSA"]"#;
    for a in ["ACC", "TLC", "LSA"] {
        let lower = a.to_lowercase();
        let (got, printed) =
            flect::with_custom_data_for_tests(None, Some(acr), "titleize", lower.as_bytes());
        assert!(printed.is_empty());
        // an acronym part is upper-cased by xappend
        assert_eq!(got, Ok(a.as_bytes().to_vec()));
    }
}

#[test]
fn humanize_panics_like_go() {
    // A string that is not blank but keeps no characters indexes Parts[0] of an empty slice.
    for s in ["+", "😀", "ั", "+++"] {
        assert_eq!(
            flect::try_humanize(s)
                .err()
                .map(|e| e.message().to_string()),
            Some("runtime error: index out of range [0] with length 0".to_string()),
            "{s:?}"
        );
    }
    assert_eq!(
        flect::try_humanize(" ").map_err(|e| e.message().to_string()),
        Ok(" ".to_string())
    );
}
