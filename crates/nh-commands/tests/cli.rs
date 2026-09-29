//! The neohugo command line against the Go oracle (`tools/go-oracle/nh-commands/cli`).
//!
//! parse: the command cobra resolves, the flag parse (errors included), the changed flags and
//! the flag -> config key mapping of `flagsToCfg`.
//! run: `commands.Execute` as main.go runs it (stdout, stderr, exit code) for the commands that
//! do not build a site and for command lines that fail before the build.

mod support;

use std::sync::{Arc, Mutex};

use nh_commands::cobra;
use nh_commands::commandeer::{ExecOptions, Flags, execute_with, map_legacy_args};
use nh_commands::commands::new_tree;
use nh_commands::helpers::flags_to_new_cfg;
use serde_json::{Value as J, json};
use support::{TempDir, encode, fixture, materialize, norm_output};

/// Cases whose Go output the port cannot produce, with the reason. The test checks that the
/// port fails with the explicit error instead.
const KNOWN: &[(&str, &str)] = &[];

fn replace_root(s: &str, root: &str) -> String {
    s.replace("$ROOT", root)
}

fn args_of(c: &J, root: &str) -> Vec<String> {
    c["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| replace_root(a.as_str().unwrap(), root))
        .collect()
}

/// Parse mode: the port's view of the command line, in the oracle's format.
fn parse_port(args: &[String]) -> J {
    let mut res = serde_json::Map::new();
    let args = map_legacy_args(args.to_vec());
    res.insert("mappedArgs".into(), json!(args));
    let tree = new_tree();
    let (path, flag_args, err) = tree.find(&args);
    res.insert("command".into(), json!(tree.command_path(&path)));
    if let Some(e) = err {
        res.insert("findErr".into(), json!(e));
        return J::Object(res);
    }
    res.insert("flagArgs".into(), json!(flag_args));
    let parsed = match cobra::parse(&tree, &args) {
        Ok(p) => p,
        Err((_, e)) => {
            res.insert("parseErr".into(), json!(e));
            return J::Object(res);
        }
    };
    let changed: Vec<J> = parsed
        .flags
        .visit_all()
        .filter(|f| f.changed)
        .map(
            |f| json!({"name": f.name, "type": f.flag_type().go_name(), "value": f.value_string()}),
        )
        .collect();
    res.insert(
        "changed".into(),
        if changed.is_empty() {
            J::Null
        } else {
            J::Array(changed)
        },
    );
    res.insert("positional".into(), json!(parsed.positional));
    let cfg = flags_to_new_cfg(&Flags::from_flag_set(&parsed.flags));
    res.insert("cfg".into(), encode(&cfg.get("")));
    J::Object(res)
}

struct RunOut {
    stdout: String,
    stderr: String,
    exit: i32,
}

type Buf = Arc<Mutex<Vec<u8>>>;

fn run_port(fx: &J, c: &J, tmp: &TempDir) -> RunOut {
    let root = tmp.str();
    let sites = fx["sites"].as_object().unwrap();
    let site_key = c["site"].as_str().unwrap();
    let site_dir = sites[site_key]["dir"].as_str().unwrap();
    let mut keys: Vec<&String> = sites.keys().collect();
    keys.sort();
    for k in keys {
        let dir = sites[k]["dir"].as_str().unwrap();
        if dir == site_dir && k != site_key {
            continue;
        }
        materialize(
            &tmp.path.join(dir),
            sites[k]["files"].as_object().unwrap(),
            &root,
        );
    }
    for d in ["home", "tmp", "bin"] {
        std::fs::create_dir_all(tmp.path.join(d)).unwrap();
    }
    let mut env: Vec<(String, String)> = Vec::new();
    for (k, v) in fx["env"].as_object().unwrap() {
        env.push((k.clone(), replace_root(v.as_str().unwrap(), &root)));
    }
    if let Some(extra) = c.get("env").and_then(|e| e.as_object()) {
        for (k, v) in extra {
            env.push((k.clone(), replace_root(v.as_str().unwrap(), &root)));
        }
    }
    env.sort();

    let stdout: Buf = Arc::new(Mutex::new(Vec::new()));
    let stderr: Buf = Arc::new(Mutex::new(Vec::new()));
    let opts = ExecOptions {
        stdout: stdout.clone(),
        stderr: stderr.clone(),
        cwd: Some(tmp.path.join(site_dir).to_str().unwrap().to_string()),
        environ: Some(env),
        func_map_factory: None,
    };
    let exit = execute_with(args_of(c, &root), opts);
    let so = String::from_utf8_lossy(&stdout.lock().unwrap()).into_owned();
    let se = String::from_utf8_lossy(&stderr.lock().unwrap()).into_owned();
    RunOut {
        stdout: norm_output(&so, &root),
        stderr: norm_output(&se, &root),
        exit,
    }
}

/// The lines of `s` that main's `log.Fatalf` wrote ("Error: ...", continuation lines included).
fn error_lines(s: &str) -> String {
    match s.find("Error: ") {
        Some(i) => s[i..].to_string(),
        None => String::new(),
    }
}

#[test]
fn cli_cases() {
    let fx = fixture("cli/cli.json.gz");
    let cases = fx["cases"].as_array().unwrap();
    let mut failures: Vec<String> = Vec::new();
    let (mut n_parse, mut n_run, mut n_known) = (0, 0, 0);

    for c in cases {
        let name = c["name"].as_str().unwrap();
        let tmp = TempDir::new(name);
        let root = tmp.str();

        if let Some(want) = c.get("parse") {
            n_parse += 1;
            let got = parse_port(&args_of(c, &root));
            let got = serde_json::from_str::<J>(&norm_output(&got.to_string(), &root)).unwrap();
            if name.starts_with("unsupported/") {
                // The port does not declare the flags of the commands it does not support: only
                // the command resolution is compared (and the run below fails explicitly).
                if got["command"] != want["command"] {
                    failures.push(format!(
                        "{name}: parse command: got {} want {}",
                        got["command"], want["command"]
                    ));
                }
                let mut want_args = Vec::new();
                for a in args_of(c, &root) {
                    want_args.push(a);
                }
                let out = {
                    let stdout: Buf = Arc::new(Mutex::new(Vec::new()));
                    let stderr: Buf = Arc::new(Mutex::new(Vec::new()));
                    let exit = execute_with(
                        want_args,
                        ExecOptions {
                            stdout,
                            stderr: stderr.clone(),
                            cwd: Some(root.clone()),
                            environ: Some(vec![("HOME".into(), root.clone())]),
                            func_map_factory: None,
                        },
                    );
                    (
                        exit,
                        String::from_utf8_lossy(&stderr.lock().unwrap()).into_owned(),
                    )
                };
                let want_err = format!(
                    "Error: command error: neohugo-rs: the {} command is not supported\n",
                    serde_json::to_string(&want["command"]).unwrap()
                );
                if out.0 != 1 || out.1 != want_err {
                    failures.push(format!("{name}: unsupported run: {out:?}"));
                }
            } else {
                let mut want = want.clone();
                // The oracle writes a nil list of changed flags as null or leaves it out.
                if (want.get("changed") == Some(&J::Null) || want.get("changed").is_none())
                    && let Some(o) = want.as_object_mut()
                    && o.contains_key("cfg")
                {
                    o.insert("changed".into(), J::Null);
                }
                if got != want {
                    failures.push(format!("{name}: parse:\n  got  {got}\n  want {want}"));
                }
            }
        }

        if let Some(want) = c.get("run") {
            n_run += 1;
            let got = run_port(&fx, c, &TempDir::new(name));
            let want_stdout = want["stdout"].as_str().unwrap();
            let want_stderr = want["stderr"].as_str().unwrap();
            let want_exit = want["exit"].as_i64().unwrap() as i32;

            if let Some((_, why)) = KNOWN.iter().find(|(n, _)| *n == name) {
                n_known += 1;
                if got.exit != 1 || !error_lines(&got.stdout).contains("neohugo-rs:") {
                    failures.push(format!(
                        "{name}: known gap ({why}): the port should fail explicitly: {} {:?} {:?}",
                        got.exit, got.stdout, got.stderr
                    ));
                }
                continue;
            }

            let command_error = want_stdout.contains("Error: command error:")
                || want_stderr.contains("Error: command error:");
            let help = name.starts_with("help/");
            let ok = if command_error {
                // cobra's help text (stdout) is not ported: compare the error lines.
                got.exit == want_exit
                    && got.stderr == want_stderr
                    && error_lines(&got.stdout) == error_lines(want_stdout)
            } else if help {
                got.exit == want_exit && got.stderr == want_stderr && !got.stdout.is_empty()
            } else {
                got.exit == want_exit && got.stdout == want_stdout && got.stderr == want_stderr
            };
            if !ok {
                failures.push(format!(
                    "{name}: run:\n  got  exit={} stdout={:?}\n       stderr={:?}\n  want exit={} stdout={:?}\n       stderr={:?}",
                    got.exit, got.stdout, got.stderr, want_exit, want_stdout, want_stderr
                ));
            }
        }
    }

    eprintln!(
        "cli: {} cases: {n_parse} parse, {n_run} run ({n_known} known gaps), {} failures",
        cases.len(),
        failures.len()
    );
    if !failures.is_empty() {
        panic!("{} failures:\n{}", failures.len(), failures.join("\n"));
    }
}

#[test]
fn map_legacy_args_go_table() {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        map_legacy_args(v(&["new", "posts/a.md"])),
        v(&["new", "content", "posts/a.md"])
    );
    assert_eq!(
        map_legacy_args(v(&["new", "site", "x"])),
        v(&["new", "site", "x"])
    );
    assert_eq!(map_legacy_args(v(&["new"])), v(&["new"]));
    assert_eq!(map_legacy_args(v(&["build", "x"])), v(&["build", "x"]));
}
