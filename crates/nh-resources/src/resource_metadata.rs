//! Port of `resources/resource_metadata.go`.
//!
//! Owner: Wave B task T14 (resources-core).


use go_value::Map;

/// Go: `resources.metaResource` (front matter `resources` name/title/params overrides; `:counter`).
#[derive(Clone, Debug, Default)]
pub struct MetaResource {
    pub changed: bool,
    pub title: String,
    pub name: String,
    pub params: Option<Map>,
}

/// Go: `resources.AssignMetadata(metadata, resources...)`.
// Go: resources/resource_metadata.go:AssignMetadata
pub fn assign_metadata(metadata: &[Map], resources: &mut nh_resource::resourcetypes::Resources) -> nh_common::Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_metadata.go (224 lines; 0/10 funcs executed)
//   types: metaAssigner, mediaTypeAssigner, metaResource
//    L65-67: (r *metaResource) Name() string
//    L69-71: (r *metaResource) Title() string
//    L73-75: (r *metaResource) Params() maps.Params
//    L77-80: (r *metaResource) setTitle(title string)
//    L82-85: (r *metaResource) setName(name string)
//    L87-93: (r *metaResource) updateParams(params map[string]any)
//    L96-118: cloneWithMetadataFromResourceConfigIfNeeded(rc *pagemeta.ResourceConfig, r resource.Resource) resource.Resource
//    L121-139: CloneWithMetadataFromMapIfNeeded(m []map[string]any, r resource.Resource) resource.Resource
//    L146-220: assignMetadata(metadata []map[string]any, ma *metaResource) error
//    L222-224: replaceResourcePlaceholders(in string, counter int) string
// ---------------------------------------------------------------------------
