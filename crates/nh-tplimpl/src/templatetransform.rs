//! Port of `tpl/tplimpl/templatetransform.go`.
//!
//! Owner: Wave B task T13 (tplimpl).

//! Go `templatetransform.go`: AST rewrites on gotemplate parse trees — partial `return` wrapping
//! (`{{ $_hugo_dot := $ }}{{ $ := .Arg }}{{ range (slice .Arg) }}...{{ $_hugo_dot.Set (...) }}{{ end }}`),
//! shortcode `$_hugo_config`, `.Inner` detection, `templates.Defer`. Operates directly on the
//! gotemplate crate's parse nodes (engine.rs exposes them).
//!
//! Go mutates the `*parse.Tree` nodes in place. Here each tree is transformed on a copy of its
//! root that is written back through the shared tree handle (`SharedTree::update`), so every
//! template holding the tree (namespaces, clones, aliases) sees the result, as in Go.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;

use crate::category::Category;
use crate::engine::parse::{CommandNode, ListNode, Node, NodeLike, PipeNode, SharedTree};
use crate::engine::{Template, TextTemplate};
use crate::template_info::{ParseConfig, ParseInfo, default_parse_info};
use crate::templatestore::TemplInfo;

/// Go: `lookupFn func(name string, in *TemplInfo) *TemplInfo` (only the template is used).
pub(crate) type LookupFn<'a> = &'a dyn Fn(&str, &TemplInfo) -> Option<Template>;

/// Go: `templateTransformContext`.
pub(crate) struct TemplateTransformContext<'a> {
    visited: HashMap<String, bool>,
    template_not_found: HashMap<String, bool>,
    pub(crate) defer_nodes: BTreeMap<String, ListNode>,
    lookup_fn: LookupFn<'a>,

    /// The last error encountered.
    err: Option<Error>,

    /// Set when we're done checking for config header.
    config_checked: bool,

    t: &'a Arc<TemplInfo>,
    /// Go `c.t.ParseInfo`, written back to the template at the end.
    parse_info: ParseInfo,

    /// Store away the return node in partials (Go: a pointer to the command, which is removed
    /// from its pipeline and later placed in the wrapper). `return_node_set` is Go's
    /// `c.returnNode != nil`; the node itself is recorded after its arguments are transformed.
    return_node_set: bool,
    return_node: Option<CommandNode>,

    /// Trees whose transformation is in progress (their root is a working copy).
    in_progress: Vec<SharedTree>,
}

impl<'a> TemplateTransformContext<'a> {
    // Go: tpl/tplimpl/templatetransform.go:getIfNotVisited
    fn get_if_not_visited(&mut self, name: &str) -> Option<Template> {
        if self.visited.get(name).copied().unwrap_or(false) {
            return None;
        }
        self.visited.insert(name.to_string(), true);
        let templ = (self.lookup_fn)(name, self.t);
        if templ.is_none() {
            // This may be a inline template defined outside of this file
            // and not yet parsed. Unusual, but it happens.
            // Store the name to try again later.
            self.template_not_found.insert(name.to_string(), true);
        }
        templ
    }

    fn category(&self) -> Option<Category> {
        self.t.category
    }

    // Go: tpl/tplimpl/templatetransform.go:wrapInPartialReturnWrapper
    /// Copies and modifies the parsed nodes of a predefined partial return wrapper to insert
    /// those of a user-defined partial.
    fn wrap_in_partial_return_wrapper(&self, n: ListNode) -> ListNode {
        let mut wrapper = partial_return_wrapper().copy_list();
        let Node::Range(range_node) = &mut wrapper.nodes[2] else {
            panic!("partialReturnWrapper: node 2 is not a range");
        };
        let mut retn = range_node.list.nodes[0].clone();
        {
            let Node::Action(action) = &mut retn else {
                panic!("partialReturnWrapper: not an action");
            };
            let set_cmd = &mut action.pipe.cmds[0];
            let Node::Pipe(set_pipe) = &mut set_cmd.args[1] else {
                panic!("partialReturnWrapper: not a pipe");
            };
            // Replace PLACEHOLDER with the real return value.
            // Note that this is a PipeNode, so it will be wrapped in parens.
            set_pipe.cmds = vec![self.return_node.clone().expect("return node")];
        }
        let mut nodes = n.nodes;
        nodes.push(retn);
        range_node.list.nodes = nodes;

        wrapper
    }

    /// Go: `applyTransformations` on a `*parse.ListNode` (nil-safe).
    fn apply_list(&mut self, x: &mut ListNode) {
        for n in x.nodes.iter_mut() {
            self.apply_transformations(n);
        }
    }

    /// Go: `applyTransformations` on a `*parse.PipeNode`.
    fn apply_pipe(&mut self, x: &mut PipeNode) {
        self.collect_config(x);
        // Go: `for i, cmd := range x.Cmds { ...; if !keep { x.Cmds = slices.Delete(x.Cmds, i,
        // i+1) } }` — the range is over the original slice header, so after a deletion the
        // command shifted into index i is skipped, and the trailing (nil'ed) slots are no-ops.
        let n = x.cmds.len();
        let mut i = 0;
        while i < n {
            if i >= x.cmds.len() {
                // A nil *parse.CommandNode: keep.
                i += 1;
                continue;
            }
            let keep = self.apply_command(&mut x.cmds[i]);
            if !keep {
                x.cmds.remove(i);
            }
            i += 1;
        }
    }

    /// Go: `applyTransformations` on a `*parse.CommandNode`.
    fn apply_command(&mut self, x: &mut CommandNode) -> bool {
        self.collect_inner(x);
        let keep = self.collect_return_node(x);

        for elem in x.args.iter_mut() {
            if let Node::Pipe(an) = elem {
                self.apply_pipe(an);
            }
        }
        if !keep {
            // Go keeps a pointer to x; its arguments were transformed above.
            self.return_node = Some(x.clone());
        }
        keep
    }

    /// Transforms the tree of another template (Go: `applyTransformationsToNodes(
    /// getParseTree(subTempl.Template).Root)`).
    fn apply_sub_tree(&mut self, tree: SharedTree) {
        if self.in_progress.iter().any(|t| t.ptr_eq(&tree)) {
            // Go would transform the same nodes again (re-entrantly); see PORTING.md.
            return;
        }
        self.in_progress.push(tree.clone());
        let mut root = tree.get().root.clone();
        if let Some(r) = root.as_mut() {
            self.apply_list(r);
        }
        tree.update(|t| t.root = root);
        self.in_progress.pop();
    }

    /// applyTransformations do 2 things:
    /// 1) Parses partial return statement.
    /// 2) Tracks template (partial) dependencies and some other info.
    // Go: tpl/tplimpl/templatetransform.go:applyTransformations
    fn apply_transformations(&mut self, n: &mut Node) -> bool {
        match n {
            Node::List(x) => self.apply_list(x),
            Node::Action(x) => self.apply_pipe(&mut x.pipe),
            Node::If(x) | Node::Range(x) => {
                self.apply_pipe(&mut x.pipe);
                self.apply_list(&mut x.list);
                if let Some(e) = x.else_list.as_mut() {
                    self.apply_list(e);
                }
            }
            Node::With(x) => {
                self.handle_defer(x);
                self.apply_pipe(&mut x.pipe);
                self.apply_list(&mut x.list);
                if let Some(e) = x.else_list.as_mut() {
                    self.apply_list(e);
                }
            }
            Node::Template(x) => {
                let name = x.name.clone();
                if let Some(sub_templ) = self.get_if_not_visited(&name)
                    && let Some(tree) = sub_templ.tree()
                {
                    self.apply_sub_tree(tree);
                }
            }
            Node::Pipe(x) => self.apply_pipe(x),
            Node::Command(x) => return self.apply_command(x),
            _ => {}
        }
        true
    }

    // Go: tpl/tplimpl/templatetransform.go:handleDefer
    fn handle_defer(&mut self, with_node: &mut crate::engine::parse::BranchNode) {
        if with_node.pipe.cmds.len() != 1 {
            return;
        }
        let cmd = &mut with_node.pipe.cmds[0];
        if cmd.args.len() != 1 {
            return;
        }
        let Node::Pipe(p) = &mut cmd.args[0] else {
            return;
        };

        if p.cmds.len() != 1 {
            return;
        }

        let cmd = &mut p.cmds[0];

        if cmd.args.len() != 2 {
            return;
        }

        {
            let Node::Chain(id) = &cmd.args[0] else {
                return;
            };
            if id.field.len() != 1 || id.field[0] != "Defer" {
                return;
            }
            match &*id.node {
                Node::Identifier(id2) if id2.ident == "templates" => {}
                _ => return,
            }
        }

        let defer_arg = cmd.args[1].clone();
        cmd.args.truncate(1);

        let mut l = do_defer().copy_list();

        let inner = with_node.list.copy_list();
        let s = inner.to_bytes();
        if memchr_contains(&s, b"resources.PostProcess") {
            self.err = Some(Error::new(
                "resources.PostProcess cannot be used in a deferred template",
            ));
            return;
        }
        let inner_hash = nh_common::hashing::xxhash_from_string_hex_encoded(&s);
        let deferred_id = format!(
            "{}{}",
            nh_tpl::template::HUGO_DEFERRED_TEMPLATE_PREFIX,
            inner_hash
        );

        self.defer_nodes.insert(deferred_id.clone(), inner);

        {
            let Node::Action(n) = &mut l.nodes[0] else {
                panic!("doDefer: not an action");
            };
            let args = &mut n.pipe.cmds[0].args;
            if let Node::Pipe(p1) = &mut args[1]
                && let Node::String(sn) = &mut p1.cmds[0].args[0]
            {
                // Go sets only Text (Quoted, which String() prints, keeps "PLACEHOLDER1").
                sn.text = deferred_id.into_bytes();
            }
            args[2] = defer_arg;
        }
        with_node.list = l;
    }

    // Go: tpl/tplimpl/templatetransform.go:hasIdent
    fn has_ident(&self, idents: &[String], ident: &str) -> bool {
        idents.iter().any(|i| i == ident)
    }

    /// collectConfig collects and parses any leading template config variable declaration.
    /// This will be the first PipeNode in the template, and will be a variable declaration
    /// on the form:
    ///
    /// `{{ $_hugo_config:= `{ "version": 1 }` }}`
    // Go: tpl/tplimpl/templatetransform.go:collectConfig
    fn collect_config(&mut self, n: &PipeNode) {
        if self.category() != Some(Category::Shortcode) {
            return;
        }
        if self.config_checked {
            return;
        }
        self.config_checked = true;

        if n.decl.len() != 1 || n.cmds.len() != 1 {
            // This cannot be a config declaration
            return;
        }

        let v = &n.decl[0];

        if v.ident.is_empty() || v.ident[0] != "$_hugo_config" {
            return;
        }

        let cmd = &n.cmds[0];

        if cmd.args.is_empty() {
            return;
        }

        if let Node::String(s) = &cmd.args[0] {
            let err_msg = |e: &dyn std::fmt::Display| {
                Error::new(format!("failed to decode $_hugo_config in template: {e}"))
            };
            let m = match nh_common::maps::maps::to_string_map_e(&Value::string(s.text.clone())) {
                Ok(m) => m,
                Err(e) => {
                    self.err = Some(err_msg(&e));
                    return;
                }
            };
            if let Err(e) = weak_decode_parse_config(&m, &mut self.parse_info.config) {
                self.err = Some(err_msg(&e));
            }
        }
    }

    /// collectInner determines if the given CommandNode represents a
    /// shortcode call to its .Inner.
    // Go: tpl/tplimpl/templatetransform.go:collectInner
    fn collect_inner(&mut self, n: &CommandNode) {
        if self.category() != Some(Category::Shortcode) {
            return;
        }
        if self.parse_info.is_inner || n.args.is_empty() {
            return;
        }

        for arg in &n.args {
            let idents: &[String] = match arg {
                Node::Field(nt) => &nt.ident,
                Node::Variable(nt) => &nt.ident,
                _ => &[],
            };

            if self.has_ident(idents, "Inner") || self.has_ident(idents, "InnerDeindent") {
                self.parse_info.is_inner = true;
                break;
            }
        }
    }

    // Go: tpl/tplimpl/templatetransform.go:collectReturnNode
    fn collect_return_node(&mut self, n: &mut CommandNode) -> bool {
        if self.category() != Some(Category::Partial) || self.return_node_set {
            return true;
        }

        if n.args.len() < 2 {
            return true;
        }

        match &n.args[0] {
            Node::Identifier(ident) if ident.ident == "return" => {}
            _ => return true,
        }

        self.return_node_set = true;
        // Remove the "return" identifiers
        n.args.remove(0);

        false
    }
}

fn memchr_contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Go: `mapstructure.WeakDecode(m, &c.t.ParseInfo.Config)` for `ParseConfig{Version int}`: the
/// field matches a key case-insensitively (an exact-case key wins); weakly typed input (bools
/// as 0/1, floats truncated, numeric strings parsed, an empty string as 0).
fn weak_decode_parse_config(
    m: &go_value::Map,
    cfg: &mut ParseConfig,
) -> std::result::Result<(), String> {
    // mapstructure: exact field name first, then a case-insensitive match (map order; with a
    // single field "Version" the candidates are the keys that EqualFold "version").
    let mut v = m.get(b"Version");
    if v.is_none() {
        for (k, val) in m.entries.iter() {
            if go_unicode::strings::equal_fold(k.as_bytes(), b"Version") {
                v = Some(val);
                break;
            }
        }
    }
    let Some(v) = v else {
        return Ok(());
    };
    let n = match v {
        Value::Invalid => return Ok(()),
        Value::Int(i, _) => *i,
        Value::Uint(u, _) => *u as i64,
        Value::Float(f, _) => *f as i64,
        Value::Bool(b) => i64::from(*b),
        Value::String(s) => {
            if s.is_empty() {
                0
            } else {
                match go_strconv::parse_int(s.as_bytes(), 0, 64) {
                    Ok(i) => i,
                    Err(e) => {
                        return Err(format!(
                            "1 error(s) decoding:\n\n* cannot parse 'Version' as int: {e}"
                        ));
                    }
                }
            }
        }
        other => {
            return Err(format!(
                "1 error(s) decoding:\n\n* 'Version' expected type 'int', got unconvertible type '{}', value: '{}'",
                other.go_type_name(),
                String::from_utf8_lossy(&go_fmt::sprint(std::slice::from_ref(other)))
            ));
        }
    };
    cfg.version = n;
    Ok(())
}

// Go: tpl/tplimpl/templatetransform.go:newTemplateTransformContext
fn new_template_transform_context<'a>(
    t: &'a Arc<TemplInfo>,
    lookup_fn: LookupFn<'a>,
) -> TemplateTransformContext<'a> {
    TemplateTransformContext {
        t,
        lookup_fn,
        visited: HashMap::new(),
        template_not_found: HashMap::new(),
        defer_nodes: BTreeMap::new(),
        err: None,
        config_checked: false,
        parse_info: ParseInfo::default(),
        return_node_set: false,
        return_node: None,
        in_progress: Vec::new(),
    }
}

/// Go: `applyTemplateTransformers(t, lookupFn)`.
// Go: tpl/tplimpl/templatetransform.go:applyTemplateTransformers
pub(crate) fn apply_template_transformers<'a>(
    t: &'a Arc<TemplInfo>,
    lookup_fn: LookupFn<'a>,
) -> Result<TemplateTransformContext<'a>> {
    let mut c = new_template_transform_context(t, lookup_fn);
    c.parse_info = default_parse_info();
    let tree = getparse_tree(t).unwrap_or_else(|| panic!("template {} not parsed", t.string()));

    c.in_progress.push(tree.clone());
    let mut root = tree.get().root.clone();
    if let Some(r) = root.as_mut() {
        c.apply_list(r);
    }
    c.in_progress.pop();

    let err = c.err.clone();
    if err.is_none() && c.return_node.is_some() {
        // This is a partial with a return statement.
        c.parse_info.has_return = true;
        root = root.map(|r| c.wrap_in_partial_return_wrapper(r));
    }
    tree.update(|tr| tr.root = root);
    t.state_mut().parse_info = c.parse_info.clone();

    match err {
        Some(e) => Err(e),
        None => Ok(c),
    }
}

/// Go: `applyTemplateTransformers` — kept as the skeleton's documentation anchor.
// Go: tpl/tplimpl/templatetransform.go:applyTemplateTransformers
pub fn apply_template_transformers_doc() {}

/// Go: `getParseTree(templ)` of a template info's template.
// Go: tpl/tplimpl/templatetransform.go:getParseTree
fn getparse_tree(t: &TemplInfo) -> Option<SharedTree> {
    t.template().and_then(|templ| templ.tree())
}

// Go: tpl/tplimpl/templatetransform.go:partialReturnWrapperTempl
/// We parse this template and modify the nodes in order to assign the return value of a partial
/// to a contextWrapper via Set. We use "range" over a one-element slice so we can shift dot to
/// the partial's argument, Arg, while allowing Arg to be falsy.
const PARTIAL_RETURN_WRAPPER_TEMPL: &str = r#"{{ $_hugo_dot := $ }}{{ $ := .Arg }}{{ range (slice .Arg) }}{{ $_hugo_dot.Set ("PLACEHOLDER") }}{{ end }}"#;

// Go: tpl/tplimpl/templatetransform.go:doDeferTempl
const DO_DEFER_TEMPL: &str = r#"{{ doDefer ("PLACEHOLDER1") ("PLACEHOLDER2") }}"#;

/// Go: `partialReturnWrapper` (parsed in `init()`).
// Go: tpl/tplimpl/templatetransform.go:init
fn partial_return_wrapper() -> &'static ListNode {
    static W: OnceLock<ListNode> = OnceLock::new();
    W.get_or_init(|| {
        let templ = TextTemplate::new("")
            .parse(PARTIAL_RETURN_WRAPPER_TEMPL.as_bytes())
            .unwrap_or_else(|e| panic!("{e}"));
        templ
            .tree()
            .and_then(|t| t.get().root.clone())
            .expect("partialReturnWrapper root")
    })
}

/// Go: `doDefer` (parsed in `init()` with a `doDefer` func).
// Go: tpl/tplimpl/templatetransform.go:init
fn do_defer() -> &'static ListNode {
    static W: OnceLock<ListNode> = OnceLock::new();
    W.get_or_init(|| {
        let t = TextTemplate::new("");
        t.func_names(["doDefer"]);
        let templ = t
            .parse(DO_DEFER_TEMPL.as_bytes())
            .unwrap_or_else(|e| panic!("{e}"));
        templ
            .tree()
            .and_then(|t| t.get().root.clone())
            .expect("doDefer root")
    })
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/templatetransform.go (352 lines; 13/13 funcs executed)
//   types: templateTransformContext
// OK L38-52: (c templateTransformContext) getIfNotVisited(name string) *TemplInfo
// OK L54-65: newTemplateTransformContext( t *TemplInfo, lookupFn func(name string, in *TemplInfo) *TemplInfo, ) *templateTransformContext
// OK L67-91: applyTemplateTransformers( t *TemplInfo, lookupFn func(name string, in *TemplInfo) *TemplInfo, ) (*templateTransformContext, error)
// OK L93-98: getParseTree(templ tpl.Template) *parse.Tree
// OK L115-127: init()
// OK L131-143: (c *templateTransformContext) wrapInPartialReturnWrapper(n *parse.ListNode) *parse.ListNode
// OK L148-194: (c *templateTransformContext) applyTransformations(n parse.Node) (bool, error)
// OK L196-251: (c *templateTransformContext) handleDefer(withNode *parse.WithNode)
// OK L253-257: (c *templateTransformContext) applyTransformationsToNodes(nodes ...parse.Node)
// OK L259-261: (c *templateTransformContext) hasIdent(idents []string, ident string) bool
// OK L268-305: (c *templateTransformContext) collectConfig(n *parse.PipeNode)
// OK L309-331: (c *templateTransformContext) collectInner(n *parse.CommandNode)
// OK L333-352: (c *templateTransformContext) collectReturnNode(n *parse.CommandNode) bool
// ---------------------------------------------------------------------------
