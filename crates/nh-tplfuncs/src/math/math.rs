//! Port of `tpl/math/math.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! The `math` package functions are Go's own (arm64 build, fused sites included) from
//! [`super::gomath`]; `Sqrt`, `Floor`, `Ceil` and `Abs` are exact IEEE operations.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use go_value::{HostCtx, Object, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

use super::gomath;
use crate::collections::reflect_helpers::as_slice;

const ERR_MUST_TWO_NUMBERS: &str = "must provide at least two numbers";
const ERR_MUST_ONE_NUMBER: &str = "must provide at least one number";

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

/// Go: `math.Namespace` (template value `*math.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

/// One float argument through `cast.ToFloat64E`, with the function's error text.
fn float_arg(a: &[Value], name: &str, msg: &str) -> GoResult<f64> {
    args::exactly(a, 1, name)?;
    caste::to_float64_e(&a[0]).map_err(|_| err(msg))
}

impl Namespace {
    // Go: tpl/math:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/math:Abs
    pub fn abs(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Abs
        let af = float_arg(
            a,
            "Abs",
            "the math.Abs function requires a numeric argument",
        )?;
        Ok(Value::float64(af.abs()))
    }

    // Go: tpl/math:Acos
    pub fn acos(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Acos
        let af = float_arg(a, "Acos", "requires a numeric argument")?;
        Ok(Value::float64(gomath::acos(af)))
    }

    // Go: tpl/math:Add
    pub fn add(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Add
        self.do_arithmetic(a, '+')
    }

    // Go: tpl/math:Asin
    pub fn asin(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Asin
        let af = float_arg(a, "Asin", "requires a numeric argument")?;
        Ok(Value::float64(gomath::asin(af)))
    }

    // Go: tpl/math:Atan
    pub fn atan(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Atan
        let af = float_arg(a, "Atan", "requires a numeric argument")?;
        Ok(Value::float64(gomath::atan(af)))
    }

    // Go: tpl/math:Atan2
    pub fn atan2(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Atan2")?;
        // Go: tpl/math/math.go:Atan2
        let afx = caste::to_float64_e(&a[0]).map_err(|_| err("requires numeric arguments"))?;
        let afy = caste::to_float64_e(&a[1]).map_err(|_| err("requires numeric arguments"))?;
        Ok(Value::float64(gomath::atan2(afx, afy)))
    }

    // Go: tpl/math:Ceil
    pub fn ceil(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Ceil
        let xf = float_arg(
            a,
            "Ceil",
            "Ceil operator can't be used with non-float value",
        )?;
        Ok(Value::float64(xf.ceil()))
    }

    // Go: tpl/math:Cos
    pub fn cos(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Cos
        let af = float_arg(a, "Cos", "requires a numeric argument")?;
        Ok(Value::float64(gomath::cos(af)))
    }

    // Go: tpl/math:Counter
    pub fn counter(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "Counter")?;
        // Go: tpl/math/math.go:Counter
        let n = self
            .d
            .counters
            .math_counter
            .fetch_add(1, Ordering::SeqCst)
            .wrapping_add(1);
        Ok(Value::Uint(n, go_value::UintKind::Uint64))
    }

    // Go: tpl/math:Div
    pub fn div(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Div
        self.do_arithmetic(a, '/')
    }

    // Go: tpl/math:Floor
    pub fn floor(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Floor
        let xf = float_arg(
            a,
            "Floor",
            "Floor operator can't be used with non-float value",
        )?;
        Ok(Value::float64(xf.floor()))
    }

    // Go: tpl/math:Log
    pub fn log(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Log
        let af = float_arg(
            a,
            "Log",
            "Log operator can't be used with non integer or float value",
        )?;
        Ok(Value::float64(gomath::log(af)))
    }

    // Go: tpl/math:Max
    pub fn max(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Max
        Ok(Value::float64(self.apply_op_to_scalars_or_slices(
            "Max",
            gomath::max,
            a,
        )?))
    }

    // Go: tpl/math:MaxInt64
    pub fn max_int64(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "MaxInt64")?;
        // Go: tpl/math/math.go:MaxInt64
        Ok(Value::int64(i64::MAX))
    }

    // Go: tpl/math:Min
    pub fn min(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Min
        Ok(Value::float64(self.apply_op_to_scalars_or_slices(
            "Min",
            gomath::min,
            a,
        )?))
    }

    // Go: tpl/math:Mod
    pub fn mod_(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Mod")?;
        Ok(Value::int64(self.do_mod(&a[0], &a[1])?))
    }

    // Go: tpl/math/math.go:Mod
    /// Mod returns n1 % n2.
    pub fn do_mod(&self, n1: &Value, n2: &Value) -> GoResult<i64> {
        let ai = caste::to_int64_e(n1);
        let bi = caste::to_int64_e(n2);

        let (Ok(ai), Ok(bi)) = (ai, bi) else {
            return Err(err("modulo operator can't be used with non integer value"));
        };

        if bi == 0 {
            return Err(err(
                "the number can't be divided by zero at modulo operation",
            ));
        }

        Ok(ai.wrapping_rem(bi))
    }

    // Go: tpl/math:ModBool
    pub fn mod_bool(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "ModBool")?;
        // Go: tpl/math/math.go:ModBool
        let res = self.do_mod(&a[0], &a[1])?;
        Ok(Value::Bool(res == 0))
    }

    // Go: tpl/math:Mul
    pub fn mul(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Mul
        self.do_arithmetic(a, '*')
    }

    // Go: tpl/math:Pi
    pub fn pi(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "Pi")?;
        // Go: tpl/math/math.go:Pi
        Ok(Value::float64(std::f64::consts::PI))
    }

    // Go: tpl/math:Pow
    pub fn pow(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Pow")?;
        // Go: tpl/math/math.go:Pow
        let af = caste::to_float64_e(&a[0]);
        let bf = caste::to_float64_e(&a[1]);

        let (Ok(af), Ok(bf)) = (af, bf) else {
            return Err(err("Pow operator can't be used with non-float value"));
        };

        Ok(Value::float64(gomath::pow(af, bf)))
    }

    // Go: tpl/math:Product
    pub fn product(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Product
        Ok(Value::float64(self.apply_op_to_scalars_or_slices(
            "Product",
            |x, y| gomath::arm_result(x * y, x, y),
            a,
        )?))
    }

    // Go: tpl/math:Rand
    pub fn rand(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "Rand")?;
        // Go: tpl/math/math.go:Rand
        Ok(Value::float64(rand_float64()))
    }

    // Go: tpl/math:Round
    pub fn round(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Round
        let xf = float_arg(
            a,
            "Round",
            "Round operator can't be used with non-float value",
        )?;
        Ok(Value::float64(super::round::round(xf)))
    }

    // Go: tpl/math:Sin
    pub fn sin(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Sin
        let af = float_arg(a, "Sin", "requires a numeric argument")?;
        Ok(Value::float64(gomath::sin(af)))
    }

    // Go: tpl/math:Sqrt
    pub fn sqrt(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Sqrt
        let af = float_arg(
            a,
            "Sqrt",
            "Sqrt operator can't be used with non integer or float value",
        )?;
        Ok(Value::float64(gomath::sqrt(af)))
    }

    // Go: tpl/math:Sub
    pub fn sub(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Sub
        self.do_arithmetic(a, '-')
    }

    // Go: tpl/math:Sum
    pub fn sum(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Sum
        Ok(Value::float64(self.apply_op_to_scalars_or_slices(
            "Sum",
            |x, y| gomath::arm_result(x + y, x, y),
            a,
        )?))
    }

    // Go: tpl/math:Tan
    pub fn tan(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:Tan
        let af = float_arg(a, "Tan", "requires a numeric argument")?;
        Ok(Value::float64(gomath::tan(af)))
    }

    // Go: tpl/math:ToDegrees
    pub fn to_degrees(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:ToDegrees
        let af = float_arg(a, "ToDegrees", "requires a numeric argument")?;
        Ok(Value::float64(af * 180.0 / std::f64::consts::PI))
    }

    // Go: tpl/math:ToRadians
    pub fn to_radians(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/math/math.go:ToRadians
        let af = float_arg(a, "ToRadians", "requires a numeric argument")?;
        Ok(Value::float64(af * std::f64::consts::PI / 180.0))
    }

    // Go: tpl/math/math.go:applyOpToScalarsOrSlices
    fn apply_op_to_scalars_or_slices(
        &self,
        op_name: &str,
        op: fn(f64, f64) -> f64,
        inputs: &[Value],
    ) -> GoResult<f64> {
        let mut i = 0usize;
        let mut has_value = false;
        let mut result = 0.0f64;
        for input in inputs {
            let (values, is_slice) = match self.to_floats_e(input) {
                Ok(v) => v,
                Err(_) => {
                    return Err(err(format!(
                        "{op_name} operator can't be used with non-float values"
                    )));
                }
            };
            has_value = has_value || !values.is_empty() || is_slice;
            for value in values {
                i += 1;
                if i == 1 {
                    result = value;
                    continue;
                }
                result = op(result, value);
            }
        }

        if !has_value {
            return Err(err(ERR_MUST_ONE_NUMBER));
        }
        Ok(result)
    }

    // Go: tpl/math/math.go:toFloatsE
    fn to_floats_e(&self, v: &Value) -> GoResult<(Vec<f64>, bool)> {
        if let Some(s) = as_slice(v) {
            let mut floats = Vec::with_capacity(s.items.len());
            for item in &s.items {
                floats.push(caste::to_float64_e(item)?);
            }
            return Ok((floats, true));
        }
        Ok((vec![caste::to_float64_e(v)?], false))
    }

    // Go: tpl/math/math.go:doArithmetic
    fn do_arithmetic(&self, inputs: &[Value], operation: char) -> GoResult<Value> {
        if inputs.len() < 2 {
            return Err(err(ERR_MUST_TWO_NUMBERS));
        }
        let mut value = inputs[0].clone();
        for input in &inputs[1..] {
            value = nh_common::math::do_arithmetic(&value, input, operation)?;
        }
        Ok(value)
    }
}

/// Go `rand.Float64()` over the port's own generator (random in Go too).
fn rand_float64() -> f64 {
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut s = STATE.load(Ordering::Relaxed);
    if s == 0 {
        s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
    }
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    STATE.store(s, Ordering::Relaxed);
    // 53 random bits in [0, 1).
    (s >> 11) as f64 / (1u64 << 53) as f64
}

nh_common::go_methods!(Namespace {
    "Abs" => |n, ctx, a| n.abs(ctx, a),
    "Acos" => |n, ctx, a| n.acos(ctx, a),
    "Add" => |n, ctx, a| n.add(ctx, a),
    "Asin" => |n, ctx, a| n.asin(ctx, a),
    "Atan" => |n, ctx, a| n.atan(ctx, a),
    "Atan2" => |n, ctx, a| n.atan2(ctx, a),
    "Ceil" => |n, ctx, a| n.ceil(ctx, a),
    "Cos" => |n, ctx, a| n.cos(ctx, a),
    "Counter" => |n, ctx, a| n.counter(ctx, a),
    "Div" => |n, ctx, a| n.div(ctx, a),
    "Floor" => |n, ctx, a| n.floor(ctx, a),
    "Log" => |n, ctx, a| n.log(ctx, a),
    "Max" => |n, ctx, a| n.max(ctx, a),
    "MaxInt64" => |n, ctx, a| n.max_int64(ctx, a),
    "Min" => |n, ctx, a| n.min(ctx, a),
    "Mod" => |n, ctx, a| n.mod_(ctx, a),
    "ModBool" => |n, ctx, a| n.mod_bool(ctx, a),
    "Mul" => |n, ctx, a| n.mul(ctx, a),
    "Pi" => |n, ctx, a| n.pi(ctx, a),
    "Pow" => |n, ctx, a| n.pow(ctx, a),
    "Product" => |n, ctx, a| n.product(ctx, a),
    "Rand" => |n, ctx, a| n.rand(ctx, a),
    "Round" => |n, ctx, a| n.round(ctx, a),
    "Sin" => |n, ctx, a| n.sin(ctx, a),
    "Sqrt" => |n, ctx, a| n.sqrt(ctx, a),
    "Sub" => |n, ctx, a| n.sub(ctx, a),
    "Sum" => |n, ctx, a| n.sum(ctx, a),
    "Tan" => |n, ctx, a| n.tan(ctx, a),
    "ToDegrees" => |n, ctx, a| n.to_degrees(ctx, a),
    "ToRadians" => |n, ctx, a| n.to_radians(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*math.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/math/math.go (365 lines; 5/34 funcs executed)
//   types: Namespace
// OK L35-39: New(d *deps.Deps) *Namespace
// OK L47-54: (ns *Namespace) Abs(n any) (float64, error)
// OK L57-63: (ns *Namespace) Acos(n any) (float64, error)
// OK L66-68: (ns *Namespace) Add(inputs ...any) (any, error)
// OK L71-77: (ns *Namespace) Asin(n any) (float64, error)
// OK L80-86: (ns *Namespace) Atan(n any) (float64, error)
// OK L89-99: (ns *Namespace) Atan2(n, m any) (float64, error)
// OK L102-109: (ns *Namespace) Ceil(n any) (float64, error)
// OK L112-118: (ns *Namespace) Cos(n any) (float64, error)
// OK L121-123: (ns *Namespace) Div(inputs ...any) (any, error)
// OK L126-133: (ns *Namespace) Floor(n any) (float64, error)
// OK L136-143: (ns *Namespace) Log(n any) (float64, error)
// OK L146-148: (ns *Namespace) Max(inputs ...any) (maximum float64, err error)
// OK L151-153: (ns *Namespace) MaxInt64() int64
// OK L156-158: (ns *Namespace) Min(inputs ...any) (minimum float64, err error)
// OK L161-174: (ns *Namespace) Mod(n1, n2 any) (int64, error)
// OK L177-184: (ns *Namespace) ModBool(n1, n2 any) (bool, error)
// OK L187-189: (ns *Namespace) Mul(inputs ...any) (any, error)
// OK L192-194: (ns *Namespace) Pi() float64
// OK L197-206: (ns *Namespace) Pow(n1, n2 any) (float64, error)
// OK L209-214: (ns *Namespace) Product(inputs ...any) (product float64, err error)
// OK L217-219: (ns *Namespace) Rand() float64 (own generator; random in Go)
// OK L222-229: (ns *Namespace) Round(n any) (float64, error)
// OK L232-238: (ns *Namespace) Sin(n any) (float64, error)
// OK L241-248: (ns *Namespace) Sqrt(n any) (float64, error)
// OK L251-253: (ns *Namespace) Sub(inputs ...any) (any, error)
// OK L256-261: (ns *Namespace) Sum(inputs ...any) (sum float64, err error)
// OK L264-270: (ns *Namespace) Tan(n any) (float64, error)
// OK L273-280: (ns *Namespace) ToDegrees(n any) (float64, error)
// OK L283-290: (ns *Namespace) ToRadians(n any) (float64, error)
// OK L292-319: (ns *Namespace) applyOpToScalarsOrSlices(opName string, op func(x, y float64) float64, inputs ...any) (result float64, err error)
// OK L321-341: (ns *Namespace) toFloatsE(v any) ([]float64, bool, error)
// OK L343-355: (ns *Namespace) doArithmetic(inputs []any, operation rune) (value any, err error)
// OK L363-365: (ns *Namespace) Counter() uint64
// ---------------------------------------------------------------------------
