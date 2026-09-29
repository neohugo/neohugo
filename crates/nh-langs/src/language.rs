//! Port of `langs/language.go`.
//!
//! Owner: Wave B task T03 (parser-langs).

//! Go `langs.Language`. Created per configured language (sorted by weight, then lang, by the
//! caller). Carries the locale translator (dateFormat), time formatter, collators (x/text
//! collate, CLDR 23, through the xtext-collate Wave A crate; one pair per language tag), location
//! (`time.LoadLocation(timeZone)`: UTC for `""`), params.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use go_value::{GoString, Location, Map, Object, Time, Value};
use nh_common::Result;
use nh_common::htime::TimeFormatter;
use nh_common::locales::{PluralRule, Translator};

use crate::config::LanguageConfig;

/// Go: `langs.Collator` — a mutex-protected x/text collator (`CompareStrings(a, b) int`).
pub struct Collator {
    inner: Mutex<xtext_collate::Collator>,
    /// The tag the collator was created for (`language.Tag.String()`; `en` when the
    /// language key did not parse).
    pub tag: String,
}

impl Collator {
    fn new(c: xtext_collate::Collator, tag: String) -> Self {
        Collator {
            inner: Mutex::new(c),
            tag,
        }
    }

    /// Go: `Collator.CompareStrings(a, b)` -> -1/0/1. Go leaves the locking to the caller
    /// (`c.Lock()`); here each call locks (see [`Collator::lock`] to hold the lock for a sort).
    // Go: langs/language.go:CompareStrings
    pub fn compare_strings(&self, a: &[u8], b: &[u8]) -> i32 {
        self.lock().compare_strings(a, b)
    }

    /// Go: `c.Lock()`: holds the collator for a sequence of comparisons.
    pub fn lock(&self) -> CollatorGuard<'_> {
        CollatorGuard(self.inner.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

/// A locked [`Collator`].
pub struct CollatorGuard<'a>(MutexGuard<'a, xtext_collate::Collator>);

impl CollatorGuard<'_> {
    /// CompareStrings compares a and b.
    /// It returns -1 if a < b, 1 if a > b and 0 if a == b.
    // Go: langs/language.go:CompareStrings
    pub fn compare_strings(&mut self, a: &[u8], b: &[u8]) -> i32 {
        self.0.compare(a, b)
    }
}

/// Go: `langs.Language`.
pub struct Language {
    /// The language code, e.g. "en" (lower-cased config key).
    pub lang: String,
    /// Fields from the language config.
    pub config: LanguageConfig,
    pub translator: Arc<dyn Translator>,
    pub time_formatter: TimeFormatter,
    /// Go `language.Tag` string used for collation (what `language.Parse(lang)` returned, also
    /// when it failed).
    pub tag: String,
    /// collator1 and collator2 are the same, we have 2 to prevent deadlocks.
    pub collator1: Arc<Collator>,
    pub collator2: Arc<Collator>,
    pub location: Arc<Location>,
    /// Language params (set with `SetParams` from the per-language config). This is just an
    /// alias of Site.Params.
    pub params: std::sync::RwLock<Arc<Map>>,
}

/// Go: `langs.Languages` — sorted (weight, then lang); a named slice type in templates.
pub type Languages = Vec<Arc<Language>>;

impl Language {
    /// Go: `langs.NewLanguage(lang, defaultContentLanguage, timeZone, languageConfig)`.
    ///
    /// Go returns the language together with a time zone error; the port returns the error
    /// only (Hugo's caller discards the language on error).
    // Go: langs/language.go:NewLanguage
    pub fn new(
        lang: &str,
        default_content_language: &str,
        time_zone: &str,
        cfg: LanguageConfig,
    ) -> Result<Arc<Language>> {
        let translator = get_translator(lang)
            .or_else(|| get_translator(default_content_language))
            .or_else(|| get_translator("en"))
            .expect("the en translator exists");

        let tag;
        let coll1;
        let coll2;
        match xtext_collate::language::DEFAULT.parse(lang) {
            Ok(t) => {
                tag = t.string();
                coll1 = Collator::new(xtext_collate::Collator::from_tag(&t, &[]), t.string());
                coll2 = Collator::new(xtext_collate::Collator::from_tag(&t, &[]), t.string());
            }
            Err((t, _)) => {
                tag = t.string();
                let en = xtext_collate::language::english();
                coll1 = Collator::new(xtext_collate::Collator::from_tag(&en, &[]), en.string());
                coll2 = Collator::new(xtext_collate::Collator::from_tag(&en, &[]), en.string());
            }
        }

        let location = load_location(lang, time_zone)?;

        Ok(Arc::new(Language {
            lang: lang.to_string(),
            config: cfg,
            time_formatter: TimeFormatter::new(translator.clone()),
            translator,
            tag,
            collator1: Arc::new(coll1),
            collator2: Arc::new(coll2),
            location,
            params: std::sync::RwLock::new(Arc::new(Map::new(go_value::MapType::Params))),
        }))
    }

    /// Go: `Language.LanguageCode()` — config LanguageCode, or Lang.
    // Go: langs/language.go:LanguageCode
    pub fn language_code(&self) -> &str {
        if !self.config.language_code.is_empty() {
            return &self.config.language_code;
        }
        &self.lang
    }

    /// Params returns the language params.
    /// Note that this is the same as the Site.Params, but we keep it here for legacy reasons.
    // Go: langs/language.go:Params
    pub fn params(&self) -> Arc<Map> {
        self.params
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Go: `langs.SetParams(l, params)`.
    // Go: langs/language.go:SetParams
    pub fn set_params(&self, params: Arc<Map>) {
        *self.params.write().unwrap_or_else(|e| e.into_inner()) = params;
    }

    /// Go: `langs.GetLocation(l)`.
    // Go: langs/language.go:GetLocation
    pub fn location(&self) -> Arc<Location> {
        self.location.clone()
    }

    // Go: langs/language.go:String
    pub fn string(&self) -> &str {
        &self.lang
    }

    /// Go: `langs.GetCollator1(l)`.
    // Go: langs/language.go:GetCollator1
    pub fn collator1(&self) -> &Arc<Collator> {
        &self.collator1
    }

    /// Go: `langs.GetCollator2(l)`.
    // Go: langs/language.go:GetCollator2
    pub fn collator2(&self) -> &Arc<Collator> {
        &self.collator2
    }

    /// Go: `langs.GetTranslator(l)`.
    // Go: langs/language.go:GetTranslator
    pub fn translator(&self) -> &Arc<dyn Translator> {
        &self.translator
    }

    /// Go: `langs.GetTimeFormatter(l)`.
    // Go: langs/language.go:GetTimeFormatter
    pub fn time_formatter(&self) -> &TimeFormatter {
        &self.time_formatter
    }
}

// Go: langs/language.go:loadLocation
fn load_location(lang: &str, tz_str: &str) -> Result<Arc<Location>> {
    match go_time::load_location(tz_str) {
        Ok(location) => Ok(location),
        Err(err) => Err(nh_common::Error::new(format!(
            "invalid timeZone for language {}: {}",
            go_strconv::quote(lang),
            err
        ))),
    }
}

/// Go: `localescompressed.GetTranslator(locale)`: `None` when Go returns nil. A locale Go has
/// but nh-common has no tables for gets a [`UnportedTranslator`] (Go would return its
/// translator), which fails loudly when it is used.
fn get_translator(locale: &str) -> Option<Arc<dyn Translator>> {
    match nh_common::locales::get_translator(locale) {
        Ok(t) => t,
        Err(e) => Some(Arc::new(UnportedTranslator {
            locale: canonical_locale_name(locale),
            message: e.message().to_string(),
        })),
    }
}

/// The `Locale()` name of a gohugoio/locales translator (`zh_Hant_TW`, `en_US`, `es_419`,
/// `ca_ES_VALENCIA`): Go's lookup key (`-` → `_`, lower case) with CLDR casing restored — the
/// language subtag in lower case, a four-letter script in title case, other letter subtags in
/// upper case.
fn canonical_locale_name(locale: &str) -> String {
    let key = String::from_utf8_lossy(&go_unicode::strings::to_lower(
        &go_unicode::strings::replace_all(locale.as_bytes(), b"-", b"_"),
    ))
    .into_owned();
    key.split('_')
        .enumerate()
        .map(|(i, p)| {
            if i == 0 || !p.bytes().all(|b| b.is_ascii_alphabetic()) {
                p.to_string()
            } else if p.len() == 4 {
                let mut s = p[..1].to_ascii_uppercase();
                s.push_str(&p[1..]);
                s
            } else {
                p.to_ascii_uppercase()
            }
        })
        .collect::<Vec<_>>()
        .join("_")
}

/// Stands in for a `gohugoio/locales` translator that nh-common does not port (seeksnack's
/// disabled languages: fr, pl, pt, de, es, zh-cn, zh-tw, ja, nl). Go creates a `Language` for
/// every configured language, disabled or not, but only the enabled ones (en, th) ever format
/// dates. Every method panics with the explicit `neohugo-rs: … is not supported` message.
pub struct UnportedTranslator {
    locale: String,
    message: String,
}

impl UnportedTranslator {
    fn fail(&self) -> ! {
        panic!("{}", self.message)
    }
}

impl Translator for UnportedTranslator {
    fn locale(&self) -> &str {
        &self.locale
    }
    fn plurals_cardinal(&self) -> &[PluralRule] {
        self.fail()
    }
    fn plurals_ordinal(&self) -> &[PluralRule] {
        self.fail()
    }
    fn plurals_range(&self) -> &[PluralRule] {
        self.fail()
    }
    fn cardinal_plural_rule(&self, _num: f64, _v: u64) -> PluralRule {
        self.fail()
    }
    fn ordinal_plural_rule(&self, _num: f64, _v: u64) -> PluralRule {
        self.fail()
    }
    fn range_plural_rule(&self, _num1: f64, _v1: u64, _num2: f64, _v2: u64) -> PluralRule {
        self.fail()
    }
    fn month_abbreviated(&self, _m: u32) -> &str {
        self.fail()
    }
    fn months_abbreviated(&self) -> &[&str] {
        self.fail()
    }
    fn month_narrow(&self, _m: u32) -> &str {
        self.fail()
    }
    fn months_narrow(&self) -> &[&str] {
        self.fail()
    }
    fn month_wide(&self, _m: u32) -> &str {
        self.fail()
    }
    fn months_wide(&self) -> &[&str] {
        self.fail()
    }
    fn weekday_abbreviated(&self, _d: u32) -> &str {
        self.fail()
    }
    fn weekdays_abbreviated(&self) -> &[&str] {
        self.fail()
    }
    fn weekday_narrow(&self, _d: u32) -> &str {
        self.fail()
    }
    fn weekdays_narrow(&self) -> &[&str] {
        self.fail()
    }
    fn weekday_short(&self, _d: u32) -> &str {
        self.fail()
    }
    fn weekdays_short(&self) -> &[&str] {
        self.fail()
    }
    fn weekday_wide(&self, _d: u32) -> &str {
        self.fail()
    }
    fn weekdays_wide(&self) -> &[&str] {
        self.fail()
    }
    fn fmt_date_short(&self, _t: &Time) -> String {
        self.fail()
    }
    fn fmt_date_medium(&self, _t: &Time) -> String {
        self.fail()
    }
    fn fmt_date_long(&self, _t: &Time) -> String {
        self.fail()
    }
    fn fmt_date_full(&self, _t: &Time) -> String {
        self.fail()
    }
    fn fmt_time_short(&self, _t: &Time) -> String {
        self.fail()
    }
    fn fmt_time_medium(&self, _t: &Time) -> String {
        self.fail()
    }
    fn fmt_time_long(&self, _t: &Time) -> String {
        self.fail()
    }
    fn fmt_time_full(&self, _t: &Time) -> String {
        self.fail()
    }
    fn fmt_percent(&self, _n: f64, _v: u64) -> String {
        self.fail()
    }
    fn try_fmt_number(&self, _n: f64, _v: u64) -> Result<String> {
        Err(nh_common::Error::new(self.message.clone()))
    }
    fn try_fmt_currency(&self, _n: f64, _v: u64, _currency: &str) -> Result<String> {
        Err(nh_common::Error::new(self.message.clone()))
    }
    fn try_fmt_accounting(&self, _n: f64, _v: u64, _currency: &str) -> Result<String> {
        Err(nh_common::Error::new(self.message.clone()))
    }
}

/// Go: `Languages.AsSet()`.
// Go: langs/language.go:AsSet
pub fn as_set(l: &Languages) -> BTreeMap<String, bool> {
    let mut m = BTreeMap::new();
    for lang in l {
        m.insert(lang.lang.clone(), true);
    }

    m
}

/// AsIndexSet returns a map with the language code as key and index in l as value.
// Go: langs/language.go:AsIndexSet
pub fn as_index_set(l: &Languages) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for (i, lang) in l.iter().enumerate() {
        m.insert(lang.lang.clone(), i);
    }

    m
}

/// Template API of `*langs.Language`: fields `Lang`, `LanguageName`, `LanguageCode` (field of the
/// embedded LanguageConfig; method `LanguageCode()` wins), `Title`, `LanguageDirection`, `Weight`,
/// `Disabled`; methods `LanguageCode`, `Params`, `String` (prints Lang).
pub struct LanguageObject(pub Arc<Language>);

nh_common::go_methods!(LanguageObject {
    "LanguageCode" => |l, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "LanguageCode")?;
        Ok(Value::string(l.0.language_code()))
    },
    "Params" => |l, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "Params")?;
        Ok(Value::Map(l.0.params()))
    },
    "String" => |l, _ctx, a| {
        nh_common::object::args::exactly(a, 0, "String")?;
        Ok(Value::string(l.0.lang.as_str()))
    },
});

impl Object for LanguageObject {
    nh_common::object_basics!("*langs.Language");

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Lang" => Some(Value::string(self.0.lang.as_str())),
            "LanguageName" => Some(Value::string(self.0.config.language_name.as_str())),
            "LanguageCode" => Some(Value::string(self.0.config.language_code.as_str())),
            "Title" => Some(Value::string(self.0.config.title.as_str())),
            "LanguageDirection" => Some(Value::string(self.0.config.language_direction.as_str())),
            "Weight" => Some(Value::int(self.0.config.weight)),
            "Disabled" => Some(Value::Bool(self.0.config.disabled)),
            // The embedded struct itself (its fields are promoted above).
            "LanguageConfig" => Some(Value::object(self.0.config.clone())),
            _ => None,
        }
    }

    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.lang.as_str()))
    }

    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as *const () as usize
    }
}

/// `langs.Languages` as a template value (`SliceType::Named("langs.Languages")` of `LanguageObject`s).
pub fn languages_to_value(ls: &Languages) -> Value {
    Value::list(
        go_value::SliceType::Named(Arc::from("langs.Languages")),
        ls.iter()
            .map(|l| Value::object(LanguageObject(l.clone())))
            .collect(),
    )
}

/// Downcast a template value to a language.
pub fn language_from_value(v: &Value) -> Option<Arc<Language>> {
    v.downcast::<LanguageObject>().map(|l| l.0.clone())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: langs/language.go (189 lines; 12/14 funcs executed)
//   types: Language, Languages, Collator
// OK L55-93: NewLanguage(lang, defaultContentLanguage, timeZone string, languageConfig LanguageConfig) (*Language, error)
// OK L101-106: (l *Language) Params() maps.Params
// OK L108-113: (l *Language) LanguageCode() string
// OK L115-123: (l *Language) loadLocation(tzStr string) error
// OK L125-127: (l *Language) String() string
// OK L132-139: (l Languages) AsSet() map[string]bool
// OK L142-149: (l Languages) AsIndexSet() map[string]int
// OK L154-156: SetParams(l *Language, params maps.Params)
// OK L158-160: GetTimeFormatter(l *Language) htime.TimeFormatter
// OK L162-164: GetTranslator(l *Language) locales.Translator
// OK L166-168: GetLocation(l *Language) *time.Location
// OK L170-172: GetCollator1(l *Language) *Collator
// OK L174-176: GetCollator2(l *Language) *Collator
// OK L187-189: (c *Collator) CompareStrings(a, b string) int
// ---------------------------------------------------------------------------
