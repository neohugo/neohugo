//! Port of `tpl/tplimpl/template_info.go`.
//!
//! Owner: Wave B task T13 (tplimpl).


/// Go: `tplimpl.TemplateVersion`.
pub const TEMPLATE_VERSION: i64 = 2;

/// Go: `tplimpl.ParseConfig` (shortcode `$_hugo_config`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParseConfig {
    pub version: i64,
}

/// Go: `tplimpl.ParseInfo`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParseInfo {
    /// Set for shortcode templates with any {{ .Inner }}.
    pub is_inner: bool,
    /// Set for partials with a return statement.
    pub has_return: bool,
    /// Config extracted from template.
    pub config: ParseConfig,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/template_info.go (46 lines; 0/1 funcs executed)
//   types: ParseInfo, ParseConfig
//    L31-33: (info ParseInfo) IsZero() bool
// ---------------------------------------------------------------------------
