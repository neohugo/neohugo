//! Red-team regressions that need Rust-side setup (the Go behaviour they
//! pin is quoted in each test; the differential cases are in
//! tests/fixtures/sass/redteam.rec.zz, run by tests/oracle.rs).

use std::sync::{Arc, Mutex};

use libsass_sys::{COMPRESSED_STYLE, Options, Transpiler, new};

/// Go: a panic in the import resolver unwinds out of `Execute` at once
/// (through the cgo callback): the compile stops and the resolver is not
/// called for later imports, even when LibSass could load the panicking
/// import itself from an include path. The Rust port used to fall back to
/// LibSass's own resolution and keep compiling (resolver calls
/// `a b c d`); Go makes `a b`.
#[test]
fn resolver_panic_stops_the_compile_like_go() {
    let dir = std::env::temp_dir().join(format!("libsass-sys-panic-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["b", "c", "d"] {
        std::fs::write(dir.join(format!("{f}.scss")), format!(".{f} {{ x: y; }}")).unwrap();
    }
    let calls = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let calls2 = calls.clone();
    let t = new(Options {
        include_paths: vec![dir.to_str().unwrap().as_bytes().to_vec()],
        import_resolver: Some(Arc::new(move |url: &[u8], _prev: &[u8]| {
            calls2.lock().unwrap().push(url.to_vec());
            if url == b"b" {
                panic!("resolver boom");
            }
            (url.to_vec(), b".x { y: z; }".to_vec(), true)
        })),
        ..Default::default()
    })
    .unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        t.execute(b"@import \"a\";\n@import \"b\";\n@import \"c\";\n@import \"d\";")
    }));
    std::fs::remove_dir_all(&dir).ok();
    let payload = r.unwrap_err();
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"resolver boom"));
    assert_eq!(*calls.lock().unwrap(), [b"a".to_vec(), b"b".to_vec()]);
    // The transpiler (and the resolver registry) still work afterwards.
    let t = new(Options {
        output_style: COMPRESSED_STYLE,
        import_resolver: Some(Arc::new(|url: &[u8], _prev: &[u8]| {
            (url.to_vec(), b"$w: 1px;".to_vec(), true)
        })),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        t.execute(b"@import \"v\";\na { b: $w; }").unwrap().css,
        b"a{b:1px}\n"
    );
}

/// A panic in a resolver of a transpile nested in another resolver
/// propagates out of both, like Go's panic through both cgo callbacks.
#[test]
fn nested_resolver_panic_propagates() {
    let t = new(Options {
        import_resolver: Some(Arc::new(|url: &[u8], _prev: &[u8]| {
            let inner = new(Options {
                import_resolver: Some(Arc::new(|_u: &[u8], _p: &[u8]| panic!("inner boom"))),
                ..Default::default()
            })
            .unwrap();
            let _ = inner.execute(b"@import \"x\";");
            (url.to_vec(), b"a { b: c; }".to_vec(), true)
        })),
        ..Default::default()
    })
    .unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        t.execute(b"@import \"o\";")
    }));
    assert_eq!(r.unwrap_err().downcast_ref::<&str>(), Some(&"inner boom"));
}

/// Go runs LibSass on the 8 MiB cgo stack whatever goroutine calls it. A
/// Sass function recursing to LibSass's limit (1024 calls) needs ~4.4 MiB
/// and an import chain of 1900 files (Go's limit) ~8 MiB; both used to
/// overflow (abort the process) when `execute` ran on a Rust thread with
/// the default 2 MiB stack. Go: `.a{b:1024}` and 1900 imports compiled.
#[test]
fn deep_recursion_from_a_small_stack_thread() {
    let h = std::thread::Builder::new()
        .stack_size(256 << 10)
        .spawn(|| {
            let t = new(Options {
                output_style: COMPRESSED_STYLE,
                ..Default::default()
            })
            .unwrap();
            let r = t
                .execute(
                    b"@function f($n) { @if $n <= 0 { @return 0; } @return f($n - 1) + 1; }\n.a { b: f(1024); }",
                )
                .unwrap();
            assert_eq!(r.css, b".a{b:1024}\n");
            let e = t
                .execute(
                    b"@function f($n) { @if $n <= 0 { @return 0; } @return f($n - 1) + 1; }\n.a { b: f(1025); }",
                )
                .unwrap_err();
            assert_eq!(
                e.to_string(),
                "file \"stdin\", line 1, col 54: Stack depth exceeded max of 1024 "
            );

            let t = new(Options {
                output_style: COMPRESSED_STYLE,
                import_resolver: Some(Arc::new(|url: &[u8], _prev: &[u8]| {
                    let n: usize = std::str::from_utf8(&url[1..]).unwrap().parse().unwrap();
                    let body = if n < 1900 {
                        format!("@import \"i{}\";\n.n{n} {{ a: b; }}", n + 1)
                    } else {
                        ".leaf { a: b; }".to_string()
                    };
                    (format!("v/i{n}.scss").into_bytes(), body.into_bytes(), true)
                })),
                ..Default::default()
            })
            .unwrap();
            let r = t.execute(b"@import \"i1\";").unwrap();
            assert_eq!(r.css.len(), 19793);
            assert!(r.css.starts_with(b".leaf{a:b}.n1899{a:b}"));
        })
        .unwrap();
    h.join().unwrap();
}
