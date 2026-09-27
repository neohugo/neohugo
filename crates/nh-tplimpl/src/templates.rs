//! Port of `tpl/tplimpl/templates.go`.
//!
//! Owner: Wave B task T13 (tplimpl).


//! Go `tpl/tplimpl/templates.go`: parse namespaces, `applyBaseTemplate` (clone namespace, parse
//! baseof THEN overlay into the same named template), `needsBaseTemplate`, BOM removal.

use crate::engine::{HtmlTemplate, TextTemplate};

/// Go: `templateNamespace`.
pub struct TemplateNamespace {
    pub(crate) parse_text: TextTemplate,
    pub(crate) parse_html: HtmlTemplate,
    pub(crate) standalone_text: TextTemplate,
    pub(crate) name_counter: std::sync::atomic::AtomicU64,
}

/// Go: `needsBaseTemplate(templ)` — the first non-comment, non-whitespace action is `define`.
// Go: tpl/tplimpl/templates.go:needsBaseTemplate
pub fn needs_base_template(templ: &str) -> bool {
    todo!()
}

// Go: tpl/tplimpl/templates.go:removeLeadingBOM
pub fn remove_leading_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/templates.go (366 lines; 10/11 funcs executed)
//   types: templateNamespace
// EX L19-40: (t *templateNamespace) readTemplateInto(templ *TemplInfo) error
// EX L48-54: (s *TemplateStore) parseTemplate(ti *TemplInfo, replace bool) error
// EX L56-125: (t *templateNamespace) doParseTemplate(ti *TemplInfo, replace bool) error
// EX L127-182: (t *templateNamespace) applyBaseTemplate(overlay *TemplInfo, base keyTemplateInfo) error
// EX L184-202: (t *templateNamespace) templatesIn(in tpl.Template) iter.Seq[tpl.Template]
// EX L265-300: needsBaseTemplate(templ string) bool
// EX L302-315: removeLeadingBOM(s string) string
//    L331-338: (t *templateNamespace) createPrototypesParse() error
// EX L340-347: (t *templateNamespace) createPrototypes(init bool) error
// EX L349-355: newTemplateNamespace(funcs map[string]any) *templateNamespace
// EX L357-366: isText(t tpl.Template) bool
// ---------------------------------------------------------------------------
