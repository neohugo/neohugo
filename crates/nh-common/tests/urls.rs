//! `urls`: the differential test against `tools/go-oracle/nh-common/urls`
//! (`fixtures/urls/baseurl.json.gz`) and common/urls/baseURL_test.go.

mod t02support;

use nh_common::urls::{BaseURL, new_base_url_from_string};
use serde_json::{Value as J, json};
use t02support::*;

fn dump(b: &BaseURL) -> J {
    json!({
        "String": b.string(),
        "WithPath": b.with_path,
        "WithPathNoTrailingSlash": b.with_path_no_trailing_slash,
        "WithoutPath": b.without_path,
        "BasePath": b.base_path,
        "BasePathNoTrailingSlash": b.base_path_no_trailing_slash,
        "Path": b.path(),
        "HostURL": b.host_url(),
        "Port": b.port(),
        "URL": enc(&b.url().string()),
    })
}

fn result(r: nh_common::Result<BaseURL>) -> J {
    match r {
        Ok(b) => json!({ "ok": dump(&b) }),
        Err(e) => json!({ "err": e.message() }),
    }
}

/// Go succeeds with a decoded path that is not UTF-8; the port refuses such a baseURL
/// explicitly (PORTING.md).
fn is_non_utf8_path_deviation(want: &J, got: &J) -> bool {
    want.get("ok")
        .is_some_and(|o| o.get("BasePath").is_some_and(|p| p.get("hex").is_some()))
        && got["err"]
            .as_str()
            .is_some_and(|e| e.starts_with("neohugo-rs: a baseURL whose decoded path"))
}

#[test]
fn base_url_matches_go() {
    let f = fixture("urls/baseurl.json.gz");
    let protocols: Vec<String> = f["protocols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let ports: Vec<i64> = f["ports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() > 3000, "too few cases: {}", cases.len());

    let mut bad = Vec::new();
    let mut checks = 0;
    let mut deviations = 0;
    for c in cases {
        let Ok(input) = String::from_utf8(bytes(&c["in"])) else {
            continue;
        };
        checks += 1;
        let r = new_base_url_from_string(&input);
        let got = result(r.clone());
        if got != c["new"] {
            if is_non_utf8_path_deviation(&c["new"], &got) {
                deviations += 1;
            } else {
                bad.push(format!("{input:?}: new: want {} got {got}", c["new"]));
            }
            continue;
        }
        let Ok(b) = r else { continue };
        if let Some(wp) = c.get("withProtocol") {
            for (p, want) in protocols.iter().zip(wp.as_array().unwrap()) {
                checks += 1;
                let got = catch(|| result(b.with_protocol(p)));
                if !same(want, &got) {
                    bad.push(format!(
                        "{input:?}: WithProtocol({p:?}): want {want} got {got:?}"
                    ));
                }
            }
            for (p, want) in ports.iter().zip(c["withPort"].as_array().unwrap()) {
                checks += 1;
                let got = catch(|| result(b.with_port(*p)));
                if !same(want, &got) {
                    bad.push(format!("{input:?}: WithPort({p}): want {want} got {got:?}"));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
    assert!(deviations <= 2, "{deviations}");
    eprintln!(
        "baseurl: {} inputs, {checks} checks, {deviations} non-UTF-8 paths refused",
        cases.len()
    );
}

// Go: common/urls/baseURL_test.go:TestBaseURL
#[test]
fn go_test_base_url() {
    let b = new_base_url_from_string("http://example.com/").unwrap();
    assert_eq!(b.string(), "http://example.com/");

    let b = new_base_url_from_string("http://example.com").unwrap();
    assert_eq!(b.string(), "http://example.com/");
    assert_eq!(b.with_path_no_trailing_slash, "http://example.com");
    assert_eq!(b.base_path, "/");

    let p = b.with_protocol("webcal://").unwrap();
    assert_eq!(p.string(), "webcal://example.com/");

    let p = b.with_protocol("webcal").unwrap();
    assert_eq!(p.string(), "webcal://example.com/");

    assert!(b.with_protocol("mailto:").is_err());

    let b = new_base_url_from_string("mailto:hugo@rules.com").unwrap();
    assert_eq!(b.string(), "mailto:hugo@rules.com");

    // These are pretty constructed
    let p = b.with_protocol("webcal").unwrap();
    assert_eq!(p.string(), "webcal:hugo@rules.com");

    let p = b.with_protocol("webcal://").unwrap();
    assert_eq!(p.string(), "webcal://hugo@rules.com");

    // Test with "non-URLs". Some people will try to use these as a way to get
    // relative URLs working etc.
    let b = new_base_url_from_string("/").unwrap();
    assert_eq!(b.string(), "/");

    let b = new_base_url_from_string("").unwrap();
    assert_eq!(b.string(), "/");

    // BaseURL with sub path
    let b = new_base_url_from_string("http://example.com/sub").unwrap();
    assert_eq!(b.string(), "http://example.com/sub/");
    assert_eq!(b.with_path_no_trailing_slash, "http://example.com/sub");
    assert_eq!(b.base_path, "/sub/");
    assert_eq!(b.base_path_no_trailing_slash, "/sub");

    let b = new_base_url_from_string("http://example.com/sub/").unwrap();
    assert_eq!(b.string(), "http://example.com/sub/");
    assert_eq!(b.host_url(), "http://example.com");
}
