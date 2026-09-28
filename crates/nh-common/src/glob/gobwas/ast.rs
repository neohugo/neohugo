//! Port of `github.com/gobwas/glob@v0.2.3` `syntax/ast/{ast,parser}.go` and `syntax/syntax.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).
//!
//! The parser keeps Go's parent pointers in an arena and returns an owned tree. `Node`'s derived
//! equality is Go's `Node.Equal` (kind, value, children), which is also what `reflect.DeepEqual`
//! decides for the compiler's nodes (their parent links are isomorphic and leaves have nil
//! children).

use go_unicode::utf8;

use super::lexer::{Lexer, Rune, TokenType};

/// Go: `ast.Kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Nothing,
    Pattern,
    List,
    Range,
    Text,
    Any,
    Super,
    Single,
    AnyOf,
}

/// Go: `ast.Node.Value` (`nil`, `ast.List`, `ast.Range` or `ast.Text`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Nil,
    /// Go: `ast.List{Not, Chars}`.
    List {
        not: bool,
        chars: Vec<u8>,
    },
    /// Go: `ast.Range{Not, Lo, Hi}`.
    Range {
        not: bool,
        lo: Rune,
        hi: Rune,
    },
    /// Go: `ast.Text{Text}`.
    Text(Vec<u8>),
}

/// Go: `ast.Node` (without the parent pointer).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub kind: Kind,
    pub value: Value,
    pub children: Vec<Node>,
}

impl Node {
    // Go: github.com/gobwas/glob syntax/ast/ast.go:NewNode
    pub fn new(kind: Kind, value: Value, children: Vec<Node>) -> Node {
        Node {
            kind,
            value,
            children,
        }
    }
}

/// The parser's arena node (Go's `*ast.Node` with `Parent`).
struct PNode {
    kind: Kind,
    value: Value,
    children: Vec<usize>,
    parent: Option<usize>,
}

struct Arena {
    nodes: Vec<PNode>,
}

impl Arena {
    // Go: github.com/gobwas/glob syntax/ast/ast.go:NewNode
    fn new_node(&mut self, kind: Kind, value: Value) -> usize {
        self.nodes.push(PNode {
            kind,
            value,
            children: Vec::new(),
            parent: None,
        });
        self.nodes.len() - 1
    }

    // Go: github.com/gobwas/glob syntax/ast/ast.go:Insert
    fn insert(&mut self, parent: usize, child: usize) {
        self.nodes[parent].children.push(child);
        self.nodes[child].parent = Some(parent);
    }

    fn parent(&self, n: usize) -> usize {
        self.nodes[n]
            .parent
            .expect("the lexer only emits separators and closes inside terms")
    }

    fn to_node(&self, n: usize) -> Node {
        let p = &self.nodes[n];
        Node {
            kind: p.kind,
            value: p.value.clone(),
            children: p.children.iter().map(|&c| self.to_node(c)).collect(),
        }
    }
}

/// Go: `syntax.Parse(s)` = `ast.Parse(lexer.NewLexer(s))`.
// Go: github.com/gobwas/glob syntax/syntax.go:Parse
pub fn parse(s: &[u8]) -> Result<Node, String> {
    let mut lexer = Lexer::new(s);
    parse_tokens(&mut lexer)
}

enum ParseFn {
    Main,
    Range,
}

// Go: github.com/gobwas/glob syntax/ast/parser.go:Parse
fn parse_tokens(lexer: &mut Lexer<'_>) -> Result<Node, String> {
    let mut arena = Arena { nodes: Vec::new() };
    let root = arena.new_node(Kind::Pattern, Value::Nil);

    let mut parser = Some(ParseFn::Main);
    let mut tree = root;
    while let Some(p) = parser {
        let (next, t) = match p {
            ParseFn::Main => parser_main(&mut arena, tree, lexer)?,
            ParseFn::Range => parser_range(&mut arena, tree, lexer)?,
        };
        parser = next;
        tree = t;
    }

    Ok(arena.to_node(root))
}

type ParseResult = Result<(Option<ParseFn>, usize), String>;

// Go: github.com/gobwas/glob syntax/ast/parser.go:parserMain
fn parser_main(arena: &mut Arena, tree: usize, lex: &mut Lexer<'_>) -> ParseResult {
    let token = lex.next_token();
    match token.typ {
        TokenType::Eof => Ok((None, tree)),

        TokenType::Error => Err(String::from_utf8_lossy(&token.raw).into_owned()),

        TokenType::Text => {
            let n = arena.new_node(Kind::Text, Value::Text(token.raw));
            arena.insert(tree, n);
            Ok((Some(ParseFn::Main), tree))
        }

        TokenType::Any => {
            let n = arena.new_node(Kind::Any, Value::Nil);
            arena.insert(tree, n);
            Ok((Some(ParseFn::Main), tree))
        }

        TokenType::Super => {
            let n = arena.new_node(Kind::Super, Value::Nil);
            arena.insert(tree, n);
            Ok((Some(ParseFn::Main), tree))
        }

        TokenType::Single => {
            let n = arena.new_node(Kind::Single, Value::Nil);
            arena.insert(tree, n);
            Ok((Some(ParseFn::Main), tree))
        }

        TokenType::RangeOpen => Ok((Some(ParseFn::Range), tree)),

        TokenType::TermsOpen => {
            let a = arena.new_node(Kind::AnyOf, Value::Nil);
            arena.insert(tree, a);

            let p = arena.new_node(Kind::Pattern, Value::Nil);
            arena.insert(a, p);

            Ok((Some(ParseFn::Main), p))
        }

        TokenType::Separator => {
            let p = arena.new_node(Kind::Pattern, Value::Nil);
            let parent = arena.parent(tree);
            arena.insert(parent, p);

            Ok((Some(ParseFn::Main), p))
        }

        TokenType::TermsClose => {
            let parent = arena.parent(tree);
            Ok((Some(ParseFn::Main), arena.parent(parent)))
        }

        _ => Err(format!("unexpected token: {}", token.string())),
    }
}

// Go: github.com/gobwas/glob syntax/ast/parser.go:parserRange
fn parser_range(arena: &mut Arena, tree: usize, lex: &mut Lexer<'_>) -> ParseResult {
    let mut not = false;
    let mut lo: Rune = 0;
    let mut hi: Rune = 0;
    let mut chars: Vec<u8> = Vec::new();
    loop {
        let token = lex.next_token();
        match token.typ {
            TokenType::Eof => return Err("unexpected end".to_string()),

            TokenType::Error => return Err(String::from_utf8_lossy(&token.raw).into_owned()),

            TokenType::Not => not = true,

            TokenType::RangeLo => {
                let (r, w) = utf8::decode_rune_in_string(&token.raw);
                if token.raw.len() > w {
                    return Err("unexpected length of lo character".to_string());
                }
                lo = r;
            }

            TokenType::RangeBetween => {}

            TokenType::RangeHi => {
                let (r, w) = utf8::decode_rune_in_string(&token.raw);
                if token.raw.len() > w {
                    return Err("unexpected length of lo character".to_string());
                }

                hi = r;

                if hi < lo {
                    return Err(format!(
                        "hi character '{}' should be greater than lo '{}'",
                        String::from_utf8_lossy(&utf8::rune_to_string(hi)),
                        String::from_utf8_lossy(&utf8::rune_to_string(lo))
                    ));
                }
            }

            TokenType::Text => chars = token.raw,

            TokenType::RangeClose => {
                let is_range = lo != 0 && hi != 0;
                let is_chars = !chars.is_empty();

                if is_chars == is_range {
                    return Err("could not parse range".to_string());
                }

                let n = if is_range {
                    arena.new_node(Kind::Range, Value::Range { lo, hi, not })
                } else {
                    arena.new_node(Kind::List, Value::List { chars, not })
                };
                arena.insert(tree, n);

                return Ok((Some(ParseFn::Main), tree));
            }

            _ => {}
        }
    }
}
