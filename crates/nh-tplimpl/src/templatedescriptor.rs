//! Port of `tpl/tplimpl/templatedescriptor.go`.
//!
//! Owner: Wave B task T13 (tplimpl).

//! Go `templatedescriptor.go`: descriptor matching weights (kind +5, standard layout +4 / "all" +2,
//! custom layout +6 (w2=2), lang +1 (or default lang), output format +4, media type +1,
//! variant1 +6, variant2 +4) and rejection rules. See specs/templates-inventory.md §5.1.

use nh_common::kinds::{KIND_PAGE, KIND_TEMPORARY};

use crate::category::Category;
use crate::templatestore::{LAYOUT_ALL, LAYOUT_LIST, LAYOUT_SINGLE};

/// Go: `baseNameBaseof`.
pub const BASE_NAME_BASEOF: &str = "baseof";

/// Go: `tplimpl.TemplateDescriptor` (used both as a key and in lookups).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateDescriptor {
    /// Group 1: kind (page, home, section, taxonomy, term and only those).
    pub kind: String,
    /// "list", "single", "all", mycustomlayout (from the template file).
    pub layout_from_template: String,
    /// Custom layout set in front matter, e.g. list, single, all, mycustomlayout.
    pub layout_from_user: String,
    /// Group 2: output format name (rss, csv ...).
    pub output_format: String,
    /// text/html, text/plain, ...
    pub media_type: String,
    /// en, nn, fr, ...
    pub lang: String,
    /// Contextual variant, e.g. "link" in render hooks.
    pub variant1: String,
    /// Contextual variant, e.g. "id" in render.
    pub variant2: String,
    /// If set, we only look for the exact layout.
    pub layout_from_user_must_match: bool,
    /// Whether this is a plain text template.
    pub is_plain_text: bool,
    /// Whether to e.g. allow plain text templates to be rendered in HTML.
    pub always_allow_plain_text: bool,
}

impl TemplateDescriptor {
    // Go: tpl/tplimpl/templatedescriptor.go:normalizeFromFile
    pub(crate) fn normalize_from_file(&mut self) {
        if self.layout_from_template == self.output_format {
            self.layout_from_template = String::new();
        }

        if self.kind == KIND_TEMPORARY {
            self.kind = String::new();
        }

        if self.layout_from_template == self.kind {
            self.layout_from_template = String::new();
        }
    }

    // Go: tpl/tplimpl/templatedescriptor.go:IsZero
    pub fn is_zero(&self) -> bool {
        *self == TemplateDescriptor::default()
    }

    // Go: tpl/tplimpl/templatedescriptor.go:isKindInLayout
    pub(crate) fn is_kind_in_layout(&self, layout: &str) -> bool {
        if self.kind.is_empty() {
            return true;
        }
        if self.kind != KIND_PAGE {
            return layout != LAYOUT_SINGLE;
        }
        layout != LAYOUT_LIST
    }

    /// Go: `fmt.Sprintf("%+v", d)` (the oracle's descriptor dump).
    pub fn go_string_plus_v(&self) -> String {
        format!(
            "{{Kind:{} LayoutFromTemplate:{} LayoutFromUser:{} OutputFormat:{} MediaType:{} Lang:{} Variant1:{} Variant2:{} LayoutFromUserMustMatch:{} IsPlainText:{} AlwaysAllowPlainText:{}}}",
            self.kind,
            self.layout_from_template,
            self.layout_from_user,
            self.output_format,
            self.media_type,
            self.lang,
            self.variant1,
            self.variant2,
            self.layout_from_user_must_match,
            self.is_plain_text,
            self.always_allow_plain_text
        )
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
    /// Go: `weightNoMatch`.
    pub const NO_MATCH: Weight = Weight {
        w1: -1,
        w2: 0,
        w3: 0,
        distance: 0,
    };

    // Go: tpl/tplimpl/templatestore.go:isEqualWeights
    pub fn is_equal_weights(&self, other: &Weight) -> bool {
        self.w1 == other.w1 && self.w2 == other.w2 && self.w3 == other.w3
    }
}

/// Go: `descriptorHandler` (Go holds the whole `StoreOptions`; only these two are read).
pub struct DescriptorHandler {
    pub default_content_language: String,
    pub default_output_format: String,
}

impl DescriptorHandler {
    /// Note that this in this setup is usually a descriptor constructed from a page, so we want
    /// to find the best match for that page.
    // Go: tpl/tplimpl/templatedescriptor.go:compareDescriptors
    pub fn compare_descriptors(
        &self,
        category: Category,
        _is_embedded: bool,
        this: &TemplateDescriptor,
        other: &TemplateDescriptor,
    ) -> Weight {
        if this.layout_from_user_must_match && this.layout_from_user != other.layout_from_template {
            return Weight::NO_MATCH;
        }

        let mut w = do_compare(this, category, &self.default_content_language, other);

        if w.w1 <= 0 {
            if category == Category::Markup
                && (this.variant1 == other.variant1)
                && (this.variant2 == other.variant2
                    || !this.variant2.is_empty() && other.variant2.is_empty())
            {
                // See issue 13242.
                if this.output_format != other.output_format
                    && this.output_format == self.default_output_format
                {
                    return w;
                }

                w.w1 = 1;
            }

            if category == Category::Shortcode
                && ((this.is_plain_text == other.is_plain_text || !other.is_plain_text)
                    || this.always_allow_plain_text)
            {
                w.w1 = 1;
            }
        }

        w
    }
}

// Go: tpl/tplimpl/templatedescriptor.go:doCompare
pub fn do_compare(
    this: &TemplateDescriptor,
    category: Category,
    default_content_language: &str,
    other: &TemplateDescriptor,
) -> Weight {
    let mut w = Weight::NO_MATCH;

    if !this.always_allow_plain_text {
        // HTML in plain text is OK, but not the other way around.
        if other.is_plain_text && !this.is_plain_text {
            return w;
        }
    }

    if !other.kind.is_empty() && other.kind != this.kind {
        return w;
    }

    if !other.layout_from_template.is_empty()
        && other.layout_from_template != LAYOUT_ALL
        && (this.layout_from_user.is_empty() || this.layout_from_user != other.layout_from_template)
        && other.layout_from_template != this.layout_from_template
    {
        return w;
    }

    if !other.lang.is_empty() && other.lang != this.lang {
        return w;
    }

    if !other.output_format.is_empty() && other.output_format != this.output_format {
        if this.media_type != other.media_type {
            return w;
        }

        // We want e.g. home page in amp output format (media type text/html) to
        // find a template even if one isn't specified for that output format,
        // when one exist for the html output format (same media type).
        let mut skip = category != Category::Baseof
            && (this.kind.is_empty()
                || (this.kind != other.kind
                    && (this.layout_from_template != other.layout_from_template
                        && other.layout_from_template != LAYOUT_ALL)));
        if !this.layout_from_user.is_empty() {
            skip = skip && (this.layout_from_user != other.layout_from_template);
        }
        if skip {
            return w;
        }

        // Continue.
    }

    if other.media_type != this.media_type {
        return w;
    }

    // One example of variant1 and 2 is for render codeblocks:
    // variant1=codeblock, variant2=go (language).
    if !other.variant1.is_empty() {
        if other.variant1 != this.variant1 {
            return w;
        }

        if !other.variant2.is_empty() && other.variant2 != this.variant2 {
            return w;
        }
    }

    const WEIGHT_KIND: i64 = 5; // page, home, section, taxonomy, term (and only those)
    const WEIGHT_CUSTOM_LAYOUT: i64 = 6; // custom layout (mylayout, set in e.g. front matter)
    const WEIGHT_LAYOUT_STANDARD: i64 = 4; // standard layouts (single,list)
    const WEIGHT_LAYOUT_ALL: i64 = 2; // the "all" layout
    const WEIGHT_OUTPUT_FORMAT: i64 = 4; // a configured output format (e.g. rss, html, json)
    const WEIGHT_MEDIA_TYPE: i64 = 1; // a configured media type (e.g. text/html, text/plain)
    const WEIGHT_LANG: i64 = 1; // a configured language (e.g. en, nn, fr, ...)
    const WEIGHT_VARIANT1: i64 = 6; // currently used for render hooks, e.g. "link", "image"
    const WEIGHT_VARIANT2: i64 = 4; // currently used for render hooks, e.g. the language "go" in code blocks.

    // We will use the values for group 2 and 3
    // if the distance up to the template is shorter than
    // the one we're comparing with.
    // E.g for a page in /posts/mypage.md with the
    // two templates /layouts/posts/single.html and /layouts/page.html,
    // the first one is the best match even if the second one
    // has a higher w1 value.
    const WEIGHT2_GROUP1: i64 = 1; // kind, standardl layout (single,list,all)
    const WEIGHT2_GROUP2: i64 = 2; // custom layout (mylayout)

    const WEIGHT3: i64 = 1; // for media type, lang, output format.

    // Now we now know that the other descriptor is a subset of this.
    // Now calculate the weights.
    w.w1 += 1;

    if !other.kind.is_empty() && other.kind == this.kind {
        w.w1 += WEIGHT_KIND;
        w.w2 = WEIGHT2_GROUP1;
    }

    if !other.layout_from_template.is_empty()
        && (other.layout_from_template == this.layout_from_template)
    {
        w.w1 += WEIGHT_LAYOUT_STANDARD;
        w.w2 = WEIGHT2_GROUP1;
    } else if other.layout_from_template == LAYOUT_ALL {
        w.w1 += WEIGHT_LAYOUT_ALL;
        w.w2 = WEIGHT2_GROUP1;
    }

    // LayoutCustom is only set in this (usually from Page.Layout).
    if !this.layout_from_user.is_empty() && this.layout_from_user == other.layout_from_template {
        w.w1 += WEIGHT_CUSTOM_LAYOUT;
        w.w2 = WEIGHT2_GROUP2;
    }

    if (!other.lang.is_empty() && other.lang == this.lang)
        || (other.lang.is_empty() && this.lang == default_content_language)
    {
        w.w1 += WEIGHT_LANG;
        w.w3 += WEIGHT3;
    }

    if !other.output_format.is_empty() && other.output_format == this.output_format {
        w.w1 += WEIGHT_OUTPUT_FORMAT;
        w.w3 += WEIGHT3;
    }

    if !other.media_type.is_empty() && other.media_type == this.media_type {
        w.w1 += WEIGHT_MEDIA_TYPE;
        w.w3 += WEIGHT3;
    }

    if !other.variant1.is_empty() && other.variant1 == this.variant1 {
        w.w1 += WEIGHT_VARIANT1;
    }

    if !other.variant1.is_empty() && other.variant2 == this.variant2 {
        w.w1 += WEIGHT_VARIANT2;
    }

    w
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/templatedescriptor.go (238 lines; 4/5 funcs executed)
//   types: TemplateDescriptor, descriptorHandler
// OK L43-55: (d *TemplateDescriptor) normalizeFromFile()
// OK L63-88: (s descriptorHandler) compareDescriptors(category Category, isEmbedded bool, this, other TemplateDescriptor) weight
// OK L91-223: (this TemplateDescriptor) doCompare(category Category, defaultContentLanguage string, other TemplateDescriptor) weight
// OK L225-227: (d TemplateDescriptor) IsZero() bool
// OK L230-238: (this TemplateDescriptor) isKindInLayout(layout string) bool
// ---------------------------------------------------------------------------
