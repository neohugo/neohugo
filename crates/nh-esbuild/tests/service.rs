//! The esbuild service client (`service::protocol`, `service::client`).
//!
//! The codec tests need nothing. The service tests run the pinned esbuild binary
//! (`tools/esbuild/build.sh` builds it) named by `NEOHUGO_ESBUILD_BINARY`; without it they print
//! `SKIPPED` to stderr (not captured by the test harness) and pass. The byte parity of whole
//! `js.Build` results against Go's in-process esbuild is tested in
//! crates/nh-resource-transformers/tests/jsbuild.rs.

use std::io::Write;
use std::sync::Arc;

use nh_esbuild::options::CompiledBuildOptions;
use nh_esbuild::service::client::{
    OnLoadArgs, OnLoadFn, OnLoadResult, OnResolveArgs, OnResolveFn, OnResolveResult, Plugin,
    ServiceClient, build_flags, find_binary_with,
};
use nh_esbuild::service::protocol::{
    Packet, PacketValue, decode_packet, encode_packet, read_length_prefixed_slice,
};

fn esbuild() -> Option<String> {
    match std::env::var("NEOHUGO_ESBUILD_BINARY") {
        Ok(b) if !b.is_empty() => Some(b),
        _ => {
            let _ = writeln!(
                std::io::stderr(),
                "SKIPPED: NEOHUGO_ESBUILD_BINARY is not set (tools/esbuild/build.sh builds the pinned esbuild 0.25.6)"
            );
            None
        }
    }
}

fn tmpdir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nh-esbuild-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d.canonicalize().unwrap()
}

#[test]
fn codec_matches_go_encoding() {
    // encodePacket(packet{id: 1, isRequest: true, value: {"b": [true, nil], "a": 5}}) computed
    // from stdio_protocol.go by hand: sorted keys, little-endian lengths, the id shifted left.
    let p = Packet {
        id: 1,
        is_request: true,
        value: PacketValue::map([
            (
                "b",
                PacketValue::Array(vec![PacketValue::Bool(true), PacketValue::Null]),
            ),
            ("a", PacketValue::Int(5)),
        ]),
    };
    let want: Vec<u8> = vec![
        32, 0, 0, 0, // length
        2, 0, 0, 0, // id << 1
        6, 2, 0, 0, 0, // map, 2 keys
        1, 0, 0, 0, b'a', 2, 5, 0, 0, 0, // "a": 5
        1, 0, 0, 0, b'b', 5, 2, 0, 0, 0, 1, 1, 0, // "b": [true, nil]
    ];
    assert_eq!(encode_packet(&p), want);

    // A response has bit 0 set; strings and bytes are length-prefixed.
    let r = Packet {
        id: 3,
        is_request: false,
        value: PacketValue::Array(vec![
            PacketValue::str("hé"),
            PacketValue::Bytes(vec![0, 1]),
            PacketValue::Int(-1),
        ]),
    };
    let b = encode_packet(&r);
    assert_eq!(&b[4..8], &[7, 0, 0, 0]);
    let (body, rest) = read_length_prefixed_slice(&b).unwrap();
    assert!(rest.is_empty());
    let (q, n) = decode_packet(body).unwrap();
    assert_eq!(n, body.len());
    assert_eq!(q, r);
    assert_eq!(&b[b.len() - 5..], &[2, 0xff, 0xff, 0xff, 0xff]);

    // Go: trailing bytes, truncations and unknown tags are invalid packets.
    let mut long = body.to_vec();
    long.push(0);
    assert!(decode_packet(&long).is_none());
    for i in 0..body.len() {
        assert!(decode_packet(&body[..i]).is_none(), "truncated at {i}");
    }
    assert!(decode_packet(&[0, 0, 0, 0, 9]).is_none());
    // A repeated map key keeps the last value (Go map assignment).
    let dup = [
        0, 0, 0, 0, 6, 2, 0, 0, 0, 1, 0, 0, 0, b'k', 2, 1, 0, 0, 0, 1, 0, 0, 0, b'k', 2, 2, 0, 0, 0,
    ];
    let (d, _) = decode_packet(&dup).unwrap();
    assert_eq!(d.value, PacketValue::map([("k", PacketValue::Int(2))]));
}

#[test]
fn flags_for_hugo_options() {
    let mut o = CompiledBuildOptions {
        bundle: true,
        format: "iife".into(),
        platform: "browser".into(),
        target: "es2015".into(),
        sourcemap: "none".into(),
        sources_content: true,
        outdir: "/p/public/".into(),
        stdin_contents: Some(b"x".to_vec()),
        stdin_loader: "ts".into(),
        jsx: "transform".into(),
        ..Default::default()
    };
    assert_eq!(
        build_flags(&o).unwrap(),
        [
            "--log-level=silent",
            "--log-limit=0",
            "--target=es2015",
            "--format=iife",
            "--platform=browser",
            "--jsx=transform",
            "--bundle",
            "--outdir=/p/public/",
            "--loader=ts"
        ]
    );
    o.minify_whitespace = true;
    o.minify_syntax = true;
    o.minify_identifiers = true;
    o.sources_content = false;
    o.sourcemap = "linked".into();
    o.define.insert("process.env.X".into(), "\"y\"".into());
    o.external.push("react".into());
    o.loader.insert(".png".into(), "dataurl".into());
    let f = build_flags(&o).unwrap();
    for want in [
        "--sources-content=false",
        "--minify-syntax",
        "--minify-whitespace",
        "--minify-identifiers",
        "--define:process.env.X=\"y\"",
        "--sourcemap=linked",
        "--external:react",
        "--loader:.png=dataurl",
    ] {
        assert!(f.iter().any(|x| x == want), "{want} in {f:?}");
    }
    o.define.insert("a=b".into(), "1".into());
    assert!(build_flags(&o).is_err());
}

#[test]
fn binary_lookup_order() {
    let d = tmpdir("lookup");
    let wd = d.to_string_lossy().into_owned();
    // Env first.
    assert_eq!(
        find_binary_with(Some("/x/esbuild"), &wd, None).unwrap(),
        "/x/esbuild"
    );
    // Nothing found.
    let e = find_binary_with(None, &wd, Some("/nonexistent")).unwrap_err();
    assert!(e.is_feature_not_available(), "{e}");
    // PATH.
    let bin = d.join("pathbin");
    std::fs::create_dir_all(&bin).unwrap();
    let exe = bin.join("esbuild");
    std::fs::write(&exe, "#!/bin/sh\n").unwrap();
    make_executable(&exe);
    let p = bin.to_string_lossy().into_owned();
    assert_eq!(
        find_binary_with(None, &wd, Some(&p)).unwrap(),
        exe.to_string_lossy()
    );
    // node_modules/.bin before PATH, @esbuild/<platform> before that.
    let dotbin = d.join("node_modules/.bin/esbuild");
    std::fs::create_dir_all(dotbin.parent().unwrap()).unwrap();
    std::fs::write(&dotbin, "").unwrap();
    assert_eq!(
        find_binary_with(None, &wd, Some(&p)).unwrap(),
        dotbin.to_string_lossy()
    );
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        o => o,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        a => a,
    };
    let native = d.join(format!("node_modules/@esbuild/{os}-{arch}/bin/esbuild"));
    std::fs::create_dir_all(native.parent().unwrap()).unwrap();
    std::fs::write(&native, "").unwrap();
    assert_eq!(
        find_binary_with(None, &wd, Some(&p)).unwrap(),
        native.to_string_lossy()
    );
    let _ = std::fs::remove_dir_all(&d);
}

fn make_executable(p: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(p).unwrap().permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(p, perm).unwrap();
}

#[test]
fn version_mismatch_is_an_error() {
    let d = tmpdir("version");
    let fake = d.join("esbuild");
    // Writes the protocol's version header "0.0.1" and exits.
    std::fs::write(&fake, "#!/bin/sh\nprintf '\\005\\000\\000\\0000.0.1'\n").unwrap();
    make_executable(&fake);
    let err = match ServiceClient::start(&fake.to_string_lossy()) {
        Ok(_) => panic!("expected an error"),
        Err(e) => e,
    };
    assert_eq!(
        err.to_string(),
        "Cannot start service: Host version \"0.25.6\" does not match binary version \"0.0.1\""
    );
    let _ = std::fs::remove_dir_all(&d);
}

fn stdin_opts(dir: &str, contents: &str, loader: &str) -> CompiledBuildOptions {
    CompiledBuildOptions {
        bundle: true,
        format: "iife".into(),
        platform: "browser".into(),
        target: "es2015".into(),
        sourcemap: "none".into(),
        sources_content: true,
        outdir: format!("{dir}/public/"),
        abs_working_dir: dir.to_string(),
        stdin_contents: Some(contents.as_bytes().to_vec()),
        stdin_resolve_dir: dir.to_string(),
        stdin_loader: loader.to_string(),
        jsx: "transform".into(),
        ..Default::default()
    }
}

#[test]
fn service_build_equals_cli() {
    let Some(bin) = esbuild() else { return };
    let d = tmpdir("cli");
    let wd = d.to_string_lossy().into_owned();
    std::fs::write(d.join("dep.ts"), "export const x: number = 42;\n").unwrap();
    let src = "import { x } from './dep';\nconst f = (a: number) => a ** 2;\nconsole.log(f(x), `t${x}`);\n";
    let svc = ServiceClient::start(&bin).unwrap();
    let r = svc
        .build_with_plugins(&stdin_opts(&wd, src, "ts"), &[])
        .unwrap();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.output_files.len(), 1);
    assert_eq!(r.output_files[0].path, format!("{wd}/public/stdin.js"));

    let mut cli = std::process::Command::new(&bin)
        .args([
            "--bundle",
            "--format=iife",
            "--platform=browser",
            "--target=es2015",
            "--loader=ts",
            "--log-level=silent",
        ])
        .current_dir(&d)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    cli.stdin.take().unwrap().write_all(src.as_bytes()).unwrap();
    let out = cli.wait_with_output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&r.output_files[0].contents),
        String::from_utf8_lossy(&out.stdout)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn service_plugins_errors_and_concurrency() {
    let Some(bin) = esbuild() else { return };
    let d = tmpdir("plugins");
    let wd = d.to_string_lossy().into_owned();
    let svc = Arc::new(ServiceClient::start(&bin).unwrap());

    // Two onResolve callbacks match `virtual:*`: the first returns no path (Go: continue), the
    // second resolves into a namespace whose onLoad returns the contents.
    let log = std::sync::Mutex::new(Vec::<String>::new());
    let plugins = vec![
        Plugin {
            name: "first".into(),
            on_resolve: vec![(
                ".*".into(),
                String::new(),
                Box::new(|a: &OnResolveArgs| {
                    log.lock()
                        .unwrap()
                        .push(format!("first {} {} {}", a.path, a.importer, a.kind));
                    Ok(OnResolveResult::default())
                }) as OnResolveFn<'_>,
            )],
            on_load: vec![],
        },
        Plugin {
            name: "second".into(),
            on_resolve: vec![
                (
                    "^virtual:".into(),
                    String::new(),
                    Box::new(|a: &OnResolveArgs| {
                        Ok(OnResolveResult {
                            path: a.path.trim_start_matches("virtual:").to_string(),
                            namespace: "virt".into(),
                            external: false,
                        })
                    }) as OnResolveFn<'_>,
                ),
                (
                    "^ext$".into(),
                    String::new(),
                    Box::new(|a: &OnResolveArgs| {
                        Ok(OnResolveResult {
                            path: a.path.clone(),
                            external: true,
                            ..Default::default()
                        })
                    }) as OnResolveFn<'_>,
                ),
            ],
            on_load: vec![(
                ".*".into(),
                "virt".into(),
                Box::new(|a: &OnLoadArgs| {
                    Ok(OnLoadResult {
                        contents: Some(format!("export default {:?};", a.path).into_bytes()),
                        loader: "js".into(),
                        ..Default::default()
                    })
                }) as OnLoadFn<'_>,
            )],
        },
    ];
    let mut o = stdin_opts(
        &wd,
        "import v from 'virtual:hello';\nimport e from 'ext';\nconsole.log(v, e);\n",
        "js",
    );
    o.format = "esm".into();
    let r = svc.build_with_plugins(&o, &plugins).unwrap();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let out = String::from_utf8_lossy(&r.output_files[0].contents).into_owned();
    assert!(out.contains("\"hello\""), "{out}");
    assert!(out.contains("from \"ext\""), "{out}");
    assert!(out.contains("// virt:hello"), "{out}");
    drop(plugins);
    let log = log.into_inner().unwrap();
    assert!(
        log.contains(&"first virtual:hello <stdin> import-statement".to_string()),
        "{log:?}"
    );

    // A callback error becomes a build error with the plugin's name.
    let failing = vec![Plugin {
        name: "failing".into(),
        on_resolve: vec![(
            ".*".into(),
            String::new(),
            Box::new(|_: &OnResolveArgs| Err(nh_common::Error::new("boom"))) as OnResolveFn<'_>,
        )],
        on_load: vec![],
    }];
    let r = svc
        .build_with_plugins(&stdin_opts(&wd, "import 'x';", "js"), &failing)
        .unwrap();
    assert_eq!(r.errors.len(), 1);
    assert_eq!(r.errors[0].text, "boom");
    assert_eq!(r.errors[0].plugin_name, "failing");

    // Syntax errors come back with their location.
    let r = svc
        .build_with_plugins(&stdin_opts(&wd, "let x = ;\n", "js"), &[])
        .unwrap();
    assert_eq!(r.errors.len(), 1);
    let loc = r.errors[0].location.as_ref().unwrap();
    assert_eq!((loc.file.as_str(), loc.line, loc.column), ("<stdin>", 1, 8));
    assert_eq!(r.errors[0].text, "Unexpected \";\"");

    // Builds from several threads share one service.
    std::thread::scope(|s| {
        for i in 0..8 {
            let svc = svc.clone();
            let wd = wd.clone();
            s.spawn(move || {
                let src = format!("console.log({i} + 1);\n");
                let r = svc
                    .build_with_plugins(&stdin_opts(&wd, &src, "js"), &[])
                    .unwrap();
                let out = String::from_utf8_lossy(&r.output_files[0].contents).into_owned();
                assert!(out.contains(&format!("console.log({i} + 1)")), "{out}");
            });
        }
    });
    let _ = std::fs::remove_dir_all(&d);
}
