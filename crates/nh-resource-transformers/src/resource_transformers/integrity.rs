//! Port of `resources/resource_transformers/integrity/integrity.go`.
//!
//! Owner: Wave B task T15 (resource-factories).

//! Go `integrity`: `fingerprint` (sha256 default; md5/sha384/sha512): content unchanged,
//! `Data.Integrity = algo + "-" + base64(digest)`, `AddOutPathIdentifier("." + hex(digest))`.
//!
//! The digests come from the `md-5` and `sha2` crates and the encoding from `base64`
//! (`STANDARD`, padded); they cannot change output bytes (README rule 2).

use std::io::{Read, SeekFrom};
use std::sync::Arc;

use base64::Engine;
use go_value::Value;
use md5::Digest;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;
use nh_resources::transform::{ResourceAdapter, ResourceTransformation, ResourceTransformationCtx};

/// Go: `defaultHashAlgo`.
const DEFAULT_HASH_ALGO: &str = "sha256";

/// Go: `constants.ResourceTransformationFingerprint`.
const RESOURCE_TRANSFORMATION_FINGERPRINT: &str = "fingerprint";

/// Go: `integrity.Client`.
pub struct Client {
    pub rs: Arc<Spec>,
}

/// Go: `fingerprintTransformation`.
struct FingerprintTransformation {
    algo: String,
}

impl ResourceTransformation for FingerprintTransformation {
    // Go: resources/resource_transformers/integrity/integrity.go:(*fingerprintTransformation).Key
    fn key(&self) -> ResourceTransformationKey {
        ResourceTransformationKey::new(
            RESOURCE_TRANSFORMATION_FINGERPRINT,
            vec![Value::string(self.algo.as_str())],
        )
    }

    /// Transform creates a MD5 hash of the Resource content and inserts that hash before the
    /// extension in the filename.
    // Go: resources/resource_transformers/integrity/integrity.go:(*fingerprintTransformation).Transform
    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()> {
        let mut h = new_hash(&self.algo)?;

        if let Some(rc) = ctx.from.as_read_seeker() {
            // This transformation does not change the content, so try to avoid writing to To
            // if we can.
            let mut b = Vec::new();
            let r = rc.read_to_end(&mut b);
            // (Go: `defer rc.Seek(0, 0)`, its error ignored.)
            let _ = rc.seek(SeekFrom::Start(0));
            r?;
            h.write(&b);
        } else {
            let b = ctx.from.read_all()?;
            h.write(&b);
            ctx.to.extend_from_slice(&b);
        }
        let d = digest(h);

        ctx.data
            .insert("Integrity", Value::string(integrity(&self.algo, &d)));
        ctx.add_out_path_identifier(&format!(".{}", hex_encode(&d)));
        Ok(())
    }
}

/// Go's `hash.Hash` of the supported algorithms.
enum Hash {
    Md5(md5::Md5),
    Sha256(sha2::Sha256),
    Sha384(sha2::Sha384),
    Sha512(sha2::Sha512),
}

impl Hash {
    fn write(&mut self, b: &[u8]) {
        match self {
            Hash::Md5(h) => h.update(b),
            Hash::Sha256(h) => h.update(b),
            Hash::Sha384(h) => h.update(b),
            Hash::Sha512(h) => h.update(b),
        }
    }
}

// Go: resources/resource_transformers/integrity/integrity.go:newHash
fn new_hash(algo: &str) -> Result<Hash> {
    match algo {
        "md5" => Ok(Hash::Md5(md5::Md5::new())),
        "sha256" => Ok(Hash::Sha256(sha2::Sha256::new())),
        "sha384" => Ok(Hash::Sha384(sha2::Sha384::new())),
        "sha512" => Ok(Hash::Sha512(sha2::Sha512::new())),
        _ => Err(Error::new(format!(
            "unsupported hash algorithm: {}, use either md5, sha256, sha384 or sha512",
            go_strconv::quote(algo)
        ))),
    }
}

impl Client {
    // Go: resources/resource_transformers/integrity/integrity.go:New
    pub fn new(rs: Arc<Spec>) -> Client {
        Client { rs }
    }

    /// Fingerprint applies fingerprinting of the given resource and hash algorithm. It defaults
    /// to sha256 if none given, and the options are md5, sha256 or sha512. The same algo is used
    /// for both the fingerprinting part (aka cache busting) and the base64-encoded Subresource
    /// Integrity hash.
    ///
    /// `r` must be a `resources.ResourceTransformer` (a resource adapter). Go's `Fingerprint`
    /// calls `res.Transform` (no context), so `_ctx` is not used.
    // Go: resources/resource_transformers/integrity/integrity.go:Fingerprint
    pub fn fingerprint(
        &self,
        _ctx: &nh_tpl::template::TplContext,
        r: Arc<dyn Resource>,
        algo: &str,
    ) -> Result<Arc<dyn Resource>> {
        let algo = if algo.is_empty() {
            DEFAULT_HASH_ALGO
        } else {
            algo
        };

        let res = transformer(&r)?;
        Ok(res.transform(vec![Arc::new(FingerprintTransformation {
            algo: algo.to_string(),
        })])?)
    }
}

/// The `resources.ResourceTransformer` behind a resource (Go's static parameter type of the
/// transformer clients: only a resource adapter implements it). The tpl namespace checks the
/// type before calling a client; this is the same check for direct callers.
pub(crate) fn transformer(r: &Arc<dyn Resource>) -> Result<Arc<ResourceAdapter>> {
    nh_resources::transform::resource_adapter(r)
        .ok_or_else(|| Error::new(format!("{} can not be transformed", r.tpl_type_name())))
}

// Go: resources/resource_transformers/integrity/integrity.go:integrity
fn integrity(algo: &str, sum: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(sum);
    format!("{algo}-{encoded}")
}

// Go: resources/resource_transformers/integrity/integrity.go:digest
fn digest(h: Hash) -> Vec<u8> {
    match h {
        Hash::Md5(h) => h.finalize().to_vec(),
        Hash::Sha256(h) => h.finalize().to_vec(),
        Hash::Sha384(h) => h.finalize().to_vec(),
        Hash::Sha512(h) => h.finalize().to_vec(),
    }
}

/// Go `hex.EncodeToString` (lower case).
fn hex_encode(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for &c in b {
        s.push(HEX[(c >> 4) as usize] as char);
        s.push(HEX[(c & 0x0f) as usize] as char);
    }
    s
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/integrity/integrity.go (124 lines; 7/7 funcs executed)
//   types: Client, fingerprintTransformation
// OK L42-44: New(rs *resources.Spec) *Client
// OK L50-52: (t *fingerprintTransformation) Key() internal.ResourceTransformationKey
// OK L56-85: (t *fingerprintTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// OK L87-100: newHash(algo string) (hash.Hash, error)
// OK L108-114: (c *Client) Fingerprint(res resources.ResourceTransformer, algo string) (resource.Resource, error)
// OK L116-119: integrity(algo string, sum []byte) string
// OK L121-124: digest(h hash.Hash) ([]byte, error)
// ---------------------------------------------------------------------------
