//! Go: minify/v2@v2.23.8/js/vars.go — the variable renamer, var
//! declaration merging and `hoistVars`.

use tdewolff_parse::GoBytes;
use tdewolff_parse_js::*;

use crate::js::JsMinifier;

pub(crate) const IDENT_START_LEN: usize = 54;
pub(crate) const IDENT_CONTINUE_LEN: usize = 64;

/// Go: vars.go:renamer
pub(crate) struct Renamer {
    pub(crate) ident_start: &'static [u8],
    pub(crate) ident_continue: &'static [u8],
    /// Go `map[byte]int`; a missing key reads as 0.
    pub(crate) ident_order: [i64; 256],
    pub(crate) rename: bool,
}

impl Renamer {
    // Go: vars.go:newRenamer
    pub(crate) fn new(rename: bool, use_char_freq: bool) -> Renamer {
        // reserved = js.Keywords (looked up with tdewolff_parse_js::keyword)
        let mut ident_start: &'static [u8] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_$";
        let mut ident_continue: &'static [u8] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_$0123456789";
        if use_char_freq {
            // sorted based on character frequency of a collection of JS samples
            ident_start = b"etnsoiarclduhmfpgvbjy_wOxCEkASMFTzDNLRPHIBV$WUKqYGXQZJ";
            ident_continue = b"etnsoiarcldu14023hm8f6pg57v9bjy_wOxCEkASMFTzDNLRPHIBV$WUKqYGXQZJ";
        }
        if ident_start.len() != IDENT_START_LEN || ident_continue.len() != IDENT_CONTINUE_LEN {
            panic!("bad identStart or identContinue lengths");
        }
        let mut ident_order = [0i64; 256];
        for (i, &c) in ident_start.iter().enumerate() {
            ident_order[c as usize] = i as i64;
        }
        Renamer {
            ident_start,
            ident_continue,
            ident_order,
            rename,
        }
    }

    // Go: vars.go:renamer.renameScope
    /// `scope` is passed by value in Go; only the shared `Declared` array
    /// (sorted in place) and the `*Var`s are written.
    pub(crate) fn rename_scope(&self, ast: &mut Ast, scope: ScopeId) {
        if !self.rename {
            return;
        }

        let mut i = 0usize;
        // keep function argument declaration order to improve GZIP compression
        let from = ast.scope(scope).num_func_args as usize;
        ast.sort_vars_by_uses(scope, from);
        let declared = ast.scope(scope).declared.clone();
        for v in declared {
            let d = ast.var(v).data.clone();
            ast.var_mut(v).data = self.get_name(d, i);
            i += 1;
            while self.is_reserved(ast, &ast.var(v).data, &ast.scope(scope).undeclared) {
                let d = ast.var(v).data.clone();
                ast.var_mut(v).data = self.get_name(d, i);
                i += 1;
            }
        }
    }

    // Go: vars.go:renamer.isReserved
    pub(crate) fn is_reserved(&self, ast: &Ast, name: &GoBytes, undeclared: &[NodeId]) -> bool {
        if 1 < name.len() {
            // there are no keywords or known globals that are one character long
            if keyword(&name.to_vec()).is_some() {
                return true;
            }
        }
        for &v in undeclared {
            let mut v = v;
            while ast.var(v).link.is_some() {
                v = ast.var(v).link;
            }
            if ast.var(v).data == *name {
                return true;
            }
        }
        false
    }

    // Go: vars.go:renamer.getIndex
    /// The inverse of `get_name` (only used by the tests, as in Go).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn get_index(&self, name: &[u8]) -> i64 {
        let mut index: i64 = 0;
        'name_loop: for i in (0..name.len()).rev() {
            let chars = if i == 0 {
                index = index.wrapping_mul(IDENT_START_LEN as i64);
                self.ident_start
            } else {
                index = index.wrapping_mul(IDENT_CONTINUE_LEN as i64);
                self.ident_continue
            };
            for (j, &c) in chars.iter().enumerate() {
                if name[i] == c {
                    index = index.wrapping_add(j as i64);
                    continue 'name_loop;
                }
            }
            return -1;
        }
        for n in 0..name.len().saturating_sub(1) {
            let mut offset = IDENT_START_LEN as i64;
            for _ in 0..n {
                offset = offset.wrapping_mul(IDENT_CONTINUE_LEN as i64);
            }
            index = index.wrapping_add(offset);
        }
        index
    }

    // Go: vars.go:renamer.getName
    /// Writes the new name into `name` in place when it fits (as Go does:
    /// `name[0] = ...`, `name[:n]` within capacity), else allocates.
    pub(crate) fn get_name(&self, name: GoBytes, index: usize) -> GoBytes {
        // Generate new names for variables where the last character is (a-zA-Z$_) and others are (a-zA-Z).
        // Thus we can have 54 one-character names and 52*54=2808 two-character names for every branch leaf.
        // That is sufficient for virtually all input.
        let mut index = index;

        // one character
        if index < IDENT_START_LEN {
            name.set(0, self.ident_start[index]);
            return name.slice_to(1);
        }
        index -= IDENT_START_LEN;

        // two characters or more
        let mut n = 2usize;
        loop {
            let mut offset = IDENT_START_LEN;
            for _ in 0..n - 1 {
                offset *= IDENT_CONTINUE_LEN;
            }
            if index < offset {
                break;
            }
            index -= offset;
            n += 1;
        }

        let name = if name.cap() < n {
            GoBytes::make(n, n)
        } else {
            name.slice_to(n)
        };
        name.set(0, self.ident_start[index % IDENT_START_LEN]);
        index /= IDENT_START_LEN;
        for i in 1..n {
            name.set(i, self.ident_continue[index % IDENT_CONTINUE_LEN]);
            index /= IDENT_CONTINUE_LEN;
        }
        name
    }
}

////////////////////////////////////////////////////////////////

// Go: vars.go:hasDefines
pub(crate) fn has_defines(ast: &Ast, v: NodeId) -> bool {
    for item in &ast.var_decl(v).list {
        if item.default.is_some() {
            return true;
        }
    }
    false
}

// Go: vars.go:bindingVars
pub(crate) fn binding_vars(ast: &Ast, ibinding: NodeId) -> Vec<NodeId> {
    let mut vs = Vec::new();
    binding_vars_into(ast, ibinding, &mut vs);
    vs
}

fn binding_vars_into(ast: &Ast, ibinding: NodeId, vs: &mut Vec<NodeId>) {
    match ast.node(ibinding) {
        Node::Var(_) => vs.push(ibinding),
        Node::BindingArray(binding) => {
            for item in &binding.list {
                if item.binding.is_some() {
                    binding_vars_into(ast, item.binding, vs);
                }
            }
            if binding.rest.is_some() {
                binding_vars_into(ast, binding.rest, vs);
            }
        }
        Node::BindingObject(binding) => {
            for item in &binding.list {
                if item.value.binding.is_some() {
                    binding_vars_into(ast, item.value.binding, vs);
                }
            }
            if binding.rest.is_some() {
                vs.push(binding.rest);
            }
        }
        _ => {}
    }
}

/// `v, ok := item.Binding.(*js.Var); ok && item.Default == nil && v == vbind`
#[inline]
fn is_plain_binding_of(ast: &Ast, item: &BindingElement, vbind: NodeId) -> bool {
    ast.is_var(item.binding) && item.default.is_nil() && item.binding == vbind
}

// Go: vars.go:addDefinition
pub(crate) fn add_definition(
    ast: &mut Ast,
    decl: NodeId,
    binding: NodeId,
    value: NodeId,
    forward: bool,
) {
    if ast.var_decl(decl).token_type != ErrorToken {
        // see if not already defined in variable declaration list
        // if forward is set, binding=value comes before decl, otherwise the reverse holds true
        let vars = binding_vars(ast, binding);

        // remove variables in destination
        'remove_vars_loop: for vbind in vars {
            let n = ast.var_decl(decl).list.len();
            for i in 0..n {
                let item = ast.var_decl(decl).list[i];
                if is_plain_binding_of(ast, &item, vbind) {
                    let v = ast.var_mut(vbind);
                    v.uses = v.uses.wrapping_sub(1);
                    ast.var_decl_mut(decl).list.remove(i);
                    continue 'remove_vars_loop;
                }
            }

            if value.is_some() {
                // variable declaration must be somewhere else, find and remove it
                let func = ast.scope(ast.var_decl(decl).scope).func;
                let var_decls = ast.scope(func).var_decls.clone();
                for decl2 in var_decls {
                    if !ast.var_decl(decl2).in_for_in_of {
                        let n = ast.var_decl(decl2).list.len();
                        for i in 0..n {
                            let item = ast.var_decl(decl2).list[i];
                            if is_plain_binding_of(ast, &item, vbind) {
                                let v = ast.var_mut(vbind);
                                v.uses = v.uses.wrapping_sub(1);
                                ast.var_decl_mut(decl2).list.remove(i);
                                continue 'remove_vars_loop;
                            }
                        }
                    }
                }
            }
        }
    }

    // add declaration to destination
    let item = BindingElement {
        binding,
        default: value,
    };
    let list = &mut ast.var_decl_mut(decl).list;
    if forward {
        list.insert(0, item);
    } else {
        list.push(item);
    }
}

// Go: vars.go:mergeVarDecls
/// Merges var declarations by moving declarations from src to dst. If forward
/// is set, src comes first and dst after, otherwise the order is reverse.
pub(crate) fn merge_var_decls(ast: &mut Ast, dst: NodeId, src: NodeId, forward: bool) {
    if forward {
        // reverse order so we can iterate from beginning to end, sometimes addDefinition may remove another declaration in the src list
        let list = &mut ast.var_decl_mut(src).list;
        let n = list.len() as isize - 1;
        let mut j: isize = 0;
        while j < list.len() as isize / 2 {
            list.swap(j as usize, (n - j) as usize);
            j += 1;
        }
    }
    let mut j = 0usize;
    while j < ast.var_decl(src).list.len() {
        let item = ast.var_decl(src).list[j];
        add_definition(ast, dst, item.binding, item.default, forward);
        j += 1;
    }
    ast.var_decl_mut(src).list.clear();
}

// Go: vars.go:mergeVarDeclExprStmt
/// Merges var declarations with an assignment expression. If forward is set
/// then expr comes first and decl after, otherwise the order is reverse.
pub(crate) fn merge_var_decl_expr_stmt(
    ast: &mut Ast,
    decl: NodeId,
    expr_stmt: NodeId,
    forward: bool,
) -> bool {
    let value = match ast.node(expr_stmt) {
        Node::ExprStmt(e) => e.value,
        _ => unreachable!(),
    };
    match ast.node(value) {
        Node::VarDecl(_) => {
            // this happens when a variable declarations is converted to an expression due to hoisting
            merge_var_decls(ast, decl, value, forward);
            true
        }
        Node::CommaExpr(_) => {
            let comma = value;
            let mut n = 0usize;
            let len = |ast: &Ast| match ast.node(comma) {
                Node::CommaExpr(c) => c.list.len(),
                _ => unreachable!(),
            };
            let at = |ast: &Ast, k: usize| match ast.node(comma) {
                Node::CommaExpr(c) => c.list[k],
                _ => unreachable!(),
            };
            let mut i = 0usize;
            while i < len(ast) {
                let mut item = at(ast, i);
                if forward {
                    item = at(ast, len(ast) - i - 1);
                }
                if let Node::VarDecl(_) = ast.node(item) {
                    // this happens when a variable declarations is converted to an expression due to hoisting
                    merge_var_decls(ast, decl, item, forward);
                    n += 1;
                    i += 1;
                    continue;
                } else if let Node::BinaryExpr(b) = ast.node(item) {
                    if b.op == EqToken {
                        let (bx, by) = (b.x, b.y);
                        if let Node::Var(v) = ast.node(bx) {
                            if v.decl == VariableDecl {
                                add_definition(ast, decl, bx, by, forward);
                                n += 1;
                                i += 1;
                                continue;
                            }
                        }
                    }
                }
                break;
            }
            let total = len(ast);
            let merge = n == total;
            let list = &mut crate::util::comma_mut(ast, comma).list;
            if !forward {
                list.drain(..n);
            } else {
                list.truncate(total - n);
            }
            merge
        }
        Node::BinaryExpr(b) if b.op == EqToken => {
            let (bx, by) = (b.x, b.y);
            if let Node::Var(v) = ast.node(bx) {
                if v.decl == VariableDecl {
                    add_definition(ast, decl, bx, by, forward);
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

impl JsMinifier<'_> {
    // Go: vars.go:jsMinifier.countHoistLength
    pub(crate) fn count_hoist_length(&self, ibinding: NodeId) -> i64 {
        if !self.o.keep_var_names {
            return binding_vars(&self.ast, ibinding).len() as i64 * 2; // assume that var name will be of length one, +1 for the comma
        }

        let mut n = 0i64;
        for v in binding_vars(&self.ast, ibinding) {
            n += self.ast.var(v).data.len() as i64 + 1; // +1 for the comma when added to other declaration
        }
        n
    }

    // Go: vars.go:jsMinifier.hoistVars
    /// Hoists all variable declarations in the current module/function scope
    /// to the variable declaration that reduces file size the most. All other
    /// declarations are converted to expressions and their variable names are
    /// copied to the only remaining declaration. This is possible because an
    /// ArrayBindingPattern and ObjectBindingPattern can be converted to an
    /// ArrayLiteral or ObjectLiteral respectively, as they are supersets of
    /// the BindingPatterns.
    pub(crate) fn hoist_vars(&mut self, body: NodeId) {
        let body_scope = self.ast.block(body).scope;
        let var_decls = self.ast.scope(body_scope).var_decls.clone();
        if 1 < var_decls.len() {
            // Select which variable declarations will be hoisted (convert to expression) and which not
            let mut best = 0usize;
            let mut scores = vec![0i64; var_decls.len()]; // savings if hoisting target
            let mut hoist = vec![false; var_decls.len()];
            for (i, &var_decl) in var_decls.iter().enumerate() {
                hoist[i] = true;
                if self.ast.var_decl(var_decl).in_for_in_of {
                    continue;
                }

                // variable names in for-in or for-of cannot be removed
                let mut n = 0i64; // total number of vars with decls
                let mut score: i64 = 3; // "var"
                let mut n_arrays = 0i64; // of which lhs arrays
                let mut n_objects = 0i64; // of which lhs objects
                let mut has_definitions = false;
                let len = self.ast.var_decl(var_decl).list.len();
                for j in 0..len {
                    let item = self.ast.var_decl(var_decl).list[j];
                    if item.default.is_some() {
                        // move arrays/objects to the front (saves a space)
                        match self.ast.node(item.binding) {
                            Node::BindingObject(_) => {
                                if j != 0 && n_arrays == 0 && n_objects == 0 {
                                    self.ast.var_decl_mut(var_decl).list.swap(0, j);
                                }
                                n_objects += 1;
                            }
                            Node::BindingArray(_) => {
                                if j != 0 && n_arrays == 0 && n_objects == 0 {
                                    self.ast.var_decl_mut(var_decl).list.swap(0, j);
                                }
                                n_arrays += 1;
                            }
                            _ => {}
                        }
                        score -= self.count_hoist_length(item.binding); // var names and commas
                        has_definitions = true;
                        n += 1;
                    }
                }
                let vd = self.ast.var_decl(var_decl);
                if n_arrays == 0 && n_objects == 0 {
                    score += 1; // required space after var
                }
                if !has_definitions && vd.in_for {
                    score -= 1; // semicolon can be reused
                }
                if n_objects != 0 && !vd.in_for && n_objects == n {
                    // required parenthesis around braces to not confound it with a block statement
                    score -= 2;
                }
                if score < scores[best] || self.ast.var_decl(var_decls[best]).in_for_in_of {
                    // select var decl that reduces the least when hoist target
                    best = i;
                }
                if score < 0 {
                    // don't hoist if it increases the amount of characters
                    hoist[i] = false;
                }
                scores[i] = score;
            }
            if self.ast.var_decl(var_decls[best]).in_for_in_of {
                // no savings possible
                return;
            }

            let decl = var_decls[best];
            if 10000 < self.ast.var_decl(decl).list.len() {
                return;
            }
            hoist[best] = false;

            // get original declarations
            let mut orig: Vec<NodeId> = Vec::new();
            for item in self.ast.var_decl(decl).list.clone() {
                orig.extend(binding_vars(&self.ast, item.binding));
            }

            // hoist other variable declarations in this function scope but don't initialize yet
            let mut j = 0usize;
            for (i, &var_decl) in var_decls.iter().enumerate() {
                if hoist[i] {
                    self.ast.var_decl_mut(var_decl).token_type = ErrorToken;
                    let items = self.ast.var_decl(var_decl).list.clone();
                    for item in items {
                        let refs = binding_vars(&self.ast, item.binding);
                        let mut binding_elements: Vec<BindingElement> =
                            Vec::with_capacity(refs.len());
                        'declared_loop: for r in refs {
                            for &v in &orig {
                                if r == v {
                                    continue 'declared_loop;
                                }
                            }
                            binding_elements.push(BindingElement {
                                binding: r,
                                default: NodeId::NIL,
                            });
                            orig.push(r);

                            let mut s = self.ast.var_decl(decl).scope;
                            while !s.is_nil() && s != self.ast.scope(s).func {
                                self.ast.add_undeclared(s, r);
                                s = self.ast.scope(s).parent;
                            }
                            if item.default.is_some() {
                                let v = self.ast.var_mut(r);
                                v.uses = v.uses.wrapping_add(1);
                            }
                        }
                        let nb = binding_elements.len();
                        let list = &mut self.ast.var_decl_mut(decl).list;
                        if i < best {
                            // prepend
                            let tail = list.split_off(j);
                            list.extend(binding_elements);
                            list.extend(tail);
                            j += nb;
                        } else {
                            // append
                            list.extend(binding_elements);
                        }
                    }
                }
            }

            // rearrange to put array/object first
            let mut prev_refs: Vec<NodeId> = Vec::new();
            let n = self.ast.var_decl(decl).list.len();
            'begin_array_object: for i in 0..n {
                let item = self.ast.var_decl(decl).list[i];
                let refs = binding_vars(&self.ast, item.binding);
                if !self.ast.is_var(item.binding) {
                    if i != 0 {
                        let mut interferes = false;
                        if item.default.is_some() {
                            'interference_loop: for &r in &refs {
                                for &v in &prev_refs {
                                    if r == v {
                                        interferes = true;
                                        break 'interference_loop;
                                    }
                                }
                            }
                        }
                        if !interferes {
                            self.ast.var_decl_mut(decl).list.swap(0, i);
                            break 'begin_array_object;
                        }
                    } else {
                        break 'begin_array_object;
                    }
                }
                if item.default.is_some() {
                    prev_refs.extend(refs);
                }
            }
        }
    }
}
