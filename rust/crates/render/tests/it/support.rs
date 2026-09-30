//! Building a site into a render session for the suites.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use neohugo_base::{Clock, Idx, LangIdx, PageId};
use neohugo_config::LoadOptions;
use neohugo_layouts::LayoutStore;
use neohugo_render::{Project, RenderOptions, Session};
use neohugo_site::LoadModelOptions;
use neohugo_sitefuncs::Handles;
use neohugo_vfs::Vfs;
use neohugo_view::{HookVariant, Phase, RenderScope};

/// A site on disk and its session.
pub struct Site {
    /// Kept for the session's lifetime (the site's files).
    #[allow(dead_code)]
    pub dir: tempfile::TempDir,
    pub session: Arc<Session>,
}

/// Writes `files` (path → content) into a new directory.
pub fn write(files: &[(String, String)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (rel, text) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, text).expect("write");
    }
    dir
}

/// The session of the site in `dir`, with the site-function doubles and `extra`.
pub fn session_in(dir: &Path, extra: &dyn Fn(&mut tera::Tera, &Handles)) -> Arc<Session> {
    let cfg = Arc::new(
        neohugo_config::load(&LoadOptions {
            source: dir.to_path_buf(),
            config_files: Vec::new(),
            cli: neohugo_config::CliOverrides::default(),
            env: Vec::new(),
        })
        .expect("config"),
    );
    let vfs = Arc::new(Vfs::new(&cfg).expect("vfs"));
    let clock = Clock("2026-01-01T00:00:00Z".parse().expect("clock"));
    let model = neohugo_site::load_model(
        Arc::clone(&cfg),
        &vfs,
        &LoadModelOptions::from_config(&cfg, clock),
    )
    .expect("model");
    let layouts = Arc::new(LayoutStore::scan(&vfs, &cfg).expect("layouts"));
    Session::with_functions(
        Project { vfs, layouts },
        Arc::new(model),
        &RenderOptions { clock },
        &|t, h| {
            crate::fakes::register(t, h);
            extra(t, h);
        },
    )
    .expect("session")
}

/// A site from `files` with the doubles.
pub fn site(files: &[(&str, &str)]) -> Site {
    let owned: Vec<(String, String)> = files
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect();
    let dir = write(&owned);
    let session = session_in(dir.path(), &|_, _| {});
    Site { dir, session }
}

impl Site {
    /// The page with `.Path` `path` in language `lang` (0 = default).
    pub fn page(&self, path: &str, lang: usize) -> PageId {
        let m = self.session.model();
        m.pages
            .iter()
            .find(|p| p.path() == path && p.lang == LangIdx::from_index(lang))
            .unwrap_or_else(|| {
                let all: Vec<String> = m.pages.iter().map(neohugo_site::Page::path).collect();
                panic!("no page {path} in {all:?}")
            })
            .id
    }
}

/// A content-phase scope for page `p` (a C1 root).
pub fn content_scope(s: &Session, p: PageId, v: HookVariant) -> RenderScope {
    let page = &s.model().pages[p];
    let format = page
        .formats
        .first()
        .copied()
        .unwrap_or(neohugo_base::FormatId::from_raw(0));
    RenderScope {
        phase: Phase::Content,
        variant: v,
        ..RenderScope::layout(p, page.lang, format, None)
    }
}
