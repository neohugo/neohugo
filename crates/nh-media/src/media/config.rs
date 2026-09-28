//! Port of `media/config.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use go_value::{GoString, Map, MapType, Value};
use nh_common::maps::maps::{lookup_equal_fold, merge_shallow, to_string_map, to_string_map_e};
use nh_common::maps::params::clean_config_string_map;
use nh_common::{Error, Result};
use nh_config::decode::weak_decode_into;
use nh_config::namespace::{ConfigNamespace, decode_namespace};
use nh_config::struct_object;

use super::builtin::{DEFAULT_MEDIA_TYPES_CONFIG, builtin};
use super::media_type::{DEFAULT_DELIMITER, MediaType, Types, init_media_type};

/// Go: `media.defaultMediaTypesConfig` with the delimiter applied to all (Go's first `init`).
// Go: media/config.go:init
fn default_media_types_config() -> Map {
    let mut m = Map::new(MapType::StringAny);
    for (typ, suffixes) in DEFAULT_MEDIA_TYPES_CONFIG {
        let mut v = Map::new(MapType::StringAny);
        if !suffixes.is_empty() {
            v.insert("suffixes", Value::string_list(suffixes.iter().copied()));
        }
        v.insert("delimiter", Value::string("."));
        m.insert(typ, Value::map(v));
    }
    m
}

/// Go: `media.DefaultTypes` (sorted): `DecodeTypes(nil)`.
pub fn default_types() -> Types {
    static T: OnceLock<Types> = OnceLock::new();
    T.get_or_init(|| match decode_types(&Map::new(MapType::StringAny)) {
        Ok(ns) => ns.config,
        Err(e) => panic!("{e}"),
    })
    .clone()
}

/// Go: `media.MediaTypeConfig`: the configuration for a given media type.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaTypeConfig {
    /// The file suffixes used for this media type.
    pub suffixes: Vec<String>,
    /// Delimiter used before suffix.
    pub delimiter: String,
}

/// Go: `media.ContentTypeConfig` (empty struct; presence enables a content type).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContentTypeConfig {}

nh_config::decode_struct!(ContentTypeConfig, "media.ContentTypeConfig", |_s| Vec::new(
));

struct_object!(ContentTypeConfig, "media.ContentTypeConfig", |_s| []);

/// Go: `media.ContentTypes`: the media types that are considered content in Hugo.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContentTypes {
    pub html: MediaType,
    pub markdown: MediaType,
    pub ascii_doc: MediaType,
    pub pandoc: MediaType,
    pub re_structured_text: MediaType,
    pub emacs_org_mode: MediaType,
    pub(crate) types: Types,
    /// Created in init().
    pub(crate) extension_set: BTreeSet<String>,
}

/// Go: `media.DefaultContentTypes`.
// Go: media/config.go:init
pub fn default_content_types() -> ContentTypes {
    static C: OnceLock<ContentTypes> = OnceLock::new();
    C.get_or_init(|| {
        let b = builtin();
        let mut c = ContentTypes {
            html: b.html_type.clone(),
            markdown: b.markdown_type.clone(),
            ascii_doc: b.ascii_doc_type.clone(),
            pandoc: b.pandoc_type.clone(),
            re_structured_text: b.re_structured_text_type.clone(),
            emacs_org_mode: b.emacs_org_mode_type.clone(),
            ..Default::default()
        };
        c.init(&Types::default());
        c
    })
    .clone()
}

impl ContentTypes {
    // Go: media/config.go:(*ContentTypes).init
    fn init(&mut self, types: &Types) {
        // Go: sort.Slice (the types are unique by `Type` or identical).
        go_sort::sort_by(&mut self.types.0, |a, b| a.typ < b.typ);

        if let Some(tt) = types.get_by_type(&self.html.typ) {
            self.html = tt;
        }
        if let Some(tt) = types.get_by_type(&self.markdown.typ) {
            self.markdown = tt;
        }
        if let Some(tt) = types.get_by_type(&self.ascii_doc.typ) {
            self.ascii_doc = tt;
        }
        if let Some(tt) = types.get_by_type(&self.pandoc.typ) {
            self.pandoc = tt;
        }
        if let Some(tt) = types.get_by_type(&self.re_structured_text.typ) {
            self.re_structured_text = tt;
        }
        if let Some(tt) = types.get_by_type(&self.emacs_org_mode.typ) {
            self.emacs_org_mode = tt;
        }

        self.extension_set = BTreeSet::new();
        for mt in &self.types.0 {
            for suffix in mt.suffixes() {
                self.extension_set.insert(suffix);
            }
        }
    }

    // Go: media/config.go:IsContentSuffix
    pub fn is_content_suffix(&self, suffix: &str) -> bool {
        self.extension_set.contains(suffix)
    }

    /// IsContentFile returns whether the given filename is a content file.
    // Go: media/config.go:IsContentFile
    pub fn is_content_file(&self, filename: &str) -> bool {
        let ext = go_path::filepath::ext(filename);
        self.is_content_suffix(ext.strip_prefix('.').unwrap_or(ext))
    }

    /// IsIndexContentFile returns whether the given filename is an index content file.
    // Go: media/config.go:IsIndexContentFile
    pub fn is_index_content_file(&self, filename: &str) -> bool {
        if !self.is_content_file(filename) {
            return false;
        }

        let base = go_path::filepath::base(filename);

        base.starts_with("index.") || base.starts_with("_index.")
    }

    /// IsHTMLSuffix returns whether the given suffix is a HTML media type.
    // Go: media/config.go:IsHTMLSuffix
    pub fn is_html_suffix(&self, suffix: &str) -> bool {
        self.html.suffixes().iter().any(|s| s == suffix)
    }

    /// Types is a slice of media types.
    // Go: media/config.go:Types
    pub fn types(&self) -> &Types {
        &self.types
    }
}

impl nh_config::config_provider::ContentTypesProvider for ContentTypes {
    fn is_content_suffix(&self, suffix: &str) -> bool {
        ContentTypes::is_content_suffix(self, suffix)
    }
    fn is_content_file(&self, filename: &str) -> bool {
        ContentTypes::is_content_file(self, filename)
    }
    fn is_index_content_file(&self, filename: &str) -> bool {
        ContentTypes::is_index_content_file(self, filename)
    }
    fn is_html_suffix(&self, suffix: &str) -> bool {
        ContentTypes::is_html_suffix(self, suffix)
    }
}

/// Go: `media.defaultContentTypesConfig`.
fn default_content_types_config() -> Vec<String> {
    let b = builtin();
    vec![
        b.html_type.typ.clone(),
        b.markdown_type.typ.clone(),
        b.ascii_doc_type.typ.clone(),
        b.pandoc_type.typ.clone(),
        b.re_structured_text_type.typ.clone(),
        b.emacs_org_mode_type.typ.clone(),
    ]
}

/// The `map[string]ContentTypeConfig` source structure.
fn content_type_config_map(keys: &BTreeSet<String>) -> Value {
    let mut m = Map::new(MapType::Named(Arc::from(
        "map[string]media.ContentTypeConfig",
    )));
    for k in keys {
        m.insert(k.as_str(), Value::object(ContentTypeConfig {}));
    }
    Value::map(m)
}

/// Go: `media.DecodeContentTypes(in, types)`: decodes the given map of content types (the
/// default content types when it is empty).
// Go: media/config.go:DecodeContentTypes
pub fn decode_content_types(
    input: &Map,
    types: &Types,
) -> Result<ConfigNamespace<Map, ContentTypes>> {
    let build_config = |v: &Value| -> Result<(ContentTypes, Option<Value>)> {
        let mut c = default_content_types();
        let m = to_string_map_e(v)?;
        // The keys of s (Go: map[string]ContentTypeConfig).
        let mut s: BTreeSet<String> = BTreeSet::new();
        if m.is_empty() {
            s.extend(default_content_types_config());
        } else {
            let m = clean_config_string_map(&m);
            for (k, v) in &m.entries {
                let mut ctc = ContentTypeConfig {};
                weak_decode_into(v, &mut ctc)?;
                s.insert(String::from_utf8_lossy(k).into_owned());
            }
        }

        // (Go iterates s in random map order; the first unknown key in byte order is
        // reported.)
        for k in &s {
            let Some(media_type) = types.get_by_type(k) else {
                return Err(Error::new(format!(
                    "unknown media type {}",
                    go_strconv::quote(k)
                )));
            };
            c.types.0.push(media_type);
        }

        c.init(types);

        Ok((c, Some(content_type_config_map(&s))))
    };

    decode_namespace(&Value::map(input.clone()), build_config)
        .map_err(|e| Error::new(format!("failed to decode media types: {e}")))
}

/// Go: `media.DecodeTypes(in map[string]any)`: decodes the given map of media types (merged
/// with the defaults).
// Go: media/config.go:DecodeTypes
pub fn decode_types(input: &Map) -> Result<ConfigNamespace<Map, Types>> {
    let build_config = |v: &Value| -> Result<(Types, Option<Value>)> {
        let m = to_string_map_e(v)?;
        let mut m = clean_config_string_map(&m);
        // Merge with defaults.
        merge_shallow(&mut m, &default_media_types_config());

        let mut types = Types::default();

        // (Go iterates m in random map order: the first error in byte order is reported.)
        for (k, v) in &m.entries {
            let mut media_type = MediaType::from_string(&String::from_utf8_lossy(k))?;
            weak_decode_into(v, &mut media_type)?;
            let mm = to_string_map(v);
            if let Some((suffixes, _)) = lookup_equal_fold(&mm, b"suffixes") {
                let joined: Vec<GoString> = nh_common::cast::caste::to_string_slice(suffixes);
                let joined: Vec<&[u8]> = joined.iter().map(|s| s.as_bytes()).collect();
                let csv = joined.join(&b","[..]);
                let lower = go_unicode::strings::to_lower(&csv);
                let trimmed = go_unicode::strings::trim_space(&lower);
                media_type.suffixes_csv = String::from_utf8_lossy(trimmed).into_owned();
            }
            if !media_type.suffixes_csv.is_empty() && media_type.delimiter.is_empty() {
                media_type.delimiter = DEFAULT_DELIMITER.to_string();
            }
            init_media_type(&mut media_type);
            types.0.push(media_type);
        }

        types.sort();

        Ok((types, Some(Value::map(m))))
    };

    decode_namespace(&Value::map(input.clone()), build_config)
        .map_err(|e| Error::new(format!("failed to decode media types: {e}")))
}

/// Go: `media.DefaultPathParser` (its functions are nil: calling them panics).
pub fn default_path_parser() -> nh_common::paths::pathparser::PathParser {
    nh_common::paths::pathparser::PathParser::default()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: media/config.go (277 lines; 6/10 funcs executed)
//   types: ContentTypeConfig, ContentTypes, MediaTypeConfig
// OK L35-63: init()
// OK L65-76: init()
// OK L99-129: (t *ContentTypes) init(types Types)
// OK L131-133: (t ContentTypes) IsContentSuffix(suffix string) bool
// OK L136-138: (t ContentTypes) IsContentFile(filename string) bool
// OK L141-149: (t ContentTypes) IsIndexContentFile(filename string) bool
// OK L152-154: (t ContentTypes) IsHTMLSuffix(suffix string) bool
// OK L157-159: (t ContentTypes) Types() Types
// OK L179-219: DecodeContentTypes(in map[string]any, types Types) (*config.ConfigNamespace[map[string]ContentTypeConfig, ContentTypes], error)
// OK L222-267: DecodeTypes(in map[string]any) (*config.ConfigNamespace[map[string]MediaTypeConfig, Types], error)
// ---------------------------------------------------------------------------
