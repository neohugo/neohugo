//! Unit tests of the converter API around the oracle tests: Parse + Render (Hugo parses once
//! for the TOC and renders later), the explicit unsupported errors, and the context objects.

mod common;

use std::sync::Arc;

use common::*;
use go_value::Value;
use nh_markup::converter::hooks::{TableCell, table_rows_to_value};
use nh_markup::goldmark::convert::GoldmarkProvider;
use nh_markup::markup_config;

const DOC: &[u8] = b"# Title\n\n## Sub *em*\n\n| a | b |\n|:-|-:|\n| 1 | 2 |\n\n> [!NOTE]\n> x\n\n```go\ncode\n```\n";

#[test]
fn parse_then_render_twice_equals_convert() {
    let (p, _) = provider("[markup.goldmark.renderer]\nunsafe = true");
    let conv = converter(&p, document_context("doc"));
    let host = ();
    let rctx = nh_markup::converter::converter::RenderContext {
        ctx: &host,
        src: DOC,
        render_toc: true,
        get_renderer: Some(replica_renderers()),
    };
    let whole = conv.convert(&rctx).unwrap();
    let pr = conv
        .as_parse_renderer()
        .expect("goldmark is a ParseRenderer");
    let parsed = pr.parse(&rctx).unwrap();
    let toc = parsed.table_of_contents.clone().expect("TOC");
    assert_eq!(toc.identifiers.len(), 2);
    for _ in 0..2 {
        let r = pr.render(&rctx, &parsed.doc).unwrap();
        assert_eq!(r.bytes, whole.bytes);
    }
    assert_eq!(
        whole.table_of_contents.unwrap().to_html(1, -1, false),
        toc.to_html(1, -1, false)
    );
    assert_eq!(
        conv.sanitize_anchor_name("Ünï Code!").as_deref(),
        Some("ünï-code")
    );

    // Without RenderTOC there is no TOC (Go: a nil *Fragments).
    let rctx = nh_markup::converter::converter::RenderContext {
        render_toc: false,
        ..rctx
    };
    assert!(pr.parse(&rctx).unwrap().table_of_contents.is_none());
}

#[test]
fn unsupported_features_fail_explicitly() {
    for (toml, want) in [
        (
            "[markup.goldmark.extensions.passthrough]\nenable = true",
            "neohugo-rs: the goldmark passthrough extension is not supported",
        ),
        (
            "[markup.goldmark.extensions.extras.subscript]\nenable = true",
            "neohugo-rs: goldmark extras (delete, insert, mark, subscript, superscript) is not supported",
        ),
    ] {
        let m = Arc::new(decode_markup(toml).unwrap());
        let err = GoldmarkProvider::new_from_config(m, false, None)
            .err()
            .expect("error");
        assert_eq!(err.to_string(), want);
    }
    let m = Arc::new(markup_config::default_config());
    let err = GoldmarkProvider::new_from_config(m, true, None)
        .err()
        .expect("error");
    assert_eq!(
        err.to_string(),
        "neohugo-rs: enableEmoji (goldmark-emoji) is not supported"
    );

    // The default code block renderer is the (stub) highlighter.
    let h = nh_markup::highlight::highlight::new(Default::default());
    assert!(h.is_default_code_block_renderer());
    let cb = h.clone().as_code_block_renderer();
    let get: nh_markup::converter::hooks::GetRendererFunc = Arc::new(move |t, _| match t {
        nh_markup::converter::hooks::RendererType::CodeBlock => {
            Some(nh_markup::converter::hooks::Renderer::CodeBlock(cb.clone()))
        }
        nh_markup::converter::hooks::RendererType::Table => Some(
            nh_markup::converter::hooks::Renderer::Table(Arc::new(TableReplica)),
        ),
        _ => None,
    });
    let (p, _) = provider("");
    let conv = converter(&p, document_context("doc.md"));
    match convert(&*conv, DOC, true, get) {
        Outcome::Err(e) => assert_eq!(
            e,
            "\"doc.md:1:1\": neohugo-rs: highlighting (chroma) is not supported"
        ),
        _ => panic!("want the highlighting error"),
    }
}

#[test]
fn table_cells_as_values() {
    assert!(matches!(table_rows_to_value(&[]), Value::TypedNil(_)));
    let v = table_rows_to_value(&[vec![
        TableCell {
            text: nh_common::types::hstring::Html(go_value::GoString::new(b"<b>x</b>".to_vec())),
            alignment: "center".into(),
        },
        TableCell::default(),
    ]]);
    let rows = v.as_list().unwrap();
    assert_eq!(rows.ty.go_name(), "[]hooks.TableRow");
    let row = rows.items[0].as_list().unwrap();
    assert_eq!(row.ty.go_name(), "hooks.TableRow");
    let cell = row.items[0].as_object().unwrap();
    assert_eq!(cell.type_name(), "hooks.TableCell");
    assert_eq!(dump_val(&cell.field("Text").unwrap()), b"h:<b>x</b>");
    assert_eq!(dump_val(&cell.field("Alignment").unwrap()), b"s:center");
    assert!(cell.field("Nope").is_none());
}
