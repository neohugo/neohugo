//! Oracle test: helpers.PathSpec URL/path helpers and helpers.ContentSpec against
//! `tools/go-oracle/nh-helpers/pathspec` (fixtures/pathspec/pathspec.json.gz).

mod support;

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_config::config_provider::AllProvider;
use nh_helpers::content::ContentSpec;
use nh_helpers::pathspec::PathSpec;
use serde_json::Value as J;
use support::*;

fn build(lc: &J, tmp: &TempDir, n_langs: usize) -> Arc<PathSpec> {
    let mut cfg = TestCfg::new(&tmp.str());
    let lang = lc["lang"].as_str().unwrap().to_string();
    let base = lc["baseURL"].as_str().unwrap();
    cfg.base_url = nh_common::urls::new_base_url_from_string(base).unwrap();
    // The Rust BaseURL must be Go's.
    assert_eq!(cfg.base_url.with_path, lc["withPath"].as_str().unwrap());
    assert_eq!(
        cfg.base_url.without_path,
        lc["withoutPath"].as_str().unwrap()
    );
    assert_eq!(cfg.base_url.base_path, lc["basePath"].as_str().unwrap());
    assert_eq!(
        cfg.base_url.base_path_no_trailing_slash,
        lc["basePathNoTrailingSlash"].as_str().unwrap()
    );
    cfg.language_prefix = lc["languagePrefix"].as_str().unwrap().to_string();
    cfg.canonify_urls = lc["canonifyURLs"].as_bool().unwrap();
    cfg.disable_path_to_lower = lc["disablePathToLower"].as_bool().unwrap();
    cfg.remove_path_accents = lc["removePathAccents"].as_bool().unwrap();
    cfg.is_multihost = lc["isMultihost"].as_bool().unwrap();
    let mut languages = Vec::new();
    languages.push(nh_langs::language::Language::new(&lang, "en", "", Default::default()).unwrap());
    for i in 1..n_langs {
        let l = format!("l{i}");
        languages
            .push(nh_langs::language::Language::new(&l, "en", "", Default::default()).unwrap());
    }
    cfg.languages = languages;
    cfg.lang = lang;
    let fs = nh_hugofs::fs::new_from(nh_hugofs::afero::new_mem_map_fs(), &cfg.base_config());
    let ps = PathSpec::new(fs, Arc::new(cfg)).unwrap();
    assert_eq!(ps.processing_stats.name, lc["statsName"].as_str().unwrap());
    assert_eq!(ps.get_base_path(false), lc["getBasePath"].as_str().unwrap());
    assert_eq!(
        ps.get_base_path(true),
        lc["getBasePathRel"].as_str().unwrap()
    );
    if !ps.cfg.is_multihost() || n_langs > 1 {
        assert_eq!(
            ps.get_target_language_base_path(),
            lc["targetLanguageBasePath"].as_str().unwrap()
        );
    }
    ps
}

fn run(p: &PathSpec, s: &str) -> Vec<J> {
    let s = s.to_string();
    vec![
        call(|| p.make_path(&s)),
        call(|| p.make_path_sanitized(&s)),
        call(|| p.urlize(&s)),
        call(|| p.urlize_filename(&s)),
        call(|| p.url_escape(&s)),
        call(|| p.abs_url(&s, false)),
        call(|| p.abs_url(&s, true)),
        call(|| p.rel_url(&s, false)),
        call(|| p.rel_url(&s, true)),
        call(|| p.prepend_base_path(&s, false)),
        call(|| p.prepend_base_path(&s, true)),
        call(|| match p.try_is_abs_url(&s) {
            Ok(b) => b.to_string(),
            Err(e) => format!("err: {}", e.message()),
        }),
        call(|| p.permalink_for_base_url(&s, p.cfg.base_url().string())),
    ]
}

#[test]
fn pathspec_matches_go() {
    let fx = fixture("pathspec", "pathspec.json.gz");
    let funcs: Vec<String> = fx["funcs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    let tmp = TempDir::new("pathspec");
    let mut specs: BTreeMap<(String, String), Arc<PathSpec>> = BTreeMap::new();
    for st in fx["setups"].as_array().unwrap() {
        let name = st["name"].as_str().unwrap();
        for lc in st["langs"].as_array().unwrap() {
            let n = lc["languages"].as_u64().unwrap() as usize;
            specs.insert(
                (name.to_string(), lc["lang"].as_str().unwrap().to_string()),
                build(lc, &tmp, n),
            );
        }
    }

    let first_cfg = TestCfg::new(&tmp.str());
    // ContentSpec::new decodes the default content types (Go: cfg.ContentTypes()).
    let cs = ContentSpec::new(
        Arc::new(first_cfg),
        nh_config::hexec::Exec::new_with_env(Default::default(), &tmp.str(), &[], None),
    )
    .unwrap();

    let mut checks = 0usize;
    let mut accents = 0usize;
    let mut fails = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let input = String::from_utf8(gostr(&c["in"])).unwrap();
        if let Some(cs_fn) = c["cs"].as_str() {
            let got = match cs_fn {
                "ResolveMarkup" => call(|| cs.resolve_markup(&input)),
                "SanitizeAnchorName" => call(|| cs.sanitize_anchor_name(&input)),
                "TrimShortHTML" => {
                    enc(&cs.trim_short_html(input.as_bytes(), c["markup"].as_str().unwrap()))
                }
                f => panic!("{f}"),
            };
            checks += 1;
            if got != c["r"] {
                fails.push(format!("{cs_fn}({input:?}): got {got}, want {}", c["r"]));
            }
            continue;
        }
        let key = (
            c["setup"].as_str().unwrap().to_string(),
            c["lang"].as_str().unwrap().to_string(),
        );
        let p = &specs[&key];
        let got = run(p, &input);
        let want = c["r"].as_array().unwrap();
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            checks += 1;
            if p.cfg.remove_path_accents() && i < 3 {
                // MakePath, MakePathSanitized and Urlize go through RemoveAccentsString.
                accents += 1;
            }
            if g != w {
                fails.push(format!(
                    "{key:?} {}({input:?}): got {g}, want {w}",
                    funcs[i]
                ));
            }
        }
    }
    eprintln!(
        "pathspec: {checks} checks, {accents} of them through removePathAccents, {} failures",
        fails.len()
    );
    for f in fails.iter().take(40) {
        eprintln!("  {f}");
    }
    assert!(fails.is_empty());
}
