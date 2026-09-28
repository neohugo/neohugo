//! Port of `tpl/internal/resourcehelpers/helpers.go`.
//!
//! Owner: Wave B task T15 (resource-factories).
//!
//! A `resources.ResourceTransformer` is a resource adapter
//! ([`nh_resources::transform::ResourceAdapter`]); the Go-shaped forms return it, the skeleton
//! forms return it as a `resource.Resource`.

use std::sync::Arc;

use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_resource::resourcetypes::Resource;
use nh_resources::transform::ResourceAdapter;

/// Go `args[i].(resources.ResourceTransformer)`.
pub fn transformer_from_value(v: &Value) -> Option<Arc<ResourceAdapter>> {
    let r = nh_resource::resourcetypes::resource_from_value_any(v)?;
    nh_resources::transform::resource_adapter(&r)
}

/// Go `%T` of a template value.
fn type_of(v: &Value) -> String {
    v.go_type_name().into_owned()
}

/// Go: `ResolveIfFirstArgIsString(args)` — we allow string or a map as the first argument in
/// some cases: `(transformer, first arg, ok)`.
// Go: tpl/internal/resourcehelpers/helpers.go:ResolveIfFirstArgIsString
pub fn resolve_if_first_arg_is_string_go(
    args: &[Value],
) -> (Option<Arc<ResourceAdapter>>, String, bool) {
    if args.len() != 2 {
        return (None, String::new(), false);
    }

    let Value::String(v1) = &args[0] else {
        return (None, String::new(), false);
    };
    let v2 = transformer_from_value(&args[1]);
    let ok2 = v2.is_some();

    (v2, String::from_utf8_lossy(v1.as_bytes()).into_owned(), ok2)
}

/// Go: `ResolveArgs(args)` — this roundabout way of doing it is needed to get both pipeline
/// behavior and options as arguments: `(transformer, options)` (`None` = nil map).
// Go: tpl/internal/resourcehelpers/helpers.go:ResolveArgs
pub fn resolve_args_go(args: &[Value]) -> Result<(Arc<ResourceAdapter>, Option<Map>)> {
    if args.is_empty() {
        return Err(Error::new("no Resource provided in transformation"));
    }

    if args.len() == 1 {
        let Some(r) = transformer_from_value(&args[0]) else {
            return Err(Error::new(format!(
                "type {} not supported in Resource transformations",
                type_of(&args[0])
            )));
        };
        return Ok((r, None));
    }

    let Some(r) = transformer_from_value(&args[1]) else {
        if !matches!(&args[1], Value::Map(m) if m.ty == MapType::StringAny) {
            return Err(Error::new("no Resource provided in transformation"));
        }
        // (Go reports the type of the first argument here.)
        return Err(Error::new(format!(
            "type {} not supported in Resource transformations",
            type_of(&args[0])
        )));
    };

    let m = nh_common::maps::maps::to_string_map_e(&args[0])
        .map_err(|err| Error::new(format!("invalid options type: {err}")))?;

    Ok((r, Some(m)))
}

/// Go: `resourcehelpers.ResolveArgs(args)` — (options, resource) from template args (the last
/// arg is the resource: pipelines append it). The options are the `map[string]any` value
/// (`None` = nil).
// Go: tpl/internal/resourcehelpers/helpers.go:ResolveArgs
pub fn resolve_args(args: &[Value]) -> Result<(Option<Value>, Arc<dyn Resource>)> {
    let (r, m) = resolve_args_go(args)?;
    Ok((m.map(Value::map), r as Arc<dyn Resource>))
}

/// Go: `resourcehelpers.ResolveIfFirstArgIsString(args)` — `(first arg, resource)`; an error
/// when Go's `ok` is false.
// Go: tpl/internal/resourcehelpers/helpers.go:ResolveIfFirstArgIsString
pub fn resolve_if_first_arg_is_string(args: &[Value]) -> Result<(String, Arc<dyn Resource>)> {
    match resolve_if_first_arg_is_string_go(args) {
        (Some(r), s, true) => Ok((s, r as Arc<dyn Resource>)),
        _ => Err(Error::new(
            "expected a string and a resources.ResourceTransformer",
        )),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/internal/resourcehelpers/helpers.go (69 lines; 2/2 funcs executed)
// OK L27-39: ResolveIfFirstArgIsString(args []any) (resources.ResourceTransformer, string, bool)
// OK L42-69: ResolveArgs(args []any) (resources.ResourceTransformer, map[string]any, error)
// ---------------------------------------------------------------------------
