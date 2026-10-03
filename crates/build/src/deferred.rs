//! Phases E4 and E5 (REWRITE_PLAN.md §3.1, §3.4): `build_stats.json`, then the deferred wave.
//!
//! - **E4.** With `[build.buildStats] enable`, the merged stats are written to the project
//!   directory (only when changed; external tools such as Tailwind read it
//!   from there, as with Go) and injected into the resource store at the asset path of
//!   every assets mount of that file (docs: `notwatching/build_stats.json`), so templates of
//!   E5 read this build's stats.
//! - **E5.** Every `defer(...)` key registered by waves 1 and 2 renders its template once (in
//!   parallel over keys); every post-process placeholder handed out so far is resolved (its
//!   pending transforms run now, after the stats exist); placeholders inside deferred output
//!   are resolved too. The replacement map goes to `Publisher::patch_held`, which re-extracts
//!   the URL tokens of the patched outputs, then minifies and writes them.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use rayon::prelude::*;
use ssg_publish::{PublishError, Publisher};
use ssg_render::{RenderError, Session};
use ssg_resources::PpField;
use ssg_vfs::{Component, Vfs};

use crate::{BuildError, RenderPool};

/// The asset paths `file` is mounted at.
fn asset_paths(vfs: &Vfs, file: &Path) -> Vec<String> {
    vfs.mounts_of(Component::Assets)
        .filter(|(_, m)| m.abs == file)
        .map(|(_, m)| {
            let sub = m.target_dir();
            if sub.is_empty() {
                file.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                sub.to_owned()
            }
        })
        .collect()
}

/// Phase E4.
pub(crate) fn write_stats(
    session: &Session,
    publisher: &Publisher,
    vfs: &Vfs,
) -> Result<(), BuildError> {
    if !publisher.stats_enabled() {
        return Ok(());
    }
    let stats = publisher.stats();
    let path = session
        .model()
        .config
        .project_dir
        .join(ssg_config::global::STATS_FILE);
    stats
        .write_if_changed(&path)
        .map_err(|source| PublishError::Io {
            path: path.clone(),
            source,
        })?;
    let bytes: Arc<[u8]> = stats.to_json().into_bytes().into();
    for asset in asset_paths(vfs, &path) {
        session
            .handles()
            .store
            .inject_generated(&asset, Arc::clone(&bytes));
    }
    Ok(())
}

/// Phase E5; returns the number of held outputs written.
pub(crate) fn run(
    session: &Session,
    publisher: &Publisher,
    pool: &RenderPool,
) -> Result<usize, BuildError> {
    let handles = session.handles();
    let entries = handles.deferred.entries();
    let rendered: Vec<Result<(String, String), RenderError>> = pool.run(|| {
        entries
            .par_iter()
            .map(|(key, d)| {
                session
                    .render_deferred(key, d)
                    .map(|text| (format!("__nh_defer_{key}__"), text))
            })
            .collect()
    });
    let store = &handles.store;
    let mut repl = BTreeMap::new();
    for r in rendered {
        let (placeholder, text) = r?;
        let text = store.resolve_post_process(&text)?.unwrap_or(text);
        repl.insert(placeholder, text);
    }
    for pp in store.post_processes() {
        for field in PpField::ALL {
            let placeholder = pp.placeholder(field);
            let value = store
                .resolve_post_process(&placeholder)?
                .unwrap_or_default();
            repl.insert(placeholder, value);
        }
    }
    if publisher.held().is_empty() {
        return Ok(0);
    }
    Ok(pool.run(|| publisher.patch_held(&repl))?)
}
