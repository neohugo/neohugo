//! The GetRemote client (`resource_factories::create::remote`) against the `getremote` oracle
//! (tools/go-oracle/nh-resource-transformers/getremote):
//!
//! - `remoteResourceKeys` of every (uri, options) pair of the oracle's calls (the `key` option,
//!   case-insensitive option names, empty and nil options, options in any order);
//! - the 51 getresource entries of the golden seeksnack build
//!   (tools/rust-port/testdata/hugo_cache/seeksnack/filecache/getresource), each read through
//!   `FromRemote` from a file cache that holds it under the key of a synthetic URL: content, media
//!   type, target path, data, links.
//!
//! The namespace-level calls (served responses, errors, the recorded cache entries) are tested
//! in crates/nh-tplfuncs/tests/resources.rs.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use go_value::{GoString, Map, MapType, Value};
use nh_config::config_provider::{Provider, config_section};
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_helpers::pathspec::PathSpec;
use nh_resource_transformers::resource_factories::create::{create, remote};
use nh_resources::resource_spec::Spec;
use serde_json::{Value as J, json};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn fixture(rel: &str) -> J {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel);
    let f = std::fs::File::open(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

fn sha(b: &[u8]) -> String {
    use sha2::Digest;
    let d = sha2::Sha256::digest(b);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decodes `rsupport.Enc`.
fn dec(v: &J) -> Value {
    let x = &v["v"];
    match v["t"].as_str().unwrap() {
        "nil" => Value::Invalid,
        "string" => Value::string(x.as_str().unwrap()),
        "bool" => Value::Bool(x.as_bool().unwrap()),
        "int" => Value::int(x.as_i64().unwrap()),
        "int64" => Value::int64(x.as_i64().unwrap()),
        "[]string" => Value::string_list(x.as_array().unwrap().iter().map(|s| s.as_str().unwrap())),
        "[]any" => Value::any_list(x.as_array().unwrap().iter().map(dec).collect()),
        "map" => Value::map(dec_map(x)),
        other => panic!("unknown encoded type {other}"),
    }
}

fn dec_map(v: &J) -> Map {
    let mut m = Map::new(MapType::StringAny);
    for e in v.as_array().unwrap() {
        m.entries
            .insert(GoString::from(e[0].as_str().unwrap()), dec(&e[1]));
    }
    m
}

struct Site {
    spec: Arc<Spec>,
    tmp: PathBuf,
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.tmp);
    }
}

/// The synthetic site's resource spec (as `rsupport.LoadSite`, with its security config).
fn load_site() -> Site {
    let dir = repo_root().join("crates/nh-resource-transformers/tests/fixtures/site");
    let dir = dir.to_string_lossy().into_owned();
    let tmp = std::env::temp_dir().join(format!("nh-rt-getremote-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let flags = DefaultConfigProvider::new();
    flags.set("workingDir", Value::string(dir.as_str()));
    flags.set("noBuildLock", Value::Bool(true));
    flags.set(
        "cacheDir",
        Value::string(format!("{}/cache", tmp.display())),
    );
    flags.set(
        "resourceDir",
        Value::string(format!("{}/resources", tmp.display())),
    );
    let configs = nh_allconfig::load::load_config(nh_allconfig::load::ConfigSourceDescriptor {
        flags: Some(Arc::new(flags)),
        filename: format!("{dir}/hugo.toml"),
        config_dir: "config".to_string(),
        environ: vec!["NEOHUGO_ORACLE=1".to_string()],
        working_dir: dir.clone(),
        getenv: Some(Arc::new(|_k: &str| String::new())),
        ..Default::default()
    })
    .unwrap();
    let fs_cfg = DefaultConfigProvider::new();
    fs_cfg.set("workingDir", Value::string(dir.as_str()));
    fs_cfg.set(
        "publishDir",
        Value::string(configs.loading_info.base_config.publish_dir.as_str()),
    );
    let hfs = nh_hugofs::fs::new_from_source_and_destination(
        nh_hugofs::afero::new_os_fs(),
        nh_hugofs::afero::new_mem_map_fs(),
        &fs_cfg,
    );
    let conf = configs.config_langs().into_iter().next().unwrap();
    let ps = PathSpec::new(hfs, conf.clone()).unwrap();
    let caches = nh_helpers::cache::filecache::filecache::new_caches(&ps).unwrap();
    let sc = (*config_section::<nh_config::security::security_config::Config>(
        conf.as_ref(),
        "security",
    ))
    .clone();
    let spec = Spec::new(
        ps,
        None,
        caches,
        &nh_common::dynacache::Cache::new(Default::default()),
        None,
        nh_config::hexec::Exec::new_with_env(sc, &dir, &[], None),
        None,
    )
    .unwrap();
    Site { spec, tmp }
}

fn marshal(v: &Value) -> String {
    String::from_utf8(go_json::marshal(v).unwrap()).unwrap()
}

/// `rsupport.Rec(r, true)` for the text and image resources of GetRemote.
fn rec(r: &dyn nh_resource::resourcetypes::Resource) -> J {
    let mut o = serde_json::Map::new();
    o.insert("name".into(), json!(r.name()));
    o.insert("title".into(), json!(r.title()));
    o.insert("key".into(), json!(r.key()));
    if let Some(n) = r.name_normalized() {
        o.insert("nameNormalized".into(), json!(n));
    }
    let mt = r.media_type();
    o.insert("mediaType".into(), json!(mt.typ));
    o.insert(
        "mediaTypeJSON".into(),
        json!(String::from_utf8(mt.marshal_json_bytes().unwrap()).unwrap()),
    );
    o.insert("resourceType".into(), json!(r.resource_type()));
    o.insert("data".into(), json!(marshal(&r.data())));
    o.insert("params".into(), json!(marshal(&Value::Map(r.params()))));
    let ctx = nh_tpl::template::TplContext::default();
    let c = r.content(ctx.as_host()).unwrap().unwrap();
    let s = c.as_go_string().unwrap().as_bytes().to_vec();
    if mt.is_text() || mt.main_type == "text" || mt.sub_type == "json" {
        o.insert("content".into(), json!(String::from_utf8_lossy(&s)));
    } else {
        o.insert("contentSha".into(), json!(sha(&s)));
    }
    o.insert("contentLen".into(), json!(s.len()));
    o.insert("relPermalink".into(), json!(r.rel_permalink()));
    o.insert("permalink".into(), json!(r.permalink()));
    J::Object(o)
}

#[test]
fn remote_resource_keys() {
    let fx = fixture("getremote/getremote.json.gz");
    let calls = fx["calls"].as_array().unwrap();
    let mut n = 0;
    for k in fx["keys"].as_array().unwrap() {
        let call = calls.iter().find(|c| c["name"] == k["name"]).unwrap();
        let args = call["args"].as_array().unwrap();
        let Value::String(uri) = dec(&args[0]) else {
            panic!("uri")
        };
        let uri = String::from_utf8(uri.as_bytes().to_vec()).unwrap();
        let options = args.get(1).map(|a| match dec(a) {
            Value::Map(m) => (*m).clone(),
            v => panic!("options {v:?}"),
        });
        let (user_key, options_key) = remote::remote_resource_keys(&uri, options.as_ref());
        assert_eq!(user_key, k["userKey"].as_str().unwrap(), "{}", k["name"]);
        assert_eq!(
            options_key,
            k["optionsKey"].as_str().unwrap(),
            "{}",
            k["name"]
        );

        // Go maps have no order: the options in reverse insertion order hash the same.
        if let Some(m) = &options {
            let mut rev = Map::new(MapType::StringAny);
            for (k, v) in m.entries.iter().rev() {
                rev.entries.insert(k.clone(), v.clone());
            }
            assert_eq!(
                remote::remote_resource_keys(&uri, Some(&rev)),
                (user_key.clone(), options_key.clone())
            );
        }
        n += 1;
    }
    assert!(n > 30, "{n} key vectors");
}

/// The 51 getresource entries of the golden build, through FromRemote.
#[test]
fn seeksnack_entries() {
    let fx = fixture("getremote/getremote.json.gz");
    let site = load_site();
    let client = create::Client::new(site.spec.clone()).unwrap();
    let cache_dir = site.tmp.join("cache/site/filecache/getresource");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let golden =
        repo_root().join("tools/rust-port/testdata/hugo_cache/seeksnack/filecache/getresource");
    let entries = fx["seeksnack"].as_array().unwrap();
    assert_eq!(entries.len(), 51);
    for e in entries {
        let Value::String(uri) = dec(&e["args"][0]) else {
            panic!("uri")
        };
        let uri = String::from_utf8(uri.as_bytes().to_vec()).unwrap();
        let (key, _) = remote::remote_resource_keys(&uri, None);
        assert_eq!(key, e["fileCacheKey"].as_str().unwrap());
        std::fs::copy(
            golden.join(e["entry"].as_str().unwrap()),
            cache_dir.join(&key),
        )
        .unwrap();

        let r = client
            .from_remote(&uri, None)
            .unwrap_or_else(|err| panic!("{uri}: {err}"))
            .unwrap();
        let got = rec(r.as_ref());
        assert_eq!(got, e["result"]["res"], "{}", e["entry"]);
        assert_eq!(r.media_type().typ, "application/json");
    }
}
