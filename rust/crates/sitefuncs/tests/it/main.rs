//! Integration tests of `neohugo-sitefuncs` (the crate's single test binary, REWRITE_PLAN.md §2.2).

/// The signatures T34 builds against (REWRITE_PLAN.md §2.6) stay as frozen.
#[test]
fn register_signature_is_frozen() {
    let f: fn(&mut tera::Tera, &neohugo_sitefuncs::Handles) = neohugo_sitefuncs::register;
    let _ = f;
}
