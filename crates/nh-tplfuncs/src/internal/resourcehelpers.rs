//! Port of `tpl/internal/resourcehelpers/helpers.go`.
//!
//! Owner: Wave B task T15 (resource-factories).


use std::sync::Arc;

use go_value::Value;
use nh_common::Result;
use nh_resource::resourcetypes::Resource;

/// Go: `resourcehelpers.ResolveArgs(args)` — (targetPath|options, resource) from template args
/// (last arg is the resource: pipelines append it).
// Go: tpl/internal/resourcehelpers/helpers.go:ResolveArgs
pub fn resolve_args(args: &[Value]) -> Result<(Option<Value>, Arc<dyn Resource>)> {
    todo!()
}

/// Go: `resourcehelpers.ResolveIfFirstArgIsString(args)`.
// Go: tpl/internal/resourcehelpers/helpers.go:ResolveIfFirstArgIsString
pub fn resolve_if_first_arg_is_string(args: &[Value]) -> Result<(String, Arc<dyn Resource>)> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/internal/resourcehelpers/helpers.go (69 lines; 2/2 funcs executed)
// EX L27-39: ResolveIfFirstArgIsString(args []any) (resources.ResourceTransformer, string, bool)
// EX L42-69: ResolveArgs(args []any) (resources.ResourceTransformer, map[string]any, error)
// ---------------------------------------------------------------------------
