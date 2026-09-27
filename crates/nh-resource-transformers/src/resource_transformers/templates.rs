//! Port of `resources/resource_transformers/templates/execute_as_template.go`.
//!
//! Owner: Wave B task T15 (resource-factories).


//! Go `templates` (`resources.ExecuteAsTemplate targetPath data r`): text/template via
//! `TemplateStore.TextParse(InPath, content)`; OutPath = targetPath; the key IGNORES `data`
//! (cached by target path — first data wins).

use std::sync::Arc;

use go_value::Value;
use nh_common::Result;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;
use nh_tplimpl::templatestore::TemplateStore;

/// Go: `templates.Client`.
pub struct Client {
    pub rs: Arc<Spec>,
    pub t: TemplateStore,
}

impl Client {
    // Go: resources/resource_transformers/templates/execute_as_template.go:ExecuteAsTemplate
    pub fn execute_as_template(&self, ctx: &nh_tpl::template::TplContext, r: Arc<dyn Resource>, target_path: &str, data: Value) -> Result<Arc<dyn Resource>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/templates/execute_as_template.go (75 lines; 4/4 funcs executed)
//   types: Client, executeAsTemplateTransform
// EX L36-44: New(rs *resources.Spec, t tplimpl.TemplateStoreProvider) *Client
// EX L53-55: (t *executeAsTemplateTransform) Key() internal.ResourceTransformationKey
// EX L57-66: (t *executeAsTemplateTransform) Transform(ctx *resources.ResourceTransformationCtx) error
// EX L68-75: (c *Client) ExecuteAsTemplate(ctx context.Context, res resources.ResourceTransformer, targetPath string, data any) (resource.Resource, error)
// ---------------------------------------------------------------------------
