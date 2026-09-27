//! Module `goi18n::localizer`.
//!
//! PORT i18n/localizer.go
//!
//! Owner: Wave B task T17 (i18n).


use go_value::Value;
use nh_common::Result;

use super::bundle::Bundle;

/// Go: `i18n.LocalizeConfig` (subset).
pub struct LocalizeConfig {
    pub message_id: String,
    pub template_data: Value,
    pub plural_count: Option<Value>,
}

/// Go: `i18n.Localizer`.
pub struct Localizer<'a> {
    pub bundle: &'a Bundle,
    pub tags: Vec<String>,
}

impl<'a> Localizer<'a> {
    // Go: go-i18n i18n/localizer.go:NewLocalizer
    pub fn new(bundle: &'a Bundle, langs: &[&str]) -> Localizer<'a> {
        todo!()
    }

    /// Go: `LocalizeWithTag(lc)` -> (translated, tag, err). A missing message in a non-default
    /// language falls back to the default language (with a MessageNotFoundErr).
    // Go: go-i18n i18n/localizer.go:LocalizeWithTag
    pub fn localize_with_tag(&self, lc: &LocalizeConfig) -> (String, String, Option<LocalizeError>) {
        todo!()
    }
}

/// go-i18n error kinds that Hugo inspects.
#[derive(Clone, Debug)]
pub enum LocalizeError {
    MessageNotFound { tag: String, message_id: String },
    PluralFormNotFound { tag: String, message_id: String },
    Other(String),
}
