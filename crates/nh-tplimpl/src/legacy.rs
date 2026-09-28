//! Port of `tpl/tplimpl/legacy.go`.
//!
//! Owner: Wave B task T13 (tplimpl).

//! Go `legacy.go`: pre-0.146 layout path mappings (`_default/`, `partials/` -> `_partials/`,
//! `shortcodes/` -> `_shortcodes/`, `x-baseof.html` -> `baseof.x.html`, taxonomy/term/section
//! legacy files such as `taxonomy/list.html` -> kind taxonomy, `term/term.html` -> kind term).
//! The tables are compiled into ordinal maps by `TemplateStore::init` and applied in
//! `insertTemplates`.

use nh_common::kinds::{KIND_SECTION, KIND_TAXONOMY, KIND_TERM};
use nh_hugofs::fileinfo::FileMetaInfo;

use crate::category::Category;
use crate::templatedescriptor::TemplateDescriptor;

/// Go: `layoutLegacyMapping`.
#[derive(Clone, Debug)]
pub(crate) struct LayoutLegacyMapping {
    pub(crate) source_path: String,
    pub(crate) target: LayoutLegacyMappingTarget,
}

/// Go: `layoutLegacyMappingTarget`.
#[derive(Clone, Debug)]
pub(crate) struct LayoutLegacyMappingTarget {
    pub(crate) target_path: String,
    pub(crate) target_desc: TemplateDescriptor,
    pub(crate) target_category: Category,
}

/// Go: `legacyTargetPathIdentifiers` (a map key).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LegacyTargetPathIdentifiers {
    pub(crate) target_path: String,
    pub(crate) target_category: Category,
    pub(crate) kind: String,
    pub(crate) lang: String,
    pub(crate) output_format: String,
    pub(crate) ext: String,
}

/// Go: `legacyOrdinalMapping`.
#[derive(Clone, Debug)]
pub(crate) struct LegacyOrdinalMapping {
    pub(crate) ordinal: i64,
    pub(crate) mapping: LayoutLegacyMappingTarget,
}

/// Go: `legacyOrdinalMappingFi`.
#[derive(Clone)]
pub(crate) struct LegacyOrdinalMappingFi {
    pub(crate) m: LegacyOrdinalMapping,
    pub(crate) fi: FileMetaInfo,
}

fn target(target_path: &str, kind: &str) -> LayoutLegacyMappingTarget {
    LayoutLegacyMappingTarget {
        target_path: target_path.to_string(),
        target_desc: TemplateDescriptor {
            kind: kind.to_string(),
            ..Default::default()
        },
        target_category: Category::Layout,
    }
}

// Go: tpl/tplimpl/legacy.go:ltermPlural
fn lterm_plural() -> LayoutLegacyMappingTarget {
    target("/PLURAL", KIND_TERM)
}

// Go: tpl/tplimpl/legacy.go:ltermBase
fn lterm_base() -> LayoutLegacyMappingTarget {
    target("", KIND_TERM)
}

// Go: tpl/tplimpl/legacy.go:ltaxPlural
fn ltax_plural() -> LayoutLegacyMappingTarget {
    target("/PLURAL", KIND_TAXONOMY)
}

// Go: tpl/tplimpl/legacy.go:ltaxBase
fn ltax_base() -> LayoutLegacyMappingTarget {
    target("", KIND_TAXONOMY)
}

// Go: tpl/tplimpl/legacy.go:lsectBase
fn lsect_base() -> LayoutLegacyMappingTarget {
    target("", KIND_SECTION)
}

// Go: tpl/tplimpl/legacy.go:lsectTheSection
fn lsect_the_section() -> LayoutLegacyMappingTarget {
    target("/THESECTION", KIND_SECTION)
}

fn m(source_path: &str, target: LayoutLegacyMappingTarget) -> LayoutLegacyMapping {
    LayoutLegacyMapping {
        source_path: source_path.to_string(),
        target,
    }
}

// Go: tpl/tplimpl/legacy.go:legacyTermMappings
pub(crate) fn legacy_term_mappings() -> Vec<LayoutLegacyMapping> {
    vec![
        m("/PLURAL/term", lterm_plural()),
        m("/PLURAL/SINGULAR", lterm_plural()),
        m("/term/term", lterm_base()),
        m("/term/SINGULAR", lterm_plural()),
        m("/term/taxonomy", lterm_plural()),
        m("/term/list", lterm_base()),
        m("/taxonomy/term", lterm_base()),
        m("/taxonomy/SINGULAR", lterm_plural()),
        m("/SINGULAR/term", lterm_plural()),
        m("/SINGULAR/SINGULAR", lterm_plural()),
        m("/_default/SINGULAR", lterm_plural()),
        m("/_default/taxonomy", lterm_base()),
    ]
}

// Go: tpl/tplimpl/legacy.go:legacyTaxonomyMappings
pub(crate) fn legacy_taxonomy_mappings() -> Vec<LayoutLegacyMapping> {
    vec![
        m("/PLURAL/SINGULAR.terms", ltax_plural()),
        m("/PLURAL/terms", ltax_plural()),
        m("/PLURAL/taxonomy", ltax_plural()),
        m("/PLURAL/list", ltax_plural()),
        m("/SINGULAR/SINGULAR.terms", ltax_plural()),
        m("/SINGULAR/terms", ltax_plural()),
        m("/SINGULAR/taxonomy", ltax_plural()),
        m("/SINGULAR/list", ltax_plural()),
        m("/taxonomy/SINGULAR.terms", ltax_plural()),
        m("/taxonomy/terms", ltax_base()),
        m("/taxonomy/taxonomy", ltax_base()),
        m("/taxonomy/list", ltax_base()),
        m("/_default/SINGULAR.terms", ltax_base()),
        m("/_default/terms", ltax_base()),
        m("/_default/taxonomy", ltax_base()),
    ]
}

// Go: tpl/tplimpl/legacy.go:legacySectionMappings
pub(crate) fn legacy_section_mappings() -> Vec<LayoutLegacyMapping> {
    vec![
        // E.g. /mysection/mysection.html
        m("/THESECTION/THESECTION", lsect_the_section()),
        // E.g. /section/mysection.html
        m("/SECTIONKIND/THESECTION", lsect_the_section()),
        // E.g. /section/section.html
        m("/SECTIONKIND/SECTIONKIND", lsect_base()),
        // E.g. /section/list.html
        m("/SECTIONKIND/list", lsect_base()),
        // E.g. /_default/mysection.html
        m("/_default/THESECTION", lsect_the_section()),
    ]
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/legacy.go (130 lines; 0/0 funcs executed)
//   types: layoutLegacyMapping, layoutLegacyMappingTarget, legacyTargetPathIdentifiers, legacyOrdinalMapping,
//          legacyOrdinalMappingFi
// OK (all tables and types; no functions)
// ---------------------------------------------------------------------------
