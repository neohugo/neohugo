//! Collation (string order for sorting) with ICU4X.

use std::cmp::Ordering;
use std::fmt;

use icu_collator::options::CollatorOptions;
use icu_collator::{CollatorBorrowed, CollatorPreferences};
use neohugo_base::Collate;

use crate::tag;

/// Languages collated with the root order instead of their CLDR tailoring.
///
/// CLDR 24 gave Thai `[reorder Thai]` and `alternate=shifted`: Thai sorts before Latin and
/// spaces and punctuation are ignored. Hugo (x/text, CLDR 23) sorts a Thai site's mixed lists
/// Latin first, and the root order reproduces its order for every string of the reference
/// sites (docs/rust-port/specs/i18n-lang-misc.md §4).
const ROOT_ORDER: &[&str] = &["th"];

/// The collation order of one language: tertiary strength, non-ignorable punctuation (the
/// CLDR defaults).
pub struct Collator(CollatorBorrowed<'static>);

impl Collator {
    /// The collator of the language `key` (root order for unknown keys and for
    /// [`ROOT_ORDER`] languages).
    #[must_use]
    pub fn for_language(key: &str) -> Self {
        let locale = tag::icu_locale(key)
            .filter(|l| !ROOT_ORDER.contains(&l.id.language.as_str()))
            .unwrap_or(icu_locale_core::Locale::UNKNOWN);
        let prefs = CollatorPreferences::from(&locale);
        let collator = CollatorBorrowed::try_new(prefs, CollatorOptions::default())
            .expect("compiled data has a collation for every locale (root included)");
        Self(collator)
    }
}

impl Collate for Collator {
    fn compare(&self, a: &str, b: &str) -> Ordering {
        self.0.compare(a, b)
    }
}

impl fmt::Debug for Collator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Collator").finish_non_exhaustive()
    }
}
