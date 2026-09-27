//! Go: tpl/internal/go_templates/texttemplate/template.go + option.go
//!
//! A Go `*Template` is a pointer: several namespaces (after `Clone`) or
//! html/template wrappers hold the same object, and its `Tree` field is
//! reassigned in place. [`Template`] is therefore a cheap handle (`Arc`)
//! with interior mutability; clones of the handle are the same Go pointer.
//! Trees are [`SharedTree`]s (Go `*parse.Tree`).
//!
//! Locking mirrors Go: `common.tmpl` (Go `muTmpl`) and `common.funcs` (Go
//! `muFuncs`) are the namespace locks; a template's own mutable fields have
//! a small private mutex that is never held while taking another lock.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, RwLock};

use crate::parse::{self, FuncNames, SharedTree};

use super::funcs::{BUILTIN_NAMES, Func};

/// Go: `missingKeyAction`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MissingKeyAction {
    /// Return an invalid reflect.Value.
    #[default]
    Invalid,
    /// Return the zero value for the map element.
    ZeroValue,
    /// Error out
    Error,
}

/// Go: `option`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TplOption {
    pub missing_key: MissingKeyAction,
}

/// Go: `FuncMap` for execution: host functions by name. Hugo resolves
/// functions through `ExecHelper::get_func` instead; these are the
/// template's own `execFuncs` (Go `findFunction` looks here first).
pub type FuncMap = HashMap<String, Func>;

#[derive(Default)]
pub(crate) struct Funcs {
    /// Go: `parseFuncs` (only the names matter at parse time).
    pub(crate) parse: HashSet<String>,
    /// Go: `execFuncs`.
    pub(crate) exec: HashMap<String, Func>,
}

/// Go: `common` — the structure shared by associated templates.
#[derive(Default)]
pub(crate) struct Common {
    /// Map from name to defined templates (Go: `tmpl` guarded by `muTmpl`).
    pub(crate) tmpl: RwLock<HashMap<String, Template>>,
    pub(crate) option: RwLock<TplOption>,
    /// Go: `parseFuncs`/`execFuncs` guarded by `muFuncs`.
    pub(crate) funcs: RwLock<Funcs>,
}

struct TplFields {
    tree: Option<SharedTree>,
    left_delim: String,
    right_delim: String,
}

struct TemplateObj {
    name: String,
    common: Arc<Common>,
    fields: Mutex<TplFields>,
}

/// Go: `*template.Template` (text/template) — the representation of a
/// parsed template.
#[derive(Clone)]
pub struct Template(Arc<TemplateObj>);

impl std::fmt::Debug for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "text::Template({:?})", self.0.name)
    }
}

/// The builtin function names (Go `builtins()`), for parse-time checks.
struct BuiltinNames;

impl FuncNames for BuiltinNames {
    fn has_function(&self, name: &str) -> bool {
        BUILTIN_NAMES.contains(&name)
    }
}

impl FuncNames for Funcs {
    fn has_function(&self, name: &str) -> bool {
        self.parse.contains(name)
    }
}

impl Template {
    fn with_common(name: &str, common: Arc<Common>, left: String, right: String) -> Template {
        Template(Arc::new(TemplateObj {
            name: name.to_string(),
            common,
            fields: Mutex::new(TplFields {
                tree: None,
                left_delim: left,
                right_delim: right,
            }),
        }))
    }

    fn fields(&self) -> std::sync::MutexGuard<'_, TplFields> {
        self.0.fields.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn common(&self) -> &Arc<Common> {
        &self.0.common
    }

    // Go: template.go:New
    /// Allocates a new, undefined template with the given name.
    pub fn new(name: &str) -> Template {
        Template::with_common(name, Arc::new(Common::default()), String::new(), String::new())
    }

    // Go: template.go:(*Template).Name
    /// Returns the name of the template.
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// Go pointer equality.
    pub fn ptr_eq(&self, other: &Template) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Go: `t.Tree` (the embedded `*parse.Tree`, possibly nil).
    pub fn tree(&self) -> Option<SharedTree> {
        self.fields().tree.clone()
    }

    /// Go: `t.Tree = tree`.
    pub fn set_tree(&self, tree: Option<SharedTree>) {
        self.fields().tree = tree;
    }

    /// Go: `t.Tree != nil && t.Root != nil`.
    pub fn has_root(&self) -> bool {
        self.tree().is_some_and(|t| t.get().root.is_some())
    }

    /// The template's `option` (Go: `t.option`, shared by the namespace).
    pub fn option(&self) -> TplOption {
        *self.0.common.option.read().unwrap_or_else(|e| e.into_inner())
    }

    // Go: template.go:(*Template).New
    /// Allocates a new, undefined template associated with the given one and
    /// with the same delimiters. The association, which is transitive,
    /// allows one template to invoke another with a {{template}} action.
    ///
    /// Because associated templates share underlying data, template
    /// construction cannot be done safely in parallel. Once the templates
    /// are constructed, they can be executed in parallel.
    pub fn new_associated(&self, name: &str) -> Template {
        let (l, r) = {
            let f = self.fields();
            (f.left_delim.clone(), f.right_delim.clone())
        };
        Template::with_common(name, self.0.common.clone(), l, r)
    }

    // Go: template.go:(*Template).Clone
    /// Returns a duplicate of the template, including all associated
    /// templates. The actual representation is not copied, but the name
    /// space of associated templates is, so further calls to Parse in the
    /// copy will add templates to the copy but not to the original. Clone
    /// can be used to prepare common templates and use them with variant
    /// definitions for other templates by adding the variants after the
    /// clone is made.
    pub fn clone_ns(&self) -> Result<Template, crate::Error> {
        let nt = self.copy(Arc::new(Common::default()));
        let tmpl = self.0.common.tmpl.read().unwrap_or_else(|e| e.into_inner());
        let mut new_map = HashMap::new();
        for (k, v) in tmpl.iter() {
            if *k == self.0.name {
                new_map.insert(self.0.name.clone(), nt.clone());
                continue;
            }
            // The associated templates share nt's common structure.
            let t = v.copy(nt.0.common.clone());
            new_map.insert(k.clone(), t);
        }
        drop(tmpl);
        *nt.0.common.tmpl.write().unwrap_or_else(|e| e.into_inner()) = new_map;
        {
            let src = self.0.common.funcs.read().unwrap_or_else(|e| e.into_inner());
            let mut dst = nt.0.common.funcs.write().unwrap_or_else(|e| e.into_inner());
            dst.parse.extend(src.parse.iter().cloned());
            for (k, v) in &src.exec {
                dst.exec.insert(k.clone(), v.clone());
            }
        }
        // go1.24: Clone does not copy `option`.
        Ok(nt)
    }

    // Go: template.go:(*Template).copy
    /// Returns a shallow copy of t, with common set to the argument.
    fn copy(&self, c: Arc<Common>) -> Template {
        let f = self.fields();
        let nt = Template::with_common(&self.0.name, c, f.left_delim.clone(), f.right_delim.clone());
        nt.fields().tree = f.tree.clone();
        nt
    }

    // Go: template.go:(*Template).AddParseTree
    /// Associates the argument parse tree with the template t, giving it the
    /// specified name. If the template has not been defined, this tree
    /// becomes its definition. If it has been defined and already has that
    /// name, the existing definition is replaced; otherwise a new template
    /// is created, defined, and returned.
    pub fn add_parse_tree(&self, name: &str, tree: SharedTree) -> Result<Template, crate::Error> {
        let mut tmpl = self.0.common.tmpl.write().unwrap_or_else(|e| e.into_inner());
        let nt = if name != self.0.name {
            self.new_associated(name)
        } else {
            self.clone()
        };
        // Even if nt == t, we need to install it in the common.tmpl map.
        if Self::associate(&mut tmpl, &nt, &tree) || nt.tree().is_none() {
            nt.set_tree(Some(tree));
        }
        Ok(nt)
    }

    // Go: template.go:(*Template).Templates
    /// Returns a slice of defined templates associated with t.
    pub fn templates(&self) -> Vec<Template> {
        let tmpl = self.0.common.tmpl.read().unwrap_or_else(|e| e.into_inner());
        tmpl.values().cloned().collect()
    }

    /// Go: `All()` (hugo_template.go) — the templates of the namespace.
    pub fn all(&self) -> Vec<Template> {
        self.templates()
    }

    // Go: template.go:(*Template).Delims
    /// Sets the action delimiters to the specified strings, to be used in
    /// subsequent calls to Parse. An empty delimiter stands for the
    /// corresponding default: {{ or }}.
    pub fn delims(&self, left: &str, right: &str) -> &Template {
        let mut f = self.fields();
        f.left_delim = left.to_string();
        f.right_delim = right.to_string();
        self
    }

    // Go: template.go:(*Template).Funcs
    /// Adds the elements of the argument map to the template's function map.
    pub fn funcs(&self, func_map: &FuncMap) -> &Template {
        let mut funcs = self.0.common.funcs.write().unwrap_or_else(|e| e.into_inner());
        for (name, f) in func_map {
            if !good_name(name) {
                panic!("function name {} is not a valid identifier", go_strconv::quote(name));
            }
            funcs.exec.insert(name.clone(), f.clone());
            funcs.parse.insert(name.clone());
        }
        self
    }

    /// Adds names to the parse-time function set only (Go: a FuncMap whose
    /// values are never executed because an `ExecHelper` resolves them, as
    /// in Hugo).
    pub fn func_names<I, S>(&self, names: I) -> &Template
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut funcs = self.0.common.funcs.write().unwrap_or_else(|e| e.into_inner());
        for n in names {
            let n = n.into();
            if !good_name(&n) {
                panic!("function name {} is not a valid identifier", go_strconv::quote(&n));
            }
            funcs.parse.insert(n);
        }
        self
    }

    // Go: template.go:(*Template).Lookup
    /// Returns the template with the given name that is associated with t.
    pub fn lookup(&self, name: &str) -> Option<Template> {
        let tmpl = self.0.common.tmpl.read().unwrap_or_else(|e| e.into_inner());
        tmpl.get(name).cloned()
    }

    // Go: template.go:(*Template).Parse
    /// Parses text as a template body for t. Named template definitions
    /// ({{define ...}} or {{block ...}} statements) in text define
    /// additional templates associated with t and are removed from the
    /// definition of t itself.
    pub fn parse(&self, text: impl AsRef<[u8]>) -> Result<Template, crate::Error> {
        let (l, r) = {
            let f = self.fields();
            (f.left_delim.clone(), f.right_delim.clone())
        };
        let trees = {
            let funcs = self.0.common.funcs.read().unwrap_or_else(|e| e.into_inner());
            parse::parse(&self.0.name, text.as_ref(), &l, &r, &[&*funcs, &BuiltinNames])
                .map_err(crate::Error::Parse)?
        };
        // Add the newly parsed trees, including the one for t, into our common structure.
        for (name, tree) in trees {
            self.add_parse_tree(&name, SharedTree::new(tree))?;
        }
        Ok(self.clone())
    }

    // Go: template.go:(*Template).associate
    /// Installs the new template into the group of templates associated
    /// with t. The two are already known to share the common structure.
    /// The boolean return value reports whether to store this tree as t.Tree.
    fn associate(tmpl: &mut HashMap<String, Template>, new: &Template, tree: &SharedTree) -> bool {
        if let Some(old) = tmpl.get(&new.0.name) {
            if parse::is_empty_tree(tree.get().root.as_ref()) && old.tree().is_some() {
                // If a template by that name exists,
                // don't replace it with an empty template.
                return false;
            }
        }
        tmpl.insert(new.0.name.clone(), new.clone());
        true
    }

    // Go: option.go:(*Template).Option
    /// Sets options for the template. Options are described by strings,
    /// either a simple string or "key=value". There can be at most one
    /// equals sign in an option string. If the option string is
    /// unrecognized or otherwise invalid, Option panics.
    pub fn set_option(&self, opts: &[&str]) -> &Template {
        for s in opts {
            self.set_one_option(s);
        }
        self
    }

    // Go: option.go:(*Template).setOption
    fn set_one_option(&self, opt: &str) {
        if opt.is_empty() {
            panic!("empty option string");
        }
        // key=value
        if let Some((key, value)) = opt.split_once('=') {
            if key == "missingkey" {
                let mk = match value {
                    "invalid" | "default" => Some(MissingKeyAction::Invalid),
                    "zero" => Some(MissingKeyAction::ZeroValue),
                    "error" => Some(MissingKeyAction::Error),
                    _ => None,
                };
                if let Some(mk) = mk {
                    self.0.common.option.write().unwrap_or_else(|e| e.into_inner()).missing_key = mk;
                    return;
                }
            }
        }
        panic!("unrecognized option: {opt}");
    }

    // Go: exec.go:(*Template).DefinedTemplates
    /// Returns a string listing the defined templates, prefixed by the
    /// string "; defined templates are: ". If there are none, it returns
    /// the empty string. (The order follows Go's map iteration, i.e. is
    /// unspecified; this port sorts by name.)
    pub fn defined_templates(&self) -> String {
        let tmpl = self.0.common.tmpl.read().unwrap_or_else(|e| e.into_inner());
        let mut names: Vec<&String> = tmpl
            .iter()
            .filter(|(_, t)| t.has_root())
            .map(|(n, _)| n)
            .collect();
        names.sort();
        let mut b = String::new();
        for name in names {
            if b.is_empty() {
                b.push_str("; defined templates are: ");
            } else {
                b.push_str(", ");
            }
            b.push_str(&go_strconv::quote(name));
        }
        b
    }
}

// Go: funcs.go:goodName
/// Reports whether the function name is a valid identifier.
pub(crate) fn good_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    for (i, r) in name.chars().enumerate() {
        let r = r as i32;
        if r == '_' as i32 {
            continue;
        }
        if i == 0 && !go_unicode::is_letter(r) {
            return false;
        }
        if !go_unicode::is_letter(r) && !go_unicode::is_digit(r) {
            return false;
        }
    }
    true
}
