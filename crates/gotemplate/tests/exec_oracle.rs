//! Differential test of text/template execution against the forked Go
//! engine, with a Hugo-like harness: `tests/fixtures/text/exec.txt.gz` is
//! written by `tools/go-oracle/gotemplate/exec.go`, whose data model,
//! functions and `ExecHelper` are mirrored by `common_text/model.rs`. Each
//! case runs in "plain" mode (Go `Template.Execute`: reflection lookups) and
//! in "hugo" mode (`Executer.ExecuteWithContext` with Hugo's helper
//! semantics); output bytes and error text must match.
//!
//! Regenerate:
//!
//! ```text
//! tools/go-oracle/gotemplate/sync-fork.sh
//! GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate exec crates/gotemplate/tests/fixtures/text
//! ```

mod common_text;

use std::collections::BTreeMap;
use std::sync::Arc;

use common_text::model::*;
use common_text::*;
use gotemplate::Error;
use gotemplate::text::{Executer, Template};

/// Runs one case; returns the dump lines after `src`.
pub fn run_case(mode: &str, data_kind: &str, opt: &str, exec: &str, src: &[u8]) -> Vec<String> {
    let d = etx_data(data_kind);
    let t = Template::new("t");
    if !opt.is_empty() {
        t.set_option(&[opt]);
    }
    t.funcs(&etx_func_map());
    if src != b"\x00noparse"
        && let Err(e) = t.parse(src)
    {
        return vec![format!("perr {}", q(e.to_string()))];
    }
    let mut out = Vec::new();
    let res: Result<(), Error> = match mode {
        "plain" => {
            if exec.is_empty() {
                t.execute_with_helper(&(), &ReflectHelper, &mut out, &d.data)
            } else {
                match t.lookup(exec) {
                    Some(tmpl) => tmpl.execute_with_helper(&(), &ReflectHelper, &mut out, &d.data),
                    None => Err(Error::Other(format!(
                        "template: no template {} associated with template {}",
                        q(exec),
                        q(t.name())
                    ))),
                }
            }
        }
        "hugo" => {
            let p = if exec.is_empty() {
                t.clone()
            } else {
                t.lookup(exec).expect("template")
            };
            let helper = HugoHelper::new(d.site.clone(), d.site_params.clone());
            Executer::new(Arc::new(helper)).execute_with_context(&(), &p, &mut out, &d.data)
        }
        _ => panic!("mode {mode}"),
    };
    let mut lines = vec![format!("out {}", q(&out))];
    if let Err(e) = res {
        let is_exec = matches!(e, Error::Exec(_));
        lines.push(format!(
            "err {} {} {}",
            q(e.to_string()),
            is_exec,
            q(e.cause_message())
        ));
    }
    lines
}

#[test]
fn exec_oracle() {
    // Recursive templates reach Go's maxExecDepth: run on a big stack.
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(exec_oracle_body)
        .unwrap()
        .join()
        .unwrap();
}

fn exec_oracle_body() {
    let text = read_fixture("exec.txt.gz");
    let (n, bad) = check_exec_fixture(&text, 40);
    assert!(n > 10000, "fixture has {n} cases");
    assert_eq!(bad, 0, "{bad} of {n} exec cases differ from Go");
}

/// Red-team regressions: minimized cases that once differed from Go
/// (`tools/go-oracle/gotemplate/redteam_fixtures.go`).
#[test]
fn exec_redteam_regressions() {
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(|| {
            let text = read_fixture("redteam_exec.txt.gz");
            let (n, bad) = check_exec_fixture_with(&text, 50, true);
            assert!(n > 0);
            assert_eq!(bad, 0, "{bad} of {n} red-team exec cases differ from Go");
        })
        .unwrap()
        .join()
        .unwrap();
}

/// The large seeded red-team corpus (`rtexec` oracle mode, not checked
/// in): `GOTEMPLATE_RT_EXEC=<file.txt.gz> cargo test --test exec_oracle -- --ignored`.
#[test]
#[ignore]
fn exec_redteam() {
    let Some(paths) = std::env::var_os("GOTEMPLATE_RT_EXEC") else {
        eprintln!("GOTEMPLATE_RT_EXEC not set");
        return;
    };
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(move || {
            let mut total = 0;
            for path in std::env::split_paths(&paths) {
                let text = read_gz_path(&path);
                let (n, bad) = check_exec_fixture_with(&text, 60, true);
                eprintln!("{}: {n} exec cases, {bad} differ", path.display());
                total += bad;
            }
            assert_eq!(total, 0, "{total} red-team exec cases differ from Go");
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Replays an exec fixture; returns (cases, differing cases), printing the
/// first `show` differences.
fn check_exec_fixture(text: &str, show: usize) -> (usize, usize) {
    check_exec_fixture_with(text, show, false)
}

/// [`check_exec_fixture`], with the red-team corpora's extra
/// classification (`redteam_deviation`) when `redteam` is set.
fn check_exec_fixture_with(text: &str, show: usize, redteam: bool) -> (usize, usize) {
    let recs = records(text);
    let mut bad = 0;
    let mut addr = 0;
    let mut deviations: BTreeMap<&'static str, usize> = BTreeMap::new();
    for rec in &recs {
        let mode = fields(rec.lines[0]);
        assert_eq!(mode[0], "mode");
        let src_line = rec.lines[1].strip_prefix("src ").unwrap();
        if rec.lines[2] == "addr" {
            addr += 1;
            continue;
        }
        let src = unquote(src_line);
        let got = run_case(
            mode[1],
            mode[2],
            &unquote_str(mode[3]),
            &unquote_str(mode[4]),
            &src,
        );
        let Some(d) = first_diff(&rec.lines[2..], &got) else {
            continue;
        };
        if let Some(dev) = known_deviation(&src, &rec.lines[2..], &got) {
            // The red-team check is stricter: Go's type must be a static
            // interface type (`known_deviation` erases any type).
            let unverified = redteam
                && dev == "static-interface-type"
                && redteam_deviation(&rec.lines[2..], &got).is_none();
            if !unverified {
                *deviations.entry(dev).or_default() += 1;
                continue;
            }
        } else if redteam && let Some(dev) = redteam_deviation(&rec.lines[2..], &got) {
            *deviations.entry(dev).or_default() += 1;
            if dev == "pointer-derived-number"
                && std::env::var_os("GOTEMPLATE_RT_VERBOSE").is_some()
            {
                eprintln!(
                    "pointer-derived? case {}: src {src_line}\n  {d}",
                    rec.header[0]
                );
            }
            continue;
        }
        bad += 1;
        if bad <= show {
            eprintln!(
                "case {} {} [{} {} {} {}]: src {}\n  {d}",
                rec.header[0], rec.header[1], mode[1], mode[2], mode[3], mode[4], src_line
            );
            if std::env::var_os("GOTEMPLATE_RT_VERBOSE").is_some() {
                eprintln!("  go:   {:?}\n  rust: {:?}", &rec.lines[2..], got);
            }
        }
    }
    eprintln!(
        "{} exec cases, {addr} skipped (Go pointer addresses), known deviations {deviations:?}, {bad} differ",
        recs.len()
    );
    (recs.len(), bad)
}

/// The `err` line's message field, unquoted and lossily decoded.
fn err_message(lines: &[impl AsRef<str>]) -> Option<String> {
    let l = lines.iter().find(|l| l.as_ref().starts_with("err "))?;
    let f = fields(l.as_ref());
    Some(String::from_utf8_lossy(&unquote(f[1])).into_owned())
}

fn out_line(lines: &[impl AsRef<str>]) -> Option<String> {
    lines
        .iter()
        .find(|l| l.as_ref().starts_with("out "))
        .map(|l| l.as_ref().to_string())
}

/// Replaces the type in Go's field-evaluation errors (`can't evaluate
/// field F in type T`, `nil pointer evaluating T.F`) by `?`.
fn erase_field_error_type(msg: &str) -> String {
    let mut out = msg.to_string();
    if let Some(i) = out.find("can't evaluate field ") {
        if let Some(j) = out[i..].find(" in type ") {
            out.truncate(i + j + " in type ".len());
            out.push('?');
        }
    } else if let Some(i) = out.find("nil pointer evaluating ") {
        let rest = &out[i + "nil pointer evaluating ".len()..];
        if let Some(dot) = rest.rfind('.') {
            let field = rest[dot..].to_string();
            out.truncate(i + "nil pointer evaluating ".len());
            out.push('?');
            out.push_str(&field);
        }
    }
    out
}

/// Removes the `error calling X: ` wrapper of an arity error and the
/// `<context>` of the node the error is reported at.
fn norm_arity(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(" at <") {
        out.push_str(&rest[..i]);
        out.push_str(" at <?>");
        rest = &rest[i + 5..];
        match rest.find(">: ") {
            Some(j) => rest = &rest[j + 1..],
            None => break,
        }
    }
    out.push_str(rest);
    while let Some(i) = out.find("error calling ") {
        match out[i..].find(": ") {
            Some(j) if out[i + j + 2..].starts_with("wrong number of args") => {
                out.replace_range(i..i + j + 2, "")
            }
            _ => break,
        }
    }
    out
}

/// Classifies a difference as one of the documented deviations of the
/// value model (see PORTING.md), comparing everything else exactly.
fn known_deviation(src: &[u8], want: &[&str], got: &[String]) -> Option<&'static str> {
    // Output before the error must be identical.
    let (wo, go) = (out_line(want)?, out_line(got)?);
    let (we, ge) = (err_message(want), err_message(got));
    // Complex numbers are not in the value model: the literal is an error.
    if String::from_utf8_lossy(src).contains('i')
        && ge
            .as_deref()
            .is_some_and(|e| e.contains("complex constant"))
    {
        return Some("complex");
    }
    if wo != go {
        // A host arity error printed by `try`.
        if wo.contains("wrong number of args for ")
            && norm_arity(&wo) == norm_arity(&go)
            && we == ge
        {
            return Some("host-arity");
        }
        return None;
    }
    let (we, ge) = (we?, ge?);
    // Error messages are Rust strings: invalid UTF-8 is replaced.
    if we == ge {
        return Some("lossy-utf8-error");
    }
    // Go names the static type of an interface-typed slot (`interface {}`,
    // `error`, ...) where the value model only knows the dynamic type.
    if erase_field_error_type(&we) == erase_field_error_type(&ge) {
        return Some("static-interface-type");
    }
    // The same in the builtins' messages (`cannot index slice/array with
    // type interface {}` for an index read from a `map[string]any`).
    if let Some((a, b)) = we.split_once("type interface {}")
        && ge.len() > a.len() + b.len()
        && ge.starts_with(a)
        && ge[a.len()..].starts_with("type ")
        && ge.ends_with(b)
    {
        return Some("static-interface-type");
    }
    // A typed nil pointer does not know its struct's fields.
    if let (Some(i), Some(j)) = (
        we.find("can't evaluate field "),
        ge.find("nil pointer evaluating "),
    ) && we[..i] == ge[..j]
    {
        return Some("nil-pointer-unknown-field");
    }
    // Hosts check the arity of their functions and methods: the engine
    // reports it as a call error.
    if let Some(i) = we.find("wrong number of args for ")
        && ge.contains("error calling ")
        && ge.ends_with(&we[i..])
    {
        return Some("host-arity");
    }
    // Go checks a host function's or method's arity (from its Go signature)
    // before evaluating the arguments; a host checks it when called, so an
    // error in an argument is reported first (PORTING deviation 6).
    if let Some(i) = we.find("wrong number of args for ") {
        let name = we[i + "wrong number of args for ".len()..]
            .split(':')
            .next()
            .unwrap_or("");
        if HOST_NAMES.contains(&name) {
            return Some("host-arity-order");
        }
    }
    None
}

/// The red-team corpora's extra classification (after `known_deviation`):
/// generated templates hit the same documented deviations anywhere in a
/// line (inside `try` values in the output, several times per line, also
/// URL- or JS-escaped by `urlquery`), so each differing line is classified
/// on its own.
fn redteam_deviation(want: &[&str], got: &[String]) -> Option<&'static str> {
    // Go's `slice` builtin does not unwrap interface-kinded index arguments
    // (a field read from a `map[string]any`): Go fails where the value
    // model, which has no static types, slices (PORTING deviation 15).
    if want
        .iter()
        .any(|w| w.contains("cannot index slice/array with type interface {}"))
    {
        return Some("slice-interface-index");
    }
    if want.len() != got.len() {
        return None;
    }
    let mut class = None;
    for (w, g) in want.iter().zip(got) {
        if *w == g.as_str() {
            continue;
        }
        let (w, g) = (unescape_line(w), unescape_line(g));
        let c = if w.contains("cannot index slice/array with type interface {}") {
            // Go's `slice` builtin does not unwrap interface-kinded index
            // arguments (a field read from a `map[string]any`): an error
            // where the value model sees the int (PORTING deviation 15).
            "slice-interface-index"
        } else if types_equal(&w, &g) {
            // Go names the static type of an interface-typed slot (PORTING
            // deviation 15); a typed nil pointer does not know its struct's
            // fields (deviation 7).
            "static-type-or-nil-pointer"
        } else if g.contains("complex constant ") && g.contains(" is not supported") {
            // Complex numbers (PORTING deviation 1), e.g. inside `try`.
            "complex"
        } else if host_context_equal(&w, &g) {
            "host-arity-context"
        } else if host_signature_line(&w, &g) {
            "host-signature"
        } else if digits_equal(&w, &g) {
            "pointer-derived-number"
        } else {
            return None;
        };
        class.get_or_insert(c);
    }
    class
}

/// A fixture line with its Go quoting and a `urlquery` escaping undone
/// (lossily: the classification only compares texts).
fn unescape_line(s: &str) -> String {
    let f = fields(s);
    let mut out = String::new();
    for (i, x) in f.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if x.starts_with('"') {
            let b = unquote(x);
            let t = String::from_utf8_lossy(&b).into_owned();
            if t.contains("%3A") || t.contains("%3C") {
                out.push_str(&String::from_utf8_lossy(&percent_decode(&b)));
            } else {
                out.push_str(&t);
            }
        } else {
            out.push_str(x);
        }
    }
    out
}

/// `url.QueryUnescape` (`+` is a space).
fn percent_decode(b: &[u8]) -> Vec<u8> {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => match (hex(b[i + 1]), hex(b[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h * 16 + l);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            c => out.push(c),
        }
        i += 1;
    }
    out
}

/// The Go types of the model that error messages name, longest first
/// where one is a prefix of another.
const MODEL_TYPES: &[&str] = &[
    "func(interface {}, interface {}) (interface {}, error)",
    "func(interface {}) interface {}",
    "map[string]interface {}",
    "[]map[string]interface {}",
    "[]interface {}",
    "map[string]string",
    "interface {}",
    "fmt.Stringer",
    "error",
    "*main.EtxT",
    "main.EtxT",
    "*main.EtxV",
    "main.EtxV",
    "main.EtxZ",
    "*main.EtxStr",
    "main.EtxSV",
    "*main.EtxErr",
    "*main.EtxSite",
    "main.EtxPages",
    "main.EtxData",
    "maps.Params",
    "template.TryValue",
    "template.ExecError",
    "*fmt.wrapError",
    "*template.TryError",
    "template.HTML",
    "template.JS",
    "time.Time",
    "*errors.errorString",
    "[]string",
    "[]int",
    "[]bool",
    "[]float64",
    "[]uint8",
    "string",
    "bool",
    "float32",
    "float64",
    "uintptr",
    "uint64",
    "uint32",
    "uint16",
    "uint8",
    "uint",
    "int64",
    "int32",
    "int16",
    "int8",
    "int",
];

/// Go's static interface types (what Go names for an interface-typed slot).
const STATIC_TYPES: &[&str] = &["interface {}", "fmt.Stringer", "error"];

fn model_type_at(s: &str) -> Option<&'static str> {
    MODEL_TYPES
        .iter()
        .filter(|t| s.starts_with(**t))
        .max_by_key(|t| t.len())
        .copied()
}

/// Splits a message text at the types named after `in type `, `with type `
/// and `; got `, rewriting `nil pointer evaluating T.X` as `can't evaluate
/// field X in type T` first.
fn split_types(s: &str) -> (Vec<String>, Vec<&'static str>) {
    // nil pointer evaluating T.X -> can't evaluate field X in type T
    let mut norm = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("nil pointer evaluating ") {
        norm.push_str(&rest[..i]);
        let after = &rest[i + "nil pointer evaluating ".len()..];
        match model_type_at(after) {
            Some(t) if after[t.len()..].starts_with('.') => {
                let fstart = t.len() + 1;
                let flen = after[fstart..]
                    .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .unwrap_or(after.len() - fstart);
                norm.push_str(&format!(
                    "can't evaluate field {} in type {t}",
                    &after[fstart..fstart + flen]
                ));
                rest = &after[fstart + flen..];
            }
            _ => {
                norm.push_str("nil pointer evaluating ");
                rest = after;
            }
        }
    }
    norm.push_str(rest);
    let mut texts = vec![String::new()];
    let mut types = Vec::new();
    let mut rest = norm.as_str();
    loop {
        let next = [" in type ", "with type ", "; got ", "non-function <"]
            .iter()
            .filter_map(|k| rest.find(k).map(|i| (i, k.len())))
            .min();
        let Some((i, klen)) = next else {
            texts.last_mut().unwrap().push_str(rest);
            break;
        };
        texts.last_mut().unwrap().push_str(&rest[..i + klen]);
        rest = &rest[i + klen..];
        if let Some(t) = model_type_at(rest) {
            types.push(t);
            texts.push(String::new());
            rest = &rest[t.len()..];
        }
    }
    (texts, types)
}

/// Equal up to the types in messages, where Go's type is a static
/// interface type (or equal).
fn types_equal(w: &str, g: &str) -> bool {
    types_equal_by(w, g, |a, b| a == b)
}

/// [`types_equal`], comparing the texts between the types with `eq`.
fn types_equal_by(w: &str, g: &str, eq: impl Fn(&str, &str) -> bool) -> bool {
    let (tw, yw) = split_types(w);
    let (tg, yg) = split_types(g);
    tw.len() == tg.len()
        && tw.iter().zip(&tg).all(|(a, b)| eq(a, b))
        && yw.len() == yg.len()
        && yw
            .iter()
            .zip(&yg)
            .all(|(a, b)| a == b || STATIC_TYPES.contains(a))
        && (w != g)
}

/// The first byte where `w` and `g` differ, on a char boundary of both.
fn first_diff_pos(w: &str, g: &str) -> usize {
    let mut p = w.bytes().zip(g.bytes()).take_while(|(a, b)| a == b).count();
    while !w.is_char_boundary(p) || !g.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// A later error reported at a host function's identifier (`at <echo>`)
/// by Go and at its command (`at <echo 1 2>`) or at an argument here: Go's
/// arity check failed (inside `try`) before the arguments were evaluated
/// (PORTING deviation 6).
fn host_context_equal(w: &str, g: &str) -> bool {
    let p = first_diff_pos(w, g);
    // The location of the message holding the first difference.
    let Some(m) = w[..p].rfind("template: ") else {
        return false;
    };
    let Some(i) = w[m..].find(" at <").map(|i| m + i) else {
        return false;
    };
    let Some(j) = w[i..].find(">: ") else {
        return false;
    };
    let name = &w[i + 5..i + j];
    let typed_param = [" at <intarg ", " at <strarg ", " at <.Time.Format "]
        .iter()
        .any(|k| g.get(m..).is_some_and(|x| x.contains(k)));
    (HOST_NAMES.contains(&name) || typed_param) && erase_contexts(w) == erase_contexts(g)
}

/// Replaces each error location (`t:1:2: executing "t" at <node>: `).
fn erase_contexts(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("template: ") {
        let Some(j) = rest[i..].find(" at <") else {
            break;
        };
        let Some(k) = rest[i + j..].find(">: ") else {
            break;
        };
        out.push_str(&rest[..i]);
        out.push_str("template: CTX: ");
        rest = &rest[i + j + k + 3..];
    }
    out.push_str(rest);
    out
}

/// Go's messages for host argument checks (from the host's Go signature,
/// PORTING deviation 6) and Rust's counterparts in the test model.
const GO_SIGNATURE_PHRASES: &[&str] = &[
    "wrong type for value; expected ",
    "invalid value; expected ",
    "expected integer; found ",
    "expected string; found ",
];

/// A line whose first difference lies in a message about a host function's
/// or method's signature (arity or argument type) on either side.
fn host_signature_line(w: &str, g: &str) -> bool {
    let p = first_diff_pos(w, g);
    let msg_start = |s: &str| s[..p].rfind("template: ").unwrap_or(0);
    let (wm, gm) = (&w[msg_start(w)..], &g[msg_start(g)..]);
    let host_arity = |m: &str| {
        m.match_indices("wrong number of args for ").any(|(i, k)| {
            let name = m[i + k.len()..].split(':').next().unwrap_or("");
            HOST_NAMES.contains(&name)
        })
    };
    host_arity(wm)
        || host_arity(gm)
        || GO_SIGNATURE_PHRASES.iter().any(|k| wm.contains(k))
        || [
            "strarg: want",
            "intarg: want",
            "Format: want",
            " not modelled",
        ]
        .iter()
        .any(|k| gm.contains(k))
}

/// Equal once every run of digits (and of 5+ hex digits) is
/// replaced: numbers derived from Go pointer addresses (`len`/`urlquery`/
/// `%b`/`%x` of a value holding pointers).
fn digits_equal(w: &str, g: &str) -> bool {
    fn norm(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = String::new();
        let mut i = 0;
        while i < b.len() {
            let hex_run = b[i..].iter().take_while(|c| c.is_ascii_hexdigit()).count();
            let dec_run = b[i..].iter().take_while(|c| c.is_ascii_digit()).count();
            let n = if hex_run >= 5 { hex_run } else { dec_run };
            if n > 0 {
                // A padded number (`%8d`): its padding depends on its length.
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push('#');
                i += n;
                while i < b.len() && b[i] == b' ' {
                    i += 1;
                }
                continue;
            }
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
        out
    }
    norm(w) == norm(g) || types_equal_by(w, g, |a, b| norm(a) == norm(b))
}

/// The model's host functions and methods (Go signatures the engine does
/// not see).
const HOST_NAMES: &[&str] = &[
    "echo",
    "echo2",
    "fail",
    "failNil",
    "failErr",
    "nilany",
    "nilptr",
    "nilerr",
    "nilstr",
    "nilmap",
    "nilslice",
    "mkhtml",
    "try",
    "typeof",
    "isnil",
    "variadic",
    "panicky",
    "panicerr",
    "errval",
    "dict",
    "list",
    "mkT",
    "getfn",
    "strarg",
    "intarg",
    "Echo",
    "Two",
    "Var",
    "Fail",
    "NilOK",
    "Self",
    "GetSub",
    "RetNil",
    "RetNilPtr",
    "RetNilStr",
    "RetNilMap",
    "RetM",
    "Panic",
    "VM",
    "PM",
    "VV",
    "PV2",
    "Len",
    "First",
    "Reverse",
    "Pages",
    "Count",
    "IsZero",
    "String",
    "Error",
    "MainSections",
    "Format",
    "Year",
    "Unix",
    "UTC",
    "Day",
    "GetNested",
];
