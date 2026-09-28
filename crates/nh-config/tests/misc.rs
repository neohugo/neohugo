//! Differential tests against `tools/go-oracle/nh-config/misc` (fixture `misc/misc.json.gz`):
//! config/env.go, common/neohugo (versions, HugoInfo, GetExecEnviron, deprecations), the config
//! loader, the security whitelists and policies, the build cache busters, the dev server
//! matchers, DecodeNamespace hashes and Go regexp (RE2) matching.

mod support;

use std::sync::Arc;

use go_value::{HostCtx, Map, MapType, Object, Value};
use nh_config::common_config::{decode_build_config, decode_server};
use nh_config::config_loader::{
    from_config_string, is_valid_config_filename, load_config_from_dir,
};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_config::env::{
    get_memory_limit_from, get_num_worker_multiplier_from, set_env_vars, split_env_var,
};
use nh_config::goregexp::Regexp;
use nh_config::neohugo::neohugo::{
    DeprecationLevel, HugoInfo, HugoInfoConfig, deprecation_log_level_from_version,
    get_exec_environ_with,
};
use nh_config::neohugo::version::{
    CURRENT_VERSION, Version, compare_version, go_minor_version_of, parse_version,
};
use nh_config::security::security_config::decode_config as decode_security;
use nh_config::security::whitelist::Whitelist;
use serde_json::{Value as J, json};
use support::dump::Dump;
use support::{catch, decode, encode, j_string, str_enc};

fn strs(j: &J) -> Vec<String> {
    j.as_array()
        .map(|a| a.iter().map(j_string).collect())
        .unwrap_or_default()
}

fn enc_strs(v: &[String]) -> J {
    J::Array(v.iter().map(|s| str_enc(s.as_bytes())).collect())
}

fn static_suffix(s: &str) -> &'static str {
    match s {
        "-test" => "-test",
        "-DEV" => "-DEV",
        "" => "",
        other => panic!("suffix {other}"),
    }
}

fn version_json(v: &Version) -> J {
    json!([v.major, v.minor, v.patch_level, v.suffix])
}

fn err_json(e: Option<nh_common::Error>) -> J {
    match e {
        Some(e) => str_enc(e.to_string().as_bytes()),
        None => J::Null,
    }
}

struct InfoConf {
    environment: String,
    running: bool,
    working_dir: String,
    multihost: bool,
    multilingual: bool,
}

impl HugoInfoConfig for InfoConf {
    fn environment(&self) -> String {
        self.environment.clone()
    }
    fn running(&self) -> bool {
        self.running
    }
    fn working_dir(&self) -> String {
        self.working_dir.clone()
    }
    fn is_multihost(&self) -> bool {
        self.multihost
    }
    fn is_multilingual(&self) -> bool {
        self.multilingual
    }
}

fn call(o: &dyn Object, name: &str) -> Value {
    let ctx: HostCtx<'_> = &();
    o.call_method(ctx, name, &[]).unwrap().unwrap()
}

fn value_string(v: &Value) -> String {
    match v {
        Value::String(s) | Value::Safe(_, s) => s.to_str_lossy().into_owned(),
        Value::Object(o) => o.go_string().unwrap().to_str_lossy().into_owned(),
        other => panic!("not a string: {other:?}"),
    }
}

fn value_bool(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        other => panic!("not a bool: {other:?}"),
    }
}

/// Go's `http.Header.Get`: the canonical MIME header key (`textproto.CanonicalMIMEHeaderKey`
/// for the plain ASCII keys the fixture uses), first value.
fn header_get(h: &[(String, String)], k: &str) -> String {
    let canon: String = k
        .split('-')
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + &c.as_str().to_ascii_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("-");
    h.iter()
        .find(|(hk, _)| *hk == canon)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

fn check(c: &J) -> Result<(), String> {
    let op = c["op"].as_str().unwrap();
    let mut got = json!({});
    let mut want = json!({});
    macro_rules! cmp {
        ($k:expr, $g:expr) => {{
            got[$k] = $g;
            want[$k] = c[$k].clone();
        }};
    }
    match op {
        "GetNumWorkerMultiplier" => {
            let env = c["env"].as_str().unwrap();
            let cpu = c["cpu"].as_i64().unwrap();
            cmp!("r", json!(get_num_worker_multiplier_from(Some(env), cpu)));
        }
        "GetMemoryLimit" => {
            let env = c["env"].as_str().unwrap();
            let total = c["total"].as_u64().unwrap();
            cmp!("r", json!(get_memory_limit_from(Some(env), total)));
        }
        "SetEnvVars" => {
            let mut vars = strs(&c["old"]);
            let kv = strs(&c["kv"]);
            let pairs: Vec<(&str, &str)> = kv
                .chunks(2)
                .map(|p| (p[0].as_str(), p[1].as_str()))
                .collect();
            set_env_vars(&mut vars, &pairs);
            cmp!("r", enc_strs(&vars));
        }
        "SplitEnvVar" => {
            let (k, v) = split_env_var(&j_string(&c["v"]));
            cmp!("r", enc_strs(&[k, v]));
        }
        "CurrentVersion" => {
            cmp!("string", json!(CURRENT_VERSION.string()));
            cmp!("version", json!(CURRENT_VERSION.version().0));
        }
        "Version" => {
            let v = c["v"].as_array().unwrap();
            let v = Version {
                major: v[0].as_i64().unwrap(),
                minor: v[1].as_i64().unwrap(),
                patch_level: v[2].as_i64().unwrap(),
                suffix: static_suffix(v[3].as_str().unwrap()),
            };
            cmp!("string", json!(v.string()));
            cmp!("next", json!(v.next().string()));
            cmp!("prev", json!(v.prev().string()));
            cmp!("release", json!(v.release_version().string()));
            cmp!("nextPatch", json!(v.next_patch_level(3).string()));
        }
        "ParseVersion" => {
            let v = parse_version(c["s"].as_str().unwrap());
            cmp!("r", version_json(&v));
            cmp!("string", json!(v.string()));
        }
        "CompareVersion" => {
            let input = decode(&c["in"]);
            cmp!("r", json!(compare_version(&input)));
            cmp!("eq", json!(CURRENT_VERSION.version().eq_value(&input)));
        }
        "goMinorVersion" => {
            cmp!("r", json!(go_minor_version_of(c["s"].as_str().unwrap())));
        }
        "deprecationLevel" => {
            let l = match deprecation_log_level_from_version(c["s"].as_str().unwrap()) {
                DeprecationLevel::Info => "info",
                DeprecationLevel::Warn => "warn",
                DeprecationLevel::Error => "error",
            };
            cmp!("r", json!(l));
        }
        "HugoInfo" => {
            let conf = c["conf"].as_array().unwrap();
            let info = HugoInfo::new(Arc::new(InfoConf {
                environment: conf[0].as_str().unwrap().to_string(),
                running: conf[1].as_bool().unwrap(),
                working_dir: conf[2].as_str().unwrap().to_string(),
                multihost: conf[3].as_bool().unwrap(),
                multilingual: conf[4].as_bool().unwrap(),
            }));
            cmp!(
                "Environment",
                json!(value_string(&info.field("Environment").unwrap()))
            );
            for m in ["Generator", "Version", "WorkingDir"] {
                cmp!(m, json!(value_string(&call(&info, m))));
            }
            for m in [
                "IsDevelopment",
                "IsProduction",
                "IsServer",
                "IsMultihost",
                "IsMultilingual",
            ] {
                cmp!(m, json!(value_bool(&call(&info, m))));
            }
            let deps = match call(&info, "Deps") {
                Value::TypedNil(_) => 0,
                Value::List(l) => l.items.len(),
                other => panic!("deps {other:?}"),
            };
            cmp!("Deps", json!(deps));
        }
        "GetExecEnviron" => {
            let files: Vec<(String, String)> = c["files"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|f| {
                            (
                                f[0].as_str().unwrap().to_string(),
                                f[1].as_str().unwrap().to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let node_path = c["nodePath"].as_str().unwrap();
            let env = get_exec_environ_with(
                c["workDir"].as_str().unwrap(),
                c["env"].as_str().unwrap(),
                c["publishDir"].as_str().unwrap(),
                (!node_path.is_empty()).then_some(node_path),
                &files,
            );
            cmp!("r", enc_strs(&env));
        }
        "FromConfigString" => {
            match from_config_string(&j_string(&c["content"]), c["type"].as_str().unwrap()) {
                Ok(cfg) => cmp!("r", encode(&cfg.get(""))),
                Err(e) => {
                    // nh-parser deviation 1: go-toml locates an error at the very end of a buffer
                    // without spare capacity (`[]byte(config)`) at the start of the last token;
                    // the port always reports the end. Only the position differs.
                    let strip = |s: &str| -> String {
                        match s.strip_prefix("\"_stream.toml:") {
                            Some(r) => r.split_once("\": ").map_or(s, |(_, m)| m).to_string(),
                            None => s.to_string(),
                        }
                    };
                    got["err"] = json!(strip(&e.to_string()));
                    want["err"] = json!(strip(&j_string(&c["err"])));
                }
            }
        }
        "IsValidConfigFilename" => {
            cmp!(
                "r",
                json!(is_valid_config_filename(c["s"].as_str().unwrap()))
            );
        }
        "LoadConfigFromDir" => {
            let dir = std::env::temp_dir().join(format!(
                "nhconfig-{}-{}",
                std::process::id(),
                c["name"].as_str().unwrap().replace('#', "")
            ));
            let _ = std::fs::remove_dir_all(&dir);
            for f in c["files"].as_array().unwrap() {
                let p = dir.join(f[0].as_str().unwrap());
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                let content = j_string(&f[1]);
                if content == "<dir>" {
                    std::fs::create_dir_all(&p).unwrap();
                } else {
                    std::fs::write(&p, content).unwrap();
                }
            }
            let root = dir.to_string_lossy().into_owned();
            let repl = |s: &str| s.replace(&root, "$ROOT");
            match load_config_from_dir(&dir.join("config"), c["env"].as_str().unwrap()) {
                Ok((cfg, dirnames)) => {
                    let dn: Vec<String> = dirnames.iter().map(|d| repl(d)).collect();
                    cmp!("dirnames", enc_strs(&dn));
                    if let Some(cfg) = cfg {
                        cmp!("r", encode(&cfg.get("")));
                    } else {
                        cmp!("r", J::Null);
                    }
                }
                Err(e) => cmp!("err", str_enc(repl(&e.to_string()).as_bytes())),
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
        "Whitelist" => {
            let patterns = strs(&c["patterns"]);
            let refs: Vec<&str> = patterns.iter().map(String::as_str).collect();
            match Whitelist::new(&refs) {
                Err(e) => cmp!("err", str_enc(e.to_string().as_bytes())),
                Ok(w) => {
                    let names = names();
                    cmp!(
                        "accept",
                        J::Array(names.iter().map(|n| json!(w.accept(n))).collect())
                    );
                    cmp!("string", json!(w.string()));
                    cmp!(
                        "json",
                        json!(String::from_utf8(w.marshal_json_bytes().unwrap()).unwrap())
                    );
                }
            }
        }
        "Names" | "RegexpInputs" => {}
        "Security" => {
            let cfg = DefaultConfigProvider::new();
            let input = decode(&c["in"]);
            if !matches!(input, Value::Invalid | Value::TypedNil(_)) {
                cfg.set("security", input);
            }
            let (sc, err) = nh_config::security::security_config::decode_config_partial(&cfg);
            let _ = decode_security;
            cmp!("toml", json!(sc.to_toml()));
            cmp!("err", err_json(err));
            let mut checks = Vec::new();
            for n in names() {
                checks.push(json!([
                    err_json(sc.check_allowed_exec(&n).err()),
                    err_json(sc.check_allowed_get_env(&n).err()),
                    err_json(sc.check_allowed_http_url(&n).err()),
                    err_json(sc.check_allowed_http_method(&n).err()),
                ]));
            }
            cmp!("checks", J::Array(checks));
        }
        "CacheBuster" => {
            let cfg = DefaultConfigProvider::new();
            let input = decode(&c["in"]);
            if !matches!(input, Value::Invalid) {
                cfg.set("build", input);
            }
            let mut b = decode_build_config(&cfg);
            cmp!("build", b.dump());
            if let Err(e) = b.compile_config() {
                cmp!("err", str_enc(e.to_string().as_bytes()));
            } else {
                let paths = strs(&c["paths"]);
                let keys = strs(&c["keys"]);
                let mut res = Vec::new();
                for p in &paths {
                    match b.match_cache_buster(p) {
                        None => res.push(J::Null),
                        Some(m) => res.push(J::Array(keys.iter().map(|k| json!(m(k))).collect())),
                    }
                }
                cmp!("r", J::Array(res));
                let generic = nh_common::Error::new("x");
                cmp!(
                    "useCache",
                    json!([
                        b.use_resource_cache(None),
                        b.use_resource_cache(Some(&generic))
                    ])
                );
            }
        }
        "Server" => {
            let cfg = DefaultConfigProvider::new();
            let input = decode(&c["in"]);
            if !matches!(input, Value::Invalid) {
                cfg.set("server", input);
            }
            let mut s = decode_server(&cfg).unwrap();
            cmp!("server", s.dump());
            cmp!("err", J::Null);
            if let Err(e) = s.compile_config() {
                cmp!("compileErr", str_enc(e.to_string().as_bytes()));
            } else {
                let headers: Vec<Option<Vec<(String, String)>>> = vec![
                    None,
                    Some(vec![("Accept".into(), "text/html".into())]),
                    Some(vec![
                        ("Accept".into(), "application/json".into()),
                        ("X-Lang".into(), "th".into()),
                    ]),
                ];
                let mut mh = Vec::new();
                let mut mr = Vec::new();
                for p in strs(&c["patterns"]) {
                    let kv = s.match_headers(&p);
                    mh.push(if kv.is_empty() {
                        J::Null
                    } else {
                        J::Array(kv.iter().map(|(k, v)| json!([k, v])).collect())
                    });
                    let mut rs = Vec::new();
                    for h in &headers {
                        let r = match h {
                            None => s.match_redirect(&p, None),
                            Some(h) => {
                                let get = |k: &str| header_get(h, k);
                                s.match_redirect(&p, Some(&get))
                            }
                        };
                        rs.push(r.dump());
                    }
                    mr.push(J::Array(rs));
                }
                cmp!("matchHeaders", J::Array(mh));
                cmp!("matchRedirect", J::Array(mr));
            }
        }
        "DecodeNamespace" => {
            let input = decode(&c["in"]);
            let r = catch(|| {
                nh_config::namespace::decode_namespace::<(), ()>(&input, |_| Ok(((), None)))
            });
            match r {
                Err(p) => cmp!("panic", json!(p)),
                Ok(Err(e)) => cmp!("err", json!(e.to_string())),
                Ok(Ok(ns)) => {
                    cmp!("hash", json!(ns.source_hash));
                    let j = ns.marshal_json();
                    cmp!(
                        "json",
                        json!(String::from_utf8(j.clone().unwrap_or_default()).unwrap())
                    );
                    cmp!("jsonErr", err_json(j.err()));
                }
            }
            cmp!(
                "hashDirect",
                json!(nh_common::hashing::hash_string_hex(std::slice::from_ref(
                    &input
                )))
            );
        }
        "Regexp" => match Regexp::compile(&j_string(&c["pattern"])) {
            Err(e) => cmp!("err", str_enc(e.as_bytes())),
            Ok(re) => {
                let mut res = Vec::new();
                for input in regexp_inputs() {
                    let sub = match re.find_string_submatch(&input) {
                        Some(m) => enc_strs(&m),
                        None => J::Null,
                    };
                    res.push(json!([re.match_string(&input), sub]));
                }
                cmp!("r", J::Array(res));
            }
        },
        other => panic!("unknown op {other}"),
    }
    // Missing keys on either side are null.
    if let (J::Object(g), J::Object(w)) = (&mut got, &mut want) {
        for (k, v) in w.iter_mut() {
            if v.is_null() {
                g.entry(k.clone()).or_insert(J::Null);
            }
        }
    }
    if got != want {
        return Err(format!("{op}:\n  got:  {got}\n  want: {want}"));
    }
    Ok(())
}

fn fixture_cases() -> &'static [J] {
    static F: std::sync::OnceLock<Vec<J>> = std::sync::OnceLock::new();
    F.get_or_init(|| {
        support::fixture("misc/misc.json.gz")["cases"]
            .as_array()
            .unwrap()
            .clone()
    })
}

fn names() -> Vec<String> {
    let c = fixture_cases().iter().find(|c| c["op"] == "Names").unwrap();
    strs(&c["names"])
}

fn regexp_inputs() -> Vec<String> {
    let c = fixture_cases()
        .iter()
        .find(|c| c["op"] == "RegexpInputs")
        .unwrap();
    strs(&c["inputs"])
}

#[test]
fn misc_matches_go() {
    let cases = fixture_cases();
    let mut mismatches = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        match catch(|| check(c)) {
            Ok(Ok(())) => {}
            Ok(Err(m)) => mismatches.push(format!("#{i} {m}")),
            Err(p) => mismatches.push(format!("#{i} {}: panic {p}", c["op"])),
        }
    }
    eprintln!(
        "misc: {} cases, {} mismatches",
        cases.len(),
        mismatches.len()
    );
    assert!(
        mismatches.is_empty(),
        "{}",
        mismatches[..mismatches.len().min(15)].join("\n")
    );
}

#[allow(dead_code)]
fn _unused(_: Map, _: MapType) {}
