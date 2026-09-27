//! Port of `common/math/math.go`.
//!
//! Owner: Wave B task T01 (common-values).


use go_value::Value;

use crate::herrors::Result;

/// Go: `common/math.DoArithmetic(a, b any, op rune)` used by `add/sub/mul/div` and `Scratch.Add`:
/// int kinds -> int64 result, any float -> float64, uints kept when both are uint,
/// `string + string` concatenates, division by zero is an error.
// Go: common/math/math.go:DoArithmetic
pub fn do_arithmetic(a: &Value, b: &Value, op: char) -> Result<Value> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/math/math.go (133 lines; 1/1 funcs executed)
// EX L23-133: DoArithmetic(a, b any, op rune) (any, error)
// ---------------------------------------------------------------------------
