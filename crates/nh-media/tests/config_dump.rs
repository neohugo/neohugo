//! The `hugo config` JSON of the seeksnack build's `mediatypes`, `contenttypes` and
//! `outputformats` (no custom media types or output formats: the defaults, decoded the way
//! allconfig does) equals the committed dumps of the golden build
//! (`docs/rust-port/specs/architecture-core-data/config-en{,-printzero}.json`).

mod support;

use go_value::Value;
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_media::media::config::{decode_content_types, decode_types};
use nh_media::output::config::decode_config;
use nh_parser::frontmatter::ReplacingJsonMarshaller;
use serde_json::Value as J;

fn spec(file: &str) -> J {
    let path = format!(
        "{}/../../docs/rust-port/specs/architecture-core-data/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))).unwrap()
}

fn dump(v: &Value, omit_empty: bool) -> J {
    let b = ReplacingJsonMarshaller {
        value: v.clone(),
        keys_to_lower: true,
        omit_empty,
    }
    .marshal_json()
    .unwrap();
    serde_json::from_slice(&b).unwrap()
}

#[test]
fn seeksnack_config_dump() {
    // The seeksnack site config has no [mediaTypes] or [outputFormats].
    let cfg = DefaultConfigProvider::new();
    let media_types = decode_types(&cfg.get_string_map("mediatypes")).unwrap();
    let content_types =
        decode_content_types(&cfg.get_string_map("contenttypes"), &media_types.config).unwrap();
    let output_formats = decode_config(&media_types.config, &cfg.get("outputformats")).unwrap();

    for (file, omit_empty) in [
        ("config-en.json", true),
        ("config-th.json", true),
        ("config-en-printzero.json", false),
    ] {
        let want = spec(file);
        for (key, source) in [
            ("mediatypes", &media_types.source_structure),
            ("contenttypes", &content_types.source_structure),
            ("outputformats", &output_formats.source_structure),
        ] {
            assert_eq!(dump(source, omit_empty), want[key], "{file}: {key}");
        }
    }
}
