//! Port of `modules/collect.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;

use nh_common::files;
use nh_common::herrors::ErrorKind;
use nh_common::{Error, Result};
use nh_config::config_provider::Provider;

use super::client::{Client, VENDORD, split_path_version};
use super::config::{Config, FILE_SEPARATOR, Import, Mount, decode_config_with_replacements};
use super::module::{Module, Modules};

/// Go: `modules.ErrNotExist`.
pub const ERR_NOT_EXIST: &str = "module does not exist";

/// Go: `modules.vendorModulesFilename`.
const VENDOR_MODULES_FILENAME: &str = "modules.txt";

/// Go: `modules.ModulesConfig`.
#[derive(Clone, Default)]
pub struct ModulesConfig {
    /// All active modules.
    pub all_modules: Modules,
    /// Set if this is a Go modules enabled project.
    pub go_modules_filename: String,
    /// Set if a Go workspace file is configured.
    pub go_workspace_filename: String,
}

impl ModulesConfig {
    // Go: modules/collect.go:HasConfigFile
    pub fn has_config_file(&self) -> bool {
        self.all_modules
            .iter()
            .any(|m| !m.config_filenames().is_empty())
    }
}

/// Go: `*moduleAdapter` while collecting (modules are addressed by their index in
/// `collector::adapters`; the owner is an index, Go holds a pointer).
#[derive(Clone)]
struct ModuleAdapter {
    path: String,
    dir: String,
    version: String,
    vendor: bool,
    project_mod: bool,
    owner: Option<usize>,
    mounts: Vec<Mount>,
    config_filenames: Vec<String>,
    cfg: Option<Arc<dyn Provider>>,
    config: Config,
}

impl ModuleAdapter {
    // Go: modules/module.go:Dir (never a Go module here)
    fn dir(&self) -> &str {
        &self.dir
    }

    // Go: modules/module.go:Path
    fn path(&self) -> &str {
        &self.path
    }
}

/// Go: `vendoredModule`.
#[derive(Clone)]
struct VendoredModule {
    owner: usize,
    dir: String,
    version: String,
}

/// Go: `collector` + `collected`.
struct Collector<'a> {
    client: &'a Client,
    /// Store away any non-fatal error and return at the end.
    err: Option<Error>,
    /// Set to disable any Tidy operation in the end.
    skip_tidy: bool,
    /// Pick the first and prevent circular loops.
    seen: BTreeMap<String, bool>,
    /// Maps module path to a _vendor dir. These values are fetched from _vendor/modules.txt,
    /// and the first (top-most) will win.
    vendored: BTreeMap<String, VendoredModule>,
    /// All module adapters created (the project first).
    adapters: Vec<ModuleAdapter>,
    /// Ordered list of collected modules (indexes into `adapters`), excluding the project.
    modules: Vec<usize>,
}

impl Client {
    /// Go: `Client.Collect()` — only the project module (`createProjectModule` + `applyMounts`:
    /// `normalizeMounts` (Clean, silently skip missing sources) and `mountCommonJSConfig`
    /// (auto-mount package.json / postcss.config.js etc. to `assets/_jsconfig/<name>`)), themes
    /// in `themesDir` and vendored modules.
    // Go: modules/collect.go:Collect
    pub fn collect(&self) -> Result<ModulesConfig> {
        let (mut mc, adapters, err) = self.collect_inner(true);
        if let Some(err) = err {
            return Err(err);
        }

        mc.set_active_mods(&self.logger)?;

        if let Some(hook) = &self.ccfg.hook_before_finalize {
            hook(&mc.all_modules)?;
        }

        mc.finalize(adapters)?;

        Ok(mc)
    }

    /// `collect` for `Graph` (no hook, no finalize).
    pub(crate) fn collect_modules(&self) -> Result<ModulesConfig> {
        let (mc, _, err) = self.collect_inner(true);
        match err {
            Some(err) => Err(err),
            None => Ok(mc),
        }
    }

    // Go: modules/collect.go:collect
    fn collect_inner(&self, _tidy: bool) -> (ModulesConfig, Vec<ModuleAdapter>, Option<Error>) {
        let mut c = Collector {
            client: self,
            err: None,
            skip_tidy: false,
            seen: BTreeMap::new(),
            vendored: BTreeMap::new(),
            adapters: Vec::new(),
            modules: Vec::new(),
        };

        c.collect();
        if let Some(err) = c.err.take() {
            return (ModulesConfig::default(), Vec::new(), Some(err));
        }
        let _ = c.skip_tidy;

        let mut workspace_filename = String::new();
        if self.ccfg.module_config.workspace != super::config::WORKSPACE_DISABLED {
            workspace_filename = self.ccfg.module_config.workspace.clone();
        }

        // Add the project mod on top: the project (collector index 0), then the collected
        // modules. Owner indexes are remapped to positions in that order (an owner is always
        // collected before the modules it owns).
        let mut order = vec![0];
        order.extend(c.modules.iter().copied());
        let adapters: Vec<ModuleAdapter> = order
            .iter()
            .map(|&i| {
                let mut a = c.adapters[i].clone();
                a.owner = a.owner.map(|oi| {
                    order
                        .iter()
                        .position(|&x| x == oi)
                        .expect("owner is a collected module")
                });
                a
            })
            .collect();
        let all_modules = build_modules(&adapters);

        (
            ModulesConfig {
                all_modules,
                go_modules_filename: self.go_modules_filename.clone(),
                go_workspace_filename: workspace_filename,
            },
            adapters,
            None,
        )
    }
}

/// Converts the adapters (in module order, owners as positions in that order) to `Module`s.
fn build_modules(adapters: &[ModuleAdapter]) -> Modules {
    let mut out: Modules = Vec::with_capacity(adapters.len());
    for a in adapters {
        let owner = a.owner.map(|pos| out[pos].clone());
        out.push(Arc::new(Module {
            path: a.path.clone(),
            dir: a.dir.clone(),
            config: a.config.clone(),
            config_filenames: a.config_filenames.clone(),
            mounts: a.mounts.clone(),
            watch: module_watch(a),
            owner,
            is_go_mod: false,
            vendor: a.vendor,
            version: a.version.clone(),
            cfg: a.cfg.clone(),
        }));
    }
    out
}

/// Go: `moduleAdapter.Watch()` without Go modules: the project and modules inside /themes (or
/// _vendor) are watch candidates.
// Go: modules/module.go:Watch
fn module_watch(_a: &ModuleAdapter) -> bool {
    true
}

impl ModulesConfig {
    // Go: modules/collect.go:setActiveMods
    fn set_active_mods(&self, logger: &nh_common::loggers::Logger) -> Result<()> {
        for m in &self.all_modules {
            if !m.config().hugo_version_is_valid() {
                logger.warnf(format!(
                    "Module {} is not compatible with this Hugo version: {}; run \"hugo mod graph\" for more information.",
                    go_strconv::quote(m.path().as_bytes()),
                    m.config().hugo_version_string()
                ));
            }
        }
        Ok(())
    }

    // Go: modules/collect.go:finalize
    fn finalize(&mut self, mut adapters: Vec<ModuleAdapter>) -> Result<()> {
        for a in adapters.iter_mut() {
            a.mounts = filter_unwanted_mounts(std::mem::take(&mut a.mounts));
        }
        self.all_modules = build_modules(&adapters);
        Ok(())
    }
}

/// Go: `filterUnwantedMounts(mounts)`: removes duplicates.
// Go: modules/collect.go:filterUnwantedMounts
fn filter_unwanted_mounts(mounts: Vec<Mount>) -> Vec<Mount> {
    // Remove duplicates
    let mut seen: BTreeMap<String, bool> = BTreeMap::new();
    let mut tmp = Vec::with_capacity(mounts.len());
    for m in mounts {
        let key = m.key();
        if !seen.get(&key).copied().unwrap_or(false) {
            tmp.push(m);
        }
        seen.insert(key, true);
    }
    tmp
}

impl Collector<'_> {
    fn fs(&self) -> &dyn crate::afero::Fs {
        self.client.fs.as_ref()
    }

    // Go: modules/collect.go:initModules
    fn init_modules(&mut self) -> Result<()> {
        // If both these are true, we don't even need Go installed to build.
        if self.client.ccfg.ignore_vendor.is_none()
            && self.is_vendored(&self.client.ccfg.working_dir)
        {
            return Ok(());
        }

        // We may fail later if we don't find the mods.
        self.load_modules()
    }

    // Go: modules/collect.go:isSeen
    fn is_seen(&mut self, path: &str) -> bool {
        let key = path_key(path);
        if self.seen.get(&key).copied().unwrap_or(false) {
            return true;
        }
        self.seen.insert(key, true);
        false
    }

    // Go: modules/collect.go:getVendoredDir
    fn get_vendored_dir(&self, path: &str) -> Option<VendoredModule> {
        self.vendored.get(path).cloned()
    }

    // Go: modules/collect.go:add
    fn add(&mut self, owner: usize, module_import: &Import) -> Result<Option<usize>> {
        let mut module_dir = String::new();
        let mut version = String::new();
        let mut vendored = false;

        let module_path = module_import.path.clone();
        let mut real_owner = owner;

        if !self.client.ccfg.should_ignore_vendor(&module_path) {
            self.collect_modules_txt(owner)?;

            // Try _vendor first.
            if let Some(vm) = self.get_vendored_dir(&module_path) {
                vendored = true;
                module_dir = vm.dir;
                real_owner = vm.owner;
                version = vm.version;

                if self.adapters[owner].project_mod {
                    // We want to keep the go.mod intact with the versions and all.
                    self.skip_tidy = true;
                }
            }
        }

        if module_dir.is_empty() {
            // Go: c.gomods.GetByPath(modulePath) is always nil without Go modules.
            if !self.client.go_modules_filename.is_empty()
                && super::client::is_probably_module(&module_path)
            {
                // Try to "go get" it and reload the module configuration.
                // See https://golang.org/ref/mod#version-queries
                // This will select the latest release-version (not beta etc.).
                let version_query = "upgrade";
                self.client
                    .go_get(&format!("{module_path}@{version_query}"))?;
            }

            // Fall back to project/themes/<mymodule>
            if module_dir.is_empty() {
                match self.client.create_theme_dirname(
                    &module_path,
                    self.adapters[owner].project_mod || module_import.path_project_replaced,
                ) {
                    Ok(d) => module_dir = d,
                    Err(err) => {
                        self.err = Some(err);
                        return Ok(None);
                    }
                }
                if !crate::afero::exists(self.fs(), &module_dir).unwrap_or(false) {
                    self.err = self.wrap_module_not_found(format!(
                        "module {} not found in {}; either add it as a Hugo Module or store it in {}.",
                        go_strconv::quote(module_path.as_bytes()),
                        go_strconv::quote(module_dir.as_bytes()),
                        go_strconv::quote(self.client.ccfg.themes_dir.as_bytes())
                    ));
                    return Ok(None);
                }
            }
        }

        if !crate::afero::exists(self.fs(), &module_dir).unwrap_or(false) {
            self.err = self.wrap_module_not_found(format!(
                "{} not found",
                go_strconv::quote(module_dir.as_bytes())
            ));
            return Ok(None);
        }

        if !module_dir.ends_with(FILE_SEPARATOR) {
            module_dir.push_str(FILE_SEPARATOR);
        }

        let mut ma = ModuleAdapter {
            path: module_path,
            dir: module_dir,
            version,
            vendor: vendored,
            project_mod: false,
            // This may be the owner of the _vendor dir
            owner: Some(real_owner),
            mounts: Vec::new(),
            config_filenames: Vec::new(),
            cfg: None,
            config: Config::default(),
        };

        if !module_import.ignore_config {
            self.apply_theme_config(&mut ma)?;
        }

        self.apply_mounts(module_import, &mut ma)?;

        self.adapters.push(ma);
        let idx = self.adapters.len() - 1;
        self.modules.push(idx);
        Ok(Some(idx))
    }

    // Go: modules/collect.go:addAndRecurse
    fn add_and_recurse(&mut self, owner: usize) -> Result<()> {
        let module_config = self.adapters[owner].config.clone();
        if self.adapters[owner].project_mod {
            let mut ma = self.adapters[owner].clone();
            self.apply_mounts(&Import::default(), &mut ma)
                .map_err(|err| wrapf("failed to apply mounts for project", err))?;
            self.adapters[owner] = ma;
        }

        for module_import in &module_config.imports {
            if module_import.disable {
                continue;
            }
            if !self.is_seen(&module_import.path) {
                let tc = self.add(owner, module_import)?;
                let Some(tc) = tc else {
                    continue;
                };
                if module_import.ignore_imports {
                    continue;
                }
                self.add_and_recurse(tc)?;
            }
        }
        Ok(())
    }

    // Go: modules/collect.go:applyMounts
    fn apply_mounts(&mut self, module_import: &Import, m: &mut ModuleAdapter) -> Result<()> {
        if module_import.no_mounts {
            m.mounts = Vec::new();
            return Ok(());
        }

        let mut mounts = module_import.mounts.clone();

        if mounts.is_empty() {
            // Mounts not defined by the import.
            mounts = m.config.mounts.clone();
        }

        if !m.project_mod && mounts.is_empty() {
            // Create default mount points for every component folder that
            // exists in the module.
            for component_folder in files::COMPONENT_FOLDERS {
                let source_dir = go_path::filepath::join(&[m.dir(), component_folder]);
                if self.fs().stat(&source_dir).is_ok() {
                    mounts.push(Mount {
                        source: component_folder.to_string(),
                        target: component_folder.to_string(),
                        ..Default::default()
                    });
                }
            }
        }

        let mounts = self.normalize_mounts(m, mounts)?;

        let mounts = self.mount_common_js_config(m, mounts)?;

        m.mounts = mounts;
        Ok(())
    }

    // Go: modules/collect.go:applyThemeConfig
    fn apply_theme_config(&mut self, tc: &mut ModuleAdapter) -> Result<()> {
        let mut config_filename = String::new();
        let mut has_config_file = false;
        let mut theme_cfg: Option<go_value::Map> = None;

        'lookup: for config_base_name in nh_config::config_loader::DEFAULT_CONFIG_NAMES {
            for config_formats in nh_config::config_loader::VALID_CONFIG_FILE_EXTENSIONS {
                config_filename = go_path::filepath::join(&[
                    tc.dir(),
                    &format!("{config_base_name}.{config_formats}"),
                ]);
                has_config_file =
                    crate::afero::exists(self.fs(), &config_filename).unwrap_or(false);
                if has_config_file {
                    break 'lookup;
                }
            }
        }

        // The old theme information file.
        let theme_toml = go_path::filepath::join(&[tc.dir(), "theme.toml"]);

        let has_theme_toml = crate::afero::exists(self.fs(), &theme_toml).unwrap_or(false);
        if has_theme_toml {
            let data = crate::afero::read_file(self.fs(), &theme_toml)?;
            match nh_parser::metadecoders::decoder::Decoder::default()
                .unmarshal_to_map(&data, nh_parser::metadecoders::format::Format::Toml)
            {
                Err(err) => {
                    self.client.logger.warnf(format!(
                        "Failed to read module config for {} in {}: {}",
                        go_strconv::quote(tc.path().as_bytes()),
                        go_strconv::quote(theme_toml.as_bytes()),
                        err
                    ));
                }
                Ok(mut m) => {
                    nh_common::maps::params::prepare_params(&mut m);
                    theme_cfg = Some(m);
                }
            }
        }

        if has_config_file {
            if !config_filename.is_empty() {
                let cfg =
                    nh_config::config_loader::from_file(std::path::Path::new(&config_filename))?;
                tc.cfg = Some(Arc::new(cfg));
            }

            tc.config_filenames.push(config_filename.clone());
        }

        // Also check for a config dir, which we overlay on top of the file configuration.
        let config_dir = go_path::filepath::join(&[tc.dir(), "config"]);
        let (dcfg, dirnames) = nh_config::config_loader::load_config_from_dir(
            std::path::Path::new(&config_dir),
            &self.client.ccfg.environment,
        )?;

        if !dirnames.is_empty() {
            tc.config_filenames.extend(dirnames);

            let dcfg: Arc<dyn Provider> = Arc::from(dcfg.expect("dirnames imply a config"));
            if has_config_file {
                // Set will overwrite existing keys.
                tc.cfg
                    .as_ref()
                    .expect("config file loaded")
                    .set("", dcfg.get(""));
            } else {
                tc.cfg = Some(dcfg);
            }
        }

        let mut config = decode_config_with_replacements(
            tc.cfg.as_deref(),
            self.client.module_config.replacements_map.as_ref(),
        )?;

        const OLD_VERSION_KEY: &str = "min_version";

        if has_theme_toml {
            // Go: a theme.toml that failed to parse leaves themeCfg nil.
            let theme_cfg =
                theme_cfg.unwrap_or_else(|| go_value::Map::new(go_value::MapType::StringAny));
            // Merge old with new
            if let Some(min_version) = theme_cfg.get(OLD_VERSION_KEY.as_bytes())
                && config.hugo_version_min.is_empty()
            {
                config.hugo_version_min = nh_common::cast::caste::to_string(min_version)
                    .to_str_lossy()
                    .into_owned();
            }

            let params = config
                .params
                .get_or_insert_with(|| go_value::Map::new(go_value::MapType::StringAny));

            for (k, v) in theme_cfg.entries.iter() {
                if k.as_bytes() == OLD_VERSION_KEY.as_bytes() {
                    continue;
                }
                params.insert(k.clone(), v.clone());
            }
        }

        tc.config = config;

        Ok(())
    }

    // Go: modules/collect.go:collect
    fn collect(&mut self) {
        if let Err(err) = self.init_modules() {
            self.err = Some(err);
            return;
        }

        let project_mod = create_project_module(
            &self.client.ccfg.working_dir,
            self.client.module_config.clone(),
        );
        self.adapters.push(project_mod);

        if let Err(err) = self.add_and_recurse(0) {
            self.err = Some(err);
        }

        // Go adds the project mod on top (`collect_inner` builds that order).
    }

    // Go: modules/collect.go:isVendored
    fn is_vendored(&self, dir: &str) -> bool {
        self.fs()
            .stat(&go_path::filepath::join(&[
                dir,
                VENDORD,
                VENDOR_MODULES_FILENAME,
            ]))
            .is_ok()
    }

    // Go: modules/collect.go:collectModulesTXT
    fn collect_modules_txt(&mut self, owner: usize) -> Result<()> {
        let vendor_dir = go_path::filepath::join(&[self.adapters[owner].dir(), VENDORD]);
        let filename = go_path::filepath::join(&[vendor_dir.as_str(), VENDOR_MODULES_FILENAME]);

        let mut f = match self.fs().open(&filename) {
            Ok(f) => f,
            Err(err) => {
                if nh_common::herrors::is_not_exist(&err) {
                    return Ok(());
                }
                return Err(err);
            }
        };

        let mut data = Vec::new();
        f.read_to_end(&mut data).map_err(Error::from)?;
        let _ = f.close();

        // bufio.Scanner: lines split at '\n' with a trailing '\r' dropped.
        for line in scan_lines(&data) {
            // # github.com/alecthomas/chroma v0.6.3
            let line = String::from_utf8_lossy(line);
            let line = line.trim_matches(|c| c == '#' || c == ' ');
            let line = go_unicode::strings::trim_space_str(line);
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&[u8]> = go_unicode::strings::fields(line.as_bytes());
            if parts.len() != 2 {
                return Err(Error::new(format!(
                    "invalid modules list: {}",
                    go_strconv::quote(filename.as_bytes())
                )));
            }
            let path = String::from_utf8_lossy(parts[0]).into_owned();

            let mut should_add = self.client.module_config.vendor_closest;

            if !should_add && !self.vendored.contains_key(&path) {
                should_add = true;
            }

            if should_add {
                self.vendored.insert(
                    path.clone(),
                    VendoredModule {
                        owner,
                        dir: go_path::filepath::join(&[vendor_dir.as_str(), path.as_str()]),
                        version: String::from_utf8_lossy(parts[1]).into_owned(),
                    },
                );
            }
        }
        Ok(())
    }

    // Go: modules/collect.go:loadModules
    fn load_modules(&mut self) -> Result<()> {
        // Go: c.gomods = c.listGoMods() (always empty here, or unsupported).
        self.client.list_go_mods()
    }

    // Go: modules/collect.go:mountCommonJSConfig
    fn mount_common_js_config(
        &self,
        owner: &ModuleAdapter,
        mut mounts: Vec<Mount>,
    ) -> Result<Vec<Mount>> {
        for m in &mounts {
            if m.target.starts_with(files::JS_CONFIG_FOLDER_MOUNT_PREFIX) {
                // This follows the convention of the other component types (assets, content, etc.),
                // if one or more is specified by the user, we skip the defaults.
                // These mounts were added to Hugo in 0.75.
                return Ok(mounts);
            }
        }

        // Mount the common JS config files.
        let mut d = self.fs().open(owner.dir()).map_err(|err| {
            Error::new(format!(
                "failed to open dir {}: {}",
                go_strconv::quote(owner.dir().as_bytes()),
                go_strconv::quote(err.to_string().as_bytes())
            ))
        })?;
        let fis = d.read_dir(-1).map_err(|err| {
            Error::new(format!(
                "failed to read dir {}: {}",
                go_strconv::quote(owner.dir().as_bytes()),
                go_strconv::quote(err.to_string().as_bytes())
            ))
        });
        let _ = d.close();
        let fis = fis?;

        for fi in fis {
            let n = fi.name().to_string();

            let mut should =
                n == files::FILENAME_PACKAGE_HUGO_JSON || n == files::FILENAME_PACKAGE_JSON;
            should = should || common_js_configs_match(&n);

            if should {
                mounts.push(Mount {
                    source: n.clone(),
                    target: go_path::filepath::join(&[
                        files::COMPONENT_FOLDER_ASSETS,
                        files::FOLDER_JS_CONFIG,
                        n.as_str(),
                    ]),
                    ..Default::default()
                });
            }
        }

        Ok(mounts)
    }

    // Go: modules/collect.go:normalizeMounts
    fn normalize_mounts(&self, owner: &ModuleAdapter, mounts: Vec<Mount>) -> Result<Vec<Mount>> {
        let mut out: Vec<Mount> = Vec::new();
        let dir = owner.dir();

        for mut mnt in mounts {
            let err_msg = format!(
                "invalid module config for {}",
                go_strconv::quote(owner.path().as_bytes())
            );

            if mnt.source.is_empty() || mnt.target.is_empty() {
                return Err(Error::new(format!(
                    "{err_msg}: both source and target must be set"
                )));
            }

            mnt.source = go_path::filepath::clean(&mnt.source);
            mnt.target = go_path::filepath::clean(&mnt.target);

            let source_dir = if owner.project_mod && go_path::filepath::is_abs(&mnt.source) {
                // Abs paths in the main project is allowed.
                mnt.source.clone()
            } else {
                go_path::filepath::join(&[dir, mnt.source.as_str()])
            };

            // Verify that Source exists
            if self.fs().stat(&source_dir).is_err() {
                if nh_common::paths::path::is_same_file_path(
                    &source_dir,
                    &self.client.ccfg.publish_dir,
                ) {
                    // This is a little exotic, but there are use cases for mounting the public folder.
                    // This will typically also be in .gitingore, so create it.
                    self.fs().mkdir_all(&source_dir, 0o755).map_err(|err| {
                        Error::new(format!(
                            "{err_msg}: {}",
                            go_strconv::quote(err.to_string().as_bytes())
                        ))
                    })?;
                } else if source_dir.ends_with(files::FILENAME_HUGO_STATS_JSON) {
                    // A common pattern for Tailwind 3 is to mount that file to get it on the server watch list.

                    // A common pattern is also to add hugo_stats.json to .gitignore.

                    // Create an empty file.
                    let mut f = self.fs().create(&source_dir).map_err(|err| {
                        Error::new(format!(
                            "{err_msg}: {}",
                            go_strconv::quote(err.to_string().as_bytes())
                        ))
                    })?;
                    let _ = f.close();
                } else {
                    // TODO(bep) commenting out for now, as this will create to much noise.
                    continue;
                }
            }

            // Verify that target points to one of the predefined component dirs
            let target_base = match mnt.target.find('/') {
                Some(i) => &mnt.target[..i],
                None => mnt.target.as_str(),
            };
            if !files::is_component_folder(target_base) {
                return Err(Error::new(format!(
                    "{err_msg}: mount target must be one of: [{}]",
                    files::COMPONENT_FOLDERS.join(" ")
                )));
            }

            out.push(mnt);
        }

        Ok(out)
    }

    // Go: modules/collect.go:wrapModuleNotFound
    fn wrap_module_not_found(&self, msg: String) -> Option<Error> {
        if self.client.ccfg.ignore_module_does_not_exist {
            return None;
        }
        let err = Error::with_kind(ErrorKind::NotExist, format!("{msg}: {ERR_NOT_EXIST}"));
        // With a go.mod, Go explains a missing/old Go binary only after trying to run it, which
        // never happens here (goBinaryStatusOK).
        Some(err)
    }
}

/// Go: `commonJSConfigs = regexp.MustCompile("(babel|postcss|tailwind)\\.config\\.js")`
/// (`MatchString`, unanchored).
fn common_js_configs_match(n: &str) -> bool {
    ["babel.config.js", "postcss.config.js", "tailwind.config.js"]
        .iter()
        .any(|p| n.contains(p))
}

/// `bufio.ScanLines`: lines split at `\n`, a trailing `\r` dropped, a final line without `\n`
/// kept.
fn scan_lines(data: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        let (line, next) = match rest.iter().position(|&b| b == b'\n') {
            Some(i) => (&rest[..i], &rest[i + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        out.push(line);
        rest = next;
    }
    out
}

/// Go: `fmt.Errorf("<prefix>: %w", err)`.
fn wrapf(prefix: &str, err: Error) -> Error {
    Error::with_kind(err.kind(), format!("{prefix}: {err}"))
}

/// Go: `createProjectModule(gomod, workingDir, conf)` — a pseudo module for the main project.
// Go: modules/collect.go:createProjectModule
fn create_project_module(working_dir: &str, conf: Config) -> ModuleAdapter {
    // Without Go modules (gomod == nil) the path is "project".
    ModuleAdapter {
        path: "project".to_string(),
        dir: working_dir.to_string(),
        version: String::new(),
        vendor: false,
        project_mod: true,
        owner: None,
        mounts: Vec::new(),
        config_filenames: Vec::new(),
        cfg: None,
        config: conf,
    }
}

/// Go: `pathKey(p)`.
// Go: modules/collect.go:pathKey
fn path_key(p: &str) -> String {
    let (prefix, _, _) = split_path_version(p);

    go_unicode::strings::to_lower_str(prefix).into_owned()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: modules/collect.go (755 lines; 14/22 funcs executed)
//   types: ModulesConfig, collected, collector, vendoredModule
// OK L51-72: (h *Client) Collect() (ModulesConfig, error)
// OK L74-105: (h *Client) collect(tidy bool) (ModulesConfig, *collector)
// OK L118-125: (m ModulesConfig) HasConfigFile() bool
// OK L127-135: (m *ModulesConfig) setActiveMods(logger loggers.Logger) error
// OK L137-143: (m *ModulesConfig) finalize(logger loggers.Logger) error
// OK L145-156: filterUnwantedMounts(mounts []Mount) []Mount
// OK L187-201: (c *collector) initModules() error
// OK L203-210: (c *collector) isSeen(path string) bool
// OK L212-215: (c *collector) getVendoredDir(path string) (vendoredModule, bool)
// OK L217-332: (c *collector) add(owner *moduleAdapter, moduleImport Import) (*moduleAdapter, error)
// OK L334-360: (c *collector) addAndRecurse(owner *moduleAdapter) error
// OK L362-405: (c *collector) applyMounts(moduleImport Import, mod *moduleAdapter) error
// OK L407-506: (c *collector) applyThemeConfig(tc *moduleAdapter) error
// OK L508-530: (c *collector) collect()
// OK L532-535: (c *collector) isVendored(dir string) bool
// OK L537-586: (c *collector) collectModulesTXT(owner Module) error
// OK L588-595: (c *collector) loadModules() error
// OK L600-637: (c *collector) mountCommonJSConfig(owner *moduleAdapter, mounts []Mount) ([]Mount, error)
// OK L639-702: (c *collector) normalizeMounts(owner *moduleAdapter, mounts []Mount) ([]Mount, error)
// OK L704-723: (c *collector) wrapModuleNotFound(err error) error
// OK L731-745: createProjectModule(gomod *goModule, workingDir string, conf Config) *moduleAdapter
// OK L751-755: pathKey(p string) string
// ---------------------------------------------------------------------------

// Ports of modules/collect_test.go (and client.go's module path checks).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::client::is_probably_module;

    // Go: modules/collect_test.go:TestPathKey
    #[test]
    fn test_path_key() {
        for (input, expect) in [
            ("github.com/foo", "github.com/foo"),
            ("github.com/foo/v2", "github.com/foo"),
            ("github.com/foo/v12", "github.com/foo"),
            ("github.com/foo/v3d", "github.com/foo/v3d"),
            ("MyTheme", "mytheme"),
        ] {
            assert_eq!(path_key(input), expect, "{input}");
        }
    }

    fn mnt(source: &str, target: &str, lang: &str) -> Mount {
        Mount {
            source: source.to_string(),
            target: target.to_string(),
            lang: lang.to_string(),
            ..Default::default()
        }
    }

    // Go: modules/collect_test.go:TestFilterUnwantedMounts
    #[test]
    fn test_filter_unwanted_mounts() {
        let mounts = vec![
            mnt("a", "b", "en"),
            mnt("a", "b", "en"),
            mnt("b", "c", "en"),
        ];
        let filtered = filter_unwanted_mounts(mounts);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered, vec![mnt("a", "b", "en"), mnt("b", "c", "en")]);
    }

    // Hugo's `isProbablyModule` (x/mod `module.CheckPath`): a module path needs a dot in its
    // first element; theme names do not.
    #[test]
    fn check_path() {
        for (p, want) in [
            ("github.com/bep/mycomponent", true),
            ("github.com/foo/v2", true),
            ("gopkg.in/yaml.v3", true),
            ("example.com", true),
            ("mytheme", false),
            ("my-theme", false),
            ("/abs/path", false),
            ("", false),
            ("github.com/../x", false),
            ("github.com/a//b", false),
            ("-foo.com/x", false),
            ("github.com/foo/", false),
        ] {
            assert_eq!(is_probably_module(p), want, "{p:?}");
        }
    }

    #[test]
    fn split_path_versions() {
        for (p, prefix, major, ok) in [
            ("github.com/foo", "github.com/foo", "", true),
            ("github.com/foo/v2", "github.com/foo", "/v2", true),
            ("github.com/foo/v1", "github.com/foo/v1", "", false),
            ("github.com/foo/v02", "github.com/foo/v02", "", false),
            ("gopkg.in/yaml.v3", "gopkg.in/yaml", ".v3", true),
        ] {
            assert_eq!(split_path_version(p), (prefix, major, ok), "{p:?}");
        }
    }
}
