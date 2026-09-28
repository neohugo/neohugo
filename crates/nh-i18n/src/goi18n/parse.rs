//! Module `goi18n::parse`.
//!
//! PORT i18n/parse.go
//!
//! Owner: Wave B task T17 (i18n).

use go_value::{MapType, Value};
use nh_common::Result;
use xtext_collate::language::Tag;

use super::message::{Message, MessageError, is_message};

/// A decoded message file value: Go's `interface{}` as the unmarshalers Hugo registers produce
/// it (`toml` = pelletier/go-toml/v2, `yaml`/`yml` = gopkg.in/yaml.v2, `json` =
/// encoding/json). Maps keep their keys in byte order (Go iterates them randomly).
#[derive(Clone, Debug)]
pub enum Raw {
    /// Go `nil`.
    Nil,
    /// Go `string`.
    String(String),
    /// Go `map[string]interface{}` (TOML, JSON).
    StrMap(Vec<(String, Raw)>),
    /// Go `map[interface{}]interface{}` (yaml.v2).
    IfaceMap(Vec<(Raw, Raw)>),
    /// Go `[]interface{}`.
    Slice(Vec<Raw>),
    /// Any other value (numbers, bools, times), kept as a template value for `%#v`/`%T`.
    Other(Value),
}

impl Raw {
    /// From a decoded go-value (TOML, JSON).
    pub fn from_value(v: &Value) -> Raw {
        match v {
            Value::Invalid => Raw::Nil,
            Value::String(s) => Raw::String(s.to_str_lossy().into_owned()),
            Value::Map(m) if m.ty == MapType::StringAny => Raw::StrMap(
                m.entries
                    .iter()
                    .map(|(k, v)| (k.to_str_lossy().into_owned(), Raw::from_value(v)))
                    .collect(),
            ),
            Value::List(l) if l.ty == go_value::SliceType::Any => {
                Raw::Slice(l.items.iter().map(Raw::from_value).collect())
            }
            other => Raw::Other(other.clone()),
        }
    }

    /// From a yaml.v2 value.
    pub fn from_yaml(v: &go_yaml::Yaml) -> Raw {
        use go_yaml::Yaml;
        match v {
            Yaml::Nil => Raw::Nil,
            Yaml::Bool(b) => Raw::Other(Value::Bool(*b)),
            Yaml::Int(i) => Raw::Other(Value::int(*i)),
            Yaml::Uint64(u) => Raw::Other(Value::Uint(*u, go_value::UintKind::Uint64)),
            Yaml::Float64(f) => Raw::Other(Value::float64(*f)),
            Yaml::String(s) => Raw::String(String::from_utf8_lossy(s).into_owned()),
            Yaml::Seq(s) => Raw::Slice(s.iter().map(Raw::from_yaml).collect()),
            Yaml::Map(m) => Raw::IfaceMap(
                m.iter()
                    .map(|(k, v)| (Raw::from_yaml(k), Raw::from_yaml(v)))
                    .collect(),
            ),
        }
    }

    /// Back to a go-value, for Go's `%#v`/`%T` in error messages. A yaml map becomes a
    /// `map[interface {}]interface {}` with its keys printed with `%v`.
    fn to_value(&self) -> Value {
        match self {
            Raw::Nil => Value::Invalid,
            Raw::String(s) => Value::string(s.as_str()),
            Raw::StrMap(m) => {
                let mut mm = go_value::Map::new(MapType::StringAny);
                for (k, v) in m {
                    mm.insert(k.as_str(), v.to_value());
                }
                Value::map(mm)
            }
            Raw::IfaceMap(m) => {
                let mut mm = go_value::Map::new(MapType::Named(std::sync::Arc::from(
                    "map[interface {}]interface {}",
                )));
                for (k, v) in m {
                    let ks = go_fmt::sprint(&[k.to_value()]);
                    mm.insert(ks, v.to_value());
                }
                Value::map(mm)
            }
            Raw::Slice(s) => Value::any_list(s.iter().map(Raw::to_value).collect()),
            Raw::Other(v) => v.clone(),
        }
    }

    /// Go `fmt.Sprintf("%#v", v)`.
    pub(crate) fn go_sharp_v(&self) -> String {
        String::from_utf8_lossy(&go_fmt::sprintf("%#v", &[self.to_value()])).into_owned()
    }

    /// Go `fmt.Sprintf("%T", v)`.
    fn go_type(&self) -> String {
        String::from_utf8_lossy(&go_fmt::sprintf("%T", &[self.to_value()])).into_owned()
    }
}

/// An error of `ParseMessageFileBytes`: Go's `Error()` text, and the position of a
/// `*toml.DecodeError` (Hugo's `herrors.NewFileErrorFromName` reads it from the error type).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Go `(*toml.DecodeError).Position()`: (line, column).
    pub toml_position: Option<(i64, i64)>,
}

impl From<String> for ParseError {
    fn from(message: String) -> Self {
        ParseError {
            message,
            toml_position: None,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Go: `i18n.MessageFile` — a parsed message file.
#[derive(Clone, Debug)]
pub struct MessageFile {
    pub path: String,
    pub tag: Tag,
    pub format: String,
    pub messages: Vec<Message>,
}

/// Go: `ParseMessageFileBytes(buf, path, unmarshalFuncs)` with Hugo's unmarshalers (`toml`,
/// `yaml`, `yml`, `json`). The error is Go's `Error()` text.
// Go: go-i18n i18n/parse.go:ParseMessageFileBytes
pub fn parse_message_file_bytes(
    buf: &[u8],
    path: &str,
) -> std::result::Result<MessageFile, ParseError> {
    let (lang, format) = parse_path(path);
    let tag = xtext_collate::language::make(lang);
    let mut message_file = MessageFile {
        path: path.to_string(),
        tag,
        format: format.to_string(),
        messages: Vec::new(),
    };
    if buf.is_empty() {
        return Ok(message_file);
    }
    let raw = match message_file.format.as_str() {
        "toml" => match nh_parser::metadecoders::toml::unmarshal(buf) {
            Ok(v) => Raw::from_value(&v),
            Err(e) => {
                return Err(ParseError {
                    message: e.message(),
                    toml_position: e.position(),
                });
            }
        },
        "yaml" | "yml" => match go_yaml::unmarshal(buf) {
            Ok(v) => Raw::from_yaml(&v),
            Err(e) => return Err(e.message().into()),
        },
        "json" => match go_json::unmarshal(buf) {
            Ok(v) => Raw::from_value(&v),
            Err(e) => return Err(e.to_string().into()),
        },
        f => return Err(format!("no unmarshaler registered for {f}").into()),
    };

    message_file.messages = rec_get_messages(&raw, is_message(&raw), true)?;
    Ok(message_file)
}

/// Go: the messages of a file as the skeleton exposed them (errors as `nh_common` errors).
// Go: go-i18n i18n/parse.go:recGetMessages
pub fn parse_messages(v: &Value) -> Result<Vec<Message>> {
    let raw = Raw::from_value(v);
    rec_get_messages(&raw, is_message(&raw), true).map_err(nh_common::herrors::Error::new)
}

const NESTED_SEPARATOR: &str = ".";

/// Go: `errInvalidTranslationFile`.
const ERR_INVALID_TRANSLATION_FILE: &str =
    "invalid translation file, expected key-values, got a single value";

// Go: go-i18n i18n/parse.go:recGetMessages
/// Looks for translation messages inside `raw`, scanning nested maps using recursion.
fn rec_get_messages(
    raw: &Raw,
    is_map_message: bool,
    is_initial_call: bool,
) -> std::result::Result<Vec<Message>, MessageError> {
    let mut messages: Vec<Message>;
    match raw {
        Raw::String(_) => {
            if is_initial_call {
                return Err(ERR_INVALID_TRANSLATION_FILE.to_string());
            }
            let m = Message::new(raw)?;
            return Ok(vec![m]);
        }
        Raw::StrMap(data) => {
            if is_map_message {
                let m = Message::new(raw)?;
                return Ok(vec![m]);
            }
            messages = Vec::with_capacity(data.len());
            for (id, data) in data {
                // recursively scan map items
                messages = add_child_messages(id, data, messages)?;
            }
        }
        Raw::IfaceMap(data) => {
            if is_map_message {
                let m = Message::new(raw)?;
                return Ok(vec![m]);
            }
            messages = Vec::with_capacity(data.len());
            for (id, data) in data {
                let Raw::String(strid) = id else {
                    return Err(format!(
                        "expected key to be string but got {}",
                        id.go_sharp_v()
                    ));
                };
                // recursively scan map items
                messages = add_child_messages(strid, data, messages)?;
            }
        }
        Raw::Slice(data) => {
            // Backward compatibility for v1 file format.
            messages = Vec::with_capacity(data.len());
            for data in data {
                // recursively scan slice items
                let child_messages = rec_get_messages(data, is_message(data), false)?;
                messages.extend(child_messages);
            }
        }
        other => {
            return Err(format!("unsupported file format {}", other.go_type()));
        }
    }
    Ok(messages)
}

// Go: go-i18n i18n/parse.go:addChildMessages
fn add_child_messages(
    id: &str,
    data: &Raw,
    mut messages: Vec<Message>,
) -> std::result::Result<Vec<Message>, MessageError> {
    let is_child_message = is_message(data);
    let child_messages = rec_get_messages(data, is_child_message, false)?;
    for mut m in child_messages {
        if is_child_message {
            if m.id.is_empty() {
                m.id = id.to_string(); // start with innermost key
            }
        } else {
            m.id = format!("{id}{NESTED_SEPARATOR}{}", m.id); // update ID with each nested key on the way
        }
        messages.push(m);
    }
    Ok(messages)
}

/// Go: `parsePath(path)` -> (langTag, format): the format is everything after the last ".";
/// the language tag is everything after the second to last "." or after the last path
/// separator, but before the format.
// Go: go-i18n i18n/parse.go:parsePath
pub fn parse_path(path: &str) -> (&str, &str) {
    let b = path.as_bytes();
    let mut lang_tag = "";
    let mut format = "";
    let mut format_start_idx: isize = -1;
    let mut i = b.len() as isize - 1;
    while i >= 0 {
        let c = b[i as usize];
        if c == b'/' {
            // os.IsPathSeparator (unix)
            if format_start_idx != -1 {
                lang_tag = &path[(i + 1) as usize..format_start_idx as usize];
            }
            return (lang_tag, format);
        }
        if c == b'.' {
            if format_start_idx != -1 {
                lang_tag = &path[(i + 1) as usize..format_start_idx as usize];
                return (lang_tag, format);
            }
            if format_start_idx == -1 {
                format = &path[(i + 1) as usize..];
                format_start_idx = i;
            }
        }
        i -= 1;
    }
    if format_start_idx != -1 {
        lang_tag = &path[..format_start_idx as usize];
    }
    (lang_tag, format)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (go-i18n i18n/parse.go)
// OK ParseMessageFileBytes (Hugo's four unmarshalers)
// OK recGetMessages
// OK addChildMessages
// OK parsePath
// ---------------------------------------------------------------------------
