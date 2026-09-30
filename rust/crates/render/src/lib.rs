//! The render session (REWRITE_PLAN.md §2.6, §3.2–3.3): the Tera instance with the template
//! functions, the content phase, the view freeze and the render jobs of phase E.
//!
//! **State: T38 walking skeleton.** Frozen here: [`Job`], [`JobOrder`], [`Output`] and
//! `Session::{new, render_content, freeze_views, render_job}`. The bodies are the skeleton's:
//! content without shortcodes or hooks, stub site functions ([`stubs`](self) module docs),
//! job planning in [`Session::wave1`] / [`Session::wave2`]. T34 (content), T35 (site
//! functions) and T36 (orchestration) replace them.

#![forbid(unsafe_code)]

mod content;
mod job;
mod session;
mod stubs;

use neohugo_base::{FormatId, PageId};

pub use job::{AliasPlan, Job, JobOrder, Output};
pub use session::{Project, RenderOptions, Session};

/// Why a render failed.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The interim model could not compute a target path.
    #[error(transparent)]
    Model(#[from] neohugo_view::interim::FlatError),
    /// The templates did not load.
    #[error(transparent)]
    Template(#[from] neohugo_layouts::TemplateError),
    /// Markdown failed.
    #[error("{}: {message}", file.display())]
    Markup {
        file: std::path::PathBuf,
        message: String,
    },
    /// A template failed.
    #[error("{template} ({page}): {source}")]
    Render {
        template: String,
        page: String,
        #[source]
        source: Box<tera::Error>,
    },
    /// A job for a format the page is not rendered in.
    #[error("page {page} has no output in format {format}")]
    NoOutput { page: PageId, format: FormatId },
    /// A phase called out of order.
    #[error("render phase out of order: {0}")]
    Phase(&'static str),
}
