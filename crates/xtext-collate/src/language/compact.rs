//! Port of golang.org/x/text@v0.26.0/internal/language/compact (compact.go,
//! language.go `FromTag`/`Make`/`Parent`), used for the equality and
//! parent semantics of the public `language.Tag`.
//!
//! A public Tag is modelled by the internal tag `compact.Tag.Tag()` returns
//! (`CTag::tag`): `compact.Make` of that tag gives back the same
//! (language, locale, full) triple, so methods that need the triple
//! (`Parent`, `CompactIndex`) recompute it with [`make`].

use std::sync::OnceLock;

use super::internal::{
    Builder, ITag, UND, compact_core_tag, get_compact_core, must_parse, parse_region, region_string,
};
use super::tables::tables;

fn special_tags() -> &'static [ITag] {
    static S: OnceLock<Vec<ITag>> = OnceLock::new();
    S.get_or_init(|| {
        tables()
            .special_tags_str
            .split(' ')
            .map(must_parse)
            .collect()
    })
}

// Go: internal/language/compact/compact.go:getCoreIndex
fn get_core_index(t: &ITag) -> Option<u16> {
    let cci = get_compact_core(t)?;
    let core = &tables().core_tags;
    let i = core.partition_point(|&x| x < cci);
    if i == core.len() || core[i] != cci {
        return None;
    }
    Some(i as u16)
}

// Go: internal/language/compact/compact.go:ID.Tag
pub(crate) fn id_tag(id: u16) -> ITag {
    let core = &tables().core_tags;
    if id as usize >= core.len() {
        return special_tags()[id as usize - core.len()].clone();
    }
    compact_core_tag(core[id as usize])
}

// Go: internal/language/compact/language.go:FromTag
/// Returns the compact id of `t` (or of its closest compact ancestor) and
/// whether the id represents `t` exactly.
pub(crate) fn from_tag(t: &ITag) -> (u16, bool) {
    let core_len = tables().core_tags.len();
    let mut exact = true;

    let (b, s, r) = t.raw();
    let mut t = t.clone();
    if t.has_string() {
        if t.is_private_use() {
            // We have no entries for user-defined tags.
            return (0, false);
        }
        let mut has_extra = false;
        if t.has_variants() {
            if t.has_extensions() {
                let mut build = Builder::default();
                build.set_tag(&ITag::core(b, s, r));
                let v = t.variants().to_string();
                build.add_variant(&[&v]);
                exact = false;
                t = build.make();
            }
            has_extra = true;
        } else if t.extension(b'u').is_some() {
            // Strip all but the 'va' entry.
            let old = t.clone();
            let variant = t.type_for_key("va");
            t = ITag::core(b, s, r);
            if !variant.is_empty() {
                t = t.set_type_for_key("va", &variant).0;
                has_extra = true;
            }
            exact = old == t;
        } else {
            exact = false;
        }
        if has_extra {
            // We have some variants.
            for (i, st) in special_tags().iter().enumerate() {
                if *st == t {
                    return ((i + core_len) as u16, exact);
                }
            }
            exact = false;
        }
    }
    if let Some(x) = get_core_index(&t) {
        return (x, exact);
    }
    exact = false;
    if r != 0 && s == 0 {
        // Deal with cases where an extra script is inserted for the region.
        let (tm, _) = t.maximize();
        if let Some(x) = get_core_index(&tm) {
            return (x, exact);
        }
    }
    t = t.parent();
    while t != UND {
        // No variants specified: just compare core components.
        if let Some(x) = get_core_index(&t) {
            return (x, exact);
        }
        t = t.parent();
    }
    (0, exact)
}

/// Go `compact.Tag`: a language id, a locale id (they differ only for
/// `-u-rg-XXzzzz` tags) and the full tag when the ids are not exact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CTag {
    language: u16,
    locale: u16,
    full: Option<ITag>,
}

// Go: internal/language/compact/language.go:Make
/// Makes a compact Tag from a fully specified internal language Tag.
pub(crate) fn make(t: &ITag) -> CTag {
    let region = t.type_for_key("rg");
    let rb = region.as_bytes();
    // Go: len(region) == 6 && region[2:] == "zzzz" (byte-wise).
    if rb.len() == 6
        && rb[2..] == *b"zzzz"
        && let Ok(r) = parse_region(&String::from_utf8_lossy(&rb[..2]))
    {
        let t_full = t.clone();
        let (mut t, _) = t.set_type_for_key("rg", "");
        // TODO: should we not consider "va" for the language tag?
        let (language, exact1) = from_tag(&t);
        // Go: t.RegionID = r (t.str keeps the old region).
        t.region_id = r;
        let (locale, exact2) = from_tag(&t);
        let full = if !exact1 || !exact2 {
            Some(t_full)
        } else {
            None
        };
        return CTag {
            language,
            locale,
            full,
        };
    }
    let (lang, ok) = from_tag(t);
    CTag {
        language: lang,
        locale: lang,
        full: if ok { None } else { Some(t.clone()) },
    }
}

impl CTag {
    // Go: internal/language/compact/language.go:Tag.Tag
    /// Returns an internal language Tag version of this tag.
    pub(crate) fn tag(&self) -> ITag {
        if let Some(f) = &self.full {
            return f.clone();
        }
        let mut tag = id_tag(self.language);
        if self.language != self.locale {
            let loc = id_tag(self.locale);
            let v = region_string(loc.region_id).to_ascii_lowercase() + "zzzz";
            tag = tag.set_type_for_key("rg", &v).0;
        }
        tag
    }

    // Go: internal/language/compact/language.go:Tag.Parent
    /// Returns the tag of the parent language.
    pub(crate) fn parent(&self) -> CTag {
        if let Some(f) = &self.full {
            return make(&f.parent());
        }
        if self.language != self.locale {
            // Simulate stripping -u-rg-xxxxxx
            return CTag {
                language: self.language,
                locale: self.language,
                full: None,
            };
        }
        // TODO: use parent lookup table once cycle from internal package is
        // removed. Probably by internalizing the table and declaring this
        // fast enough.
        let (lang, _) = from_tag(&id_tag(self.language).parent());
        CTag {
            language: lang,
            locale: lang,
            full: None,
        }
    }

    // Go: internal/language/compact/language.go:LanguageID
    /// The compact language id and whether it represents the tag exactly.
    pub(crate) fn language_id(&self) -> (u16, bool) {
        (self.language, self.full.is_none())
    }
}
