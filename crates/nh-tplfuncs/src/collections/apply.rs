//! Port of `tpl/collections/apply.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! `indirect`/`indirectInterface` are in `reflect_helpers`. The looked-up function is a
//! [`TplFunc`]: it receives the template context and validates its own arguments, so Go's
//! `called apply using %s as type %s` assignability check (reflect on the func signature) is
//! the function's own argument error instead (PORTING.md).

use go_value::{GoString, HostCtx, Value};
use nh_common::object::GoResult;
use nh_tplimpl::engine::TplFunc;

use super::collections::Namespace;
use super::reflect_helpers::{as_slice, indirect, kind};
use nh_common::hreflect::ReflectKind;

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

impl Namespace {
    // Go: tpl/collections/apply.go:Apply
    /// Apply takes an array or slice c and returns a new slice with the function fname applied
    /// over it.
    pub fn do_apply(
        &self,
        ctx: HostCtx<'_>,
        c: &Value,
        fname: &GoString,
        args: &[Value],
    ) -> GoResult<Value> {
        if c.is_invalid() {
            return Ok(Value::any_list(Vec::new()));
        }

        if fname == "apply" {
            return Err(err("can't apply myself (no turtles allowed)"));
        }

        let (seqv, is_nil) = indirect(c);
        if is_nil {
            return Err(err("can't iterate over a nil value"));
        }

        let fname_s = String::from_utf8_lossy(fname).into_owned();
        let Some(fnv) = self.lookup_func(ctx, &fname_s)? else {
            return Err(err(format!("can't find function {fname_s}")));
        };

        match kind(&seqv) {
            ReflectKind::Slice => {
                let s = as_slice(&seqv).unwrap();
                let mut r = Vec::with_capacity(s.items.len());
                for vv in &s.items {
                    let vvv = apply_fn_to_this(ctx, &fnv, vv, args)?;
                    r.push(vvv);
                }
                Ok(Value::any_list(r))
            }
            _ => Err(err(format!(
                "can't apply over {}",
                String::from_utf8_lossy(&go_fmt::sprintf("%v", std::slice::from_ref(c)))
            ))),
        }
    }

    // Go: tpl/collections/apply.go:lookupFunc
    fn lookup_func(&self, ctx: HostCtx<'_>, fname: &str) -> GoResult<Option<TplFunc>> {
        let Some((namespace, method_name)) = fname.split_once('.') else {
            return Ok(self.get_func(fname));
        };

        // Namespace
        let Some(nv) = self.lookup_func(ctx, namespace)? else {
            return Ok(None);
        };

        let v = nv(ctx, &[])?;
        let Value::Object(o) = v else {
            return Ok(None);
        };

        // method
        if !o.has_method(method_name) {
            return Ok(None);
        }
        let method_name = method_name.to_string();
        let f: TplFunc = std::sync::Arc::new(move |ctx: HostCtx<'_>, a: &[Value]| {
            o.call_method(ctx, &method_name, a)
                .unwrap_or_else(|| Err(err(format!("method {method_name} not found"))))
        });
        Ok(Some(f))
    }

    /// Go `ns.deps.GetTemplateStore().GetFunc(fname)`; the namespace's `funcs` when the store
    /// is not set (component tests).
    fn get_func(&self, name: &str) -> Option<TplFunc> {
        if let Some(store) = self.d.template_store.get() {
            return store.get_func(name);
        }
        self.funcs.get().and_then(|f| f.get(name).cloned())
    }
}

// Go: tpl/collections/apply.go:applyFnToThis
fn apply_fn_to_this(
    ctx: HostCtx<'_>,
    f: &TplFunc,
    this: &Value,
    args: &[Value],
) -> GoResult<Value> {
    let n: Vec<Value> = args
        .iter()
        .map(|arg| match arg {
            Value::String(s) if s == "." => this.clone(),
            _ => arg.clone(),
        })
        .collect();

    f(ctx, &n)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/apply.go (162 lines; 1/5 funcs executed)
// OK L27-65: (ns *Namespace) Apply(ctx context.Context, c any, fname string, args ...any) (any, error)
// OK L67-106: applyFnToThis(ctx context.Context, fn, this reflect.Value, args ...any) (reflect.Value, error)
// OK L108-137: (ns *Namespace) lookupFunc(ctx context.Context, fname string) (reflect.Value, bool)
// OK L140-150: indirect(v reflect.Value) (rv reflect.Value, isNil bool) (reflect_helpers::indirect)
// OK L152-162: indirectInterface(v reflect.Value) (rv reflect.Value, isNil bool) (reflect_helpers::indirect_interface)
// ---------------------------------------------------------------------------
