//! `toCSS` (LibSass) against the `tocss` oracle (tools/go-oracle/nh-resource-transformers/tocss,
//! built for linux/arm64 with cgo and run under qemu: LibSass's number formatting depends on FMA
//! contraction, and the golden build is darwin/arm64): neohugo's tocss/scss client with golibsass
//! against the port with libsass-sys. The scripts, inputs and recorded attributes are in the
//! fixture (see t16_support). Needs neither Go nor esbuild nor node.

mod t16_support;

use t16_support::*;

#[test]
fn tocss_synth() {
    let fx = fixture("tocss/synth.json.gz");
    assert_eq!(fx["arch"], "arm64");
    let src = repo_root().join(fx["dir"].as_str().unwrap());
    let tmp = copy_site(&src, None, &[]);
    let dir = tmp.path.join("site");
    // LibSass makes source map paths relative to the process working directory (in Go too);
    // the oracle ran from the site dir, like the golden build. No other test of this binary
    // depends on the working directory.
    std::env::set_current_dir(&dir).unwrap();
    let mut site = load_site(&dir.to_string_lossy(), &process_environ());

    let got: Vec<_> = fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| site.run(c))
        .collect();
    let diffs = diff_results("tocss", &fx["results"], &got);
    assert!(
        diffs.is_empty(),
        "{} differences:\n{}",
        diffs.len(),
        diffs.join("\n")
    );
    eprintln!("tocss/synth: {} cases identical", got.len());
}

/// The seeksnack SCSS chain key (specs/resources-pipeline.md §3.2, T14's keys oracle): the real
/// `scss.Options` hashes like Go's struct.
#[test]
fn tocss_seeksnack_key() {
    use go_value::{GoString, Map, MapType, Value};
    use nh_resource_transformers::resource_transformers::tocss::scss::tocss;
    let mut m = Map::new(MapType::StringAny);
    m.entries
        .insert(GoString::from("enableSourceMap"), Value::Bool(false));
    m.entries.insert(
        GoString::from("includePaths"),
        Value::any_list(vec![
            Value::string("node_modules"),
            Value::string("assets/scss"),
        ]),
    );
    m.entries
        .insert(GoString::from("outputStyle"), Value::string("compressed"));
    let opts = tocss::decode_options(Some(&m)).unwrap();
    assert_eq!(
        nh_resource_transformers::resource_transformers::tocss::scss::client_extended::key_value(
            &opts
        ),
        "tocss_7149566072694007861"
    );
}
