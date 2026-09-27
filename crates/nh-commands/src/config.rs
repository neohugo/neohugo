//! Port of `commands/config.go`.
//!
//! Owner: Wave B task T25 (commands-cli).


use nh_common::Result;

/// Go: `commands.configCommand` — prints the resolved config of one language as
/// toml/yaml/json (debugging aid; not part of the byte-parity target).
pub struct ConfigCommand {
    pub format: String,
    pub lang: String,
}

impl ConfigCommand {
    /// Go: `(*configCommand).Run`.
    pub fn run(&self, root: &crate::commandeer::RootCommand) -> Result<()> {
        todo!()
    }
}

/// Go: `commands.configMountsCommand`.
pub struct ConfigMountsCommand;

impl ConfigMountsCommand {
    /// Go: `(*configMountsCommand).Run` — JSON dump of module mounts (Go: `configModMounts.MarshalJSON`).
    pub fn run(&self, root: &crate::commandeer::RootCommand) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/config.go (239 lines; 7/12 funcs executed)
//   types: configCommand, configModMount, configModMounts, configMountsCommand
// EX L35-41: newConfigCommand() *configCommand
// EX L53-55: (c *configCommand) Commands() []simplecobra.Commander
// EX L57-59: (c *configCommand) Name() string
//    L61-109: (c *configCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error
// EX L111-124: (c *configCommand) Init(cd *simplecobra.Commandeer) error
//    L126-128: (c *configCommand) PreRun(cd, runner *simplecobra.Commandeer) error
//    L142-197: (m *configModMounts) MarshalJSON() ([]byte, error)
// EX L204-206: (c *configMountsCommand) Commands() []simplecobra.Commander
// EX L208-210: (c *configMountsCommand) Name() string
//    L212-225: (c *configMountsCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error
// EX L227-234: (c *configMountsCommand) Init(cd *simplecobra.Commandeer) error
//    L236-239: (c *configMountsCommand) PreRun(cd, runner *simplecobra.Commandeer) error
// ---------------------------------------------------------------------------
