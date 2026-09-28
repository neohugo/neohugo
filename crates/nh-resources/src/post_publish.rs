//! Port of `resources/post_publish.go`.
//!
//! Owner: Wave B task T14 (resources-core).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;
use nh_resource::resourcetypes::Resource;

use crate::postpub::postpub::PostPublishResource;
use crate::resource_spec::Spec;

/// Go: `Spec.PostProcess(r)` — memoized by `r.TransformationKey()` (does NOT run the chain); new
/// entries get `id = BuildState.Incr()` and prefix `__h_pp_l1_<id>_`. (Go panics when `r` is
/// not a `transformationKeyer`; the port returns that as an error.)
// Go: resources/post_publish.go:PostProcess
pub fn post_process(spec: &Spec, r: Arc<dyn Resource>) -> Result<Arc<PostPublishResource>> {
    let Some(a) = crate::transform::resource_adapter(&r) else {
        return Err(Error::new(format!(
            "interface conversion: {} is not resources.transformationKeyer: missing method TransformationKey",
            r.tpl_type_name()
        )));
    };
    let key = a.transformation_key();

    let assets = &spec.common.post_build_assets;
    let mut m = assets
        .post_process_resources
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    if let Some(result) = m.get(&key) {
        return Ok(result.clone());
    }

    let result = PostPublishResource::new(spec.common.incr.incr(), r);
    m.insert(key, result.clone());

    Ok(result)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/post_publish.go (51 lines; 1/1 funcs executed)
//   types: transformationKeyer
// OK L26-51: (spec *Spec) PostProcess(r resource.Resource) (postpub.PostPublishedResource, error)
// ---------------------------------------------------------------------------
