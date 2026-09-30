//! `execute_as_template`: a resource's content executed as a template, published at a target
//! path. The template engine is the render layer's; this module defines the seam.

use neohugo_base::ResourceId;
use serde_json::Value as Json;

use super::{PipeError, text};
use crate::store::{CallSite, ResourceError, ResourceStore};

/// Executes template source text (implemented by the render layer with Tera).
pub trait TemplateExecutor {
    /// Executes `source` as a template named `name` with `data` as its context.
    ///
    /// # Errors
    /// A template that does not parse or fails to execute (the message is reported).
    fn execute(&self, name: &str, source: &str, data: &Json) -> Result<String, String>;
}

impl ResourceStore {
    /// `resources.ExecuteAsTemplate`: the content of `id` executed with `data`, as a resource
    /// at `target` (a named target: see [`ResourceStore::from_template_output`] for its
    /// identity; the media type comes from the target).
    ///
    /// # Errors
    /// Content that is not UTF-8, a failing template, or a target conflict.
    pub fn execute_as_template(
        &self,
        id: ResourceId,
        target: &str,
        data: &Json,
        executor: &dyn TemplateExecutor,
        call: &CallSite,
    ) -> Result<ResourceId, ResourceError> {
        let r = self.resource(id);
        let fail = |e: PipeError| ResourceError::Pipe {
            resource: r.name.clone(),
            transform: "execute_as_template",
            source: Box::new(e),
        };
        let content = self.content(id)?;
        let source = text(&content).map_err(fail)?;
        let output = executor
            .execute(r.link.as_str().trim_start_matches('/'), source, data)
            .map_err(|e| fail(PipeError::Template(e)))?;
        self.from_template_output(target, output, call)
    }
}
