//! Encoding: `jsonify`, `remarshal`, `dump`, hashes.

use std::fmt::Write as _;

use md5::Digest as _;
use neohugo_base::Value as Data;
use tera::{Kwargs, TeraResult, Value};

use super::Registrar;
use super::value::{entries, text};

pub(super) fn register(r: &mut Registrar<'_>) {
    r.filter("jsonify", |v, kw, _| {
        let indent = kw.get::<&str>("indent")?;
        let json = Json {
            indent,
            html_safe: true,
        };
        Ok(Value::safe_string(&json.encode(&v)?))
    });
    r.filter("dump", |v, _, _| {
        let json = Json {
            indent: Some("  "),
            html_safe: false,
        };
        Ok(Value::from(json.encode(&v)?))
    });
    r.filter("remarshal", |v, kw, _| remarshal(&v, kw));
    r.filter("md5", |v, _, _| {
        Ok(Value::from(hex(&md5::Md5::digest(
            text(&v, "md5")?.as_bytes(),
        ))))
    });
    r.filter("sha1", |v, _, _| {
        Ok(Value::from(hex(&sha1::Sha1::digest(
            text(&v, "sha1")?.as_bytes(),
        ))))
    });
    r.filter("sha256", |v, _, _| {
        Ok(Value::from(hex(&sha2::Sha256::digest(
            text(&v, "sha256")?.as_bytes(),
        ))))
    });
    r.filter("fnv32a", |v, _, _| {
        Ok(Value::from(fnv32a(text(&v, "fnv32a")?.as_bytes())))
    });
    r.filter("xxhash", |v, _, _| {
        let h = xxhash_rust::xxh64::xxh64(text(&v, "xxhash")?.as_bytes(), 0);
        Ok(Value::from(format!("{h:016x}")))
    });
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// FNV-1a, 32 bits.
fn fnv32a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ u32::from(*b)).wrapping_mul(0x0100_0193)
    })
}

/// A JSON writer with sorted keys and Go's number and escaping rules.
struct Json<'a> {
    /// Pretty-print with this indentation.
    indent: Option<&'a str>,
    /// Escape `<`, `>` and `&` (`<` …) so the output is safe in HTML and `<script>`.
    html_safe: bool,
}

impl Json<'_> {
    fn encode(&self, v: &Value) -> TeraResult<String> {
        let mut out = String::new();
        self.value(v, 0, &mut out)?;
        Ok(out)
    }

    fn newline(&self, depth: usize, out: &mut String) {
        if let Some(indent) = self.indent {
            out.push('\n');
            for _ in 0..depth {
                out.push_str(indent);
            }
        }
    }

    fn value(&self, v: &Value, depth: usize, out: &mut String) -> TeraResult<()> {
        if v.is_none() || v.is_undefined() {
            out.push_str("null");
        } else if let Some(b) = v.as_bool() {
            out.push_str(if b { "true" } else { "false" });
        } else if v.is_f64() {
            out.push_str(&go_float(v.as_f64().unwrap_or_default())?);
        } else if v.is_number() {
            out.push_str(&v.to_string());
        } else if let Some(s) = v.as_str() {
            self.string(s, out);
        } else if let Some(a) = v.as_array() {
            if a.is_empty() {
                out.push_str("[]");
                return Ok(());
            }
            out.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                self.newline(depth + 1, out);
                self.value(item, depth + 1, out)?;
            }
            self.newline(depth, out);
            out.push(']');
        } else if let Some(m) = v.as_map() {
            if m.is_empty() {
                out.push_str("{}");
                return Ok(());
            }
            let mut pairs: Vec<_> = entries(m).collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            out.push('{');
            for (i, (k, item)) in pairs.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                self.newline(depth + 1, out);
                self.string(&k, out);
                out.push(':');
                if self.indent.is_some() {
                    out.push(' ');
                }
                self.value(item, depth + 1, out)?;
            }
            self.newline(depth, out);
            out.push('}');
        } else if let Some(b) = v.as_bytes() {
            self.string(&String::from_utf8_lossy(b), out);
        }
        Ok(())
    }

    fn string(&self, s: &str, out: &mut String) {
        out.push('"');
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{8}' => out.push_str("\\b"),
                '\u{c}' => out.push_str("\\f"),
                '<' | '>' | '&' if self.html_safe => {
                    let _ = write!(out, "\\u{:04x}", u32::from(c));
                }
                '\u{2028}' | '\u{2029}' => {
                    let _ = write!(out, "\\u{:04x}", u32::from(c));
                }
                c if u32::from(c) < 0x20 => {
                    let _ = write!(out, "\\u{:04x}", u32::from(c));
                }
                c => out.push(c),
            }
        }
        out.push('"');
    }
}

/// A float as Go's `encoding/json` writes it: shortest digits; exponent form below `1e-6` and
/// from `1e21` (`1e-7`, `1e+21`).
fn go_float(f: f64) -> TeraResult<String> {
    if !f.is_finite() {
        return Err(tera::Error::message(format!(
            "{f} cannot be written as JSON"
        )));
    }
    let abs = f.abs();
    if abs != 0.0 && !(1e-6..1e21).contains(&abs) {
        let e = format!("{f:e}");
        return Ok(match e.split_once('e') {
            Some((m, exp)) if !exp.starts_with('-') => format!("{m}e+{exp}"),
            _ => e,
        });
    }
    Ok(format!("{f}"))
}

/// A data format of `remarshal` and `unmarshal`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Json,
    Toml,
    Yaml,
}

impl Format {
    fn parse(s: &str) -> TeraResult<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "toml" => Ok(Self::Toml),
            "yaml" | "yml" => Ok(Self::Yaml),
            other => Err(tera::Error::message(format!(
                "remarshal(format=): unknown format `{other}`; expected json, toml or yaml"
            ))),
        }
    }

    /// The format of a document, from the first of `{`/`[` (JSON), `:` (YAML) and `=` (TOML).
    fn detect(s: &str) -> Option<Self> {
        let first = |c: char| s.find(c).unwrap_or(usize::MAX);
        let json = first('{').min(if s.trim_start().starts_with('[') {
            first('[')
        } else {
            usize::MAX
        });
        let yaml = first(':');
        let toml = first('=');
        let min = json.min(yaml).min(toml);
        if min == usize::MAX {
            None
        } else if min == json {
            Some(Self::Json)
        } else if min == toml {
            Some(Self::Toml)
        } else {
            Some(Self::Yaml)
        }
    }
}

/// Re-encodes data (a map, or a JSON/TOML/YAML document) as `format`.
fn remarshal(v: &Value, kw: &Kwargs) -> TeraResult<Value> {
    let format = Format::parse(kw.must_get::<&str>("format")?)?;
    let data: Value = if v.is_none() || v.is_undefined() {
        return Ok(Value::from(""));
    } else if let Some(s) = v.as_str() {
        if s.trim().is_empty() {
            return Ok(Value::from(""));
        }
        let from = Format::detect(s).ok_or_else(|| {
            tera::Error::message("remarshal: cannot detect the format of the input")
        })?;
        let decoded = match from {
            Format::Json => Data::from_json_str(s).map_err(|e| e.to_string()),
            Format::Toml => Data::from_toml_str(s).map_err(|e| e.to_string()),
            Format::Yaml => Data::from_yaml_str(s).map_err(|e| e.to_string()),
        }
        .map_err(|e| tera::Error::message(format!("remarshal: {e}")))?;
        decoded.to_tera()
    } else if v.is_map() || v.is_array() {
        v.clone()
    } else {
        return Err(tera::Error::message(format!(
            "remarshal: cannot detect the format of a {}",
            v.name()
        )));
    };
    let out = match format {
        Format::Json => {
            let mut s = Json {
                indent: Some("   "),
                html_safe: false,
            }
            .encode(&data)?;
            s.push('\n');
            s
        }
        Format::Yaml => {
            let mut s = String::new();
            yaml(&data, 0, &mut s);
            s
        }
        Format::Toml => toml(&data)?,
    };
    Ok(Value::from(out))
}

fn sorted_entries(v: &Value) -> Vec<(String, &Value)> {
    let mut pairs: Vec<(String, &Value)> = v
        .as_map()
        .map(|m| entries(m).map(|(k, v)| (k.into_owned(), v)).collect())
        .unwrap_or_default();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

/// A YAML scalar.
fn yaml_scalar(v: &Value) -> String {
    if v.is_none() || v.is_undefined() {
        return "null".to_owned();
    }
    if let Some(s) = v.as_str() {
        let plain = !s.is_empty()
            && !s.starts_with([
                ' ', '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'',
                '"', '%', '@', '`',
            ])
            && !s.ends_with(' ')
            && !s.contains(": ")
            && !s.contains(" #")
            && !s.contains(['\n', '\t'])
            && !matches!(
                s.to_ascii_lowercase().as_str(),
                "true" | "false" | "yes" | "no" | "on" | "off" | "null" | "~" | "y" | "n"
            )
            && s.parse::<f64>().is_err();
        if plain {
            return s.to_owned();
        }
        let json = Json {
            indent: None,
            html_safe: false,
        };
        let mut out = String::new();
        json.string(s, &mut out);
        return out;
    }
    if v.is_f64() {
        return format!("{}", v.as_f64().unwrap_or_default());
    }
    v.to_string()
}

fn yaml(v: &Value, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    if v.is_map() {
        for (k, item) in sorted_entries(v) {
            let key = yaml_scalar(&Value::from(k.as_str()));
            if item.as_map().is_some_and(|m| !m.is_empty()) {
                let _ = writeln!(out, "{pad}{key}:");
                yaml(item, depth + 1, out);
            } else if item.as_array().is_some_and(|a| !a.is_empty()) {
                let _ = writeln!(out, "{pad}{key}:");
                yaml(item, depth, out);
            } else {
                let _ = writeln!(out, "{pad}{key}: {}", yaml_inline(item));
            }
        }
    } else if let Some(a) = v.as_array() {
        for item in a {
            if item.as_map().is_some_and(|m| !m.is_empty()) {
                let mut nested = String::new();
                yaml(item, depth + 1, &mut nested);
                let nested = nested.trim_start();
                let _ = write!(out, "{pad}- {nested}");
            } else if item.as_array().is_some_and(|a| !a.is_empty()) {
                let _ = writeln!(out, "{pad}-");
                yaml(item, depth + 1, out);
            } else {
                let _ = writeln!(out, "{pad}- {}", yaml_inline(item));
            }
        }
    } else {
        let _ = writeln!(out, "{pad}{}", yaml_scalar(v));
    }
}

fn yaml_inline(v: &Value) -> String {
    if v.is_map() {
        "{}".to_owned()
    } else if v.is_array() {
        "[]".to_owned()
    } else {
        yaml_scalar(v)
    }
}

/// A TOML key, bare when it can be.
fn toml_key(k: &str) -> String {
    if !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        k.to_owned()
    } else {
        let mut out = String::new();
        Json {
            indent: None,
            html_safe: false,
        }
        .string(k, &mut out);
        out
    }
}

fn toml_inline(v: &Value) -> TeraResult<String> {
    if v.is_none() || v.is_undefined() {
        return Err(tera::Error::message("remarshal: TOML has no null"));
    }
    if let Some(s) = v.as_str() {
        let mut out = String::new();
        Json {
            indent: None,
            html_safe: false,
        }
        .string(s, &mut out);
        return Ok(out);
    }
    if v.is_f64() {
        let f = v.as_f64().unwrap_or_default();
        return Ok(if f.fract() == 0.0 && f.abs() < 1e16 {
            format!("{f:.1}")
        } else {
            format!("{f}")
        });
    }
    if let Some(a) = v.as_array() {
        let items = a.iter().map(toml_inline).collect::<TeraResult<Vec<_>>>()?;
        return Ok(format!("[{}]", items.join(", ")));
    }
    if v.is_map() {
        let items = sorted_entries(v)
            .into_iter()
            .map(|(k, x)| Ok(format!("{} = {}", toml_key(&k), toml_inline(x)?)))
            .collect::<TeraResult<Vec<_>>>()?;
        return Ok(format!("{{{}}}", items.join(", ")));
    }
    Ok(v.to_string())
}

fn is_table_array(v: &Value) -> bool {
    v.as_array()
        .is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_map))
}

/// A TOML document: plain keys first, then tables (indented two spaces per level) and arrays of
/// tables.
fn toml(v: &Value) -> TeraResult<String> {
    if !v.is_map() {
        return Err(tera::Error::message(
            "remarshal: a TOML document is a table",
        ));
    }
    let mut out = String::new();
    toml_table(v, &[], &mut out)?;
    Ok(out)
}

fn toml_table(v: &Value, path: &[String], out: &mut String) -> TeraResult<()> {
    let pad = "  ".repeat(path.len());
    let pairs = sorted_entries(v);
    for (k, item) in &pairs {
        if item.is_map() || is_table_array(item) || item.is_none() {
            continue;
        }
        let _ = writeln!(out, "{pad}{} = {}", toml_key(k), toml_inline(item)?);
    }
    for (k, item) in &pairs {
        let mut sub = path.to_vec();
        sub.push(toml_key(k));
        let header_pad = "  ".repeat(path.len());
        if item.is_map() {
            let _ = write!(out, "\n{header_pad}[{}]\n", sub.join("."));
            toml_table(item, &sub, out)?;
        } else if is_table_array(item) {
            for t in item.as_array().unwrap_or_default() {
                let _ = write!(out, "\n{header_pad}[[{}]]\n", sub.join("."));
                toml_table(t, &sub, out)?;
            }
        }
    }
    Ok(())
}
