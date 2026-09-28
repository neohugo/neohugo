//! Port of `hugolib/page__meta.go` (`setMetaPost`, `setMetaPostParams`, `applyDefaultValues`).
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Split from `page__meta.go`: these run only in the assembly walks (`applyAggregates`,
//! content_map_page.go:1445, 1512, 1579), so they belong to the assembly task.
//! * `setMetaPost(cascade)`: applies the cascade, then `setMetaPostParams`, then (first run)
//!   `applyDefaultValues`; terms are delayed until after `assembleTermsAndTranslations`.
//! * `setMetaPostParams()`: the reserved-key switch into `PageConfig`, params normalisation
//!   (lower-cased keys, `[]any` of strings -> `[]string`, dates through the front matter handler,
//!   `draft`/`iscjklanguage`), the normalised `params` map (`maps.Params`).
//! * `applyDefaultValues()`: default titles for pages without a file — home = site title,
//!   section = CreateTitle(Pluralize(dir)), taxonomy, term = CreateTitle(term) (title-cased LAST
//!   term value, see content-model §7.3), 404.
//!
//! Go's `pageMeta.s` (the site) and the page's parsed content (`m.content`, for the CJK
//! detection) are passed in: `site` and `content`.

use go_value::{GoString, Map, MapType, SliceType, Time, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_config::neohugo::neohugo::{DeprecationLevel, deprecate, deprecate_level_min};
use nh_page::page_matcher::{Cascade, PageMatcher};
use nh_page::pagemeta::page_frontmatter::FrontMatterDescriptor;
use nh_page::pagemeta::pagemeta as pm;

use crate::page__content_parse::CachedContent;
use crate::page__meta::PageMeta;
use crate::site::Site;

impl PageMeta {
    /// Go: `(ps *pageState) setMetaPost(cascade)`.
    ///
    /// `cascade` is the cascade passed down the tree (Go's `*maps.Ordered`, nil = `None`).
    // Go: hugolib/page__meta.go:setMetaPost
    pub fn set_meta_post(
        &mut self,
        site: &Site,
        content: Option<&CachedContent>,
        cascade: Option<&Cascade>,
    ) -> Result<()> {
        self.set_meta_post_count += 1;
        // A second run: rebuilds, and in a build a bundled page reached from two pages'
        // resource walks (a leaf page whose key prefixes a leaf bundle's).
        let mut cascade_pre: Option<Option<Cascade>> = None;
        if self.set_meta_post_count > 1 {
            // Go compares `hashing.HashUint64(CascadeCompiled)` before and after (the hash of
            // the Ordered's values map); the port compares the values.
            cascade_pre = Some(self.page_config.cascade_compiled.clone());
            self.page_config.cascade_compiled = self.cascade_original.clone();
        }

        let mut cascade: Option<Cascade> = cascade.cloned();

        // Apply cascades first so they can be overridden later.
        if let Some(c) = &cascade {
            if let Some(own) = &mut self.page_config.cascade_compiled {
                c.range(|k, v| {
                    match own.get(k).cloned() {
                        None => own.set(k.clone(), v.clone()),
                        Some(mut vv) => {
                            // Merge (Go mutates the maps of the page's own entry in place).
                            for (ck, cv) in &v.params.entries {
                                if !vv.params.entries.contains_key(ck) {
                                    vv.params.entries.insert(ck.clone(), cv.clone());
                                }
                            }
                            for (ck, cv) in &v.fields.entries {
                                if !vv.fields.entries.contains_key(ck) {
                                    vv.fields.entries.insert(ck.clone(), cv.clone());
                                }
                            }
                            own.set(k.clone(), vv);
                        }
                    }
                    true
                });
                cascade = Some(own.clone());
            } else {
                self.page_config.cascade_compiled = Some(c.clone());
            }
        }

        if cascade.is_none() {
            cascade = self.page_config.cascade_compiled.clone();
        }

        if let Some(pre) = cascade_pre {
            self.set_meta_post_cascade_changed =
                !cascade_equal(pre.as_ref(), self.page_config.cascade_compiled.as_ref());
            if !self.set_meta_post_cascade_changed {
                // No changes, restore any value that may be changed by aggregation.
                self.page_config.dates = self.dates_original.clone();
                return Ok(());
            }
            self.set_meta_post_prepare_rebuild();
        }

        // Cascade is also applied to itself.
        if let Some(cascade) = &cascade {
            let env = site.deps.conf.environment();
            let (kind, lang, path) = (
                self.kind().to_string(),
                self.lang().to_string(),
                self.path(),
            );
            let is_from_content_adapter = self.page_config.is_from_content_adapter;
            let pc = &mut self.page_config;
            cascade.range(|k, v| {
                if !page_matcher_matches(k, &kind, &lang, &path, &env) {
                    return true;
                }
                for (kk, vv) in &v.params.entries {
                    set_if_not_found(&mut pc.params, kk, vv);
                }

                for (kk, vv) in &v.fields.entries {
                    if is_from_content_adapter {
                        set_if_not_found(&mut pc.content_adapter_data, kk, vv);
                    } else {
                        set_if_not_found(&mut pc.params, kk, vv);
                    }
                }
                true
            });
        }

        self.set_meta_post_params(site, content)?;

        self.apply_default_values(site)?;

        // Store away any original values that may be changed from aggregation.
        self.dates_original = self.page_config.dates.clone();

        Ok(())
    }

    /// Go: `(p *pageState) setMetaPostParams()`.
    // Go: hugolib/page__meta.go:setMetaPostParams
    pub fn set_meta_post_params(
        &mut self,
        site: &Site,
        content: Option<&CachedContent>,
    ) -> Result<()> {
        let mut mtime = Time::zero();
        let mut content_base_name = String::new();
        let mut ext = String::new();
        let mut is_content_adapter = false;
        if let Some(f) = &self.f {
            is_content_adapter = f.is_content_adapter();
            content_base_name = f.content_base_name();
            mtime = f.file_info().mod_time().clone();
            if !is_content_adapter {
                ext = f.ext();
            }
        }

        // Go: `p.gitInfo` is nil (enableGitInfo is not supported).
        let git_author_date = Time::zero();

        let path_or_title = self.path_or_title();
        let log = site.deps.log.clone();
        let output_formats = site
            .conf
            .output_formats
            .as_ref()
            .map(|n| n.config.clone())
            .unwrap_or_default();
        let media_types = site
            .conf
            .media_types
            .as_ref()
            .map(|n| n.config.clone())
            .unwrap_or_default();

        if is_content_adapter {
            self.page_config
                .compile(&ext, Some(&log), &output_formats, &media_types)?;
        }

        // Handle the date separately
        {
            let mut descriptor = FrontMatterDescriptor {
                base_filename: content_base_name,
                path_or_title: path_or_title.clone(),
                mod_time: mtime,
                git_author_date,
                page_config: &mut self.page_config,
                location: site.language.location(),
            };
            if let Err(err) = site.frontmatter_handler.handle_dates(&mut descriptor) {
                log.errorf(format!(
                    "Failed to handle dates for page {}: {}",
                    go_strconv::quote(&path_or_title),
                    err.message()
                ));
            }
        }

        if is_content_adapter {
            // Done.
            return Ok(());
        }

        let params_ref = self.page_config.params.as_ref();
        let (build_config, is_new_build_keyword) = match params_ref
            .and_then(|p| p.get(b"_build"))
            .cloned()
        {
            Some(v) => {
                deprecate(
                    "The \"_build\" front matter key",
                    "Use \"build\" instead. See https://gohugo.io/content-management/build-options.",
                    "0.145.0",
                );
                (v, false)
            }
            None => (
                params_ref
                    .and_then(|p| p.get(b"build"))
                    .cloned()
                    .unwrap_or(Value::Invalid),
                true,
            ),
        };
        self.page_config.build = match pm::decode_build_config(&build_config) {
            Ok(b) => b,
            Err(err) => {
                let mut msg_detail = String::new();
                if is_new_build_keyword {
                    msg_detail = ". We renamed the _build keyword to build in Hugo 0.123.0. We recommend putting user defined params in the params section, e.g.:\n---\ntitle: \"My Title\"\nparams:\n  build: \"My Build\"\n---\n´\n\n".to_string();
                }
                return Err(Error::new(format!(
                    "failed to decode build config in front matter: {}{}",
                    err.message(),
                    msg_detail
                )));
            }
        };

        let mut sitemap_set = false;

        if self.page_config.params.is_none() {
            panic!("params not set for {}", self.title());
        }

        let mut draft: Option<bool> = None;
        let mut published: Option<bool> = None;
        let mut is_cjk_language: Option<bool> = None;
        let mut user_params: Option<Map> = None;

        // Go ranges over the params map; every write below goes to the visited key (the keys
        // are lower-cased by PrepareParams) or deletes it, so a snapshot visits the same keys.
        let snapshot: Vec<(GoString, Value)> = self
            .page_config
            .params
            .as_ref()
            .map(|p| {
                p.entries
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        for (k, v) in snapshot {
            let loki = go_unicode::strings::to_lower_str(&go_str_bytes(k.as_bytes())).into_owned();

            if loki == "params" {
                let vv = nh_common::maps::maps::to_string_map_e(&v)?;
                user_params = Some(vv);
                self.params_mut().entries.remove(&k);
                continue;
            }

            if loki == "published" {
                // Intentionally undocumented
                if let Ok(vv) = nh_common::cast::caste::to_bool_e(&v) {
                    published = Some(vv);
                }
                // published may also be a date
                continue;
            }

            if site.frontmatter_handler.is_date_key(&loki) {
                continue;
            }

            if loki == "path" || loki == "kind" || loki == "lang" {
                // See issue 12484.
                deprecate_level_min(
                    &format!("{loki} in front matter"),
                    "",
                    "v0.144.0",
                    DeprecationLevel::Warn,
                );
            }

            match loki.as_str() {
                "title" => {
                    self.page_config.title = to_str(&v);
                    let t = self.page_config.title.clone();
                    self.set_param(&loki, Value::string(t));
                }
                "linktitle" => {
                    self.page_config.link_title = to_str(&v);
                    let t = self.page_config.link_title.clone();
                    self.set_param(&loki, Value::string(t));
                }
                "summary" => {
                    self.page_config.summary = to_str(&v);
                    let t = self.page_config.summary.clone();
                    self.set_param(&loki, Value::string(t));
                }
                "description" => {
                    self.page_config.description = to_str(&v);
                    let t = self.page_config.description.clone();
                    self.set_param(&loki, Value::string(t));
                }
                "slug" => {
                    // Don't start or end with a -
                    self.page_config.slug = to_str(&v).trim_matches('-').to_string();
                    let t = self.slug().to_string();
                    self.set_param(&loki, Value::string(t));
                }
                "url" => {
                    let url = to_str(&v);
                    if url.starts_with("http://") || url.starts_with("https://") {
                        return Err(Error::new(format!(
                            "URLs with protocol (http*) not supported: {}. In page {}",
                            go_strconv::quote(&url),
                            go_strconv::quote(&path_or_title)
                        )));
                    }
                    self.page_config.url = url.clone();
                    self.set_param(&loki, Value::string(url));
                }
                "type" => {
                    self.page_config.type_ = to_str(&v);
                    let t = self.page_config.type_.clone();
                    self.set_param(&loki, Value::string(t));
                }
                "keywords" => {
                    self.page_config.keywords = to_str_slice(&v);
                    let t = string_slice_value(&self.page_config.keywords);
                    self.set_param(&loki, t);
                }
                "headless" => {
                    // Legacy setting for leaf bundles.
                    // This is since Hugo 0.63 handled in a more general way for all
                    // pages.
                    let is_headless = nh_common::cast::caste::to_bool(&v);
                    self.set_param(&loki, Value::Bool(is_headless));
                    if is_headless {
                        self.page_config.build.list = pm::NEVER.to_string();
                        self.page_config.build.render = pm::NEVER.to_string();
                    }
                }
                "outputs" => {
                    // lower case names:
                    let o: Vec<String> = to_str_slice(&v)
                        .iter()
                        .map(|s| go_unicode::strings::to_lower_str(s).into_owned())
                        .collect();
                    self.page_config.outputs = o;
                }
                "draft" => {
                    draft = Some(nh_common::cast::caste::to_bool(&v));
                }
                "layout" => {
                    self.page_config.layout = to_str(&v);
                    let t = self.page_config.layout.clone();
                    self.set_param(&loki, Value::string(t));
                }
                "markup" => {
                    self.page_config.content.markup = to_str(&v);
                    let t = self.page_config.content.markup.clone();
                    self.set_param(&loki, Value::string(t));
                }
                "weight" => {
                    self.page_config.weight = nh_common::cast::caste::to_int(&v);
                    let w = self.page_config.weight;
                    self.set_param(&loki, Value::int(w));
                }
                "aliases" => {
                    let mut aliases = to_str_slice(&v);
                    for alias in aliases.iter_mut() {
                        if alias.starts_with("http://") || alias.starts_with("https://") {
                            return Err(Error::new(format!(
                                "http* aliases not supported: {}",
                                go_strconv::quote(alias.as_str())
                            )));
                        }
                        *alias = go_path::filepath::to_slash(alias).to_string();
                    }
                    self.page_config.aliases = aliases;
                    let t = string_slice_value(&self.page_config.aliases);
                    self.set_param(&loki, t);
                }
                "sitemap" => {
                    let m = nh_common::maps::maps::to_string_map(&v);
                    match nh_config::common_config::decode_sitemap_partial(
                        site.conf.sitemap.clone(),
                        &Value::map(m),
                    ) {
                        (sm, None) => self.page_config.sitemap = sm,
                        (sm, Some(err)) => {
                            self.page_config.sitemap = sm;
                            return Err(Error::new(format!(
                                "failed to decode sitemap config in front matter: {}",
                                err.message()
                            )));
                        }
                    }
                    sitemap_set = true;
                }
                "iscjklanguage" => {
                    is_cjk_language = Some(nh_common::cast::caste::to_bool(&v));
                }
                "translationkey" => {
                    self.page_config.translation_key = to_str(&v);
                    let t = self.page_config.translation_key.clone();
                    self.set_param(&loki, Value::string(t));
                }
                "resources" => {
                    let mut resources: Vec<Map> = Vec::new();
                    let mut handled = true;
                    match &v {
                        Value::List(l) if l.ty == SliceType::MapStringAny => {
                            for vvv in &l.items {
                                if let Value::Map(m) = vvv {
                                    resources.push((**m).clone());
                                }
                            }
                        }
                        Value::List(l) if l.ty == SliceType::Any => {
                            for vvv in &l.items {
                                // Go: `map[any]any` (ToStringMap) or `map[string]any`; a
                                // `maps.Params` (a named type) matches neither case.
                                if let Value::Map(m) = vvv
                                    && m.ty == MapType::StringAny
                                {
                                    resources.push(nh_common::maps::maps::to_string_map(vvv));
                                }
                            }
                        }
                        _ => handled = false,
                    }

                    if handled {
                        self.page_config.resources_meta = resources;
                    } else {
                        // Go: fallthrough to the default case.
                        self.set_default_param(&loki, v);
                    }
                }
                _ => {
                    // If not one of the explicit values, store in Params
                    self.set_default_param(&loki, v);
                }
            }
        }

        if let Some(user_params) = user_params {
            for (k, v) in user_params.entries {
                let lk =
                    go_unicode::strings::to_lower_str(&go_str_bytes(k.as_bytes())).into_owned();
                self.set_param(&lk, v);
            }
        }

        if !sitemap_set {
            self.page_config.sitemap = site.conf.sitemap.clone();
        }

        if let (Some(d), Some(_)) = (draft, published) {
            self.page_config.draft = d;
            let filename = self
                .f
                .as_ref()
                .map(|f| f.filename().to_string())
                .unwrap_or_default();
            log.warnf(format!(
                "page {} has both draft and published settings in its frontmatter. Using draft.",
                go_strconv::quote(&filename)
            ));
        } else if let Some(d) = draft {
            self.page_config.draft = d;
        } else if let Some(p) = published {
            self.page_config.draft = !p;
        }
        let d = self.page_config.draft;
        self.set_param("draft", Value::Bool(d));

        if let Some(c) = is_cjk_language {
            self.page_config.is_cjk_language = c;
        } else if site.conf.root.has_cjk_language
            && let Some(content) = content
            && content.pi.open_source.is_some()
        {
            self.page_config.is_cjk_language = cjk_re_match(&content.pi.source);
        }

        let c = self.page_config.is_cjk_language;
        self.set_param("iscjklanguage", Value::Bool(c));

        self.page_config.init(false)?;

        self.page_config
            .compile(&ext, Some(&log), &output_formats, &media_types)?;

        Ok(())
    }

    /// Go: `(p *pageMeta) applyDefaultValues()`.
    // Go: hugolib/page__meta.go:applyDefaultValues
    pub fn apply_default_values(&mut self, site: &Site) -> Result<()> {
        if self.page_config.build.is_zero() {
            self.page_config.build = pm::decode_build_config(&Value::Invalid).unwrap_or_default();
        }

        if !site.conf.is_kind_enabled(self.kind()) {
            self.page_config.build.disable();
        }

        if self.page_config.content.markup.is_empty() {
            if let Some(f) = &self.f {
                // Fall back to file extension
                self.page_config.content.markup = site.deps.content_spec().resolve_markup(&f.ext());
            }
            if self.page_config.content.markup.is_empty() {
                self.page_config.content.markup = "markdown".to_string();
            }
        }

        if self.page_config.title.is_empty() && self.f.is_none() {
            let root = &site.conf.root;
            let create_title = |s: &str| (site.conf.compiled().create_title)(s);
            match self.kind() {
                kinds::KIND_HOME => {
                    self.page_config.title = root.title.clone();
                }
                kinds::KIND_SECTION => {
                    let mut section_name = self
                        .path_info
                        .unnormalized()
                        .base_name_no_identifier()
                        .to_string();
                    if root.pluralize_list_titles {
                        section_name = nh_common::flect::pluralize(&section_name);
                    }
                    if root.capitalize_list_titles {
                        section_name = create_title(&section_name);
                    }
                    self.page_config.title = section_name;
                }
                kinds::KIND_TERM => {
                    if !self.term.is_empty() {
                        if root.capitalize_list_titles {
                            self.page_config.title = create_title(&self.term);
                        } else {
                            self.page_config.title = self.term.clone();
                        }
                    } else {
                        panic!("term not set");
                    }
                }
                kinds::KIND_TAXONOMY => {
                    let name = self.path_info.unnormalized().base_name_no_identifier();
                    if root.capitalize_list_titles {
                        self.page_config.title = create_title(name).replace('-', " ");
                    } else {
                        self.page_config.title = name.replace('-', " ");
                    }
                }
                kinds::KIND_STATUS_404 => {
                    self.page_config.title = "404 Page not found".to_string();
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Go: `(m *pageMeta) setMetaPostPrepareRebuild()` — prepare for a rebuild of the data
    /// passed in from front matter (T20's page__meta.go checklist, L72-75; only `setMetaPost`
    /// calls it). Outside watch mode the original params are nil.
    // Go: hugolib/page__meta.go:setMetaPostPrepareRebuild
    fn set_meta_post_prepare_rebuild(&mut self) {
        let params = self.params_original.clone();
        let mut pc = nh_page::pagemeta::page_frontmatter::clone_page_config_for_rebuild(
            &self.page_config,
            params.clone().unwrap_or_else(|| Map::new(MapType::Params)),
        );
        if params.is_none() {
            // Go: `xmaps.Clone(nil)` is nil.
            if pc.is_from_content_adapter {
                pc.content_adapter_data = None;
            } else {
                pc.params = None;
            }
        }
        self.page_config = pc;
    }

    /// Go: `pathOrTitle()` (page.go) over the meta.
    fn path_or_title(&self) -> String {
        if let Some(f) = &self.f {
            return f.filename().to_string();
        }
        let p = self.path();
        if !p.is_empty() {
            return p;
        }
        self.title().to_string()
    }

    fn params_mut(&mut self) -> &mut Map {
        self.page_config
            .params
            .get_or_insert_with(|| Map::new(MapType::Params))
    }

    /// Go: `params[loki] = v`.
    fn set_param(&mut self, k: &str, v: Value) {
        self.params_mut().entries.insert(GoString::from(k), v);
    }

    /// The `default:` branch of the switch: a `[]any` whose elements are all strings becomes a
    /// `[]string`, an empty one an empty `[]string`; anything else is stored as is.
    fn set_default_param(&mut self, loki: &str, v: Value) {
        match &v {
            Value::List(l) if l.ty == SliceType::Any => {
                if !l.items.is_empty() {
                    let all_strings = l.items.iter().all(|vvv| matches!(vvv, Value::String(_)));
                    if all_strings {
                        // We need tags, keywords etc. to be []string, not []interface{}.
                        let a: Vec<Value> = l
                            .items
                            .iter()
                            .map(|u| Value::String(nh_common::cast::caste::to_string(u)))
                            .collect();
                        self.set_param(loki, Value::list(SliceType::String, a));
                    } else {
                        self.set_param(loki, v.clone());
                    }
                } else {
                    self.set_param(loki, Value::list(SliceType::String, Vec::new()));
                }
            }
            _ => self.set_param(loki, v),
        }
    }
}

/// Go: `k.Matches(ps)` (`page.PageMatcher.Matches`, resources/page/page_matcher.go) over the
/// values it reads (`Kind()`, `Lang()`, `Path()`, `Site().Hugo().Environment`): the assembly
/// walks have no `page.Page` handle yet.
// Go: resources/page/page_matcher.go:Matches
pub(crate) fn page_matcher_matches(
    m: &PageMatcher,
    kind: &str,
    lang: &str,
    path: &str,
    environment: &str,
) -> bool {
    if !m.kind.is_empty()
        && let Ok(g) = nh_common::glob::glob::get_glob(&m.kind)
        && !g.matches(kind)
    {
        return false;
    }

    if !m.lang.is_empty()
        && let Ok(g) = nh_common::glob::glob::get_glob(&m.lang)
        && !g.matches(lang)
    {
        return false;
    }

    if !m.path.is_empty() {
        let g = nh_common::glob::glob::get_glob(&m.path);
        // TODO(bep) Path() vs filepath vs leading slash.
        let mut pth =
            go_unicode::strings::to_lower_str(go_path::filepath::to_slash(path)).into_owned();
        if !pth.starts_with('/') {
            pth = format!("/{pth}");
        }
        if let Ok(g) = g
            && !g.matches(&pth)
        {
            return false;
        }
    }

    if !m.environment.is_empty()
        && let Ok(g) = nh_common::glob::glob::get_glob(&m.environment)
        && !g.matches(environment)
    {
        return false;
    }

    true
}

/// Go: `cjkRe.Match(source)` with `cjkRe = \p{Han}|\p{Hangul}|\p{Hiragana}|\p{Katakana}`
/// (RE2 decodes UTF-8; an invalid byte is U+FFFD, which is in none of the scripts).
fn cjk_re_match(src: &[u8]) -> bool {
    let mut i = 0;
    while i < src.len() {
        let (r, size) = go_unicode::utf8::decode_rune(&src[i..]);
        if go_unicode::is(go_unicode::tables::HAN, r)
            || go_unicode::is(go_unicode::tables::HANGUL, r)
            || go_unicode::is(go_unicode::tables::HIRAGANA, r)
            || go_unicode::is(go_unicode::tables::KATAKANA, r)
        {
            return true;
        }
        i += size.max(1);
    }
    false
}

/// Go: `if _, found := m[k]; !found { m[k] = v }` (a nil map has no keys; writing to it
/// panics like Go's "assignment to entry in nil map").
fn set_if_not_found(m: &mut Option<Map>, k: &GoString, v: &Value) {
    if m.as_ref().is_some_and(|m| m.entries.contains_key(k)) {
        return;
    }
    let Some(m) = m.as_mut() else {
        panic!("assignment to entry in nil map");
    };
    m.entries.insert(k.clone(), v.clone());
}

/// `hashing.HashUint64(a) == hashing.HashUint64(b)` for two compiled cascades (Go hashes the
/// Ordered's values map, order-independent; a nil cascade hashes to 0): the same matchers with
/// the same params and fields.
fn cascade_equal(a: Option<&Cascade>, b: Option<&Cascade>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.len() == b.len()
                && a.keys().iter().all(|k| match (a.get(k), b.get(k)) {
                    (Some(x), Some(y)) => {
                        x.params == y.params && x.fields == y.fields && x.target == y.target
                    }
                    _ => false,
                })
        }
        _ => false,
    }
}

/// `cast.ToString(v)` as text (the page config fields are Rust strings).
fn to_str(v: &Value) -> String {
    go_str_bytes(nh_common::cast::caste::to_string(v).as_bytes())
}

/// `cast.ToStringSlice(v)`.
fn to_str_slice(v: &Value) -> Vec<String> {
    nh_common::cast::caste::to_string_slice(v)
        .iter()
        .map(|s| go_str_bytes(s.as_bytes()))
        .collect()
}

fn go_str_bytes(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// A Go `[]string` value.
fn string_slice_value(v: &[String]) -> Value {
    Value::list(
        SliceType::String,
        v.iter().map(|s| Value::string(s.as_str())).collect(),
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__meta.go (setMetaPost part; the rest is in page__meta.rs and page__init.rs)
// OK L293-388: (ps *pageState) setMetaPost(cascade *maps.Ordered[page.PageMatcher, page.PageMatcherParamsConfig]) error
// OK L390-687: (p *pageState) setMetaPostParams() error
// OK L732-786: (p *pageMeta) applyDefaultValues() error
// ---------------------------------------------------------------------------
