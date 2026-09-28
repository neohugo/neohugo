//! Port of `common/paths/pathparser.go`, `common/paths/type_string.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

//! The canonical identity of every file (content, layouts, assets, i18n, data). `Path::base()` is
//! the content-tree key; identifiers (lang, output format, kind, layout, baseof) are parsed
//! right-to-left from the last path element. Port exactly: every layout lookup, every tree key and
//! every URL depends on it.
//!
//! Positions are byte offsets into the normalized path; every one of them sits next to an ASCII
//! `/` or `.`, so slicing the `&str` is always on a char boundary.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use go_unicode::strings;

use crate::files;
use crate::herrors::Result;
use crate::identity::{Identity, StringIdentity};
use crate::kinds;
use crate::types::types::LowHigh;

/// Go: `identifierBaseof`.
pub const IDENTIFIER_BASEOF: &str = "baseof";

/// Go: `paths.Type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PathType {
    /// A generic resource, e.g. a JSON file.
    #[default]
    File,
    // All below are content files.
    /// A resource of a content type with front matter.
    ContentResource,
    /// E.g. /blog/my-post.md
    ContentSingle,
    // All below are bundled content files.
    /// Leaf bundles, e.g. /blog/my-post/index.md
    Leaf,
    /// Branch bundles, e.g. /blog/_index.md
    Branch,
    /// Content data file, _content.gotmpl.
    ContentData,
    // Layout types.
    Markup,
    Shortcode,
    Partial,
    Baseof,
}

impl PathType {
    /// Go: `Type.String()` (stringer).
    // Go: common/paths/type_string.go:String
    pub fn string(self) -> &'static str {
        match self {
            PathType::File => "TypeFile",
            PathType::ContentResource => "TypeContentResource",
            PathType::ContentSingle => "TypeContentSingle",
            PathType::Leaf => "TypeLeaf",
            PathType::Branch => "TypeBranch",
            PathType::ContentData => "TypeContentData",
            PathType::Markup => "TypeMarkup",
            PathType::Shortcode => "TypeShortcode",
            PathType::Partial => "TypePartial",
            PathType::Baseof => "TypeBaseof",
        }
    }
}

/// A `func(string) bool` field of [`PathParser`].
pub type StrPredicate = Arc<dyn Fn(&str) -> bool + Send + Sync>;
/// The `IsOutputFormat func(name, ext string) bool` field of [`PathParser`].
pub type OutputFormatPredicate = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// Go: `paths.PathParser`. `None` fields are Go's nil map/funcs: a nil `LanguageIndex` disables
/// language identifiers, a nil `IsLangDisabled` is never consulted, and calling a nil
/// `IsOutputFormat`/`IsContentExt` panics as in Go.
#[derive(Clone, Default)]
pub struct PathParser {
    /// Maps the language code to its index in the languages/sites slice.
    pub language_index: Option<BTreeMap<String, usize>>,
    /// Reports whether the given language is disabled.
    pub is_lang_disabled: Option<StrPredicate>,
    /// Reports whether the given name is a valid output format (second arg: extension, optional).
    pub is_output_format: Option<OutputFormatPredicate>,
    /// Reports whether the given ext is a content file extension.
    pub is_content_ext: Option<StrPredicate>,
}

impl fmt::Debug for PathParser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathParser")
            .field("language_index", &self.language_index)
            .field("is_lang_disabled", &self.is_lang_disabled.is_some())
            .field("is_output_format", &self.is_output_format.is_some())
            .field("is_content_ext", &self.is_content_ext.is_some())
            .finish()
    }
}

/// Go: `paths.Path`. Immutable after parsing (Go pools and resets them; Rust just allocates).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Path {
    pub(crate) s: String,
    pub(crate) pos_container_low: isize,
    pub(crate) pos_container_high: isize,
    pub(crate) pos_section_high: isize,
    pub(crate) component: String,
    pub(crate) path_type: PathType,
    pub(crate) identifiers_known: Vec<LowHigh>,
    pub(crate) identifiers_unknown: Vec<LowHigh>,
    pub(crate) pos_identifier_language: isize,
    pub(crate) pos_identifier_output_format: isize,
    pub(crate) pos_identifier_kind: isize,
    pub(crate) pos_identifier_layout: isize,
    pub(crate) pos_identifier_baseof: isize,
    pub(crate) disabled: bool,
    pub(crate) trim_leading_slash: bool,
    /// Go's `unnormalized *Path`: `None` is Go's `p.unnormalized = p` (the path itself).
    pub(crate) unnormalized: Option<Box<Path>>,
}

/// Go: `paths.NormalizePathStringBasic` — `strings.ToLower`, then " " -> "-". Nothing else.
// Go: common/paths/pathparser.go:NormalizePathStringBasic
pub fn normalize_path_string_basic(s: &str) -> String {
    // All lower case.
    let s = strings::to_lower(s.as_bytes());

    // Replace spaces with hyphens.
    let s = strings::replace_all(&s, b" ", b"-");

    String::from_utf8(s.into_owned()).expect("ToLower of valid UTF-8 is valid UTF-8")
}

impl PathParser {
    /// Go: `PathParser.ParseIdentity(c, s)`.
    // Go: common/paths/pathparser.go:ParseIdentity
    pub fn parse_identity(&self, c: &str, s: &str) -> StringIdentity {
        let p = self.parse_pooled(c, s);
        Identity(p.identifier_base())
    }

    /// Go: `ParseBaseAndBaseNameNoIdentifier` (used by `GetPage` ref normalisation).
    // Go: common/paths/pathparser.go:ParseBaseAndBaseNameNoIdentifier
    pub fn parse_base_and_base_name_no_identifier(
        &self,
        component: &str,
        s: &str,
    ) -> (String, String) {
        let p = self.parse_pooled(component, s);
        (p.base(), p.base_name_no_identifier().to_string())
    }

    // Go: common/paths/pathparser.go:parsePooled
    /// Go reuses a pooled `Path` whose `identifiersUnknown` are not reset; no caller reads them.
    fn parse_pooled(&self, c: &str, s: &str) -> Path {
        let s = normalize_path_string_basic(s);
        let mut p = new_path(c);
        self.do_parse(c, &s, &mut p);
        p
    }

    /// Go: `PathParser.Parse(component, s)` — `s` is slash-separated, relative to the component root.
    // Go: common/paths/pathparser.go:Parse
    pub fn parse(&self, component: &str, s: &str) -> Path {
        match self.try_parse(component, s) {
            Ok(p) => p,
            Err(e) => panic!("{e}"),
        }
    }

    /// Go: the unexported `parse` (Go's `doParse` never fails; the error is kept for the API).
    // Go: common/paths/pathparser.go:parse
    pub fn try_parse(&self, component: &str, s: &str) -> Result<Path> {
        let ss = normalize_path_string_basic(s);

        let mut p = new_path(component);
        self.do_parse(component, &ss, &mut p);

        if s != ss {
            // Preserve the original case for titles etc.
            let mut u = new_path(component);
            self.do_parse(component, s, &mut u);
            p.unnormalized = Some(Box::new(u));
        } else {
            p.unnormalized = None;
        }

        Ok(p)
    }

    fn call_is_output_format(&self, name: &str, ext: &str) -> bool {
        match &self.is_output_format {
            Some(f) => f(name, ext),
            None => panic!("runtime error: invalid memory address or nil pointer dereference"),
        }
    }

    fn call_is_content_ext(&self, ext: &str) -> bool {
        match &self.is_content_ext {
            Some(f) => f(ext),
            None => panic!("runtime error: invalid memory address or nil pointer dereference"),
        }
    }

    // Go: common/paths/pathparser.go:parseIdentifier
    #[allow(clippy::too_many_arguments)]
    fn parse_identifier(
        &self,
        component: &str,
        s: &str,
        p: &mut Path,
        i: usize,
        last_dot: usize,
        num_dots: usize,
        is_last: bool,
    ) {
        if p.pos_container_high != -1 {
            return;
        }
        let mut may_have_lang =
            num_dots > 1 && p.pos_identifier_language == -1 && self.language_index.is_some();
        may_have_lang = may_have_lang
            && (component == files::COMPONENT_FOLDER_CONTENT
                || component == files::COMPONENT_FOLDER_LAYOUTS);
        let may_have_output_format = component == files::COMPONENT_FOLDER_LAYOUTS;
        let may_have_kind = p.pos_identifier_kind == -1 && may_have_output_format;
        let may_have_layout = if p.path_type == PathType::Shortcode {
            !is_last && component == files::COMPONENT_FOLDER_LAYOUTS
        } else {
            component == files::COMPONENT_FOLDER_LAYOUTS
        };

        let mut found = false;
        let high = if !p.identifiers_known.is_empty() {
            last_dot
        } else {
            p.s.len()
        };
        let id = LowHigh { low: i + 1, high };
        // p.s == s here.
        let sid = &s[id.low..id.high];

        if p.identifiers_known.is_empty() {
            // The first is always the extension.
            p.identifiers_known.push(id);

            // May also be the output format.
            if may_have_output_format && self.call_is_output_format(sid, "") {
                p.pos_identifier_output_format = 0;
            }
        } else {
            if may_have_lang {
                let mut lang_found = self
                    .language_index
                    .as_ref()
                    .is_some_and(|m| m.contains_key(sid));
                if !lang_found {
                    let disabled = self.is_lang_disabled.as_ref().is_some_and(|f| f(sid));
                    if disabled {
                        p.disabled = true;
                        lang_found = true;
                    }
                }
                found = lang_found;
                if lang_found {
                    p.identifiers_known.push(id);
                    p.pos_identifier_language = p.identifiers_known.len() as isize - 1;
                }
            }

            if !found && may_have_output_format {
                // At this point we may already have resolved an output format,
                // but we need to keep looking for a more specific one, e.g. amp before html.
                // Use both name and extension to prevent
                // false positives on the form css.html.
                let ext = p.ext().to_string();
                if self.call_is_output_format(sid, &ext) {
                    found = true;
                    p.identifiers_known.push(id);
                    p.pos_identifier_output_format = p.identifiers_known.len() as isize - 1;
                }
            }

            if !found && may_have_kind && !kinds::get_kind_main(sid).is_empty() {
                found = true;
                p.identifiers_known.push(id);
                p.pos_identifier_kind = p.identifiers_known.len() as isize - 1;
            }

            if !found && sid == IDENTIFIER_BASEOF {
                found = true;
                p.identifiers_known.push(id);
                p.pos_identifier_baseof = p.identifiers_known.len() as isize - 1;
            }

            if !found && may_have_layout {
                p.identifiers_known.push(id);
                p.pos_identifier_layout = p.identifiers_known.len() as isize - 1;
                found = true;
            }

            if !found {
                p.identifiers_unknown.push(id);
            }
        }
    }

    // Go: common/paths/pathparser.go:doParse
    fn do_parse(&self, component: &str, s: &str, p: &mut Path) {
        // runtime.GOOS == "windows" is never true here.

        let mut s: String = if s.is_empty() {
            "/".to_string()
        } else {
            s.to_string()
        };

        // Leading slash, no trailing slash.
        if !s.starts_with('/') {
            s = format!("/{s}");
        }

        if s != "/" && s.ends_with('/') {
            s.pop();
        }

        p.s = s.clone();
        let s = s.as_str();
        let bs = s.as_bytes();
        let mut slash_count = 0;
        let mut last_dot = 0usize;
        let last_slash_idx = strings::last_index(bs, b"/");
        let num_dots = strings::count(&bs[(last_slash_idx + 1) as usize..], b".");
        if s.contains("/_shortcodes/") {
            p.path_type = PathType::Shortcode;
        }

        for i in (0..bs.len()).rev() {
            let c = bs[i];

            match c {
                b'.' => {
                    self.parse_identifier(component, s, p, i, last_dot, num_dots, false);
                    last_dot = i;
                }
                b'/' => {
                    slash_count += 1;
                    if p.pos_container_high == -1 {
                        if last_dot > 0 {
                            self.parse_identifier(component, s, p, i, last_dot, num_dots, true);
                        }
                        p.pos_container_high = i as isize + 1;
                    } else if p.pos_container_low == -1 {
                        p.pos_container_low = i as isize + 1;
                    }
                    if i > 0 {
                        p.pos_section_high = i as isize;
                    }
                }
                _ => {}
            }
        }

        if let Some(&id) = p.identifiers_known.last() {
            let is_content_component = p.component == files::COMPONENT_FOLDER_CONTENT
                || p.component == files::COMPONENT_FOLDER_ARCHETYPES;
            let is_content = is_content_component && {
                let ext = p.ext().to_string();
                self.call_is_content_ext(&ext)
            };

            if id.low as isize > p.pos_container_high {
                let b = &s[p.pos_container_high as usize..id.low - 1];
                if is_content {
                    p.path_type = match b {
                        "index" => PathType::Leaf,
                        "_index" => PathType::Branch,
                        _ => PathType::ContentSingle,
                    };

                    if slash_count == 2 && p.is_leaf_bundle() {
                        p.pos_section_high = 0;
                    }
                } else if b == files::NAME_CONTENT_DATA && files::is_content_data_ext(p.ext()) {
                    p.path_type = PathType::ContentData;
                }
            }
        }

        if p.path_type < PathType::Markup && component == files::COMPONENT_FOLDER_LAYOUTS {
            if p.pos_identifier_baseof != -1 {
                p.path_type = PathType::Baseof;
            } else {
                let pth = p.path();
                if pth.contains("/_shortcodes/") {
                    p.path_type = PathType::Shortcode;
                } else if pth.contains("/_markup/") {
                    p.path_type = PathType::Markup;
                } else if pth.starts_with("/_partials/") {
                    p.path_type = PathType::Partial;
                }
            }
        }

        if p.path_type == PathType::Shortcode && p.pos_identifier_layout != -1 {
            let id = p.identifiers_known[p.pos_identifier_layout as usize];
            if id.low as isize == p.pos_container_high {
                // First identifier is shortcode name.
                p.pos_identifier_layout = -1;
            }
        }
    }
}

// Go: common/paths/pathparser.go:newPath
fn new_path(component: &str) -> Path {
    let mut p = Path::default();
    p.reset();
    p.component = component.to_string();
    p
}

/// Go: `paths.ModifyPathBundleTypeResource` (files inside a leaf bundle other than its index).
// Go: common/paths/pathparser.go:ModifyPathBundleTypeResource
pub fn modify_path_bundle_type_resource(p: &mut Path) {
    if p.is_content() {
        p.path_type = PathType::ContentResource;
    } else {
        p.path_type = PathType::File;
    }
}

/// Go: `paths.HasExt` — whether the Unix styled path has an extension.
// Go: common/paths/pathparser.go:HasExt
pub fn has_ext(p: &str) -> bool {
    for &c in p.as_bytes().iter().rev() {
        if c == b'.' {
            return true;
        }
        if c == b'/' {
            return false;
        }
    }
    false
}

impl fmt::Display for Path {
    // Go: common/paths/pathparser.go:String
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.path())
    }
}

impl Path {
    // Go: common/paths/pathparser.go:reset
    fn reset(&mut self) {
        self.s.clear();
        self.pos_container_low = -1;
        self.pos_container_high = -1;
        self.pos_section_high = -1;
        self.component.clear();
        self.path_type = PathType::File;
        self.identifiers_known.clear();
        self.pos_identifier_language = -1;
        self.pos_identifier_output_format = -1;
        self.pos_identifier_kind = -1;
        self.pos_identifier_layout = -1;
        self.pos_identifier_baseof = -1;
        self.disabled = false;
        self.trim_leading_slash = false;
        self.unnormalized = None;
    }

    // Go: common/paths/pathparser.go:norm
    fn norm<'a>(&self, s: &'a str) -> &'a str {
        if self.trim_leading_slash {
            return s.strip_prefix('/').unwrap_or(s);
        }
        s
    }

    fn norm_owned(&self, s: String) -> String {
        if self.trim_leading_slash
            && let Some(t) = s.strip_prefix('/')
        {
            return t.to_string();
        }
        s
    }

    /// Go: `IdentifierBase` (satisfies `identity.Identity`): `Path()` for layouts, else `Base()`.
    // Go: common/paths/pathparser.go:IdentifierBase
    pub fn identifier_base(&self) -> String {
        if self.component() == files::COMPONENT_FOLDER_LAYOUTS {
            return self.path().to_string();
        }
        self.base()
    }

    /// The base name of the container directory for this path.
    // Go: common/paths/pathparser.go:Container
    pub fn container(&self) -> &str {
        if self.pos_container_low == -1 {
            return "";
        }
        self.norm(&self.s[self.pos_container_low as usize..self.pos_container_high as usize - 1])
    }

    /// The container directory for this path. For content bundles this will be the parent
    /// directory.
    // Go: common/paths/pathparser.go:ContainerDir
    pub fn container_dir(&self) -> &str {
        if self.pos_container_low == -1 || !self.is_bundle() {
            return self.dir();
        }
        self.norm(&self.s[..self.pos_container_low as usize - 1])
    }

    /// The first path element (section).
    // Go: common/paths/pathparser.go:Section
    pub fn section(&self) -> &str {
        if self.pos_section_high <= 0 {
            return "";
        }
        self.norm(&self.s[1..self.pos_section_high as usize])
    }

    /// Whether the path is a content file (e.g. mypost.md), also inside a bundle.
    // Go: common/paths/pathparser.go:IsContent
    pub fn is_content(&self) -> bool {
        self.path_type() >= PathType::ContentResource && self.path_type() <= PathType::ContentData
    }

    // Go: common/paths/pathparser.go:isContentPage
    /// Whether the path is a content file (e.g. mypost.md), but not if inside a leaf bundle.
    fn is_content_page(&self) -> bool {
        self.path_type() >= PathType::ContentSingle && self.path_type() <= PathType::ContentData
    }

    /// The last element of path.
    // Go: common/paths/pathparser.go:Name
    pub fn name(&self) -> &str {
        if self.pos_container_high > 0 {
            return &self.s[self.pos_container_high as usize..];
        }
        &self.s
    }

    /// The last element of path without any extension.
    // Go: common/paths/pathparser.go:NameNoExt
    pub fn name_no_ext(&self) -> &str {
        let i = self.identifier_index(0);
        if i != -1 {
            return &self.s
                [self.pos_container_high as usize..self.identifiers_known[i as usize].low - 1];
        }
        &self.s[self.pos_container_high as usize..]
    }

    /// The last element of path without any language identifier.
    // Go: common/paths/pathparser.go:NameNoLang
    pub fn name_no_lang(&self) -> String {
        let i = self.identifier_index(self.pos_identifier_language);
        if i == -1 {
            return self.name().to_string();
        }
        let id = self.identifiers_known[i as usize];
        format!(
            "{}{}",
            &self.s[self.pos_container_high as usize..id.low - 1],
            &self.s[id.high..]
        )
    }

    /// The logical base name for a resource without any identifier (e.g. no extension). For
    /// bundles this will be the containing directory's name, e.g. "blog".
    // Go: common/paths/pathparser.go:BaseNameNoIdentifier
    pub fn base_name_no_identifier(&self) -> &str {
        if self.is_bundle() {
            return self.container();
        }
        self.name_no_identifier()
    }

    /// The last element of path without any identifier (e.g. no extension).
    // Go: common/paths/pathparser.go:NameNoIdentifier
    pub fn name_no_identifier(&self) -> &str {
        let low_high = self.name_low_high();
        &self.s[low_high.low..low_high.high]
    }

    // Go: common/paths/pathparser.go:nameLowHigh
    fn name_low_high(&self) -> LowHigh {
        if let Some(&last_id) = self.identifiers_known.last() {
            if self.pos_container_high == last_id.low as isize {
                // The last identifier is the name.
                return last_id;
            }
            return LowHigh {
                low: self.pos_container_high as usize,
                high: last_id.low - 1,
            };
        }
        LowHigh {
            low: self.pos_container_high as usize,
            high: self.s.len(),
        }
    }

    /// All but the last element of path, typically the path's directory.
    // Go: common/paths/pathparser.go:Dir
    pub fn dir(&self) -> &str {
        let mut d = "";
        if self.pos_container_high > 0 {
            d = &self.s[..self.pos_container_high as usize - 1];
        }
        if d.is_empty() {
            d = "/";
        }
        self.norm(d)
    }

    /// The full normalized path, e.g. `/brands/alice/_index.md`.
    // Go: common/paths/pathparser.go:Path
    pub fn path(&self) -> &str {
        self.norm(&self.s)
    }

    /// The full path without the leading slash (Go: `p.Path()[1:]`).
    // Go: common/paths/pathparser.go:PathNoLeadingSlash
    pub fn path_no_leading_slash(&self) -> &str {
        &self.path()[1..]
    }

    /// The path as written on disk (original case and spaces).
    // Go: common/paths/pathparser.go:Unnormalized
    pub fn unnormalized(&self) -> &Path {
        self.unnormalized.as_deref().unwrap_or(self)
    }

    /// The Path but with any language identifier removed.
    // Go: common/paths/pathparser.go:PathNoLang
    pub fn path_no_lang(&self) -> String {
        self.base_impl(true, false)
    }

    /// The Path but with any identifier (ext, lang) removed.
    // Go: common/paths/pathparser.go:PathNoIdentifier
    pub fn path_no_identifier(&self) -> String {
        self.base_impl(false, false)
    }

    /// The path up to the first identifier that is not a language or output format.
    // Go: common/paths/pathparser.go:PathBeforeLangAndOutputFormatAndExt
    pub fn path_before_lang_and_output_format_and_ext(&self) -> String {
        if self.identifiers_known.is_empty() {
            return self.norm(&self.s).to_string();
        }
        let mut i = self.identifier_index(0);

        let j = self.pos_identifier_output_format;
        if i == -1 || (j != -1 && j < i) {
            i = j;
        }
        let j = self.pos_identifier_language;
        if i == -1 || (j != -1 && j < i) {
            i = j;
        }

        if i == -1 {
            return self.norm(&self.s).to_string();
        }

        let id = self.identifiers_known[i as usize];
        self.norm(&self.s[..id.low - 1]).to_string()
    }

    /// The path relative to the given owner.
    // Go: common/paths/pathparser.go:PathRel
    pub fn path_rel(&self, owner: &Path) -> String {
        let mut ob = owner.base();
        if !ob.ends_with('/') {
            ob.push('/');
        }
        let p = self.path();
        p.strip_prefix(ob.as_str()).unwrap_or(p).to_string()
    }

    /// The base path relative to the given owner (Go: `p.Base()[len(ob)+1:]`).
    // Go: common/paths/pathparser.go:BaseRel
    pub fn base_rel(&self, owner: &Path) -> String {
        let mut ob = owner.base();
        if ob == "/" {
            ob = String::new();
        }
        self.base()[ob.len() + 1..].to_string()
    }

    /// The tree key. For content files, the path without any identifiers (extension, language
    /// code etc.); any `index`/`_index` as the last path element is ignored. For other files
    /// (resources), any extension is kept. Home is `/` (its tree key is `""`, see hugolib
    /// `cleanTreeKey`).
    // Go: common/paths/pathparser.go:Base
    pub fn base(&self) -> String {
        self.base_impl(!self.is_content_page(), self.is_bundle())
    }

    /// Go: `BaseReTyped(typ)` — first path segment replaced by the front-matter `type` (layout
    /// lookup).
    // Go: common/paths/pathparser.go:BaseReTyped
    pub fn base_re_typed(&self, typ: &str) -> String {
        let base = self.base();
        if typ.is_empty() || self.section() == typ {
            return base;
        }
        let mut d = format!("/{typ}");
        if self.pos_section_high != -1 {
            d.push_str(&base[self.pos_section_high as usize..]);
        }
        self.norm_owned(d)
    }

    /// The base path without the leading slash (Go: `p.Base()[1:]`).
    // Go: common/paths/pathparser.go:BaseNoLeadingSlash
    pub fn base_no_leading_slash(&self) -> String {
        self.base()[1..].to_string()
    }

    // Go: common/paths/pathparser.go:base
    fn base_impl(&self, preserve_ext: bool, is_bundle: bool) -> String {
        if self.identifiers_known.is_empty() {
            return self.norm(&self.s).to_string();
        }

        if preserve_ext && self.identifiers_known.len() == 1 {
            // Preserve extension.
            return self.norm(&self.s).to_string();
        }

        let mut high = if is_bundle {
            self.pos_container_high as usize - 1
        } else {
            self.name_low_high().high
        };

        if high == 0 {
            high += 1;
        }

        if !preserve_ext {
            return self.norm(&self.s[..high]).to_string();
        }

        // For txt files etc. we want to preserve the extension.
        let id = self.identifiers_known[0];

        self.norm_owned(format!(
            "{}{}",
            &self.s[..high],
            &self.s[id.low - 1..id.high]
        ))
    }

    // Go: common/paths/pathparser.go:Ext
    pub fn ext(&self) -> &str {
        self.identifier_as_string(0)
    }
    // Go: common/paths/pathparser.go:OutputFormat
    pub fn output_format(&self) -> &str {
        self.identifier_as_string(self.pos_identifier_output_format)
    }
    // Go: common/paths/pathparser.go:Kind
    pub fn kind(&self) -> &str {
        self.identifier_as_string(self.pos_identifier_kind)
    }
    // Go: common/paths/pathparser.go:Layout
    pub fn layout(&self) -> &str {
        self.identifier_as_string(self.pos_identifier_layout)
    }
    // Go: common/paths/pathparser.go:Lang
    pub fn lang(&self) -> &str {
        self.identifier_as_string(self.pos_identifier_language)
    }
    // Go: common/paths/pathparser.go:Identifier
    pub fn identifier(&self, i: isize) -> &str {
        self.identifier_as_string(i)
    }
    // Go: common/paths/pathparser.go:Disabled
    pub fn disabled(&self) -> bool {
        self.disabled
    }
    // Go: common/paths/pathparser.go:Identifiers
    pub fn identifiers(&self) -> Vec<&str> {
        self.identifiers_known
            .iter()
            .map(|id| &self.s[id.low..id.high])
            .collect()
    }
    // Go: common/paths/pathparser.go:IdentifiersUnknown
    pub fn identifiers_unknown(&self) -> Vec<&str> {
        self.identifiers_unknown
            .iter()
            .map(|id| &self.s[id.low..id.high])
            .collect()
    }
    // Go: common/paths/pathparser.go:Type
    pub fn path_type(&self) -> PathType {
        self.path_type
    }
    // Go: common/paths/pathparser.go:IsBundle
    /// Leaf, branch or content data (Go: `TypeLeaf <= t <= TypeContentData`).
    pub fn is_bundle(&self) -> bool {
        self.path_type >= PathType::Leaf && self.path_type <= PathType::ContentData
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
    /// Go: `(p Path) ForType(t)` — a copy with another type. The copy's `Unnormalized()` is the
    /// original path (Go copies the pointer, which points at the original).
    // Go: common/paths/pathparser.go:ForType
    pub fn for_type(&self, t: PathType) -> Path {
        let mut c = self.copy_value();
        c.path_type = t;
        c
    }
    // Go: common/paths/pathparser.go:Component
    pub fn component(&self) -> &str {
        &self.component
    }
    /// Go: `(p Path) TrimLeadingSlash()` — a copy with the leading slash removed.
    // Go: common/paths/pathparser.go:TrimLeadingSlash
    pub fn trim_leading_slash(&self) -> Path {
        let mut c = self.copy_value();
        c.trim_leading_slash = true;
        c
    }

    /// Go's struct copy `p2 := *p`: the `unnormalized` pointer keeps pointing at the original.
    fn copy_value(&self) -> Path {
        let mut c = self.clone();
        if self.unnormalized.is_none() {
            c.unnormalized = Some(Box::new(self.clone()));
        }
        c
    }

    // Go: common/paths/pathparser.go:identifierAsString
    fn identifier_as_string(&self, i: isize) -> &str {
        let i = self.identifier_index(i);
        if i == -1 {
            return "";
        }

        let id = self.identifiers_known[i as usize];
        &self.s[id.low..id.high]
    }

    // Go: common/paths/pathparser.go:identifierIndex
    fn identifier_index(&self, i: isize) -> isize {
        if i < 0 || i >= self.identifiers_known.len() as isize {
            return -1;
        }
        i
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/paths/pathparser.go (787 lines; 50/60 funcs executed)
//   types: PathParser, Type, Path
// OK L50-58: NormalizePathStringBasic(s string) string
// OK L61-65: (pp *PathParser) ParseIdentity(c, s string) identity.StringIdentity
// OK L68-72: (pp *PathParser) ParseBaseAndBaseNameNoIdentifier(c, s string) (string, string)
// OK L74-83: (pp *PathParser) parsePooled(c, s string) *Path
// OK L86-92: (pp *PathParser) Parse(c, s string) *Path
// OK L94-99: (pp *PathParser) newPath(component string) *Path
// OK L101-121: (pp *PathParser) parse(component, s string) (*Path, error)
// OK L123-214: (pp *PathParser) parseIdentifier(component, s string, p *Path, i, lastDot, numDots int, isLast bool)
// OK L216-319: (pp *PathParser) doParse(component, s string, p *Path) (*Path, error)
// OK L321-327: ModifyPathBundleTypeResource(p *Path)
// OK L397-399: getPath() *Path (no pool: parse_pooled allocates)
// OK L401-404: putPath(p *Path) (no pool)
// OK L406-422: (p *Path) reset()
// OK L425-428: (p Path) TrimLeadingSlash() *Path
// OK L430-435: (p *Path) norm(s string) string
// OK L438-443: (p *Path) IdentifierBase() string
// OK L446-448: (p *Path) Component() string
// OK L451-456: (p *Path) Container() string
// OK L458-463: (p *Path) String() string (Display)
// OK L467-472: (p *Path) ContainerDir() string
// OK L475-480: (p *Path) Section() string
// OK L484-486: (p *Path) IsContent() bool
// OK L490-492: (p *Path) isContentPage() bool
// OK L495-500: (p *Path) Name() string
// OK L503-508: (p *Path) NameNoExt() string
// OK L511-518: (p *Path) NameNoLang() string
// OK L522-527: (p *Path) BaseNameNoIdentifier() string
// OK L530-533: (p *Path) NameNoIdentifier() string
// OK L535-551: (p *Path) nameLowHigh() types.LowHigh[string]
// OK L554-563: (p *Path) Dir() (d string)
// OK L566-568: (p *Path) Path() (d string)
// OK L571-573: (p *Path) PathNoLeadingSlash() string
// OK L576-578: (p *Path) Unnormalized() *Path
// OK L581-583: (p *Path) PathNoLang() string
// OK L586-588: (p *Path) PathNoIdentifier() string
// OK L591-610: (p *Path) PathBeforeLangAndOutputFormatAndExt() string
// OK L613-619: (p *Path) PathRel(owner *Path) string
// OK L622-628: (p *Path) BaseRel(owner *Path) string
// OK L634-636: (p *Path) Base() string
// OK L640-651: (p *Path) BaseReTyped(typ string) (d string)
// OK L654-656: (p *Path) BaseNoLeadingSlash() string
// OK L658-688: (p *Path) base(preserveExt, isBundle bool) string
// OK L690-692: (p *Path) Ext() string
// OK L694-696: (p *Path) OutputFormat() string
// OK L698-700: (p *Path) Kind() string
// OK L702-704: (p *Path) Layout() string
// OK L706-708: (p *Path) Lang() string
// OK L710-712: (p *Path) Identifier(i int) string
// OK L714-716: (p *Path) Disabled() bool
// OK L718-724: (p *Path) Identifiers() []string
// OK L726-732: (p *Path) IdentifiersUnknown() []string
// OK L734-736: (p *Path) Type() Type
// OK L738-740: (p *Path) IsBundle() bool
// OK L742-744: (p *Path) IsBranchBundle() bool
// OK L746-748: (p *Path) IsLeafBundle() bool
// OK L750-752: (p *Path) IsContentData() bool
// OK L754-757: (p Path) ForType(t Type) *Path
// OK L759-767: (p *Path) identifierAsString(i int) string
// OK L769-774: (p *Path) identifierIndex(i int) int
// OK L777-787: HasExt(p string) bool
// Source: common/paths/type_string.go (32 lines; 0/2 funcs executed)
// OK L7-21: _()
// OK L27-32: (i Type) String() string
// ---------------------------------------------------------------------------
