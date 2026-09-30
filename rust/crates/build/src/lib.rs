//! Build orchestration (REWRITE_PLAN.md §2.6, §3): configuration → file system → model →
//! layouts → render session → static copy → render waves → publisher.
//!
//! **State: T38 walking skeleton.** [`build`] runs phases A1–B2 (config, vfs, `load_model`),
//! B7 (layout scan), C1, D, E1, E2 (one sub-wave per language) and E3 (pagers, pager aliases,
//! the language redirect) and E4 on disk builds. Not yet: resources, images, the deferred wave,
//! URL-token publishing (T36). [`BuildRequest`], [`build`], [`BuildError`] and [`BuildReport`]
//! are the plan's (§2.6); fields are only added.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use neohugo_base::diag::Diagnostic;
use neohugo_base::paths::OutputPath;
use neohugo_base::{Clock, Idx, LangIdx, Sink};
use neohugo_config::{CliOverrides, ConfigError, LoadOptions};
use neohugo_layouts::{LayoutStore, TemplateError};
use neohugo_publish::{
    DiskSink, MemorySink, PublishError, PublishSettings, Publisher, StaticSyncOptions, sync_static,
};
use neohugo_render::{Job, JobOrder, Output, Project, RenderError, RenderOptions, Session};
use neohugo_site::{LoadModelOptions, ModelError};
use neohugo_vfs::{Vfs, VfsError};
use rayon::prelude::*;

/// Where the outputs go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SinkKind {
    /// The publish directory (`--destination`, else `publishDir`).
    #[default]
    Disk,
    /// Memory ([`BuildReport::memory`]): `serve` and tests.
    Memory,
}

/// What to build.
#[derive(Clone, Debug, Default)]
pub struct BuildRequest {
    /// The project directory.
    pub source: PathBuf,
    /// The publish directory (`--destination`); relative to `source`.
    pub destination: Option<PathBuf>,
    pub cli: CliOverrides,
    /// The build's "now" (`--clock`; `None`: the system clock).
    pub clock: Option<jiff::Timestamp>,
    pub sink: SinkKind,
    /// `--cleanDestinationDir`.
    pub clean_destination: bool,
}

/// Why a build failed.
#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Vfs(#[from] VfsError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Template(#[from] TemplateError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Publish(#[from] PublishError),
    #[error("{} errors", .0.len())]
    Diagnostics(Vec<Diagnostic>),
}

/// Two outputs for the same file: the later [`JobOrder`] wins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collision {
    pub path: OutputPath,
    pub winner: JobOrder,
    pub loser: JobOrder,
}

/// What a build did.
#[derive(Debug, Default)]
pub struct BuildReport {
    /// Pages of the model (all kinds, auto nodes included).
    pub pages: usize,
    /// Files published (static files not counted).
    pub outputs: usize,
    /// Alias files among them (front matter, pager, language redirect).
    pub aliases: usize,
    pub resources: usize,
    pub images: usize,
    pub collisions: Vec<Collision>,
    /// Sorted, de-duplicated warnings.
    pub diagnostics: Vec<Diagnostic>,
    pub timings: Vec<(&'static str, Duration)>,
    /// The files of a [`SinkKind::Memory`] build (static files included).
    pub memory: Option<Arc<MemorySink>>,
}

/// The process environment the configuration reads.
fn process_env() -> Vec<(String, String)> {
    std::env::vars()
        .filter(|(k, _)| {
            k.starts_with("HUGO_")
                || matches!(k.as_str(), "HOME" | "XDG_CACHE_HOME" | "TMPDIR" | "USER")
        })
        .collect()
}

/// Keeps the output of the later job for every path; reports the others.
fn resolve_collisions(outputs: Vec<Output>, collisions: &mut Vec<Collision>) -> Vec<Output> {
    let mut by_path: BTreeMap<OutputPath, Output> = BTreeMap::new();
    for o in outputs {
        match by_path.remove(&o.path) {
            None => {
                by_path.insert(o.path.clone(), o);
            }
            Some(prev) => {
                let (winner, loser) = if o.order >= prev.order {
                    (o, prev)
                } else {
                    (prev, o)
                };
                collisions.push(Collision {
                    path: winner.path.clone(),
                    winner: winner.order,
                    loser: loser.order,
                });
                by_path.insert(winner.path.clone(), winner);
            }
        }
    }
    by_path.into_values().collect()
}

fn is_alias(j: &Job) -> bool {
    matches!(
        j,
        Job::Alias(_) | Job::PagerAlias { .. } | Job::LanguageRedirect
    )
}

/// Renders `jobs` (in parallel) and publishes their outputs in [`JobOrder`].
fn wave(
    session: &Session,
    publisher: &Publisher,
    jobs: &[Job],
    report: &mut BuildReport,
) -> Result<(), BuildError> {
    let rendered: Vec<(bool, Vec<Output>)> = jobs
        .par_iter()
        .map(|j| session.render_job(j).map(|o| (is_alias(j), o)))
        .collect::<Result<_, _>>()?;
    let mut outputs = Vec::new();
    for (alias, outs) in rendered {
        report.aliases += usize::from(alias) * outs.len();
        outputs.extend(outs);
    }
    outputs.sort_by_key(|o| o.order);
    for o in resolve_collisions(outputs, &mut report.collisions) {
        publisher.emit(neohugo_publish::Output {
            path: o.path,
            text: o.text,
            format: o.format,
            lang: o.lang,
        })?;
        report.outputs += 1;
    }
    Ok(())
}

/// Builds a site (see the crate docs for the phases the skeleton runs).
///
/// # Errors
/// The first failing phase; [`BuildError::Diagnostics`] when a phase reported errors.
pub fn build(r: BuildRequest) -> Result<BuildReport, BuildError> {
    let mut report = BuildReport::default();
    let mut t = Instant::now();
    let mut lap = |report: &mut BuildReport, name: &'static str| {
        report.timings.push((name, t.elapsed()));
        t = Instant::now();
    };

    let mut cli = r.cli;
    if let Some(d) = &r.destination {
        cli.destination = Some(d.clone());
    }
    let cfg = Arc::new(neohugo_config::load(&LoadOptions {
        source: r.source.clone(),
        config_files: Vec::new(),
        cli,
        env: process_env(),
    })?);
    let vfs = Arc::new(Vfs::new(&cfg)?);
    let clock = r.clock.map_or_else(Clock::system, Clock);
    let model = neohugo_site::load_model(
        Arc::clone(&cfg),
        &vfs,
        &LoadModelOptions::from_config(&cfg, clock),
    )?;
    let model_diags = model.diagnostics.clone();
    lap(&mut report, "model");

    let layouts = Arc::new(LayoutStore::scan(&vfs, &cfg)?);
    let session = Session::new(
        Project {
            vfs: Arc::clone(&vfs),
            layouts,
        },
        Arc::new(model),
        &RenderOptions { clock },
    )?;
    for d in model_diags {
        session.diagnostics().push(d);
    }
    report.pages = session.model().pages.len();
    lap(&mut report, "templates");

    session.render_content()?;
    session.freeze_views()?;
    lap(&mut report, "content");

    let memory = (r.sink == SinkKind::Memory).then(|| Arc::new(MemorySink::new()));
    let sink: Arc<dyn Sink> = match &memory {
        Some(m) => Arc::clone(m) as Arc<dyn Sink>,
        None => {
            let publish = if cfg.dirs.publish.is_absolute() {
                cfg.dirs.publish.clone()
            } else {
                cfg.project_dir.join(&cfg.dirs.publish)
            };
            Arc::new(DiskSink::new(publish))
        }
    };
    sync_static(
        &vfs,
        sink.as_ref(),
        &StaticSyncOptions {
            clean_destination: r.clean_destination,
            ..StaticSyncOptions::default()
        },
    )?;
    let publisher = Publisher::new(
        PublishSettings::from_config(&cfg)?,
        Arc::clone(&sink),
        Arc::clone(session.diagnostics()),
    );
    lap(&mut report, "static");

    for lang in 0..cfg.sites.len() {
        let jobs = session.wave1(LangIdx::from_index(lang));
        wave(&session, &publisher, &jobs, &mut report)?;
    }
    lap(&mut report, "wave 1");
    let jobs = session.wave2();
    wave(&session, &publisher, &jobs, &mut report)?;
    lap(&mut report, "wave 2");

    if memory.is_none() && publisher.stats_enabled() {
        publisher
            .stats()
            .write_if_changed(&cfg.project_dir.join("hugo_stats.json"))
            .map_err(|source| PublishError::Io {
                path: cfg.project_dir.join("hugo_stats.json"),
                source,
            })?;
    }
    lap(&mut report, "stats");

    let diagnostics = session.diagnostics().report();
    if session.diagnostics().has_errors() {
        return Err(BuildError::Diagnostics(diagnostics));
    }
    report.diagnostics = diagnostics;
    report.memory = memory;
    Ok(report)
}
