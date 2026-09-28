//! Port of `parser/pageparser/pagelexer_intro.go`.
//!
//! Owner: Wave B task T03 (parser-langs).

use go_unicode::Rune;

use super::item::ItemType;
use super::pagelexer::{
    BYTE_ORDER_MARK, DELIM_ORG, DELIM_TOML, DELIM_YAML, EOF, PageLexer, SUMMARY_DIVIDER_ORG,
    StateFn, is_end_of_line, is_space, lex_main_section,
};

// Go: parser/pageparser/pagelexer_intro.go:lexIntroSection
pub(crate) fn lex_intro_section(l: &mut PageLexer) -> Option<StateFn> {
    loop {
        let r = l.next();
        if r == EOF {
            break;
        }

        if r == '+' as Rune {
            return lex_front_matter_section(l, ItemType::FrontMatterToml, r, "TOML", DELIM_TOML);
        } else if r == '-' as Rune {
            return lex_front_matter_section(l, ItemType::FrontMatterYaml, r, "YAML", DELIM_YAML);
        } else if r == '{' as Rune {
            return Some(StateFn(lex_front_matter_json));
        } else if r == '#' as Rune {
            return Some(StateFn(lex_front_matter_org_mode));
        } else if r == BYTE_ORDER_MARK {
            l.emit(ItemType::Ignore);
        } else if !is_space(r) && !is_end_of_line(r) {
            break;
        }
    }

    // Now move on to the shortcodes.
    Some(StateFn(lex_main_section))
}

// Go: parser/pageparser/pagelexer_intro.go:lexFrontMatterJSON
pub(crate) fn lex_front_matter_json(l: &mut PageLexer) -> Option<StateFn> {
    // Include the left delimiter
    l.backup();

    let mut in_quote = false;
    let mut level = 0;

    loop {
        let r = l.next();

        if r == EOF {
            return l.errorf(b"unexpected EOF parsing JSON front matter".to_vec());
        } else if r == '{' as Rune {
            if !in_quote {
                level += 1;
            }
        } else if r == '}' as Rune {
            if !in_quote {
                level -= 1;
            }
        } else if r == '"' as Rune {
            in_quote = !in_quote;
        } else if r == '\\' as Rune {
            // This may be an escaped quote. Make sure it's not marked as a
            // real one.
            l.next();
        }

        if level == 0 {
            break;
        }
    }

    l.consume_crlf();
    l.emit(ItemType::FrontMatterJson);

    Some(StateFn(lex_main_section))
}

// Go: parser/pageparser/pagelexer_intro.go:lexFrontMatterOrgMode
pub(crate) fn lex_front_matter_org_mode(l: &mut PageLexer) -> Option<StateFn> {
    /*
        #+TITLE: Test File For chaseadamsio/goorgeous
        #+AUTHOR: Chase Adams
        #+DESCRIPTION: Just another golang parser for org content!
    */

    l.backup();

    if !l.has_prefix(DELIM_ORG) {
        return Some(StateFn(lex_main_section));
    }

    l.summary_divider = SUMMARY_DIVIDER_ORG;

    // Read lines until we no longer see a #+ prefix
    loop {
        let r = l.next();

        if r == '\n' as Rune {
            if !l.has_prefix(DELIM_ORG) {
                break;
            }
        } else if r == EOF {
            break;
        }
    }

    l.emit(ItemType::FrontMatterOrg);

    Some(StateFn(lex_main_section))
}

/// Handle YAML or TOML front matter.
// Go: parser/pageparser/pagelexer_intro.go:lexFrontMatterSection
pub(crate) fn lex_front_matter_section(
    l: &mut PageLexer,
    tp: ItemType,
    delimr: Rune,
    name: &str,
    delim: &[u8],
) -> Option<StateFn> {
    for _ in 0..2 {
        let r = l.next();
        if r != delimr {
            return l.errorf(format!("invalid {name} delimiter").into_bytes());
        }
    }

    // Let front matter start at line 1
    let mut was_end_of_line = l.consume_crlf();
    // We don't care about the delimiters.
    l.ignore();

    let mut r: Rune = 0;

    loop {
        if !was_end_of_line {
            r = l.next();
            if r == EOF {
                return l.errorf(
                    format!("EOF looking for end {name} front matter delimiter").into_bytes(),
                );
            }
        }

        if (was_end_of_line || is_end_of_line(r)) && l.has_prefix(delim) {
            l.emit(tp);
            l.pos += 3;
            l.consume_crlf();
            l.ignore();
            break;
        }

        was_end_of_line = false;
    }

    Some(StateFn(lex_main_section))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/pagelexer_intro.go (161 lines; 2/4 funcs executed)
// OK L16-42: lexIntroSection(l *pageLexer) stateFunc
// OK L44-85: lexFrontMatterJSON(l *pageLexer) stateFunc
// OK L87-122: lexFrontMatterOrgMode(l *pageLexer) stateFunc
// OK L125-161: (l *pageLexer) lexFrontMatterSection(tp ItemType, delimr rune, name string, delim []byte) stateFunc
// ---------------------------------------------------------------------------
