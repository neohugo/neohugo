//! Port of `golang.org/x/net@v0.41.0/html/doctype.go`.

use super::node::{Node, NodeType};
use super::token::Attribute;

/// Go: `whitespace` (parse.go).
pub const WHITESPACE: &[u8] = b" \t\r\n\x0c";

/// `strings.TrimLeft(s, cutset)` for an ASCII cutset.
pub fn trim_left<'a>(s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    let n = s.iter().take_while(|c| cutset.contains(c)).count();
    &s[n..]
}

/// Go: `parseDoctype(s)` — parses the data from a DoctypeToken into a name, public identifier,
/// and system identifier. It returns a Node whose Type is DoctypeNode, whose Data is the name,
/// and which has attributes named "system" and "public" for the two identifiers if they were
/// present. quirks is whether the document should be parsed in "quirks mode".
// Go: html/doctype.go:parseDoctype
pub fn parse_doctype(s: &[u8]) -> (Node, bool) {
    let mut n = Node::new(NodeType::Doctype, 0, Vec::new(), Vec::new());
    let mut quirks = false;

    let space = s
        .iter()
        .position(|c| WHITESPACE.contains(c))
        .unwrap_or(s.len());
    n.data = s[..space].to_vec();
    if n.data != b"html" {
        quirks = true;
    }
    n.data = go_unicode::strings::to_lower(&n.data).into_owned();
    let mut s = trim_left(&s[space..], WHITESPACE);

    if s.len() < 6 {
        // It is the end of the doctype; either no identifiers or a quirky "html5" doctype.
        return (n, quirks || !s.is_empty());
    }

    let mut key = go_unicode::strings::to_lower(&s[..6]).into_owned();
    s = &s[6..];
    while key == b"public" || key == b"system" {
        s = trim_left(s, WHITESPACE);
        if s.is_empty() {
            break;
        }
        let quote = s[0];
        if quote != b'"' && quote != b'\'' {
            break;
        }
        s = &s[1..];
        let id;
        match s.iter().position(|&c| c == quote) {
            None => {
                id = s.to_vec();
                s = b"";
            }
            Some(q) => {
                id = s[..q].to_vec();
                s = &s[q + 1..];
            }
        }
        n.attr.push(Attribute {
            namespace: Vec::new(),
            key: key.clone(),
            val: id,
        });
        if key == b"public" {
            key = b"system".to_vec();
        } else {
            key = Vec::new();
        }
    }

    if !key.is_empty() || !s.is_empty() {
        quirks = true;
    } else if !n.attr.is_empty() {
        if n.attr[0].key == b"public" {
            let public = go_unicode::strings::to_lower(&n.attr[0].val).into_owned();
            match public.as_slice() {
                b"-//w3o//dtd w3 html strict 3.0//en//"
                | b"-/w3d/dtd html 4.0 transitional/en"
                | b"html" => quirks = true,
                _ => {
                    for q in QUIRKY_IDS {
                        if public.starts_with(q.as_bytes()) {
                            quirks = true;
                            break;
                        }
                    }
                }
            }
            // The following two public IDs only cause quirks mode if there is no system ID.
            if n.attr.len() == 1
                && (public.starts_with(b"-//w3c//dtd html 4.01 frameset//")
                    || public.starts_with(b"-//w3c//dtd html 4.01 transitional//"))
            {
                quirks = true;
            }
        }
        let last_attr = &n.attr[n.attr.len() - 1];
        if last_attr.key == b"system"
            && go_unicode::strings::equal_fold(
                &last_attr.val,
                b"http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd",
            )
        {
            quirks = true;
        }
    }

    (n, quirks)
}

/// Go: `quirkyIDs` — public doctype identifiers that cause a document to be interpreted in
/// quirks mode.
const QUIRKY_IDS: &[&str] = &[
    "+//silmaril//dtd html pro v0r11 19970101//",
    "-//advasoft ltd//dtd html 3.0 aswedit + extensions//",
    "-//as//dtd html 3.0 aswedit + extensions//",
    "-//ietf//dtd html 2.0 level 1//",
    "-//ietf//dtd html 2.0 level 2//",
    "-//ietf//dtd html 2.0 strict level 1//",
    "-//ietf//dtd html 2.0 strict level 2//",
    "-//ietf//dtd html 2.0 strict//",
    "-//ietf//dtd html 2.0//",
    "-//ietf//dtd html 2.1e//",
    "-//ietf//dtd html 3.0//",
    "-//ietf//dtd html 3.2 final//",
    "-//ietf//dtd html 3.2//",
    "-//ietf//dtd html 3//",
    "-//ietf//dtd html level 0//",
    "-//ietf//dtd html level 1//",
    "-//ietf//dtd html level 2//",
    "-//ietf//dtd html level 3//",
    "-//ietf//dtd html strict level 0//",
    "-//ietf//dtd html strict level 1//",
    "-//ietf//dtd html strict level 2//",
    "-//ietf//dtd html strict level 3//",
    "-//ietf//dtd html strict//",
    "-//ietf//dtd html//",
    "-//metrius//dtd metrius presentational//",
    "-//microsoft//dtd internet explorer 2.0 html strict//",
    "-//microsoft//dtd internet explorer 2.0 html//",
    "-//microsoft//dtd internet explorer 2.0 tables//",
    "-//microsoft//dtd internet explorer 3.0 html strict//",
    "-//microsoft//dtd internet explorer 3.0 html//",
    "-//microsoft//dtd internet explorer 3.0 tables//",
    "-//netscape comm. corp.//dtd html//",
    "-//netscape comm. corp.//dtd strict html//",
    "-//o'reilly and associates//dtd html 2.0//",
    "-//o'reilly and associates//dtd html extended 1.0//",
    "-//o'reilly and associates//dtd html extended relaxed 1.0//",
    "-//softquad software//dtd hotmetal pro 6.0::19990601::extensions to html 4.0//",
    "-//softquad//dtd hotmetal pro 4.0::19971010::extensions to html 4.0//",
    "-//spyglass//dtd html 2.0 extended//",
    "-//sq//dtd html 2.0 hotmetal + extensions//",
    "-//sun microsystems corp.//dtd hotjava html//",
    "-//sun microsystems corp.//dtd hotjava strict html//",
    "-//w3c//dtd html 3 1995-03-24//",
    "-//w3c//dtd html 3.2 draft//",
    "-//w3c//dtd html 3.2 final//",
    "-//w3c//dtd html 3.2//",
    "-//w3c//dtd html 3.2s draft//",
    "-//w3c//dtd html 4.0 frameset//",
    "-//w3c//dtd html 4.0 transitional//",
    "-//w3c//dtd html experimental 19960712//",
    "-//w3c//dtd html experimental 970421//",
    "-//w3c//dtd w3 html//",
    "-//w3o//dtd w3 html 3.0//",
    "-//webtechs//dtd mozilla html 2.0//",
    "-//webtechs//dtd mozilla html//",
];
