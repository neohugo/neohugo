//! Oracle test: `pagemeta.FrontMatterHandler.HandleDates`, `DecodeFrontMatterConfig` and
//! `DecodeBuildConfig` against `tools/go-oracle/nh-page/frontmatter`
//! (fixtures/frontmatter/*.json.gz). Every HandleDates call of the Go builds is replayed with
//! the site's config, then with every config variant, time zone and git date the oracle ran.

mod support;

use std::sync::{Arc, Mutex};

use go_time::GoTimeExt;
use go_value::{Map, MapType, Value};
use nh_common::loggers::{Level, Logger, Options};
use nh_config::config_provider::Provider;
use nh_page::pagemeta::page_frontmatter::{
    Dates, FrontMatterDescriptor, FrontMatterHandler, FrontmatterConfig, PageConfig,
    decode_front_matter_config,
};
use nh_page::pagemeta::pagemeta::decode_build_config;
use serde_json::Value as J;
use support::*;

fn strs(v: &J) -> Vec<String> {
    match v {
        J::Null => Vec::new(),
        _ => v
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect(),
    }
}

fn config(v: &J) -> FrontmatterConfig {
    FrontmatterConfig {
        date: strs(&v["date"]),
        lastmod: strs(&v["lastmod"]),
        publish_date: strs(&v["publishDate"]),
        expiry_date: strs(&v["expiryDate"]),
    }
}

fn config_json(c: &FrontmatterConfig) -> J {
    // Go's nil []string (an empty list) marshals as null.
    let l = |v: &Vec<String>| {
        if v.is_empty() {
            J::Null
        } else {
            serde_json::json!(v)
        }
    };
    serde_json::json!({
        "date": l(&c.date),
        "lastmod": l(&c.lastmod),
        "publishDate": l(&c.publish_date),
        "expiryDate": l(&c.expiry_date),
    })
}

fn dates_json(d: &Dates) -> J {
    serde_json::json!({
        "date": encode_time(&d.date),
        "lastmod": encode_time(&d.lastmod),
        "publishDate": encode_time(&d.publish_date),
        "expiryDate": encode_time(&d.expiry_date),
    })
}

/// The params of `after` that are not in `before` or differ (the oracle's `changed`).
fn changed(before: &Map, after: &Map) -> J {
    let mut out = serde_json::Map::new();
    for (k, v) in after.entries.iter() {
        let e = encode(v);
        match before.get(k.as_bytes()) {
            Some(bv) if encode(bv) == e => {}
            _ => {
                out.insert(String::from_utf8(k.to_vec()).unwrap(), e);
            }
        }
    }
    J::Object(out)
}

struct Sink(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn quiet_logger() -> Logger {
    let sink: nh_common::loggers::LogSink =
        Arc::new(Mutex::new(Sink(Arc::new(Mutex::new(Vec::new())))));
    Logger::with_options(Options {
        level: Level::Warn,
        std_out: Some(sink.clone()),
        std_err: Some(sink),
        store_errors: true,
        ..Default::default()
    })
}

/// Runs HandleDates on the recorded input; returns the oracle's result shape.
fn run(before: &J, cfg: &FrontmatterConfig, location: &str, git: Option<bool>) -> (J, String) {
    let params = match decode(&before["params"]) {
        Value::Map(m) => Some((*m).clone()),
        Value::TypedNil(_) => None,
        other => panic!("params {other:?}"),
    };
    let before_params = params.clone().unwrap_or_else(|| Map::new(MapType::Params));
    // The recorded call starts from the page's state; the replays from a fresh PageConfig
    // (the oracle's `&pagemeta.PageConfig{Params: ...}`).
    let mut pc = PageConfig {
        params,
        ..Default::default()
    };
    if git.is_none() {
        pc.slug = gostring(&before["slug"]);
        pc.is_from_content_adapter = before["isFromContentAdapter"].as_bool().unwrap();
        pc.dates = Dates {
            date: decode_time(&before["dates"]["date"]),
            lastmod: decode_time(&before["dates"]["lastmod"]),
            publish_date: decode_time(&before["dates"]["publishDate"]),
            expiry_date: decode_time(&before["dates"]["expiryDate"]),
        };
    }
    let mut mod_time = decode_time(&before["modTime"]);
    let git_author_date = match git {
        None => decode_time(&before["gitAuthorDate"]),
        Some(false) => go_value::Time::zero(),
        Some(true) => {
            go_time::unix(mod_time.unix() + 36 * 3600, mod_time.nsec as i64).in_loc(&go_time::utc())
        }
    };
    if git.is_some() {
        // The oracle's decodeTime: the same instant in UTC.
        mod_time = go_time::unix(mod_time.unix(), mod_time.nsec as i64).in_loc(&go_time::utc());
    }
    let logger = quiet_logger();
    let h = FrontMatterHandler::new_with_logger(Some(logger.clone()), cfg.clone()).unwrap();
    let mut d = FrontMatterDescriptor {
        base_filename: gostring(&before["baseFilename"]),
        path_or_title: gostring(&before["pathOrTitle"]),
        mod_time,
        git_author_date,
        page_config: &mut pc,
        location: go_time::load_location(location).unwrap(),
    };
    let err = h.handle_dates(&mut d).err();
    let mut out = serde_json::json!({
        "dates": dates_json(&pc.dates),
        "params": changed(&before_params, pc.params.as_ref().unwrap_or(&Map::new(MapType::Params))),
        "slug": enc(pc.slug.as_bytes()),
    });
    if let Some(e) = err {
        out["err"] = J::String(e.message().to_string());
    }
    (out, logger.errors())
}

#[test]
fn handle_dates_matches_go() {
    nh_page::page::init();
    let mut n_calls = 0;
    let mut n_variants = 0;
    let mut fails = Vec::new();
    for file in fixture_files("frontmatter") {
        if file == "decode.json.gz" {
            continue;
        }
        let fx = fixture(&format!("frontmatter/{file}"));
        let cfgs: Vec<FrontmatterConfig> = fx["configs"]
            .as_array()
            .unwrap()
            .iter()
            .map(config)
            .collect();
        let locs: Vec<String> = fx["locations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l.as_str().unwrap().to_string())
            .collect();
        for c in fx["cases"].as_array().unwrap() {
            let before = &c["before"];
            // The real call, with the site's config.
            let (got, _) = run(
                before,
                &config(&c["cfg"]),
                before["location"].as_str().unwrap(),
                None,
            );
            n_calls += 1;
            if got != c["after"] {
                fails.push(format!(
                    "{file} {}: got {got}\n   want {}",
                    before["pathOrTitle"], c["after"]
                ));
            }
            if let Some(vs) = c.get("variants") {
                for v in vs.as_array().unwrap() {
                    let cfg = &cfgs[v["cfg"].as_u64().unwrap() as usize];
                    let loc = &locs[v["loc"].as_u64().unwrap() as usize];
                    let (mut got, errors) =
                        run(before, cfg, loc, Some(v["git"].as_bool().unwrap()));
                    got["errors"] = J::String(errors);
                    n_variants += 1;
                    if got != v["out"] {
                        fails.push(format!(
                            "{file} {} cfg {} loc {loc} git {}: got {got}\n   want {}",
                            before["pathOrTitle"], v["cfg"], v["git"], v["out"]
                        ));
                    }
                }
            }
        }
    }
    eprintln!(
        "frontmatter: {n_calls} recorded HandleDates calls, {n_variants} replays, {} failures",
        fails.len()
    );
    for f in fails.iter().take(20) {
        eprintln!("  {f}");
    }
    assert!(fails.is_empty());
}

#[test]
fn decode_configs_match_go() {
    let fx = fixture("frontmatter/decode.json.gz");
    let mut fails = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let input = decode(&c["in"]);
        let got = match c["fn"].as_str().unwrap() {
            "frontmatter" => {
                let p = nh_config::default_config_provider::DefaultConfigProvider::new();
                if !matches!(input, Value::TypedNil(_)) {
                    p.set("frontmatter", input.clone());
                }
                match decode_front_matter_config(&p) {
                    Ok(cfg) => serde_json::json!({ "ok": config_json(&cfg) }),
                    Err(e) => serde_json::json!({ "err": e.message() }),
                }
            }
            "build" => {
                // Go returns the config together with the error; the port returns the error
                // alone, so the error cases compare the message only.
                match decode_build_config(&input) {
                    Ok(b) => serde_json::json!({
                        "list": b.list, "render": b.render,
                        "publishResources": b.publish_resources, "isZero": b.is_zero(),
                    }),
                    Err(e) => {
                        let mut w = c["want"].clone();
                        w["err"] = J::String(e.message().to_string());
                        w
                    }
                }
            }
            f => panic!("{f}"),
        };
        if got != c["want"] {
            fails.push(format!(
                "{} {}: got {got}, want {}",
                c["fn"], c["in"], c["want"]
            ));
        }
    }
    for f in &fails {
        eprintln!("{f}");
    }
    assert!(fails.is_empty());
}
