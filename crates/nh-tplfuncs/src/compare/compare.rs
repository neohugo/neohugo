//! Port of `tpl/compare/compare.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use go_value::{HostCtx, Kind, Location, Object, Value};
use nh_common::hreflect::{self, ReflectKind};
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

use crate::collections::reflect_helpers::{
    as_slice, bool_of, float_of, int_of, kind, len_of, string_of, uint_of,
};

// Parity notes: `eq` normalises ints to int64, floats to float64 and uses `Eqer` (`has_method("Eq")`) — `eq 1 1.0` is false; `ge/gt/le/lt` via `compareGetWithCollator` (numeric strings as numbers, time as Unix seconds); `default` rules; `cond`.

fn panic_err(msg: &str) -> go_value::Error {
    go_value::Error::new(msg)
}

/// A string comparison function (a locked collator's `CompareStrings`).
pub type StringCmp<'a> = &'a mut dyn FnMut(&[u8], &[u8]) -> i32;

/// Go: `compare.Namespace` (template value `*compare.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    /// Go `loc`.
    pub loc: Arc<Location>,
    /// Enable to do case insensitive string compares (Go `caseInsensitive`).
    pub case_insensitive: bool,
}

impl Namespace {
    // Go: tpl/compare:New
    /// The namespace the template func map gets (Go init: `New(langs.GetLocation(language),
    /// false)`).
    pub fn new(d: Arc<Deps>) -> Namespace {
        let loc = d.conf.language().location();
        Namespace::with_location(d, loc, false)
    }

    // Go: tpl/compare/compare.go:New
    /// New returns a new instance of the compare-namespaced template functions.
    pub fn with_location(d: Arc<Deps>, loc: Arc<Location>, case_insensitive: bool) -> Namespace {
        Namespace {
            d,
            loc,
            case_insensitive,
        }
    }

    // Go: tpl/compare:Conditional
    pub fn conditional(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 3, "Conditional")?;
        Ok(self.do_conditional(&a[0], &a[1], &a[2]))
    }

    // Go: tpl/compare/compare.go:Conditional
    /// Conditional can be used as a ternary operator. It returns v1 if cond is true, else v2.
    pub fn do_conditional(&self, cond: &Value, v1: &Value, v2: &Value) -> Value {
        if hreflect::is_truthful(cond) {
            return v1.clone();
        }
        v2.clone()
    }

    // Go: tpl/compare:Default
    pub fn default(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Default")?;
        self.do_default(&a[0], &a[1..])
    }

    // Go: tpl/compare/compare.go:Default
    /// Default checks whether a givenv is set and returns the default value defaultv if it is
    /// not. "Set" in this context means non-zero for numeric types and times; non-zero length
    /// for strings, arrays, slices, and maps; any boolean or struct value; or non-nil for any
    /// other types.
    pub fn do_default(&self, defaultv: &Value, givenv: &[Value]) -> GoResult<Value> {
        // given is variadic because the following construct will not pass a piped
        // argument when the key is missing:  {{ index . "key" | default "foo" }}
        // The Go template will complain that we got 1 argument when we expected 2.

        if givenv.is_empty() {
            return Ok(defaultv.clone());
        }
        if givenv.len() != 1 {
            return Err(go_value::Error::new(format!(
                "wrong number of args for default: want 2 got {}",
                givenv.len() + 1
            )));
        }

        let g = &givenv[0];
        let k = kind(g);
        if k == ReflectKind::Invalid {
            return Ok(defaultv.clone());
        }

        let set = match k {
            ReflectKind::Bool => true,
            ReflectKind::String | ReflectKind::Slice | ReflectKind::Map => len_of(g) != 0,
            ReflectKind::Int(_) => int_of(g) != 0,
            ReflectKind::Uint(_) => uint_of(g) != 0,
            ReflectKind::Float32 | ReflectKind::Float64 => float_of(g) != 0.0,
            ReflectKind::Struct => match g {
                Value::Time(t) => !t.is_zero(),
                _ => true,
            },
            _ => !g.is_nil(),
        };

        if set {
            return Ok(g.clone());
        }

        Ok(defaultv.clone())
    }

    // Go: tpl/compare:Eq
    pub fn eq(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Eq")?;
        Ok(Value::Bool(self.do_eq(ctx, &a[0], &a[1..])?))
    }

    /// Go: `normalize` in `Eq`.
    fn eq_normalize(&self, v: &Value) -> Value {
        if nh_common::types::types::is_nil(v) {
            return Value::Invalid;
        }

        if let Some(t) = as_time_provider(v, &self.loc) {
            return Value::Time(t);
        }

        match kind(v) {
            ReflectKind::Int(_) => Value::int64(int_of(v)),
            ReflectKind::Float32 | ReflectKind::Float64 => Value::float64(float_of(v)),
            ReflectKind::Uint(_) => {
                let i = uint_of(v);
                // If it can fit in an int, convert it.
                if i <= i64::MAX as u64 {
                    return Value::int64(i as i64);
                }
                Value::Uint(i, go_value::UintKind::Uint64)
            }
            ReflectKind::String => Value::String(string_of(v)),
            _ => v.clone(),
        }
    }

    // Go: tpl/compare/compare.go:Eq
    /// Eq returns the boolean truth of arg1 == arg2 || arg1 == arg3 || arg1 == arg4.
    pub fn do_eq(&self, ctx: HostCtx<'_>, first: &Value, others: &[Value]) -> GoResult<bool> {
        if self.case_insensitive {
            return Err(panic_err("caseInsensitive not implemented for Eq"));
        }
        self.check_comparison_arg_count(1, others)?;

        let norm_first = self.eq_normalize(first);
        for other in others {
            if let Some(e) = eqer(first) {
                if call_eq(e, ctx, other)? {
                    return Ok(true);
                }
                continue;
            }

            if let Some(e) = eqer(other) {
                if call_eq(e, ctx, first)? {
                    return Ok(true);
                }
                continue;
            }

            let other = self.eq_normalize(other);
            if deep_equal(&norm_first, &other) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    // Go: tpl/compare:Ge
    pub fn ge(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Ge")?;
        self.check_comparison_arg_count(1, &a[1..])?;
        for other in &a[1..] {
            let (left, right) = self.compare_get(&a[0], other);
            // Go: !(left >= right) (true for NaN).
            if left.partial_cmp(&right).is_none_or(|o| o.is_lt()) {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    // Go: tpl/compare:Gt
    pub fn gt(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Gt")?;
        self.check_comparison_arg_count(1, &a[1..])?;
        for other in &a[1..] {
            let (left, right) = self.compare_get(&a[0], other);
            // Go: !(left > right) (true for NaN).
            if left.partial_cmp(&right).is_none_or(|o| o.is_le()) {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    // Go: tpl/compare:Le
    pub fn le(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Le")?;
        self.check_comparison_arg_count(1, &a[1..])?;
        for other in &a[1..] {
            let (left, right) = self.compare_get(&a[0], other);
            // Go: !(left <= right) (true for NaN).
            if left.partial_cmp(&right).is_none_or(|o| o.is_gt()) {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    // Go: tpl/compare:Lt
    pub fn lt(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Lt")?;
        Ok(Value::Bool(self.do_lt_collate(None, &a[0], &a[1..])?))
    }

    // Go: tpl/compare:LtCollate
    /// The template method: the collator argument can only be nil from a template (there is no
    /// `*langs.Collator` template value).
    pub fn lt_collate(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "LtCollate")?;
        if !a[0].is_invalid() {
            return Err(args::wrong_type("*langs.Collator", &a[0]));
        }
        Ok(Value::Bool(self.do_lt_collate(None, &a[1], &a[2..])?))
    }

    // Go: tpl/compare/compare.go:LtCollate
    /// LtCollate returns the boolean truth of arg1 < arg2 && arg1 < arg3 && arg1 < arg4. The
    /// provided collator will be used for string comparisons.
    pub fn do_lt_collate(
        &self,
        mut collator: Option<StringCmp<'_>>,
        first: &Value,
        others: &[Value],
    ) -> GoResult<bool> {
        self.check_comparison_arg_count(1, others)?;
        for other in others {
            let (left, right) = self.compare_get_with_collator(
                collator.as_mut().map(|c| &mut **c as StringCmp<'_>),
                first,
                other,
            );
            // Go: !(left < right) (true for NaN).
            if left.partial_cmp(&right).is_none_or(|o| o.is_ge()) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    // Go: tpl/compare/compare.go:Ne
    // Go: tpl/compare:Ne
    /// Ne returns the boolean truth of arg1 != arg2 && arg1 != arg3 && arg1 != arg4.
    pub fn ne(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Ne")?;
        self.check_comparison_arg_count(1, &a[1..])?;
        for other in &a[1..] {
            if self.do_eq(ctx, &a[0], std::slice::from_ref(other))? {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    // Go: tpl/compare/compare.go:checkComparisonArgCount
    fn check_comparison_arg_count(&self, min: usize, others: &[Value]) -> GoResult<bool> {
        if others.len() < min {
            return Err(panic_err("missing arguments for comparison"));
        }
        Ok(true)
    }

    // Go: tpl/compare/compare.go:compareGet
    pub fn compare_get(&self, a: &Value, b: &Value) -> (f64, f64) {
        self.compare_get_with_collator(None, a, b)
    }

    // Go: tpl/compare/compare.go:compareTwoUints
    fn compare_two_uints(&self, a: u64, b: u64) -> (f64, f64) {
        if a < b {
            (1.0, 0.0)
        } else if a == b {
            (0.0, 0.0)
        } else {
            (0.0, 1.0)
        }
    }

    // Go: tpl/compare/compare.go:compareGetWithCollator
    pub fn compare_get_with_collator(
        &self,
        collator: Option<StringCmp<'_>>,
        a: &Value,
        b: &Value,
    ) -> (f64, f64) {
        if let Some(c) = comparer(a, b) {
            return if c < 0 {
                (1.0, 0.0)
            } else if c == 0 {
                (0.0, 0.0)
            } else {
                (0.0, 1.0)
            };
        }

        if let Some(c) = comparer(b, a) {
            return if c < 0 {
                (0.0, 1.0)
            } else if c == 0 {
                (0.0, 0.0)
            } else {
                (1.0, 0.0)
            };
        }

        let mut left = 0.0f64;
        let mut right = 0.0f64;
        let mut left_str: Option<go_value::GoString> = None;
        let mut right_str: Option<go_value::GoString> = None;
        let ak = kind(a);
        let bk = kind(b);

        match ak {
            ReflectKind::Chan | ReflectKind::Map | ReflectKind::Slice => {
                left = len_of(a) as f64;
            }
            ReflectKind::Int(_) => {
                if hreflect::is_uint_kind(bk) {
                    return self.compare_two_uints(int_of(a) as u64, uint_of(b));
                }
                left = int_of(a) as f64;
            }
            ReflectKind::Uint(go_value::UintKind::Uint64) => {
                if hreflect::is_uint_kind(bk) {
                    return self.compare_two_uints(uint_of(a), uint_of(b));
                }
            }
            ReflectKind::Uint(go_value::UintKind::Uintptr) => {}
            ReflectKind::Uint(_) => {
                left = uint_of(a) as f64;
            }
            ReflectKind::Float32 | ReflectKind::Float64 => {
                left = float_of(a);
            }
            ReflectKind::String => {
                let s = string_of(a);
                let (f, e) = go_strconv::internal::parse_float(&s, 64);
                left = f;
                // Check if float is a special floating value and cast value as string.
                if left.is_infinite() || left.is_nan() || e.is_some() {
                    left_str = Some(s);
                }
            }
            ReflectKind::Struct => {
                if let Some(t) = as_time(a, &self.loc) {
                    left = t.unix_sec as f64;
                }
            }
            ReflectKind::Bool => {
                left = 0.0;
                if bool_of(a) {
                    left = 1.0;
                }
            }
            _ => {}
        }

        match bk {
            ReflectKind::Chan | ReflectKind::Map | ReflectKind::Slice => {
                right = len_of(b) as f64;
            }
            ReflectKind::Int(_) => {
                if hreflect::is_uint_kind(ak) {
                    return self.compare_two_uints(uint_of(a), int_of(b) as u64);
                }
                right = int_of(b) as f64;
            }
            ReflectKind::Uint(go_value::UintKind::Uint64) => {
                if hreflect::is_uint_kind(ak) {
                    return self.compare_two_uints(uint_of(a), uint_of(b));
                }
            }
            ReflectKind::Uint(go_value::UintKind::Uintptr) => {}
            ReflectKind::Uint(_) => {
                right = uint_of(b) as f64;
            }
            ReflectKind::Float32 | ReflectKind::Float64 => {
                right = float_of(b);
            }
            ReflectKind::String => {
                let s = string_of(b);
                let (f, e) = go_strconv::internal::parse_float(&s, 64);
                right = f;
                // Check if float is a special floating value and cast value as string.
                if right.is_infinite() || right.is_nan() || e.is_some() {
                    right_str = Some(s);
                }
            }
            ReflectKind::Struct => {
                if let Some(t) = as_time(b, &self.loc) {
                    right = t.unix_sec as f64;
                }
            }
            ReflectKind::Bool => {
                right = 0.0;
                if bool_of(b) {
                    right = 1.0;
                }
            }
            _ => {}
        }

        if (self.case_insensitive || collator.is_some())
            && let (Some(ls), Some(rs)) = (&left_str, &right_str)
        {
            let c = match collator {
                Some(cmp) => cmp(ls, rs),
                None => nh_common::compare::strings(ls, rs),
            };
            return if c < 0 {
                (0.0, 1.0)
            } else if c > 0 {
                (1.0, 0.0)
            } else {
                (0.0, 0.0)
            };
        }

        if let (Some(ls), Some(rs)) = (&left_str, &right_str) {
            if ls.as_bytes() < rs.as_bytes() {
                return (0.0, 1.0);
            } else if ls.as_bytes() > rs.as_bytes() {
                return (1.0, 0.0);
            } else {
                return (0.0, 0.0);
            }
        }

        (left, right)
    }

    // Go: tpl/compare/compare.go:toTimeUnix
    pub fn to_time_unix(&self, v: &Value) -> i64 {
        match as_time(v, &self.loc) {
            Some(t) => t.unix_sec,
            None => panic!("coding error: argument must be time.Time type reflect Value"),
        }
    }
}

/// `v.(htime.AsTimeProvider)`: go-toml local dates (an object with an `AsTime` method).
fn as_time_provider(v: &Value, loc: &Arc<Location>) -> Option<go_value::Time> {
    match v {
        Value::Object(o) if o.has_method("AsTime") => hreflect::as_time(v, loc),
        _ => None,
    }
}

/// `hreflect.AsTime(v, loc)` for a struct-kind value.
fn as_time(v: &Value, loc: &Arc<Location>) -> Option<go_value::Time> {
    hreflect::as_time(v, loc)
}

/// `v.(compare.Eqer)`.
fn eqer(v: &Value) -> Option<&Arc<dyn Object>> {
    match v {
        Value::Object(o) if o.has_method("Eq") => Some(o),
        _ => None,
    }
}

fn call_eq(o: &Arc<dyn Object>, ctx: HostCtx<'_>, other: &Value) -> GoResult<bool> {
    match o.call_method(ctx, "Eq", std::slice::from_ref(other)) {
        Some(Ok(v)) => Ok(hreflect::is_truthful(&v)),
        Some(Err(e)) => Err(e),
        None => Ok(false),
    }
}

/// `a.(compare.Comparer)`: `a.Compare(b)`.
fn comparer(a: &Value, b: &Value) -> Option<i64> {
    let Value::Object(o) = a else {
        return None;
    };
    if !o.has_method("Compare") {
        return None;
    }
    match o.call_method(&(), "Compare", std::slice::from_ref(b)) {
        Some(Ok(Value::Int(i, _))) => Some(i),
        _ => None,
    }
}

// Go: reflect.DeepEqual
/// Go's `reflect.DeepEqual` over values normalised by `Eq` (identical dynamic types required).
pub fn deep_equal(a: &Value, b: &Value) -> bool {
    let ta = hreflect::type_of(a);
    let tb = hreflect::type_of(b);
    if ta != tb {
        return false;
    }
    match (a, b) {
        (Value::Invalid, _) | (_, Value::Invalid) => a.is_invalid() && b.is_invalid(),
        (Value::TypedNil(_), Value::TypedNil(_)) => true,
        (Value::TypedNil(_), other) | (other, Value::TypedNil(_)) => {
            // A nil slice/map never deep-equals a non-nil one; a nil pointer equals only nil.
            let _ = other;
            false
        }
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x, _), Value::Int(y, _)) => x == y,
        (Value::Uint(x, _), Value::Uint(y, _)) => x == y,
        (Value::Float(x, _), Value::Float(y, _)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Safe(_, x), Value::Safe(_, y)) => x == y,
        (Value::Time(x), Value::Time(y)) => {
            x.unix_sec == y.unix_sec
                && x.nsec == y.nsec
                && match (&x.loc, &y.loc) {
                    (None, None) => true,
                    (Some(l1), Some(l2)) => Arc::ptr_eq(l1, l2) || l1.name == l2.name,
                    _ => false,
                }
        }
        (Value::List(x), Value::List(y)) => {
            Arc::ptr_eq(x, y)
                || (x.items.len() == y.items.len()
                    && x.items.iter().zip(&y.items).all(|(p, q)| deep_equal(p, q)))
        }
        (Value::Map(x), Value::Map(y)) => {
            Arc::ptr_eq(x, y)
                || (x.entries.len() == y.entries.len()
                    && x.entries.iter().all(|(k, v)| match y.entries.get(k) {
                        Some(w) => deep_equal(v, w),
                        None => false,
                    }))
        }
        (Value::Object(x), Value::Object(y)) => {
            if let (Some(ux), Some(uy)) = (x.underlying(), y.underlying()) {
                return deep_equal(&ux, &uy);
            }
            if x.identity() == y.identity() {
                return true;
            }
            match (x.kind(), x.struct_fields(), y.struct_fields()) {
                (Kind::Struct, Some(fx), Some(fy)) => {
                    fx.len() == fy.len()
                        && fx
                            .iter()
                            .zip(fy.iter())
                            .all(|((nx, vx), (ny, vy))| nx == ny && deep_equal(vx, vy))
                }
                (Kind::Slice, _, _) => match (as_slice(a), as_slice(b)) {
                    (Some(sa), Some(sb)) => {
                        sa.items.len() == sb.items.len()
                            && sa
                                .items
                                .iter()
                                .zip(&sb.items)
                                .all(|(p, q)| deep_equal(p, q))
                    }
                    _ => false,
                },
                _ => false,
            }
        }
        _ => false,
    }
}

nh_common::go_methods!(Namespace {
    "Conditional" => |n, ctx, a| n.conditional(ctx, a),
    "Default" => |n, ctx, a| n.default(ctx, a),
    "Eq" => |n, ctx, a| n.eq(ctx, a),
    "Ge" => |n, ctx, a| n.ge(ctx, a),
    "Gt" => |n, ctx, a| n.gt(ctx, a),
    "Le" => |n, ctx, a| n.le(ctx, a),
    "Lt" => |n, ctx, a| n.lt(ctx, a),
    "LtCollate" => |n, ctx, a| n.lt_collate(ctx, a),
    "Ne" => |n, ctx, a| n.ne(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*compare.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/compare/compare.go (388 lines; 13/15 funcs executed)
//   types: Namespace
// OK L33-35: New(loc *time.Location, caseInsensitive bool) *Namespace
// OK L48-96: (*Namespace) Default(defaultv any, givenv ...any) (any, error)
// OK L99-156: (n *Namespace) Eq(first any, others ...any) bool
// OK L159-167: (n *Namespace) Ne(first any, others ...any) bool
// OK L170-179: (n *Namespace) Ge(first any, others ...any) bool
// OK L182-191: (n *Namespace) Gt(first any, others ...any) bool
// OK L194-203: (n *Namespace) Le(first any, others ...any) bool
// OK L208-217: (n *Namespace) LtCollate(collator *langs.Collator, first any, others ...any) bool
// OK L220-222: (n *Namespace) Lt(first any, others ...any) bool
// OK L224-229: (n *Namespace) checkComparisonArgCount(min int, others ...any) bool
// OK L234-239: (n *Namespace) Conditional(cond any, v1, v2 any) any
// OK L241-243: (ns *Namespace) compareGet(a any, b any) (float64, float64)
// OK L245-253: (ns *Namespace) compareTwoUints(a uint64, b uint64) (float64, float64)
// OK L255-380: (ns *Namespace) compareGetWithCollator(collator *langs.Collator, a any, b any) (float64, float64)
// OK L382-388: (ns *Namespace) toTimeUnix(v reflect.Value) int64
// ---------------------------------------------------------------------------
