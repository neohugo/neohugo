//! Acceptance for the seeksnack site (private repository): the reconstructed `hugo.toml`
//! (tests/fixtures/load/seeksnack/hugo.toml, rebuilt from the decoded Go dumps of the real
//! site) loaded like the golden build (`--minify --clock 2026-09-27T12:00:00Z`, environment
//! production) must give
//!
//! * the imaging config `SourceHash` `4bf645f71319dd1d` (it names every processed image),
//! * `neohugo config --format json [--printZero] [--lang th]` output identical to the
//!   committed dumps of the real site (docs/rust-port/specs/architecture-core-data, copied to
//!   the fixture dir), apart from the machine paths `workingdir`/`cachedir`,
//! * `neohugo config mounts` identical to the committed mounts dump (apart from `dir`).

mod support;

use std::sync::Arc;

use nh_allconfig::allconfig::Configs;
use serde_json::Value as J;
use support::*;

/// The seeksnack case of the `load` fixture (tree and descriptor).
fn seeksnack_case(name: &str) -> J {
    let fx = load_fixture(&fixture_dir("load").join("seeksnack.json.gz"));
    fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["case"]["name"] == name)
        .unwrap_or_else(|| panic!("case {name}"))["case"]
        .clone()
}

fn read(name: &str) -> String {
    std::fs::read_to_string(fixture_dir("load").join("seeksnack").join(name)).unwrap()
}

/// The committed dump with this machine's `workingdir` and `cachedir`.
fn with_paths(committed: &str, confs: &Configs) -> String {
    let wd = &confs.base.root.common_dirs.working_dir;
    let cd = &confs.base.root.common_dirs.cache_dir;
    let mut out = String::new();
    for line in committed.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let comma = if line.trim_end().ends_with(',') {
            ","
        } else {
            ""
        };
        if trimmed.starts_with("\"workingdir\": ") {
            out.push_str(&format!("  \"workingdir\": {}{comma}\n", go_json_str(wd)));
        } else if trimmed.starts_with("\"cachedir\": ") {
            out.push_str(&format!("  \"cachedir\": {}{comma}\n", go_json_str(cd)));
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Sorts the runs of consecutive `_jsconfig` mount objects of an indented JSON dump (their
/// order is the OS directory order of the site root; APFS on the golden machine lists them
/// sorted), keeping everything else byte for byte.
fn sort_jsconfig_blocks(s: &str) -> String {
    let lines: Vec<&str> = s.split_inclusive('\n').collect();
    // Object blocks: a line that is only `{` (after indentation) up to its closing `}`/`},`
    // at the same indentation.
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let mut run: Vec<String> = Vec::new();
        let mut trailing_comma = false;
        let mut j = i;
        while j < lines.len() && lines[j].trim() == "{" {
            let indent = &lines[j][..lines[j].len() - lines[j].trim_start().len()];
            let mut k = j + 1;
            while k < lines.len()
                && !(lines[k].starts_with(indent)
                    && (lines[k].trim() == "}" || lines[k].trim() == "},")
                    && lines[k].len() - lines[k].trim_start().len() == indent.len())
            {
                k += 1;
            }
            if k == lines.len() {
                break;
            }
            let block: String = lines[j..=k].concat();
            if !block.contains("\"target\": \"assets/_jsconfig/") {
                break;
            }
            trailing_comma = lines[k].trim() == "},";
            run.push(block.trim_end().trim_end_matches(',').to_string());
            j = k + 1;
        }
        if run.len() > 1 {
            run.sort();
            let n = run.len();
            for (x, b) in run.into_iter().enumerate() {
                let comma = if x + 1 < n || trailing_comma { "," } else { "" };
                out.push(format!("{b}{comma}\n"));
            }
            i = j;
        } else {
            out.push(lines[i].to_string());
            i += 1;
        }
    }
    out.concat()
}

fn go_json_str(s: &str) -> String {
    String::from_utf8(go_json::marshal(&go_value::Value::string(s)).unwrap()).unwrap()
}

#[test]
fn imaging_source_hash_and_committed_dumps() {
    for case in ["seeksnack/config", "seeksnack/build"] {
        let c = seeksnack_case(case);
        let loaded = load_case(&c);
        let confs = loaded.result.as_ref().unwrap();

        // ACCEPT: imaging SourceHash 4bf645f71319dd1d (base and every language config).
        let hash = |c: &Arc<nh_allconfig::allconfig::Config>| {
            c.imaging.as_ref().unwrap().source_hash.clone()
        };
        assert_eq!(hash(&confs.base), "4bf645f71319dd1d", "{case}");
        for (lang, lc) in &confs.language_config_map {
            assert_eq!(hash(lc), "4bf645f71319dd1d", "{case} {lang}");
        }

        // The languages of the build.
        let langs: Vec<&str> = confs.languages.iter().map(|l| l.lang.as_str()).collect();
        assert_eq!(langs, ["en", "th"]);
        assert!(!confs.is_multihost);

        if case == "seeksnack/config" {
            // ACCEPT: every section dump identical (`neohugo config` of the real site).
            for (file, lang, zero) in [
                ("config-en-printzero.json", "", true),
                ("config-en.json", "", false),
                ("config-th.json", "th", false),
            ] {
                let got = nh_allconfig::json::config_dump(confs, lang, zero).unwrap();
                let want = with_paths(&read(file), confs);
                assert_eq!(
                    sort_jsconfig_blocks(&got),
                    sort_jsconfig_blocks(&want),
                    "{file}"
                );
            }
        } else {
            let base = &confs.base;
            assert!(base.minify.minify_output);
            assert_eq!(
                go_time::GoTimeExt::format(&base.compiled().clock, go_time::RFC3339),
                "2026-09-27T12:00:00Z"
            );
        }

        // ACCEPT: the mount list identical to `neohugo config mounts`. The two `_jsconfig`
        // mounts come from a directory listing in OS order (APFS on the golden machine): they
        // are compared as a set.
        let got: J =
            serde_json::from_str(&nh_allconfig::json::mounts_dump(confs).unwrap()).unwrap();
        let mut want: J = serde_json::from_str(&read("config-mounts.json")).unwrap();
        want["dir"] = got["dir"].clone();
        let sorted = |v: &J| {
            let mut m: Vec<J> = v["mounts"].as_array().unwrap().clone();
            let tail = m.split_off(8);
            let mut tail: Vec<String> = tail.iter().map(|x| x.to_string()).collect();
            tail.sort();
            (
                m,
                tail,
                v["path"].clone(),
                v["owner"].clone(),
                v["time"].clone(),
            )
        };
        assert_eq!(sorted(&got), sorted(&want));
        assert_eq!(confs.modules.len(), 1);
    }
}
