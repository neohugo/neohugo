//! Port of Go's `texttemplate/parse/lex_test.go` (the fork's copy, which is
//! go1.24's): the lexer tables `lexTests`, `lexDelimTests`, `lexPosTests`
//! and `TestDelimsAlphaNumeric`/`TestDelimsAndMarkers`. The tables were
//! converted mechanically from the Go source; `mk` is Go's `mkItem`.

use gotemplate::parse::ItemType::{self, *};
use gotemplate::parse::{Item, Lexer};

struct LexTest {
    name: &'static str,
    input: &'static [u8],
    items: Vec<Want>,
}

/// An expected item (Go `item` built by `mkItem`, or a literal with position).
struct Want {
    typ: ItemType,
    val: &'static [u8],
    pos: Option<(usize, usize)>,
}

fn mk(typ: ItemType, val: &'static [u8]) -> Want {
    Want {
        typ,
        val,
        pos: None,
    }
}

fn mkp(typ: ItemType, pos: usize, val: &'static [u8], line: usize) -> Want {
    Want {
        typ,
        val,
        pos: Some((pos, line)),
    }
}

// Go: lex_test.go:collect
/// Gathers the emitted items into a slice.
fn collect(input: &[u8], left: &str, right: &str) -> Vec<Item> {
    let mut l = Lexer::new("test", input, left, right);
    l.options.emit_comment = true;
    l.options.break_ok = true;
    l.options.continue_ok = true;
    let mut items = Vec::new();
    loop {
        let item = l.next_item();
        let done = item.typ == Eof || item.typ == Error;
        items.push(item);
        if done {
            break;
        }
    }
    items
}

// Go: lex_test.go:equal
fn equal(got: &[Item], want: &[Want], check_pos: bool) -> bool {
    if got.len() != want.len() {
        return false;
    }
    for (g, w) in got.iter().zip(want) {
        if g.typ != w.typ || g.val != w.val {
            return false;
        }
        if check_pos {
            let (pos, line) = w.pos.expect("position");
            if g.pos != pos || g.line != line {
                return false;
            }
        }
    }
    true
}

fn show(items: &[Item]) -> std::string::String {
    items
        .iter()
        .map(|i| {
            format!(
                "{{{:?} {} {:?} {}}}",
                i.typ,
                i.pos,
                std::string::String::from_utf8_lossy(&i.val),
                i.line
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn run(tests: Vec<LexTest>, left: &str, right: &str, check_pos: bool) {
    let mut bad = 0;
    for test in &tests {
        let items = collect(test.input, left, right);
        if !equal(&items, &test.items, check_pos) {
            bad += 1;
            eprintln!("{}: got\n\t{}", test.name, show(&items));
        }
    }
    assert_eq!(bad, 0, "{bad} lexer tests failed");
}

// Go: lex_test.go:TestLex
#[test]
fn test_lex() {
    run(lex_tests(), "", "", false);
}

// Go: lex_test.go:TestDelims
#[test]
fn test_delims() {
    run(lex_delim_tests(), "$$", "@@", false);
}

// Go: lex_test.go:TestDelimsAlphaNumeric
#[test]
fn test_delims_alpha_numeric() {
    let test = LexTest {
        name: "right delimiter with alphanumeric start",
        input: b"{{hub .host hub}}",
        items: vec![
            mk(LeftDelim, b"{{hub"),
            mk(Space, b" "),
            mk(Field, b".host"),
            mk(Space, b" "),
            mk(RightDelim, b"hub}}"),
            mk(Eof, b""),
        ],
    };
    run(vec![test], "{{hub", "hub}}", false);
}

// Go: lex_test.go:TestDelimsAndMarkers
#[test]
fn test_delims_and_markers() {
    let test = LexTest {
        name: "delims that look like markers",
        input: b"{{- .x -}} {{- - .x - -}}",
        items: vec![
            mk(LeftDelim, b"{{- "),
            mk(Field, b".x"),
            mk(RightDelim, b" -}}"),
            mk(LeftDelim, b"{{- "),
            mk(Field, b".x"),
            mk(RightDelim, b" -}}"),
            mk(Eof, b""),
        ],
    };
    run(vec![test], "{{- ", " -}}", false);
}

// Go: lex_test.go:TestPos
/// The other tests don't check position, to make the test cases easier to
/// construct. This one does.
#[test]
fn test_pos() {
    run(lex_pos_tests(), "", "", true);
}

/// Go `item.String()` (lex.go), which parse error messages use.
#[test]
fn test_item_string() {
    let items = collect(b"{{if 1 \"abcdefghijklmnop\" `x`}}", "", "");
    let got: Vec<std::string::String> = items.iter().map(|i| i.string()).collect();
    assert_eq!(
        got,
        [
            "\"{{\"",
            "<if>",
            "\" \"",
            "\"1\"",
            "\" \"",
            "\"\\\"abcdefghi\"...",
            "\" \"",
            "\"`x`\"",
            "\"}}\"",
            "EOF"
        ]
    );
}

fn lex_tests() -> Vec<LexTest> {
    vec![
        LexTest {
            name: "empty",
            input: b"",
            items: vec![mk(Eof, b"")],
        },
        LexTest {
            name: "spaces",
            input: b" \t\n",
            items: vec![mk(Text, b" \t\n"), mk(Eof, b"")],
        },
        LexTest {
            name: "text",
            input: b"now is the time",
            items: vec![mk(Text, b"now is the time"), mk(Eof, b"")],
        },
        LexTest {
            name: "text with comment",
            input: b"hello-{{/* this is a comment */}}-world",
            items: vec![
                mk(Text, b"hello-"),
                mk(Comment, b"/* this is a comment */"),
                mk(Text, b"-world"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "punctuation",
            input: b"{{,@% }}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Char, b","),
                mk(Char, b"@"),
                mk(Char, b"%"),
                mk(Space, b" "),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "parens",
            input: b"{{((3))}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(LeftParen, b"("),
                mk(LeftParen, b"("),
                mk(Number, b"3"),
                mk(RightParen, b")"),
                mk(RightParen, b")"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "empty action",
            input: b"{{}}",
            items: vec![mk(LeftDelim, b"{{"), mk(RightDelim, b"}}"), mk(Eof, b"")],
        },
        LexTest {
            name: "for",
            input: b"{{for}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Identifier, b"for"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "block",
            input: b"{{block \"foo\" .}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Block, b"block"),
                mk(Space, b" "),
                mk(String, b"\"foo\""),
                mk(Space, b" "),
                mk(Dot, b"."),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "quote",
            input: b"{{\"abc \\n\\t\\\" \"}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(String, b"\"abc \\n\\t\\\" \""),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "raw quote",
            input: b"{{`abc\\n\\t\\\" `}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(RawString, b"`abc\\n\\t\\\" `"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "raw quote with newline",
            input: b"{{`now is{{\n}}the time`}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(RawString, b"`now is{{\n}}the time`"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "numbers",
            input: b"{{1 02 0x14 0X14 -7.2i 1e3 1E3 +1.2e-4 4.2i 1+2i 1_2 0x1.e_fp4 0X1.E_FP4}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Number, b"1"),
                mk(Space, b" "),
                mk(Number, b"02"),
                mk(Space, b" "),
                mk(Number, b"0x14"),
                mk(Space, b" "),
                mk(Number, b"0X14"),
                mk(Space, b" "),
                mk(Number, b"-7.2i"),
                mk(Space, b" "),
                mk(Number, b"1e3"),
                mk(Space, b" "),
                mk(Number, b"1E3"),
                mk(Space, b" "),
                mk(Number, b"+1.2e-4"),
                mk(Space, b" "),
                mk(Number, b"4.2i"),
                mk(Space, b" "),
                mk(Complex, b"1+2i"),
                mk(Space, b" "),
                mk(Number, b"1_2"),
                mk(Space, b" "),
                mk(Number, b"0x1.e_fp4"),
                mk(Space, b" "),
                mk(Number, b"0X1.E_FP4"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "characters",
            input: b"{{'a' '\\n' '\\'' '\\\\' '\\u00FF' '\\xFF' '\xe6\x9c\xac'}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(CharConstant, b"'a'"),
                mk(Space, b" "),
                mk(CharConstant, b"'\\n'"),
                mk(Space, b" "),
                mk(CharConstant, b"'\\''"),
                mk(Space, b" "),
                mk(CharConstant, b"'\\\\'"),
                mk(Space, b" "),
                mk(CharConstant, b"'\\u00FF'"),
                mk(Space, b" "),
                mk(CharConstant, b"'\\xFF'"),
                mk(Space, b" "),
                mk(CharConstant, b"'\xe6\x9c\xac'"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "bools",
            input: b"{{true false}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Bool, b"true"),
                mk(Space, b" "),
                mk(Bool, b"false"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "dot",
            input: b"{{.}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Dot, b"."),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "nil",
            input: b"{{nil}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Nil, b"nil"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "dots",
            input: b"{{.x . .2 .x.y.z}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Field, b".x"),
                mk(Space, b" "),
                mk(Dot, b"."),
                mk(Space, b" "),
                mk(Number, b".2"),
                mk(Space, b" "),
                mk(Field, b".x"),
                mk(Field, b".y"),
                mk(Field, b".z"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "keywords",
            input: b"{{range if else end with}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Range, b"range"),
                mk(Space, b" "),
                mk(If, b"if"),
                mk(Space, b" "),
                mk(Else, b"else"),
                mk(Space, b" "),
                mk(End, b"end"),
                mk(Space, b" "),
                mk(With, b"with"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "variables",
            input: b"{{$c := printf $ $hello $23 $ $var.Field .Method}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Variable, b"$c"),
                mk(Space, b" "),
                mk(Declare, b":="),
                mk(Space, b" "),
                mk(Identifier, b"printf"),
                mk(Space, b" "),
                mk(Variable, b"$"),
                mk(Space, b" "),
                mk(Variable, b"$hello"),
                mk(Space, b" "),
                mk(Variable, b"$23"),
                mk(Space, b" "),
                mk(Variable, b"$"),
                mk(Space, b" "),
                mk(Variable, b"$var"),
                mk(Field, b".Field"),
                mk(Space, b" "),
                mk(Field, b".Method"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "variable invocation",
            input: b"{{$x 23}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Variable, b"$x"),
                mk(Space, b" "),
                mk(Number, b"23"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "pipeline",
            input: b"intro {{echo hi 1.2 |noargs|args 1 \"hi\"}} outro",
            items: vec![
                mk(Text, b"intro "),
                mk(LeftDelim, b"{{"),
                mk(Identifier, b"echo"),
                mk(Space, b" "),
                mk(Identifier, b"hi"),
                mk(Space, b" "),
                mk(Number, b"1.2"),
                mk(Space, b" "),
                mk(Pipe, b"|"),
                mk(Identifier, b"noargs"),
                mk(Pipe, b"|"),
                mk(Identifier, b"args"),
                mk(Space, b" "),
                mk(Number, b"1"),
                mk(Space, b" "),
                mk(String, b"\"hi\""),
                mk(RightDelim, b"}}"),
                mk(Text, b" outro"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "declaration",
            input: b"{{$v := 3}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Variable, b"$v"),
                mk(Space, b" "),
                mk(Declare, b":="),
                mk(Space, b" "),
                mk(Number, b"3"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "2 declarations",
            input: b"{{$v , $w := 3}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Variable, b"$v"),
                mk(Space, b" "),
                mk(Char, b","),
                mk(Space, b" "),
                mk(Variable, b"$w"),
                mk(Space, b" "),
                mk(Declare, b":="),
                mk(Space, b" "),
                mk(Number, b"3"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "field of parenthesized expression",
            input: b"{{(.X).Y}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(LeftParen, b"("),
                mk(Field, b".X"),
                mk(RightParen, b")"),
                mk(Field, b".Y"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "trimming spaces before and after",
            input: b"hello- {{- 3 -}} -world",
            items: vec![
                mk(Text, b"hello-"),
                mk(LeftDelim, b"{{"),
                mk(Number, b"3"),
                mk(RightDelim, b"}}"),
                mk(Text, b"-world"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "trimming spaces before and after comment",
            input: b"hello- {{- /* hello */ -}} -world",
            items: vec![
                mk(Text, b"hello-"),
                mk(Comment, b"/* hello */"),
                mk(Text, b"-world"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "badchar",
            input: b"#{{\x01}}",
            items: vec![
                mk(Text, b"#"),
                mk(LeftDelim, b"{{"),
                mk(Error, b"unrecognized character in action: U+0001"),
            ],
        },
        LexTest {
            name: "unclosed action",
            input: b"{{",
            items: vec![mk(LeftDelim, b"{{"), mk(Error, b"unclosed action")],
        },
        LexTest {
            name: "EOF in action",
            input: b"{{range",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Range, b"range"),
                mk(Error, b"unclosed action"),
            ],
        },
        LexTest {
            name: "unclosed quote",
            input: b"{{\"\n\"}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Error, b"unterminated quoted string"),
            ],
        },
        LexTest {
            name: "unclosed raw quote",
            input: b"{{`xx}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Error, b"unterminated raw quoted string"),
            ],
        },
        LexTest {
            name: "unclosed char constant",
            input: b"{{'\n}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Error, b"unterminated character constant"),
            ],
        },
        LexTest {
            name: "bad number",
            input: b"{{3k}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Error, b"bad number syntax: \"3k\""),
            ],
        },
        LexTest {
            name: "unclosed paren",
            input: b"{{(3}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(LeftParen, b"("),
                mk(Number, b"3"),
                mk(Error, b"unclosed left paren"),
            ],
        },
        LexTest {
            name: "extra right paren",
            input: b"{{3)}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Number, b"3"),
                mk(Error, b"unexpected right paren"),
            ],
        },
        LexTest {
            name: "long pipeline deadlock",
            input: b"{{|||||}}",
            items: vec![
                mk(LeftDelim, b"{{"),
                mk(Pipe, b"|"),
                mk(Pipe, b"|"),
                mk(Pipe, b"|"),
                mk(Pipe, b"|"),
                mk(Pipe, b"|"),
                mk(RightDelim, b"}}"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "text with bad comment",
            input: b"hello-{{/*/}}-world",
            items: vec![mk(Text, b"hello-"), mk(Error, b"unclosed comment")],
        },
        LexTest {
            name: "text with comment close separated from delim",
            input: b"hello-{{/* */ }}-world",
            items: vec![
                mk(Text, b"hello-"),
                mk(Error, b"comment ends before closing delimiter"),
            ],
        },
        LexTest {
            name: "unmatched right delimiter",
            input: b"hello-{.}}-world",
            items: vec![mk(Text, b"hello-{.}}-world"), mk(Eof, b"")],
        },
    ]
}

fn lex_delim_tests() -> Vec<LexTest> {
    vec![
        LexTest {
            name: "punctuation",
            input: b"$$,@%{{}}@@",
            items: vec![
                mk(LeftDelim, b"$$"),
                mk(Char, b","),
                mk(Char, b"@"),
                mk(Char, b"%"),
                mk(Char, b"{"),
                mk(Char, b"{"),
                mk(Char, b"}"),
                mk(Char, b"}"),
                mk(RightDelim, b"@@"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "empty action",
            input: b"$$@@",
            items: vec![mk(LeftDelim, b"$$"), mk(RightDelim, b"@@"), mk(Eof, b"")],
        },
        LexTest {
            name: "for",
            input: b"$$for@@",
            items: vec![
                mk(LeftDelim, b"$$"),
                mk(Identifier, b"for"),
                mk(RightDelim, b"@@"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "quote",
            input: b"$$\"abc \\n\\t\\\" \"@@",
            items: vec![
                mk(LeftDelim, b"$$"),
                mk(String, b"\"abc \\n\\t\\\" \""),
                mk(RightDelim, b"@@"),
                mk(Eof, b""),
            ],
        },
        LexTest {
            name: "raw quote",
            input: b"$$`abc\\n\\t\\\" `@@",
            items: vec![
                mk(LeftDelim, b"$$"),
                mk(RawString, b"`abc\\n\\t\\\" `"),
                mk(RightDelim, b"@@"),
                mk(Eof, b""),
            ],
        },
    ]
}

fn lex_pos_tests() -> Vec<LexTest> {
    vec![
        LexTest {
            name: "empty",
            input: b"",
            items: vec![mkp(Eof, 0, b"", 1)],
        },
        LexTest {
            name: "punctuation",
            input: b"{{,@%#}}",
            items: vec![
                mkp(LeftDelim, 0, b"{{", 1),
                mkp(Char, 2, b",", 1),
                mkp(Char, 3, b"@", 1),
                mkp(Char, 4, b"%", 1),
                mkp(Char, 5, b"#", 1),
                mkp(RightDelim, 6, b"}}", 1),
                mkp(Eof, 8, b"", 1),
            ],
        },
        LexTest {
            name: "sample",
            input: b"0123{{hello}}xyz",
            items: vec![
                mkp(Text, 0, b"0123", 1),
                mkp(LeftDelim, 4, b"{{", 1),
                mkp(Identifier, 6, b"hello", 1),
                mkp(RightDelim, 11, b"}}", 1),
                mkp(Text, 13, b"xyz", 1),
                mkp(Eof, 16, b"", 1),
            ],
        },
        LexTest {
            name: "trimafter",
            input: b"{{x -}}\n{{y}}",
            items: vec![
                mkp(LeftDelim, 0, b"{{", 1),
                mkp(Identifier, 2, b"x", 1),
                mkp(RightDelim, 5, b"}}", 1),
                mkp(LeftDelim, 8, b"{{", 2),
                mkp(Identifier, 10, b"y", 2),
                mkp(RightDelim, 11, b"}}", 2),
                mkp(Eof, 13, b"", 2),
            ],
        },
        LexTest {
            name: "trimbefore",
            input: b"{{x}}\n{{- y}}",
            items: vec![
                mkp(LeftDelim, 0, b"{{", 1),
                mkp(Identifier, 2, b"x", 1),
                mkp(RightDelim, 3, b"}}", 1),
                mkp(LeftDelim, 6, b"{{", 2),
                mkp(Identifier, 10, b"y", 2),
                mkp(RightDelim, 11, b"}}", 2),
                mkp(Eof, 13, b"", 2),
            ],
        },
        LexTest {
            name: "longcomment",
            input: b"{{/*\n*/}}\n{{undefinedFunction \"test\"}}",
            items: vec![
                mkp(Comment, 2, b"/*\n*/", 1),
                mkp(Text, 9, b"\n", 2),
                mkp(LeftDelim, 10, b"{{", 3),
                mkp(Identifier, 12, b"undefinedFunction", 3),
                mkp(Space, 29, b" ", 3),
                mkp(String, 30, b"\"test\"", 3),
                mkp(RightDelim, 36, b"}}", 3),
                mkp(Eof, 38, b"", 3),
            ],
        },
    ]
}
