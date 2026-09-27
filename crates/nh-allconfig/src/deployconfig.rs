//! Port of `deploy/deployconfig/deployConfig.go`.
//!
//! STUB (decode only)
//!
//! Owner: Wave B task T09 (allconfig-modules).


/// STUB: `deploy/deployconfig.DeployConfig` (decoded only for `hugo config` output).
#[derive(Clone, Debug, Default)]
pub struct DeployConfig {
    pub raw: Option<go_value::Map>,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: deploy/deployconfig/deployConfig.go (not found in signature dump)
// ---------------------------------------------------------------------------
