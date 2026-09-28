//! Module `xnethtml`.
//!
//! NEW: golang.org/x/net/html tokenizer + tree builder subset used by parseHTMLElement (incl.
//! in-body quirks for th/caption/col/colgroup/frame/image)
//!
//! Owner: Wave B task T07 (transform-publisher).

//! A port of `golang.org/x/net@v0.41.0/html` as used by `parseHTMLElement`: `html.Parse` of one
//! element string (the tokenizer `token.go`, the whole tree builder `parse.go` with every
//! insertion mode, `node.go`, `foreign.go`, `const.go`, `doctype.go`, the unescaping half of
//! `escape.go` and the `atom` package). The collector's element strings are arbitrary text
//! between `<` and an unquoted `>`, so any insertion mode can be reached; the whole algorithm is
//! ported, not just "in body". Rendering, `ParseFragment` and the tokenizer's streaming/`maxBuf`
//! machinery are not ported (never used). `html5ever` and similar crates are not substitutes:
//! the collected attributes depend on Go's exact quirks (see `PORTING.md`).

pub mod atom;
mod atom_table;
pub mod doctype;
pub mod escape;
pub mod foreign;
pub mod node;
pub mod parse;
pub mod token;

pub use atom::Atom;
pub use node::{Document, Node, NodeId, NodeType};
pub use parse::parse;
pub use token::{Attribute, Token, TokenType, Tokenizer};

/// Go: `html.Parse(strings.NewReader(s))` + a walk (document order) for the element nodes with
/// `Data == tag`, returning their attributes in walk order (`None` when the parse fails, i.e.
/// where Go panics).
pub fn parse_element_attributes(s: &[u8], tag: &[u8]) -> Option<Vec<Attribute>> {
    let (d, root) = parse(s).ok()?;
    let mut out = Vec::new();
    walk_elements(&d, root, &mut |n| {
        if n.data == tag {
            out.extend(n.attr.iter().cloned());
        }
    });
    Some(out)
}

/// Visits every element node below (and including) `root` in document order (Go's recursive
/// `walk` in `parseHTMLElement`).
pub fn walk_elements(d: &Document, root: NodeId, f: &mut dyn FnMut(&Node)) {
    // An explicit stack instead of Go's recursion (deep trees must not overflow the Rust stack);
    // the visiting order is the same pre-order.
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let n = &d.nodes[id];
        if n.typ == NodeType::Element {
            f(n);
        }
        let children = d.children(id);
        for c in children.into_iter().rev() {
            stack.push(c);
        }
    }
}
