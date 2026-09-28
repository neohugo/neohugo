//! Port of `resources/resource_metadata.go`.
//!
//! Owner: Wave B task T14 (resources-core).

//! Front matter `resources` metadata (`src` globs with `name`/`title`/`params`, `:counter`).
//!
//! Go's `maps.Params` is a reference type: `CloneWithMetadataFromMapIfNeeded` starts the
//! metadata's params from the resource's own `Params()` map and `updateParams` copies into that
//! very map, so the resource's params change too. [`SharedParams`] keeps that sharing.

use std::sync::{Arc, RwLock};

use go_value::{GoString, Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_resource::resourcetypes::Resource;

const COUNTER_PLACE_HOLDER: &str = ":counter";

/// A Go `maps.Params` value: one map shared by everything that holds it.
#[derive(Clone)]
pub struct SharedParams(Arc<RwLock<Arc<Map>>>);

impl SharedParams {
    pub fn new(m: Map) -> SharedParams {
        SharedParams(Arc::new(RwLock::new(Arc::new(m))))
    }

    /// The current map.
    pub fn get(&self) -> Arc<Map> {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Go `maps.Copy(dst, src)`: `src`'s entries overwrite.
    pub fn copy_from(&self, src: &Map) {
        let mut g = self.0.write().unwrap_or_else(|e| e.into_inner());
        let m = Arc::make_mut(&mut g);
        for (k, v) in &src.entries {
            m.entries.insert(k.clone(), v.clone());
        }
    }
}

/// Go: `resources.metaResource` (front matter `resources` name/title/params overrides).
#[derive(Clone, Default)]
pub struct MetaResource {
    pub changed: bool,
    pub title: String,
    pub name: String,
    /// Go `params maps.Params` (`None` = nil).
    pub params: Option<SharedParams>,
}

impl MetaResource {
    // Go: resources/resource_metadata.go:(*metaResource).Name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    // Go: resources/resource_metadata.go:(*metaResource).Title
    pub fn title(&self) -> String {
        self.title.clone()
    }

    // Go: resources/resource_metadata.go:(*metaResource).Params
    pub fn params(&self) -> Option<Arc<Map>> {
        self.params.as_ref().map(|p| p.get())
    }

    // Go: resources/resource_metadata.go:(*metaResource).setTitle
    fn set_title(&mut self, title: String) {
        self.title = title;
        self.changed = true;
    }

    // Go: resources/resource_metadata.go:(*metaResource).setName
    fn set_name(&mut self, name: String) {
        self.name = name;
        self.changed = true;
    }

    // Go: resources/resource_metadata.go:(*metaResource).updateParams
    fn update_params(&mut self, params: &Map) {
        let p = self
            .params
            .get_or_insert_with(|| SharedParams::new(Map::new(MapType::Params)));
        p.copy_from(params);
        self.changed = true;
    }
}

/// cloneWithMetadataFromResourceConfigIfNeeded clones the given resource with the given
/// metadata if the resource supports it.
// Go: resources/resource_metadata.go:cloneWithMetadataFromResourceConfigIfNeeded
pub fn clone_with_metadata_from_resource_config_if_needed(
    rc: &mut nh_page::pagemeta::page_frontmatter::ResourceConfig,
    r: Arc<dyn Resource>,
) -> Arc<dyn Resource> {
    let Some(wmp) = crate::transform::resource_adapter(&r) else {
        return r;
    };

    if rc.name.is_empty() && rc.title.is_empty() && rc.params.as_ref().is_none_or(|p| p.is_empty())
    {
        // No metadata.
        return r;
    }

    if rc.title.is_empty() {
        rc.title = rc.name.clone();
    }

    let wrapped = MetaResource {
        changed: false,
        name: rc.name.clone(),
        title: rc.title.clone(),
        params: rc.params.clone().map(SharedParams::new),
    };

    wmp.with_resource_meta(Arc::new(wrapped))
}

/// CloneWithMetadataFromMapIfNeeded clones the given resource with the given metadata if the
/// resource supports it.
// Go: resources/resource_metadata.go:CloneWithMetadataFromMapIfNeeded
pub fn clone_with_metadata_from_map_if_needed(
    m: &[Map],
    r: Arc<dyn Resource>,
) -> Arc<dyn Resource> {
    let Some(wmp) = crate::transform::resource_adapter(&r) else {
        return r;
    };

    let mut wrapped = MetaResource {
        changed: false,
        name: wmp.name(),
        title: wmp.title(),
        params: wmp.params_shared(),
    };

    let _ = assign_metadata_to(m, &mut wrapped);
    if !wrapped.changed {
        return r;
    }

    wmp.with_resource_meta(Arc::new(wrapped))
}

/// Go: `resources.AssignMetadata` as hugolib applies it: every resource of a page goes through
/// [`clone_with_metadata_from_map_if_needed`].
pub fn assign_metadata(
    metadata: &[Map],
    resources: &mut nh_resource::resourcetypes::Resources,
) -> Result<()> {
    for r in resources.iter_mut() {
        *r = clone_with_metadata_from_map_if_needed(metadata, r.clone());
    }
    Ok(())
}

/// AssignMetadata assigns the given metadata to those resources that supports updates and
/// matching by wildcard given in `src` using `filepath.Match` with lower cased values. This
/// assignment is additive, but the most specific match needs to be first. The `name` and
/// `title` metadata field support shell-matched collection it got a match in.
// Go: resources/resource_metadata.go:assignMetadata
pub fn assign_metadata_to(metadata: &[Map], ma: &mut MetaResource) -> Result<()> {
    let mut counters: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    let mut name_set = false;
    let mut title_set = false;
    let mut name_counter: i64 = 0;
    let mut title_counter: i64 = 0;
    let mut name_counter_found = false;
    let mut title_counter_found = false;
    let resource_src_key = lower(&ma.name());

    for meta in metadata {
        let Some(src) = meta.get(b"src") else {
            return Err(Error::new("missing 'src' in metadata for resource"));
        };

        let src_key = lower(&cast_to_string(src));

        let glob = nh_common::glob::glob::get_glob(&src_key)
            .map_err(|e| e.wrap("failed to match resource with metadata"))?;

        let is_match = glob.matches(&resource_src_key);

        if is_match {
            if !name_set && let Some(name) = meta.get(b"name") {
                let name = cast_to_string(name);
                // Bundled resources in sub folders are relative paths with forward slashes.
                // Make sure any renames also matches that format:
                let name = nh_common::paths::path::trim_leading(go_path::filepath::to_slash(&name));
                if !name_counter_found {
                    name_counter_found = name.contains(COUNTER_PLACE_HOLDER);
                }
                if name_counter_found && name_counter == 0 {
                    let counter_key = format!("name_{src_key}");
                    name_counter = counters.get(&counter_key).copied().unwrap_or(0) + 1;
                    counters.insert(counter_key, name_counter);
                }

                ma.set_name(replace_resource_placeholders(&name, name_counter));
                name_set = true;
            }

            if !title_set && let Some(title) = meta.get(b"title") {
                let title = cast_to_string(title);
                if !title_counter_found {
                    title_counter_found = title.contains(COUNTER_PLACE_HOLDER);
                }
                if title_counter_found && title_counter == 0 {
                    let counter_key = format!("title_{src_key}");
                    title_counter = counters.get(&counter_key).copied().unwrap_or(0) + 1;
                    counters.insert(counter_key, title_counter);
                }
                ma.set_title(replace_resource_placeholders(&title, title_counter));
                title_set = true;
            }

            if let Some(params) = meta.get(b"params") {
                let mut m = nh_common::maps::maps::to_string_map(params);
                // Needed for case insensitive fetching of params values
                nh_common::maps::params::prepare_params(&mut m);
                ma.update_params(&m);
            }
        }
    }

    Ok(())
}

// Go: resources/resource_metadata.go:replaceResourcePlaceholders
fn replace_resource_placeholders(input: &str, counter: i64) -> String {
    input.replace(COUNTER_PLACE_HOLDER, &counter.to_string())
}

fn lower(s: &str) -> String {
    String::from_utf8_lossy(&go_unicode::strings::to_lower(s.as_bytes())).into_owned()
}

fn cast_to_string(v: &Value) -> String {
    let s: GoString = nh_common::cast::caste::to_string(v);
    String::from_utf8_lossy(s.as_bytes()).into_owned()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_metadata.go (224 lines; 0/10 funcs executed)
//   types: metaAssigner, mediaTypeAssigner, metaResource
// OK L65-67: (r *metaResource) Name() string
// OK L69-71: (r *metaResource) Title() string
// OK L73-75: (r *metaResource) Params() maps.Params
// OK L77-80: (r *metaResource) setTitle(title string)
// OK L82-85: (r *metaResource) setName(name string)
// OK L87-93: (r *metaResource) updateParams(params map[string]any)
// OK L96-118: cloneWithMetadataFromResourceConfigIfNeeded(rc *pagemeta.ResourceConfig, r resource.Resource) resource.Resource
// OK L121-139: CloneWithMetadataFromMapIfNeeded(m []map[string]any, r resource.Resource) resource.Resource
// OK L146-220: assignMetadata(metadata []map[string]any, ma *metaResource) error
// OK L222-224: replaceResourcePlaceholders(in string, counter int) string
// ---------------------------------------------------------------------------
