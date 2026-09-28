//! Module `goi18n::message`.
//!
//! PORT i18n/message.go + message_template.go
//!
//! Owner: Wave B task T17 (i18n).

use std::collections::BTreeMap;

use go_value::HostCtx;

use super::parse::Raw;
use super::plural::form::Form;
use super::template::Template;

/// Go: `i18n.Message` — a string that can be localized.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Message {
    /// ID uniquely identifies the message.
    pub id: String,
    /// Hash uniquely identifies the content of the message that this message was translated
    /// from.
    pub hash: String,
    /// Description describes the message to give additional context to translators.
    pub description: String,
    /// LeftDelim is the left Go template delimiter.
    pub left_delim: String,
    /// RightDelim is the right Go template delimiter.
    pub right_delim: String,
    /// The content of the message for the CLDR plural form "zero".
    pub zero: String,
    /// The content of the message for the CLDR plural form "one".
    pub one: String,
    /// The content of the message for the CLDR plural form "two".
    pub two: String,
    /// The content of the message for the CLDR plural form "few".
    pub few: String,
    /// The content of the message for the CLDR plural form "many".
    pub many: String,
    /// The content of the message for the CLDR plural form "other".
    pub other: String,
}

/// The errors of message decoding (Go `keyTypeErr`, `valueTypeErr`, `fmt.Errorf`); the text is
/// Go's `Error()`.
pub type MessageError = String;

impl Message {
    /// Go: `NewMessage(data)` — parses data and returns a new message.
    // Go: i18n/message.go:NewMessage
    pub(crate) fn new(data: &Raw) -> Result<Message, MessageError> {
        let mut m = Message::default();
        m.unmarshal_interface(data)?;
        Ok(m)
    }

    // Go: i18n/message.go:unmarshalInterface
    /// Unmarshals a message from data. (Go iterates the string map in random order; keys that
    /// differ only in case, e.g. `other` and `Other`, then race. The port iterates in byte
    /// order, so the last key in byte order wins.)
    fn unmarshal_interface(&mut self, v: &Raw) -> Result<(), MessageError> {
        let strdata = string_map(v)?;
        for (k, v) in strdata {
            let lk = go_unicode::strings::to_lower(k.as_bytes());
            match &*lk {
                b"id" => self.id = v,
                b"description" => self.description = v,
                b"hash" => self.hash = v,
                b"leftdelim" => self.left_delim = v,
                b"rightdelim" => self.right_delim = v,
                b"zero" => self.zero = v,
                b"one" => self.one = v,
                b"two" => self.two = v,
                b"few" => self.few = v,
                b"many" => self.many = v,
                b"other" => self.other = v,
                _ => {}
            }
        }
        Ok(())
    }
}

// Go: i18n/message.go:stringMap
fn string_map(v: &Raw) -> Result<BTreeMap<String, String>, MessageError> {
    match v {
        Raw::String(value) => {
            let mut m = BTreeMap::new();
            m.insert("other".to_string(), value.clone());
            Ok(m)
        }
        Raw::StrMap(value) => {
            let mut strdata = BTreeMap::new();
            for (k, v) in value {
                string_submap(k, v, &mut strdata)?;
            }
            Ok(strdata)
        }
        Raw::IfaceMap(value) => {
            let mut strdata = BTreeMap::new();
            for (k, v) in value {
                let Raw::String(kstr) = k else {
                    // Go: keyTypeErr
                    return Err(format!(
                        "expected key to be a string but got {}",
                        k.go_sharp_v()
                    ));
                };
                string_submap(kstr, v, &mut strdata)?;
            }
            Ok(strdata)
        }
        // Go: valueTypeErr
        _ => Err(format!("unsupported type {}", v.go_sharp_v())),
    }
}

// Go: i18n/message.go:stringSubmap
fn string_submap(
    k: &str,
    v: &Raw,
    strdata: &mut BTreeMap<String, String>,
) -> Result<(), MessageError> {
    if k == "translation" {
        match v {
            Raw::String(vt) => {
                strdata.insert("other".to_string(), vt.clone());
            }
            _ => {
                let v1_message = string_map(v)?;
                for (kk, vv) in v1_message {
                    strdata.insert(kk, vv);
                }
            }
        }
        return Ok(());
    }

    match v {
        Raw::String(vt) => {
            strdata.insert(k.to_string(), vt.clone());
            Ok(())
        }
        Raw::Nil => Ok(()),
        _ => Err(format!(
            "expected value for key {} be a string but got {}",
            go_strconv::quote(k),
            v.go_sharp_v()
        )),
    }
}

/// Go: `isMessage(v)` — whether the given data is a message, or a map containing nested
/// messages. A map is a message if it contains any of the "reserved" keys (exact case) with a
/// string value.
// Go: i18n/message.go:isMessage
pub(crate) fn is_message(v: &Raw) -> bool {
    const RESERVED_KEYS: [&str; 11] = [
        "id",
        "description",
        "hash",
        "leftdelim",
        "rightdelim",
        "zero",
        "one",
        "two",
        "few",
        "many",
        "other",
    ];
    match v {
        Raw::String(_) => true,
        Raw::StrMap(data) => {
            for key in RESERVED_KEYS {
                // v is a message if it contains a "reserved" key holding a string value
                if let Some((_, Raw::String(_))) = data.iter().find(|(k, _)| k == key) {
                    return true;
                }
            }
            false
        }
        Raw::IfaceMap(data) => {
            for key in RESERVED_KEYS {
                if let Some((_, Raw::String(_))) = data
                    .iter()
                    .find(|(k, _)| matches!(k, Raw::String(s) if s == key))
                {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Go: `i18n.MessageTemplate` — an executable template for a message (one template per plural
/// form).
#[derive(Clone, Debug)]
pub struct MessageTemplate {
    pub message: Message,
    pub plural_templates: BTreeMap<Form, Template>,
}

impl MessageTemplate {
    /// Go: `NewMessageTemplate(m)` — `None` when the message has no plural form at all (Go
    /// returns a nil `*MessageTemplate`).
    // Go: i18n/message_template.go:NewMessageTemplate
    pub fn new(m: Message) -> Option<MessageTemplate> {
        let mut plural_templates = BTreeMap::new();
        set_plural_template(&mut plural_templates, Form::Zero, &m.zero, &m);
        set_plural_template(&mut plural_templates, Form::One, &m.one, &m);
        set_plural_template(&mut plural_templates, Form::Two, &m.two, &m);
        set_plural_template(&mut plural_templates, Form::Few, &m.few, &m);
        set_plural_template(&mut plural_templates, Form::Many, &m.many, &m);
        set_plural_template(&mut plural_templates, Form::Other, &m.other, &m);
        if plural_templates.is_empty() {
            return None;
        }
        Some(MessageTemplate {
            message: m,
            plural_templates,
        })
    }

    /// Go: `Execute(pluralForm, data, funcs)` (funcs is always nil in Hugo) — executes the
    /// template for the plural form and template data.
    // Go: i18n/message_template.go:Execute
    pub fn execute(
        &self,
        ctx: HostCtx<'_>,
        plural_form: Form,
        data: &go_value::Value,
    ) -> Result<String, ExecuteError> {
        let Some(t) = self.plural_templates.get(&plural_form) else {
            return Err(ExecuteError::PluralFormNotFound {
                plural_form,
                message_id: self.message.id.clone(),
            });
        };
        t.execute_ctx(ctx, data).map_err(ExecuteError::Template)
    }
}

// Go: i18n/message_template.go:setPluralTemplate
fn set_plural_template(
    plural_templates: &mut BTreeMap<Form, Template>,
    plural_form: Form,
    src: &str,
    m: &Message,
) {
    if !src.is_empty() {
        plural_templates.insert(
            plural_form,
            Template::new(src, &m.left_delim, &m.right_delim),
        );
    }
}

/// The errors of `MessageTemplate.Execute`.
#[derive(Clone, Debug, PartialEq)]
pub enum ExecuteError {
    /// Go: `pluralFormNotFoundError`.
    PluralFormNotFound {
        plural_form: Form,
        message_id: String,
    },
    /// A text/template parse or exec error (Go's error text).
    Template(String),
}

impl std::fmt::Display for ExecuteError {
    // Go: i18n/message_template.go:(pluralFormNotFoundError).Error
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecuteError::PluralFormNotFound {
                plural_form,
                message_id,
            } => write!(
                f,
                "message {} has no plural form {}",
                go_strconv::quote(message_id),
                go_strconv::quote(plural_form.as_str())
            ),
            ExecuteError::Template(s) => f.write_str(s),
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (go-i18n i18n/message.go, i18n/message_template.go)
// OK message.go NewMessage
//    message.go MustNewMessage (panicking helper; not needed)
// OK message.go unmarshalInterface
// OK message.go keyTypeErr.Error, valueTypeErr.Error (inline format! in string_map)
// OK message.go stringMap
// OK message.go stringSubmap
// OK message.go isMessage
// OK message_template.go NewMessageTemplate
// OK message_template.go setPluralTemplate
// OK message_template.go pluralFormNotFoundError.Error
// OK message_template.go Execute
// ---------------------------------------------------------------------------
