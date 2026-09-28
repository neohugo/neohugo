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

use std::collections::BTreeSet;
use std::sync::Arc;

use go_value::{GoString, List, Map, MapType, SliceType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::text::Position;
use nh_parser::pageparser::pageparser::Iterator;
use nh_tplimpl::templatestore::{TemplInfo, TemplateStore};

/// Go: `shortcodePlaceholderPrefix` (must not contain any markup syntax).
pub const SHORTCODE_PLACEHOLDER_PREFIX: &str = "HAHAHUGOSHORTCODE";

/// Go: `shortcode`.
#[derive(Clone)]
pub struct Shortcode {
    pub name: String,
    /// Inline shortcode. Any inner will be a Go template.
    pub is_inline: bool,
    /// `{{% %}}` (markup processed) vs `{{< >}}`: if set, the rendered shortcode is sent as
    /// part of the surrounding content to Goldmark and similar.
    pub do_markup: bool,
    /// Whether a closing tag was provided.
    pub is_closing: bool,
    /// Go `params any`: `Invalid` (nil), positional (`[]interface {}`), named
    /// (`map[string]interface {}`) or, when the shortcode has none, a nil `[]string`
    /// (`TypedNil("[]string")`, set by `mapItemsAfterFrontMatter`).
    pub params: Value,
    /// Go `inner []any`: text or nested shortcodes.
    pub inner: Vec<ShortcodeInner>,
    /// The position in bytes in the source file.
    pub pos: i64,
    /// The length in bytes in the source file.
    pub length: i64,
    /// Indentation from source.
    pub indentation: Vec<u8>,
    pub ordinal: i64,
    /// The placeholder in the source when passed to Goldmark etc. This also identifies the
    /// rendered shortcode.
    pub placeholder: String,
    /// Go `templ`: the shortcode template found by name (`None` for inline shortcodes).
    pub templ: Option<Arc<TemplInfo>>,
}

/// Inner content: text or nested shortcodes.
#[derive(Clone)]
pub enum ShortcodeInner {
    Text(Vec<u8>),
    Shortcode(Arc<Shortcode>),
}

impl Shortcode {
    fn new(ordinal: i64) -> Shortcode {
        Shortcode {
            name: String::new(),
            is_inline: false,
            do_markup: false,
            is_closing: false,
            params: Value::Invalid,
            inner: Vec::new(),
            pos: 0,
            length: 0,
            indentation: Vec::new(),
            ordinal,
            placeholder: String::new(),
            templ: None,
        }
    }

    // Go: hugolib/shortcode.go:insertPlaceholder
    pub fn insert_placeholder(&self) -> bool {
        !self.do_markup || self.config_version() == 1
    }

    // Go: hugolib/shortcode.go:needsInner
    pub fn needs_inner(&self) -> bool {
        self.templ.as_ref().is_some_and(|t| t.parse_info().is_inner)
    }

    // Go: hugolib/shortcode.go:configVersion
    pub fn config_version(&self) -> i64 {
        match &self.templ {
            // Not set for inline shortcodes.
            None => 2,
            Some(t) => t.parse_info().config.version,
        }
    }

    /// Go: `innerString()` (Go asserts every inner element is a string, panicking otherwise).
    // Go: hugolib/shortcode.go:innerString
    pub fn inner_string(&self) -> Vec<u8> {
        let mut sb = Vec::new();
        for inner in &self.inner {
            match inner {
                ShortcodeInner::Text(s) => sb.extend_from_slice(s),
                ShortcodeInner::Shortcode(_) => {
                    panic!("interface conversion: interface {{}} is *hugolib.shortcode, not string")
                }
            }
        }
        sb
    }
}

/// Go: `shortcodeHandler` (per page). Filled during capture; read-only afterwards.
#[derive(Default)]
pub struct ShortcodeHandler {
    pub filename: String,
    /// Configuration (Go `s.ExecHelper.Sec().EnableInlineShortcodes`).
    pub enable_inline_shortcodes: bool,
    /// Ordered list of shortcodes for a page.
    pub shortcodes: Vec<Arc<Shortcode>>,
    /// All the shortcode names in this set.
    pub name_set: BTreeSet<String>,
}

/// Go: `createShortcodePlaceholder(sid, id, ordinal)` = `"HAHAHUGOSHORTCODE" + id + sid + ordinal + "HBHB"`.
// Go: hugolib/shortcode.go:createShortcodePlaceholder
pub fn create_shortcode_placeholder(sid: &str, id: u64, ordinal: i64) -> String {
    format!("{SHORTCODE_PLACEHOLDER_PREFIX}{id}{sid}{ordinal}HBHB")
}

const ERROR_PREFIX: &str = "failed to extract shortcode";

impl ShortcodeHandler {
    /// Go: `newShortcodeHandler(filename, s)`.
    // Go: hugolib/shortcode.go:newShortcodeHandler
    pub fn new(filename: &str, enable_inline_shortcodes: bool) -> ShortcodeHandler {
        ShortcodeHandler {
            filename: filename.to_string(),
            enable_inline_shortcodes,
            ..Default::default()
        }
    }

    /// Go: `extractShortcode(ordinal, level, source, pt)` — pageTokens state: before, positioned
    /// just before the shortcode start; after, shortcode(s) consumed (plural when they are
    /// nested). On error, Go returns the partial shortcode or nil with the error; the Rust `Err`
    /// carries only the error (callers discard the shortcode).
    // Go: hugolib/shortcode.go:extractShortcode
    pub fn extract_shortcode(
        &mut self,
        store: &TemplateStore,
        ordinal: i64,
        level: i64,
        source: &[u8],
        pt: &mut Iterator<'_>,
    ) -> Result<Shortcode> {
        let mut sc = Shortcode::new(ordinal);

        // Back up one to identify any indentation.
        if pt.pos() > 0 {
            pt.backup();
            let item = pt.next_item();
            if item.is_indentation() {
                sc.indentation = item.val_bytes(source);
            }
        }

        let mut cnt = 0;
        let mut nested_ordinal = 0;
        let next_level = level + 1;
        let mut closed = false;

        loop {
            let curr_item = pt.next_item();
            if curr_item.is_left_shortcode_delim() {
                let next = pt.peek();
                if next.is_right_shortcode_delim() {
                    // no name: {{< >}} or {{% %}}
                    return Err(Error::new("shortcode has no name"));
                }
                if next.is_shortcode_close() {
                    continue;
                }

                if cnt > 0 {
                    // nested shortcode; append it to inner content
                    pt.backup();
                    let nested =
                        self.extract_shortcode(store, nested_ordinal, next_level, source, pt)?;
                    nested_ordinal += 1;
                    if !nested.name.is_empty() {
                        self.add_name(&nested.name);
                    }
                    sc.inner.push(ShortcodeInner::Shortcode(Arc::new(nested)));
                } else {
                    sc.do_markup = curr_item.is_shortcode_markup_delimiter();
                }

                cnt += 1;
            } else if curr_item.is_right_shortcode_delim() {
                // we trust the template on this:
                // if there's no inner, we're done
                if !sc.is_inline {
                    let Some(templ) = &sc.templ else {
                        // Go dereferences the nil template (a runtime panic).
                        return Err(Error::new(
                            "runtime error: invalid memory address or nil pointer dereference",
                        ));
                    };
                    if !templ.parse_info().is_inner {
                        return Ok(sc);
                    }
                }
            } else if curr_item.is_shortcode_close() {
                closed = true;
                let next = pt.peek();
                if !sc.is_inline && !sc.needs_inner() {
                    if next.is_error() {
                        // return that error, more specific
                        continue;
                    }
                    let mut name = sc.name.clone();
                    if name.is_empty() {
                        name = next.val_str(source);
                    }
                    return Err(Error::new(format!(
                        "{ERROR_PREFIX}: shortcode {} does not evaluate .Inner or .InnerDeindent, yet a closing tag was provided",
                        go_strconv::quote(&name)
                    )));
                }
                if next.is_right_shortcode_delim() {
                    // self-closing
                    pt.consume(1);
                } else {
                    sc.is_closing = true;
                    pt.consume(2);
                }

                return Ok(sc);
            } else if curr_item.is_text() {
                sc.inner
                    .push(ShortcodeInner::Text(curr_item.val_bytes(source)));
            } else if curr_item.is_shortcode_name() {
                sc.name = curr_item.val_str(source);

                // Used to check if the template expects inner content,
                // so just pick one arbitrarily with the same name.
                let Some(templ) = store.lookup_shortcode_by_name(&sc.name) else {
                    return Err(Error::new(format!(
                        "{ERROR_PREFIX}: template for shortcode {} not found",
                        go_strconv::quote(&sc.name)
                    )));
                };
                sc.templ = Some(templ);
            } else if curr_item.is_inline_shortcode_name() {
                sc.name = curr_item.val_str(source);
                sc.is_inline = true;
            } else if curr_item.is_shortcode_param() {
                if !pt.is_value_next() {
                    continue;
                } else if pt.peek().is_shortcode_param_val() {
                    // named params
                    let key = GoString::from(curr_item.val_bytes(source));
                    match &mut sc.params {
                        Value::Invalid => {
                            let mut params = Map::new(MapType::StringAny);
                            params.insert(key, pt.next_item().val_typed(source));
                            sc.params = Value::Map(Arc::new(params));
                        }
                        Value::Map(params) => {
                            Arc::make_mut(params).insert(key, pt.next_item().val_typed(source));
                        }
                        _ => {
                            return Err(Error::new(format!(
                                "{ERROR_PREFIX}: invalid state: invalid param type map[string]interface {{}} for shortcode {}, expected a map",
                                go_strconv::quote(&sc.name)
                            )));
                        }
                    }
                } else {
                    // positional params
                    match &mut sc.params {
                        Value::Invalid => {
                            let params = vec![curr_item.val_typed(source)];
                            sc.params = Value::List(Arc::new(List::new(SliceType::Any, params)));
                        }
                        Value::List(params) => {
                            Arc::make_mut(params)
                                .items
                                .push(curr_item.val_typed(source));
                        }
                        _ => {
                            return Err(Error::new(format!(
                                "{ERROR_PREFIX}: invalid state: invalid param type []interface {{}} for shortcode {}, expected a slice",
                                go_strconv::quote(&sc.name)
                            )));
                        }
                    }
                }
            } else if curr_item.is_done() {
                if !curr_item.is_error() && !closed && sc.needs_inner() {
                    return Err(Error::new(format!(
                        "{ERROR_PREFIX}: shortcode {} must be closed or self-closed",
                        go_strconv::quote(&sc.name)
                    )));
                }
                // handled by caller
                pt.backup();
                break;
            }
        }
        Ok(sc)
    }

    // Go: hugolib/shortcode.go:addName
    pub fn add_name(&mut self, name: &str) {
        self.name_set.insert(name.to_string());
    }

    // Go: hugolib/shortcode.go:transferNames
    pub fn transfer_names(&mut self, input: &ShortcodeHandler) {
        for k in &input.name_set {
            self.name_set.insert(k.clone());
        }
    }

    // Go: hugolib/shortcode.go:hasName
    pub fn has_name(&self, name: &str) -> bool {
        self.name_set.contains(name)
    }
}

/// Go: `posFromInput(filename, input, offset)`.
// Go: hugolib/shortcode.go:posFromInput
pub fn pos_from_input(filename: &str, input: &[u8], offset: i64) -> Position {
    if offset < 0 {
        return Position {
            filename: filename.to_string(),
            ..Default::default()
        };
    }
    let input = &input[..offset as usize];
    let line_number = go_unicode::bytes::count(input, b"\n") as i64 + 1;
    let end_of_last_line = input
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|i| i as i64)
        .unwrap_or(-1);

    Position {
        filename: filename.to_string(),
        line_number,
        column_number: offset - end_of_last_line,
        offset,
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/shortcode.go (parse half; the render half is in shortcode.rs)
//   types: shortcode, shortcodeHandler
// OK L193-195: createShortcodePlaceholder(sid string, id uint64, ordinal int) string
// OK L226-228: (s shortcode) insertPlaceholder() bool
// OK L230-232: (s shortcode) needsInner() bool
// OK L234-240: (s shortcode) configVersion() int
// OK L242-250: (s shortcode) innerString() string
//    L252-276: (sc shortcode) String() string
// OK L293-303: newShortcodeHandler(filename string, s *Site) *shortcodeHandler
// OK L526-530: (s *shortcodeHandler) addName(name string)
// OK L532-538: (s *shortcodeHandler) transferNames(in *shortcodeHandler)
// OK L540-545: (s *shortcodeHandler) hasName(name string) bool
// OK L562-579: posFromInput(filename string, input []byte, offset int) text.Position
// OK L584-734: (s *shortcodeHandler) extractShortcode(ordinal, level int, source []byte, pt *pageparser.Iterator) (*shortcode, error)
// ---------------------------------------------------------------------------
