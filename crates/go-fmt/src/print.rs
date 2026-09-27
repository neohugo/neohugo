//! Port of `$GOROOT/src/fmt/print.go` (go1.27.1): the printer (`pp`), the
//! value walker (`printArg`/`printValue`/`handleMethods`) and the format
//! string interpreter (`doPrintf`), over [`go_value::Value`] instead of
//! `reflect.Value`.
//!
//! How Go's reflection maps onto the value model (see PORTING.md):
//! - `Value::Invalid` is a nil interface (`arg == nil`); inside a container
//!   it is a nil element of the container's element type.
//! - `Value::TypedNil(t)` is a nil pointer/slice/map/func/chan (classified by
//!   [`typed_nil_kind`]); a nil of interface type is the same as `Invalid`.
//! - `Value::Object` methods `go_error`/`go_string`/`go_go_string` are
//!   `error`/`Stringer`/`GoStringer`; `Kind::Ptr` objects print as
//!   `&{fields}` at depth 0 and as a pointer below; `Kind::Struct` as
//!   `{fields}`; `Kind::Map`/`Kind::Slice` via `map_keys`/`map_get`/`list`.
//! - Named slice/map types and typed nils get their `String`/`Error` method
//!   from the [`NamedMethod`] registry (`page.Pages`, `page.TaxonomyList`,
//!   `*time.Location` by default).
//! - `Value::Time` is `time.Time`: `String()`, `GoString()`, and the struct
//!   `{wall ext loc}` for verbs that do not accept a string.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use go_strconv::Rune;
use go_unicode::utf8;
use go_value::{FloatKind, Kind, List, Map, Object, SliceType, Time, UintKind, Value};

use crate::format::{Fmt, LDIGITS, SIGNED, UDIGITS, UNSIGNED};

// Strings for use with buffer.WriteString.
// This is less overhead than using buffer.Write with byte arrays.
const COMMA_SPACE_STRING: &[u8] = b", ";
const NIL_ANGLE_STRING: &[u8] = b"<nil>";
const NIL_PAREN_STRING: &[u8] = b"(nil)";
const NIL_STRING: &[u8] = b"nil";
const MAP_STRING: &[u8] = b"map[";
const PERCENT_BANG_STRING: &[u8] = b"%!";
const MISSING_STRING: &[u8] = b"(MISSING)";
const BAD_INDEX_STRING: &[u8] = b"(BADINDEX)";
const EXTRA_STRING: &[u8] = b"%!(EXTRA ";
const BAD_WIDTH_STRING: &[u8] = b"%!(BADWIDTH)";
const BAD_PREC_STRING: &[u8] = b"%!(BADPREC)";
const NO_VERB_STRING: &[u8] = b"%!(NOVERB)";
const INV_REFLECT_STRING: &[u8] = b"<invalid reflect.Value>";

const fn r(c: char) -> Rune {
    c as Rune
}

// ---------------------------------------------------------------------------
// Kind classification for the value model.

/// The `reflect.Kind` of a nil value of a Go type string, as used for
/// `Value::TypedNil` (and for nil elements of containers). Re-exported from
/// `go_value`, whose registry host crates extend with
/// [`go_value::register_named_kind`].
pub use go_value::{NilKind, typed_nil_kind};

// ---------------------------------------------------------------------------
// Methods of named types that the value model carries only by name.

/// A `String() string` or `Error() string` method of a named Go type whose
/// values the value model carries only by their type name: named slice and
/// map types (`Value::List` with `SliceType::Named`, `Value::Map` with
/// `MapType::Named`) and typed nils (`Value::TypedNil`). `fmt`'s
/// `handleMethods` consults it like Go's `arg.(error)` / `arg.(Stringer)`.
///
/// The function receives the operand and returns the method's result, or
/// `None` when the method panics on it. Go's `catchPanic` prints `<nil>` for
/// a panic on a nil pointer receiver, which is the only panic modelled.
#[derive(Clone, Copy)]
pub enum NamedMethod {
    /// `Error() string` (takes precedence over `String`, as in Go).
    Error(fn(&Value) -> Option<Vec<u8>>),
    /// `String() string` (`fmt.Stringer`).
    String(fn(&Value) -> Option<Vec<u8>>),
}

fn value_len(v: &Value) -> usize {
    match v {
        Value::List(l) => l.items.len(),
        Value::Map(m) => m.entries.len(),
        _ => 0,
    }
}

// Go: resources/page/pages.go:Pages.String
fn pages_string(v: &Value) -> Option<Vec<u8>> {
    Some(format!("Pages({})", value_len(v)).into_bytes())
}

// Go: resources/page/taxonomy.go:TaxonomyList.String
fn taxonomy_list_string(v: &Value) -> Option<Vec<u8>> {
    Some(format!("TaxonomyList({})", value_len(v)).into_bytes())
}

// Go: time/zoneinfo.go:(*Location).String on a nil receiver
fn nil_location_string(_: &Value) -> Option<Vec<u8>> {
    Some(b"UTC".to_vec())
}

fn named_methods() -> &'static RwLock<HashMap<String, NamedMethod>> {
    static REG: OnceLock<RwLock<HashMap<String, NamedMethod>>> = OnceLock::new();
    REG.get_or_init(|| {
        let mut m = HashMap::new();
        // The neohugo named slice/map types with a value-receiver String method.
        m.insert("page.Pages".to_string(), NamedMethod::String(pages_string));
        m.insert(
            "page.TaxonomyList".to_string(),
            NamedMethod::String(taxonomy_list_string),
        );
        // A nil *time.Location is UTC (Go: time.(*Location).String via l.get()).
        m.insert(
            "*time.Location".to_string(),
            NamedMethod::String(nil_location_string),
        );
        RwLock::new(m)
    })
}

/// Declares the `String`/`Error` method of a named Go type that reaches fmt
/// as a `Value::List`, `Value::Map` or `Value::TypedNil` (host objects use
/// `Object::go_string`/`go_error` instead). `page.Pages` and
/// `page.TaxonomyList` are registered by default. A pointer type whose
/// method panics on a nil receiver can be registered with a function that
/// returns `None`, so its typed nils print `<nil>` as in Go.
pub fn register_named_method(type_name: &str, m: NamedMethod) {
    named_methods()
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(type_name.to_string(), m);
}

/// The registered method of a value carried by type name (a named
/// `List`/`Map` or a `TypedNil`), for other crates that must see the same
/// `String`/`Error` methods as fmt (e.g. html/template's `jsValEscaper`).
pub fn named_method(v: &Value) -> Option<NamedMethod> {
    let name: &str = match v {
        Value::List(l) => match &l.ty {
            SliceType::Named(n) => n,
            _ => return None,
        },
        Value::Map(m) => match &m.ty {
            go_value::MapType::Named(n) => n,
            _ => return None,
        },
        Value::TypedNil(t) => t,
        _ => return None,
    };
    named_methods()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(name)
        .copied()
}

/// Go: `_, ok := arg.(error)`.
pub(crate) fn is_error(v: &Value) -> bool {
    match v {
        Value::Object(o) => o.go_error().is_some(),
        _ => matches!(named_method(v), Some(NamedMethod::Error(_))),
    }
}

/// Go: `arg == nil` for an `any` holding this value.
fn is_nil_interface(v: &Value) -> bool {
    match v {
        Value::Invalid => true,
        Value::TypedNil(t) => typed_nil_kind(t) == NilKind::Interface,
        _ => false,
    }
}

/// Whether a slice type has element kind uint8 (Go: `t.Elem().Kind() == reflect.Uint8`).
fn slice_elem_is_uint8(ty: &SliceType) -> bool {
    match ty {
        SliceType::Uint8 => true,
        SliceType::Named(n) => matches!(&**n, "[]uint8" | "[]byte" | "json.RawMessage"),
        _ => false,
    }
}

/// Whether a slice type is Go's unnamed `[]byte` (= `[]uint8`), which
/// `printArg` handles without reflection.
fn is_unnamed_byte_slice(ty: &SliceType) -> bool {
    match ty {
        SliceType::Uint8 => true,
        SliceType::Named(n) => matches!(&**n, "[]uint8" | "[]byte"),
        _ => false,
    }
}

/// Whether a nil of this type string is a nil byte slice.
fn is_byte_slice_type(ty: &str) -> bool {
    matches!(ty, "[]uint8" | "[]byte" | "json.RawMessage")
}

/// The element type string of a slice type (used for nil elements).
fn slice_elem_type(ty: &SliceType) -> Cow<'_, str> {
    match ty {
        SliceType::Any => Cow::Borrowed("interface {}"),
        SliceType::String => Cow::Borrowed("string"),
        SliceType::Int => Cow::Borrowed("int"),
        SliceType::Int64 => Cow::Borrowed("int64"),
        SliceType::Float64 => Cow::Borrowed("float64"),
        SliceType::Bool => Cow::Borrowed("bool"),
        SliceType::Uint8 => Cow::Borrowed("uint8"),
        SliceType::MapStringAny => Cow::Borrowed("map[string]interface {}"),
        SliceType::Named(n) => match n.strip_prefix("[]") {
            Some(e) => Cow::Borrowed(e),
            None => Cow::Borrowed(named_slice_elem(n)),
        },
    }
}

/// The element types of the neohugo named slice types whose elements can
/// be nil (the others have struct or string elements).
fn named_slice_elem(n: &str) -> &'static str {
    match n {
        "page.Pages" => "page.Page",
        "page.Sites" => "page.Site",
        "resource.Resources" => "resource.Resource",
        "langs.Languages" => "*langs.Language",
        "navigation.Menu" => "*navigation.MenuEntry",
        "tableofcontents.Headings" => "*tableofcontents.Heading",
        _ => "interface {}",
    }
}

/// The value type string of a map type (used for nil values).
fn map_elem_type(m: &Map) -> Cow<'_, str> {
    match &m.ty {
        go_value::MapType::StringAny | go_value::MapType::Params => Cow::Borrowed("interface {}"),
        go_value::MapType::StringString => Cow::Borrowed("string"),
        go_value::MapType::Named(n) => match n.strip_prefix("map[string]") {
            Some(e) => Cow::Borrowed(e),
            // The value types of the neohugo named map types whose values
            // can be nil (maps.Params, page.Data, exif.Tags and
            // attributes.Attributes are map[string]any).
            None => Cow::Borrowed(match &**n {
                "page.Taxonomy" => "page.WeightedPages",
                "page.TaxonomyList" => "page.Taxonomy",
                "navigation.Menus" => "navigation.Menu",
                "navigation.PageMenus" => "*navigation.MenuEntry",
                _ => "interface {}",
            }),
        },
    }
}

/// The bytes of a `[]uint8` list.
fn list_bytes(l: &List) -> Vec<u8> {
    l.items
        .iter()
        .map(|it| match it {
            Value::Uint(u, _) => *u as u8,
            Value::Int(i, _) => *i as u8,
            _ => 0,
        })
        .collect()
}

/// Go's `time.Time` internal representation (`wall`, `ext`, `loc == nil`)
/// for a time without a monotonic clock reading.
fn time_struct_fields(t: &Time) -> Vec<(Cow<'static, str>, Value)> {
    // hasMonotonic == 0: wall holds only the nanoseconds and ext the full
    // signed seconds since January 1, year 1.
    let wall = t.nsec as u64;
    let ext = t.unix_sec.wrapping_add(go_value::UNIX_TO_INTERNAL);
    let loc = match &t.loc {
        None => Value::TypedNil(Arc::from("*time.Location")),
        Some(l) if go_time::is_utc_loc(l) => Value::TypedNil(Arc::from("*time.Location")),
        Some(l) => Value::Object(Arc::new(LocPtr {
            addr: Arc::as_ptr(l) as usize,
        })),
    };
    vec![
        (Cow::Borrowed("wall"), Value::Uint(wall, UintKind::Uint64)),
        (
            Cow::Borrowed("ext"),
            Value::Int(ext, go_value::IntKind::Int64),
        ),
        (Cow::Borrowed("loc"), loc),
    ]
}

/// A non-nil `*time.Location` field of a `time.Time` (printed as a pointer).
struct LocPtr {
    addr: usize,
}

impl Object for LocPtr {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*time.Location")
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: go_value::HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn identity(&self) -> usize {
        self.addr
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// The struct a `Kind::Ptr` host object points to (Go: `f.Elem()`). Its
/// method set is empty: the host's `String`/`Error` methods are taken to
/// have pointer receivers, so they belong to the pointer only.
struct Elem {
    ptr: Arc<dyn Object>,
}

impl Object for Elem {
    fn type_name(&self) -> Cow<'_, str> {
        match self.ptr.type_name() {
            Cow::Borrowed(t) => Cow::Borrowed(t.strip_prefix('*').unwrap_or(t)),
            Cow::Owned(t) => Cow::Owned(t.strip_prefix('*').unwrap_or(&t).to_string()),
        }
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: go_value::HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        self.ptr.struct_fields()
    }
    fn identity(&self) -> usize {
        self.ptr.identity()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------
// The printer.

/// Go: `pp`, a printer's state.
#[derive(Default)]
pub(crate) struct Pp {
    /// fmt is used to format basic items such as integers or strings.
    /// It owns the output buffer (Go's `p.buf`).
    pub fmt: Fmt,

    /// reordered records whether the format string used argument reordering.
    pub reordered: bool,
    /// goodArgNum records whether the most recent reordering directive was valid.
    good_arg_num: bool,
    /// erroring is set when printing an error string to guard against calling handleMethods.
    erroring: bool,
    /// wrapErrs is set when the format string may contain a %w verb.
    pub wrap_errs: bool,
    /// wrappedErrs records the targets of the %w verb.
    pub wrapped_errs: Vec<usize>,
}

// Go: fmt/print.go:tooLarge
/// tooLarge reports whether the magnitude of the integer is
/// too large to be used as a formatting width or precision.
fn too_large(x: i64) -> bool {
    const MAX: i64 = 1_000_000;
    x > MAX || x < -MAX
}

// Go: fmt/print.go:parsenum
/// parsenum converts ASCII to integer.  num is 0 (and isnum is false) if no number present.
pub(crate) fn parsenum(s: &[u8], start: usize, end: usize) -> (i64, bool, usize) {
    if start >= end {
        return (0, false, end);
    }
    let mut num: i64 = 0;
    let mut isnum = false;
    let mut newi = start;
    while newi < end && b'0' <= s[newi] && s[newi] <= b'9' {
        if too_large(num) {
            return (0, false, end); // Overflow; crazy long number most likely.
        }
        num = num * 10 + (s[newi] - b'0') as i64;
        isnum = true;
        newi += 1;
    }
    (num, isnum, newi)
}

// Go: fmt/print.go:intFromArg
/// intFromArg gets the argNumth element of a. On return, isInt reports whether the argument has integer type.
fn int_from_arg(a: &[Value], arg_num: usize) -> (i64, bool, usize) {
    let mut new_arg_num = arg_num;
    let mut num: i64 = 0;
    let mut is_int = false;
    if arg_num < a.len() {
        match &a[arg_num] {
            // Almost always OK: a[argNum].(int), then the reflect kinds.
            Value::Int(n, _) => {
                // int64(int(n)) == n always holds with a 64-bit int.
                num = *n;
                is_int = true;
            }
            Value::Uint(n, _) => {
                let n = *n;
                if (n as i64) >= 0 {
                    num = n as i64;
                    is_int = true;
                }
            }
            _ => {
                // Already 0, false.
            }
        }
        new_arg_num = arg_num + 1;
        if too_large(num) {
            num = 0;
            is_int = false;
        }
    }
    (num, is_int, new_arg_num)
}

// Go: fmt/print.go:parseArgNumber
/// parseArgNumber returns the value of the bracketed number, minus 1
/// (explicit argument numbers are one-indexed but we want zero-indexed).
/// The opening bracket is known to be present at format[0].
/// The returned values are the index, the number of bytes to consume
/// up to the closing paren, if present, and whether the number parsed
/// ok. The bytes to consume will be 1 if no closing paren is present.
fn parse_arg_number(format: &[u8]) -> (i64, usize, bool) {
    // There must be at least 3 bytes: [n].
    if format.len() < 3 {
        return (0, 1, false);
    }

    // Find closing bracket.
    for i in 1..format.len() {
        if format[i] == b']' {
            let (width, ok, newi) = parsenum(format, 1, i);
            if !ok || newi != i {
                return (0, i + 1, false);
            }
            return (width - 1, i + 1, true); // arg numbers are one-indexed and skip paren.
        }
    }
    (0, 1, false)
}

impl Pp {
    fn buf(&mut self) -> &mut Vec<u8> {
        &mut self.fmt.buf
    }

    fn write_rune(&mut self, r: Rune) {
        utf8::append_rune(&mut self.fmt.buf, r);
    }

    // Go: fmt/print.go:(*pp).unknownType
    #[allow(dead_code)]
    fn unknown_type(&mut self, v: &Value) {
        if v.is_invalid() {
            self.buf().extend_from_slice(NIL_ANGLE_STRING);
            return;
        }
        self.buf().push(b'?');
        let t = v.go_type_name().into_owned();
        self.buf().extend_from_slice(t.as_bytes());
        self.buf().push(b'?');
    }

    // Go: fmt/print.go:(*pp).badVerb
    /// `arg` is Go's `arg any` (None = nil), `value` Go's `reflect.Value`
    /// (None = the invalid Value).
    fn bad_verb(&mut self, arg: Option<&Value>, value: Option<&Value>, verb: Rune) {
        self.erroring = true;
        self.buf().extend_from_slice(PERCENT_BANG_STRING);
        self.write_rune(verb);
        self.buf().push(b'(');
        match (arg, value) {
            (Some(arg), _) if !is_nil_interface(arg) => {
                let t = arg.go_type_name().into_owned();
                self.buf().extend_from_slice(t.as_bytes());
                self.buf().push(b'=');
                self.print_arg(arg, r('v'));
            }
            (_, Some(value)) if !value.is_invalid() => {
                let t = value.go_type_name().into_owned();
                self.buf().extend_from_slice(t.as_bytes());
                self.buf().push(b'=');
                self.print_value(value, r('v'), 0);
            }
            _ => {
                self.buf().extend_from_slice(NIL_ANGLE_STRING);
            }
        }
        self.buf().push(b')');
        self.erroring = false;
    }

    // Go: fmt/print.go:(*pp).fmtBool
    fn fmt_bool(&mut self, arg: Option<&Value>, value: Option<&Value>, v: bool, verb: Rune) {
        if verb == r('t') || verb == r('v') {
            self.fmt.fmt_boolean(v);
        } else {
            self.bad_verb(arg, value, verb);
        }
    }

    // Go: fmt/print.go:(*pp).fmt0x64
    /// fmt0x64 formats a uint64 in hexadecimal and prefixes it with 0x or
    /// not, as requested, by temporarily setting the sharp flag.
    fn fmt_0x64(&mut self, v: u64, leading0x: bool) {
        let sharp = self.fmt.f.sharp;
        self.fmt.f.sharp = leading0x;
        self.fmt.fmt_integer(v, 16, UNSIGNED, r('v'), LDIGITS);
        self.fmt.f.sharp = sharp;
    }

    // Go: fmt/print.go:(*pp).fmtInteger
    /// fmtInteger formats a signed or unsigned integer.
    fn fmt_integer(
        &mut self,
        arg: Option<&Value>,
        value: Option<&Value>,
        v: u64,
        is_signed: bool,
        verb: Rune,
    ) {
        match char::from_u32(verb as u32).unwrap_or('\u{0}') {
            'v' => {
                if self.fmt.f.sharp_v && !is_signed {
                    self.fmt_0x64(v, true);
                } else {
                    self.fmt.fmt_integer(v, 10, is_signed, verb, LDIGITS);
                }
            }
            'd' => self.fmt.fmt_integer(v, 10, is_signed, verb, LDIGITS),
            'b' => self.fmt.fmt_integer(v, 2, is_signed, verb, LDIGITS),
            'o' | 'O' => self.fmt.fmt_integer(v, 8, is_signed, verb, LDIGITS),
            'x' => self.fmt.fmt_integer(v, 16, is_signed, verb, LDIGITS),
            'X' => self.fmt.fmt_integer(v, 16, is_signed, verb, UDIGITS),
            'c' => self.fmt.fmt_c(v),
            'q' => self.fmt.fmt_qc(v),
            'U' => self.fmt.fmt_unicode(v),
            _ => self.bad_verb(arg, value, verb),
        }
    }

    // Go: fmt/print.go:(*pp).fmtFloat
    /// fmtFloat formats a float. The default precision for each verb
    /// is specified as last argument in the call to fmt_float.
    fn fmt_float(
        &mut self,
        arg: Option<&Value>,
        value: Option<&Value>,
        v: f64,
        size: i64,
        verb: Rune,
    ) {
        match char::from_u32(verb as u32).unwrap_or('\u{0}') {
            'v' => self.fmt.fmt_float(v, size, r('g'), -1),
            'b' | 'g' | 'G' | 'x' | 'X' => self.fmt.fmt_float(v, size, verb, -1),
            'f' | 'e' | 'E' => self.fmt.fmt_float(v, size, verb, 6),
            'F' => self.fmt.fmt_float(v, size, r('f'), 6),
            _ => self.bad_verb(arg, value, verb),
        }
    }

    // Go: fmt/print.go:(*pp).fmtString
    fn fmt_string(&mut self, arg: Option<&Value>, value: Option<&Value>, v: &[u8], verb: Rune) {
        match char::from_u32(verb as u32).unwrap_or('\u{0}') {
            'v' => {
                if self.fmt.f.sharp_v {
                    self.fmt.fmt_q(v);
                } else {
                    self.fmt.fmt_s(v);
                }
            }
            's' => self.fmt.fmt_s(v),
            'x' => self.fmt.fmt_sx(v, LDIGITS),
            'X' => self.fmt.fmt_sx(v, UDIGITS),
            'q' => self.fmt.fmt_q(v),
            _ => self.bad_verb(arg, value, verb),
        }
    }

    // Go: fmt/print.go:(*pp).fmtBytes
    /// `v == None` is a nil byte slice. `value` is the slice itself, for the
    /// default case (Go: `reflect.ValueOf(v)`).
    fn fmt_bytes(&mut self, v: Option<&[u8]>, value: &Value, verb: Rune, type_string: &str) {
        match char::from_u32(verb as u32).unwrap_or('\u{0}') {
            'v' | 'd' => {
                if self.fmt.f.sharp_v {
                    self.buf().extend_from_slice(type_string.as_bytes());
                    let Some(v) = v else {
                        self.buf().extend_from_slice(NIL_PAREN_STRING);
                        return;
                    };
                    self.buf().push(b'{');
                    for (i, &c) in v.iter().enumerate() {
                        if i > 0 {
                            self.buf().extend_from_slice(COMMA_SPACE_STRING);
                        }
                        self.fmt_0x64(c as u64, true);
                    }
                    self.buf().push(b'}');
                } else {
                    self.buf().push(b'[');
                    for (i, &c) in v.unwrap_or(&[]).iter().enumerate() {
                        if i > 0 {
                            self.buf().push(b' ');
                        }
                        self.fmt.fmt_integer(c as u64, 10, UNSIGNED, verb, LDIGITS);
                    }
                    self.buf().push(b']');
                }
            }
            's' => self.fmt.fmt_bs(v.unwrap_or(&[])),
            'x' => self.fmt.fmt_bx(v.unwrap_or(&[]), LDIGITS),
            'X' => self.fmt.fmt_bx(v.unwrap_or(&[]), UDIGITS),
            'q' => self.fmt.fmt_q(v.unwrap_or(&[])),
            _ => self.print_value(value, verb, 0),
        }
    }

    // Go: fmt/print.go:(*pp).fmtPointer
    fn fmt_pointer(&mut self, arg: Option<&Value>, value: &Value, verb: Rune) {
        let u: u64 = match value {
            Value::TypedNil(t) => match typed_nil_kind(t) {
                NilKind::Ptr | NilKind::Slice | NilKind::Map | NilKind::Func | NilKind::Chan => 0,
                NilKind::Interface => {
                    self.bad_verb(arg, Some(value), verb);
                    return;
                }
            },
            Value::List(l) => Arc::as_ptr(l) as usize as u64,
            Value::Map(m) => Arc::as_ptr(m) as usize as u64,
            Value::Object(o) => match o.kind() {
                Kind::Ptr | Kind::Map | Kind::Slice | Kind::Func | Kind::Interface => {
                    o.identity() as u64
                }
                Kind::Struct => {
                    self.bad_verb(arg, Some(value), verb);
                    return;
                }
            },
            _ => {
                self.bad_verb(arg, Some(value), verb);
                return;
            }
        };

        match char::from_u32(verb as u32).unwrap_or('\u{0}') {
            'v' => {
                if self.fmt.f.sharp_v {
                    self.buf().push(b'(');
                    let t = value.go_type_name().into_owned();
                    self.buf().extend_from_slice(t.as_bytes());
                    self.buf().extend_from_slice(b")(");
                    if u == 0 {
                        self.buf().extend_from_slice(NIL_STRING);
                    } else {
                        self.fmt_0x64(u, true);
                    }
                    self.buf().push(b')');
                } else if u == 0 {
                    self.fmt.pad_string(NIL_ANGLE_STRING);
                } else {
                    let sharp = self.fmt.f.sharp;
                    self.fmt_0x64(u, !sharp);
                }
            }
            'p' => {
                let sharp = self.fmt.f.sharp;
                self.fmt_0x64(u, !sharp);
            }
            'b' | 'o' | 'd' | 'x' | 'X' => self.fmt_integer(arg, Some(value), u, UNSIGNED, verb),
            _ => self.bad_verb(arg, Some(value), verb),
        }
    }

    // Go: fmt/print.go:(*pp).handleMethods
    /// Formatter and panics are not modelled (see PORTING.md); `error` is
    /// `Object::go_error`, `Stringer` is `Object::go_string`, `GoStringer`
    /// is `Object::go_go_string`, `time.Time` has `String`/`GoString`, and
    /// named slice/map types and typed nils use the [`NamedMethod`] registry.
    fn handle_methods(&mut self, arg: &Value, value: Option<&Value>, mut verb: Rune) -> bool {
        if self.erroring {
            return false;
        }
        if verb == r('w') {
            // It is invalid to use %w other than with Errorf or with a non-error arg.
            let ok = is_error(arg);
            if !ok || !self.wrap_errs {
                self.bad_verb(Some(arg), value, verb);
                return true;
            }
            // If the arg is a Formatter, pass 'v' as the verb to it.
            verb = r('v');
        }

        // Is it a Formatter? (not modelled)

        // If we're doing Go syntax and the argument knows how to supply it, take care of it now.
        if self.fmt.f.sharp_v {
            match arg {
                Value::Object(o) => {
                    if let Some(s) = o.go_go_string() {
                        // Print the result of GoString unadorned.
                        self.fmt.fmt_s(s.as_bytes());
                        return true;
                    }
                }
                Value::Time(t) => {
                    // Print the result of GoString unadorned.
                    let s = go_time::GoTimeExt::go_string(t);
                    self.fmt.fmt_s(s.as_bytes());
                    return true;
                }
                _ => {}
            }
        } else {
            // If a string is acceptable according to the format, see if
            // the value satisfies one of the string-valued interfaces.
            // Println etc. set verb to %v, which is "stringable".
            if verb == r('v')
                || verb == r('s')
                || verb == r('x')
                || verb == r('X')
                || verb == r('q')
            {
                // Is it an error or Stringer?
                match arg {
                    Value::Object(o) => {
                        if let Some(e) = o.go_error() {
                            self.fmt_string(Some(arg), value, e.as_bytes(), verb);
                            return true;
                        }
                        if let Some(s) = o.go_string() {
                            self.fmt_string(Some(arg), value, s.as_bytes(), verb);
                            return true;
                        }
                    }
                    Value::Time(t) => {
                        let s = go_time::GoTimeExt::string(t);
                        self.fmt_string(Some(arg), value, s.as_bytes(), verb);
                        return true;
                    }
                    Value::List(_) | Value::Map(_) | Value::TypedNil(_) => {
                        if let Some(m) = named_method(arg) {
                            let (NamedMethod::Error(f) | NamedMethod::String(f)) = m;
                            match f(arg) {
                                Some(s) => self.fmt_string(Some(arg), value, &s, verb),
                                // Go: catchPanic on a nil pointer receiver.
                                None => self.buf().extend_from_slice(NIL_ANGLE_STRING),
                            }
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        false
    }

    // Go: fmt/print.go:(*pp).printArg
    pub(crate) fn print_arg(&mut self, arg: &Value, verb: Rune) {
        if is_nil_interface(arg) {
            if verb == r('T') || verb == r('v') {
                self.fmt.pad_string(NIL_ANGLE_STRING);
            } else {
                self.bad_verb(None, None, verb);
            }
            return;
        }

        // Special processing considerations.
        // %T (the value's type) and %p (its address) are special; we always do them first.
        if verb == r('T') {
            let t = arg.go_type_name().into_owned();
            self.fmt.fmt_s(t.as_bytes());
            return;
        }
        if verb == r('p') {
            self.fmt_pointer(Some(arg), arg, r('p'));
            return;
        }

        // Some types can be done without reflection.
        match arg {
            Value::Bool(b) => self.fmt_bool(Some(arg), None, *b, verb),
            Value::Float(f, FloatKind::F32) => self.fmt_float(Some(arg), None, *f, 32, verb),
            Value::Float(f, FloatKind::F64) => self.fmt_float(Some(arg), None, *f, 64, verb),
            Value::Int(i, _) => self.fmt_integer(Some(arg), None, *i as u64, SIGNED, verb),
            Value::Uint(u, _) => self.fmt_integer(Some(arg), None, *u, UNSIGNED, verb),
            Value::String(s) => self.fmt_string(Some(arg), None, s, verb),
            // case []byte: (the unnamed type, however the host tagged it)
            Value::List(l) if is_unnamed_byte_slice(&l.ty) => {
                let b = list_bytes(l);
                self.fmt_bytes(Some(&b), arg, verb, "[]byte");
            }
            Value::TypedNil(t) if &**t == "[]uint8" || &**t == "[]byte" => {
                self.fmt_bytes(None, arg, verb, "[]byte");
            }
            _ => {
                // If the type is not simple, it might have methods.
                if !self.handle_methods(arg, None, verb) {
                    // Need to use reflection, since the type had no
                    // interface methods that could be used for formatting.
                    self.print_value(arg, verb, 0);
                }
            }
        }
    }

    /// An element of a container: a nil element (`Value::Invalid`) is a nil
    /// of the container's element type (Go: an interface/pointer/map/slice
    /// element holding nil).
    ///
    /// A typed nil of an interface type stored in an interface-typed element
    /// is that element type's nil too: Go has no "nil `error` inside an
    /// `interface {}`", converting a nil interface to another interface
    /// type yields the target's nil (so `[]interface {}` prints
    /// `interface {}(nil)` for it under `%#v`).
    fn print_elem(&mut self, v: &Value, elem_type: &str, verb: Rune, depth: i64) {
        let is_elem_nil = match v {
            Value::Invalid => true,
            Value::TypedNil(t) => {
                &**t != elem_type
                    && typed_nil_kind(t) == NilKind::Interface
                    && typed_nil_kind(elem_type) == NilKind::Interface
            }
            _ => false,
        };
        if is_elem_nil {
            let tn = Value::TypedNil(Arc::from(elem_type));
            self.print_value(&tn, verb, depth);
        } else {
            self.print_value(v, verb, depth);
        }
    }

    /// A struct field: the field's declared type is not known, so a nil
    /// (`Value::Invalid`) is taken to be an `interface {}` field, while a
    /// typed nil keeps its own type (it is the field's type). An unexported
    /// field cannot be interfaced (Go: `value.CanInterface()` is false), so
    /// its methods are not consulted.
    fn print_field(&mut self, v: &Value, exported: bool, verb: Rune, depth: i64) {
        let tn;
        let v = if v.is_invalid() {
            tn = Value::TypedNil(Arc::from("interface {}"));
            &tn
        } else {
            v
        };
        if exported {
            self.print_value(v, verb, depth);
        } else {
            self.print_value_kind(v, verb, depth);
        }
    }

    // Go: fmt/print.go:(*pp).printValue
    /// printValue is similar to printArg but starts with a reflect value, not an interface{} value.
    /// It does not handle 'p' and 'T' verbs because these should have been already handled by printArg.
    pub(crate) fn print_value(&mut self, value: &Value, verb: Rune, depth: i64) {
        // Handle values with special methods if not already handled by printArg (depth == 0).
        if depth > 0 && !value.is_invalid() && self.handle_methods(value, Some(value), verb) {
            return;
        }
        self.print_value_kind(value, verb, depth);
    }

    /// The `switch value.Kind()` part of printValue (without the method
    /// check, for values that cannot be interfaced).
    fn print_value_kind(&mut self, value: &Value, verb: Rune, depth: i64) {
        match value {
            Value::Invalid => {
                if depth == 0 {
                    self.buf().extend_from_slice(INV_REFLECT_STRING);
                } else if verb == r('v') {
                    self.buf().extend_from_slice(NIL_ANGLE_STRING);
                } else {
                    self.bad_verb(None, Some(value), verb);
                }
            }
            Value::Bool(b) => self.fmt_bool(None, Some(value), *b, verb),
            Value::Int(i, _) => self.fmt_integer(None, Some(value), *i as u64, SIGNED, verb),
            Value::Uint(u, _) => self.fmt_integer(None, Some(value), *u, UNSIGNED, verb),
            Value::Float(f, FloatKind::F32) => self.fmt_float(None, Some(value), *f, 32, verb),
            Value::Float(f, FloatKind::F64) => self.fmt_float(None, Some(value), *f, 64, verb),
            Value::String(s) | Value::Safe(_, s) => self.fmt_string(None, Some(value), s, verb),
            Value::Map(m) => {
                let ty = m.ty.go_name();
                let elem = map_elem_type(m);
                let entries: Vec<(Value, &Value)> = m
                    .entries
                    .iter()
                    .map(|(k, v)| (Value::String(k.clone()), v))
                    .collect();
                self.print_map(&ty, false, &elem, &entries, verb, depth);
            }
            Value::List(l) => {
                let ty = l.ty.go_name();
                let elem = slice_elem_type(&l.ty);
                let is_bytes = slice_elem_is_uint8(&l.ty);
                let items: Vec<&Value> = l.items.iter().collect();
                self.print_slice(&ty, Some(&items), is_bytes, &elem, verb, depth);
            }
            Value::Time(t) => {
                // time.Time's fields (wall, ext, loc) are unexported.
                let fields = time_struct_fields(t);
                self.print_struct("time.Time", &fields, false, verb, depth);
            }
            Value::TypedNil(t) => match typed_nil_kind(t) {
                NilKind::Map => self.print_map(t, true, "interface {}", &[], verb, depth),
                NilKind::Slice => {
                    let elem = t.strip_prefix("[]").unwrap_or("interface {}");
                    self.print_slice(t, None, is_byte_slice_type(t), elem, verb, depth);
                }
                NilKind::Interface => {
                    // Go: case reflect.Interface with a nil Elem.
                    if self.fmt.f.sharp_v {
                        self.buf().extend_from_slice(t.as_bytes());
                        self.buf().extend_from_slice(NIL_PAREN_STRING);
                    } else {
                        self.buf().extend_from_slice(NIL_ANGLE_STRING);
                    }
                }
                // A nil pointer is never printed as `&...`.
                NilKind::Ptr | NilKind::Func | NilKind::Chan => self.fmt_pointer(None, value, verb),
            },
            Value::Object(o) => self.print_object(o, value, verb, depth),
        }
    }

    /// The `reflect.Kind` dispatch of `printValue` for host objects.
    fn print_object(&mut self, o: &Arc<dyn Object>, value: &Value, verb: Rune, depth: i64) {
        match o.kind() {
            Kind::Struct => {
                let ty = o.type_name().into_owned();
                let fields = o.struct_fields().unwrap_or_default();
                self.print_struct(&ty, &fields, true, verb, depth);
            }
            Kind::Map => {
                let ty = o.type_name().into_owned();
                let keys = o.map_keys();
                let vals: Vec<Value> = keys
                    .iter()
                    .map(|k| o.map_get(k).unwrap_or(Value::Invalid))
                    .collect();
                let entries: Vec<(Value, &Value)> = keys
                    .iter()
                    .zip(vals.iter())
                    .map(|(k, v)| (Value::String(k.clone()), v))
                    .collect();
                self.print_map(&ty, false, "interface {}", &entries, verb, depth);
            }
            Kind::Slice => {
                let ty = o.type_name().into_owned();
                let items = o.list();
                let refs: Option<Vec<&Value>> = items.as_ref().map(|v| v.iter().collect());
                self.print_slice(&ty, refs.as_deref(), false, "interface {}", verb, depth);
            }
            Kind::Ptr | Kind::Interface => {
                // pointer to array or slice or struct? ok at top level
                // but not embedded (avoid loops)
                if depth == 0 {
                    // The pointee of a host object is a struct (f.Elem()).
                    self.buf().push(b'&');
                    let elem = Value::Object(Arc::new(Elem { ptr: o.clone() }));
                    self.print_value(&elem, verb, depth + 1);
                    return;
                }
                self.fmt_pointer(None, value, verb);
            }
            Kind::Func => self.fmt_pointer(None, value, verb),
        }
    }

    /// Go: printValue `case reflect.Map`. Keys are strings, already in
    /// fmtsort order.
    fn print_map(
        &mut self,
        ty: &str,
        is_nil: bool,
        elem_type: &str,
        entries: &[(Value, &Value)],
        verb: Rune,
        depth: i64,
    ) {
        if self.fmt.f.sharp_v {
            self.buf().extend_from_slice(ty.as_bytes());
            if is_nil {
                self.buf().extend_from_slice(NIL_PAREN_STRING);
                return;
            }
            self.buf().push(b'{');
        } else {
            self.buf().extend_from_slice(MAP_STRING);
        }
        // sorted := fmtsort.Sort(f): string keys in byte order.
        for (i, (k, v)) in entries.iter().enumerate() {
            if i > 0 {
                if self.fmt.f.sharp_v {
                    self.buf().extend_from_slice(COMMA_SPACE_STRING);
                } else {
                    self.buf().push(b' ');
                }
            }
            self.print_value(k, verb, depth + 1);
            self.buf().push(b':');
            self.print_elem(v, elem_type, verb, depth + 1);
        }
        if self.fmt.f.sharp_v {
            self.buf().push(b'}');
        } else {
            self.buf().push(b']');
        }
    }

    /// Go: printValue `case reflect.Struct`. `exported` tells whether the
    /// fields are exported (host `struct_fields`) or not (`time.Time`).
    fn print_struct(
        &mut self,
        ty: &str,
        fields: &[(Cow<'_, str>, Value)],
        exported: bool,
        verb: Rune,
        depth: i64,
    ) {
        if self.fmt.f.sharp_v {
            self.buf().extend_from_slice(ty.as_bytes());
        }
        self.buf().push(b'{');
        for (i, (name, v)) in fields.iter().enumerate() {
            if i > 0 {
                if self.fmt.f.sharp_v {
                    self.buf().extend_from_slice(COMMA_SPACE_STRING);
                } else {
                    self.buf().push(b' ');
                }
            }
            if (self.fmt.f.plus_v || self.fmt.f.sharp_v) && !name.is_empty() {
                self.buf().extend_from_slice(name.as_bytes());
                self.buf().push(b':');
            }
            // getField: a non-nil interface field prints its element.
            self.print_field(v, exported, verb, depth + 1);
        }
        self.buf().push(b'}');
    }

    /// Go: printValue `case reflect.Array, reflect.Slice`. `items == None`
    /// is a nil slice.
    fn print_slice(
        &mut self,
        ty: &str,
        items: Option<&[&Value]>,
        elem_is_uint8: bool,
        elem_type: &str,
        verb: Rune,
        depth: i64,
    ) {
        if verb == r('s') || verb == r('q') || verb == r('x') || verb == r('X') {
            // Handle byte and uint8 slices and arrays special for the above verbs.
            if elem_is_uint8 {
                let bytes: Option<Vec<u8>> = items.map(|items| {
                    items
                        .iter()
                        .map(|it| match it {
                            Value::Uint(u, _) => *u as u8,
                            Value::Int(i, _) => *i as u8,
                            _ => 0,
                        })
                        .collect()
                });
                // A nil slice has no bytes; fmtBytes only distinguishes nil for %v/%d.
                let v = Value::Invalid;
                self.fmt_bytes(Some(bytes.as_deref().unwrap_or(&[])), &v, verb, ty);
                return;
            }
        }
        if self.fmt.f.sharp_v {
            self.buf().extend_from_slice(ty.as_bytes());
            let Some(items) = items else {
                self.buf().extend_from_slice(NIL_PAREN_STRING);
                return;
            };
            self.buf().push(b'{');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    self.buf().extend_from_slice(COMMA_SPACE_STRING);
                }
                self.print_elem(it, elem_type, verb, depth + 1);
            }
            self.buf().push(b'}');
        } else {
            self.buf().push(b'[');
            for (i, it) in items.unwrap_or(&[]).iter().enumerate() {
                if i > 0 {
                    self.buf().push(b' ');
                }
                self.print_elem(it, elem_type, verb, depth + 1);
            }
            self.buf().push(b']');
        }
    }

    // Go: fmt/print.go:(*pp).argNumber
    /// argNumber returns the next argument to evaluate, which is either the value of the passed-in
    /// argNum or the value of the bracketed integer that begins format[i:]. It also returns
    /// the new value of i, that is, the index of the next byte of the format to process.
    fn arg_number(
        &mut self,
        arg_num: usize,
        format: &[u8],
        i: usize,
        num_args: usize,
    ) -> (usize, usize, bool) {
        if format.len() <= i || format[i] != b'[' {
            return (arg_num, i, false);
        }
        self.reordered = true;
        let (index, wid, ok) = parse_arg_number(&format[i..]);
        if ok && 0 <= index && index < num_args as i64 {
            return (index as usize, i + wid, true);
        }
        self.good_arg_num = false;
        (arg_num, i + wid, ok)
    }

    // Go: fmt/print.go:(*pp).badArgNum
    fn bad_arg_num(&mut self, verb: Rune) {
        self.buf().extend_from_slice(PERCENT_BANG_STRING);
        self.write_rune(verb);
        self.buf().extend_from_slice(BAD_INDEX_STRING);
    }

    // Go: fmt/print.go:(*pp).missingArg
    fn missing_arg(&mut self, verb: Rune) {
        self.buf().extend_from_slice(PERCENT_BANG_STRING);
        self.write_rune(verb);
        self.buf().extend_from_slice(MISSING_STRING);
    }

    // Go: fmt/print.go:(*pp).doPrintf
    pub(crate) fn do_printf(&mut self, format: &[u8], a: &[Value]) {
        let end = format.len();
        let mut arg_num: usize = 0; // we process one argument per non-trivial format
        let mut after_index: bool; // previous item in format was an index like [3].
        self.reordered = false;
        let mut i: usize = 0;
        'format_loop: while i < end {
            self.good_arg_num = true;
            let lasti = i;
            while i < end && format[i] != b'%' {
                i += 1;
            }
            if i > lasti {
                self.buf().extend_from_slice(&format[lasti..i]);
            }
            if i >= end {
                // done processing format string
                break;
            }

            // Process one verb
            i += 1;

            // Do we have flags?
            self.fmt.clearflags();
            'simple_format: while i < end {
                let c = format[i];
                match c {
                    b'#' => self.fmt.f.sharp = true,
                    b'0' => self.fmt.f.zero = true,
                    b'+' => self.fmt.f.plus = true,
                    b'-' => self.fmt.f.minus = true,
                    b' ' => self.fmt.f.space = true,
                    _ => {
                        // Fast path for common case of ascii lower case simple verbs
                        // without precision or width or argument indices.
                        if c.is_ascii_lowercase() && arg_num < a.len() {
                            if c == b'w' {
                                self.wrapped_errs.push(arg_num);
                            }
                            if c == b'w' || c == b'v' {
                                // Go syntax
                                self.fmt.f.sharp_v = self.fmt.f.sharp;
                                self.fmt.f.sharp = false;
                                // Struct-field syntax
                                self.fmt.f.plus_v = self.fmt.f.plus;
                                self.fmt.f.plus = false;
                            }
                            self.print_arg(&a[arg_num], c as Rune);
                            arg_num += 1;
                            i += 1;
                            continue 'format_loop;
                        }
                        // Format is more complex than simple flags and a verb or is malformed.
                        break 'simple_format;
                    }
                }
                i += 1;
            }

            // Do we have an explicit argument index?
            (arg_num, i, after_index) = self.arg_number(arg_num, format, i, a.len());

            // Do we have width?
            if i < end && format[i] == b'*' {
                i += 1;
                (self.fmt.wid, self.fmt.f.wid_present, arg_num) = int_from_arg(a, arg_num);

                if !self.fmt.f.wid_present {
                    self.buf().extend_from_slice(BAD_WIDTH_STRING);
                }

                // We have a negative width, so take its value and ensure
                // that the minus flag is set
                if self.fmt.wid < 0 {
                    self.fmt.wid = -self.fmt.wid;
                    self.fmt.f.minus = true;
                    self.fmt.f.zero = false; // Do not pad with zeros to the right.
                }
                after_index = false;
            } else {
                (self.fmt.wid, self.fmt.f.wid_present, i) = parsenum(format, i, end);
                if after_index && self.fmt.f.wid_present {
                    // "%[3]2d"
                    self.good_arg_num = false;
                }
            }

            // Do we have precision?
            if i + 1 < end && format[i] == b'.' {
                i += 1;
                if after_index {
                    // "%[3].2d"
                    self.good_arg_num = false;
                }
                (arg_num, i, after_index) = self.arg_number(arg_num, format, i, a.len());
                if i < end && format[i] == b'*' {
                    i += 1;
                    (self.fmt.prec, self.fmt.f.prec_present, arg_num) = int_from_arg(a, arg_num);
                    // Negative precision arguments don't make sense
                    if self.fmt.prec < 0 {
                        self.fmt.prec = 0;
                        self.fmt.f.prec_present = false;
                    }
                    if !self.fmt.f.prec_present {
                        self.buf().extend_from_slice(BAD_PREC_STRING);
                    }
                    after_index = false;
                } else {
                    (self.fmt.prec, self.fmt.f.prec_present, i) = parsenum(format, i, end);
                    if !self.fmt.f.prec_present {
                        self.fmt.prec = 0;
                        self.fmt.f.prec_present = true;
                    }
                }
            }

            if !after_index {
                (arg_num, i, _) = self.arg_number(arg_num, format, i, a.len());
            }

            if i >= end {
                self.buf().extend_from_slice(NO_VERB_STRING);
                break;
            }

            let (verb, size) = utf8::decode_rune_in_string(&format[i..]);
            i += size;

            if verb == r('%') {
                // Percent does not absorb operands and ignores f.wid and f.prec.
                self.buf().push(b'%');
            } else if !self.good_arg_num {
                self.bad_arg_num(verb);
            } else if arg_num >= a.len() {
                // No argument left over to print for the current verb.
                self.missing_arg(verb);
            } else {
                if verb == r('w') {
                    self.wrapped_errs.push(arg_num);
                }
                if verb == r('w') || verb == r('v') {
                    // Go syntax
                    self.fmt.f.sharp_v = self.fmt.f.sharp;
                    self.fmt.f.sharp = false;
                    // Struct-field syntax
                    self.fmt.f.plus_v = self.fmt.f.plus;
                    self.fmt.f.plus = false;
                }
                self.print_arg(&a[arg_num], verb);
                arg_num += 1;
            }
        }

        // Check for extra arguments unless the call accessed the arguments
        // out of order, in which case it's too expensive to detect if they've all
        // been used and arguably OK if they're not.
        if !self.reordered && arg_num < a.len() {
            self.fmt.clearflags();
            self.buf().extend_from_slice(EXTRA_STRING);
            for (i, arg) in a[arg_num..].iter().enumerate() {
                if i > 0 {
                    self.buf().extend_from_slice(COMMA_SPACE_STRING);
                }
                if is_nil_interface(arg) {
                    self.buf().extend_from_slice(NIL_ANGLE_STRING);
                } else {
                    let t = arg.go_type_name().into_owned();
                    self.buf().extend_from_slice(t.as_bytes());
                    self.buf().push(b'=');
                    self.print_arg(arg, r('v'));
                }
            }
            self.buf().push(b')');
        }
    }

    // Go: fmt/print.go:(*pp).doPrint
    pub(crate) fn do_print(&mut self, a: &[Value]) {
        let mut prev_string = false;
        for (arg_num, arg) in a.iter().enumerate() {
            let is_string = matches!(arg, Value::String(_) | Value::Safe(_, _));
            // Add a space between two non-string arguments.
            if arg_num > 0 && !is_string && !prev_string {
                self.buf().push(b' ');
            }
            self.print_arg(arg, r('v'));
            prev_string = is_string;
        }
    }

    // Go: fmt/print.go:(*pp).doPrintln
    /// doPrintln is like doPrint but always adds a space between arguments
    /// and a newline after the last argument.
    pub(crate) fn do_println(&mut self, a: &[Value]) {
        for (arg_num, arg) in a.iter().enumerate() {
            if arg_num > 0 {
                self.buf().push(b' ');
            }
            self.print_arg(arg, r('v'));
        }
        self.buf().push(b'\n');
    }
}
