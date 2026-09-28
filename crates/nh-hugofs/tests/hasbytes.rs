//! Differential test against `tools/go-oracle/nh-hugofs/hasbytes`: which written files the
//! publish-fs wrapper reports for the `__hdeferred/` and `__h_pp_l1` patterns (set up like
//! `deps.Deps.Init`, with the default media types' `IsTextSuffix`).

mod support;

use std::io::Write;
use std::sync::{Arc, Mutex};

use nh_hugofs::afero::{self, Fs, OsFs, flags};
use nh_hugofs::fs::new_base_path_fs;
use nh_hugofs::hasbytes_fs::new_has_bytes_receiver;
use serde_json::{Value as J, json};
use support::*;

fn bytes_of(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => {
            let h = o["hex"].as_str().unwrap();
            (0..h.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                .collect()
        }
        _ => panic!("bytes: {v}"),
    }
}

#[test]
fn has_bytes_receiver_matches_go() {
    let fixture = load_fixture(&fixture_dir("hasbytes").join("hasbytes.json.gz"));
    let patterns: Vec<Vec<u8>> = fixture["patterns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().as_bytes().to_vec())
        .collect();
    let tmp = TempDir::new("hasbytes");
    let publish_dir = tmp.path.join("public");
    let publish_dir = publish_dir.to_str().unwrap();

    let media_types = nh_media::media::config::default_types();
    let should_check = Arc::new(move |name: &str| {
        let ext = go_path_ext(name);
        media_types.is_text_suffix(ext.strip_prefix('.').unwrap_or(&ext))
    });

    let mut failures = Vec::new();
    let cases = fixture["cases"].as_array().unwrap();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let mode = c["mode"].as_str().unwrap();
        let chunks: Vec<Vec<u8>> = c["chunks"]
            .as_array()
            .map(|a| a.iter().map(bytes_of).collect())
            .unwrap_or_default();

        let calls: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let calls2 = calls.clone();
        let cb = Arc::new(move |n: &str, m: &[u8]| {
            calls2
                .lock()
                .unwrap()
                .push((n.to_string(), String::from_utf8_lossy(m).into_owned()));
        });
        let osfs: Arc<dyn Fs> = Arc::new(OsFs);
        let pub_fs = new_base_path_fs(osfs, publish_dir);
        let fs = new_has_bytes_receiver(pub_fs.clone(), should_check.clone(), cb, patterns.clone());
        fs.mkdir_all(&go_path::filepath::dir(name), 0o777).unwrap();
        if mode == "openfile-read" {
            afero::write_file(pub_fs.as_ref(), name, b"seed", 0o666).unwrap();
        }
        assert_eq!(
            should_check(name),
            c["shouldCheck"].as_bool().unwrap(),
            "shouldCheck({name})"
        );

        let mut f = match mode {
            "create" => fs.create(name),
            "openfile" => fs.open_file(
                name,
                flags::O_WRONLY | flags::O_CREATE | flags::O_TRUNC,
                0o666,
            ),
            "openfile-rdwr" => fs.open_file(
                name,
                flags::O_RDWR | flags::O_CREATE | flags::O_TRUNC,
                0o666,
            ),
            "openfile-read" => fs.open_file(name, flags::O_RDONLY, 0),
            _ => panic!("mode {mode}"),
        }
        .unwrap();
        if mode != "openfile-read" {
            for ch in &chunks {
                let n = f.write(ch).unwrap();
                assert_eq!(n, ch.len());
            }
        }
        f.close().unwrap();
        let written = afero::read_file(pub_fs.as_ref(), name).unwrap();
        pub_fs.remove_all(name).unwrap();

        let got_calls: Vec<J> = calls
            .lock()
            .unwrap()
            .iter()
            .map(|(n, m)| json!([n, m]))
            .collect();
        let got = json!({
            "calls": if got_calls.is_empty() { J::Null } else { J::Array(got_calls) },
            "written": written,
        });
        let want = json!({"calls": c["calls"], "written": bytes_of(&c["written"])});
        if got != want {
            failures.push(format!("{name} ({mode}): want {want}\n   got {got}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(10)].join("\n")
    );
}

/// Go: `filepath.Ext`.
fn go_path_ext(name: &str) -> String {
    go_path::filepath::ext(name).to_string()
}
