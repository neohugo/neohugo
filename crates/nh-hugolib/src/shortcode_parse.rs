//! Port of `hugolib/shortcode.go` (parse half).
//!
//! Owner: Wave B task T20 (hugolib-capture).


//! Split from `shortcode.go` because page creation parses the shortcodes of the content
//! (`mapItemsAfterFrontMatter` -> `extractShortcode`, page__content_parse.rs). This module holds
//! shortcode.go lines 193-310 (`createShortcodePlaceholder`, `shortcode`, `shortcodeHandler`,
//! `newShortcodeHandler`), 526-545 (`addName`, `transferNames`, `hasName`) and 562-736
//! (`posFromInput`, `extractShortcode`). Rendering (`prepareShortcode`, `doRenderShortcode`,
//! `prepareShortcodesForPage`, `expandShortcodeTokens`) is `shortcode.rs` (T22).
//!
//! `extractShortcode` looks up the shortcode template by name in the template store
//! (`TemplateStore.LookupShortcodeByName`) to know whether it takes inner content: capture
//! therefore needs the store (T13) with the embedded `ref`/`relref` shortcodes.

use std::sync::Arc;

use go_value::Value;
use nh_common::Result;

/// Go: `shortcode`.
#[derive(Clone)]
pub struct Shortcode {
    pub name: String,
    pub is_inline: bool,
    /// `{{% %}}` (markup processed) vs `{{< >}}`.
    pub do_markup: bool,
    pub is_closing: bool,
    /// Positional (`[]any`) or named (`map[string]any`) params.
    pub params: Value,
    pub inner: Vec<ShortcodeInner>,
    pub pos: i64,
    pub length: i64,
    pub indentation: String,
    pub ordinal: i64,
    pub placeholder: String,
    pub info: Option<Arc<nh_tplimpl::template_info::ParseInfo>>,
    pub templs: Vec<Arc<nh_tplimpl::templatestore::TemplInfo>>,
}

/// Inner content: text or nested shortcodes.
#[derive(Clone)]
pub enum ShortcodeInner {
    Text(Vec<u8>),
    Shortcode(Arc<Shortcode>),
}

/// Go: `shortcodeHandler` (per page). Filled during capture; read-only afterwards.
#[derive(Default)]
pub struct ShortcodeHandler {
    pub filename: String,
    pub enable_inline_shortcodes: bool,
    pub shortcodes: Vec<Arc<Shortcode>>,
    pub name_set: std::collections::BTreeSet<String>,
}

/// Go: `createShortcodePlaceholder(sid, id, ordinal)` = `"HAHAHUGOSHORTCODE" + id + sid + ordinal + "HBHB"`.
// Go: hugolib/shortcode.go:createShortcodePlaceholder
pub fn create_shortcode_placeholder(sid: &str, id: u64, ordinal: i64) -> String {
    format!("HAHAHUGOSHORTCODE{id}{sid}{ordinal}HBHB")
}

impl ShortcodeHandler {
    /// Go: `newShortcodeHandler(filename, s)`.
    // Go: hugolib/shortcode.go:newShortcodeHandler
    pub fn new(filename: &str, enable_inline_shortcodes: bool) -> ShortcodeHandler {
        ShortcodeHandler { filename: filename.to_string(), enable_inline_shortcodes, ..Default::default() }
    }

    /// Go: `extractShortcode(ordinal, level, source, pt)`.
    // Go: hugolib/shortcode.go:extractShortcode
    pub fn extract_shortcode(
        &mut self,
        store: &nh_tplimpl::templatestore::TemplateStore,
        ordinal: i64,
        level: i64,
        source: &[u8],
        pt: &mut nh_parser::pageparser::pageparser::Iterator<'_>,
    ) -> Result<Shortcode> {
        todo!()
    }

    // Go: hugolib/shortcode.go:addName
    pub fn add_name(&mut self, name: &str) {
        self.name_set.insert(name.to_string());
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/shortcode.go (parse half; the render half is in shortcode.rs)
//   types: shortcode, shortcodeHandler
// EX L193-195: createShortcodePlaceholder(sid string, id uint64, ordinal int) string
// EX L226-228: (s shortcode) insertPlaceholder() bool
//    L230-232: (s shortcode) needsInner() bool
//    L234-240: (s shortcode) configVersion() int
//    L242-250: (s shortcode) innerString() string
//    L252-276: (sc shortcode) String() string
// EX L293-303: newShortcodeHandler(filename string, s *Site) *shortcodeHandler
// EX L526-530: (s *shortcodeHandler) addName(name string)
//    L532-538: (s *shortcodeHandler) transferNames(in *shortcodeHandler)
//    L540-545: (s *shortcodeHandler) hasName(name string) bool
//    L562-579: posFromInput(filename string, input []byte, offset int) text.Position
// EX L584-734: (s *shortcodeHandler) extractShortcode(ordinal, level int, source []byte, pt *pageparser.Iterator) (*shortcode, error)
// ---------------------------------------------------------------------------
