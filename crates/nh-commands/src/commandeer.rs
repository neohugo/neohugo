//! Port of `commands/commandeer.go`.
//!
//! Owner: Wave B task T25 (commands-cli).

//!
//! Scope for the port: the `hugo` root command (build), `version`, `env`, `config` (+ `config mounts`).
//! `server`, `new`, `mod`, `convert`, `gen`, `import`, `list`, `deploy`, `release` are out of
//! scope: they are accepted by the parser but return an "unsupported in neohugo-rs" user error.
//! CLI parsing uses `clap` (derive off, builder API) to mirror cobra/pflag semantics that matter:
//! `--flag=value`, `--flag value`, `-D`, string slices `--theme a,b`, bool flags `--minify`.
//! Go `mapLegacyArgs` (`hugo --help`-style legacy args) is ported verbatim.

use std::sync::atomic::AtomicI32;
use std::sync::{Arc, Mutex};

use nh_allconfig::allconfig::Configs;
use nh_common::loggers::Logger;
use nh_common::Result;
use nh_config::config_provider::Provider;
use nh_hugofs::fs::Fs;
use nh_hugolib::hugo_sites::HugoSites;
use nh_hugolib::hugo_sites_build::BuildCfg;

/// Go: `commands.commonConfig`.
pub struct CommonConfig {
    pub configs: Arc<Configs>,
    /// The flags config (Go `cfg config.Provider`).
    pub cfg: Arc<dyn Provider>,
    pub fs: Arc<Fs>,
}

/// Go: `commands.configKey`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ConfigKey {
    pub counter: i32,
    pub ignore_modules_does_not_exists: bool,
}

/// Parsed flag values (Go: the flag fields of `rootCommand` plus the pflag set that
/// `flagsToCfg` reads). Keys use the Go config key names (`baseURL`, `buildDrafts`, `minify`, ...).
#[derive(Clone, Debug, Default)]
pub struct Flags {
    /// Flags that were set on the command line, in Go config-key spelling, with their raw values.
    pub set: Vec<(String, FlagValue)>,
}

#[derive(Clone, Debug)]
pub enum FlagValue {
    Bool(bool),
    String(String),
    StringSlice(Vec<String>),
}

/// Go: `commands.rootCommand`.
pub struct RootCommand {
    pub logger: Logger,
    pub config_version_id: AtomicI32,
    /// Go: `commonConfigs *lazycache.Cache[configKey, *commonConfig]`.
    pub common_configs: Mutex<Vec<(ConfigKey, Arc<CommonConfig>)>>,
    /// Go: `hugoSites *lazycache.Cache[configKey, *hugolib.HugoSites]`.
    pub hugo_sites: Mutex<Vec<(ConfigKey, Arc<HugoSites>)>>,

    // Flags.
    pub source: String,
    pub build_watch: bool,
    pub environment: String,
    pub base_url: String,
    pub gc: bool,
    pub poll: String,
    pub force_sync_static: bool,
    pub log_level: String,
    pub quiet: bool,
    pub dev_mode: bool,
    pub render_to_memory: bool,
    pub cfg_file: String,
    pub cfg_dir: String,
    /// Go: `--clock` (persistent string flag copied into config key `clock` by `flagsToCfg`).
    pub clock: String,
    pub flags: Flags,
}

/// Entry point used by `main.rs`. Returns the process exit code.
/// Go: commands/commandeer.go:Execute (+ main.go:main printing the error and `os.Exit(1)`).
pub fn execute(args: Vec<String>) -> i32 {
    todo!()
}

/// Go: commands/commandeer.go:mapLegacyArgs
pub fn map_legacy_args(args: Vec<String>) -> Vec<String> {
    todo!()
}

impl RootCommand {
    /// Go: `(*rootCommand).Build`.
    pub fn build(&self, bcfg: BuildCfg, cfg: Arc<dyn Provider>) -> Result<Arc<HugoSites>> {
        todo!()
    }

    /// Go: `(*rootCommand).ConfigFromProvider` — `allconfig.LoadConfig` with the flags provider,
    /// then `htime.Clock = clocks.Start(configs.Base.C.Clock)` when `--clock` is set (a clock that
    /// starts at the given instant and advances with wall time: `nh_common::htime::StartClock`),
    /// `hugofs.NewFrom`, `--renderToMemory`, `publishDir` creation.
    pub fn config_from_provider(&self, key: ConfigKey, cfg: Arc<dyn Provider>) -> Result<Arc<CommonConfig>> {
        todo!()
    }

    /// Go: `(*rootCommand).getOrCreateHugo`.
    pub fn get_or_create_hugo(&self, cfg: Arc<dyn Provider>, ignore_module_does_not_exist: bool) -> Result<Arc<HugoSites>> {
        todo!()
    }

    /// Go: `(*rootCommand).Run` — build, copy static, print build stats table (stdout text is
    /// not part of the parity check, but keep it faithful).
    pub fn run(&mut self, args: &[String]) -> Result<()> {
        todo!()
    }

    /// Go: `(*rootCommand).PreRun` — logger creation, `--quiet`/`--logLevel`.
    pub fn pre_run(&mut self) -> Result<()> {
        todo!()
    }

    /// Go: `(*rootCommand).createLogger`.
    pub fn create_logger(&self, running: bool) -> Result<Logger> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/commandeer.go (679 lines; 20/28 funcs executed)
//   types: commonConfig, configKey, rootCommand, simpleCommand
// EX L60-88: Execute(args []string) error
// EX L153-155: (r *rootCommand) isVerbose() bool
// EX L157-167: (r *rootCommand) Close() error
//    L169-179: (r *rootCommand) Build(cd *simplecobra.Commandeer, bcfg hugolib.BuildCfg, cfg config.Provider) (*hugolib.HugoSites, error)
// EX L181-183: (r *rootCommand) Commands() []simplecobra.Commander
//    L185-217: (r *rootCommand) ConfigFromConfig(key configKey, oldConf *commonConfig) (*commonConfig, error)
// EX L219-331: (r *rootCommand) ConfigFromProvider(key configKey, cfg config.Provider) (*commonConfig, error)
// EX L333-340: (r *rootCommand) HugFromConfig(conf *commonConfig) (*hugolib.HugoSites, error)
//    L342-344: (r *rootCommand) Neohugo(cfg config.Provider) (*hugolib.HugoSites, error)
//    L346-357: (r *rootCommand) getOrCreateHugo(cfg config.Provider, ignoreModuleDoesNotExist bool) (*hugolib.HugoSites, error)
// EX L359-361: (r *rootCommand) newDepsConfig(conf *commonConfig) deps.DepsCfg
// EX L363-365: (r *rootCommand) Name() string
// EX L367-422: (r *rootCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error
// EX L424-466: (r *rootCommand) PreRun(cd, runner *simplecobra.Commandeer) error
// EX L468-499: (r *rootCommand) createLogger(running bool) (loggers.Logger, error)
//    L501-504: (r *rootCommand) resetLogs()
//    L507-509: (r *rootCommand) IsTestRun() bool
// EX L511-513: (r *rootCommand) Init(cd *simplecobra.Commandeer) error
// EX L515-566: (r *rootCommand) initRootCommand(subCommandName string, cd *simplecobra.Commandeer) error
// EX L569-577: applyLocalFlagsBuildConfig(cmd *cobra.Command, r *rootCommand)
// EX L580-616: applyLocalFlagsBuild(cmd *cobra.Command, r *rootCommand)
// EX L618-621: (r *rootCommand) timeTrack(start time.Time, name string)
// EX L637-639: (c *simpleCommand) Commands() []simplecobra.Commander
// EX L641-643: (c *simpleCommand) Name() string
//    L645-650: (c *simpleCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error
// EX L652-664: (c *simpleCommand) Init(cd *simplecobra.Commandeer) error
//    L666-671: (c *simpleCommand) PreRun(cd, runner *simplecobra.Commandeer) error
// EX L673-679: mapLegacyArgs(args []string) []string
// ---------------------------------------------------------------------------
