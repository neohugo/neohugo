//! Port of `github.com/gohugoio/localescompressed@v1.0.1/localen.go` and the `localen` struct of
//! `locales.autogen.go`: one translator type whose data and functions are filled in per locale by
//! the generated `tables.rs`.

use go_value::Time;

use super::{PluralRule, Translator, get_currency};
use crate::herrors::Result;

/// `func(ln *localen, t time.Time) string`.
pub(super) type FnTime = fn(&Localen, &Time) -> String;
/// `func(ln *localen, num float64, v uint64) string` (Go runtime panics as errors).
pub(super) type FnNum = fn(&Localen, f64, u64) -> Result<String>;
/// `func(ln *localen, num float64, v uint64, currency currency.Type) string` (Go runtime panics as
/// errors).
pub(super) type FnCurrency = fn(&Localen, f64, u64, i64) -> Result<String>;
/// `func(ln *localen, num float64, v uint64) locales.PluralRule`.
pub(super) type FnPlural = fn(&Localen, f64, u64) -> PluralRule;
/// `func(ln *localen, month time.Month) string` / `func(ln *localen, weekday time.Weekday) string`.
pub(super) type FnIndex = fn(&Localen, i64) -> &'static str;
/// `func(ln *localen) []string`.
pub(super) type FnList = fn(&Localen) -> &'static [&'static str];
/// `func(ln *localen) []locales.PluralRule`.
pub(super) type FnPlurals = fn(&Localen) -> &'static [PluralRule];
/// `func(ln *localen) string`.
pub(super) type FnStr = fn(&Localen) -> &'static str;
/// `func(ln *localen, num1 float64, v1 uint64, num2 float64, v2 uint64) locales.PluralRule`.
pub(super) type FnRange = fn(&Localen, f64, u64, f64, u64) -> PluralRule;

// Go: localescompressed locales.autogen.go:localen
/// Go's `localen` struct (every field, in Go's order; filled in by `tables.rs`).
// Mirrors Go's struct: some fields are read by no function en or th uses (and `fnDecimal`,
// `fnGroup`, `fnMinus`, `fnMonths*`, `fnWeekdays*` by no `localen` method at all).
#[allow(dead_code)]
pub(super) struct Localen {
    pub(super) currencies: &'static [&'static str],
    pub(super) currency_negative_prefix: &'static str,
    pub(super) currency_negative_suffix: &'static str,
    pub(super) currency_positive_prefix: &'static str,
    pub(super) currency_positive_suffix: &'static str,
    pub(super) days_abbreviated: &'static [&'static str],
    pub(super) days_narrow: &'static [&'static str],
    pub(super) days_short: &'static [&'static str],
    pub(super) days_wide: &'static [&'static str],
    pub(super) decimal: &'static str,
    pub(super) eras_abbreviated: &'static [&'static str],
    pub(super) eras_narrow: &'static [&'static str],
    pub(super) eras_wide: &'static [&'static str],
    pub(super) fn_cardinal_plural_rule: FnPlural,
    pub(super) fn_decimal: FnStr,
    pub(super) fn_fmt_accounting: FnCurrency,
    pub(super) fn_fmt_currency: FnCurrency,
    pub(super) fn_fmt_date_full: FnTime,
    pub(super) fn_fmt_date_long: FnTime,
    pub(super) fn_fmt_date_medium: FnTime,
    pub(super) fn_fmt_date_short: FnTime,
    pub(super) fn_fmt_number: FnNum,
    pub(super) fn_fmt_percent: fn(&Localen, f64, u64) -> String,
    pub(super) fn_fmt_time_full: FnTime,
    pub(super) fn_fmt_time_long: FnTime,
    pub(super) fn_fmt_time_medium: FnTime,
    pub(super) fn_fmt_time_short: FnTime,
    pub(super) fn_group: FnStr,
    pub(super) fn_locale: FnStr,
    pub(super) fn_minus: FnStr,
    pub(super) fn_month_abbreviated: FnIndex,
    pub(super) fn_month_narrow: FnIndex,
    pub(super) fn_month_wide: FnIndex,
    pub(super) fn_months_abbreviated: FnList,
    pub(super) fn_months_narrow: FnList,
    pub(super) fn_months_wide: FnList,
    pub(super) fn_ordinal_plural_rule: FnPlural,
    pub(super) fn_plurals_cardinal: FnPlurals,
    pub(super) fn_plurals_ordinal: FnPlurals,
    pub(super) fn_plurals_range: FnPlurals,
    pub(super) fn_range_plural_rule: FnRange,
    pub(super) fn_weekday_abbreviated: FnIndex,
    pub(super) fn_weekday_narrow: FnIndex,
    pub(super) fn_weekday_short: FnIndex,
    pub(super) fn_weekday_wide: FnIndex,
    pub(super) fn_weekdays_abbreviated: FnList,
    pub(super) fn_weekdays_narrow: FnList,
    pub(super) fn_weekdays_short: FnList,
    pub(super) fn_weekdays_wide: FnList,
    pub(super) group: &'static str,
    pub(super) inifinity: &'static str,
    pub(super) locale: &'static str,
    pub(super) minus: &'static str,
    pub(super) months_abbreviated: &'static [&'static str],
    pub(super) months_narrow: &'static [&'static str],
    pub(super) months_wide: &'static [&'static str],
    pub(super) per_mille: &'static str,
    pub(super) percent: &'static str,
    pub(super) percent_prefix: &'static str,
    pub(super) percent_suffix: &'static str,
    pub(super) periods_abbreviated: &'static [&'static str],
    pub(super) periods_narrow: &'static [&'static str],
    pub(super) periods_short: &'static [&'static str],
    pub(super) periods_wide: &'static [&'static str],
    pub(super) plurals_cardinal: &'static [PluralRule],
    pub(super) plurals_ordinal: &'static [PluralRule],
    pub(super) plurals_range: &'static [PluralRule],
    pub(super) time_separator: &'static str,
    /// Go's `timezones` map, as (abbreviation, name) pairs sorted by abbreviation.
    pub(super) timezones: &'static [(&'static str, &'static str)],
}

impl Localen {
    /// Go: `ln.timezones[tz]`.
    pub(super) fn timezone(&self, tz: &str) -> Option<&'static str> {
        self.timezones
            .binary_search_by(|(k, _)| k.as_bytes().cmp(tz.as_bytes()))
            .ok()
            .map(|i| self.timezones[i].1)
    }
}

/// Go `s[i]` on a `[]string` with a Go `int` index (a panic when out of range, as in Go).
pub(super) fn at(s: &'static [&'static str], i: i64) -> &'static str {
    s[usize::try_from(i).unwrap_or(usize::MAX)]
}

impl Translator for Localen {
    // Go: localescompressed localen.go:Locale
    fn locale(&self) -> &str {
        (self.fn_locale)(self)
    }

    // Go: localescompressed localen.go:PluralsCardinal
    fn plurals_cardinal(&self) -> &[PluralRule] {
        (self.fn_plurals_cardinal)(self)
    }

    // Go: localescompressed localen.go:PluralsOrdinal
    fn plurals_ordinal(&self) -> &[PluralRule] {
        (self.fn_plurals_ordinal)(self)
    }

    // Go: localescompressed localen.go:PluralsRange
    fn plurals_range(&self) -> &[PluralRule] {
        self.plurals_range
    }

    // Go: localescompressed localen.go:CardinalPluralRule
    fn cardinal_plural_rule(&self, num: f64, v: u64) -> PluralRule {
        (self.fn_cardinal_plural_rule)(self, num, v)
    }

    // Go: localescompressed localen.go:OrdinalPluralRule
    fn ordinal_plural_rule(&self, num: f64, v: u64) -> PluralRule {
        (self.fn_ordinal_plural_rule)(self, num, v)
    }

    // Go: localescompressed localen.go:RangePluralRule
    fn range_plural_rule(&self, num1: f64, v1: u64, num2: f64, v2: u64) -> PluralRule {
        (self.fn_range_plural_rule)(self, num1, v1, num2, v2)
    }

    // Go: localescompressed localen.go:MonthAbbreviated
    fn month_abbreviated(&self, m: u32) -> &str {
        (self.fn_month_abbreviated)(self, m as i64)
    }

    // Go: localescompressed localen.go:MonthsAbbreviated
    fn months_abbreviated(&self) -> &[&str] {
        self.months_abbreviated
    }

    // Go: localescompressed localen.go:MonthNarrow
    fn month_narrow(&self, m: u32) -> &str {
        at(self.months_narrow, m as i64)
    }

    // Go: localescompressed localen.go:MonthsNarrow
    fn months_narrow(&self) -> &[&str] {
        &self.months_narrow[1..]
    }

    // Go: localescompressed localen.go:MonthWide
    fn month_wide(&self, m: u32) -> &str {
        at(self.months_wide, m as i64)
    }

    // Go: localescompressed localen.go:MonthsWide
    fn months_wide(&self) -> &[&str] {
        &self.months_wide[1..]
    }

    // Go: localescompressed localen.go:WeekdayAbbreviated
    fn weekday_abbreviated(&self, d: u32) -> &str {
        (self.fn_weekday_abbreviated)(self, d as i64)
    }

    // Go: localescompressed localen.go:WeekdaysAbbreviated
    fn weekdays_abbreviated(&self) -> &[&str] {
        self.days_abbreviated
    }

    // Go: localescompressed localen.go:WeekdayNarrow
    fn weekday_narrow(&self, d: u32) -> &str {
        at(self.days_narrow, d as i64)
    }

    // Go: localescompressed localen.go:WeekdaysNarrow
    fn weekdays_narrow(&self) -> &[&str] {
        self.days_narrow
    }

    // Go: localescompressed localen.go:WeekdayShort
    fn weekday_short(&self, d: u32) -> &str {
        at(self.days_short, d as i64)
    }

    // Go: localescompressed localen.go:WeekdaysShort
    fn weekdays_short(&self) -> &[&str] {
        self.days_short
    }

    // Go: localescompressed localen.go:WeekdayWide
    fn weekday_wide(&self, d: u32) -> &str {
        at(self.days_wide, d as i64)
    }

    // Go: localescompressed localen.go:WeekdaysWide
    fn weekdays_wide(&self) -> &[&str] {
        self.days_wide
    }

    // Go: localescompressed localen.go:FmtNumber
    fn try_fmt_number(&self, n: f64, v: u64) -> Result<String> {
        (self.fn_fmt_number)(self, n, v)
    }

    // Go: localescompressed localen.go:FmtPercent
    fn fmt_percent(&self, n: f64, v: u64) -> String {
        (self.fn_fmt_percent)(self, n, v)
    }

    // Go: localescompressed localen.go:FmtCurrency
    fn try_fmt_currency(&self, n: f64, v: u64, currency: &str) -> Result<String> {
        (self.fn_fmt_currency)(self, n, v, get_currency(currency))
    }

    // Go: localescompressed localen.go:FmtAccounting
    fn try_fmt_accounting(&self, n: f64, v: u64, currency: &str) -> Result<String> {
        (self.fn_fmt_accounting)(self, n, v, get_currency(currency))
    }

    // Go: localescompressed localen.go:FmtDateShort
    fn fmt_date_short(&self, t: &Time) -> String {
        (self.fn_fmt_date_short)(self, t)
    }

    // Go: localescompressed localen.go:FmtDateMedium
    fn fmt_date_medium(&self, t: &Time) -> String {
        (self.fn_fmt_date_medium)(self, t)
    }

    // Go: localescompressed localen.go:FmtDateLong
    fn fmt_date_long(&self, t: &Time) -> String {
        (self.fn_fmt_date_long)(self, t)
    }

    // Go: localescompressed localen.go:FmtDateFull
    fn fmt_date_full(&self, t: &Time) -> String {
        (self.fn_fmt_date_full)(self, t)
    }

    // Go: localescompressed localen.go:FmtTimeShort
    fn fmt_time_short(&self, t: &Time) -> String {
        (self.fn_fmt_time_short)(self, t)
    }

    // Go: localescompressed localen.go:FmtTimeMedium
    fn fmt_time_medium(&self, t: &Time) -> String {
        (self.fn_fmt_time_medium)(self, t)
    }

    // Go: localescompressed localen.go:FmtTimeLong
    fn fmt_time_long(&self, t: &Time) -> String {
        (self.fn_fmt_time_long)(self, t)
    }

    // Go: localescompressed localen.go:FmtTimeFull
    fn fmt_time_full(&self, t: &Time) -> String {
        (self.fn_fmt_time_full)(self, t)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (third-party: localescompressed v1.0.1 localen.go; written by T26).
// OK L13-15: (ln *localen) Locale() string
// OK L19-21: (ln *localen) PluralsCardinal() []locales.PluralRule
// OK L25-27: (ln *localen) PluralsOrdinal() []locales.PluralRule
// OK L31-33: (ln *localen) PluralsRange() []locales.PluralRule
// OK L36-38: (ln *localen) CardinalPluralRule(num float64, v uint64) locales.PluralRule
// OK L41-43: (ln *localen) OrdinalPluralRule(num float64, v uint64) locales.PluralRule
// OK L46-48: (ln *localen) RangePluralRule(num1 float64, v1 uint64, num2 float64, v2 uint64) locales.PluralRule
// OK L51-53: (ln *localen) MonthAbbreviated(month time.Month) string
// OK L56-58: (ln *localen) MonthsAbbreviated() []string
// OK L61-63: (ln *localen) MonthNarrow(month time.Month) string
// OK L66-68: (ln *localen) MonthsNarrow() []string
// OK L71-73: (ln *localen) MonthWide(month time.Month) string
// OK L76-78: (ln *localen) MonthsWide() []string
// OK L81-83: (ln *localen) WeekdayAbbreviated(weekday time.Weekday) string
// OK L86-88: (ln *localen) WeekdaysAbbreviated() []string
// OK L91-93: (ln *localen) WeekdayNarrow(weekday time.Weekday) string
// OK L96-98: (ln *localen) WeekdaysNarrow() []string
// OK L101-103: (ln *localen) WeekdayShort(weekday time.Weekday) string
// OK L106-108: (ln *localen) WeekdaysShort() []string
// OK L111-113: (ln *localen) WeekdayWide(weekday time.Weekday) string
// OK L116-118: (ln *localen) WeekdaysWide() []string
// OK L122-124: (ln *localen) FmtNumber(num float64, v uint64) string
// OK L128-130: (ln *localen) FmtPercent(num float64, v uint64) string
// OK L133-135: (ln *localen) FmtCurrency(num float64, v uint64, currency currency.Type) string
// OK L139-141: (ln *localen) FmtAccounting(num float64, v uint64, currency currency.Type) string
// OK L144-146: (ln *localen) FmtDateShort(t time.Time) string
// OK L149-151: (ln *localen) FmtDateMedium(t time.Time) string
// OK L154-156: (ln *localen) FmtDateLong(t time.Time) string
// OK L159-161: (ln *localen) FmtDateFull(t time.Time) string
// OK L164-166: (ln *localen) FmtTimeShort(t time.Time) string
// OK L169-171: (ln *localen) FmtTimeMedium(t time.Time) string
// OK L174-176: (ln *localen) FmtTimeLong(t time.Time) string
// OK L179-181: (ln *localen) FmtTimeFull(t time.Time) string
// ---------------------------------------------------------------------------
