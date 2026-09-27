//! Port of `parser/pageparser/pagelexer_shortcode.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


use std::collections::BTreeMap;

use super::item::ItemType;
use super::pagelexer::{PageLexer, StateFn};

/// Go: `lexerShortcodeState`.
#[derive(Clone, Debug)]
pub(crate) struct LexerShortcodeState {
    pub(crate) curr_left_delim_item: ItemType,
    pub(crate) curr_right_delim_item: ItemType,
    pub(crate) is_inline: bool,
    pub(crate) curr_shortcode_name: String,
    pub(crate) closing_state: i32,
    pub(crate) element_step_num: i32,
    pub(crate) param_elements: i32,
    pub(crate) open_shortcodes: BTreeMap<String, bool>,
}

// Go: parser/pageparser/pagelexer_shortcode.go:lexShortcodeLeftDelim
pub(crate) fn lex_shortcode_left_delim(l: &mut PageLexer) -> Option<StateFn> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/pagelexer_shortcode.go (366 lines; 11/15 funcs executed)
//   types: lexerShortcodeState
// EX L38-40: (l *pageLexer) isShortCodeStart() bool
// EX L42-51: lexShortcodeLeftDelim(l *pageLexer) stateFunc
//    L53-69: lexShortcodeComment(l *pageLexer) stateFunc
// EX L71-76: lexShortcodeRightDelim(l *pageLexer) stateFunc
// EX L85-139: lexShortcodeParam(l *pageLexer, escapedQuoteStart bool) stateFunc
//    L141-145: lexShortcodeParamVal(l *pageLexer) stateFunc
//    L147-171: lexShortCodeParamRawStringVal(l *pageLexer, typ ItemType) stateFunc
// EX L173-237: lexShortcodeQuotedParamVal(l *pageLexer, escapedQuotedValuesAllowed bool, typ ItemType) stateFunc
// EX L243-283: lexIdentifierInShortcode(l *pageLexer) stateFunc
//    L285-297: lexEndOfShortcode(l *pageLexer) stateFunc
// EX L300-344: lexInsideShortcode(l *pageLexer) stateFunc
// EX L346-348: (l *pageLexer) currentLeftShortcodeDelimItem() ItemType
// EX L350-352: (l *pageLexer) currentRightShortcodeDelimItem() ItemType
// EX L354-359: (l *pageLexer) currentLeftShortcodeDelim() []byte
// EX L361-366: (l *pageLexer) currentRightShortcodeDelim() []byte
// ---------------------------------------------------------------------------
