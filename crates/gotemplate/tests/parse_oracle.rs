//! Differential test of the parser against the forked Go
//! text/template/parse package: `tests/fixtures/text/parse.txt.gz` is
//! written by `tools/go-oracle/gotemplate/parse.go` (Go's parse/lex test
//! inputs in every mode, custom delimiters, number syntaxes, Hugo layouts
//! from this repository and seeded fuzz). For each source the Rust parser
//! must produce the same error text, or the same trees with the same
//! `String()` and the same structural dump (node kinds, positions, lines,
//! `ErrorContext` locations, number flags/values).
//!
//! Regenerate:
//!
//! ```text
//! tools/go-oracle/gotemplate/sync-fork.sh
//! GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate parse crates/gotemplate/tests/fixtures/text
//! ```

mod common_text;

use std::collections::{HashMap, HashSet};

use common_text::*;
use gotemplate::parse::{self, BranchNode, FuncNames, Mode, Node, NodeLike, Tree};

fn loc(tr: &Tree, n: &dyn NodeLike) -> String {
    let (loc, _) = parse::error_context(Some(tr), n);
    match loc.strip_prefix(&format!("{}:", tr.parse_name)) {
        Some(rest) => rest.to_string(),
        None => format!("!{}", q(&loc)),
    }
}

fn f64x(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}

fn idents(ss: &[String]) -> String {
    ss.iter().map(q).collect::<Vec<_>>().join(" ")
}

// Mirror of tools/go-oracle/gotemplate/parse.go:ptxDumpNode.
fn dump_node(out: &mut Vec<String>, tr: &Tree, n: &Node, depth: usize) {
    let ind = " ".repeat(depth);
    match n {
        Node::List(n) => dump_list(out, tr, n, depth),
        Node::Text(n) => out.push(format!("{ind}Text {} {} {}", n.pos, loc(tr, n), q(&n.text))),
        Node::Comment(n) => out.push(format!(
            "{ind}Comment {} {} {}",
            n.pos,
            loc(tr, n),
            q(&n.text)
        )),
        Node::Action(n) => {
            out.push(format!(
                "{ind}Action {} {} L{} {}",
                n.pos,
                loc(tr, n),
                n.line,
                q(n.to_bytes())
            ));
            dump_node(out, tr, &Node::Pipe(n.pipe.clone()), depth + 1);
        }
        Node::Pipe(n) => {
            out.push(format!(
                "{ind}Pipe {} {} L{} assign={} decl={} cmds={} {}",
                n.pos,
                loc(tr, n),
                n.line,
                n.is_assign,
                n.decl.len(),
                n.cmds.len(),
                q(n.to_bytes())
            ));
            for d in &n.decl {
                dump_node(out, tr, &Node::Variable(d.clone()), depth + 1);
            }
            for c in &n.cmds {
                dump_node(out, tr, &Node::Command(c.clone()), depth + 1);
            }
        }
        Node::Command(n) => {
            out.push(format!(
                "{ind}Command {} {} args={} {}",
                n.pos,
                loc(tr, n),
                n.args.len(),
                q(n.to_bytes())
            ));
            for a in &n.args {
                dump_node(out, tr, a, depth + 1);
            }
        }
        Node::Identifier(n) => out.push(format!(
            "{ind}Identifier {} {} {}",
            n.pos,
            loc(tr, n),
            q(&n.ident)
        )),
        Node::Variable(n) => out.push(format!(
            "{ind}Variable {} {} {}",
            n.pos,
            loc(tr, n),
            idents(&n.ident)
        )),
        Node::Dot(n) => out.push(format!("{ind}Dot {} {}", n.pos, loc(tr, n))),
        Node::Nil(n) => out.push(format!("{ind}Nil {} {}", n.pos, loc(tr, n))),
        Node::Field(n) => out.push(format!(
            "{ind}Field {} {} {}",
            n.pos,
            loc(tr, n),
            idents(&n.ident)
        )),
        Node::Chain(n) => {
            out.push(format!(
                "{ind}Chain {} {} {} {}",
                n.pos,
                loc(tr, n),
                idents(&n.field),
                q(n.to_bytes())
            ));
            dump_node(out, tr, &n.node, depth + 1);
        }
        Node::Bool(n) => out.push(format!("{ind}Bool {} {} {}", n.pos, loc(tr, n), n.true_)),
        Node::Number(n) => out.push(format!(
            "{ind}Number {} {} {} int={}:{} uint={}:{} float={}:{} complex={}:{}:{}",
            n.pos,
            loc(tr, n),
            q(&n.text),
            n.is_int,
            n.int64,
            n.is_uint,
            n.uint64,
            n.is_float,
            f64x(n.float64),
            n.is_complex,
            f64x(n.complex128.0),
            f64x(n.complex128.1)
        )),
        Node::String(n) => out.push(format!(
            "{ind}String {} {} {} {}",
            n.pos,
            loc(tr, n),
            q(&n.quoted),
            q(&n.text)
        )),
        Node::If(b) => dump_branch(out, tr, "If", b, depth),
        Node::Range(b) => dump_branch(out, tr, "Range", b, depth),
        Node::With(b) => dump_branch(out, tr, "With", b, depth),
        Node::Template(n) => {
            out.push(format!(
                "{ind}Template {} {} L{} {} {}",
                n.pos,
                loc(tr, n),
                n.line,
                q(&n.name),
                q(n.to_bytes())
            ));
            match &n.pipe {
                None => out.push(format!("{ind} nopipe")),
                Some(p) => dump_node(out, tr, &Node::Pipe(p.clone()), depth + 1),
            }
        }
        Node::Break(n) => out.push(format!("{ind}Break {} {} L{}", n.pos, loc(tr, n), n.line)),
        Node::Continue(n) => out.push(format!(
            "{ind}Continue {} {} L{}",
            n.pos,
            loc(tr, n),
            n.line
        )),
        Node::Else(_) | Node::End(_) => out.push(format!("{ind}UNKNOWN")),
    }
}

fn dump_list(out: &mut Vec<String>, tr: &Tree, n: &parse::ListNode, depth: usize) {
    let ind = " ".repeat(depth);
    out.push(format!("{ind}List {} {}", n.pos, loc(tr, n)));
    for c in &n.nodes {
        dump_node(out, tr, c, depth + 1);
    }
}

fn dump_branch(out: &mut Vec<String>, tr: &Tree, kind: &str, b: &BranchNode, depth: usize) {
    let ind = " ".repeat(depth);
    out.push(format!("{ind}{kind} {} {} L{}", b.pos, loc(tr, b), b.line));
    dump_node(out, tr, &Node::Pipe(b.pipe.clone()), depth + 1);
    dump_list(out, tr, &b.list, depth + 1);
    match &b.else_list {
        None => out.push(format!("{ind} noelse")),
        Some(e) => {
            out.push(format!("{ind} else"));
            dump_list(out, tr, e, depth + 1);
        }
    }
}

struct Names(HashSet<String>);

impl FuncNames for Names {
    fn has_function(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}

#[test]
fn parse_oracle() {
    let text = read_fixture("parse.txt.gz");
    let (n, lossy_names, bad) = check_parse_fixture(&text, 20);
    assert!(n > 4000, "fixture has {n} cases");
    eprintln!("{n} parse cases; {lossy_names} differ only by lossy UTF-8 template names");
    assert!(lossy_names <= 8, "{lossy_names} lossy-name cases");
    assert_eq!(bad, 0, "{bad} of {n} parse cases differ from Go");
}

/// Red-team regressions: minimized cases that once differed from Go
/// (`tools/go-oracle/gotemplate/redteam_fixtures.go`).
#[test]
fn parse_redteam_regressions() {
    let text = read_fixture("redteam_parse.txt.gz");
    let (n, _, bad) = check_parse_fixture(&text, 50);
    assert!(n > 0);
    assert_eq!(bad, 0, "{bad} of {n} red-team parse cases differ from Go");
}

/// The large seeded red-team corpus (`rtparse` oracle mode, not checked
/// in): `GOTEMPLATE_RT_PARSE=<file.txt.gz> cargo test --test parse_oracle -- --ignored`.
#[test]
#[ignore]
fn parse_redteam() {
    let Some(path) = std::env::var_os("GOTEMPLATE_RT_PARSE") else {
        eprintln!("GOTEMPLATE_RT_PARSE not set");
        return;
    };
    for path in std::env::split_paths(&path) {
        let text = read_gz_path(&path);
        let (n, lossy_names, bad) = check_parse_fixture(&text, 50);
        eprintln!(
            "{}: {n} parse cases; {lossy_names} differ only by PORTING deviations 5 (lossy UTF-8 names and messages) or 17 (% verbs in names); {bad} differ",
            path.display()
        );
        assert_eq!(bad, 0, "{bad} of {n} parse cases differ from Go");
    }
}

/// Replays a parse fixture; returns (cases, lossy-name cases, differing
/// cases), printing the first `show` differences.
fn check_parse_fixture(text: &str, show: usize) -> (usize, usize, usize) {
    let mut func_sets: HashMap<String, Names> = HashMap::new();
    for line in text.lines().take_while(|l| l.starts_with("funcs ")) {
        let f = fields(line);
        func_sets.insert(
            f[1].to_string(),
            Names(f[2..].iter().map(|s| unquote_str(s)).collect()),
        );
    }
    let recs = records(text);
    let mut bad = 0;
    let mut lossy_names = 0;
    for rec in &recs {
        let name = String::from_utf8_lossy(&unquote(rec.header[1])).into_owned();
        let mode_line = fields(rec.lines[0]);
        assert_eq!(mode_line[0], "mode");
        let mode = Mode(mode_line[1].parse().unwrap());
        let left = String::from_utf8_lossy(&unquote(mode_line[2])).into_owned();
        let right = String::from_utf8_lossy(&unquote(mode_line[3])).into_owned();
        let funcs = &func_sets[mode_line[4]];
        let src_line = rec.lines[1].strip_prefix("src ").unwrap();
        let src = unquote(src_line);
        let mut got: Vec<String> = rec.lines[..2].iter().map(|s| s.to_string()).collect();
        let funcs: [&dyn FuncNames; 1] = [funcs];
        match parse::parse_with_mode(&name, &src, &left, &right, mode, &funcs) {
            Err(e) => got.push(format!("err {}", q(e.to_string()))),
            Ok(trees) => {
                for (_, tr) in trees.iter() {
                    got.push(format!(
                        "tree {} {} {}",
                        q(&tr.name),
                        q(&tr.parse_name),
                        tr.mode.0
                    ));
                    let root = tr.root.as_ref().expect("root");
                    got.push(format!("str {}", q(root.to_bytes())));
                    dump_list(&mut got, tr, root, 0);
                    let cp = tr.copy();
                    if cp.root.as_ref().unwrap().to_bytes() != root.to_bytes() {
                        got.push("copy-differs".to_string());
                    }
                }
            }
        }
        if let Some(d) = first_diff(&rec.lines, &got) {
            // Known deviation: template names are Rust strings, so a
            // define/block/template name with invalid UTF-8 (only possible
            // through `\x` escapes in its quoted name) is converted lossily.
            if has_invalid_utf8_name(&rec.lines) && same_shape(&rec.lines, &got) {
                lossy_names += 1;
                continue;
            }
            // Known deviation (PORTING deviation 5): error texts are Rust
            // strings, so invalid UTF-8 quoted in a message is replaced.
            if lossy_err_equal(&rec.lines, &got) {
                lossy_names += 1;
                continue;
            }
            // Known deviation (PORTING deviation 17): Go splices the
            // template name into errorf's format string, so `%` verbs in a
            // name consume the message's arguments (printing Go pointers).
            if name.contains('%') && rec.lines.len() == 3 && got.len() == 3 {
                lossy_names += 1;
                continue;
            }
            bad += 1;
            if bad <= show {
                eprintln!(
                    "case {} {}: {} src {}\n  {d}",
                    rec.header[0], rec.header[1], rec.lines[0], src_line
                );
            }
        }
    }
    (recs.len(), lossy_names, bad)
}

/// Whether both dumps are one error line whose texts are equal once Go's
/// invalid UTF-8 is replaced by U+FFFD.
fn lossy_err_equal(want: &[&str], got: &[String]) -> bool {
    let (Some(w), Some(g)) = (want.get(2), got.get(2)) else {
        return false;
    };
    if !(want.len() == 3 && got.len() == 3 && w.starts_with("err ") && g.starts_with("err ")) {
        return false;
    }
    let (w, g) = (
        String::from_utf8_lossy(&unquote(&w[4..])).into_owned(),
        String::from_utf8_lossy(&unquote(&g[4..])).into_owned(),
    );
    // A name quoted in the message (`%q`): Go escapes the invalid bytes.
    w == g || collapse_replacement(&replace_high_escapes(&w)) == collapse_replacement(&g)
}

/// Replaces `\xHH` escapes of non-ASCII bytes (Go's `%q` of invalid UTF-8)
/// by U+FFFD.
fn replace_high_escapes(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\'
            && i + 3 < b.len()
            && b[i + 1] == b'x'
            && b[i + 2].is_ascii_hexdigit()
            && b[i + 3].is_ascii_hexdigit()
            && b[i + 2] >= b'8'
        {
            out.push('\u{fffd}');
            i += 4;
            continue;
        }
        let c = s[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Collapses runs of U+FFFD.
fn collapse_replacement(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c == '\u{fffd}' && out.ends_with('\u{fffd}') {
            continue;
        }
        out.push(c);
    }
    out
}

/// Whether a Go dump names a template (a `tree` line or a `Template`
/// node) whose name is not valid UTF-8.
fn has_invalid_utf8_name(lines: &[&str]) -> bool {
    lines.iter().any(|l| {
        let f = fields(l.trim_start());
        let name = match f.first() {
            Some(&"tree") => f.get(1),
            Some(&"Template") => f.get(4),
            _ => None,
        };
        name.is_some_and(|n| std::str::from_utf8(&unquote(n)).is_err())
    })
}

/// Same number of lines, and the same node kinds in the same places.
fn same_shape(want: &[&str], got: &[String]) -> bool {
    want.len() == got.len()
        && want.iter().zip(got).all(|(w, g)| {
            w.split(' ').find(|x| !x.is_empty()) == g.split(' ').find(|x| !x.is_empty())
        })
}
