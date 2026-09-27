//! Port of `media/builtin.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use super::media_type::MediaType;

/// Go: `media.BuiltinTypes` / `media.Builtin` — the built-in media types (see
/// specs/architecture-core.md §5.6 for the full table and suffixes).
#[derive(Clone, Debug)]
pub struct BuiltinTypes {
    pub calendar_type: MediaType,
    pub css_type: MediaType,
    pub scss_type: MediaType,
    pub sass_type: MediaType,
    pub gotmpl_type: MediaType,
    pub csv_type: MediaType,
    pub html_type: MediaType,
    pub javascript_type: MediaType,
    pub typescript_type: MediaType,
    pub tsx_type: MediaType,
    pub jsx_type: MediaType,
    pub json_type: MediaType,
    pub web_app_manifest_type: MediaType,
    pub rss_type: MediaType,
    pub xml_type: MediaType,
    pub svg_type: MediaType,
    pub text_type: MediaType,
    pub toml_type: MediaType,
    pub yaml_type: MediaType,
    pub png_type: MediaType,
    pub jpeg_type: MediaType,
    pub gif_type: MediaType,
    pub tiff_type: MediaType,
    pub bmp_type: MediaType,
    pub webp_type: MediaType,
    pub true_type_font_type: MediaType,
    pub open_type_font_type: MediaType,
    pub pdf_type: MediaType,
    pub markdown_type: MediaType,
    pub emacs_org_mode_type: MediaType,
    pub ascii_doc_type: MediaType,
    pub pandoc_type: MediaType,
    pub re_structured_text_type: MediaType,
    pub avi_type: MediaType,
    pub mpeg_type: MediaType,
    pub mp4_type: MediaType,
    pub ogg_type: MediaType,
    pub webm_type: MediaType,
    pub gpp_type: MediaType,
    pub wasm_type: MediaType,
    pub octet_type: MediaType,
}

/// Go: `media.Builtin`.
pub fn builtin() -> &'static BuiltinTypes {
    todo!("OnceLock-initialised table from media/builtin.go (+ defaultMediaTypesConfig suffixes)")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: media/builtin.go (169 lines; 0/0 funcs executed)
//   types: BuiltinTypes
// ---------------------------------------------------------------------------
