//! Port of `modules/config.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

use std::collections::BTreeMap;

use go_value::{Map, MapType, Value};
use nh_common::files;
use nh_common::{Error, Result};
use nh_config::config_provider::{AllProvider, Provider, get_string_slice_preserve_string};
use nh_config::decode::{AnyValue, FieldRef, weak_decode_into};
use nh_config::decode_struct;

/// Go: `modules.WorkspaceDisabled`.
pub const WORKSPACE_DISABLED: &str = "off";

/// Go: `modules.Mount`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mount {
    /// Relative path in source repo, e.g. "scss".
    pub source: String,
    /// Relative target path, e.g. "assets/bootstrap/scss".
    pub target: String,
    /// Any file in this mount will be associated with this language.
    pub lang: String,
    /// Include only files matching the given Glob patterns (Go `any`: string or slice; normalised at decode).
    pub include_files: Vec<String>,
    /// Exclude all files matching the given Glob patterns (Go `any`: string or slice; normalised at decode).
    pub exclude_files: Vec<String>,
    pub disable_watch: bool,
}

impl Mount {
    /// Go: `Mount.key()`.
    // Go: modules/config.go:key
    pub fn key(&self) -> String {
        [
            self.lang.as_str(),
            self.source.as_str(),
            self.target.as_str(),
        ]
        .join("/")
    }

    /// Go: `Mount.Component()` — first path element of Target.
    // Go: modules/config.go:Component
    pub fn component(&self) -> String {
        self.target
            .split(FILE_SEPARATOR)
            .next()
            .unwrap_or_default()
            .to_string()
    }

    /// Go: `Mount.ComponentAndName()`.
    // Go: modules/config.go:ComponentAndName
    pub fn component_and_name(&self) -> (String, String) {
        match self.target.split_once(FILE_SEPARATOR) {
            Some((c, n)) => (c.to_string(), n.to_string()),
            None => (self.target.clone(), String::new()),
        }
    }
}

/// Go: `modules.fileSeparator` (`string(os.PathSeparator)`).
pub(crate) const FILE_SEPARATOR: &str = "/";

/// Go: `modules.Import`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Import {
    pub path: String,
    /// Set when Path is replaced in project config (Go: unexported `pathProjectReplaced`).
    pub path_project_replaced: bool,
    pub ignore_config: bool,
    pub ignore_imports: bool,
    pub no_mounts: bool,
    pub no_vendor: bool,
    pub disable: bool,
    pub mounts: Vec<Mount>,
}

/// Go: `modules.Config`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub mounts: Vec<Mount>,
    pub imports: Vec<Import>,
    /// Module params (`[module.params]`), `None` if unset.
    pub params: Option<Map>,
    pub hugo_version_min: String,
    pub hugo_version_max: String,
    pub no_vendor: String,
    pub vendor_closest: bool,
    pub replacements: Vec<String>,
    /// Go: the unexported `replacementsMap` (`None` = Go's nil map).
    pub replacements_map: Option<BTreeMap<String, String>>,
    pub proxy: String,
    pub no_proxy: String,
    pub private: String,
    pub auth: String,
    pub workspace: String,
    /// Go keeps the `IncludeFiles`/`ExcludeFiles` of a mount as `any`; this holds the raw values
    /// of `mounts` (same index) for the `hugo config` JSON dump. Not part of Go's struct.
    pub mounts_raw_files: Vec<(Value, Value)>,
}

/// Go: `modules.DefaultModuleConfig`.
pub fn default_module_config() -> Config {
    Config {
        // Default to direct, which means "git clone" and similar. We
        // will investigate proxy settings in more depth later.
        // See https://github.com/golang/go/issues/26334
        proxy: "direct".to_string(),

        // Comma separated glob list matching paths that should not use the
        // proxy configured above.
        no_proxy: "none".to_string(),

        // Comma separated glob list matching paths that should be
        // treated as private.
        private: "*.*".to_string(),

        // Default is no workspace resolution.
        workspace: WORKSPACE_DISABLED.to_string(),

        // A list of replacement directives mapping a module path to a directory
        // or a theme component in the themes folder.
        replacements: Vec::new(),
        ..Default::default()
    }
}

impl Config {
    /// Go: `Config.hasModuleImport()`.
    // Go: modules/config.go:hasModuleImport
    pub fn has_module_import(&self) -> bool {
        self.imports
            .iter()
            .any(|imp| super::client::is_probably_module(&imp.path))
    }

    /// Go: `HugoVersion{Min, Max}.String()`.
    // Go: modules/config.go:String
    pub fn hugo_version_string(&self) -> String {
        let (min, max) = (&self.hugo_version_min, &self.hugo_version_max);
        if !min.is_empty() && !max.is_empty() {
            return format!("{min}/{max}");
        }
        if !min.is_empty() {
            return format!("Min {min}");
        }
        if !max.is_empty() {
            return format!("Max {max}");
        }
        String::new()
    }

    /// Go: `HugoVersion.IsValid()`.
    // Go: modules/config.go:IsValid
    pub fn hugo_version_is_valid(&self) -> bool {
        use nh_config::neohugo::version::CURRENT_VERSION;
        let current = CURRENT_VERSION.version();

        let mut is_valid = self.hugo_version_min.is_empty()
            || current.compare(&Value::string(self.hugo_version_min.as_str())) <= 0;

        if !self.hugo_version_max.is_empty()
            && current.compare(&Value::string(self.hugo_version_max.as_str())) < 0
        {
            is_valid = false;
        }

        is_valid
    }
}

// ---------------------------------------------------------------------------
// mapstructure mirrors of the Go structs (Go decodes `IncludeFiles`/`ExcludeFiles` into `any`
// and `HugoVersion` into a nested struct).

#[derive(Clone, Debug, Default, PartialEq)]
struct MountD {
    source: String,
    target: String,
    lang: String,
    include_files: AnyValue,
    exclude_files: AnyValue,
    disable_watch: bool,
}

decode_struct!(MountD, "modules.Mount", |s| vec![
    FieldRef::new("Source", &mut s.source),
    FieldRef::new("Target", &mut s.target),
    FieldRef::new("Lang", &mut s.lang),
    FieldRef::new("IncludeFiles", &mut s.include_files),
    FieldRef::new("ExcludeFiles", &mut s.exclude_files),
    FieldRef::new("DisableWatch", &mut s.disable_watch),
]);

#[derive(Clone, Debug, Default, PartialEq)]
struct ImportD {
    path: String,
    path_project_replaced: bool,
    ignore_config: bool,
    ignore_imports: bool,
    no_mounts: bool,
    no_vendor: bool,
    disable: bool,
    mounts: Vec<MountD>,
}

decode_struct!(ImportD, "modules.Import", |s| vec![
    FieldRef::new("Path", &mut s.path),
    FieldRef::new("pathProjectReplaced", &mut s.path_project_replaced).unexported(),
    FieldRef::new("IgnoreConfig", &mut s.ignore_config),
    FieldRef::new("IgnoreImports", &mut s.ignore_imports),
    FieldRef::new("NoMounts", &mut s.no_mounts),
    FieldRef::new("NoVendor", &mut s.no_vendor),
    FieldRef::new("Disable", &mut s.disable),
    FieldRef::new("Mounts", &mut s.mounts),
]);

#[derive(Clone, Debug, Default, PartialEq)]
struct HugoVersionD {
    min: String,
    max: String,
}

decode_struct!(HugoVersionD, "modules.HugoVersion", |s| vec![
    FieldRef::new("Min", &mut s.min),
    FieldRef::new("Max", &mut s.max),
]);

#[derive(Clone, Debug, PartialEq)]
struct ConfigD {
    mounts: Vec<MountD>,
    imports: Vec<ImportD>,
    params: Map,
    hugo_version: HugoVersionD,
    no_vendor: String,
    vendor_closest: bool,
    replacements: Vec<String>,
    replacements_map: BTreeMap<String, String>,
    proxy: String,
    no_proxy: String,
    private: String,
    auth: String,
    workspace: String,
}

impl Default for ConfigD {
    fn default() -> Self {
        ConfigD {
            mounts: Vec::new(),
            imports: Vec::new(),
            params: Map::new(MapType::StringAny),
            hugo_version: HugoVersionD::default(),
            no_vendor: String::new(),
            vendor_closest: false,
            replacements: Vec::new(),
            replacements_map: BTreeMap::new(),
            proxy: String::new(),
            no_proxy: String::new(),
            private: String::new(),
            auth: String::new(),
            workspace: String::new(),
        }
    }
}

decode_struct!(ConfigD, "modules.Config", |s| vec![
    FieldRef::new("Mounts", &mut s.mounts),
    FieldRef::new("Imports", &mut s.imports),
    FieldRef::new("Params", &mut s.params),
    FieldRef::new("HugoVersion", &mut s.hugo_version),
    FieldRef::new("NoVendor", &mut s.no_vendor),
    FieldRef::new("VendorClosest", &mut s.vendor_closest),
    FieldRef::new("Replacements", &mut s.replacements),
    FieldRef::new("replacementsMap", &mut s.replacements_map).unexported(),
    FieldRef::new("Proxy", &mut s.proxy),
    FieldRef::new("NoProxy", &mut s.no_proxy),
    FieldRef::new("Private", &mut s.private),
    FieldRef::new("Auth", &mut s.auth),
    FieldRef::new("Workspace", &mut s.workspace),
]);

fn mount_from_d(m: MountD) -> (Mount, (Value, Value)) {
    let norm = |v: &Value| -> Vec<String> {
        nh_common::types::convert::to_string_slice_preserve_string(v)
            .iter()
            .map(|s| s.to_str_lossy().into_owned())
            .collect()
    };
    let mnt = Mount {
        include_files: norm(&m.include_files.0),
        exclude_files: norm(&m.exclude_files.0),
        source: m.source,
        target: m.target,
        lang: m.lang,
        disable_watch: m.disable_watch,
    };
    (mnt, (m.include_files.0, m.exclude_files.0))
}

fn mount_to_d(m: &Mount, raw: Option<&(Value, Value)>) -> MountD {
    let (inc, exc) = match raw {
        Some((i, e)) => (i.clone(), e.clone()),
        None => (raw_files(&m.include_files), raw_files(&m.exclude_files)),
    };
    MountD {
        source: m.source.clone(),
        target: m.target.clone(),
        lang: m.lang.clone(),
        include_files: AnyValue(inc),
        exclude_files: AnyValue(exc),
        disable_watch: m.disable_watch,
    }
}

/// The `any` value of a normalised include/exclude list (Go's nil for an empty list).
pub fn raw_files(v: &[String]) -> Value {
    if v.is_empty() {
        Value::Invalid
    } else {
        Value::string_list(v.iter().map(|s| s.as_str()))
    }
}

impl Config {
    /// The raw (`any`) include/exclude values of mount `i`.
    pub fn mount_raw_files(&self, i: usize) -> (Value, Value) {
        match self.mounts_raw_files.get(i) {
            Some(r) => r.clone(),
            None => {
                let m = &self.mounts[i];
                (raw_files(&m.include_files), raw_files(&m.exclude_files))
            }
        }
    }

    fn to_d(&self) -> ConfigD {
        ConfigD {
            mounts: self
                .mounts
                .iter()
                .enumerate()
                .map(|(i, m)| mount_to_d(m, self.mounts_raw_files.get(i)))
                .collect(),
            imports: self
                .imports
                .iter()
                .map(|imp| ImportD {
                    path: imp.path.clone(),
                    path_project_replaced: imp.path_project_replaced,
                    ignore_config: imp.ignore_config,
                    ignore_imports: imp.ignore_imports,
                    no_mounts: imp.no_mounts,
                    no_vendor: imp.no_vendor,
                    disable: imp.disable,
                    mounts: imp.mounts.iter().map(|m| mount_to_d(m, None)).collect(),
                })
                .collect(),
            params: self
                .params
                .clone()
                .unwrap_or_else(|| Map::new(MapType::StringAny)),
            hugo_version: HugoVersionD {
                min: self.hugo_version_min.clone(),
                max: self.hugo_version_max.clone(),
            },
            no_vendor: self.no_vendor.clone(),
            vendor_closest: self.vendor_closest,
            replacements: self.replacements.clone(),
            replacements_map: self.replacements_map.clone().unwrap_or_default(),
            proxy: self.proxy.clone(),
            no_proxy: self.no_proxy.clone(),
            private: self.private.clone(),
            auth: self.auth.clone(),
            workspace: self.workspace.clone(),
        }
    }

    fn set_from_d(&mut self, d: ConfigD, params_set: bool) {
        let mut mounts = Vec::new();
        let mut raws = Vec::new();
        for m in d.mounts {
            let (m, r) = mount_from_d(m);
            mounts.push(m);
            raws.push(r);
        }
        self.mounts = mounts;
        self.mounts_raw_files = raws;
        self.imports = d
            .imports
            .into_iter()
            .map(|imp| Import {
                path: imp.path,
                path_project_replaced: imp.path_project_replaced,
                ignore_config: imp.ignore_config,
                ignore_imports: imp.ignore_imports,
                no_mounts: imp.no_mounts,
                no_vendor: imp.no_vendor,
                disable: imp.disable,
                mounts: imp.mounts.into_iter().map(|m| mount_from_d(m).0).collect(),
            })
            .collect();
        if params_set {
            self.params = Some(d.params);
        }
        self.hugo_version_min = d.hugo_version.min;
        self.hugo_version_max = d.hugo_version.max;
        self.no_vendor = d.no_vendor;
        self.vendor_closest = d.vendor_closest;
        self.replacements = d.replacements;
        self.proxy = d.proxy;
        self.no_proxy = d.no_proxy;
        self.private = d.private;
        self.auth = d.auth;
        self.workspace = d.workspace;
    }
}

/// Go: `modules.ApplyProjectConfigDefaults(mod, cfgs...)` — default mounts for components without one.
// Go: modules/config.go:ApplyProjectConfigDefaults
pub fn apply_project_config_defaults(
    m: &mut super::module::Module,
    cfgs: &[&dyn nh_config::config_provider::AllProvider],
) -> Result<()> {
    // To bridge between old and new configuration format we need
    // a way to make sure all of the core components are configured on
    // the basic level.
    let mut components_configured: BTreeMap<String, bool> = BTreeMap::new();
    for mnt in &m.mounts {
        if !mnt.target.starts_with(files::JS_CONFIG_FOLDER_MOUNT_PREFIX) {
            components_configured.insert(mnt.component(), true);
        }
    }

    let mut mounts: Vec<Mount> = Vec::new();

    for component in [
        files::COMPONENT_FOLDER_CONTENT,
        files::COMPONENT_FOLDER_DATA,
        files::COMPONENT_FOLDER_LAYOUTS,
        files::COMPONENT_FOLDER_I18N,
        files::COMPONENT_FOLDER_ARCHETYPES,
        files::COMPONENT_FOLDER_ASSETS,
        files::COMPONENT_FOLDER_STATIC,
    ] {
        if components_configured
            .get(component)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }

        let first: &dyn AllProvider = cfgs[0];
        let dirs_base = first.dirs_base();
        let is_multihost = first.is_multihost();

        for (i, cfg) in cfgs.iter().enumerate() {
            let dirs = cfg.dirs();
            let mut dir = String::new();
            let mut drop_lang = false;
            match component {
                files::COMPONENT_FOLDER_CONTENT => {
                    dir = dirs.content_dir.clone();
                    drop_lang = dir == dirs_base.content_dir;
                }
                files::COMPONENT_FOLDER_DATA => dir = dirs.data_dir.clone(),
                files::COMPONENT_FOLDER_LAYOUTS => dir = dirs.layout_dir.clone(),
                files::COMPONENT_FOLDER_I18N => dir = dirs.i18n_dir.clone(),
                files::COMPONENT_FOLDER_ARCHETYPES => dir = dirs.arche_type_dir.clone(),
                files::COMPONENT_FOLDER_ASSETS => dir = dirs.asset_dir.clone(),
                files::COMPONENT_FOLDER_STATIC => {
                    // For static dirs, we only care about the language in multihost setups.
                    drop_lang = !is_multihost;
                }
                _ => {}
            }

            let per_lang = matches!(
                component,
                files::COMPONENT_FOLDER_CONTENT | files::COMPONENT_FOLDER_STATIC
            );
            if i > 0 && !per_lang {
                continue;
            }

            let mut lang = String::new();
            if per_lang && !drop_lang {
                lang = cfg.language().lang.clone();
            }

            // Static mounts are a little special.
            if component == files::COMPONENT_FOLDER_STATIC {
                let static_dirs = cfg.static_dirs();
                for dir in static_dirs {
                    mounts.push(Mount {
                        lang: lang.clone(),
                        source: dir,
                        target: component.to_string(),
                        ..Default::default()
                    });
                }
                continue;
            }

            if !dir.is_empty() {
                mounts.push(Mount {
                    lang,
                    source: dir,
                    target: component.to_string(),
                    ..Default::default()
                });
            }
        }
    }

    m.mounts.extend(mounts);

    // Temporary: Remove duplicates.
    let mut seen: BTreeMap<String, bool> = BTreeMap::new();
    let mut new_mounts: Vec<Mount> = Vec::new();
    for mnt in &m.mounts {
        let key = format!("{}{}{}", mnt.source, mnt.target, mnt.lang);
        if seen.get(&key).copied().unwrap_or(false) {
            continue;
        }
        seen.insert(key, true);
        new_mounts.push(mnt.clone());
    }
    m.mounts = new_mounts;

    Ok(())
}

/// Go: `modules.DecodeConfig(cfg)` (maps legacy `contentDir`, `staticDir*` ... to mounts).
// Go: modules/config.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<Config> {
    decode_config_with_replacements(Some(cfg), None)
}

/// Go: `modules.decodeConfig(cfg, pathReplacements)` (`cfg` may be nil).
// Go: modules/config.go:decodeConfig
pub fn decode_config_with_replacements(
    cfg: Option<&dyn Provider>,
    path_replacements: Option<&BTreeMap<String, String>>,
) -> Result<Config> {
    let mut c = default_module_config();
    c.replacements_map = path_replacements.cloned();

    let Some(cfg) = cfg else {
        return Ok(c);
    };

    let theme_set = cfg.is_set("theme");
    let module_set = cfg.is_set("module");

    if module_set {
        let m = cfg.get_string_map("module");
        let params_set = m
            .entries
            .iter()
            .any(|(k, v)| go_unicode::strings::equal_fold(k.as_bytes(), b"Params") && !v.is_nil());
        let mut d = c.to_d();
        let res = weak_decode_into(&Value::map(m), &mut d);
        c.set_from_d(d, params_set);
        res?;

        if c.replacements_map.is_none() {
            if c.replacements.len() == 1 {
                c.replacements = c.replacements[0].split(',').map(String::from).collect();
            }

            for repl in c.replacements.iter_mut() {
                *repl = go_trim_space(repl);
            }

            let mut rm = BTreeMap::new();
            for repl in &c.replacements {
                let parts: Vec<&str> = repl.split("->").collect();
                if parts.len() != 2 {
                    c.replacements_map = Some(rm);
                    return Err(Error::new(format!(
                        "invalid module.replacements: {}; configure replacement pairs on the form \"oldpath->newpath\" ",
                        go_strconv::quote(repl.as_bytes())
                    )));
                }

                rm.insert(go_trim_space(parts[0]), go_trim_space(parts[1]));
            }
            c.replacements_map = Some(rm);
        }

        if let Some(rm) = &c.replacements_map {
            for imp in c.imports.iter_mut() {
                if let Some(new_imp) = rm.get(&imp.path) {
                    imp.path = new_imp.clone();
                    imp.path_project_replaced = true;
                }
            }
        }

        for mnt in c.mounts.iter_mut() {
            mnt.source = go_path::filepath::clean(&mnt.source);
            mnt.target = go_path::filepath::clean(&mnt.target);
        }

        if c.workspace.is_empty() {
            c.workspace = WORKSPACE_DISABLED.to_string();
        }
        if c.workspace != WORKSPACE_DISABLED {
            c.workspace = go_path::filepath::clean(&c.workspace);
            if !go_path::filepath::is_abs(&c.workspace) {
                let working_dir = cfg.get_string("workingDir");
                c.workspace =
                    go_path::filepath::join(&[working_dir.as_str(), c.workspace.as_str()]);
            }
            if std::fs::metadata(&c.workspace).is_err() {
                return Err(Error::new(format!(
                    "module workspace {} does not exist. Check your module.workspace setting (or HUGO_MODULE_WORKSPACE env var).",
                    go_strconv::quote(c.workspace.as_bytes())
                )));
            }
        }
    }

    if theme_set {
        let imports = get_string_slice_preserve_string(cfg, "theme");
        for imp in imports {
            c.imports.push(Import {
                path: imp,
                ..Default::default()
            });
        }
    }

    Ok(c)
}

/// Go: `strings.TrimSpace`.
pub(crate) fn go_trim_space(s: &str) -> String {
    go_unicode::strings::trim_space_str(s).to_string()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: modules/config.go (412 lines; 6/9 funcs executed)
//   types: Config, HugoVersion, Import, Mount
// OK L59-165: ApplyProjectConfigDefaults(mod Module, cfgs ...config.AllProvider) error
// OK L168-170: DecodeConfig(cfg config.Provider) (Config, error)
// OK L172-252: decodeConfig(cfg config.Provider, pathReplacements map[string]string) (Config, error)
// OK L314-321: (c Config) hasModuleImport() bool
// OK L332-345: (v HugoVersion) String() string
// OK L349-359: (v HugoVersion) IsValid() bool
// OK L401-403: (m Mount) key() string
// OK L405-407: (m Mount) Component() string
// OK L409-412: (m Mount) ComponentAndName() (string, string)
// ---------------------------------------------------------------------------

// Ports of modules/config_test.go.
#[cfg(test)]
mod tests {
    use super::*;
    use nh_config::config_loader::from_config_string;
    use nh_config::neohugo::version::VersionString;

    fn hv(min: &str, max: &str) -> Config {
        Config {
            hugo_version_min: min.to_string(),
            hugo_version_max: max.to_string(),
            ..Default::default()
        }
    }

    // Go: modules/config_test.go:TestConfigHugoVersionIsValid
    #[test]
    fn config_hugo_version_is_valid() {
        for (c, expect) in [
            (hv("0.33.0", ""), true),
            (hv("0.56.0-DEV", ""), true),
            (hv("0.33.0", "0.55.0"), false),
            (hv("0.33.0", "0.199.0"), true),
        ] {
            assert_eq!(c.hugo_version_is_valid(), expect, "{c:?}");
        }
    }

    // Go: modules/config_test.go:TestDecodeConfig (Basic)
    #[test]
    fn decode_config_basic() {
        let temp_dir =
            std::env::temp_dir().join(format!("nh-hugofs-modules-config-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let temp_dir = temp_dir.to_str().unwrap().to_string();
        let toml_config = format!(
            r#"
workingDir = {}
[module]
workspace = "hugo.work"
[module.hugoVersion]
min = "0.54.2"
max = "0.199.0"
[[module.mounts]]
source="src/project/blog"
target="content/blog"
lang="en"
[[module.imports]]
path="github.com/bep/mycomponent"
[[module.imports.mounts]]
source="scss"
target="assets/bootstrap/scss"
[[module.imports.mounts]]
source="src/markdown/blog"
target="content/blog"
lang="en"
"#,
            go_strconv::quote(temp_dir.as_bytes())
        );

        let hugo_work_filename = go_path::filepath::join(&[temp_dir.as_str(), "hugo.work"]);
        std::fs::write(&hugo_work_filename, b"").unwrap();
        let cfg = from_config_string(&toml_config, "toml").unwrap();

        let mcfg = decode_config(&cfg).unwrap();
        let _ = std::fs::remove_dir_all(&temp_dir);

        let v056 = VersionString("0.56.0".to_string());
        assert_eq!(
            v056.compare(&Value::string(mcfg.hugo_version_min.as_str())),
            -1
        );
        assert_eq!(
            v056.compare(&Value::string(mcfg.hugo_version_max.as_str())),
            1
        );

        assert_eq!(mcfg.workspace, hugo_work_filename);

        assert_eq!(mcfg.mounts.len(), 1);
        assert_eq!(mcfg.imports.len(), 1);
        let imp = &mcfg.imports[0];
        assert_eq!(imp.path, "github.com/bep/mycomponent");
        assert_eq!(imp.mounts[1].source, "src/markdown/blog");
        assert_eq!(imp.mounts[1].target, "content/blog");
        assert_eq!(imp.mounts[1].lang, "en");
    }

    // Go: modules/config_test.go:TestDecodeConfig (Replacements)
    #[test]
    fn decode_config_replacements() {
        for toml_config in [
            r#"
[module]
replacements="a->b,github.com/bep/mycomponent->c"
[[module.imports]]
path="github.com/bep/mycomponent"
"#,
            r#"
[module]
replacements=["a->b","github.com/bep/mycomponent->c"]
[[module.imports]]
path="github.com/bep/mycomponent"
"#,
        ] {
            let cfg = from_config_string(toml_config, "toml").unwrap();
            let mcfg = decode_config(&cfg).unwrap();
            assert_eq!(mcfg.replacements, ["a->b", "github.com/bep/mycomponent->c"]);
            let want: BTreeMap<String, String> = [
                ("a".to_string(), "b".to_string()),
                ("github.com/bep/mycomponent".to_string(), "c".to_string()),
            ]
            .into_iter()
            .collect();
            assert_eq!(mcfg.replacements_map, Some(want));
            assert_eq!(mcfg.imports[0].path, "c");
            assert!(mcfg.imports[0].path_project_replaced);
        }
    }

    // Go: modules/config_test.go:TestDecodeConfigBothOldAndNewProvided
    #[test]
    fn decode_config_both_old_and_new_provided() {
        let cfg = from_config_string(
            r#"

theme = ["b", "c"]

[module]
[[module.imports]]
path="a"

"#,
            "toml",
        )
        .unwrap();
        let mod_cfg = decode_config(&cfg).unwrap();
        assert_eq!(mod_cfg.imports.len(), 3);
        assert_eq!(mod_cfg.imports[0].path, "a");
    }

    // Go: modules/config_test.go:TestDecodeConfigTheme
    #[test]
    fn decode_config_theme() {
        let cfg = from_config_string(
            r#"

theme = ["a", "b"]
"#,
            "toml",
        )
        .unwrap();
        let mcfg = decode_config(&cfg).unwrap();
        assert_eq!(mcfg.imports.len(), 2);
        assert_eq!(mcfg.imports[0].path, "a");
        assert_eq!(mcfg.imports[1].path, "b");
    }

    #[test]
    fn mount_component() {
        let m = Mount {
            source: "scss".to_string(),
            target: "assets/bootstrap/scss".to_string(),
            lang: "en".to_string(),
            ..Default::default()
        };
        assert_eq!(m.component(), "assets");
        assert_eq!(
            m.component_and_name(),
            ("assets".to_string(), "bootstrap/scss".to_string())
        );
        assert_eq!(m.key(), "en/scss/assets/bootstrap/scss");
        let m = Mount {
            target: "static".to_string(),
            ..Default::default()
        };
        assert_eq!(
            m.component_and_name(),
            ("static".to_string(), String::new())
        );
    }

    #[test]
    fn invalid_replacement() {
        let cfg = from_config_string("[module]\nreplacements=\"a\"\n", "toml").unwrap();
        let err = decode_config(&cfg).unwrap_err().to_string();
        assert_eq!(
            err,
            "invalid module.replacements: \"a\"; configure replacement pairs on the form \"oldpath->newpath\" "
        );
    }
}
