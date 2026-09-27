//! util/ and renderer/html helpers vs Go (tools/go-oracle/goldmark util.go):
//! URLEscape (incl. its quirks), numeric/entity reference resolution,
//! case folding, FindClosure, indentation helpers, the HTML text writers,
//! IDs.Generate and the attribute filters.

mod common;

use common::fixture;
use goldmark::ast::KIND_HEADING;
use goldmark::ast::KIND_PARAGRAPH;
use goldmark::renderer::html;
use goldmark::util;

fn s(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn writer_out(f: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    f(&mut v);
    v
}

#[test]
fn util_vectors() {
    let recs = fixture("util.gmf.gz");
    let esc = html::new_writer(vec![html::with_escaped_space()]);
    let mut n = 0;
    let mut fails = Vec::new();
    for r in recs.iter().filter(|r| r.name.starts_with("util/")) {
        let v = r.get("in");
        let mut got: Vec<(String, Vec<u8>)> = vec![
            ("URLEscape/true".into(), util::url_escape(v, true)),
            ("URLEscape/false".into(), util::url_escape(v, false)),
            (
                "ResolveNumericReferences".into(),
                util::resolve_numeric_references(v).into_owned(),
            ),
            (
                "ResolveEntityNames".into(),
                util::resolve_entity_names(v).into_owned(),
            ),
            (
                "UnescapePunctuations".into(),
                util::unescape_punctuations(v).into_owned(),
            ),
            ("EscapeHTML".into(), util::escape_html(v).into_owned()),
            ("ToLinkReference".into(), util::to_link_reference(v)),
            (
                "DoFullUnicodeCaseFolding".into(),
                util::do_full_unicode_case_folding(v).into_owned(),
            ),
            (
                "ReplaceSpaces".into(),
                util::replace_spaces(v, b'+').into_owned(),
            ),
            (
                "FindEmailIndex".into(),
                util::find_email_index(v).to_string().into_bytes(),
            ),
            (
                "FindURLIndex".into(),
                util::find_url_index(v).to_string().into_bytes(),
            ),
            (
                "TrimLeftSpaceLength".into(),
                util::trim_left_space_length(v).to_string().into_bytes(),
            ),
            (
                "TrimRightSpaceLength".into(),
                util::trim_right_space_length(v).to_string().into_bytes(),
            ),
            ("TrimLeftSpace".into(), util::trim_left_space(v).to_vec()),
            ("TrimRightSpace".into(), util::trim_right_space(v).to_vec()),
            ("IsBlank".into(), util::is_blank(v).to_string().into_bytes()),
            ("VisualizeSpaces".into(), util::visualize_spaces(v)),
            (
                "IsDangerousURL".into(),
                html::is_dangerous_url(v).to_string().into_bytes(),
            ),
            (
                "FirstNonSpacePosition".into(),
                util::first_non_space_position(v).to_string().into_bytes(),
            ),
        ];
        for cs in [false, true] {
            for nest in [false, true] {
                got.push((
                    format!("FindClosure/{cs}/{nest}"),
                    util::find_closure(v, b'[', b']', cs, nest)
                        .to_string()
                        .into_bytes(),
                ));
            }
        }
        for p in [0, 3] {
            let (a, b) = util::indent_width(v, p);
            got.push((format!("IndentWidth/{p}"), format!("{a} {b}").into_bytes()));
        }
        for w in [0, 1, 2, 4, 5] {
            for pos in [0, 1, 3] {
                let (a, b) = util::indent_position(v, pos, w);
                got.push((
                    format!("IndentPosition/{pos}/{w}"),
                    format!("{a} {b}").into_bytes(),
                ));
                let (a, b) = util::indent_position_padding(v, pos, 2, w);
                got.push((
                    format!("IndentPositionPadding/{pos}/{w}"),
                    format!("{a} {b}").into_bytes(),
                ));
                let (a, b) = util::dedent_position(v, pos, w);
                got.push((
                    format!("DedentPosition/{pos}/{w}"),
                    format!("{a} {b}").into_bytes(),
                ));
                let (a, b) = util::dedent_position_padding(v, pos, 1, w);
                got.push((
                    format!("DedentPositionPadding/{pos}/{w}"),
                    format!("{a} {b}").into_bytes(),
                ));
            }
        }
        if !v.is_empty() {
            let mut rs = String::new();
            for p in 0..v.len() {
                let r = std::panic::catch_unwind(|| util::to_rune(v, p));
                match r {
                    Ok(r) => rs.push_str(&format!("{r},")),
                    Err(_) => rs.push_str("PANIC,"),
                }
            }
            got.push(("ToRune".into(), rs.into_bytes()));
        }
        let dw = html::DEFAULT_WRITER.clone();
        got.push(("Write".into(), writer_out(|w| dw.write(w, v))));
        got.push(("RawWrite".into(), writer_out(|w| dw.raw_write(w, v))));
        got.push(("SecureWrite".into(), writer_out(|w| dw.secure_write(w, v))));
        got.push(("WriteEscapedSpace".into(), writer_out(|w| esc.write(w, v))));
        let mut pr = String::new();
        let mut dec: &[u8] = v;
        while !dec.is_empty() {
            let (r, size) = go_unicode::utf8::decode_rune(dec);
            dec = &dec[size..];
            pr.push_str(&format!(
                "{}{},",
                util::is_punct_rune(r) as u8,
                util::is_space_rune(r) as u8
            ));
        }
        got.push(("IsPunctRune/IsSpaceRune".into(), pr.into_bytes()));
        for (k, g) in got {
            if r.fields.get(&k).map(|w| w.as_slice()) != Some(g.as_slice()) {
                if !r.fields.contains_key(&k) && k == "ToRune" {
                    continue;
                }
                fails.push(format!(
                    "{} {k} in={:?}\n want {:?}\n got  {:?}",
                    r.name,
                    s(v),
                    r.fields.get(&k).map(|w| s(w)),
                    s(&g)
                ));
            }
            n += 1;
        }
    }
    assert!(
        fails.is_empty(),
        "{} failures:\n{}",
        fails.len(),
        fails[..fails.len().min(20)].join("\n")
    );
    assert!(n > 100_000, "{n}");
}

#[test]
fn ids_generate() {
    let recs = fixture("util.gmf.gz");
    let r = recs.iter().find(|r| r.name == "ids").unwrap();
    let inputs: Vec<&[u8]> = recs
        .iter()
        .filter(|r| r.name.starts_with("util/"))
        .take_while(|_| true)
        .map(|r| r.get("in"))
        .collect();
    // The ids record covers the seed inputs (the first records).
    let mut pc = goldmark::parser::new_context(vec![]);
    let mut out: Vec<u8> = Vec::new();
    let want = r.get("out");
    let nseeds = want.iter().filter(|&&c| c == b'\n').count();
    for v in inputs.iter().take(nseeds) {
        out.extend(pc.ids().generate(v, KIND_HEADING));
        out.push(b'|');
        out.extend(pc.ids().generate(v, KIND_PARAGRAPH));
        out.push(b'\n');
    }
    assert_eq!(s(&out), s(want));
}

#[test]
fn attribute_filters() {
    let recs = fixture("util.gmf.gz");
    let filters: Vec<(&str, &util::BytesFilter)> = vec![
        ("Global", &html::GLOBAL_ATTRIBUTE_FILTER),
        ("Heading", &html::HEADING_ATTRIBUTE_FILTER),
        ("Blockquote", &html::BLOCKQUOTE_ATTRIBUTE_FILTER),
        ("List", &html::LIST_ATTRIBUTE_FILTER),
        ("ListItem", &html::LIST_ITEM_ATTRIBUTE_FILTER),
        ("Paragraph", &html::PARAGRAPH_ATTRIBUTE_FILTER),
        ("Thematic", &html::THEMATIC_ATTRIBUTE_FILTER),
        ("Link", &html::LINK_ATTRIBUTE_FILTER),
        ("Code", &html::CODE_ATTRIBUTE_FILTER),
        ("Emphasis", &html::EMPHASIS_ATTRIBUTE_FILTER),
        ("Image", &html::IMAGE_ATTRIBUTE_FILTER),
    ];
    for (name, f) in filters {
        let r = recs
            .iter()
            .find(|r| r.name == format!("filter/{name}"))
            .unwrap();
        let names = r.str("names");
        let got: String = names
            .split(',')
            .map(|n| if f.contains(n.as_bytes()) { '1' } else { '0' })
            .collect();
        assert_eq!(got, r.str("contains"), "filter {name}");
    }
}

#[test]
fn cjk_predicates() {
    let recs = fixture("cjk.gmf");
    for r in &recs {
        let name = r.name.strip_prefix("cjk/").unwrap();
        let f = |c: i32| -> String {
            match name {
                "IsEastAsianWideRune" => (util::is_east_asian_wide_rune(c) as u8).to_string(),
                "IsSpaceDiscardingUnicodeRune" => {
                    (util::is_space_discarding_unicode_rune(c) as u8).to_string()
                }
                "EastAsianWidth" => util::east_asian_width(c).to_string(),
                _ => panic!("{name}"),
            }
        };
        for line in r.str("runs").lines() {
            let mut it = line.split(' ');
            let lo = i32::from_str_radix(it.next().unwrap(), 16).unwrap();
            let hi = i32::from_str_radix(it.next().unwrap(), 16).unwrap();
            let v = it.next().unwrap();
            for c in lo..=hi {
                assert_eq!(f(c), v, "{name}({c:#x})");
            }
        }
    }
}

#[test]
fn html5_entity_table() {
    assert_eq!(
        util::lookup_html5_entity_by_name(b"copy")
            .unwrap()
            .characters,
        "©".as_bytes()
    );
    assert_eq!(
        util::lookup_html5_entity_by_name(b"AMP")
            .unwrap()
            .characters,
        b"&"
    );
    assert!(util::lookup_html5_entity_by_name(b"nosuch").is_none());
    assert!(util::lookup_html5_entity_by_name(b"").is_none());
}
