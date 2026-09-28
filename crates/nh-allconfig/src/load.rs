//! Port of `config/allconfig/load.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

//! Go `config/allconfig/load.go`: config file discovery (`hugo.toml`...), config dir, flags,
//! `HUGO_*` env overrides (applied twice), default merge strategies (`_merge` keys), aliases,
//! modules (project only), then `fromLoadConfigResult` + `Configs.Init`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use go_value::{Map, MapType, Value};
use nh_common::loggers::Logger;
use nh_common::{Error, Result};
use nh_config::common_config::{BaseConfig, LoadConfigResult};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_hugofs::afero::Fs;
use nh_hugofs::modules::client::{Client, ClientConfig};
use nh_hugofs::modules::collect::ModulesConfig;

use crate::allconfig::{Config, Configs, ConfigsBuild, from_load_config_result, quote};

/// Go: `allconfig.ErrNoConfigFile`.
pub const ERR_NO_CONFIG_FILE: &str = "Unable to locate config file or config directory. Perhaps you need to create a new site.\n       Run `hugo help new` for details.\n";

/// The process environment reader (Go `os.Getenv`, used by `helpers.GetCacheDir`).
pub type Getenv = Arc<dyn Fn(&str) -> String + Send + Sync>;

/// Go: `allconfig.ConfigSourceDescriptor`.
#[derive(Clone, Default)]
pub struct ConfigSourceDescriptor {
    /// Flags from the CLI (already mapped: `minify` -> `minifyOutput`, `destination` -> `publishDir`,
    /// `internal.*` keys).
    pub flags: Option<Arc<dyn Provider>>,
    /// Config filename(s) (`--config`), comma separated; default: discovery.
    pub filename: String,
    /// Config dir (`--configDir`, default "config").
    pub config_dir: String,
    /// "production" by default for builds.
    pub environment: String,
    /// `os.Environ()` (for `HUGO_*` overrides). Empty = the process environment (Go reads
    /// `os.Environ()` when it is empty, unless running as a test).
    pub environ: Vec<String>,
    pub ignore_module_does_not_exist: bool,
    /// The working dir (`--source` or cwd). Go takes it from the `workingDir` flag; when the
    /// flags do not set one, this value is set on them (like the CLI does).
    pub working_dir: String,
    /// Go: `Fs` (`hugofs.Os`; the default). The config files themselves are read from the OS
    /// file system (nh-config's loader).
    pub fs: Option<Arc<dyn Fs>>,
    /// Go: `Logger` (default: `loggers.NewDefault()`).
    pub logger: Option<Logger>,
    /// The process environment of `helpers.GetCacheDir` (`HUGO_CACHEDIR` is not read here but
    /// through `environ`; `NETLIFY`, `USER`, `HOME`/`XDG_CACHE_HOME`, `TMPDIR` are). Default:
    /// `std::env::var`.
    pub getenv: Option<Getenv>,
}

impl ConfigSourceDescriptor {
    // Go: config/allconfig/load.go:configFilenames
    fn config_filenames(&self) -> Option<Vec<String>> {
        if self.filename.is_empty() {
            return None;
        }
        Some(self.filename.split(',').map(String::from).collect())
    }
}

/// Go: `configLoader`.
struct ConfigLoader<'a> {
    cfg: Arc<DefaultConfigProvider>,
    base_config: BaseConfig,
    d: &'a ConfigSourceDescriptor,
    fs: Arc<dyn Fs>,
    getenv: Getenv,
    /// Collected (Go `ModulesConfigFiles`, appended by the modules hook).
    modules_config_files: Arc<Mutex<Vec<String>>>,
    /// The `maps.Params` values of the flags: Go's first `applyFlagsOverrides` stores the
    /// flags provider's own maps in the config tree, so every later write into them (the
    /// config file, the config dir, env overrides, merge strategies) changes the flags too,
    /// until the tree's key is replaced (`cleanExternalConfig` drops `internal`). The second
    /// `applyFlagsOverrides` then sets the (changed) flag maps again.
    flag_aliases: std::cell::RefCell<BTreeMap<String, FlagAlias>>,
}

/// See `ConfigLoader::flag_aliases`.
struct FlagAlias {
    value: Value,
    live: bool,
}

/// Go: `allconfig.LoadConfig(d)`.
// Go: config/allconfig/load.go:LoadConfig
pub fn load_config(mut d: ConfigSourceDescriptor) -> Result<Configs> {
    if d.environ.is_empty() {
        d.environ = std::env::vars().map(|(k, v)| format!("{k}={v}")).collect();
    }

    let logger = d.logger.clone().unwrap_or_else(Logger::new_default);
    let fs = d.fs.clone().unwrap_or_else(nh_hugofs::afero::new_os_fs);
    let getenv: Getenv = d
        .getenv
        .clone()
        .unwrap_or_else(|| Arc::new(|k: &str| std::env::var(k).unwrap_or_default()));

    if !d.working_dir.is_empty()
        && let Some(flags) = &d.flags
        && !flags.is_set("workingDir")
    {
        flags.set("workingDir", Value::string(d.working_dir.as_str()));
    }

    let l = ConfigLoader {
        cfg: Arc::new(DefaultConfigProvider::new()),
        base_config: BaseConfig::default(),
        d: &d,
        fs: fs.clone(),
        getenv,
        modules_config_files: Arc::new(Mutex::new(Vec::new())),
        flag_aliases: std::cell::RefCell::new(BTreeMap::new()),
    };
    let mut l = l;
    // Go deletes the merge strategies of the config tree in a deferred call (also on errors).
    // The port's provider has no delete operation; the decoded values that share maps with
    // the tree get their `_merge` keys removed in `delete_merge_strategies`.
    let mut res = l
        .load_config_main()
        .map_err(|err| wrap_err("failed to load config", err))?;

    let mut configs = from_load_config_result(&fs, &logger, &mut res)
        .map_err(|err| wrap_err("failed to create config from result", err))?;

    let (module_config, modules_client) = l
        .load_modules(&configs, d.ignore_module_does_not_exist, &logger)
        .map_err(|err| wrap_err("failed to load modules", err))?;

    let modules_config_files = l
        .modules_config_files
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if !modules_config_files.is_empty() {
        // Config merged in from modules.
        // Re-read the config.
        configs = from_load_config_result(&fs, &logger, &mut res)
            .map_err(|err| wrap_err("failed to create config from modules config", err))?;
        if let Some(err) = configs.transient_err() {
            return Err(Error::new(format!(
                "failed to create config from modules config: {err}"
            )));
        }
        configs
            .loading_info
            .config_files
            .extend(modules_config_files.iter().cloned());
    } else if let Some(err) = configs.transient_err() {
        return Err(Error::new(format!("failed to create config: {err}")));
    }

    configs.modules = module_config.all_modules;
    configs.modules_client = Some(Arc::new(modules_client));

    delete_merge_strategies(&mut configs);

    let configs = configs
        .init()
        .map_err(|err| wrap_err("failed to init config", err))?;

    nh_common::loggers::set_global_logger(logger);

    Ok(configs)
}

/// Go: `fmt.Errorf("<prefix>: %w", err)`.
pub(crate) fn wrap_err(prefix: &str, err: Error) -> Error {
    Error::with_kind(err.kind(), format!("{prefix}: {err}"))
}

/// Go's deferred `deleteMergeStrategies` deletes the `_merge` keys of every `maps.Params` of the
/// config tree. The decoded values that are the tree's own maps lose them too: the imaging
/// namespace's source structure (`images.DecodeConfig` keeps its input map).
fn delete_merge_strategies(configs: &mut ConfigsBuild) {
    type Ns<S, C> = Arc<nh_config::namespace::ConfigNamespace<S, C>>;
    fn strip_ns<S, C>(
        done: &mut BTreeMap<usize, usize>,
        all: &mut Vec<Box<dyn std::any::Any>>,
        ns: &Ns<S, C>,
    ) -> Ns<S, C>
    where
        S: Clone + 'static,
        C: Clone + 'static,
    {
        let key = Arc::as_ptr(ns) as usize;
        if let Some(&i) = done.get(&key) {
            return all[i]
                .downcast_ref::<Ns<S, C>>()
                .expect("same type")
                .clone();
        }
        let mut n = (**ns).clone();
        n.source_structure = strip_merge_top(&n.source_structure);
        let n = Arc::new(n);
        all.push(Box::new(n.clone()));
        done.insert(key, all.len() - 1);
        n
    }
    let mut done: BTreeMap<usize, usize> = BTreeMap::new();
    let mut all: Vec<Box<dyn std::any::Any>> = Vec::new();
    let mut fix = |c: &mut Config| {
        if c.imaging_live
            && let Some(ns) = c.imaging.clone()
        {
            c.imaging = Some(strip_ns(&mut done, &mut all, &ns));
        }
        if c.segments_live
            && let Some(ns) = c.segments.clone()
        {
            c.segments = Some(strip_ns(&mut done, &mut all, &ns));
        }
    };
    fix(&mut configs.base);
    for slot in configs.language_config_map.values_mut() {
        if let crate::allconfig::LangSlot::Own(c) = slot {
            fix(c);
        }
    }
}

/// `strip_merge` of a source structure that is the tree's `maps.Params` (typed
/// `map[string]interface {}` by GetStringMap, like in Go).
fn strip_merge_top(v: &Value) -> Value {
    match v {
        Value::Map(m) => {
            let mut p = (**m).clone();
            let ty = p.ty.clone();
            p.ty = MapType::Params;
            match strip_merge(&Value::map(p)) {
                Value::Map(s) => {
                    let mut s = (*s).clone();
                    s.ty = ty;
                    Value::map(s)
                }
                v => v,
            }
        }
        v => v.clone(),
    }
}

/// Removes the `_merge` keys of every `maps.Params` in v (Go `WalkParams` +
/// `DeleteMergeStrategy`).
pub fn strip_merge(v: &Value) -> Value {
    match v {
        Value::Map(m) => {
            let mut out = Map::new(m.ty.clone());
            for (k, x) in m.entries.iter() {
                if m.ty == MapType::Params
                    && k.as_bytes() == nh_common::maps::params::MERGE_STRATEGY_KEY.as_bytes()
                {
                    continue;
                }
                let x = match x {
                    Value::Map(xm) if xm.ty == MapType::Params => strip_merge(x),
                    _ => x.clone(),
                };
                out.insert(k.clone(), x);
            }
            Value::map(out)
        }
        _ => v.clone(),
    }
}

impl ConfigLoader<'_> {
    /// Handle some legacy values.
    // Go: config/allconfig/load.go:applyConfigAliases
    fn apply_config_aliases(&self) -> Result<()> {
        let aliases = [
            ("indexes", "taxonomies"),
            ("logI18nWarnings", "printI18nWarnings"),
            ("logPathWarnings", "printPathWarnings"),
            ("ignoreErrors", "ignoreLogs"),
        ];

        for (key, value) in aliases {
            if self.cfg.is_set(key) {
                let vv = self.cfg.get(key);
                self.cfg.set(value, vv);
            }
        }

        Ok(())
    }

    // Go: config/allconfig/load.go:applyDefaultConfig
    fn apply_default_config(&self) -> Result<()> {
        let mut default_settings = Map::new(MapType::Params);
        // These dirs are used early/before we build the config struct.
        default_settings.insert("themesDir", Value::string("themes"));
        default_settings.insert("configDir", Value::string("config"));

        self.cfg.set_defaults(&default_settings);

        Ok(())
    }

    // Go: config/allconfig/load.go:normalizeCfg
    fn normalize_cfg(&self, cfg: &dyn Provider) -> Result<()> {
        if let Value::Bool(true) = cfg.get("minifyOutput") {
            cfg.set("minify.minifyOutput", Value::Bool(true));
        } else if let Value::Bool(true) = cfg.get("minify") {
            let mut m = Map::new(MapType::Params);
            m.insert("minifyOutput", Value::Bool(true));
            cfg.set("minify", Value::map(m));
        }

        Ok(())
    }

    // Go: config/allconfig/load.go:cleanExternalConfig
    fn clean_external_config(&self, cfg: &dyn Provider) -> Result<()> {
        if cfg.is_set("internal") {
            cfg.set("internal", Value::Invalid);
        }
        Ok(())
    }

    // Go: config/allconfig/load.go:applyFlagsOverrides
    fn apply_flags_overrides(&self, cfg: &dyn Provider) -> Result<()> {
        let aliases = self.flag_aliases.borrow();
        for k in cfg.keys() {
            let v = match aliases.get(&k) {
                Some(a) => a.value.clone(),
                None => cfg.get(&k),
            };
            self.cfg.set(&k, v);
        }
        Ok(())
    }

    /// Records the flag maps now shared with the config tree (see `flag_aliases`).
    fn init_flag_aliases(&self, flags: &dyn Provider) {
        let mut aliases = self.flag_aliases.borrow_mut();
        for k in flags.keys() {
            if let Value::Map(m) = flags.get(&k)
                && m.ty == MapType::Params
            {
                aliases.insert(
                    k.clone(),
                    FlagAlias {
                        value: self.cfg.get(&k),
                        live: true,
                    },
                );
            }
        }
    }

    /// Replays Go's shared maps: the flag maps follow the tree's maps while they are the same.
    fn sync_flag_aliases(&self) {
        let mut aliases = self.flag_aliases.borrow_mut();
        for (k, a) in aliases.iter_mut() {
            if !a.live {
                continue;
            }
            match self.cfg.get(k) {
                Value::Map(m) if m.ty == MapType::Params => a.value = Value::Map(m),
                _ => a.live = false,
            }
        }
    }

    // Go: config/allconfig/load.go:applyOsEnvOverrides
    fn apply_os_env_overrides(&self, environ: &[String]) -> Result<()> {
        if environ.is_empty() {
            return Ok(());
        }

        const DELIM: &str = "__env__delim";

        // Extract all that start with the HUGO prefix.
        // The delimiter is the following rune, usually "_".
        const HUGO_ENV_PREFIX: &str = "HUGO";
        let mut hugo_env: Vec<(String, String)> = Vec::new();
        for v in environ {
            let (key, val) = nh_config::env::split_env_var(v);
            if let Some(delimiter_and_key) = key.strip_prefix(HUGO_ENV_PREFIX) {
                if delimiter_and_key.len() < 2 {
                    continue;
                }
                // Allow delimiters to be case sensitive.
                // It turns out there isn't that many allowed special
                // chars in environment variables when used in Bash and similar,
                // so variables on the form HUGOxPARAMSxFOO=bar is one option.
                let b = delimiter_and_key.as_bytes();
                let delim = &b[..1];
                let rest = &b[1..];
                let key = go_unicode::strings::replace_all(rest, delim, DELIM.as_bytes());
                let key = go_unicode::strings::to_lower(&key).into_owned();
                hugo_env.push((String::from_utf8_lossy(&key).into_owned(), val));
            }
        }

        for (env_key, env_value) in &hugo_env {
            let segments: Vec<&str> = env_key.split(DELIM).collect();
            let (existing, nested_key, owner) = nh_common::maps::params::get_nested_param_fn(
                env_key.as_bytes(),
                DELIM.as_bytes(),
                |k| self.cfg.get(&String::from_utf8_lossy(k)),
            )?;
            let nested_key = nested_key.to_str_lossy().into_owned();
            // The owner map lives in the config tree in Go: write through its path.
            let owner_path = |nested: &str| -> String {
                let mut parts: Vec<String> = segments[..segments.len() - 1]
                    .iter()
                    .map(|s| go_unicode::strings::to_lower_str(s).into_owned())
                    .collect();
                parts.push(nested.to_string());
                parts.join(".")
            };

            if !existing.is_invalid()
                && let Ok(val) = nh_parser::metadecoders::decoder::Decoder::default()
                    .unmarshal_string_to(env_value, &existing)
            {
                let val = self.env_val_to_val(env_key, val);
                if owner.is_some() {
                    self.cfg.set(&owner_path(&nested_key), val);
                } else {
                    self.cfg.set(env_key, val);
                }
                continue;
            }

            if owner.is_some() && !nested_key.is_empty() {
                self.cfg
                    .set(&owner_path(&nested_key), Value::string(env_value.as_str()));
            } else {
                let mut val = Value::Invalid;
                let key = env_key.replace(DELIM, ".");
                if crate::alldecoders::all_decoder_setups()
                    .iter()
                    .any(|d| d.key == key)
                {
                    // A map.
                    if let Ok(v) = nh_parser::metadecoders::decoder::Decoder::default()
                        .unmarshal_string_to(env_value, &Value::map(Map::new(MapType::StringAny)))
                    {
                        val = v;
                    }
                }

                if val.is_invalid() {
                    // A string.
                    val = self.env_string_to_val(&key, env_value);
                }
                self.cfg.set(&key, val);
            }
        }

        Ok(())
    }

    // Go: config/allconfig/load.go:envValToVal
    fn env_val_to_val(&self, k: &str, v: Value) -> Value {
        match &v {
            Value::String(s) => self.env_string_to_val(k, &s.to_str_lossy()),
            _ => v,
        }
    }

    // Go: config/allconfig/load.go:envStringToVal
    fn env_string_to_val(&self, k: &str, v: &str) -> Value {
        match k {
            "disablekinds" | "disablelanguages" => {
                if v.contains(',') {
                    Value::string_list(v.split(','))
                } else {
                    Value::string_list(
                        go_unicode::strings::fields(v.as_bytes())
                            .iter()
                            .map(|f| String::from_utf8_lossy(f).into_owned()),
                    )
                }
            }
            _ => Value::string(v),
        }
    }

    // Go: config/allconfig/load.go:loadConfigMain
    fn load_config_main(&mut self) -> Result<LoadConfigResult> {
        let d = self.d;
        let mut config_files: Vec<String> = Vec::new();

        if let Some(flags) = &d.flags {
            self.normalize_cfg(flags.as_ref())?;
        }

        if let Some(flags) = &d.flags {
            self.apply_flags_overrides(flags.as_ref())?;
            self.init_flag_aliases(flags.as_ref());
            let working_dir = go_path::filepath::clean(&self.cfg.get_string("workingDir"));

            self.base_config = BaseConfig {
                themes_dir: nh_common::paths::path::abs_pathify(
                    &working_dir,
                    &self.cfg.get_string("themesDir"),
                ),
                working_dir,
                ..Default::default()
            };
        }

        match d.config_filenames() {
            Some(names) => {
                for name in names {
                    match self.load_config(&name) {
                        Ok(filename) => config_files.push(filename),
                        Err((_, None)) => {}
                        Err((filename, Some(err))) => {
                            return Err(self.wrap_file_error(err, &filename));
                        }
                    }
                }
            }
            None => {
                for name in nh_config::config_loader::DEFAULT_CONFIG_NAMES {
                    match self.load_config(name) {
                        Ok(filename) => {
                            config_files.push(filename);
                            break;
                        }
                        Err((_, None)) => {}
                        Err((filename, Some(err))) => {
                            return Err(self.wrap_file_error(err, &filename));
                        }
                    }
                }
            }
        }

        if !d.config_dir.is_empty() {
            let abs_config_dir =
                nh_common::paths::path::abs_pathify(&self.base_config.working_dir, &d.config_dir);
            match nh_config::config_loader::load_config_from_dir(
                std::path::Path::new(&abs_config_dir),
                &d.environment,
            ) {
                Ok((dcfg, dirnames)) => {
                    if !dirnames.is_empty() {
                        let dcfg = dcfg.expect("dirnames imply a config");
                        self.normalize_cfg(dcfg.as_ref())?;
                        self.clean_external_config(dcfg.as_ref())?;
                        self.cfg.set("", dcfg.get(""));
                        self.sync_flag_aliases();
                        config_files.extend(dirnames);
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }

        let cfg: Arc<dyn Provider> = self.cfg.clone();

        self.apply_default_config()?;

        // Some settings are used before we're done collecting all settings,
        // so apply OS environment both before and after.
        self.apply_os_env_overrides(&d.environ)?;
        self.sync_flag_aliases();

        let working_dir = go_path::filepath::clean(&self.cfg.get_string("workingDir"));

        self.base_config = BaseConfig {
            cache_dir: self.cfg.get_string("cacheDir"),
            themes_dir: nh_common::paths::path::abs_pathify(
                &working_dir,
                &self.cfg.get_string("themesDir"),
            ),
            working_dir,
            ..Default::default()
        };

        let getenv = self.getenv.clone();
        let env = nh_helpers::path::CacheDirEnv {
            getenv: &move |k: &str| getenv(k),
            is_test: false,
        };
        self.base_config.cache_dir = nh_helpers::path::get_cache_dir_env(
            self.fs.as_ref(),
            &self.base_config.cache_dir,
            &env,
        )?;

        let base_config = self.base_config.clone();

        self.cfg.set_default_merge_strategy();
        self.sync_flag_aliases();

        config_files.extend(
            self.modules_config_files
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .cloned(),
        );

        if let Some(flags) = &d.flags {
            self.apply_flags_overrides(flags.as_ref())?;
        }

        self.apply_os_env_overrides(&d.environ)?;

        self.apply_config_aliases()?;

        Ok(LoadConfigResult {
            cfg,
            config_files,
            base_config,
        })
    }

    // Go: config/allconfig/load.go:loadModules
    fn load_modules(
        &self,
        configs: &ConfigsBuild,
        ignore_module_does_not_exist: bool,
        logger: &Logger,
    ) -> Result<(ModulesConfig, Client)> {
        let bcfg = &configs.loading_info.base_config;
        let conf = &configs.base;
        let working_dir = bcfg.working_dir.clone();
        let themes_dir = bcfg.themes_dir.clone();
        let publish_dir = bcfg.publish_dir.clone();

        let cfg = configs.loading_info.cfg.clone();

        let mut ignore_vendor = None;
        if !conf.root.ignore_vendor_paths.is_empty() {
            ignore_vendor = nh_common::glob::glob::get_glob(
                &nh_common::glob::glob::normalize_path(&conf.root.ignore_vendor_paths),
            )
            .ok();
        }

        let files = self.modules_config_files.clone();
        let hook = move |mods: &nh_hugofs::modules::module::Modules| -> Result<()> {
            for tc in mods {
                if !tc.config_filenames().is_empty() {
                    if tc.watch() {
                        files
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .extend(tc.config_filenames().iter().cloned());
                    }

                    // Merge in the theme config using the configured
                    // merge strategy.
                    if let Some(tcfg) = tc.cfg() {
                        cfg.merge("", tcfg.get(""));
                    }
                }
            }
            Ok(())
        };

        let modules_client = Client::new(ClientConfig {
            fs: Some(self.fs.clone()),
            logger: Some(logger.clone()),
            hook_before_finalize: Some(Arc::new(hook)),
            working_dir,
            themes_dir,
            publish_dir,
            environment: self.d.environment.clone(),
            cache_dir: nh_helpers::cache::filecache::filecache_config::cache_dir_modules(
                &conf.caches,
            ),
            module_config: conf.module.clone(),
            ignore_vendor,
            ignore_module_does_not_exist,
        });

        let module_config = modules_client.collect()?;

        // We want to watch these for changes and trigger rebuild on version
        // changes etc.
        if !module_config.go_modules_filename.is_empty() {
            self.modules_config_files
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(module_config.go_modules_filename.clone());
        }

        if !module_config.go_workspace_filename.is_empty() {
            self.modules_config_files
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(module_config.go_workspace_filename.clone());
        }

        Ok((module_config, modules_client))
    }

    /// Go returns `(filename, err)`; `Err((filename, None))` is `ErrNoConfigFile`.
    // Go: config/allconfig/load.go:loadConfig
    fn load_config(
        &self,
        config_name: &str,
    ) -> std::result::Result<String, (String, Option<Error>)> {
        let base_dir = &self.base_config.working_dir;
        let base_filename = if go_path::filepath::is_abs(config_name) {
            config_name.to_string()
        } else {
            go_path::filepath::join(&[base_dir.as_str(), config_name])
        };

        let mut filename = String::new();
        if !nh_common::paths::path::ext_no_delimiter(config_name).is_empty() {
            let exists =
                nh_hugofs::afero::exists(self.fs.as_ref(), &base_filename).unwrap_or(false);
            if exists {
                filename = base_filename;
            }
        } else {
            for ext in nh_config::config_loader::VALID_CONFIG_FILE_EXTENSIONS {
                let filename_to_check = format!("{base_filename}.{ext}");
                let exists =
                    nh_hugofs::afero::exists(self.fs.as_ref(), &filename_to_check).unwrap_or(false);
                if exists {
                    filename = filename_to_check;
                    break;
                }
            }
        }

        if filename.is_empty() {
            return Err((String::new(), None));
        }

        let m = nh_config::config_loader::from_file_to_map(std::path::Path::new(&filename))
            .map_err(|err| (filename.clone(), Some(err)))?;

        // Set overwrites keys of the same name, recursively.
        self.cfg.set("", Value::map(m));
        self.sync_flag_aliases();

        self.normalize_cfg(self.cfg.as_ref())
            .map_err(|err| (filename.clone(), Some(err)))?;
        self.sync_flag_aliases();

        self.clean_external_config(self.cfg.as_ref())
            .map_err(|err| (filename.clone(), Some(err)))?;
        self.sync_flag_aliases();

        Ok(filename)
    }

    /// Go: `configLoader.wrapFileError(err, filename)`.
    // Go: config/allconfig/load.go:wrapFileError
    fn wrap_file_error(&self, err: Error, filename: &str) -> Error {
        if let Some(pos) = err.pos() {
            let mut pos = pos.clone();
            pos.filename = filename.to_string();
            return err.at(pos);
        }
        nh_common::herrors::new_file_error_from_name(err, filename)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/allconfig/load.go (544 lines; 12/15 funcs executed)
//   types: ConfigSourceDescriptor, configLoader
// OK L43-97: LoadConfig(d ConfigSourceDescriptor) (*Configs, error)
// OK L124-129: (d ConfigSourceDescriptor) configFilenames() []string
// OK L142-158: (l configLoader) applyConfigAliases() error
// OK L160-170: (l configLoader) applyDefaultConfig() error
// OK L172-180: (l configLoader) normalizeCfg(cfg config.Provider) error
// OK L182-187: (l configLoader) cleanExternalConfig(cfg config.Provider) error
// OK L189-194: (l configLoader) applyFlagsOverrides(cfg config.Provider) error
// OK L196-270: (l configLoader) applyOsEnvOverrides(environ []string) error
// OK L272-279: (l *configLoader) envValToVal(k string, v any) any
// OK L281-292: (l *configLoader) envStringToVal(k, v string) any
// OK L294-414: (l *configLoader) loadConfigMain(d ConfigSourceDescriptor) (config.LoadConfigResult, modules.ModulesConfig, error)
// OK L416-477: (l *configLoader) loadModules(configs *Configs, ignoreModuleDoesNotExist bool) (modules.ModulesConfig, *modules.Client, error)
// OK L479-526: (l configLoader) loadConfig(configName string) (string, error)
// OK L528-533: (l configLoader) deleteMergeStrategies()
// OK L535-544: (l configLoader) wrapFileError(err error, filename string) error
// ---------------------------------------------------------------------------
