//! Execution-level differential tests of html/template (parse → escape →
//! exec) against the Go fork.
//!
//! The fixtures (`tests/fixtures/html/{gotests,corpus}.txt.gz`) are
//! written by `tools/go-oracle/gotemplate/htmlexec*.go`:
//!
//! - `gotests`: the fork's own tests (escape_test.go, content_test.go,
//!   clone_test.go, multi_test.go, exec_test.go, template_test.go) as
//!   scripts of template operations;
//! - `corpus`: small templates covering every escaping context, each
//!   executed with ~70 values of every kind (`corpus/...` scripts, plain
//!   `Execute`), and again through Hugo's execution path (`hugo/...`
//!   scripts: `Executer` with a helper like Hugo's `templateExecHelper`,
//!   so every function, the escapers included, is found through it).
//!
//! Each script is interpreted with the Rust engine and every operation's
//! result (output and error text, parse/clone errors, lookups, template
//! sets) is compared byte for byte with what the fork produced. Cases the
//! value model cannot express are listed in `KNOWN_GAPS` with the reason;
//! they must still differ (so the list shrinks when the gap is closed).
//!
//! Regenerate (repo root):
//!   tools/go-oracle/gotemplate/sync-fork.sh
//!   GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate htmlexec crates/gotemplate/tests/fixtures/html

mod common;
#[path = "common/gotypes.rs"]
mod gotypes;

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use go_value::{GoString, HostCtx, MapType, SafeKind, Value};
use gotemplate::html::Template;
use gotemplate::parse::{self, Mode, SharedTree};
use gotemplate::text::{DefaultHelper, ExecHelper, Executer, Func, FuncMap};

use common::{q, read_gz_fixture, spec_value, unquote};
use gotypes::{want_int, want_string};

// ---------------------------------------------------------------------------
// Function sets (Go: htmlexec.go funcSet)

fn func(f: impl Fn(&[Value]) -> go_value::Result<Value> + Send + Sync + 'static) -> Func {
    Arc::new(move |_ctx: HostCtx<'_>, args: &[Value]| f(args))
}

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn arity(args: &[Value], n: usize) -> go_value::Result<()> {
    if args.len() != n {
        return Err(err(format!(
            "wrong number of args: got {} want {n}",
            args.len()
        )));
    }
    Ok(())
}

fn sprint(args: &[Value]) -> GoString {
    go_fmt::sprint(args).into()
}

/// exec_test.go testExecute's FuncMap.
fn exec_funcs() -> FuncMap {
    let mut m = FuncMap::new();
    m.insert(
        "add".into(),
        func(|args| {
            let mut sum = 0i64;
            for a in args {
                sum += want_int(a)?;
            }
            Ok(Value::int(sum))
        }),
    );
    // Channels are not in the value model.
    m.insert(
        "count".into(),
        func(|_| Err(err("channels are not supported"))),
    );
    m.insert(
        "dddArg".into(),
        func(|args| {
            let Some((a, b)) = args.split_first() else {
                return Err(err("wrong number of args"));
            };
            let a = want_int(a)?;
            let b: Vec<GoString> = b.iter().map(want_string).collect::<go_value::Result<_>>()?;
            Ok(Value::String(
                go_fmt::sprintln(&[Value::int(a), Value::string_list(b)]).into(),
            ))
        }),
    );
    m.insert(
        "echo".into(),
        func(|args| {
            arity(args, 1)?;
            Ok(args[0].clone())
        }),
    );
    m.insert(
        "makemap".into(),
        func(|args| {
            if args.len() % 2 != 0 {
                return Err(err("bad makemap"));
            }
            let mut m = go_value::Map::new(MapType::StringString);
            for kv in args.chunks(2) {
                m.insert(want_string(&kv[0])?, Value::String(want_string(&kv[1])?));
            }
            Ok(Value::map(m))
        }),
    );
    m.insert(
        "mapOfThree".into(),
        func(|args| {
            arity(args, 0)?;
            let mut m = go_value::Map::new(MapType::Named(Arc::from("map[string]int")));
            m.insert("three", Value::int(3));
            Ok(Value::map(m))
        }),
    );
    m.insert(
        "oneArg".into(),
        func(|args| {
            arity(args, 1)?;
            let a = want_string(&args[0])?;
            Ok(Value::string(format!("oneArg={}", a.to_str_lossy())))
        }),
    );
    m.insert(
        "returnInt".into(),
        func(|args| {
            arity(args, 0)?;
            Ok(Value::int(7))
        }),
    );
    m.insert(
        "stringer".into(),
        func(|args| {
            arity(args, 1)?;
            match &args[0] {
                Value::Object(o) => match o.go_string() {
                    Some(s) => Ok(Value::String(s)),
                    None => Err(err("not a fmt.Stringer")),
                },
                _ => Err(err("not a fmt.Stringer")),
            }
        }),
    );
    m.insert(
        "twoArgs".into(),
        func(|args| {
            arity(args, 2)?;
            let a = want_string(&args[0])?;
            let b = want_string(&args[1])?;
            Ok(Value::string(format!(
                "twoArgs={}{}",
                a.to_str_lossy(),
                b.to_str_lossy()
            )))
        }),
    );
    m.insert(
        "typeOf".into(),
        func(|args| {
            arity(args, 1)?;
            Ok(Value::String(
                go_fmt::sprintf("%T", std::slice::from_ref(&args[0])).into(),
            ))
        }),
    );
    m.insert(
        "valueString".into(),
        func(|args| {
            arity(args, 1)?;
            want_string(&args[0])?;
            Ok(Value::string("value is ignored"))
        }),
    );
    m.insert(
        "vfunc".into(),
        func(|args| {
            arity(args, 2)?;
            Ok(Value::string("vfunc"))
        }),
    );
    m.insert(
        "zeroArgs".into(),
        func(|args| {
            arity(args, 0)?;
            Ok(Value::string("zeroArgs"))
        }),
    );
    m
}

fn safe_func(kind: SafeKind) -> Func {
    func(move |args| {
        arity(args, 1)?;
        Ok(Value::Safe(kind, sprint(args)))
    })
}

fn func_set(args: &[String], t: &HashMap<String, Template>) -> FuncMap {
    let mut m = FuncMap::new();
    match args[0].as_str() {
        "exec" => return exec_funcs(),
        "pred" => {
            m.insert(
                "pred".into(),
                func(|a| {
                    if let [Value::Int(i, go_value::IntKind::Int)] = a
                        && *i > 0
                    {
                        return Ok(Value::int(i - 1));
                    }
                    Err(err(format!(
                        "undefined pred({})",
                        String::from_utf8_lossy(&go_fmt::sprintf(
                            "%v",
                            &[Value::any_list(a.to_vec())]
                        ))
                    )))
                }),
            );
        }
        "issue5980" => {
            m.insert("customFunc".into(), func(|_| Err(err("issue5980"))));
        }
        "panic" => {
            // Go recovers the panic and reports it as the call's error.
            m.insert("doPanic".into(), func(|_| Err(err("custom panic string"))));
        }
        "identity" => {
            m.insert(
                "f".into(),
                func(|a| {
                    arity(a, 1)?;
                    Ok(Value::String(want_string(&a[0])?))
                }),
            );
        }
        "recur" => {
            let tmpl = t[&args[1]].clone();
            m.insert(
                "recur".into(),
                func(move |_| {
                    let mut out = Vec::new();
                    tmpl.execute_template(&mut out, "subroutine", &Value::Invalid)
                        .map_err(|e| err(e.to_string()))?;
                    Ok(Value::Safe(SafeKind::Html, out.into()))
                }),
            );
        }
        "corpus" => {
            m.insert("safeHTML".into(), safe_func(SafeKind::Html));
            m.insert("safeHTMLAttr".into(), safe_func(SafeKind::HtmlAttr));
            m.insert("safeCSS".into(), safe_func(SafeKind::Css));
            m.insert("safeJS".into(), safe_func(SafeKind::Js));
            m.insert("safeJSStr".into(), safe_func(SafeKind::JsStr));
            m.insert("safeURL".into(), safe_func(SafeKind::Url));
            m.insert("safeSrcset".into(), safe_func(SafeKind::Srcset));
        }
        other => panic!("unknown func set {other}"),
    }
    m
}

/// Go: `*main.recursiveInvoker` (TestRecursiveExecuteViaMethod).
struct RecursiveInvoker(Template);

impl go_value::Object for RecursiveInvoker {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("*main.recursiveInvoker")
    }
    fn has_method(&self, name: &str) -> bool {
        name == "Recur"
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if name != "Recur" {
            return None;
        }
        let mut out = Vec::new();
        Some(
            self.0
                .execute_template(&mut out, "subroutine", &Value::Invalid)
                .map(|()| Value::String(out.into()))
                .map_err(|e| err(e.to_string())),
        )
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Hugo's `templateExecHelper` (tpl/tplimpl/template_funcs.go, not
/// watching) as `templatestore.go:configureSiteStorage` sets it up: the
/// corpus funcs, then html `GoFuncs`, then text `GoFuncs`; `maps.Params`
/// keys are case-insensitive. Go: htmlexec_corpus.go `hugoHelper`.
struct HugoHelper {
    funcs: FuncMap,
}

impl HugoHelper {
    fn new() -> HugoHelper {
        let mut funcs = func_set(&["corpus".to_string()], &HashMap::new());
        for (k, v) in gotemplate::html::go_funcs()
            .into_iter()
            .chain(gotemplate::text::go_funcs())
        {
            funcs.entry(k.to_string()).or_insert(v);
        }
        HugoHelper { funcs }
    }
}

fn hugo_executer() -> &'static Executer {
    static E: std::sync::OnceLock<Executer> = std::sync::OnceLock::new();
    E.get_or_init(|| Executer::new(Arc::new(HugoHelper::new())))
}

fn is_params(v: &Value) -> Option<&go_value::Map> {
    match v {
        Value::Map(m) if m.ty == MapType::Params => Some(m),
        _ => None,
    }
}

impl ExecHelper for HugoHelper {
    fn get_func(&self, _ctx: HostCtx<'_>, name: &str) -> Option<Func> {
        self.funcs.get(name).cloned()
    }

    // Go: hreflect.GetMethodByName; of the `maps.Params` methods only
    // `IsZero` is used by the corpus.
    fn has_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool {
        match is_params(receiver) {
            Some(_) => name == "IsZero",
            None => DefaultHelper.has_method(ctx, receiver, name),
        }
    }

    fn call_method(
        &self,
        ctx: HostCtx<'_>,
        receiver: &Value,
        name: &str,
        args: &[Value],
    ) -> go_value::Result<Value> {
        match is_params(receiver) {
            // Go: common/maps/params.go:(Params).IsZero
            Some(m) => Ok(Value::Bool(match m.len() {
                0 => true,
                1 => m.entries.keys().all(|k| &k[..] == b"_merge"),
                _ => false,
            })),
            None => DefaultHelper.call_method(ctx, receiver, name, args),
        }
    }

    fn get_map_value(&self, ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value> {
        match is_params(receiver) {
            // Case insensitive.
            Some(m) => m
                .get(&go_unicode::strings::to_lower(key.as_go_string()?))
                .cloned(),
            None => DefaultHelper.get_map_value(ctx, receiver, key),
        }
    }
}

struct ErrorWriter;

impl Write for ErrorWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("always be failing"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Interpreter (Go: htmlexec.go interp.run)

/// An operation's result fields (Go strings are bytes).
type Res = Vec<Vec<u8>>;

fn r(s: impl Into<Vec<u8>>) -> Vec<u8> {
    s.into()
}

fn ok_or_err<T>(res: Result<T, gotemplate::Error>) -> Res {
    match res {
        Ok(_) => vec![r("ok")],
        Err(e) => vec![r("err"), r(e.to_string())],
    }
}

fn exec_result(out: &[u8], res: Result<(), gotemplate::Error>) -> Res {
    vec![
        out.to_vec(),
        match res {
            Ok(()) => Vec::new(),
            Err(e) => r(e.to_string()),
        },
    ]
}

/// The data of an execute operation. Go's data parameter is `any`: a nil
/// of an interface type (e.g. a nil `error`) stored in it is a nil `any`
/// (the invalid `reflect.Value`), not a typed nil.
fn exec_data(spec: &str) -> Value {
    match spec_value(spec) {
        Value::TypedNil(t) if go_value::typed_nil_kind(&t) == go_value::NilKind::Interface => {
            Value::Invalid
        }
        v => v,
    }
}

struct Interp<'a> {
    t: HashMap<String, Template>,
    tr: HashMap<String, SharedTree>,
    data: &'a [String],
}

impl Interp<'_> {
    fn tmpl(&self, name: &str) -> Template {
        self.t
            .get(name)
            .unwrap_or_else(|| panic!("undefined template variable {name}"))
            .clone()
    }

    fn spec(&self, arg: &str) -> String {
        match arg.strip_prefix('@') {
            Some(i) => self.data[i.parse::<usize>().unwrap()].clone(),
            None => arg.to_string(),
        }
    }

    fn data(&self, arg: &str) -> Value {
        exec_data(&self.spec(arg))
    }

    fn run(&mut self, kind: &str, raw: &[Vec<u8>]) -> Res {
        // Names and variables are text; template sources stay bytes.
        let a: Vec<String> = raw
            .iter()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .collect();
        match kind {
            "new" => {
                self.t.insert(a[0].clone(), Template::new(&a[1]));
                vec![]
            }
            "newassoc" => {
                let t = self.tmpl(&a[1]).new_associated(&a[2]);
                self.t.insert(a[0].clone(), t);
                vec![]
            }
            "parse" => ok_or_err(self.tmpl(&a[0]).parse(&raw[1])),
            "clone" | "cloneshallow" => {
                let from = self.tmpl(&a[1]);
                let res = if kind == "clone" {
                    from.clone_ns()
                } else {
                    from.clone_shallow()
                };
                if let Ok(c) = &res {
                    self.t.insert(a[0].clone(), c.clone());
                }
                ok_or_err(res)
            }
            "lookup" => match self.tmpl(&a[1]).lookup(&a[2]) {
                None => vec![r("nil")],
                Some(x) => {
                    self.t.insert(a[0].clone(), x);
                    vec![r("ok")]
                }
            },
            "funcs" => {
                let fm = func_set(&a[1..], &self.t);
                self.tmpl(&a[0]).funcs(&fm);
                vec![]
            }
            "delims" => {
                self.tmpl(&a[0]).delims(&a[1], &a[2]);
                vec![]
            }
            "option" => {
                self.tmpl(&a[0]).option(&[a[1].as_str()]);
                vec![]
            }
            "exec" => {
                let mut out = Vec::new();
                let res = self.tmpl(&a[0]).execute(&mut out, &self.data(&a[1]));
                exec_result(&out, res)
            }
            "exectmpl" => {
                let mut out = Vec::new();
                let res = self
                    .tmpl(&a[0])
                    .execute_template(&mut out, &a[1], &self.data(&a[2]));
                exec_result(&out, res)
            }
            "execerrw" => {
                let res = self
                    .tmpl(&a[0])
                    .execute(&mut ErrorWriter, &self.data(&a[1]));
                exec_result(b"", res)
            }
            "exechugo" => {
                let mut out = Vec::new();
                let res = hugo_executer().execute_with_context(
                    &(),
                    &self.tmpl(&a[0]),
                    &mut out,
                    &self.data(&a[1]),
                );
                exec_result(&out, res)
            }
            "execrecur" => {
                let mut out = Vec::new();
                let d = Value::object(RecursiveInvoker(self.tmpl(&a[1])));
                let res = self.tmpl(&a[0]).execute(&mut out, &d);
                exec_result(&out, res)
            }
            "execpar" => {
                let n: usize = a[2].parse().unwrap();
                let t = self.tmpl(&a[0]);
                let spec = self.spec(&a[1]);
                let results: Vec<Res> = std::thread::scope(|s| {
                    let hs: Vec<_> = (0..n)
                        .map(|_| {
                            let t = t.clone();
                            let spec = spec.clone();
                            s.spawn(move || {
                                let mut out = Vec::new();
                                let res = t.execute(&mut out, &exec_data(&spec));
                                exec_result(&out, res)
                            })
                        })
                        .collect();
                    hs.into_iter().map(|h| h.join().unwrap()).collect()
                });
                if results.iter().any(|x| *x != results[0]) {
                    return vec![r("inconsistent")];
                }
                results[0].clone()
            }
            "templates" => {
                let mut names: Vec<String> = self
                    .tmpl(&a[0])
                    .templates()
                    .iter()
                    .map(|t| t.name())
                    .collect();
                names.sort();
                let mut res = vec![r(names.len().to_string())];
                res.extend(names.into_iter().map(r));
                res
            }
            "name" => vec![r(self.tmpl(&a[0]).name())],
            "ptreq" => vec![r(self.tmpl(&a[0]).ptr_eq(&self.tmpl(&a[1])).to_string())],
            "treesync" => {
                let t = self.tmpl(&a[0]);
                let same = match (t.tree(), t.text().tree()) {
                    (None, None) => true,
                    (Some(x), Some(y)) => x.ptr_eq(&y),
                    _ => false,
                };
                vec![r(same.to_string())]
            }
            "hastree" => vec![r(self.tmpl(&a[0]).tree().is_some().to_string())],
            "parsetree" => {
                let comments = a.get(4).is_some_and(|m| m == "comments");
                let res = if comments {
                    parse::parse_with_mode(&a[1], &raw[2], "", "", Mode::PARSE_COMMENTS, &[])
                } else {
                    parse::parse(&a[1], &raw[2], "", "", &[])
                };
                match res {
                    Ok(mut trees) => {
                        let pick = if comments { &a[1] } else { &a[3] };
                        if let Some(t) = trees.remove(pick) {
                            self.tr.insert(a[0].clone(), SharedTree::new(t));
                        }
                        vec![r("ok")]
                    }
                    Err(e) => vec![r("err"), r(e.to_string())],
                }
            }
            "addtree" => {
                let tree = if let Some(tr) = a[3].strip_prefix("tree:") {
                    self.tr[tr].clone()
                } else if let Some(v) = a[3].strip_prefix("of:") {
                    self.tmpl(v).tree().expect("tree")
                } else {
                    panic!("bad tree ref {}", a[3])
                };
                let res = self.tmpl(&a[1]).add_parse_tree(&a[2], tree);
                if let Ok(x) = &res {
                    self.t.insert(a[0].clone(), x.clone());
                }
                ok_or_err(res)
            }
            other => panic!("unknown op {other}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture replay

struct Op {
    kind: String,
    args: Vec<Vec<u8>>,
    want: Res,
}

struct Script {
    name: String,
    ops: Vec<Op>,
}

fn load(name: &str) -> (Vec<String>, Vec<Script>) {
    let data = read_gz_fixture(name);
    let mut table = Vec::new();
    let mut scripts: Vec<Script> = Vec::new();
    for line in data.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "D" => {
                assert_eq!(f[1].parse::<usize>().unwrap(), table.len());
                table.push(f[2].to_string());
            }
            "S" => scripts.push(Script {
                name: String::from_utf8_lossy(&unquote(f[1])).into_owned(),
                ops: Vec::new(),
            }),
            "O" => {
                let arrow = f.iter().position(|x| *x == "=>").expect("=>");
                scripts.last_mut().unwrap().ops.push(Op {
                    kind: f[1].to_string(),
                    args: f[2..arrow].iter().map(|s| unquote(s)).collect(),
                    want: f[arrow + 1..].iter().map(|s| unquote(s)).collect(),
                });
            }
            "E" => {}
            other => panic!("bad line kind {other:?}"),
        }
    }
    (table, scripts)
}

/// How a known gap is checked.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Gap {
    /// The value model (or a reported engine difference) cannot reproduce
    /// the Go result: matching operations are not compared.
    Model,
    /// Only error texts differ: matching operations compare the output and
    /// whether there is an error.
    ErrText,
    /// Go's error text is not deterministic (map iteration order): like
    /// `ErrText`, but the entry may explain nothing in a given fixture.
    GoNondeterministic,
}

/// Differences with an explanation: (script pattern, data pattern, gap,
/// reason). A pattern is `*`, a prefix ending in `*`, or exact; the data
/// pattern matches the value spec of an execute operation (`*` also
/// matches operations without data). Patterns: see `glob`. Every entry must still explain at
/// least one difference, so the list shrinks as gaps are closed.
const KNOWN_GAPS: &[(&str, &str, Gap, &str)] = &[
    // --- Go behaviour that is not deterministic ---
    (
        "TestClone",
        "*",
        Gap::GoNondeterministic,
        "Go names the first executed template of a map iteration in the Clone error",
    ),
    // --- Value model limitations (see crates/gotemplate/PORTING.md) ---
    (
        "TestExecute/ideal complex",
        "*",
        Gap::Model,
        "complex numbers",
    ),
    ("TestExecute/if 1.5i", "*", Gap::Model, "complex numbers"),
    (
        "TestExecute/printf complex",
        "*",
        Gap::Model,
        "complex numbers",
    ),
    (
        "TestExecute/printf lots",
        "*",
        Gap::Model,
        "complex numbers",
    ),
    ("TestExecute/with 1.5i", "*", Gap::Model, "complex numbers"),
    ("TestExecute/bug16e", "*", Gap::Model, "complex numbers"),
    ("TestExecute/bug16j", "*", Gap::Model, "complex numbers"),
    (
        "TestComparison/eq 1+2i*",
        "*",
        Gap::Model,
        "complex numbers",
    ),
    (
        "TestComparison/ne 1+2i*",
        "*",
        Gap::Model,
        "complex numbers",
    ),
    (
        "TestComparison/lt 1+0i 1+0i",
        "*",
        Gap::Model,
        "complex numbers",
    ),
    (
        "TestExecute/method on nil value from slice",
        "*",
        Gap::Model,
        "a typed nil pointer has no methods (Go calls pointer-receiver methods on nil)",
    ),
    (
        "TestExecute/method on typed nil interface value",
        "*",
        Gap::Model,
        "a typed nil pointer has no methods (Go calls pointer-receiver methods on nil)",
    ),
    (
        "TestExecutePanicDuringCall/direct method call panics",
        "*",
        Gap::Model,
        "a typed nil pointer has no methods (Go calls them and recovers the panic)",
    ),
    (
        "TestExecutePanicDuringCall/indirect method call panics",
        "*",
        Gap::Model,
        "a typed nil pointer has no methods (Go calls them and recovers the panic)",
    ),
    (
        "TestEvalFieldErrors/MissingFieldOnNil",
        "*",
        Gap::ErrText,
        "a typed nil pointer carries no field set (PORTING deviation 7)",
    ),
    (
        "TestExecute/map .WRONG type",
        "*",
        Gap::Model,
        "maps have string keys",
    ),
    (
        "TestExecute/map MI64S",
        "*",
        Gap::Model,
        "maps have string keys",
    ),
    (
        "TestExecute/map MI32S",
        "*",
        Gap::Model,
        "maps have string keys",
    ),
    (
        "TestExecute/map MUI64S",
        "*",
        Gap::Model,
        "maps have string keys",
    ),
    (
        "TestExecute/map MI8S",
        "*",
        Gap::Model,
        "maps have string keys",
    ),
    (
        "TestExecute/map MUI8S",
        "*",
        Gap::Model,
        "maps have string keys",
    ),
    (
        "TestExecute/len(s) < indexes < cap(s)",
        "*",
        Gap::Model,
        "slices have no capacity (PORTING deviation 2)",
    ),
    (
        "TestExecute/indexes > cap(s)",
        "*",
        Gap::ErrText,
        "slices have no capacity (PORTING deviation 2)",
    ),
    ("TestExecute/range count", "*", Gap::Model, "no channels"),
    (
        "TestExecute/range nil count",
        "*",
        Gap::Model,
        "no channels",
    ),
    (
        "TestAddrOfIndex/*",
        "*",
        Gap::Model,
        "an addressable value's pointer-receiver methods (a range element of []V)",
    ),
    (
        "TestExecute/.unexported",
        "*",
        Gap::ErrText,
        "objects have no unexported fields (Go: \"unexported field\" error)",
    ),
    (
        "TestExecute/.NilOKFunc not nil",
        "*",
        Gap::Model,
        "a pointer to a basic type is its pointee (`PI *int` is the int), so `call` rejects it for a `*int` parameter",
    ),
    // --- Host functions check their own arguments (contract C7, PORTING
    //     deviation 6): Go reports `wrong type for value` at the argument,
    //     here it is the function's `error calling f: ...` ---
    (
        "TestExecute/bug8*",
        "*",
        Gap::ErrText,
        "host function argument checks",
    ),
    (
        "TestExecute/bug11",
        "*",
        Gap::ErrText,
        "host function argument checks",
    ),
    (
        "TestExecute/bug15",
        "*",
        Gap::ErrText,
        "host function argument checks",
    ),
    (
        "TestExecute/bug16f",
        "*",
        Gap::ErrText,
        "host function argument checks",
    ),
    (
        "TestExecute/bug16h",
        "*",
        Gap::ErrText,
        "host function argument checks",
    ),
    // --- Named string types (hstring.HTML-like, json.Number) are objects:
    //     fmt.Sprint's spacing rule, len and index treat them as strings in Go
    //     (go-fmt PORTING, known gaps) ---
    (
        "{corpus,hugo}/*",
        "obj_hs:*",
        Gap::Model,
        "named string type",
    ),
    ("corpus/*", "jnum:*", Gap::Model, "named string type"),
    // --- src/text differences (reported to its owner) ---
    (
        "{corpus,hugo}/*range*",
        "str:00ff",
        Gap::ErrText,
        "src/text: error texts are Strings (invalid UTF-8 in `range can't iterate over %v` becomes U+FFFD)",
    ),
];

/// `*` matches any run of characters; a leading `{a,b}` matches either
/// alternative (`{corpus,hugo}/...` names both corpus script families).
fn glob(pat: &str, s: &str) -> bool {
    if let Some(rest) = pat.strip_prefix('{') {
        let (alts, tail) = rest.split_once('}').expect("unterminated {");
        return alts.split(',').any(|a| glob(&format!("{a}{tail}"), s));
    }
    wildcard(pat.as_bytes(), s.as_bytes())
}

fn wildcard(p: &[u8], s: &[u8]) -> bool {
    match p.split_first() {
        None => s.is_empty(),
        Some((b'*', rest)) => (0..=s.len()).any(|i| wildcard(rest, &s[i..])),
        Some((c, rest)) => s.first() == Some(c) && wildcard(rest, &s[1..]),
    }
}

struct Outcome {
    checked: usize,
    failures: Vec<String>,
    explained: Vec<String>,
}

fn truncate(s: &str) -> String {
    if s.len() > 120 {
        let mut end = 120;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    } else {
        s.to_string()
    }
}

/// The value spec of an operation's data argument, if it has one.
fn data_spec<'a>(o: &Op, table: &'a [String]) -> Option<std::borrow::Cow<'a, str>> {
    let i = match o.kind.as_str() {
        "exec" | "execerrw" | "execpar" | "exechugo" => 1,
        "exectmpl" => 2,
        _ => return None,
    };
    let a = String::from_utf8_lossy(&o.args[i]).into_owned();
    Some(match a.strip_prefix('@') {
        Some(i) => std::borrow::Cow::Borrowed(table[i.parse::<usize>().unwrap()].as_str()),
        None => std::borrow::Cow::Owned(a),
    })
}

fn run_scripts(fixture: &str) -> Outcome {
    gotypes::register_named_methods();
    common::set_extra_nodes(gotypes::extra_nodes);
    let (table, scripts) = load(fixture);
    let mut out = Outcome {
        checked: 0,
        failures: Vec::new(),
        explained: Vec::new(),
    };
    // Differences explained by each KNOWN_GAPS entry.
    let mut used = vec![0usize; KNOWN_GAPS.len()];
    // HTML_EXEC_VERBOSE=1 also prints what each explained difference is.
    let verbose = std::env::var_os("HTML_EXEC_VERBOSE").is_some();
    for s in &scripts {
        // Values the Rust model cannot build (complex numbers).
        if s.ops
            .iter()
            .any(|o| data_spec(o, &table).is_some_and(|d| gotypes::unsupported_spec(&d)))
        {
            out.explained.push(format!("{} (complex value)", s.name));
            continue;
        }
        let mut interp = Interp {
            t: HashMap::new(),
            tr: HashMap::new(),
            data: &table,
        };
        let mut diffs = Vec::new();
        for (i, o) in s.ops.iter().enumerate() {
            let got = interp.run(&o.kind, &o.args);
            out.checked += 1;
            if got == o.want {
                continue;
            }
            let spec = data_spec(o, &table);
            let gap = KNOWN_GAPS.iter().position(|(sp, dp, _, _)| {
                glob(sp, &s.name)
                    && match &spec {
                        Some(d) => glob(dp, d),
                        None => *dp == "*",
                    }
            });
            let explained = match gap {
                Some(g) => match KNOWN_GAPS[g].2 {
                    Gap::Model => true,
                    Gap::ErrText | Gap::GoNondeterministic => {
                        o.want.len() == 2
                            && got.len() == 2
                            && got[0] == o.want[0]
                            && got[1].is_empty() == o.want[1].is_empty()
                    }
                },
                None => false,
            };
            let detail = format!(
                "  op {i} {} {:?}\n    want {}\n    got  {}",
                o.kind,
                o.args
                    .iter()
                    .map(|a| truncate(&String::from_utf8_lossy(a)))
                    .collect::<Vec<_>>(),
                o.want.iter().map(|w| q(w)).collect::<Vec<_>>().join(" "),
                got.iter().map(|w| q(w)).collect::<Vec<_>>().join(" "),
            );
            if explained {
                let g = gap.unwrap();
                used[g] += 1;
                let mut line = format!("{} op {i} ({})", s.name, KNOWN_GAPS[g].3);
                if verbose {
                    line = format!("{line}\n{detail}");
                }
                out.explained.push(line);
                continue;
            }
            diffs.push(detail);
        }
        if !diffs.is_empty() {
            out.failures
                .push(format!("{}:\n{}", s.name, diffs.join("\n")));
        }
    }
    // Entries that explain nothing in this fixture are checked by the other
    // test (both fixtures share the list); report them only when neither
    // uses them (see `stale_gaps`).
    STALE.with(|st| {
        let mut st = st.borrow_mut();
        if st.is_empty() {
            *st = vec![0; KNOWN_GAPS.len()];
        }
        for (i, n) in used.iter().enumerate() {
            st[i] += n;
        }
    });
    out
}

thread_local! {
    static STALE: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn report(what: &str, o: Outcome) {
    eprintln!(
        "{what}: {} operations checked, {} differences explained by KNOWN_GAPS",
        o.checked,
        o.explained.len()
    );
    for s in &o.explained {
        eprintln!("  explained: {s}");
    }
    if !o.failures.is_empty() {
        let n = o.failures.len();
        let shown: Vec<_> = o.failures.into_iter().take(60).collect();
        panic!(
            "{what}: {n} scripts differ from Go; first {}:\n{}",
            shown.len(),
            shown.join("\n")
        );
    }
}

/// Both fixtures in one test so that stale KNOWN_GAPS entries are detected.
#[test]
fn html_exec_go_tests_and_corpus() {
    let go = run_scripts("html/gotests.txt.gz");
    let corpus = run_scripts("html/corpus.txt.gz");
    report("gotests", go);
    report("corpus", corpus);
    let stale: Vec<&str> = STALE.with(|st| {
        st.borrow()
            .iter()
            .enumerate()
            .filter(|(i, n)| **n == 0 && KNOWN_GAPS[*i].2 != Gap::GoNondeterministic)
            .map(|(i, _)| KNOWN_GAPS[i].0)
            .collect()
    });
    assert!(
        stale.is_empty(),
        "KNOWN_GAPS entries that explain no difference any more (remove them): {stale:?}"
    );
}
