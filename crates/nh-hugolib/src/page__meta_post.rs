//! Port of `hugolib/page__meta.go` (`setMetaPost`, `setMetaPostParams`, `applyDefaultValues`).
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Split from `page__meta.go`: these run only in the assembly walks (`applyAggregates`,
//! content_map_page.go:1445, 1512, 1579), so they belong to the assembly task.
//! * `setMetaPost(cascade)`: applies the cascade, then `setMetaPostParams`, then (first run)
//!   `applyDefaultValues`; terms are delayed until after `assembleTermsAndTranslations`.
//! * `setMetaPostParams()`: the reserved-key switch into `PageConfig`, params normalisation
//!   (lower-cased keys, `[]any` of strings -> `[]string`, dates through the front matter handler,
//!   `draft`/`iscjklanguage`), the normalised `params` map (`maps.Params`).
//! * `applyDefaultValues()`: default titles for pages without a file — home = site title,
//!   section = CreateTitle(Pluralize(dir)), taxonomy, term = CreateTitle(term) (title-cased LAST
//!   term value, see content-model §7.3), 404.

use nh_common::Result;

use crate::page__meta::PageMeta;

impl PageMeta {
    /// Go: `(ps *pageState) setMetaPost(cascade)`.
    // Go: hugolib/page__meta.go:setMetaPost
    pub fn set_meta_post(&mut self /* , site, frontmatter handler, cascade */) -> Result<()> {
        todo!()
    }

    /// Go: `(p *pageState) setMetaPostParams()`.
    // Go: hugolib/page__meta.go:setMetaPostParams
    pub fn set_meta_post_params(&mut self) -> Result<()> {
        todo!()
    }

    /// Go: `(p *pageMeta) applyDefaultValues()`.
    // Go: hugolib/page__meta.go:applyDefaultValues
    pub fn apply_default_values(&mut self /* , site */) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__meta.go (setMetaPost part; the rest is in page__meta.rs and page__init.rs)
// EX L293-388: (ps *pageState) setMetaPost(cascade *maps.Ordered[page.PageMatcher, page.PageMatcherParamsConfig]) error
// EX L390-687: (p *pageState) setMetaPostParams() error
// EX L732-786: (p *pageMeta) applyDefaultValues() error
// ---------------------------------------------------------------------------
