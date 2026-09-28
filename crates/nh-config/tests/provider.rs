//! Differential tests of the config provider against `tools/go-oracle/nh-config/provider`
//! (fixture `provider/provider.json.gz`): scripted Set/Get/Merge/SetDefaults/
//! SetDefaultMergeStrategy/WalkParams/IsSet/Keys sequences, and section hashes (the imaging
//! SourceHash).

mod support;

use go_value::{GoString, Map, MapType, Value};
use nh_common::maps::params::get_merge_strategy;
use nh_config::config_provider::{Provider, get_string_slice_preserve_string};
use nh_config::default_config_provider::DefaultConfigProvider;
use serde_json::{Value as J, json};
use support::{catch, decode, encode, j_string, str_enc};

fn as_map(v: Value) -> Map {
    match v {
        Value::Map(m) => (*m).clone(),
        other => panic!("not a map: {other:?}"),
    }
}

fn apply(cfg: &DefaultConfigProvider, op: &str, k: &str, v: Option<Value>) -> J {
    match op {
        "Set" => {
            cfg.set(k, v.unwrap());
            J::Null
        }
        "Merge" => {
            cfg.merge(k, v.unwrap());
            J::Null
        }
        "SetDefaults" => {
            cfg.set_defaults(&as_map(v.unwrap()));
            J::Null
        }
        "SetDefaultMergeStrategy" => {
            cfg.set_default_merge_strategy();
            J::Null
        }
        "Get" => encode(&cfg.get(k)),
        "GetString" => str_enc(cfg.get_string(k).as_bytes()),
        "GetInt" => json!(cfg.get_int(k)),
        "GetBool" => json!(cfg.get_bool(k)),
        "GetParams" => match cfg.get_params(k) {
            Some(m) => encode(&Value::map(m)),
            None => json!({"t": "nil:maps.Params"}),
        },
        "GetStringMap" => encode(&Value::map(cfg.get_string_map(k))),
        "GetStringMapString" => encode(&Value::map(cfg.get_string_map_string(k))),
        "GetStringSlice" => encode(&string_slice(cfg.get_string_slice(k))),
        "GetStringSlicePreserveString" => {
            encode(&string_slice(get_string_slice_preserve_string(cfg, k)))
        }
        "IsSet" => json!(cfg.is_set(k)),
        "Keys" => J::Array(cfg.keys().iter().map(|k| str_enc(k.as_bytes())).collect()),
        "Walk" => {
            let mut visited: Vec<String> = Vec::new();
            cfg.walk_params(&mut |params| {
                let path: Vec<String> = params
                    .iter()
                    .map(|p| String::from_utf8_lossy(&p.key).into_owned())
                    .collect();
                let (s, found) = get_merge_strategy(&params[params.len() - 1].params);
                visited.push(format!("{}|{}|{}", path.join("/"), s.as_str(), found));
                false
            });
            visited.sort();
            J::Array(visited.iter().map(|v| str_enc(v.as_bytes())).collect())
        }
        "Hash" => json!(nh_common::hashing::hash_string_hex(&[Value::map(
            cfg.get_string_map(k)
        )])),
        other => panic!("unknown op {other}"),
    }
}

/// `GetStringMap`/`GetStringMapString` return an empty map where Go returns a nil one
/// (nh-common deviation 13: the map conversions cannot return Go's nil map).
fn nil_map_as_empty(j: &J) -> J {
    match j["t"].as_str() {
        Some(t @ ("nil:map[string]interface {}" | "nil:map[string]string")) => {
            json!({"t": &t[4..], "entries": []})
        }
        _ => j.clone(),
    }
}

/// Go's `[]string` result (nil when empty).
fn string_slice(s: Vec<String>) -> Value {
    if s.is_empty() {
        return Value::TypedNil(std::sync::Arc::from("[]string"));
    }
    Value::string_list(s.iter().map(|x| GoString::from(x.as_str())))
}

#[test]
fn provider_matches_go() {
    let fx = fixture_cases();
    let mut mismatches = Vec::new();
    let mut ops = 0;
    let mut skipped = 0;
    for c in &fx {
        let name = c["name"].as_str().unwrap();
        let cfg = match c.get("from") {
            Some(f) => DefaultConfigProvider::new_from(as_map(decode(f))),
            None => DefaultConfigProvider::new(),
        };
        let nondet = c.get("nondet").and_then(J::as_u64).map(|n| n as usize);
        for (i, rec) in c["ops"].as_array().unwrap().iter().enumerate() {
            if nondet.is_some_and(|n| i >= n) {
                // Go's result depends on its random map order from here on.
                skipped += 1;
                break;
            }
            ops += 1;
            let op = rec["op"].as_str().unwrap();
            let k = j_string(&rec["k"]);
            let v = rec.get("v").map(decode);
            let got = catch(|| apply(&cfg, op, &k, v));
            let mut got_rec = json!({});
            match got {
                Ok(r) => got_rec["r"] = r,
                Err(p) => got_rec["panic"] = str_enc(p.as_bytes()),
            }
            let mut want_rec = json!({});
            if let Some(p) = rec.get("panic") {
                want_rec["panic"] = p.clone();
            } else {
                want_rec["r"] = nil_map_as_empty(&rec["r"]);
            }
            if let Some(root) = rec.get("root") {
                want_rec["root"] = root.clone();
                got_rec["root"] = encode(&cfg.get(""));
            }
            if got_rec != want_rec {
                mismatches.push(format!(
                    "{name} op#{i} {op}({k:?}):\n  got:  {got_rec}\n  want: {want_rec}"
                ));
                break;
            }
        }
    }
    eprintln!(
        "provider: {} scripts, {ops} ops, {skipped} nondeterministic script tails, {} mismatches",
        fx.len(),
        mismatches.len()
    );
    assert!(
        mismatches.is_empty(),
        "{}",
        mismatches[..mismatches.len().min(20)].join("\n")
    );
}

fn fixture_cases() -> Vec<J> {
    let fx = support::fixture("provider/provider.json.gz");
    fx["cases"].as_array().unwrap().clone()
}

/// The imaging config SourceHash of the seeksnack site (specs/images.md §4.3): the section as
/// `GetStringMap("imaging")` returns it after `SetDefaultMergeStrategy`, `_merge` keys included.
#[test]
fn imaging_source_hash() {
    let cfg = DefaultConfigProvider::new();
    let mut exif = Map::new(MapType::StringAny);
    exif.insert("disableDate", Value::Bool(false));
    exif.insert("disableLatLong", Value::Bool(false));
    exif.insert("excludeFields", Value::string(".*"));
    exif.insert("includeFields", Value::string(""));
    let mut imaging = Map::new(MapType::StringAny);
    imaging.insert("exif", Value::map(exif));
    let mut root = Map::new(MapType::StringAny);
    root.insert("imaging", Value::map(imaging));
    cfg.set("", Value::map(root));

    let before = nh_config::namespace::decode_namespace::<(), ()>(
        &Value::map(cfg.get_string_map("imaging")),
        |_| Ok(((), None)),
    )
    .unwrap();
    assert_eq!(before.source_hash, "d18d537662de763a");

    cfg.set_default_merge_strategy();
    let ns = nh_config::namespace::decode_namespace::<(), ()>(
        &Value::map(cfg.get_string_map("imaging")),
        |_| Ok(((), None)),
    )
    .unwrap();
    assert_eq!(ns.source_hash, "4bf645f71319dd1d");
}
