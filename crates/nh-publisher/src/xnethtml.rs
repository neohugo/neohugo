//! Module `xnethtml`.
//!
//! NEW: golang.org/x/net/html tokenizer + tree builder subset used by parseHTMLElement (incl. in-body quirks for th/caption/col/colgroup/frame/image)
//!
//! Owner: Wave B task T07 (transform-publisher).


//! Subset of `golang.org/x/net/html` (tokenizer + tree construction "in body" insertion mode) as
//! used by `parseHTMLElement`: parse one element's start tag string and find the element node
//! with the given tag name, returning its attributes. Must reproduce: `<th>`, `<caption>`, `<col>`,
//! `<colgroup>`, `<frame>` ignored in body (tag names still collected by the caller), `<image>` ->
//! `img`, attribute name lower-casing, entity decoding in attribute values.

/// A parsed attribute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub key: String,
    pub val: String,
}

/// Go: `html.Parse(strings.NewReader(s))` + walk for `ElementNode` with `Data == tag`.
pub fn parse_element_attributes(s: &str, tag: &str) -> Option<Vec<Attribute>> {
    todo!()
}
