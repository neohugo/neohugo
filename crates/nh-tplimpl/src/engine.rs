//! Module `engine`.
//!
//! NEW: facade over the gotemplate crate (text/html template namespaces, parse trees, exec, escaper)
//!
//! Owner: Wave B task T13 (tplimpl).

//! The single adaptation point between the Hugo template store and the Wave A `gotemplate` crate
//! (Go text/template + html/template as forked in tpl/internal/go_templates, go1.24 semantics).
//!
//! CONTRACT: `crates/GOTEMPLATE_CONTRACT.md` is the pinned host contract between gotemplate and
//! the Hugo layer (it extends template-engine spec §16). The `ExecHelper` trait (clause C1) is
//! gotemplate's own, re-exported here; the template types are thin wrappers over gotemplate's
//! handles (a clone of a wrapper is the same Go pointer). Nothing outside nh-tplimpl (except
//! nh-tpl's `strip_html` and nh-i18n's message templates) touches gotemplate.
//!
//! Semantics that live INSIDE the engine — see the contract: truthiness through
//! `ExecHelper::is_true` (C5); methods before fields and map keys for every receiver kind (C2);
//! `and`/`or` return the deciding operand (C6); Go `evalArg`/`validateType`/ideal-constant rules
//! and nil conversions (C7); `printValue` `<no value>` vs `<nil>` (C8); html escapers unwrap
//! `Object::printable_value` (C9); the `mainsections` pointer-identity special case (C10,
//! implemented by the helper, requires the engine to keep `Arc` identity); one host context
//! passed unchanged to every call, and re-entrant nested execution (C4).

use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_common::Result;
use nh_common::herrors::Error;

/// Go: `texttemplate.ExecHelper` (texttemplate/hugo_template.go:45-51) as implemented by
/// `tplimpl.templateExecHelper` (template_funcs.go), plus Hugo's `isTrue`. Contract clause C1:
/// gotemplate's trait, with the contract's signatures. The engine calls these for every function
/// lookup, method call, map lookup and truth test, with the SAME host context it was given (C4).
/// No method may assume it is the only execution in flight: helpers are called re-entrantly
/// from nested executions.
pub use gotemplate::text::ExecHelper;

/// A template function (Go: any func in the FuncMap). Receives the template context as `HostCtx`
/// (Go injects `context.Context` when the first param is one; Rust always passes it). Arguments
/// arrive converted per contract C7 (nil interfaces -> `Invalid`, ideal constants -> int/float64).
pub type TplFunc = gotemplate::text::Func;

/// Go: `map[string]any` func map (Hugo namespaces + aliases). Built by nh-tplfuncs `tplimplinit`.
pub type FuncMap = BTreeMap<String, TplFunc>;

/// The parse-tree API of the engine (Go `texttemplate/parse`), used by `templatetransform` and
/// the tree dumps: nodes, `SharedTree` (Go `*parse.Tree`), `is_empty_tree`.
pub mod parse {
    pub use gotemplate::parse::{
        ActionNode, BranchNode, ChainNode, CommandNode, FieldNode, IdentifierNode, ListNode, Node,
        NodeLike, NodeType, PipeNode, SharedTree, StringNode, TemplateNode, Tree, VariableNode,
        is_empty_tree,
    };
}

/// Converts an engine error (parse, escape, exec) to the Hugo error type, by its Go text.
pub(crate) fn engine_err(e: gotemplate::Error) -> Error {
    Error::new(e.message())
}

/// A text/template template (Go `*texttemplate.Template`, a pointer: clones are the same one).
#[derive(Clone)]
pub struct TextTemplate {
    pub(crate) inner: gotemplate::text::Template,
}

/// An html/template template (Go `*htmltemplate.Template`, a pointer: clones are the same one).
#[derive(Clone)]
pub struct HtmlTemplate {
    pub(crate) inner: gotemplate::html::Template,
}

impl TextTemplate {
    // Go: texttemplate.New
    pub fn new(name: &str) -> TextTemplate {
        TextTemplate {
            inner: gotemplate::text::Template::new(name),
        }
    }

    pub fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Go: `t.New(name)`: a new template in the same namespace.
    pub fn new_associated(&self, name: &str) -> TextTemplate {
        TextTemplate {
            inner: self.inner.new_associated(name),
        }
    }

    /// Go: `Funcs(funcs)` with a func map used for name validation only (the exec helper
    /// resolves the functions).
    pub fn func_names<I, S>(&self, names: I) -> &TextTemplate
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.inner.func_names(names);
        self
    }

    // Go: (*texttemplate.Template).Parse
    pub fn parse(&self, src: &[u8]) -> Result<TextTemplate> {
        self.inner
            .parse(src)
            .map(|t| TextTemplate { inner: t })
            .map_err(engine_err)
    }

    // Go: (*texttemplate.Template).Lookup
    pub fn lookup(&self, name: &str) -> Option<TextTemplate> {
        self.inner.lookup(name).map(|t| TextTemplate { inner: t })
    }

    // Go: (*texttemplate.Template).Clone
    pub fn clone_ns(&self) -> Result<TextTemplate> {
        self.inner
            .clone_ns()
            .map(|t| TextTemplate { inner: t })
            .map_err(engine_err)
    }

    // Go: (*texttemplate.Template).AddParseTree
    pub fn add_parse_tree(&self, name: &str, tree: parse::SharedTree) -> Result<TextTemplate> {
        self.inner
            .add_parse_tree(name, tree)
            .map(|t| TextTemplate { inner: t })
            .map_err(engine_err)
    }

    /// Go: `All()` / `Templates()` — every template of the namespace, sorted by name (Go: map
    /// order).
    pub fn all(&self) -> Vec<TextTemplate> {
        let mut v: Vec<TextTemplate> = self
            .inner
            .all()
            .into_iter()
            .map(|t| TextTemplate { inner: t })
            .collect();
        v.sort_by_key(|a| a.name());
        v
    }

    /// Go: `t.Tree`.
    pub fn tree(&self) -> Option<parse::SharedTree> {
        self.inner.tree()
    }

    /// Go pointer equality.
    pub fn ptr_eq(&self, other: &TextTemplate) -> bool {
        self.inner.ptr_eq(&other.inner)
    }
}

impl HtmlTemplate {
    // Go: htmltemplate.New
    pub fn new(name: &str) -> HtmlTemplate {
        HtmlTemplate {
            inner: gotemplate::html::Template::new(name),
        }
    }

    pub fn name(&self) -> String {
        self.inner.name()
    }

    /// Go: `t.New(name)`: a new template in the same namespace.
    pub fn new_associated(&self, name: &str) -> HtmlTemplate {
        HtmlTemplate {
            inner: self.inner.new_associated(name),
        }
    }

    /// Go: `Funcs(funcs)` with a func map used for name validation only.
    pub fn func_names<I, S>(&self, names: I) -> &HtmlTemplate
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.inner.func_names(names);
        self
    }

    // Go: (*htmltemplate.Template).Parse
    pub fn parse(&self, src: &[u8]) -> Result<HtmlTemplate> {
        self.inner
            .parse(src)
            .map(|t| HtmlTemplate { inner: t })
            .map_err(engine_err)
    }

    // Go: (*htmltemplate.Template).Lookup
    pub fn lookup(&self, name: &str) -> Option<HtmlTemplate> {
        self.inner.lookup(name).map(|t| HtmlTemplate { inner: t })
    }

    // Go: (*htmltemplate.Template).Clone
    pub fn clone_ns(&self) -> Result<HtmlTemplate> {
        self.inner
            .clone_ns()
            .map(|t| HtmlTemplate { inner: t })
            .map_err(engine_err)
    }

    // Go: (*htmltemplate.Template).CloneShallow (Hugo)
    pub fn clone_shallow(&self) -> Result<HtmlTemplate> {
        self.inner
            .clone_shallow()
            .map(|t| HtmlTemplate { inner: t })
            .map_err(engine_err)
    }

    // Go: (*htmltemplate.Template).AddParseTree
    pub fn add_parse_tree(&self, name: &str, tree: parse::SharedTree) -> Result<HtmlTemplate> {
        self.inner
            .add_parse_tree(name, tree)
            .map(|t| HtmlTemplate { inner: t })
            .map_err(engine_err)
    }

    /// Go: `All()` — the html templates of the namespace, sorted by name (Go: map order).
    pub fn all(&self) -> Vec<HtmlTemplate> {
        let mut v: Vec<HtmlTemplate> = self
            .inner
            .all()
            .into_iter()
            .map(|t| HtmlTemplate { inner: t })
            .collect();
        v.sort_by_key(|a| a.name());
        v
    }

    /// Go: `t.Tree` (the html template's view; nil after an escaping error).
    pub fn tree(&self) -> Option<parse::SharedTree> {
        self.inner.tree()
    }

    /// The underlying text/template (Go `t.text`), e.g. to dump every tree of the namespace
    /// including the derived `name$htmltemplate_*` templates.
    pub fn text(&self) -> TextTemplate {
        TextTemplate {
            inner: self.inner.text(),
        }
    }

    // Go: (*htmltemplate.Template).Prepare (Hugo)
    pub fn prepare(&self) -> Result<TextTemplate> {
        self.inner
            .prepare()
            .map(|t| TextTemplate { inner: t })
            .map_err(engine_err)
    }

    /// Go pointer equality.
    pub fn ptr_eq(&self, other: &HtmlTemplate) -> bool {
        self.inner.ptr_eq(&other.inner)
    }
}

/// Go: `tpl.Template` — either engine.
#[derive(Clone)]
pub enum Template {
    Text(TextTemplate),
    Html(HtmlTemplate),
}

impl Template {
    pub fn name(&self) -> String {
        match self {
            Template::Text(t) => t.name(),
            Template::Html(t) => t.name(),
        }
    }

    /// Go: `Prepare()` — html: escape (once) and return the rewritten text template.
    pub fn prepare(&self) -> Result<TextTemplate> {
        match self {
            Template::Text(t) => Ok(t.clone()),
            Template::Html(t) => t.prepare(),
        }
    }

    /// Go: `getParseTree(templ)` — the text template's tree, or the html template's.
    pub fn tree(&self) -> Option<parse::SharedTree> {
        match self {
            Template::Text(t) => t.tree(),
            Template::Html(t) => t.tree(),
        }
    }

    /// Go: `isText(t)`.
    pub fn is_text(&self) -> bool {
        matches!(self, Template::Text(_))
    }

    /// Go: `Lookup(name)` in the template's own namespace (of the same engine).
    pub fn lookup(&self, name: &str) -> Option<Template> {
        match self {
            Template::Text(t) => t.lookup(name).map(Template::Text),
            Template::Html(t) => t.lookup(name).map(Template::Html),
        }
    }

    /// Go pointer equality.
    pub fn ptr_eq(&self, other: &Template) -> bool {
        match (self, other) {
            (Template::Text(a), Template::Text(b)) => a.ptr_eq(b),
            (Template::Html(a), Template::Html(b)) => a.ptr_eq(b),
            _ => false,
        }
    }
}

/// Go: `texttemplate.Executer` — executes a prepared template with a helper. MUST be re-entrant
/// (contract C4): a func or method called during an execution may execute other templates (or
/// the same one) on the same thread before returning — partials, render hooks through
/// `.Content`, `resources.ExecuteAsTemplate`, `markdownify`. Templates are immutable and shared
/// after `prepare`; all execution state is per call; no lock is held while calling out.
pub trait Executer: Send + Sync {
    fn execute_with_context(
        &self,
        ctx: HostCtx<'_>,
        t: &Template,
        w: &mut Vec<u8>,
        data: &Value,
    ) -> Result<()>;
}

/// The gotemplate executer (Go `texttemplate.NewExecuter(helper)`).
pub struct GoExecuter {
    inner: gotemplate::text::Executer,
}

impl GoExecuter {
    // Go: texttemplate.NewExecuter
    pub fn new(helper: Arc<dyn ExecHelper>) -> GoExecuter {
        GoExecuter {
            inner: gotemplate::text::Executer::new(helper),
        }
    }
}

impl Executer for GoExecuter {
    // Go: texttemplate/hugo_template.go:(*executer).ExecuteWithContext
    fn execute_with_context(
        &self,
        ctx: HostCtx<'_>,
        t: &Template,
        w: &mut Vec<u8>,
        data: &Value,
    ) -> Result<()> {
        match t {
            Template::Text(t) => self.inner.execute_with_context(ctx, &t.inner, w, data),
            Template::Html(t) => self.inner.execute_with_context(ctx, &t.inner, w, data),
        }
        .map_err(engine_err)
    }
}

/// Go: `htmltemplate.StripTags` (re-exported for nh-tpl).
pub fn strip_tags(s: &[u8]) -> Vec<u8> {
    gotemplate::html::strip_tags(s)
}

/// Go: `htmltemplate.GoFuncs` — the escaper functions.
pub fn html_go_funcs() -> Vec<(&'static str, TplFunc)> {
    gotemplate::html::go_funcs()
}

/// Go: `texttemplate.GoFuncs` — the builtins (the engine recognises them when handed back).
pub fn text_go_funcs() -> Vec<(&'static str, TplFunc)> {
    gotemplate::text::go_funcs()
}

/// Go: `texttemplate.IsTrue`/`hreflect.IsTruthfulValue` as the engine implements it.
pub fn is_truthful_value(v: &Value) -> bool {
    gotemplate::text::is_truthful_value(v)
}
