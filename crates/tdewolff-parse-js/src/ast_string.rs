//! Go: parse/v2/js/ast.go — the `String()` methods (a debug representation
//! used by the upstream tests). Go strings are bytes, so these return
//! `Vec<u8>`.

use tdewolff_parse::GoBytes;

use crate::ast::*;
use crate::tokentype::*;

fn app(s: &mut Vec<u8>, b: &[u8]) {
    s.extend_from_slice(b);
}

fn app_bytes(s: &mut Vec<u8>, b: &GoBytes) {
    b.write_to(s);
}

impl Ast {
    // Go: ast.go:AST.String
    pub fn string(&self) -> Vec<u8> {
        let mut s = Vec::new();
        for (i, &item) in self.block(self.block_stmt).list.iter().enumerate() {
            if i != 0 {
                app(&mut s, b" ");
            }
            self.w(&mut s, item);
        }
        s
    }

    /// Go `n.String()` for any node.
    pub fn node_string(&self, id: NodeId) -> Vec<u8> {
        let mut s = Vec::new();
        self.w(&mut s, id);
        s
    }

    // Go: ast.go:VarArray.String
    pub fn var_array_string(&self, vs: &[NodeId]) -> Vec<u8> {
        let mut s = b"[".to_vec();
        for (i, &v) in vs.iter().enumerate() {
            if i != 0 {
                app(&mut s, b", ");
            }
            let mut v = v;
            let mut links = 0;
            while self.var(v).link.is_some() {
                v = self.var(v).link;
                links += 1;
            }
            let vv = self.var(v);
            app(&mut s, b"Var{");
            app(&mut s, vv.decl.string().as_bytes());
            app(&mut s, b" ");
            app_bytes(&mut s, &vv.data);
            app(&mut s, format!(" {} {}", links, vv.uses).as_bytes());
            app(&mut s, b"}");
        }
        app(&mut s, b"]");
        s
    }

    // Go: ast.go:Scope.String
    pub fn scope_string(&self, sc: ScopeId) -> Vec<u8> {
        let scope = self.scope(sc);
        let mut s = b"Scope{Declared: ".to_vec();
        s.extend(self.var_array_string(&scope.declared));
        app(&mut s, b", Undeclared: ");
        s.extend(self.var_array_string(&scope.undeclared));
        app(&mut s, b"}");
        s
    }

    // Go: ast.go:PropertyName.String
    pub fn property_name_string(&self, n: &PropertyName) -> Vec<u8> {
        if n.computed.is_some() {
            let val = self.node_string(n.computed);
            if val[0] == b'(' {
                let mut s = b"[".to_vec();
                app(&mut s, &val[1..val.len() - 1]);
                app(&mut s, b"]");
                return s;
            }
            let mut s = b"[".to_vec();
            app(&mut s, &val);
            app(&mut s, b"]");
            return s;
        }
        n.literal.data.to_vec()
    }

    // Go: ast.go:BindingElement.String
    fn w_binding_element(&self, s: &mut Vec<u8>, n: &BindingElement) {
        if n.binding.is_nil() {
            app(s, b"Binding()");
            return;
        }
        app(s, b"Binding(");
        self.w(s, n.binding);
        if n.default.is_some() {
            app(s, b" = ");
            self.w(s, n.default);
        }
        app(s, b")");
    }

    // Go: ast.go:Params.String
    fn w_params(&self, s: &mut Vec<u8>, n: &Params) {
        app(s, b"Params(");
        for (i, item) in n.list.iter().enumerate() {
            if i != 0 {
                app(s, b", ");
            }
            self.w_binding_element(s, item);
        }
        if n.rest.is_some() {
            if !n.list.is_empty() {
                app(s, b", ");
            }
            app(s, b"...Binding(");
            self.w(s, n.rest);
            app(s, b")");
        }
        app(s, b")");
    }

    // Go: ast.go:Alias.String
    fn w_alias(&self, s: &mut Vec<u8>, alias: &Alias) {
        if !alias.name.is_nil() {
            app_bytes(s, &alias.name);
            app(s, b" as ");
        }
        app_bytes(s, &alias.binding);
    }

    // Go: ast.go:Args.String
    fn w_args(&self, s: &mut Vec<u8>, n: &Args) {
        app(s, b"(");
        for (i, item) in n.list.iter().enumerate() {
            if i != 0 {
                app(s, b", ");
            }
            // Go: ast.go:Arg.String
            if item.rest {
                app(s, b"...");
            }
            self.w(s, item.value);
        }
        app(s, b")");
    }

    // Go: ast.go:Field.String
    fn w_field(&self, s: &mut Vec<u8>, n: &Field) {
        app(s, b"Field(");
        if n.static_ {
            app(s, b"static ");
        }
        s.extend(self.property_name_string(&n.name));
        if n.init.is_some() {
            app(s, b" = ");
            self.w(s, n.init);
        }
        app(s, b")");
    }

    /// Writes Go's `n.String()`; a nil node panics as in Go.
    fn w(&self, s: &mut Vec<u8>, id: NodeId) {
        match self.node(id) {
            Node::Nil => panic!("invalid memory address or nil pointer dereference"),
            // Go: ast.go:Comment.String
            Node::Comment(n) => {
                app(s, b"Stmt(");
                app_bytes(s, &n.value);
                app(s, b")");
            }
            // Go: ast.go:BlockStmt.String
            Node::BlockStmt(n) => {
                app(s, b"Stmt({");
                for &item in &n.list {
                    app(s, b" ");
                    self.w(s, item);
                }
                app(s, b" })");
            }
            // Go: ast.go:EmptyStmt.String
            Node::EmptyStmt => app(s, b"Stmt()"),
            // Go: ast.go:ExprStmt.String
            Node::ExprStmt(n) => {
                let val = self.node_string(n.value);
                if val[0] == b'(' && val[val.len() - 1] == b')' {
                    app(s, b"Stmt");
                    app(s, &val);
                } else {
                    app(s, b"Stmt(");
                    app(s, &val);
                    app(s, b")");
                }
            }
            // Go: ast.go:IfStmt.String
            Node::IfStmt(n) => {
                app(s, b"Stmt(if ");
                self.w(s, n.cond);
                app(s, b" ");
                self.w(s, n.body);
                if n.else_.is_some() {
                    app(s, b" else ");
                    self.w(s, n.else_);
                }
                app(s, b")");
            }
            // Go: ast.go:DoWhileStmt.String
            Node::DoWhileStmt(n) => {
                app(s, b"Stmt(do ");
                self.w(s, n.body);
                app(s, b" while ");
                self.w(s, n.cond);
                app(s, b")");
            }
            // Go: ast.go:WhileStmt.String
            Node::WhileStmt(n) => {
                app(s, b"Stmt(while ");
                self.w(s, n.cond);
                app(s, b" ");
                self.w(s, n.body);
                app(s, b")");
            }
            // Go: ast.go:ForStmt.String
            Node::ForStmt(n) => {
                app(s, b"Stmt(for");
                let print_init = match self.node(n.init) {
                    Node::VarDecl(v) => !v.list.is_empty(),
                    _ => n.init.is_some(),
                };
                if print_init {
                    app(s, b" ");
                    self.w(s, n.init);
                }
                app(s, b" ;");
                if n.cond.is_some() {
                    app(s, b" ");
                    self.w(s, n.cond);
                }
                app(s, b" ;");
                if n.post.is_some() {
                    app(s, b" ");
                    self.w(s, n.post);
                }
                app(s, b" ");
                self.w(s, n.body);
                app(s, b")");
            }
            // Go: ast.go:ForInStmt.String
            Node::ForInStmt(n) => {
                app(s, b"Stmt(for ");
                self.w(s, n.init);
                app(s, b" in ");
                self.w(s, n.value);
                app(s, b" ");
                self.w(s, n.body);
                app(s, b")");
            }
            // Go: ast.go:ForOfStmt.String
            Node::ForOfStmt(n) => {
                app(s, b"Stmt(for");
                if n.await_ {
                    app(s, b" await");
                }
                app(s, b" ");
                self.w(s, n.init);
                app(s, b" of ");
                self.w(s, n.value);
                app(s, b" ");
                self.w(s, n.body);
                app(s, b")");
            }
            // Go: ast.go:SwitchStmt.String
            Node::SwitchStmt(n) => {
                app(s, b"Stmt(switch ");
                self.w(s, n.init);
                for clause in &n.list {
                    // Go: ast.go:CaseClause.String
                    app(s, b" Clause(");
                    app(s, clause.token_type.string().as_bytes());
                    if clause.cond.is_some() {
                        app(s, b" ");
                        self.w(s, clause.cond);
                    }
                    for &item in &clause.list {
                        app(s, b" ");
                        self.w(s, item);
                    }
                    app(s, b")");
                }
                app(s, b")");
            }
            // Go: ast.go:BranchStmt.String
            Node::BranchStmt(n) => {
                app(s, b"Stmt(");
                app(s, n.type_.string().as_bytes());
                if !n.label.is_nil() {
                    app(s, b" ");
                    app_bytes(s, &n.label);
                }
                app(s, b")");
            }
            // Go: ast.go:ReturnStmt.String
            Node::ReturnStmt(n) => {
                app(s, b"Stmt(return");
                if n.value.is_some() {
                    app(s, b" ");
                    self.w(s, n.value);
                }
                app(s, b")");
            }
            // Go: ast.go:WithStmt.String
            Node::WithStmt(n) => {
                app(s, b"Stmt(with ");
                self.w(s, n.cond);
                app(s, b" ");
                self.w(s, n.body);
                app(s, b")");
            }
            // Go: ast.go:LabelledStmt.String
            Node::LabelledStmt(n) => {
                app(s, b"Stmt(");
                app_bytes(s, &n.label);
                app(s, b" : ");
                self.w(s, n.value);
                app(s, b")");
            }
            // Go: ast.go:ThrowStmt.String
            Node::ThrowStmt(n) => {
                app(s, b"Stmt(throw ");
                self.w(s, n.value);
                app(s, b")");
            }
            // Go: ast.go:TryStmt.String
            Node::TryStmt(n) => {
                app(s, b"Stmt(try ");
                self.w(s, n.body);
                if n.catch.is_some() {
                    app(s, b" catch");
                    if n.binding.is_some() {
                        app(s, b" Binding(");
                        self.w(s, n.binding);
                        app(s, b")");
                    }
                    app(s, b" ");
                    self.w(s, n.catch);
                }
                if n.finally.is_some() {
                    app(s, b" finally ");
                    self.w(s, n.finally);
                }
                app(s, b")");
            }
            // Go: ast.go:DebuggerStmt.String
            Node::DebuggerStmt => app(s, b"Stmt(debugger)"),
            // Go: ast.go:ImportStmt.String
            Node::ImportStmt(n) => {
                app(s, b"Stmt(import");
                if !n.default.is_nil() {
                    app(s, b" ");
                    app_bytes(s, &n.default);
                    if n.list.is_some() {
                        app(s, b" ,");
                    }
                }
                let list: &[Alias] = n.list.as_deref().unwrap_or(&[]);
                if list.len() == 1 && list[0].name.len() == 1 && list[0].name.at(0) == b'*' {
                    app(s, b" ");
                    self.w_alias(s, &list[0]);
                } else if n.list.is_some() {
                    app(s, b" {");
                    for (i, item) in list.iter().enumerate() {
                        if i != 0 {
                            app(s, b" ,");
                        }
                        if !item.binding.is_nil() {
                            app(s, b" ");
                            self.w_alias(s, item);
                        }
                    }
                    app(s, b" }");
                }
                if !n.default.is_nil() || n.list.is_some() {
                    app(s, b" from");
                }
                app(s, b" ");
                app_bytes(s, &n.module);
                app(s, b")");
            }
            // Go: ast.go:ExportStmt.String
            Node::ExportStmt(n) => {
                app(s, b"Stmt(export");
                if n.decl.is_some() {
                    if n.default {
                        app(s, b" default");
                    }
                    app(s, b" ");
                    self.w(s, n.decl);
                    app(s, b")");
                    return;
                } else if n.list.len() == 1
                    && (n.list[0].name.len() == 1 && n.list[0].name.at(0) == b'*'
                        || n.list[0].name.is_nil()
                            && n.list[0].binding.len() == 1
                            && n.list[0].binding.at(0) == b'*')
                {
                    app(s, b" ");
                    self.w_alias(s, &n.list[0]);
                } else if !n.list.is_empty() {
                    app(s, b" {");
                    for (i, item) in n.list.iter().enumerate() {
                        if i != 0 {
                            app(s, b" ,");
                        }
                        if !item.binding.is_nil() {
                            app(s, b" ");
                            self.w_alias(s, item);
                        }
                    }
                    app(s, b" }");
                }
                if !n.module.is_nil() {
                    app(s, b" from ");
                    app_bytes(s, &n.module);
                }
                app(s, b")");
            }
            // Go: ast.go:DirectivePrologueStmt.String
            Node::DirectivePrologueStmt(n) => {
                app(s, b"Stmt(");
                app_bytes(s, &n.value);
                app(s, b")");
            }
            // Go: ast.go:VarDecl.String
            Node::VarDecl(n) => {
                app(s, b"Decl(");
                app(s, n.token_type.string().as_bytes());
                for item in &n.list {
                    app(s, b" ");
                    self.w_binding_element(s, item);
                }
                app(s, b")");
            }
            // Go: ast.go:FuncDecl.String
            Node::FuncDecl(n) => {
                app(s, b"Decl(");
                if n.async_ {
                    app(s, b"async function");
                } else {
                    app(s, b"function");
                }
                if n.generator {
                    app(s, b"*");
                }
                if n.name.is_some() {
                    app(s, b" ");
                    app_bytes(s, &self.var(n.name).data);
                }
                app(s, b" ");
                self.w_params(s, &n.params);
                app(s, b" ");
                self.w(s, n.body);
                app(s, b")");
            }
            // Go: ast.go:MethodDecl.String
            Node::MethodDecl(n) => {
                let mut t = Vec::new();
                if n.static_ {
                    app(&mut t, b" static");
                }
                if n.async_ {
                    app(&mut t, b" async");
                }
                if n.generator {
                    app(&mut t, b" *");
                }
                if n.get {
                    app(&mut t, b" get");
                }
                if n.set {
                    app(&mut t, b" set");
                }
                app(&mut t, b" ");
                t.extend(self.property_name_string(&n.name));
                app(&mut t, b" ");
                self.w_params(&mut t, &n.params);
                app(&mut t, b" ");
                self.w(&mut t, n.body);
                app(s, b"Method(");
                app(s, &t[1..]);
                app(s, b")");
            }
            // Go: ast.go:ClassDecl.String
            Node::ClassDecl(n) => {
                app(s, b"Decl(class");
                if n.name.is_some() {
                    app(s, b" ");
                    app_bytes(s, &self.var(n.name).data);
                }
                if n.extends.is_some() {
                    app(s, b" extends ");
                    self.w(s, n.extends);
                }
                for item in &n.list {
                    app(s, b" ");
                    // Go: ast.go:ClassElement.String
                    if item.static_block.is_some() {
                        app(s, b"Static(");
                        self.w(s, item.static_block);
                        app(s, b")");
                    } else if item.method.is_some() {
                        self.w(s, item.method);
                    } else {
                        self.w_field(s, &item.field);
                    }
                }
                app(s, b")");
            }
            // Go: ast.go:Var.String
            Node::Var(_) => {
                app_bytes(s, self.var_name(id));
            }
            // Go: ast.go:BindingArray.String
            Node::BindingArray(n) => {
                app(s, b"[");
                for (i, item) in n.list.iter().enumerate() {
                    if i != 0 {
                        app(s, b",");
                    }
                    app(s, b" ");
                    self.w_binding_element(s, item);
                }
                if n.rest.is_some() {
                    if !n.list.is_empty() {
                        app(s, b",");
                    }
                    app(s, b" ...Binding(");
                    self.w(s, n.rest);
                    app(s, b")");
                } else if !n.list.is_empty() && n.list[n.list.len() - 1].binding.is_nil() {
                    app(s, b",");
                }
                app(s, b" ]");
            }
            // Go: ast.go:BindingObject.String
            Node::BindingObject(n) => {
                app(s, b"{");
                for (i, item) in n.list.iter().enumerate() {
                    if i != 0 {
                        app(s, b",");
                    }
                    // Go: ast.go:BindingObjectItem.String
                    if let Some(key) = &item.key {
                        let ident = match self.node(item.value.binding) {
                            Node::Var(v) => key.is_ident(&v.data),
                            _ => false,
                        };
                        if !ident {
                            app(s, b" ");
                            s.extend(self.property_name_string(key));
                            app(s, b":");
                        }
                    }
                    app(s, b" ");
                    self.w_binding_element(s, &item.value);
                }
                if n.rest.is_some() {
                    if !n.list.is_empty() {
                        app(s, b",");
                    }
                    app(s, b" ...Binding(");
                    app_bytes(s, &self.var(n.rest).data);
                    app(s, b")");
                }
                app(s, b" }");
            }
            // Go: ast.go:LiteralExpr.String
            Node::LiteralExpr(n) => app_bytes(s, &n.data),
            // Go: ast.go:ArrayExpr.String
            Node::ArrayExpr(n) => {
                app(s, b"[");
                for (i, item) in n.list.iter().enumerate() {
                    if i != 0 {
                        app(s, b", ");
                    }
                    if item.value.is_some() {
                        if item.spread {
                            app(s, b"...");
                        }
                        self.w(s, item.value);
                    }
                }
                if !n.list.is_empty() && n.list[n.list.len() - 1].value.is_nil() {
                    app(s, b",");
                }
                app(s, b"]");
            }
            // Go: ast.go:ObjectExpr.String
            Node::ObjectExpr(n) => {
                app(s, b"{");
                for (i, item) in n.list.iter().enumerate() {
                    if i != 0 {
                        app(s, b", ");
                    }
                    // Go: ast.go:Property.String
                    if let Some(name) = &item.name {
                        let ident = match self.node(item.value) {
                            Node::Var(v) => name.is_ident(&v.data),
                            _ => false,
                        };
                        if !ident {
                            s.extend(self.property_name_string(name));
                            app(s, b": ");
                        }
                    } else if item.spread {
                        app(s, b"...");
                    }
                    self.w(s, item.value);
                    if item.init.is_some() {
                        app(s, b" = ");
                        self.w(s, item.init);
                    }
                }
                app(s, b"}");
            }
            // Go: ast.go:TemplateExpr.String
            Node::TemplateExpr(n) => {
                if n.tag.is_some() {
                    self.w(s, n.tag);
                    if n.optional {
                        app(s, b"?.");
                    }
                }
                for item in &n.list {
                    // Go: ast.go:TemplatePart.String
                    app_bytes(s, &item.value);
                    self.w(s, item.expr);
                }
                app_bytes(s, &n.tail);
            }
            // Go: ast.go:GroupExpr.String
            Node::GroupExpr(n) => {
                app(s, b"(");
                self.w(s, n.x);
                app(s, b")");
            }
            // Go: ast.go:IndexExpr.String
            Node::IndexExpr(n) => {
                app(s, b"(");
                self.w(s, n.x);
                if n.optional {
                    app(s, b"?.[");
                } else {
                    app(s, b"[");
                }
                self.w(s, n.y);
                app(s, b"])");
            }
            // Go: ast.go:DotExpr.String
            Node::DotExpr(n) => {
                app(s, b"(");
                self.w(s, n.x);
                if n.optional {
                    app(s, b"?.");
                } else {
                    app(s, b".");
                }
                app_bytes(s, &n.y.data);
                app(s, b")");
            }
            // Go: ast.go:NewTargetExpr.String
            Node::NewTargetExpr => app(s, b"(new.target)"),
            // Go: ast.go:ImportMetaExpr.String
            Node::ImportMetaExpr => app(s, b"(import.meta)"),
            // Go: ast.go:NewExpr.String
            Node::NewExpr(n) => {
                app(s, b"(new ");
                self.w(s, n.x);
                if let Some(args) = &n.args {
                    self.w_args(s, args);
                }
                app(s, b")");
            }
            // Go: ast.go:CallExpr.String
            Node::CallExpr(n) => {
                app(s, b"(");
                self.w(s, n.x);
                if n.optional {
                    app(s, b"?.");
                }
                self.w_args(s, &n.args);
                app(s, b")");
            }
            // Go: ast.go:UnaryExpr.String
            Node::UnaryExpr(n) => {
                if n.op == PostIncrToken || n.op == PostDecrToken {
                    app(s, b"(");
                    self.w(s, n.x);
                    app(s, n.op.string().as_bytes());
                    app(s, b")");
                } else if is_identifier_name(n.op) {
                    app(s, b"(");
                    app(s, n.op.string().as_bytes());
                    app(s, b" ");
                    self.w(s, n.x);
                    app(s, b")");
                } else {
                    app(s, b"(");
                    app(s, n.op.string().as_bytes());
                    self.w(s, n.x);
                    app(s, b")");
                }
            }
            // Go: ast.go:BinaryExpr.String
            Node::BinaryExpr(n) => {
                app(s, b"(");
                self.w(s, n.x);
                if is_identifier_name(n.op) {
                    app(s, b" ");
                    app(s, n.op.string().as_bytes());
                    app(s, b" ");
                } else {
                    app(s, n.op.string().as_bytes());
                }
                self.w(s, n.y);
                app(s, b")");
            }
            // Go: ast.go:CondExpr.String
            Node::CondExpr(n) => {
                app(s, b"(");
                self.w(s, n.cond);
                app(s, b" ? ");
                self.w(s, n.x);
                app(s, b" : ");
                self.w(s, n.y);
                app(s, b")");
            }
            // Go: ast.go:YieldExpr.String
            Node::YieldExpr(n) => {
                if n.x.is_nil() {
                    app(s, b"(yield)");
                    return;
                }
                app(s, b"(yield");
                if n.generator {
                    app(s, b"*");
                }
                app(s, b" ");
                self.w(s, n.x);
                app(s, b")");
            }
            // Go: ast.go:ArrowFunc.String
            Node::ArrowFunc(n) => {
                app(s, b"(");
                if n.async_ {
                    app(s, b"async ");
                }
                self.w_params(s, &n.params);
                app(s, b" => ");
                self.w(s, n.body);
                app(s, b")");
            }
            // Go: ast.go:CommaExpr.String
            Node::CommaExpr(n) => {
                app(s, b"(");
                for (i, &item) in n.list.iter().enumerate() {
                    if i != 0 {
                        app(s, b",");
                    }
                    self.w(s, item);
                }
                app(s, b")");
            }
        }
    }
}
