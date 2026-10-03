//! With the `npm` feature, the binary exports the N-API symbols (`napi_*`) that the native
//! addons of npm packages (Tailwind's oxide, lightningcss) link to when the embedded JavaScript
//! runtime loads them.

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    #[cfg(feature = "npm")]
    deno_napi::print_linker_flags(&bin_name());
}

/// The binary's name, from `[[bin]]` in Cargo.toml (where it is written).
#[cfg(feature = "npm")]
fn bin_name() -> String {
    let manifest = std::fs::read_to_string("Cargo.toml").expect("read Cargo.toml");
    manifest
        .split("[[bin]]")
        .nth(1)
        .and_then(|bin| {
            bin.lines().find_map(|l| {
                let value = l
                    .trim()
                    .strip_prefix("name")?
                    .trim_start()
                    .strip_prefix('=')?;
                Some(
                    value
                        .trim()
                        .strip_prefix('"')?
                        .split('"')
                        .next()?
                        .to_owned(),
                )
            })
        })
        .expect("[[bin]] name in Cargo.toml")
}
