//! Port of `$GOROOT/src/fmt/format.go` (go1.27.1): the raw formatter used by
//! the printer for integers, floats, strings, byte slices and runes.
//!
//! Deviation: Go's `fmt` holds a pointer to the printer's buffer; here the
//! buffer is owned by [`Fmt`] and the printer writes through `fmt.buf`.

use go_strconv::Rune;
use go_unicode::utf8;

pub(crate) const LDIGITS: &[u8; 17] = b"0123456789abcdefx";
pub(crate) const UDIGITS: &[u8; 17] = b"0123456789ABCDEFX";

pub(crate) const SIGNED: bool = true;
pub(crate) const UNSIGNED: bool = false;

/// Go: `fmtFlags`, flags placed in a separate struct for easy clearing.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct FmtFlags {
    pub wid_present: bool,
    pub prec_present: bool,
    pub minus: bool,
    pub plus: bool,
    pub sharp: bool,
    pub space: bool,
    pub zero: bool,

    // For the formats %+v %#v, we set the plusV/sharpV flags
    // and clear the plus/sharp flags since %+v and %#v are in effect
    // different, flagless formats set at the top level.
    pub plus_v: bool,
    pub sharp_v: bool,
}

/// Go: `fmt`, the raw formatter used by Printf etc.
#[derive(Default)]
pub(crate) struct Fmt {
    /// Go: `*f.buf` (shared with the printer's `p.buf`).
    pub buf: Vec<u8>,

    pub f: FmtFlags,

    /// width
    pub wid: i64,
    /// precision
    pub prec: i64,
}

impl Fmt {
    // Go: fmt/format.go:(*fmt).clearflags
    pub(crate) fn clearflags(&mut self) {
        self.f = FmtFlags::default();
        self.wid = 0;
        self.prec = 0;
    }

    // Go: fmt/format.go:(*fmt).init
    #[allow(dead_code)]
    pub(crate) fn init(&mut self) {
        self.clearflags();
    }

    // Go: fmt/format.go:(*fmt).writePadding
    /// writePadding generates n bytes of padding.
    pub(crate) fn write_padding(&mut self, n: i64) {
        if n <= 0 {
            // No padding bytes needed.
            return;
        }
        // Decide which byte the padding should be filled with.
        let mut pad_byte = b' ';
        // Zero padding is allowed only to the left.
        if self.f.zero && !self.f.minus {
            pad_byte = b'0';
        }
        // Fill padding with padByte.
        let new_len = self.buf.len() + n as usize;
        self.buf.resize(new_len, pad_byte);
    }

    // Go: fmt/format.go:(*fmt).pad
    /// pad appends b to f.buf, padded on left (!f.minus) or right (f.minus).
    pub(crate) fn pad(&mut self, b: &[u8]) {
        if !self.f.wid_present || self.wid == 0 {
            self.buf.extend_from_slice(b);
            return;
        }
        let width = self.wid - utf8::rune_count(b) as i64;
        if !self.f.minus {
            // left padding
            self.write_padding(width);
            self.buf.extend_from_slice(b);
        } else {
            // right padding
            self.buf.extend_from_slice(b);
            self.write_padding(width);
        }
    }

    // Go: fmt/format.go:(*fmt).padString
    /// padString appends s to f.buf, padded on left (!f.minus) or right (f.minus).
    pub(crate) fn pad_string(&mut self, s: &[u8]) {
        if !self.f.wid_present || self.wid == 0 {
            self.buf.extend_from_slice(s);
            return;
        }
        let width = self.wid - utf8::rune_count_in_string(s) as i64;
        if !self.f.minus {
            // left padding
            self.write_padding(width);
            self.buf.extend_from_slice(s);
        } else {
            // right padding
            self.buf.extend_from_slice(s);
            self.write_padding(width);
        }
    }

    // Go: fmt/format.go:(*fmt).fmtBoolean
    /// fmtBoolean formats a boolean.
    pub(crate) fn fmt_boolean(&mut self, v: bool) {
        if v {
            self.pad_string(b"true");
        } else {
            self.pad_string(b"false");
        }
    }

    // Go: fmt/format.go:(*fmt).fmtUnicode
    /// fmtUnicode formats a uint64 as "U+0078" or with f.sharp set as "U+0078 'x'".
    pub(crate) fn fmt_unicode(&mut self, mut u: u64) {
        let mut intbuf = [0u8; 68];
        let mut big: Vec<u8>;
        let mut buf: &mut [u8] = &mut intbuf;

        // With default precision set the maximum needed buf length is 18
        // for formatting -1 with %#U ("U+FFFFFFFFFFFFFFFF") which fits
        // into the already allocated intbuf with a capacity of 68 bytes.
        let mut prec: i64 = 4;
        if self.f.prec_present && self.prec > 4 {
            prec = self.prec;
            // Compute space needed for "U+" , number, " '", character, "'".
            let width = 2 + prec + 2 + utf8::UTF_MAX as i64 + 1;
            if width > buf.len() as i64 {
                big = vec![0u8; width as usize];
                buf = &mut big;
            }
        }

        // Format into buf, ending at buf[i]. Formatting numbers is easier right-to-left.
        let mut i = buf.len();

        // For %#U we want to add a space and a quoted character at the end of the buffer.
        if self.f.sharp && u <= utf8::MAX_RUNE as u64 && go_strconv::is_print(u as Rune) {
            i -= 1;
            buf[i] = b'\'';
            i -= utf8::rune_len(u as Rune) as usize;
            utf8::encode_rune(&mut buf[i..], u as Rune);
            i -= 1;
            buf[i] = b'\'';
            i -= 1;
            buf[i] = b' ';
        }
        // Format the Unicode code point u as a hexadecimal number.
        while u >= 16 {
            i -= 1;
            buf[i] = UDIGITS[(u & 0xF) as usize];
            prec -= 1;
            u >>= 4;
        }
        i -= 1;
        buf[i] = UDIGITS[u as usize];
        prec -= 1;
        // Add zeros in front of the number until requested precision is reached.
        while prec > 0 {
            i -= 1;
            buf[i] = b'0';
            prec -= 1;
        }
        // Add a leading "U+".
        i -= 1;
        buf[i] = b'+';
        i -= 1;
        buf[i] = b'U';

        let old_zero = self.f.zero;
        self.f.zero = false;
        self.pad(&buf[i..]);
        self.f.zero = old_zero;
    }

    // Go: fmt/format.go:(*fmt).fmtInteger
    /// fmtInteger formats signed and unsigned integers.
    pub(crate) fn fmt_integer(
        &mut self,
        mut u: u64,
        base: i64,
        is_signed: bool,
        verb: Rune,
        digits: &[u8; 17],
    ) {
        let negative = is_signed && (u as i64) < 0;
        if negative {
            u = u.wrapping_neg();
        }

        let mut intbuf = [0u8; 68];
        let mut big: Vec<u8>;
        let mut buf: &mut [u8] = &mut intbuf;
        // The already allocated f.intbuf with a capacity of 68 bytes
        // is large enough for integer formatting when no precision or width is set.
        if self.f.wid_present || self.f.prec_present {
            // Account 3 extra bytes for possible addition of a sign and "0x".
            let width = 3 + self.wid + self.prec; // wid and prec are always positive.
            if width > buf.len() as i64 {
                // We're going to need a bigger boat.
                big = vec![0u8; width as usize];
                buf = &mut big;
            }
        }

        // Two ways to ask for extra leading zero digits: %.3d or %03d.
        // If both are specified the f.zero flag is ignored and
        // padding with spaces is used instead.
        let mut prec: i64 = 0;
        if self.f.prec_present {
            prec = self.prec;
            // Precision of 0 and value of 0 means "print nothing" but padding.
            if prec == 0 && u == 0 {
                let old_zero = self.f.zero;
                self.f.zero = false;
                self.write_padding(self.wid);
                self.f.zero = old_zero;
                return;
            }
        } else if self.f.zero && !self.f.minus && self.f.wid_present {
            // Zero padding is allowed only to the left.
            prec = self.wid;
            if negative || self.f.plus || self.f.space {
                prec -= 1; // leave room for sign
            }
        }

        // Because printing is easier right-to-left: format u into buf, ending at buf[i].
        // We could make things marginally faster by splitting the 32-bit case out
        // into a separate block but it's not worth the duplication, so u has 64 bits.
        let mut i = buf.len();
        // Use constants for the division and modulo for more efficient code.
        // Switch cases ordered by popularity.
        match base {
            10 => {
                while u >= 10 {
                    i -= 1;
                    let next = u / 10;
                    buf[i] = (b'0' as u64).wrapping_add(u).wrapping_sub(next * 10) as u8;
                    u = next;
                }
            }
            16 => {
                while u >= 16 {
                    i -= 1;
                    buf[i] = digits[(u & 0xF) as usize];
                    u >>= 4;
                }
            }
            8 => {
                while u >= 8 {
                    i -= 1;
                    buf[i] = b'0' + (u & 7) as u8;
                    u >>= 3;
                }
            }
            2 => {
                while u >= 2 {
                    i -= 1;
                    buf[i] = b'0' + (u & 1) as u8;
                    u >>= 1;
                }
            }
            _ => panic!("fmt: unknown base; can't happen"),
        }
        i -= 1;
        buf[i] = digits[u as usize];
        while i > 0 && prec > (buf.len() - i) as i64 {
            i -= 1;
            buf[i] = b'0';
        }

        // Various prefixes: 0x, -, etc.
        if self.f.sharp {
            match base {
                2 => {
                    // Add a leading 0b.
                    i -= 1;
                    buf[i] = b'b';
                    i -= 1;
                    buf[i] = b'0';
                }
                8 => {
                    if buf[i] != b'0' {
                        i -= 1;
                        buf[i] = b'0';
                    }
                }
                16 => {
                    // Add a leading 0x or 0X.
                    i -= 1;
                    buf[i] = digits[16];
                    i -= 1;
                    buf[i] = b'0';
                }
                _ => {}
            }
        }
        if verb == 'O' as Rune {
            i -= 1;
            buf[i] = b'o';
            i -= 1;
            buf[i] = b'0';
        }

        if negative {
            i -= 1;
            buf[i] = b'-';
        } else if self.f.plus {
            i -= 1;
            buf[i] = b'+';
        } else if self.f.space {
            i -= 1;
            buf[i] = b' ';
        }

        // Left padding with zeros has already been handled like precision earlier
        // or the f.zero flag is ignored due to an explicitly set precision.
        let old_zero = self.f.zero;
        self.f.zero = false;
        self.pad(&buf[i..]);
        self.f.zero = old_zero;
    }

    // Go: fmt/format.go:(*fmt).truncateString
    /// truncateString truncates the string s to the specified precision, if present.
    pub(crate) fn truncate_string<'a>(&self, s: &'a [u8]) -> &'a [u8] {
        if self.f.prec_present {
            let mut n = self.prec;
            // for i := range s
            let mut i = 0;
            while i < s.len() {
                n -= 1;
                if n < 0 {
                    return &s[..i];
                }
                let (_, wid) = utf8::decode_rune_in_string(&s[i..]);
                i += wid;
            }
        }
        s
    }

    // Go: fmt/format.go:(*fmt).truncate
    /// truncate truncates the byte slice b as a string of the specified precision, if present.
    pub(crate) fn truncate<'a>(&self, b: &'a [u8]) -> &'a [u8] {
        if self.f.prec_present {
            let mut n = self.prec;
            let mut i = 0;
            while i < b.len() {
                n -= 1;
                if n < 0 {
                    return &b[..i];
                }
                let (_, wid) = utf8::decode_rune(&b[i..]);
                i += wid;
            }
        }
        b
    }

    // Go: fmt/format.go:(*fmt).fmtS
    /// fmtS formats a string.
    pub(crate) fn fmt_s(&mut self, s: &[u8]) {
        let s = self.truncate_string(s);
        self.pad_string(s);
    }

    // Go: fmt/format.go:(*fmt).fmtBs
    /// fmtBs formats the byte slice b as if it was formatted as string with fmtS.
    pub(crate) fn fmt_bs(&mut self, b: &[u8]) {
        let b = self.truncate(b);
        self.pad(b);
    }

    // Go: fmt/format.go:(*fmt).fmtSbx
    /// fmtSbx formats a string or byte slice as a hexadecimal encoding of its bytes.
    /// `b == None` is Go's `b == nil` (the string s is encoded).
    pub(crate) fn fmt_sbx(&mut self, s: &[u8], b: Option<&[u8]>, digits: &[u8; 17]) {
        let mut length = b.map_or(0, |b| b.len()) as i64;
        if b.is_none() {
            // No byte slice present. Assume string s should be encoded.
            length = s.len() as i64;
        }
        // Set length to not process more bytes than the precision demands.
        if self.f.prec_present && self.prec < length {
            length = self.prec;
        }
        // Compute width of the encoding taking into account the f.sharp and f.space flag.
        let mut width = 2 * length;
        if width > 0 {
            if self.f.space {
                // Each element encoded by two hexadecimals will get a leading 0x or 0X.
                if self.f.sharp {
                    width *= 2;
                }
                // Elements will be separated by a space.
                width += length - 1;
            } else if self.f.sharp {
                // Only a leading 0x or 0X will be added for the whole string.
                width += 2;
            }
        } else {
            // The byte slice or string that should be encoded is empty.
            if self.f.wid_present {
                self.write_padding(self.wid);
            }
            return;
        }
        // Handle padding to the left.
        if self.f.wid_present && self.wid > width && !self.f.minus {
            self.write_padding(self.wid - width);
        }
        // Write the encoding directly into the output buffer.
        let buf = &mut self.buf;
        if self.f.sharp {
            // Add leading 0x or 0X.
            buf.push(b'0');
            buf.push(digits[16]);
        }
        let mut c: u8;
        for i in 0..length as usize {
            if self.f.space && i > 0 {
                // Separate elements with a space.
                buf.push(b' ');
                if self.f.sharp {
                    // Add leading 0x or 0X for each element.
                    buf.push(b'0');
                    buf.push(digits[16]);
                }
            }
            if let Some(b) = b {
                c = b[i]; // Take a byte from the input byte slice.
            } else {
                c = s[i]; // Take a byte from the input string.
            }
            // Encode each byte as two hexadecimal digits.
            buf.push(digits[(c >> 4) as usize]);
            buf.push(digits[(c & 0xF) as usize]);
        }
        // Handle padding to the right.
        if self.f.wid_present && self.wid > width && self.f.minus {
            self.write_padding(self.wid - width);
        }
    }

    // Go: fmt/format.go:(*fmt).fmtSx
    /// fmtSx formats a string as a hexadecimal encoding of its bytes.
    pub(crate) fn fmt_sx(&mut self, s: &[u8], digits: &[u8; 17]) {
        self.fmt_sbx(s, None, digits);
    }

    // Go: fmt/format.go:(*fmt).fmtBx
    /// fmtBx formats a byte slice as a hexadecimal encoding of its bytes.
    pub(crate) fn fmt_bx(&mut self, b: &[u8], digits: &[u8; 17]) {
        self.fmt_sbx(b"", Some(b), digits);
    }

    // Go: fmt/format.go:(*fmt).fmtQ
    /// fmtQ formats a string as a double-quoted, escaped Go string constant.
    /// If f.sharp is set a raw (backquoted) string may be returned instead
    /// if the string does not contain any control characters other than tab.
    pub(crate) fn fmt_q(&mut self, s: &[u8]) {
        let s = self.truncate_string(s);
        if self.f.sharp && go_strconv::can_backquote(s) {
            let mut q = Vec::with_capacity(s.len() + 2);
            q.push(b'`');
            q.extend_from_slice(s);
            q.push(b'`');
            self.pad_string(&q);
            return;
        }
        let mut buf = Vec::new();
        if self.f.plus {
            go_strconv::append_quote_to_ascii(&mut buf, s);
        } else {
            go_strconv::append_quote(&mut buf, s);
        }
        self.pad(&buf);
    }

    // Go: fmt/format.go:(*fmt).fmtC
    /// fmtC formats an integer as a Unicode character.
    /// If the character is not valid Unicode, it will print '�'.
    pub(crate) fn fmt_c(&mut self, c: u64) {
        // Explicitly check whether c exceeds utf8.MaxRune since the conversion
        // of a uint64 to a rune may lose precision that indicates an overflow.
        let mut r = c as Rune;
        if c > utf8::MAX_RUNE as u64 {
            r = utf8::RUNE_ERROR;
        }
        let mut buf = Vec::with_capacity(4);
        utf8::append_rune(&mut buf, r);
        self.pad(&buf);
    }

    // Go: fmt/format.go:(*fmt).fmtQc
    /// fmtQc formats an integer as a single-quoted, escaped Go character constant.
    /// If the character is not valid Unicode, it will print '�'.
    pub(crate) fn fmt_qc(&mut self, c: u64) {
        let mut r = c as Rune;
        if c > utf8::MAX_RUNE as u64 {
            r = utf8::RUNE_ERROR;
        }
        let mut buf = Vec::new();
        if self.f.plus {
            go_strconv::append_quote_rune_to_ascii(&mut buf, r);
        } else {
            go_strconv::append_quote_rune(&mut buf, r);
        }
        self.pad(&buf);
    }

    // Go: fmt/format.go:(*fmt).fmtFloat
    /// fmtFloat formats a float64. It assumes that verb is a valid format specifier
    /// for strconv.AppendFloat and therefore fits into a byte.
    pub(crate) fn fmt_float(&mut self, v: f64, size: i64, verb: Rune, mut prec: i64) {
        // Explicit precision in format specifier overrules default precision.
        if self.f.prec_present {
            prec = self.prec;
        }
        // Format number, reserving space for leading + sign if needed.
        // Go: strconv.AppendFloat(f.intbuf[:1], ...): num[0] is a placeholder.
        let mut num: Vec<u8> = vec![0u8];
        go_strconv::append_float(&mut num, v, verb as u8, prec, size);
        // `start` is Go's re-slicing `num = num[1:]`.
        let mut start = 0usize;
        if num[1] == b'-' || num[1] == b'+' {
            start = 1;
        } else {
            num[0] = b'+';
        }
        // f.space means to add a leading space instead of a "+" sign unless
        // the sign is explicitly asked for by f.plus.
        if self.f.space && num[start] == b'+' && !self.f.plus {
            num[start] = b' ';
        }
        // Special handling for infinities and NaN,
        // which don't look like a number so shouldn't be padded with zeros.
        if num[start + 1] == b'I' || num[start + 1] == b'N' {
            let old_zero = self.f.zero;
            self.f.zero = false;
            // Remove sign before NaN if not asked for.
            if num[start + 1] == b'N' && !self.f.space && !self.f.plus {
                start += 1;
            }
            self.pad(&num[start..]);
            self.f.zero = old_zero;
            return;
        }
        let mut num: Vec<u8> = num.split_off(start);
        // The sharp flag forces printing a decimal point for non-binary formats
        // and retains trailing zeros, which we may need to restore.
        if self.f.sharp && verb != 'b' as Rune {
            let mut digits: i64 = 0;
            if verb == 'v' as Rune
                || verb == 'g' as Rune
                || verb == 'G' as Rune
                || verb == 'x' as Rune
            {
                digits = prec;
                // If no precision is set explicitly use a precision of 6.
                if digits == -1 {
                    digits = 6;
                }
            }

            // Buffer pre-allocated with enough room for
            // exponent notations of the form "e+123" or "p-1023".
            let mut tail: Vec<u8> = Vec::with_capacity(6);

            let mut has_decimal_point = false;
            let mut saw_nonzero_digit = false;
            // Starting from i = 1 to skip sign at num[0].
            let mut i = 1;
            while i < num.len() {
                let c = num[i];
                let mut is_default = false;
                match c {
                    b'.' => has_decimal_point = true,
                    b'p' | b'P' => {
                        tail.extend_from_slice(&num[i..]);
                        num.truncate(i);
                    }
                    b'e' | b'E' => {
                        if verb != 'x' as Rune && verb != 'X' as Rune {
                            tail.extend_from_slice(&num[i..]);
                            num.truncate(i);
                        } else {
                            // fallthrough
                            is_default = true;
                        }
                    }
                    _ => is_default = true,
                }
                if is_default {
                    if c != b'0' {
                        saw_nonzero_digit = true;
                    }
                    // Count significant digits after the first non-zero digit.
                    if saw_nonzero_digit {
                        digits -= 1;
                    }
                }
                i += 1;
            }
            if !has_decimal_point {
                // Leading digit 0 should contribute once to digits.
                if num.len() == 2 && num[1] == b'0' {
                    digits -= 1;
                }
                num.push(b'.');
            }
            while digits > 0 {
                num.push(b'0');
                digits -= 1;
            }
            num.extend_from_slice(&tail);
        }
        // We want a sign if asked for and if the sign is not positive.
        if self.f.plus || num[0] != b'+' {
            // If we're zero padding to the left we want the sign before the leading zeros.
            // Achieve this by writing the sign out and then padding the unsigned number.
            // Zero padding is allowed only to the left.
            if self.f.zero && !self.f.minus && self.f.wid_present && self.wid > num.len() as i64 {
                self.buf.push(num[0]);
                self.write_padding(self.wid - num.len() as i64);
                self.buf.extend_from_slice(&num[1..]);
                return;
            }
            self.pad(&num);
            return;
        }
        // No sign to show and the number is positive; just print the unsigned number.
        self.pad(&num[1..]);
    }
}
