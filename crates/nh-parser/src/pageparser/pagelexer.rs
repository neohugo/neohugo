//! Port of `parser/pageparser/pagelexer.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


use super::item::{Item, ItemType, Items};

/// Go: `pageparser.Config`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    pub no_front_matter: bool,
    pub no_summary_divider: bool,
}

/// Go: `stateFunc func(*pageLexer) stateFunc`. The few Go closures that capture state
/// (`sectionHandlers.lex(origin)`) keep that state in the lexer instead.
#[derive(Clone, Copy)]
pub(crate) struct StateFn(pub(crate) fn(&mut PageLexer) -> Option<StateFn>);

/// Go: `pageLexer` — the front matter / summary divider / shortcode lexer (a Rob Pike style
/// state machine, ported state by state).
pub struct PageLexer {
    pub(crate) input: Vec<u8>,
    pub(crate) pos: usize,
    pub(crate) start: usize,
    pub(crate) width: usize,
    pub(crate) cfg: Config,
    pub(crate) summary_divider: Vec<u8>,
    pub(crate) summary_divider_checked: bool,
    pub(crate) shortcode: super::pagelexer_shortcode::LexerShortcodeState,
    pub(crate) items: Items,
    pub(crate) err: Option<String>,
}

impl PageLexer {
    // Go: parser/pageparser/pagelexer.go:newPageLexer
    pub(crate) fn new(input: Vec<u8>, cfg: Config) -> Self {
        todo!()
    }

    // Go: parser/pageparser/pagelexer.go:run
    pub(crate) fn run(&mut self) {
        todo!()
    }
}

// Go: parser/pageparser/pagelexer.go:lexMainSection
pub(crate) fn lex_main_section(l: &mut PageLexer) -> Option<StateFn> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/pagelexer.go (530 lines; 23/33 funcs executed)
//   types: stateFunc, pageLexer, Config, sectionHandlers, sectionHandler
//    L57-59: (l *pageLexer) Iterator() *Iterator
//    L61-63: (l *pageLexer) Input() []byte
// EX L72-89: newPageLexer(input []byte, stateStart stateFunc, cfg Config) *pageLexer
// EX L92-97: (l *pageLexer) run() *pageLexer
// EX L109-120: (l *pageLexer) next() rune
//    L123-127: (l *pageLexer) peek() rune
// EX L130-132: (l *pageLexer) backup()
// EX L134-139: (l *pageLexer) append(item Item)
// EX L142-170: (l *pageLexer) emit(t ItemType)
// EX L173-176: (l *pageLexer) emitString(t ItemType)
// EX L178-180: (l *pageLexer) isEOF() bool
//    L183-215: (l *pageLexer) ignoreEscapesAndEmit(t ItemType, isString bool)
//    L218-220: (l *pageLexer) current() []byte
// EX L223-225: (l *pageLexer) ignore()
//    L230-233: (l *pageLexer) errorf(format string, args ...any) stateFunc
// EX L235-245: (l *pageLexer) consumeCRLF() bool
//    L247-255: (l *pageLexer) consumeToSpace()
//    L257-265: (l *pageLexer) consumeSpace()
// EX L278-299: (s *sectionHandlers) skip() int
// EX L301-374: createSectionHandlers(l *pageLexer) *sectionHandlers
// EX L376-400: (s *sectionHandlers) lex(origin stateFunc) stateFunc
// EX L418-428: (s *sectionHandler) skip() int
// EX L430-452: lexMainSection(l *pageLexer) stateFunc
// EX L454-461: lexDone(l *pageLexer) stateFunc
//    L464-466: (l *pageLexer) printCurrentInput()
// EX L470-472: (l *pageLexer) index(sep []byte) int
// EX L474-476: (l *pageLexer) hasPrefix(prefix []byte) bool
// EX L481-495: minIndex(indices ...int) int
//    L497-511: indexNonWhiteSpace(s []byte, in rune) int
// EX L513-515: isSpace(r rune) bool
// EX L517-520: isAlphaNumericOrHyphen(r rune) bool
// EX L524-526: isEndOfLine(r rune) bool
// EX L528-530: isAlphaNumeric(r rune) bool
// ---------------------------------------------------------------------------
