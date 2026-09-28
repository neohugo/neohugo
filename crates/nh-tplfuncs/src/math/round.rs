//! Port of `tpl/math/round.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

const MASK: u64 = 0x7FF;
const SHIFT: u64 = 64 - 11 - 1;
const BIAS: u64 = 1023;

// Go: tpl/math/round.go:_round
/// Round returns the nearest integer, rounding half away from zero.
///
/// Special cases are: Round(±0) = ±0, Round(±Inf) = ±Inf, Round(NaN) = NaN.
pub fn round(x: f64) -> f64 {
    const SIGN_MASK: u64 = 1 << 63;
    const FRAC_MASK: u64 = (1 << SHIFT) - 1;
    const HALF: u64 = 1 << (SHIFT - 1);
    const ONE: u64 = BIAS << SHIFT;

    let mut bits = x.to_bits();
    let mut e = (bits >> SHIFT) & MASK;
    if e < BIAS {
        // Round abs(x) < 1 including denormals.
        bits &= SIGN_MASK; // +-0
        if e == BIAS - 1 {
            bits |= ONE; // +-1
        }
    } else if e < BIAS + SHIFT {
        // Round any abs(x) >= 1 containing a fractional component [0,1).
        //
        // Numbers with larger exponents are returned unchanged since they
        // must be either an integer, infinity, or NaN.
        e -= BIAS;
        bits = bits.wrapping_add(HALF >> e);
        bits &= !(FRAC_MASK >> e);
    }
    f64::from_bits(bits)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/math/round.go (62 lines; 0/1 funcs executed)
// OK L27-62: _round(x float64) float64
// ---------------------------------------------------------------------------
