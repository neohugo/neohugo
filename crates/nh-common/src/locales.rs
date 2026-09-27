//! Module `locales`.
//!
//! PORT gohugoio/locales + gohugoio/localescompressed subset: Translator trait + generated en/th tables
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).


//! Subset of `github.com/gohugoio/locales` (+ `localescompressed`): the `Translator` API used by
//! `htime.TimeFormatter` and `lang.FormatNumber*`, with generated data for the languages in use.
//! Generate the tables from the Go module (`localescompressed/locales.autogen.go`) with a small Go
//! program; do not hand-write them. Needed now: en, th (plus the fallback rules of GetTranslator).

use std::sync::Arc;

use go_value::Time;

/// Go: `locales.Translator` (subset).
pub trait Translator: Send + Sync {
    fn locale(&self) -> &str;
    /// `m` is 1..=12.
    fn month_abbreviated(&self, m: u32) -> &str;
    fn month_wide(&self, m: u32) -> &str;
    /// `d` is 0 (Sunday)..=6.
    fn weekday_abbreviated(&self, d: u32) -> &str;
    fn weekday_wide(&self, d: u32) -> &str;
    fn fmt_date_short(&self, t: &Time) -> String;
    fn fmt_date_medium(&self, t: &Time) -> String;
    fn fmt_date_long(&self, t: &Time) -> String;
    fn fmt_date_full(&self, t: &Time) -> String;
    fn fmt_time_short(&self, t: &Time) -> String;
    fn fmt_time_medium(&self, t: &Time) -> String;
    fn fmt_time_long(&self, t: &Time) -> String;
    fn fmt_time_full(&self, t: &Time) -> String;
    fn fmt_number(&self, n: f64, v: u64) -> String;
    fn fmt_percent(&self, n: f64, v: u64) -> String;
    fn fmt_currency(&self, n: f64, v: u64, currency: &str) -> String;
    fn fmt_accounting(&self, n: f64, v: u64, currency: &str) -> String;
}

/// Go: `localescompressed.GetTranslator(locale)`; `None` if the locale is unknown (callers fall back
/// to the default content language, then "en").
pub fn get_translator(locale: &str) -> Option<Arc<dyn Translator>> {
    todo!()
}
