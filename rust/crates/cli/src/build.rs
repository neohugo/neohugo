//! `build`: flags → [`BuildRequest`] → [`neohugo_build::build`] → the report.

use std::time::Duration;

use neohugo_build::{BuildReport, BuildRequest, SinkKind};

use crate::Exit;
use crate::args::BuildArgs;
use crate::report;

/// The build request of the flags.
pub(crate) fn request(a: &BuildArgs) -> anyhow::Result<BuildRequest> {
    let p = &a.project;
    let mut cli = p.overrides();
    cli.minify = a.minify.then_some(true);
    Ok(BuildRequest {
        source: p.source_dir()?,
        destination: a.output.destination.clone(),
        config_files: p.config.clone(),
        cli,
        clock: p.clock,
        sink: if a.output.render_to_memory {
            SinkKind::Memory
        } else {
            SinkKind::Disk
        },
        clean_destination: a.output.clean_destination_dir,
        threads: a.threads,
        config: None,
        live_reload: None,
    })
}

pub(crate) fn run(a: &BuildArgs) -> anyhow::Result<Exit> {
    match neohugo_build::build(request(a)?) {
        Ok(r) => {
            report::diagnostics(&r.diagnostics);
            if !a.quiet {
                print_summary(&r);
                let total: Duration = r.timings.iter().map(|(_, d)| *d).sum();
                println!("Total in {} ms", total.as_millis());
            }
            Ok(Exit::Success)
        }
        Err(e) => {
            report::build_failed(&e);
            Ok(Exit::Failure)
        }
    }
}

/// What a build produced (`build`, and `server`'s first build).
pub(crate) fn print_summary(r: &BuildReport) {
    println!(
        "pages {} | files {} (aliases {}) | resources {} | processed images {} | static files {}",
        r.pages, r.outputs, r.aliases, r.resources, r.images, r.static_files
    );
    if !r.collisions.is_empty() {
        println!("target collisions {}", r.collisions.len());
    }
}
