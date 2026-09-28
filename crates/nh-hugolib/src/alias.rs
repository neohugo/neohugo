//! Port of `hugolib/alias.go`.
//!
//! Owner: Wave B task T24 (hugolib-build).

//! Go `hugolib/alias.go`: alias pages via the embedded `alias.html` (LookupPagesLayout with
//! OutputFormat alias), `targetPathAlias`, published with the page's HTML output format
//! (canonify applies; counts as HTML for hugo_stats).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::Result;
use nh_media::output::output_format::OutputFormat;

use crate::hugo_sites::HugoSites;

/// Go: `aliasPage{Permalink string; page.Page}` — `.Permalink` is the struct FIELD (the redirect
/// target); other members are the embedded page's.
pub struct AliasPage {
    pub permalink: String,
    pub page: Option<nh_page::page::PageRef>,
}

impl Object for AliasPage {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("hugolib.aliasPage")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        self.page
            .as_ref()
            .map(|p| p.has_method(name))
            .unwrap_or(false)
            && name != "Permalink"
    }
    fn call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if name == "Permalink" {
            return None;
        }
        self.page
            .as_ref()
            .and_then(|p| p.call_method(ctx, name, args))
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Permalink" => Some(Value::string(self.permalink.as_str())),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `targetPathAlias(src)`.
// Go: hugolib/alias.go:targetPathAlias
pub fn target_path_alias(src: &str) -> Result<String> {
    todo!()
}

/// Go: `Site.publishDestAlias(allowRoot, path, permalink, outputFormat, p)`.
// Go: hugolib/alias.go:publishDestAlias
pub fn publish_dest_alias(
    h: &Arc<HugoSites>,
    site_idx: usize,
    allow_root: bool,
    path: &str,
    permalink: &str,
    f: &OutputFormat,
    p: Option<nh_page::page::PageRef>,
) -> Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/alias.go (183 lines; 5/5 funcs executed)
//   types: aliasHandler, aliasPage
// EX L41-43: newAliasHandler(ts *tplimpl.TemplateStore, l loggers.Logger, allowRoot bool) aliasHandler
// EX L50-85: (a aliasHandler) renderAlias(permalink string, p page.Page) (io.Reader, error)
// EX L87-89: (s *Site) writeDestAlias(path, permalink string, outputFormat output.Format, p page.Page) (err error)
// EX L91-116: (s *Site) publishDestAlias(allowRoot bool, path, permalink string, outputFormat output.Format, p page.Page) (err error)
// EX L118-183: (a aliasHandler) targetPathAlias(src string) (string, error)
// ---------------------------------------------------------------------------
