//! Go: tpl/internal/go_templates/texttemplate/parse — parse trees for
//! templates as defined by text/template and html/template.

mod lex;
mod node;
#[allow(clippy::module_inception)]
mod parse;

use std::sync::{Arc, RwLock};

pub use lex::{Item, ItemType, Lexer};
pub use node::*;
pub use parse::{
    FuncNames, Mode, ParseError, Tree, error_context, is_empty_node, is_empty_tree, parse,
    parse_with_mode,
};

/// Go: a `*parse.Tree` pointer — a parse tree shared by reference.
///
/// Go templates hold `*parse.Tree` pointers: `Clone`, `CloneShallow` and
/// `AddParseTree` make several templates (possibly in different
/// namespaces) share one tree, and in-place mutations (the html/template
/// escaper's edits, Hugo's AST transforms) are seen by all of them.
/// `SharedTree` gives exactly that: clones of a `SharedTree` are the same
/// Go pointer, and [`SharedTree::update`] mutates the tree for everyone.
///
/// Readers (execution) take an immutable snapshot with
/// [`SharedTree::get`] and walk it without holding any lock; a later update
/// copies-on-write if a snapshot is still in use, so a running execution
/// never observes a half-edited tree.
#[derive(Clone)]
pub struct SharedTree(Arc<RwLock<Arc<Tree>>>);

impl SharedTree {
    pub fn new(t: Tree) -> SharedTree {
        SharedTree(Arc::new(RwLock::new(Arc::new(t))))
    }

    /// The current tree (a cheap snapshot).
    pub fn get(&self) -> Arc<Tree> {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Mutates the tree in place (Go: writing through the pointer).
    pub fn update<R>(&self, f: impl FnOnce(&mut Tree) -> R) -> R {
        let mut g = self.0.write().unwrap_or_else(|e| e.into_inner());
        f(Arc::make_mut(&mut g))
    }

    /// Replaces the whole tree value (Go: `*p = *t`).
    pub fn set(&self, t: Tree) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(t);
    }

    /// Go pointer equality.
    pub fn ptr_eq(&self, other: &SharedTree) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// The tree's name (Go: `t.Name`).
    pub fn name(&self) -> String {
        self.get().name.clone()
    }
}

impl std::fmt::Debug for SharedTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharedTree({:?})", self.get().name)
    }
}
