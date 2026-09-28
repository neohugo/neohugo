//! Port of `tpl/collections/collections.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::any::Any;
use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use go_value::{GoString, HostCtx, Kind, List, Map, MapType, Object, SliceType, Value};
use nh_common::cast::caste;
use nh_common::hreflect::{self, ReflectKind};
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;
use nh_tplimpl::engine::FuncMap;

use super::reflect_helpers::{
    self as rh, KeySet, as_slice, convert_number, elem_is_interface, indirect, indirect_interface,
    is_number, kind, normalize, normalize_interface, number_to_float, static_elem_kind, to_string,
    type_comparable, value_comparable,
};

// Parity notes: Collection funcs keep the input slice type (`page.Pages` stays `page.Pages`); `sort` is `sort.Stable` with `compare.LtCollate` (site-language x/text collator, strings that parse as floats compare numerically) — use go-sort's Stable for non-strict comparators; `where` walks dotted paths via methods then Params (case-insensitive); `index` on a string returns a `uint8`; `slice` = common/collections.Slice; `dict` -> `map[string]interface {}`; `seq` -> `[]int`.

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

/// Go: `collections.Namespace` (template value `*collections.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    /// Go `loc` (`langs.GetLocation(language)`).
    pub loc: Arc<go_value::Location>,
    /// Go `sortComp` (`compare.New(loc, true)`).
    pub sort_comp: crate::compare::compare::Namespace,
    /// The func map `apply` looks functions up in when the site's template store is not set
    /// (component tests; a build always has the store, Go `deps.GetTemplateStore().GetFunc`).
    pub funcs: OnceLock<Arc<FuncMap>>,
}

impl Namespace {
    // Go: tpl/collections:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let language = d.conf.language();
        let loc = language.location();
        Namespace {
            sort_comp: crate::compare::compare::Namespace::with_location(
                d.clone(),
                loc.clone(),
                true,
            ),
            loc,
            d,
            funcs: OnceLock::new(),
        }
    }

    // Go: tpl/collections:After
    pub fn after(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "After")?;
        self.do_after(&a[0], &a[1])
    }

    // Go: tpl/collections/collections.go:After
    /// After returns all the items after the first n items in list l.
    pub fn do_after(&self, n: &Value, l: &Value) -> GoResult<Value> {
        if n.is_invalid() || l.is_invalid() {
            return Err(err("both limit and seq must be provided"));
        }

        let nv = caste::to_int_e(n)?;

        if nv < 0 {
            return Err(err(format!("sequence bounds out of range [{nv}:]")));
        }

        let (lv, is_nil) = indirect(l);
        if is_nil {
            return Err(err("can't iterate over a nil value"));
        }

        slice_or_string(&lv, l, |len| {
            let nv = nv as usize;
            if nv >= len { (0, 0) } else { (nv, len) }
        })
    }

    // Go: tpl/collections:Append
    pub fn append(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/collections/append.go:Append
        if a.len() < 2 {
            return Err(err("need at least 2 arguments to append"));
        }

        let to = &a[a.len() - 1];
        let from = &a[..a.len() - 1];

        Ok(nh_common::collections::append::append(ctx, to, from)?)
    }

    // Go: tpl/collections:Apply
    pub fn apply(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "Apply")?;
        let fname = args::string(a, 1)?;
        self.do_apply(ctx, &a[0], &fname, &a[2..])
    }

    // Go: tpl/collections:Complement
    pub fn complement(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        self.do_complement(a)
    }

    // Go: tpl/collections:Delimit
    pub fn delimit(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "Delimit")?;
        Ok(Value::String(self.do_delimit(
            ctx,
            &a[0],
            &a[1],
            &a[2..],
        )?))
    }

    // Go: tpl/collections/collections.go:Delimit
    /// Delimit takes a given list l and returns a string delimited by sep. If last is passed to
    /// the function, it will be used as the final delimiter.
    pub fn do_delimit(
        &self,
        ctx: HostCtx<'_>,
        l: &Value,
        sep: &Value,
        last: &[Value],
    ) -> GoResult<GoString> {
        let d = caste::to_string_e(sep)?;

        let mut d_last: Option<GoString> = None;
        if !last.is_empty() {
            d_last = caste::to_string_e(&last[0]).ok();
        }

        let (lv, is_nil) = indirect(l);
        if is_nil {
            return Err(err("can't iterate over a nil value"));
        }

        let items: Vec<Value> = match kind(&lv) {
            ReflectKind::Map => {
                let sort_seq = self.do_sort(ctx, l, &[])?;
                as_slice(&sort_seq).map(|s| s.items).unwrap_or_default()
            }
            ReflectKind::Slice => as_slice(&lv).map(|s| s.items).unwrap_or_default(),
            ReflectKind::String => rh::string_of(&lv)
                .iter()
                .map(|b| Value::Uint(*b as u64, go_value::UintKind::Uint8))
                .collect(),
            _ => {
                return Err(err(format!("can't iterate over {}", l.go_type_name())));
            }
        };

        let mut s: Vec<u8> = Vec::new();
        let n = items.len();
        for (i, val) in items.iter().enumerate() {
            let Ok(val_str) = caste::to_string_e(val) else {
                continue;
            };
            if let Some(dl) = d_last.as_ref().filter(|_| n >= 2 && i == n - 2) {
                s.extend_from_slice(&val_str);
                s.extend_from_slice(dl);
            } else if i == n - 1 {
                s.extend_from_slice(&val_str);
            } else {
                s.extend_from_slice(&val_str);
                s.extend_from_slice(&d);
            }
        }

        Ok(GoString::from(s))
    }

    // Go: tpl/collections:Dictionary
    pub fn dictionary(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        self.do_dictionary(a)
    }

    // Go: tpl/collections/collections.go:Dictionary
    /// Dictionary creates a new map from the given parameters by treating values as key-value
    /// pairs. The number of values must be even. The keys can be string slices, which will
    /// create the needed nested structure.
    pub fn do_dictionary(&self, values: &[Value]) -> GoResult<Value> {
        if !values.len().is_multiple_of(2) {
            return Err(err("invalid dictionary call"));
        }

        let mut root = Map::new(MapType::StringAny);

        let mut i = 0;
        while i < values.len() {
            match &values[i] {
                Value::String(key) => {
                    root.insert(key.clone(), values[i + 1].clone());
                }
                Value::List(l) if l.ty == SliceType::String => {
                    let keys: Vec<GoString> = l.items.iter().map(rh::string_of).collect();
                    if keys.is_empty() {
                        return Err(err("runtime error: index out of range [-1]"));
                    }
                    dict_set_nested(&mut root, &keys, values[i + 1].clone())?;
                }
                Value::TypedNil(t) if &**t == "[]string" => {
                    return Err(err("runtime error: index out of range [-1]"));
                }
                _ => return Err(err("invalid dictionary key")),
            }
            i += 2;
        }

        Ok(Value::map(root))
    }

    // Go: tpl/collections:First
    pub fn first(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "First")?;
        self.do_first(&a[0], &a[1])
    }

    // Go: tpl/collections/collections.go:First
    /// First returns the first limit items in list l.
    pub fn do_first(&self, limit: &Value, l: &Value) -> GoResult<Value> {
        if limit.is_invalid() || l.is_invalid() {
            return Err(err("both limit and seq must be provided"));
        }

        let limitv = caste::to_int_e(limit)?;

        if limitv < 0 {
            return Err(err("sequence length must be non-negative"));
        }

        let (lv, is_nil) = indirect(l);
        if is_nil {
            return Err(err("can't iterate over a nil value"));
        }

        slice_or_string(&lv, l, |len| (0, (limitv as usize).min(len)))
    }

    // Go: tpl/collections:Group
    pub fn group(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Group")?;
        self.do_group(&a[0], &a[1])
    }

    // Go: tpl/collections/collections.go:Group
    /// Group groups a set of items by the given key. This is currently only supported for Pages.
    pub fn do_group(&self, key: &Value, items: &Value) -> GoResult<Value> {
        if key.is_invalid() {
            return Err(err("nil is not a valid key to group by"));
        }

        // collections.Grouper: an object with a `Group` method (a page: `pageState.Group`).
        if let Value::Object(o) = items
            && o.has_method("Group")
        {
            return o
                .call_method(&(), "Group", &[key.clone(), items.clone()])
                .unwrap_or_else(|| Err(err("method Group not found")));
        }

        // collections.Grouper: page.Pages (Pages.Group).
        let is_pages = match items {
            Value::List(l) => l.ty == SliceType::Named(Arc::from(nh_page::page::PAGES_TYPE)),
            Value::TypedNil(t) => &**t == nh_page::page::PAGES_TYPE,
            _ => false,
        };
        if is_pages {
            let g = nh_page::pages::group(key, items)?;
            return Ok(g.to_value());
        }

        // Go: `in := newSliceElement(items)`; a `*T` is never a Grouper for the element types
        // the value model has.
        let in_type = rh::new_slice_element_type(items).unwrap_or_else(|| "<nil>".to_string());
        Err(err(format!(
            "grouping not supported for type {} {}",
            items.go_type_name(),
            in_type
        )))
    }

    // Go: tpl/collections:In
    pub fn in_(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "In")?;
        Ok(Value::Bool(self.do_in(&a[0], &a[1])?))
    }

    // Go: tpl/collections/collections.go:In
    /// In returns whether v is in the list l. l may be an array or slice.
    pub fn do_in(&self, l: &Value, v: &Value) -> GoResult<bool> {
        if l.is_invalid() || v.is_invalid() {
            return Ok(false);
        }

        let vvk = normalize(v);

        if let Some(sl) = as_slice(l) {
            for item in &sl.items {
                let (lvv, is_nil) = indirect_interface(item);
                if is_nil {
                    continue;
                }

                let lvvk = normalize(&lvv);

                if lvvk == vvk {
                    return Ok(true);
                }
            }
        }
        let Ok(ss) = caste::to_string_e(l) else {
            return Ok(false);
        };

        let Ok(su) = caste::to_string_e(v) else {
            return Ok(false);
        };
        Ok(go_unicode::strings::contains(&ss, &su))
    }

    // Go: tpl/collections:Index
    pub fn index(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Index")?;
        self.do_index_outer(&a[0], &a[1..])
    }

    // Go: tpl/collections:Intersect
    pub fn intersect(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Intersect")?;
        self.do_intersect(&a[0], &a[1])
    }

    // Go: tpl/collections/collections.go:Intersect
    /// Intersect returns the common elements in the given sets, l1 and l2. l1 and l2 must be of
    /// the same type and may be either arrays or slices.
    pub fn do_intersect(&self, l1: &Value, l2: &Value) -> GoResult<Value> {
        if l1.is_invalid() || l2.is_invalid() {
            return Ok(Value::any_list(Vec::new()));
        }

        let Some(l1v) = as_slice(l1) else {
            return Err(err(format!("can't iterate over {}", l1.go_type_name())));
        };
        let Some(l2v) = as_slice(l2) else {
            return Err(err(format!("can't iterate over {}", l2.go_type_name())));
        };
        let mut ins = Intersector::new(l1v.ty.clone());
        let e1 = l1v.elem_type();
        let e2 = l2v.elem_type();
        for l1vv in &l1v.items {
            if !elem_comparable(&e1) {
                return Err(err(
                    "intersect does not support slices or arrays of uncomparable types",
                ));
            }

            for l2vv in &l2v.items {
                if !elem_comparable(&e2) {
                    return Err(err(
                        "intersect does not support slices or arrays of uncomparable types",
                    ));
                }

                ins.handle_value_pair(l1vv, static_elem_kind(&e1, l1vv), l2vv)?;
            }
        }
        Ok(ins.value())
    }

    // Go: tpl/collections:IsSet
    pub fn is_set(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "IsSet")?;
        Ok(Value::Bool(self.do_is_set(&a[0], &a[1])?))
    }

    // Go: tpl/collections/collections.go:IsSet
    /// IsSet returns whether a given array, channel, slice, or map in c has the given key
    /// defined.
    pub fn do_is_set(&self, c: &Value, key: &Value) -> GoResult<bool> {
        match kind(c) {
            ReflectKind::Slice | ReflectKind::Chan => {
                let Ok(k) = caste::to_int_e(key) else {
                    return Err(err(format!(
                        "isset unable to use key of type {} as index",
                        key.go_type_name()
                    )));
                };
                if rh::len_of(c) as i64 > k {
                    return Ok(true);
                }
            }
            ReflectKind::Map => {
                if key.is_invalid() {
                    // Go: kv.Type() of the zero Value panics.
                    return Err(err("reflect: call of reflect.Value.Type on zero Value"));
                }
                // The map key type is string (Hugo normalises map keys).
                if matches!(key, Value::String(_)) {
                    let k = key.as_go_string().unwrap();
                    return Ok(match c {
                        Value::Map(m) => m.get(k).is_some(),
                        Value::Object(o) => o.map_get(k).is_some(),
                        _ => false,
                    });
                }
            }
            k => {
                self.d.log.warnf(format!(
                    "calling IsSet with unsupported type \"{}\" ({}) will always return false.\n",
                    rh::kind_name(k),
                    c.go_type_name()
                ));
            }
        }

        Ok(false)
    }

    // Go: tpl/collections:KeyVals
    pub fn key_vals(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "KeyVals")?;
        // Go: tpl/collections/collections.go:KeyVals
        Ok(Value::object(KeyValuesObject {
            key: a[0].clone(),
            values: a[1..].to_vec(),
        }))
    }

    // Go: tpl/collections:Last
    pub fn last(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Last")?;
        self.do_last(&a[0], &a[1])
    }

    // Go: tpl/collections/collections.go:Last
    /// Last returns the last limit items in the list l.
    pub fn do_last(&self, limit: &Value, l: &Value) -> GoResult<Value> {
        if limit.is_invalid() || l.is_invalid() {
            return Err(err("both limit and seq must be provided"));
        }

        let limitv = caste::to_int_e(limit)?;

        if limitv < 0 {
            return Err(err("sequence length must be non-negative"));
        }

        let (seqv, is_nil) = indirect(l);
        if is_nil {
            return Err(err("can't iterate over a nil value"));
        }

        slice_or_string(&seqv, l, |len| {
            let limitv = (limitv as usize).min(len);
            (len - limitv, len)
        })
    }

    // Go: tpl/collections:Merge
    pub fn merge(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        self.do_merge(a)
    }

    // Go: tpl/collections:NewScratch
    pub fn new_scratch(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "NewScratch")?;
        // Go: tpl/collections/collections.go:NewScratch
        Ok(Value::object(nh_common::maps::scratch::Scratch::new()))
    }

    // Go: tpl/collections:Querify
    pub fn querify(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        Ok(Value::String(self.do_querify(a)?))
    }

    // Go: tpl/collections:Reverse
    pub fn reverse(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Reverse")?;
        self.do_reverse(&a[0])
    }

    // Go: tpl/collections/collections.go:Reverse
    /// Reverse creates a copy of the list l and reverses it.
    pub fn do_reverse(&self, l: &Value) -> GoResult<Value> {
        if l.is_invalid() {
            return Ok(Value::Invalid);
        }
        let Some(v) = as_slice(l) else {
            return Err(err("argument must be a slice"));
        };
        let mut items = v.items.clone();
        items.reverse();
        Ok(v.with(items))
    }

    // Go: tpl/collections:Seq
    pub fn seq(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        let seq = self.do_seq(a)?;
        Ok(Value::list(
            SliceType::Int,
            seq.into_iter().map(Value::int).collect(),
        ))
    }

    // Go: tpl/collections/collections.go:Seq
    /// Seq creates a sequence of integers from args. It's named and used as GNU's seq.
    pub fn do_seq(&self, a: &[Value]) -> GoResult<Vec<i64>> {
        if a.is_empty() || a.len() > 3 {
            return Err(err("invalid number of arguments to Seq"));
        }

        let int_args = caste::to_int_slice(&Value::any_list(a.to_vec()));
        if int_args.is_empty() || int_args.len() > 3 {
            return Err(err("invalid arguments to Seq"));
        }

        let mut inc: i64 = 1;
        let last: i64;
        let mut first = int_args[0];

        if int_args.len() == 1 {
            last = first;
            if last == 0 {
                return Ok(Vec::new());
            } else if last > 0 {
                first = 1;
            } else {
                first = -1;
                inc = -1;
            }
        } else if int_args.len() == 2 {
            last = int_args[1];
            if last < first {
                inc = -1;
            }
        } else {
            inc = int_args[1];
            last = int_args[2];
            if inc == 0 {
                return Err(err("'increment' must not be 0"));
            }
            if first < last && inc < 0 {
                return Err(err("'increment' must be > 0"));
            }
            if first > last && inc > 0 {
                return Err(err("'increment' must be < 0"));
            }
        }

        // sanity check
        if last < -100000 {
            return Err(err("size of result exceeds limit"));
        }
        let size = last.wrapping_sub(first).wrapping_div(inc).wrapping_add(1);

        // sanity check
        if size <= 0 || size > 2000 {
            return Err(err("size of result exceeds limit"));
        }

        let size = size as usize;
        let mut seq = vec![0i64; size];
        let mut val = first;
        let mut i = 0usize;
        loop {
            if i >= size {
                // Go: a runtime panic (the sequence wrapped around).
                return Err(err(format!(
                    "runtime error: index out of range [{i}] with length {size}"
                )));
            }
            seq[i] = val;
            val = val.wrapping_add(inc);
            if (inc < 0 && val < last) || (inc > 0 && val > last) {
                break;
            }
            i += 1;
        }

        Ok(seq)
    }

    // Go: tpl/collections:Shuffle
    pub fn shuffle(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Shuffle")?;
        self.do_shuffle(&a[0])
    }

    // Go: tpl/collections/collections.go:Shuffle
    /// Shuffle returns list l in a randomized order (Go `rand.Perm`; the port's permutation
    /// comes from its own generator, see PORTING.md).
    pub fn do_shuffle(&self, l: &Value) -> GoResult<Value> {
        if l.is_invalid() {
            return Err(err("both count and seq must be provided"));
        }

        let (lv, is_nil) = indirect(l);
        if is_nil {
            return Err(err("can't iterate over a nil value"));
        }

        let items: Vec<Value> = match kind(&lv) {
            ReflectKind::Slice => as_slice(&lv).map(|s| s.items).unwrap_or_default(),
            ReflectKind::String => {
                // Go: reflect.MakeSlice(reflect.TypeOf(l), ..) panics for a string.
                return Err(err("reflect.MakeSlice of non-slice type"));
            }
            _ => {
                return Err(err(format!("can't iterate over {}", l.go_type_name())));
            }
        };

        let random_indices = rand_perm(items.len());
        let mut shuffled = vec![Value::Invalid; items.len()];
        for (index, value) in random_indices.into_iter().enumerate() {
            shuffled[value] = items[index].clone();
        }

        Ok(as_slice(&lv).unwrap().with(shuffled))
    }

    // Go: tpl/collections:Slice
    pub fn slice(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/collections/collections.go:Slice
        if a.is_empty() {
            return Ok(Value::any_list(Vec::new()));
        }

        Ok(nh_common::collections::slice::slice(ctx, a))
    }

    // Go: tpl/collections:Sort
    pub fn sort(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Sort")?;
        self.do_sort(ctx, &a[0], &a[1..])
    }

    // Go: tpl/collections:SymDiff
    pub fn sym_diff(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "SymDiff")?;
        self.do_sym_diff(&a[0], &a[1])
    }

    // Go: tpl/collections:Union
    pub fn union(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Union")?;
        self.do_union(&a[0], &a[1])
    }

    // Go: tpl/collections/collections.go:Union
    /// Union returns the union of the given sets, l1 and l2. l1 and l2 must be of the same type
    /// and may be either arrays or slices. If l1 and l2 aren't of the same type then l1 will be
    /// returned. If either l1 or l2 is nil then the non-nil list will be returned.
    pub fn do_union(&self, l1: &Value, l2: &Value) -> GoResult<Value> {
        if l1.is_invalid() && l2.is_invalid() {
            return Ok(Value::any_list(Vec::new()));
        } else if l1.is_invalid() {
            return Ok(l2.clone());
        } else if !l1.is_invalid() && l2.is_invalid() {
            return Ok(l1.clone());
        }

        let Some(l1v) = as_slice(l1) else {
            return Err(err(format!("can't iterate over {}", l1.go_type_name())));
        };
        let Some(l2v) = as_slice(l2) else {
            return Err(err(format!("can't iterate over {}", l2.go_type_name())));
        };

        let mut ins = Intersector::new(l1v.ty.clone());
        let e1 = l1v.elem_type();
        let e2 = l2v.elem_type();

        if l1v.ty.go_name() != l2v.ty.go_name()
            && !elem_is_interface(&e1)
            && !elem_is_interface(&e2)
        {
            return Ok(ins.value());
        }

        // The last l1 element as Go's l1vv: (value, kind after indirectInterface).
        let mut l1vv: Option<(Value, ReflectKind)> = None;

        for item in &l1v.items {
            let (vv, is_nil) = indirect_interface(item);
            // Kind after indirectInterface: a nil interface element and a non-empty interface
            // element (page.Page) stay of interface kind.
            let k = if is_nil || nonempty_interface(&e1) {
                ReflectKind::Interface
            } else {
                kind(&vv)
            };
            let comparable = if elem_is_interface(&e1) {
                is_nil || nonempty_interface(&e1) || value_comparable(&vv)
            } else {
                type_comparable(&e1)
            };
            if !comparable {
                return Err(err(
                    "union does not support slices or arrays of uncomparable types",
                ));
            }

            if !is_nil {
                if k == ReflectKind::Interface {
                    ins.append_if_not_seen_interface(vv.clone())?;
                } else {
                    ins.append_if_not_seen(vv.clone());
                }
            }
            l1vv = Some((vv, k));
        }

        if l1vv.is_none() {
            // The first slice may be empty. Pick the first value of the second
            // to use as a prototype.
            if let Some(first) = l2v.items.first() {
                l1vv = Some((first.clone(), static_elem_kind(&e2, first)));
            }
        }

        let proto_kind = l1vv.map(|(_, k)| k).unwrap_or(ReflectKind::Invalid);
        let l2_iface = elem_is_interface(&e2);
        for l2vv in &l2v.items {
            match proto_kind {
                ReflectKind::String => {
                    if let Ok(l2t) = to_string(l2vv) {
                        ins.append_checked(Value::String(l2t), "string", &e1, false)?;
                    }
                }
                k if is_number(k) => {
                    if let Ok(v) = convert_number(l2vv, k) {
                        let t = v.go_type_name().into_owned();
                        ins.append_checked(v, &t, &e1, false)?;
                    }
                }
                ReflectKind::Interface | ReflectKind::Struct | ReflectKind::Ptr => {
                    ins.append_checked(l2vv.clone(), &e2, &e1, l2_iface)?;
                }
                _ => {}
            }
        }

        Ok(ins.value())
    }

    // Go: tpl/collections:Uniq
    pub fn uniq(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Uniq")?;
        self.do_uniq(&a[0])
    }

    // Go: tpl/collections/collections.go:Uniq
    /// Uniq returns a new list with duplicate elements in the list l removed.
    pub fn do_uniq(&self, l: &Value) -> GoResult<Value> {
        if l.is_invalid() {
            return Ok(Value::any_list(Vec::new()));
        }

        let Some(v) = as_slice(l) else {
            return Err(err(format!("type {} not supported", l.go_type_name())));
        };

        let mut seen = KeySet::default();
        let mut out = Vec::new();

        for item in &v.items {
            let (ev, _) = indirect_interface(item);

            let key = normalize(&ev);

            if seen.insert(key) {
                out.push(ev);
            }
        }

        Ok(v.with(out))
    }

    // Go: tpl/collections:Where
    pub fn where_(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "Where")?;
        self.do_where(ctx, &a[0], &a[1], &a[2..])
    }
}

/// Whether the static element type is a non-empty interface (Go keeps such an element of
/// interface kind in `indirect`/`indirectInterface`: `page.Page`, `resource.Resource`).
pub(crate) fn nonempty_interface(elem: &str) -> bool {
    elem_is_interface(elem) && elem != "interface {}"
}

/// `!l.Type().Elem().Comparable()` guard of Intersect (static element type).
fn elem_comparable(elem: &str) -> bool {
    elem_is_interface(elem) || type_comparable(elem)
}

/// Go `lv.Slice(i, j).Interface()` over a slice or string kind value; `bounds(len)` gives
/// `(i, j)`. Errors with Go's text for other kinds.
fn slice_or_string(
    lv: &Value,
    orig: &Value,
    bounds: impl Fn(usize) -> (usize, usize),
) -> GoResult<Value> {
    match kind(lv) {
        ReflectKind::Slice => {
            let s = as_slice(lv).unwrap();
            let (i, j) = bounds(s.items.len());
            Ok(s.sub(i, j))
        }
        ReflectKind::String => {
            let s = rh::string_of(lv);
            let (i, j) = bounds(s.len());
            // A named string type keeps its type.
            Ok(rh::with_string(lv, GoString::from(&s[i..j])))
        }
        _ => Err(err(format!("can't iterate over {}", orig.go_type_name()))),
    }
}

/// Sets `keys` (a nested path) to `v` in `dict` (Go's loop over the `[]string` key).
fn dict_set_nested(dict: &mut Map, keys: &[GoString], v: Value) -> GoResult<()> {
    if keys.len() == 1 {
        dict.insert(keys[0].clone(), v);
        return Ok(());
    }
    let key = &keys[0];
    match dict.entries.get_mut(key) {
        Some(Value::Map(m)) if m.ty == MapType::StringAny => {
            dict_set_nested(Arc::make_mut(m), &keys[1..], v)
        }
        Some(Value::TypedNil(t)) if &**t == "map[string]interface {}" => {
            Err(err("assignment to entry in nil map"))
        }
        Some(other) => {
            let t = hreflect::type_of(other).map_or_else(|| "nil".to_string(), |t| t.into_owned());
            Err(err(format!(
                "interface conversion: interface {{}} is {t}, not map[string]interface {{}}"
            )))
        }
        None => {
            let mut m = Map::new(MapType::StringAny);
            dict_set_nested(&mut m, &keys[1..], v)?;
            dict.insert(key.clone(), Value::map(m));
            Ok(())
        }
    }
}

/// Go `rand.Perm(n)` over the port's own generator (the result is random in Go too).
fn rand_perm(n: usize) -> Vec<usize> {
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut s = STATE.load(Ordering::Relaxed);
    if s == 0 {
        s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
    }
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    // Go: m[i] = m[j]; m[j] = i for j = rand(i+1).
    let mut m = vec![0usize; n];
    for i in 0..n {
        let j = (next() % (i as u64 + 1)) as usize;
        m[i] = m[j];
        m[j] = i;
    }
    STATE.store(s, Ordering::Relaxed);
    m
}

/// Go: `intersector`.
pub(crate) struct Intersector {
    ty: SliceType,
    r: Vec<Value>,
    seen: KeySet,
}

impl Intersector {
    fn new(ty: SliceType) -> Intersector {
        Intersector {
            ty,
            r: Vec::new(),
            seen: KeySet::default(),
        }
    }

    fn value(self) -> Value {
        Value::List(Arc::new(List::new(self.ty, self.r)))
    }

    // Go: tpl/collections/collections.go:appendIfNotSeen
    fn append_if_not_seen(&mut self, v: Value) {
        let k = normalize(&v);
        if self.seen.insert(k) {
            self.r.push(v);
        }
    }

    /// `appendIfNotSeen` of a value of static type `static_ty` into the slice of element type
    /// `elem_ty`: `reflect.Append` panics when the type is not assignable.
    fn append_checked(
        &mut self,
        v: Value,
        static_ty: &str,
        elem_ty: &str,
        iface: bool,
    ) -> GoResult<()> {
        let k = if iface {
            self.interface_key(&v)?
        } else {
            normalize(&v)
        };
        if self.seen.contains(&k) {
            return Ok(());
        }
        if !hreflect::type_assignable_to(static_ty, elem_ty) {
            return Err(err(format!(
                "reflect.Set: value of type {static_ty} is not assignable to type {elem_ty}"
            )));
        }
        self.seen.insert(k);
        self.r.push(v);
        Ok(())
    }

    /// `normalize` of a value of static interface kind. Go's panic for an unhashable dynamic
    /// key has two texts: a lookup in the empty `seen` map (`mapKeyError`) and one in a
    /// non-empty map (the hash function).
    fn interface_key(&self, v: &Value) -> GoResult<rh::NormKey> {
        normalize_interface(v).map_err(|e| {
            if self.seen.is_empty() {
                let msg = e.message();
                if let Some(t) = msg.strip_prefix("runtime error: hash of unhashable type ") {
                    return err(format!("hash of unhashable type: {t}"));
                }
            }
            e
        })
    }

    /// `appendIfNotSeen` of a value of static interface kind.
    fn append_if_not_seen_interface(&mut self, v: Value) -> GoResult<()> {
        let k = self.interface_key(&v)?;
        if self.seen.insert(k) {
            self.r.push(v);
        }
        Ok(())
    }

    // Go: tpl/collections/collections.go:handleValuePair
    /// `k1` is the kind of `l1vv` as Go sees it (the static kind of the slice element).
    fn handle_value_pair(&mut self, l1vv: &Value, k1: ReflectKind, l2vv: &Value) -> GoResult<()> {
        match k1 {
            ReflectKind::String => {
                if let Ok(l2t) = to_string(l2vv)
                    && rh::string_of(l1vv) == l2t
                {
                    self.append_if_not_seen(l1vv.clone());
                }
            }
            k if is_number(k) => {
                let f1 = number_to_float(l1vv);
                let f2 = number_to_float(l2vv);
                if let (Ok(f1), Ok(f2)) = (f1, f2)
                    && f1 == f2
                {
                    self.append_if_not_seen(l1vv.clone());
                }
            }
            ReflectKind::Ptr | ReflectKind::Struct => {
                let u1 = nh_common::types::types::unwrapv(l1vv);
                let u2 = nh_common::types::types::unwrapv(l2vv);
                if rh::interface_equal(&u1, &u2) {
                    self.append_if_not_seen(l1vv.clone());
                }
            }
            ReflectKind::Interface => {
                if l1vv.is_invalid() {
                    // reflect.ValueOf(nil): an invalid value matches no case.
                    return Ok(());
                }
                self.handle_value_pair(l1vv, kind(l1vv), l2vv)?;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Go: `types.KeyValues` as a template value (`keyVals`): fields `Key` and `Values`
/// (`[]interface {}`), methods `KeyString` and `String`.
pub struct KeyValuesObject {
    pub key: Value,
    pub values: Vec<Value>,
}

impl KeyValuesObject {
    fn kv(&self) -> nh_common::types::types::KeyValues {
        nh_common::types::types::KeyValues {
            key: self.key.clone(),
            values: self.values.clone(),
        }
    }
}

nh_common::go_methods!(KeyValuesObject {
    "KeyString" => |k, _ctx, a| {
        args::exactly(a, 0, "KeyString")?;
        Ok(Value::String(k.kv().key_string()))
    },
    "String" => |k, _ctx, a| {
        args::exactly(a, 0, "String")?;
        Ok(Value::String(GoString::from(k.kv().string())))
    },
});

impl Object for KeyValuesObject {
    nh_common::object_basics!("types.KeyValues");

    fn kind(&self) -> Kind {
        Kind::Struct
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Key" => Some(self.key.clone()),
            "Values" => Some(Value::any_list(self.values.clone())),
            _ => None,
        }
    }

    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.kv().string()))
    }

    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Key"), self.key.clone()),
            (
                Cow::Borrowed("Values"),
                Value::any_list(self.values.clone()),
            ),
        ])
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

/// Unused-import guard for `Any` (kept from the skeleton).
#[allow(dead_code)]
fn _any(_: &dyn Any) {}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/collections.go (687 lines; 7/20 funcs executed)
//   types: Namespace, intersector
// OK L37-49: New(deps *deps.Deps) *Namespace
// OK L59-91: (ns *Namespace) After(n any, l any) (any, error)
// OK L95-149: (ns *Namespace) Delimit(ctx context.Context, l, sep any, last ...any) (string, error)
// OK L154-188: (ns *Namespace) Dictionary(values ...any) (map[string]any, error)
// OK L191-223: (ns *Namespace) First(limit any, l any) (any, error)
// OK L226-261: (ns *Namespace) In(l any, v any) (bool, error)
// OK L265-302: (ns *Namespace) Intersect(l1, l2 any) (any, error)
// OK L306-322: (ns *Namespace) Group(key any, items any) (any, error)
// OK L326-348: (ns *Namespace) IsSet(c any, key any) (bool, error)
// OK L351-383: (ns *Namespace) Last(limit any, l any) (any, error)
// OK L386-406: (ns *Namespace) Reverse(l any) (any, error)
// OK L417-482: (ns *Namespace) Seq(args ...any) ([]int, error)
// OK L485-512: (ns *Namespace) Shuffle(l any) (any, error)
// OK L515-521: (ns *Namespace) Slice(args ...any) any
// OK L528-534: (i *intersector) appendIfNotSeen(v reflect.Value)
// OK L536-556: (i *intersector) handleValuePair(l1vv, l2vv reflect.Value)
// OK L562-641: (ns *Namespace) Union(l1, l2 any) (any, error)
// OK L644-676: (ns *Namespace) Uniq(l any) (any, error)
// OK L679-681: (ns *Namespace) KeyVals(key any, values ...any) (types.KeyValues, error)
// OK L685-687: (ns *Namespace) NewScratch() *maps.Scratch
// ---------------------------------------------------------------------------
