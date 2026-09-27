// Go: github.com/yuin/goldmark@v1.7.12/parser/attribute.go

use crate::ast::AttrValue;
use crate::text::{EOF, Reader};
use crate::util;

pub(crate) const ATTR_NAME_ID: &[u8] = b"id";
pub(crate) const ATTR_NAME_CLASS: &[u8] = b"class";

/// An Attribute is an attribute of the markdown elements.
pub type Attribute = crate::ast::Attribute;

/// An Attributes is a collection of attributes.
pub type Attributes = Vec<Attribute>;

// Go: parser/attribute.go:Attributes.Find
/// Find returns a (value, true) if an attribute correspond with given name is found, otherwise (nil, false).
pub fn find<'x>(attrs: &'x [Attribute], name: &[u8]) -> Option<&'x AttrValue> {
    attrs.iter().find(|a| a.name == name).map(|a| &a.value)
}

// Go: parser/attribute.go:Attributes.findUpdate
fn find_update(
    attrs: &mut [Attribute],
    name: &[u8],
    cb: impl FnOnce(&AttrValue) -> AttrValue,
) -> bool {
    for a in attrs.iter_mut() {
        if a.name == name {
            a.value = cb(&a.value);
            return true;
        }
    }
    false
}

// Go: parser/attribute.go:ParseAttributes
/// ParseAttributes parses attributes into a map.
/// ParseAttributes returns a parsed attributes and true if could parse
/// attributes, otherwise nil and false.
pub fn parse_attributes<'a>(reader: &mut dyn Reader<'a>) -> Option<Attributes> {
    let (saved_line, saved_position) = reader.position();
    reader.skip_spaces();
    if reader.peek() != b'{' {
        reader.set_position(saved_line, saved_position);
        return None;
    }
    reader.advance(1);
    let mut attrs: Attributes = Vec::new();
    loop {
        if reader.peek() == b'}' {
            reader.advance(1);
            return Some(attrs);
        }
        let Some(attr) = parse_attribute(reader) else {
            reader.set_position(saved_line, saved_position);
            return None;
        };
        if attr.name == ATTR_NAME_CLASS {
            let v2 = attr.value.clone();
            if !find_update(&mut attrs, ATTR_NAME_CLASS, |v| {
                let v = v.as_bytes().expect("interface conversion: not []byte");
                let a = v2.as_bytes().expect("interface conversion: not []byte");
                let mut ret = Vec::with_capacity(v.len() + 1 + a.len());
                ret.extend_from_slice(v);
                ret.push(b' ');
                ret.extend_from_slice(a);
                AttrValue::Bytes(ret)
            }) {
                attrs.push(attr);
            }
        } else {
            attrs.push(attr);
        }
        reader.skip_spaces();
        if reader.peek() == b',' {
            reader.advance(1);
            reader.skip_spaces();
        }
    }
}

// Go: parser/attribute.go:parseAttribute
fn parse_attribute<'a>(reader: &mut dyn Reader<'a>) -> Option<Attribute> {
    reader.skip_spaces();
    let c = reader.peek();
    if c == b'#' || c == b'.' {
        reader.advance(1);
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let mut i = 0;
        // HTML5 allows any kind of characters as id, but XHTML restricts characters for id.
        // CommonMark is basically defined for XHTML(even though it is legacy).
        // So we restrict id characters.
        while i < line.len()
            && !util::is_space(line[i])
            && (!util::is_punct(line[i])
                || line[i] == b'_'
                || line[i] == b'-'
                || line[i] == b':'
                || line[i] == b'.')
        {
            i += 1;
        }
        let mut name = ATTR_NAME_CLASS;
        if c == b'#' {
            name = ATTR_NAME_ID;
        }
        reader.advance(i as i64);
        return Some(Attribute {
            name: name.to_vec(),
            value: AttrValue::Bytes(line[0..i].to_vec()),
        });
    }
    let (line, _) = reader.peek_line();
    let line = line.unwrap_or_default();
    if line.is_empty() {
        return None;
    }
    let c = line[0];
    if !(c.is_ascii_lowercase() || c.is_ascii_uppercase() || c == b'_' || c == b':') {
        return None;
    }
    let mut i = 0;
    while i < line.len() {
        let c = line[i];
        if !(c.is_ascii_lowercase()
            || c.is_ascii_uppercase()
            || c.is_ascii_digit()
            || c == b'_'
            || c == b':'
            || c == b'.'
            || c == b'-')
        {
            break;
        }
        i += 1;
    }
    let name = line[..i].to_vec();
    reader.advance(i as i64);
    reader.skip_spaces();
    let c = reader.peek();
    if c != b'=' {
        return None;
    }
    reader.advance(1);
    reader.skip_spaces();
    let value = parse_attribute_value(reader)?;
    if name == ATTR_NAME_CLASS && !matches!(value, AttrValue::Bytes(_)) {
        return None;
    }
    Some(Attribute { name, value })
}

// Go: parser/attribute.go:parseAttributeValue
fn parse_attribute_value<'a>(reader: &mut dyn Reader<'a>) -> Option<AttrValue> {
    reader.skip_spaces();
    let c = reader.peek();
    // Go returns (Attribute{}, false) for EOF: the value is discarded.
    let value = match c {
        EOF => return None,
        b'{' => parse_attributes(reader).map(AttrValue::Attributes),
        b'[' => parse_attribute_array(reader).map(AttrValue::Array),
        b'"' => parse_attribute_string(reader).map(AttrValue::Bytes),
        _ => {
            if c == b'-' || c == b'+' || util::is_numeric(c) {
                parse_attribute_number(reader).map(AttrValue::Float)
            } else {
                parse_attribute_others(reader)
            }
        }
    };
    value
}

// Go: parser/attribute.go:parseAttributeArray
fn parse_attribute_array<'a>(reader: &mut dyn Reader<'a>) -> Option<Vec<AttrValue>> {
    reader.advance(1); // skip [
    let mut ret: Vec<AttrValue> = Vec::new();
    let mut i = 0;
    loop {
        let c = reader.peek();
        let mut comma = false;
        if i != 0 && c == b',' {
            reader.advance(1);
            comma = true;
        }
        if c == b']' {
            if !comma {
                reader.advance(1);
                return Some(ret);
            }
            return None;
        }
        reader.skip_spaces();
        let value = parse_attribute_value(reader)?;
        ret.push(value);
        reader.skip_spaces();
        i += 1;
    }
}

// Go: parser/attribute.go:parseAttributeString
fn parse_attribute_string<'a>(reader: &mut dyn Reader<'a>) -> Option<Vec<u8>> {
    reader.advance(1); // skip "
    let (line, _) = reader.peek_line();
    let line = line.unwrap_or_default();
    let mut i = 0;
    let l = line.len();
    let mut buf: Vec<u8> = Vec::new();
    while i < l {
        let c = line[i];
        if c == b'\\' && i != l - 1 {
            let n = line[i + 1];
            match n {
                b'"' | b'/' | b'\\' => {
                    buf.push(n);
                    i += 2;
                }
                b'b' => {
                    buf.push(0x08);
                    i += 2;
                }
                b'f' => {
                    buf.push(0x0c);
                    i += 2;
                }
                b'n' => {
                    buf.push(b'\n');
                    i += 2;
                }
                b'r' => {
                    buf.push(b'\r');
                    i += 2;
                }
                b't' => {
                    buf.push(b'\t');
                    i += 2;
                }
                _ => {
                    buf.push(b'\\');
                    i += 1;
                }
            }
            continue;
        }
        if c == b'"' {
            reader.advance((i + 1) as i64);
            return Some(buf);
        }
        buf.push(c);
        i += 1;
    }
    None
}

// Go: parser/attribute.go:scanAttributeDecimal
fn scan_attribute_decimal<'a>(reader: &mut dyn Reader<'a>, w: &mut Vec<u8>) {
    loop {
        let c = reader.peek();
        if util::is_numeric(c) {
            w.push(c);
        } else {
            return;
        }
        reader.advance(1);
    }
}

// Go: parser/attribute.go:parseAttributeNumber
fn parse_attribute_number<'a>(reader: &mut dyn Reader<'a>) -> Option<f64> {
    let mut sign = 1.0;
    let c = reader.peek();
    if c == b'-' {
        sign = -1.0;
        reader.advance(1);
    } else if c == b'+' {
        reader.advance(1);
    }
    let mut buf: Vec<u8> = Vec::new();
    if !util::is_numeric(reader.peek()) {
        return None;
    }
    scan_attribute_decimal(reader, &mut buf);
    if buf.is_empty() {
        return None;
    }
    let c = reader.peek();
    if c == b'.' {
        buf.push(c);
        reader.advance(1);
        scan_attribute_decimal(reader, &mut buf);
    }
    let c = reader.peek();
    if c == b'e' || c == b'E' {
        buf.push(c);
        reader.advance(1);
        let c = reader.peek();
        if c == b'-' || c == b'+' {
            buf.push(c);
            reader.advance(1);
        }
        scan_attribute_decimal(reader, &mut buf);
    }
    let f = go_strconv::parse_float(&buf, 64).ok()?;
    Some(sign * f)
}

// Go: parser/attribute.go:parseAttributeOthers
fn parse_attribute_others<'a>(reader: &mut dyn Reader<'a>) -> Option<AttrValue> {
    let (line, _) = reader.peek_line();
    let line = line.unwrap_or_default();
    // Go: line[0] panics on an empty line.
    let c = line[0];
    if !(c.is_ascii_lowercase() || c.is_ascii_uppercase() || c == b'_' || c == b':') {
        return None;
    }
    let mut i = 0;
    while i < line.len() {
        let c = line[i];
        if !(c.is_ascii_lowercase()
            || c.is_ascii_uppercase()
            || c.is_ascii_digit()
            || c == b'_'
            || c == b':'
            || c == b'.'
            || c == b'-')
        {
            break;
        }
        i += 1;
    }
    let value = line[..i].to_vec();
    reader.advance(i as i64);
    if value == b"true" {
        return Some(AttrValue::Bool(true));
    }
    if value == b"false" {
        return Some(AttrValue::Bool(false));
    }
    if value == b"null" {
        return Some(AttrValue::Nil);
    }
    Some(AttrValue::Bytes(value))
}
