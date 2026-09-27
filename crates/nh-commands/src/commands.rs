//! Port of `commands/commands.go`.
//!
//! Owner: Wave B task T25 (commands-cli).

//!
//! Go wires commands through `simplecobra`. The port uses a plain enum; only the commands in
//! scope are implemented (see `commandeer`).

use nh_common::Result;

/// The sub-command selected on the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `neohugo` / `neohugo build` (Go: rootCommand, hugoBuildCommand).
    Build,
    /// `neohugo version`.
    Version,
    /// `neohugo env`.
    Env,
    /// `neohugo config` (`--format toml|yaml|json`, `--lang`).
    Config { format: String, lang: String },
    /// `neohugo config mounts`.
    ConfigMounts,
    /// Any other Go command name: rejected with a user error.
    Unsupported(String),
}

/// Go: commands/commands.go:newExec — parse `args` into the root command + selected sub-command.
pub fn new_exec(args: &[String]) -> Result<(crate::commandeer::RootCommand, Command)> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/commands.go (73 lines; 5/7 funcs executed)
//   types: hugoBuildCommand
// EX L23-43: newExec() (*simplecobra.Exec, error)
// EX L45-47: newHugoBuildCmd() simplecobra.Commander
// EX L54-56: (c *hugoBuildCommand) Commands() []simplecobra.Commander
// EX L58-60: (c *hugoBuildCommand) Name() string
// EX L62-65: (c *hugoBuildCommand) Init(cd *simplecobra.Commandeer) error
//    L67-69: (c *hugoBuildCommand) PreRun(cd, runner *simplecobra.Commandeer) error
//    L71-73: (c *hugoBuildCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error
// ---------------------------------------------------------------------------
