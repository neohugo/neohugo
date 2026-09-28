//! Oracle test: the free functions of helpers (general.go, path.go, content.go) and
//! ProcessingStatsTable against `tools/go-oracle/nh-helpers/general`
//! (fixtures/general/general.json.gz).

mod support;

use std::sync::atomic::Ordering;

use nh_helpers::content::{extract_toc, total_words};
use nh_helpers::general::*;
use nh_helpers::path::*;
use nh_helpers::processing_stats::{ProcessingStats, processing_stats_table};
use serde_json::{Value as J, json};
use support::*;

fn s(v: &J) -> String {
    String::from_utf8(gostr(v)).unwrap()
}

fn list(v: &J) -> Vec<String> {
    match v {
        J::Null => Vec::new(),
        J::Array(a) => a.iter().map(s).collect(),
        _ => panic!("list {v}"),
    }
}

fn enc_list(v: &[String]) -> J {
    J::Array(v.iter().map(|x| enc(x.as_bytes())).collect())
}

fn groups(g: Option<Vec<NamedSlice>>) -> J {
    match g {
        None => J::Null,
        Some(g) => J::Array(
            g.iter()
                .map(|n| {
                    json!({
                        "name": enc(n.name.as_bytes()),
                        "slice": n.slice.as_ref().map(|s| enc_list(s)).unwrap_or(J::Null),
                        "string": enc(n.string().as_bytes()),
                    })
                })
                .collect(),
        ),
    }
}

#[test]
fn general_matches_go() {
    let fx = fixture("general", "general.json.gz");
    let styles: Vec<String> = fx["titleStyles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let title_funcs: Vec<TitleFunc> = styles.iter().map(|st| get_title_func(st)).collect();

    let mut n = std::collections::BTreeMap::<String, usize>::new();
    let mut fails = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let f = c["fn"].as_str().unwrap();
        let input = &c["in"];
        let want = &c["r"];
        let got: J = match f {
            "FirstUpper" => call(|| first_upper(&s(input))),
            "MakeTitle" => call(|| make_title(&s(input))),
            "GetTitleFunc" => {
                let x = s(input);
                J::Array(title_funcs.iter().map(|tf| call(|| tf(&x))).collect())
            }
            "TotalWords" => json!(total_words(&gostr(input))),
            "GetDottedRelativePath" => call(|| get_dotted_relative_path(&s(input))),
            "MakePathRelative" => {
                let (rel, err) = make_path_relative(&s(input), &["sect/", "/a/", "docs"]);
                json!([rel, err.map(|e| json!({"err": e.message()}))])
            }
            "UniqueStrings" => enc_list(&unique_strings(&list(input))),
            "UniqueStringsReuse" => enc_list(&unique_strings_reuse(list(input))),
            "UniqueStringsSorted" => match unique_strings_sorted(list(input)) {
                None => J::Null,
                Some(v) => enc_list(&v),
            },
            "SliceToLower" => {
                let l = list(&input["l"]);
                let arg = if input["nil"].as_bool().unwrap() {
                    None
                } else {
                    Some(&l[..])
                };
                match slice_to_lower(arg) {
                    None => J::Null,
                    Some(v) => enc_list(&v),
                }
            }
            "StringSliceToList" => {
                enc(string_slice_to_list(&list(&input[0]), input[1].as_str().unwrap()).as_bytes())
            }
            "HasStringsPrefix" => json!(has_strings_prefix(&list(&input[0]), &list(&input[1]))),
            "HasStringsSuffix" => json!(has_strings_suffix(&list(&input[0]), &list(&input[1]))),
            "ExtractAndGroupRootPaths" => groups(extract_and_group_root_paths(&list(input))),
            "ExtractRootPaths" => enc_list(&extract_root_paths(&list(input))),
            "FormatByteCount" => json!(format_byte_count(
                input.as_str().unwrap().parse::<u64>().unwrap()
            )),
            "ReaderContains" => {
                if input.is_null() {
                    json!(reader_contains(None, b"a"))
                } else {
                    let b = gostr(&input[0]);
                    let sub = gostr(&input[1]);
                    let mut r: &[u8] = &b;
                    json!(reader_contains(Some(&mut r), &sub))
                }
            }
            "ReaderToBytes" | "ReaderToString" => {
                let b = gostr(input);
                let mut r: &[u8] = &b;
                enc(&reader_to_bytes(Some(&mut r)))
            }
            "ExtractTOC" => {
                let (nc, toc) = extract_toc(&gostr(input));
                json!([enc(&nc), toc.map(|t| enc(&t)).unwrap_or(J::Null)])
            }
            "IsWhitespace" => {
                json!(is_whitespace(
                    char::from_u32(input.as_u64().unwrap() as u32).unwrap()
                ))
            }
            "ProcessingStatsTable" => {
                let stats: Vec<ProcessingStats> = match input {
                    J::Null => Vec::new(),
                    J::Array(a) => a
                        .iter()
                        .map(|st| {
                            let p = ProcessingStats::new(st["name"].as_str().unwrap());
                            let vals: Vec<u64> = st["vals"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| v.as_str().unwrap().parse().unwrap())
                                .collect();
                            let counters = [
                                &p.pages,
                                &p.paginator_pages,
                                &p.files,
                                &p.static_,
                                &p.processed_images,
                                &p.aliases,
                                &p.cleaned,
                            ];
                            for (c, v) in counters.iter().zip(vals) {
                                c.store(v, Ordering::Relaxed);
                            }
                            p
                        })
                        .collect(),
                    _ => panic!(),
                };
                let refs: Vec<&ProcessingStats> = stats.iter().collect();
                let mut buf = Vec::new();
                processing_stats_table(&mut buf, &refs);
                enc(&buf)
            }
            _ => panic!("unknown fn {f}"),
        };
        *n.entry(f.to_string()).or_default() += 1;
        if &got != want {
            fails.push(format!("{f}({input}): got {got}, want {want}"));
        }
    }
    eprintln!("general: {n:?}, {} failures", fails.len());
    for f in fails.iter().take(30) {
        eprintln!("  {f}");
    }
    assert!(fails.is_empty());
}
