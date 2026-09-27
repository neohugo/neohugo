//! The Rust mirror of the data model, functions and `ExecHelper`s of
//! `tools/go-oracle/gotemplate/exec.go` (keep the two files in sync).
//!
//! Go structs become [`SObj`] objects (`ptr` = a `*T`, `addressable` = a
//! `T` reached through a pointer, whose method set is `*T`'s); named slice
//! and map types are `List`/`Map` values with a `Named` type; methods are
//! dispatched by [`call`] on (Go type, method name), exactly the methods Go's
//! reflection finds. Go funcs/methods whose result type is `any` return a
//! nil interface as `TypedNil("interface {}")`, as reflection does.

#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Once};

use go_time::GoTimeExt;
use go_value::{
    FloatKind, GoString, HostCtx, IntKind, Kind, Map, MapType, NilKind, Object, SafeKind,
    SliceType, Time, UintKind, Value,
};
use gotemplate::text::{ExecHelper, Func, FuncMap, FuncValue};

pub type GoResult = go_value::Result<Value>;

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

pub fn s(x: impl Into<GoString>) -> Value {
    Value::String(x.into())
}

pub fn int(i: i64) -> Value {
    Value::int(i)
}

/// A nil `interface {}` (Go: an interface-kind reflect.Value holding nil).
pub fn nil_any() -> Value {
    Value::TypedNil(Arc::from("interface {}"))
}

pub fn tnil(t: &str) -> Value {
    Value::TypedNil(Arc::from(t))
}

fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        go_value::register_named_kind("main.EtxPages", NilKind::Slice);
        go_value::register_named_kind("main.EtxData", NilKind::Map);
    });
}

// ---------------------------------------------------------------------------
// Objects

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// A Go struct (or pointer to struct) value.
pub struct SObj {
    /// The struct's Go type, e.g. `main.EtxT`.
    pub ty: &'static str,
    /// A `*T` (else a `T`).
    pub ptr: bool,
    /// A `T` that is addressable (reached through a pointer): its method set
    /// is `*T`'s.
    pub addressable: bool,
    pub fields: Vec<(&'static str, Value)>,
    pub id: usize,
}

impl SObj {
    pub fn new(ty: &'static str, ptr: bool, fields: Vec<(&'static str, Value)>) -> SObj {
        SObj {
            ty,
            ptr,
            addressable: false,
            fields,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
    }

    pub fn i(&self, name: &str) -> i64 {
        match self.get(name) {
            Some(Value::Int(i, _)) => *i,
            _ => 0,
        }
    }

    pub fn s(&self, name: &str) -> String {
        match self.get(name) {
            Some(Value::String(s)) => s.to_str_lossy().into_owned(),
            _ => String::new(),
        }
    }

    /// Whether the method set is `*T`'s.
    fn ptr_set(&self) -> bool {
        self.ptr || self.addressable
    }
}

impl Object for SObj {
    fn type_name(&self) -> Cow<'_, str> {
        if self.ptr {
            Cow::Owned(format!("*{}", self.ty))
        } else {
            Cow::Borrowed(self.ty)
        }
    }
    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }
    fn has_method(&self, name: &str) -> bool {
        has_method_on(self.ty, self.ptr_set(), name)
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.get(name).cloned()
    }
    fn is_zero(&self) -> Option<bool> {
        match self.ty {
            "main.EtxZ" => Some(matches!(self.get("Zero"), Some(Value::Bool(true)))),
            _ => None,
        }
    }
    fn go_string(&self) -> Option<GoString> {
        match self.ty {
            "main.EtxStr" if self.ptr_set() => Some(format!("Str({})", self.s("S")).into()),
            "main.EtxSV" => Some(format!("SV({})", self.s("S")).into()),
            // exec_test.go: `(*V).String`, `*bytes.Buffer`, `*strings.Builder`.
            "template.V" if self.ptr_set() => Some(format!("<{}>", self.i("j")).into()),
            "bytes.Buffer" | "strings.Builder" if self.ptr_set() => Some(self.s("buf").into()),
            _ => None,
        }
    }
    fn go_error(&self) -> Option<String> {
        match self.ty {
            "main.EtxErr" if self.ptr_set() => Some(format!("Err({})", self.s("Msg"))),
            // exec_test.go: `(*W).Error`.
            "template.W" if self.ptr_set() => Some(format!("[{}]", self.i("k"))),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(
            self.fields
                .iter()
                .map(|(n, v)| (Cow::Borrowed(*n), v.clone()))
                .collect(),
        )
    }
    fn identity(&self) -> usize {
        self.id
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// A Go `error` value made by `errors.New` (`*errors.errorString`).
pub struct GoErr(pub String);

impl Object for GoErr {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*errors.errorString")
    }
    fn has_method(&self, name: &str) -> bool {
        name == "Error"
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn go_error(&self) -> Option<String> {
        Some(self.0.clone())
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![(Cow::Borrowed("s"), s(self.0.as_str()))])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub fn obj(o: SObj) -> Value {
    Value::Object(Arc::new(o))
}

fn func_value(ty: &str, f: impl Fn(&[Value]) -> GoResult + Send + Sync + 'static) -> Value {
    Value::object(FuncValue {
        type_name: ty.to_string(),
        func: Arc::new(move |_ctx: HostCtx<'_>, args: &[Value]| f(args)),
    })
}

fn sprint(args: &[Value]) -> Vec<u8> {
    go_fmt::sprint(args)
}

fn sprintf(format: &str, args: &[Value]) -> Vec<u8> {
    go_fmt::sprintf(format, args)
}

fn time_date(y: i64, mo: i64, d: i64, h: i64, mi: i64, sec: i64, ns: i64) -> Time {
    go_time::date(y, go_time::Month(mo), d, h, mi, sec, ns, &go_time::utc())
}

// ---------------------------------------------------------------------------
// The data model (Go: etxRoot).

pub fn etx_v(a: &str, b: i64, addressable: bool) -> Value {
    let mut o = SObj::new("main.EtxV", false, vec![("A", s(a)), ("B", int(b))]);
    o.addressable = addressable;
    obj(o)
}

pub fn etx_pv(a: &str, b: i64) -> Value {
    obj(SObj::new(
        "main.EtxV",
        true,
        vec![("A", s(a)), ("B", int(b))],
    ))
}

/// Go: `EtxT{...}` fields in declaration order. `v_addressable`: the `V`
/// field of a struct reached through a pointer is addressable.
#[allow(clippy::too_many_arguments)]
fn etx_t_fields(
    name: impl AsRef<[u8]>,
    n: i64,
    f: f64,
    b: bool,
    sub: Value,
    v: Value,
    pv: Value,
    iface: Value,
    err_: Value,
    str_: Value,
    m: Value,
    ss: Value,
    l: Value,
    fn_: Value,
    pages: Value,
    time: Time,
    h: &str,
) -> Vec<(&'static str, Value)> {
    vec![
        ("Name", s(name.as_ref())),
        ("N", int(n)),
        ("F", Value::float64(f)),
        ("B", Value::Bool(b)),
        ("Sub", sub),
        ("NilSub", tnil("*main.EtxT")),
        ("V", v),
        ("PV", pv),
        ("Iface", iface),
        ("NilIface", nil_any()),
        ("Err", err_),
        ("NilErr", tnil("error")),
        ("Str", str_),
        ("NilStr", tnil("fmt.Stringer")),
        ("M", m),
        ("NilM", tnil("map[string]interface {}")),
        ("SS", ss),
        ("NilSS", tnil("[]string")),
        ("L", l),
        ("Fn", fn_),
        ("NilFn", tnil("func(interface {}) interface {}")),
        ("Pages", pages),
        ("Time", Value::Time(time)),
        ("H", Value::Safe(SafeKind::Html, h.into())),
    ]
}

/// A zero `EtxT` except for the given name (Go: `EtxT{Name: name}`),
/// as a pointer or a (non-addressable) value.
pub fn etx_t_zero(name: impl AsRef<[u8]>, ptr: bool, v: Value) -> Value {
    obj(SObj::new(
        "main.EtxT",
        ptr,
        etx_t_fields(
            name,
            0,
            0.0,
            false,
            tnil("*main.EtxT"),
            v,
            tnil("*main.EtxV"),
            nil_any(),
            tnil("error"),
            tnil("fmt.Stringer"),
            tnil("map[string]interface {}"),
            tnil("[]string"),
            tnil("[]interface {}"),
            tnil("func(interface {}) interface {}"),
            tnil("main.EtxPages"),
            Time::zero(),
            "",
        ),
    ))
}

fn string_any_map(entries: Vec<(&str, Value)>) -> Value {
    map_of(MapType::StringAny, entries)
}

fn map_of(ty: MapType, entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new(ty);
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::map(m)
}

fn any_list(items: Vec<Value>) -> Value {
    Value::list(SliceType::Any, items)
}

fn fn_value() -> Value {
    func_value("func(interface {}) interface {}", |args| {
        check_args("", args, 1, false)?;
        Ok(s(sprint(&[s("fn:"), args[0].clone()])))
    })
}

/// The data of a case, the site params map and the site (Go: etxData).
pub struct Data {
    pub data: Value,
    pub site_params: Value,
    pub site: Value,
}

pub fn etx_root() -> (Value, Value, Value) {
    register();
    let sub = obj(SObj::new(
        "main.EtxT",
        true,
        etx_t_fields(
            "sub",
            2,
            0.0,
            false,
            tnil("*main.EtxT"),
            etx_v("sv", 3, true),
            tnil("*main.EtxV"),
            nil_any(),
            tnil("error"),
            tnil("fmt.Stringer"),
            tnil("map[string]interface {}"),
            tnil("[]string"),
            tnil("[]interface {}"),
            tnil("func(interface {}) interface {}"),
            tnil("main.EtxPages"),
            Time::zero(),
            "",
        ),
    ));
    let t = obj(SObj::new(
        "main.EtxT",
        true,
        etx_t_fields(
            "t",
            1,
            1.5,
            true,
            sub.clone(),
            etx_v("v", 7, true),
            etx_pv("pv", 8),
            int(42),
            obj(SObj::new("main.EtxErr", true, vec![("Msg", s("field"))])),
            obj(SObj::new("main.EtxStr", true, vec![("S", s("field"))])),
            string_any_map(vec![
                ("a", int(1)),
                ("b", s("two")),
                ("nil", Value::Invalid),
                ("Key", s("K")),
                ("key", s("k")),
            ]),
            Value::string_list(["x", "y"]),
            any_list(vec![int(1), s("two"), Value::Invalid]),
            fn_value(),
            Value::list(
                SliceType::Named(Arc::from("main.EtxPages")),
                vec![sub.clone()],
            ),
            time_date(2020, 5, 6, 7, 8, 9, 10),
            "<b>h</b>",
        ),
    ));
    let site_params = map_of(
        MapType::Params,
        vec![
            ("title", s("Site")),
            ("mainsections", Value::string_list(["from-params"])),
            (
                "social",
                map_of(
                    MapType::Params,
                    vec![(
                        "facebook",
                        map_of(MapType::Params, vec![("enable", Value::Bool(true))]),
                    )],
                ),
            ),
        ],
    );
    let site = obj(SObj::new("main.EtxSite", true, vec![("Title", s("Site"))]));
    let pages =
        |items: Vec<Value>| Value::list(SliceType::Named(Arc::from("main.EtxPages")), items);
    let root = string_any_map(vec![
        ("T", t.clone()),
        ("TV", etx_t_zero("tv", false, etx_v("tvv", 1, false))),
        ("NilT", tnil("*main.EtxT")),
        ("Nil", Value::Invalid),
        ("V", etx_v("val", 9, false)),
        ("PVal", etx_pv("ptr", 10)),
        ("Pages", pages(vec![t.clone(), sub.clone()])),
        ("EmptyPages", pages(vec![])),
        ("NilPages", tnil("main.EtxPages")),
        (
            "D",
            map_of(
                MapType::Named(Arc::from("main.EtxData")),
                vec![
                    ("Singular", s("s")),
                    ("Pages", s("key-pages")),
                    ("Count", s("key-count")),
                ],
            ),
        ),
        (
            "P",
            map_of(
                MapType::Params,
                vec![
                    ("title", s("Title")),
                    (
                        "ymap",
                        map_of(
                            MapType::Params,
                            vec![
                                ("b", int(2)),
                                ("c", map_of(MapType::Params, vec![("d", s("deep"))])),
                            ],
                        ),
                    ),
                    ("ynull", Value::Invalid),
                    ("list", any_list(vec![s("a"), int(1)])),
                    ("mixed_case", s("mc")),
                    ("iszero", s("key-iszero")),
                ],
            ),
        ),
        (
            "PMerge",
            map_of(MapType::Params, vec![("_merge", s("deep"))]),
        ),
        ("PEmpty", map_of(MapType::Params, vec![])),
        ("SiteParams", site_params.clone()),
        ("Site", site.clone()),
        (
            "M",
            string_any_map(vec![
                ("a", int(1)),
                ("b", s("two")),
                ("nil", Value::Invalid),
                ("Key", s("K")),
                ("key", s("k")),
                (
                    "sub",
                    string_any_map(vec![("x", s("X")), ("nil", Value::Invalid)]),
                ),
                ("list", any_list(vec![int(1), int(2), int(3)])),
                ("v", etx_v("inmap", 5, false)),
                ("empty", string_any_map(vec![])),
            ]),
        ),
        (
            "MS",
            map_of(MapType::StringString, vec![("a", s("A")), ("b", s("B"))]),
        ),
        (
            "L",
            any_list(vec![
                int(1),
                s("two"),
                Value::Invalid,
                Value::float64(3.5),
                Value::Bool(true),
                any_list(vec![s("n")]),
                string_any_map(vec![("k", s("v"))]),
            ]),
        ),
        ("SS", Value::string_list(["a", "b", "c"])),
        (
            "SI",
            Value::list(SliceType::Int, vec![int(3), int(4), int(5)]),
        ),
        (
            "SB",
            Value::list(SliceType::Bool, vec![Value::Bool(true), Value::Bool(false)]),
        ),
        (
            "SF",
            Value::list(
                SliceType::Float64,
                vec![Value::float64(1.5), Value::float64(2.0)],
            ),
        ),
        (
            "SBytes",
            Value::list(
                SliceType::Uint8,
                vec![
                    Value::Uint(b'h' as u64, UintKind::Uint8),
                    Value::Uint(b'i' as u64, UintKind::Uint8),
                ],
            ),
        ),
        (
            "SM",
            Value::list(
                SliceType::MapStringAny,
                vec![
                    string_any_map(vec![("a", int(1))]),
                    string_any_map(vec![("a", int(2))]),
                ],
            ),
        ),
        ("Empty", any_list(vec![])),
        ("EmptySS", Value::string_list(Vec::<&str>::new())),
        ("EmptyM", string_any_map(vec![])),
        ("NilM", tnil("map[string]interface {}")),
        ("NilSS", tnil("[]string")),
        ("I", int(42)),
        ("I8", Value::Int(-8, IntKind::Int8)),
        ("I16", Value::Int(-16, IntKind::Int16)),
        ("I32", Value::Int(-32, IntKind::Int32)),
        ("I64", Value::Int(-64, IntKind::Int64)),
        ("U", Value::Uint(7, UintKind::Uint)),
        ("U8", Value::Uint(255, UintKind::Uint8)),
        ("U16", Value::Uint(16, UintKind::Uint16)),
        ("U32", Value::Uint(32, UintKind::Uint32)),
        ("U64", Value::Uint(u64::MAX, UintKind::Uint64)),
        ("UP", Value::Uint(9, UintKind::Uintptr)),
        ("F32", Value::Float(1.5, FloatKind::F32)),
        ("F64", Value::float64(2.5)),
        ("FBig", Value::float64(1e21)),
        ("FSmall", Value::float64(1e-7)),
        ("MaxI", int(i64::MAX)),
        ("MinI", int(i64::MIN)),
        ("Neg", int(-3)),
        ("Zero", int(0)),
        ("ZeroU", Value::Uint(0, UintKind::Uint)),
        ("ZeroF", Value::float64(0.0)),
        ("NegZero", Value::float64(-0.0)),
        ("NaN", Value::float64(f64::NAN)),
        ("Inf", Value::float64(f64::INFINITY)),
        ("S", s("hello")),
        ("ES", s("")),
        ("Bad", s(&b"a\xffb"[..])),
        ("Uni", s("héllo wörld")),
        ("HTML", Value::Safe(SafeKind::Html, "<b>x</b>".into())),
        ("EHTML", Value::Safe(SafeKind::Html, "".into())),
        ("JS", Value::Safe(SafeKind::Js, "x<y".into())),
        ("True", Value::Bool(true)),
        ("False", Value::Bool(false)),
        ("Time", Value::Time(time_date(2021, 1, 2, 3, 4, 5, 6))),
        ("ZeroTime", Value::Time(Time::zero())),
        (
            "Z",
            obj(SObj::new(
                "main.EtxZ",
                false,
                vec![("Zero", Value::Bool(true)), ("Label", s("z"))],
            )),
        ),
        (
            "NZ",
            obj(SObj::new(
                "main.EtxZ",
                false,
                vec![("Zero", Value::Bool(false)), ("Label", s("nz"))],
            )),
        ),
        (
            "Str",
            obj(SObj::new("main.EtxStr", true, vec![("S", s("s"))])),
        ),
        (
            "SV",
            obj(SObj::new("main.EtxSV", false, vec![("S", s("v"))])),
        ),
        (
            "Err",
            obj(SObj::new("main.EtxErr", true, vec![("Msg", s("e"))])),
        ),
        ("Fn", fn_value()),
        (
            "Fn2",
            func_value(
                "func(interface {}, interface {}) (interface {}, error)",
                |args| {
                    check_args("", args, 2, false)?;
                    Err(err("fn2 failed"))
                },
            ),
        ),
        ("NilFn", tnil("func(interface {}) interface {}")),
        (
            "Nested",
            string_any_map(vec![(
                "a",
                string_any_map(vec![("b", string_any_map(vec![("c", s("abc"))]))]),
            )]),
        ),
        (
            "NumKeys",
            string_any_map(vec![
                ("1", s("one")),
                ("10", s("ten")),
                ("2", s("two")),
                ("A", s("a")),
                ("a", s("lower")),
                ("_", s("u")),
                ("é", s("e")),
            ]),
        ),
    ]);
    (root, site_params, site)
}

/// Go: etxData.
pub fn etx_data(kind: &str) -> Data {
    let (root, site_params, site) = etx_root();
    let field = |k: &str| root.as_map().unwrap().get(k.as_bytes()).unwrap().clone();
    let data = match kind {
        "root" => root.clone(),
        "nil" => Value::Invalid,
        "int" => int(7),
        "string" => s("str"),
        "typednil" => tnil("*main.EtxT"),
        "t" => field("T"),
        "list" => field("L"),
        "params" => field("P"),
        "siteparams" => site_params.clone(),
        _ => panic!("unknown data {kind}"),
    };
    Data {
        data,
        site_params,
        site,
    }
}

// ---------------------------------------------------------------------------
// Methods (Go: the method sets reflection sees).

/// Go `time.Time`'s exported methods (value receivers, and the pointer
/// receivers of `*time.Time`).
const TIME_METHODS: &[&str] = &[
    "Add",
    "AddDate",
    "After",
    "AppendBinary",
    "AppendFormat",
    "AppendText",
    "Before",
    "Clock",
    "Compare",
    "Date",
    "Day",
    "Equal",
    "Format",
    "GoString",
    "GobEncode",
    "Hour",
    "ISOWeek",
    "In",
    "IsDST",
    "IsZero",
    "Local",
    "Location",
    "MarshalBinary",
    "MarshalJSON",
    "MarshalText",
    "Minute",
    "Month",
    "Nanosecond",
    "Round",
    "Second",
    "String",
    "Sub",
    "Truncate",
    "UTC",
    "Unix",
    "UnixMicro",
    "UnixMilli",
    "UnixNano",
    "Weekday",
    "Year",
    "YearDay",
    "Zone",
    "ZoneBounds",
];

/// (value-receiver methods, pointer-receiver methods) of a Go type.
fn method_set(ty: &str) -> (&'static [&'static str], &'static [&'static str]) {
    match ty {
        "main.EtxT" => (
            &["VM"],
            &[
                "PM",
                "Echo",
                "Two",
                "Var",
                "Fail",
                "NilOK",
                "Self",
                "GetSub",
                "RetNil",
                "RetNilPtr",
                "RetNilStr",
                "RetNilMap",
                "RetM",
                "Panic",
            ],
        ),
        "main.EtxV" => (&["VV"], &["PV2"]),
        "main.EtxPages" => (&["First", "Len", "Reverse"], &[]),
        "main.EtxData" => (&["Count", "Pages"], &[]),
        "main.EtxZ" => (&["IsZero"], &[]),
        "main.EtxStr" => (&[], &["String"]),
        "main.EtxSV" => (&["String"], &[]),
        "main.EtxErr" => (&[], &["Error"]),
        "main.EtxSite" => (&[], &["MainSections"]),
        "errors.errorString" => (&[], &["Error"]),
        "maps.Params" => (
            &[
                "DeleteMergeStrategy",
                "GetMergeStrategy",
                "GetNested",
                "IsZero",
                "SetMergeStrategy",
            ],
            &[],
        ),
        "time.Time" => (
            TIME_METHODS,
            &[
                "GobDecode",
                "UnmarshalBinary",
                "UnmarshalJSON",
                "UnmarshalText",
            ],
        ),
        _ => super::gotests::method_set(ty),
    }
}

fn has_method_on(ty: &str, ptr_set: bool, name: &str) -> bool {
    let (v, p) = method_set(ty);
    v.contains(&name) || (ptr_set && p.contains(&name))
}

/// The Go type (without `*`) and whether the method set is the pointer's,
/// for a receiver that has methods.
fn receiver_type(recv: &Value) -> Option<(Cow<'_, str>, bool)> {
    match recv {
        Value::Object(o) => {
            if let Some(so) = o.as_any().downcast_ref::<SObj>() {
                return Some((Cow::Borrowed(so.ty), so.ptr_set()));
            }
            if o.as_any().downcast_ref::<GoErr>().is_some() {
                return Some((Cow::Borrowed("errors.errorString"), true));
            }
            None
        }
        Value::TypedNil(t) => t
            .strip_prefix('*')
            .map(|b| (Cow::Owned(b.to_string()), true))
            .or_else(|| {
                // A nil named slice/map still has its value methods.
                Some((Cow::Owned(t.to_string()), false))
            }),
        Value::List(l) => match &l.ty {
            SliceType::Named(n) => Some((Cow::Borrowed(n), false)),
            _ => None,
        },
        Value::Map(m) => match &m.ty {
            MapType::Params => Some((Cow::Borrowed("maps.Params"), false)),
            MapType::Named(n) => Some((Cow::Borrowed(n), false)),
            _ => None,
        },
        Value::Time(_) => Some((Cow::Borrowed("time.Time"), false)),
        _ => None,
    }
}

/// Go reflection: whether `recv` has the exported method `name`. Objects
/// of other crates (the engine's `TryValue`/`TryError`) answer themselves.
pub fn reflect_has_method(recv: &Value, name: &str) -> bool {
    if let Some(o) = foreign_object(recv) {
        return o.has_method(name);
    }
    match receiver_type(recv) {
        Some((ty, ptr_set)) => has_method_on(&ty, ptr_set, name),
        None => false,
    }
}

/// An object that is not one of this model's types.
fn foreign_object(recv: &Value) -> Option<&Arc<dyn Object>> {
    let o = recv.as_object()?;
    if o.as_any().downcast_ref::<SObj>().is_some() || o.as_any().downcast_ref::<GoErr>().is_some() {
        return None;
    }
    Some(o)
}

/// Go `evalCall`'s arity check, which Go does before calling (here the
/// host does it, so the engine reports it as a call error).
pub fn check_args(
    name: &str,
    args: &[Value],
    num_in: usize,
    variadic: bool,
) -> go_value::Result<()> {
    if variadic {
        if args.len() < num_in - 1 {
            return Err(err(format!(
                "wrong number of args for {name}: want at least {} got {}",
                num_in - 1,
                args.len()
            )));
        }
    } else if args.len() != num_in {
        return Err(err(format!(
            "wrong number of args for {name}: want {num_in} got {}",
            args.len()
        )));
    }
    Ok(())
}

fn nil_deref() -> go_value::Error {
    err("runtime error: invalid memory address or nil pointer dereference")
}

/// Go's result for a method returning `any`: a nil interface keeps its
/// interface kind.
fn ret_any(v: &Value) -> Value {
    if v.is_invalid() { nil_any() } else { v.clone() }
}

fn want_string(v: &Value) -> Option<Vec<u8>> {
    match v {
        Value::String(s) => Some(s.to_vec()),
        _ => None,
    }
}

// Go: common/maps/params.go:getNested
fn get_nested(m: &Value, indices: &[Vec<u8>]) -> Value {
    if indices.is_empty() {
        return Value::Invalid;
    }
    let Some(map) = m.as_map() else {
        return Value::Invalid;
    };
    let first = go_unicode::strings::to_lower(&indices[0]);
    let Some(v) = map.get(&first) else {
        return Value::Invalid;
    };
    if indices.len() == 1 {
        return v.clone();
    }
    match v {
        Value::Map(m2) if matches!(m2.ty, MapType::Params | MapType::StringAny) => {
            get_nested(v, &indices[1..])
        }
        _ => Value::Invalid,
    }
}

/// Calls the method `name` of `recv` (found by [`reflect_has_method`]).
pub fn call(recv: &Value, name: &str, args: &[Value]) -> GoResult {
    if let Some(o) = foreign_object(recv) {
        return o
            .call_method(&(), name, args)
            .unwrap_or_else(|| Err(err(format!("method {name} not found"))));
    }
    let (ty, _) = receiver_type(recv).ok_or_else(|| err("no methods"))?;
    let so = recv.downcast::<SObj>();
    let nil = matches!(recv, Value::TypedNil(_));
    match (&*ty, name) {
        ("main.EtxT", _) => {
            let field = |f: &str| so.and_then(|o| o.get(f)).cloned().unwrap_or(Value::Invalid);
            match name {
                "VM" => {
                    check_args(name, args, 0, false)?;
                    if nil {
                        return Err(err(
                            "value method main.EtxT.VM called using nil *EtxT pointer",
                        ));
                    }
                    Ok(s(format!("VM:{}", so.unwrap().s("Name"))))
                }
                "PM" => {
                    check_args(name, args, 0, false)?;
                    let o = so.ok_or_else(nil_deref)?;
                    Ok(s(format!("PM:{}", o.s("Name"))))
                }
                "Echo" => {
                    check_args(name, args, 1, false)?;
                    Ok(ret_any(&args[0]))
                }
                "Two" => {
                    check_args(name, args, 2, false)?;
                    Ok(s(sprint(&[args[0].clone(), s("|"), args[1].clone()])))
                }
                "Var" => {
                    check_args(name, args, 1, true)?;
                    let list = any_list(args.to_vec());
                    Ok(s(sprintf("%d%v", &[int(args.len() as i64), list])))
                }
                "Fail" => {
                    check_args(name, args, 0, false)?;
                    Err(err("method failed"))
                }
                "NilOK" => {
                    check_args(name, args, 0, false)?;
                    match so {
                        None => Ok(s("nil receiver")),
                        Some(o) => Ok(s(format!("non-nil:{}", o.s("Name")))),
                    }
                }
                "Self" => {
                    check_args(name, args, 0, false)?;
                    Ok(recv.clone())
                }
                "GetSub" => {
                    check_args(name, args, 0, false)?;
                    so.ok_or_else(nil_deref)?;
                    Ok(field("Sub"))
                }
                "RetNil" => {
                    check_args(name, args, 0, false)?;
                    Ok(nil_any())
                }
                "RetNilPtr" => {
                    check_args(name, args, 0, false)?;
                    Ok(tnil("*main.EtxT"))
                }
                "RetNilStr" => {
                    check_args(name, args, 0, false)?;
                    Ok(tnil("fmt.Stringer"))
                }
                "RetNilMap" => {
                    check_args(name, args, 0, false)?;
                    Ok(tnil("map[string]interface {}"))
                }
                "RetM" => {
                    check_args(name, args, 0, false)?;
                    so.ok_or_else(nil_deref)?;
                    Ok(field("M"))
                }
                "Panic" => {
                    check_args(name, args, 0, false)?;
                    Err(err("method panic"))
                }
                _ => Err(err("no such method")),
            }
        }
        ("main.EtxV", "VV") => {
            check_args(name, args, 0, false)?;
            let o =
                so.ok_or_else(|| err("value method main.EtxV.VV called using nil *EtxV pointer"))?;
            Ok(s(format!("VV:{}", o.s("A"))))
        }
        ("main.EtxV", "PV2") => {
            check_args(name, args, 0, false)?;
            let o = so.ok_or_else(nil_deref)?;
            Ok(s(format!("PV:{}", o.s("A"))))
        }
        ("main.EtxPages", _) => {
            let items: Vec<Value> = recv.as_list().map(|l| l.items.clone()).unwrap_or_default();
            check_args(name, args, 0, false)?;
            match name {
                "Len" => Ok(int(items.len() as i64)),
                "First" => Ok(items.first().cloned().unwrap_or_else(|| tnil("*main.EtxT"))),
                "Reverse" => {
                    let mut r = items;
                    r.reverse();
                    Ok(Value::list(SliceType::Named(Arc::from("main.EtxPages")), r))
                }
                _ => Err(err("no such method")),
            }
        }
        ("main.EtxData", _) => {
            check_args(name, args, 0, false)?;
            match name {
                "Pages" => Ok(s("method-pages")),
                "Count" => Ok(int(recv.as_map().map(|m| m.len()).unwrap_or(0) as i64)),
                _ => Err(err("no such method")),
            }
        }
        ("main.EtxZ", "IsZero") => {
            check_args(name, args, 0, false)?;
            Ok(Value::Bool(so.unwrap().is_zero().unwrap()))
        }
        ("main.EtxStr", "String") => {
            check_args(name, args, 0, false)?;
            let o = so.ok_or_else(nil_deref)?;
            Ok(s(format!("Str({})", o.s("S"))))
        }
        ("main.EtxSV", "String") => {
            check_args(name, args, 0, false)?;
            Ok(s(format!("SV({})", so.unwrap().s("S"))))
        }
        ("main.EtxErr", "Error") => {
            check_args(name, args, 0, false)?;
            let o = so.ok_or_else(nil_deref)?;
            Ok(s(format!("Err({})", o.s("Msg"))))
        }
        ("main.EtxSite", "MainSections") => {
            check_args(name, args, 0, false)?;
            Ok(Value::string_list(["posts", "blog"]))
        }
        ("errors.errorString", "Error") => {
            check_args(name, args, 0, false)?;
            let e = recv.downcast::<GoErr>().unwrap();
            Ok(s(e.0.as_str()))
        }
        ("maps.Params", _) => match name {
            "IsZero" => {
                check_args(name, args, 0, false)?;
                let m = recv.as_map();
                let zero = match m {
                    None => true,
                    Some(m) => {
                        m.entries.is_empty()
                            || (m.entries.len() == 1
                                && m.entries.keys().next().unwrap().as_bytes() == b"_merge")
                    }
                };
                Ok(Value::Bool(zero))
            }
            "GetNested" => {
                let mut idx = Vec::new();
                for a in args {
                    idx.push(want_string(a).ok_or_else(|| err("GetNested: want strings"))?);
                }
                Ok(ret_any(&get_nested(recv, &idx)))
            }
            _ => Err(err(format!("maps.Params.{name} not modelled"))),
        },
        ("time.Time", _) => {
            let Value::Time(t) = recv else {
                return Err(err("not a time"));
            };
            match name {
                "Format" => {
                    check_args(name, args, 1, false)?;
                    let layout = want_string(&args[0]).ok_or_else(|| err("Format: want string"))?;
                    Ok(s(t.format_bytes(&layout)))
                }
                "IsZero" => {
                    check_args(name, args, 0, false)?;
                    Ok(Value::Bool(t.go_is_zero()))
                }
                "Year" => {
                    check_args(name, args, 0, false)?;
                    Ok(int(t.year()))
                }
                "Day" => {
                    check_args(name, args, 0, false)?;
                    Ok(int(t.day()))
                }
                "Unix" => {
                    check_args(name, args, 0, false)?;
                    Ok(Value::int64(t.go_unix()))
                }
                "UTC" => {
                    check_args(name, args, 0, false)?;
                    Ok(Value::Time(t.utc()))
                }
                "String" => {
                    check_args(name, args, 0, false)?;
                    Ok(s(GoTimeExt::string(t)))
                }
                _ => Err(err(format!("time.Time.{name} not modelled"))),
            }
        }
        _ => super::gotests::call(recv, &ty, name, args),
    }
}

// ---------------------------------------------------------------------------
// Functions (Go: etxFuncs).

fn f(g: impl Fn(&[Value]) -> GoResult + Send + Sync + 'static) -> Func {
    Arc::new(move |_ctx: HostCtx<'_>, args: &[Value]| g(args))
}

pub fn etx_funcs() -> BTreeMap<&'static str, Func> {
    let mut m: BTreeMap<&'static str, Func> = BTreeMap::new();
    m.insert(
        "echo",
        f(|a| {
            check_args("echo", a, 1, false)?;
            Ok(ret_any(&a[0]))
        }),
    );
    m.insert(
        "echo2",
        f(|a| {
            check_args("echo2", a, 2, false)?;
            Ok(s(sprint(&[a[0].clone(), s("|"), a[1].clone()])))
        }),
    );
    m.insert(
        "fail",
        f(|a| {
            check_args("fail", a, 1, false)?;
            Err(err(String::from_utf8_lossy(&sprint(&a[..1]))))
        }),
    );
    m.insert(
        "failNil",
        f(|a| {
            check_args("failNil", a, 0, false)?;
            Ok(nil_any())
        }),
    );
    m.insert(
        "failErr",
        f(|a| {
            check_args("failErr", a, 0, false)?;
            Err(err("Err(typed)"))
        }),
    );
    m.insert(
        "nilany",
        f(|a| {
            check_args("nilany", a, 0, false)?;
            Ok(nil_any())
        }),
    );
    for (name, ty) in [
        ("nilptr", "*main.EtxT"),
        ("nilerr", "error"),
        ("nilstr", "fmt.Stringer"),
        ("nilmap", "map[string]interface {}"),
        ("nilslice", "[]string"),
    ] {
        m.insert(
            name,
            f(move |a| {
                check_args(name, a, 0, false)?;
                Ok(tnil(ty))
            }),
        );
    }
    m.insert(
        "mkhtml",
        f(|a| {
            check_args("mkhtml", a, 1, false)?;
            Ok(Value::Safe(SafeKind::Html, GoString::from(sprint(&a[..1]))))
        }),
    );
    m.insert(
        "try",
        f(|a| {
            check_args("try", a, 1, false)?;
            Ok(ret_any(&a[0]))
        }),
    );
    m.insert(
        "typeof",
        f(|a| {
            check_args("typeof", a, 1, false)?;
            Ok(s(sprintf("%T", &a[..1])))
        }),
    );
    m.insert(
        "isnil",
        f(|a| {
            check_args("isnil", a, 1, false)?;
            Ok(Value::Bool(a[0].is_invalid()))
        }),
    );
    m.insert(
        "variadic",
        f(|a| {
            check_args("variadic", a, 1, true)?;
            Ok(s(sprintf(
                "%d:%v",
                &[int(a.len() as i64), any_list(a.to_vec())],
            )))
        }),
    );
    m.insert(
        "panicky",
        f(|a| {
            check_args("panicky", a, 0, false)?;
            Err(err("host panic"))
        }),
    );
    m.insert(
        "panicerr",
        f(|a| {
            check_args("panicerr", a, 0, false)?;
            Err(err("host panic err"))
        }),
    );
    m.insert(
        "errval",
        f(|a| {
            check_args("errval", a, 0, false)?;
            Ok(Value::object(GoErr("an error value".to_string())))
        }),
    );
    m.insert(
        "dict",
        f(|a| {
            check_args("dict", a, 1, true)?;
            if a.len() % 2 != 0 {
                return Err(err("invalid dictionary call"));
            }
            let mut mm = Map::new(MapType::StringAny);
            for kv in a.chunks(2) {
                let Value::String(k) = &kv[0] else {
                    return Err(err("dictionary keys must be strings"));
                };
                mm.insert(k.clone(), kv[1].clone());
            }
            Ok(Value::map(mm))
        }),
    );
    m.insert(
        "list",
        f(|a| {
            check_args("list", a, 1, true)?;
            Ok(any_list(a.to_vec()))
        }),
    );
    m.insert(
        "mkT",
        f(|a| {
            check_args("mkT", a, 1, false)?;
            Ok(etx_t_zero(sprint(&a[..1]), true, etx_v("", 0, true)))
        }),
    );
    m.insert(
        "getfn",
        f(|a| {
            check_args("getfn", a, 0, false)?;
            Ok(func_value("func(interface {}) interface {}", |args| {
                check_args("", args, 1, false)?;
                Ok(s(sprint(&[s("got:"), args[0].clone()])))
            }))
        }),
    );
    m.insert(
        "strarg",
        f(|a| {
            check_args("strarg", a, 1, false)?;
            let v = want_string(&a[0]).ok_or_else(|| err("strarg: want string"))?;
            let mut out = b"s:".to_vec();
            out.extend_from_slice(&v);
            Ok(s(out))
        }),
    );
    m.insert(
        "intarg",
        f(|a| {
            check_args("intarg", a, 1, false)?;
            match &a[0] {
                Value::Int(i, IntKind::Int) => Ok(int(i.wrapping_mul(2))),
                _ => Err(err("intarg: want int")),
            }
        }),
    );
    m
}

/// The funcs as a template `FuncMap` (plain mode: Go's `Funcs`).
pub fn etx_func_map() -> FuncMap {
    etx_funcs()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

// ---------------------------------------------------------------------------
// Helpers.

/// Go's template execution without a helper: reflection lookups
/// (exact-key `MapIndex`, `MethodByName`).
pub struct ReflectHelper;

impl ExecHelper for ReflectHelper {
    fn has_method(&self, _ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool {
        reflect_has_method(receiver, name)
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        receiver: &Value,
        name: &str,
        args: &[Value],
    ) -> GoResult {
        call(receiver, name, args)
    }
    fn get_map_value(&self, _ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value> {
        let key = key.as_go_string()?;
        receiver.as_map()?.get(key).cloned()
    }
}

/// Go: etxHelper (Hugo's templateExecHelper).
pub struct HugoHelper {
    pub site: Value,
    pub site_params: Value,
    pub funcs: BTreeMap<String, Func>,
}

impl HugoHelper {
    pub fn new(site: Value, site_params: Value) -> HugoHelper {
        let mut funcs: BTreeMap<String, Func> = etx_funcs()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        // As Hugo's configureSiteStorage: the builtins only if not present.
        for (k, v) in gotemplate::text::go_funcs() {
            funcs.entry(k.to_string()).or_insert(v);
        }
        HugoHelper {
            site,
            site_params,
            funcs,
        }
    }

    fn is_site_params(&self, receiver: &Value) -> bool {
        match (receiver, &self.site_params) {
            (Value::Map(a), Value::Map(b)) => a.ty == MapType::Params && Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl ExecHelper for HugoHelper {
    fn get_func(&self, _ctx: HostCtx<'_>, name: &str) -> Option<Func> {
        self.funcs.get(name).cloned()
    }
    fn has_method(&self, _ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool {
        if go_unicode::strings::equal_fold_str(name, "mainsections")
            && self.is_site_params(receiver)
        {
            return true;
        }
        reflect_has_method(receiver, name)
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        receiver: &Value,
        name: &str,
        args: &[Value],
    ) -> GoResult {
        if go_unicode::strings::equal_fold_str(name, "mainsections")
            && self.is_site_params(receiver)
        {
            return call(&self.site, "MainSections", args);
        }
        call(receiver, name, args)
    }
    fn get_map_value(&self, _ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value> {
        let key = key.as_go_string()?;
        let m = receiver.as_map()?;
        if m.ty == MapType::Params {
            // Case insensitive; reflect.ValueOf(nil) is invalid (as missing).
            let v = m.get(&go_unicode::strings::to_lower(key))?;
            if v.is_invalid() {
                return None;
            }
            return Some(v.clone());
        }
        m.get(key).cloned()
    }
}
