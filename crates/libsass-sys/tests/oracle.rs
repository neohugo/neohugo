//! Differential test against github.com/bep/golibsass@v1.2.0 (fixtures from
//! tools/go-oracle/libsass-sys): the seeksnack SCSS with Hugo's settings and
//! resolver, feature/precision/error snippets, indented syntax, the
//! import-resolver bridge (BridgeImport) semantics including the `prev`
//! paths LibSass hands to the resolver, and the red-team corpus (error
//! translation over thousands of failing inputs, deep recursion).
//!
//! Every case runs once serially and, except the slow seeksnack ones, once
//! more with several transpilations in parallel (outputs, errors and
//! resolver traces must stay per call).
//!
//! This binary holds a single test because it changes the process working
//! directory (LibSass resolves "stdin"-relative imports and source-map paths
//! against the CWD, as in Hugo).

mod common;

use std::path::Path;
use std::sync::{Arc, Mutex};

use common::{Case, Trace, options_for, read_cases, replace};
use libsass_sys::{Transpiler, new};

fn show(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    if s.len() > 400 {
        format!("{}...[{} bytes]", &s[..400], b.len())
    } else {
        s.into_owned()
    }
}

/// Outcome counters of one pass.
#[derive(Default)]
struct Counts {
    ok: usize,
    err: usize,
    site_bytes: usize,
}

/// Runs one case and returns its differences from Go (empty: none).
fn check_case(c: &Case, root_b: &[u8], website: &[u8], counts: &Mutex<Counts>) -> Vec<String> {
    let unsub = |v: &[u8]| replace(v, root_b, b"@SITE@");
    let trace: Trace = Arc::new(Mutex::new(Vec::new()));
    let opts = options_for(c, root_b, &trace);
    let src = if c.src == b"@WEBSITE@" {
        website.to_vec()
    } else {
        replace(&c.src, b"@SITE@", root_b)
    };
    let res = new(opts).unwrap().execute(&src);
    let got_trace = unsub(&trace.lock().unwrap().join(&b'\n'));
    let mut diffs = Vec::new();
    match (&res, c.expect.status.as_str()) {
        (Ok(r), "ok") => {
            {
                let mut n = counts.lock().unwrap();
                n.ok += 1;
                if c.name.starts_with("site/") {
                    n.site_bytes += r.css.len();
                }
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
            counts.lock().unwrap().err += 1;
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
    diffs
}

/// Runs `cases` on `threads` threads; returns the failure descriptions.
fn run_pass(
    cases: &[&Case],
    threads: usize,
    root_b: &[u8],
    website: &[u8],
    counts: &Mutex<Counts>,
) -> Vec<String> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let fails = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= cases.len() {
                        break;
                    }
                    let c = cases[i];
                    let diffs = check_case(c, root_b, website, counts);
                    if !diffs.is_empty() {
                        fails.lock().unwrap().push(format!(
                            "{}:\n  {}",
                            c.name,
                            diffs.join("\n  ")
                        ));
                    }
                }
            });
        }
    });
    let mut f = fails.into_inner().unwrap();
    f.sort();
    f
}

#[test]
fn golibsass_oracle() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sass");
    let root = common::extract_site(&fixtures.join("site.pack.zz"));
    std::env::set_current_dir(&root).unwrap();
    let root_b = root.to_str().unwrap().as_bytes().to_vec();
    let website = std::fs::read(root.join("assets/scss/website.scss")).unwrap();

    let mut cases = read_cases(&fixtures.join("cases.rec.zz"));
    assert!(cases.len() >= 250, "{}", cases.len());
    // Adversarial + randomized cases (tools/go-oracle/libsass-sys adv.go).
    let adv = read_cases(&fixtures.join("adv.rec.zz"));
    assert!(adv.len() >= 1500, "{}", adv.len());
    cases.extend(adv);
    // Red-team corpus (tools/go-oracle/libsass-sys redteam.go).
    let rt = read_cases(&fixtures.join("redteam.rec.zz"));
    assert!(rt.len() >= 2500, "{}", rt.len());
    cases.extend(rt);
    // LIBSASS_EXTRA_CASES: more fixture files (e.g. a larger fuzz run made
    // outside the repository), separated by ':'.
    if let Ok(extra) = std::env::var("LIBSASS_EXTRA_CASES") {
        for p in extra.split(':').filter(|p| !p.is_empty()) {
            cases.extend(read_cases(Path::new(p)));
        }
    }
    let all: Vec<&Case> = cases.iter().collect();
    let counts = Mutex::new(Counts::default());
    let fails = run_pass(&all, 1, &root_b, &website, &counts);
    let n = counts.into_inner().unwrap();
    // Concurrent pass (LIBSASS_THREADS, default 4) over the fast cases.
    let threads = std::env::var("LIBSASS_THREADS")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(4);
    let fast: Vec<&Case> = cases
        .iter()
        .filter(|c| !c.name.starts_with("site/"))
        .collect();
    let pfails = run_pass(&fast, threads, &root_b, &website, &Mutex::default());
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
    eprintln!(
        "golibsass oracle: {} cases ({} ok, {} errors, {} site CSS bytes), {} differ; \
         concurrent pass ({threads} threads): {} cases, {} differ",
        cases.len(),
        n.ok,
        n.err,
        n.site_bytes,
        fails.len(),
        fast.len(),
        pfails.len()
    );
    assert!(
        fails.is_empty() && pfails.is_empty(),
        "{} of {} cases differ:\n{}\nconcurrent pass: {} of {} differ:\n{}",
        fails.len(),
        cases.len(),
        fails.join("\n"),
        pfails.len(),
        fast.len(),
        pfails.join("\n")
    );
}
