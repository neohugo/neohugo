//! Port of `tpl/resources/resources.go`.
//!
//! Owner: Wave B task T15 (resource-factories).
//!
//! Methods whose Go form panics (`Get`, `GetMatch`, `Match`, `ByType`, `Copy`'s cast) return the
//! panic value as the error (text/template reports a panicking method call as an error).
//! Parameters with a static Go type (`Copy`'s and `PostProcess`'s `resource.Resource`,
//! `Minify`'s `resources.ResourceTransformer`) are checked like text/template's
//! `validateType` (`wrong type for value; expected ...; got ...`).

use std::sync::Arc;

use go_value::{HostCtx, Map, Object, Value};
use nh_common::cast::caste;
use nh_common::herrors::Error;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;
use nh_resource::resourcetypes::{
    RESOURCE_TYPE, RESOURCES_TYPE, Resource, Resources, resource_from_value_any, resources_to_value,
};
use nh_resource_transformers::resource_factories::{bundler, create::create};
use nh_resource_transformers::resource_transformers::{integrity, minifier, templates};
use nh_tpl::template::TplContext;
use nh_tplimpl::templatestore::{TemplateStore, TemplateStoreProvider};

// Parity notes: `Get` (nil for missing), `GetRemote` (getresource file cache), `Concat` (first caller wins per target path), `ExecuteAsTemplate`, `Fingerprint`, `Minify`, `PostProcess`, `ToCSS`/`PostCSS` (delegating to tpl/css).

/// The clients Go creates in `New` (absent when the deps have no resource spec: Go's empty
/// `&Namespace{}`, whose methods dereference nil pointers).
struct Clients {
    create_client: Arc<create::Client>,
    bundler_client: bundler::Client,
    integrity_client: integrity::Client,
    minify_client: minifier::Client,
    templates_client: templates::Client,
}

/// Go: `resources.Namespace` (template value `*resources.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    clients: Option<Clients>,
}

/// Go's `*deps.Deps` as a `tplimpl.TemplateStoreProvider` (the store is set on the deps after
/// the namespaces are created).
struct DepsStoreProvider(Arc<Deps>);

impl TemplateStoreProvider for DepsStoreProvider {
    fn get_template_store(&self) -> TemplateStore {
        self.0.get_template_store().clone()
    }
}

fn nil_deref() -> go_value::Error {
    go_value::Error::new("runtime error: invalid memory address or nil pointer dereference")
}

fn gerr(e: Error) -> go_value::Error {
    e.into()
}

/// A `resource.Resource` result (`TypedNil` for Go's nil interface).
fn resource_value(r: Option<Arc<dyn Resource>>) -> Value {
    match r {
        Some(r) => r.to_value(),
        None => Value::TypedNil(Arc::from(RESOURCE_TYPE)),
    }
}

/// A `resource.Resources` result (an empty list is Go's nil slice here: the create client only
/// appends).
fn resources_value(r: &Resources) -> Value {
    if r.is_empty() {
        return Value::TypedNil(Arc::from(RESOURCES_TYPE));
    }
    resources_to_value(r)
}

/// text/template's check of a `resource.Resource` parameter.
fn resource_param(v: &Value) -> GoResult<Arc<dyn Resource>> {
    if v.is_invalid() {
        // (Go passes a nil interface; the method dereferences it.)
        return Err(nil_deref());
    }
    resource_from_value_any(v).ok_or_else(|| args::wrong_type(RESOURCE_TYPE, v))
}

/// text/template's check of a `resources.ResourceTransformer` parameter.
fn transformer_param(v: &Value) -> GoResult<Arc<dyn Resource>> {
    if v.is_invalid() {
        return Err(nil_deref());
    }
    crate::internal::resourcehelpers::transformer_from_value(v)
        .map(|r| r as Arc<dyn Resource>)
        .ok_or_else(|| args::wrong_type("resources.ResourceTransformer", v))
}

impl Namespace {
    /// Go: `New(deps)` — Go's template func init panics when the minifier config is invalid;
    /// so does this (see [`Namespace::try_new`]).
    // Go: tpl/resources:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        match Namespace::try_new(d) {
            Ok(ns) => ns,
            Err(e) => panic!("{}", e.message()),
        }
    }

    /// New returns a new instance of the resources-namespaced template functions.
    // Go: tpl/resources/resources.go:New
    pub fn try_new(d: Arc<Deps>) -> nh_common::Result<Namespace> {
        let Some(rs) = d.resource_spec.clone() else {
            return Ok(Namespace { d, clients: None });
        };

        let minify_client = minifier::Client::new(rs.clone())?;

        let clients = Clients {
            create_client: create::Client::new(rs.clone())?,
            bundler_client: bundler::Client::new(rs.clone()),
            integrity_client: integrity::Client::new(rs.clone()),
            minify_client,
            templates_client: templates::Client::new(
                rs.clone(),
                Arc::new(DepsStoreProvider(d.clone())),
            ),
        };
        Ok(Namespace {
            d,
            clients: Some(clients),
        })
    }

    fn clients(&self) -> GoResult<&Clients> {
        self.clients.as_ref().ok_or_else(nil_deref)
    }

    /// Go: `ns.cssNs` / `ns.jsNs` are the css and js namespaces of the same func map (set in
    /// `OnCreated`); the port creates them over the same deps.
    fn css_ns(&self) -> crate::css::css::Namespace {
        crate::css::css::Namespace::new(self.d.clone())
    }

    fn js_ns(&self) -> crate::js::js::Namespace {
        crate::js::js::Namespace::new(self.d.clone())
    }

    /// Babel processes the given Resource with Babel. Deprecated: moved to the js namespace in
    /// Hugo 0.128.0.
    // Go: tpl/resources:Babel
    pub fn babel(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        nh_config::neohugo::neohugo::deprecate("resources.Babel", "Use js.Babel.", "v0.128.0");
        self.js_ns().babel(ctx, args)
    }

    /// ByType returns resources of a given resource type (e.g. "image").
    // Go: tpl/resources:ByType
    pub fn by_type(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ByType")?;
        let c = self.clients()?;
        let tp = caste::to_string(&a[0]);
        let res = c
            .create_client
            .by_type(&String::from_utf8_lossy(tp.as_bytes()))
            .map_err(gerr)?;
        Ok(resources_value(&res))
    }

    /// Concat concatenates a slice of Resource objects. These resources must (currently) be of
    /// the same Media Type.
    // Go: tpl/resources:Concat
    pub fn concat(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Concat")?;
        let target_path = caste::to_string_e(&a[0])?;
        let c = self.clients()?;

        let rr: Resources = match &a[1] {
            Value::List(l) if &*l.ty.go_name() == RESOURCES_TYPE => {
                nh_resource::resources::values_to_resources(&l.items).ok_or_else(|| {
                    go_value::Error::new("resource.Resources holds a non-resource value")
                })?
            }
            Value::TypedNil(t) if &**t == RESOURCES_TYPE => Vec::new(),
            // Go `resource.ResourcesConverter` (`page.Pages.ToResources`).
            Value::List(l) if &*l.ty.go_name() == "page.Pages" => {
                nh_resource::resources::values_to_resources(&l.items)
                    .ok_or_else(|| go_value::Error::new("page.Pages holds a non-resource value"))?
            }
            Value::TypedNil(t) if &**t == "page.Pages" => Vec::new(),
            v => {
                return Err(go_value::Error::new(format!(
                    "expected slice of Resource objects, received {} instead",
                    v.go_type_name()
                )));
            }
        };

        if rr.is_empty() {
            return Err(go_value::Error::new(
                "must provide one or more Resource objects to concat",
            ));
        }

        let r = c
            .bundler_client
            .concat(&String::from_utf8_lossy(target_path.as_bytes()), &rr)
            .map_err(gerr)?;
        Ok(r.to_value())
    }

    /// Copy copies r to the new targetPath in s.
    // Go: tpl/resources:Copy
    pub fn copy(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Copy")?;
        let r = resource_param(&a[1])?;
        // (Go panics on a cast error.)
        let target_path = caste::to_string_e(&a[0])?;
        let c = self.clients()?;
        let res = c
            .create_client
            .copy(&r, &String::from_utf8_lossy(target_path.as_bytes()))
            .map_err(gerr)?;
        Ok(res.to_value())
    }

    /// ExecuteAsTemplate creates a Resource from a Go template, parsed and executed with the
    /// given data, and published to the relative target path.
    // Go: tpl/resources:ExecuteAsTemplate
    pub fn execute_as_template(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.len() != 3 {
            return Err(go_value::Error::new(
                "must provide targetPath, the template data context and a Resource object",
            ));
        }
        let target_path = caste::to_string_e(&a[0])?;
        let data = a[1].clone();

        let Some(r) = crate::internal::resourcehelpers::transformer_from_value(&a[2]) else {
            return Err(go_value::Error::new(format!(
                "type {} not supported in Resource transformations",
                a[2].go_type_name()
            )));
        };

        let c = self.clients()?;
        let tctx = TplContext::from_host(ctx).cloned().unwrap_or_default();
        let res = c
            .templates_client
            .execute_as_template(
                &tctx,
                r,
                &String::from_utf8_lossy(target_path.as_bytes()),
                data,
            )
            .map_err(gerr)?;
        Ok(res.to_value())
    }

    /// Fingerprint transforms the given Resource with a MD5 hash of the content in the
    /// RelPermalink and Permalink.
    // Go: tpl/resources:Fingerprint
    pub fn fingerprint(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.is_empty() {
            return Err(go_value::Error::new("must provide a Resource object"));
        }

        if a.len() > 2 {
            return Err(go_value::Error::new(
                "must not provide more arguments than Resource and hash algorithm",
            ));
        }

        let mut algo = String::new();
        let mut res_idx = 0;

        if a.len() == 2 {
            res_idx = 1;
            algo = String::from_utf8_lossy(caste::to_string_e(&a[0])?.as_bytes()).into_owned();
        }

        let Some(r) = crate::internal::resourcehelpers::transformer_from_value(&a[res_idx]) else {
            return Err(go_value::Error::new(format!(
                "{} can not be transformed",
                a[res_idx].go_type_name()
            )));
        };

        let c = self.clients()?;
        let tctx = TplContext::from_host(ctx).cloned().unwrap_or_default();
        let res = c
            .integrity_client
            .fingerprint(&tctx, r, &algo)
            .map_err(gerr)?;
        Ok(res.to_value())
    }

    /// FromString creates a Resource from a string published to the relative target path.
    // Go: tpl/resources:FromString
    pub fn from_string(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "FromString")?;
        let target_path = caste::to_string_e(&a[0])?;
        let content = caste::to_string_e(&a[1])?;

        let c = self.clients()?;
        let r = c
            .create_client
            .from_string(
                &String::from_utf8_lossy(target_path.as_bytes()),
                content.as_bytes(),
            )
            .map_err(gerr)?;
        Ok(r.to_value())
    }

    /// Get locates the filename given in Hugo's assets filesystem and creates a Resource object
    /// that can be used for further transformations.
    // Go: tpl/resources:Get
    pub fn get(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Get")?;
        let filenamestr = caste::to_string_e(&a[0])?;

        if filenamestr.is_empty() {
            return Ok(resource_value(None));
        }

        let c = self.clients()?;
        let r = c
            .create_client
            .get(&String::from_utf8_lossy(filenamestr.as_bytes()))
            .map_err(gerr)?;

        Ok(resource_value(r))
    }

    /// GetMatch finds the first Resource matching the given pattern, or nil if none found.
    // Go: tpl/resources:GetMatch
    pub fn get_match(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "GetMatch")?;
        let pattern_str = caste::to_string_e(&a[0])?;

        let c = self.clients()?;
        let r = c
            .create_client
            .get_match(&String::from_utf8_lossy(pattern_str.as_bytes()))
            .map_err(gerr)?;

        Ok(resource_value(r))
    }

    /// GetRemote gets the URL (via HTTP(s)) in the first argument in args and creates Resource
    /// object that can be used for further transformations. A second argument may be provided
    /// with an option map.
    ///
    /// Go wraps every error in a `resource.ResourceError` (with the HTTP error's `Data`); the
    /// template error carries its message (go_value errors have no data).
    // Go: tpl/resources:GetRemote
    pub fn get_remote(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        let get = |a: &[Value]| -> nh_common::Result<Option<Arc<dyn Resource>>> {
            if a.is_empty() || a.len() > 2 {
                return Err(Error::new(
                    "must provide an URL and optionally an options map",
                ));
            }

            let urlstr = caste::to_string_e(&a[0])?;

            let mut options: Option<Map> = None;

            if a.len() > 1 {
                options = Some(nh_common::maps::maps::to_string_map_e(&a[1])?);
            }

            let c = self.clients().map_err(Error::from)?;
            c.create_client.from_remote(
                &String::from_utf8_lossy(urlstr.as_bytes()),
                options.as_ref(),
            )
        };

        match get(a) {
            Ok(r) => Ok(resource_value(r)),
            Err(err) => {
                let re = nh_resource::resourcetypes::ResourceError::new(&err, None);
                Err(go_value::Error::new(re.msg))
            }
        }
    }

    /// Match gets all resources matching the given base path prefix, e.g "*.png" will match
    /// all png files. The matching is case insensitive. It looks for files in the assets file
    /// system.
    // Go: tpl/resources:Match
    pub fn match_(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Match")?;
        let pattern_str = caste::to_string_e(&a[0])?;

        let c = self.clients()?;
        let r = c
            .create_client
            .match_(&String::from_utf8_lossy(pattern_str.as_bytes()))
            .map_err(gerr)?;

        Ok(resources_value(&r))
    }

    /// Minify minifies the given Resource using the MediaType to pick the correct minifier.
    // Go: tpl/resources:Minify
    pub fn minify(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Minify")?;
        let r = transformer_param(&a[0])?;
        let c = self.clients()?;
        let tctx = TplContext::from_host(ctx).cloned().unwrap_or_default();
        let res = c.minify_client.minify(&tctx, r).map_err(gerr)?;
        Ok(res.to_value())
    }

    /// PostCSS processes the given Resource with PostCSS. Deprecated: moved to the css
    /// namespace in Hugo 0.128.0.
    // Go: tpl/resources:PostCSS
    pub fn post_css(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        nh_config::neohugo::neohugo::deprecate(
            "resources.PostCSS",
            "Use css.PostCSS instead.",
            "v0.128.0",
        );
        self.css_ns().post_css(ctx, args)
    }

    /// PostProcess processes r after the build.
    // Go: tpl/resources:PostProcess
    pub fn post_process(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "PostProcess")?;
        let r = resource_param(&a[0])?;
        let rs = self.d.resource_spec.as_ref().ok_or_else(nil_deref)?;
        let p = rs.post_process(r).map_err(gerr)?;
        Ok(p.to_value())
    }

    /// ToCSS converts the given Resource to CSS. Deprecated: moved to the css namespace in Hugo
    /// 0.128.0.
    // Go: tpl/resources:ToCSS
    pub fn to_css(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        nh_config::neohugo::neohugo::deprecate(
            "resources.ToCSS",
            "Use css.Sass instead.",
            "v0.128.0",
        );
        self.css_ns().sass(ctx, args)
    }
}

nh_common::go_methods!(Namespace {
    "Babel" => |n, ctx, a| n.babel(ctx, a),
    "ByType" => |n, ctx, a| n.by_type(ctx, a),
    "Concat" => |n, ctx, a| n.concat(ctx, a),
    "Copy" => |n, ctx, a| n.copy(ctx, a),
    "ExecuteAsTemplate" => |n, ctx, a| n.execute_as_template(ctx, a),
    "Fingerprint" => |n, ctx, a| n.fingerprint(ctx, a),
    "FromString" => |n, ctx, a| n.from_string(ctx, a),
    "Get" => |n, ctx, a| n.get(ctx, a),
    "GetMatch" => |n, ctx, a| n.get_match(ctx, a),
    "GetRemote" => |n, ctx, a| n.get_remote(ctx, a),
    "Match" => |n, ctx, a| n.match_(ctx, a),
    "Minify" => |n, ctx, a| n.minify(ctx, a),
    "PostCSS" => |n, ctx, a| n.post_css(ctx, a),
    "PostProcess" => |n, ctx, a| n.post_process(ctx, a),
    "ToCSS" => |n, ctx, a| n.to_css(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*resources.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/resources/resources.go (330 lines; 8/16 funcs executed)
//   types: Namespace
// OK L44-62: New(deps *deps.Deps) (*Namespace, error)
// OK L83-89: (ns *Namespace) Copy(s any, r resource.Resource) (resource.Resource, error)
// OK L93-109: (ns *Namespace) Get(filename any) resource.Resource
// OK L118-151: (ns *Namespace) GetRemote(args ...any) (resource.Resource, error)
// OK L158-170: (ns *Namespace) GetMatch(pattern any) resource.Resource
// OK L173-175: (ns *Namespace) ByType(typ any) resource.Resources
// OK L193-205: (ns *Namespace) Match(pattern any) resource.Resources
// OK L209-231: (ns *Namespace) Concat(targetPathIn any, r any) (resource.Resource, error)
// OK L234-245: (ns *Namespace) FromString(targetPathIn, contentIn any) (resource.Resource, error)
// OK L249-265: (ns *Namespace) ExecuteAsTemplate(ctx context.Context, args ...any) (resource.Resource, error)
// OK L269-296: (ns *Namespace) Fingerprint(args ...any) (resource.Resource, error)
// OK L300-302: (ns *Namespace) Minify(r resources.ResourceTransformer) (resource.Resource, error)
// OK L308-311: (ns *Namespace) ToCSS(args ...any) (resource.Resource, error)
// OK L315-318: (ns *Namespace) PostCSS(args ...any) (resource.Resource, error)
// OK L321-323: (ns *Namespace) PostProcess(r resource.Resource) (postpub.PostPublishedResource, error)
// OK L327-330: (ns *Namespace) Babel(args ...any) (resource.Resource, error)
// ---------------------------------------------------------------------------
