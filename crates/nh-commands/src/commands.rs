//! Port of `commands/commands.go`.
//!
//! Owner: Wave B task T25 (commands-cli).

//!
//! Go wires commands through `simplecobra`. The port builds the same cobra command tree
//! (module `cobra`) with every Go command name, so a command line resolves to the same command
//! as in Go; only the commands in scope are implemented (see `commandeer`).

use nh_common::Result;
use nh_common::herrors::Error;

use crate::cobra::{CmdKind, CmdPath, CobraCommand, Tree};
use crate::commandeer::{ExecError, ExecOptions, RootCommand};

/// The sub-command selected on the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `neohugo` / `neohugo build` (Go: rootCommand, hugoBuildCommand).
    Build,
    /// `neohugo version`.
    Version,
    /// `neohugo env`.
    Env,
    /// `neohugo config` (`--format toml|yaml|json`, `--lang`, `--printZero`).
    Config {
        format: String,
        lang: String,
        print_zero: bool,
    },
    /// `neohugo config mounts`.
    ConfigMounts,
    /// `--help` / `-h` / `neohugo help [command]` (cobra's help; the text is not ported).
    Help(String),
    /// Any other Go command name (the full command path): rejected with a user error.
    Unsupported(String),
}

/// Go: commands/commands.go:newExec — parse `args` into the root command + selected sub-command.
// Go: commands/commands.go:newExec
pub fn new_exec(args: &[String]) -> Result<(RootCommand, Command)> {
    match new_exec_with(args, ExecOptions::default()) {
        Ok((r, c, ..)) => Ok((r, c)),
        Err(e) => Err(Error::new(e.to_string())),
    }
}

/// The result of [`new_exec_with`]: the root command with the executed command's flags bound,
/// the command, the tree, the command's path in it and the full argument list (for
/// simplecobra's `checkArgs`).
pub type ExecParts = (RootCommand, Command, Tree, CmdPath, Vec<String>);

/// [`new_exec`] with explicit [`ExecOptions`]; parse errors are Go's `CommandError`s.
pub fn new_exec_with(
    args: &[String],
    opts: ExecOptions,
) -> std::result::Result<ExecParts, ExecError> {
    let tree = new_tree();
    // The commands the port does not support: their flags are not declared, so they are
    // rejected as soon as cobra's Find resolves them.
    let (path, _, find_err) = tree.find(args);
    if find_err.is_none() && tree.get(&path).kind == CmdKind::Unsupported {
        let cmd = Command::Unsupported(tree.command_path(&path));
        return Ok((RootCommand::new(opts), cmd, tree, path, args.to_vec()));
    }
    let parsed = match crate::cobra::parse(&tree, args) {
        Ok(p) => p,
        Err((_, msg)) => return Err(ExecError::Command(msg)),
    };
    let mut r = RootCommand::new(opts);
    r.bind_flags(&parsed.flags);
    let c = tree.get(&parsed.path);
    let cmd = if parsed.help {
        Command::Help(tree.command_path(&parsed.path))
    } else {
        match c.kind {
            CmdKind::Build => Command::Build,
            CmdKind::Version => Command::Version,
            CmdKind::Env => Command::Env,
            CmdKind::Config => Command::Config {
                format: parsed.flags.get_string("format"),
                lang: parsed.flags.get_string("lang"),
                print_zero: parsed.flags.get_bool("printZero"),
            },
            CmdKind::ConfigMounts => Command::ConfigMounts,
            CmdKind::Help => Command::Help(tree.command_path(&parsed.path)),
            CmdKind::Unsupported => Command::Unsupported(tree.command_path(&parsed.path)),
        }
    };
    Ok((r, cmd, tree, parsed.path, args.to_vec()))
}

/// The cobra command tree of `newExec` (Go: `simplecobra.New(rootCmd)` + each command's `Init`),
/// with cobra's `help` and `completion` commands (added by `ExecuteC`) at the end.
pub fn new_tree() -> Tree {
    let mut root = CobraCommand::new("neohugo", "Build your site", CmdKind::Build);
    crate::commandeer::init_root_command(&mut root);

    let u = |name: &str| CobraCommand::new(name, "", CmdKind::Unsupported);
    let us = |name: &str, subs: &[&str]| {
        CobraCommand::new(name, "", CmdKind::Unsupported)
            .with_commands(subs.iter().map(|s| u(s)).collect())
    };

    root.commands = vec![
        new_hugo_build_cmd(),
        CobraCommand::new("version", "Display version", CmdKind::Version),
        CobraCommand::new("env", "Display version and environment info", CmdKind::Env),
        {
            let mut s = us("server", &["trust"]);
            s.aliases = vec!["serve".to_string()];
            s
        },
        u("deploy"),
        crate::config::new_config_command(),
        us("new", &["content", "site", "theme"]),
        us("convert", &["toJSON", "toTOML", "toYAML"]),
        us("import", &["jekyll"]),
        us("list", &["drafts", "future", "expired", "all", "published"]),
        {
            let mut m = us(
                "mod",
                &["init", "verify", "graph", "clean", "tidy", "vendor", "get"],
            );
            m.commands.insert(0, us("npm", &["pack"]));
            m
        },
        {
            let mut g = us("gen", &["chromastyles", "man", "doc", "docshelper"]);
            g.commands[3].hidden = true;
            g
        },
        {
            let mut r = u("release");
            r.hidden = true;
            r
        },
        CobraCommand::new("help", "Help about any command", CmdKind::Help),
        {
            let mut c = us("completion", &["bash", "fish", "powershell", "zsh"]);
            c.has_args_validator = true;
            c
        },
    ];
    Tree { root }
}

/// Go: `newHugoBuildCmd()` — `build` declares the root command's flags again.
// Go: commands/commands.go:newHugoBuildCmd
fn new_hugo_build_cmd() -> CobraCommand {
    let mut c = CobraCommand::new("build", "Build your site", CmdKind::Build);
    // Go: (*hugoBuildCommand).Init -> initRootCommand("build", cd).
    crate::commandeer::init_root_command(&mut c);
    c
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/commands.go (73 lines; 5/7 funcs executed)
//   types: hugoBuildCommand
// OK L23-43: newExec() (*simplecobra.Exec, error)
// OK L45-47: newHugoBuildCmd() simplecobra.Commander
// OK L54-56: (c *hugoBuildCommand) Commands() []simplecobra.Commander
// OK L58-60: (c *hugoBuildCommand) Name() string
// OK L62-65: (c *hugoBuildCommand) Init(cd *simplecobra.Commandeer) error
// OK L67-69: (c *hugoBuildCommand) PreRun(cd, runner *simplecobra.Commandeer) error
// OK L71-73: (c *hugoBuildCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error
// ---------------------------------------------------------------------------
