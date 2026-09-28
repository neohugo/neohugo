//! Port of `tpl/js/js.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! `js.Build` runs T16's esbuild client (`js::build::Client::new_default`), which starts one
//! esbuild service on its first build. Go creates the client once per site (in `New`, called by
//! the namespace `init`), and the `resources` namespace reaches the same namespace through
//! `OnCreated`. The port creates the clients once per `Deps` and shares them between every
//! `Namespace::new(d)` on those deps (the func map's and T15's `resources::Namespace::js_ns`).
//!
//! STUB: `Batch` (the JS batcher is not ported); `Babel` is a T16 stub.

use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, MapType, Object, Value};
use nh_common::dynacache::{ClearWhen, OptionsPartition, get_or_create_partition};
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;
use nh_resource_transformers::resource_transformers::babel;
use nh_resource_transformers::resource_transformers::js::build::Client as JsTransformClient;
use nh_tpl::template::TplContext;

use crate::internal::resourcehelpers::{resolve_args_go, resolve_if_first_arg_is_string_go};

/// The clients Go creates in `New` (absent when the deps have no resource spec: Go's empty
/// `&Namespace{}`).
struct Clients {
    js_transform_client: JsTransformClient,
    babel_client: babel::Client,
}

/// Go: `js.Namespace` (template value `*js.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    clients: Option<Arc<Clients>>,
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn nil_deref() -> go_value::Error {
    gerr("runtime error: invalid memory address or nil pointer dereference")
}

impl Namespace {
    /// New returns a new instance of the js-namespaced template functions (the clients are
    /// shared by all namespaces on the same deps, see the module docs).
    // Go: tpl/js/js.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let Some(rs) = d.resource_spec.clone() else {
            return Namespace { d, clients: None };
        };

        let shared = get_or_create_partition::<usize, Arc<Clients>>(
            &d.mem_cache,
            "/tmpl/js/clients",
            OptionsPartition {
                weight: 1,
                clear_when: ClearWhen::Never,
            },
        );
        let key = Arc::as_ptr(&d) as usize;
        let clients = shared
            .get_or_create(key, |_| {
                Ok(Arc::new(Clients {
                    js_transform_client: JsTransformClient::new_default(rs.clone()),
                    babel_client: babel::Client::new(rs.clone()),
                }))
            })
            .expect("creating the js clients cannot fail");

        Namespace {
            d,
            clients: Some(clients),
        }
    }

    fn clients(&self) -> GoResult<&Clients> {
        self.clients.as_deref().ok_or_else(nil_deref)
    }

    /// Babel processes the given Resource with Babel.
    // Go: tpl/js/js.go:Babel
    pub fn babel(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.len() > 2 {
            return Err(gerr(
                "must not provide more arguments than resource object and options",
            ));
        }

        let (r, m) = resolve_args_go(a)?;
        let mut options = babel::Options::default();
        if m.is_some() {
            options = babel::decode_options(m.as_ref())?;
        }

        let default = TplContext::default();
        let c = self.clients()?;
        let res =
            c.babel_client
                .process(TplContext::from_host(ctx).unwrap_or(&default), r, options)?;
        Ok(res.to_value())
    }

    /// Batch creates a new Batcher with the given ID. STUB: the JS batcher
    /// (`internal/js/esbuild` batch) is not ported.
    // Go: tpl/js/js.go:Batch
    pub fn batch(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Batch")?;
        args::string(a, 0)?;
        Err(gerr("neohugo-rs: js.Batch is not supported"))
    }

    /// Build processes the given Resource with ESBuild.
    // Go: tpl/js/js.go:Build
    pub fn build(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        let (r, target_path, ok) = resolve_if_first_arg_is_string_go(a);

        let (r, mut m) = match (r, ok) {
            (Some(r), true) => (r, None),
            _ => resolve_args_go(a)?,
        };
        let target_path = if ok { target_path } else { String::new() };

        if !target_path.is_empty() {
            let mut mm = Map::new(MapType::StringAny);
            mm.entries.insert(
                GoString::from("targetPath"),
                Value::string(target_path.as_str()),
            );
            m = Some(mm);
        }

        let default = TplContext::default();
        let c = self.clients()?;
        let res = c.js_transform_client.process(
            TplContext::from_host(ctx).unwrap_or(&default),
            r,
            m.as_ref(),
        )?;
        Ok(res.to_value())
    }
}

nh_common::go_methods!(Namespace {
    "Babel" => |n, ctx, a| n.babel(ctx, a),
    "Batch" => |n, ctx, a| n.batch(ctx, a),
    "Build" => |n, ctx, a| n.build(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*js.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/js/js.go (117 lines; 2/4 funcs executed)
//   types: Namespace
// OK L32-43: New(d *deps.Deps) (*Namespace, error)
// OK L55-78: (ns *Namespace) Build(args ...any) (resource.Resource, error)
// STUB L84-96: (ns *Namespace) Batch(id string) (js.Batcher, error)
// OK L99-117: (ns *Namespace) Babel(args ...any) (resource.Resource, error) (the client is a T16 STUB)
// ---------------------------------------------------------------------------
