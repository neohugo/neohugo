//! Port of `resources/page/page_author.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


use std::collections::BTreeMap;

/// Go: `page.AuthorList` / `Author` (deprecated `.Site.Authors`; kept for API shape).
#[derive(Clone, Debug, Default)]
pub struct Author {
    pub given_name: String,
    pub family_name: String,
    pub display_name: String,
    pub thumbnail: String,
    pub image: String,
    pub short_bio: String,
    pub long_bio: String,
    pub email: String,
    pub social: BTreeMap<String, String>,
}

pub type AuthorList = BTreeMap<String, Author>;

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_author.go (47 lines; 0/0 funcs executed)
//   types: AuthorList, Author, AuthorSocial
// ---------------------------------------------------------------------------
