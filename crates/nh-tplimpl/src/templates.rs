//! Port of `tpl/tplimpl/templates.go`.
//!
//! Owner: Wave B task T13 (tplimpl).

//! Go `tpl/tplimpl/templates.go`: parse namespaces, `applyBaseTemplate` (clone namespace, parse
//! baseof THEN overlay into the same named template), `needsBaseTemplate`, BOM removal.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use nh_common::Result;
use nh_common::paths::pathparser::PathType;
use nh_doctree::simpletree::SimpleTree;

use crate::category::{Category, SubCategory};
use crate::engine::{HtmlTemplate, Template, TextTemplate};
use crate::templatestore::{
    KeyTemplateInfo, ProcessingState, TemplInfo, TemplInfoState, TemplWithBaseApplied,
};

/// Go: `templateNamespace`.
pub struct TemplateNamespace {
    pub(crate) parse_text: TextTemplate,
    pub(crate) parse_html: HtmlTemplate,
    pub(crate) prototype_text: Mutex<Option<TextTemplate>>,
    pub(crate) prototype_html: Mutex<Option<HtmlTemplate>>,
    pub(crate) standalone_text: TextTemplate,
    pub(crate) name_counter: AtomicU64,
    pub(crate) baseof_text_clones: Mutex<Vec<TextTemplate>>,
    pub(crate) baseof_html_clones: Mutex<Vec<HtmlTemplate>>,
}

impl TemplateNamespace {
    /// Go: `readTemplateInto(templ)` — the file content (BOM removed) and, unless already set,
    /// `noBaseOf = !needsBaseTemplate(content)`.
    // Go: tpl/tplimpl/templates.go:readTemplateInto
    pub(crate) fn read_template_into(&self, templ: &TemplInfo) -> Result<()> {
        let fi = templ
            .fi
            .as_ref()
            .expect("readTemplateInto: template without a file");
        let b = fi.meta().read_all()?;
        let content = remove_leading_bom_bytes(&b).to_vec();
        let mut m = templ.state_mut();
        if !m.no_base_of {
            m.no_base_of = !needs_base_template_bytes(&content);
        }
        m.content = content;
        Ok(())
    }

    // Go: tpl/tplimpl/templates.go:doParseTemplate
    pub(crate) fn do_parse_template(&self, ti: &TemplInfo, replace: bool) -> Result<()> {
        let (no_base_of, is_plain_text, sub_category, content) = {
            let m = ti.state();
            (
                m.no_base_of,
                m.d.is_plain_text,
                m.sub_category,
                m.content.clone(),
            )
        };
        if !no_base_of || ti.category == Some(Category::Baseof) {
            // Delay parsing until we have the base template.
            return Ok(());
        }
        let pi = ti
            .path_info
            .as_ref()
            .expect("doParseTemplate: template without a path");
        let mut name = pi.path_no_leading_slash().to_string();

        let templ = if is_plain_text {
            let prototype = &self.parse_text;
            if !replace && prototype.lookup(&name).is_some() {
                name += &format!("-{}", self.name_counter.fetch_add(1, Ordering::SeqCst) + 1);
            }
            Template::Text(prototype.new_associated(&name).parse(&content)?)
        } else {
            let prototype = &self.parse_html;
            if !replace && prototype.lookup(&name).is_some() {
                name += &format!("-{}", self.name_counter.fetch_add(1, Ordering::SeqCst) + 1);
            }
            let templ = prototype.new_associated(&name).parse(&content)?;

            if sub_category == SubCategory::Embedded {
                // In Hugo 0.146.0 we moved the internal templates around.
                // For the "_internal/twitter_cards.html" style templates, they
                // were moved to the _partials directory.
                // But we need to make them accessible from the old path for a while.
                if pi.path_type() == PathType::Partial {
                    let alias_name = name.strip_prefix("_partials/").unwrap_or(&name);
                    let alias_name = format!("_internal/{alias_name}");
                    prototype.add_parse_tree(&alias_name, tree_of_html(&templ))?;
                }

                // This was also possible before Hugo 0.146.0, but this should be deprecated.
                if pi.path_type() == PathType::Shortcode {
                    let alias_name = name.strip_prefix("_shortcodes/").unwrap_or(&name);
                    let alias_name = format!("_internal/shortcodes/{alias_name}");
                    prototype.add_parse_tree(&alias_name, tree_of_html(&templ))?;
                }
            }

            // Issue #13599.
            if ti.category == Some(Category::Partial)
                && ti.fi.as_ref().is_some_and(|fi| {
                    fi.meta()
                        .path_info
                        .as_ref()
                        .is_some_and(|p| p.section() == "partials")
                })
            {
                let alias_name = name.strip_prefix('_').unwrap_or(&name);
                prototype.add_parse_tree(alias_name, tree_of_html(&templ))?;
            }
            Template::Html(templ)
        };

        ti.state_mut().template = Some(templ);

        Ok(())
    }

    // Go: tpl/tplimpl/templates.go:applyBaseTemplate
    pub(crate) fn apply_base_template(
        &self,
        overlay: &Arc<TemplInfo>,
        base: &KeyTemplateInfo,
    ) -> Result<()> {
        base.info.state_mut().overlays.push(overlay.clone());

        let (overlay_content, overlay_d, overlay_plain) = {
            let m = overlay.state();
            (m.content.clone(), m.d.clone(), m.d.is_plain_text)
        };
        let base_content = base.info.state().content.clone();
        let overlay_name = overlay
            .path_info
            .as_ref()
            .expect("applyBaseTemplate: overlay without a path")
            .path_no_leading_slash()
            .to_string();

        let templ = if overlay_plain {
            let tt = self.parse_text.clone_ns()?.new_associated(&overlay_name);
            let tt = tt.parse(&base_content)?;
            let tt = tt.parse(&overlay_content)?;
            self.baseof_text_clones
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(tt.clone());
            Template::Text(tt)
        } else {
            let tt = self
                .parse_html
                .clone_shallow()?
                .new_associated(&overlay_name);
            let tt = tt.parse(&base_content)?;
            let tt = tt.parse(&overlay_content)?;
            self.baseof_html_clones
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(tt.clone());
            Template::Html(tt)
        };

        let template = Arc::new(TemplInfo {
            category: None,
            path_info: overlay.path_info.clone(),
            fi: overlay.fi.clone(),
            base: Some(base.info.clone()),
            execution_counter: AtomicU64::new(0),
            m: RwLock::new(TemplInfoState {
                template: Some(templ),
                d: overlay_d,
                no_base_of: true,
                ..Default::default()
            }),
        });

        let tb = Arc::new(TemplWithBaseApplied {
            overlay: overlay.clone(),
            base: base.info.clone(),
            template,
        });

        let base_d = base.info.state().d.clone();
        let mut m = overlay.state_mut();
        let variants_tree = m.base_variants.get_or_insert_with(SimpleTree::new);
        match variants_tree.get_mut(&base.key) {
            Some(variants) => {
                variants.insert(base_d, tb);
            }
            None => {
                let mut variants = std::collections::BTreeMap::new();
                variants.insert(base_d, tb);
                variants_tree.insert(&base.key, variants);
            }
        }
        Ok(())
    }

    /// Go: `templatesIn(in)` — every template of the namespace of `in` (sorted by name; Go:
    /// map order).
    // Go: tpl/tplimpl/templates.go:templatesIn
    pub(crate) fn templates_in(&self, t: &Template) -> Vec<Template> {
        match t {
            Template::Html(h) => h.all().into_iter().map(Template::Html).collect(),
            Template::Text(x) => x.all().into_iter().map(Template::Text).collect(),
        }
    }

    // Go: tpl/tplimpl/templates.go:createPrototypesParse
    /// Only used when refreshing templates (server/watch mode, not supported).
    pub(crate) fn create_prototypes_parse(&self) -> Result<()> {
        Err(nh_common::herrors::Error::new(
            "neohugo-rs: template refresh (server/watch mode) is not supported",
        ))
    }

    // Go: tpl/tplimpl/templates.go:createPrototypes
    pub(crate) fn create_prototypes(&self, init: bool) -> Result<()> {
        if init {
            *self
                .prototype_html
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(self.parse_html.clone_ns()?);
            *self
                .prototype_text
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(self.parse_text.clone_ns()?);
        }
        Ok(())
    }
}

/// The html template's `Tree` (always set right after a successful parse).
fn tree_of_html(t: &HtmlTemplate) -> crate::engine::parse::SharedTree {
    t.tree().expect("parsed html template without a tree")
}

/// Go: `newTemplateNamespace(funcs)` — the func map only serves name validation here (the exec
/// helper resolves the functions).
// Go: tpl/tplimpl/templates.go:newTemplateNamespace
pub(crate) fn new_template_namespace(func_names: &[String]) -> TemplateNamespace {
    let parse_html = HtmlTemplate::new("");
    parse_html.func_names(func_names.iter().cloned());
    let parse_text = TextTemplate::new("");
    parse_text.func_names(func_names.iter().cloned());
    let standalone_text = TextTemplate::new("");
    standalone_text.func_names(func_names.iter().cloned());
    TemplateNamespace {
        parse_text,
        parse_html,
        prototype_text: Mutex::new(None),
        prototype_html: Mutex::new(None),
        standalone_text,
        name_counter: AtomicU64::new(0),
        baseof_text_clones: Mutex::new(Vec::new()),
        baseof_html_clones: Mutex::new(Vec::new()),
    }
}

/// Go: `isText(t)`.
// Go: tpl/tplimpl/templates.go:isText
pub fn is_text(t: &Template) -> bool {
    t.is_text()
}

/// Go: `needsBaseTemplate(templ)` — the first non-comment, non-whitespace action is `define`.
// Go: tpl/tplimpl/templates.go:needsBaseTemplate
pub fn needs_base_template(templ: &str) -> bool {
    needs_base_template_bytes(templ.as_bytes())
}

/// [`needs_base_template`] over Go string bytes.
pub fn needs_base_template_bytes(templ: &[u8]) -> bool {
    let mut idx: Option<usize> = None;
    let mut in_comment = false;
    let mut i = 0;
    while i < templ.len() {
        let rest = &templ[i..];
        if !in_comment && rest.starts_with(b"{{/*") {
            in_comment = true;
            i += 4;
        } else if !in_comment && rest.starts_with(b"{{- /*") {
            in_comment = true;
            i += 6;
        } else if in_comment && rest.starts_with(b"*/}}") {
            in_comment = false;
            i += 4;
        } else if in_comment && rest.starts_with(b"*/ -}}") {
            in_comment = false;
            i += 6;
        } else {
            let (r, size) = go_unicode::utf8::decode_rune(rest);
            if !in_comment {
                if rest.starts_with(b"{{") {
                    idx = Some(i);
                    break;
                } else if !go_unicode::is_space(r) {
                    break;
                }
            }
            i += size;
        }
    }

    match idx {
        None => false,
        Some(idx) => base_template_define_re_match(&templ[idx..]),
    }
}

/// Go: `baseTemplateDefineRe.MatchString(s)` with `^{{-?\s*define` (RE2 `\s` is
/// `[\t\n\f\r ]`).
// Go: tpl/tplimpl/templates.go:baseTemplateDefineRe
fn base_template_define_re_match(s: &[u8]) -> bool {
    let Some(mut rest) = s.strip_prefix(b"{{") else {
        return false;
    };
    if let Some(r) = rest.strip_prefix(b"-") {
        rest = r;
    }
    let n = rest
        .iter()
        .take_while(|&&c| matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' '))
        .count();
    rest[n..].starts_with(b"define")
}

/// Go: `removeLeadingBOM(s)` over a UTF-8 `&str`.
// Go: tpl/tplimpl/templates.go:removeLeadingBOM
pub fn remove_leading_bom(s: &str) -> &str {
    // SAFETY-free: the bytes version only ever strips a whole 3-byte rune.
    let b = remove_leading_bom_bytes(s.as_bytes());
    &s[s.len() - b.len()..]
}

/// Go: `removeLeadingBOM(s)` over Go string bytes: a leading U+FEFF is removed only when more
/// runes follow (Go returns `s[i:]` at the second rune; a string that is just the BOM is
/// returned unchanged).
pub fn remove_leading_bom_bytes(s: &[u8]) -> &[u8] {
    const BOM: i32 = 0xFEFF;
    let mut i = 0;
    while i < s.len() {
        let (r, size) = go_unicode::utf8::decode_rune(&s[i..]);
        if i == 0 && r != BOM {
            return s;
        }
        if i > 0 {
            return &s[i..];
        }
        i += size;
    }
    s
}

/// The processing state of a template (Go `processingState`), re-exported for the store.
pub(crate) fn is_transformed(ti: &TemplInfo) -> bool {
    ti.state().state == ProcessingState::Transformed
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/templates.go (366 lines; 10/11 funcs executed)
//   types: templateNamespace
// OK L19-40: (t *templateNamespace) readTemplateInto(templ *TemplInfo) error
// OK L48-54: (s *TemplateStore) parseTemplate(ti *TemplInfo, replace bool) error (templatestore.rs)
// OK L56-125: (t *templateNamespace) doParseTemplate(ti *TemplInfo, replace bool) error
// OK L127-182: (t *templateNamespace) applyBaseTemplate(overlay *TemplInfo, base keyTemplateInfo) error
// OK L184-202: (t *templateNamespace) templatesIn(in tpl.Template) iter.Seq[tpl.Template]
// OK L265-300: needsBaseTemplate(templ string) bool
// OK L302-315: removeLeadingBOM(s string) string
// OK L331-338: (t *templateNamespace) createPrototypesParse() error (STUB: watch mode only)
// OK L340-347: (t *templateNamespace) createPrototypes(init bool) error
// OK L349-355: newTemplateNamespace(funcs map[string]any) *templateNamespace
// OK L357-366: isText(t tpl.Template) bool
// ---------------------------------------------------------------------------
