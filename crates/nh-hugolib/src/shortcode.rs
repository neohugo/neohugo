//! Port of `hugolib/shortcode.go`.
//!
//! Owner: Wave B task T22 (hugolib-content).


//! Go `hugolib/shortcode.go` (render half; extraction is `shortcode_parse.rs`, T20): rendering
//! with the shortcode template (data = `ShortcodeWithPage`, template execution through
//! `crate::template_exec::execute` with `ExecKind::Shortcode`), `prepareShortcodesForPage` and
//! `expandShortcodeTokens`. For `{{% %}}` shortcodes in markdown, `prepareShortcode` sets
//! `is_in_goldmark` on the context (shortcode.go:327-332) — the ONLY place it is set.

use std::sync::Arc;

use go_value::Value;
use nh_common::Result;

pub use crate::shortcode_parse::{create_shortcode_placeholder, Shortcode, ShortcodeHandler, ShortcodeInner};

/// Go: `expandShortcodeTokens(ctx, source, tokenHandler)`.
// Go: hugolib/shortcode.go:expandShortcodeTokens
pub fn expand_shortcode_tokens(source: &[u8], token_handler: &mut dyn FnMut(&str) -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/shortcode.go (794 lines; 12/26 funcs executed) — parse half in shortcode_parse.rs (T20)
//   types: ShortcodeWithPage
//    L79-97: (scp *ShortcodeWithPage) InnerDeindent() template.HTML
//    L101-108: (scp *ShortcodeWithPage) Position() text.Position
//    L111-113: (scp *ShortcodeWithPage) Site() page.Site
// EX L117-119: (scp *ShortcodeWithPage) Ref(args map[string]any) (string, error)
//    L123-125: (scp *ShortcodeWithPage) RelRef(args map[string]any) (string, error)
//    L128-133: (scp *ShortcodeWithPage) Store() *maps.Scratch
//    L138-140: (scp *ShortcodeWithPage) Scratch() *maps.Scratch
//    L143-183: (scp *ShortcodeWithPage) Get(key any) any
// EX L186-188: (scp *ShortcodeWithPage) Unwrapv() any
// EX L311-346: prepareShortcode( ctx context.Context, level int, s *Site, sc *shortcode, parent *ShortcodeWithPage, po *pageOutput, isRenderString bool, ) (shortc...
// EX L348-524: doRenderShortcode( ctx context.Context, level int, s *Site, sc *shortcode, parent *ShortcodeWithPage, po *pageOutput, isRenderString bool, ) (short...
// EX L547-560: (s *shortcodeHandler) prepareShortcodesForPage(ctx context.Context, po *pageOutput, isRenderString bool) (map[string]shortcodeRenderer, error)
// EX L738-783: expandShortcodeTokens( ctx context.Context, source []byte, tokenHandler func(ctx context.Context, token string) ([]byte, error), ) ([]byte, error)
// EX L785-794: renderShortcodeWithPage(ctx context.Context, h *tplimpl.TemplateStore, tmpl *tplimpl.TemplInfo, data *ShortcodeWithPage) (string, error)
// ---------------------------------------------------------------------------
