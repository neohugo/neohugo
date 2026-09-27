//! Port of `tpl/collections/collections.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: Collection funcs keep the input slice type (`page.Pages` stays `page.Pages`); `sort` is `sort.Stable` with `compare.LtCollate` (site-language x/text collator, strings that parse as floats compare numerically) — use go-sort's Stable for non-strict comparators; `where` walks dotted paths via methods then Params (case-insensitive); `index` on a string returns a `uint8`; `slice` = common/collections.Slice; `dict` -> `map[string]interface {}`; `seq` -> `[]int`.

/// Go: `collections.Namespace` (template value `*collections.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/collections:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/collections:After
    pub fn after(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Append
    pub fn append(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Apply
    pub fn apply(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Complement
    pub fn complement(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Delimit
    pub fn delimit(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Dictionary
    pub fn dictionary(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:First
    pub fn first(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Group
    pub fn group(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:In
    pub fn in_(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Index
    pub fn index(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Intersect
    pub fn intersect(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:IsSet
    pub fn is_set(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:KeyVals
    pub fn key_vals(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Last
    pub fn last(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Merge
    pub fn merge(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:NewScratch
    pub fn new_scratch(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Querify
    pub fn querify(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Reverse
    pub fn reverse(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Seq
    pub fn seq(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Shuffle
    pub fn shuffle(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Slice
    pub fn slice(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Sort
    pub fn sort(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:SymDiff
    pub fn sym_diff(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Union
    pub fn union(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Uniq
    pub fn uniq(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/collections:Where
    pub fn where_(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "After" => |n, ctx, a| n.after(ctx, a),
    "Append" => |n, ctx, a| n.append(ctx, a),
    "Apply" => |n, ctx, a| n.apply(ctx, a),
    "Complement" => |n, ctx, a| n.complement(ctx, a),
    "Delimit" => |n, ctx, a| n.delimit(ctx, a),
    "Dictionary" => |n, ctx, a| n.dictionary(ctx, a),
    "First" => |n, ctx, a| n.first(ctx, a),
    "Group" => |n, ctx, a| n.group(ctx, a),
    "In" => |n, ctx, a| n.in_(ctx, a),
    "Index" => |n, ctx, a| n.index(ctx, a),
    "Intersect" => |n, ctx, a| n.intersect(ctx, a),
    "IsSet" => |n, ctx, a| n.is_set(ctx, a),
    "KeyVals" => |n, ctx, a| n.key_vals(ctx, a),
    "Last" => |n, ctx, a| n.last(ctx, a),
    "Merge" => |n, ctx, a| n.merge(ctx, a),
    "NewScratch" => |n, ctx, a| n.new_scratch(ctx, a),
    "Querify" => |n, ctx, a| n.querify(ctx, a),
    "Reverse" => |n, ctx, a| n.reverse(ctx, a),
    "Seq" => |n, ctx, a| n.seq(ctx, a),
    "Shuffle" => |n, ctx, a| n.shuffle(ctx, a),
    "Slice" => |n, ctx, a| n.slice(ctx, a),
    "Sort" => |n, ctx, a| n.sort(ctx, a),
    "SymDiff" => |n, ctx, a| n.sym_diff(ctx, a),
    "Union" => |n, ctx, a| n.union(ctx, a),
    "Uniq" => |n, ctx, a| n.uniq(ctx, a),
    "Where" => |n, ctx, a| n.where_(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*collections.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/collections.go (687 lines; 7/20 funcs executed)
//   types: Namespace, intersector
// EX L37-49: New(deps *deps.Deps) *Namespace
//    L59-91: (ns *Namespace) After(n any, l any) (any, error)
//    L95-149: (ns *Namespace) Delimit(ctx context.Context, l, sep any, last ...any) (string, error)
// EX L154-188: (ns *Namespace) Dictionary(values ...any) (map[string]any, error)
// EX L191-223: (ns *Namespace) First(limit any, l any) (any, error)
//    L226-261: (ns *Namespace) In(l any, v any) (bool, error)
//    L265-302: (ns *Namespace) Intersect(l1, l2 any) (any, error)
//    L306-322: (ns *Namespace) Group(key any, items any) (any, error)
// EX L326-348: (ns *Namespace) IsSet(c any, key any) (bool, error)
//    L351-383: (ns *Namespace) Last(limit any, l any) (any, error)
//    L386-406: (ns *Namespace) Reverse(l any) (any, error)
// EX L417-482: (ns *Namespace) Seq(args ...any) ([]int, error)
//    L485-512: (ns *Namespace) Shuffle(l any) (any, error)
// EX L515-521: (ns *Namespace) Slice(args ...any) any
//    L528-534: (i *intersector) appendIfNotSeen(v reflect.Value)
//    L536-556: (i *intersector) handleValuePair(l1vv, l2vv reflect.Value)
//    L562-641: (ns *Namespace) Union(l1, l2 any) (any, error)
//    L644-676: (ns *Namespace) Uniq(l any) (any, error)
//    L679-681: (ns *Namespace) KeyVals(key any, values ...any) (types.KeyValues, error)
// EX L685-687: (ns *Namespace) NewScratch() *maps.Scratch
// ---------------------------------------------------------------------------
