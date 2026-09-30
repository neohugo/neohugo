//! The command line (clap). Flags are kebab-case; Hugo's camelCase spellings are aliases
//! (`--clean-destination-dir` / `--cleanDestinationDir`, `--base-url` / `--baseURL`).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// neohugo-rs: builds a Hugo site with Tera layouts.
#[derive(Debug, Parser)]
#[command(
    name = "neohugo-rs",
    version = crate::VERSION,
    about,
    args_conflicts_with_subcommands = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
    /// Without a command: `build`.
    #[command(flatten)]
    pub build: BuildArgs,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Builds the site into the publish directory (the default command).
    Build(BuildArgs),
    /// Template tooling.
    #[command(subcommand)]
    Templates(TemplatesCommand),
    /// Prints the resolved configuration.
    Config(ConfigArgs),
    /// Prints the version.
    Version,
}

#[derive(Debug, Subcommand)]
pub enum TemplatesCommand {
    /// Checks the layouts: Tera parse and name errors, context names, lookup coverage and the
    /// conversion lints (REWRITE_PLAN.md §4.8).
    Check(CheckArgs),
}

/// What every command that loads the project takes.
#[derive(Clone, Debug, Default, Args)]
pub struct ProjectArgs {
    /// The project directory (default: the working directory).
    #[arg(short = 's', long, value_name = "DIR")]
    pub source: Option<PathBuf>,
    /// Configuration files, relative to the source (comma-separated; the first wins). Default:
    /// the first of neohugo.{toml,yaml,yml,json}, hugo.*, config.*.
    #[arg(long, value_name = "FILES", value_delimiter = ',')]
    pub config: Vec<PathBuf>,
    /// The configuration directory (default `config`).
    #[arg(long, alias = "configDir", value_name = "DIR")]
    pub config_dir: Option<PathBuf>,
    /// The build environment (default `production`; `HUGO_ENVIRONMENT`, `HUGO_ENV`).
    #[arg(short = 'e', long, value_name = "ENV")]
    pub environment: Option<String>,
    /// The site's base URL.
    #[arg(short = 'b', long, aliases = ["baseURL", "baseUrl"], value_name = "URL")]
    pub base_url: Option<String>,
    /// Themes to use (comma-separated).
    #[arg(short = 't', long, value_delimiter = ',', value_name = "THEMES")]
    pub theme: Vec<String>,
    /// The themes directory.
    #[arg(long, alias = "themesDir", value_name = "DIR")]
    pub themes_dir: Option<PathBuf>,
    /// The cache directory.
    #[arg(long, alias = "cacheDir", value_name = "DIR")]
    pub cache_dir: Option<PathBuf>,
    /// Ignores the cache directory.
    #[arg(long, alias = "ignoreCache")]
    pub ignore_cache: bool,
    #[command(flatten)]
    pub include: IncludeArgs,
    /// The build's "now" (RFC 3339, e.g. `2026-09-27T12:00:00Z`), for dates and `now()`.
    #[arg(long, value_name = "TIME", value_parser = parse_clock)]
    pub clock: Option<jiff::Timestamp>,
}

/// Which content the build policy lets in besides published content.
#[derive(Clone, Copy, Debug, Default, Args)]
pub struct IncludeArgs {
    /// Includes content marked as draft.
    #[arg(short = 'D', long, alias = "buildDrafts")]
    pub build_drafts: bool,
    /// Includes expired content.
    #[arg(short = 'E', long, alias = "buildExpired")]
    pub build_expired: bool,
    /// Includes content with a publish date in the future.
    #[arg(short = 'F', long, alias = "buildFuture")]
    pub build_future: bool,
}

/// `build` (and the command line without a command).
#[derive(Clone, Debug, Default, Args)]
pub struct BuildArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    #[command(flatten)]
    pub output: OutputArgs,
    /// Minifies the supported output formats.
    #[arg(long)]
    pub minify: bool,
    /// The render thread count (default: `RAYON_NUM_THREADS`, else the CPUs). The output does
    /// not depend on it.
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,
    /// Prints only warnings and errors.
    #[arg(short = 'q', long)]
    pub quiet: bool,
}

/// Where a build writes.
#[derive(Clone, Debug, Default, Args)]
pub struct OutputArgs {
    /// The publish directory, relative to the source.
    #[arg(short = 'd', long, value_name = "DIR")]
    pub destination: Option<PathBuf>,
    /// Removes files from the publish directory that the static directories do not have.
    #[arg(long, alias = "cleanDestinationDir")]
    pub clean_destination_dir: bool,
    /// Renders into memory only (a dry run: nothing is written).
    #[arg(short = 'M', long, alias = "renderToMemory")]
    pub render_to_memory: bool,
}

/// `templates check`.
#[derive(Clone, Debug, Default, Args)]
pub struct CheckArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    /// How much of the lookup coverage to print.
    #[arg(long, value_enum, default_value_t = Coverage::Summary)]
    pub coverage: Coverage,
    /// Exits with 1 when there are warnings, too.
    #[arg(long)]
    pub deny_warnings: bool,
}

/// The lookup coverage listing of `templates check`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Coverage {
    /// Templates with their query counts, every miss.
    #[default]
    Summary,
    /// Every (page, format) query, shortcode and hook lookup.
    Full,
    /// No coverage (the site's content is not loaded).
    None,
}

/// `config`.
#[derive(Clone, Debug, Default, Args)]
pub struct ConfigArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    #[arg(long, value_enum, default_value_t = ConfigFormat::Json)]
    pub format: ConfigFormat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ConfigFormat {
    #[default]
    Json,
    Toml,
}

fn parse_clock(s: &str) -> Result<jiff::Timestamp, String> {
    s.parse::<jiff::Timestamp>()
        .map_err(|e| format!("not an RFC 3339 time with an offset: {e}"))
}
