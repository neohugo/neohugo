//! Port of `tpl/crypto/crypto.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! MD5, SHA-256 and SHA-512 come from the `md-5` and `sha2` crates (README rule 2). SHA-1 (the
//! `sha1` crate is not available offline), HMAC (`crypto/hmac`), FNV-1a and hex encoding are
//! small fixed algorithms written out here.

use std::sync::Arc;

use go_value::{GoString, HostCtx, Object, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

/// Go: `crypto.Namespace` (template value `*crypto.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/crypto:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/crypto:FNV32a
    pub fn fnv32a(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "FNV32a")?;
        // Go: tpl/crypto/crypto.go:FNV32a
        nh_config::neohugo::neohugo::deprecate("crypto.FNV32a", "Use hash.FNV32a.", "v0.129.0");
        let conv = caste::to_string_e(&a[0])?;
        Ok(Value::int(fnv32a(&conv) as i64))
    }

    // Go: tpl/crypto:HMAC
    pub fn hmac(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 3, "HMAC")?;
        Ok(Value::String(self.do_hmac(&a[0], &a[1], &a[2], &a[3..])?))
    }

    // Go: tpl/crypto/crypto.go:HMAC
    /// HMAC returns a cryptographic hash that uses a key to sign a message.
    pub fn do_hmac(&self, h: &Value, k: &Value, m: &Value, e: &[Value]) -> GoResult<GoString> {
        let ha = caste::to_string_e(h)?;

        let alg = match ha.as_bytes() {
            b"md5" => Alg::Md5,
            b"sha1" => Alg::Sha1,
            b"sha256" => Alg::Sha256,
            b"sha512" => Alg::Sha512,
            _ => {
                return Err(go_value::Error::new(format!(
                    "hmac: {ha} is not a supported hash function"
                )));
            }
        };

        let msg = caste::to_string_e(m)?;

        let key = caste::to_string_e(k)?;

        let mac = hmac(alg, &key, &msg);

        let mut encoding = GoString::from("hex");
        if let Some(e0) = e.first()
            && !e0.is_invalid()
        {
            encoding = caste::to_string_e(e0)?;
        }

        match encoding.as_bytes() {
            b"binary" => Ok(GoString::from(mac)),
            b"hex" => Ok(GoString::from(hex_encode(&mac))),
            _ => Err(go_value::Error::new(format!(
                "{} is not a supported encoding method",
                go_strconv::quote(&encoding)
            ))),
        }
    }

    // Go: tpl/crypto:MD5
    pub fn md5(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "MD5")?;
        // Go: tpl/crypto/crypto.go:MD5
        let conv = caste::to_string_e(&a[0])?;
        Ok(Value::string(hex_encode(&digest(Alg::Md5, &conv))))
    }

    // Go: tpl/crypto:SHA1
    pub fn sha1(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "SHA1")?;
        // Go: tpl/crypto/crypto.go:SHA1
        let conv = caste::to_string_e(&a[0])?;
        Ok(Value::string(hex_encode(&digest(Alg::Sha1, &conv))))
    }

    // Go: tpl/crypto:SHA256
    pub fn sha256(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "SHA256")?;
        // Go: tpl/crypto/crypto.go:SHA256
        let conv = caste::to_string_e(&a[0])?;
        Ok(Value::string(hex_encode(&digest(Alg::Sha256, &conv))))
    }
}

/// The hash functions `hmac` accepts.
#[derive(Clone, Copy)]
pub(crate) enum Alg {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl Alg {
    fn block_size(self) -> usize {
        match self {
            Alg::Sha512 => 128,
            _ => 64,
        }
    }
}

/// The digest of `data`.
pub(crate) fn digest(alg: Alg, data: &[u8]) -> Vec<u8> {
    use sha2::Digest;
    match alg {
        Alg::Md5 => md5::Md5::digest(data).to_vec(),
        Alg::Sha1 => sha1(data).to_vec(),
        Alg::Sha256 => sha2::Sha256::digest(data).to_vec(),
        Alg::Sha512 => sha2::Sha512::digest(data).to_vec(),
    }
}

// Go: crypto/hmac/hmac.go:New (+ Write, Sum)
/// HMAC (RFC 2104) of `msg` with `key`.
pub(crate) fn hmac(alg: Alg, key: &[u8], msg: &[u8]) -> Vec<u8> {
    let bs = alg.block_size();
    let mut k = if key.len() > bs {
        digest(alg, key)
    } else {
        key.to_vec()
    };
    k.resize(bs, 0);
    let mut inner: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    inner.extend_from_slice(msg);
    let ih = digest(alg, &inner);
    let mut outer: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    outer.extend_from_slice(&ih);
    digest(alg, &outer)
}

// Go: crypto/sha1/sha1.go:Sum
/// SHA-1 (FIPS 180-4).
pub(crate) fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, x) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

// Go: hash/fnv/fnv.go:New32a (+ Write, Sum32)
/// FNV-1a, 32 bit.
pub(crate) fn fnv32a(data: &[u8]) -> u32 {
    let mut h: u32 = 2166136261;
    for b in data {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

// Go: encoding/hex/hex.go:EncodeToString
/// Lower-case hex.
pub(crate) fn hex_encode(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push(HEX[(x >> 4) as usize] as char);
        s.push(HEX[(x & 15) as usize] as char);
    }
    s
}

nh_common::go_methods!(Namespace {
    "FNV32a" => |n, ctx, a| n.fnv32a(ctx, a),
    "HMAC" => |n, ctx, a| n.hmac(ctx, a),
    "MD5" => |n, ctx, a| n.md5(ctx, a),
    "SHA1" => |n, ctx, a| n.sha1(ctx, a),
    "SHA256" => |n, ctx, a| n.sha256(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*crypto.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/crypto/crypto.go (139 lines; 1/6 funcs executed)
//   types: Namespace
// OK L33-35: New() *Namespace
// OK L41-49: (ns *Namespace) MD5(v any) (string, error)
// OK L52-60: (ns *Namespace) SHA1(v any) (string, error)
// OK L63-71: (ns *Namespace) SHA256(v any) (string, error)
// OK L75-84: (ns *Namespace) FNV32a(v any) (int, error)
// OK L87-139: (ns *Namespace) HMAC(h any, k any, m any, e ...any) (string, error)
// ---------------------------------------------------------------------------
