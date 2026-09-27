//! Port of `tpl/strings/strings.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: Go `strings` semantics with go-unicode (simple case mapping); `substr`/`truncate` rune based.

/// Go: `strings.Namespace` (template value `*strings.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/strings:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/strings:Chomp
    pub fn chomp(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Contains
    pub fn contains(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:ContainsAny
    pub fn contains_any(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:ContainsNonSpace
    pub fn contains_non_space(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Count
    pub fn count(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:CountRunes
    pub fn count_runes(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:CountWords
    pub fn count_words(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Diff
    pub fn diff(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:FindRE
    pub fn find_re(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:FindRESubmatch
    pub fn find_re_submatch(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:FirstUpper
    pub fn first_upper(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:HasPrefix
    pub fn has_prefix(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:HasSuffix
    pub fn has_suffix(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Repeat
    pub fn repeat(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Replace
    pub fn replace(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:ReplaceRE
    pub fn replace_re(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:RuneCount
    pub fn rune_count(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:SliceString
    pub fn slice_string(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Split
    pub fn split(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Substr
    pub fn substr(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Title
    pub fn title(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:ToLower
    pub fn to_lower(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:ToUpper
    pub fn to_upper(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Trim
    pub fn trim(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:TrimLeft
    pub fn trim_left(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:TrimPrefix
    pub fn trim_prefix(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:TrimRight
    pub fn trim_right(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:TrimSpace
    pub fn trim_space(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:TrimSuffix
    pub fn trim_suffix(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/strings:Truncate
    pub fn truncate(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Chomp" => |n, ctx, a| n.chomp(ctx, a),
    "Contains" => |n, ctx, a| n.contains(ctx, a),
    "ContainsAny" => |n, ctx, a| n.contains_any(ctx, a),
    "ContainsNonSpace" => |n, ctx, a| n.contains_non_space(ctx, a),
    "Count" => |n, ctx, a| n.count(ctx, a),
    "CountRunes" => |n, ctx, a| n.count_runes(ctx, a),
    "CountWords" => |n, ctx, a| n.count_words(ctx, a),
    "Diff" => |n, ctx, a| n.diff(ctx, a),
    "FindRE" => |n, ctx, a| n.find_re(ctx, a),
    "FindRESubmatch" => |n, ctx, a| n.find_re_submatch(ctx, a),
    "FirstUpper" => |n, ctx, a| n.first_upper(ctx, a),
    "HasPrefix" => |n, ctx, a| n.has_prefix(ctx, a),
    "HasSuffix" => |n, ctx, a| n.has_suffix(ctx, a),
    "Repeat" => |n, ctx, a| n.repeat(ctx, a),
    "Replace" => |n, ctx, a| n.replace(ctx, a),
    "ReplaceRE" => |n, ctx, a| n.replace_re(ctx, a),
    "RuneCount" => |n, ctx, a| n.rune_count(ctx, a),
    "SliceString" => |n, ctx, a| n.slice_string(ctx, a),
    "Split" => |n, ctx, a| n.split(ctx, a),
    "Substr" => |n, ctx, a| n.substr(ctx, a),
    "Title" => |n, ctx, a| n.title(ctx, a),
    "ToLower" => |n, ctx, a| n.to_lower(ctx, a),
    "ToUpper" => |n, ctx, a| n.to_upper(ctx, a),
    "Trim" => |n, ctx, a| n.trim(ctx, a),
    "TrimLeft" => |n, ctx, a| n.trim_left(ctx, a),
    "TrimPrefix" => |n, ctx, a| n.trim_prefix(ctx, a),
    "TrimRight" => |n, ctx, a| n.trim_right(ctx, a),
    "TrimSpace" => |n, ctx, a| n.trim_space(ctx, a),
    "TrimSuffix" => |n, ctx, a| n.trim_suffix(ctx, a),
    "Truncate" => |n, ctx, a| n.truncate(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*strings.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/strings/strings.go (546 lines; 5/27 funcs executed)
//   types: Namespace
// EX L36-38: New(d *deps.Deps) *Namespace
//    L48-62: (ns *Namespace) CountRunes(s any) (int, error)
//    L65-71: (ns *Namespace) RuneCount(s any) (int, error)
//    L74-100: (ns *Namespace) CountWords(s any) (int, error)
//    L104-114: (ns *Namespace) Count(substr, s any) (int, error)
//    L117-130: (ns *Namespace) Chomp(s any) (any, error)
//    L133-145: (ns *Namespace) Contains(s, substr any) (bool, error)
//    L148-160: (ns *Namespace) ContainsAny(s, chars any) (bool, error)
//    L165-177: (ns *Namespace) ContainsNonSpace(s any) (bool, error)
//    L181-191: (ns *Namespace) Diff(oldname string, old any, newname string, new any) (string, error)
// EX L194-206: (ns *Namespace) HasPrefix(s, prefix any) (bool, error)
//    L209-221: (ns *Namespace) HasSuffix(s, suffix any) (bool, error)
//    L226-252: (ns *Namespace) Replace(s, old, new any, limit ...any) (string, error)
//    L257-299: (ns *Namespace) SliceString(a any, startEnd ...any) (string, error)
//    L302-309: (ns *Namespace) Split(a any, delimiter string) ([]string, error)
//    L323-394: (ns *Namespace) Substr(a any, nums ...any) (string, error)
//    L398-404: (ns *Namespace) Title(s any) (string, error)
//    L407-414: (ns *Namespace) FirstUpper(s any) (string, error)
// EX L418-425: (ns *Namespace) ToLower(s any) (string, error)
//    L429-436: (ns *Namespace) ToUpper(s any) (string, error)
// EX L440-452: (ns *Namespace) Trim(s, cutset any) (string, error)
//    L456-463: (ns *Namespace) TrimSpace(s any) (string, error)
//    L467-479: (ns *Namespace) TrimLeft(cutset, s any) (string, error)
// EX L483-495: (ns *Namespace) TrimPrefix(prefix, s any) (string, error)
//    L499-511: (ns *Namespace) TrimRight(cutset, s any) (string, error)
//    L515-527: (ns *Namespace) TrimSuffix(suffix, s any) (string, error)
//    L530-546: (ns *Namespace) Repeat(n, s any) (string, error)
// ---------------------------------------------------------------------------
