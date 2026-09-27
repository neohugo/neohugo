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
    let recs = records(&text);
    assert!(recs.len() > 10000, "fixture has {} cases", recs.len());
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
            *deviations.entry(dev).or_default() += 1;
            continue;
        }
        bad += 1;
        if bad <= 40 {
            eprintln!(
                "case {} {} [{}]: src {}\n  {d}",
                rec.header[0], rec.header[1], mode[1], src_line
            );
        }
    }
    eprintln!(
        "{} exec cases, {addr} skipped (Go pointer addresses), known deviations {deviations:?}, {bad} differ",
        recs.len()
    );
    assert_eq!(bad, 0, "{bad} of {} exec cases differ from Go", recs.len());
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
    None
}
