//! Port of github.com/bep/golibsass@v1.2.0/libsass/transpiler_test.go.

use std::sync::Arc;

use libsass_sys::{
    COMPACT_STYLE, COMPRESSED_STYLE, EXPANDED_STYLE, ImportResolver, NESTED_STYLE, Options,
    SourceMapOptions, Transpiler, new, parse_output_style,
};

const SASS_SAMPLE: &str = "nav {
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li { display: inline-block; }

  a {
    display: block;
    padding: 6px 12px;
    text-decoration: none;
  }
}";
const SASS_SAMPLE_TRANSPILED: &str = "nav ul {\n  margin: 0;\n  padding: 0;\n  list-style: none; }\n\nnav li {\n  display: inline-block; }\n\nnav a {\n  display: block;\n  padding: 6px 12px;\n  text-decoration: none; }\n";

fn resolver(
    f: impl Fn(&[u8], &[u8]) -> (Vec<u8>, Vec<u8>, bool) + Send + Sync + 'static,
) -> Option<ImportResolver> {
    Some(Arc::new(f))
}

#[test]
fn test_transpiler() {
    let import_resolver = || {
        // This will make every import the same, which is probably not a common use
        // case.
        resolver(|url, _prev| (url.to_vec(), b"$white:    #fff".to_vec(), true))
    };

    struct T {
        name: &'static str,
        opts: Options,
        src: &'static str,
        expect: Option<&'static str>, // None: should fail
    }
    let tests = vec![
        T {
            name: "Output style compressed",
            opts: Options {
                output_style: COMPRESSED_STYLE,
                ..Default::default()
            },
            src: "div { color: #ccc; }",
            expect: Some("div{color:#ccc}\n"),
        },
        T {
            name: "Invalid syntax",
            opts: Options {
                output_style: COMPRESSED_STYLE,
                ..Default::default()
            },
            src: "div { color: $white; }",
            expect: None,
        },
        T {
            name: "Import not found",
            opts: Options {
                output_style: COMPRESSED_STYLE,
                ..Default::default()
            },
            src: "@import \"foo\"",
            expect: None,
        },
        T {
            name: "Sass syntax",
            opts: Options {
                output_style: COMPRESSED_STYLE,
                sass_syntax: true,
                ..Default::default()
            },
            src: "$color: #ccc\ndiv { p { color: $color; } }",
            expect: Some("div p{color:#ccc}\n"),
        },
        T {
            name: "Import resolver",
            opts: Options {
                import_resolver: import_resolver(),
                ..Default::default()
            },
            src: "@import \"colors\";\ndiv { p { color: $white; } }",
            expect: Some("div p {\n  color: #fff; }\n"),
        },
        T {
            name: "Precision",
            opts: Options {
                precision: 3,
                ..Default::default()
            },
            src: "div { width: percentage(1 / 3); }",
            expect: Some("div {\n  width: 33.333%; }\n"),
        },
    ];
    for t in tests {
        let transpiler = new(t.opts).unwrap();
        let result = transpiler.execute(t.src.as_bytes());
        match t.expect {
            None => assert!(result.is_err(), "{}", t.name),
            Some(e) => assert_eq!(
                String::from_utf8(result.unwrap().css).unwrap(),
                e,
                "{}",
                t.name
            ),
        }
    }
}

#[test]
fn test_error() {
    let transpiler = new(Options {
        output_style: COMPRESSED_STYLE,
        ..Default::default()
    })
    .unwrap();
    let err = transpiler
        .execute(b"\n\ndiv { color: $blue; }")
        .unwrap_err();
    assert_eq!(err.line, 3);
    assert_eq!(err.column, 14);
    assert_eq!(err.message, "Undefined variable: \"$blue\".");
    assert_eq!(
        err.to_string(),
        "file \"stdin\", line 3, col 14: Undefined variable: \"$blue\". "
    );
}

#[test]
fn test_source_map_settings() {
    let src = b"div { p { color: blue; } }";
    let transpiler = new(Options {
        source_map_options: SourceMapOptions {
            enable_embedded: false,
            contents: true,
            omit_url: false,
            filename: b"source.map".to_vec(),
            output_path: b"outout.css".to_vec(),
            input_path: b"input.scss".to_vec(),
            root: b"/my/root".to_vec(),
        },
        ..Default::default()
    })
    .unwrap();
    let result = transpiler.execute(src).unwrap();
    assert_eq!(
        result.css,
        b"div p {\n  color: blue; }\n\n/*# sourceMappingURL=source.map */"
    );
    assert_eq!(result.source_map_filename, b"source.map");
    let sm = String::from_utf8(result.source_map_content).unwrap();
    assert!(sm.contains(r#""sourceRoot": "/my/root","#), "{sm}");
    assert!(sm.contains(r#""file": "outout.css","#), "{sm}");
    // (The Go test's remaining assertions are tautologies; the full source
    // map is checked against the Go oracle in tests/oracle.rs.)
    assert!(sm.contains(r#""input.scss""#), "{sm}");
}

#[test]
fn test_include_paths() {
    let base = std::env::temp_dir().join(format!(
        "libsass-sys-test-include-paths-{}",
        std::process::id()
    ));
    let dir1 = base.join("dir1");
    let dir2 = base.join("dir2");
    std::fs::create_dir_all(&dir1).unwrap();
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(
        dir1.join("_colors.scss"),
        "\n$moo:       #f442d1 !default;\n",
    )
    .unwrap();
    std::fs::write(dir2.join("_content.scss"), "\ncontent { color: #ccc; }\n").unwrap();

    let src = "
@import \"colors\";
@import \"content\";
div { p { color: $moo; } }";

    let transpiler = new(Options {
        include_paths: vec![
            dir1.to_str().unwrap().as_bytes().to_vec(),
            dir2.to_str().unwrap().as_bytes().to_vec(),
        ],
        output_style: COMPRESSED_STYLE,
        // Let LibSass resolve the import.
        import_resolver: resolver(|_url, _prev| (Vec::new(), Vec::new(), false)),
        ..Default::default()
    })
    .unwrap();
    let result = transpiler.execute(src.as_bytes());
    std::fs::remove_dir_all(&base).ok();
    assert_eq!(
        result.unwrap().css,
        b"content{color:#ccc}div p{color:#f442d1}\n"
    );
}

#[test]
fn test_concurrent_transpile() {
    let transpiler = new(Options {
        output_style: COMPRESSED_STYLE,
        import_resolver: resolver(|url, _prev| (url.to_vec(), b"$white:    #fff".to_vec(), true)),
        ..Default::default()
    })
    .unwrap();
    std::thread::scope(|s| {
        for _ in 0..10 {
            s.spawn(|| {
                for _ in 0..10 {
                    let src = "\n@import \"colors\";\n\ndiv { p { color: $white; } }";
                    let result = transpiler.execute(src.as_bytes()).unwrap();
                    assert_eq!(result.css, b"div p{color:#fff}\n");
                }
            });
        }
    });
}

#[test]
fn test_import_resolver_concurrent() {
    std::thread::scope(|s| {
        for _ in 0..10 {
            s.spawn(|| {
                for j in 0..100 {
                    let transpiler = new(Options {
                        output_style: COMPRESSED_STYLE,
                        import_resolver: resolver(move |url, _prev| {
                            (url.to_vec(), format!("$width:  {j}").into_bytes(), true)
                        }),
                        ..Default::default()
                    })
                    .unwrap();
                    let src = "\n@import \"widths\";\n\ndiv { p { width: $width; } }";
                    for _ in 0..10 {
                        let result = transpiler.execute(src.as_bytes()).unwrap();
                        assert_eq!(result.css, format!("div p{{width:{j}}}\n").into_bytes());
                    }
                }
            });
        }
    });
}

#[test]
fn test_benchmark_samples() {
    let t = new(Options::default()).unwrap();
    assert_eq!(
        t.execute(SASS_SAMPLE.as_bytes()).unwrap().css,
        SASS_SAMPLE_TRANSPILED.as_bytes()
    );
    let t = new(Options {
        output_style: COMPRESSED_STYLE,
        sass_syntax: true,
        ..Default::default()
    })
    .unwrap();
    let src = "\n$color: #333;\n\n.content-navigation\n  border-color: $color";
    assert_eq!(
        t.execute(src.as_bytes()).unwrap().css,
        b".content-navigation{border-color:#333}\n"
    );
}

#[test]
fn test_parse_output_style() {
    assert_eq!(parse_output_style(b"nested"), NESTED_STYLE);
    assert_eq!(parse_output_style(b"expanded"), EXPANDED_STYLE);
    assert_eq!(parse_output_style(b"compact"), COMPACT_STYLE);
    assert_eq!(parse_output_style(b"compressed"), COMPRESSED_STYLE);
    assert_eq!(parse_output_style(b"EXPANDED"), EXPANDED_STYLE);
    assert_eq!(parse_output_style(b"foo"), NESTED_STYLE);
}

#[test]
fn resolver_panic_is_propagated() {
    let t = new(Options {
        import_resolver: resolver(|_url, _prev| panic!("resolver boom")),
        ..Default::default()
    })
    .unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        t.execute(b"@import \"x\";")
    }));
    let payload = r.unwrap_err();
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"resolver boom"));
    // The transpiler is still usable afterwards.
    let t = new(Options::default()).unwrap();
    assert_eq!(t.execute(b"a{b:c}").unwrap().css, b"a {\n  b: c; }\n");
}

#[test]
fn version() {
    assert_eq!(libsass_sys::libsass_version(), b"[NA]");
}
