//! Go: github.com/yuin/goldmark@v1.7.12/extension — goldmark's extensions
//! (GFM tables, strikethrough, linkify, task lists; PHP Markdown Extra
//! definition lists and footnotes; typographer; CJK).
//!
//! Go's package-level extension values (`extension.Table`, `extension.GFM`,
//! ...) are constructor functions here (`extension::table()`, `gfm()`, ...)
//! returning a `Box<dyn Extender>` for [`crate::with_extensions`]; the
//! `NewX(opts...)` constructors take the options as a `Vec`.

pub mod ast;
mod cjk;
mod definition_list;
mod footnote;
mod gfm;
mod linkify;
mod strikethrough;
mod table;
mod tasklist;
mod typographer;

pub use cjk::{
    CJKOption, CjkExt, EastAsianLineBreaks, cjk, new_cjk, with_east_asian_line_breaks,
    with_escaped_space,
};
pub use definition_list::{
    DEFINITION_DESCRIPTION_ATTRIBUTE_FILTER, DEFINITION_LIST_ATTRIBUTE_FILTER,
    DEFINITION_TERM_ATTRIBUTE_FILTER, DefinitionListExt, DefinitionListHTMLRenderer,
    definition_list, new_definition_description_parser, new_definition_list_html_renderer,
    new_definition_list_parser,
};
pub use footnote::{
    FootnoteConfig, FootnoteExt, FootnoteHTMLRenderer, FootnoteOption, IDPrefixFunction, footnote,
    new_footnote, new_footnote_ast_transformer, new_footnote_block_parser, new_footnote_config,
    new_footnote_html_renderer, new_footnote_parser, with_footnote_backlink_class,
    with_footnote_backlink_html, with_footnote_backlink_title, with_footnote_html_options,
    with_footnote_id_prefix, with_footnote_id_prefix_function, with_footnote_link_class,
    with_footnote_link_title,
};
pub use gfm::{GfmExt, gfm};
pub use linkify::{
    LinkifyConfig, LinkifyExt, LinkifyOption, LinkifyRegexp, URL_REGEXP, UrlRegexp, WWW_URL_REGEXP,
    WwwUrlRegexp, linkify, new_linkify, new_linkify_parser, with_linkify_allowed_protocols,
    with_linkify_email_regexp, with_linkify_url_regexp, with_linkify_www_regexp,
};
pub use strikethrough::{
    STRIKETHROUGH_ATTRIBUTE_FILTER, StrikethroughExt, StrikethroughHTMLRenderer,
    new_strikethrough_html_renderer, new_strikethrough_parser, strikethrough,
};
pub use table::{
    TABLE_ATTRIBUTE_FILTER, TABLE_HEADER_ATTRIBUTE_FILTER, TABLE_ROW_ATTRIBUTE_FILTER,
    TABLE_TD_CELL_ATTRIBUTE_FILTER, TABLE_TH_CELL_ATTRIBUTE_FILTER, TableCellAlignMethod,
    TableConfig, TableExt, TableHTMLRenderer, TableOption, new_table, new_table_ast_transformer,
    new_table_config, new_table_html_renderer, new_table_paragraph_transformer, table,
    with_table_cell_align_method, with_table_html_options,
};
pub use tasklist::{
    TaskCheckBoxHTMLRenderer, TaskListExt, new_task_check_box_html_renderer,
    new_task_check_box_parser, task_list,
};
pub use typographer::{
    Substitutions, TypographerConfig, TypographerExt, TypographerOption, TypographicPunctuation,
    new_typographer, new_typographer_parser, typographer, with_typographic_substitutions,
};

#[cfg(test)]
mod regex_vectors {
    //! The hand-written matchers that replace the extension package's
    //! regular expressions vs Go's regexp (fixture from
    //! `goldmark extregex`, tools/go-oracle/goldmark/ext.go).
    use std::io::Read;

    use super::{LinkifyRegexp, UrlRegexp, WwwUrlRegexp};

    fn records() -> Vec<Vec<(String, Vec<u8>)>> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ext-regex.gmf.gz"
        );
        let raw = std::fs::read(path).unwrap();
        let mut data = Vec::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_end(&mut data)
            .unwrap();
        let mut recs: Vec<Vec<(String, Vec<u8>)>> = Vec::new();
        let mut i = 0;
        while i < data.len() {
            let nl = i + data[i..].iter().position(|&c| c == b'\n').unwrap();
            let line = std::str::from_utf8(&data[i..nl]).unwrap().to_string();
            i = nl + 1;
            if line.starts_with("=== ") {
                recs.push(Vec::new());
                continue;
            }
            let sp = line.rfind(' ').unwrap();
            let n: usize = line[sp + 1..].parse().unwrap();
            recs.last_mut()
                .unwrap()
                .push((line[..sp].to_string(), data[i..i + n].to_vec()));
            i += n + 1;
        }
        recs
    }

    fn pair(m: Option<(usize, usize)>) -> String {
        match m {
            None => "-".to_string(),
            Some((a, b)) => format!("{a} {b}"),
        }
    }

    fn flag(b: bool) -> String {
        if b { "1" } else { "0" }.to_string()
    }

    #[test]
    fn extension_regexps_match_go() {
        let recs = records();
        assert!(recs.len() > 10000, "{}", recs.len());
        let mut checked = 0;
        for r in &recs {
            let get = |k: &str| {
                String::from_utf8(r.iter().find(|(n, _)| n == k).unwrap().1.clone()).unwrap()
            };
            let input = &r.iter().find(|(n, _)| n == "in").unwrap().1;
            let ctx = String::from_utf8_lossy(input).into_owned();
            assert_eq!(
                pair(UrlRegexp.find_submatch_index(input)),
                get("url"),
                "url {ctx:?}"
            );
            assert_eq!(
                pair(WwwUrlRegexp.find_submatch_index(input)),
                get("www"),
                "www {ctx:?}"
            );
            assert_eq!(
                pair(super::tasklist::task_list_regexp_match(input)),
                get("task"),
                "task {ctx:?}"
            );
            assert_eq!(
                flag(super::table::table_delim_left(input)),
                get("left"),
                "left {ctx:?}"
            );
            assert_eq!(
                flag(super::table::table_delim_right(input)),
                get("right"),
                "right {ctx:?}"
            );
            assert_eq!(
                flag(super::table::table_delim_center(input)),
                get("center"),
                "center {ctx:?}"
            );
            assert_eq!(
                flag(super::table::table_delim_none(input)),
                get("none"),
                "none {ctx:?}"
            );
            checked += 1;
        }
        assert_eq!(checked, recs.len());
    }
}
