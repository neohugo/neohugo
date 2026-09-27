//! Go: tpl/internal/go_templates/htmltemplate/escape.go (go1.24.0 fork)
//!
//! The escaper walks parse trees read-only, recording edits keyed by
//! [`NodeId`] (Go: node pointers), and `commit` applies them to the shared
//! trees of the namespace. See PORTING.md for how Go's pointer-keyed maps
//! and in-place mutation map onto [`SharedTree`].

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::parse::*;
use crate::text;

use super::context::{
    Attr, Context, Delim, JsCtx, State, UrlPart, is_comment, is_in_script_literal,
};
use super::error::{Error, ErrorCode, errorf};
use super::js::{contains_special_script_tag, escape_special_script_tags, is_js_type};
use super::transition::{attr_start_state, delim_ends, t_special_tag_end, transition};

// Go: escape.go:escapeTemplate
/// Rewrites the named template, which must be associated with t, to
/// guarantee that the output of any of the named templates is properly
/// escaped. If no error is returned, then the named templates have been
/// modified. Otherwise the named templates have been rendered unusable.
///
/// Called with the namespace locked (`ns` is the locked inner state).
pub(crate) fn escape_template(
    ns: &mut super::template::NsInner,
    node: &dyn NodeLike,
    name: &str,
) -> Result<(), Arc<Error>> {
    let text_ns = ns.arbitrary_text();
    let set_has = |n: &str| ns.set.contains_key(n);
    let (c, _) = {
        let esc = &mut ns.esc;
        esc.escape_tree(&text_ns, &set_has, Context::default(), node, name, 0)
    };
    let mut err: Option<Arc<Error>> = None;
    if let Some(e) = &c.err {
        let mut e2 = (**e).clone();
        e2.name = name.to_string();
        err = Some(Arc::new(e2));
    } else if c.state != State::Text {
        err = Some(Arc::new(Error {
            error_code: ErrorCode::ErrEndContext,
            node: None,
            name: name.to_string(),
            line: 0,
            description: format!("ends in a non-text context: {c}"),
        }));
    }
    if let Some(err) = err {
        // Prevent execution of unsafe templates.
        if let Some(t) = ns.set.get(name) {
            t.set_escape_failed(err.clone());
        }
        return Err(err);
    }
    ns.esc.commit(&text_ns);
    if let Some(t) = ns.set.get(name) {
        t.set_escape_ok();
    }
    Ok(())
}

/// The escaper funcs (Go: `funcMap`) as text/template funcs.
pub(crate) fn esc_func_map() -> text::FuncMap {
    let mut m = text::FuncMap::new();
    for &name in super::ESC_FUNC_NAMES {
        let f = super::esc_func(name).expect("escaper func");
        let func: text::Func =
            Arc::new(move |_ctx, args: &[go_value::Value]| Ok(go_value::Value::string(f(args))));
        m.insert(name.to_string(), func);
    }
    m
}

/// Go: `rangeContext` — information about the current range loop.
#[derive(Default)]
pub(crate) struct RangeContext {
    /// outer loop
    outer: Option<Arc<Mutex<RangeContext>>>,
    /// context at each break action
    breaks: Vec<Context>,
    /// context at each continue action
    continues: Vec<Context>,
}

/// Go: `escaper` — collects type inferences about templates and changes
/// needed to make templates injection safe.
#[derive(Default)]
pub(crate) struct Escaper {
    /// output[templateName] is the output context for a templateName that
    /// has been mangled to include its input context.
    output: HashMap<String, Context>,
    /// derived[c.mangle(name)] maps to a template derived from the template
    /// named name templateName for the start context c.
    derived: HashMap<String, SharedTree>,
    /// called[templateName] is a set of called mangled template names.
    called: HashSet<String>,
    /// xxxNodeEdits are the accumulated edits to apply during commit.
    /// Such edits are not applied immediately in case a template set
    /// executes a given template in different escaping contexts.
    action_node_edits: HashMap<NodeId, Vec<String>>,
    template_node_edits: HashMap<NodeId, String>,
    text_node_edits: HashMap<NodeId, Vec<u8>>,
    /// rangeContext holds context about the current range loop (shared by
    /// pointer between an escaper and the conditional escapers it spawns).
    range_context: Option<Arc<Mutex<RangeContext>>>,
}

/// A tree found by `Escaper::template` (Go returns the `*template.Template`
/// whose `Tree` is walked).
struct FoundTemplate {
    name: String,
    tree: Option<SharedTree>,
}

impl Escaper {
    // Go: escape.go:makeEscaper
    /// Creates a blank escaper for the given set.
    pub(crate) fn new() -> Escaper {
        Escaper::default()
    }

    // Go: escape.go:(*escaper).escape
    /// Escapes a template node.
    fn escape(&mut self, env: &Env<'_>, c: Context, n: &Node) -> Context {
        match n {
            Node::Action(n) => self.escape_action(c, n),
            Node::Break(b) => {
                let mut c = c;
                c.n = Some(Node::Break(b.clone()));
                if let Some(rc) = &self.range_context {
                    rc.lock().unwrap_or_else(|e| e.into_inner()).breaks.push(c);
                }
                Context {
                    state: State::Dead,
                    ..Default::default()
                }
            }
            Node::Comment(_) => c,
            Node::Continue(cn) => {
                let mut c = c;
                c.n = Some(Node::Continue(cn.clone()));
                if let Some(rc) = &self.range_context {
                    rc.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .continues
                        .push(c);
                }
                Context {
                    state: State::Dead,
                    ..Default::default()
                }
            }
            Node::If(b) => self.escape_branch(env, c, b, "if"),
            Node::List(l) => self.escape_list(env, c, Some(l)),
            Node::Range(b) => self.escape_branch(env, c, b, "range"),
            Node::Template(t) => self.escape_template_node(env, c, t),
            Node::Text(t) => self.escape_text(c, t),
            Node::With(b) => self.escape_branch(env, c, b, "with"),
            _ => panic!("escaping {n} is unimplemented"),
        }
    }

    // Go: escape.go:(*escaper).escapeAction
    /// Escapes an action template node.
    fn escape_action(&mut self, c: Context, n: &ActionNode) -> Context {
        if !n.pipe.decl.is_empty() {
            // A local variable assignment, not an interpolation.
            return c;
        }
        let mut c = nudge(c);
        // Check for disallowed use of predefined escapers in the pipeline.
        for (pos, id_node) in n.pipe.cmds.iter().enumerate() {
            let Node::Identifier(node) = &id_node.args[0] else {
                // A predefined escaper "esc" will never be found as an identifier in a
                // Chain or Field node, since:
                // - "esc.x ..." is invalid, since predefined escapers return strings, and
                //   strings do not have methods, keys or fields.
                // - "... .esc" is invalid, since predefined escapers are global functions,
                //   not methods or fields of any types.
                // Therefore, it is safe to ignore these two node types.
                continue;
            };
            let ident = node.ident.as_str();
            if is_predefined_escaper(ident)
                && (pos < n.pipe.cmds.len() - 1
                    || c.state == State::Attr && c.delim == Delim::SpaceOrTagEnd && ident == "html")
            {
                return Context {
                    state: State::Error,
                    err: Some(Arc::new(errorf(
                        ErrorCode::ErrPredefinedEscaper,
                        Some(n),
                        n.line,
                        format!(
                            "predefined escaper {} disallowed in template",
                            go_strconv::quote(ident)
                        ),
                    ))),
                    ..Default::default()
                };
            }
        }
        let mut s: Vec<String> = Vec::with_capacity(3);
        match c.state {
            State::Error => return c,
            State::Url
            | State::CssDqStr
            | State::CssSqStr
            | State::CssDqUrl
            | State::CssSqUrl
            | State::CssUrl => {
                match c.url_part {
                    UrlPart::None | UrlPart::PreQuery => {
                        if c.url_part == UrlPart::None {
                            s.push("_html_template_urlfilter".to_string());
                        }
                        // fallthrough
                        match c.state {
                            State::CssDqStr | State::CssSqStr => {
                                s.push("_html_template_cssescaper".to_string())
                            }
                            _ => s.push("_html_template_urlnormalizer".to_string()),
                        }
                    }
                    UrlPart::QueryOrFrag => s.push("_html_template_urlescaper".to_string()),
                    UrlPart::Unknown => {
                        return Context {
                            state: State::Error,
                            err: Some(Arc::new(errorf(
                                ErrorCode::ErrAmbigContext,
                                Some(n),
                                n.line,
                                format!(
                                    "{} appears in an ambiguous context within a URL",
                                    n.to_string_lossy()
                                ),
                            ))),
                            ..Default::default()
                        };
                    }
                }
            }
            State::Js => {
                s.push("_html_template_jsvalescaper".to_string());
                // A slash after a value starts a div operator.
                c.js_ctx = JsCtx::DivOp;
            }
            State::JsDqStr | State::JsSqStr => s.push("_html_template_jsstrescaper".to_string()),
            State::JsTmplLit => s.push("_html_template_jstmpllitescaper".to_string()),
            State::JsRegexp => s.push("_html_template_jsregexpescaper".to_string()),
            State::Css => s.push("_html_template_cssvaluefilter".to_string()),
            State::Text => s.push("_html_template_htmlescaper".to_string()),
            State::Rcdata => s.push("_html_template_rcdataescaper".to_string()),
            State::Attr => {
                // Handled below in delim check.
            }
            State::AttrName | State::Tag => {
                c.state = State::AttrName;
                s.push("_html_template_htmlnamefilter".to_string());
            }
            State::Srcset => s.push("_html_template_srcsetescaper".to_string()),
            _ => {
                if is_comment(c.state) {
                    s.push("_html_template_commentescaper".to_string());
                } else {
                    panic!("unexpected state {}", c.state);
                }
            }
        }
        match c.delim {
            Delim::None => {
                // No extra-escaping needed for raw text content.
            }
            Delim::SpaceOrTagEnd => s.push("_html_template_nospaceescaper".to_string()),
            _ => s.push("_html_template_attrescaper".to_string()),
        }
        self.edit_action_node(n, s);
        c
    }

    // Go: escape.go:(*escaper).escapeBranch
    /// Escapes a branch template node: "if", "range" and "with".
    fn escape_branch(
        &mut self,
        env: &Env<'_>,
        c: Context,
        n: &BranchNode,
        node_name: &str,
    ) -> Context {
        if node_name == "range" {
            self.range_context = Some(Arc::new(Mutex::new(RangeContext {
                outer: self.range_context.take(),
                ..Default::default()
            })));
        }
        let mut c0 = self.escape_list(env, c.clone(), Some(&n.list));
        if node_name == "range" {
            if c0.state != State::Error {
                c0 = join_range(c0, self.range_context.as_ref().expect("range context"));
            }
            self.pop_range_context();
            if c0.state == State::Error {
                return c0;
            }

            // The "true" branch of a "range" node can execute multiple times.
            // We check that executing n.List once results in the same context
            // as executing n.List twice.
            self.range_context = Some(Arc::new(Mutex::new(RangeContext {
                outer: self.range_context.take(),
                ..Default::default()
            })));
            let (c1, _) = self.escape_list_conditionally(env, c0.clone(), &n.list, None);
            c0 = join(c0, c1, Some(n), node_name);
            if c0.state == State::Error {
                self.pop_range_context();
                // Make clear that this is a problem on loop re-entry
                // since developers tend to overlook that branch when
                // debugging templates.
                if let Some(e) = &c0.err {
                    let mut e2 = (**e).clone();
                    e2.line = n.line;
                    e2.description = format!("on range loop re-entry: {}", e2.description);
                    c0.err = Some(Arc::new(e2));
                }
                return c0;
            }
            c0 = join_range(c0, self.range_context.as_ref().expect("range context"));
            self.pop_range_context();
            if c0.state == State::Error {
                return c0;
            }
        }
        let c1 = self.escape_list(env, c, n.else_list.as_ref());
        join(c0, c1, Some(n), node_name)
    }

    fn pop_range_context(&mut self) {
        let outer = self
            .range_context
            .as_ref()
            .and_then(|rc| rc.lock().unwrap_or_else(|e| e.into_inner()).outer.clone());
        self.range_context = outer;
    }

    // Go: escape.go:(*escaper).escapeList
    /// Escapes a list template node.
    fn escape_list(&mut self, env: &Env<'_>, mut c: Context, n: Option<&ListNode>) -> Context {
        let Some(n) = n else {
            return c;
        };
        for m in &n.nodes {
            c = self.escape(env, c, m);
            if c.state == State::Dead {
                break;
            }
        }
        c
    }

    // Go: escape.go:(*escaper).escapeListConditionally
    /// Escapes a list node but only preserves edits and inferences in e if
    /// the inferences and output context satisfy filter. It returns the best
    /// guess at an output context, and the result of the filter which is the
    /// same as whether e was updated.
    fn escape_list_conditionally(
        &mut self,
        env: &Env<'_>,
        c: Context,
        n: &ListNode,
        filter: Option<&dyn Fn(&Escaper, &Context) -> bool>,
    ) -> (Context, bool) {
        let mut e1 = Escaper::new();
        e1.range_context = self.range_context.clone();
        // Make type inferences available to f.
        for (k, v) in &self.output {
            e1.output.insert(k.clone(), v.clone());
        }
        let c = e1.escape_list(env, c, Some(n));
        let ok = filter.is_some_and(|f| f(&e1, &c));
        if ok {
            // Copy inferences and edits from e1 back into e.
            for (k, v) in e1.output {
                self.output.insert(k, v);
            }
            for (k, v) in e1.derived {
                self.derived.insert(k, v);
            }
            for k in e1.called {
                self.called.insert(k);
            }
            for (k, v) in e1.action_node_edits {
                self.edit_action_node_id(k, v);
            }
            for (k, v) in e1.template_node_edits {
                self.edit_template_node_id(k, v);
            }
            for (k, v) in e1.text_node_edits {
                self.edit_text_node_id(k, v);
            }
        }
        (c, ok)
    }

    // Go: escape.go:(*escaper).escapeTemplate
    /// Escapes a {{template}} call node.
    fn escape_template_node(&mut self, env: &Env<'_>, c: Context, n: &TemplateNode) -> Context {
        let (c, name) = self.escape_tree(env.text_ns, env.set_has, c, n, &n.name, n.line);
        if name != n.name {
            self.edit_template_node(n, name);
        }
        c
    }

    // Go: escape.go:(*escaper).escapeTree
    /// Escapes the named template starting in the given context as
    /// necessary and returns its output context.
    pub(crate) fn escape_tree(
        &mut self,
        text_ns: &text::Template,
        set_has: &dyn Fn(&str) -> bool,
        c: Context,
        node: &dyn NodeLike,
        name: &str,
        line: usize,
    ) -> (Context, String) {
        // Mangle the template name with the input context to produce a reliable
        // identifier.
        let dname = c.mangle(name);
        self.called.insert(dname.clone());
        if let Some(out) = self.output.get(&dname) {
            // Already escaped.
            return (out.clone(), dname);
        }
        let t = self.template(text_ns, name);
        let t = match t {
            Some(t) if t.tree.as_ref().is_some_and(|tr| tr.get().root.is_some()) => t,
            _ => {
                // Two cases: The template exists but is empty, or has never been mentioned at
                // all. Distinguish the cases in the error messages.
                if set_has(name) {
                    return (
                        Context {
                            state: State::Error,
                            err: Some(Arc::new(errorf(
                                ErrorCode::ErrNoSuchTemplate,
                                Some(node),
                                line,
                                format!(
                                    "{} is an incomplete or empty template",
                                    go_strconv::quote(name)
                                ),
                            ))),
                            ..Default::default()
                        },
                        dname,
                    );
                }
                return (
                    Context {
                        state: State::Error,
                        err: Some(Arc::new(errorf(
                            ErrorCode::ErrNoSuchTemplate,
                            Some(node),
                            line,
                            format!("no such template {}", go_strconv::quote(name)),
                        ))),
                        ..Default::default()
                    },
                    dname,
                );
            }
        };
        let t = if dname != name {
            // Use any template derived during an earlier call to escapeTemplate
            // with different top level templates, or clone if necessary.
            match self.template(text_ns, &dname) {
                Some(dt) if dt.tree.is_some() => dt,
                _ => {
                    let root = t
                        .tree
                        .as_ref()
                        .and_then(|tr| tr.get().root.as_ref().map(|r| r.copy_list()));
                    let mut tree = Tree::new(dname.clone());
                    tree.root = root;
                    let st = SharedTree::new(tree);
                    self.derived.insert(dname.clone(), st.clone());
                    FoundTemplate {
                        name: dname.clone(),
                        tree: Some(st),
                    }
                }
            }
        } else {
            t
        };
        let env = Env { text_ns, set_has };
        (self.compute_out_ctx(&env, c, &t), dname)
    }

    // Go: escape.go:(*escaper).computeOutCtx
    /// Takes a template and its start context and computes the output
    /// context while storing any inferences in e.
    fn compute_out_ctx(&mut self, env: &Env<'_>, c: Context, t: &FoundTemplate) -> Context {
        // Propagate context over the body.
        let (mut c1, mut ok) = self.escape_template_body(env, c, t);
        if !ok {
            // Look for a fixed point by assuming c1 as the output context.
            let (c2, ok2) = self.escape_template_body(env, c1.clone(), t);
            if ok2 {
                c1 = c2;
                ok = true;
            }
            // Use c1 as the error context if neither assumption worked.
        }
        if !ok && c1.state != State::Error {
            let tree = t.tree.as_ref().map(|t| t.get());
            let root = tree.as_ref().and_then(|t| t.root.as_ref());
            return Context {
                state: State::Error,
                err: Some(Arc::new(errorf(
                    ErrorCode::ErrOutputContext,
                    root.map(|r| r as &dyn NodeLike),
                    0,
                    format!("cannot compute output context for template {}", t.name),
                ))),
                ..Default::default()
            };
        }
        c1
    }

    // Go: escape.go:(*escaper).escapeTemplateBody
    /// Escapes the given template assuming the given output context, and
    /// returns the best guess at the output context and whether the
    /// assumption was correct.
    fn escape_template_body(
        &mut self,
        env: &Env<'_>,
        c: Context,
        t: &FoundTemplate,
    ) -> (Context, bool) {
        let t_name = t.name.clone();
        let assumed = c.clone();
        let filter = move |e1: &Escaper, c1: &Context| -> bool {
            if c1.state == State::Error {
                // Do not update the input escaper, e.
                return false;
            }
            if !e1.called.contains(&t_name) {
                // If t is not recursively called, then c1 is an
                // accurate output context.
                return true;
            }
            // c1 is accurate if it matches our assumed output context.
            assumed.eq(c1)
        };
        // We need to assume an output context so that recursive template calls
        // take the fast path out of escapeTree instead of infinitely recurring.
        // Naively assuming that the input context is the same as the output
        // works >90% of the time.
        self.output.insert(t.name.clone(), c.clone());
        let tree = t.tree.as_ref().map(|t| t.get());
        let Some(root) = tree.as_ref().and_then(|t| t.root.as_ref()) else {
            return (c, false);
        };
        self.escape_list_conditionally(env, c, root, Some(&filter))
    }

    // Go: escape.go:(*escaper).escapeText
    /// Escapes a text template node.
    fn escape_text(&mut self, mut c: Context, n: &TextNode) -> Context {
        let s = &n.text[..];
        let mut written = 0usize;
        let mut i = 0usize;
        let mut b: Vec<u8> = Vec::new();
        while i != s.len() {
            let (c1, nread) = context_after_text(c.clone(), &s[i..]);
            let i1 = i + nread;
            if c.state == State::Text || c.state == State::Rcdata {
                let mut end = i1;
                if c1.state != c.state {
                    let mut j = end as isize - 1;
                    while j >= i as isize {
                        if s[j as usize] == b'<' {
                            end = j as usize;
                            break;
                        }
                        j -= 1;
                    }
                }
                for j in i..end {
                    if s[j] == b'<' && !has_prefix_upper(&s[j..], DOCTYPE_BYTES) {
                        b.extend_from_slice(&s[written..j]);
                        b.extend_from_slice(b"&lt;");
                        written = j + 1;
                    }
                }
            } else if is_comment(c.state) && c.delim == Delim::None {
                match c.state {
                    State::JsBlockCmt => {
                        // https://es5.github.io/#x7.4:
                        // "Comments behave like white space and are
                        // discarded except that, if a MultiLineComment
                        // contains a line terminator character, then
                        // the entire comment is considered to be a
                        // LineTerminator for purposes of parsing by
                        // the syntactic grammar."
                        if contains_any_js_line_terminator(&s[written..i1]) {
                            b.push(b'\n');
                        } else {
                            b.push(b' ');
                        }
                    }
                    State::CssBlockCmt => b.push(b' '),
                    _ => {}
                }
                written = i1;
            }
            if c.state != c1.state && is_comment(c1.state) && c1.delim == Delim::None {
                // Preserve the portion between written and the comment start.
                let mut cs = i1 - 2;
                if c1.state == State::HtmlCmt || c1.state == State::JsHtmlOpenCmt {
                    // "<!--" instead of "/*" or "//"
                    cs -= 2;
                } else if c1.state == State::JsHtmlCloseCmt {
                    // "-->" instead of "/*" or "//"
                    cs -= 1;
                }
                b.extend_from_slice(&s[written..cs]);
                written = i1;
            }
            if is_in_script_literal(c.state) && contains_special_script_tag(&s[i..i1]) {
                b.extend_from_slice(&s[written..i]);
                b.extend_from_slice(&escape_special_script_tags(&s[i..i1]));
                written = i1;
            }
            if i == i1 && c.state == c1.state {
                panic!(
                    "infinite loop from {} to {} on {:?}..{:?}",
                    c,
                    c1,
                    String::from_utf8_lossy(&s[..i]),
                    String::from_utf8_lossy(&s[i..])
                );
            }
            c = c1;
            i = i1;
        }

        if written != 0 && c.state != State::Error {
            if !is_comment(c.state) || c.delim != Delim::None {
                b.extend_from_slice(&n.text[written..]);
            }
            self.edit_text_node(n, b);
        }
        c
    }

    // Go: escape.go:(*escaper).editActionNode
    /// Records a change to an action pipeline for later commit.
    fn edit_action_node(&mut self, n: &ActionNode, cmds: Vec<String>) {
        if self.action_node_edits.contains_key(&n.id) {
            panic!("node {} shared between templates", n.to_string_lossy());
        }
        self.action_node_edits.insert(n.id, cmds);
    }

    fn edit_action_node_id(&mut self, id: NodeId, cmds: Vec<String>) {
        if self.action_node_edits.contains_key(&id) {
            panic!("node {id:?} shared between templates");
        }
        self.action_node_edits.insert(id, cmds);
    }

    // Go: escape.go:(*escaper).editTemplateNode
    /// Records a change to a {{template}} callee for later commit.
    fn edit_template_node(&mut self, n: &TemplateNode, callee: String) {
        if self.template_node_edits.contains_key(&n.id) {
            panic!("node {} shared between templates", n.to_string_lossy());
        }
        self.template_node_edits.insert(n.id, callee);
    }

    fn edit_template_node_id(&mut self, id: NodeId, callee: String) {
        if self.template_node_edits.contains_key(&id) {
            panic!("node {id:?} shared between templates");
        }
        self.template_node_edits.insert(id, callee);
    }

    // Go: escape.go:(*escaper).editTextNode
    /// Records a change to a text node for later commit.
    fn edit_text_node(&mut self, n: &TextNode, text: Vec<u8>) {
        if self.text_node_edits.contains_key(&n.id) {
            panic!("node {} shared between templates", n.to_string_lossy());
        }
        self.text_node_edits.insert(n.id, text);
    }

    fn edit_text_node_id(&mut self, id: NodeId, text: Vec<u8>) {
        if self.text_node_edits.contains_key(&id) {
            panic!("node {id:?} shared between templates");
        }
        self.text_node_edits.insert(id, text);
    }

    // Go: escape.go:(*escaper).commit
    /// Applies changes to actions and template calls needed to contextually
    /// autoescape content and adds any derived templates to the set.
    pub(crate) fn commit(&mut self, text_ns: &text::Template) {
        if !self.output.is_empty() {
            // Go: e.template(name).Funcs(funcMap) for every escaped name; all
            // but derived templates share text_ns's function maps.
            text_ns.funcs(&esc_func_map());
        }
        // Any template from the name space associated with this escaper can be used
        // to add derived templates to the underlying text/template name space.
        let mut derived: Vec<(&String, &SharedTree)> = self.derived.iter().collect();
        derived.sort_by(|a, b| a.0.cmp(b.0));
        for (name, tree) in derived {
            if text_ns.add_parse_tree(name, tree.clone()).is_err() {
                panic!("error adding derived template");
            }
        }

        // Apply the edits to the trees walked since the last commit (Go
        // mutates the nodes through their pointers).
        let mut edits = Edits {
            action: std::mem::take(&mut self.action_node_edits),
            template: std::mem::take(&mut self.template_node_edits),
            text: std::mem::take(&mut self.text_node_edits),
        };
        let total = edits.action.len() + edits.template.len() + edits.text.len();
        let mut applied = 0usize;
        let mut seen: Vec<SharedTree> = Vec::new();
        let mut names: Vec<&String> = self.called.iter().collect();
        names.sort();
        for name in names {
            let tree = text_ns
                .lookup(name)
                .and_then(|t| t.tree())
                .or_else(|| self.derived.get(name.as_str()).cloned());
            if let Some(tree) = tree {
                if seen.iter().any(|s| s.ptr_eq(&tree)) {
                    continue;
                }
                seen.push(tree.clone());
                applied += tree.update(|t| edits.apply_tree(t));
            }
        }
        if applied < total {
            // Safety net: edits for nodes of trees reached under another
            // name; walk every template of the namespace.
            for t in text_ns.templates() {
                if let Some(tree) = t.tree() {
                    if seen.iter().any(|s| s.ptr_eq(&tree)) {
                        continue;
                    }
                    seen.push(tree.clone());
                    tree.update(|t| edits.apply_tree(t));
                }
            }
        }
        // Reset state that is specific to this commit so that the same changes are
        // not re-applied to the template on subsequent calls to commit.
        self.called = HashSet::new();
    }

    // Go: escape.go:(*escaper).template
    /// Returns the named template given a mangled template name.
    fn template(&self, text_ns: &text::Template, name: &str) -> Option<FoundTemplate> {
        // Any template from the name space associated with this escaper can be used
        // to look up templates in the underlying text/template name space.
        if let Some(t) = text_ns.lookup(name) {
            return Some(FoundTemplate {
                name: name.to_string(),
                tree: t.tree(),
            });
        }
        self.derived.get(name).map(|t| FoundTemplate {
            name: name.to_string(),
            tree: Some(t.clone()),
        })
    }
}

/// The namespace view an escape pass needs (Go reaches it through `e.ns`).
struct Env<'e> {
    text_ns: &'e text::Template,
    set_has: &'e dyn Fn(&str) -> bool,
}

/// Pending node edits, applied by walking trees.
struct Edits {
    action: HashMap<NodeId, Vec<String>>,
    template: HashMap<NodeId, String>,
    text: HashMap<NodeId, Vec<u8>>,
}

impl Edits {
    fn apply_tree(&mut self, t: &mut Tree) -> usize {
        let mut n = 0;
        if let Some(root) = t.root.as_mut() {
            self.apply_list(root, &mut n);
        }
        n
    }

    fn apply_list(&mut self, l: &mut ListNode, n: &mut usize) {
        for node in &mut l.nodes {
            self.apply_node(node, n);
        }
    }

    fn apply_node(&mut self, node: &mut Node, n: &mut usize) {
        match node {
            Node::Action(a) => {
                if let Some(s) = self.action.remove(&a.id) {
                    ensure_pipeline_contains(&mut a.pipe, s);
                    *n += 1;
                }
            }
            Node::Template(t) => {
                if let Some(name) = self.template.remove(&t.id) {
                    t.name = name;
                    *n += 1;
                }
            }
            Node::Text(t) => {
                if let Some(s) = self.text.remove(&t.id) {
                    t.text = s;
                    *n += 1;
                }
            }
            Node::If(b) | Node::Range(b) | Node::With(b) => {
                self.apply_list(&mut b.list, n);
                if let Some(el) = b.else_list.as_mut() {
                    self.apply_list(el, n);
                }
            }
            Node::List(l) => self.apply_list(l, n),
            _ => {}
        }
    }
}

// Go: escape.go:ensurePipelineContains
/// Ensures that the pipeline ends with the commands with the identifiers in
/// s in order. If the pipeline ends with a predefined escaper (i.e. "html"
/// or "urlquery"), merge it with the identifiers in s.
pub(crate) fn ensure_pipeline_contains(p: &mut PipeNode, mut s: Vec<String>) {
    // This method rewrites the pipeline to ensure that the escapers in s
    // are present in the pipeline, merging or replacing escapers as
    // necessary.
    if s.is_empty() {
        // Do not rewrite pipeline if we have no escapers to insert.
        return;
    }
    // Precondition: p.Cmds contains at most one predefined escaper and the
    // escaper will be present at p.Cmds[len(p.Cmds)-1]. This precondition is
    // always true because of the checks in escapeAction.
    let mut pipeline_len = p.cmds.len();
    if pipeline_len > 0 {
        let last = pipeline_len - 1;
        if let Node::Identifier(id_node) = &p.cmds[last].args[0] {
            let esc = id_node.ident.clone();
            if is_predefined_escaper(&esc) {
                // Pipeline ends with a predefined escaper.
                if p.cmds.len() == 1 && p.cmds[last].args.len() > 1 {
                    // Special case: pipeline is of the form {{ esc arg1 arg2 ... argN }},
                    // where esc is the predefined escaper, and arg1...argN are its arguments.
                    // Convert this into the equivalent form
                    // {{ _eval_args_ arg1 arg2 ... argN | esc }}, so that esc can be easily
                    // merged with the escapers in s.
                    let pos = p.cmds[last].args[0].position();
                    p.cmds[last].args[0] =
                        Node::Identifier(IdentifierNode::new("_eval_args_", None, pos));
                    let cmds = std::mem::take(&mut p.cmds);
                    p.cmds = append_cmd(cmds, new_ident_cmd(&esc, p.pos));
                    pipeline_len += 1;
                }
                // If any of the commands in s that we are about to insert is equivalent
                // to the predefined escaper, use the predefined escaper instead.
                let mut dup = false;
                for escaper in s.iter_mut() {
                    if esc_fns_eq(&esc, escaper) {
                        *escaper = esc.clone();
                        dup = true;
                    }
                }
                if dup {
                    // The predefined escaper will already be inserted along with the
                    // escapers in s, so do not copy it to the rewritten pipeline.
                    pipeline_len -= 1;
                }
            }
        }
    }
    // Rewrite the pipeline, creating the escapers in s at the end of the pipeline.
    let mut new_cmds: Vec<CommandNode> = Vec::with_capacity(pipeline_len + s.len());
    let mut inserted_idents: HashSet<String> = HashSet::new();
    for i in 0..pipeline_len {
        let cmd = p.cmds[i].clone();
        if let Node::Identifier(id_node) = &cmd.args[0] {
            inserted_idents.insert(normalize_esc_fn(&id_node.ident).to_string());
        }
        new_cmds.push(cmd);
    }
    for name in &s {
        if !inserted_idents.contains(normalize_esc_fn(name)) {
            // When two templates share an underlying parse tree via the use of
            // AddParseTree and one template is executed after the other, this check
            // ensures that escapers that were already inserted into the pipeline on
            // the first escaping pass do not get inserted again.
            new_cmds = append_cmd(new_cmds, new_ident_cmd(name, p.pos));
        }
    }
    p.cmds = new_cmds;
}

/// Go: `predefinedEscapers` — template predefined escapers that are
/// equivalent to some contextual escapers. Keep in sync with equivEscapers.
fn is_predefined_escaper(name: &str) -> bool {
    name == "html" || name == "urlquery"
}

/// Go: `equivEscapers` — matches contextual escapers to equivalent
/// predefined template escapers.
fn equiv_escaper(e: &str) -> Option<&'static str> {
    match e {
        // The following pairs of HTML escapers provide equivalent security
        // guarantees, since they all escape '\000', '\'', '"', '&', '<', and '>'.
        "_html_template_attrescaper"
        | "_html_template_htmlescaper"
        | "_html_template_rcdataescaper" => Some("html"),
        // These two URL escapers produce URLs safe for embedding in a URL query by
        // percent-encoding all the reserved characters specified in RFC 3986 Section
        // 2.2
        "_html_template_urlescaper" => Some("urlquery"),
        // These two functions are not actually equivalent; urlquery is stricter as it
        // escapes reserved characters (e.g. '#'), while _html_template_urlnormalizer
        // does not. It is therefore only safe to replace _html_template_urlnormalizer
        // with urlquery (this happens in ensurePipelineContains), but not the otherI've
        // way around. We keep this entry around to preserve the behavior of templates
        // written before Go 1.9, which might depend on this substitution taking place.
        "_html_template_urlnormalizer" => Some("urlquery"),
        _ => None,
    }
}

// Go: escape.go:escFnsEq
/// Reports whether the two escaping functions are equivalent.
fn esc_fns_eq(a: &str, b: &str) -> bool {
    normalize_esc_fn(a) == normalize_esc_fn(b)
}

// Go: escape.go:normalizeEscFn
/// normalizeEscFn(a) is equal to normalizeEscFn(b) for any pair of names of
/// escaper functions a and b that are equivalent.
fn normalize_esc_fn(e: &str) -> &str {
    equiv_escaper(e).unwrap_or(e)
}

// Go: escape.go:redundantFuncs
/// redundantFuncs[a][b] implies that funcMap[b](funcMap[a](x)) == funcMap[a](x)
/// for all x.
fn redundant_funcs(a: &str, b: &str) -> bool {
    match a {
        "_html_template_commentescaper" => {
            b == "_html_template_attrescaper" || b == "_html_template_htmlescaper"
        }
        "_html_template_cssescaper" => b == "_html_template_attrescaper",
        "_html_template_jsregexpescaper" => b == "_html_template_attrescaper",
        "_html_template_jsstrescaper" => b == "_html_template_attrescaper",
        "_html_template_jstmpllitescaper" => b == "_html_template_attrescaper",
        "_html_template_urlescaper" => b == "_html_template_urlnormalizer",
        _ => false,
    }
}

// Go: escape.go:appendCmd
/// Appends the given command to the end of the command pipeline unless it
/// is redundant with the last command.
fn append_cmd(mut cmds: Vec<CommandNode>, cmd: CommandNode) -> Vec<CommandNode> {
    if let Some(last) = cmds.last() {
        if let (Node::Identifier(last), Node::Identifier(next)) = (&last.args[0], &cmd.args[0]) {
            if redundant_funcs(&last.ident, &next.ident) {
                return cmds;
            }
        }
    }
    cmds.push(cmd);
    cmds
}

// Go: escape.go:newIdentCmd
/// Produces a command containing a single identifier node.
fn new_ident_cmd(identifier: &str, pos: Pos) -> CommandNode {
    CommandNode {
        pos: 0,
        tr: None,
        args: vec![Node::Identifier(IdentifierNode::new(identifier, None, pos))], // TODO: SetTree.
    }
}

// Go: escape.go:nudge
/// Returns the context that would result from following empty string
/// transitions from the input context. For example, parsing:
///
///     `<a href=`
///
/// will end in context{stateBeforeValue, attrURL}, but parsing one extra rune:
///
///     `<a href=x`
///
/// will end in context{stateURL, delimSpaceOrTagEnd, ...}.
/// There are two transitions that happen when the 'x' is seen:
/// (1) Transition from a before-value state to a start-of-value state without
///     consuming any character.
/// (2) Consume 'x' and transition past the first value character.
/// In this case, nudging produces the context after (1) happens.
pub(crate) fn nudge(mut c: Context) -> Context {
    match c.state {
        State::Tag => {
            // In `<foo {{.}}`, the action should emit an attribute.
            c.state = State::AttrName;
        }
        State::BeforeValue => {
            // In `<foo bar={{.}}`, the action is an undelimited value.
            c.state = attr_start_state(c.attr);
            c.delim = Delim::SpaceOrTagEnd;
            c.attr = Attr::None;
        }
        State::AfterName => {
            // In `<foo bar {{.}}`, the action is an attribute name.
            c.state = State::AttrName;
            c.attr = Attr::None;
        }
        _ => {}
    }
    c
}

// Go: escape.go:join
/// Joins the two contexts of a branch template node. The result is an error
/// context if either of the input contexts are error contexts, or if the
/// input contexts differ.
pub(crate) fn join(
    a: Context,
    b: Context,
    node: Option<&dyn NodeLike>,
    node_name: &str,
) -> Context {
    if a.state == State::Error {
        return a;
    }
    if b.state == State::Error {
        return b;
    }
    if a.state == State::Dead {
        return b;
    }
    if b.state == State::Dead {
        return a;
    }
    if a.eq(&b) {
        return a;
    }

    let mut c = a.clone();
    c.url_part = b.url_part;
    if c.eq(&b) {
        // The contexts differ only by urlPart.
        c.url_part = UrlPart::Unknown;
        return c;
    }

    let mut c = a.clone();
    c.js_ctx = b.js_ctx;
    if c.eq(&b) {
        // The contexts differ only by jsCtx.
        c.js_ctx = JsCtx::Unknown;
        return c;
    }

    // Allow a nudged context to join with an unnudged one.
    // This means that
    //   <p title={{if .C}}{{.}}{{end}}
    // ends in an unquoted value state even though the else branch
    // ends in stateBeforeValue.
    let (c, d) = (nudge(a.clone()), nudge(b.clone()));
    if !(c.eq(&a) && d.eq(&b)) {
        let e = join(c, d, node, node_name);
        if e.state != State::Error {
            return e;
        }
    }

    Context {
        state: State::Error,
        err: Some(Arc::new(errorf(
            ErrorCode::ErrBranchEnd,
            node,
            0,
            format!("{{{{{node_name}}}}} branches end in different contexts: {a}, {b}"),
        ))),
        ..Default::default()
    }
}

// Go: escape.go:joinRange
fn join_range(mut c0: Context, rc: &Arc<Mutex<RangeContext>>) -> Context {
    // Merge contexts at break and continue statements into overall body context.
    // In theory we could treat breaks differently from continues, but for now it is
    // enough to treat them both as going back to the start of the loop (which may then stop).
    let rc = rc.lock().unwrap_or_else(|e| e.into_inner());
    for c in &rc.breaks {
        c0 = join(
            c0,
            c.clone(),
            c.n.as_ref().map(|n| n as &dyn NodeLike),
            "range",
        );
        if c0.state == State::Error {
            if let (Some(e), Some(Node::Break(b))) = (&c0.err, &c.n) {
                let mut e2 = (**e).clone();
                e2.line = b.line;
                e2.description = format!("at range loop break: {}", e2.description);
                c0.err = Some(Arc::new(e2));
            }
            return c0;
        }
    }
    for c in &rc.continues {
        c0 = join(
            c0,
            c.clone(),
            c.n.as_ref().map(|n| n as &dyn NodeLike),
            "range",
        );
        if c0.state == State::Error {
            if let (Some(e), Some(Node::Continue(b))) = (&c0.err, &c.n) {
                let mut e2 = (**e).clone();
                e2.line = b.line;
                e2.description = format!("at range loop continue: {}", e2.description);
                c0.err = Some(Arc::new(e2));
            }
            return c0;
        }
    }
    c0
}

const DOCTYPE_BYTES: &[u8] = b"<!DOCTYPE";

/// Go `bytes.HasPrefix(bytes.ToUpper(s), prefix)` for an ASCII prefix.
/// `bytes.ToUpper` maps runes with Unicode simple case mapping; only ASCII
/// letters can map to the ASCII letters of `<!DOCTYPE` except U+0131
/// (dotless i, no) and U+017F (long s, uppercases to 'S'), which is handled
/// by uppercasing the whole prefix-length region rune by rune.
fn has_prefix_upper(s: &[u8], prefix: &[u8]) -> bool {
    let mut out: Vec<u8> = Vec::with_capacity(prefix.len());
    let mut i = 0;
    while out.len() < prefix.len() && i < s.len() {
        let (r, w) = go_unicode::utf8::decode_rune(&s[i..]);
        let u = if r == go_unicode::utf8::RUNE_ERROR && w == 1 {
            // bytes.ToUpper keeps invalid bytes as the replacement char?
            // Go's bytes.Map writes RuneError (3 bytes) for invalid input.
            r
        } else {
            go_unicode::to_upper(r)
        };
        let mut buf = [0u8; 4];
        let n = go_unicode::utf8::encode_rune(&mut buf, u);
        out.extend_from_slice(&buf[..n]);
        i += w;
    }
    out.starts_with(prefix)
}

/// `bytes.ContainsAny(s, "\n\r  ")`.
fn contains_any_js_line_terminator(s: &[u8]) -> bool {
    let mut i = 0;
    while i < s.len() {
        let (r, w) = go_unicode::utf8::decode_rune(&s[i..]);
        if r == '\n' as i32 || r == '\r' as i32 || r == 0x2028 || r == 0x2029 {
            return true;
        }
        i += w;
    }
    false
}

// Go: escape.go:contextAfterText
/// Starts in context c, consumes some tokens from the front of s, then
/// returns the context after those tokens and the unprocessed suffix.
pub(crate) fn context_after_text(c: Context, s: &[u8]) -> (Context, usize) {
    if c.delim == Delim::None {
        let (c1, i) = t_special_tag_end(c.clone(), s);
        if i == 0 {
            // A special end tag (`</script>`) has been seen and
            // all content preceding it has been consumed.
            return (c1, 0);
        }
        // Consider all content up to any end tag.
        return transition(c, &s[..i]);
    }

    // We are at the beginning of an attribute value.

    let ends = delim_ends(c.delim);
    let mut i = s.iter().position(|b| ends.contains(b)).unwrap_or(s.len());
    if c.delim == Delim::SpaceOrTagEnd {
        // https://www.w3.org/TR/html5/syntax.html#attribute-value-(unquoted)-state
        // lists the runes below as error characters.
        // Error out because HTML parsers may differ on whether
        // "<a id= onclick=f("     ends inside id's or onclick's value,
        // "<a class=`foo "        ends inside a value,
        // "<a style=font:'Arial'" needs open-quote fixup.
        // IE treats '`' as a quotation character.
        if let Some(j) = s[..i].iter().position(|b| b"\"'<=`".contains(b)) {
            return (
                Context {
                    state: State::Error,
                    err: Some(Arc::new(errorf(
                        ErrorCode::ErrBadHTML,
                        None,
                        0,
                        format!(
                            "{} in unquoted attr: {}",
                            go_strconv::quote(&s[j..j + 1]),
                            go_strconv::quote(&s[..i])
                        ),
                    ))),
                    ..Default::default()
                },
                s.len(),
            );
        }
    }
    if i == s.len() {
        // Remain inside the attribute.
        // Decode the value so non-HTML rules can easily handle
        //     <button onclick="alert(&quot;Hi!&quot;)">
        // without having to entity decode token boundaries.
        let mut c = c;
        let u = go_html::unescape_string_bytes(s);
        let mut u: &[u8] = &u;
        while !u.is_empty() {
            let (c1, i1) = transition(c, u);
            c = c1;
            u = &u[i1..];
        }
        return (c, s.len());
    }

    let mut element = c.element;

    // If this is a non-JS "type" attribute inside "script" tag, do not treat the contents as JS.
    if c.state == State::Attr
        && c.element == super::context::Element::Script
        && c.attr == Attr::ScriptType
        && !is_js_type(&s[..i])
    {
        element = super::context::Element::None;
    }

    if c.delim != Delim::SpaceOrTagEnd {
        // Consume any quote.
        i += 1;
    }
    // On exiting an attribute, we discard all state information
    // except the state and element.
    (
        Context {
            state: State::Tag,
            element,
            ..Default::default()
        },
        i,
    )
}
