//! Port of `tpl/tplimpl/templatestore.go`.
//!
//! Owner: Wave B task T13 (tplimpl).


//! Go `tpl/tplimpl/templatestore.go`: the template store. Built once (layouts fs + embedded
//! templates), then `WithSiteOpts` per site (own func map + exec helper). Lookups:
//! `LookupPagesLayout` (tree walk from "" to the query path with descriptor weights, then the
//! baseof variant), `LookupPartial`, `LookupShortcode`, `TextParse` (ExecuteAsTemplate).

use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, OnceLock};

use go_value::Value;
use nh_common::object::NamedTypeRegistry;
use nh_common::paths::pathparser::{Path, PathParser};
use nh_common::Result;
use nh_doctree::simpletree::SimpleTree;
use nh_hugofs::afero::Fs;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_markup::goldmark::goldmark_config::RenderHooks;
use nh_media::media::media_type::Types as MediaTypes;
use nh_media::output::output_format::{Formats, OutputFormat};
use nh_page::site::SiteRef;
use nh_tpl::template::TplContext;

use crate::category::{Category, SubCategory};
use crate::engine::{Executer, FuncMap, Template};
use crate::template_info::ParseInfo;
use crate::templatedescriptor::{DescriptorHandler, TemplateDescriptor, Weight};

pub const LAYOUT_ALL: &str = "all";
pub const LAYOUT_LIST: &str = "list";
pub const LAYOUT_SINGLE: &str = "single";

/// Go: `tplimpl.StoreOptions`.
#[derive(Clone)]
pub struct StoreOptions {
    /// The filesystem to use (the layouts component fs).
    pub fs: Arc<dyn Fs>,
    pub path_parser: Arc<PathParser>,
    /// Set when output formats need to be looked up (and for per-format embedded render-table copies).
    pub output_formats: Formats,
    pub media_types: MediaTypes,
    pub default_content_language: String,
    pub default_output_format: String,
    /// Taxonomy config (singular -> plural) for legacy mappings.
    pub taxonomy_singular_plural: BTreeMap<String, String>,
    pub watching: bool,
    /// `useEmbedded` link/image hook config (auto -> fallback for multilingual single-host).
    pub render_hooks: RenderHooks,
    /// Methods of named list/map types (page.Pages, maps.Params, ...), built by nh-hugolib.
    pub named_types: Arc<NamedTypeRegistry>,
}

/// Go: `tplimpl.SiteOptions`.
#[derive(Clone)]
pub struct SiteOptions {
    /// The site (for the `mainsections` special case). Set once HugoSites is frozen.
    pub site: Arc<OnceLock<SiteRef>>,
    pub template_funcs: Arc<FuncMap>,
}

/// Go: `tplimpl.TemplInfo`.
pub struct TemplInfo {
    pub category: Category,
    pub sub_category: SubCategory,
    pub path_info: Arc<Path>,
    /// Nil for embedded / inline templates.
    pub fi: Option<FileMetaInfo>,
    /// The template content (BOM removed; CRLF->LF for embedded).
    pub(crate) content: String,
    /// The parsed (and, after `prepareTemplates`, escaped) template.
    pub template: OnceLock<Template>,
    /// If true, this template will not be combined with any base template.
    pub(crate) no_base_of: bool,
    /// Base template variants (key: base tree key + descriptor).
    pub(crate) base_variants: SimpleTree<BTreeMap<TemplateDescriptor, Arc<TemplWithBaseApplied>>>,
    /// The template this template is overlaid on (for baseof-applied variants).
    pub(crate) base: Option<Arc<TemplInfo>>,
    pub d: TemplateDescriptor,
    pub parse_info: ParseInfo,
    pub(crate) execution_counter: AtomicU64,
    pub(crate) is_legacy_mapped: bool,
}

impl TemplInfo {
    // Go: tpl/tplimpl/templatestore.go:Name
    pub fn name(&self) -> String {
        todo!()
    }

    /// Go: `Filename()` ("" for embedded).
    pub fn filename(&self) -> String {
        todo!()
    }

    // Go: tpl/tplimpl/templatestore.go:Prepare
    pub fn prepare(&self) -> Result<Template> {
        todo!()
    }
}

/// Go: `tplimpl.TemplWithBaseApplied`.
pub struct TemplWithBaseApplied {
    /// The template that's overlaid on top of the base template.
    pub overlay: Arc<TemplInfo>,
    /// The base template.
    pub base: Arc<TemplInfo>,
    /// This is the final template that can be used to render a page.
    pub template: Arc<TemplInfo>,
}

/// Go: `tplimpl.TemplateQuery`.
#[derive(Clone)]
pub struct TemplateQuery {
    /// The path to walk down to (e.g. `/snacks/koalas-march` via `PathInfo().BaseReTyped(type)`).
    pub path: String,
    /// The name to look for. Used for shortcode lookups.
    pub name: String,
    pub category: Category,
    /// The template descriptor to match against.
    pub desc: TemplateDescriptor,
    /// Optional filter (render hooks: consider only candidates of the right variant/format).
    pub consider: Option<Arc<dyn Fn(&TemplInfo) -> bool + Send + Sync>>,
}

/// Go: `bestMatch` (tie-breaks in `isBetter`).
pub(crate) struct BestMatch {
    pub(crate) templ: Option<Arc<TemplInfo>>,
    pub(crate) desc: TemplateDescriptor,
    pub(crate) w: Weight,
    pub(crate) key: String,
    pub(crate) default_output_format: String,
}

/// Go: `nodeKey`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeKey {
    pub c: Category,
    pub d: TemplateDescriptor,
}

/// The shared, parsed part of the store (Go: everything except `storeSite`).
pub struct StoreShared {
    pub opts: StoreOptions,
    pub html_format: OutputFormat,
    pub(crate) tree_main: SimpleTree<BTreeMap<NodeKey, Arc<TemplInfo>>>,
    pub(crate) tree_shortcodes: SimpleTree<BTreeMap<String, BTreeMap<TemplateDescriptor, Arc<TemplInfo>>>>,
    pub(crate) templates_by_path: std::sync::Mutex<BTreeMap<String, Arc<TemplInfo>>>,
    pub(crate) shortcodes_by_name: std::sync::Mutex<BTreeMap<String, Arc<TemplInfo>>>,
    pub(crate) dh: DescriptorHandler,
    pub(crate) cache_lookup_partials: std::sync::Mutex<BTreeMap<String, Option<Arc<TemplInfo>>>>,
}

/// Go: `storeSite` — per-site func map + exec helper + executer.
pub struct StoreSite {
    pub opts: SiteOptions,
    pub exec_helper: Arc<crate::template_funcs::TemplateExecHelper>,
    pub executer: Arc<dyn Executer>,
}

/// Go: `tplimpl.TemplateStore` (a per-site view over the shared store).
#[derive(Clone)]
pub struct TemplateStore {
    pub shared: Arc<StoreShared>,
    pub store_site: Arc<StoreSite>,
}

impl TemplateStore {
    /// Go: `tplimpl.NewStore(opts, siteOpts)` — init legacy mappings, insertTemplates (layouts
    /// fs), insertEmbedded, parseTemplates (noBaseOf first, then baseof variants, then
    /// shortcodes), extractInlinePartials, transformTemplates, prepareTemplates (escape all).
    // Go: tpl/tplimpl/templatestore.go:NewStore
    pub fn new(opts: StoreOptions, site_opts: SiteOptions) -> Result<TemplateStore> {
        todo!()
    }

    /// Go: `WithSiteOpts(opts)` — same parsed templates, another site's funcs.
    // Go: tpl/tplimpl/templatestore.go:WithSiteOpts
    pub fn with_site_opts(&self, opts: SiteOptions) -> TemplateStore {
        todo!()
    }

    /// Go: `ExecuteWithContext(ctx, ti, wr, data)` — pushes CurrentTemplateInfo (level+1, max 999).
    // Go: tpl/tplimpl/templatestore.go:ExecuteWithContext
    pub fn execute_with_context(&self, ctx: &TplContext, ti: &Arc<TemplInfo>, w: &mut Vec<u8>, data: &Value) -> Result<()> {
        todo!()
    }

    // Go: tpl/tplimpl/templatestore.go:LookupPagesLayout
    pub fn lookup_pages_layout(&self, q: &TemplateQuery) -> Option<Arc<TemplInfo>> {
        todo!()
    }

    /// Go: `LookupPartial(pth)` — parsed as a layouts path of type partial; no extension -> html.
    // Go: tpl/tplimpl/templatestore.go:LookupPartial
    pub fn lookup_partial(&self, pth: &str) -> Option<Arc<TemplInfo>> {
        todo!()
    }

    // Go: tpl/tplimpl/templatestore.go:LookupShortcodeByName
    pub fn lookup_shortcode_by_name(&self, name: &str) -> Option<Arc<TemplInfo>> {
        todo!()
    }

    // Go: tpl/tplimpl/templatestore.go:LookupShortcode
    pub fn lookup_shortcode(&self, q: &TemplateQuery) -> Result<Option<Arc<TemplInfo>>> {
        todo!()
    }

    /// Go: `TextParse(name, tpl)` — a standalone text/template (resources.ExecuteAsTemplate).
    // Go: tpl/tplimpl/templatestore.go:TextParse
    pub fn text_parse(&self, name: &str, src: &str) -> Result<Arc<TemplInfo>> {
        todo!()
    }

    // Go: tpl/tplimpl/templatestore.go:TextLookup
    pub fn text_lookup(&self, name: &str) -> Option<Arc<TemplInfo>> {
        todo!()
    }

    /// Go: `GetFunc(name)`.
    pub fn get_func(&self, name: &str) -> Option<crate::engine::TplFunc> {
        todo!()
    }

    /// Go: `UnusedTemplates()` (`--printUnusedTemplates`).
    pub fn unused_templates(&self) -> Vec<Arc<TemplInfo>> {
        todo!()
    }
}

/// Go: `TemplateStoreProvider`.
pub trait TemplateStoreProvider {
    fn get_template_store(&self) -> TemplateStore;
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/templatestore.go (2096 lines; 43/70 funcs executed)
//   types: Category, SiteOptions, StoreOptions, SubCategory, TemplInfo, TemplWithBaseApplied, TemplateQuery,
//          TemplateStore, PathTemplateDescriptor, TemplateStoreProvider, TextTemplatHandler, bestMatch, byPath,
//          keyTemplateInfo, nodeKey, processingState, storeSite, weight
// EX L112-164: NewStore(opts StoreOptions, siteOpts SiteOptions) (*TemplateStore, error)
//    L265-267: (ti *TemplInfo) SubCategory() SubCategory
// EX L269-280: (ti *TemplInfo) BaseVariantsSeq() iter.Seq[*TemplWithBaseApplied]
//    L282-287: (t *TemplInfo) IdentifierBase() string
//    L289-291: (t *TemplInfo) GetIdentity() identity.Identity
// EX L293-300: (ti *TemplInfo) Name() string
// EX L302-304: (ti *TemplInfo) Prepare() (*texttemplate.Template, error)
//    L306-308: (t *TemplInfo) IsProbablyDependency(other identity.Identity) bool
//    L310-317: (t *TemplInfo) IsProbablyDependent(other identity.Identity) bool
//    L319-324: (ti *TemplInfo) String() string
// EX L326-347: (ti *TemplInfo) findBestMatchBaseof(s *TemplateStore, d1 TemplateDescriptor, k1 string, slashCountK1 int, best *bestMatch)
//    L349-359: (t *TemplInfo) isProbablyTheSameIDAs(other identity.Identity) bool
//    L362-364: (ti *TemplInfo) Base() tpl.CurrentTemplateInfoCommonOps
//    L366-371: (ti *TemplInfo) Filename() string
// EX L400-425: (q *TemplateQuery) init()
//    L457-459: (s *TemplateStore) NewFromOpts() (*TemplateStore, error)
// EX L468-485: (s *TemplateStore) FindAllBaseTemplateCandidates(overlayKey string, desc TemplateDescriptor) []keyTemplateInfo
// EX L487-524: (t *TemplateStore) ExecuteWithContext(ctx context.Context, ti *TemplInfo, wr io.Writer, data any) error
//    L526-529: (t *TemplateStore) GetFunc(name string) (reflect.Value, bool)
//    L531-538: (s *TemplateStore) GetIdentity(p string) identity.Identity
//    L540-543: (t *TemplateStore) LookupByPath(templatePath string) *TemplInfo
// EX L551-556: (s *TemplateStore) getBest() *bestMatch
// EX L558-561: (s *TemplateStore) putBest(b *bestMatch)
// EX L563-584: (s *TemplateStore) LookupPagesLayout(q TemplateQuery) *TemplInfo
// EX L586-607: (s *TemplateStore) LookupPartial(pth string) *TemplInfo
// EX L609-616: (s *TemplateStore) LookupShortcodeByName(name string) *TemplInfo
// EX L618-671: (s *TemplateStore) LookupShortcode(q TemplateQuery) (*TemplInfo, error)
//    L674-702: (s *TemplateStore) PrintDebug(prefix string, category Category, w io.Writer)
//    L704-706: (s *TemplateStore) clearCaches()
//    L709-738: (s *TemplateStore) RefreshFiles(include func(fi hugofs.FileMetaInfo) bool) error
//    L740-744: (s *TemplateStore) HasTemplate(templatePath string) bool
//    L746-754: (t *TemplateStore) TextLookup(name string) *TemplInfo
// EX L756-764: (t *TemplateStore) TextParse(name, tpl string) (*TemplInfo, error)
//    L766-781: (t *TemplateStore) UnusedTemplates() []*TemplInfo
// EX L786-790: (s TemplateStore) WithSiteOpts(opts SiteOptions) *TemplateStore
// EX L792-814: (s *TemplateStore) findBestMatchGet(key string, category Category, consider func(candidate *TemplInfo) bool, desc TemplateDescriptor, best *bestMatch)
// EX L816-821: (s *TemplateStore) inPath(k1, k2 string) bool
// EX L823-852: (s *TemplateStore) findBestMatchWalkPath(q TemplateQuery, k1 string, slashCountK1 int, best *bestMatch)
//    L854-887: (t *TemplateStore) addDeferredTemplate(owner *TemplInfo, name string, n *parse.ListNode) error
//    L889-942: (s *TemplateStore) addFileContext(ti *TemplInfo, what string, inerr error) error
//    L944-951: (s *TemplateStore) extractIdentifiers(line string) []string
// EX L953-991: (s *TemplateStore) extractInlinePartials(rebuild bool) error
// EX L993-1022: (s *TemplateStore) allRawTemplates() iter.Seq[tpl.Template]
// EX L1024-1098: (s *TemplateStore) insertEmbedded() error
// EX L1100-1102: (s *TemplateStore) setTemplateByPath(p string, ti *TemplInfo)
// EX L1104-1147: (s *TemplateStore) insertShortcode(pi *paths.Path, fi hugofs.FileMetaInfo, replace bool, tree doctree.Tree[map[string]map[TemplateDescriptor]*Templ...
// EX L1149-1164: (s *TemplateStore) insertTemplate(pi *paths.Path, fi hugofs.FileMetaInfo, subCategory SubCategory, replace bool, tree doctree.Tree[map[nodeKey]*Tem...
// EX L1166-1238: (s *TemplateStore) insertTemplate2( pi *paths.Path, fi hugofs.FileMetaInfo, key string, category Category, subCategory SubCategory, d TemplateDescr...
// EX L1240-1513: (s *TemplateStore) insertTemplates(include func(fi hugofs.FileMetaInfo) bool, partialRebuild bool) error
// EX L1515-1521: (s *TemplateStore) key(dir string) string
//    L1523-1529: (s *TemplateStore) createTemplatesSnapshot() error
// EX L1531-1596: (s *TemplateStore) parseTemplates(replace bool) error
// EX L1599-1609: (s *TemplateStore) prepareTemplates() error
// EX L1620-1650: (s *TemplateStore) resolveOutputFormatAndOrMediaType(ofs, mns string) (output.Format, media.Type)
// EX L1655-1682: (s *TemplateStore) templates() iter.Seq[*TemplInfo]
// EX L1684-1781: (s *TemplateStore) toKeyCategoryAndDescriptor(p *paths.Path) (string, string, Category, TemplateDescriptor, error)
// EX L1783-1824: (s *TemplateStore) transformTemplates() error
// EX L1826-1898: (s *TemplateStore) init() error
// EX L1921-1927: (best *bestMatch) reset()
//    L1929-1938: (best *bestMatch) candidatesAsStringSlice() []string
// EX L1940-1993: (best *bestMatch) isBetter(w weight, ti *TemplInfo) bool
// EX L1995-2000: (best *bestMatch) updateValues(w weight, key string, k TemplateDescriptor, vv *TemplInfo)
//    L2004-2004: (a byPath) Len() int
//    L2005-2007: (a byPath) Less(i, j int) bool
//    L2009-2009: (a byPath) Swap(i, j int)
// EX L2037-2044: isLayoutStandard(s string) bool
// EX L2046-2048: (w weight) isEqualWeights(other weight) bool
// EX L2050-2088: configureSiteStorage(opts SiteOptions, watching bool) *storeSite
// EX L2090-2092: isBackupFile(path string) bool
// EX L2094-2096: isDotFile(path string) bool
// ---------------------------------------------------------------------------
