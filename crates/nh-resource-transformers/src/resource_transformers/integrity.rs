//! Port of `resources/resource_transformers/integrity/integrity.go`.
//!
//! Owner: Wave B task T15 (resource-factories).


//! Go `integrity`: `fingerprint` (sha256 default; md5/sha384/sha512): content unchanged,
//! `Data.Integrity = algo + "-" + base64(digest)`, `AddOutPathIdentifier("." + hex(digest))`.

use std::sync::Arc;

use nh_common::Result;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;

/// Go: `integrity.Client`.
pub struct Client {
    pub rs: Arc<Spec>,
}

impl Client {
    // Go: resources/resource_transformers/integrity/integrity.go:Fingerprint
    pub fn fingerprint(&self, ctx: &nh_tpl::template::TplContext, r: Arc<dyn Resource>, algo: &str) -> Result<Arc<dyn Resource>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/integrity/integrity.go (124 lines; 7/7 funcs executed)
//   types: Client, fingerprintTransformation
// EX L42-44: New(rs *resources.Spec) *Client
// EX L50-52: (t *fingerprintTransformation) Key() internal.ResourceTransformationKey
// EX L56-85: (t *fingerprintTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// EX L87-100: newHash(algo string) (hash.Hash, error)
// EX L108-114: (c *Client) Fingerprint(res resources.ResourceTransformer, algo string) (resource.Resource, error)
// EX L116-119: integrity(algo string, sum []byte) string
// EX L121-124: digest(h hash.Hash) ([]byte, error)
// ---------------------------------------------------------------------------
