//! Port of `tpl/tplimpl/templatestore.go`.
//!
//! Owner: Wave B task T13 (tplimpl).

//! Go `tpl/tplimpl/templatestore.go`: the template store. Built once (layouts fs + embedded
//! templates), then `WithSiteOpts` per site (own func map + exec helper). Lookups:
//! `LookupPagesLayout` (tree walk from "" to the query path with descriptor weights, then the
//! baseof variant), `LookupPartial`, `LookupShortcode`, `TextParse` (ExecuteAsTemplate).
//!
//! Go's `*TemplInfo` is a pointer that the construction steps mutate after it has been put into
//! the trees (content, parsed template, base variants, transform results). Here the mutable part
//! is behind a lock (`TemplInfo::state`); after `TemplateStore::new` only the execution counter
//! changes. Go's maps keyed by `nodeKey`/`TemplateDescriptor` iterate in random order; here they
//! are `BTreeMap`s (one of Go's possible orders).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use go_value::Value;
use nh_common::Result;
use nh_common::files::COMPONENT_FOLDER_LAYOUTS;
use nh_common::herrors::Error;
use nh_common::kinds::{KIND_HOME, KIND_PAGE, KIND_SECTION, KIND_TEMPORARY, get_kind_main};
use nh_common::loggers::Logger;
use nh_common::object::NamedTypeRegistry;
use nh_common::paths::pathparser::{Path, PathParser, PathType, has_ext};
use nh_doctree::simpletree::SimpleTree;
use nh_hugofs::afero::Fs;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_markup::goldmark::goldmark_config::RenderHooks;
use nh_media::media::media_type::{MediaType, Types as MediaTypes};
use nh_media::output::output_format::{Formats, OutputFormat};
use nh_page::site::SiteRef;
use nh_tpl::template::{CurrentTemplateBase, TplContext};

use crate::category::{Category, SubCategory};
use crate::embedded::EMBEDDED_TEMPLATES;
use crate::engine::{Executer, FuncMap, GoExecuter, Template, TextTemplate};
use crate::legacy::{
    LayoutLegacyMapping, LegacyOrdinalMapping, LegacyOrdinalMappingFi, LegacyTargetPathIdentifiers,
    legacy_section_mappings, legacy_taxonomy_mappings, legacy_term_mappings,
};
use crate::template_funcs::TemplateExecHelper;
use crate::template_info::ParseInfo;
use crate::templatedescriptor::{BASE_NAME_BASEOF, DescriptorHandler, TemplateDescriptor, Weight};
use crate::templates::{TemplateNamespace, is_text, new_template_namespace};
use crate::templatetransform::apply_template_transformers;

pub const LAYOUT_ALL: &str = "all";
pub const LAYOUT_LIST: &str = "list";
pub const LAYOUT_SINGLE: &str = "single";

// Go: tpl/tplimpl/templatestore.go:containerMarkup etc.
const CONTAINER_MARKUP: &str = "_markup";
const CONTAINER_SHORTCODES: &str = "_shortcodes";
const CONTAINER_PARTIALS: &str = "_partials";

/// Go: `tplimpl.StoreOptions`.
#[derive(Clone)]
pub struct StoreOptions {
    /// The filesystem to use (the layouts component fs).
    pub fs: Arc<dyn Fs>,
    /// The logger to use (Go `Log`; `None` logs nothing).
    pub log: Option<Logger>,
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

/// Go: `processingState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ProcessingState {
    #[default]
    Initial,
    Transformed,
}

/// The fields of a Go `*TemplInfo` that the store construction mutates in place.
#[derive(Default)]
pub(crate) struct TemplInfoState {
    pub(crate) sub_category: SubCategory,
    /// The template content with any leading BOM removed (Go string bytes).
    pub(crate) content: Vec<u8>,
    /// The parsed template. Note that any baseof template will be applied later.
    pub(crate) template: Option<Template>,
    /// If no baseof is needed, this will be set to true.
    pub(crate) no_base_of: bool,
    /// If `no_base_of` is false, we will look for the final template in this tree.
    pub(crate) base_variants:
        Option<SimpleTree<BTreeMap<TemplateDescriptor, Arc<TemplWithBaseApplied>>>>,
    /// The template variants that are based on this template.
    pub(crate) overlays: Vec<Arc<TemplInfo>>,
    /// The descriptor that this template represents.
    pub(crate) d: TemplateDescriptor,
    /// Parser state.
    pub(crate) parse_info: ParseInfo,
    pub(crate) state: ProcessingState,
    pub(crate) is_legacy_mapped: bool,
}

/// Go: `tplimpl.TemplInfo`.
pub struct TemplInfo {
    /// The category of this template (`None` is Go's zero category: base-applied variants,
    /// deferred and standalone text templates).
    pub category: Option<Category>,
    /// Path info (nil for templates created by `TextParse`/`TextLookup`).
    pub path_info: Option<Arc<Path>>,
    /// Set when backed by a file.
    pub fi: Option<FileMetaInfo>,
    /// The base template used, if any.
    pub(crate) base: Option<Arc<TemplInfo>>,
    /// The execution counter for this template.
    pub(crate) execution_counter: AtomicU64,
    pub(crate) m: RwLock<TemplInfoState>,
}

impl TemplInfo {
    pub(crate) fn state(&self) -> RwLockReadGuard<'_, TemplInfoState> {
        self.m.read().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn state_mut(&self) -> RwLockWriteGuard<'_, TemplInfoState> {
        self.m.write().unwrap_or_else(|e| e.into_inner())
    }

    fn new_info(
        category: Option<Category>,
        path_info: Option<Arc<Path>>,
        fi: Option<FileMetaInfo>,
        st: TemplInfoState,
    ) -> Arc<TemplInfo> {
        Arc::new(TemplInfo {
            category,
            path_info,
            fi,
            base: None,
            execution_counter: AtomicU64::new(0),
            m: RwLock::new(st),
        })
    }

    // Go: tpl/tplimpl/templatestore.go:SubCategory
    pub fn sub_category(&self) -> SubCategory {
        self.state().sub_category
    }

    /// Go: `ti.D`.
    pub fn d(&self) -> TemplateDescriptor {
        self.state().d.clone()
    }

    /// Go: `ti.ParseInfo`.
    pub fn parse_info(&self) -> ParseInfo {
        self.state().parse_info.clone()
    }

    /// Go: `ti.Template`.
    pub fn template(&self) -> Option<Template> {
        self.state().template.clone()
    }

    /// The template content (BOM removed).
    pub fn content(&self) -> Vec<u8> {
        self.state().content.clone()
    }

    /// Go: `ti.noBaseOf`.
    pub fn no_base_of(&self) -> bool {
        self.state().no_base_of
    }

    /// Go: `ti.isLegacyMapped`.
    pub fn is_legacy_mapped(&self) -> bool {
        self.state().is_legacy_mapped
    }

    /// Go: `ti.base` — the base template (baseof) this variant was applied to.
    pub fn base_template(&self) -> Option<&Arc<TemplInfo>> {
        self.base.as_ref()
    }

    /// Go: `ti.executionCounter.Load()`.
    pub fn execution_count(&self) -> u64 {
        self.execution_counter.load(Ordering::SeqCst)
    }

    /// Go: `BaseVariantsSeq()` — the base-applied variants, in tree key order (Go: map order
    /// within a key).
    // Go: tpl/tplimpl/templatestore.go:BaseVariantsSeq
    pub fn base_variants_seq(&self) -> Vec<Arc<TemplWithBaseApplied>> {
        let mut out = Vec::new();
        if let Some(t) = &self.state().base_variants {
            for (_, v) in t.all() {
                for vv in v.values() {
                    out.push(vv.clone());
                }
            }
        }
        out
    }

    /// The base variants with their base tree key and base descriptor, in tree key order.
    pub fn base_variants_tree(&self) -> Vec<(String, TemplateDescriptor, Arc<TemplInfo>)> {
        let mut out = Vec::new();
        if let Some(t) = &self.state().base_variants {
            for (k, v) in t.all() {
                for (d, vv) in v.iter() {
                    out.push((k.clone(), d.clone(), vv.template.clone()));
                }
            }
        }
        out
    }

    /// Go: `ti.overlays` — the templates applied to this base template.
    pub fn overlays(&self) -> Vec<Arc<TemplInfo>> {
        self.state().overlays.clone()
    }

    // Go: tpl/tplimpl/templatestore.go:IdentifierBase
    pub fn identifier_base(&self) -> String {
        match &self.path_info {
            None => self.name(),
            Some(p) => p.identifier_base(),
        }
    }

    // Go: tpl/tplimpl/templatestore.go:Name
    pub fn name(&self) -> String {
        let templ = self.template();
        match templ {
            None => match &self.path_info {
                Some(p) => p.path_no_leading_slash().to_string(),
                None => panic!("TemplInfo.Name: no template and no path"),
            },
            Some(t) => t.name(),
        }
    }

    /// Go: `Filename()` ("" for embedded).
    // Go: tpl/tplimpl/templatestore.go:Filename
    pub fn filename(&self) -> String {
        match &self.fi {
            None => String::new(),
            Some(fi) => fi.meta().filename.clone(),
        }
    }

    // Go: tpl/tplimpl/templatestore.go:Prepare
    pub fn prepare(&self) -> Result<Template> {
        let templ = self
            .template()
            .ok_or_else(|| Error::new(format!("template {} not parsed", self.string())))?;
        let t = templ.prepare()?;
        Ok(Template::Text(t))
    }

    // Go: tpl/tplimpl/templatestore.go:String
    pub fn string(&self) -> String {
        match &self.path_info {
            Some(p) => p.to_string(),
            None => "<nil>".to_string(),
        }
    }

    /// Go: `ti.PathInfo.Path()` ("" when there is no path info).
    fn path(&self) -> &str {
        match &self.path_info {
            Some(p) => p.path(),
            None => "",
        }
    }

    // Go: tpl/tplimpl/templatestore.go:IsProbablyDependency
    pub fn is_probably_dependency(&self, other_identifier_base: &str) -> bool {
        self.is_probably_the_same_id_as(other_identifier_base)
    }

    // Go: tpl/tplimpl/templatestore.go:IsProbablyDependent
    pub fn is_probably_dependent(&self, other_identifier_base: &str) -> bool {
        for overlay in self.state().overlays.iter() {
            if overlay.is_probably_the_same_id_as(other_identifier_base) {
                return true;
            }
        }
        self.is_probably_the_same_id_as(other_identifier_base)
    }

    // Go: tpl/tplimpl/templatestore.go:isProbablyTheSameIDAs
    fn is_probably_the_same_id_as(&self, other_identifier_base: &str) -> bool {
        if self.identifier_base() == other_identifier_base {
            return true;
        }

        if let Some(fi) = &self.fi
            && let Some(fpi) = &fi.meta().path_info
        {
            let same = match &self.path_info {
                Some(p) => Arc::ptr_eq(fpi, p),
                None => false,
            };
            if !same {
                return other_identifier_base == fpi.identifier_base();
            }
        }

        false
    }

    /// Go: `Base()` (`tpl.CurrentTemplateInfoCommonOps` of the base template).
    // Go: tpl/tplimpl/templatestore.go:Base
    pub fn base(&self) -> Option<CurrentTemplateBase> {
        self.base.as_ref().map(|b| CurrentTemplateBase {
            name: b.name(),
            filename: b.filename(),
        })
    }

    // Go: tpl/tplimpl/templatestore.go:findBestMatchBaseof
    fn find_best_match_baseof(
        &self,
        s: &StoreShared,
        d1: &TemplateDescriptor,
        k1: &str,
        slash_count_k1: i64,
        best: &mut BestMatch,
    ) {
        let st = self.state();
        let Some(base_variants) = &st.base_variants else {
            return;
        };

        let _ = base_variants.walk_path(k1, &mut |k2, v| {
            if !s.in_path(k1, k2) {
                return Ok(false);
            }
            let slash_count_k2 = count_slashes(k2);
            let distance = slash_count_k1 - slash_count_k2;

            for (d2, vv) in v.iter() {
                let mut weight = s.dh.compare_descriptors(Category::Baseof, false, d1, d2);
                weight.distance = distance;
                if best.is_better(weight, &vv.template) {
                    best.update_values(weight, k2, d2, &vv.template);
                }
            }
            Ok(false)
        });
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

impl TemplateQuery {
    // Go: tpl/tplimpl/templatestore.go:init
    fn init(&mut self) {
        if self.desc.kind == KIND_TEMPORARY || get_kind_main(&self.desc.kind).is_empty() {
            self.desc.kind = String::new();
        }
        if self.desc.layout_from_template.is_empty() && !self.desc.kind.is_empty() {
            if self.desc.kind == KIND_PAGE {
                self.desc.layout_from_template = LAYOUT_SINGLE.to_string();
            } else {
                self.desc.layout_from_template = LAYOUT_LIST.to_string();
            }
        }

        if self.consider.is_none() {
            self.consider = Some(Arc::new(|_| true));
        }

        self.name = go_lower(&self.name);
    }

    fn consider(&self, ti: &TemplInfo) -> bool {
        match &self.consider {
            Some(f) => f(ti),
            None => true,
        }
    }
}

/// Go `strings.ToLower`.
fn go_lower(s: &str) -> String {
    String::from_utf8_lossy(&go_unicode::strings::to_lower(s.as_bytes())).into_owned()
}

/// Go `strings.Count(s, "/")`.
fn count_slashes(s: &str) -> i64 {
    s.bytes().filter(|&b| b == b'/').count() as i64
}

/// Go: `bestMatch` (tie-breaks in `isBetter`).
pub(crate) struct BestMatch {
    pub(crate) templ: Option<Arc<TemplInfo>>,
    pub(crate) desc: TemplateDescriptor,
    pub(crate) w: Weight,
    pub(crate) key: String,
    pub(crate) candidates: Vec<Arc<TemplInfo>>,
    /// Go sets it (getBest) but never reads it.
    #[allow(dead_code)]
    pub(crate) default_output_format: String,
}

impl BestMatch {
    fn new(default_output_format: &str) -> BestMatch {
        BestMatch {
            templ: None,
            desc: TemplateDescriptor::default(),
            w: Weight::default(),
            key: String::new(),
            candidates: Vec::new(),
            default_output_format: default_output_format.to_string(),
        }
    }

    // Go: tpl/tplimpl/templatestore.go:reset
    fn reset(&mut self) {
        self.templ = None;
        self.w = Weight::default();
        self.desc = TemplateDescriptor::default();
        self.key = String::new();
        self.candidates = Vec::new();
    }

    // Go: tpl/tplimpl/templatestore.go:candidatesAsStringSlice
    fn candidates_as_string_slice(&self) -> Option<Vec<String>> {
        if self.candidates.is_empty() {
            return None;
        }
        Some(
            self.candidates
                .iter()
                .map(|v| v.path().to_string())
                .collect(),
        )
    }

    // Go: tpl/tplimpl/templatestore.go:isBetter
    fn is_better(&self, w: Weight, ti: &TemplInfo) -> bool {
        let Some(best_templ) = &self.templ else {
            // Anything is better than nothing.
            return true;
        };

        if w.w1 <= 0 {
            if self.w.w1 <= 0 {
                return ti.path() < best_templ.path();
            }
            return false;
        }

        // Note that for render hook templates, we need to make
        // the embedded render hook template wih if they're a better match,
        // e.g. render-codeblock-goat.html.
        if best_templ.category != Some(Category::Markup) && self.w.w1 > 0 {
            let current_best_is_embedded = best_templ.sub_category() == SubCategory::Embedded;
            if current_best_is_embedded {
                if ti.sub_category() != SubCategory::Embedded {
                    return true;
                }
            } else if ti.sub_category() == SubCategory::Embedded {
                // Prefer user provided template.
                return false;
            }
        }

        if w.distance < self.w.distance {
            if w.w2 < self.w.w2 {
                return false;
            }
            if w.w3 < self.w.w3 {
                return false;
            }
        } else if w.w1 < self.w.w1 {
            return false;
        }

        if w.is_equal_weights(&self.w) {
            // Tie breakers.
            if w.distance < self.w.distance {
                return true;
            }

            return ti.path() < best_templ.path();
        }

        true
    }

    // Go: tpl/tplimpl/templatestore.go:updateValues
    fn update_values(&mut self, w: Weight, key: &str, k: &TemplateDescriptor, vv: &Arc<TemplInfo>) {
        self.w = w;
        self.templ = Some(vv.clone());
        self.desc = k.clone();
        self.key = key.to_string();
    }
}

/// Go: `nodeKey`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeKey {
    pub c: Category,
    pub d: TemplateDescriptor,
}

/// Go: `keyTemplateInfo`.
#[derive(Clone)]
pub struct KeyTemplateInfo {
    pub key: String,
    pub info: Arc<TemplInfo>,
}

/// The shared, parsed part of the store (Go: everything except `storeSite`).
pub struct StoreShared {
    pub opts: StoreOptions,
    pub html_format: OutputFormat,
    pub(crate) tree_main: SimpleTree<BTreeMap<NodeKey, Arc<TemplInfo>>>,
    pub(crate) tree_shortcodes:
        SimpleTree<BTreeMap<String, BTreeMap<TemplateDescriptor, Arc<TemplInfo>>>>,
    pub(crate) templates_by_path: Mutex<BTreeMap<String, Arc<TemplInfo>>>,
    pub(crate) shortcodes_by_name: Mutex<BTreeMap<String, Arc<TemplInfo>>>,
    pub(crate) dh: DescriptorHandler,
    pub(crate) cache_lookup_partials: Mutex<BTreeMap<String, Option<Arc<TemplInfo>>>>,
    /// The template namespace.
    pub(crate) tns: TemplateNamespace,
    /// Go: the compiled `StoreOptions.legacyMapping*`.
    legacy_mapping_taxonomy: BTreeMap<String, LegacyOrdinalMapping>,
    legacy_mapping_term: BTreeMap<String, LegacyOrdinalMapping>,
    legacy_mapping_section: BTreeMap<String, LegacyOrdinalMapping>,
    /// For testing/benchmarking (Go `siteOptsOrig`).
    site_opts_orig: SiteOptions,
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
        nh_tpl::template::register_markup_scope_getter();

        let Some(html) = opts.output_formats.get_by_name("html") else {
            panic!("HTML output format not found");
        };
        // Note that the funcs passed below is just for name validation.
        let func_names: Vec<String> = site_opts.template_funcs.keys().cloned().collect();
        let store_site = configure_site_storage(&site_opts, opts.watching, &opts.named_types);
        let mut s = StoreShared {
            dh: DescriptorHandler {
                default_content_language: opts.default_content_language.clone(),
                default_output_format: opts.default_output_format.clone(),
            },
            opts,
            html_format: html,
            tree_main: SimpleTree::new(),
            tree_shortcodes: SimpleTree::new(),
            templates_by_path: Mutex::new(BTreeMap::new()),
            shortcodes_by_name: Mutex::new(BTreeMap::new()),
            cache_lookup_partials: Mutex::new(BTreeMap::new()),
            tns: new_template_namespace(&func_names),
            legacy_mapping_taxonomy: BTreeMap::new(),
            legacy_mapping_term: BTreeMap::new(),
            legacy_mapping_section: BTreeMap::new(),
            site_opts_orig: site_opts,
        };

        s.init()?;
        s.insert_templates(None, false)?;
        s.insert_embedded()?;
        s.parse_templates(false)?;
        s.extract_inline_partials(false)?;
        s.transform_templates()?;
        s.tns.create_prototypes(true)?;
        s.prepare_templates()?;

        Ok(TemplateStore {
            shared: Arc::new(s),
            store_site: Arc::new(store_site),
        })
    }

    /// Go: `NewFromOpts()` — a new store with the same configuration as the original (testing).
    // Go: tpl/tplimpl/templatestore.go:NewFromOpts
    pub fn new_from_opts(&self) -> Result<TemplateStore> {
        TemplateStore::new(self.shared.opts.clone(), self.shared.site_opts_orig.clone())
    }

    /// Go: `WithSiteOpts(opts)` — same parsed templates, another site's funcs.
    // Go: tpl/tplimpl/templatestore.go:WithSiteOpts
    pub fn with_site_opts(&self, opts: SiteOptions) -> TemplateStore {
        TemplateStore {
            shared: self.shared.clone(),
            store_site: Arc::new(configure_site_storage(
                &opts,
                self.shared.opts.watching,
                &self.shared.opts.named_types,
            )),
        }
    }

    /// Go: `FindAllBaseTemplateCandidates(overlayKey, desc)`.
    // Go: tpl/tplimpl/templatestore.go:FindAllBaseTemplateCandidates
    pub fn find_all_base_template_candidates(
        &self,
        overlay_key: &str,
        desc: &TemplateDescriptor,
    ) -> Vec<KeyTemplateInfo> {
        self.shared
            .find_all_base_template_candidates(overlay_key, desc)
    }

    /// Go: `ExecuteWithContext(ctx, ti, wr, data)` — pushes CurrentTemplateInfo (level+1, max 999).
    // Go: tpl/tplimpl/templatestore.go:ExecuteWithContext
    pub fn execute_with_context(
        &self,
        ctx: &TplContext,
        ti: &Arc<TemplInfo>,
        w: &mut Vec<u8>,
        data: &Value,
    ) -> Result<()> {
        let count = || {
            ti.execution_counter.fetch_add(1, Ordering::SeqCst);
            if let Some(b) = &ti.base {
                b.execution_counter.fetch_add(1, Ordering::SeqCst);
            }
        };

        let templ = ti
            .template()
            .ok_or_else(|| Error::new(format!("template {} not parsed", ti.string())))?;

        let ctx = ctx.with_current_template_info(&ti.name(), &ti.filename(), ti.base());
        let level = ctx.current_template.as_ref().map(|c| c.level).unwrap_or(0);

        const LEVEL_THRESHOLD: i64 = 999;
        if level > LEVEL_THRESHOLD {
            count();
            return Err(Error::new(format!(
                "maximum template call stack size exceeded in {}",
                go_strconv::quote(ti.filename())
            )));
        }

        let exec_err =
            self.store_site
                .executer
                .execute_with_context(ctx.as_host(), &templ, w, data);
        count();
        if let Err(e) = exec_err {
            return Err(self
                .shared
                .add_file_context(ti, "execute of template failed", e));
        }
        Ok(())
    }

    /// Go: `GetFunc(name)`.
    // Go: tpl/tplimpl/templatestore.go:GetFunc
    pub fn get_func(&self, name: &str) -> Option<crate::engine::TplFunc> {
        self.store_site.exec_helper.funcs.get(name).cloned()
    }

    /// Go: `GetIdentity(p)` — the template registered for the path, as its identity.
    // Go: tpl/tplimpl/templatestore.go:GetIdentity
    pub fn get_identity(&self, p: &str) -> Option<Arc<TemplInfo>> {
        let p = nh_common::paths::path::add_leading_slash(p);
        self.shared.templates_by_path().get(&p).cloned()
    }

    // Go: tpl/tplimpl/templatestore.go:LookupByPath
    pub fn lookup_by_path(&self, template_path: &str) -> Option<Arc<TemplInfo>> {
        self.shared.templates_by_path().get(template_path).cloned()
    }

    // Go: tpl/tplimpl/templatestore.go:LookupPagesLayout
    pub fn lookup_pages_layout(&self, q: &TemplateQuery) -> Option<Arc<TemplInfo>> {
        self.shared.lookup_pages_layout(q)
    }

    /// Go: `LookupPartial(pth)` — parsed as a layouts path of type partial; no extension -> html.
    // Go: tpl/tplimpl/templatestore.go:LookupPartial
    pub fn lookup_partial(&self, pth: &str) -> Option<Arc<TemplInfo>> {
        self.shared.lookup_partial(pth)
    }

    // Go: tpl/tplimpl/templatestore.go:LookupShortcodeByName
    pub fn lookup_shortcode_by_name(&self, name: &str) -> Option<Arc<TemplInfo>> {
        let name = go_lower(name);
        self.shared.shortcodes_by_name_map().get(&name).cloned()
    }

    // Go: tpl/tplimpl/templatestore.go:LookupShortcode
    pub fn lookup_shortcode(&self, q: &TemplateQuery) -> Result<Option<Arc<TemplInfo>>> {
        self.shared.lookup_shortcode(q)
    }

    /// Go: `PrintDebug(prefix, category, w)` (testing/debugging only).
    // Go: tpl/tplimpl/templatestore.go:PrintDebug
    pub fn print_debug(&self, prefix: &str, category: Category, w: &mut Vec<u8>) {
        let print_one = |w: &mut Vec<u8>, key: &str, vv: &TemplInfo| {
            let level = count_slashes(key) as usize;
            if Some(category) != vv.category {
                return;
            }
            let st = vv.state();
            let content = String::from_utf8_lossy(&st.content)
                .trim()
                .replace('\n', " ");
            let short: String = content.chars().take(30).collect();
            let ts = format!(
                "kind: {} layout: {} lang: {} content: {}",
                go_strconv::quote(&st.d.kind),
                go_strconv::quote(&st.d.layout_from_template),
                go_strconv::quote(&st.d.lang),
                short
            );
            w.extend_from_slice(format!("{}{} {}\n", " ".repeat(level), key, ts).as_bytes());
        };
        let _ = self.shared.tree_main.walk_prefix(prefix, &mut |key, v| {
            for vv in v.values() {
                print_one(w, key, vv);
            }
            Ok(false)
        });
        let _ = self
            .shared
            .tree_shortcodes
            .walk_prefix(prefix, &mut |key, v| {
                for vv in v.values() {
                    for vv2 in vv.values() {
                        print_one(w, key, vv2);
                    }
                }
                Ok(false)
            });
    }

    /// Go: `RefreshFiles(include)` — server/watch mode only.
    // Go: tpl/tplimpl/templatestore.go:RefreshFiles
    pub fn refresh_files(&self) -> Result<()> {
        self.shared.tns.create_prototypes_parse()
    }

    // Go: tpl/tplimpl/templatestore.go:HasTemplate
    pub fn has_template(&self, template_path: &str) -> bool {
        let template_path = go_lower(template_path);
        let template_path = nh_common::paths::path::add_leading_slash(&template_path);
        self.shared.templates_by_path().contains_key(&template_path)
    }

    /// Go: `TextParse(name, tpl)` — a standalone text/template (resources.ExecuteAsTemplate).
    // Go: tpl/tplimpl/templatestore.go:TextParse
    pub fn text_parse(&self, name: &str, src: &str) -> Result<Arc<TemplInfo>> {
        self.text_parse_bytes(name, src.as_bytes())
    }

    /// [`TemplateStore::text_parse`] over Go string bytes.
    pub fn text_parse_bytes(&self, name: &str, src: &[u8]) -> Result<Arc<TemplInfo>> {
        let templ = self
            .shared
            .tns
            .standalone_text
            .new_associated(name)
            .parse(src)?;
        Ok(TemplInfo::new_info(
            None,
            None,
            None,
            TemplInfoState {
                template: Some(Template::Text(templ)),
                ..Default::default()
            },
        ))
    }

    // Go: tpl/tplimpl/templatestore.go:TextLookup
    pub fn text_lookup(&self, name: &str) -> Option<Arc<TemplInfo>> {
        let templ = self.shared.tns.standalone_text.lookup(name)?;
        Some(TemplInfo::new_info(
            None,
            None,
            None,
            TemplInfoState {
                template: Some(Template::Text(templ)),
                ..Default::default()
            },
        ))
    }

    /// Go: `UnusedTemplates()` (`--printUnusedTemplates`).
    // Go: tpl/tplimpl/templatestore.go:UnusedTemplates
    pub fn unused_templates(&self) -> Vec<Arc<TemplInfo>> {
        let mut unused = Vec::new();

        for vv in self.shared.templates() {
            if vv.sub_category() != SubCategory::Main || vv.is_legacy_mapped() {
                // Skip inline partials and internal templates.
                continue;
            }
            if vv.execution_counter.load(Ordering::SeqCst) == 0 {
                unused.push(vv);
            }
        }

        // Go: sort.Sort(byPath(unused)).
        go_sort::sort_by(&mut unused, |a, b| a.path() < b.path());
        unused
    }

    /// Go: `templates()` — every template in use: the templates of the main tree (the
    /// base-applied variants in place of templates that need a base template), then the
    /// shortcodes.
    pub fn templates(&self) -> Vec<Arc<TemplInfo>> {
        self.shared.templates()
    }

    /// Every entry of the main tree: (key, node key, template), in tree order.
    pub fn main_tree_entries(&self) -> Vec<(String, NodeKey, Arc<TemplInfo>)> {
        let mut out = Vec::new();
        for (k, v) in self.shared.tree_main.all() {
            for (nk, ti) in v.iter() {
                out.push((k.clone(), nk.clone(), ti.clone()));
            }
        }
        out
    }

    /// Every entry of the shortcodes tree: (key, name, descriptor, template), in tree order.
    pub fn shortcode_tree_entries(
        &self,
    ) -> Vec<(String, String, TemplateDescriptor, Arc<TemplInfo>)> {
        let mut out = Vec::new();
        for (k, v) in self.shared.tree_shortcodes.all() {
            for (name, m) in v.iter() {
                for (d, ti) in m.iter() {
                    out.push((k.clone(), name.clone(), d.clone(), ti.clone()));
                }
            }
        }
        out
    }

    /// Every entry of `shortcodesByName`, in key order.
    pub fn shortcodes_by_name_entries(&self) -> Vec<(String, Arc<TemplInfo>)> {
        self.shared
            .shortcodes_by_name_map()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// The main template namespaces (Go `tns.parseHTML`'s text namespace and `tns.parseText`),
    /// e.g. to dump their trees.
    pub fn main_namespaces(&self) -> (TextTemplate, TextTemplate) {
        (
            self.shared.tns.parse_html.text(),
            self.shared.tns.parse_text.clone(),
        )
    }

    /// Every entry of `templatesByPath`, in key order.
    pub fn templates_by_path_entries(&self) -> Vec<(String, Arc<TemplInfo>)> {
        self.shared
            .templates_by_path()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

/// Go: `TemplateStoreProvider`.
pub trait TemplateStoreProvider {
    fn get_template_store(&self) -> TemplateStore;
}

impl StoreShared {
    fn templates_by_path(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Arc<TemplInfo>>> {
        self.templates_by_path
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    fn shortcodes_by_name_map(
        &self,
    ) -> std::sync::MutexGuard<'_, BTreeMap<String, Arc<TemplInfo>>> {
        self.shortcodes_by_name
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    // Go: tpl/tplimpl/templatestore.go:FindAllBaseTemplateCandidates
    fn find_all_base_template_candidates(
        &self,
        _overlay_key: &str,
        desc: &TemplateDescriptor,
    ) -> Vec<KeyTemplateInfo> {
        let mut result = Vec::new();
        let desc_baseof = desc;
        let _ = self.tree_main.walk(&mut |k, v| {
            for vv in v.values() {
                if vv.category != Some(Category::Baseof) {
                    continue;
                }

                let d = vv.d();
                if d.is_kind_in_layout(&desc.layout_from_template)
                    && self
                        .dh
                        .compare_descriptors(Category::Baseof, false, desc_baseof, &d)
                        .w1
                        > 0
                {
                    result.push(KeyTemplateInfo {
                        key: k.to_string(),
                        info: vv.clone(),
                    });
                }
            }
            Ok(false)
        });

        result
    }

    // Go: tpl/tplimpl/templatestore.go:LookupPagesLayout
    fn lookup_pages_layout(&self, q: &TemplateQuery) -> Option<Arc<TemplInfo>> {
        let mut q = q.clone();
        q.init();
        let key = self.key(&q.path);

        let slash_count_key = count_slashes(&key);
        let mut best1 = self.get_best();
        self.find_best_match_walk_path(&q, &key, slash_count_key, &mut best1);
        if best1.w.w1 <= 0 {
            return None;
        }
        let m = best1.templ.clone().expect("best template");
        if m.no_base_of() {
            return Some(m);
        }
        best1.reset();
        m.find_best_match_baseof(self, &q.desc, &key, slash_count_key, &mut best1);
        if best1.w.w1 <= 0 {
            return None;
        }
        best1.templ
    }

    // Go: tpl/tplimpl/templatestore.go:LookupPartial
    fn lookup_partial(&self, pth: &str) -> Option<Arc<TemplInfo>> {
        if let Some(v) = self
            .cache_lookup_partials
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(pth)
        {
            return v.clone();
        }

        let ti = (|| -> Result<Option<Arc<TemplInfo>>> {
            let pi = Arc::new(
                self.opts
                    .path_parser
                    .parse(COMPONENT_FOLDER_LAYOUTS, pth)
                    .for_type(PathType::Partial),
            );
            let (k1, _, _, mut desc) = self.to_key_category_and_descriptor(&pi)?;
            if desc.output_format.is_empty() && desc.media_type.is_empty() {
                // Assume HTML.
                desc.output_format = self.html_format.name.clone();
                desc.media_type = self.html_format.media_type.typ.clone();
                desc.is_plain_text = self.html_format.is_plain_text;
            }

            let mut best = self.get_best();
            self.find_best_match_get(
                &self.key(&go_path::path::join(&[CONTAINER_PARTIALS, k1.as_str()])),
                Category::Partial,
                None,
                &desc,
                &mut best,
            );
            Ok(best.templ)
        })();

        match ti {
            Ok(ti) => {
                let mut cache = self
                    .cache_lookup_partials
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                // First stored value wins (Go GetOrCreate).
                cache.entry(pth.to_string()).or_insert(ti).clone()
            }
            Err(_) => None,
        }
    }

    // Go: tpl/tplimpl/templatestore.go:LookupShortcode
    fn lookup_shortcode(&self, q: &TemplateQuery) -> Result<Option<Arc<TemplInfo>>> {
        let mut q = q.clone();
        q.init();
        let k1 = self.key(&q.path);

        let slash_count_k1 = count_slashes(&k1);

        let mut best = self.get_best();

        let _ = self.tree_shortcodes.walk_path(&k1, &mut |k2, m| {
            if !self.in_path(&k1, k2) {
                return Ok(false);
            }
            let slash_count_k2 = count_slashes(k2);
            let distance = slash_count_k1 - slash_count_k2;

            let Some(v) = m.get(&q.name) else {
                return Ok(false);
            };

            for (k, vv) in v.iter() {
                best.candidates.push(vv.clone());
                if !q.consider(vv) {
                    continue;
                }

                let mut weight = self.dh.compare_descriptors(
                    q.category,
                    vv.sub_category() == SubCategory::Embedded,
                    &q.desc,
                    k,
                );
                weight.distance = distance;
                let is_better = best.is_better(weight, vv);
                if is_better {
                    best.update_values(weight, k2, k, vv);
                }
            }

            Ok(false)
        });

        if best.w.w1 <= 0 {
            let err = if let Some(s) = best.candidates_as_string_slice() {
                let mut msg = format!(
                    "no compatible template found for shortcode {} in [{}]",
                    go_strconv::quote(&q.name),
                    s.join(" ")
                );
                if !q.desc.is_plain_text {
                    msg += "; note that to use plain text template shortcodes in HTML you need to use the shortcode {{% delimiter";
                }
                Error::new(msg)
            } else {
                Error::new(format!(
                    "no template found for shortcode {}",
                    go_strconv::quote(&q.name)
                ))
            };
            return Err(err);
        }

        Ok(best.templ)
    }

    // Go: tpl/tplimpl/templatestore.go:clearCaches
    #[allow(dead_code)]
    fn clear_caches(&self) {
        self.cache_lookup_partials
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    // Go: tpl/tplimpl/templatestore.go:getBest
    fn get_best(&self) -> BestMatch {
        BestMatch::new(&self.opts.default_output_format)
    }

    // Go: tpl/tplimpl/templatestore.go:findBestMatchGet
    fn find_best_match_get(
        &self,
        key: &str,
        category: Category,
        consider: Option<&dyn Fn(&TemplInfo) -> bool>,
        desc: &TemplateDescriptor,
        best: &mut BestMatch,
    ) {
        let key = go_lower(key);

        let Some(v) = self.tree_main.get(&key) else {
            return;
        };

        for (k, vv) in v.iter() {
            if vv.category != Some(category) {
                continue;
            }

            if let Some(consider) = consider
                && !consider(vv)
            {
                continue;
            }

            let weight = self.dh.compare_descriptors(
                category,
                vv.sub_category() == SubCategory::Embedded,
                desc,
                &k.d,
            );
            if best.is_better(weight, vv) {
                best.update_values(weight, &key, &k.d, vv);
            }
        }
    }

    // Go: tpl/tplimpl/templatestore.go:inPath
    fn in_path(&self, k1: &str, k2: &str) -> bool {
        if k1 != k2 && !k1.starts_with(&format!("{k2}/")) {
            return false;
        }
        true
    }

    // Go: tpl/tplimpl/templatestore.go:findBestMatchWalkPath
    fn find_best_match_walk_path(
        &self,
        q: &TemplateQuery,
        k1: &str,
        slash_count_k1: i64,
        best: &mut BestMatch,
    ) {
        let _ = self.tree_main.walk_path(k1, &mut |k2, v| {
            if !self.in_path(k1, k2) {
                return Ok(false);
            }
            let slash_count_k2 = count_slashes(k2);
            let distance = slash_count_k1 - slash_count_k2;

            for (k, vv) in v.iter() {
                if vv.category != Some(q.category) {
                    continue;
                }

                if !q.consider(vv) {
                    continue;
                }

                let mut weight = self.dh.compare_descriptors(
                    q.category,
                    vv.sub_category() == SubCategory::Embedded,
                    &q.desc,
                    &k.d,
                );

                weight.distance = distance;
                let is_better = best.is_better(weight, vv);

                if is_better {
                    best.update_values(weight, k2, &k.d, vv);
                }
            }

            Ok(false)
        });
    }

    // Go: tpl/tplimpl/templatestore.go:addDeferredTemplate
    fn add_deferred_template(
        &self,
        owner: &TemplInfo,
        name: &str,
        n: crate::engine::parse::ListNode,
    ) -> Result<()> {
        if self.templates_by_path().contains_key(name) {
            return Ok(());
        }

        let d = owner.d();
        let templ = if d.is_plain_text {
            let prototype = &self.tns.parse_text;
            let tt = prototype.new_associated(name).parse(b"").map_err(|e| {
                Error::new(format!(
                    "failed to parse empty text template {}: {}",
                    go_strconv::quote(name),
                    e
                ))
            })?;
            if let Some(tree) = tt.tree() {
                tree.update(|t| t.root = Some(n));
            }
            Template::Text(tt)
        } else {
            let prototype = &self.tns.parse_html;
            let tt = prototype.new_associated(name).parse(b"").map_err(|e| {
                Error::new(format!(
                    "failed to parse empty HTML template {}: {}",
                    go_strconv::quote(name),
                    e
                ))
            })?;
            if let Some(tree) = tt.tree() {
                tree.update(|t| t.root = Some(n));
            }
            Template::Html(tt)
        };

        self.templates_by_path().insert(
            name.to_string(),
            Arc::new(TemplInfo {
                category: None,
                path_info: owner.path_info.clone(),
                fi: owner.fi.clone(),
                base: None,
                execution_counter: AtomicU64::new(0),
                m: RwLock::new(TemplInfoState {
                    d,
                    template: Some(templ),
                    ..Default::default()
                }),
            }),
        );

        Ok(())
    }

    /// Go: `addFileContext(ti, what, inerr)`. Go reads the template file (and the base
    /// template's) to locate the error line; here the error gets the file name and the
    /// position found in the message (see PORTING.md).
    // Go: tpl/tplimpl/templatestore.go:addFileContext
    fn add_file_context(&self, ti: &TemplInfo, what: &str, inerr: Error) -> Error {
        let Some(fi) = &ti.fi else {
            return inerr;
        };

        let inerr = Error::new(format!("{what}: {}", inerr.message()));
        nh_common::herrors::new_file_error_from_name(inerr, &fi.meta().filename)
    }

    // Go: tpl/tplimpl/templatestore.go:extractIdentifiers
    #[allow(dead_code)]
    fn extract_identifiers(&self, line: &str) -> Vec<String> {
        // identifiersRe = `at \<(.*?)(\.{3})?\>:`
        let mut identifiers = Vec::new();
        let mut rest = line;
        while let Some(i) = rest.find("at <") {
            let after = &rest[i + 4..];
            let Some(j) = after.find(">:") else {
                break;
            };
            let mut id = &after[..j];
            if let Some(s) = id.strip_suffix("...") {
                id = s;
            }
            identifiers.push(id.to_string());
            rest = &after[j + 2..];
        }
        identifiers
    }

    // Go: tpl/tplimpl/templatestore.go:extractInlinePartials
    fn extract_inline_partials(&mut self, _rebuild: bool) -> Result<()> {
        let is_partial_name =
            |s: &str| -> bool { s.starts_with("partials/") || s.starts_with("_partials/") };

        // We may find both inline and external partials in the current template namespaces,
        // so only add the ones we have not seen before.
        for templ in self.all_raw_templates() {
            let tname = templ.name();
            if tname.is_empty() || !is_partial_name(&tname) {
                continue;
            }
            let mut name = tname;
            if !has_ext(&name) {
                // Assume HTML. This in line with how the lookup works.
                name += &self.html_format.media_type.first_suffix.full_suffix;
            }
            if !name.starts_with('_') {
                name = format!("_{name}");
            }
            let pi = Arc::new(self.opts.path_parser.parse(COMPONENT_FOLDER_LAYOUTS, &name));
            let ti = self.insert_template(&pi, None, SubCategory::Inline, false)?;

            if let Some(ti) = ti {
                let mut m = ti.state_mut();
                m.d.is_plain_text = is_text(&templ);
                m.template = Some(templ);
                m.no_base_of = true;
                m.sub_category = SubCategory::Inline;
            }
        }

        Ok(())
    }

    // Go: tpl/tplimpl/templatestore.go:allRawTemplates
    fn all_raw_templates(&self) -> Vec<Template> {
        let p = &self.tns;
        let mut out = Vec::new();
        out.extend(p.templates_in(&Template::Html(p.parse_html.clone())));
        out.extend(p.templates_in(&Template::Text(p.parse_text.clone())));

        let html_clones = p
            .baseof_html_clones
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        for tt in html_clones {
            out.extend(p.templates_in(&Template::Html(tt)));
        }
        let text_clones = p
            .baseof_text_clones
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        for tt in text_clones {
            out.extend(p.templates_in(&Template::Text(tt)));
        }
        out
    }

    // Go: tpl/tplimpl/templatestore.go:insertEmbedded
    fn insert_embedded(&mut self) -> Result<()> {
        for (name, templb) in embedded_walk_order() {
            let base = name.rsplit('/').next().unwrap_or(name);
            if base.starts_with('.') {
                continue;
            }

            // Get the newlines on Windows in line with how we had it back when we used Go Generate
            // to write the templates to Go files.
            let templ = go_unicode::bytes::replace_all(templb, b"\r\n", b"\n");

            // Copy the embedded HTML table render hook to each output format.
            // See https://github.com/gohugoio/hugo/issues/13351.
            if name == go_path::path::join(&[CONTAINER_MARKUP, "render-table.html"]) {
                let ofs = self.opts.output_formats.0.clone();
                for of in ofs.iter() {
                    let path = format!(
                        "{}.{}{}",
                        nh_common::paths::url::trim_ext(name),
                        of.name,
                        of.media_type.first_suffix.full_suffix
                    );
                    self.insert_embedded_one(&path, &templ)?;
                }

                continue;
            }

            self.insert_embedded_one(name, &templ)?;

            if let Some(aliases) = embedded_templates_aliases(name) {
                for alias in aliases {
                    self.insert_embedded_one(alias, &templ)?;
                }
            }
        }
        Ok(())
    }

    /// Go: `insertOne(name, content)` of `insertEmbedded`.
    fn insert_embedded_one(&mut self, name: &str, content: &[u8]) -> Result<()> {
        let pi = Arc::new(self.opts.path_parser.parse(COMPONENT_FOLDER_LAYOUTS, name));
        let ti = if pi.section() == CONTAINER_SHORTCODES {
            self.insert_shortcode(&pi, None, false)?
        } else {
            self.insert_template(&pi, None, SubCategory::Embedded, false)?
        };

        if let Some(ti) = ti {
            // Currently none of the embedded templates need a baseof template.
            let mut m = ti.state_mut();
            m.no_base_of = true;
            m.content = content.to_vec();
            m.sub_category = SubCategory::Embedded;
        }

        Ok(())
    }

    // Go: tpl/tplimpl/templatestore.go:setTemplateByPath
    fn set_template_by_path(&self, p: &str, ti: &Arc<TemplInfo>) {
        self.templates_by_path().insert(p.to_string(), ti.clone());
    }

    // Go: tpl/tplimpl/templatestore.go:insertShortcode
    fn insert_shortcode(
        &mut self,
        pi: &Arc<Path>,
        fi: Option<&FileMetaInfo>,
        replace: bool,
    ) -> Result<Option<Arc<TemplInfo>>> {
        let (k1, k2, _, d) = self.to_key_category_and_descriptor(pi)?;
        if self.tree_shortcodes.get(&k1).is_none() {
            self.tree_shortcodes.insert(&k1, BTreeMap::new());
        }
        let m = self.tree_shortcodes.get_mut(&k1).expect("inserted above");

        match m.get(&k2) {
            Some(m1) => {
                if m1.contains_key(&d) && !replace {
                    return Ok(None);
                }
            }
            None => {
                m.insert(k2.clone(), BTreeMap::new());
            }
        }

        let ti = TemplInfo::new_info(
            Some(Category::Shortcode),
            Some(pi.clone()),
            fi.cloned(),
            TemplInfoState {
                d: d.clone(),
                no_base_of: true,
                ..Default::default()
            },
        );

        m.get_mut(&k2)
            .expect("inserted above")
            .insert(d, ti.clone());

        self.shortcodes_by_name_map().insert(k2, ti.clone());
        self.set_template_by_path(pi.path(), &ti);

        if let Some(fi) = fi
            && let Some(pi2) = &fi.meta().path_info
            && !Arc::ptr_eq(pi2, pi)
        {
            self.set_template_by_path(pi2.path(), &ti);
        }

        Ok(Some(ti))
    }

    // Go: tpl/tplimpl/templatestore.go:insertTemplate
    fn insert_template(
        &mut self,
        pi: &Arc<Path>,
        fi: Option<&FileMetaInfo>,
        sub_category: SubCategory,
        replace: bool,
    ) -> Result<Option<Arc<TemplInfo>>> {
        let (key, _, category, d) = match self.to_key_category_and_descriptor(pi) {
            Ok(v) => v,
            // See #13577. Warn for now.
            Err(err) => {
                let loc = match fi {
                    Some(fi) => format!("file {}", go_strconv::quote(&fi.meta().filename)),
                    None => format!("path {}", go_strconv::quote(pi.path())),
                };
                if let Some(log) = &self.opts.log {
                    log.warnf(format!("skipping template {loc}: {}", err.message()));
                }
                return Ok(None);
            }
        };

        self.insert_template2(pi, fi, &key, category, sub_category, d, replace, false)
    }

    // Go: tpl/tplimpl/templatestore.go:insertTemplate2
    fn insert_template2(
        &mut self,
        pi: &Arc<Path>,
        fi: Option<&FileMetaInfo>,
        key: &str,
        category: Category,
        sub_category: SubCategory,
        mut d: TemplateDescriptor,
        mut replace: bool,
        is_legacy_mapped: bool,
    ) -> Result<Option<Arc<TemplInfo>>> {
        if category == Category::Partial && d.output_format.is_empty() && d.media_type.is_empty() {
            // See issue #13601.
            d.output_format = self.html_format.name.clone();
            d.media_type = self.html_format.media_type.typ.clone();
        }

        let nk = NodeKey {
            c: category,
            d: d.clone(),
        };

        if self.tree_main.get(key).is_none() {
            self.tree_main.insert(key, BTreeMap::new());
        }

        let nk_existing = self.tree_main.get(key).and_then(|m| m.get(&nk)).cloned();

        if let Some(nk_existing) = &nk_existing {
            if !replace
                && let Some(fi) = fi
                && let Some(efi) = &nk_existing.fi
            {
                // See issue #13715.
                // We do the merge on the file system level, but from Hugo v0.146.0 we have a situation where
                // the project may well have a different layouts layout compared to the theme(s) it uses.
                // We could possibly have fixed that on a lower (file system) level, but since this is just
                // a temporary situation (until all projects are updated),
                // do a replace here if the file comes from higher up in the module chain.
                replace = fi.meta().module_ordinal < efi.meta().module_ordinal;
            }

            if !replace {
                // Always replace inline partials to allow for reloading.
                replace = sub_category == SubCategory::Inline
                    && nk_existing.sub_category() == SubCategory::Inline;
            }

            if !replace {
                let existing_ids = nk_existing
                    .path_info
                    .as_ref()
                    .map(|p| p.identifiers().len())
                    .unwrap_or(0);
                if pi.identifiers().len() >= existing_ids {
                    // e.g. /pages/home.foo.html and  /pages/home.html where foo may be a valid language name in another site.
                    return Ok(None);
                }
            }
        }

        let ti = TemplInfo::new_info(
            Some(category),
            Some(pi.clone()),
            fi.cloned(),
            TemplInfoState {
                d,
                no_base_of: category > Category::Layout,
                is_legacy_mapped,
                ..Default::default()
            },
        );

        self.tree_main
            .get_mut(key)
            .expect("inserted above")
            .insert(nk, ti.clone());

        if !is_legacy_mapped {
            self.set_template_by_path(pi.path(), &ti);
            if let Some(fi) = fi
                && let Some(pi2) = &fi.meta().path_info
                && !Arc::ptr_eq(pi2, pi)
            {
                self.set_template_by_path(pi2.path(), &ti);
            }
        }

        Ok(Some(ti))
    }

    /// Go: `fromLegacyPath(pi)` of `insertTemplates`: convert any legacy value to the new format.
    fn legacy_path_to_new(&self, pi: &Arc<Path>) -> Arc<Path> {
        let mut p = pi.path().to_string();
        p = p.strip_prefix("/_default").unwrap_or(&p).to_string();
        if p.starts_with("/shortcodes") || p.starts_with("/partials") {
            // Insert an underscore so it becomes /_shortcodes or /_partials.
            p = format!("/_{}", &p[1..]);
        }

        if p.contains(&format!("-{BASE_NAME_BASEOF}")) {
            // Before Hugo 0.146.0 we prepended one identifier (layout, type or kind) in front of the baseof keyword,
            // and then separated with a hyphen before the baseof keyword.
            // This identifier needs to be moved right after the baseof keyword and the hyphen removed, e.g.
            // /docs/list-baseof.html => /docs/baseof.list.html.
            let (dir, name) = go_path::path::split(&p);
            let (dir, name) = (dir.to_string(), name.to_string());
            if let Some(hyphen_idx) = name.find('-')
                && hyphen_idx > 0
            {
                let id = &name[..hyphen_idx];
                let mut name = name[hyphen_idx + 1 + BASE_NAME_BASEOF.len()..].to_string();
                if !name.starts_with('.') {
                    name = format!(".{name}");
                }
                p = go_path::path::join(&[dir.as_str(), &format!("{BASE_NAME_BASEOF}.{id}{name}")]);
            }
        }
        if p == pi.path() {
            return pi.clone();
        }
        Arc::new(self.opts.path_parser.parse(COMPONENT_FOLDER_LAYOUTS, &p))
    }

    // Go: tpl/tplimpl/templatestore.go:insertTemplates
    fn insert_templates(
        &mut self,
        include: Option<&dyn Fn(&FileMetaInfo) -> bool>,
        partial_rebuild: bool,
    ) -> Result<()> {
        // Set if we need to reset the base variants.
        let mut reset_base_variants = false;

        let mut legacy_ordinal_mappings: BTreeMap<
            LegacyTargetPathIdentifiers,
            LegacyOrdinalMappingFi,
        > = BTreeMap::new();

        let fs = self.opts.fs.clone();
        let mut walker = |pth: &str, fi: &FileMetaInfo| -> Result<()> {
            if fi.is_dir() {
                return Ok(());
            }

            if is_dot_file(pth) || is_backup_file(pth) {
                return Ok(());
            }

            if let Some(include) = include
                && !include(fi)
            {
                return Ok(());
            }

            let pi_orig = fi
                .meta()
                .path_info
                .clone()
                .expect("layouts file without PathInfo");

            let mut pi = pi_orig.clone();
            let apply_legacy_mapping;
            match pi.section() {
                CONTAINER_PARTIALS | CONTAINER_SHORTCODES | CONTAINER_MARKUP => {
                    // OK.
                    apply_legacy_mapping = false;
                }
                _ => {
                    pi = self.legacy_path_to_new(&pi);
                    apply_legacy_mapping = count_slashes(pi.path()) <= 2;
                }
            }

            if apply_legacy_mapping {
                let handle_mapping = |m1: LegacyOrdinalMapping,
                                      map: &mut BTreeMap<
                    LegacyTargetPathIdentifiers,
                    LegacyOrdinalMappingFi,
                >| {
                    let key = LegacyTargetPathIdentifiers {
                        target_path: m1.mapping.target_path.clone(),
                        target_category: m1.mapping.target_category,
                        kind: m1.mapping.target_desc.kind.clone(),
                        lang: pi.lang().to_string(),
                        ext: pi.ext().to_string(),
                        output_format: pi.output_format().to_string(),
                    };

                    match map.get(&key) {
                        Some(m2) => {
                            if m1.ordinal < m2.m.ordinal {
                                // Higher up == better match.
                                map.insert(
                                    key,
                                    LegacyOrdinalMappingFi {
                                        m: m1,
                                        fi: fi.clone(),
                                    },
                                );
                            }
                        }
                        None => {
                            map.insert(
                                key,
                                LegacyOrdinalMappingFi {
                                    m: m1,
                                    fi: fi.clone(),
                                },
                            );
                        }
                    }
                };

                let base_orig = pi_orig.path_before_lang_and_output_format_and_ext();

                if let Some(m1) = self.legacy_mapping_taxonomy.get(&base_orig) {
                    handle_mapping(m1.clone(), &mut legacy_ordinal_mappings);
                }

                if let Some(m1) = self.legacy_mapping_term.get(&base_orig) {
                    handle_mapping(m1.clone(), &mut legacy_ordinal_mappings);
                }

                const SECTION_KIND_TOKEN: &str = "SECTIONKIND";
                const SECTION_TOKEN: &str = "THESECTION";

                let base = base_orig.clone();
                let mut identifiers: Vec<String> = Vec::new();
                if !pi.layout().is_empty() {
                    identifiers.push(pi.layout().to_string());
                }
                if !pi.kind().is_empty() {
                    identifiers.push(pi.kind().to_string());
                }

                let should_include_section = |section: &str| -> bool {
                    match section {
                        CONTAINER_SHORTCODES | CONTAINER_PARTIALS | CONTAINER_MARKUP => false,
                        "taxonomy" | "" => false,
                        _ => {
                            for (k, v) in self.opts.taxonomy_singular_plural.iter() {
                                if k == section || v == section {
                                    return false;
                                }
                            }
                            true
                        }
                    }
                };
                if should_include_section(pi.section()) {
                    identifiers.push(pi.section().to_string());
                }

                let identifiers = nh_helpers::general::unique_strings(&identifiers);

                // Tokens on e.g. form /SECTIONKIND/THESECTION
                let insert_section_tokens = |section: &str| -> Vec<String> {
                    let kind_only = is_layout_standard(section);
                    let mut ss = Vec::new();
                    let mut s1 = base.clone();
                    if !kind_only {
                        s1 = s1.replace(section, SECTION_TOKEN);
                    }
                    s1 = s1.replace(KIND_SECTION, SECTION_KIND_TOKEN);
                    if s1 != base {
                        ss.push(s1);
                    }
                    let mut s1 = base.replace(KIND_SECTION, SECTION_KIND_TOKEN);
                    if !kind_only {
                        s1 = s1.replace(section, SECTION_TOKEN);
                    }
                    if s1 != base {
                        ss.push(s1);
                    }

                    // Go: helpers.UniqueStringsReuse(ss) with the result discarded: ss keeps
                    // its length (a duplicate pair stays a pair).
                    let _ = nh_helpers::general::unique_strings_reuse(ss.clone());

                    ss
                };

                for id in identifiers.iter() {
                    if id.is_empty() {
                        continue;
                    }

                    let p = insert_section_tokens(id);
                    for ss in p.iter() {
                        if let Some(m1) = self.legacy_mapping_section.get(ss) {
                            let mut m1 = m1.clone();
                            let mut target_path = m1.mapping.target_path.clone();

                            if !target_path.is_empty() {
                                target_path = target_path.replace(SECTION_TOKEN, id);
                                target_path = target_path.replace(SECTION_KIND_TOKEN, id);
                                target_path = target_path.replace("//", "/");
                            }
                            m1.mapping.target_path = target_path;
                            handle_mapping(m1, &mut legacy_ordinal_mappings);
                        }
                    }
                }
            }

            if partial_rebuild && pi.name_no_identifier() == BASE_NAME_BASEOF {
                // A baseof file has changed.
                reset_base_variants = true;
            }

            enum Insert {
                Template,
                Shortcode,
                None,
            }
            let mut insert_func = Insert::Template;

            match pi.path_type() {
                PathType::Shortcode => insert_func = Insert::Shortcode,
                PathType::Markup => {
                    let skip_image_render_hook = pi.name() == "render-image.html"
                        && self.opts.render_hooks.image.use_embedded == "always";
                    let skip_link_render_hook = pi.name() == "render-link.html"
                        && self.opts.render_hooks.link.use_embedded == "always";
                    if skip_image_render_hook || skip_link_render_hook {
                        insert_func = Insert::None;
                    }
                }
                _ => {}
            }

            let ti = match insert_func {
                Insert::Template => {
                    match self.insert_template(&pi, Some(fi), SubCategory::Main, partial_rebuild)? {
                        Some(ti) => Some(ti),
                        None => return Ok(()),
                    }
                }
                Insert::Shortcode => match self.insert_shortcode(&pi, Some(fi), partial_rebuild)? {
                    Some(ti) => Some(ti),
                    None => return Ok(()),
                },
                Insert::None => None,
            };

            if let Some(ti) = ti {
                self.tns.read_template_into(&ti)?;
            }

            Ok(())
        };

        if let Err(err) = nh_helpers::path::walk(fs, "", &mut walker) {
            if !nh_common::herrors::is_not_exist(&err) {
                return Err(err);
            }
            return Ok(());
        }

        for (k, v) in legacy_ordinal_mappings.iter() {
            let target_path = &k.target_path;
            let m = &v.m.mapping;
            let fi = &v.fi;
            let pi = fi
                .meta()
                .path_info
                .clone()
                .expect("layouts file without PathInfo");
            let (output_format, media_type) =
                self.resolve_output_format_and_or_media_type(&k.output_format, &k.ext);
            let category = m.target_category;
            let mut desc = m.target_desc.clone();
            desc.kind = k.kind.clone();
            desc.lang = k.lang.clone();
            desc.output_format = output_format.name.clone();
            desc.is_plain_text = output_format.is_plain_text;
            desc.media_type = media_type.typ.clone();

            let ti = self.insert_template2(
                &pi,
                Some(fi),
                target_path,
                category,
                SubCategory::Main,
                desc,
                true,
                true,
            )?;
            let Some(ti) = ti else {
                continue;
            };
            ti.state_mut().is_legacy_mapped = true;
            self.tns.read_template_into(&ti)?;
        }

        if reset_base_variants {
            self.tns
                .baseof_html_clones
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            self.tns
                .baseof_text_clones
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            for (_, v) in self.tree_main.all() {
                for vv in v.values() {
                    let mut m = vv.state_mut();
                    if !m.no_base_of {
                        m.state = ProcessingState::Initial;
                    }
                }
            }
        }

        Ok(())
    }

    // Go: tpl/tplimpl/templatestore.go:key
    fn key(&self, dir: &str) -> String {
        let dir = nh_common::paths::path::add_leading_slash(dir);
        if dir == "/" {
            return String::new();
        }
        nh_common::paths::path::trim_trailing(&dir)
    }

    // Go: tpl/tplimpl/templates.go:parseTemplate
    fn parse_template(&self, ti: &Arc<TemplInfo>, replace: bool) -> Result<()> {
        self.tns
            .do_parse_template(ti, replace)
            .map_err(|err| self.add_file_context(ti, "parse of template failed", err))
    }

    // Go: tpl/tplimpl/templatestore.go:parseTemplates
    fn parse_templates(&mut self, replace: bool) -> Result<()> {
        // Read and parse all templates.
        let main: Vec<(String, Vec<Arc<TemplInfo>>)> = self
            .tree_main
            .all()
            .map(|(k, v)| (k.clone(), v.values().cloned().collect()))
            .collect();
        for (_, v) in main.iter() {
            for vv in v {
                if crate::templates::is_transformed(vv) {
                    continue;
                }
                self.parse_template(vv, replace)?;
            }
        }

        // Lookup and apply base templates where needed.
        for (key, v) in main.iter() {
            for vv in v {
                if crate::templates::is_transformed(vv) {
                    continue;
                }
                if !vv.no_base_of() {
                    let d = vv.d();
                    // Find all compatible base templates.
                    let base_templates = self.find_all_base_template_candidates(key, &d);
                    if base_templates.is_empty() {
                        // The regular expression used to detect if a template needs a base template has some
                        // rare false positives. Assume we don't need one.
                        vv.state_mut().no_base_of = true;
                        self.parse_template(vv, replace)?;
                        continue;
                    }
                    vv.state_mut().base_variants = Some(SimpleTree::new());

                    for base in base_templates.iter() {
                        self.tns.apply_base_template(vv, base)?;
                    }
                }
            }
        }

        // Prese shortcodes.
        let shortcodes: Vec<Arc<TemplInfo>> = self
            .tree_shortcodes
            .all()
            .flat_map(|(_, v)| v.values().flat_map(|vv| vv.values().cloned()))
            .collect();
        for vvv in shortcodes.iter() {
            if crate::templates::is_transformed(vvv) {
                continue;
            }
            self.parse_template(vvv, replace)?;
        }

        Ok(())
    }

    /// prepareTemplates prepares all templates for execution.
    // Go: tpl/tplimpl/templatestore.go:prepareTemplates
    fn prepare_templates(&self) -> Result<()> {
        for t in self.templates() {
            if t.category == Some(Category::Baseof) {
                continue;
            }
            t.prepare()?;
        }
        Ok(())
    }

    /// resolveOutputFormatAndOrMediaType resolves the output format and/or media type
    /// based on the given output format suffix and media type suffix.
    /// Either of the suffixes can be empty, and the function will try to find a match
    /// based on the other suffix. If both are empty, the function will return zero values.
    // Go: tpl/tplimpl/templatestore.go:resolveOutputFormatAndOrMediaType
    fn resolve_output_format_and_or_media_type(
        &self,
        ofs: &str,
        mns: &str,
    ) -> (OutputFormat, MediaType) {
        let mut output_format = OutputFormat::default();
        let mut media_type = MediaType::default();

        if !ofs.is_empty()
            && let Some(of) = self.opts.output_formats.get_by_name(ofs)
        {
            media_type = of.media_type.clone();
            output_format = of;
        }

        if !mns.is_empty() && media_type.is_zero() {
            if let Some(of) = self.opts.output_formats.get_by_suffix(mns) {
                media_type = of.media_type.clone();
                output_format = of;
            } else if let Some((mt, _)) = self.opts.media_types.get_first_by_suffix(mns) {
                if output_format.is_zero() {
                    // For e.g. index.xml we will in the default confg now have the application/rss+xml  media type.
                    // Try a last time to find the output format using the SubType as the name.
                    // As to template resolution, this value is currently only used to
                    // decide if this is a text or HTML template.
                    output_format = self
                        .opts
                        .output_formats
                        .get_by_name(&mt.sub_type)
                        .unwrap_or_default();
                }
                media_type = mt;
            }
        }

        (output_format, media_type)
    }

    /// templates iterates over all templates in the store.
    /// Note that for templates with one or more base templates applied,
    /// we will yield the variants, e.g. the templates that's actually in use.
    // Go: tpl/tplimpl/templatestore.go:templates
    fn templates(&self) -> Vec<Arc<TemplInfo>> {
        let mut out = Vec::new();
        for (_, v) in self.tree_main.all() {
            for vv in v.values() {
                if !vv.no_base_of() {
                    for vvv in vv.base_variants_seq() {
                        out.push(vvv.template.clone());
                    }
                } else {
                    out.push(vv.clone());
                }
            }
        }
        for (_, v) in self.tree_shortcodes.all() {
            for vv in v.values() {
                for vvv in vv.values() {
                    out.push(vvv.clone());
                }
            }
        }
        out
    }

    // Go: tpl/tplimpl/templatestore.go:toKeyCategoryAndDescriptor
    fn to_key_category_and_descriptor(
        &self,
        p: &Path,
    ) -> Result<(String, String, Category, TemplateDescriptor)> {
        let mut k1 = p.dir().to_string();
        let mut k2 = String::new();

        let (output_format, media_type) =
            self.resolve_output_format_and_or_media_type(p.output_format(), p.ext());
        let name_no_identifier = p.name_no_identifier();

        let mut d = TemplateDescriptor {
            lang: p.lang().to_string(),
            output_format: p.output_format().to_string(),
            media_type: media_type.typ.clone(),
            kind: p.kind().to_string(),
            layout_from_template: p.layout().to_string(),
            is_plain_text: output_format.is_plain_text,
            ..Default::default()
        };

        d.normalize_from_file();

        let section = p.section();

        let mut category = match p.path_type() {
            PathType::Shortcode => Some(Category::Shortcode),
            PathType::Partial => Some(Category::Partial),
            PathType::Markup => Some(Category::Markup),
            _ => None,
        };

        if category.is_none() {
            if name_no_identifier == BASE_NAME_BASEOF {
                category = Some(Category::Baseof);
            } else {
                category = Some(match section {
                    "_hugo" => Category::Hugo,
                    "_server" => Category::Server,
                    _ => Category::Layout,
                });
            }
        }
        let category = category.expect("set above");

        if category == Category::Partial {
            d.layout_from_template = String::new();
            k1 = p.path_no_identifier();
        }

        if category == Category::Shortcode {
            k1 = p.path_no_identifier();

            let sep = format!("/{CONTAINER_SHORTCODES}/");
            let parts: Vec<&str> = k1.split(sep.as_str()).collect();
            let first = parts[0].to_string();
            if parts.len() > 1 {
                k2 = parts[1].to_string();
            }
            k1 = self.key(&first);
        }

        // Legacy layout for home page.
        if d.layout_from_template == "index" {
            if d.kind.is_empty() {
                d.kind = KIND_HOME.to_string();
            }
            d.layout_from_template = String::new();
        }

        if d.layout_from_template == d.kind {
            d.layout_from_template = String::new();
        }

        k1 = k1.strip_prefix("/_default").unwrap_or(&k1).to_string();
        if k1 == "/" {
            k1 = String::new();
        }

        if category == Category::Markup {
            // We store all template nodes for a given directory on the same level.
            k1 = k1.strip_suffix("/_markup").unwrap_or(&k1).to_string();
            let Some(v) = d.layout_from_template.strip_prefix("render-") else {
                return Err(Error::new("unrecognized render hook template"));
            };
            let v = v.to_string();
            let hyphen_idx = v.find('-');

            d.variant1 = v.clone();
            if let Some(hyphen_idx) = hyphen_idx
                && hyphen_idx > 0
            {
                d.variant1 = v[..hyphen_idx].to_string();
                d.variant2 = v[hyphen_idx + 1..].to_string();
            }

            d.layout_from_template = String::new(); // This allows using page layout as part of the key for lookups.
        }

        Ok((k1, k2, category, d))
    }

    // Go: tpl/tplimpl/templatestore.go:transformTemplates
    fn transform_templates(&self) -> Result<()> {
        let lookup = |name: &str, in_: &TemplInfo| -> Option<Template> {
            let templ = in_.template()?;
            if in_.d().is_plain_text {
                match templ {
                    Template::Text(t) => t.lookup(name).map(Template::Text),
                    Template::Html(_) => panic!("plain text template is not a text template"),
                }
            } else {
                match templ {
                    Template::Html(t) => t.lookup(name).map(Template::Html),
                    Template::Text(_) => panic!("HTML template is not an HTML template"),
                }
            }
        };

        for vv in self.templates() {
            if crate::templates::is_transformed(&vv) {
                continue;
            }
            vv.state_mut().state = ProcessingState::Transformed;
            if vv.category == Some(Category::Baseof) {
                continue;
            }
            let tctx = apply_template_transformers(&vv, &lookup)?;
            let defer_nodes = tctx.defer_nodes.clone();
            drop(tctx);
            for (name, node) in defer_nodes {
                self.add_deferred_template(&vv, &name, node)?;
            }
        }

        Ok(())
    }

    // Go: tpl/tplimpl/templatestore.go:init
    fn init(&mut self) -> Result<()> {
        // Before Hugo 0.146 we had a very elaborate template lookup system, especially for
        // terms and taxonomies. This is a way of preserving backwards compatibility
        // by mapping old paths into the new tree.
        self.legacy_mapping_taxonomy = BTreeMap::new();
        self.legacy_mapping_term = BTreeMap::new();
        self.legacy_mapping_section = BTreeMap::new();

        // Placeholders.
        const SINGULAR: &str = "SINGULAR";
        const PLURAL: &str = "PLURAL";

        let replace_tokens = |s: &str, singularv: &str, pluralv: &str| -> String {
            let s = s.replace(SINGULAR, singularv);
            s.replace(PLURAL, pluralv)
        };

        let has_singular_or_plural =
            |s: &str| -> bool { s.contains(SINGULAR) || s.contains(PLURAL) };

        let expand = |v: &LayoutLegacyMapping| -> Vec<LayoutLegacyMapping> {
            let mut result = Vec::new();

            if has_singular_or_plural(&v.source_path)
                || has_singular_or_plural(&v.target.target_path)
            {
                for (s, p) in self.opts.taxonomy_singular_plural.iter() {
                    let mut target = v.target.clone();
                    target.target_path = replace_tokens(&target.target_path, s, p);
                    let vv = replace_tokens(&v.source_path, s, p);
                    result.push(LayoutLegacyMapping {
                        source_path: vv,
                        target,
                    });
                }
            } else {
                result.push(v.clone());
            }
            result
        };

        let expand_sections = |v: &LayoutLegacyMapping| -> Vec<LayoutLegacyMapping> {
            let mut result = vec![v.clone()];
            let mut baseof_variant = v.clone();
            baseof_variant.source_path += &format!("-{BASE_NAME_BASEOF}");
            baseof_variant.target.target_category = Category::Baseof;
            result.push(baseof_variant);
            result
        };

        let mut terms = Vec::new();
        for v in legacy_term_mappings().iter() {
            terms.extend(expand(v));
        }
        let mut taxonomies = Vec::new();
        for v in legacy_taxonomy_mappings().iter() {
            taxonomies.extend(expand(v));
        }
        let mut sections = Vec::new();
        for v in legacy_section_mappings().iter() {
            sections.extend(expand_sections(v));
        }

        for (i, m) in terms.into_iter().enumerate() {
            self.legacy_mapping_term.insert(
                m.source_path,
                LegacyOrdinalMapping {
                    ordinal: i as i64,
                    mapping: m.target,
                },
            );
        }
        for (i, m) in taxonomies.into_iter().enumerate() {
            self.legacy_mapping_taxonomy.insert(
                m.source_path,
                LegacyOrdinalMapping {
                    ordinal: i as i64,
                    mapping: m.target,
                },
            );
        }
        for (i, m) in sections.into_iter().enumerate() {
            self.legacy_mapping_section.insert(
                m.source_path,
                LegacyOrdinalMapping {
                    ordinal: i as i64,
                    mapping: m.target,
                },
            );
        }

        Ok(())
    }
}

/// Go: `embeddedTemplatesAliases`. The tweet and twitter shortcodes were deprecated in favor of
/// the x shortcode in v0.141.0. We can remove these aliases in v0.155.0 or later.
// Go: tpl/tplimpl/templates.go:embeddedTemplatesAliases
fn embedded_templates_aliases(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "_shortcodes/twitter.html" => Some(&["_shortcodes/tweet.html"]),
        _ => None,
    }
}

/// The embedded templates in `fs.WalkDir` order (each directory's entries sorted by name,
/// depth first).
fn embedded_walk_order() -> Vec<(&'static str, &'static [u8])> {
    let mut v: Vec<(&'static str, &'static [u8])> = EMBEDDED_TEMPLATES.to_vec();
    // Sorting the path components (not the joined strings) is WalkDir's order.
    v.sort_by(|a, b| {
        let ca: Vec<&str> = a.0.split('/').collect();
        let cb: Vec<&str> = b.0.split('/').collect();
        ca.cmp(&cb)
    });
    v
}

// Go: tpl/tplimpl/templatestore.go:isLayoutStandard
fn is_layout_standard(s: &str) -> bool {
    matches!(s, LAYOUT_ALL | LAYOUT_LIST | LAYOUT_SINGLE)
}

// Go: tpl/tplimpl/templatestore.go:configureSiteStorage
fn configure_site_storage(
    opts: &SiteOptions,
    _watching: bool,
    named_types: &Arc<NamedTypeRegistry>,
) -> StoreSite {
    let mut funcsv: FuncMap = FuncMap::new();

    for (k, v) in opts.template_funcs.iter() {
        funcsv.insert(k.clone(), v.clone());
    }

    // Duplicate Go's internal funcs here for faster lookups.
    for (k, v) in crate::engine::html_go_funcs() {
        funcsv.entry(k.to_string()).or_insert(v);
    }

    for (k, v) in crate::engine::text_go_funcs() {
        funcsv.entry(k.to_string()).or_insert(v);
    }

    let exec_helper = Arc::new(TemplateExecHelper {
        funcs: Arc::new(funcsv),
        site: opts.site.clone(),
        named_types: named_types.clone(),
    });

    let executer: Arc<dyn Executer> = Arc::new(GoExecuter::new(exec_helper.clone()));

    StoreSite {
        opts: opts.clone(),
        exec_helper,
        executer,
    }
}

// Go: tpl/tplimpl/templatestore.go:isBackupFile
fn is_backup_file(path: &str) -> bool {
    path.as_bytes()[path.len() - 1] == b'~'
}

// Go: tpl/tplimpl/templatestore.go:isDotFile
fn is_dot_file(path: &str) -> bool {
    go_path::filepath::base(path).as_bytes()[0] == b'.'
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/templatestore.go (2096 lines; 43/70 funcs executed)
//   types: Category, SiteOptions, StoreOptions, SubCategory, TemplInfo, TemplWithBaseApplied, TemplateQuery,
//          TemplateStore, PathTemplateDescriptor, TemplateStoreProvider, TextTemplatHandler, bestMatch, byPath,
//          keyTemplateInfo, nodeKey, processingState, storeSite, weight
// OK L112-164: NewStore(opts StoreOptions, siteOpts SiteOptions) (*TemplateStore, error)
// OK L265-267: (ti *TemplInfo) SubCategory() SubCategory
// OK L269-280: (ti *TemplInfo) BaseVariantsSeq() iter.Seq[*TemplWithBaseApplied]
// OK L282-287: (t *TemplInfo) IdentifierBase() string
// OK L289-291: (t *TemplInfo) GetIdentity() identity.Identity (the Arc<TemplInfo> itself)
// OK L293-300: (ti *TemplInfo) Name() string
// OK L302-304: (ti *TemplInfo) Prepare() (*texttemplate.Template, error)
// OK L306-308: (t *TemplInfo) IsProbablyDependency(other identity.Identity) bool
// OK L310-317: (t *TemplInfo) IsProbablyDependent(other identity.Identity) bool
// OK L319-324: (ti *TemplInfo) String() string
// OK L326-347: (ti *TemplInfo) findBestMatchBaseof(s *TemplateStore, d1 TemplateDescriptor, k1 string, slashCountK1 int, best *bestMatch)
// OK L349-359: (t *TemplInfo) isProbablyTheSameIDAs(other identity.Identity) bool
// OK L362-364: (ti *TemplInfo) Base() tpl.CurrentTemplateInfoCommonOps
// OK L366-371: (ti *TemplInfo) Filename() string
// OK L400-425: (q *TemplateQuery) init()
// OK L457-459: (s *TemplateStore) NewFromOpts() (*TemplateStore, error)
// OK L468-485: (s *TemplateStore) FindAllBaseTemplateCandidates(overlayKey string, desc TemplateDescriptor) []keyTemplateInfo
// OK L487-524: (t *TemplateStore) ExecuteWithContext(ctx context.Context, ti *TemplInfo, wr io.Writer, data any) error
// OK L526-529: (t *TemplateStore) GetFunc(name string) (reflect.Value, bool)
// OK L531-538: (s *TemplateStore) GetIdentity(p string) identity.Identity
// OK L540-543: (t *TemplateStore) LookupByPath(templatePath string) *TemplInfo
// OK L551-556: (s *TemplateStore) getBest() *bestMatch
// OK L558-561: (s *TemplateStore) putBest(b *bestMatch) (no pool: a new bestMatch per lookup)
// OK L563-584: (s *TemplateStore) LookupPagesLayout(q TemplateQuery) *TemplInfo
// OK L586-607: (s *TemplateStore) LookupPartial(pth string) *TemplInfo
// OK L609-616: (s *TemplateStore) LookupShortcodeByName(name string) *TemplInfo
// OK L618-671: (s *TemplateStore) LookupShortcode(q TemplateQuery) (*TemplInfo, error)
// OK L674-702: (s *TemplateStore) PrintDebug(prefix string, category Category, w io.Writer)
// OK L704-706: (s *TemplateStore) clearCaches()
// OK L709-738: (s *TemplateStore) RefreshFiles(include func(fi hugofs.FileMetaInfo) bool) error (STUB: server/watch mode)
// OK L740-744: (s *TemplateStore) HasTemplate(templatePath string) bool
// OK L746-754: (t *TemplateStore) TextLookup(name string) *TemplInfo
// OK L756-764: (t *TemplateStore) TextParse(name, tpl string) (*TemplInfo, error)
// OK L766-781: (t *TemplateStore) UnusedTemplates() []*TemplInfo
// OK L786-790: (s TemplateStore) WithSiteOpts(opts SiteOptions) *TemplateStore
// OK L792-814: (s *TemplateStore) findBestMatchGet(key string, category Category, consider func(candidate *TemplInfo) bool, desc TemplateDescriptor, best *bestMatch)
// OK L816-821: (s *TemplateStore) inPath(k1, k2 string) bool
// OK L823-852: (s *TemplateStore) findBestMatchWalkPath(q TemplateQuery, k1 string, slashCountK1 int, best *bestMatch)
// OK L854-887: (t *TemplateStore) addDeferredTemplate(owner *TemplInfo, name string, n *parse.ListNode) error
// OK L889-942: (s *TemplateStore) addFileContext(ti *TemplInfo, what string, inerr error) error (file name only; see PORTING.md)
// OK L944-951: (s *TemplateStore) extractIdentifiers(line string) []string
// OK L953-991: (s *TemplateStore) extractInlinePartials(rebuild bool) error
// OK L993-1022: (s *TemplateStore) allRawTemplates() iter.Seq[tpl.Template]
// OK L1024-1098: (s *TemplateStore) insertEmbedded() error
// OK L1100-1102: (s *TemplateStore) setTemplateByPath(p string, ti *TemplInfo)
// OK L1104-1147: (s *TemplateStore) insertShortcode(pi *paths.Path, fi hugofs.FileMetaInfo, replace bool, tree doctree.Tree[map[string]map[TemplateDescriptor]*Templ...
// OK L1149-1164: (s *TemplateStore) insertTemplate(pi *paths.Path, fi hugofs.FileMetaInfo, subCategory SubCategory, replace bool, tree doctree.Tree[map[nodeKey]*Tem...
// OK L1166-1238: (s *TemplateStore) insertTemplate2( pi *paths.Path, fi hugofs.FileMetaInfo, key string, category Category, subCategory SubCategory, d TemplateDescr...
// OK L1240-1513: (s *TemplateStore) insertTemplates(include func(fi hugofs.FileMetaInfo) bool, partialRebuild bool) error
// OK L1515-1521: (s *TemplateStore) key(dir string) string
//    L1523-1529: (s *TemplateStore) createTemplatesSnapshot() error (watch mode only; not ported)
// OK L1531-1596: (s *TemplateStore) parseTemplates(replace bool) error
// OK L1599-1609: (s *TemplateStore) prepareTemplates() error
// OK L1620-1650: (s *TemplateStore) resolveOutputFormatAndOrMediaType(ofs, mns string) (output.Format, media.Type)
// OK L1655-1682: (s *TemplateStore) templates() iter.Seq[*TemplInfo]
// OK L1684-1781: (s *TemplateStore) toKeyCategoryAndDescriptor(p *paths.Path) (string, string, Category, TemplateDescriptor, error)
// OK L1783-1824: (s *TemplateStore) transformTemplates() error
// OK L1826-1898: (s *TemplateStore) init() error
// OK L1921-1927: (best *bestMatch) reset()
// OK L1929-1938: (best *bestMatch) candidatesAsStringSlice() []string
// OK L1940-1993: (best *bestMatch) isBetter(w weight, ti *TemplInfo) bool
// OK L1995-2000: (best *bestMatch) updateValues(w weight, key string, k TemplateDescriptor, vv *TemplInfo)
// OK L2004-2004: (a byPath) Len() int
// OK L2005-2007: (a byPath) Less(i, j int) bool
// OK L2009-2009: (a byPath) Swap(i, j int)
// OK L2037-2044: isLayoutStandard(s string) bool
// OK L2046-2048: (w weight) isEqualWeights(other weight) bool
// OK L2050-2088: configureSiteStorage(opts SiteOptions, watching bool) *storeSite
// OK L2090-2092: isBackupFile(path string) bool
// OK L2094-2096: isDotFile(path string) bool
// ---------------------------------------------------------------------------
