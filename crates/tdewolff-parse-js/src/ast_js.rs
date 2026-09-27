//! Go: parse/v2/js/ast.go — the `JS()` writers (JavaScript source output
//! with 4-space indentation) and the `JSON()` writers.
//!
//! Go passes an `io.Writer` that may be a `parse.Indenter`; several writers
//! type-assert on it (to bypass the indentation for literals, or to nest
//! it). Indenters never nest in Go (`NewIndenter` flattens them), so a
//! writer is modelled as an output buffer plus an optional indentation:
//! [`JsWriter`] with `indent: None` is a plain writer, `Some(n)` an
//! `Indenter` of `n` spaces.

use tdewolff_parse::GoBytes;

use crate::ast::*;
use crate::tokentype::*;

/// A Go `io.Writer` that is either plain or a `parse.Indenter`.
pub struct JsWriter<'a> {
    out: &'a mut Vec<u8>,
    indent: Option<usize>,
}

impl<'a> JsWriter<'a> {
    /// A plain writer appending to `out`.
    pub fn new(out: &'a mut Vec<u8>) -> JsWriter<'a> {
        JsWriter { out, indent: None }
    }

    // Go: parse/util.go:Indenter.Write (or the plain writer's Write)
    fn write(&mut self, b: &[u8]) {
        let Some(n) = self.indent else {
            self.out.extend_from_slice(b);
            return;
        };
        let mut j = 0;
        for (i, &c) in b.iter().enumerate() {
            if c == b'\n' {
                self.out.extend_from_slice(&b[j..i + 1]);
                self.out.extend(std::iter::repeat_n(b' ', n));
                j = i + 1;
            }
        }
        self.out.extend_from_slice(&b[j..]);
    }

    fn write_bytes(&mut self, b: &GoBytes) {
        let v = b.to_vec();
        self.write(&v);
    }

    /// `if wi, ok := w.(parse.Indenter); ok { w = wi.Writer }`
    fn plain(&mut self) -> JsWriter<'_> {
        JsWriter {
            out: &mut *self.out,
            indent: None,
        }
    }

    /// Go: parse/util.go:NewIndenter(w, n) (flattens a nested Indenter).
    fn indenter(&mut self, n: usize) -> JsWriter<'_> {
        JsWriter {
            out: &mut *self.out,
            indent: Some(self.indent.unwrap_or(0) + n),
        }
    }
}

/// Go: ast.go:ErrInvalidJSON
pub const ERR_INVALID_JSON: &str = "invalid JSON";

fn json_err(msg: String) -> Result<(), Vec<u8>> {
    Err(msg.into_bytes())
}

/// Go `fmt.Errorf("%v: <what>: %v", ErrInvalidJSON, js)` (js is arbitrary
/// bytes).
fn json_err_with(what: &str, js: &[u8]) -> Result<(), Vec<u8>> {
    let mut e = format!("{}: {}: ", ERR_INVALID_JSON, what).into_bytes();
    e.extend_from_slice(js);
    Err(e)
}

impl Ast {
    // Go: ast.go:AST.JS
    /// Writes JavaScript to `w`.
    pub fn js(&self, w: &mut JsWriter<'_>) {
        for (i, &item) in self.block(self.block_stmt).list.iter().enumerate() {
            if i != 0 {
                w.write(b"\n");
            }
            self.node_js(item, w);
            if matches!(self.node(item), Node::VarDecl(_)) {
                w.write(b";");
            }
        }
    }

    // Go: ast.go:AST.JSString
    /// Returns a string of JavaScript.
    pub fn js_string(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.js(&mut JsWriter::new(&mut out));
        out
    }

    // Go: ast.go:AST.JSON
    /// Writes JSON to `out`; the error is Go's `err.Error()`.
    pub fn json(&self, out: &mut Vec<u8>) -> Result<(), Vec<u8>> {
        let list = &self.block(self.block_stmt).list;
        if 1 < list.len() {
            return json_err(format!(
                "{}: JS must be a single statement",
                ERR_INVALID_JSON
            ));
        } else if list.is_empty() {
            return Ok(());
        }
        let Node::ExprStmt(expr_stmt) = self.node(list[0]) else {
            return json_err(format!(
                "{}: JS must be an expression statement",
                ERR_INVALID_JSON
            ));
        };
        let mut expr = expr_stmt.value;
        if let Node::GroupExpr(group) = self.node(expr) {
            expr = group.x; // allow parsing expr contained in group expr
        }
        if !self.is_jsoner(expr) {
            return json_err(format!(
                "{}: JS must be a valid JSON expression",
                ERR_INVALID_JSON
            ));
        }
        self.node_json(expr, &mut JsWriter::new(out))
    }

    // Go: ast.go:AST.JSONString
    /// Returns a string of JSON if valid.
    pub fn json_string(&self) -> (Vec<u8>, Result<(), Vec<u8>>) {
        let mut out = Vec::new();
        let err = self.json(&mut out);
        (out, err)
    }

    /// Go `n.JS(w)` into a fresh plain buffer (`strings.Builder`).
    pub fn node_js_string(&self, id: NodeId) -> Vec<u8> {
        let mut out = Vec::new();
        self.node_js(id, &mut JsWriter::new(&mut out));
        out
    }

    /// Writes `; ` et al. after statements that are VarDecls.
    fn is_var_decl(&self, id: NodeId) -> bool {
        matches!(self.node(id), Node::VarDecl(_))
    }

    fn is_empty_stmt(&self, id: NodeId) -> bool {
        matches!(self.node(id), Node::EmptyStmt)
    }

    // Go: ast.go:BindingElement.JS
    fn binding_element_js(&self, n: &BindingElement, w: &mut JsWriter<'_>) {
        if n.binding.is_nil() {
            return;
        }

        self.node_js(n.binding, w);
        if n.default.is_some() {
            w.write(b" = ");
            self.node_js(n.default, w);
        }
    }

    // Go: ast.go:Params.JS
    fn params_js(&self, n: &Params, w: &mut JsWriter<'_>) {
        w.write(b"(");
        for (j, item) in n.list.iter().enumerate() {
            if j != 0 {
                w.write(b", ");
            }
            self.binding_element_js(item, w);
        }
        if n.rest.is_some() {
            if !n.list.is_empty() {
                w.write(b", ");
            }
            w.write(b"...");
            self.node_js(n.rest, w);
        }
        w.write(b")");
    }

    // Go: ast.go:PropertyName.JS
    fn property_name_js(&self, n: &PropertyName, w: &mut JsWriter<'_>) {
        if n.computed.is_some() {
            w.write(b"[");
            self.node_js(n.computed, w);
            w.write(b"]");
            return;
        }
        let mut w = w.plain();
        w.write_bytes(&n.literal.data);
    }

    // Go: ast.go:Alias.JS
    fn alias_js(&self, alias: &Alias, w: &mut JsWriter<'_>) {
        if !alias.name.is_nil() {
            w.write_bytes(&alias.name);
            w.write(b" as ");
        }
        w.write_bytes(&alias.binding);
    }

    // Go: ast.go:Args.JS
    fn args_js(&self, n: &Args, w: &mut JsWriter<'_>) {
        for (j, item) in n.list.iter().enumerate() {
            if j != 0 {
                w.write(b", ");
            }
            // Go: ast.go:Arg.JS
            if item.rest {
                w.write(b"...");
            }
            self.node_js(item.value, w);
        }
    }

    // Go: ast.go:LiteralExpr.JS
    fn literal_js(&self, n: &LiteralExpr, w: &mut JsWriter<'_>) {
        let mut w = w.plain();
        w.write_bytes(&n.data);
    }

    // Go: ast.go:Field.JS
    fn field_js(&self, n: &Field, w: &mut JsWriter<'_>) {
        if n.static_ {
            w.write(b"static ");
        }
        self.property_name_js(&n.name, w);
        if n.init.is_some() {
            w.write(b" = ");
            self.node_js(n.init, w);
        }
    }

    /// Writes Go's `n.JS(w)` for any node; nil panics as in Go.
    pub fn node_js(&self, id: NodeId, w: &mut JsWriter<'_>) {
        match self.node(id) {
            Node::Nil => panic!("invalid memory address or nil pointer dereference"),
            // Go: ast.go:Comment.JS
            Node::Comment(n) => {
                let mut w = w.plain();
                w.write_bytes(&n.value);
            }
            // Go: ast.go:BlockStmt.JS
            Node::BlockStmt(n) => {
                if n.list.is_empty() {
                    w.write(b"{}");
                    return;
                }

                w.write(b"{");
                for &item in &n.list {
                    let mut wi = w.indenter(4);
                    wi.write(b"\n");
                    self.node_js(item, &mut wi);
                    if self.is_var_decl(item) {
                        w.write(b";");
                    }
                }
                w.write(b"\n}");
            }
            // Go: ast.go:EmptyStmt.JS
            Node::EmptyStmt => w.write(b";"),
            // Go: ast.go:ExprStmt.JS
            Node::ExprStmt(n) => {
                let mut buf = Vec::new();
                {
                    // make sure that buf is indenter if w is so as well
                    // this is to prevent newlines in literals from indenting
                    let mut wb = JsWriter {
                        out: &mut buf,
                        indent: w.indent,
                    };
                    self.node_js(n.value, &mut wb);
                }
                let mut w = w.plain();
                let expr = buf;

                let group = expr.starts_with(b"let ");
                if group {
                    w.write(b"(");
                }
                w.write(&expr);
                if group {
                    w.write(b")");
                }
                w.write(b";");
            }
            // Go: ast.go:IfStmt.JS
            Node::IfStmt(n) => {
                w.write(b"if (");
                self.node_js(n.cond, w);
                w.write(b")");
                if !self.is_empty_stmt(n.body) {
                    w.write(b" ");
                }
                self.node_js(n.body, w);
                if self.is_var_decl(n.body) {
                    w.write(b";");
                }
                if n.else_.is_some() {
                    w.write(b" else");
                    if !self.is_empty_stmt(n.else_) {
                        w.write(b" ");
                    }
                    self.node_js(n.else_, w);
                    if self.is_var_decl(n.else_) {
                        w.write(b";");
                    }
                }
            }
            // Go: ast.go:DoWhileStmt.JS
            Node::DoWhileStmt(n) => {
                w.write(b"do");
                if !self.is_empty_stmt(n.body) {
                    w.write(b" ");
                }
                self.node_js(n.body, w);
                if self.is_var_decl(n.body) {
                    w.write(b"; ");
                } else if !matches!(self.node(n.body), Node::Comment(_)) {
                    w.write(b" ");
                }
                w.write(b"while (");
                self.node_js(n.cond, w);
                w.write(b");");
            }
            // Go: ast.go:WhileStmt.JS
            Node::WhileStmt(n) => {
                w.write(b"while (");
                self.node_js(n.cond, w);
                w.write(b")");
                if self.is_empty_stmt(n.body) {
                    w.write(b";");
                    return;
                }
                w.write(b" ");
                self.node_js(n.body, w);
                if self.is_var_decl(n.body) {
                    w.write(b";");
                }
            }
            // Go: ast.go:ForStmt.JS
            Node::ForStmt(n) => {
                w.write(b"for (");
                let print_init = match self.node(n.init) {
                    Node::VarDecl(v) => !v.list.is_empty(),
                    _ => n.init.is_some(),
                };
                if print_init {
                    self.node_js(n.init, w);
                } else {
                    w.write(b" ");
                }
                w.write(b"; ");
                if n.cond.is_some() {
                    self.node_js(n.cond, w);
                }
                w.write(b"; ");
                if n.post.is_some() {
                    self.node_js(n.post, w);
                }
                w.write(b") ");
                self.node_js(n.body, w);
            }
            // Go: ast.go:ForInStmt.JS
            Node::ForInStmt(n) => {
                w.write(b"for (");
                self.node_js(n.init, w);
                w.write(b" in ");
                self.node_js(n.value, w);
                w.write(b") ");
                self.node_js(n.body, w);
            }
            // Go: ast.go:ForOfStmt.JS
            Node::ForOfStmt(n) => {
                w.write(b"for");
                if n.await_ {
                    w.write(b" await");
                }
                w.write(b" (");
                self.node_js(n.init, w);
                w.write(b" of ");
                self.node_js(n.value, w);
                w.write(b") ");
                self.node_js(n.body, w);
            }
            // Go: ast.go:SwitchStmt.JS
            Node::SwitchStmt(n) => {
                w.write(b"switch (");
                self.node_js(n.init, w);
                if n.list.is_empty() {
                    w.write(b") {}");
                    return;
                }
                w.write(b") {");
                for clause in &n.list {
                    w.write(b"\n");
                    // Go: ast.go:CaseClause.JS
                    if clause.cond.is_some() {
                        w.write(b"case ");
                        self.node_js(clause.cond, w);
                    } else {
                        w.write(b"default");
                    }
                    w.write(b":");
                    for &item in &clause.list {
                        let mut wi = w.indenter(4);
                        wi.write(b"\n");
                        self.node_js(item, &mut wi);
                        if self.is_var_decl(item) {
                            w.write(b";");
                        }
                    }
                }
                w.write(b"\n}");
            }
            // Go: ast.go:BranchStmt.JS
            Node::BranchStmt(n) => {
                w.write(n.type_.bytes().unwrap_or(b""));
                if !n.label.is_nil() {
                    w.write(b" ");
                    w.write_bytes(&n.label);
                }
                w.write(b";");
            }
            // Go: ast.go:ReturnStmt.JS
            Node::ReturnStmt(n) => {
                w.write(b"return");
                if n.value.is_some() {
                    w.write(b" ");
                    self.node_js(n.value, w);
                }
                w.write(b";");
            }
            // Go: ast.go:WithStmt.JS
            Node::WithStmt(n) => {
                w.write(b"with (");
                self.node_js(n.cond, w);
                w.write(b")");
                if !self.is_empty_stmt(n.body) {
                    w.write(b" ");
                }
                self.node_js(n.body, w);
                if self.is_var_decl(n.body) {
                    w.write(b";");
                }
            }
            // Go: ast.go:LabelledStmt.JS
            Node::LabelledStmt(n) => {
                w.write_bytes(&n.label);
                w.write(b":");
                if !self.is_empty_stmt(n.value) {
                    w.write(b" ");
                }
                self.node_js(n.value, w);
                if self.is_var_decl(n.value) {
                    w.write(b";");
                }
            }
            // Go: ast.go:ThrowStmt.JS
            Node::ThrowStmt(n) => {
                w.write(b"throw ");
                self.node_js(n.value, w);
                w.write(b";");
            }
            // Go: ast.go:TryStmt.JS
            Node::TryStmt(n) => {
                w.write(b"try ");
                self.node_js(n.body, w);
                if n.catch.is_some() {
                    w.write(b" catch");
                    if n.binding.is_some() {
                        w.write(b"(");
                        self.node_js(n.binding, w);
                        w.write(b")");
                    }
                    w.write(b" ");
                    self.node_js(n.catch, w);
                }
                if n.finally.is_some() {
                    w.write(b" finally ");
                    self.node_js(n.finally, w);
                }
            }
            // Go: ast.go:DebuggerStmt.JS
            Node::DebuggerStmt => w.write(b"debugger;"),
            // Go: ast.go:ImportStmt.JS
            Node::ImportStmt(n) => {
                let mut w = w.plain();
                w.write(b"import");
                if !n.default.is_nil() {
                    w.write(b" ");
                    w.write_bytes(&n.default);
                    if n.list.is_some() {
                        w.write(b",");
                    }
                }
                let list: &[Alias] = n.list.as_deref().unwrap_or(&[]);
                if list.len() == 1 && list[0].name.len() == 1 && list[0].name.at(0) == b'*' {
                    w.write(b" ");
                    self.alias_js(&list[0], &mut w);
                } else if n.list.is_some() {
                    if list.is_empty() {
                        w.write(b" {}");
                    } else {
                        w.write(b" {");
                        for (j, item) in list.iter().enumerate() {
                            if j != 0 {
                                w.write(b",");
                            }
                            if !item.binding.is_nil() {
                                w.write(b" ");
                                self.alias_js(item, &mut w);
                            }
                        }
                        w.write(b" }");
                    }
                }
                if !n.default.is_nil() || n.list.is_some() {
                    w.write(b" from");
                }
                w.write(b" ");
                w.write_bytes(&n.module);
                w.write(b";");
            }
            // Go: ast.go:ExportStmt.JS
            Node::ExportStmt(n) => {
                let mut w = w.plain();
                w.write(b"export");
                if n.decl.is_some() {
                    if n.default {
                        w.write(b" default");
                    }
                    w.write(b" ");
                    self.node_js(n.decl, &mut w);
                    w.write(b";");
                    return;
                } else if n.list.len() == 1
                    && (n.list[0].name.len() == 1 && n.list[0].name.at(0) == b'*'
                        || n.list[0].name.is_nil()
                            && n.list[0].binding.len() == 1
                            && n.list[0].binding.at(0) == b'*')
                {
                    w.write(b" ");
                    self.alias_js(&n.list[0], &mut w);
                } else if n.list.is_empty() {
                    w.write(b" {}");
                } else {
                    w.write(b" {");
                    for (j, item) in n.list.iter().enumerate() {
                        if j != 0 {
                            w.write(b",");
                        }
                        if !item.binding.is_nil() {
                            w.write(b" ");
                            self.alias_js(item, &mut w);
                        }
                    }
                    w.write(b" }");
                }
                if !n.module.is_nil() {
                    w.write(b" from ");
                    w.write_bytes(&n.module);
                }
                w.write(b";");
            }
            // Go: ast.go:DirectivePrologueStmt.JS
            Node::DirectivePrologueStmt(n) => {
                let mut w = w.plain();
                w.write_bytes(&n.value);
                w.write(b";");
            }
            // Go: ast.go:VarDecl.JS
            Node::VarDecl(n) => {
                w.write(n.token_type.bytes().unwrap_or(b""));
                for (j, item) in n.list.iter().enumerate() {
                    if j != 0 {
                        w.write(b",");
                    }
                    w.write(b" ");
                    self.binding_element_js(item, w);
                }
            }
            // Go: ast.go:FuncDecl.JS
            Node::FuncDecl(n) => {
                if n.async_ {
                    w.write(b"async function");
                } else {
                    w.write(b"function");
                }

                if n.generator {
                    w.write(b"*");
                }
                if n.name.is_some() {
                    w.write(b" ");
                    w.write_bytes(&self.var(n.name).data);
                }
                self.params_js(&n.params, w);
                w.write(b" ");
                self.node_js(n.body, w);
            }
            // Go: ast.go:MethodDecl.JS
            Node::MethodDecl(n) => {
                let mut writen = false;
                if n.static_ {
                    w.write(b"static");
                    writen = true;
                }
                if n.async_ {
                    if writen {
                        w.write(b" ");
                    }
                    w.write(b"async");
                    writen = true;
                }
                if n.generator {
                    if writen {
                        w.write(b" ");
                    }
                    w.write(b"*");
                    writen = true;
                }
                if n.get {
                    if writen {
                        w.write(b" ");
                    }
                    w.write(b"get");
                    writen = true;
                }
                if n.set {
                    if writen {
                        w.write(b" ");
                    }
                    w.write(b"set");
                    writen = true;
                }
                if writen {
                    w.write(b" ");
                }
                self.property_name_js(&n.name, w);
                w.write(b" ");
                self.params_js(&n.params, w);
                w.write(b" ");
                self.node_js(n.body, w);
            }
            // Go: ast.go:ClassDecl.JS
            Node::ClassDecl(n) => {
                w.write(b"class");
                if n.name.is_some() {
                    w.write(b" ");
                    w.write_bytes(&self.var(n.name).data);
                }
                if n.extends.is_some() {
                    w.write(b" extends ");
                    self.node_js(n.extends, w);
                }
                if n.list.is_empty() {
                    w.write(b" {}");
                    return;
                }
                w.write(b" {");
                for item in &n.list {
                    let mut wi = w.indenter(4);
                    wi.write(b"\n");
                    // Go: ast.go:ClassElement.JS
                    if item.static_block.is_some() {
                        wi.write(b"static ");
                        self.node_js(item.static_block, &mut wi);
                        continue;
                    } else if item.method.is_some() {
                        self.node_js(item.method, &mut wi);
                        continue;
                    }
                    self.field_js(&item.field, &mut wi);
                    wi.write(b";");
                }
                w.write(b"\n}");
            }
            // Go: ast.go:Var.JS
            Node::Var(_) => {
                let name = self.var_name(id).clone();
                w.write_bytes(&name);
            }
            // Go: ast.go:BindingArray.JS
            Node::BindingArray(n) => {
                w.write(b"[");
                for (j, item) in n.list.iter().enumerate() {
                    if j != 0 {
                        w.write(b",");
                    }
                    if item.binding.is_some() {
                        if j != 0 {
                            w.write(b" ");
                        }
                        self.binding_element_js(item, w);
                    }
                }
                if n.rest.is_some() {
                    if !n.list.is_empty() {
                        w.write(b", ");
                    }
                    w.write(b"...");
                    self.node_js(n.rest, w);
                } else if !n.list.is_empty() && n.list[n.list.len() - 1].binding.is_nil() {
                    w.write(b",");
                }
                w.write(b"]");
            }
            // Go: ast.go:BindingObject.JS
            Node::BindingObject(n) => {
                w.write(b"{");
                for (j, item) in n.list.iter().enumerate() {
                    if j != 0 {
                        w.write(b", ");
                    }
                    // Go: ast.go:BindingObjectItem.JS
                    if let Some(key) = &item.key {
                        let ident = match self.node(item.value.binding) {
                            Node::Var(v) => key.is_ident(&v.data),
                            _ => false,
                        };
                        if !ident {
                            self.property_name_js(key, w);
                            w.write(b": ");
                        }
                    }
                    self.binding_element_js(&item.value, w);
                }
                if n.rest.is_some() {
                    if !n.list.is_empty() {
                        w.write(b", ");
                    }
                    w.write(b"...");
                    let data = self.var(n.rest).data.clone();
                    w.write_bytes(&data);
                }
                w.write(b"}");
            }
            // Go: ast.go:LiteralExpr.JS
            Node::LiteralExpr(n) => self.literal_js(n, w),
            // Go: ast.go:ArrayExpr.JS
            Node::ArrayExpr(n) => {
                w.write(b"[");
                for (j, item) in n.list.iter().enumerate() {
                    if j != 0 {
                        w.write(b", ");
                    }
                    if item.value.is_some() {
                        if item.spread {
                            w.write(b"...");
                        }
                        self.node_js(item.value, w);
                    }
                }
                if !n.list.is_empty() && n.list[n.list.len() - 1].value.is_nil() {
                    w.write(b",");
                }
                w.write(b"]");
            }
            // Go: ast.go:ObjectExpr.JS
            Node::ObjectExpr(n) => {
                w.write(b"{");
                for (j, item) in n.list.iter().enumerate() {
                    if j != 0 {
                        w.write(b", ");
                    }
                    self.property_js(item, w);
                }
                w.write(b"}");
            }
            // Go: ast.go:TemplateExpr.JS
            Node::TemplateExpr(n) => {
                let mut w = w.plain();
                if n.tag.is_some() {
                    self.node_js(n.tag, &mut w);
                    if n.optional {
                        w.write(b"?.");
                    }
                }
                for item in &n.list {
                    // Go: ast.go:TemplatePart.JS
                    w.write_bytes(&item.value);
                    self.node_js(item.expr, &mut w);
                }
                w.write_bytes(&n.tail);
            }
            // Go: ast.go:GroupExpr.JS
            Node::GroupExpr(n) => {
                w.write(b"(");
                self.node_js(n.x, w);
                w.write(b")");
            }
            // Go: ast.go:IndexExpr.JS
            Node::IndexExpr(n) => {
                self.node_js(n.x, w);
                if n.optional {
                    w.write(b"?.[");
                } else {
                    w.write(b"[");
                }
                self.node_js(n.y, w);
                w.write(b"]");
            }
            // Go: ast.go:DotExpr.JS
            Node::DotExpr(n) => {
                let group = match self.node(n.x) {
                    Node::LiteralExpr(lit) => {
                        !n.optional
                            && (lit.token_type == DecimalToken || lit.token_type == IntegerToken)
                    }
                    _ => false,
                };
                if group {
                    w.write(b"(");
                }
                self.node_js(n.x, w);
                if n.optional {
                    w.write(b"?.");
                } else {
                    if group {
                        w.write(b")");
                    }
                    w.write(b".");
                }
                self.literal_js(&n.y, w);
            }
            // Go: ast.go:NewTargetExpr.JS
            Node::NewTargetExpr => w.write(b"new.target"),
            // Go: ast.go:ImportMetaExpr.JS
            Node::ImportMetaExpr => w.write(b"import.meta"),
            // Go: ast.go:NewExpr.JS
            Node::NewExpr(n) => {
                w.write(b"new ");
                self.node_js(n.x, w);
                if let Some(args) = &n.args {
                    w.write(b"(");
                    self.args_js(args, w);
                    w.write(b")");
                } else {
                    w.write(b"()");
                }
            }
            // Go: ast.go:CallExpr.JS
            Node::CallExpr(n) => {
                self.node_js(n.x, w);
                if n.optional {
                    w.write(b"?.(");
                } else {
                    w.write(b"(");
                }
                self.args_js(&n.args, w);
                w.write(b")");
            }
            // Go: ast.go:UnaryExpr.JS
            Node::UnaryExpr(n) => {
                let op = n.op.bytes().unwrap_or(b"");
                if n.op == PostIncrToken || n.op == PostDecrToken {
                    self.node_js(n.x, w);
                    w.write(op);
                    return;
                }
                let spaced = match self.node(n.x) {
                    Node::UnaryExpr(unary) => {
                        n.op == PosToken && (unary.op == PreIncrToken || unary.op == PosToken)
                            || n.op == NegToken
                                && (unary.op == PreDecrToken || unary.op == NegToken)
                    }
                    _ => false,
                };
                if spaced || is_identifier_name(n.op) {
                    w.write(op);
                    w.write(b" ");
                    self.node_js(n.x, w);
                    return;
                }
                w.write(op);
                self.node_js(n.x, w);
            }
            // Go: ast.go:BinaryExpr.JS
            Node::BinaryExpr(n) => {
                self.node_js(n.x, w);
                w.write(b" ");
                w.write(n.op.bytes().unwrap_or(b""));
                w.write(b" ");
                self.node_js(n.y, w);
            }
            // Go: ast.go:CondExpr.JS
            Node::CondExpr(n) => {
                self.node_js(n.cond, w);
                w.write(b" ? ");
                self.node_js(n.x, w);
                w.write(b" : ");
                self.node_js(n.y, w);
            }
            // Go: ast.go:YieldExpr.JS
            Node::YieldExpr(n) => {
                w.write(b"yield");
                if n.x.is_nil() {
                    return;
                }
                if n.generator {
                    w.write(b"*");
                }
                w.write(b" ");
                self.node_js(n.x, w);
            }
            // Go: ast.go:ArrowFunc.JS
            Node::ArrowFunc(n) => {
                if n.async_ {
                    w.write(b"async ");
                }
                self.params_js(&n.params, w);
                w.write(b" => ");
                self.node_js(n.body, w);
            }
            // Go: ast.go:CommaExpr.JS
            Node::CommaExpr(n) => {
                for (j, &item) in n.list.iter().enumerate() {
                    if j != 0 {
                        w.write(b",");
                    }
                    self.node_js(item, w);
                }
            }
        }
    }

    // Go: ast.go:Property.JS
    fn property_js(&self, n: &Property, w: &mut JsWriter<'_>) {
        if let Some(name) = &n.name {
            let ident = match self.node(n.value) {
                Node::Var(v) => name.is_ident(&v.data),
                _ => false,
            };
            if !ident {
                self.property_name_js(name, w);
                w.write(b": ");
            }
        } else if n.spread {
            w.write(b"...");
        }
        self.node_js(n.value, w);
        if n.init.is_some() {
            w.write(b" = ");
            self.node_js(n.init, w);
        }
    }

    /// `_, ok := n.(JSONer)`
    fn is_jsoner(&self, id: NodeId) -> bool {
        matches!(
            self.node(id),
            Node::LiteralExpr(_)
                | Node::ArrayExpr(_)
                | Node::ObjectExpr(_)
                | Node::TemplateExpr(_)
                | Node::UnaryExpr(_)
        )
    }

    // Go: ast.go:LiteralExpr.JSON
    fn literal_json(&self, n: &LiteralExpr, w: &mut JsWriter<'_>) -> Result<(), Vec<u8>> {
        let mut w = w.plain();
        let tt = n.token_type;
        if tt == TrueToken
            || tt == FalseToken
            || tt == NullToken
            || tt == DecimalToken
            || tt == IntegerToken
        {
            w.write_bytes(&n.data);
            return Ok(());
        } else if tt == StringToken {
            let mut data = n.data.to_vec();
            if n.data.at(0) == b'\'' {
                data = replace_all(&data, b"\\'", b"'");
                data = replace_all(&data, b"\"", b"\\\"");
                data[0] = b'"';
                let l = data.len();
                data[l - 1] = b'"';
            }
            w.write(&data);
            return Ok(());
        }
        let mut js = Vec::new();
        self.literal_js(n, &mut JsWriter::new(&mut js));
        json_err_with("literal expression is not valid JSON", &js)
    }

    /// Go `val.JSON(w)` for a JSONer node.
    fn node_json(&self, id: NodeId, w: &mut JsWriter<'_>) -> Result<(), Vec<u8>> {
        match self.node(id) {
            Node::LiteralExpr(n) => self.literal_json(n, w),
            // Go: ast.go:ArrayExpr.JSON
            Node::ArrayExpr(n) => {
                w.write(b"[");
                for (i, item) in n.list.iter().enumerate() {
                    if i != 0 {
                        w.write(b", ");
                    }
                    if item.value.is_nil() || item.spread {
                        let js = self.node_js_string(id);
                        return json_err_with("array literal is not valid JSON", &js);
                    }
                    if !self.is_jsoner(item.value) {
                        let js = self.node_js_string(item.value);
                        return json_err_with("value is not valid JSON", &js);
                    }
                    self.node_json(item.value, w)?;
                }
                w.write(b"]");
                Ok(())
            }
            // Go: ast.go:ObjectExpr.JSON
            Node::ObjectExpr(n) => {
                w.write(b"{");
                for (i, item) in n.list.iter().enumerate() {
                    if i != 0 {
                        w.write(b", ");
                    }
                    self.property_json(item, w)?;
                }
                w.write(b"}");
                Ok(())
            }
            // Go: ast.go:TemplateExpr.JSON
            Node::TemplateExpr(n) => {
                let mut w = w.plain();
                if n.tag.is_some() || !n.list.is_empty() {
                    let js = self.node_js_string(id);
                    return json_err_with("value is not valid JSON", &js);
                }

                // allow template literal string to be converted to normal string (to allow for minified JS)
                let mut data = n.tail.to_vec();
                data = replace_all(&data, b"\n", b"\\n");
                data = replace_all(&data, b"\r", b"\\r");
                data = replace_all(&data, b"\\`", b"`");
                data = replace_all(&data, b"\\$", b"$");
                data = replace_all(&data, b"\"", b"\\\"");
                data[0] = b'"';
                let l = data.len();
                data[l - 1] = b'"';
                w.write(&data);
                Ok(())
            }
            // Go: ast.go:UnaryExpr.JSON
            Node::UnaryExpr(n) => {
                let lit = match self.node(n.x) {
                    Node::LiteralExpr(lit) => Some(lit),
                    _ => None,
                };
                if let Some(lit) = lit {
                    if n.op == NegToken
                        && (lit.token_type == DecimalToken || lit.token_type == IntegerToken)
                    {
                        w.write(b"-");
                        w.write_bytes(&lit.data);
                        return Ok(());
                    }
                }
                if n.op == NotToken {
                    // Go dereferences the (possibly nil) *LiteralExpr here
                    let lit = lit.expect("invalid memory address or nil pointer dereference");
                    if lit.token_type == IntegerToken
                        && (lit.data.at(0) == b'0' || lit.data.at(0) == b'1')
                    {
                        if lit.data.at(0) == b'0' {
                            w.write(b"true");
                        } else {
                            w.write(b"false");
                        }
                        return Ok(());
                    }
                }
                let js = self.node_js_string(id);
                json_err_with("unary expression is not valid JSON", &js)
            }
            _ => unreachable!("not a JSONer"),
        }
    }

    // Go: ast.go:Property.JSON
    fn property_json(&self, n: &Property, w: &mut JsWriter<'_>) -> Result<(), Vec<u8>> {
        let js_of = |p: &Property| {
            let mut js = Vec::new();
            self.property_js(p, &mut JsWriter::new(&mut js));
            js
        };
        let Some(name) = n.name.as_ref().filter(|_| !n.spread && n.init.is_nil()) else {
            return json_err_with("property is not valid JSON", &js_of(n));
        };
        let tt = name.literal.token_type;
        if tt == StringToken {
            let _ = self.literal_json(&name.literal, w);
        } else if tt == IdentifierToken || tt == IntegerToken || tt == DecimalToken {
            w.write(b"\"");
            w.write_bytes(&name.literal.data);
            w.write(b"\"");
        } else {
            return json_err_with("property is not valid JSON", &js_of(n));
        }
        w.write(b": ");

        if !self.is_jsoner(n.value) {
            let js = self.node_js_string(n.value);
            return json_err_with("value is not valid JSON", &js);
        }
        self.node_json(n.value, w)
    }
}

/// Go `bytes.ReplaceAll(s, old, new)` for a non-empty `old`.
fn replace_all(s: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with(old) {
            out.extend_from_slice(new);
            i += old.len();
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}
