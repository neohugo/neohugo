//! Port of `tpl/urls/urls.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: `urlize` = PathSpec.URLize (uppercase %XX); `relLangURL`/`absLangURL` = PathSpec.RelURL/AbsURL(addLanguage) with the canonifyURLs branch; `urls.Parse` returns a `*url.URL` object (String, IsAbs, Path, RawQuery, Fragment) over go-url; `ref` -> page.Ref(argsm).

/// Go: `urls.Namespace` (template value `*urls.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/urls:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/urls:AbsLangURL
    pub fn abs_lang_url(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:AbsURL
    pub fn abs_url(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:Anchorize
    pub fn anchorize(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:JoinPath
    pub fn join_path(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:Parse
    pub fn parse(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:Ref
    pub fn ref_(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:RelLangURL
    pub fn rel_lang_url(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:RelRef
    pub fn rel_ref(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:RelURL
    pub fn rel_url(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:URLDecode
    pub fn url_decode(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:URLEncode
    pub fn url_encode(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/urls:URLize
    pub fn ur_lize(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "AbsLangURL" => |n, ctx, a| n.abs_lang_url(ctx, a),
    "AbsURL" => |n, ctx, a| n.abs_url(ctx, a),
    "Anchorize" => |n, ctx, a| n.anchorize(ctx, a),
    "JoinPath" => |n, ctx, a| n.join_path(ctx, a),
    "Parse" => |n, ctx, a| n.parse(ctx, a),
    "Ref" => |n, ctx, a| n.ref_(ctx, a),
    "RelLangURL" => |n, ctx, a| n.rel_lang_url(ctx, a),
    "RelRef" => |n, ctx, a| n.rel_ref(ctx, a),
    "RelURL" => |n, ctx, a| n.rel_url(ctx, a),
    "URLDecode" => |n, ctx, a| n.url_decode(ctx, a),
    "URLEncode" => |n, ctx, a| n.url_encode(ctx, a),
    "URLize" => |n, ctx, a| n.ur_lize(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*urls.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/urls/urls.go (254 lines; 7/14 funcs executed)
//   types: Namespace
// EX L29-34: New(deps *deps.Deps) *Namespace
//    L43-50: (ns *Namespace) AbsURL(s any) (string, error)
// EX L54-61: (ns *Namespace) Parse(rawurl any) (*url.URL, error)
//    L65-72: (ns *Namespace) RelURL(s any) (string, error)
// EX L75-81: (ns *Namespace) URLize(s any) (string, error)
//    L85-91: (ns *Namespace) Anchorize(s any) (string, error)
// EX L94-105: (ns *Namespace) Ref(p any, args any) (string, error)
//    L108-120: (ns *Namespace) RelRef(p any, args any) (string, error)
// EX L122-164: (ns *Namespace) refArgsToMap(args any) (map[string]any, error)
// EX L168-175: (ns *Namespace) RelLangURL(s any) (string, error)
// EX L180-187: (ns *Namespace) AbsLangURL(s any) (string, error)
//    L192-224: (ns *Namespace) JoinPath(elements ...any) (string, error)
//    L228-235: (ns *Namespace) URLEncode(rawurl interface{}) (template.HTML, error)
//    L240-254: (ns *Namespace) URLDecode(rawurl interface{}) (string, error)
// ---------------------------------------------------------------------------
