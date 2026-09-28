//! The `resources` oracle (tools/go-oracle/nh-resources/resources): every asset and bundle
//! resource of the docs site, hugolib/testsite and the synthetic site, recreated from the
//! descriptor hugolib or the create client used, with the page's front matter `resources`
//! metadata applied; RelPermalink, Permalink, Key, Name, Title, NameNormalized, MediaType,
//! ResourceType, Data, Params, Content (text) and Width/Height compared with Go, and the files
//! the links and `Publish` wrote.

mod support;

use std::sync::Arc;

use nh_resource::resourcetypes::Resource;
use nh_resources::resource::ResourceSourceDescriptor;
use nh_resources::transform::ResourceAdapter;
use serde_json::Value as J;
use support::*;

fn run(name: &str) -> (usize, usize) {
    let fx = fixture(&format!("resources/{name}.json.gz"));
    let root = repo_root();
    let root_s = root.to_string_lossy().into_owned();
    let dir = root.join(fx["dir"].as_str().unwrap());
    let site = load_site(&dir.to_string_lossy(), None);

    let langs: Vec<String> = fx["langs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap().to_string())
        .collect();
    assert_eq!(site.langs, langs, "{name}: languages");

    let mut failures = Vec::new();
    let mut created: Vec<Arc<ResourceAdapter>> = Vec::new();
    let empty = Vec::new();
    let records = fx["records"].as_array().unwrap_or(&empty);
    let mut unsupported = 0;
    for (i, rc) in records.iter().enumerate() {
        let what = format!("{name}[{i}] {} {}", rc["where"], rc["rd"]["file"]);
        let rd = &rc["rd"];
        let lang = rc["lang"].as_u64().unwrap() as usize;

        let r = if let Some(same) = rc.get("same").and_then(J::as_u64) {
            created[same as usize].clone()
        } else {
            let mut d = ResourceSourceDescriptor {
                open_read_seek_closer: Some(open_file(root.join(rd["file"].as_str().unwrap()))),
                name_normalized: rd["nameNormalized"].as_str().unwrap().to_string(),
                name_original: rd["nameOriginal"].as_str().unwrap().to_string(),
                title: rd["title"].as_str().unwrap().to_string(),
                target_base_paths: rd["targetBasePaths"]
                    .as_array()
                    .map(|a| a.iter().map(|s| s.as_str().unwrap().to_string()).collect())
                    .unwrap_or_default(),
                target_path: rd["targetPath"].as_str().unwrap().to_string(),
                base_path_rel_permalink: rd["basePathRelPermalink"].as_str().unwrap().to_string(),
                base_path_target_path: rd["basePathTargetPath"].as_str().unwrap().to_string(),
                source_filename_or_path: rd["sourceFilenameOrPath"]
                    .as_str()
                    .unwrap()
                    .replace(PLACEHOLDER, &root_s),
                data: dec_opt_map(&rd["data"]),
                params: dec_opt_map(&rd["params"]),
                lazy_publish: rd["lazyPublish"].as_bool().unwrap(),
                ..Default::default()
            };
            site.specs[lang]
                .new_resource_adapter(&mut d)
                .unwrap_or_else(|e| panic!("{what}: {e}"))
        };
        created.push(r.clone());

        let got = rec(&r, true);
        if !rd["lazyPublish"].as_bool().unwrap() {
            r.publish()
                .unwrap_or_else(|e| panic!("{what}: publish: {e}"));
        }

        // GIF: nh-images does not decode GIF configs (T10 stub), so Width/Height are 0 where
        // Go decodes them.
        let gif = got["mediaType"] == "image/gif";
        let skip: &[&str] = if gif { &["width", "height"] } else { &[] };
        if gif && rc["base"].get("width").is_some() {
            unsupported += 1;
            assert_eq!(got["width"], 0, "{what}");
        }
        failures.extend(diff(&format!("{what} base"), &rc["base"], &got, skip));

        if let Some(meta) = rc.get("meta").and_then(J::as_array) {
            let metas: Vec<go_value::Map> = meta
                .iter()
                .map(|m| dec_opt_map(m).expect("meta map"))
                .collect();
            let r2 = nh_resources::resource_metadata::clone_with_metadata_from_map_if_needed(
                &metas,
                r.clone() as Arc<dyn Resource>,
            );
            let r2 = nh_resources::transform::resource_adapter(&r2).unwrap();
            let fin = rec(&r2, false);
            failures.extend(diff(&format!("{what} final"), &rc["final"], &fin, skip));
            let after = go_json::marshal(&go_value::Value::Map(Resource::params(r.as_ref())))
                .map(|b| String::from_utf8(b).unwrap())
                .unwrap();
            if rc["baseParamsAfter"] != J::String(after.clone()) {
                failures.push(format!(
                    "{what}: base params after the metadata: want {}, got {after}",
                    rc["baseParamsAfter"]
                ));
            }
        }
    }

    failures.extend(diff(
        &format!("{name} published"),
        &fx["published"],
        &site.published_files(),
        &[],
    ));

    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    (records.len(), unsupported)
}

#[test]
fn synth() {
    let (n, u) = run("synth");
    assert!(n >= 40, "{n}");
    eprintln!("synth: {n} resources ({u} GIF sizes not supported)");
}

#[test]
fn docs() {
    let (n, u) = run("docs");
    assert!(n >= 80, "{n}");
    eprintln!("docs: {n} resources ({u} GIF sizes not supported)");
}

#[test]
fn testsite() {
    let (n, _) = run("testsite");
    assert_eq!(n, 0);
}
