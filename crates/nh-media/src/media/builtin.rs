//! Port of `media/builtin.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::sync::OnceLock;

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

/// The `Type` strings of the builtin types, in field order (Go: the `Builtin` literal).
pub(crate) const BUILTIN_TYPE_NAMES: [(&str, &str); 41] = [
    ("CalendarType", "text/calendar"),
    ("CSSType", "text/css"),
    ("SCSSType", "text/x-scss"),
    ("SASSType", "text/x-sass"),
    ("GotmplType", "text/x-gotmpl"),
    ("CSVType", "text/csv"),
    ("HTMLType", "text/html"),
    ("JavascriptType", "text/javascript"),
    ("TypeScriptType", "text/typescript"),
    ("TSXType", "text/tsx"),
    ("JSXType", "text/jsx"),
    ("JSONType", "application/json"),
    ("WebAppManifestType", "application/manifest+json"),
    ("RSSType", "application/rss+xml"),
    ("XMLType", "application/xml"),
    ("SVGType", "image/svg+xml"),
    ("TextType", "text/plain"),
    ("TOMLType", "application/toml"),
    ("YAMLType", "application/yaml"),
    ("PNGType", "image/png"),
    ("JPEGType", "image/jpeg"),
    ("GIFType", "image/gif"),
    ("TIFFType", "image/tiff"),
    ("BMPType", "image/bmp"),
    ("WEBPType", "image/webp"),
    ("TrueTypeFontType", "font/ttf"),
    ("OpenTypeFontType", "font/otf"),
    ("PDFType", "application/pdf"),
    ("MarkdownType", "text/markdown"),
    ("EmacsOrgModeType", "text/org"),
    ("AsciiDocType", "text/asciidoc"),
    ("PandocType", "text/pandoc"),
    ("ReStructuredTextType", "text/rst"),
    ("AVIType", "video/x-msvideo"),
    ("MPEGType", "video/mpeg"),
    ("MP4Type", "video/mp4"),
    ("OGGType", "video/ogg"),
    ("WEBMType", "video/webm"),
    ("GPPType", "video/3gpp"),
    ("WasmType", "application/wasm"),
    ("OctetType", "application/octet-stream"),
];

/// Go: `media.defaultMediaTypesConfig`: type -> suffixes (`delimiter: "."` is added to every
/// entry by `media/config.go`'s `init`).
pub(crate) const DEFAULT_MEDIA_TYPES_CONFIG: [(&str, &[&str]); 41] = [
    ("text/calendar", &["ics"]),
    ("text/css", &["css"]),
    ("text/x-scss", &["scss"]),
    ("text/x-sass", &["sass"]),
    ("text/csv", &["csv"]),
    ("text/html", &["html", "htm"]),
    ("text/javascript", &["js", "jsm", "mjs"]),
    ("text/typescript", &["ts"]),
    ("text/tsx", &["tsx"]),
    ("text/jsx", &["jsx"]),
    ("text/x-gotmpl", &["gotmpl"]),
    ("application/json", &["json"]),
    ("application/manifest+json", &["webmanifest"]),
    ("application/rss+xml", &["xml", "rss"]),
    ("application/xml", &["xml"]),
    ("image/svg+xml", &["svg"]),
    ("text/plain", &["txt"]),
    ("application/toml", &["toml"]),
    ("application/yaml", &["yaml", "yml"]),
    // Common image types
    ("image/png", &["png"]),
    ("image/jpeg", &["jpg", "jpeg", "jpe", "jif", "jfif"]),
    ("image/gif", &["gif"]),
    ("image/tiff", &["tif", "tiff"]),
    ("image/bmp", &["bmp"]),
    ("image/webp", &["webp"]),
    // Common font types
    ("font/ttf", &["ttf"]),
    ("font/otf", &["otf"]),
    // Common document types
    ("application/pdf", &["pdf"]),
    ("text/markdown", &["md", "mdown", "markdown"]),
    ("text/asciidoc", &["adoc", "asciidoc", "ad"]),
    ("text/pandoc", &["pandoc", "pdc"]),
    ("text/rst", &["rst"]),
    ("text/org", &["org"]),
    // Common video types
    ("video/x-msvideo", &["avi"]),
    ("video/mpeg", &["mpg", "mpeg"]),
    ("video/mp4", &["mp4"]),
    ("video/ogg", &["ogv"]),
    ("video/webm", &["webm"]),
    ("video/3gpp", &["3gpp", "3gp"]),
    // wasm
    ("application/wasm", &["wasm"]),
    ("application/octet-stream", &[]),
];

/// Go: `media.Builtin`, initialised (in `media/config.go`'s `init`) with the values of
/// `DefaultTypes`.
// Go: media/config.go:init
pub fn builtin() -> &'static BuiltinTypes {
    static B: OnceLock<BuiltinTypes> = OnceLock::new();
    B.get_or_init(|| {
        let default_types = super::config::default_types();
        let get = |i: usize| -> MediaType {
            let (field_name, typ) = BUILTIN_TYPE_NAMES[i];
            match default_types.get_by_type(typ) {
                Some(t) => t,
                None => panic!("missing default type for field builtin type: {field_name:?}"),
            }
        };
        BuiltinTypes {
            calendar_type: get(0),
            css_type: get(1),
            scss_type: get(2),
            sass_type: get(3),
            gotmpl_type: get(4),
            csv_type: get(5),
            html_type: get(6),
            javascript_type: get(7),
            typescript_type: get(8),
            tsx_type: get(9),
            jsx_type: get(10),
            json_type: get(11),
            web_app_manifest_type: get(12),
            rss_type: get(13),
            xml_type: get(14),
            svg_type: get(15),
            text_type: get(16),
            toml_type: get(17),
            yaml_type: get(18),
            png_type: get(19),
            jpeg_type: get(20),
            gif_type: get(21),
            tiff_type: get(22),
            bmp_type: get(23),
            webp_type: get(24),
            true_type_font_type: get(25),
            open_type_font_type: get(26),
            pdf_type: get(27),
            markdown_type: get(28),
            emacs_org_mode_type: get(29),
            ascii_doc_type: get(30),
            pandoc_type: get(31),
            re_structured_text_type: get(32),
            avi_type: get(33),
            mpeg_type: get(34),
            mp4_type: get(35),
            ogg_type: get(36),
            webm_type: get(37),
            gpp_type: get(38),
            wasm_type: get(39),
            octet_type: get(40),
        }
    })
}

impl BuiltinTypes {
    /// The builtin types with their Go field names, in declaration order.
    pub fn fields(&self) -> Vec<(&'static str, &MediaType)> {
        let all = [
            &self.calendar_type,
            &self.css_type,
            &self.scss_type,
            &self.sass_type,
            &self.gotmpl_type,
            &self.csv_type,
            &self.html_type,
            &self.javascript_type,
            &self.typescript_type,
            &self.tsx_type,
            &self.jsx_type,
            &self.json_type,
            &self.web_app_manifest_type,
            &self.rss_type,
            &self.xml_type,
            &self.svg_type,
            &self.text_type,
            &self.toml_type,
            &self.yaml_type,
            &self.png_type,
            &self.jpeg_type,
            &self.gif_type,
            &self.tiff_type,
            &self.bmp_type,
            &self.webp_type,
            &self.true_type_font_type,
            &self.open_type_font_type,
            &self.pdf_type,
            &self.markdown_type,
            &self.emacs_org_mode_type,
            &self.ascii_doc_type,
            &self.pandoc_type,
            &self.re_structured_text_type,
            &self.avi_type,
            &self.mpeg_type,
            &self.mp4_type,
            &self.ogg_type,
            &self.webm_type,
            &self.gpp_type,
            &self.wasm_type,
            &self.octet_type,
        ];
        BUILTIN_TYPE_NAMES
            .iter()
            .zip(all)
            .map(|((n, _), t)| (*n, t))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: media/builtin.go (169 lines; 0/0 funcs executed)
//   types: BuiltinTypes
// ---------------------------------------------------------------------------
