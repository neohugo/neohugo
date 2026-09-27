//! Module `goi18n::message`.
//!
//! PORT i18n/message.go + message_template.go
//!
//! Owner: Wave B task T17 (i18n).


use std::collections::BTreeMap;

use super::plural::form::Form;
use super::template::Template;

/// Go: `i18n.Message`.
#[derive(Clone, Debug, Default)]
pub struct Message {
    pub id: String,
    pub hash: String,
    pub description: String,
    pub left_delim: String,
    pub right_delim: String,
    pub zero: String,
    pub one: String,
    pub two: String,
    pub few: String,
    pub many: String,
    pub other: String,
}

/// Go: `i18n.MessageTemplate` (one template per plural form).
#[derive(Clone)]
pub struct MessageTemplate {
    pub message: Message,
    pub plural_templates: BTreeMap<Form, Template>,
}
