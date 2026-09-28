//! Port of `tpl/partials/partials.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use go_value::{HostCtx, Object, Value};
use nh_common::constants::WARN_PARTIAL_SUPERFLUOUS_PREFIX;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;
use nh_tpl::template::TplContext;
use nh_tplimpl::templatestore::TemplInfo;

// Parity notes: `partial NAME DATA`: TemplateStore.LookupPartial (lower-cased, html by default), returns `template.HTML` (text partials: string) or the `return` value; executes with a child TplContext (level+1).

/// Go: `partialCacheKey`.
struct PartialCacheKey<'a> {
    name: &'a str,
    variants: &'a [Value],
}

impl PartialCacheKey<'_> {
    /// Go: `Key()` — the name, or the hash of the name and the variants (Go's `Variants` is nil
    /// when `partialCached` gets no variant).
    // Go: tpl/partials/partials.go:Key
    fn key(&self) -> GoResult<String> {
        if self.variants.is_empty() {
            return Ok(self.name.to_string());
        }
        let mut vs = vec![Value::string(self.name)];
        vs.extend_from_slice(self.variants);
        Ok(nh_common::hashing::try_hash_string(&vs)?)
    }
}

/// Go: `includeResult` (only successful results are cached).
#[derive(Clone)]
struct IncludeResult {
    name: String,
    result: Value,
}

/// Go: `lazycache.Cache[string, includeResult]` with `MaxEntries: 1000` — an LRU
/// (hashicorp `simplelru`). As in lazycache, an entry is added before its value is created and
/// removed again when the creation fails.
struct PartialCache {
    max_entries: usize,
    /// Most recently used last.
    order: VecDeque<String>,
    entries: HashMap<String, Option<IncludeResult>>,
}

impl PartialCache {
    fn new(max_entries: usize) -> PartialCache {
        PartialCache {
            max_entries,
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    fn touch(&mut self, key: &str) {
        if let Some(i) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(i).expect("index in range");
            self.order.push_back(k);
        }
    }

    /// simplelru `Get`: the entry (moved to the front), if any.
    fn get(&mut self, key: &str) -> Option<Option<IncludeResult>> {
        let v = self.entries.get(key).cloned()?;
        self.touch(key);
        Some(v)
    }

    /// simplelru `Add`.
    fn add(&mut self, key: &str, v: Option<IncludeResult>) {
        if self.entries.contains_key(key) {
            self.entries.insert(key.to_string(), v);
            self.touch(key);
            return;
        }
        self.entries.insert(key.to_string(), v);
        self.order.push_back(key.to_string());
        if self.order.len() > self.max_entries
            && let Some(oldest) = self.order.pop_front()
        {
            self.entries.remove(&oldest);
        }
    }

    fn remove(&mut self, key: &str) {
        if self.entries.remove(key).is_some()
            && let Some(i) = self.order.iter().position(|k| k == key)
        {
            self.order.remove(i);
        }
    }

    // Go: tpl/partials/partials.go:clear
    fn clear(&mut self) {
        self.order.clear();
        self.entries.clear();
    }
}

/// Go: `partials.Namespace` (template value `*partials.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    cached_partials: Arc<Mutex<PartialCache>>,
}

/// Go: `*partials.contextWrapper` — makes room for a return value in a partial invocation.
pub struct ContextWrapper {
    pub arg: Value,
    pub result: Mutex<Value>,
}

nh_common::go_methods!(ContextWrapper {
    // Go: tpl/partials/partials.go:(*contextWrapper).Set
    "Set" => |c, _ctx, a| {
        args::exactly(a, 1, "Set")?;
        *c.result.lock().unwrap_or_else(|e| e.into_inner()) = a[0].clone();
        Ok(Value::string(""))
    },
});

impl Object for ContextWrapper {
    nh_common::object_basics!("*partials.contextWrapper");
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Arg" => Some(self.arg.clone()),
            "Result" => Some(
                self.result
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone(),
            ),
            _ => None,
        }
    }
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

impl Namespace {
    /// New returns a new instance of the templates-namespaced template functions.
    // Go: tpl/partials/partials.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        // This lazycache was introduced in Hugo 0.111.0.
        let cache = Arc::new(Mutex::new(PartialCache::new(1000)));
        let c = cache.clone();
        d.build_start_listeners.add(Box::new(move |_: &[()]| {
            c.lock().unwrap_or_else(|e| e.into_inner()).clear();
            false
        }));

        Namespace {
            d,
            cached_partials: cache,
        }
    }

    /// Include executes the named partial. If the partial contains a return statement, that
    /// value will be returned. Else, the rendered output will be returned: a string if the
    /// partial is a text/template, or template.HTML when html/template.
    // Go: tpl/partials/partials.go:Include
    pub fn include(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Include")?;
        let name = args::string(a, 0)?;
        let res = self.include_impl(ctx, &name.to_str_lossy(), &a[1..])?;
        Ok(res.result)
    }

    // Go: tpl/partials/partials.go:include
    fn include_impl(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        data_list: &[Value],
    ) -> GoResult<IncludeResult> {
        let v = self.lookup(name)?;
        self.do_include(ctx, &v, data_list)
    }

    // Go: tpl/partials/partials.go:lookup
    fn lookup(&self, name: &str) -> GoResult<Arc<TemplInfo>> {
        if name.starts_with("partials/") {
            // This is most likely not what the user intended.
            // This worked before Hugo 0.146.0.
            self.d.log.warnidf(
                WARN_PARTIAL_SUPERFLUOUS_PREFIX,
                format!(
                    "Doubtful use of partial function in {{{{ partial \"{name}\"}}}}), this is most likely not what you want. Consider removing superfluous prefix \"partials/\" from template name given as first function argument."
                ),
            );
        }
        match self.d.get_template_store().lookup_partial(name) {
            Some(v) => Ok(v),
            None => Err(gerr(format!(
                "partial {} not found",
                go_strconv::quote(name.as_bytes())
            ))),
        }
    }

    /// include is a helper function that lookups and executes the named partial. Returns the
    /// final template name and the rendered output.
    // Go: tpl/partials/partials.go:doInclude
    fn do_include(
        &self,
        ctx: HostCtx<'_>,
        templ: &Arc<TemplInfo>,
        data_list: &[Value],
    ) -> GoResult<IncludeResult> {
        let mut data = data_list.first().cloned().unwrap_or(Value::Invalid);

        let info = templ.parse_info();

        let mut wrapper: Option<Arc<ContextWrapper>> = None;
        if info.has_return {
            // Wrap the context sent to the template to capture the return value. Note that the
            // template is rewritten to make sure that the dot (".") and the $ variable points to
            // Arg.
            let w = Arc::new(ContextWrapper {
                arg: data,
                result: Mutex::new(Value::Invalid),
            });
            data = Value::Object(w.clone());
            wrapper = Some(w);
        }

        // (With a return value, Go discards the template output.)
        let mut w: Vec<u8> = Vec::new();

        let default_ctx = TplContext::default();
        let tctx = TplContext::from_host(ctx).unwrap_or(&default_ctx);
        self.d
            .get_template_store()
            .execute_with_context(tctx, templ, &mut w, &data)?;

        let result = if let Some(wrapper) = wrapper {
            wrapper
                .result
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else if templ.template().is_some_and(|t| t.is_text()) {
            Value::string(w)
        } else {
            Value::html(w)
        };

        Ok(IncludeResult {
            name: templ.name(),
            result,
        })
    }

    /// IncludeCached executes and caches partial templates. The cache is created with
    /// name+variants as the key.
    // Go: tpl/partials/partials.go:IncludeCached
    pub fn include_cached(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "IncludeCached")?;
        let name = args::string(a, 0)?;
        let name = name.to_str_lossy().into_owned();
        let context = &a[1];
        let key = PartialCacheKey {
            name: &name,
            variants: &a[2..],
        };
        let ti = self.lookup(&name)?;

        if let Some(mut parent) =
            TplContext::from_host(ctx).and_then(|c| c.current_template.clone())
        {
            let (ti_name, ti_filename) = (ti.name(), ti.filename());
            loop {
                if parent.name == ti_name && parent.filename == ti_filename {
                    // This will deadlock if we continue.
                    return Err(gerr(format!(
                        "circular call stack detected in partial {}",
                        go_strconv::quote(ti_filename.as_bytes())
                    )));
                }
                match parent.parent.clone() {
                    Some(p) => parent = p,
                    None => break,
                }
            }
        }

        let key = key.key()?;

        // lazycache GetOrCreate: the entry is added before the value is created (never under
        // the lock while creating, HUGO_LAYER.md §4.8).
        {
            let mut cache = self
                .cached_partials
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(found) = cache.get(&key) {
                return match found {
                    Some(r) => Ok(r.result),
                    // Go waits for the creation in progress (a partial that calls itself
                    // through partialCached is caught by the circular check above).
                    None => Err(gerr(format!(
                        "circular call stack detected in partial {}",
                        go_strconv::quote(ti.filename().as_bytes())
                    ))),
                };
            }
            cache.add(&key, None);
        }

        let r = self.do_include(ctx, &ti, std::slice::from_ref(context));

        let mut cache = self
            .cached_partials
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match r {
            Ok(r) => {
                if cache.entries.contains_key(&key) {
                    cache.entries.insert(key, Some(r.clone()));
                }
                Ok(r.result)
            }
            Err(e) => {
                cache.remove(&key);
                Err(e)
            }
        }
    }
}

nh_common::go_methods!(Namespace {
    "Include" => |n, ctx, a| n.include(ctx, a),
    "IncludeCached" => |n, ctx, a| n.include_cached(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*partials.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/partials/partials.go (251 lines; 5/9 funcs executed)
//   types: partialCacheKey, includeResult, partialCache, Namespace, contextWrapper
// OK L51-56: (k partialCacheKey) Key() string
// OK L63-67: (p *partialCache) clear()
// OK L70-87: New(deps *deps.Deps) *Namespace
// OK L102-105: (c *contextWrapper) Set(in any) string
// OK L112-123: (ns *Namespace) Include(ctx context.Context, name string, contextList ...any) (any, error)
// OK L125-131: (ns *Namespace) include(ctx context.Context, name string, dataList ...any) includeResult
// OK L133-144: (ns *Namespace) lookup(name string) (*tplimpl.TemplInfo, error)
// OK L148-192: (ns *Namespace) doInclude(ctx context.Context, templ *tplimpl.TemplInfo, dataList ...any) includeResult
// OK L196-251: (ns *Namespace) IncludeCached(ctx context.Context, name string, context any, variants ...any) (any, error)
// ---------------------------------------------------------------------------
