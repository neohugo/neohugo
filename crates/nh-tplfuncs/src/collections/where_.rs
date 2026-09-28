//! Port of `tpl/collections/where.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! `reflect.Value` results are `Option<Value>`: `None` is Go's invalid (zero) `reflect.Value`
//! (a missing map key, a failed path step), `Some(Value::Invalid)` a valid value holding a nil
//! interface (a `nil` element of a `[]interface {}`, a map value that is nil).

use std::sync::Arc;

use go_value::{
    GoString, HostCtx, Kind, List, Map, MapType, NilKind, SliceType, Value, typed_nil_kind,
};
use nh_common::hreflect::{self, ReflectKind};
use nh_common::object::GoResult;

use super::collections::Namespace;
use super::reflect_helpers::{
    self as rh, SliceVal, as_slice, bool_of, float_of, indirect_kind, int_of, kind, string_of,
    to_float, to_int, to_string,
};

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

/// The outcome of `evaluateSubElem` that is not a value: an error (Go `error`, ignored by
/// `where`) or a Go runtime panic (propagated).
pub(crate) enum EvalFail {
    Err(String),
    Panic(go_value::Error),
}

impl From<EvalFail> for go_value::Error {
    fn from(f: EvalFail) -> go_value::Error {
        match f {
            EvalFail::Err(s) => go_value::Error::new(s),
            EvalFail::Panic(e) => e,
        }
    }
}

/// The Go type string of a value used as `obj.Type()` in `evaluateSubElem` messages: the static
/// type when known, else the dynamic one.
pub(crate) fn dyn_type(v: &Value) -> String {
    v.go_type_name().into_owned()
}

impl Namespace {
    // Go: tpl/collections/where.go:Where
    /// Where returns a filtered subset of collection c.
    pub fn do_where(
        &self,
        ctx: HostCtx<'_>,
        c: &Value,
        key: &Value,
        args: &[Value],
    ) -> GoResult<Value> {
        let (seqv, is_nil) = rh::indirect(c);
        if is_nil {
            return Err(err(format!(
                "can't iterate over a nil value of type {}",
                c.go_type_name()
            )));
        }

        let (mv, op) = parse_where_args(args)?;

        let mut path: Vec<GoString> = Vec::new();
        if kind(key) == ReflectKind::String {
            let s = string_of(key);
            let trimmed = go_unicode::strings::trim(&s, b".");
            path = go_unicode::strings::split(trimmed, b".")
                .into_iter()
                .map(GoString::from)
                .collect();
        }

        match kind(&seqv) {
            ReflectKind::Slice => {
                let sl = as_slice(&seqv).unwrap();
                self.check_where_array(ctx, &sl, key, mv.as_ref(), &path, &op)
            }
            ReflectKind::Map => self.check_where_map(ctx, &seqv, key, mv.as_ref(), &path, &op),
            _ => Err(err(format!("can't iterate over {}", c.go_type_name()))),
        }
    }

    // Go: tpl/collections/where.go:checkCondition
    pub(crate) fn check_condition(
        &self,
        v: Option<&Value>,
        mv: Option<&Value>,
        op: &str,
    ) -> GoResult<bool> {
        let v_is_nil = match v {
            None => true,
            Some(v) => cond_is_nil(v),
        };
        let mv_is_nil = match mv {
            None => true,
            Some(mv) => cond_is_nil(mv),
        };
        if v_is_nil || mv_is_nil {
            return Ok(match op {
                "" | "=" | "==" | "eq" => v_is_nil == mv_is_nil,
                "!=" | "<>" | "ne" => v_is_nil != mv_is_nil,
                _ => false,
            });
        }
        let v = v.unwrap();
        let mv = mv.unwrap();
        let vk = indirect_kind(v);
        let mvk = indirect_kind(mv);

        if vk == ReflectKind::Bool && mvk == ReflectKind::Bool {
            return Ok(match op {
                "" | "=" | "==" | "eq" => bool_of(v) == bool_of(mv),
                "!=" | "<>" | "ne" => bool_of(v) != bool_of(mv),
                _ => false,
            });
        }

        let mut ivp: Option<i64> = None;
        let mut imvp: Option<i64> = None;
        let mut fvp: Option<f64> = None;
        let mut fmvp: Option<f64> = None;
        let mut svp: Option<GoString> = None;
        let mut smvp: Option<GoString> = None;
        let mut slv = Value::Invalid;
        let mut slmv = Value::Invalid;
        let mut ima: Vec<i64> = Vec::new();
        let mut fma: Vec<f64> = Vec::new();
        let mut sma: Vec<GoString> = Vec::new();

        if mvk == vk {
            match vk {
                ReflectKind::Int(_) => {
                    ivp = Some(int_of(v));
                    imvp = Some(int_of(mv));
                }
                ReflectKind::String => {
                    svp = Some(string_of(v));
                    smvp = Some(string_of(mv));
                }
                ReflectKind::Float64 => {
                    fvp = Some(float_of(v));
                    fmvp = Some(float_of(mv));
                }
                ReflectKind::Struct => {
                    if hreflect::is_time(v) {
                        ivp = Some(self.to_time_unix(v));
                        imvp = Some(self.to_time_unix(mv));
                    }
                }
                ReflectKind::Slice => {
                    slv = v.clone();
                    slmv = mv.clone();
                }
                _ => {}
            }
        } else if rh::is_number(vk) && rh::is_number(mvk) {
            fvp = Some(to_float(v)?);
            fmvp = Some(to_float(mv)?);
        } else {
            if mvk != ReflectKind::Slice {
                return Ok(false);
            }

            let msl = as_slice(mv).unwrap();
            if msl.items.is_empty() {
                if op == "not in" {
                    return Ok(true);
                }
                return Ok(false);
            }

            let m_elem = msl.elem_type();
            if !rh::elem_is_interface(&m_elem) && m_elem != dyn_type(v) && vk != ReflectKind::Slice
            {
                return Ok(false);
            }
            match vk {
                ReflectKind::Int(_) => {
                    ivp = Some(int_of(v));
                    for item in &msl.items {
                        if let Ok(an_int) = to_int(item) {
                            ima.push(an_int);
                        }
                    }
                }
                ReflectKind::String => {
                    svp = Some(string_of(v));
                    for item in &msl.items {
                        if let Ok(a_string) = to_string(item) {
                            sma.push(a_string);
                        }
                    }
                }
                ReflectKind::Float64 => {
                    fvp = Some(float_of(v));
                    for item in &msl.items {
                        if let Ok(a_float) = to_float(item) {
                            fma.push(a_float);
                        }
                    }
                }
                ReflectKind::Struct => {
                    if hreflect::is_time(v) {
                        ivp = Some(self.to_time_unix(v));
                        for item in &msl.items {
                            ima.push(self.to_time_unix_checked(item)?);
                        }
                    }
                }
                ReflectKind::Slice => {
                    slv = v.clone();
                    slmv = mv.clone();
                }
                _ => {}
            }
        }

        match op {
            "" | "=" | "==" | "eq" => {
                if let (Some(a), Some(b)) = (ivp, imvp) {
                    return Ok(a == b);
                } else if let (Some(a), Some(b)) = (&svp, &smvp) {
                    return Ok(a == b);
                } else if let (Some(a), Some(b)) = (fvp, fmvp) {
                    return Ok(a == b);
                }
            }
            "!=" | "<>" | "ne" => {
                if let (Some(a), Some(b)) = (ivp, imvp) {
                    return Ok(a != b);
                } else if let (Some(a), Some(b)) = (&svp, &smvp) {
                    return Ok(a != b);
                } else if let (Some(a), Some(b)) = (fvp, fmvp) {
                    return Ok(a != b);
                }
            }
            ">=" | "ge" => {
                if let (Some(a), Some(b)) = (ivp, imvp) {
                    return Ok(a >= b);
                } else if let (Some(a), Some(b)) = (&svp, &smvp) {
                    return Ok(a.as_bytes() >= b.as_bytes());
                } else if let (Some(a), Some(b)) = (fvp, fmvp) {
                    return Ok(a >= b);
                }
            }
            ">" | "gt" => {
                if let (Some(a), Some(b)) = (ivp, imvp) {
                    return Ok(a > b);
                } else if let (Some(a), Some(b)) = (&svp, &smvp) {
                    return Ok(a.as_bytes() > b.as_bytes());
                } else if let (Some(a), Some(b)) = (fvp, fmvp) {
                    return Ok(a > b);
                }
            }
            "<=" | "le" => {
                if let (Some(a), Some(b)) = (ivp, imvp) {
                    return Ok(a <= b);
                } else if let (Some(a), Some(b)) = (&svp, &smvp) {
                    return Ok(a.as_bytes() <= b.as_bytes());
                } else if let (Some(a), Some(b)) = (fvp, fmvp) {
                    return Ok(a <= b);
                }
            }
            "<" | "lt" => {
                if let (Some(a), Some(b)) = (ivp, imvp) {
                    return Ok(a < b);
                } else if let (Some(a), Some(b)) = (&svp, &smvp) {
                    return Ok(a.as_bytes() < b.as_bytes());
                } else if let (Some(a), Some(b)) = (fvp, fmvp) {
                    return Ok(a < b);
                }
            }
            "in" | "not in" => {
                let r = if let (Some(iv), false) = (ivp, ima.is_empty()) {
                    self.do_in(
                        &Value::list(
                            SliceType::Int64,
                            ima.into_iter().map(Value::int64).collect(),
                        ),
                        &Value::int64(iv),
                    )
                    .unwrap_or(false)
                } else if let (Some(fv), false) = (fvp, fma.is_empty()) {
                    self.do_in(
                        &Value::list(
                            SliceType::Float64,
                            fma.into_iter().map(Value::float64).collect(),
                        ),
                        &Value::float64(fv),
                    )
                    .unwrap_or(false)
                } else if let Some(sv) = &svp {
                    if !sma.is_empty() {
                        self.do_in(&Value::string_list(sma), &Value::String(sv.clone()))
                            .unwrap_or(false)
                    } else if let Some(smv) = &smvp {
                        self.do_in(&Value::String(smv.clone()), &Value::String(sv.clone()))
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    return Ok(false);
                };
                if op == "not in" {
                    return Ok(!r);
                }
                return Ok(r);
            }
            "intersect" => {
                let r = self.do_intersect(&slv, &slmv)?;
                if kind(&r) == ReflectKind::Slice {
                    return Ok(rh::len_of(&r) > 0);
                }
                return Err(err("invalid intersect values"));
            }
            "like" => {
                if let (Some(_sv), Some(smv)) = (&svp, &smvp) {
                    nh_common::hstrings::get_or_compile_regexp(&smv.to_str_lossy())?;
                    // Unreachable until a Go regexp port exists (the stub above errors).
                    return Ok(false);
                }
            }
            _ => return Err(err("no such operator")),
        }
        Ok(false)
    }

    // Go: tpl/collections/where.go:checkWhereArray
    /// checkWhereArray handles the where-matching logic when the seqv value is an Array or
    /// Slice.
    pub(crate) fn check_where_array(
        &self,
        ctx: HostCtx<'_>,
        seqv: &SliceVal,
        kv: &Value,
        mv: Option<&Value>,
        path: &[GoString],
        op: &str,
    ) -> GoResult<Value> {
        let mut rv: Vec<Value> = Vec::new();
        let elem = seqv.elem_type();

        for rvv in &seqv.items {
            let mut vvv: Option<Value> = None;

            if kind(kv) == ReflectKind::String {
                if let Value::Map(m) = rvv
                    && m.ty == MapType::Params
                {
                    let idx: Vec<&[u8]> = path.iter().map(|p| p.as_bytes()).collect();
                    vvv = value_of(nh_common::maps::params::get_nested(m, &idx));
                } else {
                    vvv = Some(rvv.clone());
                    let mut typ = elem.clone();
                    for (i, elem_name) in path.iter().enumerate() {
                        match evaluate_sub_elem(ctx, vvv.as_ref(), &typ, elem_name) {
                            Ok((v, t)) => {
                                vvv = v;
                                typ = t;
                            }
                            Err(EvalFail::Err(_)) => {
                                vvv = None;
                                continue;
                            }
                            Err(EvalFail::Panic(e)) => return Err(e),
                        }

                        if i < path.len() - 1
                            && let Some(Value::Map(m)) = &vvv
                            && m.ty == MapType::Params
                        {
                            // The current path element is the map itself, .Params.
                            let idx: Vec<&[u8]> =
                                path[i + 1..].iter().map(|p| p.as_bytes()).collect();
                            vvv = value_of(nh_common::maps::params::get_nested(m, &idx));
                            break;
                        }
                    }
                }
            } else {
                let vv = rvv;
                if indirect_kind(vv) == ReflectKind::Map {
                    if kv.is_invalid() {
                        return Err(err("reflect: call of reflect.Value.Type on zero Value"));
                    }
                    // The map key type is string; only a string key is assignable.
                    if let Value::String(k) = kv {
                        vvv = map_index(vv, k);
                    }
                }
            }

            match self.check_condition(vvv.as_ref(), mv, op) {
                Ok(true) => rv.push(rvv.clone()),
                Ok(false) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(seqv.with(rv))
    }

    // Go: tpl/collections/where.go:checkWhereMap
    /// checkWhereMap handles the where-matching logic when the seqv value is a Map.
    pub(crate) fn check_where_map(
        &self,
        ctx: HostCtx<'_>,
        seqv: &Value,
        kv: &Value,
        mv: Option<&Value>,
        path: &[GoString],
        op: &str,
    ) -> GoResult<Value> {
        let (ty, entries): (MapType, Vec<(GoString, Value)>) = match seqv {
            Value::Map(m) => (
                m.ty.clone(),
                m.entries
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
            Value::TypedNil(t) => (hreflect::map_type_from_name(t), Vec::new()),
            Value::Object(o) => (
                MapType::Named(Arc::from(&*o.type_name())),
                o.map_keys()
                    .into_iter()
                    .filter_map(|k| o.map_get(&k).map(|v| (k, v)))
                    .collect(),
            ),
            _ => (MapType::StringAny, Vec::new()),
        };
        let mut rv = Map::new(ty);
        for (k, elemv) in entries {
            // Both the Array/Slice and the Interface cases of Go end up here: an interface
            // element is indirected first (a nil one is skipped).
            if elemv.is_invalid() {
                continue;
            }
            if let Some(sl) = as_slice(&elemv) {
                let r = self.check_where_array(ctx, &sl, kv, mv, path, op)?;
                if rh::len_of(&r) > 0 {
                    rv.insert(k, elemv);
                }
            }
        }
        Ok(Value::map(rv))
    }

    // Go: tpl/collections/where.go:toTimeUnix
    pub(crate) fn to_time_unix(&self, v: &Value) -> i64 {
        match hreflect::as_time(v, &self.loc) {
            Some(t) => t.unix_sec,
            None => panic!("coding error: argument must be time.Time type reflect Value"),
        }
    }

    /// `toTimeUnix` of an element that may not be a time (Go panics with this message).
    fn to_time_unix_checked(&self, v: &Value) -> GoResult<i64> {
        match hreflect::as_time(v, &self.loc) {
            Some(t) => Ok(t.unix_sec),
            None => Err(err(
                "coding error: argument must be time.Time type reflect Value",
            )),
        }
    }
}

/// Whether `indirect(v)` gives isNil (or an invalid value) in `checkCondition`.
fn cond_is_nil(v: &Value) -> bool {
    match v {
        Value::Invalid => true,
        Value::TypedNil(t) => matches!(typed_nil_kind(t), NilKind::Ptr | NilKind::Interface),
        _ => false,
    }
}

/// `reflect.ValueOf(x)`: a nil interface is the invalid value.
fn value_of(v: Value) -> Option<Value> {
    if v.is_invalid() { None } else { Some(v) }
}

/// The element type of a map-kind value (`MapIndex`'s static type).
pub(crate) fn map_elem_type(v: &Value) -> String {
    match v {
        Value::Map(m) => match &m.ty {
            MapType::StringAny | MapType::Params => "interface {}".to_string(),
            MapType::StringString => "string".to_string(),
            MapType::Named(n) => {
                hreflect::elem_type(n).unwrap_or_else(|| "interface {}".to_string())
            }
        },
        _ => hreflect::elem_type(&v.go_type_name()).unwrap_or_else(|| "interface {}".to_string()),
    }
}

/// `v.MapIndex(k)` of a map-kind value: `None` for a missing key.
fn map_index(v: &Value, k: &GoString) -> Option<Value> {
    match v {
        Value::Map(m) => m.get(k).cloned(),
        Value::Object(o) if o.kind() == Kind::Map => o.map_get(k),
        _ => None,
    }
}

// Go: tpl/collections/where.go:evaluateSubElem
/// Evaluates the method, struct field or map key `elem_name` of `obj`. `typ` is Go's
/// `obj.Type()` (the static type when the caller knows it) for the error messages. Returns the
/// value and its static type (a map's element type for a map value; the dynamic type of a
/// method or field result).
pub(crate) fn evaluate_sub_elem(
    ctx: HostCtx<'_>,
    obj: Option<&Value>,
    typ: &str,
    elem_name: &[u8],
) -> Result<(Option<Value>, String), EvalFail> {
    let Some(obj) = obj else {
        return Err(EvalFail::Err("can't evaluate an invalid value".into()));
    };
    let name = String::from_utf8_lossy(elem_name).into_owned();

    if obj.is_invalid()
        || matches!(obj, Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Interface)
    {
        // A nil interface: Go calls Type() on the zero Value of its Elem().
        return Err(EvalFail::Panic(err(
            "reflect: call of reflect.Value.Type on zero Value",
        )));
    }

    let (_, is_nil) = rh::indirect(obj);

    // Methods.
    let method = match obj {
        Value::Object(o) if o.has_method(&name) => Some(o.call_method(ctx, &name, &[])),
        Value::Time(t) if nh_tplimpl::template_funcs::time_has_method(&name) => {
            Some(nh_tplimpl::template_funcs::time_call_method(t, &name, &[]))
        }
        Value::Map(m)
            if m.ty == MapType::Params && nh_common::maps::params::params_has_method(&name) =>
        {
            Some(nh_common::maps::params::params_call_method(
                ctx,
                obj,
                &name,
                &[],
            ))
        }
        _ => None,
    };
    if let Some(res) = method {
        return match res {
            None => Err(EvalFail::Err(format!(
                "{name} is neither a struct field, a method nor a map element of type {typ}"
            ))),
            Some(Ok(v)) => {
                let t = dyn_type(&v);
                Ok((Some(v), t))
            }
            Some(Err(e)) => {
                let msg = e.message();
                if msg.starts_with("wrong number of args for ") {
                    Err(EvalFail::Err(format!(
                        "{name} is a method of type {typ} but requires more than 1 parameter"
                    )))
                } else {
                    Err(EvalFail::Err(format!(
                        "error at calling a method {name} of type {typ}: {msg}"
                    )))
                }
            }
        };
    }

    // elemName isn't a method so next start to check whether it is
    // a struct field or a map value. In both cases, it mustn't be
    // a nil value
    if is_nil {
        return Err(EvalFail::Err(format!(
            "can't evaluate a nil pointer of type {typ} by a struct field or map key name {name}"
        )));
    }
    match indirect_kind(obj) {
        ReflectKind::Struct => {
            if let Value::Object(o) = obj
                && let Some(f) = o.field(&name)
            {
                let t = dyn_type(&f);
                return Ok((Some(f), t));
            }
            Err(EvalFail::Err(format!(
                "{name} isn't a field of struct type {typ}"
            )))
        }
        ReflectKind::Map => Ok((
            map_index(obj, &GoString::from(elem_name)),
            map_elem_type(obj),
        )),
        _ => Err(EvalFail::Err(format!(
            "{name} is neither a struct field, a method nor a map element of type {typ}"
        ))),
    }
}

// Go: tpl/collections/where.go:parseWhereArgs
/// parseWhereArgs parses the end arguments to the where function. Return a match value and an
/// operator, if one is defined.
fn parse_where_args(args: &[Value]) -> GoResult<(Option<Value>, String)> {
    match args.len() {
        1 => Ok((value_of(args[0].clone()), String::new())),
        2 => {
            let Value::String(op) = &args[0] else {
                return Err(err("operator argument must be string type"));
            };
            let lower = go_unicode::strings::to_lower(op);
            let op = go_unicode::strings::trim_space(&lower);
            Ok((
                value_of(args[1].clone()),
                String::from_utf8_lossy(op).into_owned(),
            ))
        }
        _ => Err(err(
            "can't evaluate the array by no match argument or more than or equal to two arguments",
        )),
    }
}

/// Unused-import guard.
#[allow(dead_code)]
fn _list(_: &List) {}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/where.go (543 lines; 5/11 funcs executed)
// OK L29-56: (ns *Namespace) Where(ctx context.Context, c, key any, args ...any) (any, error)
// OK L58-294: (ns *Namespace) checkCondition(v, mv reflect.Value, op string) (bool, error)
// OK L296-376: evaluateSubElem(ctx, obj reflect.Value, elemName string) (reflect.Value, error)
// OK L380-396: parseWhereArgs(args ...any) (mv reflect.Value, op string, err error)
// OK L400-442: (ns *Namespace) checkWhereArray(ctxv, seqv, kv, mv reflect.Value, path []string, op string) (any, error)
// OK L445-489: (ns *Namespace) checkWhereMap(ctxv, seqv, kv, mv reflect.Value, path []string, op string) (any, error)
// OK L492-502: toFloat(v reflect.Value) (float64, error) (reflect_helpers::to_float)
// OK L506-514: toInt(v reflect.Value) (int64, error) (reflect_helpers::to_int)
// OK L516-524: toUint(v reflect.Value) (uint64, error) (reflect_helpers::to_uint)
// OK L527-535: toString(v reflect.Value) (string, error) (reflect_helpers::to_string)
// OK L537-543: (ns *Namespace) toTimeUnix(v reflect.Value) int64
// ---------------------------------------------------------------------------
