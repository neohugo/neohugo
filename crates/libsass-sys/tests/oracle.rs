//! Differential test against github.com/bep/golibsass@v1.2.0 (fixtures from
//! tools/go-oracle/libsass-sys): the seeksnack SCSS with Hugo's settings and
//! resolver, feature/precision/error snippets, indented syntax, and the
//! import-resolver bridge (BridgeImport) semantics including the `prev`
//! paths LibSass hands to the resolver.
//!
//! This binary holds a single test because it changes the process working
//! directory (LibSass resolves "stdin"-relative imports and source-map paths
//! against the CWD, as in Hugo).

mod common;

use std::path::Path;
use std::sync::{Arc, Mutex};

use common::{Trace, options_for, read_cases, replace};
use libsass_sys::{Transpiler, new};

fn show(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    if s.len() > 400 {
        format!("{}...[{} bytes]", &s[..400], b.len())
    } else {
        s.into_owned()
    }
}

#[test]
fn golibsass_oracle() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sass");
    let root = common::extract_site(&fixtures.join("site.pack.zz"));
    std::env::set_current_dir(&root).unwrap();
    let root_b = root.to_str().unwrap().as_bytes().to_vec();
    let website = std::fs::read(root.join("assets/scss/website.scss")).unwrap();
    let unsub = |v: &[u8]| replace(v, &root_b, b"@SITE@");

    let mut cases = read_cases(&fixtures.join("cases.rec.zz"));
    assert!(cases.len() >= 250, "{}", cases.len());
    // Adversarial + randomized cases (tools/go-oracle/libsass-sys adv.go).
    let adv = read_cases(&fixtures.join("adv.rec.zz"));
    assert!(adv.len() >= 1500, "{}", adv.len());
    cases.extend(adv);
    // LIBSASS_EXTRA_CASES: more fixture files (e.g. a larger fuzz run made
    // outside the repository), separated by ':'.
    if let Ok(extra) = std::env::var("LIBSASS_EXTRA_CASES") {
        for p in extra.split(':').filter(|p| !p.is_empty()) {
            cases.extend(read_cases(Path::new(p)));
        }
    }
    let mut fails = Vec::new();
    let (mut n_ok, mut n_err, mut site_bytes) = (0, 0, 0);
    for c in &cases {
        let trace: Trace = Arc::new(Mutex::new(Vec::new()));
        let opts = options_for(c, &root_b, &trace);
        let src = if c.src == b"@WEBSITE@" {
            website.clone()
        } else {
            replace(&c.src, b"@SITE@", &root_b)
        };
        let res = new(opts).unwrap().execute(&src);
        let got_trace = unsub(&trace.lock().unwrap().join(&b'\n'));
        let mut diffs = Vec::new();
        match (&res, c.expect.status.as_str()) {
            (Ok(r), "ok") => {
                n_ok += 1;
                if c.name.starts_with("site/") {
                    site_bytes += r.css.len();
                }
                if unsub(&r.css) != c.expect.css {
                    if let Ok(d) = std::env::var("LIBSASS_DUMP_DIR") {
                        let f = c.name.replace('/', "_");
                        std::fs::write(Path::new(&d).join(format!("{f}.got.css")), unsub(&r.css))
                            .unwrap();
                        std::fs::write(Path::new(&d).join(format!("{f}.want.css")), &c.expect.css)
                            .unwrap();
                    }
                    diffs.push(format!(
                        "css: got {} want {}",
                        show(&unsub(&r.css)),
                        show(&c.expect.css)
                    ));
                }
                if unsub(&r.source_map_filename) != c.expect.smfile {
                    diffs.push("source map filename".to_string());
                }
                if unsub(&r.source_map_content) != c.expect.smcontent {
                    diffs.push(format!(
                        "source map: got {} want {}",
                        show(&unsub(&r.source_map_content)),
                        show(&c.expect.smcontent)
                    ));
                }
            }
            (Err(e), "err") => {
                n_err += 1;
                if unsub(e.to_string().as_bytes()) != c.expect.err {
                    diffs.push(format!(
                        "error: got {:?} want {:?}",
                        e.to_string(),
                        show(&c.expect.err)
                    ));
                }
                let fields = (
                    e.status,
                    e.line,
                    e.column,
                    unsub(e.file.as_bytes()),
                    unsub(e.message.as_bytes()),
                );
                let want = (
                    c.expect.err_status,
                    c.expect.err_line,
                    c.expect.err_column,
                    c.expect.err_file.clone(),
                    c.expect.err_message.clone(),
                );
                if fields != want {
                    diffs.push(format!("error fields: got {fields:?} want {want:?}"));
                }
            }
            (r, want) => diffs.push(format!(
                "status: got {:?} want {want}",
                r.as_ref().map(|r| show(&r.css))
            )),
        }
        if got_trace != c.expect.trace {
            diffs.push(format!(
                "importer trace: got {:?} want {:?}",
                show(&got_trace),
                show(&c.expect.trace)
            ));
        }
        if !diffs.is_empty() {
            fails.push(format!("{}:\n  {}", c.name, diffs.join("\n  ")));
        }
    }
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
    eprintln!(
        "golibsass oracle: {} cases ({n_ok} ok, {n_err} errors, {site_bytes} site CSS bytes), {} differ",
        cases.len(),
        fails.len()
    );
    assert!(
        fails.is_empty(),
        "{} of {} cases differ:\n{}",
        fails.len(),
        cases.len(),
        fails.join("\n")
    );
}
