//! `tpl.StripHTML` against the Go oracle (`tools/go-oracle/nh-tpl/striphtml`): edge cases,
//! every 1- and 2-byte string over HTML-significant bytes, seeded random HTML-ish strings and
//! slices of this repository's docs/ content, byte for byte (inputs and outputs hex-encoded).

use std::io::Read;
use std::path::PathBuf;

use nh_tpl::template::strip_html;

fn unhex(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let v = |c: u8| -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("bad hex {c}"),
        }
    };
    b.chunks(2).map(|p| v(p[0]) << 4 | v(p[1])).collect()
}

#[test]
fn strip_html_oracle() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/striphtml/striphtml.json.gz");
    let mut s = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(&path).unwrap())
        .read_to_string(&mut s)
        .unwrap();
    let fx: serde_json::Value = serde_json::from_str(&s).unwrap();
    let cases = fx["cases"].as_array().unwrap();
    assert!(cases.len() > 4000);
    let mut failures = Vec::new();
    for c in cases {
        let input = unhex(c[0].as_str().unwrap());
        let want = unhex(c[1].as_str().unwrap());
        let got = strip_html(&input);
        if got != want {
            failures.push(format!(
                "{:?}\n  go   {:?}\n  rust {:?}",
                String::from_utf8_lossy(&input),
                String::from_utf8_lossy(&want),
                String::from_utf8_lossy(&got)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(10)].join("\n")
    );
}

#[test]
fn markup_scope_getter() {
    nh_tpl::template::register_markup_scope_getter();
    let ctx = nh_tpl::template::TplContext::default().with_markup_scope("myscope");
    assert_eq!(
        nh_config::neohugo::neohugo::get_markup_scope(ctx.as_host()),
        "myscope"
    );
    assert_eq!(nh_config::neohugo::neohugo::get_markup_scope(&()), "");
}

#[test]
fn current_template_levels() {
    let ctx = nh_tpl::template::TplContext::default();
    let c1 = ctx.with_current_template("a", "fa");
    let c2 = c1.with_current_template_info(
        "b",
        "",
        Some(nh_tpl::template::CurrentTemplateBase {
            name: "base".into(),
            filename: "fb".into(),
        }),
    );
    let ti = c2.current_template.as_ref().unwrap();
    assert_eq!(ti.level, 1);
    assert_eq!(ti.parent.as_ref().unwrap().level, 0);
    let anc = ti.ancestors();
    assert_eq!(anc.len(), 1);
    assert_eq!(anc[0].name, "a");
    let rev = nh_tpl::template::reverse_current_template_infos(&[anc[0].clone(), ti.clone()]);
    assert_eq!(rev[0].name, "b");
}
