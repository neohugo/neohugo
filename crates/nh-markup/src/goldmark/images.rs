//! Port of `markup/goldmark/images/transform.go`.
//!
//! Owner: Wave B task T06 (markup).

use std::sync::Arc;

use goldmark::ast::{self, Ast, AttrValue, NodeId, WalkStatus};
use goldmark::parser::{self, Context};
use goldmark::text::Reader;
use goldmark::util;
use goldmark::{Extender, Markdown};

/// Used to signal to the rendering step that an image is used in a block context.
/// Don't change this; the prefix must match the internalAttrPrefix in the root goldmark package.
pub const ATTR_IS_BLOCK: &str = "_h__isBlock";
/// The image's ordinal (a Go `int`, stored as an `AttrValue::Other(i64)`).
pub const ATTR_ORDINAL: &str = "_h__ordinal";

// Go: markup/goldmark/images/transform.go:New
pub fn new(wrap_stand_alone_image_within_paragraph: bool) -> Box<dyn Extender> {
    Box::new(ImagesExtension {
        wrap_stand_alone_image_within_paragraph,
    })
}

struct ImagesExtension {
    wrap_stand_alone_image_within_paragraph: bool,
}

impl Extender for ImagesExtension {
    // Go: markup/goldmark/images/transform.go:Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser()
            .add_options(vec![parser::with_ast_transformers(vec![
                util::prioritized(
                    Box::new(Transformer {
                        wrap_stand_alone_image_within_paragraph: self
                            .wrap_stand_alone_image_within_paragraph,
                    }) as Box<dyn parser::AstTransformer>,
                    300,
                ),
            ])]);
    }
}

/// Go: `images.Transformer`.
pub struct Transformer {
    wrap_stand_alone_image_within_paragraph: bool,
}

/// The `_h__ordinal` value of an image node.
pub fn image_ordinal(ast: &Ast, n: NodeId) -> Option<i64> {
    match ast.attribute_string(n, ATTR_ORDINAL) {
        Some(AttrValue::Other(o)) => Some(
            *o.downcast_ref::<i64>()
                .expect("interface conversion: interface {} is not int"),
        ),
        Some(other) => panic!(
            "interface conversion: interface {{}} is {}, not int",
            other.go_type_name()
        ),
        None => None,
    }
}

impl parser::AstTransformer for Transformer {
    /// Transform transforms the provided Markdown AST.
    // Go: markup/goldmark/images/transform.go:Transform
    fn transform<'a>(
        &self,
        doc: &mut Ast,
        root: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pctx: &mut Context,
    ) {
        let mut ordinal: i64 = 0;
        let _ = ast::walk(doc, root, &mut |ast, node, enter| {
            if !enter {
                return Ok(WalkStatus::Continue);
            }

            if ast.kind(node) == ast::KIND_IMAGE {
                let n = node;
                let parent = ast.parent(n).expect("image parent");
                ast.set_attribute_string(n, ATTR_ORDINAL, AttrValue::Other(Arc::new(ordinal)));
                ordinal += 1;

                if !self.wrap_stand_alone_image_within_paragraph {
                    let is_block = ast.child_count(parent) == 1;
                    if is_block {
                        ast.set_attribute_string(n, ATTR_IS_BLOCK, AttrValue::Bool(true));
                    }

                    if is_block && ast.kind(parent) == ast::KIND_PARAGRAPH {
                        let attrs: Vec<ast::Attribute> = ast
                            .attributes(parent)
                            .map(|a| a.to_vec())
                            .unwrap_or_default();
                        for attr in attrs {
                            // Transfer any attribute set down to the image.
                            // Image elements does not support attributes on its own,
                            // so it's safe to just set without checking first.
                            ast.set_attribute(n, &attr.name, attr.value.clone());
                        }
                        let grand_parent = ast.parent(parent).expect("paragraph parent");
                        ast.replace_child(grand_parent, parent, n);
                    }
                }
            }

            Ok(WalkStatus::Continue)
        });
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/images/transform.go (76 lines; 3/3 funcs executed)
//   types: (group), Transformer
// OK L24-26: New(wrapStandAloneImageWithinParagraph bool) goldmark.Extender
// OK L28-34: (e *imagesExtension) Extend(m goldmark.Markdown)
// OK L41-76: (t *Transformer) Transform(doc *ast.Document, reader text.Reader, pctx parser.Context)
// ---------------------------------------------------------------------------
