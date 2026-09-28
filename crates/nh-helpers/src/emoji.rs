//! Port of `helpers/emoji.go`.
//!
//! STUB: `Emojify` is never executed by the seeksnack build (`enableEmoji = false`, no
//! `emojify` template call; 0 of 86 lines executed). It needs the `kyokomi/emoji` code map, which
//! is not ported, so it returns an explicit unsupported error instead of approximating.
//!
//! Owner: Wave B task T08 (helpers-source-cache).

use nh_common::Result;
use nh_common::herrors::Error;

/// Go: `helpers.Emojify(source)` — replaces `:emoji:` codes. Not supported (see the module docs).
// Go: helpers/emoji.go:Emojify
pub fn emojify(_source: &[u8]) -> Result<Vec<u8>> {
    Err(Error::new(
        "neohugo-rs: emojify (enableEmoji) is not supported",
    ))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/emoji.go (86 lines; 0/2 funcs executed)
//    L36-74: Emojify(source []byte) []byte — STUB (explicit unsupported error)
//    L76-86: initEmoji() — STUB
// ---------------------------------------------------------------------------
