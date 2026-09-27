//! Port of `tpl/time/time.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: `dateFormat` = htime.ToTimeInDefaultLocationE (RFC3339 round trip drops sub-seconds) + TimeFormatter (locale month/day substitution); `now` = htime.Now() (--clock).

/// Go: `time.Namespace` (template value `*time.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/time:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/time:AsTime
    pub fn as_time(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/time:Duration
    pub fn duration(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/time:Format
    pub fn format(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/time:In
    pub fn in_(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/time:Now
    pub fn now(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/time:ParseDuration
    pub fn parse_duration(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "AsTime" => |n, ctx, a| n.as_time(ctx, a),
    "Duration" => |n, ctx, a| n.duration(ctx, a),
    "Format" => |n, ctx, a| n.format(ctx, a),
    "In" => |n, ctx, a| n.in_(ctx, a),
    "Now" => |n, ctx, a| n.now(ctx, a),
    "ParseDuration" => |n, ctx, a| n.parse_duration(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*time.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/time/time.go (150 lines; 3/7 funcs executed)
//   types: Namespace
// EX L29-44: New(timeFormatter htime.TimeFormatter, location *time.Location, deps *deps.Deps) *Namespace
//    L56-70: (ns *Namespace) AsTime(v any, args ...any) (any, error)
// EX L74-81: (ns *Namespace) Format(layout string, v any) (string, error)
// EX L84-86: (ns *Namespace) Now() time.Time
//    L92-101: (ns *Namespace) In(timeZoneName string, t time.Time) (time.Time, error)
//    L109-116: (ns *Namespace) ParseDuration(s any) (time.Duration, error)
//    L136-150: (ns *Namespace) Duration(unit any, number any) (time.Duration, error)
// ---------------------------------------------------------------------------
