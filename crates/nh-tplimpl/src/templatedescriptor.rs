//! Port of `tpl/tplimpl/templatedescriptor.go`.
//!
//! Owner: Wave B task T13 (tplimpl).


//! Go `templatedescriptor.go`: descriptor matching weights (kind +5, standard layout +4 / "all" +2,
//! custom layout +6 (w2=2), lang +1 (or default lang), output format +4, media type +1,
//! variant1 +6, variant2 +4) and rejection rules. See specs/templates-inventory.md §5.1.

/// Go: `tplimpl.TemplateDescriptor`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateDescriptor {
    /// Group 1: kind (home, page, section...).
    pub kind: String,
    /// "single", "list", "all" (from the template file) ...
    pub layout_from_template: String,
    /// Front matter `layout`.
    pub layout_from_user: String,
    /// Group 2: output format name.
    pub output_format: String,
    pub media_type: String,
    pub lang: String,
    /// Group 3: variants (render hook type, e.g. "link", "image", "heading", "table").
    pub variant1: String,
    pub variant2: String,
    /// Group 4 flags.
    pub layout_from_user_must_match: bool,
    pub is_plain_text: bool,
    pub always_allow_plain_text: bool,
}

impl TemplateDescriptor {
    // Go: tpl/tplimpl/templatedescriptor.go:normalizeFromFile
    pub(crate) fn normalize_from_file(&mut self) {
        todo!()
    }
}

/// Go: `weight`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Weight {
    pub w1: i64,
    pub w2: i64,
    pub w3: i64,
    pub distance: i64,
}

impl Weight {
    pub const NO_MATCH: Weight = Weight { w1: -1, w2: 0, w3: 0, distance: 0 };

    // Go: tpl/tplimpl/templatestore.go:isEqualWeights
    pub fn is_equal_weights(&self, other: &Weight) -> bool {
        self.w1 == other.w1 && self.w2 == other.w2 && self.w3 == other.w3
    }
}

/// Go: `descriptorHandler`.
pub struct DescriptorHandler {
    pub default_content_language: String,
    pub default_output_format: String,
}

impl DescriptorHandler {
    // Go: tpl/tplimpl/templatedescriptor.go:compareDescriptors
    pub fn compare_descriptors(&self, category: crate::category::Category, is_embedded: bool, this: &TemplateDescriptor, other: &TemplateDescriptor) -> Weight {
        todo!()
    }
}

// Go: tpl/tplimpl/templatedescriptor.go:doCompare
pub fn do_compare(this: &TemplateDescriptor, category: crate::category::Category, default_content_language: &str, other: &TemplateDescriptor) -> Weight {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/templatedescriptor.go (238 lines; 4/5 funcs executed)
//   types: TemplateDescriptor, descriptorHandler
// EX L43-55: (d *TemplateDescriptor) normalizeFromFile()
// EX L63-88: (s descriptorHandler) compareDescriptors(category Category, isEmbedded bool, this, other TemplateDescriptor) weight
// EX L91-223: (this TemplateDescriptor) doCompare(category Category, defaultContentLanguage string, other TemplateDescriptor) weight
//    L225-227: (d TemplateDescriptor) IsZero() bool
// EX L230-238: (this TemplateDescriptor) isKindInLayout(layout string) bool
// ---------------------------------------------------------------------------
