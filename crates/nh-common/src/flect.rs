//! Module `flect`.
//!
//! PORT gobuffalo/flect@v1.0.3: ident.go, humanize.go, titleize.go, ordinalize.go, pluralize.go, plural_rules.go, acronyms.go, custom_data.go, flect.go, rule.go, capitalize.go
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).


//! Port of `github.com/gobuffalo/flect@v1.0.3` (the parts Hugo uses): `Pluralize` (section titles),
//! `Humanize` (template `humanize`, drops Thai combining marks!), `Ordinalize`, `Titleize`,
//! `Capitalize`. Includes the rule/dictionary tables (plural_rules.go, acronyms.go) and the
//! custom-data loading from `inflections.json`/`acronyms.json` in the working dir.

/// Go: `flect.Pluralize(s)`.
pub fn pluralize(s: &str) -> String {
    todo!()
}

/// Go: `flect.Singularize(s)`.
pub fn singularize(s: &str) -> String {
    todo!()
}

/// Go: `flect.Humanize(s)`.
pub fn humanize(s: &str) -> String {
    todo!()
}

/// Go: `flect.Ordinalize(s)`.
pub fn ordinalize(s: &str) -> String {
    todo!()
}

/// Go: `flect.Titleize(s)`.
pub fn titleize(s: &str) -> String {
    todo!()
}

/// Go: `flect.Capitalize(s)`.
pub fn capitalize(s: &str) -> String {
    todo!()
}
