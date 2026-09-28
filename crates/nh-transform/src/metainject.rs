//! Port of `transform/metainject/hugogenerator.go`.
//!
//! Never active in neohugo builds (hugolib sets `AddHugoGeneratorTag =
//! DisableHugoGeneratorInject`, i.e. false by default); ported because it is small.
//!
//! Owner: Wave B task T07 (transform-publisher).

use nh_common::Result;

use crate::chain::FromTo;

/// Go: `metaTagsCheck.Match(b)` = `(?i)<meta\s+name=['|"]?generator['|"]?` (unanchored). `\s` is
/// RE2's `[\t\n\f\r ]`; the letters of `meta`, `name` and `generator` have no non-ASCII simple
/// folds, so `(?i)` is ASCII case folding.
fn meta_tags_check(b: &[u8]) -> bool {
    let is_space = |c: u8| matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ');
    let fold =
        |s: &[u8], lit: &[u8]| s.len() >= lit.len() && s[..lit.len()].eq_ignore_ascii_case(lit);
    for i in 0..b.len() {
        if b[i] != b'<' || !fold(&b[i + 1..], b"meta") {
            continue;
        }
        let mut j = i + 5;
        if j >= b.len() || !is_space(b[j]) {
            continue;
        }
        while j < b.len() && is_space(b[j]) {
            j += 1;
        }
        if !fold(&b[j..], b"name=") {
            continue;
        }
        j += 5;
        if j < b.len() && matches!(b[j], b'\'' | b'|' | b'"') {
            j += 1;
        }
        if fold(&b[j..], b"generator") {
            return true;
        }
    }
    false
}

/// Go: `hugoGeneratorTag`.
fn hugo_generator_tag() -> String {
    format!(
        r#"<meta name="generator" content="Neohugo {}" />"#,
        nh_config::neohugo::version::CURRENT_VERSION.string()
    )
}

/// `bytes.Replace(b, old, new, 1)`.
fn replace_first(b: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    match b.windows(old.len()).position(|w| w == old) {
        Some(i) => {
            let mut out = Vec::with_capacity(b.len() + new.len());
            out.extend_from_slice(&b[..i]);
            out.extend_from_slice(new);
            out.extend_from_slice(&b[i + old.len()..]);
            out
        }
        None => b.to_vec(),
    }
}

/// Go: `metainject.HugoGenerator(ft)` — injects the generator meta tag after `<head>` (or
/// `<HEAD>`) unless the document already has one.
// Go: transform/metainject/hugogenerator.go:HugoGenerator
pub fn hugo_generator_transform(ft: &mut FromTo<'_>) -> Result<()> {
    let b = ft.from;
    if meta_tags_check(b) {
        ft.to.extend_from_slice(b);
        return Ok(());
    }

    let tag = hugo_generator_tag();
    let head = "<head>";
    let replace = format!("{head}\n\t{tag}");
    let mut newcontent = replace_first(b, head.as_bytes(), replace.as_bytes());

    if newcontent.len() == b.len() {
        let head = "<HEAD>";
        let replace = format!("{head}\n\t{tag}");
        newcontent = replace_first(b, head.as_bytes(), replace.as_bytes());
    }

    ft.to.extend_from_slice(&newcontent);

    Ok(())
}

/// The publisher's `metainject.HugoGenerator` transformer.
pub fn hugo_generator_transformer() -> crate::chain::Transformer {
    Box::new(hugo_generator_transform)
}

/// Kept from the skeleton: the generator tag is never injected in neohugo builds, so a build
/// has no such transformer.
pub fn hugo_generator() -> Option<crate::chain::Transformer> {
    None
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/metainject/hugogenerator.go (56 lines; 0/1 funcs executed)
// OK L32-56: HugoGenerator(ft transform.FromTo) error
// ---------------------------------------------------------------------------
