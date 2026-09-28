//! Port of `common/constants/constants.go`.
//!
//! Owner: Wave B task T01 (common-values).

// Error/Warning IDs. Do not change these values.

/// Go: `constants.ErrRemoteGetJSON` (IDs for remote errors in tpl/data).
pub const ERR_REMOTE_GET_JSON: &str = "error-remote-getjson";
/// Go: `constants.ErrRemoteGetCSV`.
pub const ERR_REMOTE_GET_CSV: &str = "error-remote-getcsv";

/// Go: `constants.WarnFrontMatterParamsOverrides`.
pub const WARN_FRONT_MATTER_PARAMS_OVERRIDES: &str = "warning-frontmatter-params-overrides";
/// Go: `constants.WarnRenderShortcodesInHTML`.
pub const WARN_RENDER_SHORTCODES_IN_HTML: &str = "warning-rendershortcodes-in-html";
/// Go: `constants.WarnGoldmarkRawHTML`.
pub const WARN_GOLDMARK_RAW_HTML: &str = "warning-goldmark-raw-html";
/// Go: `constants.WarnPartialSuperfluousPrefix`.
pub const WARN_PARTIAL_SUPERFLUOUS_PREFIX: &str = "warning-partial-superfluous-prefix";
/// Go: `constants.WarnHomePageIsLeafBundle`.
pub const WARN_HOME_PAGE_IS_LEAF_BUNDLE: &str = "warning-home-page-is-leaf-bundle";

/// Not in neohugo's constants.go (skeleton-era name, kept for API stability).
pub const WARN_GO_MODULES_NOT_FOUND: &str = "warning-gomodules-not-found";

// Field/method names with special meaning.

/// Go: `constants.FieldRelPermalink`.
pub const FIELD_REL_PERMALINK: &str = "RelPermalink";
/// Go: `constants.FieldPermalink`.
pub const FIELD_PERMALINK: &str = "Permalink";

// Go: common/constants/constants.go:IsFieldRelOrPermalink
/// IsFieldRelOrPermalink returns whether the given name is a RelPermalink or Permalink.
pub fn is_field_rel_or_permalink(name: &str) -> bool {
    name == FIELD_REL_PERMALINK || name == FIELD_PERMALINK
}

// Resource transformations.

/// Go: `constants.ResourceTransformationFingerprint`.
pub const RESOURCE_TRANSFORMATION_FINGERPRINT: &str = "fingerprint";

// Go: common/constants/constants.go:IsResourceTransformationPermalinkHash
/// IsResourceTransformationPermalinkHash returns whether the given name is a resource
/// transformation that changes the permalink based on the content.
pub fn is_resource_transformation_permalink_hash(name: &str) -> bool {
    name == RESOURCE_TRANSFORMATION_FINGERPRINT
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/constants/constants.go (49 lines; 0/2 funcs executed)
// OK L37-39: IsFieldRelOrPermalink(name string) bool
// OK L47-49: IsResourceTransformationPermalinkHash(name string) bool
// ---------------------------------------------------------------------------
