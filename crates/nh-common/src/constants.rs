//! Port of `common/constants/constants.go`.
//!
//! Owner: Wave B task T01 (common-values).


/// Go: `constants.ResourceTransformationFingerprint`.
pub const RESOURCE_TRANSFORMATION_FINGERPRINT: &str = "fingerprint";
/// Go: `constants.FieldRelPermalink` etc. (postpub fields).
pub const FIELD_REL_PERMALINK: &str = "RelPermalink";
pub const FIELD_PERMALINK: &str = "Permalink";
/// Go: `constants.ErrRemoteGetJSON` / warning ids (only used in log messages).
pub const WARN_GO_MODULES_NOT_FOUND: &str = "warning-gomodules-not-found";

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/constants/constants.go (49 lines; 0/2 funcs executed)
//    L37-39: IsFieldRelOrPermalink(name string) bool
//    L47-49: IsResourceTransformationPermalinkHash(name string) bool
// ---------------------------------------------------------------------------
