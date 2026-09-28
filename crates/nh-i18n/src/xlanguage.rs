//! Module `xlanguage`: the parts of `golang.org/x/text@v0.26.0/language` that go-i18n uses and
//! the `xtext-collate` crate does not export.
//!
//! Owner: Wave B task T17 (i18n).
//!
//! go-i18n resolves a localizer's language with x/text's `language.Matcher`
//! (`bundle.matcher.Match(l.tags...)`, whose returned INDEX picks the bundle tag) and parses
//! the localizer's languages with `language.ParseAcceptLanguage`. Tag parsing, canonicalization,
//! `Parent`, `Base` and `String` come from `xtext-collate` (a verified port of the same x/text
//! version); this module ports what that crate keeps private or does not port:
//!
//! - `language/match.go`: `NewMatcher`, `matcher.Match` (the index and confidence; the returned
//!   tag with its copied region/extensions is not built because go-i18n ignores it), `getBest`,
//!   `bestMatch.update`, `regionGroupDist`, `isParadigmLocale`, `equalsRest`,
//!   `isExactEquivalent`, `altScript`, `addIfNew`, the `init` of `notEquivalent` and
//!   `paradigmLocales`;
//! - `internal/language/match.go`: `addTags` (`Maximize`), `specializeRegion`;
//! - `internal/language/language.go`: `Region.Contains`, `Tag.VariantOrPrivateUseTags`,
//!   `Tag.IsPrivateUse`; `lookup.go`: `Language.SuppressScript`;
//! - `language/language.go`: `Tag.Script`;
//! - `language/parse.go`: `ParseAcceptLanguage`.
//!
//! Representation: x/text works on internal ids (`Language`, `Script`, `Region` = uint16). This
//! port works on the subtag strings instead ("" = id 0), which are in bijection with the ids
//! (`tools/go-oracle/nh-i18n/gentables` decodes every id with x/text's own `String()` and fails
//! on duplicates). The tables in `tables.rs` are generated from x/text's `tables.go`.

#[rustfmt::skip]
mod tables;

use std::collections::HashMap;
use std::sync::OnceLock;

pub use xtext_collate::language::Confidence;
use xtext_collate::language::{self as xl, CanonType, Tag};

// Table row types (the generated tables use them).

/// Go: `likelyScriptRegion`; `region_n`/`script_n` are the list index and size when
/// `flags & isList != 0`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LikelyScriptRegion {
    pub region: &'static str,
    pub script: &'static str,
    pub region_n: u16,
    pub script_n: u16,
    pub flags: u8,
}

/// Go: `likelyLangScript`; `lang_n`/`script_n` are the list index and size when
/// `flags & isList != 0`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LikelyLangScript {
    pub lang: &'static str,
    pub script: &'static str,
    pub lang_n: u16,
    pub script_n: u16,
    pub flags: u8,
}

/// Go: `likelyLangRegion`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LikelyLangRegion {
    pub lang: &'static str,
    pub region: &'static str,
}

/// Go: `likelyTag`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LikelyTag {
    pub lang: &'static str,
    pub region: &'static str,
    pub script: &'static str,
}

/// Go: `mutualIntelligibility`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MutualIntelligibility {
    pub want: &'static str,
    pub have: &'static str,
    pub distance: u8,
    pub oneway: bool,
}

/// Go: `scriptIntelligibility`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScriptIntelligibility {
    pub want_lang: &'static str,
    pub have_lang: &'static str,
    pub want_script: &'static str,
    pub have_script: &'static str,
    /// Part of Go's table; Go's matcher does not read it either.
    #[allow(dead_code)]
    pub distance: u8,
}

/// Go: `regionIntelligibility`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RegionIntelligibility {
    pub lang: &'static str,
    pub script: &'static str,
    pub group: u8,
    pub distance: u8,
}

// Go: internal/language/match.go scriptRegionFlags.
const IS_LIST: u8 = 1;
const SCRIPT_IN_FROM: u8 = 2;

fn lookup<T: Copy>(table: &[(&str, T)], key: &str) -> Option<T> {
    table
        .binary_search_by(|(k, _)| (*k).cmp(key))
        .ok()
        .map(|i| table[i].1)
}

const ZERO_LANG_SCRIPT: LikelyLangScript = LikelyLangScript {
    lang: "",
    script: "",
    lang_n: 0,
    script_n: 0,
    flags: 0,
};

/// The internal-tag view the matcher works on: Go `internal/language.Tag` reduced to what
/// matching reads (ids as subtag strings, the variant/private-use part and `IsPrivateUse`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ITag {
    pub lang: String,
    pub script: String,
    pub region: String,
    /// Go `VariantOrPrivateUseTags()`.
    pub variants: String,
    /// Go `IsPrivateUse()` (a tag that is only a private use tag, `x-…`).
    pub private_use: bool,
}

impl ITag {
    /// Go: `t.tag()` of a public tag, as the fields matching reads.
    pub fn from_tag(t: &Tag) -> ITag {
        let (b, s, r) = t.raw();
        let str_ = t.string();
        let private_use = str_.starts_with("x-");
        ITag {
            lang: if b == Default::default() {
                String::new()
            } else {
                b.to_string()
            },
            script: if s == Default::default() {
                String::new()
            } else {
                s.to_string()
            },
            region: if r == Default::default() {
                String::new()
            } else {
                r.to_string()
            },
            variants: variant_or_private_use_tags(&str_, private_use),
            private_use,
        }
    }
}

// Go: internal/language/language.go:VariantOrPrivateUseTags
/// `t.str[t.pVariant:t.pExt]` computed from the tag's canonical string: the variant subtags
/// (with their leading `-`), or the whole string of a private-use-only tag.
fn variant_or_private_use_tags(s: &str, private_use: bool) -> String {
    if private_use {
        return s.to_string();
    }
    let toks: Vec<&str> = s.split('-').collect();
    let mut i = 1;
    if i < toks.len() && toks[i].len() == 4 && toks[i].bytes().all(|b| b.is_ascii_alphabetic()) {
        i += 1;
    }
    if i < toks.len()
        && ((toks[i].len() == 2 && toks[i].bytes().all(|b| b.is_ascii_alphabetic()))
            || (toks[i].len() == 3 && toks[i].bytes().all(|b| b.is_ascii_digit())))
    {
        i += 1;
    }
    let mut out = String::new();
    while i < toks.len() && toks[i].len() > 1 {
        out.push('-');
        out.push_str(toks[i]);
        i += 1;
    }
    out
}

// Go: internal/language/match.go:setUndefinedLang
fn set_undefined_lang(t: &mut ITag, id: &str) {
    if t.lang.is_empty() {
        t.lang = id.to_string();
    }
}

// Go: internal/language/match.go:setUndefinedScript
fn set_undefined_script(t: &mut ITag, id: &str) {
    if t.script.is_empty() {
        t.script = id.to_string();
    }
}

// Go: internal/language/match.go:setUndefinedRegion
fn set_undefined_region(t: &mut ITag, id: &str) {
    if t.region.is_empty() || region_contains(&t.region, id) {
        t.region = id.to_string();
    }
}

fn region_inclusion(r: &str) -> u8 {
    lookup(tables::REGION_INCLUSION, r).expect("every region id is in regionInclusion")
}

// Go: internal/language/language.go:Region.Contains
/// Whether the region `r` contains the region `c`.
pub fn region_contains(r: &str, c: &str) -> bool {
    if r == c {
        return true;
    }
    let g = region_inclusion(r);
    if g >= tables::N_REGION_GROUPS {
        return false;
    }
    let m = tables::REGION_CONTAINMENT[g as usize];

    let d = region_inclusion(c);
    let b = tables::REGION_INCLUSION_BITS[d as usize];

    // A contained country may belong to multiple disjoint groups. Matching any
    // of these indicates containment. If the contained region is a group, it
    // must strictly be a subset.
    if d >= tables::N_REGION_GROUPS {
        return b & m != 0;
    }
    b & !m == 0
}

// Go: internal/language/match.go:specializeRegion
fn specialize_region(t: &mut ITag) -> bool {
    let i = region_inclusion(&t.region);
    if i < tables::N_REGION_GROUPS {
        let x = tables::LIKELY_REGION_GROUP[i as usize];
        if x.lang == t.lang && x.script == t.script {
            t.region = x.region.to_string();
        }
        return true;
    }
    false
}

/// Go: `ErrMissingLikelyTagsData`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrMissingLikelyTagsData;

/// Go: `Tag.Maximize()` — the tag with missing subtags filled in (`addTags`).
// Go: internal/language/match.go:Maximize
pub fn maximize(t: &ITag) -> (ITag, Option<ErrMissingLikelyTagsData>) {
    add_tags(t.clone())
}

// Go: internal/language/match.go:addTags
fn add_tags(mut t: ITag) -> (ITag, Option<ErrMissingLikelyTagsData>) {
    // We leave private use identifiers alone.
    if t.private_use {
        return (t, None);
    }
    if !t.script.is_empty() && !t.region.is_empty() {
        if !t.lang.is_empty() {
            // already fully specified
            specialize_region(&mut t);
            return (t, None);
        }
        // Search matches for und-script-region. Note that for these cases
        // region will never be a group so there is no need to check for this.
        let x = lookup(tables::LIKELY_REGION, &t.region).unwrap_or(ZERO_LANG_SCRIPT);
        let list: Vec<LikelyLangScript> = if x.flags & IS_LIST != 0 {
            tables::LIKELY_REGION_LIST[x.lang_n as usize..(x.lang_n + x.script_n) as usize].to_vec()
        } else {
            vec![x]
        };
        for x in list {
            // Deviating from the spec. See match_test.go for details.
            if x.script == t.script {
                set_undefined_lang(&mut t, x.lang);
                return (t, None);
            }
        }
    }
    if !t.lang.is_empty() {
        // Search matches for lang-script and lang-region, where lang != und.
        if let Some(x) = lookup(tables::LIKELY_LANG, &t.lang)
            && x.flags & IS_LIST != 0
        {
            let list =
                &tables::LIKELY_LANG_LIST[x.region_n as usize..(x.region_n + x.script_n) as usize];
            if !t.script.is_empty() {
                for x in list {
                    if x.script == t.script && x.flags & SCRIPT_IN_FROM != 0 {
                        set_undefined_region(&mut t, x.region);
                        return (t, None);
                    }
                }
            } else if !t.region.is_empty() {
                let mut count = 0;
                let mut good_script = true;
                let mut tt = t.clone();
                for x in list {
                    // We visit all entries for which the script was not
                    // defined, including the ones where the region was not
                    // defined. This allows for proper disambiguation within
                    // regions.
                    if x.flags & SCRIPT_IN_FROM == 0 && region_contains(&t.region, x.region) {
                        tt.region = x.region.to_string();
                        set_undefined_script(&mut tt, x.script);
                        good_script = good_script && tt.script == x.script;
                        count += 1;
                    }
                }
                if count == 1 {
                    return (tt, None);
                }
                // Even if we fail to find a unique Region, we might have
                // an unambiguous script.
                if good_script {
                    t.script = tt.script;
                }
            }
        }
    } else {
        // Search matches for und-script.
        if !t.script.is_empty()
            && let Some(x) = lookup(tables::LIKELY_SCRIPT, &t.script)
            && !x.region.is_empty()
        {
            set_undefined_region(&mut t, x.region);
            set_undefined_lang(&mut t, x.lang);
            return (t, None);
        }
        // Search matches for und-region. If und-script-region exists, it would
        // have been found earlier.
        if !t.region.is_empty() {
            let i = region_inclusion(&t.region);
            if i < tables::N_REGION_GROUPS {
                let x = tables::LIKELY_REGION_GROUP[i as usize];
                if !x.region.is_empty() {
                    set_undefined_lang(&mut t, x.lang);
                    set_undefined_script(&mut t, x.script);
                    t.region = x.region.to_string();
                }
            } else {
                let mut x = lookup(tables::LIKELY_REGION, &t.region).unwrap_or(ZERO_LANG_SCRIPT);
                if x.flags & IS_LIST != 0 {
                    x = tables::LIKELY_REGION_LIST[x.lang_n as usize];
                }
                if !x.script.is_empty() && x.flags != SCRIPT_IN_FROM {
                    set_undefined_lang(&mut t, x.lang);
                    set_undefined_script(&mut t, x.script);
                    return (t, None);
                }
            }
        }
    }

    // Search matches for lang.
    if let Some(mut x) = lookup(tables::LIKELY_LANG, &t.lang) {
        if x.flags & IS_LIST != 0 {
            x = tables::LIKELY_LANG_LIST[x.region_n as usize];
        }
        if !x.region.is_empty() {
            set_undefined_script(&mut t, x.script);
            set_undefined_region(&mut t, x.region);
        }
        specialize_region(&mut t);
        if t.lang.is_empty() {
            t.lang = "en".to_string(); // default language
        }
        return (t, None);
    }
    (t, Some(ErrMissingLikelyTagsData))
}

// Go: internal/language/lookup.go:SuppressScript
fn suppress_script(lang: &str) -> &'static str {
    lookup(tables::SUPPRESS_SCRIPT, lang).unwrap_or("")
}

fn canon(c: u32, t: &Tag) -> ITag {
    ITag::from_tag(&CanonType(c).canonicalize(t))
}

// Go: language/language.go CanonType bits.
const DEPRECATED: u32 = xl::DEPRECATED.0;
const MACRO: u32 = xl::MACRO.0;
const LEGACY: u32 = xl::LEGACY.0;
const ALL: u32 = xl::ALL.0;

/// Go: `Tag.Script()` — the script of the tag, inferred with a confidence when unspecified.
/// Returns the script subtag string ("" = id 0; an unknown script is Go's `Zzzz`).
// Go: language/language.go:Script
pub fn tag_script(t: &Tag) -> (String, Confidence) {
    let tt = ITag::from_tag(t);
    if !tt.script.is_empty() {
        return (tt.script, Confidence::Exact);
    }
    let (mut sc, mut c) = ("Zzzz".to_string(), Confidence::No);
    let scr = suppress_script(&tt.lang);
    if !scr.is_empty() {
        // Note: it is not always the case that a language with a suppress
        // script value is only written in one script (e.g. kk, ms, pa).
        if tt.region.is_empty() {
            return (scr.to_string(), Confidence::High);
        }
        sc = scr.to_string();
        c = Confidence::High;
    }
    match maximize(&tt) {
        (tag, None) => {
            if tag.script != sc {
                sc = tag.script;
                c = Confidence::Low;
            }
        }
        (_, Some(_)) => {
            let tt = canon(DEPRECATED | MACRO, t);
            if let (tag, None) = maximize(&tt)
                && tag.script != sc
            {
                sc = tag.script;
                c = Confidence::Low;
            }
        }
    }
    (sc, c)
}

/// Go: `language.Matcher` as returned by `NewMatcher` (the `matcher` type).
#[derive(Clone, Debug)]
pub struct Matcher {
    default_: Option<HaveTag>,
    supported: Vec<HaveTag>,
    index: HashMap<String, MatchHeader>,
    prefer_same_script: bool,
}

/// Go: `matchHeader`.
#[derive(Clone, Debug, Default)]
struct MatchHeader {
    have_tags: Vec<HaveTag>,
    original: bool,
}

/// Go: `haveTag`.
#[derive(Clone, Debug)]
struct HaveTag {
    tag: ITag,
    /// index of this tag in the original list of supported tags.
    index: usize,
    /// the maximum confidence that can result from matching this haveTag.
    conf: Confidence,
    max_region: String,
    max_script: String,
    alt_script: String,
    /// index of the next haveTag with the same maximized tags.
    next_max: u16,
}

// Go: language/match.go:makeHaveTag
fn make_have_tag(tag: &Tag, index: usize) -> (HaveTag, String) {
    let t = ITag::from_tag(tag);
    let mut max = t.clone();
    if !t.lang.is_empty() || !t.region.is_empty() || !t.script.is_empty() {
        max = canon(ALL, tag);
        max = maximize(&max).0;
    }
    let alt = alt_script(&max.lang, &max.script);
    (
        HaveTag {
            tag: t,
            index,
            conf: Confidence::Exact,
            max_region: max.region,
            max_script: max.script,
            alt_script: alt.to_string(),
            next_max: 0,
        },
        max.lang,
    )
}

// Go: language/match.go:altScript
/// An alternative script that may match the given script with a low confidence.
fn alt_script(l: &str, s: &str) -> &'static str {
    for alt in tables::MATCH_SCRIPT {
        // TODO: also match cases where language is not the same.
        if (alt.want_lang == l || alt.have_lang == l) && alt.have_script == s {
            return alt.want_script;
        }
    }
    ""
}

impl MatchHeader {
    // Go: language/match.go:addIfNew
    /// Adds a haveTag to the list of tags only if it is a unique tag. Tags that have the same
    /// maximized values are linked by index.
    fn add_if_new(&mut self, n: HaveTag, exact: bool) {
        self.original = self.original || exact;
        // Don't add new exact matches.
        for v in &self.have_tags {
            if equals_rest(&v.tag, &n.tag) {
                return;
            }
        }
        // Allow duplicate maximized tags, but create a linked list to allow quickly
        // comparing the equivalents and bail out.
        for i in 0..self.have_tags.len() {
            let v = &self.have_tags[i];
            if v.max_script == n.max_script
                && v.max_region == n.max_region
                && v.tag.variants == n.tag.variants
            {
                let mut i = i;
                while self.have_tags[i].next_max != 0 {
                    i = self.have_tags[i].next_max as usize;
                }
                self.have_tags[i].next_max = self.have_tags.len() as u16;
                break;
            }
        }
        self.have_tags.push(n);
    }
}

// Go: language/match.go:toConf
fn to_conf(d: u8) -> Confidence {
    if d <= 10 {
        return Confidence::High;
    }
    if d < 30 {
        return Confidence::Low;
    }
    Confidence::No
}

/// Go `notEquivalent` and the maximized `paradigmLocales` (language/match.go `init`).
struct MatchInit {
    not_equivalent: Vec<&'static str>,
    paradigm_locales: Vec<[String; 3]>,
}

fn match_init() -> &'static MatchInit {
    static INIT: OnceLock<MatchInit> = OnceLock::new();
    INIT.get_or_init(|| {
        // Go: language/match.go:init
        // Create a list of all languages for which canonicalization may alter the
        // script or region.
        let mut not_equivalent = Vec::new();
        for &(from, _, _) in tables::ALIAS_MAP {
            let tag = CanonType(0).make(from);
            let t = canon(ALL, &tag);
            if !t.script.is_empty() || !t.region.is_empty() {
                not_equivalent.push(from);
            }
        }
        // Maximize undefined regions of paradigm locales.
        let mut paradigm_locales = Vec::new();
        for &(l, r1, r2) in tables::PARADIGM_LOCALES {
            let t = ITag {
                lang: l.to_string(),
                ..Default::default()
            };
            let (max, _) = maximize(&t);
            let mut v = [l.to_string(), r1.to_string(), r2.to_string()];
            if v[1].is_empty() {
                v[1] = max.region.clone();
            }
            if v[2].is_empty() {
                v[2] = max.region.clone();
            }
            paradigm_locales.push(v);
        }
        MatchInit {
            not_equivalent,
            paradigm_locales,
        }
    })
}

impl Matcher {
    fn header(&mut self, l: &str) -> &mut MatchHeader {
        self.index.entry(l.to_string()).or_default()
    }

    /// Go: `NewMatcher(t)` — matches preferred tags against the supported tags; the first
    /// supported tag is the default.
    // Go: language/match.go:newMatcher
    pub fn new(supported: &[Tag]) -> Matcher {
        let mut m = Matcher {
            default_: None,
            supported: Vec::new(),
            index: HashMap::new(),
            prefer_same_script: true,
        };
        if supported.is_empty() {
            return m;
        }
        // Add supported languages to the index. Add exact matches first to give
        // them precedence.
        for (i, tag) in supported.iter().enumerate() {
            let tt = ITag::from_tag(tag);
            let (pair, _) = make_have_tag(tag, i);
            m.header(&tt.lang).add_if_new(pair.clone(), true);
            m.supported.push(pair);
        }
        let l0 = ITag::from_tag(&supported[0]).lang;
        m.default_ = Some(m.header(&l0).have_tags[0].clone());
        // Keep these in two different loops to support the case that two equivalent
        // languages are distinguished, such as iw and he.
        for (i, tag) in supported.iter().enumerate() {
            let tt = ITag::from_tag(tag);
            let (pair, max) = make_have_tag(tag, i);
            if max != tt.lang {
                m.header(&max).add_if_new(pair, true);
            }
        }

        // Add entries for languages with mutual intelligibility as defined by CLDR's
        // languageMatch data.
        for ml in tables::MATCH_LANG {
            m.update(ml.want, ml.have, to_conf(ml.distance));
            if !ml.oneway {
                m.update(ml.have, ml.want, to_conf(ml.distance));
            }
        }

        // Add entries for possible canonicalizations. This is an optimization to
        // ensure that only one map lookup needs to be done at runtime per desired tag.
        // First we match deprecated equivalents. If they are perfect equivalents
        // (their canonicalization simply substitutes a different language code, but
        // nothing else), the match confidence is Exact, otherwise it is High.
        for &(from, to, typ) in tables::ALIAS_MAP {
            // If deprecated codes match and there is no fiddling with the script
            // or region, we consider it an exact match.
            let mut conf = Confidence::Exact;
            if typ != 1 {
                // AliasTypes[i] != language.Macro
                if !is_exact_equivalent(from) {
                    conf = Confidence::High;
                }
                m.update(to, from, conf);
            }
            m.update(from, to, conf);
        }
        m
    }

    /// Go: the `update` closure of `newMatcher` — adds index entries for equivalent
    /// languages (only to original indexes, no transitive relations).
    fn update(&mut self, want: &str, have: &str, conf: Confidence) {
        let Some(hh) = self.index.get(have) else {
            return;
        };
        if !hh.original {
            return;
        }
        let hh_original = hh.original;
        let have_tags = hh.have_tags.clone();
        let hw = self.header(want);
        for ht in have_tags {
            let mut v = ht;
            if conf < v.conf {
                v.conf = conf;
            }
            v.next_max = 0; // this value needs to be recomputed
            if !v.alt_script.is_empty() {
                v.alt_script = alt_script(want, &v.max_script).to_string();
            }
            hw.add_if_new(v, conf == Confidence::Exact && hh_original);
        }
    }

    /// Go: `matcher.Match(want...)` — the index of the best supported tag and the confidence.
    /// (Go also returns the matched tag, augmented with the wanted region and extensions;
    /// go-i18n discards it, so it is not built here.)
    // Go: language/match.go:Match
    pub fn match_tags(&self, want: &[Tag]) -> (usize, Confidence) {
        let (m, _w, c) = self.get_best(want);
        if let Some(m) = m {
            return (m.index, c);
        }
        let default_ = match &self.default_ {
            Some(d) => d,
            // Go: `m.default_ = &haveTag{}` for an empty matcher.
            None => return (0, c),
        };
        let mut index = default_.index;
        if self.prefer_same_script {
            'outer: for w in want {
                let (script, _) = tag_script(w);
                if script.is_empty() {
                    // Don't do anything if there is no script, such as with
                    // private subtags.
                    continue;
                }
                for (i, h) in self.supported.iter().enumerate() {
                    if script == h.max_script {
                        index = i;
                        break 'outer;
                    }
                }
            }
        }
        (index, c)
    }

    // Go: language/match.go:getBest
    fn get_best(&self, want: &[Tag]) -> (Option<&HaveTag>, ITag, Confidence) {
        let mut best = BestMatch::new();
        for (i, ww) in want.iter().enumerate() {
            let mut w = ITag::from_tag(ww);
            let max;
            // Check for exact match first.
            let mut h = self.index.get(&w.lang);
            if !w.lang.is_empty() {
                let Some(_) = h else {
                    continue;
                };
                // Base language is defined.
                let cmax = canon(LEGACY | DEPRECATED | MACRO, ww);
                // A region that is added through canonicalization is stronger than
                // a maximized region: set it in the original (e.g. mo -> ro-MD).
                if w.region != cmax.region {
                    w.region = cmax.region.clone();
                }
                // TODO: should we do the same for scripts?
                // See test case: en, sr, nl ; sh ; sr
                max = maximize(&cmax).0;
            } else {
                // Base language is not defined.
                if let Some(h) = h {
                    for have in &h.have_tags {
                        if equals_rest(&have.tag, &w) {
                            return (Some(have), w, Confidence::Exact);
                        }
                    }
                }
                if w.script.is_empty() && w.region.is_empty() {
                    // We skip all tags matching und for approximate matching, including
                    // private tags.
                    continue;
                }
                max = maximize(&w).0;
                h = self.index.get(&max.lang);
                if h.is_none() {
                    continue;
                }
            }
            let h = h.expect("checked");
            let mut pin = true;
            for t in &want[i + 1..] {
                if w.lang == ITag::from_tag(t).lang {
                    pin = false;
                    break;
                }
            }
            // Check for match based on maximized tag.
            for have_i in 0..h.have_tags.len() {
                let mut have = have_i;
                best.update(
                    h,
                    have,
                    &w,
                    &max.script,
                    &max.region,
                    pin,
                    &match_init().paradigm_locales,
                );
                if best.conf == Confidence::Exact {
                    while h.have_tags[have].next_max != 0 {
                        have = h.have_tags[have].next_max as usize;
                        best.update(
                            h,
                            have,
                            &w,
                            &max.script,
                            &max.region,
                            pin,
                            &match_init().paradigm_locales,
                        );
                    }
                    let got = best.have.map(|(hh, idx)| &hh.have_tags[idx]);
                    return (got, best.want.clone(), best.conf);
                }
            }
        }
        if best.conf <= Confidence::No {
            if !want.is_empty() {
                return (None, ITag::from_tag(&want[0]), Confidence::No);
            }
            return (None, ITag::default(), Confidence::No);
        }
        let got = best.have.map(|(hh, idx)| &hh.have_tags[idx]);
        (got, best.want.clone(), best.conf)
    }
}

/// Go: `bestMatch` — the best match so far.
struct BestMatch<'a> {
    have: Option<(&'a MatchHeader, usize)>,
    want: ITag,
    conf: Confidence,
    pinned_region: String,
    pin_language: bool,
    same_region_group: bool,
    // Cached results from applying tie-breaking rules.
    orig_lang: bool,
    orig_reg: bool,
    paradigm_reg: bool,
    reg_group_dist: u8,
    orig_script: bool,
}

impl<'a> BestMatch<'a> {
    fn new() -> BestMatch<'a> {
        BestMatch {
            have: None,
            want: ITag::default(),
            conf: Confidence::No,
            pinned_region: String::new(),
            pin_language: false,
            same_region_group: false,
            orig_lang: false,
            orig_reg: false,
            paradigm_reg: false,
            reg_group_dist: 0,
            orig_script: false,
        }
    }

    // Go: language/match.go:update
    /// Updates the existing best match if the new pair is considered to be a better match.
    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        h: &'a MatchHeader,
        have_idx: usize,
        tag: &ITag,
        max_script: &str,
        max_region: &str,
        pin: bool,
        paradigm_locales: &[[String; 3]],
    ) {
        let have = &h.have_tags[have_idx];
        // Bail if the maximum attainable confidence is below that of the current best match.
        let mut c = have.conf;
        if c < self.conf {
            return;
        }
        // Don't change the language once we already have found an exact match.
        if self.pin_language && tag.lang != self.want.lang {
            return;
        }
        // Pin the region group if we are comparing tags for the same language.
        if tag.lang == self.want.lang && self.same_region_group {
            let (_, same_group) = region_group_dist(
                &self.pinned_region,
                &have.max_region,
                &have.max_script,
                &self.want.lang,
            );
            if !same_group {
                return;
            }
        }
        if c == Confidence::Exact && have.max_script == max_script {
            // If there is another language and then another entry of this language,
            // don't pin anything, otherwise pin the language.
            self.pin_language = pin;
        }
        if equals_rest(&have.tag, tag) {
        } else if have.max_script != max_script {
            // There is usually very little comprehension between different scripts.
            // In a few cases there may still be Low comprehension. This possibility
            // is pre-computed and stored in have.altScript.
            if Confidence::Low < self.conf || have.alt_script != max_script {
                return;
            }
            c = Confidence::Low;
        } else if have.max_region != max_region && Confidence::High < c {
            // There is usually a small difference between languages across regions.
            c = Confidence::High;
        }

        // We store the results of the computations of the tie-breaker rules along
        // with the best match. There is no need to do the checks once we determine
        // we have a winner, but we do still need to do the tie-breaker computations.
        // We use "beaten" to keep track if we still need to do the checks.
        let mut beaten = false; // true if the new pair defeats the current one.
        if c != self.conf {
            if c < self.conf {
                return;
            }
            beaten = true;
        }

        // Tie-breaker rules:
        // We prefer if the pre-maximized language was specified and identical.
        let orig_lang = have.tag.lang == tag.lang && !tag.lang.is_empty();
        if !beaten && self.orig_lang != orig_lang {
            if self.orig_lang {
                return;
            }
            beaten = true;
        }

        // We prefer if the pre-maximized region was specified and identical.
        let orig_reg = have.tag.region == tag.region && !tag.region.is_empty();
        if !beaten && self.orig_reg != orig_reg {
            if self.orig_reg {
                return;
            }
            beaten = true;
        }

        let (reg_group_dist, same_group) =
            region_group_dist(&have.max_region, max_region, max_script, &tag.lang);
        if !beaten && self.reg_group_dist != reg_group_dist {
            if reg_group_dist > self.reg_group_dist {
                return;
            }
            beaten = true;
        }

        let paradigm_reg = is_paradigm_locale(&tag.lang, &have.max_region, paradigm_locales);
        if !beaten && self.paradigm_reg != paradigm_reg {
            if !paradigm_reg {
                return;
            }
            beaten = true;
        }

        // Next we prefer if the pre-maximized script was specified and identical.
        let orig_script = have.tag.script == tag.script && !tag.script.is_empty();
        if !beaten && self.orig_script != orig_script {
            if self.orig_script {
                return;
            }
            beaten = true;
        }

        // Update m to the newly found best match.
        if beaten {
            self.have = Some((h, have_idx));
            self.want = tag.clone();
            self.conf = c;
            self.pinned_region = max_region.to_string();
            self.same_region_group = same_group;
            self.orig_lang = orig_lang;
            self.orig_reg = orig_reg;
            self.paradigm_reg = paradigm_reg;
            self.orig_script = orig_script;
            self.reg_group_dist = reg_group_dist;
        }
    }
}

// Go: language/match.go:isParadigmLocale
fn is_paradigm_locale(lang: &str, r: &str, paradigm_locales: &[[String; 3]]) -> bool {
    for e in paradigm_locales {
        if e[0] == lang && (r == e[1] || r == e[2]) {
            return true;
        }
    }
    false
}

// Go: language/match.go:regionGroupDist
/// The distance between two regions based on their CLDR grouping.
fn region_group_dist(a: &str, b: &str, script: &str, lang: &str) -> (u8, bool) {
    const DEFAULT_DISTANCE: u8 = 4;

    let group_of = |r: &str| -> u32 {
        (lookup(tables::REGION_TO_GROUPS, r).expect("every region id is in regionToGroups") as u32)
            << 1
    };
    let a_group = group_of(a);
    let b_group = group_of(b);
    for ri in tables::MATCH_REGION {
        if ri.lang == lang && (ri.script.is_empty() || ri.script == script) {
            let group: u32 = 1 << (ri.group & !0x80);
            if 0x80 & ri.group == 0 {
                if a_group & b_group & group != 0 {
                    // Both regions are in the group.
                    return (ri.distance, ri.distance == DEFAULT_DISTANCE);
                }
            } else if (a_group | b_group) & group == 0 {
                // Both regions are not in the group.
                return (ri.distance, ri.distance == DEFAULT_DISTANCE);
            }
        }
    }
    (DEFAULT_DISTANCE, true)
}

// Go: language/match.go:equalsRest
/// Compares everything except the language.
fn equals_rest(a: &ITag, b: &ITag) -> bool {
    // TODO: don't include extensions in this comparison. To do this efficiently,
    // though, we should handle private tags separately.
    a.script == b.script && a.region == b.region && a.variants == b.variants
}

// Go: language/match.go:isExactEquivalent
/// Whether canonicalizing the language will not alter the script or region of a tag.
fn is_exact_equivalent(l: &str) -> bool {
    for o in &match_init().not_equivalent {
        if *o == l {
            return false;
        }
    }
    true
}

/// Go: `language.ParseAcceptLanguage` error values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcceptLanguageError {
    /// `errTagListTooLarge`.
    TagListTooLarge,
    /// `errInvalidWeight`.
    InvalidWeight,
    /// A `language.Parse` error (or `ErrSyntax` after a recovered panic).
    Parse(String),
}

impl std::fmt::Display for AcceptLanguageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcceptLanguageError::TagListTooLarge => f.write_str("tag list exceeds max length"),
            AcceptLanguageError::InvalidWeight => {
                f.write_str("ParseAcceptLanguage: invalid weight")
            }
            AcceptLanguageError::Parse(s) => f.write_str(s),
        }
    }
}

// Go: language/parse.go:ParseAcceptLanguage
/// Parses an Accept-Language header: tags sorted by highest weight first, then by first
/// occurrence; zero weights dropped.
pub fn parse_accept_language(s: &str) -> Result<(Vec<Tag>, Vec<f32>), AcceptLanguageError> {
    if s.matches('-').count() > 1000 {
        return Err(AcceptLanguageError::TagListTooLarge);
    }

    let mut tag: Vec<Tag> = Vec::new();
    let mut q: Vec<f32> = Vec::new();
    let mut s = s;
    while !s.is_empty() {
        let (entry, rest) = split(s, b',');
        s = rest;
        if entry.is_empty() {
            continue;
        }

        let (entry, weight) = split(entry, b';');

        // Scan the language.
        let t = match xl::parse(entry) {
            Ok(t) => t,
            Err(err) => {
                // Add hack mapping to deal with a small number of cases that occur
                // in Accept-Language (with reasonable frequency).
                let id = match entry {
                    "english" => "en",
                    "deutsch" => "de",
                    "italian" => "it",
                    "french" => "fr",
                    "*" => "mul", // defined in the spec to match all languages.
                    _ => return Err(AcceptLanguageError::Parse(err.to_string())),
                };
                CanonType(0).make(id)
            }
        };

        // Scan the optional weight.
        let mut w: f64 = 1.0;
        if !weight.is_empty() {
            let weight = consume(weight, b'q');
            let weight = consume(weight, b'=');
            // consume returns the empty string when a token could not be
            // consumed, resulting in an error for ParseFloat.
            match go_strconv::parse_float(weight, 32) {
                Ok(v) => w = v,
                Err(_) => return Err(AcceptLanguageError::InvalidWeight),
            }
            // Drop tags with a quality weight of 0.
            if w <= 0.0 {
                continue;
            }
        }

        tag.push(t);
        q.push(w as f32);
    }
    let mut pairs: Vec<(Tag, f32)> = tag.into_iter().zip(q).collect();
    go_sort::stable_by(&mut pairs, |a, b| a.1 > b.1);
    Ok(pairs.into_iter().unzip())
}

// Go: language/parse.go:consume
fn consume(s: &str, c: u8) -> &str {
    if s.is_empty() || s.as_bytes()[0] != c {
        return "";
    }
    go_trim_space(&s[1..])
}

// Go: language/parse.go:split
fn split(s: &str, c: u8) -> (&str, &str) {
    if let Some(i) = s.as_bytes().iter().position(|&b| b == c) {
        return (go_trim_space(&s[..i]), go_trim_space(&s[i + 1..]));
    }
    (go_trim_space(s), "")
}

/// Go `strings.TrimSpace`.
fn go_trim_space(s: &str) -> &str {
    go_unicode::strings::trim_space_str(s)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (golang.org/x/text@v0.26.0; only what go-i18n reaches and xtext-collate
// does not export)
// OK language/match.go newMatcher (NewMatcher, options: PreferSameScript default true)
// OK language/match.go matcher.Match (index + confidence; the result tag is not built)
// OK language/match.go makeHaveTag
// OK language/match.go altScript
// OK language/match.go matchHeader.addIfNew
// OK language/match.go matcher.header
// OK language/match.go toConf
// OK language/match.go matcher.getBest
// OK language/match.go bestMatch.update
// OK language/match.go isParadigmLocale
// OK language/match.go regionGroupDist
// OK language/match.go equalsRest
// OK language/match.go isExactEquivalent
// OK language/match.go init (notEquivalent, paradigmLocales)
//    language/match.go MatchStrings, Comprehends (not used by go-i18n)
// OK language/language.go Tag.Script
// OK language/parse.go ParseAcceptLanguage, consume, split, acceptFallback, tagSort (go_sort::stable_by)
// OK internal/language/match.go setUndefinedLang, setUndefinedScript, setUndefinedRegion
// OK internal/language/match.go specializeRegion
// OK internal/language/match.go Maximize, addTags
//    internal/language/match.go addLikelySubtags, minimize, minimizeTags (not used by matching)
// OK internal/language/language.go Region.Contains
// OK internal/language/language.go Tag.VariantOrPrivateUseTags, Tag.IsPrivateUse (from the tag string)
// OK internal/language/lookup.go Language.SuppressScript
// OK internal/language/tables.go, language/tables.go (generated: gentables -> xlanguage/tables.rs)
// ---------------------------------------------------------------------------
