//! Port of `tpl/templates/templates.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{HostCtx, Object, SliceType, Value};
use nh_common::object::{GoResult, NamedMethods, args};
use nh_deps::deps::Deps;
use nh_tpl::template::{
    CurrentTemplateBase, CurrentTemplateInfo, TplContext, reverse_current_template_infos,
};

/// Go: `templates.Namespace` (template value `*templates.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

/// The Go type of `templates.Current`'s result.
pub const CURRENT_TEMPLATE_INFO_TYPE: &str = "*tpl.CurrentTemplateInfo";
/// Go: `tpl.CurrentTemplateInfos` (`Ancestors()`).
pub const CURRENT_TEMPLATE_INFOS_TYPE: &str = "tpl.CurrentTemplateInfos";

/// Go: `*tpl.CurrentTemplateInfo` as a template value: the fields `Parent`, `Level` and the
/// embedded `CurrentTemplateInfoOps` (a `*tplimpl.TemplInfo`: `Name`, `Filename`, `Base`), and
/// `Ancestors`.
#[derive(Clone)]
pub struct CurrentTemplateInfoObject(pub Arc<CurrentTemplateInfo>);

impl CurrentTemplateInfoObject {
    /// `*tpl.CurrentTemplateInfo` (a nil pointer for `None`).
    pub fn value(ti: Option<Arc<CurrentTemplateInfo>>) -> Value {
        match ti {
            Some(ti) => Value::object(CurrentTemplateInfoObject(ti)),
            None => Value::TypedNil(Arc::from(CURRENT_TEMPLATE_INFO_TYPE)),
        }
    }
}

/// `tpl.CurrentTemplateInfos` as a template value.
fn infos_value(infos: Vec<Arc<CurrentTemplateInfo>>) -> Value {
    // Go returns a nil slice when there are no ancestors; an empty list prints, ranges, `len`s
    // and tests like it (the value model's typed nil of an unregistered named type would be a
    // nil pointer to `len`).
    Value::list(
        SliceType::Named(Arc::from(CURRENT_TEMPLATE_INFOS_TYPE)),
        infos
            .into_iter()
            .map(|ti| Value::object(CurrentTemplateInfoObject(ti)))
            .collect(),
    )
}

nh_common::go_methods!(CurrentTemplateInfoObject {
    // Go: tpl/template.go:(*CurrentTemplateInfo).Ancestors
    "Ancestors" => |c, _ctx, a| { args::exactly(a, 0, "Ancestors")?; Ok(infos_value(c.0.ancestors())) },
    "Base" => |c, _ctx, a| {
        args::exactly(a, 0, "Base")?;
        Ok(match &c.0.base {
            Some(b) => Value::object(TemplInfoBaseObject(b.clone())),
            None => Value::TypedNil(Arc::from("*tplimpl.TemplInfo")),
        })
    },
    "Filename" => |c, _ctx, a| { args::exactly(a, 0, "Filename")?; Ok(Value::string(c.0.filename.as_str())) },
    "Name" => |c, _ctx, a| { args::exactly(a, 0, "Name")?; Ok(Value::string(c.0.name.as_str())) },
});

impl Object for CurrentTemplateInfoObject {
    nh_common::object_basics!(CURRENT_TEMPLATE_INFO_TYPE);
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Parent" => Some(CurrentTemplateInfoObject::value(self.0.parent.clone())),
            "Level" => Some(Value::int(self.0.level)),
            _ => None,
        }
    }
    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

/// The base template of a `CurrentTemplateInfo` (Go: the `*tplimpl.TemplInfo` returned by
/// `Base()`, seen through `tpl.CurrentTemplateInfoCommonOps`).
#[derive(Clone)]
pub struct TemplInfoBaseObject(pub CurrentTemplateBase);

nh_common::go_methods!(TemplInfoBaseObject {
    "Filename" => |c, _ctx, a| { args::exactly(a, 0, "Filename")?; Ok(Value::string(c.0.filename.as_str())) },
    "Name" => |c, _ctx, a| { args::exactly(a, 0, "Name")?; Ok(Value::string(c.0.name.as_str())) },
});

impl Object for TemplInfoBaseObject {
    nh_common::object_basics!("*tplimpl.TemplInfo");
}

/// Go: the methods of the named slice `tpl.CurrentTemplateInfos` (`Reverse`), for the
/// named-type registry (nh-hugolib `tplapi::named_types`).
pub const CURRENT_TEMPLATE_INFOS_METHODS: NamedMethods = NamedMethods {
    has_method: |name| name == "Reverse",
    call: |_ctx, recv, name, a| {
        if name != "Reverse" {
            return None;
        }
        Some(args::exactly(a, 0, "Reverse").map(|_| {
            // Go: tpl/template.go:(CurrentTemplateInfos).Reverse
            let infos: Vec<Arc<CurrentTemplateInfo>> = match recv {
                Value::List(l) => l
                    .items
                    .iter()
                    .filter_map(|v| {
                        v.downcast::<CurrentTemplateInfoObject>()
                            .map(|c| c.0.clone())
                    })
                    .collect(),
                _ => Vec::new(),
            };
            if infos.is_empty() {
                return recv.clone();
            }
            infos_value(reverse_current_template_infos(&infos))
        }))
    },
};

impl Namespace {
    /// New returns a new instance of the templates-namespaced template functions.
    // Go: tpl/templates/templates.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    /// Get information about the currently executing template.
    // Go: tpl/templates/templates.go:Current
    pub fn current(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "Current")?;
        let ti = TplContext::from_host(ctx).and_then(|c| c.current_template.clone());
        Ok(CurrentTemplateInfoObject::value(ti))
    }

    /// Defer defers the execution of a template block.
    ///
    /// Go first calls `ns.deps.Site.CheckReady()`, which panics when defer is used in a content
    /// adapter (never true in a build that renders templates).
    // Go: tpl/templates/templates.go:Defer
    pub fn defer(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if !a.is_empty() {
            return Err(go_value::Error::new("Defer does not take any arguments"));
        }
        Ok(Value::Bool(true))
    }

    /// DoDefer defers the execution of a template block. For internal use only.
    ///
    /// STUB: the deferred executions (`BuildState.DeferredExecutions`) and their execution after
    /// the render are not ported (seeksnack does not use `templates.Defer`): an explicit error
    /// instead of output without the deferred block.
    // Go: tpl/templates/templates.go:DoDefer
    pub fn do_defer(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "DoDefer")?;
        args::string(a, 0)?;
        Err(go_value::Error::new(
            "neohugo-rs: templates.Defer is not supported",
        ))
    }

    /// Exists returns whether the template with the given name exists. Note that this is the
    /// Unix-styled relative path including filename suffix, e.g. partials/header.html
    // Go: tpl/templates/templates.go:Exists
    pub fn exists(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Exists")?;
        let name = args::string(a, 0)?;
        Ok(Value::Bool(
            self.d
                .get_template_store()
                .has_template(&name.to_str_lossy()),
        ))
    }
}

nh_common::go_methods!(Namespace {
    "Current" => |n, ctx, a| n.current(ctx, a),
    "Defer" => |n, ctx, a| n.defer(ctx, a),
    "DoDefer" => |n, ctx, a| n.do_defer(ctx, a),
    "Exists" => |n, ctx, a| n.exists(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*templates.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/templates/templates.go (109 lines; 1/5 funcs executed)
//   types: Namespace, DeferOpts
// OK L30-36: New(deps *deps.Deps) *Namespace
// OK L46-48: (ns *Namespace) Exists(name string) bool
// OK L51-60: (ns *Namespace) Defer(args ...any) (bool, error)
// STUB L75-104: (ns *Namespace) DoDefer(ctx context.Context, id string, optsv any) string (explicit unsupported error)
// OK L107-109: (ns *Namespace) Current(ctx context.Context) *tpl.CurrentTemplateInfo
// ---------------------------------------------------------------------------
