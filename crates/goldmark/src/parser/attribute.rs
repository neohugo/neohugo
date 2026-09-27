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

// Go: parser/attribute.go:ParseAttributes, parseAttribute, parseAttributeValue,
// parseAttributeArray
//
// goldmark recurses ParseAttributes → parseAttribute → parseAttributeValue →
// ParseAttributes / parseAttributeArray on growable goroutine stacks, so it
// parses e.g. `{a=[[[[…` 100k levels deep; a Rust thread stack would
// overflow. The four functions run here on an explicit stack of frames
// instead, making the same reader calls in the same order (PORTING.md,
// deviation 10).

/// A ParseAttributes or parseAttributeArray call in progress.
enum Frame {
    /// ParseAttributes: its saved reader position, the attributes so far and
    /// the name of the attribute whose value is being parsed.
    Object {
        saved_line: i64,
        saved_position: crate::text::Segment,
        attrs: Attributes,
        name: Vec<u8>,
    },
    /// parseAttributeArray: the values so far and the loop counter.
    Array { ret: Vec<AttrValue>, i: i64 },
}

/// Where the explicit-stack parser continues.
enum Step {
    /// Call ParseAttributes.
    EnterObject,
    /// The loop of ParseAttributes (the top frame is an Object).
    ObjectLoop,
    /// parseAttribute returned an attribute to the top Object frame.
    ObjectAttr(Attribute),
    /// parseAttribute returned false: ParseAttributes resets and fails.
    ObjectFail,
    /// Call parseAttributeValue.
    EnterValue,
    /// The loop of parseAttributeArray (the top frame is an Array).
    ArrayLoop,
    /// A ParseAttributes / parseAttributeValue call returns (None = false).
    Return(Option<AttrValue>),
}

/// parseAttribute up to its parseAttributeValue call.
enum AttributeHead {
    /// `#id` / `.class`: the whole attribute.
    Done(Attribute),
    /// `name=`: the value follows.
    Value(Vec<u8>),
    /// (Attribute{}, false)
    Fail,
}

/// ParseAttributes parses attributes into a map.
/// ParseAttributes returns a parsed attributes and true if could parse
/// attributes, otherwise nil and false.
pub fn parse_attributes<'a>(reader: &mut dyn Reader<'a>) -> Option<Attributes> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut step = Step::EnterObject;
    loop {
        step = match step {
            // Go: ParseAttributes (prologue)
            Step::EnterObject => {
                let (saved_line, saved_position) = reader.position();
                reader.skip_spaces();
                if reader.peek() != b'{' {
                    reader.set_position(saved_line, saved_position);
                    Step::Return(None)
                } else {
                    reader.advance(1);
                    stack.push(Frame::Object {
                        saved_line,
                        saved_position,
                        attrs: Vec::new(),
                        name: Vec::new(),
                    });
                    Step::ObjectLoop
                }
            }
            // Go: ParseAttributes (for loop)
            Step::ObjectLoop => {
                if reader.peek() == b'}' {
                    reader.advance(1);
                    let Some(Frame::Object { attrs, .. }) = stack.pop() else {
                        unreachable!()
                    };
                    Step::Return(Some(AttrValue::Attributes(attrs)))
                } else {
                    match parse_attribute_head(reader) {
                        AttributeHead::Done(attr) => Step::ObjectAttr(attr),
                        AttributeHead::Fail => Step::ObjectFail,
                        AttributeHead::Value(n) => {
                            let Some(Frame::Object { name, .. }) = stack.last_mut() else {
                                unreachable!()
                            };
                            *name = n;
                            Step::EnterValue
                        }
                    }
                }
            }
            Step::ObjectAttr(attr) => {
                let Some(Frame::Object { attrs, .. }) = stack.last_mut() else {
                    unreachable!()
                };
                if attr.name == ATTR_NAME_CLASS {
                    if !find_update(attrs, ATTR_NAME_CLASS, |v| {
                        let v = v.as_bytes().expect("interface conversion: not []byte");
                        let a = attr
                            .value
                            .as_bytes()
                            .expect("interface conversion: not []byte");
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
                Step::ObjectLoop
            }
            Step::ObjectFail => {
                let Some(Frame::Object {
                    saved_line,
                    saved_position,
                    ..
                }) = stack.pop()
                else {
                    unreachable!()
                };
                reader.set_position(saved_line, saved_position);
                Step::Return(None)
            }
            // Go: parseAttributeValue
            Step::EnterValue => {
                reader.skip_spaces();
                let c = reader.peek();
                match c {
                    // Go returns (Attribute{}, false) for EOF: the value is discarded.
                    EOF => Step::Return(None),
                    b'{' => Step::EnterObject,
                    b'[' => {
                        reader.advance(1); // skip [
                        stack.push(Frame::Array {
                            ret: Vec::new(),
                            i: 0,
                        });
                        Step::ArrayLoop
                    }
                    b'"' => Step::Return(parse_attribute_string(reader).map(AttrValue::Bytes)),
                    _ => {
                        if c == b'-' || c == b'+' || util::is_numeric(c) {
                            Step::Return(parse_attribute_number(reader).map(AttrValue::Float))
                        } else {
                            Step::Return(parse_attribute_others(reader))
                        }
                    }
                }
            }
            // Go: parseAttributeArray (for loop)
            Step::ArrayLoop => {
                let Some(Frame::Array { i, .. }) = stack.last() else {
                    unreachable!()
                };
                let c = reader.peek();
                let mut comma = false;
                if *i != 0 && c == b',' {
                    reader.advance(1);
                    comma = true;
                }
                if c == b']' {
                    let Some(Frame::Array { ret, .. }) = stack.pop() else {
                        unreachable!()
                    };
                    if !comma {
                        reader.advance(1);
                        Step::Return(Some(AttrValue::Array(ret)))
                    } else {
                        Step::Return(None)
                    }
                } else {
                    reader.skip_spaces();
                    Step::EnterValue
                }
            }
            Step::Return(value) => match stack.last_mut() {
                None => {
                    return match value {
                        Some(mut v) => match &mut v {
                            AttrValue::Attributes(attrs) => Some(std::mem::take(attrs)),
                            _ => unreachable!(),
                        },
                        None => None,
                    };
                }
                // Go: parseAttribute (after parseAttributeValue)
                Some(Frame::Object { name, .. }) => match value {
                    None => Step::ObjectFail,
                    Some(value) => {
                        let name = std::mem::take(name);
                        if name == ATTR_NAME_CLASS && !matches!(value, AttrValue::Bytes(_)) {
                            Step::ObjectFail
                        } else {
                            Step::ObjectAttr(Attribute { name, value })
                        }
                    }
                },
                // Go: parseAttributeArray (after parseAttributeValue)
                Some(Frame::Array { ret, i }) => match value {
                    None => {
                        stack.pop();
                        Step::Return(None)
                    }
                    Some(value) => {
                        ret.push(value);
                        reader.skip_spaces();
                        *i += 1;
                        Step::ArrayLoop
                    }
                },
            },
        };
    }
}

// Go: parser/attribute.go:parseAttribute (up to the parseAttributeValue call)
fn parse_attribute_head<'a>(reader: &mut dyn Reader<'a>) -> AttributeHead {
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
        return AttributeHead::Done(Attribute {
            name: name.to_vec(),
            value: AttrValue::Bytes(line[0..i].to_vec()),
        });
    }
    let (line, _) = reader.peek_line();
    let line = line.unwrap_or_default();
    if line.is_empty() {
        return AttributeHead::Fail;
    }
    let c = line[0];
    if !(c.is_ascii_lowercase() || c.is_ascii_uppercase() || c == b'_' || c == b':') {
        return AttributeHead::Fail;
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
        return AttributeHead::Fail;
    }
    reader.advance(1);
    reader.skip_spaces();
    AttributeHead::Value(name)
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
