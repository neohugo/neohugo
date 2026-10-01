//! `to_math` (feature `math`) on every formula of the Hugo documentation site
//! (`docs/content/**/*.md`, delimiters `$$…$$`, `\[…\]` and `\(…\)` as its passthrough
//! configuration declares), code examples included.

use std::collections::BTreeSet;
use std::path::Path;

use tera::Context;

use crate::support::Harness;

/// Formulas using mhchem (`\ce`, `\pu`), a KaTeX extension pulldown-latex does not have: an
/// error, and with `optional=true` a warning and the unknown commands as `<merror>` (accepted
/// deviation).
fn needs_mhchem(tex: &str) -> bool {
    tex.contains("\\ce{") || tex.contains("\\pu{")
}

fn collect(dir: &Path, out: &mut BTreeSet<(String, bool)>) {
    let re = regex::Regex::new(r"(?s)\$\$(.+?)\$\$|\\\[(.+?)\\\]|\\\((.+?)\\\)").expect("valid");
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("docs dir")
        .flatten()
        .collect();
    entries.sort_by_key(std::fs::DirEntry::path);
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            let text = std::fs::read_to_string(&p).expect("utf-8 markdown");
            for caps in re.captures_iter(&text) {
                let (tex, display) = match (caps.get(1), caps.get(2), caps.get(3)) {
                    (Some(m), _, _) | (_, Some(m), _) => (m.as_str(), true),
                    (_, _, Some(m)) => (m.as_str(), false),
                    _ => continue,
                };
                // configuration listings (`['\[', '\]']`) and regular expressions are not math
                let latex_like = tex.contains(['^', '_', '\\', '=']);
                let listing = tex.contains(['\'', '"']) || tex.contains("\\|");
                if latex_like && !listing {
                    out.insert((tex.trim().to_owned(), display));
                }
            }
        }
    }
}

#[test]
fn renders_every_docs_formula() {
    let docs = neohugo_testkit::fixture::repo_dir().join("docs/content");
    let mut formulas = BTreeSet::new();
    collect(&docs, &mut formulas);
    assert!(
        formulas.len() >= 5,
        "found only {} formulas",
        formulas.len()
    );
    let h = Harness::new();
    let mut ctx = Context::new();
    let mut deviations = 0;
    for (tex, display) in &formulas {
        ctx.insert("tex", tex);
        let strict = format!("{{{{ tex | to_math(display={display}) }}}}");
        let src = format!("{{{{ tex | to_math(display={display}, optional=true) }}}}");
        let out = h
            .render(&src, &ctx)
            .unwrap_or_else(|e| panic!("{tex}: {e}"));
        assert!(out.starts_with("<math"), "{tex}: {out}");
        if needs_mhchem(tex) {
            deviations += 1;
            assert!(out.contains("<merror"), "{tex}: {out}");
            let e = h.render(&strict, &ctx).expect_err("mhchem is an error");
            assert!(e.contains("to_math: `"), "{tex}: {e}");
        } else {
            assert_eq!(
                h.render(&strict, &ctx).as_deref(),
                Ok(out.as_str()),
                "{tex}"
            );
            assert!(!out.contains("<merror"), "{tex}: {out}");
        }
        if *display {
            assert!(out.contains("display=\"block\""), "{tex}: {out}");
        }
    }
    eprintln!(
        "to_math: {} distinct docs formulas rendered, {deviations} with mhchem commands as <merror>",
        formulas.len()
    );
}

#[test]
fn inline_and_block() {
    let h = Harness::new();
    let ctx = Context::new();
    let inline = h.render("{{ 'a^2 + b^2 = c^2' | to_math }}", &ctx).unwrap();
    let block = h
        .render("{{ '\\\\frac{1}{2}' | to_math(display=true) }}", &ctx)
        .unwrap();
    neohugo_testkit::snapshot::settings().bind(|| {
        insta::assert_yaml_snapshot!("to_math", [inline, block]);
    });
}

/// Go's `try (transform.ToMath …)`: `optional=true` turns a parse error into a warning (id
/// `to_math`, like `get_remote(optional=true)`) and renders the construct as `<merror>`.
#[test]
fn optional_turns_errors_into_warnings() {
    let env = crate::support::env();
    let h = Harness::with_env(&env);
    let ctx = Context::new();
    let e = h
        .render("{{ '\\\\nosuchcommand{x}' | to_math }}", &ctx)
        .expect_err("an unknown command is an error");
    assert!(e.contains("to_math: `\\nosuchcommand{x}`: "), "{e}");
    assert!(env.diagnostics.report().is_empty());
    let out = h
        .render(
            "{{ '\\\\nosuchcommand{x}' | to_math(optional=true) }}",
            &ctx,
        )
        .expect("optional");
    assert!(out.starts_with("<math") && out.contains("<merror"), "{out}");
    let report = env.diagnostics.report();
    assert_eq!(report.len(), 1, "{report:?}");
    assert_eq!(report[0].severity, neohugo_base::diag::Severity::Warning);
    assert_eq!(report[0].id.as_deref(), Some("to_math"));
    assert!(
        report[0]
            .message
            .starts_with("to_math: `\\nosuchcommand{x}`: "),
        "{}",
        report[0].message
    );
}
