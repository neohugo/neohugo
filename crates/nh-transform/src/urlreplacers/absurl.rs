//! Port of `transform/urlreplacers/absurl.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).


use crate::chain::Transformer;

/// Go: `urlreplacers.NewAbsURLTransformer(path)` — HTML quotes `"` and `'`.
// Go: transform/urlreplacers/absurl.go:NewAbsURLTransformer
pub fn new_abs_url_transformer(path: &str) -> Transformer {
    let path = path.to_string();
    Box::new(move |ft| {
        super::absurlreplacer::replace_in_html(&path, ft);
        Ok(())
    })
}

/// Go: `urlreplacers.NewAbsURLInXMLTransformer(path)` — XML quotes `&#34;` and `&#39;`.
// Go: transform/urlreplacers/absurl.go:NewAbsURLInXMLTransformer
pub fn new_abs_url_in_xml_transformer(path: &str) -> Transformer {
    let path = path.to_string();
    Box::new(move |ft| {
        super::absurlreplacer::replace_in_xml(&path, ft);
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/urlreplacers/absurl.go (36 lines; 2/2 funcs executed)
// EX L22-27: NewAbsURLTransformer(path string) transform.Transformer
// EX L31-36: NewAbsURLInXMLTransformer(path string) transform.Transformer
// ---------------------------------------------------------------------------
