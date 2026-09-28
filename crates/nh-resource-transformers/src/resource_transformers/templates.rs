//! Port of `resources/resource_transformers/templates/execute_as_template.go`.
//!
//! Owner: Wave B task T15 (resource-factories).

//! Go `templates` (`resources.ExecuteAsTemplate targetPath data r`): text/template via
//! `TemplateStore.TextParse(InPath, content)`; OutPath = targetPath; the key IGNORES `data`
//! (cached by target path — first data wins).

use std::sync::Arc;

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;
use nh_resources::transform::{ResourceTransformation, ResourceTransformationCtx};
use nh_tplimpl::templatestore::TemplateStoreProvider;

/// Go: `tplimpl.TemplateStoreProvider` as the client holds it (Go's `*deps.Deps`: the store is
/// read when a transformation runs, after the site's store is set).
pub type StoreProvider = Arc<dyn TemplateStoreProvider + Send + Sync>;

/// Go: `templates.Client` — template processing of Resource objects.
pub struct Client {
    pub rs: Arc<Spec>,
    pub t: StoreProvider,
}

/// Go: `executeAsTemplateTransform`.
struct ExecuteAsTemplateTransform {
    t: StoreProvider,
    target_path: String,
    data: Value,
}

impl ResourceTransformation for ExecuteAsTemplateTransform {
    // Go: resources/resource_transformers/templates/execute_as_template.go:(*executeAsTemplateTransform).Key
    fn key(&self) -> ResourceTransformationKey {
        ResourceTransformationKey::new(
            "execute-as-template",
            vec![Value::string(self.target_path.as_str())],
        )
    }

    // Go: resources/resource_transformers/templates/execute_as_template.go:(*executeAsTemplateTransform).Transform
    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()> {
        let tpl_str = nh_helpers::general::reader_to_string(Some(&mut ctx.from));
        let th = self.t.get_template_store();
        let ti = th.text_parse_bytes(&ctx.in_path, &tpl_str).map_err(|err| {
            Error::new(format!(
                "failed to parse Resource {} as Template:: {}",
                go_strconv::quote(&ctx.in_path),
                err
            ))
        })?;
        ctx.out_path = self.target_path.clone();
        th.execute_with_context(ctx.ctx, &ti, ctx.to, &self.data)
    }
}

impl Client {
    /// Go: `New(rs, t)` (Go panics on a nil spec or provider; both are required here).
    // Go: resources/resource_transformers/templates/execute_as_template.go:New
    pub fn new(rs: Arc<Spec>, t: StoreProvider) -> Client {
        Client { rs, t }
    }

    /// `r` must be a `resources.ResourceTransformer` (a resource adapter).
    // Go: resources/resource_transformers/templates/execute_as_template.go:ExecuteAsTemplate
    pub fn execute_as_template(
        &self,
        ctx: &nh_tpl::template::TplContext,
        r: Arc<dyn Resource>,
        target_path: &str,
        data: Value,
    ) -> Result<Arc<dyn Resource>> {
        let res = super::integrity::transformer(&r)?;
        Ok(res.transform_with_context(
            ctx,
            vec![Arc::new(ExecuteAsTemplateTransform {
                target_path: nh_common::paths::path::to_slash_trim_leading(target_path),
                t: self.t.clone(),
                data,
            })],
        )?)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/templates/execute_as_template.go (75 lines; 4/4 funcs executed)
//   types: Client, executeAsTemplateTransform
// OK L36-44: New(rs *resources.Spec, t tplimpl.TemplateStoreProvider) *Client
// OK L53-55: (t *executeAsTemplateTransform) Key() internal.ResourceTransformationKey
// OK L57-66: (t *executeAsTemplateTransform) Transform(ctx *resources.ResourceTransformationCtx) error
// OK L68-75: (c *Client) ExecuteAsTemplate(ctx context.Context, res resources.ResourceTransformer, targetPath string, data any) (resource.Resource, error)
// ---------------------------------------------------------------------------
