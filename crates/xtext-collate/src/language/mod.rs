//! Port of the parts of golang.org/x/text@v0.26.0/language (language.go,
//! parse.go) that collation needs: `Parse`, `Make`, `CanonType.Canonicalize`,
//! `CanonType.Compose`, `Tag.Base`, `Tag.Raw`, `Tag.Parent`,
//! `Tag.Extensions`, `Tag.TypeForKey`, `Tag.SetTypeForKey`, plus the
//! internal packages they rest on (see `internal` and `compact`).

pub(crate) mod compact;
pub(crate) mod internal;
pub(crate) mod tables;

use std::fmt;

pub use internal::LangError;
use internal::{Builder, ITag, c, norm_lang, region_canonicalize, suppress_script};

/// A BCP 47 language tag (Go `language.Tag`). Equality is Go's `==`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Tag(pub(crate) ITag);

/// An ISO 639 base language (Go `language.Base`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Base(pub(crate) internal::Language);

/// An ISO 15924 script (Go `language.Script`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Script(pub(crate) internal::Script);

/// An ISO 3166-1 or UN M.49 region (Go `language.Region`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Region(pub(crate) internal::Region);

/// A single BCP 47 extension (Go `language.Extension`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Extension(pub(crate) String);

/// Confidence indicates the level of certainty for a given return value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    No,
    Low,
    High,
    Exact,
}

impl fmt::Display for Base {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&internal::lang_string(self.0))
    }
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&internal::script_string(self.0))
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&internal::region_string(self.0))
    }
}

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Region {
    // Go: language/language.go:Region.IsCountry
    pub fn is_country(&self) -> bool {
        internal::region_is_country(self.0)
    }
    // Go: language/language.go:Region.IsGroup
    pub fn is_group(&self) -> bool {
        internal::region_is_group(self.0)
    }
    // Go: language/language.go:Region.M49
    pub fn m49(&self) -> i32 {
        internal::region_m49(self.0)
    }
}

/// CanonType can be used to enable or disable various types of
/// canonicalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanonType(pub u32);

/// Replace deprecated base languages with their preferred replacements.
pub const DEPRECATED_BASE: CanonType = CanonType(1);
/// Replace deprecated scripts with their preferred replacements.
pub const DEPRECATED_SCRIPT: CanonType = CanonType(2);
/// Replace deprecated regions with their preferred replacements.
pub const DEPRECATED_REGION: CanonType = CanonType(4);
/// Remove redundant scripts.
pub const SUPPRESS_SCRIPT: CanonType = CanonType(8);
/// Normalize legacy encodings.
pub const LEGACY: CanonType = CanonType(16);
/// Map the dominant language of a macro language group to the macro
/// language subtag.
pub const MACRO: CanonType = CanonType(32);
/// Full compatibility with CLDR.
pub const CLDR: CanonType = CanonType(64);
/// Raw can be used to Compose or Parse without Canonicalization.
pub const RAW: CanonType = CanonType(0);
/// Replace all deprecated tags with their preferred replacements.
pub const DEPRECATED: CanonType = CanonType(1 | 2 | 4);
/// All canonicalizations recommended by BCP 47.
pub const BCP47: CanonType = CanonType(1 | 2 | 4 | 8);
/// All canonicalizations.
pub const ALL: CanonType = CanonType(1 | 2 | 4 | 8 | 16 | 32);
/// Default is the canonicalization used by Parse, Make and Compose.
pub const DEFAULT: CanonType = CanonType(1 | 2 | 4 | 16);
const CANON_LANG: u32 = 1 | 16 | 32;

// Go: language/language.go:canonicalize
/// Returns the canonicalized equivalent of the tag and whether there was any
/// change.
fn canonicalize(c_: CanonType, mut t: ITag) -> (ITag, bool) {
    let cc = c_.0;
    if cc == 0 {
        return (t, false);
    }
    let mut changed = false;
    if cc & SUPPRESS_SCRIPT.0 != 0 && suppress_script(t.lang_id) == t.script_id {
        t.script_id = 0;
        changed = true;
    }
    if cc & CANON_LANG != 0 {
        loop {
            let (l, alias_type) = norm_lang(t.lang_id);
            if l != t.lang_id {
                match alias_type {
                    2 => {
                        // Legacy
                        if cc & LEGACY.0 != 0 {
                            if t.lang_id == c("_sh") && t.script_id == 0 {
                                t.script_id = c("_Latn");
                            }
                            t.lang_id = l;
                            changed = true;
                        }
                    }
                    1 => {
                        // Macro
                        if cc & MACRO.0 != 0 {
                            // We deviate here from CLDR. The mapping "nb" ->
                            // "no" qualifies as a typical Macro language
                            // mapping. However, for legacy reasons, CLDR maps
                            // "no", the macro language code for Norwegian, to
                            // the dominant variant "nb".
                            if cc & CLDR.0 == 0 || t.lang_id != c("_nb") {
                                changed = true;
                                t.lang_id = l;
                            }
                        }
                    }
                    0 => {
                        // Deprecated
                        if cc & DEPRECATED_BASE.0 != 0 {
                            if t.lang_id == c("_mo") && t.region_id == 0 {
                                t.region_id = c("_MD");
                            }
                            t.lang_id = l;
                            changed = true;
                            // Other canonicalization types may still apply.
                            continue;
                        }
                    }
                    _ => {}
                }
            } else if cc & LEGACY.0 != 0 && t.lang_id == c("_no") && cc & CLDR.0 != 0 {
                t.lang_id = c("_nb");
                changed = true;
            }
            break;
        }
    }
    if cc & DEPRECATED_SCRIPT.0 != 0 && t.script_id == c("_Qaai") {
        changed = true;
        t.script_id = c("_Zinh");
    }
    if cc & DEPRECATED_REGION.0 != 0 {
        let r = region_canonicalize(t.region_id);
        if r != t.region_id {
            changed = true;
            t.region_id = r;
        }
    }
    (t, changed)
}

// Go: language/language.go:makeTag (compact.Make + Tag.tag())
/// Wraps an internal tag as a public Tag, stored as the internal tag Go's
/// `Tag.tag()` returns. When compact.FromTag reports an exact compact id, Go
/// keeps only the id, so the tag reads back as the id's tag (which differs
/// from `t` when the script id overflows the 8-bit CompactCoreInfo field,
/// e.g. "pa-Zzzz" -> "pa-Arab", or for `-u-rg-XXzzzz` tags whose ids are
/// exact, e.g. "th-TH-u-rg-thzzzz" -> "th-TH").
pub(crate) fn make_tag(t: ITag) -> Tag {
    Tag(compact::make(&t).tag())
}

/// A part passed to [`CanonType::compose`] (Go's `...interface{}`).
#[derive(Clone, Debug)]
pub enum Part {
    Tag(Tag),
    Base(Base),
    Script(Script),
    Region(Region),
    /// `[]Extension`: replaces all extensions.
    Extensions(Vec<Extension>),
    /// A single `Extension`.
    Extension(Extension),
}

impl CanonType {
    // Go: language/language.go:CanonType.Canonicalize
    /// Returns the canonicalized equivalent of the tag.
    pub fn canonicalize(self, t: &Tag) -> Tag {
        let (mut tag, changed) = canonicalize(self, t.0.clone());
        if changed {
            tag.remake_string();
            return make_tag(tag);
        }
        t.clone()
    }

    // Go: language/parse.go:CanonType.Parse
    /// Parses the given BCP 47 string. On failure it returns the error and
    /// any part of the tag that could be parsed (like Go).
    pub fn parse(self, s: &str) -> Result<Tag, (Tag, LangError)> {
        let (tt, err) = internal::parse(s);
        if let Some(e) = err {
            return Err((make_tag(tt), e));
        }
        let (mut tt, changed) = canonicalize(self, tt);
        if changed {
            tt.remake_string();
        }
        Ok(make_tag(tt))
    }

    // Go: language/language.go:CanonType.Make
    /// Parse, ignoring errors.
    pub fn make(self, s: &str) -> Tag {
        match self.parse(s) {
            Ok(t) => t,
            Err((t, _)) => t,
        }
    }

    // Go: language/tags.go:CanonType.MustParse
    pub fn must_parse(self, s: &str) -> Tag {
        match self.parse(s) {
            Ok(t) => t,
            Err((_, e)) => panic!("language.MustParse({s:?}): {e}"),
        }
    }

    // Go: language/parse.go:CanonType.Compose
    /// Creates a Tag from individual parts. Like Go it returns the tag even
    /// when an error is reported.
    pub fn compose(self, parts: &[Part]) -> (Tag, Option<LangError>) {
        let mut b = Builder::default();
        if let Some(e) = update(&mut b, parts) {
            return (Tag::default(), Some(e));
        }
        let (t, _) = canonicalize(self, b.tag.clone());
        b.tag = t;
        let t = b.make();
        if b.panicked {
            // Go: defer func() { if recover() != nil { t = Tag{}; err = ErrSyntax } }()
            return (Tag::default(), Some(LangError::Syntax));
        }
        (make_tag(t), None)
    }
}

// Go: language/parse.go:update
fn update(b: &mut Builder, parts: &[Part]) -> Option<LangError> {
    let mut err = None;
    for x in parts {
        match x {
            Part::Tag(v) => b.set_tag(&v.0),
            Part::Base(v) => b.tag.lang_id = v.0,
            Part::Script(v) => b.tag.script_id = v.0,
            Part::Region(v) => b.tag.region_id = v.0,
            Part::Extension(v) => {
                if v.0.is_empty() {
                    err = Some(LangError::InvalidArgument);
                } else {
                    b.set_ext(&v.0);
                }
            }
            Part::Extensions(v) => {
                b.clear_extensions();
                for e in v {
                    b.set_ext(&e.0);
                }
            }
        }
    }
    err
}

// Go: language/parse.go:Parse
/// Parses the given BCP 47 string with the Default canonicalization.
pub fn parse(s: &str) -> Result<Tag, LangError> {
    DEFAULT.parse(s).map_err(|(_, e)| e)
}

// Go: language/language.go:Make
/// Parses `s`, returning a sensible default on error.
pub fn make(s: &str) -> Tag {
    DEFAULT.make(s)
}

/// `language.English`.
pub fn english() -> Tag {
    Tag(ITag::core(c("_en"), 0, 0))
}

/// `language.Und`.
pub fn und() -> Tag {
    Tag::default()
}

impl Tag {
    // Go: language/language.go:Tag.String
    pub fn string(&self) -> String {
        self.0.string()
    }

    // Go: language/language.go:Tag.Raw
    /// Returns the raw base language, script and region, without making an
    /// attempt to infer their values.
    pub fn raw(&self) -> (Base, Script, Region) {
        (
            Base(self.0.lang_id),
            Script(self.0.script_id),
            Region(self.0.region_id),
        )
    }

    // Go: language/language.go:Tag.IsRoot
    pub fn is_root(&self) -> bool {
        self.0.is_root()
    }

    // Go: language/language.go:Tag.Base
    /// Returns the base language of the tag, inferring it (with a confidence)
    /// if unspecified.
    pub fn base(&self) -> (Base, Confidence) {
        let b = self.0.lang_id;
        if b != 0 {
            return (Base(b), Confidence::Exact);
        }
        let tt = &self.0;
        let mut conf = Confidence::High;
        if tt.script_id == 0 && !internal::region_is_country(tt.region_id) {
            conf = Confidence::Low;
        }
        let (tag, err) = tt.maximize();
        if err.is_none() && tag.lang_id != 0 {
            return (Base(tag.lang_id), conf);
        }
        (Base(0), Confidence::No)
    }

    // Go: language/language.go:Tag.Parent
    /// Returns the CLDR parent of t.
    pub fn parent(&self) -> Tag {
        Tag(compact::make(&self.0).parent().tag())
    }

    // Go: language/language.go:Tag.Extensions
    pub fn extensions(&self) -> Vec<Extension> {
        self.0.extensions().into_iter().map(Extension).collect()
    }

    // Go: language/language.go:Tag.Extension
    pub fn extension(&self, x: u8) -> Option<Extension> {
        self.0.extension(x).map(Extension)
    }

    // Go: language/language.go:Tag.TypeForKey
    /// Returns the type associated with the given key of the Unicode locale
    /// extension ('u'), or "".
    pub fn type_for_key(&self, key: &str) -> String {
        self.0.type_for_key(key)
    }

    // Go: language/language.go:Tag.SetTypeForKey
    /// Returns a new Tag with the key set to type; an empty value removes the
    /// key. Like Go, the (unchanged) tag is also returned with an error.
    pub fn set_type_for_key(&self, key: &str, value: &str) -> (Tag, Option<LangError>) {
        let (t, e) = self.0.set_type_for_key(key, value);
        (make_tag(t), e)
    }

    /// The compact index of the tag (Go `language.CompactIndex`).
    pub fn compact_index(&self) -> (usize, bool) {
        let (id, exact) = compact::make(&self.0).language_id();
        (id as usize, exact)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}
