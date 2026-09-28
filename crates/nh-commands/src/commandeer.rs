//! Port of `commands/commandeer.go`.
//!
//! Owner: Wave B task T25 (commands-cli).

//!
//! Scope for the port: the `hugo` root command (build), `version`, `env`, `config` (+ `config mounts`).
//! `server`, `new`, `mod`, `convert`, `gen`, `import`, `list`, `deploy`, `release`, `completion`
//! are resolved by the parser but return an "is not supported" user error.
//! CLI parsing is a port of pflag/cobra/simplecobra (modules `pflag` and `cobra`; clap is not in
//! the offline crate cache, and a port reproduces Go's parse rules and error texts exactly).
//! Go `mapLegacyArgs` is ported verbatim.

use std::io::Write;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use go_value::Value;
use nh_allconfig::allconfig::Configs;
use nh_allconfig::load::{ConfigSourceDescriptor, Getenv, load_config};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::loggers::{Level, LogSink, Logger, Options};
use nh_config::config_provider::Provider;
use nh_hugofs::afero::Fs as AferoFs;
use nh_hugofs::fs::Fs;
use nh_hugolib::hugo_sites::{FuncMapFactory, HugoSites, NewHugoSitesCfg};
use nh_hugolib::hugo_sites_build::BuildCfg;

use crate::pflag::{self, FlagSet};

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

/// Parsed flag values (Go: the pflag set of the executed command that `flagsToCfg` reads).
#[derive(Clone, Debug, Default)]
pub struct Flags {
    /// Flags that were set on the command line (pflag `Changed`), by flag name in `VisitAll`
    /// (sorted) order, with their values.
    pub set: Vec<(String, FlagValue)>,
    /// Every flag of the command (Go `flags.Lookup` for `setValueFromFlag`'s `force`).
    pub all: Vec<(String, FlagValue)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagValue {
    Bool(bool),
    String(String),
    StringSlice(Vec<String>),
}

impl Flags {
    /// The flag values of a parsed pflag set.
    pub fn from_flag_set(fs: &FlagSet) -> Flags {
        let mut out = Flags::default();
        for f in fs.visit_all() {
            let v = match &f.value {
                pflag::Value::Bool(b) => FlagValue::Bool(*b),
                pflag::Value::String(s) => FlagValue::String(s.clone()),
                pflag::Value::StringSlice { value, .. } => FlagValue::StringSlice(value.clone()),
            };
            if f.changed {
                out.set.push((f.name.clone(), v.clone()));
            }
            out.all.push((f.name.clone(), v));
        }
        out
    }
}

/// Where the command writes and what it sees of the process (Go: `os.Stdout`, `os.Stderr`,
/// `os.Getwd`, `os.Environ`/`os.Getenv`), plus the template func map factory handed to
/// `NewHugoSites`. The defaults are the process's.
#[derive(Clone)]
pub struct ExecOptions {
    pub stdout: LogSink,
    pub stderr: LogSink,
    /// The working directory (`None` = the process's).
    pub cwd: Option<String>,
    /// The process environment (`None` = the process's).
    pub environ: Option<Vec<(String, String)>>,
    /// `None` = `nh_tplfuncs::tplimplinit::create_func_map` (hugolib's default).
    pub func_map_factory: Option<FuncMapFactory>,
}

impl Default for ExecOptions {
    fn default() -> Self {
        ExecOptions {
            stdout: Arc::new(Mutex::new(std::io::stdout())),
            stderr: Arc::new(Mutex::new(std::io::stderr())),
            cwd: None,
            environ: None,
            func_map_factory: None,
        }
    }
}

/// Go: `io.Discard`.
struct Discard;

impl Write for Discard {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Go: `commands.rootCommand`.
pub struct RootCommand {
    pub logger: Logger,
    pub config_version_id: AtomicI32,
    /// Go: `commonConfigs *lazycache.Cache[configKey, *commonConfig]`.
    pub common_configs: Mutex<Vec<(ConfigKey, Arc<CommonConfig>)>>,
    /// Go: `hugoSites *lazycache.Cache[configKey, *hugolib.HugoSites]` (the built sites; a
    /// one-shot build creates them once).
    pub hugo_sites: Mutex<Vec<(ConfigKey, Arc<HugoSites>)>>,

    // Flags.
    pub source: String,
    pub build_watch: bool,
    pub environment: String,
    pub base_url: String,
    pub gc: bool,
    pub poll: String,
    pub force_sync_static: bool,
    pub cpuprofile: String,
    pub memprofile: String,
    pub mutexprofile: String,
    pub traceprofile: String,
    pub printm: bool,
    pub log_level: String,
    pub quiet: bool,
    pub dev_mode: bool,
    pub render_to_memory: bool,
    pub cfg_file: String,
    pub cfg_dir: String,
    /// Go: `--clock` (persistent string flag copied into config key `internal.clock` by
    /// `flagsToCfg`).
    pub clock: String,
    pub flags: Flags,
    /// The executed command's parsed flag set (Go `cd.CobraCommand.Flags()`).
    pub flag_set: FlagSet,

    /// Go `r.StdOut` / `r.StdErr` (`io.Discard` with `--quiet`).
    pub std_out: LogSink,
    pub std_err: LogSink,
    /// The writer of Go's `log` package (main's `Error: ...` line): stderr until `PreRun`
    /// calls `log.SetOutput(r.StdOut)`.
    pub log_output: LogSink,
    /// The process's stdout/stderr and environment (see [`ExecOptions`]).
    pub opts: ExecOptions,
}

/// An error of [`execute`]: Go's `*simplecobra.CommandError` (a user error: unknown command or
/// flag, invalid flag value, a failing `PreRun`) or an error of the command's `Run`.
#[derive(Debug)]
pub enum ExecError {
    Command(String),
    Run(Error),
}

impl std::fmt::Display for ExecError {
    // Go: simplecobra simplecobra.go:(*CommandError).Error
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::Command(m) => write!(f, "command error: {m}"),
            ExecError::Run(e) => write!(f, "{e}"),
        }
    }
}

/// Entry point used by `main.rs`. Returns the process exit code.
/// Go: commands/commandeer.go:Execute (+ main.go:main printing the error and `os.Exit(1)`).
// Go: commands/commandeer.go:Execute
pub fn execute(args: Vec<String>) -> i32 {
    execute_with(args, ExecOptions::default())
}

/// [`execute`] with explicit stdout/stderr, working dir, environment and func map factory.
// Go: main.go:main
pub fn execute_with(args: Vec<String>, opts: ExecOptions) -> i32 {
    let (res, log_output) = execute_inner(args, opts);
    match res {
        Ok(()) => 0,
        Err(err) => {
            // Go: log.Fatalf("Error: %s", err) with log.SetFlags(0); the log package writes to
            // stderr, or to r.StdOut once the root command's PreRun ran (`log.SetOutput`).
            // log.Output appends a newline only when the message has none.
            let mut msg = format!("Error: {err}");
            if !msg.ends_with('\n') {
                msg.push('\n');
            }
            let mut w = log_output.lock().unwrap_or_else(|e| e.into_inner());
            let _ = w.write_all(msg.as_bytes());
            1
        }
    }
}

/// Go: `commands.Execute(args)`: the error main prints. A command error prints the command's
/// help to stdout first (Go `cd.CobraCommand.Help()` + a blank line).
// Go: commands/commandeer.go:Execute
pub fn execute_err(args: Vec<String>, opts: ExecOptions) -> std::result::Result<(), ExecError> {
    execute_inner(args, opts).0
}

/// [`execute_err`] and the writer of Go's `log` package afterwards.
fn execute_inner(
    args: Vec<String>,
    opts: ExecOptions,
) -> (std::result::Result<(), ExecError>, LogSink) {
    let args = map_legacy_args(args);
    let stdout = opts.stdout.clone();
    let stderr = opts.stderr.clone();
    let res = crate::commands::new_exec_with(&args, opts);
    let (mut r, cmd, tree, path, all_args) = match res {
        Ok(v) => v,
        Err(err) => {
            if let ExecError::Command(_) = &err {
                print_help(&stdout, None);
                let _ = writeln!(stdout.lock().unwrap_or_else(|e| e.into_inner()));
            }
            return (Err(err), stderr);
        }
    };
    let res = r.run_command(&cmd).and_then(|()| {
        // Go: simplecobra's checkArgs runs after the command.
        match crate::cobra::check_args(&tree, &path, &all_args) {
            Some(e) => Err(ExecError::Command(e)),
            None => Ok(()),
        }
    });
    r.close();
    if let Err(ExecError::Command(_)) = &res {
        print_help(&stdout, Some(&cmd));
        let _ = writeln!(stdout.lock().unwrap_or_else(|e| e.into_inner()));
    }
    (res, r.log_output.clone())
}

/// The help text (Go: cobra's help template; not ported, stdout only).
pub(crate) fn print_help(stdout: &LogSink, cmd: Option<&crate::commands::Command>) {
    let mut w = stdout.lock().unwrap_or_else(|e| e.into_inner());
    let name = match cmd {
        Some(crate::commands::Command::Config { .. }) => "neohugo config",
        Some(crate::commands::Command::ConfigMounts) => "neohugo config mounts",
        _ => "neohugo",
    };
    let _ = writeln!(
        w,
        "Usage:\n  {name} [flags]\n\nneohugo-rs builds a Hugo site (commands: build, version, env, config, config mounts).\nRun the Go neohugo binary for the full help text."
    );
}

/// Go: commands/commandeer.go:mapLegacyArgs
// Go: commands/commandeer.go:mapLegacyArgs
pub fn map_legacy_args(mut args: Vec<String>) -> Vec<String> {
    if args.len() > 1
        && args[0] == "new"
        && !["site", "theme", "content"].contains(&args[1].as_str())
    {
        // Insert "content" as the second argument
        args.insert(1, "content".to_string());
    }
    args
}

impl RootCommand {
    /// A root command with the flag defaults of `initRootCommand` and the given options.
    pub fn new(opts: ExecOptions) -> RootCommand {
        RootCommand {
            logger: Logger::new_default(),
            config_version_id: AtomicI32::new(0),
            common_configs: Mutex::new(Vec::new()),
            hugo_sites: Mutex::new(Vec::new()),
            source: String::new(),
            build_watch: false,
            environment: String::new(),
            base_url: String::new(),
            gc: false,
            poll: String::new(),
            force_sync_static: false,
            cpuprofile: String::new(),
            memprofile: String::new(),
            mutexprofile: String::new(),
            traceprofile: String::new(),
            printm: false,
            log_level: String::new(),
            quiet: false,
            dev_mode: false,
            render_to_memory: false,
            cfg_file: String::new(),
            cfg_dir: "config".to_string(),
            clock: String::new(),
            flags: Flags::default(),
            flag_set: FlagSet::new(),
            std_out: opts.stdout.clone(),
            std_err: opts.stderr.clone(),
            log_output: opts.stderr.clone(),
            opts,
        }
    }

    /// Binds the parsed flags to the root command's fields (Go: the `StringVarP(&r.source, ...)`
    /// bindings of `initRootCommand` / `applyLocalFlagsBuild*`).
    pub fn bind_flags(&mut self, fs: &FlagSet) {
        let s = |n: &str, d: &str| -> String {
            match fs.lookup(n) {
                Some(_) => fs.get_string(n),
                None => d.to_string(),
            }
        };
        self.source = s("source", "");
        self.environment = s("environment", "");
        self.cfg_file = s("config", "");
        self.cfg_dir = s("configDir", "config");
        self.log_level = s("logLevel", "");
        self.clock = s("clock", "");
        self.base_url = s("baseURL", "");
        self.poll = s("poll", "");
        self.cpuprofile = s("profile-cpu", "");
        self.memprofile = s("profile-mem", "");
        self.mutexprofile = s("profile-mutex", "");
        self.traceprofile = s("trace", "");
        self.quiet = fs.get_bool("quiet");
        self.render_to_memory = fs.get_bool("renderToMemory");
        self.dev_mode = fs.get_bool("devMode");
        self.build_watch = fs.get_bool("watch");
        self.gc = fs.get_bool("gc");
        self.force_sync_static = fs.get_bool("forceSyncStatic");
        self.printm = fs.get_bool("printMemoryUsage");
        self.flags = Flags::from_flag_set(fs);
        self.flag_set = fs.clone();
    }

    /// Go: `os.Getenv(key)` of the command's environment.
    pub fn getenv(&self, key: &str) -> String {
        match &self.opts.environ {
            Some(env) => env
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .unwrap_or_default(),
            None => std::env::var(key).unwrap_or_default(),
        }
    }

    /// Go: `os.Getwd()`.
    pub fn getwd(&self) -> String {
        match &self.opts.cwd {
            Some(d) => d.clone(),
            None => std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }

    /// Go: `filepath.Abs(path)`.
    pub fn abs(&self, path: &str) -> String {
        if go_path::filepath::is_abs(path) {
            return go_path::filepath::clean(path);
        }
        go_path::filepath::join(&[self.getwd().as_str(), path])
    }

    /// Go: `r.Printf` / `r.Println` (nothing with `--quiet`).
    pub fn print(&self, s: &str) {
        if !self.quiet {
            let mut w = self.std_out.lock().unwrap_or_else(|e| e.into_inner());
            let _ = w.write_all(s.as_bytes());
        }
    }

    /// Go: `(*rootCommand).isVerbose`.
    // Go: commands/commandeer.go:(*rootCommand).isVerbose
    pub fn is_verbose(&self) -> bool {
        self.logger.level() <= Level::Info
    }

    /// Go: `(*rootCommand).Close`.
    // Go: commands/commandeer.go:(*rootCommand).Close
    pub fn close(&self) {
        self.hugo_sites
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Runs the parsed command: simplecobra's `PreRun` of the root (and of the command), then
    /// its `Run`.
    pub fn run_command(
        &mut self,
        cmd: &crate::commands::Command,
    ) -> std::result::Result<(), ExecError> {
        use crate::commands::Command;
        if let Command::Unsupported(name) = cmd {
            return Err(ExecError::Command(format!(
                "neohugo-rs: the {} command is not supported",
                go_strconv::quote(name)
            )));
        }
        if let Command::Help(_) = cmd {
            print_help(&self.opts.stdout, None);
            return Ok(());
        }
        // PreRun errors are cobra errors (simplecobra's wrapErr: not a runErr).
        self.pre_run()
            .map_err(|e| ExecError::Command(e.to_string()))?;
        let res = match cmd {
            Command::Build => self.run(&[]),
            Command::Version => crate::env::run_version_with(self),
            Command::Env => crate::env::run_env_with(self),
            Command::Config {
                format,
                lang,
                print_zero,
            } => crate::config::ConfigCommand {
                format: format.clone(),
                lang: lang.clone(),
                print_zero: *print_zero,
            }
            .run(self),
            Command::ConfigMounts => crate::config::ConfigMountsCommand.run(self),
            Command::Help(_) | Command::Unsupported(_) => Ok(()),
        };
        res.map_err(ExecError::Run)
    }

    /// Go: `(*rootCommand).Build`.
    // Go: commands/commandeer.go:(*rootCommand).Build
    pub fn build(&self, bcfg: BuildCfg, cfg: Arc<dyn Provider>) -> Result<Arc<HugoSites>> {
        let conf = self.config_from_provider(
            ConfigKey {
                counter: self.config_version_id.load(Ordering::SeqCst),
                ignore_modules_does_not_exists: false,
            },
            cfg,
        )?;
        let h = self.new_hugo_sites(&conf)?;
        let h = nh_hugolib::hugo_sites_build::build(h, bcfg)?;
        self.hugo_sites
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((ConfigKey::default(), h.clone()));
        Ok(h)
    }

    /// Go: `(*rootCommand).ConfigFromProvider` — `allconfig.LoadConfig` with the flags provider,
    /// then `htime.Clock = clocks.Start(configs.Base.C.Clock)` when `--clock` is set (a clock that
    /// starts at the given instant and advances with wall time: `nh_common::htime::StartClock`),
    /// `hugofs.NewFrom`, `--renderToMemory`, `publishDir` creation.
    // Go: commands/commandeer.go:(*rootCommand).ConfigFromProvider
    pub fn config_from_provider(
        &self,
        key: ConfigKey,
        cfg: Arc<dyn Provider>,
    ) -> Result<Arc<CommonConfig>> {
        if let Some((_, cc)) = self
            .common_configs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|(k, _)| *k == key)
        {
            return Ok(cc.clone());
        }

        let dir = if !self.source.is_empty() {
            self.abs(&self.source)
        } else {
            self.getwd()
        };

        if !cfg.is_set("workingDir") {
            cfg.set("workingDir", Value::string(dir.as_str()));
        } else {
            let wd = cfg.get_string("workingDir");
            std::fs::create_dir_all(&wd).map_err(|e| {
                Error::new(format!(
                    "failed to create workingDir: {}",
                    nh_hugofs::oserror::from_io("mkdir", &wd, &e)
                ))
            })?;
        }

        // Load the config first to allow publishDir to be configured in config file.
        let configs = load_config(ConfigSourceDescriptor {
            flags: Some(cfg.clone()),
            filename: self.cfg_file.clone(),
            config_dir: self.cfg_dir.clone(),
            environment: self.environment.clone(),
            environ: self.environ_list(),
            ignore_module_does_not_exist: key.ignore_modules_does_not_exists,
            working_dir: String::new(),
            fs: None,
            logger: Some(self.logger.clone()),
            getenv: Some(self.getenv_fn()),
        })?;

        let base = configs.base.clone();
        let publish_dir = base.root.common_dirs.publish_dir.clone();
        cfg.set("publishDir", Value::string(publish_dir.as_str()));
        cfg.set("publishDirStatic", Value::string(publish_dir.as_str()));
        cfg.set("publishDirDynamic", Value::string(publish_dir.as_str()));

        let render_static_to_disk = cfg.get_bool("renderStaticToDisk");

        let source_fs = nh_hugofs::fs::os();
        let destination_fs: Arc<dyn AferoFs> = if cfg.get_bool("renderToMemory") {
            if render_static_to_disk {
                // Hybrid, render dynamic content to Root.
                cfg.set("publishDirDynamic", Value::string("/"));
            } else {
                // Rendering to memoryFS, publish to Root regardless of publishDir.
                cfg.set("publishDirDynamic", Value::string("/"));
                cfg.set("publishDirStatic", Value::string("/"));
            }
            nh_hugofs::afero::new_mem_map_fs()
        } else {
            nh_hugofs::fs::os()
        };

        let fs = nh_hugofs::fs::new_from_source_and_destination(source_fs, destination_fs, &*cfg);

        if render_static_to_disk {
            // Go: the server's `--renderStaticToDisk` (an overlayfs of the dynamic and static
            // publish dirs).
            return Err(Error::new(
                "neohugo-rs: renderStaticToDisk is not supported",
            ));
        }

        if let Some(c) = &base.c
            && !c.clock.is_zero()
        {
            nh_common::htime::set_clock(Arc::new(nh_common::htime::StartClock::new(&c.clock)));
        }

        // Go: `if base.PrintPathWarnings { fs.PublishDir = hugofs.NewCreateCountingFs(fs.PublishDir) }`
        // — the duplicate target path counter is not ported (nh-hugofs has no CreateCountingFs;
        // T24's printPathWarningsOnce reports nothing).

        let common_config = Arc::new(CommonConfig {
            configs: Arc::new(configs),
            cfg,
            fs: Arc::new(fs),
        });
        self.common_configs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((key, common_config.clone()));
        Ok(common_config)
    }

    /// `ConfigSourceDescriptor.Environ` (`KEY=value` pairs; empty = the process's).
    fn environ_list(&self) -> Vec<String> {
        match &self.opts.environ {
            Some(env) => env.iter().map(|(k, v)| format!("{k}={v}")).collect(),
            None => Vec::new(),
        }
    }

    fn getenv_fn(&self) -> Getenv {
        match &self.opts.environ {
            Some(env) => {
                let env = env.clone();
                Arc::new(move |k: &str| {
                    env.iter()
                        .find(|(kk, _)| kk == k)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default()
                })
            }
            None => Arc::new(|k: &str| std::env::var(k).unwrap_or_default()),
        }
    }

    /// Go: `(*rootCommand).HugFromConfig` + `newDepsConfig` + the logger `hugolib.NewHugoSites`
    /// creates from `DepsCfg{LogLevel, StdOut, StdErr}` (DistinctLevel warn, the `ignoreLogs`
    /// statements, errors stored when watching). Returns the unbuilt sites: `build` consumes
    /// them (Go keeps one pointer).
    // Go: commands/commandeer.go:(*rootCommand).HugFromConfig
    pub fn new_hugo_sites(&self, conf: &CommonConfig) -> Result<HugoSites> {
        let first = conf.configs.get_first_language_config();
        let log = Logger::with_options(Options {
            level: self.logger.level(),
            std_out: Some(self.std_out.clone()),
            std_err: Some(self.std_err.clone()),
            distinct_level: Some(Level::Warn),
            store_errors: first.watching(),
            suppress_statements: first.ignored_logs(),
        });
        HugoSites::new(NewHugoSitesCfg {
            configs: conf.configs.clone(),
            fs: (*conf.fs).clone(),
            log,
            func_map_factory: self.opts.func_map_factory.clone(),
        })
    }

    /// Go: `(*rootCommand).getOrCreateHugo`.
    // Go: commands/commandeer.go:(*rootCommand).getOrCreateHugo
    pub fn get_or_create_hugo(
        &self,
        cfg: Arc<dyn Provider>,
        ignore_module_does_not_exist: bool,
    ) -> Result<Arc<HugoSites>> {
        let k = ConfigKey {
            counter: self.config_version_id.load(Ordering::SeqCst),
            ignore_modules_does_not_exists: ignore_module_does_not_exist,
        };
        if let Some((_, h)) = self
            .hugo_sites
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|(kk, _)| *kk == k)
        {
            return Ok(h.clone());
        }
        let conf = self.config_from_provider(k, cfg)?;
        let h = self.new_hugo_sites(&conf)?;
        let h = nh_hugolib::hugo_sites_build::build(h, BuildCfg::default())?;
        self.hugo_sites
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((k, h.clone()));
        Ok(h)
    }

    /// Go: `(*rootCommand).Run` — build, copy static, print build stats table (stdout text is
    /// not part of the parity check, but keep it faithful).
    // Go: commands/commandeer.go:(*rootCommand).Run
    pub fn run(&mut self, _args: &[String]) -> Result<()> {
        if self.build_watch {
            return Err(Error::new(
                "neohugo-rs: watch mode (-w/--watch) is not supported",
            ));
        }
        let b = crate::hugobuilder::HugoBuilder::new();
        let start = std::time::Instant::now();
        let res = (|| -> Result<()> {
            b.load_config(self, false)?;
            b.build(self)
        })();
        // Go: `defer b.postBuild("Total", time.Now())`.
        b.post_build(self, "Total", start);
        res
    }

    /// Go: `(*rootCommand).PreRun` — logger creation, `--quiet`/`--logLevel`.
    // Go: commands/commandeer.go:(*rootCommand).PreRun
    pub fn pre_run(&mut self) -> Result<()> {
        self.std_out = self.opts.stdout.clone();
        self.std_err = self.opts.stderr.clone();
        if self.quiet {
            self.std_out = Arc::new(Mutex::new(Discard));
            self.std_err = Arc::new(Mutex::new(Discard));
        }
        // Used by mkcert (server).
        self.log_output = self.std_out.clone();
        self.logger = self.create_logger(false)?;
        // Set up the global logger early to allow info deprecations during config load.
        nh_common::loggers::set_global_logger(self.logger.clone());
        Ok(())
    }

    /// Go: `(*rootCommand).createLogger`.
    // Go: commands/commandeer.go:(*rootCommand).createLogger
    pub fn create_logger(&self, running: bool) -> Result<Logger> {
        let mut level = Level::Warn;
        if self.dev_mode {
            level = Level::Trace;
        } else if !self.log_level.is_empty() {
            level = match go_unicode::strings::to_lower_str(&self.log_level).as_ref() {
                "debug" => Level::Debug,
                "info" => Level::Info,
                "warn" | "warning" => Level::Warn,
                "error" => Level::Error,
                _ => {
                    return Err(Error::new(format!(
                        "invalid log level: {}, must be one of debug, warn, info or error",
                        go_strconv::quote(&self.log_level)
                    )));
                }
            };
        }
        Ok(Logger::with_options(Options {
            distinct_level: Some(Level::Warn),
            level,
            std_out: Some(self.std_out.clone()),
            std_err: Some(self.std_err.clone()),
            store_errors: running,
            ..Default::default()
        }))
    }

    /// Go: `(*rootCommand).timeTrack`.
    // Go: commands/commandeer.go:(*rootCommand).timeTrack
    pub fn time_track(&self, start: std::time::Instant, name: &str) {
        let elapsed = start.elapsed();
        self.print(&format!(
            "{} in {} ms\n",
            name,
            (1000.0 * elapsed.as_secs_f64()) as i64
        ));
    }
}

/// Go: `initRootCommand(subCommandName, cd)` — the persistent flags of the root (and of the
/// `build` command, which declares them again) and the local build flags.
// Go: commands/commandeer.go:(*rootCommand).initRootCommand
pub fn init_root_command(cmd: &mut crate::cobra::CobraCommand) {
    let p = &mut cmd.persistent_flags;
    p.string_p(
        "source",
        "s",
        "",
        "filesystem path to read files relative from",
    );
    p.string_p("destination", "d", "", "filesystem path to write files to");
    p.string_p("environment", "e", "", "build environment");
    p.string_p("themesDir", "", "", "filesystem path to themes directory");
    p.string_p(
        "ignoreVendorPaths",
        "",
        "",
        "ignores any _vendor for module paths matching the given Glob pattern",
    );
    p.bool_p(
        "noBuildLock",
        "",
        false,
        "don't create .hugo_build.lock file",
    );
    p.string_p(
        "clock",
        "",
        "",
        "set the clock used by Hugo, e.g. --clock 2021-11-06T22:30:00.00+09:00",
    );
    p.string_p(
        "config",
        "",
        "",
        "config file (default is hugo.yaml|json|toml)",
    );
    p.string_p("configDir", "", "config", "config dir");
    p.bool_p("quiet", "", false, "build in quiet mode");
    p.bool_p(
        "renderToMemory",
        "M",
        false,
        "render to memory (mostly useful when running the server)",
    );
    p.bool_p(
        "devMode",
        "",
        false,
        "only used for internal testing, flag hidden.",
    );
    p.string_p("logLevel", "", "", "log level (debug|info|warn|error)");
    cmd.local_flags.bool_p(
        "watch",
        "w",
        false,
        "watch filesystem for changes and recreate as needed",
    );
    cmd.persistent_flags.mark_hidden("devMode");

    // Configure local flags
    apply_local_flags_build(&mut cmd.local_flags);
}

/// Go: `applyLocalFlagsBuildConfig` — a sub set of the complete build flags (also used by
/// `config`, `config mounts`, `new` and `mod`).
// Go: commands/commandeer.go:applyLocalFlagsBuildConfig
pub fn apply_local_flags_build_config(f: &mut FlagSet) {
    f.string_slice_p(
        "theme",
        "t",
        &[],
        "themes to use (located in /themes/THEMENAME/)",
    );
    f.string_p(
        "baseURL",
        "b",
        "",
        "hostname (and path) to the root, e.g. https://spf13.com/",
    );
    f.string_p("cacheDir", "", "", "filesystem path to cache directory");
    f.string_p(
        "contentDir",
        "c",
        "",
        "filesystem path to content directory",
    );
    f.string_slice_p(
        "renderSegments",
        "",
        &[],
        "named segments to render (configured in the segments config)",
    );
}

/// Go: `applyLocalFlagsBuild` — flags needed to do a build.
// Go: commands/commandeer.go:applyLocalFlagsBuild
pub fn apply_local_flags_build(f: &mut FlagSet) {
    apply_local_flags_build_config(f);
    f.bool_p(
        "cleanDestinationDir",
        "",
        false,
        "remove files from destination not found in static directories",
    );
    f.bool_p("buildDrafts", "D", false, "include content marked as draft");
    f.bool_p(
        "buildFuture",
        "F",
        false,
        "include content with publishdate in the future",
    );
    f.bool_p("buildExpired", "E", false, "include expired content");
    f.bool_p("ignoreCache", "", false, "ignores the cache directory");
    f.bool_p(
        "enableGitInfo",
        "",
        false,
        "add Git revision, date, author, and CODEOWNERS info to the pages",
    );
    f.string_p("layoutDir", "l", "", "filesystem path to layout directory");
    f.bool_p(
        "gc",
        "",
        false,
        "enable to run some cleanup tasks (remove unused cache files) after the build",
    );
    f.string_p(
        "poll",
        "",
        "",
        "set this to a poll interval, e.g --poll 700ms, to use a poll based approach to watch for file system changes",
    );
    f.bool_p("panicOnWarning", "", false, "panic on first WARNING log");
    f.bool_p(
        "templateMetrics",
        "",
        false,
        "display metrics about template executions",
    );
    f.bool_p(
        "templateMetricsHints",
        "",
        false,
        "calculate some improvement hints when combined with --templateMetrics",
    );
    f.bool_p(
        "forceSyncStatic",
        "",
        false,
        "copy all files when static is changed.",
    );
    f.bool_p(
        "noTimes",
        "",
        false,
        "don't sync modification time of files",
    );
    f.bool_p("noChmod", "", false, "don't sync permission mode of files");
    f.bool_p("printI18nWarnings", "", false, "print missing translations");
    f.bool_p(
        "printPathWarnings",
        "",
        false,
        "print warnings on duplicate target paths etc.",
    );
    f.bool_p(
        "printUnusedTemplates",
        "",
        false,
        "print warnings on unused templates.",
    );
    f.string_p("profile-cpu", "", "", "write cpu profile to `file`");
    f.string_p("profile-mem", "", "", "write memory profile to `file`");
    f.bool_p(
        "printMemoryUsage",
        "",
        false,
        "print memory usage to screen at intervals",
    );
    f.string_p("profile-mutex", "", "", "write Mutex profile to `file`");
    f.string_p(
        "trace",
        "",
        "",
        "write trace to `file` (not useful in general)",
    );

    // Hide these for now.
    f.mark_hidden("profile-cpu");
    f.mark_hidden("profile-mem");
    f.mark_hidden("profile-mutex");

    f.string_slice_p(
        "disableKinds",
        "",
        &[],
        "disable different kind of pages (home, RSS etc.)",
    );
    f.bool_p(
        "minify",
        "",
        false,
        "minify any supported output format (HTML, XML etc.)",
    );
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/commandeer.go (679 lines; 20/28 funcs executed)
//   types: commonConfig, configKey, rootCommand, simpleCommand
// OK L60-88: Execute(args []string) error
// OK L153-155: (r *rootCommand) isVerbose() bool
// OK L157-167: (r *rootCommand) Close() error
// OK L169-179: (r *rootCommand) Build(cd *simplecobra.Commandeer, bcfg hugolib.BuildCfg, cfg config.Provider) (*hugolib.HugoSites, error)
// OK L181-183: (r *rootCommand) Commands() []simplecobra.Commander (commands::new_tree)
//    L185-217: (r *rootCommand) ConfigFromConfig(key configKey, oldConf *commonConfig) (*commonConfig, error) (server reloads; not ported)
// OK L219-331: (r *rootCommand) ConfigFromProvider(key configKey, cfg config.Provider) (*commonConfig, error)
// OK L333-340: (r *rootCommand) HugFromConfig(conf *commonConfig) (*hugolib.HugoSites, error) (new_hugo_sites)
// OK L342-344: (r *rootCommand) Neohugo(cfg config.Provider) (*hugolib.HugoSites, error) (get_or_create_hugo)
// OK L346-357: (r *rootCommand) getOrCreateHugo(cfg config.Provider, ignoreModuleDoesNotExist bool) (*hugolib.HugoSites, error)
// OK L359-361: (r *rootCommand) newDepsConfig(conf *commonConfig) deps.DepsCfg (new_hugo_sites)
// OK L363-365: (r *rootCommand) Name() string
// OK L367-422: (r *rootCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error (watch: unsupported)
// OK L424-466: (r *rootCommand) PreRun(cd, runner *simplecobra.Commandeer) error
// OK L468-499: (r *rootCommand) createLogger(running bool) (loggers.Logger, error)
//    L501-504: (r *rootCommand) resetLogs() (server only)
//    L507-509: (r *rootCommand) IsTestRun() bool (server only)
// OK L511-513: (r *rootCommand) Init(cd *simplecobra.Commandeer) error
// OK L515-566: (r *rootCommand) initRootCommand(subCommandName string, cd *simplecobra.Commandeer) error
// OK L569-577: applyLocalFlagsBuildConfig(cmd *cobra.Command, r *rootCommand)
// OK L580-616: applyLocalFlagsBuild(cmd *cobra.Command, r *rootCommand)
// OK L618-621: (r *rootCommand) timeTrack(start time.Time, name string)
// OK L637-639: (c *simpleCommand) Commands() []simplecobra.Commander
// OK L641-643: (c *simpleCommand) Name() string
// OK L645-650: (c *simpleCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error
// OK L652-664: (c *simpleCommand) Init(cd *simplecobra.Commandeer) error
// OK L666-671: (c *simpleCommand) PreRun(cd, runner *simplecobra.Commandeer) error
// OK L673-679: mapLegacyArgs(args []string) []string
// ---------------------------------------------------------------------------
