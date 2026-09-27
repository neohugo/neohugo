//! Port of `publisher/htmlElementsCollector.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).


//! Go `publisher/htmlElementsCollector.go` — port the state machine VERBATIM (it decides the CSS
//! purged into every page). Stops at the first `utf8.RuneError` of a Write; `pre|textarea|script|
//! style` prefix match; per element string `x/net/html.Parse` (see `xnethtml`) with the in-body
//! quirks; classes from `(?i)^class$|transition` attributes; `sort.Strings` + dedupe on merge.

use std::collections::BTreeSet;

use nh_config::common_config::BuildStats;

/// Go: `publisher.HTMLElements` (JSON keys `tags`, `classes`, `ids`; nil slices -> `null`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HtmlElements {
    pub tags: Option<Vec<String>>,
    pub classes: Option<Vec<String>>,
    pub ids: Option<Vec<String>>,
}

impl HtmlElements {
    /// Go: `Merge(other)` (append + UniqueStringsReuse).
    // Go: publisher/htmlElementsCollector.go:Merge
    pub fn merge(&mut self, other: &HtmlElements) {
        todo!()
    }

    /// Go: `Sort()` (`sort.Strings` each).
    // Go: publisher/htmlElementsCollector.go:Sort
    pub fn sort(&mut self) {
        todo!()
    }
}

/// Go: `htmlElement`.
#[derive(Clone, Debug, Default)]
pub struct HtmlElement {
    pub tag: String,
    pub classes: Vec<String>,
    pub ids: Vec<String>,
}

/// Go: `htmlElementsCollector`.
pub struct HtmlElementsCollector {
    pub conf: BuildStats,
    pub(crate) element_set: BTreeSet<String>,
    pub(crate) elements: Vec<HtmlElement>,
}

impl HtmlElementsCollector {
    // Go: publisher/htmlElementsCollector.go:newHTMLElementsCollector
    pub fn new(conf: BuildStats) -> Self {
        HtmlElementsCollector { conf, element_set: BTreeSet::new(), elements: Vec::new() }
    }

    /// Feeds one published document (Go: a single `Write` of the whole buffer).
    // Go: publisher/htmlElementsCollector.go:Write
    pub fn write(&mut self, p: &[u8]) {
        todo!()
    }

    // Go: publisher/htmlElementsCollector.go:getHTMLElements
    pub fn get_html_elements(&self) -> HtmlElements {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: publisher/htmlElementsCollector.go (556 lines; 17/19 funcs executed)
//   types: HTMLElements, htmlElement, htmlElementsCollector, htmlElementsCollectorWriter, htmlCollectorStateFunc
// EX L50-55: newHTMLElementsCollector(conf config.BuildStats) *htmlElementsCollector
// EX L57-66: newHTMLElementsCollectorWriter(collector *htmlElementsCollector) *htmlElementsCollectorWriter
// EX L75-83: (h *HTMLElements) Merge(other HTMLElements)
// EX L85-89: (h *HTMLElements) Sort()
// EX L110-136: (c *htmlElementsCollector) getHTMLElements() HTMLElements
// EX L160-179: (w *htmlElementsCollectorWriter) Write(p []byte) (int, error)
// EX L181-184: (l *htmlElementsCollectorWriter) backup()
// EX L186-197: (w *htmlElementsCollectorWriter) consumeBuffUntil(condition func() bool, resolve htmlCollectorStateFunc) htmlCollectorStateFunc
// EX L199-208: (w *htmlElementsCollectorWriter) consumeRuneUntil(condition func(r rune) bool, resolve htmlCollectorStateFunc) htmlCollectorStateFunc
// EX L211-276: (w *htmlElementsCollectorWriter) lexElementInside(resolve htmlCollectorStateFunc) htmlCollectorStateFunc
// EX L278-289: (l *htmlElementsCollectorWriter) next() rune
// EX L296-347: htmlLexElementStart(w *htmlElementsCollectorWriter) htmlCollectorStateFunc
// EX L351-359: htmlLexStart(w *htmlElementsCollectorWriter) htmlCollectorStateFunc
//    L362-371: htmlLexToEndOfComment(w *htmlElementsCollectorWriter) htmlCollectorStateFunc
// EX L373-447: (w *htmlElementsCollectorWriter) parseHTMLElement(elStr string) (el htmlElement, err error)
// EX L453-470: parseStartTag(s string) string
// EX L473-526: isClosedByTag(b, tagName []byte) bool
// EX L528-530: isSpace(b byte) bool
//    L532-556: extractSingleQuotedStrings(s string) []string
// ---------------------------------------------------------------------------
