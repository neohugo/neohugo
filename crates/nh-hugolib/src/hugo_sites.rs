//! Port of `hugolib/hugo_sites.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture), crate lead of nh-hugolib.

//! Go `hugolib/hugo_sites.go` + the construction half of `site.go` (`NewHugoSites`,
//! `newHugoSites`, `newSiteRefLinker`; site.go:143-454, 858-867). T20 owns construction because
//! its acceptance test (tree dumps after `process`) runs the real `NewHugoSites` path.
//! `Data()`/`loadData`/`handleDataFile` (`.Site.Data`) are in hugo_sites_data.rs (T23): capture
//! does not need them.
//!
//! Construction needs the template store (shortcode lookup during capture) and therefore a
//! func map. Production passes `None` for `NewHugoSitesCfg::func_map_factory` and gets
//! `nh_tplfuncs::tplimplinit::create_func_map`; T20's tests pass a names-only func map built from
//! a Go fixture (every registered function name bound to a stub), so capture does not wait for
//! T18/T19.
//!
//! Ownership model (HUGO_LAYER.md §4): `HugoSites` owns everything in arenas — sites
//! (`Vec<Site>`, language order), pages (`Vec<PageState>`, indexed by [`PageId`]), the shared
//! content trees keyed like Go's doctree. It is built and assembled through `&mut HugoSites`
//! (process + assemble), then FROZEN into `Arc<HugoSites>` for rendering; from then on only
//! interior-mutable caches change (OnceLock/Mutex/atomics). Template-visible handles
//! (`PageHandle`, `SiteHandle`) are `{ h: Arc<HugoSites>, index }` — the Go pointers.
//!
//! Construction order (Go `NewHugoSites` + `newHugoSites`), as ported in [`HugoSites::new`]:
//! 1. the memory cache and the first site's `Deps::init` (publish fs wrapped with the HasBytes
//!    receiver, PathSpec + BaseFs, ContentSpec, SourceSpec, file caches, resources Spec);
//! 2. `Configs.Validate`; the page trees;
//! 3. per language (`Configs.ConfigLangs()` order: default content language first): the front
//!    matter handler, `langs.SetParams`, the site's `Deps` (`Deps::clone_for` for all but the
//!    first), the page map, the ref linker, the publisher (`DestinationPublisher` with its
//!    minifier client) and the related docs handler;
//! 4. sites sorted (default content language, weight, lang);
//! 5. `HugoInfo`; per site: the template store (`TemplateStore::new` for the first site,
//!    `with_site_opts` for the others, each with its own func map from the factory) and the i18n
//!    translators ([`compile_deps`]: `TranslationProvider::new_resource` for the first site,
//!    `clone_resource` for the others).
//!
//! The site's `page.Site` (`Deps.site`) needs the frozen `Arc<HugoSites>`: [`HugoSites::freeze`]
//! sets it.

use std::io::Write;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use go_value::{Map, Time};
use nh_allconfig::allconfig::Configs;
use nh_common::Result;
use nh_common::dynacache::{Cache as MemCache, Options as MemCacheOptions, Partition};
use nh_common::herrors::Error;
use nh_common::loggers::Logger;
use nh_common::maps::scratch::Scratch;
use nh_config::common_config::BuildConfig;
use nh_config::config_provider::{AllProvider, config_section};
use nh_config::neohugo::neohugo::{Dependency, HugoInfo, HugoInfoConfig};
use nh_deps::deps::{BuildState, Counters, Deps, DepsCfg};
use nh_doctree::nodeshifttree::WalkConfig;
use nh_i18n::translation_provider::TranslationProvider;
use nh_media::output::output_format::Formats;
use nh_page::page::Pages;
use nh_page::pagemeta::page_frontmatter::FrontMatterHandler;
use nh_page::pages_related::RelatedDocsHandler;
use nh_publisher::publisher::DestinationPublisher;
use nh_tplimpl::templatestore::{SiteOptions, StoreOptions, TemplateStore};

use crate::content_map::{ContentMapConfig, taxonomies_config_values};
use crate::content_map_trees::{ContentNode, PageTrees, new_page_map};
use crate::page::{PageId, PageState};
use crate::site::{Site, SiteHandle};

/// Go: `hugolib.HugoSites`.
pub struct HugoSites {
    /// Sites in language order: default content language first, then weight, then lang.
    pub sites: Vec<Site>,
    pub configs: Arc<Configs>,
    pub hugo_info: HugoInfo,
    /// Render formats of ALL sites concatenated (page outputs are indexed by this global index).
    pub render_formats: Formats,
    /// Go `h.currentSite` — index of the site being rendered (collator of `Site.Current()`).
    pub current_site: AtomicUsize,
    /// The first site's deps (shared services).
    pub deps: Arc<Deps>,
    /// The page arena.
    pub pages: Vec<PageState>,
    /// The shared content trees (pages, resources, taxonomy entries).
    pub page_trees: PageTrees,
    /// Go `h.init.data` — `.Site.Data` from the data component (lazy).
    pub data: OnceLock<Arc<Map>>,
    /// Go `cachePages` ("/pags/all"; global lists, e.g. `HugoSites.Pages()`). Dynacache
    /// pattern: never compute under the lock, first stored value wins (HUGO_LAYER.md §4.8).
    pub cache_pages: Partition<String, Pages>,
    /// Test seam for every template execution nh-hugolib starts (template_exec.rs); `None` in a
    /// normal build.
    pub template_executor: Option<Arc<dyn crate::template_exec::TemplateExecutor>>,
    /// Go `Deps.TranslationProvider` (the i18n translator of every site; see nh-deps' docs).
    pub translation_provider: TranslationProvider,
    /// Go's package-level `pageIDCounter` (page `pid`s; any unique numbering works).
    pub page_id_counter: AtomicU64,
    /// Go `buildCounter`: tracks invocations of the Build method.
    pub build_counter: AtomicU64,
    /// Go `*fatalErrorHandler`.
    pub fatal_error_handler: FatalErrorHandler,
    /// Set once frozen: a weak self-reference for creating handles. Shared (`Arc`) with the page
    /// outputs created before freezing (T22: content rendering reaches `Arc<HugoSites>` through
    /// it, see `page__output::HugoSitesRef`).
    pub(crate) self_ref: crate::page__output::HugoSitesRef,
}

impl HugoSites {
    /// Go: `hugolib.NewHugoSites(cfg deps.DepsCfg)` — creates deps (first site `Init`, others
    /// `Clone`), sites (publisher, frontmatter handler, related docs handler, page map), the template
    /// store (+ `WithSiteOpts` per site) and the i18n translators. See the module docs for the
    /// order.
    // Go: hugolib/site.go:NewHugoSites
    pub fn new(cfg: NewHugoSitesCfg) -> Result<HugoSites> {
        let confm = cfg.configs.clone();
        let conf = confm.get_first_language_config();
        let logger = cfg.log.clone();

        let mem_cache = Arc::new(MemCache::new(MemCacheOptions {
            watching: conf.watching(),
            ..Default::default()
        }));

        let first_site_deps = Deps::init(DepsCfg {
            fs: cfg.fs.clone(),
            conf: conf.clone(),
            log: logger.clone(),
            build_state: Arc::new(BuildState::default()),
            mem_cache,
            counters: Arc::new(Counters::default()),
        })?;

        // Go: `esbuild.NewBatcherClient(firstSiteDeps)` (js.Batch) is not ported: seeksnack does
        // not use js.Batch.

        confm.validate(&logger)?;

        let num_languages = confm.languages.len();
        let page_trees = PageTrees::new(num_languages);

        let mut sites: Vec<Site> = Vec::new();
        for (i, confp) in confm.config_langs().into_iter().enumerate() {
            let language = confp.language();
            if language.config.disabled {
                continue;
            }
            let k = language.lang.clone();
            let conf = confm
                .language_config_map
                .get(&k)
                .cloned()
                .ok_or_else(|| Error::new(format!("no config for language {k:?}")))?;
            let frontmatter_handler = FrontMatterHandler::new_with_logger(
                Some(first_site_deps.log.clone()),
                conf.frontmatter.clone(),
            )?;

            language.set_params(conf.params.clone());

            let deps = if i == 0 {
                first_site_deps.clone()
            } else {
                first_site_deps.clone_for(confp.clone())?
            };

            let page_map = new_page_map(i, new_content_map_config(&conf, &language.lang));

            // Go `newPageFinder(s.pageMap)` is T21's (pagecollections); the ref linker is
            // computed but not stored (see `SiteRefLinker`).
            let _ = new_site_ref_linker(&conf, &deps.log);

            // Set up the main publishing chain.
            let rs = first_site_deps.resource_spec();
            let site_output_formats = conf
                .output_formats
                .as_ref()
                .map(|n| n.config.clone())
                .unwrap_or_default();
            let site_media_types = conf
                .media_types
                .as_ref()
                .map(|n| n.config.clone())
                .unwrap_or_default();
            let min = nh_transform::minifiers::minifiers::Client::new(
                &site_media_types,
                &site_output_formats,
                &*rs.path_spec.cfg,
            )?;
            let build_config = config_section::<BuildConfig>(&*rs.path_spec.cfg, "build");
            let publisher = DestinationPublisher::new(
                rs.path_spec.base_fs.publish_fs.clone(),
                min,
                &build_config.build_stats,
            );

            let related_docs_handler: Arc<RelatedDocsHandler> =
                RelatedDocsHandler::new(conf.related.clone());
            // Site deps end.

            // Go `s.prepareInits()` (the lazy taxonomies, menus, prev/next): `OnceLock`s here.
            sites.push(Site {
                idx: i,
                conf,
                language,
                deps,
                page_map,
                store: Arc::new(Scratch::new()),
                taxonomies: OnceLock::new(),
                menus: OnceLock::new(),
                home: None,
                lastmod: Time::zero(),
                related_docs_handler,
                publisher: Arc::new(publisher),
                frontmatter_handler,
                render_formats: Formats::default(),
                template_store: OnceLock::new(),
            });
        }

        if sites.is_empty() {
            return Err(Error::new("no sites to build"));
        }

        // Pull the default content language to the top, then sort the sites by language weight
        // (if set) or lang. (`ConfigLangs` is already in this order.)
        let default_content_language = confm.base.root.default_content_language.clone();
        sites.sort_by(|a, b| {
            let li = &a.language;
            let lj = &b.language;
            let less = |li: &nh_langs::language::Language, lj: &nh_langs::language::Language| {
                if li.lang == default_content_language {
                    return true;
                }
                if lj.lang == default_content_language {
                    return false;
                }
                if li.config.weight != lj.config.weight {
                    return li.config.weight < lj.config.weight;
                }
                li.lang < lj.lang
            };
            if li.lang == lj.lang {
                std::cmp::Ordering::Equal
            } else if less(li, lj) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });

        new_hugo_sites(cfg, first_site_deps, page_trees, sites)
    }

    /// Freezes the build state for rendering and wires the `OnceLock`s that need handles:
    /// every site's `Deps.site` (the `page.Site` the template store and the namespaces read).
    /// The handles hold `Arc<HugoSites>`, so the frozen sites form a reference cycle that lives
    /// until the process ends (like Go's pointer graph; a one-shot build).
    pub fn freeze(self) -> Arc<HugoSites> {
        let h = Arc::new(self);
        let _ = h.self_ref.set(Arc::downgrade(&h));
        for (idx, s) in h.sites.iter().enumerate() {
            let site_ref = SiteHandle { h: h.clone(), idx }.site_ref();
            let _ = s.deps.site.set(site_ref);
        }
        h
    }

    pub fn page(&self, id: PageId) -> &PageState {
        &self.pages[id.0 as usize]
    }

    pub fn page_mut(&mut self, id: PageId) -> &mut PageState {
        &mut self.pages[id.0 as usize]
    }

    /// Go: `Close()`.
    // Go: hugolib/hugo_sites.go:Close
    pub fn close(&self) -> Result<()> {
        self.deps.close()
    }

    /// Go: `isRebuild()` (never in a one-shot build).
    // Go: hugolib/hugo_sites.go:isRebuild
    pub fn is_rebuild(&self) -> bool {
        self.build_counter.load(Ordering::SeqCst) > 0
    }

    /// Go: `HugoSites.Pages()` — all sites' `Pages()`, then `SortByDefault` (stable).
    // Go: hugolib/hugo_sites.go:Pages
    pub fn all_pages(self: &Arc<Self>) -> Pages {
        self.cache_pages
            .get_or_create("pages".to_string(), |_| {
                let mut pages: Pages = Pages::new();
                for idx in 0..self.sites.len() {
                    let s = SiteHandle {
                        h: self.clone(),
                        idx,
                    };
                    pages.extend(nh_page::site::Site::pages(&s).iter().cloned());
                }
                nh_page::pages_sort::sort_by_default(&mut pages);
                Ok(pages)
            })
            .unwrap_or_else(|err| panic!("{}", err.message()))
    }

    /// Go: `HugoSites.RegularPages()`.
    // Go: hugolib/hugo_sites.go:RegularPages
    pub fn all_regular_pages(self: &Arc<Self>) -> Pages {
        self.cache_pages
            .get_or_create("regular-pages".to_string(), |_| {
                let mut pages: Pages = Pages::new();
                for idx in 0..self.sites.len() {
                    let s = SiteHandle {
                        h: self.clone(),
                        idx,
                    };
                    pages.extend(nh_page::site::Site::regular_pages(&s).iter().cloned());
                }
                nh_page::pages_sort::sort_by_default(&mut pages);
                Ok(pages)
            })
            .unwrap_or_else(|err| panic!("{}", err.message()))
    }

    /// Go: `(s *Site) preparePagesForRender(isRenderingSite, idx)` — `shiftToOutputFormat` on every
    /// page of site `site_idx` (walk order), called for ALL sites before each render format.
    /// (Thin loop owned here; `shift_to_output_format` is T21's page__init.rs; tested by T24.)
    // Go: hugolib/hugo_sites.go:preparePagesForRender
    pub fn prepare_pages_for_render(
        self: &Arc<Self>,
        site_idx: usize,
        is_rendering_site: bool,
        idx: usize,
    ) -> Result<()> {
        self.for_each_page_including_bundled_pages(site_idx, &mut |p| {
            p.shift_to_output_format(self, is_rendering_site, idx)?;
            Ok(false)
        })
    }

    /// Go: `(m *pageMap) forEeachPageIncludingBundledPages(nil, fn)` — the pages of the site's
    /// language in `treePages` walk order, then the bundled pages in `treeResources`.
    // Go: hugolib/content_map_page.go:forEeachPageIncludingBundledPages
    pub fn for_each_page_including_bundled_pages(
        &self,
        site_idx: usize,
        f: &mut dyn FnMut(&PageState) -> Result<bool>,
    ) -> Result<()> {
        let cfg = WalkConfig {
            dims: self.sites[site_idx].page_map.dims,
            ..Default::default()
        };
        self.page_trees
            .tree_pages
            .walk(&cfg, |_w, _key, n, _match| {
                if let Some(id) = n.page_id() {
                    return f(self.page(id));
                }
                Ok(false)
            })?;
        self.page_trees
            .tree_resources
            .walk(&cfg, |_w, _key, n, _match| {
                if let ContentNode::Resource(rs) = n
                    && let Some(id) = rs.page
                {
                    return f(self.page(id));
                }
                Ok(false)
            })
    }

    /// Go: `(h *HugoSites) resolveSite(lang)` — `""` is the default content language.
    // Go: hugolib/hugo_sites.go:resolveSite
    pub fn resolve_site(&self, lang: &str) -> Option<usize> {
        let lang = if lang.is_empty() {
            self.deps.conf.default_content_language()
        } else {
            lang.to_string()
        };
        self.sites.iter().position(|s| s.language.lang == lang)
    }

    /// Go: `gitInfoForPage(p)` / `codeownersForPage(p)`: `enableGitInfo` is off for every site
    /// in scope (Git info and CODEOWNERS are not ported); an explicit error otherwise.
    // Go: hugolib/hugo_sites.go:gitInfoForPage
    pub fn git_info_for_page(&self) -> Result<()> {
        if self.configs.base.root.enable_git_info {
            return Err(Error::new(
                "failed to load Git data: neohugo-rs: enableGitInfo is not supported",
            ));
        }
        Ok(())
    }

    /// Go: `pickOneAndLogTheRest(errors)` — the first error with a file position (else the
    /// first); up to 5 of the others are logged.
    // Go: hugolib/hugo_sites.go:pickOneAndLogTheRest
    pub fn pick_one_and_log_the_rest(&self, errors: Vec<Error>) -> Option<Error> {
        if errors.is_empty() {
            return None;
        }

        let mut i = 0;
        for (j, err) in errors.iter().enumerate() {
            // If this is in server mode, we want to return an error to the client
            // with a file context, if possible.
            if err.pos().is_some() {
                i = j;
                break;
            }
        }

        // Log the rest, but add a threshold to avoid flooding the log.
        const ERR_LOG_THRESHOLD: usize = 5;

        for (j, err) in errors.iter().enumerate() {
            if j == i {
                continue;
            }
            if j >= ERR_LOG_THRESHOLD {
                break;
            }
            self.deps.log.errorf(err.to_string());
        }

        errors.into_iter().nth(i)
    }

    /// Go: `PrintProcessingStats(w)`.
    // Go: hugolib/hugo_sites.go:PrintProcessingStats
    pub fn print_processing_stats(&self, w: &mut dyn Write) {
        let stats: Vec<_> = self
            .sites
            .iter()
            .map(|s| s.deps.path_spec().processing_stats.clone())
            .collect();
        let refs: Vec<_> = stats.iter().map(|s| s.as_ref()).collect();
        nh_helpers::processing_stats::processing_stats_table(w, &refs);
    }
}

/// Go: `newHugoSites(cfg, d, pageTrees, sites)`.
// Go: hugolib/site.go:newHugoSites
fn new_hugo_sites(
    cfg: NewHugoSitesCfg,
    d: Arc<Deps>,
    page_trees: PageTrees,
    sites: Vec<Site>,
) -> Result<HugoSites> {
    // Assemble dependencies to be used in neohugo.Deps.
    fn dep_from_mod(m: &nh_hugofs::modules::module::Module) -> Arc<Dependency> {
        Arc::new(Dependency {
            path: m.path().to_string(),
            version: m.version().to_string(),
            time: Some(m.time()),
            vendor: m.vendor(),
            // These are pointers, but this all came from JSON so there's no recursive
            // navigation, so just create new values.
            replace: m.replace().map(|r| dep_from_mod(&r)),
            owner: m.owner().map(|o| dep_from_mod(o)),
        })
    }
    let dependencies: Vec<Arc<Dependency>> = d
        .path_spec()
        .all_modules()
        .iter()
        .map(|m| dep_from_mod(m))
        .collect();

    let hugo_info = HugoInfo::new_with_deps(
        Arc::new(HugoInfoConf(cfg.configs.get_first_language_config())),
        dependencies,
    );

    let mut h = HugoSites {
        deps: sites[0].deps.clone(),
        sites,
        configs: cfg.configs.clone(),
        hugo_info,
        render_formats: Formats::default(),
        current_site: AtomicUsize::new(0),
        pages: Vec::new(),
        page_trees,
        data: OnceLock::new(),
        cache_pages: Partition::new("/pags/all"),
        template_executor: None,
        translation_provider: TranslationProvider::new(),
        page_id_counter: AtomicU64::new(0),
        build_counter: AtomicU64::new(0),
        fatal_error_handler: FatalErrorHandler::default(),
        self_ref: Arc::new(OnceLock::new()),
    };

    let factory = cfg.func_map_factory.clone();
    let create_func_map = |deps: &Arc<Deps>| -> nh_tplimpl::engine::FuncMap {
        match &factory {
            Some(f) => f(deps),
            None => nh_tplfuncs::tplimplinit::create_func_map(deps),
        }
    };

    let mut prototype: Option<Arc<Deps>> = None;
    for i in 0..h.sites.len() {
        let s = &h.sites[i];
        // The template store needs to be initialized after the h container is set on s.
        let template_store = if i == 0 {
            TemplateStore::new(
                StoreOptions {
                    fs: s
                        .deps
                        .path_spec()
                        .base_fs
                        .source_filesystems
                        .layouts
                        .fs
                        .clone(),
                    log: Some(s.deps.log.clone()),
                    path_parser: s.deps.conf.path_parser(),
                    output_formats: s
                        .conf
                        .output_formats
                        .as_ref()
                        .map(|n| n.config.clone())
                        .unwrap_or_default(),
                    media_types: s
                        .conf
                        .media_types
                        .as_ref()
                        .map(|n| n.config.clone())
                        .unwrap_or_default(),
                    default_content_language: s.deps.conf.default_content_language(),
                    default_output_format: s.conf.root.default_output_format.clone(),
                    taxonomy_singular_plural: s.conf.taxonomies.clone(),
                    watching: s.deps.conf.watching(),
                    render_hooks: s.conf.markup.goldmark.render_hooks.clone(),
                    named_types: Arc::new(crate::tplapi::named_types::registry()),
                },
                SiteOptions {
                    site: s.deps.site.clone(),
                    template_funcs: Arc::new(create_func_map(&s.deps)),
                },
            )?
        } else {
            let proto = prototype.as_ref().expect("prototype deps");
            proto.get_template_store().with_site_opts(SiteOptions {
                site: s.deps.site.clone(),
                template_funcs: Arc::new(create_func_map(&s.deps)),
            })
        };
        let _ = s.deps.template_store.set(template_store.clone());
        let _ = s.template_store.set(template_store);
        compile_deps(&h.translation_provider, &s.deps, prototype.as_deref())?;
        if i == 0 {
            prototype = Some(s.deps.clone());
        }
    }

    // Go `h.init.data` / `h.init.gitInfo` lazy inits: `HugoSites.data` (T23) and
    // `git_info_for_page`.
    h.current_site = AtomicUsize::new(0);

    Ok(h)
}

/// Go: `(d *Deps) Compile(prototype)` — the i18n translators: `TranslationProvider.NewResource`
/// for the first site (no prototype), `CloneResource` for the others.
// Go: deps/deps.go:Compile
pub fn compile_deps(tp: &TranslationProvider, d: &Deps, prototype: Option<&Deps>) -> Result<()> {
    match prototype {
        None => tp.new_resource(d),
        Some(prototype) => tp.clone_resource(d, prototype),
    }
}

/// Go: `contentMapConfig` as `newPageMap` builds it from the site config.
// Go: hugolib/content_map_page.go:newPageMap
pub fn new_content_map_config(
    conf: &nh_allconfig::allconfig::Config,
    lang: &str,
) -> ContentMapConfig {
    ContentMapConfig {
        lang: lang.to_string(),
        taxonomy_config: taxonomies_config_values(&conf.taxonomies),
        taxonomy_disabled: !conf.is_kind_enabled(nh_common::kinds::KIND_TAXONOMY),
        taxonomy_term_disabled: !conf.is_kind_enabled(nh_common::kinds::KIND_TERM),
        page_disabled: !conf.is_kind_enabled(nh_common::kinds::KIND_PAGE),
        is_rebuild: false,
    }
}

/// Go: `siteRefLinker` (site.go): the ref/relref not-found handling. T23 owns the `Site` side
/// (`refLink`); construction computes it here.
#[derive(Clone)]
pub struct SiteRefLinker {
    /// Go `errorLogger`: warnings instead of errors when `refLinksErrorLevel` is "warning".
    pub log_warning: bool,
    pub logger: Logger,
    pub not_found_url: String,
}

/// Go: `newSiteRefLinker(s)`.
// Go: hugolib/site.go:newSiteRefLinker
pub fn new_site_ref_linker(conf: &nh_allconfig::allconfig::Config, log: &Logger) -> SiteRefLinker {
    let not_found_url = conf.root.ref_links_not_found_url.clone();
    let err_level = &conf.root.ref_links_error_level;
    SiteRefLinker {
        log_warning: go_unicode::strings::equal_fold_str(err_level, "warning"),
        logger: log.clone(),
        not_found_url,
    }
}

/// `neohugo.NewInfo`'s config view of a site config (Go passes the `config.AllProvider`).
struct HugoInfoConf(Arc<dyn AllProvider>);

impl HugoInfoConfig for HugoInfoConf {
    fn environment(&self) -> String {
        self.0.environment()
    }
    fn running(&self) -> bool {
        self.0.running()
    }
    fn working_dir(&self) -> String {
        self.0.working_dir()
    }
    fn is_multihost(&self) -> bool {
        self.0.is_multihost()
    }
    fn is_multilingual(&self) -> bool {
        self.0.is_multilingual()
    }
}

/// Go: `fatalErrorHandler` — used in some rare situations where it does not make sense to
/// continue processing, to abort as soon as possible and log the error.
#[derive(Default)]
pub struct FatalErrorHandler {
    err: Mutex<Option<Error>>,
    done: std::sync::atomic::AtomicBool,
}

impl FatalErrorHandler {
    // Go: hugolib/hugo_sites.go:FatalError
    pub fn fatal_error(&self, err: Error) {
        self.done.store(true, Ordering::SeqCst);
        *self.err.lock().unwrap_or_else(|e| e.into_inner()) = Some(err);
    }

    // Go: hugolib/hugo_sites.go:getErr
    pub fn get_err(&self) -> Option<Error> {
        self.err.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Go: `Done()` (a channel closed on the first fatal error).
    // Go: hugolib/hugo_sites.go:Done
    pub fn done(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }
}

/// Builds a site's template func map (Go `tplimplinit.CreateFuncMap(s.Deps)`).
pub type FuncMapFactory = Arc<dyn Fn(&Arc<Deps>) -> nh_tplimpl::engine::FuncMap + Send + Sync>;

/// Inputs of `NewHugoSites` (Go: `deps.DepsCfg`).
pub struct NewHugoSitesCfg {
    pub configs: Arc<Configs>,
    pub fs: nh_hugofs::fs::Fs,
    pub log: nh_common::loggers::Logger,
    /// `None` = `nh_tplfuncs::tplimplinit::create_func_map` (production). Tests may pass a
    /// names-only func map (see the module docs).
    pub func_map_factory: Option<FuncMapFactory>,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites.go (616 lines; 18/30 funcs executed) — Data/loadData/handleDataFile/readData
//          -> hugo_sites_data.rs (T23)
//   types: HugoSites, buildCounters, fatalErrorHandler, hugoSitesInit, BuildCfg
//    L109-113: (h *HugoSites) ShouldSkipFileChangeEvent(ev fsnotify.Event) bool
// OK L115-117: (h *HugoSites) Close() error
// OK L119-121: (h *HugoSites) isRebuild() bool
// OK L123-135: (h *HugoSites) resolveSite(lang string) *Site
// EX L142-147: (c *buildCounters) loggFields() logg.Fields
// OK L162-170: (f *fatalErrorHandler) FatalError(err error)
// OK L172-176: (f *fatalErrorHandler) getErr() error
// OK L178-180: (f *fatalErrorHandler) Done() <-chan bool
// OK L199-213: (h *HugoSites) Pages() page.Pages
// OK L216-231: (h *HugoSites) RegularPages() page.Pages
// OK L233-243: (h *HugoSites) gitInfoForPage(p page.Page) (*source.GitInfo, error)
// OK L245-255: (h *HugoSites) codeownersForPage(p page.Page) ([]string, error)
// OK L257-289: (h *HugoSites) pickOneAndLogTheRest(errors []error) error
//    L291-293: (h *HugoSites) isMultilingual() bool
//    L296-302: (h *HugoSites) LanguageSet() map[string]int
//    L304-309: (h *HugoSites) NumLogErrors() int
// OK L311-317: (h *HugoSites) PrintProcessingStats(w io.Writer)
//    L321-346: (h *HugoSites) GetContentPage(filename string) page.Page
// EX L348-365: (h *HugoSites) loadGitInfo() error  [unsupported: enableGitInfo off in scope]
//    L368-373: (h *HugoSites) reset(config *BuildCfg)
//    L376-381: (h *HugoSites) resetLogs()
//    L383-390: (h *HugoSites) withSite(fn func(s *Site) error) error
//    L392-403: (h *HugoSites) withPage(fn func(s string, p *pageState) bool)
// EX L432-479: (cfg *BuildCfg) shouldRender(infol logg.LevelLogger, p *pageState) bool  [T24: needs the render state of T21-T23]
// OK L481-496: (s *Site) preparePagesForRender(isRenderingSite bool, idx int) error
// Source: hugolib/site.go (construction only; the rest of site.go is in site.rs and others)
// OK L143-335: NewHugoSites(cfg deps.DepsCfg) (*HugoSites, error)
// OK L337-454: newHugoSites(cfg deps.DepsCfg, d *deps.Deps, pageTrees *pageTrees, sites []*Site) (*HugoSites, error)
// OK L858-867: newSiteRefLinker(s *Site) (siteRefLinker, error)
// ---------------------------------------------------------------------------
