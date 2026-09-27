//! Port of `langs/language.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


//! Go `langs.Language`. Created per enabled language (sorted by weight, then lang). Carries the
//! locale translator (dateFormat), time formatter, collators (x/text collate, CLDR 23 — use the
//! xtext-collate Wave A crate; do NOT use ICU4X `th`), location (UTC unless `timeZone`), params.

use std::any::Any;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use go_value::{GoString, HostCtx, Location, Map, Object, Value};
use nh_common::htime::TimeFormatter;
use nh_common::locales::Translator;
use nh_common::Result;

use crate::config::LanguageConfig;

/// Go: `langs.Collator` — a mutex-protected x/text collator (`CompareStrings(a, b) int`).
pub struct Collator {
    /// Wave B: `Mutex<xtext_collate::Collator>`.
    inner: Mutex<()>,
    pub tag: String,
}

impl Collator {
    /// Go: `Collator.CompareStrings(a, b)` -> -1/0/1.
    pub fn compare_strings(&self, a: &[u8], b: &[u8]) -> i32 {
        todo!("xtext_collate collator for self.tag (default options: tertiary, non-ignorable)")
    }
}

/// Go: `langs.Language`.
pub struct Language {
    /// The language code, e.g. "en" (lower-cased config key).
    pub lang: String,
    pub config: LanguageConfig,
    pub translator: Arc<dyn Translator>,
    pub time_formatter: TimeFormatter,
    /// Go `language.Tag` string used for collation.
    pub tag: String,
    pub collator1: Arc<Collator>,
    pub collator2: Arc<Collator>,
    pub location: Arc<Location>,
    /// Language params (set with `SetParams` from the per-language config).
    pub params: std::sync::RwLock<Arc<Map>>,
}

/// Go: `langs.Languages` — sorted (weight, then lang); a named slice type in templates.
pub type Languages = Vec<Arc<Language>>;

impl Language {
    /// Go: `langs.NewLanguage(lang, defaultContentLanguage, timeZone, languageConfig)`.
    // Go: langs/language.go:NewLanguage
    pub fn new(lang: &str, default_content_language: &str, time_zone: &str, cfg: LanguageConfig) -> Result<Arc<Language>> {
        todo!()
    }

    /// Go: `Language.LanguageCode()` — config LanguageCode, or Lang.
    // Go: langs/language.go:LanguageCode
    pub fn language_code(&self) -> &str {
        if self.config.language_code.is_empty() { &self.lang } else { &self.config.language_code }
    }

    // Go: langs/language.go:Params
    pub fn params(&self) -> Arc<Map> {
        self.params.read().unwrap().clone()
    }

    /// Go: `langs.SetParams(l, params)`.
    pub fn set_params(&self, params: Arc<Map>) {
        *self.params.write().unwrap() = params;
    }

    // Go: langs/language.go:Location
    pub fn location(&self) -> Arc<Location> {
        self.location.clone()
    }

    /// Go: `langs.GetTranslator(l)`, `GetTimeFormatter(l)`, `GetLocation(l)`, `GetCollator1(l)`, `GetCollator2(l)`.
    pub fn collator1(&self) -> &Arc<Collator> {
        &self.collator1
    }
}

/// Template API of `*langs.Language`: fields `Lang`, `LanguageName`, `LanguageCode` (field of the
/// embedded LanguageConfig; method `LanguageCode()` wins), `Title`, `LanguageDirection`, `Weight`,
/// `Disabled`; methods `LanguageCode`, `Params`, `String` (prints Lang), `IsDefault` (deprecated).
pub struct LanguageObject(pub Arc<Language>);

nh_common::go_methods!(LanguageObject {
    "LanguageCode" => |l, _ctx, _a| Ok(Value::string(l.0.language_code())),
    "Params" => |l, _ctx, _a| Ok(Value::Map(l.0.params())),
    "String" => |l, _ctx, _a| Ok(Value::string(l.0.lang.as_str())),
});

impl Object for LanguageObject {
    nh_common::object_basics!("*langs.Language");

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Lang" => Some(Value::string(self.0.lang.as_str())),
            "LanguageName" => Some(Value::string(self.0.config.language_name.as_str())),
            "Title" => Some(Value::string(self.0.config.title.as_str())),
            "LanguageDirection" => Some(Value::string(self.0.config.language_direction.as_str())),
            "Weight" => Some(Value::int(self.0.config.weight)),
            "Disabled" => Some(Value::Bool(self.0.config.disabled)),
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
        ls.iter().map(|l| Value::object(LanguageObject(l.clone()))).collect(),
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
// EX L55-93: NewLanguage(lang, defaultContentLanguage, timeZone string, languageConfig LanguageConfig) (*Language, error)
// EX L101-106: (l *Language) Params() maps.Params
// EX L108-113: (l *Language) LanguageCode() string
// EX L115-123: (l *Language) loadLocation(tzStr string) error
// EX L125-127: (l *Language) String() string
//    L132-139: (l Languages) AsSet() map[string]bool
// EX L142-149: (l Languages) AsIndexSet() map[string]int
// EX L154-156: SetParams(l *Language, params maps.Params)
// EX L158-160: GetTimeFormatter(l *Language) htime.TimeFormatter
// EX L162-164: GetTranslator(l *Language) locales.Translator
// EX L166-168: GetLocation(l *Language) *time.Location
// EX L170-172: GetCollator1(l *Language) *Collator
//    L174-176: GetCollator2(l *Language) *Collator
// EX L187-189: (c *Collator) CompareStrings(a, b string) int
// ---------------------------------------------------------------------------
