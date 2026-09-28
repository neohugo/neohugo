//! Module `cobra`.
//!
//! NEW: the subset of `github.com/spf13/cobra` v1.9.1 + `github.com/bep/simplecobra` v0.6.0 that
//! resolves a neohugo command line: the command tree, `Find` (`stripFlags`, `argsMinusFirstX`,
//! `findNext`), `legacyArgs` with the "Did you mean this?" suggestions, the flag merging of
//! persistent flags, `ParseFlags` and simplecobra's `checkArgs` / `CommandError`.
//!
//! Owner: Wave B task T25 (commands-cli).
//!
//! Help and usage texts are not ported (stdout only, not part of the parity target): `--help`
//! prints a short summary instead of cobra's usage template.

use crate::pflag::{FlagSet, ParseError};

/// What a command does in the port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdKind {
    /// `neohugo` (root) and `neohugo build`.
    Build,
    Version,
    Env,
    Config,
    ConfigMounts,
    /// cobra's `help [command]`.
    Help,
    /// A Go command the port does not support.
    Unsupported,
}

/// A cobra command (Go: `*cobra.Command` built by simplecobra from a `Commander`).
#[derive(Clone, Debug)]
pub struct CobraCommand {
    /// Go `Name()` (the first word of `Use`).
    pub name: String,
    pub short: String,
    /// Go `Aliases`.
    pub aliases: Vec<String>,
    pub hidden: bool,
    /// Go `Args` is set (`cobra.NoArgs` on `completion`): no `legacyArgs` check.
    pub has_args_validator: bool,
    pub kind: CmdKind,
    /// Go `PersistentFlags()`.
    pub persistent_flags: FlagSet,
    /// Go `LocalFlags` (the flags declared with `Flags()`).
    pub local_flags: FlagSet,
    pub commands: Vec<CobraCommand>,
}

impl CobraCommand {
    pub fn new(name: &str, short: &str, kind: CmdKind) -> Self {
        CobraCommand {
            name: name.to_string(),
            short: short.to_string(),
            aliases: Vec::new(),
            hidden: false,
            has_args_validator: false,
            kind,
            persistent_flags: FlagSet::new(),
            local_flags: FlagSet::new(),
            commands: Vec::new(),
        }
    }

    pub fn with_commands(mut self, cmds: Vec<CobraCommand>) -> Self {
        self.commands = cmds;
        self
    }

    /// Go: `HasSubCommands()`.
    pub fn has_sub_commands(&self) -> bool {
        !self.commands.is_empty()
    }

    /// Go: `IsAvailableCommand()` (every simplecobra command is runnable; the help command of
    /// the root is not available).
    // Go: cobra command.go:(*Command).IsAvailableCommand
    fn is_available_command(&self) -> bool {
        if self.hidden || self.kind == CmdKind::Help {
            return false;
        }
        true
    }
}

/// A resolved command: the path of child indexes from the root.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CmdPath(pub Vec<usize>);

/// The command tree (Go: the root `*cobra.Command`).
#[derive(Clone, Debug)]
pub struct Tree {
    pub root: CobraCommand,
}

impl Tree {
    /// The command at `path`.
    pub fn get(&self, path: &CmdPath) -> &CobraCommand {
        let mut c = &self.root;
        for &i in &path.0 {
            c = &c.commands[i];
        }
        c
    }

    /// Go: `CommandPath()` ("neohugo config mounts").
    // Go: cobra command.go:(*Command).CommandPath
    pub fn command_path(&self, path: &CmdPath) -> String {
        let mut names = vec![self.root.name.clone()];
        let mut c = &self.root;
        for &i in &path.0 {
            c = &c.commands[i];
            names.push(c.name.clone());
        }
        names.join(" ")
    }

    /// Go: `c.Flags()` after `mergePersistentFlags()`: the local flags, then the command's own
    /// persistent flags, then the parents' persistent flags (nearest parent first); a name
    /// already present wins.
    // Go: cobra command.go:(*Command).mergePersistentFlags
    pub fn merged_flags(&self, path: &CmdPath) -> FlagSet {
        let mut chain: Vec<&CobraCommand> = vec![&self.root];
        let mut c = &self.root;
        for &i in &path.0 {
            c = &c.commands[i];
            chain.push(c);
        }
        let cmd = chain[chain.len() - 1];
        let mut fs = cmd.local_flags.clone();
        for f in cmd.persistent_flags.visit_all() {
            fs.add_flag_if_absent(f);
        }
        for parent in chain[..chain.len() - 1].iter().rev() {
            for f in parent.persistent_flags.visit_all() {
                fs.add_flag_if_absent(f);
            }
        }
        fs
    }

    /// Go: `(*Command).Find(args)`: the command and the remaining args, and the `legacyArgs`
    /// error of the found command.
    // Go: cobra command.go:(*Command).Find
    pub fn find(&self, args: &[String]) -> (CmdPath, Vec<String>, Option<String>) {
        let mut path = CmdPath::default();
        let mut inner_args: Vec<String> = args.to_vec();
        loop {
            let args_wo_flags = strip_flags(&inner_args, &self.merged_flags(&path));
            if args_wo_flags.is_empty() {
                break;
            }
            let next_sub_cmd = &args_wo_flags[0];
            let c = self.get(&path);
            match find_next(c, next_sub_cmd) {
                Some(i) => {
                    inner_args =
                        args_minus_first_x(&inner_args, next_sub_cmd, &self.merged_flags(&path));
                    path.0.push(i);
                }
                None => break,
            }
        }
        let found = self.get(&path);
        let err = if !found.has_args_validator {
            legacy_args(
                self,
                &path,
                &strip_flags(&inner_args, &self.merged_flags(&path)),
            )
        } else {
            None
        };
        (path, inner_args, err)
    }

    /// Go: `(*Command).SuggestionsFor(typedName)`.
    // Go: cobra command.go:(*Command).SuggestionsFor
    fn suggestions_for(&self, c: &CobraCommand, typed_name: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        for cmd in &c.commands {
            if cmd.is_available_command() {
                let levenshtein_distance = ld(typed_name, &cmd.name, true);
                let suggest_by_levenshtein = levenshtein_distance <= SUGGESTIONS_MINIMUM_DISTANCE;
                let suggest_by_prefix = go_unicode::strings::to_lower_str(&cmd.name)
                    .starts_with(go_unicode::strings::to_lower_str(typed_name).as_ref());
                if suggest_by_levenshtein || suggest_by_prefix {
                    suggestions.push(cmd.name.clone());
                }
            }
        }
        suggestions
    }

    /// Go: `(*Command).findSuggestions(arg)`.
    // Go: cobra command.go:(*Command).findSuggestions
    pub fn find_suggestions(&self, c: &CobraCommand, arg: &str) -> String {
        let mut sb = String::new();
        let suggestions = self.suggestions_for(c, arg);
        if !suggestions.is_empty() {
            sb.push_str("\n\nDid you mean this?\n");
            for s in suggestions {
                sb.push('\t');
                sb.push_str(&s);
                sb.push('\n');
            }
        }
        sb
    }
}

/// simplecobra sets `SuggestionsMinimumDistance: 2`.
const SUGGESTIONS_MINIMUM_DISTANCE: usize = 2;

// Go: cobra command.go:stripFlags
fn strip_flags(args: &[String], flags: &FlagSet) -> Vec<String> {
    let mut commands = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let s = &args[i];
        i += 1;
        if s == "--" {
            // "--" terminates the flags
            break;
        }
        let long_with_value =
            s.starts_with("--") && !s.contains('=') && !flags.has_no_opt_def_val(&s[2..]);
        let short_with_value = s.starts_with('-')
            && !s.contains('=')
            && s.len() == 2
            && !flags.short_has_no_opt_def_val(&s[1..]);
        if long_with_value || short_with_value {
            // If '--flag arg' / '-f arg' then delete arg from args (or break if it is the last).
            if args.len() - i <= 1 {
                break;
            }
            i += 1;
            continue;
        }
        if !s.is_empty() && !s.starts_with('-') {
            commands.push(s.clone());
        }
    }
    commands
}

// Go: cobra command.go:(*Command).argsMinusFirstX
fn args_minus_first_x(args: &[String], x: &str, flags: &FlagSet) -> Vec<String> {
    let mut pos = 0;
    while pos < args.len() {
        let s = &args[pos];
        if s == "--" {
            break;
        }
        let long_with_value =
            s.starts_with("--") && !s.contains('=') && !flags.has_no_opt_def_val(&s[2..]);
        let short_with_value = s.starts_with('-')
            && !s.contains('=')
            && s.len() == 2
            && !flags.short_has_no_opt_def_val(&s[1..]);
        if long_with_value || short_with_value {
            pos += 2;
            continue;
        }
        if !s.starts_with('-') && s == x {
            let mut ret = Vec::with_capacity(args.len() - 1);
            ret.extend_from_slice(&args[..pos]);
            ret.extend_from_slice(&args[pos + 1..]);
            return ret;
        }
        pos += 1;
    }
    args.to_vec()
}

// Go: cobra command.go:(*Command).findNext
fn find_next(c: &CobraCommand, next: &str) -> Option<usize> {
    c.commands
        .iter()
        .position(|cmd| cmd.name == next || cmd.aliases.iter().any(|a| a == next))
}

// Go: cobra args.go:legacyArgs
fn legacy_args(tree: &Tree, path: &CmdPath, args: &[String]) -> Option<String> {
    let cmd = tree.get(path);
    // no subcommand, always take args
    if !cmd.has_sub_commands() {
        return None;
    }
    // root command with subcommands, do subcommand checking.
    if path.0.is_empty() && !args.is_empty() {
        return Some(format!(
            "unknown command {} for {}{}",
            go_strconv::quote(&args[0]),
            go_strconv::quote(tree.command_path(path)),
            tree.find_suggestions(cmd, &args[0])
        ));
    }
    None
}

/// Go: cobra `ld(s, t, ignoreCase)` — the Levenshtein distance of the byte strings.
// Go: cobra cobra.go:ld
fn ld(s: &str, t: &str, ignore_case: bool) -> usize {
    let (s, t) = if ignore_case {
        (
            go_unicode::strings::to_lower_str(s).into_owned(),
            go_unicode::strings::to_lower_str(t).into_owned(),
        )
    } else {
        (s.to_string(), t.to_string())
    };
    let (s, t) = (s.as_bytes(), t.as_bytes());
    let mut d = vec![vec![0usize; t.len() + 1]; s.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, v) in d[0].iter_mut().enumerate() {
        *v = j;
    }
    for j in 1..=t.len() {
        for i in 1..=s.len() {
            if s[i - 1] == t[j - 1] {
                d[i][j] = d[i - 1][j - 1];
            } else {
                let mut min = d[i - 1][j];
                if d[i][j - 1] < min {
                    min = d[i][j - 1];
                }
                if d[i - 1][j - 1] < min {
                    min = d[i - 1][j - 1];
                }
                d[i][j] = min + 1;
            }
        }
    }
    d[s.len()][t.len()]
}

/// Go: simplecobra `checkArgs(cmd, args)` — cobra only suggests for the root command; this
/// rejects an unknown sub command of a command with sub commands. Go formats the error with
/// `args[1]` whatever the unknown name was.
// Go: simplecobra simplecobra.go:checkArgs
pub fn check_args(tree: &Tree, path: &CmdPath, args: &[String]) -> Option<String> {
    let cmd = tree.get(path);
    // no subcommand, always take args.
    if !cmd.has_sub_commands() {
        return None;
    }
    let mut command_name = "";
    for arg in args {
        if arg.starts_with('-') {
            break;
        }
        command_name = arg;
    }
    if command_name.is_empty() || cmd.name == command_name {
        return None;
    }
    // Also check the aliases.
    if cmd.aliases.iter().any(|a| a == command_name) {
        return None;
    }
    let arg1 = args.get(1).cloned().unwrap_or_default();
    Some(format!(
        "unknown command {} for {}{}",
        go_strconv::quote(&arg1),
        go_strconv::quote(tree.command_path(path)),
        tree.find_suggestions(cmd, command_name)
    ))
}

/// The outcome of parsing a command line (Go: cobra `ExecuteC` up to the `Run` call).
#[derive(Clone, Debug)]
pub struct Parsed {
    pub path: CmdPath,
    /// The found command's merged flags after `ParseFlags`.
    pub flags: FlagSet,
    /// Go `cmd.Flags().Args()`.
    pub positional: Vec<String>,
    /// `--help`/`-h` was given (Go `flag.ErrHelp` from `execute`).
    pub help: bool,
}

/// Go: cobra's `ExecuteC` up to `execute`'s `ParseFlags` + the help flag check: `Find`, then
/// `InitDefaultHelpFlag` and `ParseFlags` on the found command. Errors are the cobra error
/// texts (simplecobra turns them into a `CommandError`); `Err((path, msg))` keeps the command
/// cobra reports on.
// Go: cobra command.go:(*Command).ExecuteC
pub fn parse(tree: &Tree, args: &[String]) -> Result<Parsed, (CmdPath, String)> {
    let (path, flag_args, err) = tree.find(args);
    if let Some(err) = err {
        return Err((path, err));
    }
    let mut flags = tree.merged_flags(&path);
    // Go: InitDefaultHelpFlag.
    if flags.lookup("help").is_none() {
        let name = tree.get(&path).name.clone();
        let usage = format!("help for {name}");
        if flags.shorthands.contains_key(&b'h') {
            flags.bool_p("help", "", false, &usage);
        } else {
            flags.bool_p("help", "h", false, &usage);
        }
    }
    match flags.parse(&flag_args) {
        Ok(()) => {}
        Err(ParseError::Help) => {
            return Ok(Parsed {
                path,
                positional: flags.args.clone(),
                flags,
                help: true,
            });
        }
        Err(ParseError::Msg(m)) => return Err((path, m)),
    }
    let help = flags.get_bool("help");
    Ok(Parsed {
        positional: flags.args.clone(),
        path,
        flags,
        help,
    })
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (the cobra v1.9.1 / simplecobra v0.6.0 functions the neohugo command line
// executes; help/usage templates and shell completion are not ported).
// OK command.go: stripFlags, argsMinusFirstX, Find, findNext, findSuggestions, SuggestionsFor,
//    CommandPath, mergePersistentFlags, InitDefaultHelpFlag, ParseFlags, ExecuteC (parse part)
// OK args.go: legacyArgs
// OK cobra.go: ld
// OK simplecobra.go: checkArgs, wrapErr (CommandError), (*CommandError).Error
//    command.go: Help/Usage templates (not ported: stdout only)
// ---------------------------------------------------------------------------
