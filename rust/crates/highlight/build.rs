//! Builds the syntax set once, at compile time: two-face's syntaxes (the `extra_newlines`
//! dump) plus the crate's own Go template syntaxes, linked and dumped into `OUT_DIR`.
//!
//! Linking the set (`SyntaxSetBuilder::build`) takes about 0.45 s; done at run time it was the
//! largest fixed cost of every build (T70, A-P). The dump loads lazily in a few milliseconds.

use std::path::PathBuf;

use syntect::parsing::SyntaxDefinition;

const OWN_SYNTAXES: &[&str] = &["GoTemplate.sublime-syntax", "GoHtmlTemplate.sublime-syntax"];

fn main() {
    let mut builder = two_face::syntax::extra_newlines().into_builder();
    for file in OWN_SYNTAXES {
        let path = format!("src/syntaxes/{file}");
        println!("cargo::rerun-if-changed={path}");
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let def = SyntaxDefinition::load_from_str(&src, true, None)
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        builder.add(def);
    }
    println!("cargo::rerun-if-changed=build.rs");
    let set = builder.build();
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("syntaxes.dump");
    syntect::dumps::dump_to_file(&set, &out).unwrap_or_else(|e| panic!("{}: {e}", out.display()));
}
