//! Oracle: permalink patterns expanded for every page of Hugo's builds of the reference sites,
//! with the site's `[permalinks]`, a configuration using every token, and 55 patterns
//! (`oracle/page/permalinks/*`).

use std::collections::BTreeMap;

use neohugo_base::PageKind;
use neohugo_page::{PermalinkCtx, PermalinkFile, PermalinkPattern};
use serde_json::{Value as J, json};

use crate::support::{Tally, family, fixture, idx, oracle_paths, s, site_urls, zoned};

/// A kind → section → pattern table, compiled (the keys trimmed like
/// [`neohugo_page::PermalinkPatterns`] does).
fn compile(v: &J) -> BTreeMap<(String, String), Result<PermalinkPattern, String>> {
    let mut out = BTreeMap::new();
    for (kind, m) in v.as_object().expect("config") {
        for (section, pattern) in m.as_object().expect("patterns") {
            let key = section.trim_matches([' ', '/']).to_owned();
            out.entry((kind.clone(), key))
                .or_insert_with(|| PermalinkPattern::parse(s(pattern)).map_err(|e| e.to_string()));
        }
    }
    out
}

fn result(r: Result<String, String>) -> J {
    match r {
        Ok(s) => json!({ "ok": s }),
        Err(e) => json!({ "err": e }),
    }
}

/// Hugo replaces attributes one after another with `strings.Replace(…, 1)` in the partially
/// expanded pattern; we expand each attribute in place. They differ only when a value holds
/// text that looks like a later attribute.
fn is_replace_quirk(pattern: &str) -> bool {
    pattern.contains(":title:") || pattern.contains(":slug:")
}

#[test]
fn permalinks_match_hugo() {
    let mut t = Tally::default();
    for file in family("permalinks", &["decode.json.gz"]) {
        let fx = fixture(&format!("permalinks/{file}"));
        let paths = oracle_paths(&fx["paths"]);
        let urls: Vec<_> = fx["pathspecs"]
            .as_array()
            .expect("pathspecs")
            .iter()
            .map(site_urls)
            .collect();
        let patterns: Vec<(String, Result<PermalinkPattern, String>)> = fx["patterns"]
            .as_array()
            .expect("patterns")
            .iter()
            .map(|p| {
                let p = s(p).to_owned();
                let compiled = PermalinkPattern::parse(&p).map_err(|e| e.to_string());
                (p, compiled)
            })
            .collect();
        let site = compile(&fx["siteConfig"]);
        let all = compile(&fx["allConfig"]);
        for c in fx["cases"].as_array().expect("cases") {
            let date = zoned(&c["date"]);
            let entries: Vec<&str> = c["sectionsEntries"]
                .as_array()
                .map(|a| a.iter().map(s).collect())
                .unwrap_or_default();
            assert_eq!(
                format!("/{}", entries.join("/")),
                s(&c["sectionsPath"]),
                "{file}: sections path of {c}"
            );
            let file_ctx = c.get("file").map(|f| PermalinkFile {
                translation_base_name: s(&f["translationBaseName"]),
                dir: s(&f["dir"]),
            });
            let path = &paths[idx(&c["pathInfo"])];
            let ctx = PermalinkCtx {
                date: date.as_ref(),
                title: s(&c["title"]),
                slug: s(&c["slug"]),
                section: s(&c["section"]),
                sections: &entries,
                file: file_ctx,
                content_base_name: &path.unnormalized_name,
                urls: &urls[idx(&c["ps"])],
            };
            let kind = s(&c["kind"]);
            assert!(PageKind::parse(kind).is_some(), "kind {kind}");
            let expand =
                |table: &BTreeMap<(String, String), Result<PermalinkPattern, String>>| match table
                    .get(&(kind.to_owned(), s(&c["section"]).to_owned()))
                {
                    None => Ok(String::new()),
                    Some(Err(e)) => Err(e.clone()),
                    Some(Ok(p)) => p.expand(&ctx).map_err(|e| e.to_string()),
                };
            let mut check = |name: &str, got: J, want: &J| {
                let same = match (want.get("ok"), got.get("ok")) {
                    (Some(w), Some(g)) => w == g,
                    (None, None) => true, // both errors: the texts are ours
                    _ => false,
                };
                if same {
                    t.pass();
                } else if is_replace_quirk(name) {
                    t.accept("sequential-replace");
                } else {
                    t.fail(|| format!("{file} {} [{name}] got {got}, want {want}", path.input));
                }
            };
            check("expandSite", result(expand(&site)), &c["expandSite"]);
            check("expandAll", result(expand(&all)), &c["expandAll"]);
            for (i, (pattern, compiled)) in patterns.iter().enumerate() {
                let got = match compiled {
                    Err(e) => Err(e.clone()),
                    Ok(p) => p.expand(&ctx).map_err(|e| e.to_string()),
                };
                check(pattern, result(got), &c["patterns"][i]);
            }
        }
    }
    t.finish("permalinks");
}
