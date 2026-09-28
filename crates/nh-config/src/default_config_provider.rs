//! Port of `config/defaultConfigProvider.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::sync::{Arc, RwLock};

use go_value::{GoString, Map, MapType, Value};
use nh_common::cast::caste;
use nh_common::maps::params::{
    KeyParams, ParamsMergeStrategy, get_merge_strategy, merge_params, merge_params_with_strategy,
    must_to_params_and_prepare, new_params, params_is_zero, prepare_params, set_merge_strategy,
    set_params, to_params_and_prepare,
};

use crate::config_provider::Provider;

/// Go: `config.defaultConfigProvider` — a `maps.Params` tree behind a RWMutex; keys lower-cased,
/// dot paths navigate nested Params. `Set("", m)` replaces the root (after `ToParamsAndPrepare`).
pub struct DefaultConfigProvider {
    root: RwLock<Map>,
}

impl DefaultConfigProvider {
    /// New creates a Provider backed by an empty maps.Params.
    // Go: config/defaultConfigProvider.go:New
    pub fn new() -> Self {
        DefaultConfigProvider {
            root: RwLock::new(Map::new(MapType::Params)),
        }
    }

    /// NewFrom creates a Provider backed by params.
    // Go: config/defaultConfigProvider.go:NewFrom
    pub fn new_from(mut params: Map) -> Self {
        prepare_params(&mut params);
        params.ty = MapType::Params;
        DefaultConfigProvider {
            root: RwLock::new(params),
        }
    }

    /// Snapshot of the root params.
    pub fn root(&self) -> Map {
        self.root.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Default for DefaultConfigProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Go `strings.ToLower`.
fn lower(k: &str) -> String {
    go_unicode::strings::to_lower_str(k).into_owned()
}

/// Go: `case map[string]any, map[any]any, map[string]string: v = maps.MustToParamsAndPrepare(vv)`.
fn params_if_plain_map(v: Value) -> Value {
    match &v {
        Value::Map(m) if matches!(m.ty, MapType::StringAny | MapType::StringString) => {
            Value::map(must_to_params_and_prepare(&v))
        }
        _ => v,
    }
}

/// Go: `v.(maps.Params)` (panics like Go for another type).
fn as_params_or_panic(v: &Value) -> &Map {
    match v {
        Value::Map(m) if m.ty == MapType::Params => m,
        _ => panic!(
            "interface conversion: interface {{}} is {}, not maps.Params",
            if v.is_invalid() {
                "nil".into()
            } else {
                v.go_type_name()
            }
        ),
    }
}

/// Go: `getNestedKeyAndMap(key, false)`: the last key part and the map that holds it.
// Go: config/defaultConfigProvider.go:getNestedKeyAndMap
fn get_nested_key_and_map<'a>(root: &'a Map, key: &str) -> Option<(String, &'a Map)> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        let next = current.get(part.as_bytes())?;
        match next {
            Value::Map(m) if m.ty == MapType::Params => current = m,
            // E.g. a string, not a map that we can store values in.
            _ => return None,
        }
    }
    Some((parts[parts.len() - 1].to_string(), current))
}

/// Go: `getNestedKeyAndMap(key, true)` (missing intermediate maps are created).
fn get_nested_key_and_map_create<'a>(
    root: &'a mut Map,
    key: &str,
) -> Option<(String, &'a mut Map)> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        let next = current
            .entries
            .entry(GoString::from(*part))
            .or_insert_with(|| Value::map(new_params()));
        match next {
            Value::Map(m) if m.ty == MapType::Params => current = Arc::make_mut(m),
            // E.g. a string, not a map that we can store values in.
            _ => return None,
        }
    }
    Some((parts[parts.len() - 1].to_string(), current))
}

/// Go: `determineMergeStrategy(params...)`. `None` is Go's empty strategy.
// Go: config/defaultConfigProvider.go:determineMergeStrategy
fn determine_merge_strategy(params: &[KeyParams]) -> Option<ParamsMergeStrategy> {
    if params.is_empty() {
        return Some(ParamsMergeStrategy::None);
    }

    let mut strategy: Option<ParamsMergeStrategy> = None;
    let mut prev_is_root = false;
    let curr = &params[params.len() - 1];

    if params.len() > 1 {
        let prev = &params[params.len() - 2];
        prev_is_root = prev.key.is_empty();

        // Inherit from parent (but not from the root unless it's set by user).
        let (s, found) = get_merge_strategy(&prev.params);
        if !prev_is_root && !found {
            panic!("invalid state, merge strategy not set on parent");
        }
        if found || !prev_is_root {
            strategy = Some(s);
        }
    }

    match curr.key.as_bytes() {
        b"" => {
            // Don't set a merge strategy on the root unless set by user. This will be handled
            // as a special case.
        }
        b"params" => strategy = Some(ParamsMergeStrategy::Deep),
        b"outputformats" | b"mediatypes" => {
            if prev_is_root {
                strategy = Some(ParamsMergeStrategy::Shallow);
            }
        }
        b"menus" => {
            let mut is_menu_key = prev_is_root;
            if !is_menu_key {
                // Can also be set below languages.
                // root > languages > en > menus
                if params.len() == 4 && params[1].key.as_bytes() == b"languages" {
                    is_menu_key = true;
                }
            }
            if is_menu_key {
                strategy = Some(ParamsMergeStrategy::Shallow);
            }
        }
        _ => {
            if strategy.is_none() {
                strategy = Some(ParamsMergeStrategy::None);
            }
        }
    }

    strategy
}

/// The recursive walk of `SetDefaultMergeStrategy` (Go: `WalkParams` with a walk func that
/// sets `_merge` in place; children are visited after their parent, in byte order).
fn set_default_merge_strategy_walk(path: &mut Vec<KeyParams>, key: GoString, p: &mut Map) {
    // The walk func.
    let (_, found) = get_merge_strategy(p);
    if !found {
        path.push(KeyParams {
            key: key.clone(),
            params: p.clone(),
        });
        let strategy = determine_merge_strategy(path);
        path.pop();
        if let Some(s) = strategy {
            set_merge_strategy(p, s);
        }
    }

    path.push(KeyParams {
        key,
        params: p.clone(),
    });
    let keys: Vec<GoString> = p.entries.keys().cloned().collect();
    for k in keys {
        if let Some(Value::Map(m)) = p.entries.get_mut(&k)
            && m.ty == MapType::Params
        {
            set_default_merge_strategy_walk(path, k, Arc::make_mut(m));
        }
    }
    path.pop();
}

/// Go: `WalkParams`'s recursive `walk`.
fn walk(params: &mut Vec<KeyParams>, walk_fn: &mut dyn FnMut(&[KeyParams]) -> bool) {
    if walk_fn(params) {
        return;
    }
    let p1 = params[params.len() - 1].params.clone();
    for (k, v) in &p1.entries {
        if let Value::Map(p2) = v
            && p2.ty == MapType::Params
        {
            params.push(KeyParams {
                key: k.clone(),
                params: (**p2).clone(),
            });
            walk(params, walk_fn);
            params.pop();
        }
    }
}

impl Provider for DefaultConfigProvider {
    // Go: config/defaultConfigProvider.go:GetString
    fn get_string(&self, key: &str) -> String {
        let v = self.get(key);
        String::from_utf8_lossy(&caste::to_string(&v)).into_owned()
    }

    // Go: config/defaultConfigProvider.go:GetInt
    fn get_int(&self, key: &str) -> i64 {
        caste::to_int(&self.get(key))
    }

    // Go: config/defaultConfigProvider.go:GetBool
    fn get_bool(&self, key: &str) -> bool {
        caste::to_bool(&self.get(key))
    }

    // Go: config/defaultConfigProvider.go:GetParams
    fn get_params(&self, key: &str) -> Option<Map> {
        let v = self.get(key);
        if v.is_invalid() {
            return None;
        }
        Some(as_params_or_panic(&v).clone())
    }

    // Go: config/defaultConfigProvider.go:GetStringMap
    fn get_string_map(&self, key: &str) -> Map {
        nh_common::maps::maps::to_string_map(&self.get(key))
    }

    // Go: config/defaultConfigProvider.go:GetStringMapString
    fn get_string_map_string(&self, key: &str) -> Map {
        nh_common::maps::maps::to_string_map_string(&self.get(key))
    }

    // Go: config/defaultConfigProvider.go:GetStringSlice
    fn get_string_slice(&self, key: &str) -> Vec<String> {
        caste::to_string_slice(&self.get(key))
            .iter()
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    }

    // Go: config/defaultConfigProvider.go:Get
    fn get(&self, k: &str) -> Value {
        let root = self.root.read().unwrap_or_else(|e| e.into_inner());
        if k.is_empty() {
            return Value::map(root.clone());
        }
        let Some((key, m)) = get_nested_key_and_map(&root, &lower(k)) else {
            return Value::Invalid;
        };
        m.get(key.as_bytes()).cloned().unwrap_or(Value::Invalid)
    }

    // Go: config/defaultConfigProvider.go:Set
    fn set(&self, k: &str, v: Value) {
        let mut root = self.root.write().unwrap_or_else(|e| e.into_inner());

        let k = lower(k);

        if k.is_empty() {
            match to_params_and_prepare(&v) {
                // Set the values directly in root.
                Ok(p) => set_params(&mut root, &p),
                Err(_) => {
                    root.insert(k.as_str(), v);
                }
            }
            return;
        }

        let v = params_if_plain_map(v);

        let Some((key, m)) = get_nested_key_and_map_create(&mut root, &k) else {
            return;
        };

        if let Some(existing) = m.entries.get_mut(key.as_bytes())
            && let Value::Map(p1) = existing
            && p1.ty == MapType::Params
            && let Value::Map(p2) = &v
            && p2.ty == MapType::Params
        {
            set_params(Arc::make_mut(p1), p2);
            return;
        }

        m.insert(key.as_str(), v);
    }

    /// Go returns the keys in random map order; here in byte order.
    // Go: config/defaultConfigProvider.go:Keys
    fn keys(&self) -> Vec<String> {
        let root = self.root.read().unwrap_or_else(|e| e.into_inner());
        root.entries
            .keys()
            .map(|k| String::from_utf8_lossy(k).into_owned())
            .collect()
    }

    // Go: config/defaultConfigProvider.go:Merge
    fn merge(&self, k: &str, v: Value) {
        let mut root = self.root.write().unwrap_or_else(|e| e.into_inner());
        let k = lower(k);

        if k.is_empty() {
            let (rs, f) = get_merge_strategy(&root);
            if f && rs == ParamsMergeStrategy::None {
                // The user has set a "no merge" strategy on this, nothing more to do.
                return;
            }

            match to_params_and_prepare(&v) {
                Ok(p) => {
                    // As there may be keys in p not in root, we need to handle those as a
                    // special case.
                    let mut keys_to_delete: Vec<GoString> = Vec::new();
                    for (kk, vv) in &p.entries {
                        let Value::Map(pp) = vv else { continue };
                        if pp.ty != MapType::Params {
                            continue;
                        }
                        match root.entries.get_mut(kk) {
                            Some(pppi) => {
                                as_params_or_panic(pppi);
                                if let Value::Map(ppp) = pppi {
                                    merge_params_with_strategy("", Arc::make_mut(ppp), pp);
                                }
                            }
                            None => {
                                // We need to use the default merge strategy for this key.
                                let mut np = new_params();
                                let strategy = determine_merge_strategy(&[
                                    KeyParams {
                                        key: GoString::empty(),
                                        params: root.clone(),
                                    },
                                    KeyParams {
                                        key: kk.clone(),
                                        params: np.clone(),
                                    },
                                ]);
                                match strategy {
                                    Some(s) => set_merge_strategy(&mut np, s),
                                    // Go: `SetMergeStrategy` panics on the empty strategy (a
                                    // key "" merged into a root without `_merge`).
                                    None => panic!("invalid merge strategy \"\""),
                                }
                                merge_params_with_strategy("", &mut np, pp);
                                let zero = params_is_zero(&np);
                                root.insert(kk.clone(), Value::map(np));
                                if zero {
                                    // Just keep it until merge is done.
                                    keys_to_delete.push(kk.clone());
                                }
                            }
                        }
                    }
                    // Merge the rest.
                    merge_params(&mut root, &p);
                    for k in keys_to_delete {
                        root.entries.remove(&k);
                    }
                }
                Err(_) => panic!("unsupported type {} received in Merge", v.go_type_name()),
            }

            return;
        }

        let v = params_if_plain_map(v);

        let Some((key, m)) = get_nested_key_and_map_create(&mut root, &k) else {
            return;
        };

        match m.entries.get_mut(key.as_bytes()) {
            Some(existing) => {
                if let Value::Map(p1) = existing
                    && p1.ty == MapType::Params
                    && let Value::Map(p2) = &v
                    && p2.ty == MapType::Params
                {
                    merge_params_with_strategy("", Arc::make_mut(p1), p2);
                }
            }
            None => {
                m.insert(key.as_str(), v);
            }
        }
    }

    /// SetDefaults will set values from params if not already set.
    // Go: config/defaultConfigProvider.go:SetDefaults
    fn set_defaults(&self, params: &Map) {
        let mut params = params.clone();
        prepare_params(&mut params);
        let mut root = self.root.write().unwrap_or_else(|e| e.into_inner());
        for (k, v) in &params.entries {
            if !root.entries.contains_key(k) {
                root.entries.insert(k.clone(), v.clone());
            }
        }
    }

    /// Adds `_merge` keys per the rules in defaultConfigProvider.go:263-335 (params deep, root
    /// outputformats/mediatypes shallow, menus shallow, else none) — these keys ARE part of the
    /// imaging-config hash (see specs/images.md §4.3).
    // Go: config/defaultConfigProvider.go:SetDefaultMergeStrategy
    fn set_default_merge_strategy(&self) {
        let mut root = self.root.write().unwrap_or_else(|e| e.into_inner());
        let mut path = Vec::new();
        set_default_merge_strategy_walk(&mut path, GoString::empty(), &mut root);
    }

    /// The walk visits nested Params in byte order (Go: random map order).
    // Go: config/defaultConfigProvider.go:WalkParams
    fn walk_params(&self, walk_fn: &mut dyn FnMut(&[KeyParams]) -> bool) {
        let root = self.root();
        let mut params = vec![KeyParams {
            key: GoString::empty(),
            params: root,
        }];
        walk(&mut params, walk_fn);
    }

    // Go: config/defaultConfigProvider.go:IsSet
    fn is_set(&self, k: &str) -> bool {
        let root = self.root.read().unwrap_or_else(|e| e.into_inner());
        match get_nested_key_and_map(&root, &lower(k)) {
            Some((key, m)) => m.entries.contains_key(key.as_bytes()),
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/defaultConfigProvider.go (366 lines; 16/19 funcs executed)
//   types: defaultConfigProvider
// OK L29-33: New() Provider
// OK L36-41: NewFrom(params maps.Params) Provider
// OK L52-65: (c *defaultConfigProvider) Get(k string) any
// OK L67-70: (c *defaultConfigProvider) GetBool(k string) bool
// OK L72-75: (c *defaultConfigProvider) GetInt(k string) int
// OK L77-86: (c *defaultConfigProvider) IsSet(k string) bool
// OK L88-91: (c *defaultConfigProvider) GetString(k string) string
// OK L93-99: (c *defaultConfigProvider) GetParams(k string) maps.Params
// OK L101-104: (c *defaultConfigProvider) GetStringMap(k string) map[string]any
// OK L106-109: (c *defaultConfigProvider) GetStringMapString(k string) map[string]string
// OK L111-114: (c *defaultConfigProvider) GetStringSlice(k string) []string
// OK L116-154: (c *defaultConfigProvider) Set(k string, v any)
// OK L157-164: (c *defaultConfigProvider) SetDefaults(params maps.Params)
// OK L166-235: (c *defaultConfigProvider) Merge(k string, v any)
// OK L237-241: (c *defaultConfigProvider) Keys() []string
// OK L243-261: (c *defaultConfigProvider) WalkParams(walkFn func(params ...maps.KeyParams) bool)
// OK L263-317: (c *defaultConfigProvider) determineMergeStrategy(params ...maps.KeyParams) maps.ParamsMergeStrategy
// OK L319-336: (c *defaultConfigProvider) SetDefaultMergeStrategy()
// OK L338-366: (c *defaultConfigProvider) getNestedKeyAndMap(key string, create bool) (string, maps.Params)
// ---------------------------------------------------------------------------
