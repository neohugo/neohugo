//! Port of golang.org/x/text@v0.26.0/internal/colltab.

pub mod collelem;
pub mod contract;
pub mod iter;
pub mod numeric;
pub mod table;
pub mod trie;
pub mod weighter;

#[cfg(test)]
mod tests;

use crate::language::{self, Base, Confidence, Part, Region, Script, Tag};

// Go: internal/colltab/colltab.go:MatchLang
/// Finds the index of t in tags, using a matching algorithm used for
/// collation and search. tags[0] must be language.Und, the remaining tags
/// should be sorted alphabetically.
///
/// Language matching for collation and search is different from the matching
/// defined by language.Matcher: the (inferred) base language must be an
/// exact match for the relevant fields. For example, "gsw" should not match
/// "de". Also the parent relation is different, as a parent may have a
/// different script. So usually the parent of zh-Hant is und, whereas for
/// MatchLang it is zh.
pub fn match_lang(t: &Tag, tags: &[Tag]) -> usize {
    // Canonicalize the values, including collapsing macro languages.
    let t = language::ALL.canonicalize(t);

    let (base, conf) = t.base();
    // Estimate the base language, but only use high-confidence values.
    if conf < Confidence::High {
        // The root locale supports "search" and "standard". We assume that
        // any implementation will only use one of both.
        return 0;
    }

    // Maximize base and script and normalize the tag.
    let (_, s, r) = t.raw();
    let t = if r != Region::default() {
        let (p, _) = language::RAW.compose(&[Part::Base(base), Part::Script(s), Part::Region(r)]);
        // Taking the parent forces the script to be maximized.
        let p = p.parent();
        // Add back region and extensions.
        language::RAW
            .compose(&[
                Part::Tag(p),
                Part::Region(r),
                Part::Extensions(t.extensions()),
            ])
            .0
    } else {
        // Set the maximized base language.
        language::RAW
            .compose(&[
                Part::Base(base),
                Part::Script(s),
                Part::Extensions(t.extensions()),
            ])
            .0
    };

    // Find start index of the language tag.
    let base_str = base.to_string();
    let start = 1 + {
        // sort.Search(len(tags)-1, func(i int) bool { base.String() <= tags[i+1].base.String() })
        let (mut lo, mut hi) = (0usize, tags.len() - 1);
        while lo < hi {
            let h = lo + (hi - lo) / 2;
            let (b, _, _) = tags[h + 1].raw();
            if !(base_str <= b.to_string()) {
                lo = h + 1;
            } else {
                hi = h;
            }
        }
        lo
    };
    if start < tags.len() {
        let (b, _, _) = tags[start].raw();
        if b != base {
            return 0;
        }
    }

    // Besides the base language, script and region, only the collation type
    // and the custom variant defined in the 'u' extension are used to
    // distinguish a locale.
    // Strip all variants and extensions and add back the custom variant.
    let (rb, rs, rr) = t.raw();
    let (tdef, _) = language::RAW.compose(&[Part::Base(rb), Part::Script(rs), Part::Region(rr)]);
    let (tdef, _) = tdef.set_type_for_key("va", &t.type_for_key("va"));

    // First search for a specialized collation type, if present.
    let mut try_ = vec![tdef.clone()];
    let co = t.type_for_key("co");
    if !co.is_empty() {
        let (tco, _) = tdef.set_type_for_key("co", &co);
        try_ = vec![tco, tdef];
    }

    let und = language::und();
    for tx in try_ {
        let mut tx = tx;
        while tx != und {
            for (i, t) in tags[start..].iter().enumerate() {
                let (b, _, _) = t.raw();
                if b != base {
                    break;
                }
                if tx == *t {
                    return start + i;
                }
            }
            tx = parent(&tx);
        }
    }
    0
}

// Go: internal/colltab/colltab.go:parent
/// Computes the structural parent. This means inheritance may change
/// script. So, unlike the CLDR parent, parent(zh-Hant) == zh.
fn parent(t: &Tag) -> Tag {
    if !t.type_for_key("va").is_empty() {
        let (t, _) = t.set_type_for_key("va", "");
        return t;
    }
    let mut result = language::und();
    let (b, s, r) = t.raw();
    if r != Region::default() {
        result = language::RAW
            .compose(&[
                Part::Base(b),
                Part::Script(s),
                Part::Extensions(t.extensions()),
            ])
            .0;
    } else if s != Script::default() {
        result = language::RAW
            .compose(&[Part::Base(b), Part::Extensions(t.extensions())])
            .0;
    } else if b != Base::default() {
        result = language::RAW.compose(&[Part::Extensions(t.extensions())]).0;
    }
    result
}
