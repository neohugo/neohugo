//! Port of `resources/post_publish.go`.
//!
//! Owner: Wave B task T14 (resources-core).


use std::sync::Arc;

use nh_common::Result;
use nh_resource::resourcetypes::Resource;

use crate::postpub::postpub::PostPublishResource;
use crate::resource_spec::Spec;

/// Go: `Spec.PostProcess(r)` — memoized by `r.TransformationKey()` (does NOT run the chain); new
/// entries get `id = BuildState.Incr()` and prefix `__h_pp_l1_<id>_`.
// Go: resources/post_publish.go:PostProcess
pub fn post_process(spec: &Spec, r: Arc<dyn Resource>) -> Result<Arc<PostPublishResource>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/post_publish.go (51 lines; 1/1 funcs executed)
//   types: transformationKeyer
// EX L26-51: (spec *Spec) PostProcess(r resource.Resource) (postpub.PostPublishedResource, error)
// ---------------------------------------------------------------------------
