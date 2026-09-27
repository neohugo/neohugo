//! Module `goi18n::bundle`.
//!
//! PORT gohugoio/go-i18n/v2 i18n/bundle.go
//!
//! Owner: Wave B task T17 (i18n).


//! `gohugoio/go-i18n/v2` `i18n/bundle.go`.

use std::collections::BTreeMap;

use nh_common::Result;

use super::message::MessageTemplate;
use super::plural::rules::Rules;

/// Go: `i18n.Bundle`.
pub struct Bundle {
    pub default_language: String,
    /// Tags in insertion order (default first).
    pub tags: Vec<String>,
    /// tag -> message id -> template.
    pub message_templates: BTreeMap<String, BTreeMap<String, MessageTemplate>>,
    pub plural_rules: Rules,
}

impl Bundle {
    // Go: go-i18n i18n/bundle.go:NewBundle
    pub fn new(default_language: &str) -> Bundle {
        todo!()
    }

    /// Go: `ParseMessageFileBytes(buf, path)` (unmarshal by extension; toml/yaml/json).
    // Go: go-i18n i18n/bundle.go:ParseMessageFileBytes
    pub fn parse_message_file_bytes(&mut self, buf: &[u8], path: &str) -> Result<()> {
        todo!()
    }
}
