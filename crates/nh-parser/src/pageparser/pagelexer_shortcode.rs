//! Port of `parser/pageparser/pagelexer_shortcode.go`.
//!
//! Owner: Wave B task T03 (parser-langs).

use std::collections::BTreeMap;

use go_unicode::Rune;

use super::item::ItemType;
use super::pagelexer::{
    EOF, PageLexer, StateFn, is_alpha_numeric, is_alpha_numeric_or_hyphen, is_end_of_line,
    is_space, lex_main_section,
};

/// Go: `lexerShortcodeState`.
#[derive(Clone, Debug)]
pub(crate) struct LexerShortcodeState {
    pub(crate) curr_left_delim_item: ItemType,
    pub(crate) curr_right_delim_item: ItemType,
    pub(crate) is_inline: bool,
    /// is only set when a shortcode is in opened state (Go string bytes)
    pub(crate) curr_shortcode_name: Vec<u8>,
    /// > 0 = on its way to be closed
    pub(crate) closing_state: i32,
    /// step number in element
    pub(crate) element_step_num: i32,
    /// number of elements (name + value = 2) found first
    pub(crate) param_elements: i32,
    /// set of shortcodes in open state
    pub(crate) open_shortcodes: BTreeMap<Vec<u8>, bool>,
}

impl LexerShortcodeState {
    /// The initial state set by Go's `newPageLexer`.
    pub(crate) fn new() -> Self {
        LexerShortcodeState {
            curr_left_delim_item: ItemType::LeftDelimScNoMarkup,
            curr_right_delim_item: ItemType::RightDelimScNoMarkup,
            is_inline: false,
            curr_shortcode_name: Vec::new(),
            closing_state: 0,
            element_step_num: 0,
            param_elements: 0,
            open_shortcodes: BTreeMap::new(),
        }
    }
}

// Shortcode syntax
pub(crate) const LEFT_DELIM_SC: &[u8] = b"{{";
pub(crate) const LEFT_DELIM_SC_NO_MARKUP: &[u8] = b"{{<";
pub(crate) const RIGHT_DELIM_SC_NO_MARKUP: &[u8] = b">}}";
pub(crate) const LEFT_DELIM_SC_WITH_MARKUP: &[u8] = b"{{%";
pub(crate) const RIGHT_DELIM_SC_WITH_MARKUP: &[u8] = b"%}}";
/// comments in this context us used to to mark shortcodes as "not really a shortcode"
pub(crate) const LEFT_COMMENT: &[u8] = b"/*";
pub(crate) const RIGHT_COMMENT: &[u8] = b"*/";

/// Inline shortcodes has the form {{< myshortcode.inline >}}
const INLINE_IDENTIFIER: &[u8] = b"inline ";

impl PageLexer {
    // Go: parser/pageparser/pagelexer_shortcode.go:isShortCodeStart
    pub(crate) fn is_short_code_start(&self) -> bool {
        self.has_prefix(LEFT_DELIM_SC_WITH_MARKUP) || self.has_prefix(LEFT_DELIM_SC_NO_MARKUP)
    }

    // Go: parser/pageparser/pagelexer_shortcode.go:currentLeftShortcodeDelimItem
    pub(crate) fn current_left_shortcode_delim_item(&self) -> ItemType {
        self.shortcode.curr_left_delim_item
    }

    // Go: parser/pageparser/pagelexer_shortcode.go:currentRightShortcodeDelimItem
    pub(crate) fn current_right_shortcode_delim_item(&self) -> ItemType {
        self.shortcode.curr_right_delim_item
    }

    // Go: parser/pageparser/pagelexer_shortcode.go:currentLeftShortcodeDelim
    pub(crate) fn current_left_shortcode_delim(&self) -> &'static [u8] {
        if self.shortcode.curr_left_delim_item == ItemType::LeftDelimScWithMarkup {
            return LEFT_DELIM_SC_WITH_MARKUP;
        }
        LEFT_DELIM_SC_NO_MARKUP
    }

    // Go: parser/pageparser/pagelexer_shortcode.go:currentRightShortcodeDelim
    pub(crate) fn current_right_shortcode_delim(&self) -> &'static [u8] {
        if self.shortcode.curr_right_delim_item == ItemType::RightDelimScWithMarkup {
            return RIGHT_DELIM_SC_WITH_MARKUP;
        }
        RIGHT_DELIM_SC_NO_MARKUP
    }
}

/// Go: `fmt.Sprintf("...'%s'...", l.current())` helper.
fn msg_with(prefix: &str, v: &[u8], suffix: &str) -> Vec<u8> {
    let mut m = prefix.as_bytes().to_vec();
    m.extend_from_slice(v);
    m.extend_from_slice(suffix.as_bytes());
    m
}

// Go: parser/pageparser/pagelexer_shortcode.go:lexShortcodeLeftDelim
pub(crate) fn lex_shortcode_left_delim(l: &mut PageLexer) -> Option<StateFn> {
    l.pos += l.current_left_shortcode_delim().len();
    if l.has_prefix(LEFT_COMMENT) {
        return Some(StateFn(lex_shortcode_comment));
    }
    l.emit(l.current_left_shortcode_delim_item());
    l.shortcode.element_step_num = 0;
    l.shortcode.param_elements = 0;
    Some(StateFn(lex_inside_shortcode))
}

// Go: parser/pageparser/pagelexer_shortcode.go:lexShortcodeComment
pub(crate) fn lex_shortcode_comment(l: &mut PageLexer) -> Option<StateFn> {
    let mut sep = RIGHT_COMMENT.to_vec();
    sep.extend_from_slice(l.current_right_shortcode_delim());
    let pos_right_comment = l.index(&sep);
    if pos_right_comment <= 1 {
        return l.errorf(b"comment must be closed".to_vec());
    }
    // we emit all as text, except the comment markers
    l.emit(ItemType::Text);
    l.pos += LEFT_COMMENT.len();
    l.ignore();
    l.pos += pos_right_comment as usize - LEFT_COMMENT.len();
    l.emit(ItemType::Text);
    l.pos += RIGHT_COMMENT.len();
    l.ignore();
    l.pos += l.current_right_shortcode_delim().len();
    l.emit(ItemType::Text);
    Some(StateFn(lex_main_section))
}

// Go: parser/pageparser/pagelexer_shortcode.go:lexShortcodeRightDelim
pub(crate) fn lex_shortcode_right_delim(l: &mut PageLexer) -> Option<StateFn> {
    l.shortcode.closing_state = 0;
    l.pos += l.current_right_shortcode_delim().len();
    l.emit(l.current_right_shortcode_delim_item());
    Some(StateFn(lex_main_section))
}

/// either:
/// 1. param
/// 2. "param" or "param\"
/// 3. param="123" or param="123\"
/// 4. param="Some \"escaped\" text"
/// 5. `param`
/// 6. param=`123`
// Go: parser/pageparser/pagelexer_shortcode.go:lexShortcodeParam
pub(crate) fn lex_shortcode_param(l: &mut PageLexer, escaped_quote_start: bool) -> Option<StateFn> {
    let mut first = true;
    let mut next_eq = false;

    loop {
        let r = l.next();
        if first {
            if r == '"' as Rune || (r == '`' as Rune && !escaped_quote_start) {
                // a positional param with quotes
                if l.shortcode.param_elements == 2 {
                    return l.errorf(
                        b"got quoted positional parameter. Cannot mix named and positional parameters"
                            .to_vec(),
                    );
                }
                l.shortcode.param_elements = 1;
                l.backup();
                if r == '"' as Rune {
                    return lex_shortcode_quoted_param_val(
                        l,
                        !escaped_quote_start,
                        ItemType::ScParam,
                    );
                }
                return lex_short_code_param_raw_string_val(l, ItemType::ScParam);
            } else if r == '`' as Rune && escaped_quote_start {
                return l.errorf(b"unrecognized escape character".to_vec());
            }
            first = false;
        } else if r == '=' as Rune {
            // a named param
            l.backup();
            next_eq = true;
            break;
        }

        if !is_alpha_numeric_or_hyphen(r) && r != '.' as Rune {
            // Floats have period
            l.backup();
            break;
        }
    }

    if l.shortcode.param_elements == 0 {
        l.shortcode.param_elements += 1;

        if next_eq {
            l.shortcode.param_elements += 1;
        }
    } else if next_eq && l.shortcode.param_elements == 1 {
        let m = msg_with(
            "got named parameter '",
            l.current(),
            "'. Cannot mix named and positional parameters",
        );
        return l.errorf(m);
    } else if !next_eq && l.shortcode.param_elements == 2 {
        let m = msg_with(
            "got positional parameter '",
            l.current(),
            "'. Cannot mix named and positional parameters",
        );
        return l.errorf(m);
    }

    l.emit(ItemType::ScParam);
    Some(StateFn(lex_inside_shortcode))
}

// Go: parser/pageparser/pagelexer_shortcode.go:lexShortcodeParamVal
pub(crate) fn lex_shortcode_param_val(l: &mut PageLexer) -> Option<StateFn> {
    l.consume_to_space();
    l.emit(ItemType::ScParamVal);
    Some(StateFn(lex_inside_shortcode))
}

// Go: parser/pageparser/pagelexer_shortcode.go:lexShortCodeParamRawStringVal
pub(crate) fn lex_short_code_param_raw_string_val(
    l: &mut PageLexer,
    typ: ItemType,
) -> Option<StateFn> {
    let mut open_backtick_found = false;

    loop {
        let r = l.next();
        if r == '`' as Rune {
            if open_backtick_found {
                l.backup();
                break;
            } else {
                open_backtick_found = true;
                l.ignore();
            }
        } else if r == EOF {
            let m = msg_with(
                "unterminated raw string in shortcode parameter-argument: '",
                l.current(),
                "'",
            );
            return l.errorf(m);
        }
    }

    l.emit_string(typ);
    l.next();
    l.ignore();

    Some(StateFn(lex_inside_shortcode))
}

// Go: parser/pageparser/pagelexer_shortcode.go:lexShortcodeQuotedParamVal
pub(crate) fn lex_shortcode_quoted_param_val(
    l: &mut PageLexer,
    escaped_quoted_values_allowed: bool,
    typ: ItemType,
) -> Option<StateFn> {
    let mut open_quote_found = false;
    let mut escaped_inner_quote_found = false;
    let mut escaped_quote_state = 0;

    loop {
        let r = l.next();
        if r == '\\' as Rune {
            if l.peek() == '"' as Rune {
                if open_quote_found && !escaped_quoted_values_allowed {
                    l.backup();
                    break;
                } else if open_quote_found {
                    // the coming quote is inside
                    escaped_inner_quote_found = true;
                    escaped_quote_state = 1;
                }
            } else if l.peek() == '`' as Rune {
                return l.errorf(b"unrecognized escape character".to_vec());
            }
        } else if r == EOF || r == '\n' as Rune {
            let m = msg_with(
                "unterminated quoted string in shortcode parameter-argument: '",
                l.current(),
                "'",
            );
            return l.errorf(m);
        } else if r == '"' as Rune {
            if escaped_quote_state == 0 {
                if open_quote_found {
                    l.backup();
                    break;
                } else {
                    open_quote_found = true;
                    l.ignore();
                }
            } else {
                escaped_quote_state = 0;
            }
        }
    }

    if escaped_inner_quote_found {
        l.ignore_escapes_and_emit(typ, true);
    } else {
        l.emit_string(typ);
    }

    let r = l.next();

    if r == '\\' as Rune {
        if l.peek() == '"' as Rune {
            // ignore the escaped closing quote
            l.ignore();
            l.next();
            l.ignore();
        }
    } else if r == '"' as Rune {
        // ignore closing quote
        l.ignore();
    } else {
        // handled by next state
        l.backup();
    }

    Some(StateFn(lex_inside_shortcode))
}

/// scans an alphanumeric inside shortcode
// Go: parser/pageparser/pagelexer_shortcode.go:lexIdentifierInShortcode
pub(crate) fn lex_identifier_in_shortcode(l: &mut PageLexer) -> Option<StateFn> {
    let mut look_for_end = false;
    loop {
        let r = l.next();
        if is_alpha_numeric_or_hyphen(r) {
            // Allow forward slash inside names to make it possible to create namespaces.
        } else if r == '/' as Rune {
        } else if r == '.' as Rune {
            l.shortcode.is_inline = l.has_prefix(INLINE_IDENTIFIER);
            if !l.shortcode.is_inline {
                return l.errorf(
                    b"period in shortcode name only allowed for inline identifiers".to_vec(),
                );
            }
        } else {
            l.backup();
            let word = l.input[l.start..l.pos].to_vec();
            let open = l
                .shortcode
                .open_shortcodes
                .get(&word)
                .copied()
                .unwrap_or(false);
            if l.shortcode.closing_state > 0 && !open {
                let m = msg_with(
                    "closing tag for shortcode '",
                    &word,
                    "' does not match start tag",
                );
                return l.errorf(m);
            } else if l.shortcode.closing_state > 0 {
                l.shortcode.open_shortcodes.insert(word.clone(), false);
                look_for_end = true;
            }

            l.shortcode.closing_state = 0;
            l.shortcode.curr_shortcode_name = word.clone();
            l.shortcode.open_shortcodes.insert(word, true);
            l.shortcode.element_step_num += 1;
            if l.shortcode.is_inline {
                l.emit(ItemType::ScNameInline);
            } else {
                l.emit(ItemType::ScName);
            }
            break;
        }
    }

    if look_for_end {
        return Some(StateFn(lex_end_of_shortcode));
    }
    Some(StateFn(lex_inside_shortcode))
}

// Go: parser/pageparser/pagelexer_shortcode.go:lexEndOfShortcode
pub(crate) fn lex_end_of_shortcode(l: &mut PageLexer) -> Option<StateFn> {
    l.shortcode.is_inline = false;
    if l.has_prefix(l.current_right_shortcode_delim()) {
        return Some(StateFn(lex_shortcode_right_delim));
    }
    let r = l.next();
    if is_space(r) {
        l.ignore();
    } else {
        return l.errorf(b"unclosed shortcode".to_vec());
    }
    Some(StateFn(lex_end_of_shortcode))
}

/// scans the elements inside shortcode tags
// Go: parser/pageparser/pagelexer_shortcode.go:lexInsideShortcode
pub(crate) fn lex_inside_shortcode(l: &mut PageLexer) -> Option<StateFn> {
    if l.has_prefix(l.current_right_shortcode_delim()) {
        return Some(StateFn(lex_shortcode_right_delim));
    }
    let r = l.next();
    if r == EOF {
        // eol is allowed inside shortcodes; this may go to end of document before it fails
        return l.errorf(b"unclosed shortcode action".to_vec());
    } else if is_space(r) || is_end_of_line(r) {
        l.ignore();
    } else if r == '=' as Rune {
        l.consume_space();
        l.ignore();
        let peek = l.peek();
        if peek == '"' as Rune || peek == '\\' as Rune {
            return lex_shortcode_quoted_param_val(l, peek != '\\' as Rune, ItemType::ScParamVal);
        } else if peek == '`' as Rune {
            return lex_short_code_param_raw_string_val(l, ItemType::ScParamVal);
        }
        return Some(StateFn(lex_shortcode_param_val));
    } else if r == '/' as Rune {
        if l.shortcode.curr_shortcode_name.is_empty() {
            return l.errorf(b"got closing shortcode, but none is open".to_vec());
        }
        l.shortcode.closing_state += 1;
        l.shortcode.is_inline = false;
        l.shortcode.element_step_num = 0;
        l.emit(ItemType::ScClose);
    } else if r == '\\' as Rune {
        l.ignore();
        if l.peek() == '"' as Rune || l.peek() == '`' as Rune {
            return lex_shortcode_param(l, true);
        }
    } else if l.shortcode.element_step_num > 0
        && (is_alpha_numeric_or_hyphen(r) || r == '"' as Rune || r == '`' as Rune)
    {
        // positional params can have quotes
        l.backup();
        return lex_shortcode_param(l, false);
    } else if is_alpha_numeric(r) {
        l.backup();
        return Some(StateFn(lex_identifier_in_shortcode));
    } else {
        let mut m = b"unrecognized character in shortcode action: ".to_vec();
        m.extend_from_slice(&format_sharp_u(r));
        m.extend_from_slice(b". Note: Parameters with non-alphanumeric args must be quoted");
        return l.errorf(m);
    }
    Some(StateFn(lex_inside_shortcode))
}

/// Go: `fmt.Sprintf("%#U", r)`: `U+0041 'A'` (the quoted rune only when printable).
fn format_sharp_u(r: Rune) -> Vec<u8> {
    let mut out = format!("U+{:04X}", r as u32 as u64 & 0xFFFF_FFFF).into_bytes();
    if (0..=go_unicode::utf8::MAX_RUNE).contains(&r) && go_strconv::is_print(r) {
        out.extend_from_slice(b" '");
        go_unicode::utf8::append_rune(&mut out, r);
        out.push(b'\'');
    }
    out
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/pagelexer_shortcode.go (366 lines; 11/15 funcs executed)
//   types: lexerShortcodeState
// OK L38-40: (l *pageLexer) isShortCodeStart() bool
// OK L42-51: lexShortcodeLeftDelim(l *pageLexer) stateFunc
// OK L53-69: lexShortcodeComment(l *pageLexer) stateFunc
// OK L71-76: lexShortcodeRightDelim(l *pageLexer) stateFunc
// OK L85-139: lexShortcodeParam(l *pageLexer, escapedQuoteStart bool) stateFunc
// OK L141-145: lexShortcodeParamVal(l *pageLexer) stateFunc
// OK L147-171: lexShortCodeParamRawStringVal(l *pageLexer, typ ItemType) stateFunc
// OK L173-237: lexShortcodeQuotedParamVal(l *pageLexer, escapedQuotedValuesAllowed bool, typ ItemType) stateFunc
// OK L243-283: lexIdentifierInShortcode(l *pageLexer) stateFunc
// OK L285-297: lexEndOfShortcode(l *pageLexer) stateFunc
// OK L300-344: lexInsideShortcode(l *pageLexer) stateFunc
// OK L346-348: (l *pageLexer) currentLeftShortcodeDelimItem() ItemType
// OK L350-352: (l *pageLexer) currentRightShortcodeDelimItem() ItemType
// OK L354-359: (l *pageLexer) currentLeftShortcodeDelim() []byte
// OK L361-366: (l *pageLexer) currentRightShortcodeDelim() []byte
// ---------------------------------------------------------------------------
