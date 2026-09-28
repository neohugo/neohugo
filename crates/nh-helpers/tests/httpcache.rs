//! Oracle test: Go's http.ReadResponse over httputil.DumpResponse files, the gohugoio/httpcache
//! Transport.RoundTrip over a filecache.Cache, and cache/httpcache's config, against
//! `tools/go-oracle/nh-helpers/httpcache` (fixtures/httpcache/httpcache.json.gz).

mod support;

use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, SystemTime};

use nh_common::Result;
use nh_common::herrors::Error;
use nh_helpers::cache::filecache::filecache::Cache;
use nh_helpers::cache::httpcache::http::{Header, Response, read_response};
use nh_helpers::cache::httpcache::httpcache as hc;
use nh_helpers::cache::httpcache::transport::{AroundGuard, Request, RoundTripper, Transport};
use serde_json::{Value as J, json};
use sha2::{Digest, Sha256};
use support::goval::decode_goval_map;
use support::*;

fn header_dump(h: Option<&Header>) -> J {
    match h {
        None => J::Null,
        Some(h) => J::Array(
            h.0.iter()
                .map(|(k, vs)| {
                    // Go's declared trailer keys hold a nil slice (the only empty values).
                    if vs.is_empty() {
                        json!([k, null])
                    } else {
                        json!([k, vs.iter().map(|v| enc(v)).collect::<Vec<_>>()])
                    }
                })
                .collect(),
        ),
    }
}

fn resp_dump(r: &Response) -> J {
    json!({
        "status": enc(&r.status),
        "statusCode": r.status_code,
        "proto": r.proto,
        "major": r.proto_major,
        "minor": r.proto_minor,
        "header": header_dump(Some(&r.header)),
        "contentLength": r.content_length,
        "transferEncoding": if r.transfer_encoding.is_empty() { J::Null } else { json!(r.transfer_encoding) },
        "close": r.close,
        "trailer": header_dump(r.trailer.as_ref()),
        "bodyLen": r.body.len(),
        "bodySha": hex(&Sha256::digest(&r.body)),
        "bodyErr": r.body_err.clone().map(J::String).unwrap_or(J::Null),
    })
}

fn read(dump: &[u8], method: &str) -> J {
    match read_response(dump, method) {
        Ok(r) => json!({ "ok": resp_dump(&r) }),
        Err(e) => json!({ "err": e }),
    }
}

#[test]
fn read_response_matches_go() {
    let fx = fixture("httpcache", "httpcache.json.gz");
    let dumps: Vec<Vec<u8>> = fx["dumps"].as_array().unwrap().iter().map(gostr).collect();
    let mut fails = Vec::new();
    let mut checks = 0;
    for c in fx["cases"].as_array().unwrap() {
        let mut d = dumps[c["dump"].as_u64().unwrap() as usize].clone();
        if let Some(n) = c["prefix"].as_u64() {
            d.truncate(n as usize);
        }
        if let Some(muts) = c["mut"].as_array() {
            for m in muts {
                d[m[0].as_u64().unwrap() as usize] = m[1].as_u64().unwrap() as u8;
            }
        }
        let method = c["method"].as_str().unwrap_or("GET");
        checks += 1;
        let got = read(&d, method);
        if got != c["get"] {
            fails.push(format!("{c}: got {got}"));
        }
        if !c["head"].is_null() {
            checks += 1;
            let got = read(&d, "HEAD");
            if got != c["head"] {
                fails.push(format!("HEAD {}: got {got}, want {}", c["dump"], c["head"]));
            }
        }
    }
    eprintln!("ReadResponse: {checks} reads, {} failures", fails.len());
    for f in fails.iter().take(20) {
        eprintln!("  {f}");
    }
    assert!(fails.is_empty());
}

/// The inner transport: records the requests and fails (or answers 200).
struct Rec {
    calls: Mutex<Vec<J>>,
    answer: bool,
}

impl RoundTripper for Rec {
    fn round_trip(&self, req: &Request) -> Result<Response> {
        self.calls.lock().unwrap().push(json!({
            "method": req.method,
            "url": req.url,
            "header": header_dump(Some(&req.header)),
        }));
        if self.answer {
            return Ok(Response {
                status: b"200 OK".to_vec(),
                status_code: 200,
                proto: "HTTP/1.1".to_string(),
                proto_major: 1,
                proto_minor: 1,
                body: b"fresh".to_vec(),
                ..Default::default()
            });
        }
        Err(Error::new("network disabled"))
    }
}

fn duration(name: &str) -> (i64, Option<StdDuration>) {
    const H: i64 = 3_600_000_000_000;
    match name {
        "forever" => (-1, Some(StdDuration::from_secs(7200))),
        "disabled" => (0, Some(StdDuration::from_secs(600))),
        "fresh" => (H, Some(StdDuration::from_secs(600))),
        "expired" => (H, Some(StdDuration::from_secs(7200))),
        "missing" => (-1, None),
        _ => panic!("{name}"),
    }
}

fn request_for(name: &str) -> Request {
    let (method, hdr): (&str, &[(&str, &str)]) = match name {
        "get" => ("GET", &[]),
        "head" => ("HEAD", &[]),
        "post" => ("POST", &[]),
        "range" => ("GET", &[("Range", "bytes=0-1")]),
        "no-cache" => ("GET", &[("Cache-Control", "no-cache")]),
        "max-age-0" => ("GET", &[("Cache-Control", "max-age=0")]),
        "max-age-big" => ("GET", &[("Cache-Control", "max-age=999999999999")]),
        "only-if-cached" => ("GET", &[("Cache-Control", "only-if-cached")]),
        "max-stale" => ("GET", &[("Cache-Control", "max-stale")]),
        "max-stale-n" => ("GET", &[("Cache-Control", "max-stale=999999999999")]),
        "min-fresh" => ("GET", &[("Cache-Control", "min-fresh=10")]),
        "stale-if-error" => ("GET", &[("Cache-Control", "stale-if-error")]),
        "accept-html" => ("GET", &[("Accept", "text/html")]),
        "accept-json" => ("GET", &[("Accept", "application/json")]),
        "etag-set" => ("GET", &[("Etag", "\"mine\"")]),
        _ => panic!("{name}"),
    };
    let mut h = Header::default();
    for (k, v) in hdr {
        h.0.insert(k.to_string(), vec![v.as_bytes().to_vec()]);
    }
    Request {
        method: method.to_string(),
        url: "https://www.googleapis.com/youtube/v3/videos?id=iIsZs0m-BVU&part=statistics"
            .to_string(),
        header: h,
    }
}

#[test]
fn round_trip_matches_go() {
    let fx = fixture("httpcache", "httpcache.json.gz");
    let tmp = TempDir::new("rt");
    // The entries, by name, as the oracle dumped them (the first round trip of each entry).
    let mut fails = Vec::new();
    let mut unsupported = 0;
    let mut n = 0;
    let entries = entry_dumps(&fx);
    for c in fx["roundTrips"].as_array().unwrap() {
        n += 1;
        let dir = tmp.path.join(format!("rt{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        let (max_age, age) = duration(c["cache"].as_str().unwrap());
        let bfs =
            nh_hugofs::fs::new_base_path_fs(nh_hugofs::afero::new_os_fs(), &dir.to_string_lossy());
        let cache = Cache::new(bfs, go_time::Duration(max_age), "");
        const ID: &str = "5844198154546968338";
        let dump = &entries[c["rt"].as_str().unwrap()];
        if let Some(age) = age {
            let info = cache.write(ID, dump).unwrap();
            let f = std::fs::File::options()
                .write(true)
                .open(dir.join(&info.name))
                .unwrap();
            f.set_modified(SystemTime::now() - age).unwrap();
        }
        let answer = c["answer"].as_i64().unwrap() == 200;
        let rec = Arc::new(Rec {
            calls: Mutex::new(Vec::new()),
            answer,
        });
        let always = c["always"].as_bool().unwrap();
        let c2 = cache.clone();
        let mut t = Transport::new(cache.as_http_cache());
        t.transport = Some(rec.clone());
        t.cache_key = Some(Arc::new(|_| ID.to_string()));
        t.around = Some(Arc::new(move |_, key| {
            let g: AroundGuard = Box::new(c2.named_lock(key));
            g
        }));
        t.always_use_cached_response = Some(Arc::new(move |_, _| always));
        t.should_cache = Some(Arc::new(|_, r, _| r.status_code == 200));
        t.mark_cached_responses = true;
        t.enable_etag_pair = true;

        let req = request_for(c["req"].as_str().unwrap());
        let got = match t.round_trip(&req) {
            Ok(r) => json!({ "ok": resp_dump(&r) }),
            Err(e) => json!({ "err": e.message() }),
        };
        let calls = J::Array(rec.calls.lock().unwrap().clone());
        let exists = dir.join(ID).exists();
        let want_calls = if c["calls"].is_null() {
            json!([])
        } else {
            c["calls"].clone()
        };
        let key = format!(
            "{}/{}/{}/always={}/answer={}",
            c["rt"], c["cache"], c["req"], always, answer
        );
        let store_path = answer && c["r"].get("ok").is_some() && want_calls != json!([]);
        if store_path {
            // Go stores the network response (the store path); the port has no network code.
            let msg = got["err"].as_str().unwrap_or("");
            if !msg.starts_with("neohugo-rs:") {
                fails.push(format!(
                    "{key}: want the explicit unsupported error, got {got}"
                ));
            }
            unsupported += 1;
        } else if got != c["r"] {
            fails.push(format!("{key}: got {got}, want {}", c["r"]));
        }
        if calls != want_calls {
            fails.push(format!("{key}: calls {calls}, want {want_calls}"));
        }
        if !store_path && json!(exists) != c["exists"] {
            fails.push(format!("{key}: exists {exists}, want {}", c["exists"]));
        }
    }
    eprintln!(
        "RoundTrip: {n} cases, {unsupported} store-path cases are the explicit unsupported error, {} failures",
        fails.len()
    );
    for f in fails.iter().take(20) {
        eprintln!("  {f}");
    }
    assert!(fails.is_empty());
}

/// The dumps of the round-trip entries: re-read from the oracle's recorded dumps list is not
/// possible (the entries are separate), so they are rebuilt from the recorded first-case
/// responses' names.
fn entry_dumps(fx: &J) -> std::collections::HashMap<String, Vec<u8>> {
    fx["rtEntries"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), gostr(v)))
        .collect()
}

#[test]
fn http_cache_config_matches_go() {
    let fx = fixture("httpcache", "httpcache.json.gz");
    let urls: Vec<String> = fx["httpURLs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u.as_str().unwrap().to_string())
        .collect();
    let strs = |v: &[String]| -> J { if v.is_empty() { J::Null } else { json!(v) } };
    for c in fx["httpConfigs"].as_array().unwrap() {
        let m = decode_goval_map(&c["in"]);
        let cfg = match hc::decode_config(&m) {
            Ok(cfg) => cfg,
            Err(e) => {
                assert_eq!(json!(e.message()), c["err"], "{}", c["cfg"]);
                continue;
            }
        };
        assert!(c["err"].is_null(), "{}: Go failed: {}", c["cfg"], c["err"]);
        let polls: Vec<String> = cfg.polls.iter().map(|p| p.marshal_json()).collect();
        let got = json!({
            "cacheIncludes": strs(&cfg.cache.for_.includes),
            "cacheExcludes": strs(&cfg.cache.for_.excludes),
            "polls": polls,
        });
        assert_eq!(got, c["decoded"], "{}", c["cfg"]);
        let cc = match cfg.compile() {
            Ok(cc) => cc,
            Err(e) => {
                assert_eq!(json!(e.message()), c["compileErr"], "{}", c["cfg"]);
                continue;
            }
        };
        let fors: Vec<J> = urls
            .iter()
            .map(|u| {
                let pc = cc.poll_config_for(u);
                let pcd = if pc.is_zero() {
                    J::Null
                } else {
                    J::String(pc.config.marshal_json())
                };
                json!([(cc.for_)(u), pcd])
            })
            .collect();
        assert_eq!(J::Array(fors), c["for"], "{}", c["cfg"]);
        assert_eq!(
            json!(cc.is_polling_disabled()),
            c["pollingDisabled"],
            "{}",
            c["cfg"]
        );
    }
}
