//! `paths`: the differential tests against `tools/go-oracle/nh-common/paths`
//! (`fixtures/paths/pathparser.json.gz`: every `Path` accessor for every input and parser;
//! `fixtures/paths/strings.json.gz`: the path.go/url.go string helpers over the string corpus)
//! and the Go test tables of common/paths (pathparser_test.go, path_test.go, url_test.go).

mod t02support;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use nh_common::files;
use nh_common::kinds;
use nh_common::paths::path as p;
use nh_common::paths::pathparser::{
    Path, PathParser, PathType, has_ext, modify_path_bundle_type_resource,
    normalize_path_string_basic,
};
use nh_common::paths::url as u;
use serde_json::{Value as J, json};
use t02support::*;

// ---------------------------------------------------------------------------
// PathParser oracle

type Misses = Arc<Mutex<Vec<String>>>;

/// Rebuilds a fixture parser: the language index and the recorded callback answers (a call
/// Go never made is a miss).
fn build_parser(d: &J, misses: &Misses) -> PathParser {
    let language_index = d["languageIndex"].as_object().map(|m| {
        m.iter()
            .map(|(k, v)| (k.clone(), v.as_u64().unwrap() as usize))
            .collect::<BTreeMap<_, _>>()
    });

    let log1 = |k: &str| -> HashMap<String, bool> {
        d[k].as_array()
            .unwrap()
            .iter()
            .map(|e| (e[0].as_str().unwrap().to_string(), e[1].as_bool().unwrap()))
            .collect()
    };
    let of: HashMap<(String, String), bool> = d["outputFormatLog"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                (
                    e[0].as_str().unwrap().to_string(),
                    e[1].as_str().unwrap().to_string(),
                ),
                e[2].as_bool().unwrap(),
            )
        })
        .collect();
    let ce = log1("contentExtLog");
    let ld = log1("langDisabledLog");

    let mut pp = PathParser {
        language_index,
        ..Default::default()
    };
    if d["isLangDisabled"].as_bool().unwrap() {
        let m = misses.clone();
        pp.is_lang_disabled = Some(Arc::new(move |l: &str| {
            ld.get(l).copied().unwrap_or_else(|| {
                m.lock().unwrap().push(format!("IsLangDisabled({l:?})"));
                false
            })
        }));
    }
    if d["isOutputFormat"].as_bool().unwrap() {
        let m = misses.clone();
        pp.is_output_format = Some(Arc::new(move |n: &str, e: &str| {
            of.get(&(n.to_string(), e.to_string()))
                .copied()
                .unwrap_or_else(|| {
                    m.lock()
                        .unwrap()
                        .push(format!("IsOutputFormat({n:?}, {e:?})"));
                    false
                })
        }));
    }
    if d["isContentExt"].as_bool().unwrap() {
        let m = misses.clone();
        pp.is_content_ext = Some(Arc::new(move |e: &str| {
            ce.get(e).copied().unwrap_or_else(|| {
                m.lock().unwrap().push(format!("IsContentExt({e:?})"));
                false
            })
        }));
    }
    pp
}

fn s(v: &str) -> J {
    J::String(v.to_string())
}

fn strs(v: Vec<&str>) -> J {
    J::Array(v.into_iter().map(s).collect())
}

/// One `Path` accessor by its fixture key.
fn accessor(p: &Path, key: &str) -> J {
    match key {
        "Base" => s(&p.base()),
        "BaseNameNoIdentifier" => s(p.base_name_no_identifier()),
        "BaseNoLeadingSlash" => s(&p.base_no_leading_slash()),
        "BaseReTyped:posts" => s(&p.base_re_typed("posts")),
        "BaseReTyped:Posts X" => s(&p.base_re_typed("Posts X")),
        "Component" => s(p.component()),
        "Container" => s(p.container()),
        "ContainerDir" => s(p.container_dir()),
        "Dir" => s(p.dir()),
        "Disabled" => J::Bool(p.disabled()),
        "Ext" => s(p.ext()),
        "Identifier:-1" => s(p.identifier(-1)),
        "Identifier:len" => s(p.identifier(p.identifiers().len() as isize)),
        "IdentifierBase" => s(&p.identifier_base()),
        "Identifiers" => strs(p.identifiers()),
        "IdentifiersUnknown" => strs(p.identifiers_unknown()),
        "IsBranchBundle" => J::Bool(p.is_branch_bundle()),
        "IsBundle" => J::Bool(p.is_bundle()),
        "IsContent" => J::Bool(p.is_content()),
        "IsContentData" => J::Bool(p.is_content_data()),
        "IsLeafBundle" => J::Bool(p.is_leaf_bundle()),
        "Kind" => s(p.kind()),
        "Lang" => s(p.lang()),
        "Layout" => s(p.layout()),
        "Name" => s(p.name()),
        "NameNoExt" => s(p.name_no_ext()),
        "NameNoIdentifier" => s(p.name_no_identifier()),
        "NameNoLang" => s(&p.name_no_lang()),
        "OutputFormat" => s(p.output_format()),
        "Path" => s(p.path()),
        "PathBeforeLangAndOutputFormatAndExt" => s(&p.path_before_lang_and_output_format_and_ext()),
        "PathNoIdentifier" => s(&p.path_no_identifier()),
        "PathNoLang" => s(&p.path_no_lang()),
        "PathNoLeadingSlash" => s(p.path_no_leading_slash()),
        "Section" => s(p.section()),
        "String" => s(&p.to_string()),
        "Type" => s(p.path_type().string()),
        "Unnormalized.Path" => s(p.unnormalized().path()),
        _ => panic!("unknown accessor {key}"),
    }
}

/// Compares `want` (an array in `keys` order) with `p`'s accessors; returns the differences.
fn check_dump(p: &Path, keys: &[J], want: &J, what: &str) -> Vec<String> {
    let want = want.as_array().unwrap();
    assert_eq!(keys.len(), want.len());
    let mut bad = Vec::new();
    for (k, w) in keys.iter().zip(want) {
        let k = k.as_str().unwrap();
        let got = catch(|| accessor(p, k));
        if !same(w, &got) {
            bad.push(format!("{what}.{k}: want {w} got {got:?}"));
        }
    }
    bad
}

#[test]
fn pathparser_matches_go() {
    let f = fixture("paths/pathparser.json.gz");
    let misses: Misses = Arc::new(Mutex::new(Vec::new()));
    let parsers: HashMap<String, PathParser> = f["parsers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, d)| (k.clone(), build_parser(d, &misses)))
        .collect();
    let keys = &f["keys"];
    let out_keys = keys["out"].as_array().unwrap();
    let mini_keys = keys["mini"].as_array().unwrap();
    let tls_keys = keys["tls"].as_array().unwrap();
    let mini_sets: Vec<&str> = keys["miniSets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() > 10_000, "too few cases: {}", cases.len());

    let mut bad = Vec::new();
    let mut checks = 0usize;
    let mut base_rel_cut_rune = 0usize;
    let mut sets: BTreeMap<String, usize> = BTreeMap::new();
    for c in cases {
        let pp = &parsers[c["p"].as_str().unwrap()];
        let comp = c["c"].as_str().unwrap();
        let input = String::from_utf8(bytes(&c["in"])).expect("inputs are UTF-8");
        let set = c["set"].as_str().unwrap();
        *sets.entry(set.to_string()).or_default() += 1;
        let mut errs: Vec<String> = Vec::new();

        let mut eq = |what: &str, want: &J, got: Result<J, String>| {
            checks += 1;
            if !same(want, &got) {
                errs.push(format!("{what}: want {want} got {got:?}"));
            }
        };
        eq(
            "norm",
            &c["norm"],
            Ok(s(&normalize_path_string_basic(&input))),
        );
        eq("hasExt", &c["hasExt"], Ok(J::Bool(has_ext(&input))));
        eq(
            "pb",
            &c["pb"],
            catch(|| {
                let (a, b) = pp.parse_base_and_base_name_no_identifier(comp, &input);
                json!([a, b])
            }),
        );
        eq(
            "pid",
            &c["pid"],
            catch(|| s(&pp.parse_identity(comp, &input).0)),
        );

        let parsed = catch(|| pp.parse(comp, &input));
        if let Some(pw) = c.get("parse") {
            if parsed.is_ok() {
                errs.push(format!("parse: want {pw}, Rust did not panic"));
            }
        } else {
            match parsed {
                Err(e) => errs.push(format!("parse panicked: {e}")),
                Ok(mut p) => {
                    let ks = if mini_sets.contains(&set) {
                        mini_keys
                    } else {
                        out_keys
                    };
                    checks += 2 * ks.len();
                    errs.extend(check_dump(&p, ks, &c["out"], "out"));
                    let self_unnormalized = std::ptr::eq(p.unnormalized(), &p);
                    if c["u"] == "self" {
                        if !self_unnormalized {
                            errs.push("u: want self".to_string());
                        }
                    } else if self_unnormalized {
                        errs.push("u: want a separate unnormalized path".to_string());
                    } else {
                        errs.extend(check_dump(p.unnormalized(), ks, &c["u"], "u"));
                    }

                    if !mini_sets.contains(&set) {
                        checks += tls_keys.len() + 12;
                        let t = p.trim_leading_slash();
                        errs.extend(check_dump(&t, tls_keys, &c["tls"], "tls"));

                        let ft = p.for_type(PathType::Partial);
                        let got = catch(|| {
                            json!([
                                ft.path_type().string(),
                                ft.is_content(),
                                ft.base(),
                                ft.unnormalized().path_type().string()
                            ])
                        });
                        if !same(&c["ft"], &got) {
                            errs.push(format!("ft: want {} got {got:?}", c["ft"]));
                        }

                        let owner = pp.parse(comp, &format!("{}/_index.md", p.dir()));
                        let want = c["rel"].as_array().unwrap();
                        let rel = [
                            Ok(s(&owner.base())),
                            catch(|| s(&p.path_rel(&owner))),
                            catch(|| s(&p.base_rel(&owner))),
                        ];
                        for (i, (w, g)) in want.iter().zip(rel).enumerate() {
                            // Deviation (PORTING.md): Go's `p.Base()[len(ob)+1:]` cuts a rune
                            // when the owner is not an ancestor; the &str port panics there.
                            if i == 2 && w.get("hex").is_some() && g.is_err() {
                                base_rel_cut_rune += 1;
                                continue;
                            }
                            if !same(w, &g) {
                                errs.push(format!("rel[{i}]: want {w} got {g:?}"));
                            }
                        }

                        modify_path_bundle_type_resource(&mut p);
                        let want = c["mod"].as_array().unwrap();
                        let m = [
                            Ok(s(p.path_type().string())),
                            Ok(J::Bool(p.is_content())),
                            catch(|| s(&p.base())),
                            catch(|| s(p.base_name_no_identifier())),
                            Ok(s(p.unnormalized().path_type().string())),
                        ];
                        for (i, (w, g)) in want.iter().zip(m).enumerate() {
                            if !same(w, &g) {
                                errs.push(format!("mod[{i}]: want {w} got {g:?}"));
                            }
                        }
                    }
                }
            }
        }

        if !errs.is_empty() {
            bad.push(format!(
                "{} {} {:?} ({}):\n    {}",
                c["p"],
                comp,
                input,
                set,
                errs.join("\n    ")
            ));
        }
    }

    let misses = misses.lock().unwrap();
    assert!(
        misses.is_empty(),
        "{} callback calls Go never made, e.g. {:?}",
        misses.len(),
        &misses[..misses.len().min(20)]
    );
    assert!(
        bad.is_empty(),
        "{} of {} cases differ:\n{}",
        bad.len(),
        cases.len(),
        bad[..bad.len().min(30)].join("\n")
    );
    // Only the layouts sweep's non-ancestor owners with a Thai file name hit it.
    assert!(base_rel_cut_rune <= 60, "{base_rel_cut_rune}");
    eprintln!(
        "pathparser: {} cases, {checks} checks, {base_rel_cut_rune} BaseRel rune cuts, sets {sets:?}",
        cases.len()
    );
}

// ---------------------------------------------------------------------------
// String helpers oracle

#[test]
fn string_helpers_match_go() {
    let f = fixture("paths/strings.json.gz");
    let roots: Vec<String> = f["contextRoots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let hosts: Vec<String> = f["permalinkHosts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() > 4000, "too few cases: {}", cases.len());

    let mut bad = Vec::new();
    let mut checks = 0usize;
    let mut invalid = 0usize;
    let mut text_only = 0usize;
    for c in cases {
        let b = bytes(&c["in"]);
        let mut errs = Vec::new();
        let mut eq = |k: &str, want: &J, got: Result<J, String>| {
            checks += 1;
            if !same(want, &got) {
                errs.push(format!("{k}: want {want} got {got:?}"));
            }
        };
        let err_or = |r: nh_common::Result<J>| -> Result<J, String> {
            r.map_err(|e| e.message().to_string())
        };

        // Byte-level functions (every input).
        eq("Sanitize", &c["Sanitize"], Ok(enc(&p::sanitize_bytes(&b))));
        eq(
            "MakeTitle",
            &c["MakeTitle"],
            Ok(enc(&p::make_title_bytes(&b))),
        );
        eq(
            "PathEscape",
            &c["PathEscape"],
            err_or(p::try_path_escape(&b).map(|v| s(&v))),
        );
        eq(
            "URLEscape",
            &c["URLEscape"],
            err_or(u::try_url_escape(&b).map(|v| enc(&v))),
        );

        let Ok(st) = std::str::from_utf8(&b) else {
            invalid += 1;
            if !errs.is_empty() {
                bad.push(format!("{:?}: {}", c["in"], errs.join("; ")));
            }
            continue;
        };
        // The &str forms of the byte-level functions.
        if !is_panic(&c["Sanitize"]) {
            eq("sanitize(str)", &c["Sanitize"], Ok(s(&p::sanitize(st))));
            eq(
                "make_title(str)",
                &c["MakeTitle"],
                Ok(s(&p::make_title(st))),
            );
        }
        eq(
            "path_escape(str)",
            &c["PathEscape"],
            catch(|| s(&p::path_escape(st))),
        );
        eq(
            "url_escape(str)",
            &c["URLEscape"],
            catch(|| s(&u::url_escape(st))),
        );
        eq("HasExt", &c["HasExt"], Ok(J::Bool(has_ext(st))));
        eq(
            "NormalizePathStringBasic",
            &c["NormalizePathStringBasic"],
            Ok(s(&normalize_path_string_basic(st))),
        );
        if c.get("textOnly").is_some() {
            text_only += 1;
            if !errs.is_empty() {
                bad.push(format!("{:?}: {}", st, errs.join("; ")));
            }
            continue;
        }

        let pair = |(a, b): (String, String)| json!([a, b]);
        eq(
            "AddLeadingSlash",
            &c["AddLeadingSlash"],
            Ok(s(&p::add_leading_slash(st))),
        );
        eq(
            "AddTrailingSlash",
            &c["AddTrailingSlash"],
            Ok(s(&p::add_trailing_slash(st))),
        );
        eq(
            "AddLeadingAndTrailingSlash",
            &c["AddLeadingAndTrailingSlash"],
            Ok(s(&p::add_leading_and_trailing_slash(st))),
        );
        eq("Ext", &c["Ext"], Ok(s(p::ext(st))));
        eq(
            "ExtNoDelimiter",
            &c["ExtNoDelimiter"],
            Ok(s(p::ext_no_delimiter(st))),
        );
        eq(
            "PathAndExt",
            &c["PathAndExt"],
            Ok(pair(p::path_and_ext(st))),
        );
        eq(
            "FileAndExt",
            &c["FileAndExt"],
            Ok(pair(p::file_and_ext(st))),
        );
        eq(
            "FileAndExtNoDelimiter",
            &c["FileAndExtNoDelimiter"],
            Ok(pair(p::file_and_ext_no_delimiter(st))),
        );
        eq("Filename", &c["Filename"], Ok(s(&p::filename(st))));
        eq(
            "ReplaceExtension",
            &c["ReplaceExtension"],
            Ok(s(&p::replace_extension(st, "xml"))),
        );
        eq("Dir", &c["Dir"], Ok(s(&p::dir(st))));
        eq(
            "FieldsSlash",
            &c["FieldsSlash"],
            Ok(J::Array(p::fields_slash(st).iter().map(|x| s(x)).collect())),
        );
        eq(
            "ToSlashTrimLeading",
            &c["ToSlashTrimLeading"],
            Ok(s(&p::to_slash_trim_leading(st))),
        );
        eq(
            "TrimLeading",
            &c["TrimLeading"],
            Ok(s(&p::trim_leading(st))),
        );
        eq(
            "ToSlashTrimTrailing",
            &c["ToSlashTrimTrailing"],
            Ok(s(&p::to_slash_trim_trailing(st))),
        );
        eq(
            "TrimTrailing",
            &c["TrimTrailing"],
            Ok(s(&p::trim_trailing(st))),
        );
        eq(
            "ToSlashTrim",
            &c["ToSlashTrim"],
            Ok(s(&p::to_slash_trim(st))),
        );
        eq(
            "ToSlashPreserveLeading",
            &c["ToSlashPreserveLeading"],
            Ok(s(&p::to_slash_preserve_leading(st))),
        );
        eq(
            "IsSameFilePath",
            &c["IsSameFilePath"],
            Ok(J::Bool(p::is_same_file_path(st, "/a/b/c"))),
        );
        eq(
            "CommonDirPath",
            &c["CommonDirPath"],
            Ok(s(&p::common_dir_path(st, "/a/b/c"))),
        );
        eq(
            "AbsPathify",
            &c["AbsPathify"],
            Ok(s(&p::abs_pathify("/work/dir", st))),
        );
        let rel = |base: &str| match p::get_relative_path(st, base) {
            Ok(v) => json!({ "ok": v }),
            Err(e) => json!({ "err": e.message() }),
        };
        eq("GetRelativePath", &c["GetRelativePath"], Ok(rel("/a")));
        eq(
            "GetRelativePathNoBase",
            &c["GetRelativePathNoBase"],
            Ok(rel("")),
        );
        eq("TrimExt", &c["TrimExt"], Ok(s(&u::trim_ext(st))));
        eq(
            "PrettifyURLPath",
            &c["PrettifyURLPath"],
            Ok(s(&u::prettify_url_path(st))),
        );
        eq(
            "PrettifyURL",
            &c["PrettifyURL"],
            Ok(s(&u::prettify_url(st))),
        );
        eq("Uglify", &c["Uglify"], Ok(s(&u::uglify(st))));
        eq("UrlStringToFilename", &c["UrlStringToFilename"], {
            let (f, ok) = u::url_string_to_filename(st);
            Ok(json!([enc(&f), ok]))
        });
        eq(
            "UrlFromFilename",
            &c["UrlFromFilename"],
            Ok(match u::url_from_filename(st) {
                Ok(v) => json!({ "ok": enc(&v.string()) }),
                Err(e) => json!({ "err": e.message() }),
            }),
        );
        for (i, base) in roots.iter().enumerate() {
            eq(
                &format!("AddContextRoot[{base}]"),
                &c["AddContextRoot"][i],
                err_or(u::try_add_context_root(base, st).map(|v| enc(&v))),
            );
            if !is_panic(&c["AddContextRoot"][i])
                && std::str::from_utf8(&bytes(&c["AddContextRoot"][i])).is_ok()
            {
                eq(
                    &format!("add_context_root[{base}]"),
                    &c["AddContextRoot"][i],
                    catch(|| s(&u::add_context_root(base, st))),
                );
            }
        }
        for (i, host) in hosts.iter().enumerate() {
            eq(
                &format!("MakePermalink[{host}]"),
                &c["MakePermalink"][i],
                err_or(u::try_make_permalink_url(host, st).map(|v| enc(&v.string()))),
            );
            eq(
                &format!("make_permalink[{host}]"),
                &c["MakePermalink"][i],
                catch(|| s(&u::make_permalink(host, st))),
            );
        }

        if !errs.is_empty() {
            bad.push(format!("{:?}: {}", st, errs.join("; ")));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} inputs differ:\n{}",
        bad.len(),
        cases.len(),
        bad[..bad.len().min(30)].join("\n")
    );
    eprintln!(
        "strings: {} inputs ({text_only} text-only, {invalid} invalid UTF-8), {checks} checks",
        cases.len()
    );
}

// ---------------------------------------------------------------------------
// Go: common/paths/pathparser_test.go

fn test_parser() -> PathParser {
    PathParser {
        language_index: Some(
            [("no", 0usize), ("en", 1), ("fr", 2)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        ),
        is_content_ext: Some(Arc::new(|ext: &str| ext == "md")),
        is_output_format: Some(Arc::new(|name: &str, _ext: &str| {
            matches!(name, "html" | "amp" | "csv" | "rss")
        })),
        ..Default::default()
    }
}

fn parse_content(s: &str) -> Path {
    test_parser().parse(files::COMPONENT_FOLDER_CONTENT, s)
}

fn parse_layouts(s: &str) -> Path {
    test_parser().parse(files::COMPONENT_FOLDER_LAYOUTS, s)
}

// Go: common/paths/pathparser_test.go:TestParse
#[test]
fn go_test_parse() {
    // Basic text file
    let p = parse_content("/a/b.txt");
    assert_eq!(p.name(), "b.txt");
    assert_eq!(p.base(), "/a/b.txt");
    assert_eq!(p.container(), "a");
    assert_eq!(p.dir(), "/a");
    assert_eq!(p.ext(), "txt");
    assert!(!p.is_content());

    // Basic text file, upper case
    let p = parse_content("/A/B.txt");
    assert_eq!(p.name(), "b.txt");
    assert_eq!(p.name_no_ext(), "b");
    assert_eq!(p.name_no_identifier(), "b");
    assert_eq!(p.base_name_no_identifier(), "b");
    assert_eq!(p.base(), "/a/b.txt");
    assert_eq!(p.ext(), "txt");

    // Spaces in dir and filename
    assert_eq!(parse_content("/a b/c.txt").base(), "/a-b/c.txt");
    assert_eq!(parse_content("/a  b/c.txt").base(), "/a--b/c.txt");
    assert_eq!(parse_content("/a/b c.txt").base(), "/a/b-c.txt");
    assert_eq!(parse_content("/a/b  c.txt").base(), "/a/b--c.txt");

    // Basic text file, mixed case and spaces, unnormalized
    let p = parse_content("/a/Foo BAR.txt");
    assert_eq!(p.unnormalized().base_name_no_identifier(), "Foo BAR");

    // Basic Markdown file
    let p = parse_content("/a/b/c.md");
    assert_eq!(p.ext(), "md");
    assert_eq!(p.path_type(), PathType::ContentSingle);
    assert!(p.is_content());
    assert!(!p.is_leaf_bundle());
    assert_eq!(p.name(), "c.md");
    assert_eq!(p.base(), "/a/b/c");
    assert_eq!(p.base_re_typed("foo"), "/foo/b/c");
    assert_eq!(p.section(), "a");
    assert_eq!(p.base_name_no_identifier(), "c");
    assert_eq!(p.path(), "/a/b/c.md");
    assert_eq!(p.dir(), "/a/b");
    assert_eq!(p.container(), "b");
    assert_eq!(p.container_dir(), "/a/b");

    // Content resource
    let mut p = parse_content("/a/b.md");
    assert_eq!(p.name(), "b.md");
    assert_eq!(p.base(), "/a/b");
    assert_eq!(p.base_no_leading_slash(), "a/b");
    assert_eq!(p.section(), "a");
    assert_eq!(p.base_name_no_identifier(), "b");
    // Reclassify it as a content resource.
    modify_path_bundle_type_resource(&mut p);
    assert_eq!(p.path_type(), PathType::ContentResource);
    assert!(p.is_content());
    assert_eq!(p.name(), "b.md");
    assert_eq!(p.base(), "/a/b.md");

    // No ext
    let p = parse_content("/a/b");
    assert_eq!(p.name(), "b");
    assert_eq!(p.name_no_ext(), "b");
    assert_eq!(p.base(), "/a/b");
    assert_eq!(p.ext(), "");

    // No ext, trailing slash
    let p = parse_content("/a/b/");
    assert_eq!(p.name(), "b");
    assert_eq!(p.base(), "/a/b");
    assert_eq!(p.ext(), "");

    // Identifiers
    let p = parse_content("/a/b.a.b.no.txt");
    assert_eq!(p.name(), "b.a.b.no.txt");
    assert_eq!(p.name_no_identifier(), "b.a.b");
    assert_eq!(p.name_no_lang(), "b.a.b.txt");
    assert_eq!(p.identifiers(), vec!["txt", "no"]);
    assert_eq!(p.identifiers_unknown(), vec!["b", "a", "b"]);
    assert_eq!(p.base(), "/a/b.a.b.txt");
    assert_eq!(p.base_no_leading_slash(), "a/b.a.b.txt");
    assert_eq!(p.path(), "/a/b.a.b.no.txt");
    assert_eq!(p.path_no_lang(), "/a/b.a.b.txt");
    assert_eq!(p.ext(), "txt");
    assert_eq!(p.path_no_identifier(), "/a/b.a.b");

    // Home branch cundle
    let p = parse_content("/_index.md");
    assert_eq!(p.identifiers(), vec!["md"]);
    assert!(p.is_branch_bundle());
    assert!(p.is_bundle());
    assert_eq!(p.base(), "/");
    assert_eq!(p.base_re_typed("foo"), "/foo");
    assert_eq!(p.path(), "/_index.md");
    assert_eq!(p.container(), "");
    assert_eq!(p.container_dir(), "/");

    // Index content file in root
    let p = parse_content("/a/index.md");
    assert_eq!(p.base(), "/a");
    assert_eq!(p.base_re_typed("foo"), "/foo/a");
    assert_eq!(p.base_name_no_identifier(), "a");
    assert_eq!(p.container(), "a");
    assert_eq!(p.container_dir(), "");
    assert_eq!(p.dir(), "/a");
    assert_eq!(p.ext(), "md");
    assert_eq!(p.identifiers_unknown(), vec!["index"]);
    assert_eq!(p.identifiers(), vec!["md"]);
    assert!(!p.is_branch_bundle());
    assert!(p.is_bundle());
    assert!(p.is_leaf_bundle());
    assert_eq!(p.lang(), "");
    assert_eq!(p.name_no_ext(), "index");
    assert_eq!(p.name_no_identifier(), "index");
    assert_eq!(p.name_no_lang(), "index.md");
    assert_eq!(p.section(), "");

    // Index content file with lang
    let p = parse_content("/a/b/index.no.md");
    assert_eq!(p.base(), "/a/b");
    assert_eq!(p.base_name_no_identifier(), "b");
    assert_eq!(p.base_re_typed("foo"), "/foo/b");
    assert_eq!(p.container(), "b");
    assert_eq!(p.container_dir(), "/a");
    assert_eq!(p.dir(), "/a/b");
    assert_eq!(p.ext(), "md");
    assert_eq!(p.identifiers(), vec!["md", "no"]);
    assert!(!p.is_branch_bundle());
    assert!(p.is_bundle());
    assert!(p.is_leaf_bundle());
    assert_eq!(p.lang(), "no");
    assert_eq!(p.name_no_ext(), "index.no");
    assert_eq!(p.name_no_identifier(), "index");
    assert_eq!(p.name_no_lang(), "index.md");
    assert_eq!(p.path(), "/a/b/index.no.md");
    assert_eq!(p.path_no_lang(), "/a/b/index.md");
    assert_eq!(p.section(), "a");

    // Index branch content file
    let p = parse_content("/a/b/_index.no.md");
    assert_eq!(p.base(), "/a/b");
    assert_eq!(p.base_name_no_identifier(), "b");
    assert_eq!(p.container(), "b");
    assert_eq!(p.container_dir(), "/a");
    assert_eq!(p.ext(), "md");
    assert_eq!(p.identifiers(), vec!["md", "no"]);
    assert!(p.is_branch_bundle());
    assert!(p.is_bundle());
    assert!(!p.is_leaf_bundle());
    assert_eq!(p.name_no_ext(), "_index.no");
    assert_eq!(p.name_no_lang(), "_index.md");

    // Index root no slash / Index root
    for s in ["_index.md", "/_index.md"] {
        let p = parse_content(s);
        assert_eq!(p.base(), "/");
        assert_eq!(p.ext(), "md");
        assert_eq!(p.name(), "_index.md");
    }

    // Index first
    assert_eq!(parse_content("/a/_index.md").section(), "a");

    // Index text file
    let p = parse_content("/a/b/index.no.txt");
    assert_eq!(p.base(), "/a/b/index.txt");
    assert_eq!(p.ext(), "txt");
    assert_eq!(p.identifiers(), vec!["txt", "no"]);
    assert!(!p.is_leaf_bundle());
    assert_eq!(p.path_no_identifier(), "/a/b/index");

    // Empty
    let p = parse_content("");
    assert_eq!(p.base(), "/");
    assert_eq!(p.ext(), "");
    assert_eq!(p.name(), "");
    assert_eq!(p.path(), "/");

    // Slash
    let p = parse_content("/");
    assert_eq!(p.base(), "/");
    assert_eq!(p.ext(), "");
    assert_eq!(p.name(), "");

    // Trim Leading Slash bundle
    let p = parse_content("foo/bar/index.no.md");
    assert_eq!(p.path(), "/foo/bar/index.no.md");
    let pp = p.trim_leading_slash();
    assert_eq!(pp.path(), "foo/bar/index.no.md");
    assert_eq!(pp.path_no_lang(), "foo/bar/index.md");
    assert_eq!(pp.base(), "foo/bar");
    assert_eq!(pp.dir(), "foo/bar");
    assert_eq!(pp.container_dir(), "foo");
    assert_eq!(pp.container(), "bar");
    assert_eq!(pp.base_name_no_identifier(), "bar");

    // Trim Leading Slash file
    let p = parse_content("foo/bar.txt");
    assert_eq!(p.path(), "/foo/bar.txt");
    let pp = p.trim_leading_slash();
    assert_eq!(pp.path(), "foo/bar.txt");
    assert_eq!(pp.path_no_lang(), "foo/bar.txt");
    assert_eq!(pp.base(), "foo/bar.txt");
    assert_eq!(pp.dir(), "foo");
    assert_eq!(pp.container_dir(), "foo");
    assert_eq!(pp.container(), "foo");
    assert_eq!(pp.base_name_no_identifier(), "bar");

    // File separator (filepath.FromSlash is the identity on unix)
    let p = parse_content("/a/b/c.txt");
    assert_eq!(p.base(), "/a/b/c.txt");
    assert_eq!(p.ext(), "txt");
    assert_eq!(p.name(), "c.txt");
    assert_eq!(p.path(), "/a/b/c.txt");

    // Content data file gotmpl
    let p = parse_content("/a/b/_content.gotmpl");
    assert_eq!(p.path(), "/a/b/_content.gotmpl");
    assert_eq!(p.ext(), "gotmpl");
    assert!(p.is_content_data());

    // Content data file yaml
    assert!(!parse_content("/a/b/_content.yaml").is_content_data());
}

// Go: common/paths/pathparser_test.go:TestParseLayouts
#[test]
fn go_test_parse_layouts() {
    let empty: Vec<&str> = vec![];

    let p = parse_layouts("/list.html");
    assert_eq!(p.base(), "/list.html");
    assert_eq!(p.output_format(), "html");

    let p = parse_layouts("/list.no.html");
    assert_eq!(p.identifiers(), vec!["html", "no", "list"]);
    assert_eq!(p.identifiers_unknown(), empty);
    assert_eq!(p.base(), "/list.html");
    assert_eq!(p.lang(), "no");

    let p = parse_layouts("/section.no.html");
    assert_eq!(p.kind(), kinds::KIND_SECTION);
    assert_eq!(p.identifiers(), vec!["html", "no", "section"]);
    assert_eq!(p.identifiers_unknown(), empty);
    assert_eq!(p.base(), "/section.html");
    assert_eq!(p.lang(), "no");

    let p = parse_layouts("/list.section.no.html");
    assert_eq!(p.layout(), "list");
    assert_eq!(p.identifiers(), vec!["html", "no", "section", "list"]);
    assert_eq!(p.identifiers_unknown(), empty);
    assert_eq!(p.base(), "/list.html");
    assert_eq!(p.lang(), "no");

    let p = parse_layouts("/mylayout.list.section.no.html");
    assert_eq!(p.layout(), "mylayout");
    assert_eq!(
        p.identifiers(),
        vec!["html", "no", "section", "list", "mylayout"]
    );
    assert_eq!(p.identifiers_unknown(), empty);
    assert_eq!(p.base(), "/mylayout.html");
    assert_eq!(p.lang(), "no");

    assert_eq!(
        parse_layouts("/_shortcodes/myshort.list.no.html").layout(),
        "list"
    );
    assert_eq!(parse_layouts("/baseof.list.no.html").layout(), "list");

    let p = parse_layouts("/list.no.amp.not.html");
    assert_eq!(p.identifiers(), vec!["html", "not", "amp", "no", "list"]);
    assert_eq!(p.output_format(), "amp");
    assert_eq!(p.ext(), "html");
    assert_eq!(p.lang(), "no");
    assert_eq!(p.base(), "/list.html");

    let p = parse_layouts("/term.html");
    assert_eq!(p.base(), "/term.html");
    assert_eq!(p.identifiers(), vec!["html", "term"]);
    assert_eq!(p.path_no_identifier(), "/term");
    assert_eq!(p.path_before_lang_and_output_format_and_ext(), "/term");
    assert_eq!(p.lang(), "");
    assert_eq!(p.kind(), "term");
    assert_eq!(p.output_format(), "html");

    let p = parse_layouts("/_shortcodes/myshortcode.list.html");
    assert_eq!(p.base(), "/_shortcodes/myshortcode.html");
    assert_eq!(p.path_type(), PathType::Shortcode);
    assert_eq!(p.identifiers(), vec!["html", "list"]);
    assert_eq!(p.layout(), "list");
    assert_eq!(p.path_no_identifier(), "/_shortcodes/myshortcode");
    assert_eq!(
        p.path_before_lang_and_output_format_and_ext(),
        "/_shortcodes/myshortcode.list"
    );
    assert_eq!(p.lang(), "");
    assert_eq!(p.kind(), "");
    assert_eq!(p.output_format(), "html");

    let p = parse_layouts("/pages/home.html");
    assert_eq!(p.identifiers(), vec!["html", "home"]);
    assert_eq!(p.lang(), "");
    assert_eq!(p.kind(), "home");
    assert_eq!(p.output_format(), "html");
    assert_eq!(p.dir(), "/pages");

    let p = parse_layouts("/pages/baseof.list.section.fr.amp.html");
    assert_eq!(
        p.identifiers(),
        vec!["html", "amp", "fr", "section", "list", "baseof"]
    );
    assert_eq!(p.identifiers_unknown(), empty);
    assert_eq!(p.kind(), kinds::KIND_SECTION);
    assert_eq!(p.lang(), "fr");
    assert_eq!(p.output_format(), "amp");
    assert_eq!(p.dir(), "/pages");
    assert_eq!(p.name_no_identifier(), "baseof");
    assert_eq!(p.path_type(), PathType::Baseof);
    assert_eq!(
        p.identifier_base(),
        "/pages/baseof.list.section.fr.amp.html"
    );

    assert_eq!(
        parse_layouts("/_markup/render-link.html").path_type(),
        PathType::Markup
    );
    assert_eq!(
        parse_layouts("/foo/_markup/render-link.html").path_type(),
        PathType::Markup
    );
    for s in [
        "/_shortcodes/myshortcode.html",
        "/foo/_shortcodes/myshortcode.html",
        "/foo/_shortcodes/foo/myshortcode.html",
    ] {
        assert_eq!(parse_layouts(s).path_type(), PathType::Shortcode, "{s}");
    }
    assert_eq!(
        parse_layouts("/_partials/foo.bar").path_type(),
        PathType::Partial
    );

    let p = parse_layouts("/_shortcodes/no.html");
    assert_eq!(p.path_type(), PathType::Shortcode);
    assert_eq!(p.lang(), "");
    assert_eq!(p.name_no_identifier(), "no");

    let p = parse_layouts("/_shortcodes/myshortcode.no.html");
    assert_eq!(p.path_type(), PathType::Shortcode);
    assert_eq!(p.lang(), "no");
    assert_eq!(p.layout(), "");
    assert_eq!(p.name_no_identifier(), "myshortcode");
}

// Go: common/paths/pathparser_test.go:TestHasExt
#[test]
fn go_test_has_ext() {
    assert!(has_ext("/a/b/c.txt"));
    assert!(has_ext("/a/b.c/d.txt"));
    assert!(!has_ext("/a/b/c"));
    assert!(!has_ext("/a/b.c/d"));
}

/// A nil `IsOutputFormat` panics like Go's nil func call (layouts only).
#[test]
fn nil_callbacks() {
    let pp = PathParser {
        is_content_ext: Some(Arc::new(|_: &str| true)),
        ..Default::default()
    };
    // Go: source/fileInfo.go contentPathParser.
    let p = pp.parse("content", "/a/b.en.md");
    assert_eq!(p.lang(), "");
    assert_eq!(p.identifiers_unknown(), vec!["en", "b"]);
    assert!(catch(|| pp.parse("layouts", "/a/b.html")).is_err());
}

// ---------------------------------------------------------------------------
// Go: common/paths/path_test.go

// Go: common/paths/path_test.go:TestGetRelativePath
#[test]
fn go_test_get_relative_path() {
    for (path, base, expect) in [
        ("/a/b", "/a", Some("b")),
        ("/a/b/c/", "/a", Some("b/c/")),
        ("/c", "/a/b", Some("../../c")),
        ("/c", "", None),
    ] {
        let r = p::get_relative_path(path, base);
        match expect {
            None => assert!(r.is_err(), "{path} {base}"),
            Some(e) => assert_eq!(r.unwrap(), e),
        }
    }
}

// Go: common/paths/path_test.go:TestMakePathRelative
#[test]
fn go_test_make_path_relative() {
    for (in_path, path1, path2, output) in [
        ("/abc/bcd/ab.css", "/abc/bcd", "/bbc/bcd", "/ab.css"),
        ("/abc/bcd/ab.css", "/abcd/bcd", "/abc/bcd", "/ab.css"),
    ] {
        assert_eq!(
            p::make_path_relative(in_path, &[path1, path2]).unwrap(),
            output
        );
    }
    assert!(p::make_path_relative("a/b/c.ss", &["/a/c", "/d/c", "/e/f"]).is_err());
}

// Go: common/paths/path_test.go:TestMakeTitle
#[test]
fn go_test_make_title() {
    for (input, expected) in [
        ("Make-Title", "Make Title"),
        ("MakeTitle", "MakeTitle"),
        ("make_title", "make_title"),
    ] {
        assert_eq!(p::make_title(input), expected);
    }
}

// Go: common/paths/path_test.go:TestReplaceExtension
#[test]
fn go_test_replace_extension() {
    for (input, newext, expected) in [
        ("/some/random/path/file.xml", "html", "file.html"),
        ("/banana.html", "xml", "banana.xml"),
        ("./banana.html", "xml", "banana.xml"),
        ("banana/pie/index.html", "xml", "index.xml"),
        ("../pies/fish/index.html", "xml", "index.xml"),
        (
            "filename-without-an-ext",
            "ext",
            "filename-without-an-ext.ext",
        ),
        (
            "/filename-without-an-ext",
            "ext",
            "filename-without-an-ext.ext",
        ),
        ("/directory/mydir/", "ext", ".ext"),
        ("mydir/", "ext", ".ext"),
    ] {
        assert_eq!(p::replace_extension(input, newext), expected, "{input}");
    }
}

// Go: common/paths/path_test.go:TestExtNoDelimiter
#[test]
fn go_test_ext_no_delimiter() {
    assert_eq!(p::ext_no_delimiter("/my/data.json"), "json");
}

const FILE_AND_EXT: [(&str, &str, &str); 14] = [
    ("index.html", "index", ".html"),
    ("./index.html", "index", ".html"),
    ("/index.html", "index", ".html"),
    ("index", "index", ""),
    ("/tmp/index.html", "index", ".html"),
    ("./filename-no-ext", "filename-no-ext", ""),
    ("/filename-no-ext", "filename-no-ext", ""),
    ("filename-no-ext", "filename-no-ext", ""),
    ("directory/", "", ""),
    ("directory/.hidden.ext", ".hidden", ".ext"),
    ("./directory/../~/banana/gold.fish", "gold", ".fish"),
    ("../directory/banana.man", "banana", ".man"),
    ("~/mydir/filename.ext", "filename", ".ext"),
    ("./directory//tmp/filename.ext", "filename", ".ext"),
];

// Go: common/paths/path_test.go:TestFilename
#[test]
fn go_test_filename() {
    for (input, expected, _) in FILE_AND_EXT {
        assert_eq!(p::filename(input), expected, "{input}");
    }
}

// Go: common/paths/path_test.go:TestFileAndExt
#[test]
fn go_test_file_and_ext() {
    for (input, file, ext) in FILE_AND_EXT {
        assert_eq!(
            p::file_and_ext(input),
            (file.to_string(), ext.to_string()),
            "{input}"
        );
    }
}

// Go: common/paths/path_test.go:TestSanitize
#[test]
fn go_test_sanitize() {
    for (input, expected) in [
        ("  Foo bar  ", "Foo-bar"),
        ("Foo.Bar/foo_Bar-Foo", "Foo.Bar/foo_Bar-Foo"),
        ("fOO,bar:foobAR", "fOObarfoobAR"),
        ("FOo/BaR.html", "FOo/BaR.html"),
        ("FOo/Ba---R.html", "FOo/Ba---R.html"),
        ("FOo/Ba       R.html", "FOo/Ba-R.html"),
        ("трям/трям", "трям/трям"),
        ("은행", "은행"),
        ("Банковский кассир", "Банковский-кассир"),
        ("संस्कृत", "संस्कृत"),
        ("a%C3%B1ame", "a%C3%B1ame"),
        ("this+is+a+test", "this+is+a+test"),
        ("~foo", "~foo"),
    ] {
        assert_eq!(p::sanitize(input), expected, "{input}");
    }
    // Go: BenchmarkSanitize's expectations.
    assert_eq!(p::sanitize("foo/bar"), "foo/bar");
    assert_eq!(p::sanitize("foo bar"), "foo-bar");
}

// Go: common/paths/path_test.go:TestDir
#[test]
fn go_test_dir() {
    assert_eq!(p::dir("/a/b/c/d"), "/a/b/c");
    assert_eq!(p::dir("/a"), "/");
    assert_eq!(p::dir("/"), "/");
    assert_eq!(p::dir(""), "");
}

// Go: common/paths/path_test.go:TestFieldsSlash
#[test]
fn go_test_fields_slash() {
    let abc = vec!["a", "b", "c"];
    assert_eq!(p::fields_slash("a/b/c"), abc);
    assert_eq!(p::fields_slash("/a/b/c"), abc);
    assert_eq!(p::fields_slash("/a/b/c/"), abc);
    assert_eq!(p::fields_slash("a/b/c/"), abc);
    assert!(p::fields_slash("/").is_empty());
    assert!(p::fields_slash("").is_empty());
}

// Go: common/paths/path_test.go:TestCommonDirPath
#[test]
fn go_test_common_dir_path() {
    for (a, b, expected) in [
        ("/a/b/c", "/a/b/d", "/a/b"),
        ("/a/b/c", "a/b/d", "/a/b"),
        ("a/b/c", "/a/b/d", "/a/b"),
        ("a/b/c", "a/b/d", "a/b"),
        ("/a/b/c", "/a/b/c", "/a/b/c"),
        ("/a/b/c", "/a/b/c/d", "/a/b/c"),
        ("/a/b/c", "/a/b", "/a/b"),
        ("/a/b/c", "/a", "/a"),
        ("/a/b/c", "/d/e/f", ""),
    ] {
        assert_eq!(p::common_dir_path(a, b), expected, "a: {a} b: {b}");
    }
}

// Go: common/paths/path_test.go:TestIsSameFilePath
#[test]
fn go_test_is_same_file_path() {
    for (a, b, expected) in [
        ("/a/b/c", "/a/b/c", true),
        ("/a/b/c", "/a/b/c/", true),
        ("/a/b/c", "/a/b/d", false),
        ("/a/b/c", "/a/b", false),
        ("/a/b/c", "/a/b/c/d", false),
        ("/a/b/c", "/a/b/cd", false),
        ("/a/b/c", "/a/b/cc", false),
        ("/a/b/c", "/a/b/c/", true),
        ("/a/b/c", "/a/b/c//", true),
        ("/a/b/c", "/a/b/c/.", true),
        ("/a/b/c", "/a/b/c/./", true),
        ("/a/b/c", "/a/b/c/./.", true),
        ("/a/b/c", "/a/b/c/././", true),
        ("/a/b/c", "/a/b/c/././.", true),
        ("/a/b/c", "/a/b/c/./././", true),
        ("/a/b/c", "/a/b/c/./././.", true),
        ("/a/b/c", "/a/b/c/././././", true),
    ] {
        assert_eq!(p::is_same_file_path(a, b), expected, "a: {a} b: {b}");
    }
}

// ---------------------------------------------------------------------------
// Go: common/paths/url_test.go

// Go: common/paths/url_test.go:TestMakePermalink
#[test]
fn go_test_make_permalink() {
    for (host, link, output) in [
        (
            "http://abc.com/foo",
            "post/bar",
            "http://abc.com/foo/post/bar",
        ),
        (
            "http://abc.com/foo/",
            "post/bar",
            "http://abc.com/foo/post/bar",
        ),
        ("http://abc.com", "post/bar", "http://abc.com/post/bar"),
        ("http://abc.com", "bar", "http://abc.com/bar"),
        (
            "http://abc.com/foo/bar",
            "post/bar",
            "http://abc.com/foo/bar/post/bar",
        ),
        (
            "http://abc.com/foo/bar",
            "post/bar/",
            "http://abc.com/foo/bar/post/bar/",
        ),
        (
            "http://abc.com/foo",
            "post/bar?a=b#c",
            "http://abc.com/foo/post/bar?a=b#c",
        ),
    ] {
        assert_eq!(u::make_permalink(host, link), output);
    }
}

// Go: common/paths/url_test.go:TestAddContextRoot
#[test]
fn go_test_add_context_root() {
    for (base_url, url, expected) in [
        ("http://example.com/sub/", "/foo", "/sub/foo"),
        (
            "http://example.com/sub/",
            "/foo/index.html",
            "/sub/foo/index.html",
        ),
        ("http://example.com/sub1/sub2", "/foo", "/sub1/sub2/foo"),
        ("http://example.com", "/foo", "/foo"),
        // cannot guess that the context root is already added int the example below
        ("http://example.com/sub/", "/sub/foo", "/sub/sub/foo"),
        ("http://example.com/тря", "/трям/", "/тря/трям/"),
        ("http://example.com", "/", "/"),
        ("http://example.com/bar", "//", "/bar/"),
    ] {
        assert_eq!(u::add_context_root(base_url, url), expected);
    }
}

// Go: common/paths/url_test.go:TestPretty
#[test]
fn go_test_pretty() {
    assert_eq!(
        u::prettify_url_path("/section/name.html"),
        "/section/name/index.html"
    );
    assert_eq!(
        u::prettify_url_path("/section/sub/name.html"),
        "/section/sub/name/index.html"
    );
    assert_eq!(
        u::prettify_url_path("/section/name/"),
        "/section/name/index.html"
    );
    assert_eq!(
        u::prettify_url_path("/section/name/index.html"),
        "/section/name/index.html"
    );
    assert_eq!(u::prettify_url_path("/index.html"), "/index.html");
    assert_eq!(u::prettify_url_path("/name.xml"), "/name/index.xml");
    assert_eq!(u::prettify_url_path("/"), "/");
    assert_eq!(u::prettify_url_path(""), "/");
    assert_eq!(u::prettify_url("/section/name.html"), "/section/name");
    assert_eq!(
        u::prettify_url("/section/sub/name.html"),
        "/section/sub/name"
    );
    assert_eq!(u::prettify_url("/section/name/"), "/section/name");
    assert_eq!(u::prettify_url("/section/name/index.html"), "/section/name");
    assert_eq!(u::prettify_url("/index.html"), "/");
    assert_eq!(u::prettify_url("/name.xml"), "/name/index.xml");
    assert_eq!(u::prettify_url("/"), "/");
    assert_eq!(u::prettify_url(""), "/");
}

// Go: common/paths/url_test.go:TestUgly
#[test]
fn go_test_ugly() {
    assert_eq!(u::uglify("/section/name.html"), "/section/name.html");
    assert_eq!(
        u::uglify("/section/sub/name.html"),
        "/section/sub/name.html"
    );
    assert_eq!(u::uglify("/section/name/"), "/section/name.html");
    assert_eq!(u::uglify("/section/name/index.html"), "/section/name.html");
    assert_eq!(u::uglify("/index.html"), "/index.html");
    assert_eq!(u::uglify("/name.xml"), "/name.xml");
    assert_eq!(u::uglify("/"), "/");
    assert_eq!(u::uglify(""), "/");
}
