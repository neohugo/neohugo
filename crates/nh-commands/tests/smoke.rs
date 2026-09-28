//! The build command end to end against the Go oracle (`tools/go-oracle/nh-commands/smoke`):
//! `neohugo [build] --minify --clock ... -d <out>` (and variants) on small synthetic sites
//! whose templates use only the template functions ported at this task's base. The published
//! files, `hugo_stats.json`, `.hugo_build.lock`, stdout (the processing stats table), stderr
//! and the exit code must equal Go's. Full-site parity is I01's.

mod support;

use std::sync::{Arc, Mutex};

use nh_commands::commandeer::{ExecOptions, execute_with};
use serde_json::Value as J;
use support::{TempDir, fixture, norm_output};

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|c| format!("{c:02x}")).collect()
}

/// The oracle's `dump`: every regular file below root except home, tmp, bin and the site's
/// sources.
fn dump(root: &std::path::Path, rel: &str, out: &mut serde_json::Map<String, J>) {
    for e in std::fs::read_dir(root.join(rel)).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_str().unwrap().to_string();
        let r = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if e.file_type().unwrap().is_dir() {
            if matches!(
                r.as_str(),
                "home" | "tmp" | "bin" | "site/content" | "site/layouts" | "site/static"
            ) {
                continue;
            }
            dump(root, &r, out);
            continue;
        }
        if r == "site/hugo.toml" {
            continue;
        }
        let b = std::fs::read(root.join(&r)).unwrap();
        out.insert(r, J::String(hex_encode(&b)));
    }
}

fn run_case(fx: &J, c: &J) -> J {
    let tmp = TempDir::new(c["name"].as_str().unwrap());
    let root = tmp.str();
    let site = tmp.path.join("site");
    for (p, content) in c["files"].as_object().unwrap() {
        let fp = site.join(p);
        std::fs::create_dir_all(fp.parent().unwrap()).unwrap();
        std::fs::write(fp, content.as_str().unwrap().replace("$ROOT", &root)).unwrap();
    }
    for d in ["home", "tmp", "bin"] {
        std::fs::create_dir_all(tmp.path.join(d)).unwrap();
    }
    let mut env: Vec<(String, String)> = Vec::new();
    for (k, v) in fx["env"].as_object().unwrap() {
        env.push((k.clone(), v.as_str().unwrap().replace("$ROOT", &root)));
    }
    if let Some(extra) = c["env"].as_object() {
        for (k, v) in extra {
            env.push((k.clone(), v.as_str().unwrap().replace("$ROOT", &root)));
        }
    }
    env.sort();
    let args: Vec<String> = c["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap().replace("$ROOT", &root))
        .collect();
    let stdout = Arc::new(Mutex::new(Vec::<u8>::new()));
    let stderr = Arc::new(Mutex::new(Vec::<u8>::new()));
    let opts = ExecOptions {
        stdout: stdout.clone(),
        stderr: stderr.clone(),
        cwd: Some(site.to_str().unwrap().to_string()),
        environ: Some(env),
        func_map_factory: nh_commands::funcmap::production_func_map_factory(),
    };
    // Template execution recurses like Go (HUGO_LAYER.md §5 deep input): a large stack.
    let exit = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || execute_with(args, opts))
        .unwrap()
        .join()
        .unwrap();
    let mut tree = serde_json::Map::new();
    dump(&tmp.path, "", &mut tree);
    let so = String::from_utf8_lossy(&stdout.lock().unwrap()).into_owned();
    let se = String::from_utf8_lossy(&stderr.lock().unwrap()).into_owned();
    serde_json::json!({
        "stdout": norm_output(&so, &root),
        "stderr": norm_output(&se, &root),
        "exit": exit,
        "tree": J::Object(tree),
    })
}

#[test]
fn smoke_builds() {
    let fx = fixture("smoke/smoke.json.gz");
    let cases = fx["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut files = 0;
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let want = &c["result"];
        let got = run_case(&fx, c);
        files += want["tree"].as_object().unwrap().len();
        if &got != want {
            let mut msg = format!("{name}:");
            for k in ["exit", "stdout", "stderr"] {
                if got[k] != want[k] {
                    msg.push_str(&format!("\n  {k}: got {} want {}", got[k], want[k]));
                }
            }
            let (gt, wt) = (
                got["tree"].as_object().unwrap(),
                want["tree"].as_object().unwrap(),
            );
            for (p, v) in wt {
                match gt.get(p) {
                    None => msg.push_str(&format!("\n  missing {p}")),
                    Some(g) if g != v => {
                        msg.push_str(&format!("\n  differs {p}: got {g} want {v}"))
                    }
                    _ => {}
                }
            }
            for p in gt.keys() {
                if !wt.contains_key(p) {
                    msg.push_str(&format!("\n  extra {p}"));
                }
            }
            failures.push(msg);
        }
    }
    eprintln!(
        "smoke: {} builds, {files} files, {} failures",
        cases.len(),
        failures.len()
    );
    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
}
