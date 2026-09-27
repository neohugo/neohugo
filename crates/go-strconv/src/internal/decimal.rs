// Port of go1.27.1 src/internal/strconv/decimal.go.
//
// Multiprecision decimal numbers.
// For floating-point formatting only; not general purpose.
// Only operations are assign and (binary) left/right shift.
// Can do binary floating point in multiprecision decimal precisely
// because 2 divides 10; cannot do decimal floating point
// in multiprecision binary precisely.

#[derive(Clone)]
pub struct Decimal {
    pub(crate) d: [u8; 800], // digits, big-endian representation
    pub(crate) nd: i64,      // number of digits used
    pub(crate) dp: i64,      // decimal point
    pub(crate) neg: bool,    // negative flag
    pub(crate) trunc: bool,  // discarded nonzero digits beyond d[:nd]
}

impl Default for Decimal {
    fn default() -> Self {
        Decimal {
            d: [0; 800],
            nd: 0,
            dp: 0,
            neg: false,
            trunc: false,
        }
    }
}

impl std::fmt::Display for Decimal {
    // Go: internal/strconv/decimal.go:(*decimal).String
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let a = self;
        let mut buf: Vec<u8> = Vec::new();
        if a.nd == 0 {
            return f.write_str("0");
        } else if a.dp <= 0 {
            // zeros fill space between decimal point and digits
            buf.push(b'0');
            buf.push(b'.');
            for _ in 0..-a.dp {
                buf.push(b'0');
            }
            buf.extend_from_slice(&a.d[0..a.nd as usize]);
        } else if a.dp < a.nd {
            // decimal point in middle of digits
            buf.extend_from_slice(&a.d[0..a.dp as usize]);
            buf.push(b'.');
            buf.extend_from_slice(&a.d[a.dp as usize..a.nd as usize]);
        } else {
            // zeros fill space between digits and decimal point
            buf.extend_from_slice(&a.d[0..a.nd as usize]);
            for _ in 0..a.dp - a.nd {
                buf.push(b'0');
            }
        }
        f.write_str(&String::from_utf8_lossy(&buf))
    }
}

// Go: internal/strconv/decimal.go:trim
/// trim trailing zeros from number.
/// (They are meaningless; the decimal point is tracked
/// independent of the number of digits.)
fn trim(a: &mut Decimal) {
    while a.nd > 0 && a.d[(a.nd - 1) as usize] == b'0' {
        a.nd -= 1;
    }
    if a.nd == 0 {
        a.dp = 0;
    }
}

/// Maximum shift that we can do in one pass without overflow.
/// A uint has 32 or 64 bits, and we have to be able to accommodate 9<<k.
const UINT_SIZE: u32 = 64;
const MAX_SHIFT: u32 = UINT_SIZE - 4;

impl Decimal {
    /// Returns a new decimal holding v (Go test helper NewDecimal).
    pub fn new(v: u64) -> Decimal {
        let mut d = Decimal::default();
        d.assign(v);
        d
    }

    // Go: internal/strconv/decimal.go:(*decimal).Assign
    /// Assign v to a.
    pub fn assign(&mut self, v: u64) {
        let a = self;
        let mut buf = [0u8; 24];
        let mut v = v;

        // Write reversed decimal in buf.
        let mut n = 0usize;
        while v > 0 {
            let v1 = v / 10;
            v -= 10 * v1;
            buf[n] = (v as u8) + b'0';
            n += 1;
            v = v1;
        }

        // Reverse again to produce forward decimal in a.d.
        a.nd = 0;
        while n > 0 {
            n -= 1;
            a.d[a.nd as usize] = buf[n];
            a.nd += 1;
        }
        a.dp = a.nd;
        trim(a);
    }

    // Go: internal/strconv/decimal.go:(*decimal).Shift
    /// Binary shift left (k > 0) or right (k < 0).
    pub fn shift(&mut self, k: i64) {
        let a = self;
        let mut k = k;
        if a.nd == 0 {
            // nothing to do: a == 0
        } else if k > 0 {
            while k > MAX_SHIFT as i64 {
                left_shift(a, MAX_SHIFT);
                k -= MAX_SHIFT as i64;
            }
            left_shift(a, k as u32);
        } else if k < 0 {
            while k < -(MAX_SHIFT as i64) {
                right_shift(a, MAX_SHIFT);
                k += MAX_SHIFT as i64;
            }
            right_shift(a, (-k) as u32);
        }
    }

    // Go: internal/strconv/decimal.go:(*decimal).Round
    /// Round a to nd digits (or fewer).
    /// If nd is zero, it means we're rounding
    /// just to the left of the digits, as in
    /// 0.09 -> 0.1.
    pub fn round(&mut self, nd: i64) {
        if nd < 0 || nd >= self.nd {
            return;
        }
        if should_round_up(self, nd) {
            self.round_up(nd);
        } else {
            self.round_down(nd);
        }
    }

    // Go: internal/strconv/decimal.go:(*decimal).RoundDown
    /// Round a down to nd digits (or fewer).
    pub fn round_down(&mut self, nd: i64) {
        if nd < 0 || nd >= self.nd {
            return;
        }
        self.nd = nd;
        trim(self);
    }

    // Go: internal/strconv/decimal.go:(*decimal).RoundUp
    /// Round a up to nd digits (or fewer).
    pub fn round_up(&mut self, nd: i64) {
        let a = self;
        if nd < 0 || nd >= a.nd {
            return;
        }

        // round up
        let mut i = nd - 1;
        while i >= 0 {
            let c = a.d[i as usize];
            if c < b'9' {
                // can stop after this digit
                a.d[i as usize] += 1;
                a.nd = i + 1;
                return;
            }
            i -= 1;
        }

        // Number is all 9s.
        // Change to single 1 with adjusted decimal point.
        a.d[0] = b'1';
        a.nd = 1;
        a.dp += 1;
    }

    // Go: internal/strconv/decimal.go:(*decimal).RoundedInteger
    /// Extract integer part, rounded appropriately.
    /// No guarantees about overflow.
    pub fn rounded_integer(&self) -> u64 {
        let a = self;
        if a.dp > 20 {
            return 0xFFFFFFFFFFFFFFFF;
        }
        let mut i = 0i64;
        let mut n = 0u64;
        while i < a.dp && i < a.nd {
            n = n
                .wrapping_mul(10)
                .wrapping_add((a.d[i as usize] - b'0') as u64);
            i += 1;
        }
        while i < a.dp {
            n = n.wrapping_mul(10);
            i += 1;
        }
        if should_round_up(a, a.dp) {
            n = n.wrapping_add(1);
        }
        n
    }
}

// Go: internal/strconv/decimal.go:rightShift
/// Binary shift right (/ 2) by k bits.  k <= maxShift to avoid overflow.
fn right_shift(a: &mut Decimal, k: u32) {
    let mut r: i64 = 0; // read pointer
    let mut w: i64 = 0; // write pointer

    // Pick up enough leading digits to cover first shift.
    let mut n: u64 = 0;
    while n >> k == 0 {
        if r >= a.nd {
            if n == 0 {
                // a == 0; shouldn't get here, but handle anyway.
                a.nd = 0;
                return;
            }
            while n >> k == 0 {
                n = n.wrapping_mul(10);
                r += 1;
            }
            break;
        }
        let c = a.d[r as usize] as u64;
        n = n.wrapping_mul(10).wrapping_add(c).wrapping_sub(b'0' as u64);
        r += 1;
    }
    a.dp -= r - 1;

    let mask: u64 = (1u64 << k) - 1;

    // Pick up a digit, put down a digit.
    while r < a.nd {
        let c = a.d[r as usize] as u64;
        let dig = n >> k;
        n &= mask;
        a.d[w as usize] = (dig as u8).wrapping_add(b'0');
        w += 1;
        n = n.wrapping_mul(10).wrapping_add(c).wrapping_sub(b'0' as u64);
        r += 1;
    }

    // Put down extra digits.
    while n > 0 {
        let dig = n >> k;
        n &= mask;
        if (w as usize) < a.d.len() {
            a.d[w as usize] = (dig as u8).wrapping_add(b'0');
            w += 1;
        } else if dig > 0 {
            a.trunc = true;
        }
        n = n.wrapping_mul(10);
    }

    a.nd = w;
    trim(a);
}

/// Cheat sheet for left shift: table indexed by shift count giving
/// number of new digits that will be introduced by that shift.
///
/// For example, leftcheats[4] = {2, "625"}.  That means that
/// if we are shifting by 4 (multiplying by 16), it will add 2 digits
/// when the string prefix is "625" through "999", and one fewer digit
/// if the string prefix is "000" through "624".
///
/// Credit for this trick goes to Ken.
struct LeftCheat {
    delta: i64,            // number of new digits
    cutoff: &'static [u8], // minus one digit if original < a.
}

macro_rules! lc {
    ($d:expr, $s:expr) => {
        LeftCheat {
            delta: $d,
            cutoff: $s,
        }
    };
}

static LEFTCHEATS: [LeftCheat; 61] = [
    // Leading digits of 1/2^i = 5^i.
    lc!(0, b""),
    lc!(1, b"5"),                                           // * 2
    lc!(1, b"25"),                                          // * 4
    lc!(1, b"125"),                                         // * 8
    lc!(2, b"625"),                                         // * 16
    lc!(2, b"3125"),                                        // * 32
    lc!(2, b"15625"),                                       // * 64
    lc!(3, b"78125"),                                       // * 128
    lc!(3, b"390625"),                                      // * 256
    lc!(3, b"1953125"),                                     // * 512
    lc!(4, b"9765625"),                                     // * 1024
    lc!(4, b"48828125"),                                    // * 2048
    lc!(4, b"244140625"),                                   // * 4096
    lc!(4, b"1220703125"),                                  // * 8192
    lc!(5, b"6103515625"),                                  // * 16384
    lc!(5, b"30517578125"),                                 // * 32768
    lc!(5, b"152587890625"),                                // * 65536
    lc!(6, b"762939453125"),                                // * 131072
    lc!(6, b"3814697265625"),                               // * 262144
    lc!(6, b"19073486328125"),                              // * 524288
    lc!(7, b"95367431640625"),                              // * 1048576
    lc!(7, b"476837158203125"),                             // * 2097152
    lc!(7, b"2384185791015625"),                            // * 4194304
    lc!(7, b"11920928955078125"),                           // * 8388608
    lc!(8, b"59604644775390625"),                           // * 16777216
    lc!(8, b"298023223876953125"),                          // * 33554432
    lc!(8, b"1490116119384765625"),                         // * 67108864
    lc!(9, b"7450580596923828125"),                         // * 134217728
    lc!(9, b"37252902984619140625"),                        // * 268435456
    lc!(9, b"186264514923095703125"),                       // * 536870912
    lc!(10, b"931322574615478515625"),                      // * 1073741824
    lc!(10, b"4656612873077392578125"),                     // * 2147483648
    lc!(10, b"23283064365386962890625"),                    // * 4294967296
    lc!(10, b"116415321826934814453125"),                   // * 8589934592
    lc!(11, b"582076609134674072265625"),                   // * 17179869184
    lc!(11, b"2910383045673370361328125"),                  // * 34359738368
    lc!(11, b"14551915228366851806640625"),                 // * 68719476736
    lc!(12, b"72759576141834259033203125"),                 // * 137438953472
    lc!(12, b"363797880709171295166015625"),                // * 274877906944
    lc!(12, b"1818989403545856475830078125"),               // * 549755813888
    lc!(13, b"9094947017729282379150390625"),               // * 1099511627776
    lc!(13, b"45474735088646411895751953125"),              // * 2199023255552
    lc!(13, b"227373675443232059478759765625"),             // * 4398046511104
    lc!(13, b"1136868377216160297393798828125"),            // * 8796093022208
    lc!(14, b"5684341886080801486968994140625"),            // * 17592186044416
    lc!(14, b"28421709430404007434844970703125"),           // * 35184372088832
    lc!(14, b"142108547152020037174224853515625"),          // * 70368744177664
    lc!(15, b"710542735760100185871124267578125"),          // * 140737488355328
    lc!(15, b"3552713678800500929355621337890625"),         // * 281474976710656
    lc!(15, b"17763568394002504646778106689453125"),        // * 562949953421312
    lc!(16, b"88817841970012523233890533447265625"),        // * 1125899906842624
    lc!(16, b"444089209850062616169452667236328125"),       // * 2251799813685248
    lc!(16, b"2220446049250313080847263336181640625"),      // * 4503599627370496
    lc!(16, b"11102230246251565404236316680908203125"),     // * 9007199254740992
    lc!(17, b"55511151231257827021181583404541015625"),     // * 18014398509481984
    lc!(17, b"277555756156289135105907917022705078125"),    // * 36028797018963968
    lc!(17, b"1387778780781445675529539585113525390625"),   // * 72057594037927936
    lc!(18, b"6938893903907228377647697925567626953125"),   // * 144115188075855872
    lc!(18, b"34694469519536141888238489627838134765625"),  // * 288230376151711744
    lc!(18, b"173472347597680709441192448139190673828125"), // * 576460752303423488
    lc!(19, b"867361737988403547205962240695953369140625"), // * 1152921504606846976
];

// Go: internal/strconv/decimal.go:prefixIsLessThan
/// Is the leading prefix of b lexicographically less than s?
fn prefix_is_less_than(b: &[u8], s: &[u8]) -> bool {
    for i in 0..s.len() {
        if i >= b.len() {
            return true;
        }
        if b[i] != s[i] {
            return b[i] < s[i];
        }
    }
    false
}

// Go: internal/strconv/decimal.go:leftShift
/// Binary shift left (* 2) by k bits.  k <= maxShift to avoid overflow.
fn left_shift(a: &mut Decimal, k: u32) {
    let mut delta = LEFTCHEATS[k as usize].delta;
    if prefix_is_less_than(&a.d[0..a.nd as usize], LEFTCHEATS[k as usize].cutoff) {
        delta -= 1;
    }

    let mut r = a.nd; // read index
    let mut w = a.nd + delta; // write index

    // Pick up a digit, put down a digit.
    let mut n: u64 = 0;
    r -= 1;
    while r >= 0 {
        n = n.wrapping_add(((a.d[r as usize] as u64).wrapping_sub(b'0' as u64)) << k);
        let quo = n / 10;
        let rem = n - 10 * quo;
        w -= 1;
        if (w as usize) < a.d.len() {
            a.d[w as usize] = (rem as u8) + b'0';
        } else if rem != 0 {
            a.trunc = true;
        }
        n = quo;
        r -= 1;
    }

    // Put down extra digits.
    while n > 0 {
        let quo = n / 10;
        let rem = n - 10 * quo;
        w -= 1;
        if (w as usize) < a.d.len() {
            a.d[w as usize] = (rem as u8) + b'0';
        } else if rem != 0 {
            a.trunc = true;
        }
        n = quo;
    }

    a.nd += delta;
    if a.nd >= a.d.len() as i64 {
        a.nd = a.d.len() as i64;
    }
    a.dp += delta;
    trim(a);
}

// Go: internal/strconv/decimal.go:shouldRoundUp
/// If we chop a at nd digits, should we round up?
pub(crate) fn should_round_up(a: &Decimal, nd: i64) -> bool {
    if nd < 0 || nd >= a.nd {
        return false;
    }
    if a.d[nd as usize] == b'5' && nd + 1 == a.nd {
        // exactly halfway - round to even
        // if we truncated, a little higher than what's recorded - always round up
        if a.trunc {
            return true;
        }
        return nd > 0 && (a.d[(nd - 1) as usize] - b'0') % 2 != 0;
    }
    // not halfway - digit tells all
    a.d[nd as usize] >= b'5'
}
