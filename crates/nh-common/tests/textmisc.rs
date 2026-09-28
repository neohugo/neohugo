//! `kinds`, `files`, `text`, `hstrings`, `hugio`: the differential test against
//! `tools/go-oracle/nh-common/textmisc` and the Go test tables of those packages.

mod t02support;

use nh_common::files;
use nh_common::hstrings::{self, StringEqualFold};
use nh_common::hugio::{self, HasBytesWriter};
use nh_common::kinds;
use nh_common::loggers::{Level, Logger, Options};
use nh_common::text::{self, Position};
use serde_json::{Value as J, json};
use t02support::*;

#[test]
fn textmisc_matches_go() {
    let f = fixture("textmisc/textmisc.json.gz");
    let cases = f["cases"].as_array().unwrap();
    let mut bad = Vec::new();
    let mut counts = [0usize; 6];
    for c in cases {
        let got: J;
        let want: J;
        if let Some(s) = c.get("kinds") {
            let Ok(s) = String::from_utf8(bytes(s)) else {
                continue;
            };
            counts[0] += 1;
            want = c.clone();
            got = json!({
                "kinds": s,
                "GetKindMain": kinds::get_kind_main(&s),
                "GetKindAny": kinds::get_kind_any(&s),
                "IsBranch": kinds::is_branch(&s),
                "IsDeprecatedAndReplacedWith": kinds::is_deprecated_and_replaced_with(&s),
            });
        } else if let Some(s) = c.get("files") {
            let Ok(s) = String::from_utf8(bytes(s)) else {
                continue;
            };
            counts[1] += 1;
            want = c.clone();
            got = json!({
                "files": s,
                "ResolveComponentFolder": files::resolve_component_folder(&s),
                "IsComponentFolder": files::is_component_folder(&s),
                "IsContentDataExt": files::is_content_data_ext(&s),
            });
        } else if let Some(s) = c.get("text") {
            let b = bytes(s);
            counts[2] += 1;
            let chomp = enc(text::chomp_bytes(&b));
            let Ok(s) = String::from_utf8(b) else {
                if chomp != c["Chomp"] {
                    bad.push(format!("chomp_bytes: want {} got {chomp}", c["Chomp"]));
                }
                continue;
            };
            let mut lines = Vec::new();
            text::visit_lines_after(&s, |l| lines.push(J::String(l.to_string())));
            want = c.clone();
            got = json!({
                "text": s,
                "Chomp": text::chomp(&s),
                "Puts": text::puts(&s),
                "VisitLinesAfter": if lines.is_empty() { J::Null } else { J::Array(lines) },
            });
            if chomp != c["Chomp"] {
                bad.push(format!("chomp_bytes: want {} got {chomp}", c["Chomp"]));
            }
        } else if let Some(p) = c.get("position") {
            counts[3] += 1;
            let pos = Position {
                filename: p[0].as_str().unwrap().to_string(),
                line_number: p[1].as_i64().unwrap(),
                column_number: p[2].as_i64().unwrap(),
                offset: p[3].as_i64().unwrap(),
            };
            want = c.clone();
            got = json!({ "position": p, "String": pos.string(), "IsValid": pos.is_valid() });
        } else if let Some(fold) = c.get("fold") {
            let (Ok(s), Ok(o), Ok(t)) = (
                String::from_utf8(bytes(&fold[0])),
                String::from_utf8(bytes(&fold[1])),
                String::from_utf8(bytes(&fold[2])),
            ) else {
                continue;
            };
            counts[4] += 1;
            let sef = StringEqualFold(s.clone());
            want = json!({
                "EqualFold": c["EqualFold"],
                "Eq": c["Eq"],
                "InSlicEqualFold": c["InSlicEqualFold"],
                "InSlice": c["InSlice"],
            });
            got = json!({
                "EqualFold": sef.equal_fold(&o),
                "Eq": sef.eq_any(&go_value::Value::string(o.as_str())),
                "InSlicEqualFold": hstrings::in_slice_equal_fold(&[&t, &o], &s),
                "InSlice": hstrings::in_slice(&[&t, &o], &s),
            });
        } else if let Some(ps) = c.get("hasBytes") {
            counts[5] += 1;
            let patterns: Vec<Vec<u8>> = ps.as_array().unwrap().iter().map(bytes).collect();
            let stream = bytes(&c["stream"]);
            let writes: Vec<usize> = c["writes"]
                .as_array()
                .map(|a| a.iter().map(|n| n.as_u64().unwrap() as usize).collect())
                .unwrap_or_default();
            let res = catch(|| {
                let mut h = HasBytesWriter::new(patterns);
                let mut pos = 0;
                for n in writes {
                    assert_eq!(h.write(&stream[pos..pos + n]), n);
                    pos += n;
                }
                J::Array(h.patterns.iter().map(|p| J::Bool(p.matched)).collect())
            });
            if !same(&c["matched"], &res) {
                bad.push(format!(
                    "HasBytesWriter {ps} {:?}: want {} got {res:?}",
                    c["stream"], c["matched"]
                ));
            }
            continue;
        } else {
            continue;
        }
        if got != want {
            bad.push(format!("want {want}\n got {got}"));
        }
    }
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad[..bad.len().min(20)].join("\n")
    );
    eprintln!(
        "textmisc: kinds {}, files {}, text {}, positions {}, folds {}, hasBytes {}",
        counts[0], counts[1], counts[2], counts[3], counts[4], counts[5]
    );
}

// Go: resources/kinds/kinds_test.go:TestKind
#[test]
fn go_test_kind() {
    // Add tests for these constants to make sure they don't change
    assert_eq!(kinds::KIND_PAGE, "page");
    assert_eq!(kinds::KIND_HOME, "home");
    assert_eq!(kinds::KIND_SECTION, "section");
    assert_eq!(kinds::KIND_TAXONOMY, "taxonomy");
    assert_eq!(kinds::KIND_TERM, "term");

    assert_eq!(kinds::get_kind_main("TAXONOMYTERM"), kinds::KIND_TAXONOMY);
    assert_eq!(kinds::get_kind_main("Taxonomy"), kinds::KIND_TAXONOMY);
    assert_eq!(kinds::get_kind_main("Page"), kinds::KIND_PAGE);
    assert_eq!(kinds::get_kind_main("Home"), kinds::KIND_HOME);
    assert_eq!(kinds::get_kind_main("SEction"), kinds::KIND_SECTION);

    assert_eq!(kinds::get_kind_any("Page"), kinds::KIND_PAGE);
    assert_eq!(kinds::get_kind_any("Robotstxt"), kinds::KIND_ROBOTS_TXT);
}

// Go: hugofs/files/classifier_test.go:TestComponentFolders
#[test]
fn go_test_component_folders() {
    // It's important that these are absolutely right and not changed.
    assert_eq!(files::COMPONENT_FOLDERS.len(), 7);
    for (name, want) in [
        ("archetypes", true),
        ("layouts", true),
        ("data", true),
        ("i18n", true),
        ("assets", true),
        ("resources", false),
        ("static", true),
        ("content", true),
        ("foo", false),
        ("", false),
    ] {
        assert_eq!(files::is_component_folder(name), want, "{name}");
    }
}

// Go: common/text/position_test.go:TestPositionStringFormatter
#[test]
fn go_test_position_string_formatter() {
    let pos = Position {
        filename: "/my/file.txt".to_string(),
        line_number: 12,
        column_number: 13,
        offset: 14,
    };

    let f = text::create_position_string_formatter;
    assert_eq!(f(":file|:col|:line").format(&pos), "/my/file.txt|13|12");
    assert_eq!(f(":col|:file|:line").format(&pos), "13|/my/file.txt|12");
    assert_eq!(f("好::col").format(&pos), "好:13");
    assert_eq!(f("").format(&pos), "\"/my/file.txt:12:13\"");
    assert_eq!(pos.string(), "\"/my/file.txt:12:13\"");
}

// Go: common/text/transform_test.go:TestChomp, TestPuts, TestVisitLinesAfter
#[test]
fn go_test_transform() {
    assert_eq!(text::chomp("\nA\n"), "\nA");
    assert_eq!(text::chomp("A\r\n"), "A");

    assert_eq!(text::puts("A"), "A\n");
    assert_eq!(text::puts("\nA\n"), "\nA\n");
    assert_eq!(text::puts(""), "");

    let lines = "line 1\nline 2\n\nline 3";
    let mut collected = Vec::new();
    text::visit_lines_after(lines, |s| collected.push(s.to_string()));
    assert_eq!(collected, vec!["line 1\n", "line 2\n", "\n", "line 3"]);

    // Go: common/text/transform_test.go:TestRemoveAccents (the oracle test is tests/norm.rs).
    assert_eq!(text::remove_accents("Resumé".as_bytes()), b"Resume");
    assert_eq!(text::remove_accents(b"Hugo Rocks!"), b"Hugo Rocks!");
    assert_eq!(text::remove_accents_string("Resumé"), "Resume");
}

// Go: common/hstrings/strings_test.go:TestStringEqualFold
#[test]
fn go_test_string_equal_fold() {
    let s1 = "A";
    let s2 = "a";
    let f = |s: &str| StringEqualFold(s.to_string());
    assert!(f(s1).equal_fold(s2));
    assert!(f(s1).equal_fold(s1));
    assert!(f(s2).equal_fold(s1));
    assert!(f(s2).equal_fold(s2));
    assert!(!f(s1).equal_fold("b"));
    assert!(f(s1).eq_any(&go_value::Value::string(s2)));
    assert!(!f(s1).eq_any(&go_value::Value::string("b")));
    assert!(hstrings::in_slice_equal_fold(&["x", "A"], "a"));
    assert!(!hstrings::in_slice(&["x", "A"], "a"));
    assert!(hstrings::get_or_compile_regexp(r"\d+").is_err());
}

// Go: common/hugio/hasBytesWriter_test.go:TestHasBytesWriter (a fixed pseudo-random sequence
// instead of Go's time-seeded one).
#[test]
fn go_test_has_bytes_writer() {
    let neww = || HasBytesWriter::new(vec![b"__foo".to_vec()]);
    let mut seed = 42u64;
    let mut rnd_str = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        "ab cfo".repeat(((seed >> 33) % 33) as usize)
    };

    for _ in 0..22 {
        let mut h = neww();
        h.write(format!("{}abc __foobar{}", rnd_str(), rnd_str()).as_bytes());
        assert!(h.patterns[0].matched);

        let mut h = neww();
        h.write(format!("{}abc __f", rnd_str()).as_bytes());
        h.write(format!("oo bar{}", rnd_str()).as_bytes());
        assert!(h.patterns[0].matched);

        let mut h = neww();
        h.write(format!("{}abc __moo bar", rnd_str()).as_bytes());
        assert!(!h.patterns[0].matched);
    }

    let mut h = neww();
    h.write(b"__foo");
    assert!(h.patterns[0].matched);

    // Readers.
    let r = hugio::new_read_seeker_no_op_closer_from_string("abc");
    assert_eq!(r.read_string(), "abc");
}

// Go: common/loggers/logger_test.go (counters, distinct and suppressed entries).
#[test]
fn loggers_minimal() {
    let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let l = Logger::with_options(Options {
        level: Level::Warn,
        distinct_level: Some(Level::Warn),
        std_err: Some(sink.clone()),
        suppress_statements: ["my-id".to_string()].into_iter().collect(),
        ..Default::default()
    });
    l.infof("below the level");
    l.warnf("  a warning  ");
    l.warnf("  a warning  "); // distinct: dropped
    l.errorf("an error");
    l.erroridf("MY-ID", "suppressed");
    l.warnidf("other-id", "not suppressed");
    assert_eq!(l.logg_count(Level::Info), 0);
    assert_eq!(l.log_counter_warnings(), 2);
    assert_eq!(l.log_counter_errors(), 1);
    let out = String::from_utf8(sink.lock().unwrap().clone()).unwrap();
    assert!(out.starts_with("WARN  a warning\nERROR an error\nWARN  not suppressed\nYou can suppress this warning by adding the following to your site configuration:\nignoreLogs = ['other-id']\n"), "{out}");
    l.reset();
    assert_eq!(l.log_counter_errors(), 0);
}
