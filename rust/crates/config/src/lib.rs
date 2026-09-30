//! The configuration of a neohugo project (docs/rust-port/REWRITE_PLAN.md §2.4, §3.1 A1).
//!
//! One `Value`-tree pipeline, then typed structs:
//!
//! 1. **Bootstrap**: the environment and the config directory come from [`CliOverrides`] and
//!    `HUGO_ENVIRONMENT`/`HUGO_ENV`.
//! 2. **Sources**: the project file (`hugo.toml` … `config.json`, or the explicit list), then
//!    `config/_default/**` and `config/<environment>/**` (file names place their content:
//!    `params.toml` under `params`, `menus.en.toml` under `languages.en.menus`).
//! 3. **Normalise** each tree ([`tree::normalize_keys`]) and migrate legacy keys
//!    ([`tree::migrate_legacy_keys`]).
//! 4. **Merge once**: file < directory < CLI < environment ([`env`]).
//! 5. **Per language**: `languages.X` over the root (`params` merge deeply, `menus`,
//!    `taxonomies` and `permalinks` replace).
//! 6. **Typed decode** with serde ([`de`]) into structs whose `Default` holds Hugo's defaults.
//!
//! Errors point at the file, line and column the offending value was written on.

#![forbid(unsafe_code)]

pub mod de;
pub mod duration;
pub mod env;
mod error;
pub mod global;
pub mod markup;
pub mod media;
pub mod output;
pub mod sections;
pub mod site;
mod source;
pub mod tree;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use neohugo_base::diag::Diagnostic;
use neohugo_base::{IdVec, Idx, LangIdx, Map, Params, Value};
use serde::Serialize;

pub use error::ConfigError;
pub use global::{
    BuildConfig, CachesConfig, ContentFilter, Dirs, ImagingConfig, MinifyConfig, MountConfig,
    PrivacyConfig, SecurityPolicy,
};
pub use markup::MarkupConfig;
pub use media::{ContentTypes, MediaType, MediaTypes};
pub use output::{OutputFormat, OutputFormats};
pub use sections::{
    CascadeConfig, CascadeTarget, DateField, DateSource, KindOutputs, Permalinks, SitemapConfig,
    TaxonomyDef, decode_cascade, decode_front_matter,
};
pub use site::{Direction, Language, SiteConfig, TitleConfig};

/// Settings from the command line; they override the configuration files (the environment
/// overrides them in turn).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CliOverrides {
    /// `--baseURL`.
    pub base_url: Option<String>,
    /// `--environment` (default `production`).
    pub environment: Option<String>,
    /// `--destination`: the publish directory.
    pub destination: Option<PathBuf>,
    /// `--minify`.
    pub minify: Option<bool>,
    /// `--buildDrafts`.
    pub build_drafts: Option<bool>,
    /// `--buildFuture`.
    pub build_future: Option<bool>,
    /// `--buildExpired`.
    pub build_expired: Option<bool>,
    /// `--cacheDir`.
    pub cache_dir: Option<PathBuf>,
    /// `--themesDir`.
    pub themes_dir: Option<PathBuf>,
    /// `--theme` (comma-separated in the CLI).
    pub theme: Option<Vec<String>>,
    /// `--ignoreCache`.
    pub ignore_cache: Option<bool>,
    /// `--configDir` (default `config`).
    pub config_dir: Option<PathBuf>,
}

impl CliOverrides {
    /// The overrides as a configuration tree (lower-case keys).
    #[must_use]
    pub fn to_tree(&self) -> Map {
        let mut m = Map::new();
        let path_str = |p: &Path| Value::string(&p.to_string_lossy());
        let mut set = |k: &str, v: Option<Value>| {
            if let Some(v) = v {
                tree::set_path(&mut m, k, v);
            }
        };
        set("baseurl", self.base_url.as_deref().map(Value::string));
        set(
            "environment",
            self.environment.as_deref().map(Value::string),
        );
        set("publishdir", self.destination.as_deref().map(path_str));
        set("minify.minifyoutput", self.minify.map(Value::Bool));
        set("builddrafts", self.build_drafts.map(Value::Bool));
        set("buildfuture", self.build_future.map(Value::Bool));
        set("buildexpired", self.build_expired.map(Value::Bool));
        set("cachedir", self.cache_dir.as_deref().map(path_str));
        set("themesdir", self.themes_dir.as_deref().map(path_str));
        set(
            "theme",
            self.theme
                .as_ref()
                .map(|t| Value::array(t.iter().map(|s| Value::string(s)).collect())),
        );
        set("ignorecache", self.ignore_cache.map(Value::Bool));
        m
    }
}

/// What to load.
#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    /// The project directory (`--source`).
    pub source: PathBuf,
    /// `--config` files, relative to `source`; the first has the highest precedence. Empty:
    /// the first of `hugo.toml`, `hugo.yaml`, `hugo.yml`, `hugo.json`, `config.*`.
    pub config_files: Vec<PathBuf>,
    pub cli: CliOverrides,
    /// The process environment: `HUGO_*` overrides, and `HOME`, `XDG_CACHE_HOME`, `TMPDIR`
    /// and `USER` for the default cache directory.
    pub env: Vec<(String, String)>,
}

/// The configuration of a project: settings shared by every language, and one
/// [`SiteConfig`] per enabled language.
#[derive(Clone, Debug, Serialize)]
pub struct Config {
    pub project_dir: PathBuf,
    /// `production`, `development`, …
    pub environment: String,
    /// The files the configuration was read from, lowest precedence first.
    pub config_files: Vec<PathBuf>,
    /// Enabled languages: the default language first, then by (weight, key).
    pub sites: IdVec<LangIdx, SiteConfig>,
    /// Languages switched off with `disabled` or `disableLanguages`.
    pub disabled_languages: Vec<String>,
    /// Every language has its own `baseURL`.
    pub multihost: bool,
    /// The default language's content is under `/<lang>/` too.
    pub default_language_in_subdir: bool,
    pub output_formats: Arc<OutputFormats>,
    pub media_types: Arc<MediaTypes>,
    pub content_types: ContentTypes,
    /// The format of `.Permalink` and friends outside a page context (`defaultOutputFormat`).
    pub default_output_format: String,
    pub dirs: Dirs,
    /// The resolved `cacheDir`.
    pub cache_dir: PathBuf,
    /// `[[module.mounts]]` as configured (default mounts are added by the file system layer).
    pub mounts: Vec<MountConfig>,
    /// `theme`.
    pub themes: Vec<String>,
    pub build: BuildConfig,
    pub caches: CachesConfig,
    pub security: SecurityPolicy,
    pub privacy: PrivacyConfig,
    pub imaging: ImagingConfig,
    pub minify: MinifyConfig,
    pub content: ContentFilter,
    /// Maximum time for one template execution.
    pub timeout: Duration,
    /// `ignoreFiles`: regular expressions of content paths to skip.
    pub ignore_files: Vec<String>,
    /// `ignoreLogs`: diagnostic ids to drop (lower case).
    pub ignore_logs: Vec<String>,
    pub enable_git_info: bool,
    /// The merged configuration tree (root level, keys lower case).
    pub raw: Params,
    /// Deprecations and other notices found while loading.
    #[serde(skip)]
    pub diagnostics: Vec<Diagnostic>,
}

impl Config {
    /// The default language's configuration.
    #[must_use]
    pub fn default_site(&self) -> &SiteConfig {
        &self.sites[LangIdx::from_index(0)]
    }

    /// The configuration of the language with key `key`.
    #[must_use]
    pub fn site(&self, key: &str) -> Option<&SiteConfig> {
        self.sites.iter().find(|s| s.language.key == key)
    }
}

/// Loads the configuration of `o.source`.
///
/// # Errors
/// Unreadable or invalid files, values of the wrong type, or inconsistent languages.
pub fn load(o: &LoadOptions) -> Result<Config, ConfigError> {
    Loader::new(o).run()
}

struct Loader<'a> {
    o: &'a LoadOptions,
    sources: source::Sources,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Loader<'a> {
    fn new(o: &'a LoadOptions) -> Self {
        Self {
            o,
            sources: source::Sources::default(),
            diagnostics: Vec::new(),
        }
    }

    fn env(&self, name: &str) -> Option<&str> {
        self.o
            .env
            .iter()
            .rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn run(mut self) -> Result<Config, ConfigError> {
        let project = self.o.source.clone();
        let environment = self
            .o
            .cli
            .environment
            .clone()
            .or_else(|| self.env("HUGO_ENVIRONMENT").map(str::to_owned))
            .or_else(|| self.env("HUGO_ENV").map(str::to_owned))
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| "production".to_owned());

        // Step 2: sources.
        let config_dir = project.join(
            self.o
                .cli
                .config_dir
                .as_deref()
                .unwrap_or(Path::new("config")),
        );
        self.sources.files = source::project_files(&project, &self.o.config_files)?;
        for sub in ["_default", environment.as_str()] {
            let dir = config_dir.join(sub);
            if dir.is_dir() {
                self.sources.files.extend(source::dir_files(&dir)?);
            }
        }
        if self.sources.files.is_empty() {
            return Err(ConfigError::NotFound { dir: project });
        }

        // Steps 3 and 4: normalise, migrate, merge.
        let mut root = self.sources.merged();
        self.migrate(&mut root);
        tree::merge_deep(&mut root, &tree::normalize_keys(&self.o.cli.to_tree()));
        let hugo_env: Vec<(String, String)> = self
            .o
            .env
            .iter()
            .filter(|(k, _)| k.starts_with("HUGO"))
            .cloned()
            .collect();
        env::apply(&mut root, &hugo_env);
        let mut root = tree::normalize_keys(&root);
        self.migrate(&mut root);
        for key in ["disablekinds", "disablelanguages"] {
            if let Some(v) = root.get(key) {
                let v = tree::split_list(v);
                root.insert(key, v);
            }
        }

        // Step 5: languages.
        let langs = self.languages(&root)?;
        let root = strip_map(&root);
        let default_tree = &langs.trees[0];

        // Step 6: typed decode. Project-wide settings come from the default language.
        let media_types = MediaTypes::decode(&section(default_tree, "mediatypes"))
            .map_err(|e| self.locate(e, &langs.keys[0]))?;
        let output_formats =
            OutputFormats::decode(&section(default_tree, "outputformats"), &media_types)
                .map_err(|e| self.locate(e, &langs.keys[0]))?;
        let content_types =
            ContentTypes::decode(&section(default_tree, "contenttypes"), &media_types)
                .map_err(|e| self.locate(e, &langs.keys[0]))?;
        let duplicate_resources =
            tree::get_path(default_tree, "markup.goldmark.duplicateresourcefiles")
                .and_then(de::weak_bool)
                .unwrap_or(false);
        let use_embedded = if langs.configured && !langs.multihost && !duplicate_resources {
            markup::UseEmbedded::Fallback
        } else {
            markup::UseEmbedded::Auto
        };

        let default_has_tags = match root.get("taxonomies") {
            Some(Value::Map(m)) => m.contains_key("tag"),
            Some(_) => false,
            None => true,
        };
        let mut sites = IdVec::with_capacity(langs.keys.len());
        for (i, (key, tree)) in langs.keys.iter().zip(&langs.trees).enumerate() {
            let lang = LangIdx::from_index(i);
            let url_prefix = if i == 0 && !langs.in_subdir {
                String::new()
            } else {
                key.clone()
            };
            let own = langs.own.get(key).cloned().unwrap_or_default();
            let cx = site::SiteContext {
                lang,
                key,
                own: &own,
                url_prefix,
                output_formats: &output_formats,
                use_embedded,
                default_has_tags,
                diagnostics: &mut self.diagnostics,
            };
            let site = site::decode_site(tree, cx).map_err(|e| self.locate(e, key))?;
            sites.push(site);
        }

        let g = self
            .global(default_tree, &root, &project)
            .map_err(|e| self.locate(e, &langs.keys[0]))?;
        if output_formats.by_name(&g.default_output_format).is_none() {
            return Err(self.locate(
                ConfigError::invalid(
                    "defaultOutputFormat",
                    format_args!("unknown output format {:?}", g.default_output_format),
                ),
                &langs.keys[0],
            ));
        }

        Ok(Config {
            project_dir: project,
            environment,
            config_files: self
                .sources
                .files
                .iter()
                .map(|s| s.path.to_path_buf())
                .collect(),
            sites,
            disabled_languages: langs.disabled,
            multihost: langs.multihost,
            default_language_in_subdir: langs.in_subdir,
            output_formats: Arc::new(output_formats),
            media_types: Arc::new(media_types),
            content_types,
            default_output_format: g.default_output_format,
            dirs: g.dirs,
            cache_dir: g.cache_dir,
            mounts: g.mounts,
            themes: g.themes,
            build: g.build,
            caches: g.caches,
            security: g.security,
            privacy: g.privacy,
            imaging: g.imaging,
            minify: g.minify,
            content: g.content,
            timeout: g.timeout,
            ignore_files: g.ignore_files,
            ignore_logs: g.ignore_logs,
            enable_git_info: g.enable_git_info,
            raw: Params::fold(&root),
            diagnostics: self.diagnostics,
        })
    }

    /// Migrates legacy keys at the root and in each language table.
    fn migrate(&mut self, root: &mut Map) {
        let mut done = tree::migrate_legacy_keys(root);
        if let Some(Value::Map(langs)) = root.get_mut("languages") {
            for (_, l) in Arc::make_mut(langs).iter_mut() {
                if let Value::Map(l) = l {
                    done.extend(tree::migrate_legacy_keys(Arc::make_mut(l)));
                }
            }
        }
        for m in done {
            let message = if m.to.is_empty() {
                format!("config: {} is no longer supported and is ignored", m.from)
            } else {
                format!("config: {} is deprecated; use {}", m.from, m.to)
            };
            self.diagnostics.push(
                Diagnostic::warning(message)
                    .with_id(format!("deprecated-config-{}", m.from.to_lowercase())),
            );
        }
    }

    /// Adds the file position of the offending key to a value error.
    fn locate(&self, e: ConfigError, lang: &str) -> ConfigError {
        match e {
            ConfigError::Invalid {
                key,
                position: None,
                message,
            } => {
                let segs = key_segments(&key);
                let mut in_lang = vec!["languages".to_owned(), lang.to_owned()];
                in_lang.extend(segs.iter().cloned());
                match self
                    .sources
                    .locate(&in_lang)
                    .or_else(|| self.sources.locate(&segs))
                {
                    Some(found) => ConfigError::Invalid {
                        key: found.dotted_key(),
                        position: Some(found.position),
                        message,
                    },
                    None => ConfigError::Invalid {
                        key,
                        position: None,
                        message,
                    },
                }
            }
            other => other,
        }
    }

    fn languages(&self, root: &Map) -> Result<Languages, ConfigError> {
        let configured_tables = root
            .get("languages")
            .and_then(Value::as_map)
            .cloned()
            .unwrap_or_default();
        let configured = !configured_tables.is_empty();
        let explicit_default = root
            .get("defaultcontentlanguage")
            .and_then(de::weak_string)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase());
        let mut tables: Vec<(String, Map)> = Vec::new();
        if configured {
            for (k, v) in configured_tables.iter() {
                match v {
                    Value::Map(m) => tables.push((k.to_lowercase(), (**m).clone())),
                    Value::Null => tables.push((k.to_lowercase(), Map::new())),
                    _ => {
                        return Err(self.locate(
                            ConfigError::invalid(format!("languages.{k}"), "expected a table"),
                            k,
                        ));
                    }
                }
            }
        } else {
            tables.push(("en".to_owned(), Map::new()));
        }
        let disable_list: Vec<String> = root
            .get("disablelanguages")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(de::weak_string)
                    .map(|s| s.to_lowercase())
                    .collect()
            })
            .unwrap_or_default();
        let multihost = tables.iter().any(|(_, t)| t.contains_key("baseurl"));
        let mut all: Vec<(i64, String, Map, bool)> = Vec::with_capacity(tables.len());
        for (key, table) in tables {
            let is_disabled = disable_list.contains(&key)
                || table
                    .get("disabled")
                    .and_then(de::weak_bool)
                    .unwrap_or(false);
            let weight = match table.get("weight") {
                None | Some(Value::Null) => 0,
                Some(v) => v
                    .as_i64()
                    .or_else(|| de::weak_string(v).and_then(|s| s.trim().parse().ok()))
                    .ok_or_else(|| {
                        self.locate(
                            ConfigError::invalid(
                                format!("languages.{key}.weight"),
                                "expected an integer",
                            ),
                            &key,
                        )
                    })?,
            };
            all.push((weight, key, table, is_disabled));
        }
        all.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        // Without `defaultContentLanguage`: `en` if configured, else the first language.
        let default = explicit_default.unwrap_or_else(|| {
            if all.iter().any(|(_, k, _, _)| k == "en") {
                "en".to_owned()
            } else {
                all.iter()
                    .find(|(.., d)| !d)
                    .map_or_else(|| "en".to_owned(), |(_, k, _, _)| k.clone())
            }
        });
        if !all.iter().any(|(_, k, _, _)| *k == default) {
            return Err(self.locate(
                ConfigError::invalid(
                    "defaultContentLanguage",
                    format_args!("{default:?} is not one of the configured languages"),
                ),
                &default,
            ));
        }
        let mut enabled: Vec<(i64, String, Map)> = Vec::new();
        let mut disabled = Vec::new();
        for (weight, key, table, is_disabled) in all {
            if is_disabled {
                if key == default {
                    return Err(ConfigError::invalid(
                        "disableLanguages",
                        format_args!("the default content language {key:?} cannot be disabled"),
                    ));
                }
                disabled.push(key);
            } else {
                enabled.push((weight, key, table));
            }
        }
        let pos = enabled
            .iter()
            .position(|(_, k, _)| *k == default)
            .expect("the default language is enabled");
        let first = enabled.remove(pos);
        enabled.insert(0, first);

        let mut base = root.clone();
        base.remove("languages");
        let mut keys = Vec::with_capacity(enabled.len());
        let mut trees = Vec::with_capacity(enabled.len());
        let mut own = std::collections::BTreeMap::new();
        for (_, key, table) in enabled {
            let mut t = base.clone();
            merge_language(&mut t, &table);
            keys.push(key.clone());
            trees.push(strip_map(&t));
            own.insert(key, table);
        }
        Ok(Languages {
            keys,
            trees,
            own,
            disabled,
            configured,
            multihost,
            in_subdir: root
                .get("defaultcontentlanguageinsubdir")
                .and_then(de::weak_bool)
                .unwrap_or(false),
        })
    }

    fn global(&self, t: &Map, root: &Map, project: &Path) -> Result<Global, ConfigError> {
        let dirs = Dirs::from_tree(root);
        let cache_dir = self.cache_dir(t);
        let build: BuildConfig =
            de::from_map(&section(t, "build")).map_err(|e| decode_error("build", &e))?;
        for (i, cb) in build.cache_busters.iter().enumerate() {
            for (field, pattern) in [("source", &cb.source), ("target", &cb.target)] {
                regex::Regex::new(pattern).map_err(|e| {
                    ConfigError::invalid(format!("build.cacheBusters[{i}].{field}"), e)
                })?;
            }
        }
        let ignore_cache = t
            .get("ignorecache")
            .and_then(de::weak_bool)
            .unwrap_or(false);
        let caches = CachesConfig::decode(
            &section(t, "caches"),
            &cache_dir,
            project,
            &dirs.resources,
            ignore_cache,
        )?;
        let security = SecurityPolicy::decode(&section(t, "security"))
            .map_err(|e| decode_error("security", &e))?;
        let privacy: PrivacyConfig =
            de::from_map(&section(t, "privacy")).map_err(|e| decode_error("privacy", &e))?;
        let imaging: ImagingConfig =
            de::from_map(&section(t, "imaging")).map_err(|e| decode_error("imaging", &e))?;
        if !(1..=100).contains(&imaging.quality) {
            return Err(ConfigError::invalid(
                "imaging.quality",
                "must be between 1 and 100",
            ));
        }
        let minify =
            MinifyConfig::decode(&section(t, "minify")).map_err(|e| decode_error("minify", &e))?;
        let mounts: Vec<MountConfig> = match tree::get_path(t, "module.mounts") {
            None | Some(Value::Null) => Vec::new(),
            Some(v) => de::from_value(v).map_err(|e| decode_error("module.mounts", &e))?,
        };
        for (i, m) in mounts.iter().enumerate() {
            let component = m
                .target
                .trim_start_matches(['/', '\\'])
                .split(['/', '\\'])
                .next();
            if !component.is_some_and(|c| global::COMPONENTS.contains(&c)) {
                return Err(ConfigError::invalid(
                    format!("module.mounts[{i}].target"),
                    format_args!(
                        "{:?} is not under a component directory ({})",
                        m.target,
                        global::COMPONENTS.join(", ")
                    ),
                ));
            }
        }
        let timeout = match t.get("timeout") {
            None => Duration::from_secs(60),
            Some(v) => duration::from_value(v)
                .map(|d| {
                    if d.negative {
                        Duration::ZERO
                    } else {
                        d.duration
                    }
                })
                .map_err(|e| ConfigError::invalid("timeout", e))?,
        };
        let strings = |k: &str| -> Vec<String> {
            match t.get(k) {
                Some(Value::Array(a)) => a.iter().filter_map(de::weak_string).collect(),
                Some(v) => de::weak_string(v).into_iter().collect(),
                None => Vec::new(),
            }
        };
        for (i, p) in strings("ignorefiles").iter().enumerate() {
            regex::Regex::new(p)
                .map_err(|e| ConfigError::invalid(format!("ignoreFiles[{i}]"), e))?;
        }
        let flag = |k: &str| t.get(k).and_then(de::weak_bool).unwrap_or(false);
        let default_output_format = t
            .get("defaultoutputformat")
            .and_then(de::weak_string)
            .filter(|s| !s.is_empty())
            .map_or_else(|| "html".to_owned(), |s| s.to_lowercase());
        Ok(Global {
            default_output_format,
            dirs,
            cache_dir,
            mounts,
            themes: strings("theme"),
            build,
            caches,
            security,
            privacy,
            imaging,
            minify,
            content: ContentFilter {
                drafts: flag("builddrafts"),
                future: flag("buildfuture"),
                expired: flag("buildexpired"),
            },
            timeout,
            ignore_files: strings("ignorefiles"),
            ignore_logs: strings("ignorelogs")
                .iter()
                .map(|s| s.to_lowercase())
                .collect(),
            enable_git_info: flag("enablegitinfo"),
        })
    }

    /// `cacheDir`, else `$XDG_CACHE_HOME/hugo_cache` (or `$HOME/.cache/hugo_cache`) when it can
    /// exist, else `$TMPDIR/hugo_cache_$USER`.
    fn cache_dir(&self, t: &Map) -> PathBuf {
        if let Some(dir) = t
            .get("cachedir")
            .and_then(de::weak_string)
            .filter(|s| !s.is_empty())
        {
            // A relative directory is rejected when the caches are resolved.
            return PathBuf::from(dir);
        }
        let user_cache = self
            .env("XDG_CACHE_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| self.env("HOME").map(|h| Path::new(h).join(".cache")));
        if let Some(base) = user_cache {
            let candidate = base.join("hugo_cache");
            let creatable = candidate
                .ancestors()
                .find(|a| a.exists())
                .is_some_and(Path::is_dir);
            if creatable {
                return candidate;
            }
        }
        let tmp = self
            .env("TMPDIR")
            .filter(|s| !s.is_empty())
            .map_or_else(std::env::temp_dir, PathBuf::from);
        match self.env("USER").filter(|s| !s.is_empty()) {
            Some(user) => tmp.join(format!("hugo_cache_{user}")),
            None => tmp.join("hugo_cache"),
        }
    }
}

struct Languages {
    /// Enabled language keys, default first.
    keys: Vec<String>,
    /// The merged tree of each enabled language.
    trees: Vec<Map>,
    /// Each language's own table.
    own: std::collections::BTreeMap<String, Map>,
    disabled: Vec<String>,
    /// `[languages]` is configured.
    configured: bool,
    multihost: bool,
    in_subdir: bool,
}

struct Global {
    default_output_format: String,
    dirs: Dirs,
    cache_dir: PathBuf,
    mounts: Vec<MountConfig>,
    themes: Vec<String>,
    build: BuildConfig,
    caches: CachesConfig,
    security: SecurityPolicy,
    privacy: PrivacyConfig,
    imaging: ImagingConfig,
    minify: MinifyConfig,
    content: ContentFilter,
    timeout: Duration,
    ignore_files: Vec<String>,
    ignore_logs: Vec<String>,
    enable_git_info: bool,
}

/// Merges a language table over the root tree: tables merge deeply, except `menus`,
/// `taxonomies` and `permalinks`, which the language replaces as a whole.
fn merge_language(tree: &mut Map, lang: &Map) {
    for (k, v) in lang.iter() {
        match k {
            "menus" | "taxonomies" | "permalinks" => {
                tree.insert(k, tree::strip_merge(v));
            }
            _ => {
                let mut single = Map::new();
                single.insert(k, v.clone());
                tree::merge_deep(tree, &single);
            }
        }
    }
}

fn strip_map(m: &Map) -> Map {
    match tree::strip_merge(&Value::map(m.clone())) {
        Value::Map(m) => Arc::unwrap_or_clone(m),
        _ => unreachable!("a table stays a table"),
    }
}

fn section(t: &Map, key: &str) -> Map {
    t.get(key)
        .and_then(Value::as_map)
        .cloned()
        .unwrap_or_default()
}

fn key_segments(key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in key.split('.') {
        let (name, idx) = match part.find('[') {
            Some(i) => (&part[..i], Some(&part[i..])),
            None => (part, None),
        };
        if !name.is_empty() {
            out.push(name.to_lowercase());
        }
        if let Some(idx) = idx {
            out.extend(
                idx.split(['[', ']'])
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
            );
        }
    }
    out
}

/// A typed-decode error under `prefix`, as a [`ConfigError::Invalid`] (position added later).
pub(crate) fn decode_error(prefix: &str, e: &de::DeError) -> ConfigError {
    let path = e.dotted_path();
    let key = match (prefix.is_empty(), path.is_empty()) {
        (true, _) => path,
        (false, true) => prefix.to_owned(),
        (false, false) if path.starts_with('[') => format!("{prefix}{path}"),
        (false, false) => format!("{prefix}.{path}"),
    };
    ConfigError::invalid(key, &e.message)
}

/// A table, with `null` (an unset section) as the empty table.
pub(crate) fn de_map<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Map, D::Error> {
    match <Value as serde::Deserialize>::deserialize(d)? {
        Value::Map(m) => Ok(Arc::unwrap_or_clone(m)),
        Value::Null => Ok(Map::new()),
        other => Err(serde::de::Error::custom(format_args!(
            "expected a table, found {other:?}"
        ))),
    }
}
