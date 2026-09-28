//! Port of `commands/env.go`.
//!
//! Owner: Wave B task T25 (commands-cli).

use nh_common::Result;
use nh_config::neohugo::neohugo::{GO_VERSION, get_dependency_list, get_dependency_list_non_go};
use nh_config::neohugo::version::{BuildInfo, build_version_string};

use crate::commandeer::{ExecOptions, RootCommand};

/// Go: commands/env.go:newEnvCommand (prints version, GOOS/GOARCH analogues, dependency versions).
// Go: commands/env.go:newEnvCommand
pub fn run_env() -> Result<()> {
    let mut r = RootCommand::new(ExecOptions::default());
    r.pre_run()?;
    run_env_with(&r)
}

/// Go: `newEnvCommand`'s run func, printing through the root command (`r.Printf`).
pub fn run_env_with(r: &RootCommand) -> Result<()> {
    let bi = BuildInfo::current();
    r.print(&format!("{}\n", build_version_string()));
    r.print(&format!("GOOS={}\n", go_strconv::quote(&bi.go_os)));
    r.print(&format!("GOARCH={}\n", go_strconv::quote(&bi.go_arch)));
    r.print(&format!("GOVERSION={}\n", go_strconv::quote(GO_VERSION)));

    let deps = if r.is_verbose() {
        // Go lists every Go module of the binary's build info too; the port has none.
        get_dependency_list()
    } else {
        // These are also included in the GetDependencyList above;
        // always print these as these are most likely the most useful to know about.
        get_dependency_list_non_go()
    };
    for dep in deps {
        r.print(&format!("{dep}\n"));
    }
    Ok(())
}

/// Go: commands/env.go:newVersionCmd (prints `neohugo.BuildVersionString()`).
// Go: commands/env.go:newVersionCmd
pub fn run_version() -> Result<()> {
    let mut r = RootCommand::new(ExecOptions::default());
    r.pre_run()?;
    run_version_with(&r)
}

/// Go: `newVersionCmd`'s run func (`r.Println`).
pub fn run_version_with(r: &RootCommand) -> Result<()> {
    r.print(&format!("{}\n", build_version_string()));
    Ok(())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/env.go (70 lines; 2/2 funcs executed)
// OK L25-55: newEnvCommand() simplecobra.Commander
// OK L57-70: newVersionCmd() simplecobra.Commander
// ---------------------------------------------------------------------------
