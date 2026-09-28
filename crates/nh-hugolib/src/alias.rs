//! Port of `hugolib/alias.go`.
//!
//! Owner: Wave B task T24 (hugolib-build).

//! Go `hugolib/alias.go`: alias pages via the embedded `alias.html` (LookupPagesLayout with
//! OutputFormat alias), `targetPathAlias`, published with the page's HTML output format
//! (canonify applies; counts as HTML for hugo_stats).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::loggers::Logger;
use nh_media::output::output_format::{OutputFormat, builtin_formats};
use nh_publisher::publisher::{Descriptor, Publisher};
use nh_tpl::template::TplContext;
use nh_tplimpl::templatedescriptor::TemplateDescriptor;
use nh_tplimpl::templatestore::{TemplateQuery, TemplateStore};

use crate::hugo_sites::HugoSites;
use crate::page::{PageHandle, PageWrapper};
use crate::template_exec::{ExecCall, ExecKind};

/// Go: `aliasHandler`.
pub struct AliasHandler<'a> {
    pub ts: &'a TemplateStore,
    pub log: &'a Logger,
    pub allow_root: bool,
}

/// Go: `newAliasHandler(ts, l, allowRoot)`.
// Go: hugolib/alias.go:newAliasHandler
pub fn new_alias_handler<'a>(
    ts: &'a TemplateStore,
    l: &'a Logger,
    allow_root: bool,
) -> AliasHandler<'a> {
    AliasHandler {
        ts,
        log: l,
        allow_root,
    }
}

/// Go: `aliasPage{Permalink string; page.Page}` — `.Permalink` and `.Page` are struct FIELDS at
/// depth 0 (the redirect target and the embedded page), so they hide the embedded page's
/// `Permalink` and `Page` methods (Go's selector depth rule, HUGO_LAYER.md §5.1); the other
/// members are the embedded page's promoted methods.
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
        if name == "Permalink" || name == "Page" {
            return false;
        }
        self.page
            .as_ref()
            .map(|p| p.has_method(name))
            .unwrap_or(false)
    }
    fn call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if name == "Permalink" || name == "Page" {
            return None;
        }
        self.page
            .as_ref()
            .and_then(|p| p.call_method(ctx, name, args))
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Permalink" => Some(Value::string(self.permalink.as_str())),
            // The embedded `page.Page`: a nil interface for the main-language redirect.
            "Page" => Some(match &self.page {
                Some(p) => p.to_value(),
                None => Value::TypedNil(Arc::from("page.Page")),
            }),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl AliasHandler<'_> {
    /// Go: `renderAlias(permalink, p)` — the alias template (a layout looked up with the page's
    /// template descriptor, but the `alias` output format and no kind or user layout), executed
    /// with `aliasPage{permalink, p}` and the page in the context.
    // Go: hugolib/alias.go:renderAlias
    pub fn render_alias(
        &self,
        h: &Arc<HugoSites>,
        site_idx: usize,
        permalink: &str,
        p: Option<&nh_page::page::PageRef>,
    ) -> Result<Vec<u8>> {
        let mut template_desc = TemplateDescriptor::default();
        let mut base = String::new();
        let mut page_id = None;
        if let Some(p) = p
            && let Some(ph) = p.0.as_any().downcast_ref::<PageHandle>()
            && ph.wrapper == PageWrapper::None
        {
            let ps = ph.state();
            page_id = Some(ph.id);
            if let Some(po) = ps.current_output_opt() {
                (base, template_desc) = po.get_internal_template_base_path_and_descriptor(ps);
            }
        }
        let alias = &builtin_formats().alias_html;
        template_desc.layout_from_user = String::new();
        template_desc.kind = String::new();
        template_desc.output_format = alias.name.clone();
        template_desc.media_type = alias.media_type.typ.clone();

        let q = TemplateQuery {
            path: base,
            name: String::new(),
            category: nh_tplimpl::category::Category::Layout,
            desc: template_desc,
            consider: None,
        };

        let Some(t) = self.ts.lookup_pages_layout(&q) else {
            return Err(Error::new("no alias template found"));
        };

        let data = Value::object(AliasPage {
            permalink: permalink.to_string(),
            page: p.cloned(),
        });

        let ctx = TplContext {
            page: p.map(|p| p.to_value()),
            ..Default::default()
        };

        let mut buffer = Vec::new();
        crate::template_exec::execute(
            h,
            site_idx,
            &ctx,
            &t,
            &mut buffer,
            &data,
            &ExecCall {
                page: page_id,
                output_format: alias.name.clone(),
                kind: ExecKind::Alias,
                ordinal: 0,
            },
        )?;
        Ok(buffer)
    }

    /// Go: `targetPathAlias(src)`.
    // Go: hugolib/alias.go:targetPathAlias
    pub fn target_path_alias(&self, src: &str) -> Result<String> {
        target_path_alias_with(self.log, self.allow_root, src)
    }
}

/// Go: `targetPathAlias(src)` of an alias handler that does not allow the root, logging to the
/// global logger (the skeleton's entry point; the render loop uses [`AliasHandler`]).
// Go: hugolib/alias.go:targetPathAlias
pub fn target_path_alias(src: &str) -> Result<String> {
    target_path_alias_with(&nh_common::loggers::log(), false, src)
}

/// Go: `(a aliasHandler) targetPathAlias(src)` — the file an alias is written to:
/// `path.Clean`ed, not the website root (unless `allow_root`), no `..` traversal; names Windows
/// cannot store are only logged (at INFO) off Windows; `index.html` is appended unless the
/// alias ends with `.html`.
// Go: hugolib/alias.go:targetPathAlias
pub fn target_path_alias_with(log: &Logger, allow_root: bool, src: &str) -> Result<String> {
    let original_alias = src;
    if src.is_empty() {
        return Err(Error::new("alias \"\" is an empty string"));
    }

    // filepath.ToSlash is the identity off Windows.
    let mut alias = go_path::path::clean(src);

    if !allow_root && alias == "/" {
        return Err(Error::new(format!(
            "alias \"{original_alias}\" resolves to website root directory"
        )));
    }

    let components: Vec<&str> = alias.split('/').collect();

    // Validate against directory traversal
    if components[0] == ".." {
        return Err(Error::new(format!(
            "alias \"{original_alias}\" traverses outside the website root directory"
        )));
    }

    // Handle Windows file and directory naming restrictions
    // See "Naming Files, Paths, and Namespaces" on MSDN
    // https://msdn.microsoft.com/en-us/library/aa365247%28v=VS.85%29.aspx?f=255&MSPPError=-2147217396
    let mut msgs: Vec<String> = Vec::new();
    const RESERVED_NAMES: [&str; 24] = [
        "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
        "LPT9",
    ];

    if alias.contains([':', '*', '?', '"', '<', '>', '|']) {
        msgs.push(format!(
            "Alias \"{original_alias}\" contains invalid characters on Windows: : * ? \" < > |"
        ));
    }
    // Go ranges over the runes; a rune below ' ' is a single ASCII byte in UTF-8.
    for ch in alias.chars() {
        if ch < ' ' {
            msgs.push(format!(
                "Alias \"{original_alias}\" contains ASCII control code (0x00 to 0x1F), invalid on Windows: : * ? \" < > |"
            ));
        }
    }
    for comp in &components {
        if comp.ends_with(' ') || comp.ends_with('.') {
            msgs.push(format!(
                "Alias \"{original_alias}\" contains component with a trailing space or period, problematic on Windows"
            ));
        }
        for r in RESERVED_NAMES {
            if *comp == r {
                msgs.push(format!(
                    "Alias \"{original_alias}\" contains component with reserved name \"{r}\" on Windows"
                ));
            }
        }
    }
    // runtime.GOOS is never "windows" for the port's targets: log only.
    for m in &msgs {
        log.infof(m);
    }

    // Add the final touch
    if let Some(rest) = alias.strip_prefix('/') {
        alias = rest.to_string();
    }
    if alias.ends_with('/') {
        alias.push_str("index.html");
    } else if !alias.ends_with(".html") {
        alias.push_str("/index.html");
    }

    // filepath.FromSlash is the identity off Windows.
    Ok(alias)
}

/// Go: `(s *Site) writeDestAlias(path, permalink, outputFormat, p)` — an alias that must not
/// resolve to the root.
// Go: hugolib/alias.go:writeDestAlias
pub fn write_dest_alias(
    h: &Arc<HugoSites>,
    site_idx: usize,
    path: &str,
    permalink: &str,
    f: &OutputFormat,
    p: Option<nh_page::page::PageRef>,
) -> Result<()> {
    publish_dest_alias(h, site_idx, false, path, permalink, f, p)
}

/// Go: `Site.publishDestAlias(allowRoot, path, permalink, outputFormat, p)` — renders the alias
/// page and publishes it (canonified with `relativeURLs`/`canonifyURLs`; counted in the
/// `Aliases` stat; an HTML format, so it feeds the build stats).
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
    let s = &h.sites[site_idx];
    let handler = new_alias_handler(s.deps.get_template_store(), &s.deps.log, allow_root);

    let target_path = handler.target_path_alias(path)?;

    let alias_content = handler.render_alias(h, site_idx, permalink, p.as_ref())?;

    let mut pd = Descriptor {
        src: &alias_content,
        target_path: target_path.clone(),
        stat_counter: Some(&s.deps.path_spec().processing_stats.aliases),
        output_format: f.clone(),
        live_reload_base_url: None,
        add_hugo_generator_tag: false,
        abs_url_path: String::new(),
        minify: false,
    };

    if s.conf.root.relative_urls || s.conf.root.canonify_urls {
        pd.abs_url_path = s.abs_url_path(&target_path);
    }

    s.publisher.publish(pd)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/alias.go (183 lines; 5/5 funcs executed)
//   types: aliasHandler, aliasPage
// OK L41-43: newAliasHandler(ts *tplimpl.TemplateStore, l loggers.Logger, allowRoot bool) aliasHandler
// OK L50-85: (a aliasHandler) renderAlias(permalink string, p page.Page) (io.Reader, error)
// OK L87-89: (s *Site) writeDestAlias(path, permalink string, outputFormat output.Format, p page.Page) (err error)
// OK L91-116: (s *Site) publishDestAlias(allowRoot bool, path, permalink string, outputFormat output.Format, p page.Page) (err error)
// OK L118-183: (a aliasHandler) targetPathAlias(src string) (string, error)
// ---------------------------------------------------------------------------
