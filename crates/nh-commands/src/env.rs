//! Port of `commands/env.go`.
//!
//! Owner: Wave B task T25 (commands-cli).


use nh_common::Result;

/// Go: commands/env.go:newEnvCommand (prints version, GOOS/GOARCH analogues, dependency versions).
pub fn run_env() -> Result<()> {
    todo!()
}

/// Go: commands/env.go:newVersionCmd (prints `neohugo.BuildVersionString()`).
pub fn run_version() -> Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/env.go (70 lines; 2/2 funcs executed)
// EX L25-55: newEnvCommand() simplecobra.Commander
// EX L57-70: newVersionCmd() simplecobra.Commander
// ---------------------------------------------------------------------------
