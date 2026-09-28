//! Port of `common/math/math.go`.
//!
//! Owner: Wave B task T01 (common-values).

use go_value::{FloatKind, IntKind, UintKind, Value};

use crate::herrors::{Error, Result};

/// A number read with Go's `reflect.Value.Int/Uint/Float` (named basic types through
/// `Object::underlying`), or a string (`reflect.Value.String`).
enum Operand {
    Int(i64),
    Uint(u64),
    Float(f64),
    String(go_value::GoString),
    Other,
}

fn operand(v: &Value) -> Operand {
    match v {
        Value::Int(i, _) => Operand::Int(*i),
        // reflect.Uintptr is not in DoArithmetic's uint cases.
        Value::Uint(_, UintKind::Uintptr) => Operand::Other,
        Value::Uint(u, _) => Operand::Uint(*u),
        // A float32 is stored widened; reflect.Value.Float returns the same float64.
        Value::Float(f, _) => Operand::Float(*f),
        Value::String(s) | Value::Safe(_, s) => Operand::String(s.clone()),
        Value::Object(o) => match o.underlying() {
            Some(u @ (Value::Int(..) | Value::Uint(..) | Value::Float(..) | Value::String(_))) => {
                operand(&u)
            }
            _ => Operand::Other,
        },
        _ => Operand::Other,
    }
}

fn cant_apply() -> Error {
    Error::new("can't apply the operator to the values")
}

/// The golden build ran on arm64, where an invalid float operation (`Inf - Inf`, `0 * Inf`,
/// `Inf / Inf`) yields the default NaN `0x7FF8000000000000`; x86-64 yields `0xFFF8000000000000`.
/// NaN operands propagate the same way on both.
fn arm64_nan(r: f64, a: f64, b: f64) -> f64 {
    if r.is_nan() && !a.is_nan() && !b.is_nan() {
        f64::from_bits(0x7FF8_0000_0000_0000)
    } else {
        r
    }
}

// Go: common/math/math.go:DoArithmetic
/// DoArithmetic performs arithmetic operations (+,-,*,/) using reflection to determine the type
/// of the two terms (used by `add/sub/mul/div` and `Scratch.Add`): two signed integers give an
/// `int64`, any float gives a `float64`, a non-negative signed with an unsigned (or two unsigned)
/// gives a `uint64`, `string + string` concatenates (to a `string`); integer arithmetic wraps;
/// division by zero is an error; other ops (`%`) are "there is no such an operation".
pub fn do_arithmetic(a: &Value, b: &Value, op: char) -> Result<Value> {
    let av = operand(a);
    let bv = operand(b);
    let (mut ai, mut bi): (i64, i64) = (0, 0);
    let (mut af, mut bf): (f64, f64) = (0.0, 0.0);
    let (mut au, mut bu): (u64, u64) = (0, 0);
    let (mut is_int, mut is_float, mut is_uint) = (false, false, false);
    match av {
        Operand::Int(a_int) => {
            ai = a_int;
            match bv {
                Operand::Int(b_int) => {
                    is_int = true;
                    bi = b_int;
                }
                Operand::Float(b_float) => {
                    is_float = true;
                    af = ai as f64; // may overflow
                    bf = b_float;
                }
                Operand::Uint(b_uint) => {
                    bu = b_uint;
                    if ai >= 0 {
                        is_uint = true;
                        au = ai as u64;
                    } else {
                        is_int = true;
                        bi = bu as i64; // may overflow
                    }
                }
                _ => return Err(cant_apply()),
            }
        }
        Operand::Float(a_float) => {
            is_float = true;
            af = a_float;
            match bv {
                Operand::Int(b_int) => bf = b_int as f64, // may overflow
                Operand::Float(b_float) => bf = b_float,
                Operand::Uint(b_uint) => bf = b_uint as f64, // may overflow
                _ => return Err(cant_apply()),
            }
        }
        Operand::Uint(a_uint) => {
            au = a_uint;
            match bv {
                Operand::Int(b_int) => {
                    bi = b_int;
                    if bi >= 0 {
                        is_uint = true;
                        bu = bi as u64;
                    } else {
                        is_int = true;
                        ai = au as i64; // may overflow
                    }
                }
                Operand::Float(b_float) => {
                    is_float = true;
                    af = au as f64; // may overflow
                    bf = b_float;
                }
                Operand::Uint(b_uint) => {
                    is_uint = true;
                    bu = b_uint;
                }
                _ => return Err(cant_apply()),
            }
        }
        Operand::String(a_str) => {
            if let (Operand::String(b_str), '+') = (bv, op) {
                let mut s = a_str.to_vec();
                s.extend_from_slice(b_str.as_bytes());
                return Ok(Value::string(s));
            }
            return Err(cant_apply());
        }
        Operand::Other => return Err(cant_apply()),
    }

    let int64 = |i: i64| Value::Int(i, IntKind::Int64);
    let float64 = |f: f64| Value::Float(f, FloatKind::F64);
    let uint64 = |u: u64| Value::Uint(u, UintKind::Uint64);
    match op {
        '+' => {
            if is_int {
                Ok(int64(ai.wrapping_add(bi)))
            } else if is_float {
                Ok(float64(arm64_nan(af + bf, af, bf)))
            } else {
                Ok(uint64(au.wrapping_add(bu)))
            }
        }
        '-' => {
            if is_int {
                Ok(int64(ai.wrapping_sub(bi)))
            } else if is_float {
                Ok(float64(arm64_nan(af - bf, af, bf)))
            } else {
                Ok(uint64(au.wrapping_sub(bu)))
            }
        }
        '*' => {
            if is_int {
                Ok(int64(ai.wrapping_mul(bi)))
            } else if is_float {
                Ok(float64(arm64_nan(af * bf, af, bf)))
            } else {
                Ok(uint64(au.wrapping_mul(bu)))
            }
        }
        '/' => {
            if is_int && bi != 0 {
                // Go: MinInt64 / -1 == MinInt64 (two's-complement overflow, no panic).
                Ok(int64(ai.wrapping_div(bi)))
            } else if is_float && bf != 0.0 {
                Ok(float64(arm64_nan(af / bf, af, bf)))
            } else if is_uint && bu != 0 {
                Ok(uint64(au / bu))
            } else {
                Err(Error::new("can't divide the value by 0"))
            }
        }
        _ => Err(Error::new("there is no such an operation")),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/math/math.go (133 lines; 1/1 funcs executed)
// OK L23-133: DoArithmetic(a, b any, op rune) (any, error)
// ---------------------------------------------------------------------------
