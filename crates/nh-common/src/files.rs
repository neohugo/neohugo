//! Port of `hugofs/files/classifier.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

pub const FILENAME_PACKAGE_HUGO_JSON: &str = "package.hugo.json";
pub const FILENAME_PACKAGE_JSON: &str = "package.json";
pub const FILENAME_HUGO_STATS_JSON: &str = "hugo_stats.json";

pub const COMPONENT_FOLDER_ARCHETYPES: &str = "archetypes";
pub const COMPONENT_FOLDER_STATIC: &str = "static";
pub const COMPONENT_FOLDER_LAYOUTS: &str = "layouts";
pub const COMPONENT_FOLDER_CONTENT: &str = "content";
pub const COMPONENT_FOLDER_DATA: &str = "data";
pub const COMPONENT_FOLDER_ASSETS: &str = "assets";
pub const COMPONENT_FOLDER_I18N: &str = "i18n";

pub const FOLDER_RESOURCES: &str = "resources";
/// Mounted below /assets with postcss.config.js etc.
pub const FOLDER_JS_CONFIG: &str = "_jsconfig";
pub const NAME_CONTENT_DATA: &str = "_content";

/// Go: `files.ComponentFolders` (sorted).
pub const COMPONENT_FOLDERS: [&str; 7] = [
    COMPONENT_FOLDER_ARCHETYPES,
    COMPONENT_FOLDER_ASSETS,
    COMPONENT_FOLDER_CONTENT,
    COMPONENT_FOLDER_DATA,
    COMPONENT_FOLDER_I18N,
    COMPONENT_FOLDER_LAYOUTS,
    COMPONENT_FOLDER_STATIC,
];

/// Go: `files.IsGoTmplExt`.
pub fn is_go_tmpl_ext(ext: &str) -> bool {
    ext == "gotmpl"
}

/// Go: `files.ResolveComponentFolder("content/blog/foo.md") == "content"`.
// Go: hugofs/files/classifier.go:ResolveComponentFolder
pub fn resolve_component_folder(filename: &str) -> &'static str {
    todo!()
}

/// Go: `files.IsComponentFolder`.
pub fn is_component_folder(name: &str) -> bool {
    COMPONENT_FOLDERS.contains(&name)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/files/classifier.go (93 lines; 3/5 funcs executed)
//    L32-34: IsGoTmplExt(ext string) bool
//    L37-39: IsContentDataExt(ext string) bool
// EX L72-77: init()
// EX L80-89: ResolveComponentFolder(filename string) string
// EX L91-93: IsComponentFolder(name string) bool
// ---------------------------------------------------------------------------
