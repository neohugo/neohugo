use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use neohugo_esbuild::service::{
    BuildRequest, Hook, LoadArgs, Loaded, Plugin, ResolveArgs, Resolved, Stdin, binary_version,
};
use neohugo_esbuild::{Loader, Service, ServiceError, SourceMap};

fn stdin(code: &str) -> Stdin {
    Stdin {
        contents: code.as_bytes().to_vec(),
        loader: Loader::Js,
        resolve_dir: None,
    }
}

fn request(code: &str) -> BuildRequest {
    BuildRequest::new(stdin(code), std::env::temp_dir())
}

fn output(service: &Service, req: &BuildRequest, plugins: &[Plugin<'_>]) -> String {
    let r = service.build(req, plugins).expect("build");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.output_files.len(), 1);
    String::from_utf8(r.output_files[0].contents.clone()).unwrap()
}

#[test]
fn version_ping_and_build_round_trip() {
    let Some(binary) = crate::esbuild_binary("version_ping_and_build_round_trip") else {
        return;
    };
    let service = Service::start(&binary).unwrap();
    assert_eq!(service.version(), binary_version(&binary).unwrap());
    assert!(
        service.version().split('.').count() == 3,
        "{}",
        service.version()
    );
    service.ping().unwrap();

    let mut req = request("export const x = 1 + 1; console.log(x)");
    req.minify = true;
    assert_eq!(
        output(&service, &req, &[]),
        "(()=>{var o=2;console.log(2);})();\n"
    );
    service.ping().unwrap();
}

#[test]
fn build_errors_are_messages() {
    let Some(service) = crate::service("build_errors_are_messages") else {
        return;
    };
    let r = service.build(&request("const x = ;"), &[]).unwrap();
    assert!(r.output_files.is_empty());
    let [e] = r.errors.as_slice() else {
        panic!("{:?}", r.errors)
    };
    assert_eq!(e.text, "Unexpected \";\"");
    let loc = e.location.as_ref().unwrap();
    assert_eq!(
        (loc.file.as_str(), loc.line, loc.column),
        ("<stdin>", 1, 10)
    );
}

#[test]
fn inline_source_map() {
    let Some(service) = crate::service("inline_source_map") else {
        return;
    };
    let mut req = request("console.log(1)");
    req.source_map = SourceMap::Inline;
    let out = output(&service, &req, &[]);
    assert!(
        out.contains("//# sourceMappingURL=data:application/json;base64,"),
        "{out}"
    );
}

#[test]
fn plugin_callbacks() {
    let Some(service) = crate::service("plugin_callbacks") else {
        return;
    };
    let seen = RefCell::new(Vec::<String>::new());
    let log = |s: String| seen.borrow_mut().push(s);
    let plugins = [
        Plugin {
            name: "first".to_owned(),
            on_resolve: vec![Hook {
                filter: ".*".to_owned(),
                namespace: None,
                callback: Box::new(|a: &ResolveArgs| {
                    log(format!("first {} from {}", a.path, a.importer));
                    Ok(match a.path.as_str() {
                        "virtual:a" => Some(Resolved::Module {
                            path: "a".to_owned(),
                            namespace: Some("virt".to_owned()),
                        }),
                        "ext" => Some(Resolved::External {
                            path: "ext-lib".to_owned(),
                        }),
                        "boom" => return Err("no boom here".into()),
                        // Passes on to the next plugin.
                        _ => None,
                    })
                }),
            }],
            on_load: vec![Hook {
                filter: ".*".to_owned(),
                namespace: Some("virt".to_owned()),
                callback: Box::new(|a: &LoadArgs| {
                    log(format!("load {} in {}", a.path, a.namespace));
                    Ok(Some(Loaded {
                        contents: b"export default 'A'; import b from 'virtual:b'; export { b }"
                            .to_vec(),
                        resolve_dir: None,
                        loader: Some(Loader::Js),
                    }))
                }),
            }],
        },
        Plugin {
            name: "second".to_owned(),
            on_resolve: vec![Hook {
                filter: "^virtual:b$".to_owned(),
                namespace: None,
                callback: Box::new(|a: &ResolveArgs| {
                    log(format!("second {}", a.path));
                    Ok(Some(Resolved::Module {
                        path: "b".to_owned(),
                        namespace: Some("virt2".to_owned()),
                    }))
                }),
            }],
            on_load: vec![Hook {
                filter: ".*".to_owned(),
                namespace: Some("virt2".to_owned()),
                callback: Box::new(|_: &LoadArgs| {
                    Ok(Some(Loaded {
                        contents: br#"{"b": 2}"#.to_vec(),
                        resolve_dir: None,
                        loader: Some(Loader::Json),
                    }))
                }),
            }],
        },
    ];
    let mut req = request(
        "import a, { b } from 'virtual:a'; import * as e from 'ext'; console.log(a, b.b, e)",
    );
    req.format = neohugo_esbuild::Format::Esm;
    let out = output(&service, &req, &plugins);
    assert!(out.contains("from \"ext-lib\""), "{out}");
    assert!(out.contains("var a_default = \"A\""), "{out}");
    assert!(out.contains("// virt2:b"), "{out}");
    {
        let seen = seen.borrow();
        assert!(
            seen.contains(&"first virtual:a from <stdin>".to_owned()),
            "{seen:?}"
        );
        assert!(seen.contains(&"load a in virt".to_owned()), "{seen:?}");
        // `first` saw virtual:b and passed it on to `second`.
        assert!(
            seen.contains(&"first virtual:b from a".to_owned()),
            "{seen:?}"
        );
        assert!(seen.contains(&"second virtual:b".to_owned()), "{seen:?}");
    }

    // A callback error is a build error of that plugin.
    let r = service.build(&request("import 'boom'"), &plugins).unwrap();
    let [e] = r.errors.as_slice() else {
        panic!("{:?}", r.errors)
    };
    assert_eq!(
        (e.plugin_name.as_str(), e.text.as_str()),
        ("first", "no boom here")
    );
}

#[test]
fn concurrent_builds_share_one_service() {
    let Some(service) = crate::service("concurrent_builds_share_one_service") else {
        return;
    };
    std::thread::scope(|s| {
        for i in 0..8 {
            let service = Arc::clone(&service);
            s.spawn(move || {
                let hook = move |a: &ResolveArgs| {
                    Ok(Some(Resolved::Module {
                        path: format!("{}-{i}", a.path),
                        namespace: Some("n".to_owned()),
                    }))
                };
                let load = move |a: &LoadArgs| {
                    Ok(Some(Loaded {
                        contents: format!("export default {:?}", a.path).into_bytes(),
                        resolve_dir: None,
                        loader: Some(Loader::Js),
                    }))
                };
                let plugins = [Plugin {
                    name: format!("p{i}"),
                    on_resolve: vec![Hook {
                        filter: "^m$".to_owned(),
                        namespace: None,
                        callback: Box::new(hook),
                    }],
                    on_load: vec![Hook {
                        filter: ".*".to_owned(),
                        namespace: Some("n".to_owned()),
                        callback: Box::new(load),
                    }],
                }];
                for _ in 0..5 {
                    let mut req = request("import m from 'm'; console.log(m)");
                    req.minify = true;
                    let out = output(&service, &req, &plugins);
                    assert!(out.contains(&format!("\"m-{i}\"")), "{out}");
                }
            });
        }
    });
}

#[test]
fn a_missing_binary_is_a_spawn_error() {
    let e = Service::start(&PathBuf::from("/nonexistent/esbuild")).unwrap_err();
    assert!(matches!(e, ServiceError::Spawn { .. }), "{e}");
}

#[cfg(unix)]
#[test]
fn a_version_mismatch_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let dir = crate::scratch("fake-esbuild");
    let fake = dir.join("esbuild");
    // `--version` says 1.2.3; the service announces 9.9.9 (length-prefixed).
    std::fs::write(
        &fake,
        "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 1.2.3; exit 0; fi\nprintf '\\005\\000\\000\\0009.9.9'\ncat >/dev/null\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    match Service::start(&fake) {
        Err(ServiceError::VersionMismatch { expected, got }) => {
            assert_eq!((expected.as_str(), got.as_str()), ("1.2.3", "9.9.9"));
        }
        other => panic!("{other:?}"),
    }
}
