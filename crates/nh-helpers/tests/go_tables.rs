//! Go's own tests of the ported packages: helpers (url_test.go, path_test.go, general_test.go,
//! content_test.go), cache/httpcache (httpcache_test.go), cache/filecache
//! (filecache_config_test.go, filecache_test.go).

mod support;

use std::sync::Arc;

use go_value::{Map, MapType, Value};
use nh_config::common_config::BaseConfig;
use nh_config::config_provider::AllProvider;
use nh_helpers::cache::filecache::filecache::Cache;
use nh_helpers::cache::filecache::filecache_config::decode_config;
use nh_helpers::cache::httpcache::httpcache::{
    GlobMatcher, decode_config as http_decode, default_config,
};
use nh_helpers::content::{ContentSpec, extract_toc, total_words};
use nh_helpers::general::*;
use nh_helpers::path::*;
use nh_helpers::pathspec::PathSpec;
use support::*;

struct Opts<'a> {
    base_url: &'a str,
    lang: &'a str,
    default_lang: &'a str,
    in_subdir: bool,
    multilingual: bool,
    canonify: bool,
    disable_path_to_lower: bool,
}

impl Default for Opts<'_> {
    fn default() -> Self {
        Opts {
            base_url: "",
            lang: "en",
            default_lang: "en",
            in_subdir: false,
            multilingual: false,
            canonify: false,
            disable_path_to_lower: false,
        }
    }
}

fn path_spec(o: &Opts<'_>, tmp: &TempDir) -> Arc<PathSpec> {
    let mut cfg = TestCfg::new(&tmp.str());
    cfg.base_url = nh_common::urls::new_base_url_from_string(o.base_url).unwrap();
    cfg.lang = o.lang.to_string();
    let mut langs = vec![
        nh_langs::language::Language::new(o.lang, o.default_lang, "", Default::default()).unwrap(),
    ];
    if o.multilingual {
        let other = if o.lang == "en" { "fr" } else { "en" };
        langs.push(
            nh_langs::language::Language::new(other, o.default_lang, "", Default::default())
                .unwrap(),
        );
    }
    cfg.languages = langs;
    // Go: allconfig.ConfigLanguage.LanguagePrefix.
    cfg.language_prefix = if o.in_subdir && o.default_lang == o.lang {
        o.lang.to_string()
    } else if !o.multilingual || o.default_lang == o.lang {
        String::new()
    } else {
        o.lang.to_string()
    };
    cfg.canonify_urls = o.canonify;
    cfg.disable_path_to_lower = o.disable_path_to_lower;
    let fs = nh_hugofs::fs::new_from(nh_hugofs::afero::new_mem_map_fs(), &cfg.base_config());
    PathSpec::new(fs, Arc::new(cfg)).unwrap()
}

#[test]
fn test_urlize() {
    let tmp = TempDir::new("gt");
    let p = path_spec(&Opts::default(), &tmp);
    for (input, expected) in [
        ("  foo bar  ", "foo-bar"),
        ("foo.bar/foo_bar-foo", "foo.bar/foo_bar-foo"),
        ("foo,bar:foobar", "foobarfoobar"),
        ("foo/bar.html", "foo/bar.html"),
        (
            "трям/трям",
            "%D1%82%D1%80%D1%8F%D0%BC/%D1%82%D1%80%D1%8F%D0%BC",
        ),
        ("100%-google", "100-google"),
    ] {
        assert_eq!(p.urlize(input), expected);
    }
}

#[test]
fn test_abs_url() {
    let tmp = TempDir::new("gt");
    for default_in_sub_dir in [true, false] {
        for add_language in [true, false] {
            for multilingual in [true, false] {
                for lang in ["en", "fr"] {
                    let mut tests: Vec<(String, &str, String)> = vec![
                        (
                            "foo/bar".into(),
                            "https://example.org/foo/",
                            "https://example.org/foo/MULTIfoo/bar".into(),
                        ),
                        (
                            "/foo/bar".into(),
                            "https://example.org/foo/",
                            "https://example.org/MULTIfoo/bar".into(),
                        ),
                        (
                            "/test/foo".into(),
                            "http://base/",
                            "http://base/MULTItest/foo".into(),
                        ),
                        (
                            format!("/{lang}/test/foo"),
                            "http://base/",
                            format!("http://base/{lang}/test/foo"),
                        ),
                        (
                            "".into(),
                            "http://base/ace/",
                            "http://base/ace/MULTI".into(),
                        ),
                        (
                            "/test/2/foo/".into(),
                            "http://base",
                            "http://base/MULTItest/2/foo/".into(),
                        ),
                        ("http://abs".into(), "http://base/", "http://abs".into()),
                        ("schema://abs".into(), "http://base/", "schema://abs".into()),
                        ("//schemaless".into(), "http://base/", "//schemaless".into()),
                        (
                            "test/2/foo/".into(),
                            "http://base/path",
                            "http://base/path/MULTItest/2/foo/".into(),
                        ),
                        (
                            format!("{lang}/test/2/foo/"),
                            "http://base/path",
                            format!("http://base/path/{lang}/test/2/foo/"),
                        ),
                        (
                            "/test/2/foo/".into(),
                            "http://base/path",
                            "http://base/MULTItest/2/foo/".into(),
                        ),
                        (
                            "http//foo".into(),
                            "http://base/path",
                            "http://base/path/MULTIhttp/foo".into(),
                        ),
                    ];
                    if multilingual && add_language && default_in_sub_dir {
                        tests.push((
                            format!("{lang}test"),
                            "http://base/",
                            format!("http://base/{lang}/{lang}test"),
                        ));
                        tests.push((
                            format!("/{lang}test"),
                            "http://base/",
                            format!("http://base/{lang}/{lang}test"),
                        ));
                    }
                    let default_lang = if multilingual { "en" } else { lang };
                    for (input, base, expected) in tests {
                        let p = path_spec(
                            &Opts {
                                base_url: base,
                                lang,
                                default_lang,
                                in_subdir: default_in_sub_dir,
                                multilingual,
                                ..Default::default()
                            },
                            &tmp,
                        );
                        let output = p.abs_url(&input, add_language);
                        let mut add = add_language;
                        if add {
                            add = default_in_sub_dir && lang == default_lang;
                            add = add || (lang != default_lang && multilingual);
                        }
                        let expected = if add {
                            expected.replacen("MULTI", &format!("{lang}/"), 1)
                        } else {
                            expected.replacen("MULTI", "", 1)
                        };
                        assert_eq!(
                            output, expected,
                            "{input} {base} {default_in_sub_dir} {add_language} {multilingual} {lang}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_rel_url() {
    let tmp = TempDir::new("gt");
    for default_in_sub_dir in [true, false] {
        for add_language in [true, false] {
            for multilingual in [true, false] {
                for lang in ["en", "fr"] {
                    let mut tests: Vec<(String, &str, bool, String)> = vec![
                        (
                            "/foo/bar".into(),
                            "https://example.org/foo/",
                            false,
                            "MULTI/foo/bar".into(),
                        ),
                        (
                            "foo/bar".into(),
                            "https://example.org/foo/",
                            false,
                            "/fooMULTI/foo/bar".into(),
                        ),
                        (
                            "mailto:a@b.com".into(),
                            "http://base/",
                            false,
                            "mailto:a@b.com".into(),
                        ),
                        (
                            "ftp://b.com/a.txt".into(),
                            "http://base/",
                            false,
                            "ftp://b.com/a.txt".into(),
                        ),
                        (
                            "/test/foo".into(),
                            "http://base/",
                            false,
                            "MULTI/test/foo".into(),
                        ),
                        (
                            format!("/{lang}/test/foo"),
                            "http://base/",
                            false,
                            format!("/{lang}/test/foo"),
                        ),
                        (
                            format!("{lang}/test/foo"),
                            "http://base/",
                            false,
                            format!("/{lang}/test/foo"),
                        ),
                        (
                            "test.css".into(),
                            "http://base/sub",
                            false,
                            "/subMULTI/test.css".into(),
                        ),
                        (
                            "test.css".into(),
                            "http://base/sub",
                            true,
                            "MULTI/test.css".into(),
                        ),
                        ("/test/".into(), "http://base/", false, "MULTI/test/".into()),
                        (
                            "test/".into(),
                            "http://base/sub/",
                            false,
                            "/subMULTI/test/".into(),
                        ),
                        (
                            "/test/".into(),
                            "http://base/sub/",
                            true,
                            "MULTI/test/".into(),
                        ),
                        ("".into(), "http://base/ace/", false, "/aceMULTI/".into()),
                        ("".into(), "http://base/ace", false, "/aceMULTI/".into()),
                        (
                            "http://abs".into(),
                            "http://base/",
                            false,
                            "http://abs".into(),
                        ),
                        (
                            "//schemaless".into(),
                            "http://base/",
                            false,
                            "//schemaless".into(),
                        ),
                    ];
                    if multilingual && add_language && default_in_sub_dir {
                        tests.push((
                            format!("{lang}test"),
                            "http://base/",
                            false,
                            format!("/{lang}/{lang}test"),
                        ));
                        tests.push((
                            format!("/{lang}test"),
                            "http://base/",
                            false,
                            format!("/{lang}/{lang}test"),
                        ));
                    }
                    let default_lang = if multilingual { "en" } else { lang };
                    for (input, base, canonify, expected) in tests {
                        let p = path_spec(
                            &Opts {
                                base_url: base,
                                lang,
                                default_lang,
                                in_subdir: default_in_sub_dir,
                                multilingual,
                                canonify,
                                ..Default::default()
                            },
                            &tmp,
                        );
                        let output = p.rel_url(&input, add_language);
                        let mut add = add_language;
                        if add {
                            add = default_in_sub_dir && lang == default_lang;
                            add = add || (lang != default_lang && multilingual);
                        }
                        let expected = if add {
                            expected.replacen("MULTI", &format!("/{lang}"), 1)
                        } else {
                            expected.replacen("MULTI", "", 1)
                        };
                        assert_eq!(output, expected, "{input} {base}");
                    }
                }
            }
        }
    }
}

#[test]
fn test_make_path() {
    let tmp = TempDir::new("gt");
    let p = path_spec(&Opts::default(), &tmp);
    // The removeAccents rows need RemoveAccentsString (an nh-common stub); these are the rows
    // with removePathAccents off, and the accent-free rows.
    for (input, expected) in [
        (
            "dot.slash/backslash\\underscore_pound#plus+hyphen-",
            "dot.slash/backslash\\underscore_pound#plus+hyphen-",
        ),
        ("abcXYZ0123456789", "abcXYZ0123456789"),
        ("%20 %2", "%20-2"),
        ("foo- bar", "foo-bar"),
        ("  Foo bar  ", "Foo-bar"),
        ("Foo.Bar/foo_Bar-Foo", "Foo.Bar/foo_Bar-Foo"),
        ("fOO,bar:foobAR", "fOObarfoobAR"),
        ("FOo/BaR.html", "FOo/BaR.html"),
        ("трям/трям", "трям/трям"),
        ("은행", "은행"),
        ("संस्कृत", "संस्कृत"),
        ("a%C3%B1ame", "a%C3%B1ame"),
        ("this+is+a+test", "this+is+a+test"),
        ("~foo", "~foo"),
        ("foo--bar", "foo--bar"),
        ("foo@bar", "foo@bar"),
    ] {
        assert_eq!(p.make_path(input), expected);
    }
}

#[test]
fn test_make_path_sanitized() {
    let tmp = TempDir::new("gt");
    let p = path_spec(&Opts::default(), &tmp);
    for (input, expected) in [
        ("  FOO bar  ", "foo-bar"),
        ("Foo.Bar/fOO_bAr-Foo", "foo.bar/foo_bar-foo"),
        ("FOO,bar:FooBar", "foobarfoobar"),
        ("foo/BAR.HTML", "foo/bar.html"),
        ("трям/трям", "трям/трям"),
        ("은행", "은행"),
    ] {
        assert_eq!(p.make_path_sanitized(input), expected);
    }
    let p = path_spec(
        &Opts {
            disable_path_to_lower: true,
            ..Default::default()
        },
        &tmp,
    );
    for (input, expected) in [
        ("  FOO bar  ", "FOO-bar"),
        ("Foo.Bar/fOO_bAr-Foo", "Foo.Bar/fOO_bAr-Foo"),
        ("FOO,bar:FooBar", "FOObarFooBar"),
        ("foo/BAR.HTML", "foo/BAR.HTML"),
        ("трям/трям", "трям/трям"),
        ("은행", "은행"),
    ] {
        assert_eq!(p.make_path_sanitized(input), expected);
    }
}

#[test]
fn test_make_path_relative() {
    for (in_path, p1, p2, output) in [
        ("/abc/bcd/ab.css", "/abc/bcd", "/bbc/bcd", "/ab.css"),
        ("/abc/bcd/ab.css", "/abcd/bcd", "/abc/bcd", "/ab.css"),
    ] {
        assert_eq!(make_path_relative(in_path, &[p1, p2]).0, output);
    }
    assert!(
        make_path_relative("a/b/c.ss", &["/a/c", "/d/c", "/e/f"])
            .1
            .is_some()
    );
}

#[test]
fn test_get_dotted_relative_path() {
    for (input, expected) in [
        ("", "./"),
        ("/", "./"),
        ("post", "../"),
        ("/post", "../"),
        ("post/", "../"),
        ("tags/foo.html", "../"),
        ("/tags/foo.html", "../"),
        ("/post/", "../"),
        ("////post/////", "../"),
        ("/foo/bar/index.html", "../../"),
        ("/foo/bar/foo/", "../../../"),
        ("/foo/bar/foo", "../../../"),
        ("foo/bar/foo/", "../../../"),
        ("foo/bar/foo/bar", "../../../../"),
        ("404.html", "./"),
        ("404.xml", "./"),
        ("/404.html", "./"),
    ] {
        assert_eq!(get_dotted_relative_path(input), expected, "{input}");
    }
}

#[test]
fn test_make_title() {
    for (input, expected) in [
        ("Make-Title", "Make Title"),
        ("MakeTitle", "MakeTitle"),
        ("make_title", "make_title"),
    ] {
        assert_eq!(make_title(input), expected);
    }
}

#[test]
fn test_extract_root_paths() {
    let input: Vec<String> = [
        "/a/b/c/d",
        "/a/b/c/e",
        "/a/b/e/f",
        "/a/b",
        "/a/b/c/b/g",
        "/c/d/e",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let result = extract_and_group_root_paths(&input).unwrap();
    let s: Vec<String> = result.iter().map(|n| n.string()).collect();
    assert_eq!(format!("[{}]", s.join(" ")), "[/a/b/{c,e} /c/d/e]");

    let input: Vec<String> = ["a/b", "a/b/c/", "b", "/c/d", "d/", "//e//"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        extract_root_paths(&input),
        vec!["a", "a", "b", "c", "d", "e"]
    );
}

#[test]
fn test_write_to_disk_and_dirs() {
    let fs = nh_hugofs::afero::new_mem_map_fs();
    let mut r: &[u8] = b"content";
    write_to_disk("/a/b/c.txt", &mut r, fs.as_ref()).unwrap();
    assert!(exists("/a/b/c.txt", fs.as_ref()).unwrap());
    assert!(dir_exists("/a/b", fs.as_ref()).unwrap());
    assert!(!dir_exists("/a/b/c.txt", fs.as_ref()).unwrap());
    assert!(!dir_exists("/nope", fs.as_ref()).unwrap());
    assert!(is_dir("/a", fs.as_ref()).unwrap());
    let mut r: &[u8] = b"again";
    let err = safe_write_to_disk("/a/b/c.txt", &mut r, fs.as_ref()).unwrap_err();
    assert_eq!(err.message(), "/a/b/c.txt already exists");
    let mut w =
        open_files_for_writing(fs.as_ref(), &["/x/1".to_string(), "/y/2".to_string()]).unwrap();
    std::io::Write::write_all(&mut w, b"multi").unwrap();
    w.close().unwrap();
    assert!(exists("/x/1", fs.as_ref()).unwrap() && exists("/y/2", fs.as_ref()).unwrap());
}

#[test]
fn test_content_spec() {
    let tmp = TempDir::new("gt");
    let cs = ContentSpec::new(
        Arc::new(TestCfg::new(&tmp.str())),
        nh_config::hexec::Exec::new_with_env(Default::default(), &tmp.str(), &[], None),
    )
    .unwrap();
    for (markup, input, output) in [
        ("markdown", "", ""),
        ("markdown", "Plain text", "Plain text"),
        ("markdown", "<p>Simple paragraph</p>", "Simple paragraph"),
        (
            "markdown",
            "\n  \n \t  <p> \t Whitespace\nHTML  \n\t </p>\n\t",
            "Whitespace\nHTML",
        ),
        (
            "markdown",
            "<p>Multiple</p><p>paragraphs</p>",
            "<p>Multiple</p><p>paragraphs</p>",
        ),
        (
            "markdown",
            "<p>Nested<p>paragraphs</p></p>",
            "<p>Nested<p>paragraphs</p></p>",
        ),
        (
            "markdown",
            "<p>Hello</p>\n<ul>\n<li>list1</li>\n<li>list2</li>\n</ul>",
            "<p>Hello</p>\n<ul>\n<li>list1</li>\n<li>list2</li>\n</ul>",
        ),
        (
            "markdown",
            "<h2 id=`a`>b</h2>\n\n<p>c</p>",
            "<h2 id=`a`>b</h2>\n\n<p>c</p>",
        ),
        (
            "markdown",
            "<div class=\"paragraph\">\n<p>foo</p>\n</div>",
            "<div class=\"paragraph\">\n<p>foo</p>\n</div>",
        ),
        (
            "asciidoc",
            "<div class=\"paragraph\">\n<p>foo</p>\n</div>",
            "foo",
        ),
    ] {
        assert_eq!(
            cs.trim_short_html(input.as_bytes(), markup),
            output.as_bytes()
        );
    }
    for (input, expect) in [
        ("md", "markdown"),
        ("markdown", "markdown"),
        ("mdown", "markdown"),
        ("asciidocext", "asciidoc"),
        ("adoc", "asciidoc"),
        ("ad", "asciidoc"),
        ("rst", "rst"),
        ("pandoc", "pandoc"),
        ("pdc", "pandoc"),
        ("html", "html"),
        ("htm", "html"),
        ("org", "org"),
        ("excel", ""),
    ] {
        assert_eq!(cs.resolve_markup(input), expect, "{input}");
    }
    assert_eq!(
        nh_helpers::content::bytes_to_html(b"dobedobedo"),
        Value::html(b"dobedobedo".to_vec())
    );
}

#[test]
fn test_extract_toc_and_total_words() {
    let (tocless, toc) = extract_toc(b"<nav>\n<ul>\nTOC<li><a href=\"#");
    assert_eq!(tocless, b"TOC<li><a href=\"#");
    assert_eq!(toc.unwrap(), b"<nav id=\"TableOfContents\">\n<ul>\n");
    let long = b"<nav>\n<ul>\nTOC This is a very long content which will definitely be greater than seventy, I promise you that.<li><a href=\"#";
    let (tocless, toc) = extract_toc(long);
    assert_eq!(tocless, long);
    assert!(toc.is_none());
    let (tocless, toc) = extract_toc(b"TOC");
    assert_eq!(tocless, b"TOC");
    assert!(toc.is_none());

    let bench = "Hugo Rocks ".repeat(200);
    for (s, n) in [
        ("Two, Words!", 2),
        ("Word", 1),
        ("", 0),
        ("One, Two,      Three", 3),
        (bench.as_str(), 400),
    ] {
        assert_eq!(total_words(s.as_bytes()), n);
    }
}

#[test]
fn test_general() {
    for (i, e) in [
        ("foo", "Foo"),
        ("foo bar", "Foo bar"),
        ("Foo Bar", "Foo Bar"),
        ("", ""),
        ("å", "Å"),
    ] {
        assert_eq!(first_upper(i), e);
    }
    let title = "somewhere over the Rainbow";
    assert_eq!(get_title_func("go")(title), "Somewhere Over The Rainbow");
    assert_eq!(
        get_title_func("chicago")(title),
        "Somewhere over the Rainbow"
    );
    assert_eq!(
        get_title_func("Chicago")(title),
        "Somewhere over the Rainbow"
    );
    assert_eq!(get_title_func("ap")(title), "Somewhere Over the Rainbow");
    assert_eq!(get_title_func("")(title), "Somewhere Over the Rainbow");
    assert_eq!(
        get_title_func("unknown")(title),
        "Somewhere Over the Rainbow"
    );
    assert_eq!(get_title_func("none")(title), title);
    assert_eq!(
        get_title_func("firstupper")(title),
        "Somewhere over the Rainbow"
    );

    let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let input = v(&["a", "b", "a", "b", "c", "", "a", "", "d"]);
    assert_eq!(unique_strings(&input), v(&["a", "b", "c", "", "d"]));
    assert_eq!(unique_strings_reuse(input), v(&["a", "b", "c", "", "d"]));
    assert_eq!(
        unique_strings_sorted(v(&["a", "a", "b", "c", "b", "", "a", "", "d"])).unwrap(),
        v(&["", "a", "b", "c", "d"])
    );
    assert!(unique_strings_sorted(Vec::new()).is_none());

    for (slice, conj, want) in [
        (v(&[]), "", ""),
        (v(&["foo"]), "", "foo"),
        (v(&["foo"]), "and", "foo"),
        (v(&["foo", "bar"]), "", "foo and bar"),
        (v(&["foo", "bar"]), "and", "foo and bar"),
        (v(&["foo", "bar"]), "or", "foo or bar"),
        (v(&["foo", "bar", "baz"]), "", "foo, bar, and baz"),
        (v(&["foo", "bar", "baz"]), "and", "foo, bar, and baz"),
        (v(&["foo", "bar", "baz"]), "or", "foo, bar, or baz"),
    ] {
        assert_eq!(string_slice_to_list(&slice, conj), want);
    }
    assert!(!reader_contains(None, b"a"));
    assert!(!reader_contains(None, b""));
    let mut r: &[u8] = b"abcdefgh";
    assert!(reader_contains(Some(&mut r), b"def"));
}

#[test]
fn test_glob_matcher() {
    let g = GlobMatcher {
        includes: vec!["**/*.jpg".into(), "**.png".into(), "**/bar/**".into()],
        excludes: vec!["**/foo.jpg".into(), "**.css".into()],
    };
    let p = g.compile_predicate().unwrap();
    let t = |s: &str| p(&s.to_string());
    assert!(!t("foo.jpg"));
    assert!(t("foo.png"));
    assert!(t("foo/bar.jpg"));
    assert!(t("foo/bar.png"));
    assert!(!t("foo/bar/foo.jpg"));
    assert!(!t("foo/bar/foo.css"));
    assert!(!t("foo.css"));
    assert!(t("foo/bar/foo.xml"));

    default_config().compile().unwrap();
    let cfg = http_decode(&Map::new(MapType::StringAny)).unwrap();
    assert_eq!(cfg, default_config());
    cfg.compile().unwrap();
}

#[test]
fn test_filecache_decode_config_default() {
    let bcfg = BaseConfig {
        working_dir: "/my/cool/hugoproject".to_string(),
        cache_dir: "/cache/thecache".to_string(),
        ..Default::default()
    };
    let fs = nh_hugofs::afero::new_mem_map_fs();
    let decoded = decode_config(fs.as_ref(), &bcfg, &Map::new(MapType::StringAny)).unwrap();
    assert_eq!(decoded.len(), 7);
    assert_eq!(decoded["images"].dir_compiled, "_gen/images");
    assert_eq!(
        decoded["getjson"].dir_compiled,
        "/cache/thecache/hugoproject/filecache/getjson"
    );
    assert_eq!(decoded["assets"].dir_compiled, "_gen/assets");
    assert!(decoded["images"].is_resource_dir);
}

#[test]
fn test_file_cache_read_or_create_error_in_read() {
    let fs = nh_hugofs::afero::new_mem_map_fs();
    let cache = Cache::new(fs, go_time::Duration(-1), "");
    let result = std::cell::RefCell::new(String::new());
    let mut read_err: Option<nh_common::herrors::Error> =
        Some(nh_common::herrors::Error::new("error in read"));
    for _ in 0..2 {
        let res = cache.read_or_create(
            "a",
            &mut |_info, r| {
                let mut b = String::new();
                std::io::Read::read_to_string(r, &mut b).unwrap();
                *result.borrow_mut() = b;
                match read_err.take() {
                    Some(e) => Err(e),
                    None => Ok(()),
                }
            },
            &mut |_info, mut w| {
                let r = std::io::Write::write_all(&mut w, b"v1");
                let _ = w.close();
                *result.borrow_mut() = "v1".to_string();
                r.map_err(|e| nh_common::herrors::Error::new(e.to_string()))
            },
        );
        res.unwrap();
    }
    assert_eq!(*result.borrow(), "v1");
}
