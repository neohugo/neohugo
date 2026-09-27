//! Port of `transform/urlreplacers/absurlreplacer.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).


//! Go `absurlreplacer.go` — the canonifyURLs lexer. Port LITERALLY including its quirks (stale
//! `nextPos`, `nextPos=0` initial value, srcset whitespace collapsing, unquoted href rewriting,
//! case-sensitive prefixes); see specs/output-publishing.md §3.3 for the test vectors.

use crate::chain::FromTo;

/// Go: `absURLReplacer.replaceInHTML`.
// Go: transform/urlreplacers/absurlreplacer.go:replaceInHTML
pub fn replace_in_html(path: &str, ft: &mut FromTo<'_>) {
    do_replace(path, ft, &[b"\"", b"'"]);
}

/// Go: `absURLReplacer.replaceInXML`.
// Go: transform/urlreplacers/absurlreplacer.go:replaceInXML
pub fn replace_in_xml(path: &str, ft: &mut FromTo<'_>) {
    do_replace(path, ft, &[b"&#34;", b"&#39;"]);
}

// Go: transform/urlreplacers/absurlreplacer.go:doReplace
fn do_replace(path: &str, ft: &mut FromTo<'_>, quotes: &[&[u8]]) {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/urlreplacers/absurlreplacer.go (274 lines; 12/12 funcs executed)
//   types: absurllexer, prefix, absURLReplacer
// EX L53-71: (p *prefix) find(bs []byte, start int) bool
// EX L73-81: newPrefixState() []*prefix
// EX L83-86: (l *absurllexer) emit()
// EX L93-102: (l *absurllexer) consumeQuote() []byte
// EX L105-131: checkCandidateBase(l *absurllexer)
// EX L133-142: (l *absurllexer) posAfterURL(q []byte) int
// EX L145-202: checkCandidateSrcset(l *absurllexer)
// EX L205-238: (l *absurllexer) replace()
// EX L240-254: doReplace(path string, ct transform.FromTo, quotes [][]byte)
// EX L261-266: newAbsURLReplacer() *absURLReplacer
// EX L268-270: (au *absURLReplacer) replaceInHTML(path string, ct transform.FromTo)
// EX L272-274: (au *absURLReplacer) replaceInXML(path string, ct transform.FromTo)
// ---------------------------------------------------------------------------
