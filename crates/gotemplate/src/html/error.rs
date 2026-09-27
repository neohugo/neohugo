//! Go: tpl/internal/go_templates/htmltemplate/error.go

use std::fmt;
use std::sync::Arc;

use crate::parse::{NodeLike, NodeType, Pos, TreeRef, TreeSrc, error_context};

/// Go: `ErrorCode` — a code for a kind of error.
///
/// We define codes for each error that manifests while escaping templates,
/// but escaped templates may also fail at runtime.
///
/// Output: "ZgotmplZ" is a special value that indicates that unsafe content
/// reached a CSS or URL context at runtime.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum ErrorCode {
    /// Go: `OK` — the lack of an error.
    Ok = 0,
    /// Go: `ErrAmbigContext` — "... appears in an ambiguous context within a URL".
    ErrAmbigContext,
    /// Go: `ErrBadHTML` — "expected space, attr name, or end of tag, but got
    /// ...", "... in unquoted attr", "... in attribute name".
    ErrBadHTML,
    /// Go: `ErrBranchEnd` — "{{if}} branches end in different contexts".
    ErrBranchEnd,
    /// Go: `ErrEndContext` — "... ends in a non-text context: ...".
    ErrEndContext,
    /// Go: `ErrNoSuchTemplate` — "no such template ...".
    ErrNoSuchTemplate,
    /// Go: `ErrOutputContext` — "cannot compute output context for template ...".
    ErrOutputContext,
    /// Go: `ErrPartialCharset` — "unfinished JS regexp charset in ...".
    ErrPartialCharset,
    /// Go: `ErrPartialEscape` — "unfinished escape sequence in ...".
    ErrPartialEscape,
    /// Go: `ErrRangeLoopReentry` — "on range loop re-entry: ...".
    ErrRangeLoopReentry,
    /// Go: `ErrSlashAmbig` — '/' could start a division or regexp.
    ErrSlashAmbig,
    /// Go: `ErrPredefinedEscaper` — "predefined escaper ... disallowed in template".
    ErrPredefinedEscaper,
    /// Go: `ErrJSTemplate` — "... appears in a JS template literal"
    /// (deprecated: no longer returned).
    ErrJSTemplate,
}

impl ErrorCode {
    /// Go's numeric value (`ErrorCode` is an `int`).
    pub fn value(self) -> i64 {
        self as u8 as i64
    }
}

/// Go: the `Node` field of an `Error` (a `parse.Node`).
///
/// Go keeps the node only to compute the error location with
/// `(*parse.Tree)(nil).ErrorContext(e.Node)`, which reads the node's
/// position and its tree (`tr`); the context text is discarded. `ErrNode`
/// keeps exactly those two things, so an `Error` does not own a copy of a
/// parse subtree.
#[derive(Clone, Debug)]
pub struct ErrNode {
    pub pos: Pos,
    pub tr: TreeRef,
}

impl ErrNode {
    /// The location-relevant parts of `n`.
    pub fn from_node(n: &dyn NodeLike) -> ErrNode {
        ErrNode {
            pos: n.position(),
            tr: n.tree().cloned(),
        }
    }
}

impl NodeLike for ErrNode {
    /// Not meaningful: only `position` and `tree` are used (by
    /// `ErrorContext`).
    fn node_type(&self) -> NodeType {
        NodeType::Text
    }
    fn position(&self) -> Pos {
        self.pos
    }
    fn tree(&self) -> Option<&Arc<TreeSrc>> {
        self.tr.as_ref()
    }
    /// The context string of `ErrorContext` is discarded by `Error()`.
    fn write_to(&self, _sb: &mut Vec<u8>) {}
}

/// Go: `Error` — a problem encountered during template escaping.
///
/// Go passes `*Error` around and mutates it in place (`c.err.Name = name`,
/// `c0.err.Line = ...`); contexts compare errors by pointer. Here contexts
/// hold an `Arc<Error>`, `Clone` gives a new error value.
#[derive(Clone, Debug)]
pub struct Error {
    /// ErrorCode describes the kind of error.
    pub error_code: ErrorCode,
    /// Node is the node that caused the problem, if known.
    /// If not nil, it overrides Name and Line.
    pub node: Option<ErrNode>,
    /// Name is the name of the template in which the error was encountered.
    pub name: String,
    /// Line is the line number of the error in the template source or 0.
    pub line: usize,
    /// Description is a human-readable description of the problem.
    pub description: String,
}

impl Error {
    /// `&Error{code, nil, "", 0, description}`.
    pub fn new(error_code: ErrorCode, description: String) -> Error {
        Error {
            error_code,
            node: None,
            name: String::new(),
            line: 0,
            description,
        }
    }
}

// Go: error.go:(*Error).Error
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(node) = &self.node {
            let (loc, _) = error_context(None, node);
            write!(f, "html/template:{}: {}", loc, self.description)
        } else if self.line != 0 {
            write!(
                f,
                "html/template:{}:{}: {}",
                self.name, self.line, self.description
            )
        } else if !self.name.is_empty() {
            write!(f, "html/template:{}: {}", self.name, self.description)
        } else {
            write!(f, "html/template: {}", self.description)
        }
    }
}

impl std::error::Error for Error {}

// Go: error.go:errorf
/// errorf creates an error given a (pre-formatted) description. The
/// template Name still needs to be supplied.
pub(crate) fn errorf(k: ErrorCode, node: Option<&dyn NodeLike>, line: usize, msg: String) -> Error {
    Error {
        error_code: k,
        node: node.map(ErrNode::from_node),
        name: String::new(),
        line,
        description: msg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_strings() {
        let mut e = errorf(ErrorCode::ErrBadHTML, None, 0, "boom".to_string());
        assert_eq!(e.to_string(), "html/template: boom");
        e.name = "t".to_string();
        assert_eq!(e.to_string(), "html/template:t: boom");
        e.line = 3;
        assert_eq!(e.to_string(), "html/template:t:3: boom");
        let src = Arc::new(TreeSrc {
            parse_name: "p".to_string(),
            text: Arc::from(&b"ab\ncd{{.X}}"[..]),
        });
        e.node = Some(ErrNode {
            pos: 5,
            tr: Some(src),
        });
        assert_eq!(e.to_string(), "html/template:p:2:2: boom");
        assert_eq!(ErrorCode::ErrJSTemplate.value(), 12);
        assert_eq!(ErrorCode::Ok.value(), 0);
    }
}
