//! Go: tpl/internal/go_templates/texttemplate/exec.go, merged with the
//! Hugo replacements in hugo_template.go (`state`, `evalFunction`,
//! `evalField`, `evalCall`, `isTrue`).
//!
//! Go panics with `ExecError` (and `walkBreak`/`walkContinue`) and recovers
//! at the top; here every evaluation returns `Result<_, Ctrl>`.
//! Reflection is replaced by the value model: see PORTING.md, "Host
//! contract", for the resolution rules (they are the contract clauses
//! C2–C12 implemented below).

use std::borrow::Cow;
use std::io::Write;
use std::sync::Arc;

use go_value::{HostCtx, Kind, MapType, NilKind, Object, SliceType, Value, typed_nil_kind};

use crate::error::{Error, ExecError};
use crate::parse::*;

use super::funcs::{
    self, ArgType, Builtin, Func, FuncValue, indirect, indirect_interface, sprint_v, type_name,
};
use super::hugo::{ExecHelper, TryError, TryValue};
use super::template::{MissingKeyAction, Template};

/// Go: `maxExecDepth` — the maximum stack depth of templates within
/// templates.
pub const MAX_EXEC_DEPTH: usize = 100000;

/// Non-local control flow (Go panics).
pub(crate) enum Ctrl {
    /// Go `walkBreak`.
    Break,
    /// Go `walkContinue`.
    Continue,
    /// Go `ExecError` / `writeError`.
    Err(Error),
}

type R<T> = Result<T, Ctrl>;

/// Go: `variable` — holds the dynamic value of a variable such as $, $x etc.
struct Variable {
    name: String,
    value: Value,
}

/// What a command calls (Go: the `fun reflect.Value` of `evalCall`).
enum Callee {
    Builtin(Builtin),
    Func(Func),
    /// A method, bound to its receiver.
    Method(Value),
}

/// An empty struct standing in for Go's `missingVal` when a pipeline has
/// no commands (possible only after AST rewrites); prints `{}`.
struct MissingVal;

impl Object for MissingVal {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("template.missingValType")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(Vec::new())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// The struct a non-nil `Kind::Ptr` object points to (Go `v.Elem()`),
/// for printing: `{a b}` instead of `&{a b}`.
pub(crate) struct StructView(pub(crate) Arc<dyn Object>);

impl Object for StructView {
    fn type_name(&self) -> Cow<'_, str> {
        let t = self.0.type_name();
        match t.strip_prefix('*') {
            Some(s) => Cow::Owned(s.to_string()),
            None => t,
        }
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.0.field(name)
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        self.0.struct_fields()
    }
    fn identity(&self) -> usize {
        self.0.identity()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go `indirect` for printing: a non-nil pointer object without `String`
/// or `Error` methods is replaced by the struct it points to.
pub(crate) fn deref_ptr_object(v: &Value) -> Value {
    if let Value::Object(o) = v {
        if o.kind() == Kind::Ptr && o.go_string().is_none() && o.go_error().is_none() {
            return Value::Object(Arc::new(StructView(o.clone())));
        }
    }
    v.clone()
}

// Go: exec.go:printableValue
/// Returns the, possibly indirected, interface value inside v that is best
/// for a call to formatted printer.
pub(crate) fn printable_value(v: &Value) -> Result<Value, ()> {
    match v {
        Value::Invalid => Ok(Value::string("<no value>")),
        Value::TypedNil(t) => match typed_nil_kind(t) {
            NilKind::Func | NilKind::Chan => Err(()),
            _ => Ok(v.clone()),
        },
        Value::Object(o) if o.kind() == Kind::Func => Err(()),
        _ => Ok(deref_ptr_object(v)),
    }
}

/// Go: `state` — the state of an execution. It's not part of the template
/// so that multiple executions of the same template can execute in
/// parallel.
pub(crate) struct State<'a> {
    tmpl: Template,
    /// The executing template's tree (Go: `s.tmpl.Tree`, the fallback of
    /// `ErrorContext`).
    tree: &'a Tree,
    /// Added for Hugo. The original data context.
    ctx: HostCtx<'a>,
    /// Added for Hugo.
    helper: &'a dyn ExecHelper,
    wr: &'a mut dyn Write,
    /// current node, for errors
    node: Option<&'a dyn NodeLike>,
    /// push-down stack of variable values.
    vars: Vec<Variable>,
    /// the height of the stack of executing templates.
    depth: usize,
}

/// Runs `tmpl` (Go: `executeWithState` preceded by the state setup of
/// `Execute`/`ExecuteWithContext`).
pub(crate) fn execute_template(
    tmpl: &Template,
    ctx: HostCtx<'_>,
    helper: &dyn ExecHelper,
    wr: &mut dyn Write,
    data: &Value,
) -> Result<(), Error> {
    // Go: `reflect.ValueOf(data)` of the `data any` parameter — a nil of an
    // interface type (e.g. a nil `error`) passed as `any` is an untyped nil.
    let data = &indirect_interface(data);
    let snapshot = tmpl.tree().map(|t| t.get());
    let empty;
    let tree: &Tree = match &snapshot {
        Some(t) => t,
        None => {
            empty = Tree::new(tmpl.name());
            &empty
        }
    };
    let mut s = State {
        tmpl: tmpl.clone(),
        tree,
        ctx,
        helper,
        wr,
        node: None,
        vars: vec![Variable {
            name: "$".to_string(),
            value: data.clone(),
        }],
        depth: 0,
    };
    let Some(root) = tree.root.as_ref().filter(|_| snapshot.is_some()) else {
        return Err(unwrap_ctrl(s.errorf(format!(
            "{} is an incomplete or empty template",
            go_strconv::quote(tmpl.name())
        ))));
    };
    match s.walk_list(data, root) {
        Ok(()) => Ok(()),
        Err(c) => Err(unwrap_ctrl(c)),
    }
}

fn unwrap_ctrl(c: Ctrl) -> Error {
    match c {
        Ctrl::Err(e) => e,
        // Unreachable: the parser rejects break/continue outside range.
        Ctrl::Break => Error::Other("break".to_string()),
        Ctrl::Continue => Error::Other("continue".to_string()),
    }
}

// Go: exec.go:doublePercent is not needed: messages are assembled from
// already-formatted parts instead of through a format string.

// Go: exec.go:isRuneInt
fn is_rune_int(s: &[u8]) -> bool {
    !s.is_empty() && s[0] == b'\''
}

// Go: exec.go:isHexInt
fn is_hex_int(s: &[u8]) -> bool {
    s.len() > 2
        && s[0] == b'0'
        && (s[1] == b'x' || s[1] == b'X')
        && !s.iter().any(|c| b"pP".contains(c))
}

impl<'a> State<'a> {
    // Go: exec.go:(*state).push
    /// Pushes a new variable on the stack.
    fn push(&mut self, name: &str, value: Value) {
        self.vars.push(Variable {
            name: name.to_string(),
            value,
        });
    }

    // Go: exec.go:(*state).mark
    /// Returns the length of the variable stack.
    fn mark(&self) -> usize {
        self.vars.len()
    }

    // Go: exec.go:(*state).pop
    /// Pops the variable stack up to the mark.
    fn pop(&mut self, mark: usize) {
        self.vars.truncate(mark);
    }

    // Go: exec.go:(*state).setVar
    /// Overwrites the last declared variable with the given name.
    /// Used by variable assignments.
    fn set_var(&mut self, name: &str, value: Value) -> R<()> {
        for i in (0..self.mark()).rev() {
            if self.vars[i].name == name {
                self.vars[i].value = value;
                return Ok(());
            }
        }
        Err(self.errorf(format!("undefined variable: {name}")))
    }

    // Go: exec.go:(*state).setTopVar
    /// Overwrites the top-nth variable on the stack. Used by range iterations.
    fn set_top_var(&mut self, n: usize, value: Value) {
        let l = self.vars.len();
        self.vars[l - n].value = value;
    }

    // Go: exec.go:(*state).varValue
    /// Returns the value of the named variable.
    fn var_value(&self, name: &str) -> R<Value> {
        for i in (0..self.mark()).rev() {
            if self.vars[i].name == name {
                return Ok(self.vars[i].value.clone());
            }
        }
        Err(self.errorf(format!("undefined variable: {name}")))
    }

    // Go: exec.go:(*state).at
    /// Marks the state to be on node n, for error reporting.
    fn at(&mut self, node: &'a dyn NodeLike) {
        self.node = Some(node);
    }

    // Go: exec.go:(*state).errorf
    /// Records an ExecError and terminates processing.
    fn errorf(&self, msg: String) -> Ctrl {
        self.errorf_cause(msg, None)
    }

    fn errorf_cause(&self, msg: String, cause: Option<go_value::Error>) -> Ctrl {
        let name = self.tmpl.name();
        let message = match self.node {
            None => format!("template: {name}: {msg}"),
            Some(node) => {
                let (location, context) = error_context(Some(self.tree), node);
                format!(
                    "template: {location}: executing {} at <{context}>: {msg}",
                    go_strconv::quote(name)
                )
            }
        };
        Ctrl::Err(Error::Exec(ExecError {
            name: name.to_string(),
            message,
            cause,
        }))
    }

    // Go: exec.go:(*state).writeError
    fn write_error(&self, err: std::io::Error) -> Ctrl {
        Ctrl::Err(Error::Write(Arc::new(err)))
    }

    /// Go `truth(arg)`: `isTrue(indirectInterface(arg))`, where `isTrue` is
    /// Hugo's (`ExecHelper::is_true`).
    fn truth(&self, arg: &Value) -> bool {
        self.helper.is_true(&indirect_interface(arg))
    }

    // Go: exec.go:(*state).walk
    /// Walk functions step through the major pieces of the template
    /// structure, generating output as they go.
    fn walk(&mut self, dot: &Value, node: &'a Node) -> R<()> {
        self.at(node);
        match node {
            Node::Action(n) => {
                // Do not pop variables so they persist until next end.
                // Also, if the action declares variables, don't print the result.
                let val = self.eval_pipeline(dot, Some(&n.pipe))?;
                if n.pipe.decl.is_empty() {
                    self.print_value(n, &val)?;
                }
                Ok(())
            }
            Node::Break(_) => Err(Ctrl::Break),
            Node::Comment(_) => Ok(()),
            Node::Continue(_) => Err(Ctrl::Continue),
            Node::If(n) => {
                self.walk_if_or_with(NodeType::If, dot, &n.pipe, &n.list, n.else_list.as_ref())
            }
            Node::List(n) => self.walk_list(dot, n),
            Node::Range(n) => self.walk_range(dot, n),
            Node::Template(n) => self.walk_template(dot, n),
            Node::Text(n) => {
                if let Err(e) = self.wr.write_all(&n.text) {
                    return Err(self.write_error(e));
                }
                Ok(())
            }
            Node::With(n) => {
                self.walk_if_or_with(NodeType::With, dot, &n.pipe, &n.list, n.else_list.as_ref())
            }
            _ => Err(self.errorf(format!("unknown node: {node}"))),
        }
    }

    /// `walk` of a `*ListNode`.
    fn walk_list(&mut self, dot: &Value, list: &'a ListNode) -> R<()> {
        self.at(list);
        for node in &list.nodes {
            self.walk(dot, node)?;
        }
        Ok(())
    }

    // Go: exec.go:(*state).walkIfOrWith
    /// Walks an 'if' or 'with' node. The two control structures are
    /// identical in behavior except that 'with' sets dot.
    fn walk_if_or_with(
        &mut self,
        typ: NodeType,
        dot: &Value,
        pipe: &'a PipeNode,
        list: &'a ListNode,
        else_list: Option<&'a ListNode>,
    ) -> R<()> {
        let mark = self.mark();
        let r = (|| {
            let val = self.eval_pipeline(dot, Some(pipe))?;
            let truth = self.helper.is_true(&indirect_interface(&val));
            if truth {
                if typ == NodeType::With {
                    self.walk_list(&val, list)
                } else {
                    self.walk_list(dot, list)
                }
            } else if let Some(else_list) = else_list {
                self.walk_list(dot, else_list)
            } else {
                Ok(())
            }
        })();
        self.pop(mark);
        r
    }

    // Go: exec.go:(*state).walkRange
    fn walk_range(&mut self, dot: &Value, r: &'a BranchNode) -> R<()> {
        self.at(r);
        let mark = self.mark();
        let res = self.walk_range_inner(dot, r);
        self.pop(mark);
        match res {
            Err(Ctrl::Break) => Ok(()),
            other => other,
        }
    }

    fn one_iteration(
        &mut self,
        r: &'a BranchNode,
        mark: usize,
        index: Value,
        elem: Value,
    ) -> R<()> {
        let decl = &r.pipe.decl;
        if !decl.is_empty() {
            if r.pipe.is_assign {
                // With two variables, index comes first.
                // With one, we use the element.
                if decl.len() > 1 {
                    self.set_var(&decl[0].ident[0], index.clone())?;
                } else {
                    self.set_var(&decl[0].ident[0], elem.clone())?;
                }
            } else {
                // Set top var (lexically the second if there
                // are two) to the element.
                self.set_top_var(1, elem.clone());
            }
        }
        if decl.len() > 1 {
            if r.pipe.is_assign {
                self.set_var(&decl[1].ident[0], elem.clone())?;
            } else {
                // Set next var (lexically the first if there
                // are two) to the index.
                self.set_top_var(2, index);
            }
        }
        let res = self.walk_list(&elem, &r.list);
        self.pop(mark);
        match res {
            // Consume panic(walkContinue)
            Err(Ctrl::Continue) => Ok(()),
            other => other,
        }
    }

    fn walk_range_inner(&mut self, dot: &Value, r: &'a BranchNode) -> R<()> {
        let val = indirect(&self.eval_pipeline(dot, Some(&r.pipe))?).0;
        // mark top of stack before any variables in the body are pushed.
        let mark = self.mark();
        match &val {
            Value::Int(n, k) => {
                if r.pipe.decl.len() > 1 {
                    return Err(self.errorf(format!(
                        "can't use {} to iterate over more than one variable",
                        sprint_v(&val)
                    )));
                }
                let mut run = false;
                let mut i = 0i64;
                while i < *n {
                    run = true;
                    // Pass element as second value, as we do for channels.
                    self.one_iteration(r, mark, Value::Invalid, Value::Int(i, *k))?;
                    i += 1;
                }
                if run {
                    return Ok(());
                }
            }
            Value::Uint(n, k) => {
                if r.pipe.decl.len() > 1 {
                    return Err(self.errorf(format!(
                        "can't use {} to iterate over more than one variable",
                        sprint_v(&val)
                    )));
                }
                let mut run = false;
                let mut i = 0u64;
                while i < *n {
                    run = true;
                    self.one_iteration(r, mark, Value::Invalid, Value::Uint(i, *k))?;
                    i += 1;
                }
                if run {
                    return Ok(());
                }
            }
            Value::List(l) => {
                if !l.items.is_empty() {
                    for (i, elem) in l.items.iter().enumerate() {
                        let elem = list_elem(&l.ty, elem);
                        self.one_iteration(r, mark, Value::int(i as i64), elem)?;
                    }
                    return Ok(());
                }
            }
            Value::Map(m) => {
                if !m.entries.is_empty() {
                    for (k, v) in m.entries.iter() {
                        let v = map_elem(&m.ty, v);
                        self.one_iteration(r, mark, Value::String(k.clone()), v)?;
                    }
                    return Ok(());
                }
            }
            Value::Object(o) if o.kind() == Kind::Slice => {
                let items = o.list().unwrap_or_default();
                if !items.is_empty() {
                    for (i, elem) in items.into_iter().enumerate() {
                        let elem = list_elem(&SliceType::Any, &elem);
                        self.one_iteration(r, mark, Value::int(i as i64), elem)?;
                    }
                    return Ok(());
                }
            }
            Value::Object(o) if o.kind() == Kind::Map => {
                let keys = o.map_keys();
                if !keys.is_empty() {
                    for k in keys {
                        let v = o.map_get(&k).unwrap_or(Value::Invalid);
                        let v = map_elem(&MapType::StringAny, &v);
                        self.one_iteration(r, mark, Value::String(k), v)?;
                    }
                    return Ok(());
                }
            }
            Value::TypedNil(t)
                if matches!(
                    typed_nil_kind(t),
                    NilKind::Slice | NilKind::Map | NilKind::Chan
                ) =>
            {
                // A nil slice, map or channel is empty.
            }
            Value::Invalid => {
                // An invalid value is likely a nil map, etc. and acts like an empty map.
            }
            _ => {
                return Err(self.errorf(format!("range can't iterate over {}", sprint_v(&val))));
            }
        }
        if let Some(else_list) = &r.else_list {
            self.walk_list(dot, else_list)?;
        }
        Ok(())
    }

    // Go: exec.go:(*state).walkTemplate
    fn walk_template(&mut self, dot: &Value, t: &'a TemplateNode) -> R<()> {
        self.at(t);
        let Some(tmpl) = self.tmpl.lookup(&t.name) else {
            return Err(self.errorf(format!(
                "template {} not defined",
                go_strconv::quote(&t.name)
            )));
        };
        if self.depth == MAX_EXEC_DEPTH {
            return Err(self.errorf(format!(
                "exceeded maximum template depth ({MAX_EXEC_DEPTH})"
            )));
        }
        // Variables declared by the pipeline persist.
        let dot = self.eval_pipeline(dot, t.pipe.as_ref())?;
        let Some(snapshot) = tmpl.tree().map(|tr| tr.get()) else {
            // Go dereferences a nil tree here (a runtime panic).
            return Err(self.errorf(format!(
                "{} is an incomplete or empty template",
                go_strconv::quote(tmpl.name())
            )));
        };
        let Some(root) = snapshot.root.as_ref() else {
            return Err(self.errorf(format!(
                "{} is an incomplete or empty template",
                go_strconv::quote(tmpl.name())
            )));
        };
        let mut new_state = State {
            tmpl,
            tree: &snapshot,
            ctx: self.ctx,
            helper: self.helper,
            wr: &mut *self.wr,
            node: None,
            // No dynamic scoping: template invocations inherit no variables.
            vars: vec![Variable {
                name: "$".to_string(),
                value: dot.clone(),
            }],
            depth: self.depth + 1,
        };
        new_state.walk_list(&dot, root)
    }

    // Eval functions evaluate pipelines, commands, and their elements and extract
    // values from the data structure by examining fields, calling methods, and so on.
    // The printing of those values happens only through walk functions.

    // Go: exec.go:(*state).evalPipeline
    /// Returns the value acquired by evaluating a pipeline. If the pipeline
    /// has a variable declaration, the variable will be pushed on the stack.
    /// Callers should therefore pop the stack after they are finished
    /// executing commands depending on the pipeline value.
    fn eval_pipeline(&mut self, dot: &Value, pipe: Option<&'a PipeNode>) -> R<Value> {
        let Some(pipe) = pipe else {
            return Ok(Value::Invalid);
        };
        self.at(pipe);
        let mut value: Option<Value> = None; // missingVal
        for cmd in &pipe.cmds {
            let v = self.eval_command(dot, cmd, value)?; // previous value is this one's final arg.
            // If the object has type interface{}, dig down one level to the
            // thing inside. Values are concrete, except a nil `interface {}`
            // (a nil `any` from a map, slice, field or result), whose Elem is
            // the invalid Value.
            value = Some(if is_nil_empty_interface(&v) {
                Value::Invalid
            } else {
                v
            });
        }
        let value = value.unwrap_or_else(|| Value::Object(Arc::new(MissingVal)));
        for variable in &pipe.decl {
            if pipe.is_assign {
                self.set_var(&variable.ident[0], value.clone())?;
            } else {
                self.push(&variable.ident[0], value.clone());
            }
        }
        Ok(value)
    }

    // Go: exec.go:(*state).notAFunction
    fn not_a_function(&self, args: &[Node], final_: &Option<Value>) -> R<()> {
        if args.len() > 1 || final_.is_some() {
            return Err(self.errorf(format!("can't give argument to non-function {}", args[0])));
        }
        Ok(())
    }

    // Go: exec.go:(*state).evalCommand
    fn eval_command(
        &mut self,
        dot: &Value,
        cmd: &'a CommandNode,
        final_: Option<Value>,
    ) -> R<Value> {
        let first_word = &cmd.args[0];
        match first_word {
            Node::Field(n) => return self.eval_field_node(dot, n, &cmd.args, final_),
            Node::Chain(n) => return self.eval_chain_node(dot, n, &cmd.args, final_),
            Node::Identifier(n) => {
                // Must be a function.
                return self.eval_function(dot, n, cmd, &cmd.args, final_);
            }
            Node::Pipe(n) => {
                // Parenthesized pipeline. The arguments are all inside the pipeline; final must be absent.
                self.not_a_function(&cmd.args, &final_)?;
                return self.eval_pipeline(dot, Some(n));
            }
            Node::Variable(n) => return self.eval_variable_node(dot, n, &cmd.args, final_),
            _ => {}
        }
        self.at(first_word);
        self.not_a_function(&cmd.args, &final_)?;
        match first_word {
            Node::Bool(word) => return Ok(Value::Bool(word.true_)),
            Node::Dot(_) => return Ok(dot.clone()),
            Node::Nil(_) => return Err(self.errorf("nil is not a command".to_string())),
            Node::Number(word) => return self.ideal_constant(word),
            Node::String(word) => return Ok(Value::string(word.text.as_slice())),
            _ => {}
        }
        Err(self.errorf(format!(
            "can't evaluate command {}",
            go_strconv::quote(first_word.to_bytes())
        )))
    }

    // Go: exec.go:(*state).idealConstant
    /// Is called to return the value of a number in a context where we don't
    /// know the type. In that case, the syntax of the number tells us its
    /// type, and we use Go rules to resolve. Note there is no such thing as
    /// a uint ideal constant in this situation - the value must be of int
    /// type.
    fn ideal_constant(&mut self, constant: &'a NumberNode) -> R<Value> {
        // These are ideal constants but we don't know the type
        // and we have no context.  (If it was a method argument,
        // we'd know what we need.) The syntax guides us to some extent.
        self.at(constant);
        if constant.is_complex {
            // Deviation: the value model has no complex numbers.
            return Err(self.errorf(format!(
                "complex constant {} is not supported",
                String::from_utf8_lossy(&constant.text)
            )));
        }
        if constant.is_float
            && !is_hex_int(&constant.text)
            && !is_rune_int(&constant.text)
            && constant.text.iter().any(|c| b".eEpP".contains(c))
        {
            return Ok(Value::float64(constant.float64));
        }
        if constant.is_int {
            // Go int is 64-bit: int(constant.Int64) never overflows.
            return Ok(Value::int(constant.int64));
        }
        if constant.is_uint {
            return Err(self.errorf(format!(
                "{} overflows int",
                String::from_utf8_lossy(&constant.text)
            )));
        }
        Ok(Value::Invalid)
    }

    // Go: exec.go:(*state).evalFieldNode
    fn eval_field_node(
        &mut self,
        dot: &Value,
        field: &'a FieldNode,
        args: &'a [Node],
        final_: Option<Value>,
    ) -> R<Value> {
        self.at(field);
        self.eval_field_chain(dot, dot.clone(), field, &field.ident, args, final_)
    }

    // Go: exec.go:(*state).evalChainNode
    fn eval_chain_node(
        &mut self,
        dot: &Value,
        chain: &'a ChainNode,
        args: &'a [Node],
        final_: Option<Value>,
    ) -> R<Value> {
        self.at(chain);
        if chain.field.is_empty() {
            return Err(self.errorf("internal error: no fields in evalChainNode".to_string()));
        }
        if chain.node.node_type() == NodeType::Nil {
            return Err(self.errorf(format!(
                "indirection through explicit nil in {}",
                chain.to_string_lossy()
            )));
        }
        // (pipe).Field1.Field2 has pipe as .Node, fields as .Field. Eval the pipeline, then the fields.
        let pipe = self.eval_arg(dot, ArgType::Any, &chain.node)?;
        self.eval_field_chain(dot, pipe, chain, &chain.field, args, final_)
    }

    // Go: exec.go:(*state).evalVariableNode
    fn eval_variable_node(
        &mut self,
        dot: &Value,
        variable: &'a VariableNode,
        args: &'a [Node],
        final_: Option<Value>,
    ) -> R<Value> {
        // $x.Field has $x as the first ident, Field as the second. Eval the var, then the fields.
        self.at(variable);
        let value = self.var_value(&variable.ident[0])?;
        if variable.ident.len() == 1 {
            self.not_a_function(args, &final_)?;
            return Ok(value);
        }
        self.eval_field_chain(dot, value, variable, &variable.ident[1..], args, final_)
    }

    // Go: exec.go:(*state).evalFieldChain
    /// Evaluates .X.Y.Z possibly followed by arguments. dot is the
    /// environment in which to evaluate arguments, while receiver is the
    /// value being walked along the chain.
    fn eval_field_chain(
        &mut self,
        dot: &Value,
        mut receiver: Value,
        node: &'a dyn NodeLike,
        ident: &'a [String],
        args: &'a [Node],
        final_: Option<Value>,
    ) -> R<Value> {
        let n = ident.len();
        for i in 0..n - 1 {
            receiver = self.eval_field(dot, &ident[i], node, &[], None, receiver)?;
        }
        // Now if it's a method, it gets the arguments.
        self.eval_field(dot, &ident[n - 1], node, args, final_, receiver)
    }

    // Go: hugo_template.go:(*state).evalFunction
    fn eval_function(
        &mut self,
        dot: &Value,
        node: &'a IdentifierNode,
        cmd: &'a dyn NodeLike,
        args: &'a [Node],
        final_: Option<Value>,
    ) -> R<Value> {
        self.at(node);
        let name = node.ident.as_str();

        // Added for Hugo.
        let mut is_builtin = name == "and" || name == "or";
        let mut function = self.helper.get_func(self.ctx, name).map(callee_of);

        if function.is_none() {
            // Go: findFunction(name, s.tmpl)
            let exec = {
                let funcs = self
                    .tmpl
                    .common()
                    .funcs
                    .read()
                    .unwrap_or_else(|e| e.into_inner());
                funcs.exec.get(name).cloned()
            };
            if let Some(f) = exec {
                function = Some(callee_of(f));
                is_builtin = false;
            } else if let Some(b) = Builtin::from_name(name) {
                function = Some(Callee::Builtin(b));
                is_builtin = true;
            }
        }

        let Some(function) = function else {
            return Err(self.errorf(format!(
                "{} is not a defined function",
                go_strconv::quote(name)
            )));
        };
        self.eval_call(dot, function, is_builtin, cmd, name, args, final_)
    }

    // Go: hugo_template.go:(*state).evalField
    /// Evaluates an expression like (.Field) or (.Field arg1 arg2). The
    /// 'final' argument represents the return value from the preceding
    /// value of the pipeline, if any.
    fn eval_field(
        &mut self,
        dot: &Value,
        field_name: &str,
        node: &'a dyn NodeLike,
        args: &'a [Node],
        final_: Option<Value>,
        receiver: Value,
    ) -> R<Value> {
        if receiver.is_invalid() {
            if self.tmpl.option().missing_key == MissingKeyAction::Error {
                // Treat invalid value as missing map key.
                return Err(self.errorf(format!(
                    "nil data; no entry for key {}",
                    go_strconv::quote(field_name)
                )));
            }
            return Ok(Value::Invalid);
        }
        let typ = type_name(&receiver);
        let (receiver, is_nil) = indirect(&receiver);
        // Unless it's an interface, need to get to a value of type *T to
        // guarantee we see all methods of T and *T.
        let receiver = funcs::addr(receiver);
        let nil_kind = match &receiver {
            Value::TypedNil(t) => Some(typed_nil_kind(t)),
            _ => None,
        };
        if is_nil && nil_kind == Some(NilKind::Interface) {
            // Calling a method on a nil interface can't work. The
            // MethodByName method call below would panic.
            return Err(self.errorf(format!("nil pointer evaluating {typ}.{field_name}")));
        }

        // Added for Hugo.
        if self.helper.has_method(self.ctx, &receiver, field_name) {
            return self.eval_call(
                dot,
                Callee::Method(receiver),
                false,
                node,
                field_name,
                args,
                final_,
            );
        }

        let has_args = args.len() > 1 || final_.is_some();
        // It's not a method; must be a field of a struct or an element of a map.
        let is_map = match &receiver {
            Value::Map(_) => true,
            Value::Object(o) => o.kind() == Kind::Map,
            Value::TypedNil(_) => nil_kind == Some(NilKind::Map),
            _ => false,
        };
        if let Value::Object(o) = &receiver {
            if matches!(o.kind(), Kind::Struct | Kind::Ptr | Kind::Interface) {
                if let Some(field) = o.field(field_name) {
                    // If it's a function, we must call it.
                    if has_args {
                        return Err(self.errorf(format!(
                            "{field_name} has arguments but cannot be invoked as function"
                        )));
                    }
                    // A Go struct field is never the invalid Value: a nil
                    // `any` field (`Invalid` in the value model, e.g.
                    // `TryValue.Value` after an error) is a nil
                    // `interface {}`, so `.Value.X` fails as in Go.
                    if field.is_invalid() {
                        return Ok(nil_empty_interface());
                    }
                    return Ok(field);
                }
            }
        }
        if is_map {
            // If it's a map, attempt to use the field name as a key.
            if has_args {
                return Err(self.errorf(format!("{field_name} is not a method but has arguments")));
            }
            // Added for Hugo.
            let result = self
                .helper
                .get_map_value(self.ctx, &receiver, &Value::string(field_name));
            return match result {
                // A present key holding nil: Go's MapIndex result is the
                // element type's nil, a nil interface for these maps.
                Some(Value::Invalid) => Ok(nil_empty_interface()),
                Some(v) => Ok(v),
                None => match self.tmpl.option().missing_key {
                    // Just use the invalid value.
                    MissingKeyAction::Invalid => Ok(Value::Invalid),
                    MissingKeyAction::ZeroValue => Ok(map_zero_elem(&receiver)),
                    MissingKeyAction::Error => Err(self.errorf(format!(
                        "map has no entry for key {}",
                        go_strconv::quote(field_name)
                    ))),
                },
            };
        }
        if is_nil && nil_kind == Some(NilKind::Ptr) {
            return Err(self.errorf(format!("nil pointer evaluating {typ}.{field_name}")));
        }
        Err(self.errorf(format!("can't evaluate field {field_name} in type {typ}")))
    }

    // Go: hugo_template.go:(*state).evalCall
    /// Executes a function or method call. If it's a method, fun already has
    /// the receiver bound, so it looks just like a function call. The arg
    /// list, if non-nil, includes (in the manner of the shell), arg[0] as
    /// the function itself.
    fn eval_call(
        &mut self,
        dot: &Value,
        fun: Callee,
        is_builtin: bool,
        node: &'a dyn NodeLike,
        name: &str,
        args: &'a [Node],
        final_: Option<Value>,
    ) -> R<Value> {
        // Added for Hugo.
        if name == "try" {
            return match self.eval_call_inner(dot, fun, is_builtin, node, name, args, final_) {
                Ok(v) => Ok(Value::object(TryValue {
                    value: v,
                    err: None,
                })),
                Err(Ctrl::Err(e)) => Ok(Value::object(TryValue {
                    value: Value::Invalid,
                    err: Some(Arc::new(TryError::new(e))),
                })),
                Err(other) => Err(other),
            };
        }
        self.eval_call_inner(dot, fun, is_builtin, node, name, args, final_)
    }

    fn eval_call_inner(
        &mut self,
        dot: &Value,
        mut fun: Callee,
        is_builtin: bool,
        node: &'a dyn NodeLike,
        name: &str,
        args: &'a [Node],
        final_: Option<Value>,
    ) -> R<Value> {
        // Zeroth arg is function name/node; not passed to function.
        let args: &'a [Node] = if args.is_empty() { args } else { &args[1..] };
        let num_in = args.len() + usize::from(final_.is_some());
        // Go checks the function's reflect signature. Host functions and
        // methods check their own arguments; the engine knows the builtins'
        // (and/or always use theirs).
        let builtin_sig = match &fun {
            Callee::Builtin(b) => Some(*b),
            _ if is_builtin => Builtin::from_name(name),
            _ => None,
        };
        if let Some(b) = builtin_sig {
            let sig = b.sig();
            if sig.variadic {
                let num_fixed = sig.num_in - 1; // last arg is the variadic one.
                if num_in < num_fixed {
                    return Err(self.errorf(format!(
                        "wrong number of args for {name}: want at least {} got {}",
                        sig.num_in - 1,
                        args.len()
                    )));
                }
            } else if num_in != sig.num_in {
                return Err(self.errorf(format!(
                    "wrong number of args for {name}: want {} got {num_in}",
                    sig.num_in
                )));
            }
        }

        // Special case for builtin and/or, which short-circuit.
        if is_builtin && (name == "and" || name == "or") {
            let mut v = Value::Invalid;
            for arg in args {
                v = self.eval_arg(dot, ArgType::Any, arg)?;
                if self.truth(&v) == (name == "or") {
                    // This value was already unwrapped
                    // by the .Interface().(reflect.Value).
                    return Ok(v);
                }
            }
            if let Some(f) = final_ {
                // The last argument to and/or is coming from
                // the pipeline. We didn't short circuit on an earlier
                // argument, so we are going to return this one.
                // We don't have to evaluate final, but we do
                // have to check its type. Then, since we are
                // going to return it, we have to unwrap it.
                v = f;
            }
            return Ok(v);
        }

        let arg_type = |i: usize| match builtin_sig {
            Some(b) => b.arg_type(i),
            None => ArgType::Any,
        };

        // Build the arg list.
        let mut argv: Vec<Value> = Vec::with_capacity(num_in);
        // Args must be evaluated. Fixed args first, then the ... args.
        for (i, arg) in args.iter().enumerate() {
            let v = self.eval_arg(dot, arg_type(i), arg)?;
            argv.push(v);
        }
        // Add final value if necessary.
        if let Some(f) = final_.clone() {
            let t = arg_type(argv.len());
            let v = self.validate_type(f, t)?;
            argv.push(v);
        }

        // Special case for the "call" builtin.
        // Insert the name of the callee function as the first argument.
        let mut callee_name: Option<String> = None;
        if is_builtin && name == "call" {
            callee_name = Some(if args.is_empty() {
                // final must be present or we would have errored out above.
                reflect_value_string(final_.as_ref().unwrap_or(&Value::Invalid))
            } else {
                args[0].to_string_lossy()
            });
            fun = Callee::Builtin(Builtin::Call);
        }

        let result: Result<Value, go_value::Error> = match &fun {
            Callee::Builtin(Builtin::Not) => Ok(Value::Bool(!self.truth(&argv[0]))),
            Callee::Builtin(Builtin::Call) if callee_name.is_some() => {
                let cname = callee_name.unwrap_or_default();
                self.call(&cname, &argv[0], &argv[1..])
            }
            // Go: emptyCall, and, or (reached without the special cases,
            // e.g. through ExecHelper.GetFunc under another name).
            Callee::Builtin(Builtin::Call)
            | Callee::Builtin(Builtin::And)
            | Callee::Builtin(Builtin::Or) => Err(go_value::Error::new("unreachable")),
            Callee::Builtin(b) if b.takes_any() => {
                let converted = convert_args(&argv);
                funcs::call_builtin_simple(*b, &converted).map_err(go_value::Error::new)
            }
            Callee::Builtin(b) => {
                funcs::call_builtin_simple(*b, &argv).map_err(go_value::Error::new)
            }
            Callee::Func(f) => {
                let converted = convert_args(&argv);
                f(self.ctx, &converted)
            }
            Callee::Method(receiver) => {
                let converted = convert_args(&argv);
                self.helper
                    .call_method(self.ctx, receiver, name, &converted)
            }
        };

        // If we have an error that is not nil, stop execution and return that
        // error to the caller.
        let v = match result {
            Ok(v) => v,
            Err(err) => {
                self.at(node);
                return Err(self.errorf_cause(
                    format!("error calling {name}: {}", err.message()),
                    Some(err),
                ));
            }
        };

        // Added for Hugo
        self.helper.on_called(self.ctx, name, &argv, &v);

        // A Go call result is never the invalid Value: a nil returned as
        // `any` (which hosts return as `Invalid`, contract C7) is a nil
        // `interface {}`, so `f.X` and `(try f).Value.X` fail with
        // "nil pointer evaluating interface {}.X" as in Go; the end of the
        // pipeline command turns it back into `Invalid` (evalPipeline).
        if v.is_invalid()
            && matches!(
                fun,
                Callee::Func(_) | Callee::Method(_) | Callee::Builtin(Builtin::Call)
            )
        {
            return Ok(nil_empty_interface());
        }
        Ok(v)
    }

    // Go: funcs.go:call
    /// Returns the result of evaluating the first argument as a function.
    /// The function must return 1 result, or 2 results, the second of
    /// which is an error.
    fn call(&self, name: &str, fn_: &Value, args: &[Value]) -> Result<Value, go_value::Error> {
        let fn_ = indirect_interface(fn_);
        if fn_.is_invalid() {
            return Err(go_value::Error::new("call of nil"));
        }
        // The function's type: a FuncValue's Go type string, or a nil func.
        let fv = fn_.downcast::<FuncValue>();
        let typ = match (&fn_, fv) {
            (_, Some(fv)) => fv.type_name.as_str(),
            (Value::TypedNil(t), None) if typed_nil_kind(t) == NilKind::Func => t,
            _ => {
                return Err(go_value::Error::new(format!(
                    "non-function {name} of type {}",
                    type_name(&fn_)
                )));
            }
        };
        let mut argv: Vec<Value> = args.iter().map(indirect_interface).collect();
        // Go checks the signature (goodFunc, arity, prepareArg). The value
        // model only has the type string; when it is a func literal type the
        // checks are Go's, otherwise the function checks its own arguments.
        if let Some(sig) = funcs::FuncSig::parse(typ) {
            sig.good_func(name).map_err(go_value::Error::new)?;
            let num_in = sig.params.len();
            if sig.variadic {
                if args.len() < num_in - 1 {
                    return Err(go_value::Error::new(format!(
                        "wrong number of args for {name}: got {} want at least {}",
                        args.len(),
                        num_in - 1
                    )));
                }
            } else if args.len() != num_in {
                return Err(go_value::Error::new(format!(
                    "wrong number of args for {name}: got {} want {num_in}",
                    args.len()
                )));
            }
            for (i, arg) in argv.iter_mut().enumerate() {
                let arg_type = sig.in_type(i);
                *arg = funcs::prepare_arg(arg, &arg_type)
                    .map_err(|e| go_value::Error::new(format!("arg {i}: {e}")))?;
            }
        }
        match fv {
            Some(fv) => (fv.func)(self.ctx, &argv),
            // Go: safeCall recovers reflect's panic.
            None => Err(go_value::Error::new(
                "reflect.Value.Call: call of nil function",
            )),
        }
    }

    // Go: exec.go:(*state).validateType
    /// Guarantees that the value is valid and assignable to the type.
    fn validate_type(&self, value: Value, typ: ArgType) -> R<Value> {
        match typ {
            // Everything is assignable to interface{}.
            ArgType::Any => Ok(value),
            ArgType::String => match value {
                Value::Invalid => Err(self.errorf("invalid value; expected string".to_string())),
                Value::String(_) => Ok(value),
                other => Err(self.errorf(format!(
                    "wrong type for value; expected string; got {}",
                    type_name(&other)
                ))),
            },
        }
    }

    // Go: exec.go:(*state).evalArg
    fn eval_arg(&mut self, dot: &Value, typ: ArgType, n: &'a Node) -> R<Value> {
        self.at(n);
        match n {
            Node::Dot(_) => return self.validate_type(dot.clone(), typ),
            Node::Nil(_) => {
                if typ == ArgType::Any {
                    return Ok(Value::Invalid);
                }
                return Err(self.errorf("cannot assign nil to string".to_string()));
            }
            Node::Field(arg) => {
                let v = self.eval_field_node(dot, arg, std::slice::from_ref(n), None)?;
                return self.validate_type(v, typ);
            }
            Node::Variable(arg) => {
                let v = self.eval_variable_node(dot, arg, &[], None)?;
                return self.validate_type(v, typ);
            }
            Node::Pipe(arg) => {
                let v = self.eval_pipeline(dot, Some(arg))?;
                return self.validate_type(v, typ);
            }
            Node::Identifier(arg) => {
                let v = self.eval_function(dot, arg, arg, &[], None)?;
                return self.validate_type(v, typ);
            }
            Node::Chain(arg) => {
                let v = self.eval_chain_node(dot, arg, &[], None)?;
                return self.validate_type(v, typ);
            }
            _ => {}
        }
        match typ {
            ArgType::Any => self.eval_empty_interface(dot, n),
            ArgType::String => self.eval_string(n),
        }
    }

    // Go: exec.go:(*state).evalString
    fn eval_string(&mut self, n: &'a Node) -> R<Value> {
        self.at(n);
        if let Node::String(n) = n {
            return Ok(Value::string(n.text.as_slice()));
        }
        Err(self.errorf(format!("expected string; found {n}")))
    }

    // Go: exec.go:(*state).evalEmptyInterface
    fn eval_empty_interface(&mut self, dot: &Value, n: &'a Node) -> R<Value> {
        self.at(n);
        match n {
            Node::Bool(n) => Ok(Value::Bool(n.true_)),
            Node::Dot(_) => Ok(dot.clone()),
            Node::Field(n) => self.eval_field_node(dot, n, &[], None),
            Node::Identifier(n) => self.eval_function(dot, n, n, &[], None),
            Node::Nil(_) => {
                // NilNode is handled in evalArg, the only place that calls here.
                Err(self.errorf("evalEmptyInterface: nil (can't happen)".to_string()))
            }
            Node::Number(n) => self.ideal_constant(n),
            Node::String(n) => Ok(Value::string(n.text.as_slice())),
            Node::Variable(n) => self.eval_variable_node(dot, n, &[], None),
            Node::Pipe(n) => self.eval_pipeline(dot, Some(n)),
            _ => Err(self.errorf(format!(
                "can't handle assignment of {n} to empty interface argument"
            ))),
        }
    }

    // Go: exec.go:(*state).printValue
    /// Writes the textual representation of the value to the output of the
    /// template.
    fn print_value(&mut self, n: &'a dyn NodeLike, v: &Value) -> R<()> {
        self.at(n);
        let Ok(iface) = printable_value(v) else {
            return Err(self.errorf(format!(
                "can't print {} of type {}",
                n.to_string_lossy(),
                type_name(v)
            )));
        };
        if let Err(e) = go_fmt::fprint(self.wr, &[iface]) {
            return Err(self.write_error(e));
        }
        Ok(())
    }
}

/// Go reflect `Value.String()` (used for the callee name of `call` when it
/// comes from the pipeline).
fn reflect_value_string(v: &Value) -> String {
    match v {
        Value::Invalid => "<invalid Value>".to_string(),
        Value::String(s) | Value::Safe(_, s) => s.to_str_lossy().into_owned(),
        other => format!("<{} Value>", type_name(other)),
    }
}

/// A function found by name: the engine's own builtins (Go: their
/// `reflect.Value`s, whose signatures `evalCall` inspects) when a host
/// hands them back (`go_funcs`), else a host function.
fn callee_of(f: Func) -> Callee {
    match funcs::builtin_of(&f) {
        Some(b) => Callee::Builtin(b),
        None => Callee::Func(f),
    }
}

/// Contract C7: arguments passed to host functions and methods — a nil
/// interface of a non-empty interface type arrives as an untyped nil.
fn convert_args(argv: &[Value]) -> Cow<'_, [Value]> {
    if argv
        .iter()
        .any(|v| matches!(v, Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Interface))
    {
        Cow::Owned(argv.iter().map(indirect_interface).collect())
    } else {
        Cow::Borrowed(argv)
    }
}

/// A nil `interface {}` (Go: an interface-kind `reflect.Value` holding
/// nil, e.g. the value of a `map[string]any` key holding nil). Field access
/// on it is Go's "nil pointer evaluating interface {}.X"; a pipeline turns it
/// into the invalid Value (`evalPipeline`'s `value.Elem()`).
pub(crate) fn nil_empty_interface() -> Value {
    Value::TypedNil(Arc::from(EMPTY_INTERFACE))
}

const EMPTY_INTERFACE: &str = "interface {}";

/// Whether `v` is a nil of the empty interface type (Go: `value.Kind() ==
/// reflect.Interface && value.Type().NumMethod() == 0` for a nil value).
pub(crate) fn is_nil_empty_interface(v: &Value) -> bool {
    matches!(v, Value::TypedNil(t) if &**t == EMPTY_INTERFACE)
}

/// A slice element as Go's `Index` sees it: an element that is nil in the
/// value model (`Value::Invalid`) is the element type's nil — a nil
/// interface for `[]any` (and for named or unknown element types), a nil map
/// for `[]map[string]any`.
fn list_elem(ty: &SliceType, elem: &Value) -> Value {
    if !elem.is_invalid() {
        return elem.clone();
    }
    match ty {
        SliceType::MapStringAny => Value::TypedNil(Arc::from("map[string]interface {}")),
        _ => nil_empty_interface(),
    }
}

/// A map value as Go's `MapIndex` sees it (see [`list_elem`]).
fn map_elem(_ty: &MapType, v: &Value) -> Value {
    if v.is_invalid() {
        nil_empty_interface()
    } else {
        v.clone()
    }
}

/// The zero value of a map's element type (Go `reflect.Zero(receiver.Type().Elem())`).
/// Maps whose element type the model does not know (`map[string]any`,
/// `maps.Params`, host maps) have interface elements, whose zero is a nil
/// interface.
fn map_zero_elem(m: &Value) -> Value {
    funcs::map_elem_zero(m).unwrap_or_else(nil_empty_interface)
}
