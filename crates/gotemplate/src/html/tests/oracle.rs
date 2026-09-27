//! Replays the fixtures written by the Go oracle
//! (`tools/go-oracle/gotemplate/escfuncs.go`, mode `escfuncs`) against the
//! Rust leaf functions. Every comparison is byte-exact.

use go_value::Value;

use super::super::attr::attr_type;
use super::super::context::{Attr, Context, Delim, Element, IntSlice, JsCtx, State, UrlPart};
use super::super::css::{decode_css, ends_with_css_keyword};
use super::super::html::strip_tags;
use super::super::js::{
    contains_special_script_tag, escape_special_script_tags, is_js_type, next_js_ctx,
    replace_script_tag_re,
};
use super::super::transition::{index_tag_end, transition};
use super::super::url::is_safe_url;
use super::super::{ESC_FUNC_NAMES, esc_func};
use super::common::{q, read_gz_fixture, spec_args, unquote};

/// Collects mismatches and fails with the first few.
struct Failures {
    what: &'static str,
    list: Vec<String>,
    checked: usize,
}

impl Failures {
    fn new(what: &'static str) -> Self {
        Failures {
            what,
            list: Vec::new(),
            checked: 0,
        }
    }
    fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        self.checked += 1;
        if !ok {
            self.list.push(msg());
        }
    }
    fn finish(self) {
        if !self.list.is_empty() {
            let n = self.list.len();
            let shown: Vec<_> = self.list.into_iter().take(30).collect();
            panic!(
                "{}: {} of {} checks failed; first {}:\n{}",
                self.what,
                n,
                self.checked,
                shown.len(),
                shown.join("\n")
            );
        }
        eprintln!("{}: {} checks passed", self.what, self.checked);
    }
}

/// Oracle cases that fail because of a gap in another crate, as
/// `(escaper, args spec, reason)`. They must still fail (so the entry is
/// removed once the other crate is fixed).
const KNOWN_EXTERNAL_GAPS: &[(&str, &str, &str)] = &[];

#[test]
fn oracle_escfuncs() {
    let data = read_gz_fixture("html/escfuncs.txt.gz");
    let mut f = Failures::new("escfuncs");
    let mut args: Vec<Value> = Vec::new();
    let mut spec = "";
    let mut skipped = 0;
    let mut inputs = 0;
    let mut known = 0;
    for line in data.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.splitn(3, '\t').collect();
        match fields[0] {
            "I" => {
                spec = fields[1];
                args = spec_args(spec);
                inputs += 1;
            }
            "O" => {
                let idx: usize = fields[1].parse().unwrap();
                let name = ESC_FUNC_NAMES[idx];
                let res = fields[2];
                let Some(want) = res.strip_prefix('=') else {
                    // P (Go panicked) or N (pointer addresses): not comparable.
                    skipped += 1;
                    continue;
                };
                let want = unquote(want);
                let func = esc_func(name).unwrap();
                let got = func(&args);
                if let Some((_, _, why)) = KNOWN_EXTERNAL_GAPS
                    .iter()
                    .find(|(n, s, _)| *n == name && *s == spec)
                {
                    known += 1;
                    assert_ne!(
                        got, want,
                        "{name}({spec}) passes now: remove the known gap ({why})"
                    );
                    continue;
                }
                f.check(got == want, || {
                    format!("{name}({spec}):\n  want {}\n  got  {}", q(&want), q(&got))
                });
            }
            other => panic!("bad line kind {other:?}"),
        }
    }
    assert!(inputs > 2000, "only {inputs} inputs");
    assert_eq!(
        skipped, 0,
        "the corpus has no panicking or nondeterministic calls"
    );
    assert_eq!(
        known,
        KNOWN_EXTERNAL_GAPS.len(),
        "every known gap is in the corpus"
    );
    f.finish();
}

fn parse_ints(s: &str) -> Vec<i64> {
    let s = s.trim_start_matches('[').trim_end_matches(']');
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',').map(|x| x.parse().unwrap()).collect()
}

fn ints_string(xs: &[i64]) -> String {
    let parts: Vec<String> = xs.iter().map(|x| x.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn ctx_from_fields(fields: &str) -> Context {
    let v: Vec<u8> = fields.split(',').map(|x| x.parse().unwrap()).collect();
    Context {
        state: State::from_u8(v[0]).unwrap(),
        delim: Delim::from_u8(v[1]).unwrap(),
        url_part: UrlPart::from_u8(v[2]).unwrap(),
        js_ctx: JsCtx::from_u8(v[3]).unwrap(),
        attr: Attr::from_u8(v[4]).unwrap(),
        element: Element::from_u8(v[5]).unwrap(),
        ..Context::default()
    }
}

fn err_string(c: &Context) -> String {
    c.err.as_ref().map(|e| e.to_string()).unwrap_or_default()
}

/// Go: `sameOracleCtx`.
fn same_ctx(a: &Context, b: &Context) -> bool {
    a.state == b.state
        && a.delim == b.delim
        && a.url_part == b.url_part
        && a.js_ctx == b.js_ctx
        && a.attr == b.attr
        && a.element == b.element
        && err_string(a) == err_string(b)
        && a.js_brace_depth.to_vec() == b.js_brace_depth.to_vec()
}

/// Go: `stepString`.
fn step_string(res: &Context, n: usize, initial: &IntSlice) -> String {
    let errq = match &res.err {
        Some(e) => go_strconv::quote(e.to_string().as_bytes()),
        None => String::new(),
    };
    format!(
        "{}|{},{},{},{},{},{}|{}|{}|{}|{}",
        n,
        res.state as u8,
        res.delim as u8,
        res.url_part as u8,
        res.js_ctx as u8,
        res.attr as u8,
        res.element as u8,
        ints_string(&res.js_brace_depth.to_vec()),
        res.js_brace_depth.cap(),
        ints_string(&initial.to_vec()),
        errq
    )
}

#[test]
fn oracle_transitions() {
    let data = read_gz_fixture("html/transitions.txt.gz");
    let mut texts: Vec<Vec<u8>> = Vec::new();
    let mut ctxs: Vec<(Context, Option<Vec<i64>>)> = Vec::new();
    let mut f = Failures::new("transitions");
    let mut steps_checked = 0usize;
    for line in data.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields[0] {
            "T" => texts.push(unquote(fields[1])),
            "C" => {
                let braces = if fields[2] == "-" {
                    None
                } else {
                    Some(parse_ints(fields[2]))
                };
                ctxs.push((ctx_from_fields(fields[1]), braces));
            }
            "R" => {
                let ti: usize = fields[1].parse().unwrap();
                let ci: usize = fields[2].parse().unwrap();
                let want: Vec<&str> = fields[3..].to_vec();
                let (start, braces) = &ctxs[ci];
                let initial = match braces {
                    Some(b) => IntSlice::from_vec(b.clone()),
                    None => IntSlice::default(),
                };
                let mut c = start.clone();
                c.js_brace_depth = initial.clone();
                let text = &texts[ti];
                let mut s: &[u8] = text;
                let mut got: Vec<String> = Vec::new();
                while !s.is_empty() {
                    if got.len() >= 2000 {
                        got.push("LIMIT".to_string());
                        break;
                    }
                    let (res, n) = transition(c.clone(), s);
                    got.push(step_string(&res, n, &initial));
                    if n == 0 && same_ctx(&c, &res) {
                        got.push("STALL".to_string());
                        break;
                    }
                    c = res;
                    s = &s[n..];
                }
                steps_checked += got.len();
                f.check(got == want, || {
                    let i = got
                        .iter()
                        .zip(&want)
                        .position(|(a, b)| a != b)
                        .unwrap_or(got.len().min(want.len()));
                    format!(
                        "text {} from ctx {} {:?}: step {}:\n  want {:?}\n  got  {:?}",
                        q(text),
                        ci,
                        braces,
                        i,
                        want.get(i),
                        got.get(i)
                    )
                });
            }
            other => panic!("bad line kind {other:?}"),
        }
    }
    assert!(steps_checked > 10000, "only {steps_checked} steps");
    eprintln!("transitions: {steps_checked} steps");
    f.finish();
}

fn b01(b: bool) -> &'static str {
    if b { "1" } else { "0" }
}

#[test]
fn oracle_leaf() {
    let data = read_gz_fixture("html/leaf.txt.gz");
    let mut f = Failures::new("leaf");
    let mut kinds = std::collections::BTreeMap::<String, usize>::new();
    for line in data.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields[0], "L");
        let kind = fields[1];
        *kinds.entry(kind.to_string()).or_default() += 1;
        match kind {
            "striptags" => {
                let input = unquote(fields[2]);
                let want = unquote(fields[3]);
                let got = strip_tags(&input);
                f.check(got == want, || {
                    format!(
                        "stripTags({}): want {} got {}",
                        q(&input),
                        q(&want),
                        q(&got)
                    )
                });
            }
            "nextjs" => {
                let input = unquote(fields[2]);
                let p: u8 = fields[3].parse().unwrap();
                let want: u8 = fields[4].parse().unwrap();
                let got = next_js_ctx(&input, JsCtx::from_u8(p).unwrap()) as u8;
                f.check(got == want, || {
                    format!("nextJSCtx({}, {p}): want {want} got {got}", q(&input))
                });
            }
            "attrtype" => {
                let input = unquote(fields[2]);
                let want: u8 = fields[3].parse().unwrap();
                let got = attr_type(&input) as u8;
                f.check(got == want, || {
                    format!("attrType({}): want {want} got {got}", q(&input))
                });
            }
            "isjstype" => {
                let input = unquote(fields[2]);
                let got = b01(is_js_type(&input));
                f.check(got == fields[3], || {
                    format!("isJSType({}): want {} got {got}", q(&input), fields[3])
                });
            }
            "issafeurl" => {
                let input = unquote(fields[2]);
                let got = b01(is_safe_url(&input));
                f.check(got == fields[3], || {
                    format!("isSafeURL({}): want {} got {got}", q(&input), fields[3])
                });
            }
            "decodecss" => {
                let input = unquote(fields[2]);
                let want = unquote(fields[3]);
                let got = decode_css(&input);
                f.check(got == want, || {
                    format!(
                        "decodeCSS({}): want {} got {}",
                        q(&input),
                        q(&want),
                        q(&got)
                    )
                });
            }
            "specialtag" => {
                let input = unquote(fields[2]);
                let got = b01(contains_special_script_tag(&input));
                f.check(got == fields[3], || {
                    format!(
                        "containsSpecialScriptTag({}): want {} got {got}",
                        q(&input),
                        fields[3]
                    )
                });
                let want = unquote(fields[4]);
                let got = escape_special_script_tags(&input);
                f.check(got == want, || {
                    format!(
                        "escapeSpecialScriptTags({}): want {} got {}",
                        q(&input),
                        q(&want),
                        q(&got)
                    )
                });
            }
            "scripttagre" => {
                let input = unquote(fields[2]);
                let want = unquote(fields[3]);
                let got = replace_script_tag_re(&input);
                f.check(got == want, || {
                    format!(
                        "scriptTagRe({}): want {} got {}",
                        q(&input),
                        q(&want),
                        q(&got)
                    )
                });
            }
            "indextagend" => {
                let input = unquote(fields[2]);
                let tag = unquote(fields[3]);
                let want: isize = fields[4].parse().unwrap();
                let got = index_tag_end(&input, &tag);
                f.check(got == want, || {
                    format!(
                        "indexTagEnd({}, {}): want {want} got {got}",
                        q(&input),
                        q(&tag)
                    )
                });
            }
            "endswithcss" => {
                let input = unquote(fields[2]);
                let kw = unquote(fields[3]);
                let got = b01(ends_with_css_keyword(&input, &kw));
                f.check(got == fields[4], || {
                    format!(
                        "endsWithCSSKeyword({}, {}): want {} got {got}",
                        q(&input),
                        q(&kw),
                        fields[4]
                    )
                });
            }
            "ctx" => {
                let c = ctx_from_fields(fields[2]);
                let want_s = unquote(fields[3]);
                let want_m = unquote(fields[4]);
                f.check(c.string().as_bytes() == want_s, || {
                    format!(
                        "String({}): want {} got {}",
                        fields[2],
                        q(&want_s),
                        c.string()
                    )
                });
                f.check(c.mangle("name").as_bytes() == want_m, || {
                    format!(
                        "mangle({}): want {} got {}",
                        fields[2],
                        q(&want_m),
                        c.mangle("name")
                    )
                });
            }
            "funcmap" => {
                let mut names: Vec<&str> = ESC_FUNC_NAMES.to_vec();
                names.sort();
                f.check(names.join(",") == fields[2], || {
                    format!("funcMap keys: want {} got {}", fields[2], names.join(","))
                });
            }
            other => panic!("unknown leaf kind {other:?}"),
        }
    }
    eprintln!("leaf kinds: {kinds:?}");
    assert!(kinds.len() >= 12, "{kinds:?}");
    f.finish();
}
