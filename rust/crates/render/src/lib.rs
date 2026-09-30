//! The render session (REWRITE_PLAN.md §2.6, §3.2–3.3): the Tera instance with the template
//! functions, the content phase, the view freeze and the render jobs of phase E.
//!
//! **State: T34.** The content engine is real: shortcodes (the private `shortcode` and
//! `tokens` modules), summaries ([`summary`]), render hooks through Tera, memo cells that never
//! block with cycle detection through the scope's chain, fragments as their own stage,
//! page-store writes buffered per computation and committed by the winner, one content variant
//! per hook format.
//! Frozen by T38: [`Job`], [`JobOrder`], [`Output`] and
//! `Session::{new, render_content, freeze_views, render_job}`. Still the skeleton's: the stub
//! site functions (`stubs`; T35) and job planning in [`Session::wave1`] / [`Session::wave2`]
//! (T36).

#![forbid(unsafe_code)]

mod content;
mod hooks;
mod job;
mod memo;
mod session;
mod shortcode;
mod stubs;
pub mod summary;
mod tokens;

use neohugo_base::{FormatId, PageId};

pub use job::{AliasPlan, Job, JobOrder, Output};
pub use session::{Project, RenderOptions, Session};

/// Why a render failed.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// A pager's file or link could not be made.
    #[error(transparent)]
    Target(Box<neohugo_view::TargetError>),
    /// The views could not be built.
    #[error(transparent)]
    View(Box<neohugo_view::ViewError>),
    /// The alias plan could not be made.
    #[error(transparent)]
    Nav(Box<neohugo_nav::NavError>),
    /// The templates did not load.
    #[error(transparent)]
    Template(#[from] neohugo_layouts::TemplateError),
    /// Markdown failed.
    #[error("{}: {message}", file.display())]
    Markup {
        file: std::path::PathBuf,
        message: String,
    },
    /// The content of a page failed (a shortcode, a hook, Markdown, a cycle).
    #[error("{page}: {source}")]
    Content {
        page: String,
        #[source]
        source: Box<neohugo_view::ContentError>,
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

impl From<neohugo_view::TargetError> for RenderError {
    fn from(e: neohugo_view::TargetError) -> Self {
        Self::Target(Box::new(e))
    }
}

impl From<neohugo_view::ViewError> for RenderError {
    fn from(e: neohugo_view::ViewError) -> Self {
        Self::View(Box::new(e))
    }
}

impl From<neohugo_nav::NavError> for RenderError {
    fn from(e: neohugo_nav::NavError) -> Self {
        Self::Nav(Box::new(e))
    }
}
