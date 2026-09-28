//! Port of `tpl/css/css.go` (the namespace and its `init()`, which lives in this file in Go).
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! The clients are T16's (`nh_resource_transformers::resource_transformers`): LibSass
//! (`tocss::scss::tocss::Client`), PostCSS (`cssjs::postcss`), TailwindCSS and Babel (explicit
//! stubs there). None of them holds state (the LibSass transpiler and the PostCSS process run per
//! transformation), so the `resources` namespace's css namespace (`Namespace::new` on the same
//! deps, Go's `OnCreated`) behaves exactly like the one in the func map. Dart Sass is not ported
//! (`transpiler: dartsass` is an explicit error).

use std::sync::Arc;

use go_value::{HostCtx, Map, Object, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_common::types::css::{QuotedString, UnquotedString};
use nh_deps::deps::Deps;
use nh_resource_transformers::resource_transformers::babel;
use nh_resource_transformers::resource_transformers::cssjs::postcss::{
    PostCssClient, new_post_css_client,
};
use nh_resource_transformers::resource_transformers::cssjs::tailwindcss::{
    TailwindCssClient, new_tailwind_css_client,
};
use nh_resource_transformers::resource_transformers::tocss::sass::helpers::{
    TRANSPILER_DART, TRANSPILER_LIB_SASS,
};
use nh_resource_transformers::resource_transformers::tocss::scss::tocss::{
    Client as ScssClient, Options as ScssOptions, decode_options,
};
use nh_tpl::template::TplContext;

use crate::internal::resourcehelpers::{resolve_args_go, resolve_if_first_arg_is_string_go};

/// The clients Go creates in `init` (absent when the deps have no resource spec).
struct Clients {
    scss_client_lib_sass: ScssClient,
    postcss_client: PostCssClient,
    tailwindcss_client: TailwindCssClient,
    #[allow(dead_code)]
    babel_client: babel::Client,
}

/// Go: `css.Namespace` (template value `*css.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    clients: Option<Clients>,
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn nil_deref() -> go_value::Error {
    gerr("runtime error: invalid memory address or nil pointer dereference")
}

fn tctx<'a>(ctx: HostCtx<'a>, default: &'a TplContext) -> &'a TplContext {
    TplContext::from_host(ctx).unwrap_or(default)
}

impl Namespace {
    /// Go: the namespace built by `init()` (`scss.New(d.Assets, d.ResourceSpec)` panics on
    /// error, as here).
    // Go: tpl/css/css.go:init
    pub fn new(d: Arc<Deps>) -> Namespace {
        let clients = d.resource_spec.clone().map(|rs| {
            let scss_client = match ScssClient::new(rs.path_spec.base_fs.assets.clone(), rs.clone())
            {
                Ok(c) => c,
                Err(e) => panic!("{}", e.message()),
            };
            Clients {
                scss_client_lib_sass: scss_client,
                postcss_client: new_post_css_client(rs.clone()),
                tailwindcss_client: new_tailwind_css_client(rs.clone()),
                babel_client: babel::Client::new(rs),
            }
        });
        Namespace { d, clients }
    }

    fn clients(&self) -> GoResult<&Clients> {
        self.clients.as_ref().ok_or_else(nil_deref)
    }

    /// PostCSS processes the given Resource with PostCSS.
    // Go: tpl/css/css.go:PostCSS
    pub fn post_css(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.len() > 2 {
            return Err(gerr(
                "must not provide more arguments than resource object and options",
            ));
        }

        let (r, m) = resolve_args_go(a)?;

        let default = TplContext::default();
        let res = self
            .clients()?
            .postcss_client
            .process(tctx(ctx, &default), r, m.as_ref())?;
        Ok(res.to_value())
    }

    /// Quoted returns a string that needs to be quoted in CSS.
    // Go: tpl/css/css.go:Quoted
    pub fn quoted(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Quoted")?;
        Ok(Value::object(QuotedString(caste::to_string(&a[0]))))
    }

    /// Sass processes the given Resource with SASS.
    // Go: tpl/css/css.go:Sass
    pub fn sass(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.len() > 2 {
            return Err(gerr(
                "must not provide more arguments than resource object and options",
            ));
        }

        let mut m: Option<Map> = None;
        let mut transpiler = TRANSPILER_LIB_SASS.to_string();

        let (r, target_path, ok) = resolve_if_first_arg_is_string_go(a);

        let r = match (r, ok) {
            (Some(r), true) => r,
            _ => {
                let (r, mm) = resolve_args_go(a)?;
                m = mm;
                r
            }
        };
        // (Go keeps the target path only when the first form matched.)
        let target_path = if ok { target_path } else { String::new() };

        if let Some(m) = &m
            && let Some((t, _)) = nh_common::maps::maps::lookup_equal_fold(m, b"transpiler")
        {
            match t {
                Value::String(s)
                    if s.as_bytes() == TRANSPILER_DART.as_bytes()
                        || s.as_bytes() == TRANSPILER_LIB_SASS.as_bytes() =>
                {
                    transpiler = caste::to_string(t).to_str_lossy().into_owned();
                }
                _ => {
                    return Err(gerr(format!(
                        "unsupported transpiler {}; valid values are {} or {}",
                        String::from_utf8_lossy(&go_fmt::sprintf("%q", std::slice::from_ref(t))),
                        go_strconv::quote(TRANSPILER_LIB_SASS.as_bytes()),
                        go_strconv::quote(TRANSPILER_DART.as_bytes())
                    )));
                }
            }
        }

        if transpiler == TRANSPILER_LIB_SASS {
            let mut options = ScssOptions::default();
            if !target_path.is_empty() {
                options.target_path = nh_common::paths::path::to_slash_trim_leading(&target_path);
            } else if m.is_some() {
                options = decode_options(m.as_ref())?;
            }

            let default = TplContext::default();
            let res =
                self.clients()?
                    .scss_client_lib_sass
                    .to_css(tctx(ctx, &default), r, options)?;
            return Ok(res.to_value());
        }

        // Go: the Dart Sass client (`getscssClientDartSass`), an external process.
        Err(gerr(
            "neohugo-rs: css.Sass with transpiler \"dartsass\" (Dart Sass) is not supported",
        ))
    }

    /// TailwindCSS processes the given Resource with tailwindcss (an explicit T16 stub).
    // Go: tpl/css/css.go:TailwindCSS
    pub fn tailwind_css(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.len() > 2 {
            return Err(gerr(
                "must not provide more arguments than resource object and options",
            ));
        }

        let (r, m) = resolve_args_go(a)?;

        let default = TplContext::default();
        let res = self
            .clients()?
            .tailwindcss_client
            .process(tctx(ctx, &default), r, m.as_ref())?;
        Ok(res.to_value())
    }

    /// Unquoted returns a string that does not need to be quoted in CSS.
    // Go: tpl/css/css.go:Unquoted
    pub fn unquoted(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Unquoted")?;
        Ok(Value::object(UnquotedString(caste::to_string(&a[0]))))
    }
}

nh_common::go_methods!(Namespace {
    "PostCSS" => |n, ctx, a| n.post_css(ctx, a),
    "Quoted" => |n, ctx, a| n.quoted(ctx, a),
    "Sass" => |n, ctx, a| n.sass(ctx, a),
    "TailwindCSS" => |n, ctx, a| n.tailwind_css(ctx, a),
    "Unquoted" => |n, ctx, a| n.unquoted(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*css.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/css/css.go (192 lines; 3/7 funcs executed)
//   types: Namespace
// OK L43-46: (ns *Namespace) Quoted(v any) css.QuotedString
// OK L49-52: (ns *Namespace) Unquoted(v any) css.UnquotedString
// OK L55-66: (ns *Namespace) PostCSS(args ...any) (resource.Resource, error)
// OK L69-80: (ns *Namespace) TailwindCSS(args ...any) (resource.Resource, error) (the client is a T16 STUB)
// OK L83-144: (ns *Namespace) Sass(args ...any) (resource.Resource, error) (Dart Sass: explicit unsupported error)
// OK L146-179: init()
// STUB L181-192: (ns *Namespace) getscssClientDartSass() (*dartsass.Client, error)
// ---------------------------------------------------------------------------
