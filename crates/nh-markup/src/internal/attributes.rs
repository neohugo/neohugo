//! Port of `markup/internal/attributes/attributes.go`.
//!
//! Owner: Wave B task T06 (markup).

use std::sync::{Arc, LazyLock, OnceLock};

use go_value::{GoString, List, Map, MapType, SliceType, Value};
use goldmark::ast::{AttrValue, Attribute as AstAttribute};
use goldmark::util::BufWriter;

/// Markdown attributes used as options by the Chroma highlighter (Go adds the lower-case form
/// of every key in `init`; only the lower-case forms are ever looked up).
static CHROMA_HIGHLIGHT_PROCESSING_ATTRIBUTES: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut v: Vec<String> = [
        "anchorLineNos",
        "guessSyntax",
        "hl_Lines",
        "hl_inline",
        "lineAnchors",
        "lineNos",
        "lineNoStart",
        "lineNumbersInTable",
        "noClasses",
        "style",
        "tabWidth",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    // Go: markup/internal/attributes/attributes.go:init
    let lower: Vec<String> = v.iter().map(|k| k.to_lowercase()).collect();
    v.extend(lower);
    v
});

fn is_chroma_attribute(name_lower: &[u8]) -> bool {
    CHROMA_HIGHLIGHT_PROCESSING_ATTRIBUTES
        .iter()
        .any(|k| k.as_bytes() == name_lower)
}

/// Go: `attributes.AttributesOwnerType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributesOwnerType {
    General,
    CodeBlockChroma,
    CodeBlockCustom,
}

/// Go: `attributes.Attribute`. The value is Go's `any`: a `bool`, `float64`, `string` or the
/// `[][2]int` highlight ranges.
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: GoString,
    pub value: Value,
}

impl Attribute {
    // Go: markup/internal/attributes/attributes.go:ValueString
    pub fn value_string(&self) -> GoString {
        nh_common::cast::caste::to_string(&self.value)
    }
}

/// The Go type name (`%T`) of an AST attribute value, for Go's panics.
pub(crate) fn attr_value_go_type(v: &AttrValue) -> &'static str {
    match v {
        AttrValue::Nil => "<nil>",
        AttrValue::Other(o) if o.is::<i64>() => "int",
        _ => v.go_type_name(),
    }
}

/// New creates a new AttributesHolder from the given AST attributes.
///
/// Go panics with `not implemented: %T` for values other than `bool`, `float64`, `[]any` and
/// `[]byte` (e.g. `{a=null}` or `{a={b=1}}`); so does the port.
// Go: markup/internal/attributes/attributes.go:New
pub fn new(ast_attributes: &[AstAttribute], owner_type: AttributesOwnerType) -> AttributesHolder {
    let mut attrs = Vec::new();
    let mut opts = Vec::new();
    for v in ast_attributes {
        let name_lower = go_unicode::strings::to_lower(&v.name).into_owned();
        if name_lower.starts_with(b"on") {
            continue;
        }
        let vv = match &v.value {
            AttrValue::Bool(b) => Value::Bool(*b),
            AttrValue::Float(f) => Value::float64(*f),
            AttrValue::Array(vvv) => {
                // Highlight line number hlRanges.
                let mut hl_ranges: Option<Vec<[i64; 2]>> = None;
                for l in vvv {
                    if let AttrValue::Float(ln) = l {
                        let ln = *ln as i64;
                        hl_ranges
                            .get_or_insert_with(Vec::new)
                            .push([ln.wrapping_sub(1), ln.wrapping_sub(1)]);
                    } else if let AttrValue::Bytes(rng) = l {
                        let slices: Vec<&[u8]> = rng.split(|&c| c == b'-').collect();
                        let Ok(lhs) = go_strconv::atoi(slices[0]) else {
                            continue;
                        };
                        let mut rhs = lhs;
                        if slices.len() > 1 {
                            match go_strconv::atoi(slices[1]) {
                                Ok(v) => rhs = v,
                                Err(_) => continue,
                            }
                        }
                        hl_ranges
                            .get_or_insert_with(Vec::new)
                            .push([lhs.wrapping_sub(1), rhs.wrapping_sub(1)]);
                    }
                }
                hl_ranges_value(hl_ranges.as_deref())
            }
            AttrValue::Bytes(b) => {
                // Note that we don't do any HTML escaping here.
                Value::String(GoString::new(b.clone()))
            }
            other => panic!("not implemented: {}", attr_value_go_type(other)),
        };

        if owner_type == AttributesOwnerType::CodeBlockChroma && is_chroma_attribute(&name_lower) {
            opts.push(Attribute {
                name: GoString::new(v.name.clone()),
                value: vv,
            });
        } else {
            attrs.push(Attribute {
                name: GoString::new(name_lower),
                value: vv,
            });
        }
    }

    AttributesHolder {
        attributes: attrs,
        options: opts,
        ..Default::default()
    }
}

/// A Go `[][2]int` as a template value (`nil` when no range was parsed).
fn hl_ranges_value(r: Option<&[[i64; 2]]>) -> Value {
    match r {
        None => Value::TypedNil(Arc::from("[][2]int")),
        Some(r) => Value::List(Arc::new(List::new(
            SliceType::Named(Arc::from("[][2]int")),
            r.iter()
                .map(|p| {
                    Value::List(Arc::new(List::new(
                        SliceType::Named(Arc::from("[2]int")),
                        vec![Value::int(p[0]), Value::int(p[1])],
                    )))
                })
                .collect(),
        ))),
    }
}

/// Go: `attributes.AttributesHolder` — node attributes as a `map[string]any` plus options.
#[derive(Debug, Default)]
pub struct AttributesHolder {
    /// What we get from Goldmark.
    pub attributes: Vec<Attribute>,
    /// Attributes considered to be an option (code blocks).
    pub options: Vec<Attribute>,
    attributes_map: OnceLock<Map>,
    options_map: OnceLock<Map>,
}

impl Clone for AttributesHolder {
    fn clone(&self) -> Self {
        AttributesHolder {
            attributes: self.attributes.clone(),
            options: self.options.clone(),
            ..Default::default()
        }
    }
}

impl AttributesHolder {
    /// Go: `Attributes() map[string]any` (later attributes win for the same name).
    // Go: markup/internal/attributes/attributes.go:Attributes
    pub fn attributes(&self) -> Map {
        self.attributes_map
            .get_or_init(|| to_map(&self.attributes))
            .clone()
    }

    // Go: markup/internal/attributes/attributes.go:Options
    pub fn options(&self) -> Map {
        self.options_map
            .get_or_init(|| to_map(&self.options))
            .clone()
    }

    // Go: markup/internal/attributes/attributes.go:AttributesSlice
    pub fn attributes_slice(&self) -> &[Attribute] {
        &self.attributes
    }

    // Go: markup/internal/attributes/attributes.go:OptionsSlice
    pub fn options_slice(&self) -> &[Attribute] {
        &self.options
    }
}

fn to_map(attrs: &[Attribute]) -> Map {
    let mut m = Map::new(MapType::StringAny);
    for v in attrs {
        m.insert(v.name.clone(), v.value.clone());
    }
    m
}

/// `[]attributes.Attribute` as a template value (`AttributesSlice`, `OptionsSlice`): Go structs
/// with the fields `Name` and `Value`.
pub(crate) fn attribute_slice_value(attrs: &[Attribute]) -> Value {
    Value::List(Arc::new(List::new(
        SliceType::Named(Arc::from("[]attributes.Attribute")),
        attrs
            .iter()
            .map(|a| Value::object(AttributeObject(a.clone())))
            .collect(),
    )))
}

/// `attributes.Attribute` as a template value.
struct AttributeObject(Attribute);

nh_common::go_methods!(AttributeObject {
    "ValueString" => |a, _x, args| {
        nh_common::object::args::exactly(args, 0, "ValueString")?;
        Ok(Value::String(a.0.value_string()))
    },
});

impl go_value::Object for AttributeObject {
    nh_common::object_basics!("attributes.Attribute");
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Name" => Some(Value::String(self.0.name.clone())),
            "Value" => Some(self.0.value.clone()),
            _ => None,
        }
    }
}

/// Go: `attributes.Empty` (holds no attributes).
pub fn empty() -> Arc<AttributesHolder> {
    static EMPTY: LazyLock<Arc<AttributesHolder>> =
        LazyLock::new(|| Arc::new(AttributesHolder::default()));
    EMPTY.clone()
}

/// Go `cast.ToString` of an AST attribute value.
fn cast_attr_to_string(v: &AttrValue) -> Vec<u8> {
    match v {
        AttrValue::Bytes(b) | AttrValue::String(b) => b.clone(),
        AttrValue::Float(f) => go_strconv::format_float(*f, b'f', -1, 64).into_bytes(),
        AttrValue::Bool(b) => go_strconv::format_bool(*b).as_bytes().to_vec(),
        AttrValue::Other(o) => match o.downcast_ref::<i64>() {
            Some(i) => go_strconv::itoa(*i).into_bytes(),
            None => Vec::new(),
        },
        // nil, []any and parser.Attributes: cast returns "" (with an error).
        _ => Vec::new(),
    }
}

/// RenderASTAttributes writes the AST attributes to the given as attributes to an HTML element.
/// This is used by the default HTML renderers, e.g. for headings etc. where no hook template
/// could be found. This performs HTML escaping of string attributes.
// Go: markup/internal/attributes/attributes.go:RenderASTAttributes
pub fn render_ast_attributes(w: &mut dyn BufWriter, attributes: &[AstAttribute]) {
    for attr in attributes {
        let a = go_unicode::strings::to_lower(&attr.name);
        if a.starts_with(b"on") {
            continue;
        }

        w.write_string(" ");
        w.write(&attr.name);
        w.write_string("=\"");

        match &attr.value {
            AttrValue::Bytes(v) => w.write(&goldmark::util::escape_html(v)),
            v => w.write(&cast_attr_to_string(v)),
        }

        w.write_byte(b'"');
    }
}

/// RenderAttributes writes the attributes to the given as attributes to an HTML element.
/// This is used for the default codeblock rendering.
// Go: markup/internal/attributes/attributes.go:RenderAttributes
pub fn render_attributes(w: &mut dyn BufWriter, skip_class: bool, attributes: &[Attribute]) {
    for attr in attributes {
        let a = go_unicode::strings::to_lower(attr.name.as_bytes());
        if skip_class && a.as_ref() == b"class" {
            continue;
        }
        w.write_string(" ");
        w.write(attr.name.as_bytes());
        w.write_string("=\"");
        // The values are never `[]byte` (New converts them to strings): cast.ToString.
        w.write(nh_common::cast::caste::to_string(&attr.value).as_bytes());
        w.write_byte(b'"');
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/internal/attributes/attributes.go (225 lines; 3/9 funcs executed)
//   types: AttributesOwnerType, Attribute, AttributesHolder, Attributes
// OK L43-47: init()
// OK L57-117: New(astAttributes []ast.Attribute, ownerType AttributesOwnerType) *AttributesHolder
// OK L124-126: (a Attribute) ValueString() string
// OK L147-155: (a *AttributesHolder) Attributes() map[string]any
// OK L157-165: (a *AttributesHolder) Options() map[string]any
// OK L167-169: (a *AttributesHolder) AttributesSlice() []Attribute
// OK L171-173: (a *AttributesHolder) OptionsSlice() []Attribute
// OK L178-200: RenderASTAttributes(w hugio.FlexiWriter, attributes ...ast.Attribute)
// OK L205-225: RenderAttributes(w hugio.FlexiWriter, skipClass bool, attributes ...Attribute)
// ---------------------------------------------------------------------------
