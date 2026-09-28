//! Differential tests against `tools/go-oracle/nh-config/hexec` (fixture `hexec/hexec.json.gz`):
//! hexec.New/Npx/Run with real shell scripts in a temporary tree (binary lookup order, the npx
//! cache, the security checks, the filtered environment plus WithEnviron, and the errors of
//! Run). The trees are rebuilt from the fixture; the environment is passed explicitly with
//! `Exec::new_with_env` (Go sets the process environment).

#![cfg(unix)]

mod support;

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};

use nh_config::hexec::{CommandOptions, Exec, is_not_found};
use nh_config::security::security_config::default_config;
use nh_config::security::whitelist::Whitelist;
use serde_json::{Map as JMap, Value as J};
use support::{fixture, j_string, str_enc};

#[derive(Clone, Default)]
struct Buf(Arc<Mutex<Vec<u8>>>);

impl Write for Buf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Buf {
    fn string(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

fn strs(j: &J) -> Vec<String> {
    j.as_array()
        .map(|a| a.iter().map(j_string).collect())
        .unwrap_or_default()
}

#[test]
fn hexec_scenarios() {
    let fx = fixture("hexec/hexec.json.gz");
    let scripts = fx["scripts"].as_object().unwrap();
    let mut custom = default_config();
    custom.exec.allow = Whitelist::must_new(&[
        "^(npx|postcss|tailwindcss|babel|sass|failnf|failother|missing-tool|go|nox)$",
    ]);

    let tmp = std::env::temp_dir().join(format!("nhhexec-{}", std::process::id()));
    let mut runs_total = 0;
    let mut failures = Vec::new();
    for (ci, c) in fx["cases"].as_array().unwrap().iter().enumerate() {
        let name = j_string(&c["name"]);
        let root = tmp.join(format!("{ci}"));
        std::fs::create_dir_all(&root).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let root_s = root.to_string_lossy().into_owned();
        for f in c["files"].as_array().unwrap() {
            let rel = j_string(&f[0]);
            let kind = j_string(&f[1]);
            let mode = f[2].as_u64().unwrap() as u32;
            let p = root.join(&rel);
            if kind == "dir" {
                std::fs::create_dir_all(&p).unwrap();
            } else {
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(&p, j_string(&scripts[&kind])).unwrap();
            }
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
        }
        let path = j_string(&c["path"]).replace("$ROOT", &root_s);
        let mut environ = vec![format!("PATH={path}")];
        environ.extend(strs(&c["env"]));
        let sc = if j_string(&c["sec"]) == "custom" {
            custom.clone()
        } else {
            default_config()
        };
        let wd = root.join("wd").to_string_lossy().into_owned();
        let ex = Exec::new_with_env(sc, &wd, &environ, Some(path.clone()));
        let repl = |s: &str| s.replace(&root_s, "$ROOT");

        let runs = c["runs"].as_array().unwrap();
        let results = c["results"].as_array().unwrap();
        assert_eq!(runs.len(), results.len());
        for (ri, (r, want)) in runs.iter().zip(results).enumerate() {
            runs_total += 1;
            let op = j_string(&r["op"]);
            let bin = j_string(&r["bin"]);
            let mut got = JMap::new();
            if op == "Delete" {
                std::fs::remove_file(root.join(&bin)).unwrap();
            } else {
                let stdout = Buf::default();
                let stderr = Buf::default();
                let with_stderr = r["stderr"].as_bool().unwrap();
                let stdin = j_string(&r["stdin"]);
                let opts = CommandOptions {
                    args: strs(&r["args"]),
                    stdin: if stdin.is_empty() {
                        None
                    } else {
                        Some(Box::new(std::io::Cursor::new(stdin.into_bytes())))
                    },
                    stdout: Some(Box::new(stdout.clone())),
                    stderr: if with_stderr {
                        Some(Box::new(stderr.clone()))
                    } else {
                        None
                    },
                    env: strs(&r["env"]),
                    dir: None,
                };
                let runner = if op == "Npx" {
                    ex.npx(&bin, opts)
                } else {
                    ex.command(&bin, opts)
                };
                match runner {
                    Err(e) => {
                        got.insert("err".into(), str_enc(repl(&e.to_string()).as_bytes()));
                        got.insert("notFound".into(), J::Bool(is_not_found(&e)));
                    }
                    Ok(runner) => {
                        if let Err(e) = runner.run() {
                            got.insert("runErr".into(), str_enc(repl(&e.to_string()).as_bytes()));
                            got.insert("notFound".into(), J::Bool(is_not_found(&e)));
                        }
                        got.insert("stdout".into(), str_enc(repl(&stdout.string()).as_bytes()));
                        if with_stderr {
                            got.insert("stderr".into(), str_enc(repl(&stderr.string()).as_bytes()));
                        }
                    }
                }
            }
            let got = J::Object(got);
            if &got != want {
                failures.push(format!(
                    "{name} run {ri} ({op} {bin}):\n  want {want}\n  got  {got}"
                ));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(
        failures.is_empty(),
        "{} of {runs_total} runs differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(runs_total >= 30);
}
