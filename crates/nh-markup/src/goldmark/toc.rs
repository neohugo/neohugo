//! Port of `markup/goldmark/toc.go`.
//!
//! Owner: Wave B task T06 (markup).

use std::sync::{Arc, LazyLock};

use go_value::GoString;
use goldmark::ast::{self, Ast, AttrValue, NodeId, WalkStatus};
use goldmark::parser::{self, Context, ContextKey};
use goldmark::renderer::{Renderer, RendererOption};
use goldmark::text::Reader;
use goldmark::util;
use goldmark::{Extender, Markdown};

use super::autoid::IdFactory;
use crate::tableofcontents::{Builder, Fragments, Heading};

pub(crate) static TOC_RESULT_KEY: LazyLock<ContextKey> = LazyLock::new(parser::new_context_key);
pub(crate) static TOC_ENABLE_KEY: LazyLock<ContextKey> = LazyLock::new(parser::new_context_key);

/// The TOC result stored in the parser context (Go stores the `*tableofcontents.Fragments`).
pub(crate) struct TocResult(pub Arc<Fragments>);

struct TocTransformer {
    r: Renderer,
}

impl parser::AstTransformer for TocTransformer {
    // Go: markup/goldmark/toc.go:Transform
    fn transform<'a>(&self, a: &mut Ast, n: NodeId, reader: &mut dyn Reader<'a>, pc: &mut Context) {
        match pc.get_as::<bool>(*TOC_ENABLE_KEY) {
            Some(true) => {}
            _ => return,
        }

        let mut toc = Builder::default();
        let mut toc_heading = Heading::default();
        let mut level: i64 = 0;
        let mut row: i64 = -1;
        let mut in_heading = false;
        let mut heading_text: Vec<u8> = Vec::new();

        // Go: `pc.IDs().(stringValuesProvider).StringValues()` (Hugo's context always has an
        // idFactory; a failed type assertion panics).
        let ids = pc
            .ids()
            .as_any_mut()
            .and_then(|a| a.downcast_mut::<IdFactory>())
            .expect("interface conversion: parser.IDs is not goldmark.stringValuesProvider")
            .string_values();
        if !ids.is_empty() {
            toc.set_identifiers(ids.into_iter().map(GoString::new).collect());
        }

        let source = reader.source();
        let _ = ast::walk_ref(a, n, &mut |a, n, entering| {
            let s = WalkStatus::Continue;
            if a.kind(n) == ast::KIND_HEADING {
                if in_heading && !entering {
                    toc_heading.title = GoString::new(std::mem::take(&mut heading_text));
                    toc.add_at(
                        std::mem::take(&mut toc_heading),
                        row as usize,
                        (level - 1) as usize,
                    );
                    in_heading = false;
                    return Ok(s);
                }

                in_heading = true;
            }

            if !in_heading || !entering {
                return Ok(s);
            }

            let k = a.kind(n);
            if k == ast::KIND_HEADING {
                let heading = a.heading(n).expect("Heading");
                level = heading.level;

                if level == 1 || row == -1 {
                    row += 1;
                }

                if let Some(id) = a.attribute_string(n, "id") {
                    // Go: `string(id.([]byte))` panics for any other value type.
                    let id = match id {
                        AttrValue::Bytes(b) => b.clone(),
                        other => panic!(
                            "interface conversion: interface {{}} is {}, not []uint8",
                            other.go_type_name()
                        ),
                    };
                    toc_heading.id = GoString::new(id);
                    toc_heading.level = level;
                }
            } else if k == ast::KIND_CODE_SPAN
                || k == ast::KIND_LINK
                || k == ast::KIND_IMAGE
                || k == ast::KIND_EMPHASIS
                || k == *goldmark::extension::ast::KIND_STRIKETHROUGH
            {
                // (goldmark-emoji's KindEmoji is not supported: the emoji extension is an
                // explicit error, so no such node exists.)
                self.r.render(&mut heading_text, source, a, n)?;
                return Ok(WalkStatus::SkipChildren);
            } else if k == ast::KIND_AUTO_LINK
                || k == ast::KIND_RAW_HTML
                || k == ast::KIND_TEXT
                || k == ast::KIND_STRING
            {
                self.r.render(&mut heading_text, source, a, n)?;
            }

            Ok(s)
        });

        pc.set_value(*TOC_RESULT_KEY, TocResult(toc.build()));
    }
}

/// Go: `tocExtension`.
pub(crate) struct TocExtension {
    options: std::sync::Mutex<Option<Vec<Box<dyn RendererOption>>>>,
}

// Go: markup/goldmark/toc.go:newTocExtension
pub(crate) fn new_toc_extension(options: Vec<Box<dyn RendererOption>>) -> TocExtension {
    TocExtension {
        options: std::sync::Mutex::new(Some(options)),
    }
}

impl Extender for TocExtension {
    // Go: markup/goldmark/toc.go:Extend
    fn extend(&self, m: &mut Markdown) {
        let r = goldmark::default_renderer();
        // Go's options are values that can be applied again; the Rust options are consumed, and
        // an extension is extended once.
        let options = self.options.lock().unwrap().take().unwrap_or_default();
        r.add_options(options);
        m.parser()
            .add_options(vec![parser::with_ast_transformers(vec![
                util::prioritized(
                    Box::new(TocTransformer { r }) as Box<dyn parser::AstTransformer>,
                    // This must run after the ID generation (priority 100).
                    110,
                ),
            ])]);
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/toc.go (141 lines; 3/3 funcs executed)
//   types: tocTransformer, tocExtension
// OK L42-121: (t *tocTransformer) Transform(n *ast.Document, reader text.Reader, pc parser.Context)
// OK L127-131: newTocExtension(options []renderer.Option) goldmark.Extender
// OK L133-141: (e *tocExtension) Extend(m goldmark.Markdown)
// ---------------------------------------------------------------------------
