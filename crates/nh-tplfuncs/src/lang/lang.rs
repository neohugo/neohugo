//! Port of `tpl/lang/lang.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: `i18n`/`T` -> deps.translate(ctx, id, data) (go-i18n port in nh-i18n).

/// Go: `lang.Namespace` (template value `*lang.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/lang:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/lang:FormatAccounting
    pub fn format_accounting(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/lang:FormatCurrency
    pub fn format_currency(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/lang:FormatNumber
    pub fn format_number(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/lang:FormatNumberCustom
    pub fn format_number_custom(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/lang:FormatPercent
    pub fn format_percent(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/lang:Merge
    pub fn merge(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/lang:Translate
    pub fn translate(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "FormatAccounting" => |n, ctx, a| n.format_accounting(ctx, a),
    "FormatCurrency" => |n, ctx, a| n.format_currency(ctx, a),
    "FormatNumber" => |n, ctx, a| n.format_number(ctx, a),
    "FormatNumberCustom" => |n, ctx, a| n.format_number_custom(ctx, a),
    "FormatPercent" => |n, ctx, a| n.format_percent(ctx, a),
    "Merge" => |n, ctx, a| n.merge(ctx, a),
    "Translate" => |n, ctx, a| n.translate(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*lang.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/lang/lang.go (259 lines; 2/9 funcs executed)
//   types: Namespace, pagesLanguageMerger
// EX L34-39: New(deps *deps.Deps, translator locales.Translator) *Namespace
// EX L48-64: (ns *Namespace) Translate(ctx context.Context, id any, args ...any) (string, error)
//    L67-73: (ns *Namespace) FormatNumber(precision, number any) (string, error)
//    L77-83: (ns *Namespace) FormatPercent(precision, number any) (string, error)
//    L89-99: (ns *Namespace) FormatCurrency(precision, currency, number any) (string, error)
//    L105-115: (ns *Namespace) FormatAccounting(precision, currency, number any) (string, error)
//    L117-133: (ns *Namespace) castPrecisionNumber(precision, number any) (uint64, float64, error)
//    L144-240: (ns *Namespace) FormatNumberCustom(precision, number any, options ...any) (string, error)
//    L247-259: (ns *Namespace) Merge(p2, p1 any) (any, error)
// ---------------------------------------------------------------------------
