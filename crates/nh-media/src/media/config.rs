//! Port of `media/config.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use go_value::Map;
use nh_common::Result;
use nh_config::namespace::ConfigNamespace;

use super::media_type::{MediaType, Types};

/// Go: `media.DefaultTypes` (sorted).
pub fn default_types() -> Types {
    todo!()
}

/// Go: `media.MediaTypeConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaTypeConfig {
    pub suffixes: Vec<String>,
    pub delimiter: String,
}

/// Go: `media.ContentTypeConfig` (empty struct; presence enables a content type).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContentTypeConfig {}

/// Go: `media.ContentTypes`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContentTypes {
    pub html: MediaType,
    pub markdown: MediaType,
    pub ascii_doc: MediaType,
    pub pandoc: MediaType,
    pub re_structured_text: MediaType,
    pub emacs_org_mode: MediaType,
    pub(crate) types: Types,
    pub(crate) extension_set: std::collections::BTreeSet<String>,
}

impl ContentTypes {
    // Go: media/config.go:IsContentSuffix
    pub fn is_content_suffix(&self, suffix: &str) -> bool { todo!() }
    // Go: media/config.go:IsContentFile
    pub fn is_content_file(&self, filename: &str) -> bool { todo!() }
    // Go: media/config.go:IsIndexContentFile
    pub fn is_index_content_file(&self, filename: &str) -> bool { todo!() }
    // Go: media/config.go:IsHTMLSuffix
    pub fn is_html_suffix(&self, suffix: &str) -> bool { todo!() }
}

/// Go: `media.DecodeTypes(in map[string]any)`.
// Go: media/config.go:DecodeTypes
pub fn decode_types(input: &Map) -> Result<ConfigNamespace<Map, Types>> {
    todo!()
}

/// Go: `media.DecodeContentTypes(in, types)`.
// Go: media/config.go:DecodeContentTypes
pub fn decode_content_types(input: &Map, types: &Types) -> Result<ConfigNamespace<Map, ContentTypes>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: media/config.go (277 lines; 6/10 funcs executed)
//   types: ContentTypeConfig, ContentTypes, MediaTypeConfig
// EX L35-63: init()
// EX L65-76: init()
// EX L99-129: (t *ContentTypes) init(types Types)
// EX L131-133: (t ContentTypes) IsContentSuffix(suffix string) bool
//    L136-138: (t ContentTypes) IsContentFile(filename string) bool
//    L141-149: (t ContentTypes) IsIndexContentFile(filename string) bool
//    L152-154: (t ContentTypes) IsHTMLSuffix(suffix string) bool
//    L157-159: (t ContentTypes) Types() Types
// EX L179-219: DecodeContentTypes(in map[string]any, types Types) (*config.ConfigNamespace[map[string]ContentTypeConfig, ContentTypes], error)
// EX L222-267: DecodeTypes(in map[string]any) (*config.ConfigNamespace[map[string]MediaTypeConfig, Types], error)
// ---------------------------------------------------------------------------
