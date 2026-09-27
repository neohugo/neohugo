//! Differential test of html/template contextual escaping against the Go
//! fork: `tools/go-oracle/gotemplate/escdump.go` parses real layouts (the
//! docs site, the theme skeleton and Hugo's embedded templates) into
//! html/template namespaces the way tpl/tplimpl does, escapes every
//! file-level template and dumps every tree afterwards. This test replays
//! the same procedure and compares the dumps byte for byte.
//!
//! Regenerate (repo root):
//!   tools/go-oracle/gotemplate/sync-fork.sh
//!   GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate escdump \
//!     crates/gotemplate/tests/fixtures/html/escdump.txt \
//!     docs/layouts create/skeletons/theme/layouts tpl/tplimpl/embedded/templates

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gotemplate::html::Template;
use gotemplate::parse::{self, ListNode, Mode, Node};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct LayoutFile {
    name: String,
    content: Vec<u8>,
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<LayoutFile>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        let p = e.path();
        if p.is_dir() {
            walk(&p, root, out);
            continue;
        }
        let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
        if ext != "html" && ext != "xml" {
            continue;
        }
        let rel = p
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let b = std::fs::read(&p).unwrap();
        out.push(LayoutFile {
            name: rel,
            content: remove_leading_bom(&b).to_vec(),
        });
    }
}

fn read_layouts(dir: &Path) -> Vec<LayoutFile> {
    let mut v = Vec::new();
    walk(dir, dir, &mut v);
    v.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    v
}

// Go: tpl/tplimpl/templates.go:removeLeadingBOM
fn remove_leading_bom(s: &[u8]) -> &[u8] {
    s.strip_prefix("\u{feff}".as_bytes()).unwrap_or(s)
}

// Go: tpl/tplimpl/templates.go:needsBaseTemplate
fn needs_base_template(templ: &[u8]) -> bool {
    let mut idx: Option<usize> = None;
    let mut in_comment = false;
    let mut i = 0;
    while i < templ.len() {
        let rest = &templ[i..];
        if !in_comment && rest.starts_with(b"{{/*") {
            in_comment = true;
            i += 4;
        } else if !in_comment && rest.starts_with(b"{{- /*") {
            in_comment = true;
            i += 6;
        } else if in_comment && rest.starts_with(b"*/}}") {
            in_comment = false;
            i += 4;
        } else if in_comment && rest.starts_with(b"*/ -}}") {
            in_comment = false;
            i += 6;
        } else {
            let (r, size) = go_unicode::utf8::decode_rune_in_string(rest);
            if !in_comment {
                if rest.starts_with(b"{{") {
                    idx = Some(i);
                    break;
                } else if !go_unicode::is_space(r) {
                    break;
                }
            }
            i += size;
        }
    }
    let Some(idx) = idx else { return false };
    // `^{{-?\s*define`
    let mut s = &templ[idx + 2..];
    if s.first() == Some(&b'-') {
        s = &s[1..];
    }
    while let Some(c) = s.first() {
        if matches!(c, b' ' | b'\t' | b'\n' | b'\x0c' | b'\r') {
            s = &s[1..];
        } else {
            break;
        }
    }
    s.starts_with(b"define")
}

fn collect_idents(name: &str, text: &[u8], into: &mut BTreeSet<String>) {
    let Ok(trees) = parse::parse_with_mode(name, text, "", "", Mode::SKIP_FUNC_CHECK, &[]) else {
        return;
    };
    fn walk_list(l: &ListNode, into: &mut BTreeSet<String>) {
        for n in &l.nodes {
            walk_node(n, into);
        }
    }
    fn walk_pipe(p: &parse::PipeNode, into: &mut BTreeSet<String>) {
        for c in &p.cmds {
            for a in &c.args {
                walk_node(a, into);
            }
        }
    }
    fn walk_node(n: &Node, into: &mut BTreeSet<String>) {
        match n {
            Node::List(l) => walk_list(l, into),
            Node::Action(a) => walk_pipe(&a.pipe, into),
            Node::Pipe(p) => walk_pipe(p, into),
            Node::Command(c) => {
                for a in &c.args {
                    walk_node(a, into);
                }
            }
            Node::Identifier(i) => {
                into.insert(i.ident.clone());
            }
            Node::Chain(c) => walk_node(&c.node, into),
            Node::If(b) | Node::Range(b) | Node::With(b) => {
                walk_pipe(&b.pipe, into);
                walk_list(&b.list, into);
                if let Some(e) = &b.else_list {
                    walk_list(e, into);
                }
            }
            Node::Template(t) => {
                if let Some(p) = &t.pipe {
                    walk_pipe(p, into);
                }
            }
            _ => {}
        }
    }
    for tree in trees.values() {
        if let Some(root) = &tree.root {
            walk_list(root, into);
        }
    }
}

fn esc_dump_dir(out: &mut Vec<u8>, label: &str, dir: &Path) {
    use std::io::Write;
    let files = read_layouts(dir);
    let mut idents = BTreeSet::new();
    for f in &files {
        collect_idents(&f.name, &f.content, &mut idents);
    }

    let base = files
        .iter()
        .find(|f| f.name == "baseof.html" || f.name == "_default/baseof.html");

    writeln!(out, "=== DIR {label}").unwrap();
    let shared = Template::new("");
    shared.func_names(idents.iter().cloned());
    let mut shared_names = Vec::new();
    let mut overlays = Vec::new();
    for f in &files {
        if let Some(b) = base {
            if f.name == b.name {
                continue;
            }
            if needs_base_template(&f.content) {
                overlays.push(f);
                continue;
            }
        }
        if let Err(e) = shared.new_associated(&f.name).parse(&f.content) {
            writeln!(out, "--- PARSE {}: ERR {e}", f.name).unwrap();
            continue;
        }
        shared_names.push(f.name.clone());
    }
    let mut namespaces: Vec<(String, Template)> = vec![("shared".to_string(), shared.clone())];
    let mut overlay_names = Vec::new();
    for f in overlays {
        let tt = shared.clone_shallow().unwrap().new_associated(&f.name);
        if let Err(e) = tt.parse(&base.unwrap().content) {
            writeln!(out, "--- PARSE base {}: ERR {e}", f.name).unwrap();
            continue;
        }
        if let Err(e) = tt.parse(&f.content) {
            writeln!(out, "--- PARSE {}: ERR {e}", f.name).unwrap();
            continue;
        }
        namespaces.push((format!("base:{}", f.name), tt));
        overlay_names.push(f.name.clone());
    }

    for name in &shared_names {
        let t = shared.lookup(name).unwrap();
        match t.prepare() {
            Err(e) => writeln!(out, "--- ESCAPE {name}: ERR {e}").unwrap(),
            Ok(_) => writeln!(out, "--- ESCAPE {name}: OK").unwrap(),
        }
    }
    for (i, (_, t)) in namespaces[1..].iter().enumerate() {
        match t.prepare() {
            Err(e) => writeln!(out, "--- ESCAPE {}: ERR {e}", overlay_names[i]).unwrap(),
            Ok(_) => writeln!(out, "--- ESCAPE {}: OK", overlay_names[i]).unwrap(),
        }
    }

    for (label, t) in &namespaces {
        writeln!(out, "=== NAMESPACE {label}").unwrap();
        let text = t.text();
        let mut ts = text.templates();
        ts.sort_by(|a, b| a.name().as_bytes().cmp(b.name().as_bytes()));
        for t in ts {
            let s = match t.tree().map(|tr| tr.get()) {
                Some(tree) if tree.root.is_some() => {
                    use gotemplate::parse::NodeLike;
                    tree.root.as_ref().unwrap().to_bytes()
                }
                _ => b"<nil>".to_vec(),
            };
            writeln!(
                out,
                "### TEMPLATE {} {}",
                go_strconv::quote(t.name()),
                s.len()
            )
            .unwrap();
            out.extend_from_slice(&s);
            out.push(b'\n');
        }
    }
}

#[test]
fn escdump_matches_go() {
    let root = repo_root();
    let mut out = Vec::new();
    for dir in [
        "docs/layouts",
        "create/skeletons/theme/layouts",
        "tpl/tplimpl/embedded/templates",
    ] {
        esc_dump_dir(&mut out, dir, &root.join(dir));
    }
    let want = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/html/escdump.txt"),
    )
    .unwrap();
    if out != want {
        let got = String::from_utf8_lossy(&out);
        let want = String::from_utf8_lossy(&want);
        let gl: Vec<&str> = got.lines().collect();
        let wl: Vec<&str> = want.lines().collect();
        for (i, (g, w)) in gl.iter().zip(wl.iter()).enumerate() {
            if g != w {
                panic!(
                    "escdump differs at line {}:\n  got:  {}\n  want: {}",
                    i + 1,
                    &g[..g.len().min(400)],
                    &w[..w.len().min(400)]
                );
            }
        }
        panic!(
            "escdump differs in length: got {} lines, want {}",
            gl.len(),
            wl.len()
        );
    }
}
