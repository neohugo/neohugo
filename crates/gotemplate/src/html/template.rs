//! Go: tpl/internal/go_templates/htmltemplate/template.go +
//! hugo_template.go (`Prepare`, `All`, `CloneShallow`).
//!
//! A Go `*Template` is a pointer whose fields change (`Parse` resyncs
//! `text`/`Tree`, `escape` sets `escapeErr`, `new` overwrites an existing
//! template with `*existing = *emptyTmpl`); [`Template`] is a handle to an
//! object with interior mutability, and clones of the handle are the same
//! Go pointer. The namespace (`nameSpace`) is shared by `Arc` and guarded
//! by one mutex, like Go's `nameSpace.mu`.

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex, MutexGuard};

use go_value::{HostCtx, Value};

use crate::error::Error as CrateError;
use crate::parse::SharedTree;
use crate::text;

use super::error::Error;
use super::escape::{Escaper, escape_template};

/// Go: `escapeErr` — nil (not yet escaped), `escapeOK`, or the error.
#[derive(Clone)]
enum EscapeState {
    None,
    Ok,
    Err(Arc<Error>),
}

struct Fields {
    escape_err: EscapeState,
    /// The underlying template's parse tree, updated to be HTML-safe.
    text: text::Template,
    /// Go: `Tree *parse.Tree` — the html template's view of the tree.
    tree: Option<SharedTree>,
    /// common to all associated templates
    ns: Arc<NameSpace>,
}

struct TemplateObj {
    fields: Mutex<Fields>,
}

/// Go: `*html/template.Template` — a specialized Template from
/// "text/template" that produces a safe HTML document fragment.
#[derive(Clone)]
pub struct Template(Arc<TemplateObj>);

impl std::fmt::Debug for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "html::Template({:?})", self.name())
    }
}

/// Go: `nameSpace` — the data structure shared by all templates in an
/// association.
pub(crate) struct NameSpace {
    mu: Mutex<NsInner>,
}

pub(crate) struct NsInner {
    pub(crate) set: HashMap<String, Template>,
    pub(crate) escaped: bool,
    pub(crate) esc: Escaper,
}

impl NsInner {
    /// Go: `e.arbitraryTemplate().text` — the text/template namespace
    /// shared by all templates of this html namespace.
    pub(crate) fn arbitrary_text(&self) -> text::Template {
        match self.set.values().next() {
            Some(t) => t.fields().text.clone(),
            None => panic!("no templates in name space"),
        }
    }
}

impl NameSpace {
    fn new() -> Arc<NameSpace> {
        Arc::new(NameSpace {
            mu: Mutex::new(NsInner {
                set: HashMap::new(),
                escaped: false,
                esc: Escaper::new(),
            }),
        })
    }

    fn lock(&self) -> MutexGuard<'_, NsInner> {
        self.mu.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn html_err(msg: String) -> CrateError {
    CrateError::Other(msg)
}

fn escape_err_to_crate(e: &Arc<Error>) -> CrateError {
    CrateError::Html(Arc::new(ErrorBox(e.clone())))
}

/// `Arc<html::Error>` as a `std::error::Error` (for `CrateError::Html`).
#[derive(Debug)]
pub struct ErrorBox(pub Arc<Error>);

impl std::fmt::Display for ErrorBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&*self.0, f)
    }
}

impl std::error::Error for ErrorBox {}

impl Template {
    fn from_fields(f: Fields) -> Template {
        Template(Arc::new(TemplateObj { fields: Mutex::new(f) }))
    }

    fn fields(&self) -> MutexGuard<'_, Fields> {
        self.0.fields.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn ns(&self) -> Arc<NameSpace> {
        self.fields().ns.clone()
    }

    /// Go pointer equality.
    pub fn ptr_eq(&self, other: &Template) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// The underlying text/template (Go: `t.text`).
    pub fn text(&self) -> text::Template {
        self.fields().text.clone()
    }

    /// Go: `t.Tree`.
    pub fn tree(&self) -> Option<SharedTree> {
        self.fields().tree.clone()
    }

    pub(crate) fn set_escape_failed(&self, err: Arc<Error>) {
        let mut f = self.fields();
        f.escape_err = EscapeState::Err(err);
        f.text.set_tree(None);
        f.tree = None;
    }

    pub(crate) fn set_escape_ok(&self) {
        let mut f = self.fields();
        f.escape_err = EscapeState::Ok;
        f.tree = f.text.tree();
    }

    // Go: template.go:(*Template).Templates
    /// Returns a slice of the templates associated with t, including t itself.
    pub fn templates(&self) -> Vec<Template> {
        let ns = self.ns();
        let inner = ns.lock();
        inner.set.values().cloned().collect()
    }

    /// Go: `All()` (hugo_template.go).
    pub fn all(&self) -> Vec<Template> {
        self.templates()
    }

    // Go: template.go:(*Template).Option
    /// Sets options for the template (see text/template).
    pub fn option(&self, opts: &[&str]) -> &Template {
        self.text().set_option(opts);
        self
    }

    // Go: template.go:(*Template).checkCanParse
    /// Returns an error if it is not possible to parse t.
    fn check_can_parse(&self) -> Result<(), CrateError> {
        let ns = self.ns();
        let inner = ns.lock();
        if inner.escaped {
            return Err(html_err("html/template: cannot Parse after Execute".to_string()));
        }
        Ok(())
    }

    // Go: template.go:(*Template).escape
    /// Escapes all associated templates.
    fn escape(&self) -> Result<(), CrateError> {
        let ns = self.ns();
        let mut inner = ns.lock();
        inner.escaped = true;
        let state = self.fields().escape_err.clone();
        match state {
            EscapeState::None => {
                let (tree, text) = {
                    let f = self.fields();
                    (f.tree.clone(), f.text.clone())
                };
                if tree.is_none() {
                    return Err(html_err(format!(
                        "template: {} is an incomplete or empty template",
                        go_strconv::quote(text.name())
                    )));
                }
                let name = text.name().to_string();
                let snapshot = text.tree().map(|t| t.get());
                let root: &dyn crate::parse::NodeLike = match snapshot.as_ref().and_then(|t| t.root.as_ref()) {
                    Some(r) => r,
                    None => &crate::parse::ListNode::default(),
                };
                if let Err(e) = escape_template(&mut inner, root, &name) {
                    return Err(escape_err_to_crate(&e));
                }
                Ok(())
            }
            EscapeState::Ok => Ok(()),
            EscapeState::Err(e) => Err(escape_err_to_crate(&e)),
        }
    }

    // Go: template.go:(*Template).Execute
    /// Applies a parsed template to the specified data object, writing the
    /// output to wr.
    pub fn execute(&self, wr: &mut dyn Write, data: &Value) -> Result<(), CrateError> {
        self.escape()?;
        self.text().execute(wr, data)
    }

    // Go: template.go:(*Template).ExecuteTemplate
    /// Applies the template associated with t that has the given name to
    /// the specified data object and writes the output to wr.
    pub fn execute_template(&self, wr: &mut dyn Write, name: &str, data: &Value) -> Result<(), CrateError> {
        let tmpl = self.lookup_and_escape_template(name)?;
        tmpl.text().execute(wr, data)
    }

    // Go: template.go:(*Template).lookupAndEscapeTemplate
    /// Guarantees that the template with the given name is escaped, or
    /// returns an error if it cannot be. It returns the named template.
    fn lookup_and_escape_template(&self, name: &str) -> Result<Template, CrateError> {
        let ns = self.ns();
        let mut inner = ns.lock();
        inner.escaped = true;
        let Some(tmpl) = inner.set.get(name).cloned() else {
            return Err(html_err(format!("html/template: {} is undefined", go_strconv::quote(name))));
        };
        let state = tmpl.fields().escape_err.clone();
        if let EscapeState::Err(e) = &state {
            return Err(escape_err_to_crate(e));
        }
        let text = tmpl.text();
        if !text.has_root() {
            return Err(html_err(format!(
                "html/template: {} is an incomplete template",
                go_strconv::quote(name)
            )));
        }
        if self.text().lookup(name).is_none() {
            panic!("html/template internal error: template escaping out of sync");
        }
        if let EscapeState::None = state {
            let snapshot = text.tree().map(|t| t.get());
            let root = snapshot.as_ref().and_then(|t| t.root.as_ref()).expect("root");
            if let Err(e) = escape_template(&mut inner, root, name) {
                return Err(escape_err_to_crate(&e));
            }
        }
        Ok(tmpl)
    }

    // Go: template.go:(*Template).DefinedTemplates
    /// Returns a string listing the defined templates, prefixed by the
    /// string "; defined templates are: ".
    pub fn defined_templates(&self) -> String {
        self.text().defined_templates()
    }

    // Go: template.go:(*Template).Parse
    /// Parses text as a template body for t. Named template definitions
    /// ({{define ...}} or {{block ...}} statements) in text define
    /// additional templates associated with t and are removed from the
    /// definition of t itself.
    ///
    /// Templates can be redefined in successive calls to Parse, before the
    /// first use of Execute. A template definition with a body containing
    /// only white space and comments is considered empty and will not
    /// replace an existing template's body.
    pub fn parse(&self, text: impl AsRef<[u8]>) -> Result<Template, CrateError> {
        self.check_can_parse()?;

        let ret = self.text().parse(text)?;

        // In defining the template, we may have created new templates
        // associated with t. Make sure we're tracking them.
        let ns = self.ns();
        let mut inner = ns.lock();
        for v in ret.templates() {
            let name = v.name().to_string();
            let tmpl = match inner.set.get(&name) {
                Some(t) => t.clone(),
                None => self.new_locked(&mut inner, &name),
            };
            let mut f = tmpl.fields();
            f.tree = v.tree();
            f.text = v;
        }
        Ok(self.clone())
    }

    // Go: template.go:(*Template).AddParseTree
    /// Creates a new template with the name and parse tree and associates
    /// it with t.
    pub fn add_parse_tree(&self, name: &str, tree: SharedTree) -> Result<Template, CrateError> {
        self.check_can_parse()?;

        let ns = self.ns();
        let mut inner = ns.lock();
        let text = self.text().add_parse_tree(name, tree)?;
        let ret = Template::from_fields(Fields {
            escape_err: EscapeState::None,
            tree: text.tree(),
            text,
            ns: ns.clone(),
        });
        inner.set.insert(name.to_string(), ret.clone());
        Ok(ret)
    }

    // Go: template.go:(*Template).Clone
    /// Returns a duplicate of the template, including all associated
    /// templates. The actual representation is not copied, but the name
    /// space of associated templates is, so further calls to Parse in the
    /// copy will add templates to the copy but not to the original.
    ///
    /// It returns an error if t has already been executed.
    pub fn clone_ns(&self) -> Result<Template, CrateError> {
        let ns = self.ns();
        let inner = ns.lock();
        if !matches!(self.fields().escape_err, EscapeState::None) {
            return Err(html_err(format!(
                "html/template: cannot Clone {} after it has executed",
                go_strconv::quote(self.name())
            )));
        }
        let text_clone = self.text().clone_ns()?;
        let new_ns = NameSpace::new();
        let ret = Template::from_fields(Fields {
            escape_err: EscapeState::None,
            tree: text_clone.tree(),
            text: text_clone.clone(),
            ns: new_ns.clone(),
        });
        let mut new_inner = new_ns.lock();
        new_inner.set.insert(ret.name(), ret.clone());
        for x in text_clone.templates() {
            let name = x.name().to_string();
            let src = inner.set.get(&name);
            if src.is_none_or(|s| !matches!(s.fields().escape_err, EscapeState::None)) {
                return Err(html_err(format!(
                    "html/template: cannot Clone {} after it has executed, {} not found",
                    go_strconv::quote(self.name()),
                    go_strconv::quote(&name)
                )));
            }
            let copy = x.tree().map(|t| SharedTree::new(t.get().copy()));
            x.set_tree(copy.clone());
            new_inner.set.insert(
                name,
                Template::from_fields(Fields {
                    escape_err: EscapeState::None,
                    text: x,
                    tree: copy,
                    ns: new_ns.clone(),
                }),
            );
        }
        let name = ret.name();
        Ok(new_inner.set.get(&name).cloned().expect("clone"))
    }

    // Go: hugo_template.go:(*Template).CloneShallow
    /// Clones the underlying text template (sharing the parse trees) into a
    /// new html namespace that contains only t itself (Hugo).
    pub fn clone_shallow(&self) -> Result<Template, CrateError> {
        let ns = self.ns();
        let _inner = ns.lock();
        if !matches!(self.fields().escape_err, EscapeState::None) {
            return Err(html_err(format!(
                "html/template: cannot Clone {} after it has executed",
                go_strconv::quote(self.name())
            )));
        }
        let text_clone = self.text().clone_ns()?;
        let new_ns = NameSpace::new();
        let ret = Template::from_fields(Fields {
            escape_err: EscapeState::None,
            tree: text_clone.tree(),
            text: text_clone,
            ns: new_ns.clone(),
        });
        new_ns.lock().set.insert(ret.name(), ret.clone());
        Ok(ret)
    }

    // Go: template.go:New
    /// Allocates a new HTML template with the given name.
    pub fn new(name: &str) -> Template {
        let ns = NameSpace::new();
        let tmpl = Template::from_fields(Fields {
            escape_err: EscapeState::None,
            text: text::Template::new(name),
            tree: None,
            ns: ns.clone(),
        });
        ns.lock().set.insert(name.to_string(), tmpl.clone());
        tmpl
    }

    // Go: template.go:(*Template).New
    /// Allocates a new HTML template associated with the given one and with
    /// the same delimiters. The association, which is transitive, allows
    /// one template to invoke another with a {{template}} action.
    ///
    /// If a template with the given name already exists, the new HTML
    /// template will replace it. The existing template will be reset and
    /// disassociated with t.
    pub fn new_associated(&self, name: &str) -> Template {
        let ns = self.ns();
        let mut inner = ns.lock();
        self.new_locked(&mut inner, name)
    }

    // Go: template.go:(*Template).new
    /// The implementation of New, without the lock.
    fn new_locked(&self, inner: &mut NsInner, name: &str) -> Template {
        let (text, ns) = {
            let f = self.fields();
            (f.text.new_associated(name), f.ns.clone())
        };
        let tmpl = Template::from_fields(Fields {
            escape_err: EscapeState::None,
            text,
            tree: None,
            ns,
        });
        if let Some(existing) = inner.set.get(name) {
            let empty_tmpl = Template::new(&existing.name());
            let ef = empty_tmpl.fields();
            let mut xf = existing.fields();
            xf.escape_err = ef.escape_err.clone();
            xf.text = ef.text.clone();
            xf.tree = ef.tree.clone();
            xf.ns = ef.ns.clone();
        }
        inner.set.insert(name.to_string(), tmpl.clone());
        tmpl
    }

    // Go: template.go:(*Template).Name
    /// Returns the name of the template.
    pub fn name(&self) -> String {
        self.text().name().to_string()
    }

    // Go: template.go:(*Template).Funcs
    /// Adds the elements of the argument map to the template's function
    /// map.
    pub fn funcs(&self, func_map: &text::FuncMap) -> &Template {
        self.text().funcs(func_map);
        self
    }

    /// Adds parse-time function names (see [`text::Template::func_names`]).
    pub fn func_names<I, S>(&self, names: I) -> &Template
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.text().func_names(names);
        self
    }

    // Go: template.go:(*Template).Delims
    /// Sets the action delimiters to the specified strings.
    pub fn delims(&self, left: &str, right: &str) -> &Template {
        self.text().delims(left, right);
        self
    }

    // Go: template.go:(*Template).Lookup
    /// Returns the template with the given name that is associated with t,
    /// or nil if there is no such template.
    pub fn lookup(&self, name: &str) -> Option<Template> {
        let ns = self.ns();
        let inner = ns.lock();
        inner.set.get(name).cloned()
    }

    // Go: hugo_template.go:(*Template).Prepare
    /// Escapes the template (once) and returns the rewritten text template.
    pub fn prepare(&self) -> Result<text::Template, CrateError> {
        self.escape()?;
        Ok(self.text())
    }

    /// Executes with a helper and host context (Go:
    /// `Executer.ExecuteWithContext(ctx, t, wr, data)`).
    pub fn execute_with_helper(
        &self,
        ctx: HostCtx<'_>,
        helper: &dyn text::ExecHelper,
        wr: &mut dyn Write,
        data: &Value,
    ) -> Result<(), CrateError> {
        let t = self.prepare()?;
        t.execute_with_helper(ctx, helper, wr, data)
    }
}

impl text::Preparer for Template {
    fn prepare(&self) -> Result<text::Template, CrateError> {
        Template::prepare(self)
    }
    fn preparer_name(&self) -> String {
        self.name()
    }
}
