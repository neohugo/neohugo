//! Port of `common/paths/pathparser.go`, `common/paths/type_string.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

//! The canonical identity of every file (content, layouts, assets, i18n, data). `Path::base()` is
//! the content-tree key; identifiers (lang, output format, kind, layout, baseof) are parsed
//! right-to-left from the last path element. Port exactly: every layout lookup, every tree key and
//! every URL depends on it.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::herrors::Result;
use crate::types::types::LowHigh;

/// Go: `identifierBaseof`.
pub const IDENTIFIER_BASEOF: &str = "baseof";

/// Go: `paths.Type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PathType {
    /// A generic resource, e.g. a JSON file.
    #[default]
    File,
    /// A resource of a content type with front matter.
    ContentResource,
    /// E.g. /blog/my-post.md
    ContentSingle,
    /// Leaf bundles, e.g. /blog/my-post/index.md
    Leaf,
    /// Branch bundles, e.g. /blog/_index.md
    Branch,
    /// Content data file, _content.gotmpl.
    ContentData,
    Markup,
    Shortcode,
    Partial,
    Baseof,
}

/// Go: `paths.PathParser`.
#[derive(Clone)]
pub struct PathParser {
    /// Maps the language code to its index in the languages/sites slice.
    pub language_index: BTreeMap<String, usize>,
    /// Reports whether the given language is disabled.
    pub is_lang_disabled: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    /// Reports whether the given name is a valid output format (second arg: extension, optional).
    pub is_output_format: Arc<dyn Fn(&str, &str) -> bool + Send + Sync>,
    /// Reports whether the given ext is a content file extension.
    pub is_content_ext: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

/// Go: `paths.Path`. Immutable after parsing (Go pools and resets them; Rust just allocates).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Path {
    pub(crate) s: String,
    pub(crate) pos_container_low: i64,
    pub(crate) pos_container_high: i64,
    pub(crate) pos_section_high: i64,
    pub(crate) component: String,
    pub(crate) path_type: PathType,
    pub(crate) identifiers_known: Vec<LowHigh>,
    pub(crate) identifiers_unknown: Vec<LowHigh>,
    pub(crate) pos_identifier_language: i64,
    pub(crate) pos_identifier_output_format: i64,
    pub(crate) pos_identifier_kind: i64,
    pub(crate) pos_identifier_layout: i64,
    pub(crate) pos_identifier_baseof: i64,
    pub(crate) disabled: bool,
    pub(crate) trim_leading_slash: bool,
    pub(crate) unnormalized: Option<Box<Path>>,
}

/// Go: `paths.NormalizePathStringBasic` — `strings.ToLower`, then " " -> "-". Nothing else.
// Go: common/paths/pathparser.go:NormalizePathStringBasic
pub fn normalize_path_string_basic(s: &str) -> String {
    todo!()
}

impl PathParser {
    /// Go: `PathParser.Parse(component, s)` — `s` is slash-separated, relative to the component root.
    // Go: common/paths/pathparser.go:Parse
    pub fn parse(&self, component: &str, s: &str) -> Path {
        todo!()
    }

    // Go: common/paths/pathparser.go:parse
    pub fn try_parse(&self, component: &str, s: &str) -> Result<Path> {
        todo!()
    }

    /// Go: `ParseBaseAndBaseNameNoIdentifier` (used by `GetPage` ref normalisation).
    // Go: common/paths/pathparser.go:ParseBaseAndBaseNameNoIdentifier
    pub fn parse_base_and_base_name_no_identifier(
        &self,
        component: &str,
        s: &str,
    ) -> (String, String) {
        todo!()
    }
}

/// Go: `paths.ModifyPathBundleTypeResource` (files inside a leaf bundle other than its index).
// Go: common/paths/pathparser.go:ModifyPathBundleTypeResource
pub fn modify_path_bundle_type_resource(p: &mut Path) {
    todo!()
}

/// Go: `paths.HasExt`.
pub fn has_ext(p: &str) -> bool {
    todo!()
}

impl Path {
    // Go: common/paths/pathparser.go:Container
    pub fn container(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:ContainerDir
    pub fn container_dir(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:Section
    pub fn section(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:IsContent
    pub fn is_content(&self) -> bool {
        todo!()
    }
    // Go: common/paths/pathparser.go:Name
    pub fn name(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:NameNoExt
    pub fn name_no_ext(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:NameNoLang
    pub fn name_no_lang(&self) -> String {
        todo!()
    }
    // Go: common/paths/pathparser.go:BaseNameNoIdentifier
    pub fn base_name_no_identifier(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:NameNoIdentifier
    pub fn name_no_identifier(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:Dir
    pub fn dir(&self) -> &str {
        todo!()
    }
    /// The full normalized path, e.g. `/brands/alice/_index.md`.
    // Go: common/paths/pathparser.go:Path
    pub fn path(&self) -> &str {
        &self.s
    }
    // Go: common/paths/pathparser.go:PathNoLeadingSlash
    pub fn path_no_leading_slash(&self) -> &str {
        todo!()
    }
    /// The path as written on disk (original case and spaces).
    // Go: common/paths/pathparser.go:Unnormalized
    pub fn unnormalized(&self) -> &Path {
        self.unnormalized.as_deref().unwrap_or(self)
    }
    // Go: common/paths/pathparser.go:PathNoIdentifier
    pub fn path_no_identifier(&self) -> String {
        todo!()
    }
    // Go: common/paths/pathparser.go:PathBeforeLangAndOutputFormatAndExt
    pub fn path_before_lang_and_output_format_and_ext(&self) -> String {
        todo!()
    }
    // Go: common/paths/pathparser.go:PathRel
    pub fn path_rel(&self, owner: &Path) -> String {
        todo!()
    }
    // Go: common/paths/pathparser.go:BaseRel
    pub fn base_rel(&self, owner: &Path) -> String {
        todo!()
    }
    /// The tree key: without extension/language and without `/index`/`_index` for bundles.
    /// Home is `/` (its tree key is `""`, see hugolib `cleanTreeKey`).
    // Go: common/paths/pathparser.go:Base
    pub fn base(&self) -> String {
        todo!()
    }
    /// Go: `BaseReTyped(typ)` — first path segment replaced by the front-matter `type` (layout lookup).
    // Go: common/paths/pathparser.go:BaseReTyped
    pub fn base_re_typed(&self, typ: &str) -> String {
        todo!()
    }
    // Go: common/paths/pathparser.go:BaseNoLeadingSlash
    pub fn base_no_leading_slash(&self) -> String {
        todo!()
    }
    // Go: common/paths/pathparser.go:Ext
    pub fn ext(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:OutputFormat
    pub fn output_format(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:Kind
    pub fn kind(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:Layout
    pub fn layout(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:Lang
    pub fn lang(&self) -> &str {
        todo!()
    }
    // Go: common/paths/pathparser.go:Disabled
    pub fn disabled(&self) -> bool {
        self.disabled
    }
    // Go: common/paths/pathparser.go:Identifiers
    pub fn identifiers(&self) -> Vec<&str> {
        todo!()
    }
    // Go: common/paths/pathparser.go:IdentifiersUnknown
    pub fn identifiers_unknown(&self) -> Vec<&str> {
        todo!()
    }
    // Go: common/paths/pathparser.go:Type
    pub fn path_type(&self) -> PathType {
        self.path_type
    }
    // Go: common/paths/pathparser.go:IsBundle
    pub fn is_bundle(&self) -> bool {
        matches!(self.path_type, PathType::Leaf | PathType::Branch)
    }
    // Go: common/paths/pathparser.go:IsBranchBundle
    pub fn is_branch_bundle(&self) -> bool {
        self.path_type == PathType::Branch
    }
    // Go: common/paths/pathparser.go:IsLeafBundle
    pub fn is_leaf_bundle(&self) -> bool {
        self.path_type == PathType::Leaf
    }
    // Go: common/paths/pathparser.go:IsContentData
    pub fn is_content_data(&self) -> bool {
        self.path_type == PathType::ContentData
    }
    // Go: common/paths/pathparser.go:ForType
    pub fn for_type(&self, t: PathType) -> Path {
        todo!()
    }
    // Go: common/paths/pathparser.go:Component
    pub fn component(&self) -> &str {
        &self.component
    }
    // Go: common/paths/pathparser.go:TrimLeadingSlash
    pub fn trim_leading_slash(&self) -> Path {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/paths/pathparser.go (787 lines; 50/60 funcs executed)
//   types: PathParser, Type, Path
// EX L50-58: NormalizePathStringBasic(s string) string
//    L61-65: (pp *PathParser) ParseIdentity(c, s string) identity.StringIdentity
// EX L68-72: (pp *PathParser) ParseBaseAndBaseNameNoIdentifier(c, s string) (string, string)
// EX L74-83: (pp *PathParser) parsePooled(c, s string) *Path
// EX L86-92: (pp *PathParser) Parse(c, s string) *Path
// EX L94-99: (pp *PathParser) newPath(component string) *Path
// EX L101-121: (pp *PathParser) parse(component, s string) (*Path, error)
// EX L123-214: (pp *PathParser) parseIdentifier(component, s string, p *Path, i, lastDot, numDots int, isLast bool)
// EX L216-319: (pp *PathParser) doParse(component, s string, p *Path) (*Path, error)
// EX L321-327: ModifyPathBundleTypeResource(p *Path)
// EX L397-399: getPath() *Path
// EX L401-404: putPath(p *Path)
// EX L406-422: (p *Path) reset()
//    L425-428: (p Path) TrimLeadingSlash() *Path
// EX L430-435: (p *Path) norm(s string) string
//    L438-443: (p *Path) IdentifierBase() string
//    L446-448: (p *Path) Component() string
// EX L451-456: (p *Path) Container() string
//    L458-463: (p *Path) String() string
// EX L467-472: (p *Path) ContainerDir() string
// EX L475-480: (p *Path) Section() string
// EX L484-486: (p *Path) IsContent() bool
// EX L490-492: (p *Path) isContentPage() bool
// EX L495-500: (p *Path) Name() string
// EX L503-508: (p *Path) NameNoExt() string
//    L511-518: (p *Path) NameNoLang() string
// EX L522-527: (p *Path) BaseNameNoIdentifier() string
// EX L530-533: (p *Path) NameNoIdentifier() string
// EX L535-551: (p *Path) nameLowHigh() types.LowHigh[string]
// EX L554-563: (p *Path) Dir() (d string)
// EX L566-568: (p *Path) Path() (d string)
// EX L571-573: (p *Path) PathNoLeadingSlash() string
// EX L576-578: (p *Path) Unnormalized() *Path
//    L581-583: (p *Path) PathNoLang() string
// EX L586-588: (p *Path) PathNoIdentifier() string
// EX L591-610: (p *Path) PathBeforeLangAndOutputFormatAndExt() string
// EX L613-619: (p *Path) PathRel(owner *Path) string
// EX L622-628: (p *Path) BaseRel(owner *Path) string
// EX L634-636: (p *Path) Base() string
// EX L640-651: (p *Path) BaseReTyped(typ string) (d string)
//    L654-656: (p *Path) BaseNoLeadingSlash() string
// EX L658-688: (p *Path) base(preserveExt, isBundle bool) string
// EX L690-692: (p *Path) Ext() string
// EX L694-696: (p *Path) OutputFormat() string
// EX L698-700: (p *Path) Kind() string
// EX L702-704: (p *Path) Layout() string
// EX L706-708: (p *Path) Lang() string
//    L710-712: (p *Path) Identifier(i int) string
// EX L714-716: (p *Path) Disabled() bool
// EX L718-724: (p *Path) Identifiers() []string
//    L726-732: (p *Path) IdentifiersUnknown() []string
// EX L734-736: (p *Path) Type() Type
// EX L738-740: (p *Path) IsBundle() bool
// EX L742-744: (p *Path) IsBranchBundle() bool
// EX L746-748: (p *Path) IsLeafBundle() bool
// EX L750-752: (p *Path) IsContentData() bool
// EX L754-757: (p Path) ForType(t Type) *Path
// EX L759-767: (p *Path) identifierAsString(i int) string
// EX L769-774: (p *Path) identifierIndex(i int) int
// EX L777-787: HasExt(p string) bool
// Source: common/paths/type_string.go (32 lines; 0/2 funcs executed)
//    L7-21: _()
//    L27-32: (i Type) String() string
// ---------------------------------------------------------------------------
