//! Port of the shared `fn*` functions of `github.com/gohugoio/localescompressed@v1.0.1`
//! (`locales.autogen.go`) that the en and th translators use. Each keeps its Go name (in snake
//! case, with the content hash); `tables.rs` assigns them to the `localen` fields exactly as
//! `translatorFuncs["en"]`/`["th"]` do.

use go_time::GoTimeExt;
use go_value::Time;

use super::PluralRule;
use super::localen::{Localen, at};
use crate::herrors::{Error, Result};

// ---------------------------------------------------------------------------
// Helpers for Go runtime semantics

/// `strconv.AppendInt(b, i, 10)`.
fn append_int(b: &mut Vec<u8>, i: i64) {
    go_strconv::append_int(b, i, 10);
}

/// `string(b)` of bytes built from UTF-8 pieces (always valid).
fn string(b: Vec<u8>) -> String {
    match String::from_utf8(b) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }
}

/// Go's bounds check of `s[:high]` on a string of length `len`: the runtime panic message.
fn check_slice_high(high: i64, len: usize) -> Result<()> {
    if high < 0 {
        return Err(Error::new(format!(
            "runtime error: slice bounds out of range [:{high}]"
        )));
    }
    if high > len as i64 {
        return Err(Error::new(format!(
            "runtime error: slice bounds out of range [:{high}] with length {len}"
        )));
    }
    Ok(())
}

/// Go's `ln.currencies[currency]`, with the runtime panic message when out of range.
fn currency_symbol(ln: &Localen, currency: i64) -> Result<&'static str> {
    if currency < 0 {
        return Err(Error::new(format!(
            "runtime error: index out of range [{currency}]"
        )));
    }
    if currency >= ln.currencies.len() as i64 {
        return Err(Error::new(format!(
            "runtime error: index out of range [{currency}] with length {}",
            ln.currencies.len()
        )));
    }
    Ok(ln.currencies[currency as usize])
}

/// `strconv.FormatFloat(math.Abs(num), 'f', int(v), 64)`.
fn format_abs(num: f64, v: u64) -> Vec<u8> {
    go_strconv::format_float(num.abs(), b'f', v as i64, 64).into_bytes()
}

/// `for i, j := 0, len(b)-1; i < j; i, j = i+1, j-1 { b[i], b[j] = b[j], b[i] }`.
fn reverse(b: &mut [u8]) {
    b.reverse();
}

// ---------------------------------------------------------------------------
// Data accessors

// Go: localescompressed locales.autogen.go:fnLocale_5a217059c67094defae82f249f55f4c5
pub(super) fn fn_locale_5a217059c67094defae82f249f55f4c5(ln: &Localen) -> &'static str {
    ln.locale
}

// Go: localescompressed locales.autogen.go:fnPluralsCardinal_85b19cac1fab9f2c2cfb9eb41539aa0e
pub(super) fn fn_plurals_cardinal_85b19cac1fab9f2c2cfb9eb41539aa0e(
    ln: &Localen,
) -> &'static [PluralRule] {
    ln.plurals_cardinal
}

// Go: localescompressed locales.autogen.go:fnPluralsOrdinal_16a60aa09da9778fa82fe463e83076de
pub(super) fn fn_plurals_ordinal_16a60aa09da9778fa82fe463e83076de(
    ln: &Localen,
) -> &'static [PluralRule] {
    ln.plurals_ordinal
}

// Go: localescompressed locales.autogen.go:fnPluralsRange_332465bda13eb808b3664fa0500f54ce
pub(super) fn fn_plurals_range_332465bda13eb808b3664fa0500f54ce(
    ln: &Localen,
) -> &'static [PluralRule] {
    ln.plurals_range
}

// Go: localescompressed locales.autogen.go:fnMonthAbbreviated_aa9a9957cd9d056ba078b36a08b17478
pub(super) fn fn_month_abbreviated_aa9a9957cd9d056ba078b36a08b17478(
    ln: &Localen,
    month: i64,
) -> &'static str {
    at(ln.months_abbreviated, month)
}

// Go: localescompressed locales.autogen.go:fnMonthsAbbreviated_181f77804b62232997fae36bd57d6cf7
pub(super) fn fn_months_abbreviated_181f77804b62232997fae36bd57d6cf7(
    ln: &Localen,
) -> &'static [&'static str] {
    &ln.months_abbreviated[1..]
}

// Go: localescompressed locales.autogen.go:fnMonthNarrow_45916040ecf6fc350894d9e3e6c46dc7
pub(super) fn fn_month_narrow_45916040ecf6fc350894d9e3e6c46dc7(
    ln: &Localen,
    month: i64,
) -> &'static str {
    at(ln.months_narrow, month)
}

// Go: localescompressed locales.autogen.go:fnMonthsNarrow_75daa2ba82ae9577292175c3f52370cc
pub(super) fn fn_months_narrow_75daa2ba82ae9577292175c3f52370cc(
    ln: &Localen,
) -> &'static [&'static str] {
    &ln.months_narrow[1..]
}

// Go: localescompressed locales.autogen.go:fnMonthWide_4c946a7e7581c665e7407df1841810b5
pub(super) fn fn_month_wide_4c946a7e7581c665e7407df1841810b5(
    ln: &Localen,
    month: i64,
) -> &'static str {
    at(ln.months_wide, month)
}

// Go: localescompressed locales.autogen.go:fnMonthsWide_0ecf36a8b72255e2fee9467af362e989
pub(super) fn fn_months_wide_0ecf36a8b72255e2fee9467af362e989(
    ln: &Localen,
) -> &'static [&'static str] {
    &ln.months_wide[1..]
}

// Go: localescompressed locales.autogen.go:fnWeekdayAbbreviated_4ffbdfbd2be10e6bd8fb5a542a14664a
pub(super) fn fn_weekday_abbreviated_4ffbdfbd2be10e6bd8fb5a542a14664a(
    ln: &Localen,
    weekday: i64,
) -> &'static str {
    at(ln.days_abbreviated, weekday)
}

// Go: localescompressed locales.autogen.go:fnWeekdaysAbbreviated_454a3d7d23c662520a64ae5245d404be
pub(super) fn fn_weekdays_abbreviated_454a3d7d23c662520a64ae5245d404be(
    ln: &Localen,
) -> &'static [&'static str] {
    ln.days_abbreviated
}

// Go: localescompressed locales.autogen.go:fnWeekdayNarrow_e86921b3446df5ea9afe9504cfa20d1a
pub(super) fn fn_weekday_narrow_e86921b3446df5ea9afe9504cfa20d1a(
    ln: &Localen,
    weekday: i64,
) -> &'static str {
    at(ln.days_narrow, weekday)
}

// Go: localescompressed locales.autogen.go:fnWeekdaysNarrow_d23eb0b7a498c40ce19d1bf231cad524
pub(super) fn fn_weekdays_narrow_d23eb0b7a498c40ce19d1bf231cad524(
    ln: &Localen,
) -> &'static [&'static str] {
    ln.days_narrow
}

// Go: localescompressed locales.autogen.go:fnWeekdayShort_82b231d4f504c6d71c7e495aa35cd3c2
pub(super) fn fn_weekday_short_82b231d4f504c6d71c7e495aa35cd3c2(
    ln: &Localen,
    weekday: i64,
) -> &'static str {
    at(ln.days_short, weekday)
}

// Go: localescompressed locales.autogen.go:fnWeekdaysShort_eb6ee64d01e314991432b4469790d38b
pub(super) fn fn_weekdays_short_eb6ee64d01e314991432b4469790d38b(
    ln: &Localen,
) -> &'static [&'static str] {
    ln.days_short
}

// Go: localescompressed locales.autogen.go:fnWeekdayWide_ab7e026d8685cefd89c5aeb5df512a9a
pub(super) fn fn_weekday_wide_ab7e026d8685cefd89c5aeb5df512a9a(
    ln: &Localen,
    weekday: i64,
) -> &'static str {
    at(ln.days_wide, weekday)
}

// Go: localescompressed locales.autogen.go:fnWeekdaysWide_5e2bbf75a31e730429c100e8a0bd3ebd
pub(super) fn fn_weekdays_wide_5e2bbf75a31e730429c100e8a0bd3ebd(
    ln: &Localen,
) -> &'static [&'static str] {
    ln.days_wide
}

// Go: localescompressed locales.autogen.go:fnDecimal_a78485e7dd32c95f41af4e18420793bf
pub(super) fn fn_decimal_a78485e7dd32c95f41af4e18420793bf(ln: &Localen) -> &'static str {
    ln.decimal
}

// Go: localescompressed locales.autogen.go:fnGroup_5c4b80a5ccdee81467241784ab8fcd56
pub(super) fn fn_group_5c4b80a5ccdee81467241784ab8fcd56(ln: &Localen) -> &'static str {
    ln.group
}

// Go: localescompressed locales.autogen.go:fnMinus_8b8229905258388dc16a5e0c270b0488
pub(super) fn fn_minus_8b8229905258388dc16a5e0c270b0488(ln: &Localen) -> &'static str {
    ln.minus
}

// ---------------------------------------------------------------------------
// Plural rules

// Go: localescompressed locales.autogen.go:fnCardinalPluralRule_c037ba21b7495f414cf38823ca036d30 (en)
pub(super) fn fn_cardinal_plural_rule_c037ba21b7495f414cf38823ca036d30(
    _ln: &Localen,
    num: f64,
    v: u64,
) -> PluralRule {
    let n = num.abs();
    // Go `int64(n)`: truncation; out of range saturates and NaN is 0 on arm64 (Rust `as`).
    let i = n as i64;

    if i == 1 && v == 0 {
        return PluralRule::One;
    }

    PluralRule::Other
}

// Go: localescompressed locales.autogen.go:fnCardinalPluralRule_18b4522c3f6126d1e01414d5e423288b (th)
pub(super) fn fn_cardinal_plural_rule_18b4522c3f6126d1e01414d5e423288b(
    _ln: &Localen,
    _num: f64,
    _v: u64,
) -> PluralRule {
    PluralRule::Other
}

// Go: localescompressed locales.autogen.go:fnOrdinalPluralRule_cd651b8fbb843044a30f45f56d776cdd (en)
pub(super) fn fn_ordinal_plural_rule_cd651b8fbb843044a30f45f56d776cdd(
    _ln: &Localen,
    num: f64,
    _v: u64,
) -> PluralRule {
    let n = num.abs();
    // Go `math.Mod` is exact, as is IEEE `fmod` (Rust `%` on f64).
    let n_mod100 = n % 100.0;
    let n_mod10 = n % 10.0;

    if n_mod10 == 1.0 && n_mod100 != 11.0 {
        return PluralRule::One;
    } else if n_mod10 == 2.0 && n_mod100 != 12.0 {
        return PluralRule::Two;
    } else if n_mod10 == 3.0 && n_mod100 != 13.0 {
        return PluralRule::Few;
    }

    PluralRule::Other
}

// Go: localescompressed locales.autogen.go:fnOrdinalPluralRule_3355865f05dde9af2a36ba415bea85cb (th)
pub(super) fn fn_ordinal_plural_rule_3355865f05dde9af2a36ba415bea85cb(
    _ln: &Localen,
    _num: f64,
    _v: u64,
) -> PluralRule {
    PluralRule::Other
}

// Go: localescompressed locales.autogen.go:fnRangePluralRule_91a918d93f773c0b7589da5ebb6c4164
pub(super) fn fn_range_plural_rule_91a918d93f773c0b7589da5ebb6c4164(
    _ln: &Localen,
    _num1: f64,
    _v1: u64,
    _num2: f64,
    _v2: u64,
) -> PluralRule {
    PluralRule::Other
}

// ---------------------------------------------------------------------------
// Numbers

// Go: localescompressed locales.autogen.go:fnFmtNumber_fc1f852c40326b41ed4436818cad7bfb
pub(super) fn fn_fmt_number_fc1f852c40326b41ed4436818cad7bfb(
    ln: &Localen,
    num: f64,
    v: u64,
) -> Result<String> {
    let s = format_abs(num, v);
    // l := len(s) + 2 + 1*len(s[:len(s)-int(v)-1])/3
    let high = (s.len() as i64).wrapping_sub(v as i64).wrapping_sub(1);
    check_slice_high(high, s.len())?;
    let l = s.len() + 2 + high as usize / 3;
    let mut count = 0;
    let mut in_whole = v == 0;
    let mut b: Vec<u8> = Vec::with_capacity(l);

    for i in (0..s.len()).rev() {
        if s[i] == b'.' {
            b.push(ln.decimal.as_bytes()[0]);
            in_whole = true;
            continue;
        }

        if in_whole {
            if count == 3 {
                b.push(ln.group.as_bytes()[0]);
                count = 1;
            } else {
                count += 1;
            }
        }

        b.push(s[i]);
    }

    if num < 0.0 {
        b.push(ln.minus.as_bytes()[0]);
    }

    reverse(&mut b);

    Ok(string(b))
}

// Go: localescompressed locales.autogen.go:fnFmtPercent_1c35b3868bf10d0e69b22b2e80c40671
pub(super) fn fn_fmt_percent_1c35b3868bf10d0e69b22b2e80c40671(
    ln: &Localen,
    num: f64,
    v: u64,
) -> String {
    let s = format_abs(num, v);
    let l = s.len() + 3;
    let mut b: Vec<u8> = Vec::with_capacity(l);

    for i in (0..s.len()).rev() {
        if s[i] == b'.' {
            b.push(ln.decimal.as_bytes()[0]);
            continue;
        }

        b.push(s[i]);
    }

    if num < 0.0 {
        b.push(ln.minus.as_bytes()[0]);
    }

    reverse(&mut b);

    b.extend_from_slice(ln.percent.as_bytes());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtCurrency_6c366a94338df615a7797e45c90cead1
pub(super) fn fn_fmt_currency_6c366a94338df615a7797e45c90cead1(
    ln: &Localen,
    num: f64,
    v: u64,
    currency: i64,
) -> Result<String> {
    let s = format_abs(num, v);
    let symbol = currency_symbol(ln, currency)?.as_bytes();
    let high = (s.len() as i64).wrapping_sub(v as i64).wrapping_sub(1);
    check_slice_high(high, s.len())?;
    let l = s.len() + symbol.len() + 2 + high as usize / 3;
    let mut count = 0;
    let mut in_whole = v == 0;
    let mut b: Vec<u8> = Vec::with_capacity(l);

    for i in (0..s.len()).rev() {
        if s[i] == b'.' {
            b.push(ln.decimal.as_bytes()[0]);
            in_whole = true;
            continue;
        }

        if in_whole {
            if count == 3 {
                b.push(ln.group.as_bytes()[0]);
                count = 1;
            } else {
                count += 1;
            }
        }

        b.push(s[i]);
    }

    for j in (0..symbol.len()).rev() {
        b.push(symbol[j]);
    }

    if num < 0.0 {
        b.push(ln.minus.as_bytes()[0]);
    }

    reverse(&mut b);

    let vi = v as i64;
    if vi < 2 {
        if v == 0 {
            b.extend_from_slice(ln.decimal.as_bytes());
        }

        let mut i: i64 = 0;
        while i < 2i64.wrapping_sub(vi) {
            b.push(b'0');
            i += 1;
        }
    }

    Ok(string(b))
}

// Go: localescompressed locales.autogen.go:fnFmtAccounting_632207f09bf5cde35a195cefb6bc8623
pub(super) fn fn_fmt_accounting_632207f09bf5cde35a195cefb6bc8623(
    ln: &Localen,
    num: f64,
    v: u64,
    currency: i64,
) -> Result<String> {
    let s = format_abs(num, v);
    let symbol = currency_symbol(ln, currency)?.as_bytes();
    let high = (s.len() as i64).wrapping_sub(v as i64).wrapping_sub(1);
    check_slice_high(high, s.len())?;
    let l = s.len() + symbol.len() + 4 + high as usize / 3;
    let mut count = 0;
    let mut in_whole = v == 0;
    let mut b: Vec<u8> = Vec::with_capacity(l);

    for i in (0..s.len()).rev() {
        if s[i] == b'.' {
            b.push(ln.decimal.as_bytes()[0]);
            in_whole = true;
            continue;
        }

        if in_whole {
            if count == 3 {
                b.push(ln.group.as_bytes()[0]);
                count = 1;
            } else {
                count += 1;
            }
        }

        b.push(s[i]);
    }

    if num < 0.0 {
        for j in (0..symbol.len()).rev() {
            b.push(symbol[j]);
        }

        b.push(ln.currency_negative_prefix.as_bytes()[0]);
    } else {
        for j in (0..symbol.len()).rev() {
            b.push(symbol[j]);
        }
    }

    reverse(&mut b);

    let vi = v as i64;
    if vi < 2 {
        if v == 0 {
            b.extend_from_slice(ln.decimal.as_bytes());
        }

        let mut i: i64 = 0;
        while i < 2i64.wrapping_sub(vi) {
            b.push(b'0');
            i += 1;
        }
    }

    if num < 0.0 {
        b.extend_from_slice(ln.currency_negative_suffix.as_bytes());
    }

    Ok(string(b))
}

// ---------------------------------------------------------------------------
// Dates

/// `if t.Year() > 0 { AppendInt(b, Year) } else { AppendInt(b, -Year) }`.
fn append_abs_year(b: &mut Vec<u8>, year: i64) {
    if year > 0 {
        append_int(b, year);
    } else {
        append_int(b, year.wrapping_neg());
    }
}

/// `if t.Year() > 9 { Itoa(Year)[2:] } else { Itoa(Year)[1:] }`.
fn append_short_year(b: &mut Vec<u8>, year: i64) {
    let y = go_strconv::itoa(year);
    if year > 9 {
        b.extend_from_slice(&y.as_bytes()[2..]);
    } else {
        b.extend_from_slice(&y.as_bytes()[1..]);
    }
}

// Go: localescompressed locales.autogen.go:fnFmtDateShort_1cd909a8714b4f1989f9f3479488b0ad (en)
pub(super) fn fn_fmt_date_short_1cd909a8714b4f1989f9f3479488b0ad(
    _ln: &Localen,
    t: &Time,
) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_int(&mut b, t.month().0);
    b.push(0x2f);
    append_int(&mut b, t.day());
    b.push(0x2f);

    append_short_year(&mut b, t.year());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtDateShort_f44877b6023a2d0fb0de41f9e2966ad0 (th)
pub(super) fn fn_fmt_date_short_f44877b6023a2d0fb0de41f9e2966ad0(
    _ln: &Localen,
    t: &Time,
) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_int(&mut b, t.day());
    b.push(0x2f);
    append_int(&mut b, t.month().0);
    b.push(0x2f);

    append_short_year(&mut b, t.year());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtDateMedium_41fae806abc2b79c1a1d2ac50b3fd4dc (en)
pub(super) fn fn_fmt_date_medium_41fae806abc2b79c1a1d2ac50b3fd4dc(
    ln: &Localen,
    t: &Time,
) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    b.extend_from_slice(at(ln.months_abbreviated, t.month().0).as_bytes());
    b.push(0x20);
    append_int(&mut b, t.day());
    b.extend_from_slice(&[0x2c, 0x20]);

    append_abs_year(&mut b, t.year());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtDateMedium_cb43e8c32d5cd0a27ac6db293a7981fd (th)
pub(super) fn fn_fmt_date_medium_cb43e8c32d5cd0a27ac6db293a7981fd(
    ln: &Localen,
    t: &Time,
) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_int(&mut b, t.day());
    b.push(0x20);
    b.extend_from_slice(at(ln.months_abbreviated, t.month().0).as_bytes());
    b.push(0x20);

    append_abs_year(&mut b, t.year());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtDateLong_ea5b1048b05196ddfdf5985a09a1d219 (en)
pub(super) fn fn_fmt_date_long_ea5b1048b05196ddfdf5985a09a1d219(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    b.extend_from_slice(at(ln.months_wide, t.month().0).as_bytes());
    b.push(0x20);
    append_int(&mut b, t.day());
    b.extend_from_slice(&[0x2c, 0x20]);

    append_abs_year(&mut b, t.year());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtDateLong_4ba622cfb6604f92efa67396648b01e1 (th)
pub(super) fn fn_fmt_date_long_4ba622cfb6604f92efa67396648b01e1(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_int(&mut b, t.day());
    b.push(0x20);
    b.extend_from_slice(at(ln.months_wide, t.month().0).as_bytes());
    b.push(0x20);

    if t.year() < 0 {
        b.extend_from_slice(ln.eras_abbreviated[0].as_bytes());
    } else {
        b.extend_from_slice(ln.eras_abbreviated[1].as_bytes());
    }

    b.push(0x20);

    append_abs_year(&mut b, t.year());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtDateFull_5b5e153ac164177593d63740c82296b3 (en)
pub(super) fn fn_fmt_date_full_5b5e153ac164177593d63740c82296b3(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    b.extend_from_slice(at(ln.days_wide, t.weekday().0).as_bytes());
    b.extend_from_slice(&[0x2c, 0x20]);
    b.extend_from_slice(at(ln.months_wide, t.month().0).as_bytes());
    b.push(0x20);
    append_int(&mut b, t.day());
    b.extend_from_slice(&[0x2c, 0x20]);

    append_abs_year(&mut b, t.year());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtDateFull_2a2625729a79b75f22677ae0545a3802 (th)
pub(super) fn fn_fmt_date_full_2a2625729a79b75f22677ae0545a3802(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    b.extend_from_slice(at(ln.days_wide, t.weekday().0).as_bytes());
    b.extend_from_slice(&[0xe0, 0xb8, 0x97, 0xe0, 0xb8, 0xb5, 0xe0, 0xb9, 0x88, 0x20]);
    append_int(&mut b, t.day());
    b.push(0x20);
    b.extend_from_slice(at(ln.months_wide, t.month().0).as_bytes());
    b.push(0x20);

    if t.year() < 0 {
        b.extend_from_slice(ln.eras_wide[0].as_bytes());
    } else {
        b.extend_from_slice(ln.eras_wide[1].as_bytes());
    }

    b.push(0x20);

    append_abs_year(&mut b, t.year());

    string(b)
}

// ---------------------------------------------------------------------------
// Times

/// en: `h := t.Hour(); if h > 12 { h -= 12 }; AppendInt(b, h)`.
fn append_hour12(b: &mut Vec<u8>, t: &Time) {
    let mut h = t.hour();

    if h > 12 {
        h -= 12;
    }

    append_int(b, h);
}

/// `if x < 10 { b = append(b, '0') }; AppendInt(b, x)`.
fn append_2digits(b: &mut Vec<u8>, x: i64) {
    if x < 10 {
        b.push(b'0');
    }

    append_int(b, x);
}

/// en: `if t.Hour() < 12 { periodsAbbreviated[0] } else { periodsAbbreviated[1] }`.
fn append_period(b: &mut Vec<u8>, ln: &Localen, t: &Time) {
    if t.hour() < 12 {
        b.extend_from_slice(ln.periods_abbreviated[0].as_bytes());
    } else {
        b.extend_from_slice(ln.periods_abbreviated[1].as_bytes());
    }
}

/// th: " นาฬิกา " (`0x20, 0xe0, 0xb8, 0x99, ...`).
const TH_HOURS: &[u8] = &[
    0x20, 0xe0, 0xb8, 0x99, 0xe0, 0xb8, 0xb2, 0xe0, 0xb8, 0xac, 0xe0, 0xb8, 0xb4, 0xe0, 0xb8, 0x81,
    0xe0, 0xb8, 0xb2, 0x20,
];
/// th: " นาที ".
const TH_MINUTES: &[u8] = &[
    0x20, 0xe0, 0xb8, 0x99, 0xe0, 0xb8, 0xb2, 0xe0, 0xb8, 0x97, 0xe0, 0xb8, 0xb5, 0x20,
];
/// th: " วินาที ".
const TH_SECONDS: &[u8] = &[
    0x20, 0xe0, 0xb8, 0xa7, 0xe0, 0xb8, 0xb4, 0xe0, 0xb8, 0x99, 0xe0, 0xb8, 0xb2, 0xe0, 0xb8, 0x97,
    0xe0, 0xb8, 0xb5, 0x20,
];

// Go: localescompressed locales.autogen.go:fnFmtTimeShort_8288b817c5c0216c9815524c71c9e0b4 (en)
pub(super) fn fn_fmt_time_short_8288b817c5c0216c9815524c71c9e0b4(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_hour12(&mut b, t);
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.minute());
    b.push(0x20);
    append_period(&mut b, ln, t);

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtTimeShort_8ffba4d1a2f04a0a6486853d76cf434b (th)
pub(super) fn fn_fmt_time_short_8ffba4d1a2f04a0a6486853d76cf434b(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_2digits(&mut b, t.hour());
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.minute());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtTimeMedium_8ef07f6477b7a4944577d4e5392ff21b (en)
pub(super) fn fn_fmt_time_medium_8ef07f6477b7a4944577d4e5392ff21b(
    ln: &Localen,
    t: &Time,
) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_hour12(&mut b, t);
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.minute());
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.second());
    b.push(0x20);
    append_period(&mut b, ln, t);

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtTimeMedium_b6f697552cd4965cd1195fe56dc87003 (th)
pub(super) fn fn_fmt_time_medium_b6f697552cd4965cd1195fe56dc87003(
    ln: &Localen,
    t: &Time,
) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_2digits(&mut b, t.hour());
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.minute());
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.second());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtTimeLong_4429ef4fb58a2a9c4d7d5132fd16ecbf (en)
pub(super) fn fn_fmt_time_long_4429ef4fb58a2a9c4d7d5132fd16ecbf(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_hour12(&mut b, t);
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.minute());
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.second());
    b.push(0x20);
    append_period(&mut b, ln, t);
    b.push(0x20);

    let (tz, _) = t.zone();
    b.extend_from_slice(tz.as_bytes());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtTimeLong_9d4d0f53074023f50ec443e7d6cc145e (th)
pub(super) fn fn_fmt_time_long_9d4d0f53074023f50ec443e7d6cc145e(_ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_int(&mut b, t.hour());
    b.extend_from_slice(TH_HOURS);
    append_2digits(&mut b, t.minute());
    b.extend_from_slice(TH_MINUTES);
    append_2digits(&mut b, t.second());
    b.extend_from_slice(TH_SECONDS);

    let (tz, _) = t.zone();
    b.extend_from_slice(tz.as_bytes());

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtTimeFull_37fe4deeee789852cc196a6a1a2f0ccc (en)
pub(super) fn fn_fmt_time_full_37fe4deeee789852cc196a6a1a2f0ccc(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_hour12(&mut b, t);
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.minute());
    b.extend_from_slice(ln.time_separator.as_bytes());
    append_2digits(&mut b, t.second());
    b.push(0x20);
    append_period(&mut b, ln, t);
    b.push(0x20);

    let (tz, _) = t.zone();

    match ln.timezone(&tz) {
        Some(btz) => b.extend_from_slice(btz.as_bytes()),
        None => b.extend_from_slice(tz.as_bytes()),
    }

    string(b)
}

// Go: localescompressed locales.autogen.go:fnFmtTimeFull_54a1d4bd08c970be76504a2aa980af91 (th)
pub(super) fn fn_fmt_time_full_54a1d4bd08c970be76504a2aa980af91(ln: &Localen, t: &Time) -> String {
    let mut b: Vec<u8> = Vec::with_capacity(32);

    append_int(&mut b, t.hour());
    b.extend_from_slice(TH_HOURS);
    append_2digits(&mut b, t.minute());
    b.extend_from_slice(TH_MINUTES);
    append_2digits(&mut b, t.second());
    b.extend_from_slice(TH_SECONDS);

    let (tz, _) = t.zone();

    match ln.timezone(&tz) {
        Some(btz) => b.extend_from_slice(btz.as_bytes()),
        None => b.extend_from_slice(tz.as_bytes()),
    }

    string(b)
}
