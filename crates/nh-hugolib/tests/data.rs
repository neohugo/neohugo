//! T23 acceptance: `.Site.Data` (`HugoSites::data`: `loadData`, `handleDataFile`, `readData`)
//! against the Go oracle `tools/go-oracle/nh-hugolib/data`. For each recorded site the data tree
//! must be identical with the Go type of every value (map nesting by directory, int vs int64 vs
//! uint64 vs float64 vs string, `[]interface {}`, go-toml local dates, nil), and the log (merge
//! warnings, unexpected data types, load errors) must be the same lines.
//!
//! Deviation: after a load error Go's `Data()` is a nil map; the port's is an empty map (the
//! error is sent to the error handler either way, which fails the build).

mod support;

use serde_json::json;
use support::*;

fn run_case(name: &str) {
    let fx = fixture(&format!("data/{name}.json.gz"));
    let tmp = TempDir::new(&format!("data-{name}"));
    let b = new_sites(&fx["site"], &tmp.0).unwrap_or_else(|e| panic!("{name}: {e}"));
    let data = b.h.data();
    let mut got_data = encode_map(&data);
    let want_data = &fx["data"];
    if want_data["t"] == json!("nil:map[string]interface {}")
        && got_data["entries"].as_array().is_some_and(|e| e.is_empty())
    {
        // See the module docs (deviation).
        got_data = want_data.clone();
    }
    let got = json!({
        "site": fx["site"].clone(),
        "data": got_data,
        "log": b.log_lines(),
    });
    // The data tree is the same map on every call.
    assert!(std::sync::Arc::ptr_eq(&data, &b.h.data()));
    if let Some(d) = first_diff(name, &fx, &got) {
        panic!(
            "{name}: .Site.Data differs from Go at {d}\nlog: {:?}",
            got["log"]
        );
    }
}

macro_rules! cases {
    ($($f:ident => $n:expr),* $(,)?) => {
        $(#[test] fn $f() { run_case($n); })*
    };
}

cases! {
    data_basic => "data-basic",
    data_theme => "data-theme",
    data_themeonly => "data-themeonly",
    data_i18n => "data-i18n",
    data_mounts => "data-mounts",
    data_ignore => "data-ignore",
    data_none => "data-none",
    data_badjson => "data-badjson",
    data_badyaml => "data-badyaml",
    data_badtoml => "data-badtoml",
    data_unknownext => "data-unknownext",
    data_dirfile => "data-dirfile",
    data_docs => "data-docs",
    // XML (clbanning/mxj) gives a map; CSV (encoding/csv) gives `[][]string`, which Go's
    // `handleDataFile` logs as an unexpected data type.
    data_xml => "data-xml",
    data_csv => "data-csv",
}
