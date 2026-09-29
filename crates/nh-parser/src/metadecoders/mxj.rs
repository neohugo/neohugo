//! Port of the subset of `github.com/clbanning/mxj/v2` v2.7.0 that neohugo uses, with mxj's
//! default settings (no key coercion, `-` attribute prefix, white space trimmed, no casting, no
//! escaping):
//!
//! - `NewMapXml(data)` (`xml.go`: `xmlToMap`, `xmlToMapParser`, `cast` with `r == false`) for
//!   `metadecoders.Decoder.UnmarshalTo(XML)`, and `Map.Root` (`misc.go`);
//! - `AnyXmlIndent(v, prefix, indent, rootTag)` for a `map[string]interface{}` (`anyxml.go`,
//!   `Map.XmlIndent` and `marshalMapToXmlIndent` in `xml.go`) for `parser.InterfaceToConfig(XML)`.
//!
//! Owner: gaps follow-up of Wave B task T03 (parser-langs).

use std::collections::BTreeMap;

use go_value::{IntKind, Map, MapType, SliceType, Value};

use super::xml::{self, Attr, Decoder, Token};

const TEXT_K: &[u8] = b"#text";
const ATTR_PREFIX: &[u8] = b"-";
const TRIM_RUNES: &[u8] = b"\t\r\x08\n ";

/// A decoded mxj value (`map[string]interface{}`, `[]interface{}` or `string`).
#[derive(Clone, Debug)]
enum XVal {
    Str(Vec<u8>),
    Map(BTreeMap<Vec<u8>, XVal>),
    List(Vec<XVal>),
}

impl XVal {
    fn into_value(self) -> Value {
        match self {
            XVal::Str(s) => Value::string(s),
            XVal::Map(m) => Value::map(to_map(m)),
            XVal::List(l) => Value::list(
                SliceType::Any,
                l.into_iter().map(XVal::into_value).collect(),
            ),
        }
    }
}

fn to_map(m: BTreeMap<Vec<u8>, XVal>) -> Map {
    let mut out = Map::new(MapType::StringAny);
    for (k, v) in m {
        out.entries.insert(k.into(), v.into_value());
    }
    out
}

/// A `NewMapXml` error: `io.EOF`, or the decoder's error with mxj's prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Eof,
    Token(Vec<u8>),
}

impl DecodeError {
    /// Go's `err.Error()` bytes.
    pub fn error_bytes(&self) -> Vec<u8> {
        match self {
            DecodeError::Eof => b"EOF".to_vec(),
            DecodeError::Token(m) => m.clone(),
        }
    }
}

/// Go: `mxj.NewMapXml(data)` followed by `Map.Root()`: the root element's name and value.
/// The value is `map[string]interface{}` or a `string` (an element without attributes or
/// children).
// Go: xml.go:NewMapXml, xml.go:xmlToMap, misc.go:(Map).Root
pub fn new_map_xml_root(data: &[u8]) -> Result<(Vec<u8>, Value), DecodeError> {
    let mut p = Decoder::new(data);
    let m = xml_to_map_parser(b"", &[], &mut p)?;
    // (Root: the map from xmlToMapParser("") has exactly one key.)
    let (k, v) = m.into_iter().next().expect("one root key");
    Ok((k, v.into_value()))
}

/// xmlToMapParser (2015.11.12) - load a 'clean' XML doc into a map[string]interface{} directly.
// Go: xml.go:xmlToMapParser
fn xml_to_map_parser(
    skey: &[u8],
    a: &[Attr],
    p: &mut Decoder<'_>,
) -> Result<BTreeMap<Vec<u8>, XVal>, DecodeError> {
    // NOTE: all attributes and sub-elements parsed into 'na', 'na' is returned as value for 'skey' in 'n'.
    // Unless 'skey' is a simple element w/o attributes, in which case the xml.CharData value is the value.
    let mut n: BTreeMap<Vec<u8>, XVal> = BTreeMap::new();
    let mut na: BTreeMap<Vec<u8>, XVal> = BTreeMap::new();

    // Allocate maps and load attributes, if any.
    if !skey.is_empty() {
        for v in a {
            let mut key = ATTR_PREFIX.to_vec();
            key.extend_from_slice(&v.name.local);
            na.insert(key, XVal::Str(v.value.clone()));
        }
    }

    loop {
        let t = match p.token() {
            Ok(t) => t,
            Err(xml::Error::Eof) => return Err(DecodeError::Eof),
            Err(e) => {
                let mut m = b"xml.Decoder.Token() - ".to_vec();
                m.extend_from_slice(&e.error_bytes());
                return Err(DecodeError::Token(m));
            }
        };
        match t {
            Token::StartElement { name, attr } => {
                // First call to xmlToMapParser() doesn't pass xml.StartElement - the map key.
                if skey.is_empty() {
                    return xml_to_map_parser(&name.local, &attr, p);
                }

                // If not initializing the map, parse the element.
                // len(nn) == 1, necessarily - it is just an 'n'.
                let nn = xml_to_map_parser(&name.local, &attr, p)?;

                let (key, val) = nn.into_iter().next().expect("one key");

                // 'na' holding sub-elements of n.
                // See if 'key' already exists.
                // If 'key' exists, then this is a list, if not just add key:val to na.
                match na.remove(&key) {
                    Some(v) => {
                        let mut a = match v {
                            XVal::List(a) => a,
                            other => vec![other],
                        };
                        a.push(val);
                        na.insert(key, XVal::List(a));
                    }
                    None => {
                        na.insert(key, val); // save it as a singleton
                    }
                }
            }
            Token::EndElement { .. } => {
                // len(n) > 0 if this is a simple element w/o xml.Attrs - see xml.CharData case.
                if n.is_empty() {
                    // If len(na)==0 we have an empty element == "";
                    // it has no xml.Attr nor xml.CharData.
                    if !na.is_empty() {
                        n.insert(skey.to_vec(), XVal::Map(na));
                    } else {
                        n.insert(skey.to_vec(), XVal::Str(Vec::new())); // empty element
                    }
                } else if n.len() == 1 && !na.is_empty() {
                    // it's a simple element w/ no attributes w/ subelements
                    let v = n.values().next().cloned().expect("one value");
                    na.insert(TEXT_K.to_vec(), v);
                    n.insert(skey.to_vec(), XVal::Map(na));
                }
                return Ok(n);
            }
            Token::CharData(data) => {
                // clean up possible noise
                let tt = trim(&data, TRIM_RUNES);
                if !tt.is_empty() {
                    if !na.is_empty() {
                        na.insert(TEXT_K.to_vec(), XVal::Str(tt.to_vec()));
                    } else if !skey.is_empty() {
                        n.insert(skey.to_vec(), XVal::Str(tt.to_vec()));
                    } else {
                        // stray text in the decoder stream
                        continue;
                    }
                }
            }
            _ => {
                // noop
            }
        }
    }
}

/// Go: `strings.Trim(s, cutset)` for an ASCII cutset.
fn trim<'s>(s: &'s [u8], cutset: &[u8]) -> &'s [u8] {
    let start = s
        .iter()
        .position(|c| !cutset.contains(c))
        .unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|c| !cutset.contains(c))
        .map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}

// ------------------------------------------------------------------------------------------
// Encoding.

/// An `AnyXmlIndent` error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// Go's error text.
    Go(String),
    /// A value the port does not encode (Go would reach `encoding/xml.Marshal` or a
    /// reflection-based path).
    Unsupported(String),
}

/// Go: `pretty`.
#[derive(Clone)]
struct Pretty {
    indent: Vec<u8>,
    cnt: i64,
    padding: Vec<u8>,
    map_depth: i64,
    start: i64,
}

impl Pretty {
    // Go: xml.go:(*pretty).Indent
    fn indent(&mut self) {
        self.padding.extend_from_slice(&self.indent.clone());
        self.cnt += 1;
    }

    // Go: xml.go:(*pretty).Outdent
    fn outdent(&mut self) {
        if self.cnt > 0 {
            let n = self.padding.len() - self.indent.len();
            self.padding.truncate(n);
            self.cnt -= 1;
        }
    }
}

/// Go: `mxj.AnyXmlIndent(v, prefix, indent, rootTag)` for a `map[string]interface{}` `v` (the
/// only input `parser.InterfaceToConfig` passes): `Map(v).XmlIndent(prefix, indent, rootTag)`.
// Go: anyxml.go:AnyXmlIndent, xml.go:(Map).XmlIndent
pub fn any_xml_indent_map(
    m: &Value,
    prefix: &[u8],
    indent: &[u8],
    root_tag: &[u8],
) -> Result<Vec<u8>, EncodeError> {
    let mut b = Vec::new();
    let p = Pretty {
        indent: indent.to_vec(),
        cnt: 0,
        padding: prefix.to_vec(),
        map_depth: 0,
        start: 0,
    };
    marshal_map_to_xml_indent(true, &mut b, root_tag, m, &p)?;
    Ok(b)
}

/// The Go type switch of `marshalMapToXmlIndent` over a normalised value.
enum XIn<'a> {
    /// `map[string]interface{}` (other maps are converted with `fmt.Sprint` keys, which are
    /// the string keys themselves).
    Map(&'a Map),
    /// `[]interface{}`.
    List(&'a [Value]),
    /// `[]string`.
    StrList(&'a [Value]),
    /// `string`, `[]byte`, and every value `fmt.Sprint` or `xml.Marshal` turns into one.
    Str(Vec<u8>),
    /// `float64, bool, int, int32, int64, float32`: `fmt.Sprintf("%v", v)`.
    Num(Vec<u8>),
    /// An empty `map[string]interface{}` (a nil map).
    EmptyMap,
}

/// `fmt.Sprintf("%v", v)` of a bool or number.
fn fmt_v_scalar(v: &Value) -> Option<Vec<u8>> {
    match v {
        Value::Bool(_) | Value::Int(..) | Value::Uint(..) | Value::Float(..) => {
            Some(go_fmt::sprintf("%v", std::slice::from_ref(v)))
        }
        _ => None,
    }
}

fn unsupported(v: &Value) -> EncodeError {
    EncodeError::Unsupported(format!(
        "neohugo-rs: XML encoding of {} is not supported",
        v.go_type_name()
    ))
}

/// `xml.Marshal` of a struct implementing `encoding.TextMarshaler` (`time.Time`, the go-toml
/// local types): `<TypeName>` + escaped text + `</TypeName>`.
fn xml_marshal_text(type_name: &str, text: &[u8]) -> Vec<u8> {
    let name = type_name.rsplit('.').next().unwrap_or(type_name);
    let mut b = Vec::new();
    b.push(b'<');
    b.extend_from_slice(name.as_bytes());
    b.push(b'>');
    xml::escape_text(&mut b, text);
    b.extend_from_slice(b"</");
    b.extend_from_slice(name.as_bytes());
    b.push(b'>');
    b
}

// Go: xml.go:marshalMapToXmlIndent (the value normalisation at its top)
fn classify(v: &Value) -> Result<XIn<'_>, EncodeError> {
    Ok(match v {
        Value::Map(m) => XIn::Map(m),
        Value::List(l) => match &l.ty {
            SliceType::Any => XIn::List(&l.items),
            SliceType::String => XIn::StrList(&l.items),
            SliceType::Uint8 => XIn::Str(
                l.items
                    .iter()
                    .map(|x| match x {
                        Value::Uint(u, _) => *u as u8,
                        _ => 0,
                    })
                    .collect(),
            ),
            _ => return Err(unsupported(v)),
        },
        Value::String(s) => XIn::Str(s.as_bytes().to_vec()),
        Value::Safe(_, s) => XIn::Str(s.as_bytes().to_vec()),
        Value::Invalid => XIn::Str(Vec::new()),
        Value::Bool(_) | Value::Float(..) => XIn::Num(fmt_v_scalar(v).unwrap()),
        Value::Int(_, IntKind::Int | IntKind::Int32 | IntKind::Int64) => {
            XIn::Num(fmt_v_scalar(v).unwrap())
        }
        // Other integer kinds: `fmt.Sprint(value)`, a string.
        Value::Int(..) | Value::Uint(..) => XIn::Str(fmt_v_scalar(v).unwrap()),
        Value::Time(t) => match go_time::GoTimeExt::marshal_text(t) {
            Ok(text) => XIn::Str(xml_marshal_text("time.Time", &text)),
            Err(e) => return Err(EncodeError::Go(e.to_string())),
        },
        Value::TypedNil(t) => match go_value::typed_nil_kind(t) {
            go_value::NilKind::Map => XIn::EmptyMap,
            go_value::NilKind::Slice if &**t == "[]interface {}" => XIn::List(&[]),
            go_value::NilKind::Slice if &**t == "[]string" => XIn::StrList(&[]),
            go_value::NilKind::Ptr => XIn::Str(b"<nil>".to_vec()),
            _ => return Err(unsupported(v)),
        },
        Value::Object(o) => {
            if o.kind() == go_value::Kind::Struct
                && let Some(text) = o.marshal_text()
            {
                match text {
                    Ok(text) => XIn::Str(xml_marshal_text(&o.type_name(), &text)),
                    Err(e) => return Err(EncodeError::Go(e.message().to_string())),
                }
            } else if let Some(u @ (Value::String(_) | Value::Bool(_))) = o.underlying() {
                // A named basic type: fmt.Sprint (its String method is not reached here for
                // the types that can occur).
                match u {
                    Value::String(s) => XIn::Str(s.as_bytes().to_vec()),
                    other => XIn::Str(fmt_v_scalar(&other).unwrap()),
                }
            } else {
                return Err(unsupported(v));
            }
        }
    })
}

fn attr_name(k: &[u8]) -> Option<&[u8]> {
    if !ATTR_PREFIX.is_empty()
        && ATTR_PREFIX.len() < k.len()
        && &k[..ATTR_PREFIX.len()] == ATTR_PREFIX
    {
        Some(&k[ATTR_PREFIX.len()..])
    } else {
        None
    }
}

/// `fmt.Sprintf("%v", v)` of a `#text` value (after the string/[]byte conversion).
fn text_value(v: &Value) -> Result<Vec<u8>, EncodeError> {
    match v {
        Value::String(s) | Value::Safe(_, s) => Ok(s.as_bytes().to_vec()),
        Value::List(l) if l.ty == SliceType::Uint8 => Ok(l
            .items
            .iter()
            .map(|x| match x {
                Value::Uint(u, _) => *u as u8,
                _ => 0,
            })
            .collect()),
        _ => Ok(go_fmt::sprintf("%v", std::slice::from_ref(v))),
    }
}

/// where the work actually happens
/// returns an error if an attribute is not atomic
// Go: xml.go:marshalMapToXmlIndent
fn marshal_map_to_xml_indent(
    do_indent: bool,
    b: &mut Vec<u8>,
    key: &[u8],
    value: &Value,
    pp: &Pretty,
) -> Result<(), EncodeError> {
    let mut end_tag = false;
    let mut is_simple = false;
    let mut elen: usize = 0;
    let mut p = pp.clone();

    let empty_map = Map::new(MapType::StringAny);
    let value = classify(value)?;
    let value = match value {
        XIn::EmptyMap => XIn::Map(&empty_map),
        other => other,
    };

    // start the XML tag with required indentaton and padding
    if do_indent {
        match value {
            XIn::List(_) | XIn::StrList(_) => {
                // list processing handles indentation for all elements
            }
            _ => b.extend_from_slice(&p.padding),
        }
    }
    match value {
        XIn::List(_) => {}
        _ => {
            b.push(b'<');
            b.extend_from_slice(key);
        }
    }

    match value {
        XIn::Map(vv) => 'map: {
            let lenvv = vv.entries.len();
            // scan out attributes - attribute keys have prepended attrPrefix
            let mut attrlist: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
            for (k, v) in &vv.entries {
                if let Some(name) = attr_name(k.as_bytes()) {
                    let ss = match v {
                        Value::String(s) => s.as_bytes().to_vec(),
                        Value::Float(..)
                        | Value::Bool(_)
                        | Value::Int(_, IntKind::Int | IntKind::Int32 | IntKind::Int64) => {
                            fmt_v_scalar(v).unwrap()
                        }
                        Value::List(l) if l.ty == SliceType::Uint8 => text_value(v)?,
                        _ => {
                            return Err(EncodeError::Go(format!(
                                "invalid attribute value for: {}:<{}>",
                                String::from_utf8_lossy(k.as_bytes()),
                                v.go_type_name()
                            )));
                        }
                    };
                    attrlist.push((name.to_vec(), ss));
                }
            }
            let n = attrlist.len();
            if n > 0 {
                // sort.Sort(attrList(attrlist)): the names are distinct.
                attrlist.sort_by(|a, b| a.0.cmp(&b.0));
                for v in &attrlist {
                    b.push(b' ');
                    b.extend_from_slice(&v.0);
                    b.extend_from_slice(b"=\"");
                    b.extend_from_slice(&v.1);
                    b.push(b'"');
                }
            }
            // only attributes?
            if n == lenvv {
                b.extend_from_slice(b"/>");
                break 'map;
            }

            // simple element? Note: '#text" is an invalid XML tag.
            let mut is_complex = false;
            let text = vv.entries.get(TEXT_K);
            if let Some(v) = text
                && n + 1 == lenvv
            {
                // just the value and attributes
                b.push(b'>');
                b.extend_from_slice(&text_value(v)?);
                end_tag = true;
                elen = 1;
                is_simple = true;
                break 'map;
            } else if let Some(v) = text {
                // need to handle when there are subelements in addition to the simple element value
                b.push(b'>');
                b.extend_from_slice(&text_value(v)?);
                is_complex = true;
            }

            // close tag with possible attributes
            if !is_complex {
                b.push(b'>');
            }
            if do_indent {
                b.push(b'\n');
            }
            // something more complex
            p.map_depth += 1;
            // extract the map k:v pairs and sort on key (map keys are distinct and already
            // in sorted byte order)
            for (k, v) in &vv.entries {
                if k.as_bytes() == TEXT_K || attr_name(k.as_bytes()).is_some() {
                    continue;
                }
                let is_list = matches!(v, Value::List(l) if l.ty == SliceType::Any)
                    || matches!(v, Value::TypedNil(t) if &**t == "[]interface {}");
                if !is_list && do_indent {
                    p.indent();
                }
                marshal_map_to_xml_indent(do_indent, b, k.as_bytes(), v, &p)?;
                if !is_list && do_indent {
                    p.outdent();
                }
            }
            p.map_depth -= 1;
            end_tag = true;
            elen = 1; // we do have some content ...
        }
        XIn::List(items) | XIn::StrList(items) => {
            // special case - found during implementing Issue #23
            if items.is_empty() {
                if do_indent {
                    b.extend_from_slice(&p.padding);
                    b.extend_from_slice(&p.indent);
                }
                b.push(b'<');
                b.extend_from_slice(key);
                elen = 0;
                end_tag = true;
            } else {
                for v in items {
                    if do_indent {
                        p.indent();
                    }
                    marshal_map_to_xml_indent(do_indent, b, key, v, &p)?;
                    if do_indent {
                        p.outdent();
                    }
                }
                return Ok(());
            }
        }
        XIn::Str(v) => {
            elen = v.len();
            if elen > 0 {
                b.push(b'>');
                b.extend_from_slice(&v);
            }
            is_simple = true;
            end_tag = true;
        }
        XIn::Num(v) => {
            elen = v.len(); // always > 0
            b.push(b'>');
            b.extend_from_slice(&v);
            is_simple = true;
            end_tag = true;
        }
        XIn::EmptyMap => unreachable!(),
    }
    if end_tag {
        if do_indent && !is_simple {
            b.extend_from_slice(&p.padding);
        }
        if elen > 0 {
            b.extend_from_slice(b"</");
            b.extend_from_slice(key);
            b.push(b'>');
        } else {
            b.extend_from_slice(b"/>");
        }
    }
    if do_indent {
        if p.cnt > p.start {
            b.push(b'\n');
        }
        p.outdent();
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (github.com/clbanning/mxj/v2@v2.7.0, the subset neohugo reaches)
// OK xml.go: NewMapXml, xmlToMap, xmlToMapParser, cast (r == false)
// OK misc.go: (Map).Root
// OK anyxml.go: AnyXmlIndent (map[string]interface{} input)
// OK xml.go: (Map).XmlIndent, (*pretty).Indent, (*pretty).Outdent, marshalMapToXmlIndent,
//    attrList/elemList sorting (distinct keys)
// ---------------------------------------------------------------------------
