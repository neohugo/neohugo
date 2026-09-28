//! Shared helpers for the T04 nh-media oracle tests.

#![allow(dead_code, unused_imports)]

pub mod goval;

use nh_media::media::media_type::{MediaType, SuffixInfo, Types};
use nh_media::output::output_format::{Formats, OutputFormat};
use serde_json::{Value as J, json};

pub use goval::{decode, encode, fixture, str_enc};

/// A goval string as a Rust string (lossy).
pub fn j_string(j: &J) -> String {
    String::from_utf8_lossy(&goval::bytes(j)).into_owned()
}

/// A list of goval strings (`null` = none).
pub fn strs(j: &J) -> Vec<String> {
    j.as_array()
        .map(|a| a.iter().map(j_string).collect())
        .unwrap_or_default()
}

pub fn s(v: &str) -> J {
    str_enc(v.as_bytes())
}

pub fn err_j(e: Option<String>) -> J {
    match e {
        Some(e) => s(&e),
        None => J::Null,
    }
}

/// `[json, err]` of a MarshalJSON-like result.
pub fn bytes_result(r: nh_common::Result<Vec<u8>>) -> J {
    match r {
        Ok(b) => json!([str_enc(&b), J::Null]),
        Err(e) => json!([s(""), s(&e.to_string())]),
    }
}

/// Runs `f`, turning a panic into `Err(message)` (Go's recovered panic value).
pub fn catch<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    thread_local! {
        static IN_CATCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !IN_CATCH.with(|c| c.get()) || std::env::var_os("NH_SHOW_PANICS").is_some() {
                prev(info);
            }
        }));
    });
    IN_CATCH.with(|c| c.set(true));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    IN_CATCH.with(|c| c.set(false));
    r.map_err(|e| {
        if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "panic".to_string()
        }
    })
}

// ---------------------------------------------------------------------------
// Dumps (the format of tools/go-oracle/nh-config/cval)

pub fn dump_suffix_info(si: &SuffixInfo) -> J {
    json!({"@": "media.SuffixInfo", "Suffix": s(&si.suffix), "FullSuffix": s(&si.full_suffix)})
}

pub fn dump_type(t: &MediaType) -> J {
    json!({
        "@": "media.Type", "Type": s(&t.typ), "MainType": s(&t.main_type), "SubType": s(&t.sub_type),
        "Delimiter": s(&t.delimiter), "FirstSuffix": dump_suffix_info(&t.first_suffix),
        "SuffixesCSV": s(&t.suffixes_csv), "mimeSuffix": s(t.mime_suffix()),
    })
}

pub fn dump_types(t: &Types) -> J {
    J::Array(t.0.iter().map(dump_type).collect())
}

pub fn dump_format(f: &OutputFormat) -> J {
    json!({
        "@": "output.Format", "Name": s(&f.name), "MediaType": dump_type(&f.media_type), "Path": s(&f.path),
        "BaseName": s(&f.base_name), "Rel": s(&f.rel), "Protocol": s(&f.protocol),
        "IsPlainText": f.is_plain_text, "IsHTML": f.is_html, "NoUgly": f.no_ugly, "Ugly": f.ugly,
        "NotAlternative": f.not_alternative, "Root": f.root, "Permalinkable": f.permalinkable,
        "Weight": f.weight,
    })
}

pub fn dump_formats(f: &Formats) -> J {
    J::Array(f.0.iter().map(dump_format).collect())
}

/// `hugo config` JSON of a namespace's source structure: `{"json", "dump:true", "dump:false"}`.
pub fn dump_json(source_structure: &go_value::Value, out: &mut serde_json::Map<String, J>) {
    let plain =
        go_json::marshal(source_structure).map_err(|e| nh_common::Error::new(e.to_string()));
    out.insert("json".into(), bytes_result(plain));
    for omit in [true, false] {
        let r = nh_parser::frontmatter::ReplacingJsonMarshaller {
            value: source_structure.clone(),
            keys_to_lower: true,
            omit_empty: omit,
        }
        .marshal_json();
        out.insert(format!("dump:{omit}"), bytes_result(r));
    }
}

/// Compares a list of fixture records and panics with the first differences.
pub fn report(topic: &str, total: usize, failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{topic}: {} of {total} cases differ:\n{}",
        failures.len(),
        failures
            .iter()
            .take(if std::env::var_os("NH_ALL_FAILURES").is_some() {
                usize::MAX
            } else {
                30
            })
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
