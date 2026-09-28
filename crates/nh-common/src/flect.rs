//! Module `flect`.
//!
//! PORT gobuffalo/flect@v1.0.3: ident.go, humanize.go, titleize.go, ordinalize.go, pluralize.go, plural_rules.go, acronyms.go, custom_data.go, flect.go, rule.go, capitalize.go
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).

//! Port of `github.com/gobuffalo/flect@v1.0.3` (the parts Hugo uses): `Pluralize` (section titles),
//! `Humanize` (template `humanize`, drops Thai combining marks!), `Ordinalize`, `Titleize`,
//! `Capitalize`. Includes the rule/dictionary tables (plural_rules.go, acronyms.go) and the
//! custom-data loading from `inflections.json`/`acronyms.json` in the working dir.
//!
//! Upstream: `github.com/gobuffalo/flect v1.0.3` (acronyms.go, capitalize.go, custom_data.go,
//! flect.go, humanize.go, ident.go, ordinalize.go, plural_rules.go, pluralize.go, rule.go,
//! singular_rules.go, singularize.go, titleize.go).
//!
//! Go strings are bytes: every function has a byte-level form (`*_bytes`, over `&[u8]`, invalid
//! UTF-8 included); the `&str` forms are conveniences over them.
//!
//! Package state. Go builds its tables in `init()` functions, in file order: `custom_data.go`
//! (loads `inflections.json` then `acronyms.json` from `$INFLECT_PATH`/`$ACRONYMS_PATH` or the
//! working directory), then the two `init`s of `plural_rules.go` (the dictionary maps, then the
//! suffix rules). Here the same steps run, in the same order, on first use (see `data()`); the
//! working directory is the one at first use rather than at process start (neohugo never changes
//! it). A custom `inflections.json` that repeats a dictionary word makes Go panic in `init`
//! ("map singleToPlural already has an entry for ..."); that panic is reproduced.
//!
//! Go panics reproduced: `Humanize` of a string that is not blank but keeps no characters (for
//! example `"+"` or a lone Thai combining mark) indexes `Parts[0]` of an empty slice.
//! [`try_humanize`] returns that panic as an error (what text/template's `safeCall` reports);
//! [`humanize`] panics like Go.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock, RwLockReadGuard};

use go_unicode::{Rune, strings, utf8};

use crate::herrors::{Error, Result};

// ---------------------------------------------------------------------------
// Public API (package-level functions)

/// Go: `flect.Pluralize(s)`.
pub fn pluralize(s: &str) -> String {
    bytes_to_string(pluralize_bytes(s.as_bytes()))
}

// Go: flect pluralize.go:Pluralize
/// Go: `flect.Pluralize(s)` over Go string bytes.
pub fn pluralize_bytes(s: &[u8]) -> Vec<u8> {
    let d = data();
    Ident::new_with(&d, s).pluralize_with(&d).original
}

// Go: flect pluralize.go:PluralizeWithSize
/// Go: `flect.PluralizeWithSize(s, i)`.
pub fn pluralize_with_size(s: &str, i: i64) -> String {
    let d = data();
    let id = Ident::new_with(&d, s.as_bytes());
    let out = if i == 1 || i == -1 {
        id.singularize_with(&d)
    } else {
        id.pluralize_with(&d)
    };
    bytes_to_string(out.original)
}

/// Go: `flect.Singularize(s)`.
pub fn singularize(s: &str) -> String {
    bytes_to_string(singularize_bytes(s.as_bytes()))
}

// Go: flect singularize.go:Singularize
/// Go: `flect.Singularize(s)` over Go string bytes.
pub fn singularize_bytes(s: &[u8]) -> Vec<u8> {
    let d = data();
    Ident::new_with(&d, s).singularize_with(&d).original
}

// Go: flect singularize.go:SingularizeWithSize
/// Go: `flect.SingularizeWithSize(s, i)` (= `PluralizeWithSize`).
pub fn singularize_with_size(s: &str, i: i64) -> String {
    pluralize_with_size(s, i)
}

/// Go: `flect.Humanize(s)`. Panics where Go panics (see the module docs); use [`try_humanize`]
/// to get that panic as an error.
pub fn humanize(s: &str) -> String {
    match try_humanize_bytes(s.as_bytes()) {
        Ok(b) => bytes_to_string(b),
        Err(e) => panic!("{}", e.message()),
    }
}

/// Go: `flect.Humanize(s)`; Go's runtime panic (`index out of range [0] with length 0`) is
/// returned as an error.
pub fn try_humanize(s: &str) -> Result<String> {
    try_humanize_bytes(s.as_bytes()).map(bytes_to_string)
}

// Go: flect humanize.go:Humanize
/// Go: `flect.Humanize(s)` over Go string bytes; Go's runtime panic is returned as an error.
pub fn try_humanize_bytes(s: &[u8]) -> Result<Vec<u8>> {
    let d = data();
    Ok(Ident::new_with(&d, s).humanize_with(&d)?.original)
}

/// Go: `flect.Ordinalize(s)`.
pub fn ordinalize(s: &str) -> String {
    bytes_to_string(ordinalize_bytes(s.as_bytes()))
}

// Go: flect ordinalize.go:Ordinalize
/// Go: `flect.Ordinalize(s)` over Go string bytes.
pub fn ordinalize_bytes(s: &[u8]) -> Vec<u8> {
    let d = data();
    Ident::new_with(&d, s).ordinalize_with(&d).original
}

/// Go: `flect.Titleize(s)`.
pub fn titleize(s: &str) -> String {
    bytes_to_string(titleize_bytes(s.as_bytes()))
}

// Go: flect titleize.go:Titleize
/// Go: `flect.Titleize(s)` over Go string bytes.
pub fn titleize_bytes(s: &[u8]) -> Vec<u8> {
    let d = data();
    titleize_with(&d, s)
}

/// Go: `flect.Capitalize(s)`.
pub fn capitalize(s: &str) -> String {
    bytes_to_string(capitalize_bytes(s.as_bytes()))
}

// Go: flect capitalize.go:Capitalize
/// Go: `flect.Capitalize(s)` over Go string bytes.
pub fn capitalize_bytes(s: &[u8]) -> Vec<u8> {
    let d = data();
    capitalize_with(&d, s)
}

// Go: flect plural_rules.go:AddPlural
/// Go: `flect.AddPlural(suffix, repl)`.
pub fn add_plural(suffix: &str, repl: &str) {
    insert_plural_rule(suffix, repl);
}

// Go: flect plural_rules.go:InsertPluralRule
/// Go: `flect.InsertPluralRule(suffix, repl)`.
pub fn insert_plural_rule(suffix: &str, repl: &str) {
    let mut d = data_mut();
    d.insert_plural_rule(suffix.as_bytes(), repl.as_bytes());
}

// Go: flect singular_rules.go:AddSingular
/// Go: `flect.AddSingular(ext, repl)`.
pub fn add_singular(ext: &str, repl: &str) {
    insert_singular_rule(ext, repl);
}

// Go: flect singular_rules.go:InsertSingularRule
/// Go: `flect.InsertSingularRule(suffix, repl)`.
pub fn insert_singular_rule(suffix: &str, repl: &str) {
    let mut d = data_mut();
    d.insert_singular_rule(suffix.as_bytes(), repl.as_bytes());
}

// Go: flect custom_data.go:LoadAcronyms
/// Go: `flect.LoadAcronyms(r)` with the reader's bytes.
pub fn load_acronyms(r: &[u8]) -> Result<()> {
    let mut d = data_mut();
    d.load_acronyms(r)
}

// Go: flect custom_data.go:LoadInflections
/// Go: `flect.LoadInflections(r)` with the reader's bytes.
pub fn load_inflections(r: &[u8]) -> Result<()> {
    let mut d = data_mut();
    d.load_inflections(r)
}

// ---------------------------------------------------------------------------
// Ident (ident.go)

// Go: flect ident.go:Ident
/// Ident represents the string and it's parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    pub original: Vec<u8>,
    pub parts: Vec<Vec<u8>>,
}

impl Ident {
    // Go: flect ident.go:New
    /// New creates a new Ident from the string.
    pub fn new(s: &[u8]) -> Ident {
        let d = data();
        Ident::new_with(&d, s)
    }

    fn new_with(d: &Data, s: &[u8]) -> Ident {
        Ident {
            original: s.to_vec(),
            parts: to_parts(d, s),
        }
    }

    // Go: flect ident.go:String
    /// String implements fmt.Stringer and returns the original string.
    pub fn string(&self) -> &[u8] {
        &self.original
    }

    // Go: flect ident.go:LastPart
    /// LastPart returns the last part/word of the original string.
    pub fn last_part(&self) -> &[u8] {
        match self.parts.last() {
            Some(p) => p,
            None => b"",
        }
    }

    // Go: flect ident.go:ReplaceSuffix
    /// ReplaceSuffix creates a new Ident with the original suffix replaced by new.
    fn replace_suffix_with(&self, d: &Data, orig: &[u8], new: &[u8]) -> Ident {
        let mut s = strings::trim_suffix(&self.original, orig).to_vec();
        s.extend_from_slice(new);
        Ident::new_with(d, &s)
    }

    // Go: flect humanize.go:(Ident).Humanize
    /// Humanize First letter of sentence capitalized.
    fn humanize_with(&self, d: &Data) -> Result<Ident> {
        if self.original.is_empty() {
            return Ok(Ident::new_with(d, b""));
        }

        if strings::trim_space(&self.original).is_empty() {
            return Ok(self.clone());
        }

        let Some(first) = self.parts.first() else {
            return Err(Error::new(
                "runtime error: index out of range [0] with length 0",
            ));
        };
        let mut parts = xappend(d, Vec::new(), &[titleize_with(d, first)]);
        if self.parts.len() > 1 {
            parts = xappend(d, parts, &self.parts[1..]);
        }

        Ok(Ident::new_with(d, &strings::join(&parts, b" ")))
    }

    // Go: flect titleize.go:(Ident).Titleize
    /// Titleize will capitalize the start of each part.
    fn titleize_with(&self, d: &Data) -> Ident {
        let mut parts: Vec<Vec<u8>> = Vec::new();

        for part in &self.parts {
            // CAUTION: in unicode, []rune(str)[0] is not rune(str[0])
            let runes = utf8::to_runes(part);
            let mut x = utf8::rune_to_string(go_unicode::to_title(runes[0]));
            if runes.len() > 1 {
                x.extend_from_slice(&utf8::from_runes(&runes[1..]));
            }
            parts.push(x);
        }

        Ident::new_with(d, &strings::join(&parts, b" "))
    }

    // Go: flect capitalize.go:(Ident).Capitalize
    /// Capitalize will cap the first letter of string.
    fn capitalize_with(&self, d: &Data) -> Ident {
        if self.parts.is_empty() {
            return Ident::new_with(d, b"");
        }
        let mut runes = utf8::to_runes(&self.original);
        runes[0] = go_unicode::to_title(runes[0]);
        Ident::new_with(d, &utf8::from_runes(&runes))
    }

    // Go: flect ordinalize.go:(Ident).Ordinalize
    /// Ordinalize converts a number to an ordinal version.
    fn ordinalize_with(&self, d: &Data) -> Ident {
        let Ok(number) = go_strconv::atoi(&self.original) else {
            return self.clone();
        };
        let mut s: Vec<u8> = Vec::new();
        match abs(number) % 100 {
            11..=13 => s = format!("{}th", go_strconv::itoa(number)).into_bytes(),
            _ => match abs(number) % 10 {
                1 => s = format!("{}st", go_strconv::itoa(number)).into_bytes(),
                2 => s = format!("{}nd", go_strconv::itoa(number)).into_bytes(),
                3 => s = format!("{}rd", go_strconv::itoa(number)).into_bytes(),
                _ => {}
            },
        }
        if !s.is_empty() {
            return Ident::new_with(d, &s);
        }
        Ident::new_with(d, format!("{}th", go_strconv::itoa(number)).as_bytes())
    }

    // Go: flect pluralize.go:(Ident).Pluralize
    /// Pluralize returns a plural version of the string.
    fn pluralize_with(&self, d: &Data) -> Ident {
        let s = self.last_part();
        if s.is_empty() {
            return Ident::new_with(d, b"");
        }

        // check if the Original has an explicit entry in the map
        if let Some(p) = d.single_to_plural.get(&self.original) {
            return self.replace_suffix_with(d, &self.original, p);
        }
        if d.plural_to_single.contains_key(&self.original) {
            return self.clone();
        }

        let ls = strings::to_lower(s);
        if d.plural_to_single.contains_key(ls.as_ref()) {
            return self.clone();
        }

        if let Some(p) = d.single_to_plural.get(ls.as_ref()) {
            let mut p = p.clone();
            if s == capitalize_with(d, s).as_slice() {
                p = capitalize_with(d, &p);
            }
            return self.replace_suffix_with(d, s, &p);
        }

        for r in &d.plural_rules {
            if strings::has_suffix(s, &r.suffix) {
                return self.replace_suffix_with(d, s, &r.apply(s));
            }
        }

        if strings::has_suffix(&ls, b"s") {
            return self.clone();
        }

        let mut out = self.original.clone();
        out.push(b's');
        Ident::new_with(d, &out)
    }

    // Go: flect singularize.go:(Ident).Singularize
    /// Singularize returns a singular version of the string.
    fn singularize_with(&self, d: &Data) -> Ident {
        let s = self.last_part();
        if s.is_empty() {
            return self.clone();
        }

        // check if the Original has an explicit entry in the map
        if let Some(p) = d.plural_to_single.get(&self.original) {
            return self.replace_suffix_with(d, &self.original, p);
        }
        if d.single_to_plural.contains_key(&self.original) {
            return self.clone();
        }

        let ls = strings::to_lower(s);
        if let Some(p) = d.plural_to_single.get(ls.as_ref()) {
            let mut p = p.clone();
            if s == capitalize_with(d, s).as_slice() {
                p = capitalize_with(d, &p);
            }
            return self.replace_suffix_with(d, s, &p);
        }

        if d.single_to_plural.contains_key(ls.as_ref()) {
            return self.clone();
        }

        for r in &d.singular_rules {
            if strings::has_suffix(s, &r.suffix) {
                return self.replace_suffix_with(d, s, &r.apply(s));
            }
        }

        if strings::has_suffix(s, b"s") {
            return self.replace_suffix_with(d, b"s", b"");
        }

        self.clone()
    }
}

// Go: flect titleize.go:Titleize (with the package data already locked)
fn titleize_with(d: &Data, s: &[u8]) -> Vec<u8> {
    Ident::new_with(d, s).titleize_with(d).original
}

// Go: flect capitalize.go:Capitalize (with the package data already locked)
fn capitalize_with(d: &Data, s: &[u8]) -> Vec<u8> {
    Ident::new_with(d, s).capitalize_with(d).original
}

// Go: flect ident.go:toParts
fn to_parts(d: &Data, s: &[u8]) -> Vec<Vec<u8>> {
    let mut parts: Vec<Vec<u8>> = Vec::new();
    let s = strings::trim_space(s);
    if s.is_empty() {
        return parts;
    }
    let upper = strings::to_upper(s);
    if d.base_acronyms.contains(upper.as_ref()) {
        return vec![upper.into_owned()];
    }
    let mut prev: Rune = 0;
    let mut x: Vec<u8> = Vec::with_capacity(s.len());
    for (_, c) in utf8::runes(s) {
        if !utf8::valid_rune(c) {
            continue;
        }

        if is_space(c) {
            parts = xappend(d, parts, &[std::mem::take(&mut x)]);
            utf8::append_rune(&mut x, c);
            prev = c;
            continue;
        }

        if go_unicode::is_upper(c) && !go_unicode::is_upper(prev) {
            parts = xappend(d, parts, &[std::mem::take(&mut x)]);
            utf8::append_rune(&mut x, c);
            prev = c;
            continue;
        }
        if go_unicode::is_upper(c) && d.base_acronyms.contains(strings::to_upper(&x).as_ref()) {
            parts = xappend(d, parts, &[std::mem::take(&mut x)]);
            utf8::append_rune(&mut x, c);
            prev = c;
            continue;
        }
        if go_unicode::is_letter(c)
            || go_unicode::is_digit(c)
            || go_unicode::is_punct(c)
            || c == '`' as Rune
        {
            prev = c;
            utf8::append_rune(&mut x, c);
            continue;
        }

        parts = xappend(d, parts, &[std::mem::take(&mut x)]);
        prev = c;
    }
    xappend(d, parts, &[x])
}

// ---------------------------------------------------------------------------
// flect.go

/// Go: `var spaces = []rune{'_', ' ', ':', '-', '/'}`.
const SPACES: [Rune; 5] = [
    '_' as Rune,
    ' ' as Rune,
    ':' as Rune,
    '-' as Rune,
    '/' as Rune,
];

// Go: flect flect.go:isSpace
fn is_space(c: Rune) -> bool {
    for r in SPACES {
        if r == c {
            return true;
        }
    }
    go_unicode::is_space(c)
}

// Go: flect flect.go:xappend
fn xappend<T: AsRef<[u8]>>(d: &Data, mut a: Vec<Vec<u8>>, ss: &[T]) -> Vec<Vec<u8>> {
    for s in ss {
        let mut s: &[u8] = strings::trim_space(s.as_ref());
        for x in SPACES {
            s = strings::trim(s, &utf8::rune_to_string(x));
        }
        let upper = strings::to_upper(s);
        let s: Vec<u8> = if d.base_acronyms.contains(upper.as_ref()) {
            upper.into_owned()
        } else {
            s.to_vec()
        };
        if !s.is_empty() {
            a.push(s);
        }
    }
    a
}

// Go: flect flect.go:abs
fn abs(x: i64) -> i64 {
    if x < 0 {
        return x.wrapping_neg();
    }
    x
}

// ---------------------------------------------------------------------------
// rule.go

// Go: flect rule.go:rule
#[derive(Clone, Debug)]
struct Rule {
    suffix: Vec<u8>,
    /// `Some(repl)`: `simpleRuleFunc(suffix, repl)`; `None`: `noop`.
    repl: Option<Vec<u8>>,
}

impl Rule {
    // Go: flect rule.go:simpleRuleFunc, noop
    fn apply(&self, s: &[u8]) -> Vec<u8> {
        match &self.repl {
            Some(repl) => {
                let mut out = s[..s.len() - self.suffix.len()].to_vec();
                out.extend_from_slice(repl);
                out
            }
            None => s.to_vec(),
        }
    }
}

// ---------------------------------------------------------------------------
// Package state

/// The package-level variables of flect: `baseAcronyms`, `singleToPlural`, `pluralToSingle`,
/// `pluralRules`, `singularRules`.
struct Data {
    base_acronyms: HashSet<Vec<u8>>,
    single_to_plural: HashMap<Vec<u8>, Vec<u8>>,
    plural_to_single: HashMap<Vec<u8>, Vec<u8>>,
    plural_rules: Vec<Rule>,
    singular_rules: Vec<Rule>,
}

fn state() -> &'static RwLock<Data> {
    static DATA: OnceLock<RwLock<Data>> = OnceLock::new();
    DATA.get_or_init(|| RwLock::new(Data::init_from_env()))
}

fn data() -> RwLockReadGuard<'static, Data> {
    match state().read() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn data_mut() -> std::sync::RwLockWriteGuard<'static, Data> {
    match state().write() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

impl Data {
    /// The package variables before any `init()` runs.
    fn base() -> Data {
        Data {
            base_acronyms: BASE_ACRONYMS
                .iter()
                .map(|a| a.as_bytes().to_vec())
                .collect(),
            single_to_plural: HashMap::new(),
            plural_to_single: HashMap::new(),
            plural_rules: Vec::new(),
            singular_rules: Vec::new(),
        }
    }

    /// All `init()` functions in Go's order, with custom data from the process environment.
    fn init_from_env() -> Data {
        let mut d = Data::base();
        // Go: flect custom_data.go:init
        d.load_custom_data(
            "inflections.json",
            "INFLECT_PATH",
            "could not read inflection file",
            Data::load_inflections,
        );
        d.load_custom_data(
            "acronyms.json",
            "ACRONYMS_PATH",
            "could not read acronyms file",
            Data::load_acronyms,
        );
        d.init_dictionary();
        d.init_rules();
        d
    }

    /// All `init()` functions with the given custom data files (`None`: no file). Test seam for
    /// the custom-data oracle cases.
    fn init_with(inflections: Option<&[u8]>, acronyms: Option<&[u8]>) -> (Data, Vec<String>) {
        let mut d = Data::base();
        let mut printed = Vec::new();
        let loaded = [
            inflections.map(|b| d.load_inflections(b)),
            acronyms.map(|b| d.load_acronyms(b)),
        ];
        for e in loaded.into_iter().flatten().filter_map(|r| r.err()) {
            printed.push(e.message().to_string());
        }
        d.init_dictionary();
        d.init_rules();
        (d, printed)
    }

    // Go: flect custom_data.go:loadCustomData
    fn load_custom_data(
        &mut self,
        default_file: &str,
        env: &str,
        read_error_message: &str,
        parser: fn(&mut Data, &[u8]) -> Result<()>,
    ) {
        let pwd = std::env::current_dir().unwrap_or_default();
        let path = match std::env::var_os(env) {
            Some(p) => std::path::PathBuf::from(p),
            None => pwd.join(default_file),
        };

        if std::fs::metadata(&path).is_err() {
            return;
        }

        let b = match std::fs::read(&path) {
            Ok(b) => b,
            Err(err) => {
                println!("{} {} ({})", read_error_message, path.display(), err);
                return;
            }
        };

        if let Err(err) = parser(self, &b) {
            println!("{}", err.message());
        }
    }

    // Go: flect custom_data.go:LoadAcronyms
    fn load_acronyms(&mut self, r: &[u8]) -> Result<()> {
        let m = decode_json(r, JsonShape::StringSlice).map_err(|err| {
            Error::new(format!(
                "could not decode acronyms JSON from reader: {}",
                err
            ))
        })?;

        for (acronym, _) in m {
            self.base_acronyms.insert(acronym);
        }

        Ok(())
    }

    // Go: flect custom_data.go:LoadInflections
    //
    // Go ranges over a map (random order) and stops at the first multi-word entry; here the
    // entries are applied in byte order of the singular.
    fn load_inflections(&mut self, r: &[u8]) -> Result<()> {
        let m = decode_json(r, JsonShape::StringMap).map_err(|err| {
            Error::new(format!(
                "could not decode inflection JSON from reader: {}",
                err
            ))
        })?;

        for (s, p) in m {
            if strings::contains(&s, b" ") || strings::contains(&p, b" ") {
                // flect works with parts, so multi-words should not be allowed
                return Err(Error::new("inflection elements should be a single word"));
            }
            self.single_to_plural.insert(s.clone(), p.clone());
            self.plural_to_single.insert(p, s);
        }

        Ok(())
    }

    // Go: flect plural_rules.go:InsertPluralRule
    fn insert_plural_rule(&mut self, suffix: &[u8], repl: &[u8]) {
        self.plural_rules.insert(
            0,
            Rule {
                suffix: suffix.to_vec(),
                repl: Some(repl.to_vec()),
            },
        );
        self.plural_rules.insert(
            0,
            Rule {
                suffix: repl.to_vec(),
                repl: None,
            },
        );
    }

    // Go: flect singular_rules.go:InsertSingularRule
    fn insert_singular_rule(&mut self, suffix: &[u8], repl: &[u8]) {
        self.singular_rules.insert(
            0,
            Rule {
                suffix: suffix.to_vec(),
                repl: Some(repl.to_vec()),
            },
        );
        self.singular_rules.insert(
            0,
            Rule {
                suffix: repl.to_vec(),
                repl: None,
            },
        );
    }

    // Go: flect plural_rules.go:init (first: build singleToPlural and pluralToSingle)
    fn init_dictionary(&mut self) {
        for wd in DICTIONARY {
            let singular = wd.singular.as_bytes();
            if self
                .single_to_plural
                .get(singular)
                .is_some_and(|p| !p.is_empty())
            {
                panic!(
                    "map singleToPlural already has an entry for {}",
                    wd.singular
                );
            }

            let mut plural = wd.plural;
            if wd.uncountable && plural.is_empty() {
                plural = wd.singular;
            }

            if plural.is_empty() {
                panic!("plural for {} is not provided", wd.singular);
            }

            self.single_to_plural
                .insert(singular.to_vec(), plural.as_bytes().to_vec());

            if !wd.unidirectional {
                if self
                    .plural_to_single
                    .get(plural.as_bytes())
                    .is_some_and(|s| !s.is_empty())
                {
                    panic!("map pluralToSingle already has an entry for {}", plural);
                }
                self.plural_to_single
                    .insert(plural.as_bytes().to_vec(), singular.to_vec());

                if !wd.alternative.is_empty() {
                    if self
                        .plural_to_single
                        .get(wd.alternative.as_bytes())
                        .is_some_and(|s| !s.is_empty())
                    {
                        panic!(
                            "map pluralToSingle already has an entry for {}",
                            wd.alternative
                        );
                    }
                    self.plural_to_single
                        .insert(wd.alternative.as_bytes().to_vec(), singular.to_vec());
                }
            }
        }
    }

    // Go: flect plural_rules.go:init (second: build pluralRules and singularRules)
    fn init_rules(&mut self) {
        for (singular, plural) in SINGULAR_TO_PLURAL_SUFFIX_LIST.iter().rev() {
            self.insert_plural_rule(singular.as_bytes(), plural.as_bytes());
            self.insert_singular_rule(plural.as_bytes(), singular.as_bytes());
        }

        // build pluralRule and singularRule with dictionary for compound words
        for wd in DICTIONARY {
            if wd.exact {
                continue;
            }

            let mut plural = wd.plural;
            if wd.uncountable && plural.is_empty() {
                plural = wd.singular;
            }

            self.insert_plural_rule(wd.singular.as_bytes(), plural.as_bytes());

            if !wd.unidirectional {
                self.insert_singular_rule(plural.as_bytes(), wd.singular.as_bytes());

                if !wd.alternative.is_empty() {
                    self.insert_singular_rule(wd.alternative.as_bytes(), wd.singular.as_bytes());
                }
            }
        }
    }
}

enum JsonShape {
    /// `[]string`
    StringSlice,
    /// `map[string]string`
    StringMap,
}

/// `json.NewDecoder(r).Decode(&m)` for `m` of type `[]string` or `map[string]string`: decodes
/// the first JSON value (trailing data is not read); JSON `null` leaves `m` nil (and an element
/// or map value `null` is the zero string, v1 semantics). Returns `(elem, "")` pairs for a slice
/// and `(key, value)` pairs in byte order for a map (Go reports the first value of the wrong
/// type in document order).
fn decode_json(r: &[u8], shape: JsonShape) -> std::result::Result<Vec<(Vec<u8>, Vec<u8>)>, String> {
    use crate::cast::caste::{json_kind, json_map_type_error, json_unmarshal_type_error};
    use go_value::Value;

    let mut dec = go_json::Decoder::new(r);
    // Values are type-checked below; json.Number keeps a huge number a type error, as in Go.
    dec.use_number();
    let v = dec.decode().map_err(|e| e.to_string())?;
    let type_error = match shape {
        JsonShape::StringMap => json_map_type_error(r, "string", |v| {
            matches!(v, Value::String(_) | Value::Invalid)
        }),
        JsonShape::StringSlice => None,
    };
    if let Some(err) = type_error {
        return Err(err);
    }
    let str_of = |v: &Value, field: &str| -> std::result::Result<Vec<u8>, String> {
        match v {
            Value::String(s) => Ok(s.to_vec()),
            Value::Invalid => Ok(Vec::new()),
            other => Err(json_unmarshal_type_error(json_kind(other), field, "string")),
        }
    };
    match (shape, &v) {
        (_, Value::Invalid) => Ok(Vec::new()),
        (JsonShape::StringSlice, Value::List(l)) => l
            .items
            .iter()
            .enumerate()
            .map(|(i, it)| str_of(it, &i.to_string()).map(|s| (s, Vec::new())))
            .collect(),
        (JsonShape::StringMap, Value::Map(m)) => m
            .entries
            .iter()
            .map(|(k, it)| str_of(it, &k.to_str_lossy()).map(|s| (k.to_vec(), s)))
            .collect(),
        (JsonShape::StringSlice, other) => {
            Err(json_unmarshal_type_error(json_kind(other), "", "[]string"))
        }
        (JsonShape::StringMap, other) => Err(json_unmarshal_type_error(
            json_kind(other),
            "",
            "map[string]string",
        )),
    }
}

/// `String` from bytes produced by a string transformation of valid UTF-8 input (always valid).
fn bytes_to_string(b: Vec<u8>) -> String {
    match String::from_utf8(b) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }
}

/// Test support: runs flect's `init()` sequence with the given `inflections.json` and
/// `acronyms.json` contents (`None` = no file) on fresh package state and applies `op`
/// (`"pluralize"`, `"singularize"`, `"humanize"`, `"titleize"`, `"capitalize"`,
/// `"ordinalize"`) to `input`. Returns the result (or the Go panic message of `Humanize`) and the
/// lines Go prints to stdout while loading. Panics like Go's `init()` when the custom
/// inflections collide with the dictionary.
#[doc(hidden)]
pub fn with_custom_data_for_tests(
    inflections: Option<&[u8]>,
    acronyms: Option<&[u8]>,
    op: &str,
    input: &[u8],
) -> (std::result::Result<Vec<u8>, String>, Vec<String>) {
    let (d, printed) = Data::init_with(inflections, acronyms);
    let id = Ident::new_with(&d, input);
    let out = match op {
        "pluralize" => Ok(id.pluralize_with(&d).original),
        "singularize" => Ok(id.singularize_with(&d).original),
        "humanize" => id
            .humanize_with(&d)
            .map(|i| i.original)
            .map_err(|e| e.message().to_string()),
        "titleize" => Ok(id.titleize_with(&d).original),
        "capitalize" => Ok(id.capitalize_with(&d).original),
        "ordinalize" => Ok(id.ordinalize_with(&d).original),
        other => Err(format!("unknown op {other}")),
    };
    (out, printed)
}

// ---------------------------------------------------------------------------
// Tables

/// Go: `baseAcronyms` (acronyms.go). Lookups use `strings.ToUpper`, so the mixed-case entries
/// (`gbps`, `kbps`, `Mbps`, `MoCA`, `WiFi`) can never match.
const BASE_ACRONYMS: &[&str] = &[
    "OK", "UTF8", "HTML", "JSON", "JWT", "ID", "UUID", "SQL", "ACK", "ACL", "ADSL", "AES", "ANSI",
    "API", "ARP", "ATM", "BGP", "BSS", "CCITT", "CHAP", "CIDR", "CIR", "CLI", "CPE", "CPU", "CRC",
    "CRT", "CSMA", "CMOS", "DCE", "DEC", "DES", "DHCP", "DNS", "DRAM", "DSL", "DSLAM", "DTE",
    "DMI", "EHA", "EIA", "EIGRP", "EOF", "ESS", "FCC", "FCS", "FDDI", "FTP", "GBIC", "gbps",
    "GEPOF", "HDLC", "HTTP", "HTTPS", "IANA", "ICMP", "IDF", "IDS", "IEEE", "IETF", "IMAP", "IP",
    "IPS", "ISDN", "ISP", "kbps", "LACP", "LAN", "LAPB", "LAPF", "LLC", "MAC", "Mbps", "MC", "MDF",
    "MIB", "MoCA", "MPLS", "MTU", "NAC", "NAT", "NBMA", "NIC", "NRZ", "NRZI", "NVRAM", "OSI",
    "OSPF", "OUI", "PAP", "PAT", "PC", "PIM", "PCM", "PDU", "POP3", "POTS", "PPP", "PPTP", "PTT",
    "PVST", "RAM", "RARP", "RFC", "RIP", "RLL", "ROM", "RSTP", "RTP", "RCP", "SDLC", "SFD", "SFP",
    "SLARP", "SLIP", "SMTP", "SNA", "SNAP", "SNMP", "SOF", "SRAM", "SSH", "SSID", "STP", "SYN",
    "TDM", "TFTP", "TIA", "TOFU", "UDP", "URL", "URI", "USB", "UTP", "VC", "VLAN", "VLSM", "VPN",
    "W3C", "WAN", "WEP", "WiFi", "WPA", "WWW",
];

// Go: flect plural_rules.go:word
struct Word {
    singular: &'static str,
    plural: &'static str,
    alternative: &'static str,
    /// plural to singular is not possible (or bad)
    unidirectional: bool,
    uncountable: bool,
    exact: bool,
}

const fn w(singular: &'static str, plural: &'static str) -> Word {
    Word {
        singular,
        plural,
        alternative: "",
        unidirectional: false,
        uncountable: false,
        exact: false,
    }
}

const fn wa(singular: &'static str, plural: &'static str, alternative: &'static str) -> Word {
    Word {
        alternative,
        ..w(singular, plural)
    }
}

const fn wu(singular: &'static str, plural: &'static str) -> Word {
    Word {
        unidirectional: true,
        ..w(singular, plural)
    }
}

const fn wx(singular: &'static str, plural: &'static str) -> Word {
    Word {
        exact: true,
        ..w(singular, plural)
    }
}

const fn wc(singular: &'static str) -> Word {
    Word {
        uncountable: true,
        ..w(singular, "")
    }
}

/// Go: `dictionary` (plural_rules.go), in order.
const DICTIONARY: &[Word] = &[
    // identicals https://en.wikipedia.org/wiki/English_plurals#Nouns_with_identical_singular_and_plural
    w("aircraft", "aircraft"),
    wa("beef", "beef", "beefs"),
    w("bison", "bison"),
    wu("blues", "blues"),
    w("chassis", "chassis"),
    w("deer", "deer"),
    wa("fish", "fish", "fishes"),
    w("moose", "moose"),
    w("police", "police"),
    wa("salmon", "salmon", "salmons"),
    w("series", "series"),
    w("sheep", "sheep"),
    wa("shrimp", "shrimp", "shrimps"),
    w("species", "species"),
    wa("swine", "swine", "swines"),
    wa("trout", "trout", "trouts"),
    wa("tuna", "tuna", "tunas"),
    w("you", "you"),
    // -en https://en.wikipedia.org/wiki/English_plurals#Plurals_in_-(e)n
    w("child", "children"),
    wx("ox", "oxen"),
    // apophonic https://en.wikipedia.org/wiki/English_plurals#Apophonic_plurals
    w("foot", "feet"),
    w("goose", "geese"),
    w("man", "men"),
    w("human", "humans"), // not humen
    wx("louse", "lice"),
    w("mouse", "mice"),
    w("tooth", "teeth"),
    w("woman", "women"),
    // misc https://en.wikipedia.org/wiki/English_plurals#Miscellaneous_irregular_plurals
    wx("die", "dice"),
    w("person", "people"),
    // Words from French that end in -u add an x; in addition to eau to eaux rule
    wa("adieu", "adieux", "adieus"),
    w("fabliau", "fabliaux"),
    wa("bureau", "bureaus", "bureaux"), // popular
    // Words from Greek that end in -on change -on to -a; in addition to hedron rule
    w("criterion", "criteria"),
    wa("ganglion", "ganglia", "ganglions"),
    wa("lexicon", "lexica", "lexicons"),
    wa("mitochondrion", "mitochondria", "mitochondrions"),
    w("noumenon", "noumena"),
    w("phenomenon", "phenomena"),
    w("taxon", "taxa"),
    // Words from Latin that end in -um change -um to -a; in addition to some rules
    w("media", "media"), // popular case: media -> media
    Word {
        alternative: "mediums",
        unidirectional: true,
        ..w("medium", "media")
    },
    wa("stadium", "stadiums", "stadia"),
    wa("aquarium", "aquaria", "aquariums"),
    wa("auditorium", "auditoria", "auditoriums"),
    wa("symposium", "symposia", "symposiums"),
    wa("curriculum", "curriculums", "curricula"), // ulum
    w("quota", "quotas"),
    // Words from Latin that end in -us change -us to -i or -era
    wa("alumnus", "alumni", "alumnuses"), // -i
    w("bacillus", "bacilli"),
    wa("cactus", "cacti", "cactuses"),
    w("coccus", "cocci"),
    wa("focus", "foci", "focuses"),
    wa("locus", "loci", "locuses"),
    wa("nucleus", "nuclei", "nucleuses"),
    wa("octopus", "octupuses", "octopi"),
    wa("radius", "radii", "radiuses"),
    w("syllabus", "syllabi"),
    wa("corpus", "corpora", "corpuses"), // -ra
    w("genus", "genera"),
    // Words from Latin that end in -a change -a to -ae
    w("alumna", "alumnae"),
    w("vertebra", "vertebrae"),
    w("differentia", "differentiae"), // -tia
    w("minutia", "minutiae"),
    w("vita", "vitae"),   // -ita
    w("larva", "larvae"), // -va
    w("postcava", "postcavae"),
    w("praecava", "praecavae"),
    w("uva", "uvae"),
    // Words from Latin that end in -ex change -ex to -ices
    wa("apex", "apices", "apexes"),
    wa("codex", "codices", "codexes"),
    wa("index", "indices", "indexes"),
    wa("latex", "latices", "latexes"),
    wa("vertex", "vertices", "vertexes"),
    wa("vortex", "vortices", "vortexes"),
    // Words from Latin that end in -ix change -ix to -ices (eg, matrix becomes matrices)
    wa("appendix", "appendices", "appendixes"),
    wa("radix", "radices", "radixes"),
    wa("helix", "helices", "helixes"),
    // Words from Latin that end in -is change -is to -es
    wx("axis", "axes"),
    w("crisis", "crises"),
    wu("ellipsis", "ellipses"), // ellipse
    w("genesis", "geneses"),
    w("oasis", "oases"),
    w("thesis", "theses"),
    w("testis", "testes"),
    w("base", "bases"), // popular case
    wu("basis", "bases"),
    wx("alias", "aliases"),   // no alia, no aliasis
    w("vedalia", "vedalias"), // no vedalium, no vedaliases
    // Words that end in -ch, -o, -s, -sh, -x, -z (can be conflict with the others)
    wx("use", "uses"), // us vs use
    w("abuse", "abuses"),
    w("cause", "causes"),
    w("clause", "clauses"),
    w("cruse", "cruses"),
    w("excuse", "excuses"),
    w("fuse", "fuses"),
    w("house", "houses"),
    w("misuse", "misuses"),
    w("muse", "muses"),
    w("pause", "pauses"),
    w("ache", "aches"),
    w("topaz", "topazes"),
    wa("buffalo", "buffaloes", "buffalos"),
    w("potato", "potatoes"),
    w("tomato", "tomatoes"),
    // uncountables
    wc("equipment"),
    wc("information"),
    wc("jeans"),
    wc("money"),
    wc("news"),
    wc("rice"),
    // exceptions: -f to -ves, not -fe
    wa("dwarf", "dwarfs", "dwarves"),
    wa("hoof", "hoofs", "hooves"),
    w("thief", "thieves"),
    // exceptions: instead of -f(e) to -ves
    w("chive", "chives"),
    w("hive", "hives"),
    w("move", "moves"),
    // exceptions: instead of -y to -ies
    w("movie", "movies"),
    w("cookie", "cookies"),
    // exceptions: instead of -um to -a
    w("pretorium", "pretoriums"),
    w("agenda", "agendas"), // instead of plural of agendum
    // exceptions: instead of -um to -a (chemical element names)
    // Words from Latin that end in -a change -a to -ae
    wa("formula", "formulas", "formulae"), // also -um/-a
    // exceptions: instead of -o to -oes
    w("shoe", "shoes"),
    wx("toe", "toes"),
    w("graffiti", "graffiti"),
    // abbreviations
    wx("ID", "IDs"),
];

/// Go: `singularToPluralSuffixList` (plural_rules.go). The order is the rule priority.
const SINGULAR_TO_PLURAL_SUFFIX_LIST: &[(&str, &str)] = &[
    // Words that end in -f or -fe change -f or -fe to -ves
    ("tive", "tives"), // exception
    ("eaf", "eaves"),
    ("oaf", "oaves"),
    ("afe", "aves"),
    ("arf", "arves"),
    ("rfe", "rves"),
    ("rf", "rves"),
    ("lf", "lves"),
    ("fe", "ves"), // previously '[a-eg-km-z]fe' TODO: regex support
    // Words that end in -y preceded by a consonant change -y to -ies
    ("ay", "ays"),
    ("ey", "eys"),
    ("oy", "oys"),
    ("quy", "quies"),
    ("uy", "uys"),
    ("y", "ies"), // '[^aeiou]y'
    // Words from French that end in -u add an x (eg, château becomes châteaux)
    ("eau", "eaux"), // it seems like 'eau' is the most popular form of this rule
    // Words from Latin that end in -a change -a to -ae; before -on to -a and -um to -a
    ("bula", "bulae"),
    ("dula", "bulae"),
    ("lula", "bulae"),
    ("nula", "bulae"),
    ("vula", "bulae"),
    // Words from Greek that end in -on change -on to -a (eg, polyhedron becomes polyhedra)
    ("hedron", "hedra"),
    // Words from Latin that end in -um change -um to -a (eg, minimum becomes minima)
    ("ium", "ia"), // some exceptions especially chemical element names
    ("seum", "seums"),
    ("eum", "ea"),
    ("oum", "oa"),
    ("stracum", "straca"),
    ("dum", "da"),
    ("elum", "ela"),
    ("ilum", "ila"),
    ("olum", "ola"),
    ("ulum", "ula"),
    ("llum", "lla"),
    ("ylum", "yla"),
    ("imum", "ima"),
    ("ernum", "erna"),
    ("gnum", "gna"),
    ("brum", "bra"),
    ("crum", "cra"),
    ("terum", "tera"),
    ("serum", "sera"),
    ("trum", "tra"),
    ("antum", "anta"),
    ("atum", "ata"),
    ("entum", "enta"),
    ("etum", "eta"),
    ("itum", "ita"),
    ("otum", "ota"),
    ("utum", "uta"),
    ("ctum", "cta"),
    ("ovum", "ova"),
    // Words from Latin that end in -ex change -ex to -ices (eg, vortex becomes vortices)
    // Words from Latin that end in -ix change -ix to -ices (eg, matrix becomes matrices)
    ("trix", "trices"), // ignore a few words end in trice
    // Words from Latin that end in -is change -is to -es (eg, thesis becomes theses)
    ("iasis", "iases"),
    ("mesis", "meses"),
    ("kinesis", "kineses"),
    ("resis", "reses"),
    ("gnosis", "gnoses"), // e.g. diagnosis
    ("opsis", "opses"),   // e.g. synopsis
    ("ysis", "yses"),     // e.g. analysis
    // Words that end in -ch, -o, -s, -sh, -x, -z
    ("ouse", "ouses"),
    ("lause", "lauses"),
    ("us", "uses"), // use/uses is in the dictionary
    ("ch", "ches"),
    ("io", "ios"),
    ("sh", "shes"),
    ("ss", "sses"),
    ("ez", "ezzes"),
    ("iz", "izzes"),
    ("tz", "tzes"),
    ("zz", "zzes"),
    ("ano", "anos"),
    ("lo", "los"),
    ("to", "tos"),
    ("oo", "oos"),
    ("o", "oes"),
    ("x", "xes"),
    // for abbreviations
    ("S", "Ses"),
];

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (third-party: gobuffalo/flect v1.0.3; written by T26, not generated — the
// coverage run did not instrument third-party modules). `OK` = ported. Items without a prefix are
// not called by neohugo and are not ported (no stub needed: nothing can reach them).
// Source: acronyms.go
// OK L5-152: acronymsMoot, baseAcronyms
// Source: camelize.go — not used by neohugo
//    L12-14: Camelize(s string) string
//    L20-44: (i Ident) Camelize() Ident
// Source: capitalize.go
// OK L9-11: Capitalize(s string) string
// OK L17-24: (i Ident) Capitalize() Ident
// Source: custom_data.go
// OK L14-17: init()
//   types: CustomDataParser
// OK L23-42: loadCustomData(defaultFile, env, readErrorMessage string, parser CustomDataParser)
// OK L46-61: LoadAcronyms(r io.Reader) error
// OK L65-88: LoadInflections(r io.Reader) error
// Source: dasherize.go — not used by neohugo
//    L12-14: Dasherize(s string) string
//    L20-34: (i Ident) Dasherize() Ident
// Source: flect.go
// OK L11: spaces
// OK L13-20: isSpace(c rune) bool
// OK L22-36: xappend(a []string, ss ...string) []string
// OK L38-43: abs(x int) int
// Source: humanize.go
// OK L16-18: Humanize(s string) string
// OK L21-36: (i Ident) Humanize() Ident
// Source: ident.go
//   types: Ident
// OK L17-19: (i Ident) String() string
// OK L22-29: New(s string) Ident
// OK L31-95: toParts(s string) []string
// OK L101-106: (i *Ident) LastPart() string
// OK L109-111: (i Ident) ReplaceSuffix(orig, new string) Ident
//    L114-117: (i *Ident) UnmarshalText(data []byte) error — not used by neohugo
//    L120-122: (i Ident) MarshalText() ([]byte, error) — not used by neohugo
// Source: lower_upper.go — not used by neohugo
//    L6-8: (i Ident) ToUpper() Ident
//    L11-13: (i Ident) ToLower() Ident
// Source: ordinalize.go
// OK L12-14: Ordinalize(s string) string
// OK L20-43: (i Ident) Ordinalize() Ident
// Source: pascalize.go — not used by neohugo
//    L11-13: Pascalize(s string) string
//    L19-32: (i Ident) Pascalize() Ident
// Source: plural_rules.go
// OK L5: pluralRules
// OK L9-11: AddPlural(suffix string, repl string)
// OK L15-28: InsertPluralRule(suffix, repl string)
//   types: word
// OK L42-207: dictionary
// OK L213, L218: singleToPlural, pluralToSingle
// OK L224-254: init() (dictionary maps)
//   types: singularToPluralSuffix
// OK L266-384: singularToPluralSuffixList
// OK L391-417: init() (rules)
// Source: pluralize.go
// OK L8: pluralMoot
// OK L14-16: Pluralize(s string) string
// OK L21-26: PluralizeWithSize(s string, i int) string
// OK L32-72: (i Ident) Pluralize() Ident
// Source: rule.go
//   types: ruleFn, rule
// OK L10-15: simpleRuleFunc(suffix, repl string) func(string) string
// OK L17: noop(s string) string
// Source: singular_rules.go
// OK L3: singularRules
// OK L7-9: AddSingular(ext string, repl string)
// OK L13-26: InsertSingularRule(suffix, repl string)
// Source: singularize.go
// OK L8: singularMoot
// OK L14-16: Singularize(s string) string
// OK L21-23: SingularizeWithSize(s string, i int) string
// OK L29-69: (i Ident) Singularize() Ident
// Source: titleize.go
// OK L12-14: Titleize(s string) string
// OK L20-38: (i Ident) Titleize() Ident
// Source: underscore.go — not used by neohugo
//    L12-14: Underscore(s string) string
//    L20-35: (i Ident) Underscore() Ident
// ---------------------------------------------------------------------------
