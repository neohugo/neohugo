//! Ports of x/text collate/collate_test.go tables (TestProcessWeights,
//! TestKeyFromElems, TestNumeric). TestKey/TestCompare/TestGetColElems need
//! collate/build (the table builder), which is not ported.

use super::option::AlternateHandling as Alt;
use super::*;
use crate::colltab::collelem::{IGNORE, NUM_LEVELS, make_elem, make_quaternary};

const MAX_Q: i32 = 0x1FFFFF;

#[derive(Clone, Copy)]
struct W(i32, i32, i32, i32);

fn w(ce: &[i32]) -> W {
    W(
        ce[0],
        *ce.get(1).unwrap_or(&0x20),
        *ce.get(2).unwrap_or(&0x2),
        *ce.get(3).unwrap_or(&0),
    )
}

fn wpq(p: i32, q: i32) -> W {
    W(p, 0x20, 0x2, q)
}
fn wsq(s: i32, q: i32) -> W {
    W(0, s, 0x2, q)
}
fn wq(q: i32) -> W {
    W(0, 0, 0, q)
}
const ZERO: W = W(0, 0, 0, 0);

fn convert(ws: &[W]) -> Vec<Elem> {
    ws.iter()
        .map(|w| {
            let mut e = make_elem(w.0, w.1, w.2, 0).unwrap_or(Elem(0));
            if e == IGNORE && w.3 > 0 {
                e = make_quaternary(w.3);
            }
            e
        })
        .collect()
}

struct Opts {
    lev: i32,
    alt: Alt,
    top: u32,
    backwards: bool,
    case_level: bool,
}

fn o(alt: Alt, top: u32) -> Opts {
    Opts {
        lev: 0,
        alt,
        top,
        backwards: false,
        case_level: false,
    }
}

impl Opts {
    fn collator(&self) -> Collator {
        let mut c = new_collator(Box::new(NullWeighter));
        let mut level = self.lev - 1;
        if level < 0 {
            level = QUATERNARY as i32;
        }
        let mut ignore = [false; NUM_LEVELS];
        for (i, x) in ignore.iter_mut().enumerate() {
            *x = level < i as i32;
        }
        c.options.ignore = ignore;
        c.options.alternate = self.alt;
        c.options.backwards = self.backwards;
        c.options.case_level = self.case_level;
        c.options.variable_top = self.top;
        c
    }
}

// Go: collate/collate_test.go:TestProcessWeights
#[test]
fn test_process_weights() {
    let w = |x: &[i32]| w(x);
    let tests: Vec<(Opts, Vec<W>, Vec<W>)> = vec![
        // Shifted
        (
            o(Alt::Shifted, 100),
            vec![w(&[200]), w(&[300]), w(&[400])],
            vec![wpq(200, MAX_Q), wpq(300, MAX_Q), wpq(400, MAX_Q)],
        ),
        (
            o(Alt::Shifted, 250),
            vec![w(&[200]), w(&[300]), w(&[400])],
            vec![wq(200), wpq(300, MAX_Q), wpq(400, MAX_Q)],
        ),
        (
            o(Alt::Shifted, 999),
            vec![w(&[1000]), w(&[200]), w(&[300]), w(&[400])],
            vec![wpq(1000, MAX_Q), wq(200), wq(300), wq(400)],
        ),
        (
            o(Alt::Shifted, 999),
            vec![w(&[0, 10]), w(&[1000])],
            vec![wsq(10, MAX_Q), wpq(1000, MAX_Q)],
        ),
        (
            o(Alt::Shifted, 250),
            vec![w(&[200]), w(&[0, 10]), w(&[300]), w(&[0, 15]), w(&[400])],
            vec![
                wq(200),
                ZERO,
                wpq(300, MAX_Q),
                wsq(15, MAX_Q),
                wpq(400, MAX_Q),
            ],
        ),
        (
            o(Alt::Shifted, 250),
            vec![
                w(&[200]),
                w(&[0, 0, 10]),
                w(&[300]),
                w(&[0, 0, 15]),
                w(&[400]),
            ],
            vec![
                wq(200),
                ZERO,
                wpq(300, MAX_Q),
                W(0, 0, 15, MAX_Q),
                wpq(400, MAX_Q),
            ],
        ),
        (
            o(Alt::Shifted, 250),
            vec![w(&[200]), ZERO, w(&[300]), ZERO, w(&[400])],
            vec![wq(200), ZERO, wpq(300, MAX_Q), ZERO, wpq(400, MAX_Q)],
        ),
        // ShiftTrimmed (same as Shifted)
        (
            o(Alt::ShiftTrimmed, 100),
            vec![w(&[200]), w(&[300]), w(&[400])],
            vec![wpq(200, MAX_Q), wpq(300, MAX_Q), wpq(400, MAX_Q)],
        ),
        (
            o(Alt::ShiftTrimmed, 250),
            vec![w(&[200]), w(&[300]), w(&[400])],
            vec![wq(200), wpq(300, MAX_Q), wpq(400, MAX_Q)],
        ),
        (
            o(Alt::ShiftTrimmed, 999),
            vec![w(&[1000]), w(&[200]), w(&[300]), w(&[400])],
            vec![wpq(1000, MAX_Q), wq(200), wq(300), wq(400)],
        ),
        (
            o(Alt::ShiftTrimmed, 999),
            vec![w(&[0, 10]), w(&[1000])],
            vec![wsq(10, MAX_Q), wpq(1000, MAX_Q)],
        ),
        (
            o(Alt::ShiftTrimmed, 250),
            vec![w(&[200]), w(&[0, 10]), w(&[300]), w(&[0, 15]), w(&[400])],
            vec![
                wq(200),
                ZERO,
                wpq(300, MAX_Q),
                wsq(15, MAX_Q),
                wpq(400, MAX_Q),
            ],
        ),
        (
            o(Alt::ShiftTrimmed, 250),
            vec![
                w(&[200]),
                w(&[0, 0, 10]),
                w(&[300]),
                w(&[0, 0, 15]),
                w(&[400]),
            ],
            vec![
                wq(200),
                ZERO,
                wpq(300, MAX_Q),
                W(0, 0, 15, MAX_Q),
                wpq(400, MAX_Q),
            ],
        ),
        (
            o(Alt::ShiftTrimmed, 250),
            vec![w(&[200]), ZERO, w(&[300]), ZERO, w(&[400])],
            vec![wq(200), ZERO, wpq(300, MAX_Q), ZERO, wpq(400, MAX_Q)],
        ),
        // Blanked
        (
            o(Alt::Blanked, 100),
            vec![w(&[200]), w(&[300]), w(&[400])],
            vec![w(&[200]), w(&[300]), w(&[400])],
        ),
        (
            o(Alt::Blanked, 250),
            vec![w(&[200]), w(&[300]), w(&[400])],
            vec![ZERO, w(&[300]), w(&[400])],
        ),
        (
            o(Alt::Blanked, 999),
            vec![w(&[1000]), w(&[200]), w(&[300]), w(&[400])],
            vec![w(&[1000]), ZERO, ZERO, ZERO],
        ),
        (
            o(Alt::Blanked, 999),
            vec![w(&[0, 10]), w(&[1000])],
            vec![w(&[0, 10]), w(&[1000])],
        ),
        (
            o(Alt::Blanked, 250),
            vec![w(&[200]), w(&[0, 10]), w(&[300]), w(&[0, 15]), w(&[400])],
            vec![ZERO, ZERO, w(&[300]), w(&[0, 15]), w(&[400])],
        ),
        (
            o(Alt::Blanked, 250),
            vec![
                w(&[200]),
                w(&[0, 0, 10]),
                w(&[300]),
                w(&[0, 0, 15]),
                w(&[400]),
            ],
            vec![ZERO, ZERO, w(&[300]), w(&[0, 0, 15]), w(&[400])],
        ),
        (
            o(Alt::Blanked, 250),
            vec![w(&[200]), ZERO, w(&[300]), ZERO, w(&[400])],
            vec![ZERO, ZERO, w(&[300]), ZERO, w(&[400])],
        ),
        // Non-ignorable: input is always equal to output.
        (
            o(Alt::NonIgnorable, 999),
            vec![w(&[1000]), w(&[200]), w(&[300]), w(&[400])],
            vec![w(&[1000]), w(&[200]), w(&[300]), w(&[400])],
        ),
        (
            o(Alt::NonIgnorable, 250),
            vec![w(&[200]), w(&[0, 10]), w(&[300]), w(&[0, 15]), w(&[400])],
            vec![w(&[200]), w(&[0, 10]), w(&[300]), w(&[0, 15]), w(&[400])],
        ),
        (
            o(Alt::NonIgnorable, 250),
            vec![
                w(&[200]),
                w(&[0, 0, 10]),
                w(&[300]),
                w(&[0, 0, 15]),
                w(&[400]),
            ],
            vec![
                w(&[200]),
                w(&[0, 0, 10]),
                w(&[300]),
                w(&[0, 0, 15]),
                w(&[400]),
            ],
        ),
        (
            o(Alt::NonIgnorable, 250),
            vec![w(&[200]), ZERO, w(&[300]), ZERO, w(&[400])],
            vec![w(&[200]), ZERO, w(&[300]), ZERO, w(&[400])],
        ),
    ];
    for (i, (opt, inp, out)) in tests.iter().enumerate() {
        let mut in_e = convert(inp);
        let out_e = convert(out);
        process_weights(opt.alt, opt.top, &mut in_e);
        assert_eq!(in_e, out_e, "{i}");
    }
}

// Go: collate/collate_test.go:TestKeyFromElems
#[test]
fn test_key_from_elems() {
    let w = |x: &[i32]| w(x);
    let def_s = 0x20u8;
    let def_t = 0x2u8;
    let sep = 0u8;
    let with = |mut op: Opts, f: &dyn Fn(&mut Opts)| {
        f(&mut op);
        op
    };
    let tests: Vec<(Opts, Vec<W>, Vec<u8>)> = vec![
        (
            o(Alt::Shifted, 0),
            vec![w(&[0x200]), w(&[0x7FFF]), w(&[0, 0x30]), w(&[0x100])],
            vec![
                0x2, 0, 0x7F, 0xFF, 0x1, 0x00, sep, sep, 0, def_s, 0, def_s, 0, 0x30, 0, def_s,
                sep, sep, def_t, def_t, def_t, def_t, sep, 0xFF, 0xFF, 0xFF, 0xFF,
            ],
        ),
        (
            o(Alt::Shifted, 0),
            vec![
                w(&[0x200]),
                ZERO,
                w(&[0x7FFF]),
                w(&[0, 0x30]),
                ZERO,
                w(&[0x100]),
            ],
            vec![
                0x2, 0, 0x7F, 0xFF, 0x1, 0x00, sep, sep, 0, def_s, 0, def_s, 0, 0x30, 0, def_s,
                sep, sep, def_t, def_t, def_t, def_t, sep, 0xFF, 0xFF, 0xFF, 0xFF,
            ],
        ),
        (
            o(Alt::Shifted, 0),
            vec![w(&[0x200]), w(&[0x8000]), w(&[0, 0x30]), w(&[0x12345])],
            vec![
                0x2, 0, 0x80, 0x80, 0x00, 0x81, 0x23, 0x45, sep, sep, 0, def_s, 0, def_s, 0, 0x30,
                0, def_s, sep, sep, def_t, def_t, def_t, def_t, sep, 0xFF, 0xFF, 0xFF, 0xFF,
            ],
        ),
        (
            with(o(Alt::Shifted, 0), &|o| o.backwards = true),
            vec![w(&[0x200]), w(&[0x7FFF]), w(&[0, 0x30]), w(&[0x100])],
            vec![
                0x2, 0, 0x7F, 0xFF, 0x1, 0x00, sep, sep, 0, def_s, 0, 0x30, 0, def_s, 0, def_s,
                sep, sep, def_t, def_t, def_t, def_t, sep, 0xFF, 0xFF, 0xFF, 0xFF,
            ],
        ),
        (
            with(o(Alt::Shifted, 0), &|o| o.lev = 3),
            vec![
                w(&[0x200]),
                ZERO,
                w(&[0x7FFF]),
                w(&[0, 0x30]),
                ZERO,
                w(&[0x100]),
            ],
            vec![
                0x2, 0, 0x7F, 0xFF, 0x1, 0x00, sep, sep, 0, def_s, 0, def_s, 0, 0x30, 0, def_s,
                sep, sep, def_t, def_t, def_t, def_t,
            ],
        ),
        (
            with(o(Alt::Shifted, 0), &|o| o.lev = 2),
            vec![
                w(&[0x200]),
                ZERO,
                w(&[0x7FFF]),
                w(&[0, 0x30]),
                ZERO,
                w(&[0x100]),
            ],
            vec![
                0x2, 0, 0x7F, 0xFF, 0x1, 0x00, sep, sep, 0, def_s, 0, def_s, 0, 0x30, 0, def_s,
            ],
        ),
        (
            with(o(Alt::Shifted, 0), &|o| o.lev = 1),
            vec![
                w(&[0x200]),
                ZERO,
                w(&[0x7FFF]),
                w(&[0, 0x30]),
                ZERO,
                w(&[0x100]),
            ],
            vec![0x2, 0, 0x7F, 0xFF, 0x1, 0x00],
        ),
        (
            o(Alt::ShiftTrimmed, 0x250),
            vec![
                w(&[0x300]),
                w(&[0x200]),
                w(&[0x7FFF]),
                w(&[0, 0x30]),
                w(&[0x800]),
            ],
            vec![
                0x3, 0, 0x7F, 0xFF, 0x8, 0x00, sep, sep, 0, def_s, 0, def_s, 0, 0x30, 0, def_s,
                sep, sep, def_t, def_t, def_t, def_t, sep, 0xFF, 0x2, 0,
            ],
        ),
        (
            with(o(Alt::Shifted, 0), &|o| {
                o.lev = 1;
                o.case_level = true
            }),
            vec![w(&[0x200]), w(&[0x7FFF]), w(&[0, 0x30]), w(&[0x100])],
            vec![
                0x2, 0, 0x7F, 0xFF, 0x1, 0x00, sep, sep, sep, sep, def_t, def_t, def_t, def_t,
            ],
        ),
    ];
    for (i, (opt, inp, out)) in tests.iter().enumerate() {
        let mut buf = Buffer::new();
        let mut in_e = convert(inp);
        process_weights(opt.alt, opt.top, &mut in_e);
        opt.collator().key_from_elems(&mut buf, &in_e);
        assert_eq!(&buf.key, out, "{i}");
    }
}

// Go: collate/collate_test.go:TestNumeric
#[test]
fn test_numeric() {
    let mut c = Collator::from_tag(&language::english(), &[option::LOOSE, option::NUMERIC]);
    for (i, (a, b, want)) in [
        ("1", "2", -1),
        ("2", "12", -1),
        ("２", "１２", -1),
        ("₂", "₁₂", 1),
        ("②", "①②", 1),
        ("\u{10859}", "\u{10858}\u{10859}", 1),
        ("12", "2", 1),
        ("A-1", "A-2", -1),
        ("A-2", "A-12", -1),
        ("A-12", "A-2", 1),
        ("A-0001", "A-1", 0),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(c.compare_string(a, b), want, "{i}: {a:?} {b:?}");
    }
}

/// neohugo usage: langs.NewLanguage collators for the site languages.
#[test]
fn hugo_language_collators() {
    // th tailoring (CLDR 23): Thai prevowels are reordered, Latin first.
    let mut th = Collator::for_hugo_language("th");
    assert_eq!(th.compare_string("เนย", "ปาร์ตี้"), -1);
    assert_eq!(th.compare_string("INS 941", "เกลือไอโอดีน"), -1);
    let mut en = Collator::for_hugo_language("en");
    assert_eq!(en.compare_string("apple", "Banana"), -1);
    assert_eq!(en.compare_string("a", "A"), -1);
    assert_eq!(en.compare_string("", ""), 0);
    // Invalid language keys fall back to English like Go.
    assert_eq!(Collator::match_index(&language::english()), 20);
    assert!(language::parse("not a tag!").is_err());
}

#[test]
fn collator_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Collator>();
    assert_send_sync::<language::Tag>();
}

// Go: collate/option_test.go:TestOptions
#[test]
fn test_options() {
    use crate::colltab::collelem::{IDENTITY, PRIMARY};
    use option::{IGNORE_CASE, IGNORE_DIACRITICS, IGNORE_WIDTH, LOOSE, NUMERIC};
    // ignore(level): ignore every level > level.
    let ignore = |l: usize| -> [bool; NUM_LEVELS] {
        let mut v = [false; NUM_LEVELS];
        for (i, x) in v.iter_mut().enumerate() {
            *x = i > l;
        }
        v
    };
    let default_ignore = ignore(TERTIARY);
    let tag = |s: &str| option::options_from_tag(&language::parse(s).unwrap());
    struct Want {
        ignore: [bool; NUM_LEVELS],
        case_level: bool,
        backwards: bool,
        numeric: bool,
        alternate: Alt,
    }
    let w = |ignore: [bool; NUM_LEVELS]| Want {
        ignore,
        case_level: false,
        backwards: false,
        numeric: false,
        alternate: Alt::NonIgnorable,
    };
    let cases: Vec<(Vec<CollOption>, Want)> = vec![
        (vec![], w(default_ignore)),
        (vec![IGNORE_DIACRITICS], w([false, true, false, true, true])),
        (vec![IGNORE_CASE, IGNORE_DIACRITICS], w(ignore(PRIMARY))),
        (
            vec![IGNORE_DIACRITICS, IGNORE_WIDTH],
            Want {
                case_level: true,
                ..w(ignore(PRIMARY))
            },
        ),
        (
            vec![IGNORE_WIDTH, IGNORE_DIACRITICS],
            Want {
                case_level: true,
                ..w(ignore(PRIMARY))
            },
        ),
        (vec![IGNORE_CASE, IGNORE_WIDTH], w(ignore(SECONDARY))),
        (vec![IGNORE_CASE, IGNORE_WIDTH, LOOSE], w(ignore(PRIMARY))),
        (
            vec![option::FORCE, IGNORE_CASE, IGNORE_WIDTH, LOOSE],
            w([false, true, true, true, false]),
        ),
        (vec![IGNORE_DIACRITICS, IGNORE_CASE], w(ignore(PRIMARY))),
        (
            vec![NUMERIC],
            Want {
                numeric: true,
                ..w(default_ignore)
            },
        ),
        (vec![tag("und-u-ks-level1")], w(ignore(PRIMARY))),
        (vec![tag("und-u-ks-level4")], w(ignore(QUATERNARY))),
        (vec![tag("und-u-ks-identic")], w([false; NUM_LEVELS])),
        (
            vec![tag("und-u-kn-true-kb-true-kc-true")],
            Want {
                case_level: true,
                backwards: true,
                numeric: true,
                ..w(default_ignore)
            },
        ),
        (
            vec![
                tag("und-u-kn-true-kb-true-kc-true"),
                tag("und-u-kn-false-kb-false-kc-false"),
            ],
            w(default_ignore),
        ),
        (
            vec![
                tag("und-u-kn-true-kb-true-kc-true"),
                tag("und-u-kn-foo-kb-foo-kc-foo"),
            ],
            Want {
                case_level: true,
                backwards: true,
                numeric: true,
                ..w(default_ignore)
            },
        ),
        // Normal options take precedence over tag options.
        (
            vec![NUMERIC, IGNORE_CASE, tag("und-u-kn-false-kc-true")],
            Want {
                numeric: true,
                ..w(ignore(SECONDARY))
            },
        ),
        (
            vec![tag("und-u-ka-shifted")],
            Want {
                alternate: Alt::Shifted,
                ..w(default_ignore)
            },
        ),
        (
            vec![tag("und-u-ka-blanked")],
            Want {
                alternate: Alt::Blanked,
                ..w(default_ignore)
            },
        ),
        (
            vec![tag("und-u-ka-posix")],
            Want {
                alternate: Alt::ShiftTrimmed,
                ..w(default_ignore)
            },
        ),
    ];
    for (i, (opts, want)) in cases.into_iter().enumerate() {
        let mut c = new_collator(Box::new(NullWeighter));
        c.options.set_options(&opts);
        let o = &c.options;
        assert_eq!(o.ignore, want.ignore, "{i}: ignore");
        assert_eq!(o.case_level, want.case_level, "{i}: caseLevel");
        assert_eq!(o.backwards, want.backwards, "{i}: backwards");
        assert_eq!(o.numeric, want.numeric, "{i}: numeric");
        assert_eq!(o.alternate, want.alternate, "{i}: alternate");
        let _ = IDENTITY;
    }
}

// Go: collate/option_test.go:TestAlternateSortTypes
#[test]
fn test_alternate_sort_types() {
    let cases: [(&str, [&str; 4], [&str; 4]); 2] = [
        (
            "zh,cmn,zh-Hant-u-co-pinyin,zh-HK-u-co-pinyin,zh-pinyin",
            ["爸爸", "妈妈", "儿子", "女儿"],
            ["爸爸", "儿子", "妈妈", "女儿"],
        ),
        (
            "zh-Hant,zh-u-co-stroke,zh-Hant-u-co-stroke",
            ["爸爸", "妈妈", "儿子", "女儿"],
            ["儿子", "女儿", "妈妈", "爸爸"],
        ),
    ];
    for (lang, input, want) in cases {
        for tag in lang.split(',') {
            let mut got = input.to_vec();
            Collator::from_tag(&language::parse(tag).unwrap(), &[]).sort_strings(&mut got);
            assert_eq!(got, want, "New({tag}).SortStrings({input:?})");
        }
    }
}

// Go: collate/sort_test.go:TestSort, ExampleCollator_strings and
// collate/examples_test.go, example_sort_test.go.
#[test]
fn test_sort_and_examples() {
    struct Sorter(Vec<String>);
    impl Lister for Sorter {
        fn len(&self) -> usize {
            self.0.len()
        }
        fn swap(&mut self, i: usize, j: usize) {
            self.0.swap(i, j)
        }
        fn bytes(&self, i: usize) -> &[u8] {
            self.0[i].as_bytes()
        }
    }
    let mut c = Collator::from_tag(&language::english(), &[]);
    let mut s = Sorter(vec!["bcd".into(), "abc".into(), "ddd".into()]);
    c.sort(&mut s);
    assert_eq!(s.0, ["abc", "bcd", "ddd"]);

    let mut v = vec!["ad", "ab", "äb", "ac"];
    Collator::from_tag(&language::und(), &[]).sort_strings(&mut v);
    assert_eq!(v, ["ab", "äb", "ac", "ad"]);

    // ExampleNew
    let mut letters = vec!["ä", "å", "ö", "o", "a"];
    let mut ec = Collator::from_tag(&language::english(), &[]);
    ec.sort_strings(&mut letters);
    assert_eq!(letters, ["a", "å", "ä", "o", "ö"]);
    Collator::new("sv").sort_strings(&mut letters);
    assert_eq!(letters, ["a", "o", "å", "ä", "ö"]);
    let mut numbers = vec!["0", "11", "01", "2", "3", "23"];
    ec.sort_strings(&mut numbers);
    assert_eq!(numbers, ["0", "01", "11", "2", "23", "3"]);
    Collator::from_tag(&language::english(), &[option::NUMERIC]).sort_strings(&mut numbers);
    assert_eq!(numbers, ["0", "01", "2", "3", "11", "23"]);

    // ExampleCollator_SortStrings / CompareString / Compare
    let mut words = vec!["meow", "woof", "bark", "moo"];
    ec.sort_strings(&mut words);
    assert_eq!(words, ["bark", "meow", "moo", "woof"]);
    assert_eq!(ec.compare_string("meow", "woof"), -1);
    assert_eq!(ec.compare_string("woof", "meow"), 1);
    assert_eq!(ec.compare_string("meow", "meow"), 0);
    assert_eq!(ec.compare(b"meow", b"woof"), -1);
    assert_eq!(ec.compare(b"woof", b"meow"), 1);
    assert_eq!(ec.compare(b"meow", b"meow"), 0);

    // ExampleCollator_Sort
    let mut bc = Sorter(vec![
        "If Cats Disappeared from the World".into(),
        "The Guest Cat".into(),
        "Catwings".into(),
    ]);
    ec.sort(&mut bc);
    assert_eq!(
        bc.0,
        [
            "Catwings",
            "If Cats Disappeared from the World",
            "The Guest Cat"
        ]
    );
}
