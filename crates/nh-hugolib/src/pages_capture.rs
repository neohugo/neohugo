//! Port of `hugolib/pages_capture.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).

//! Go `hugolib/pages_capture.go`: walks the content component fs (ComponentFs order), applies
//! `IgnoreFile`, handles leaf bundles (`handleBundleLeaf`: other files become resources), branch
//! bundles, and calls `pageMap.AddFi` for each file.
//!
//! Go hands every file to a `rungroup` of `numWorkers` goroutines (`AddFi` in parallel); the
//! reference build runs with one worker, so files are added in enqueue order. The port adds
//! each file synchronously at the point Go enqueues it: the trees, the page arena and the
//! duplicate warnings come out in Go's single-worker order.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use nh_common::Result;
use nh_common::loggers::Logger;
use nh_common::paths::pathparser::{Path, modify_path_bundle_type_resource};
use nh_hugofs::afero::Fs;
use nh_hugofs::fileinfo::{FileMetaInfo, add_file_info_to_error};
use nh_hugofs::walk::{Walkway, WalkwayConfig, skip_dir};

use crate::content_map::add_fi;
use crate::hugo_sites::HugoSites;
use crate::hugo_sites_build::BuildCfg;

/// A filter of the files to collect (partial builds; `None` collects everything).
pub type InFilter<'f> = Option<&'f dyn Fn(&FileMetaInfo) -> bool>;

/// Go: `pagesCollector`.
pub struct PagesCollector<'a> {
    pub h: &'a mut HugoSites,
    /// Go `fs`: the content component fs (`sp.Content.Fs`).
    pub fs: Arc<dyn Fs>,
    pub logger: Logger,
    /// Go `m`: the page map that inserts (`h.Sites[0].pageMap`).
    pub site_idx: usize,
    pub build_config: BuildCfg,
    seen_dirs: BTreeSet<String>,
    /// Files, pages and resources processed (Go's counters, for the log).
    pub num_files_processed: u64,
    pub num_pages_processed: u64,
    pub num_resources_processed: u64,
}

/// Go: `newPagesCollector(ctx, h, sp, logger, infoLogger, m, buildConfig, ids)` (a full build:
/// no ids).
// Go: hugolib/pages_capture.go:newPagesCollector
pub fn new_pages_collector<'a>(
    h: &'a mut HugoSites,
    content_fs: Arc<dyn Fs>,
    logger: Logger,
    site_idx: usize,
    build_config: BuildCfg,
) -> PagesCollector<'a> {
    PagesCollector {
        h,
        fs: content_fs,
        logger,
        site_idx,
        build_config,
        seen_dirs: BTreeSet::new(),
        num_files_processed: 0,
        num_pages_processed: 0,
        num_resources_processed: 0,
    }
}

/// The state the walk hooks need (split from the collector's borrows).
struct Enqueuer<'a, 'h> {
    h: &'a mut HugoSites,
    site_idx: usize,
    build_config: &'a BuildCfg,
    counts: &'a mut (u64, u64, u64),
    _p: std::marker::PhantomData<&'h ()>,
}

impl Enqueuer<'_, '_> {
    /// Go: `c.g.Enqueue(fi)` + the rungroup handler (`c.m.AddFi`, errors wrapped with the file
    /// info).
    // Go: hugolib/pages_capture.go:Collect (the Handle func)
    fn enqueue(&mut self, fi: &FileMetaInfo) -> Result<()> {
        let (num_pages, num_resources) = add_fi(self.h, self.site_idx, fi, self.build_config)
            .map_err(|err| add_file_info_to_error(err, fi))?;
        self.counts.0 += 1;
        self.counts.1 += num_pages;
        self.counts.2 += num_resources;
        Ok(())
    }
}

impl PagesCollector<'_> {
    /// Collect collects content by walking the file system and storing it in the content tree.
    /// It may be restricted by filenames set on the collector (partial build: not ported).
    // Go: hugolib/pages_capture.go:Collect
    pub fn collect(&mut self) -> Result<()> {
        // Collect everything.
        self.collect_dir(None, false, None)
    }

    // Go: hugolib/pages_capture.go:collectDir
    fn collect_dir(
        &mut self,
        dir_path: Option<&Path>,
        is_dir: bool,
        in_filter: InFilter<'_>,
    ) -> Result<()> {
        let mut dpath = String::new();
        if let Some(dir_path) = dir_path {
            if is_dir {
                dpath = go_path::filepath::from_slash(dir_path.unnormalized().path()).to_string();
            } else {
                dpath = go_path::filepath::from_slash(dir_path.unnormalized().dir()).to_string();
            }
        }

        if self.seen_dirs.contains(&dpath) {
            return Ok(());
        }
        self.seen_dirs.insert(dpath.clone());

        let root = match self.fs.stat(&dpath) {
            Ok(root) => root,
            Err(err) => {
                if err.is_not_exist() {
                    return Ok(());
                }
                return Err(err);
            }
        };

        self.collect_dir_dir(&dpath, root, in_filter)
    }

    // Go: hugolib/pages_capture.go:collectDirDir
    fn collect_dir_dir(
        &mut self,
        path: &str,
        root: FileMetaInfo,
        in_filter: InFilter<'_>,
    ) -> Result<()> {
        let filter = |fim: &FileMetaInfo| match in_filter {
            Some(f) => f(fim),
            None => true,
        };

        let source_spec = self.h.deps.source_spec().clone();
        let ignore_file = move |filename: &str| source_spec.ignore_file(filename);
        let path_parser = self.h.deps.conf.path_parser();
        let print_path_warnings = self.h.configs.base.root.print_path_warnings;
        let is_rebuild = self.h.is_rebuild();
        let fs = self.fs.clone();
        let logger = self.logger.clone();

        let mut counts = (0u64, 0u64, 0u64);
        let result = {
            let mut enq = Enqueuer {
                h: self.h,
                site_idx: self.site_idx,
                build_config: &self.build_config,
                counts: &mut counts,
                _p: std::marker::PhantomData,
            };

            let mut pre_hook = |dir: &FileMetaInfo,
                                path: &str,
                                readdir: Vec<FileMetaInfo>|
             -> Result<Vec<FileMetaInfo>> {
                let readdir: Vec<FileMetaInfo> =
                    readdir.into_iter().filter(|fi| filter(fi)).collect();
                if readdir.is_empty() {
                    return Ok(Vec::new());
                }

                let mut kept = Vec::with_capacity(readdir.len());
                for fi in readdir {
                    if fi
                        .meta()
                        .path_info
                        .as_ref()
                        .is_some_and(|pi| pi.is_content_data())
                    {
                        // _content.json
                        // These are not part of any bundle, so just add them directly and
                        // remove them from the readdir slice.
                        enq.enqueue(&fi)?;
                    } else {
                        kept.push(fi);
                    }
                }
                let readdir = kept;

                // Pick the first regular file.
                let Some(first) = readdir.iter().find(|fi| !fi.is_dir()).cloned() else {
                    // Only dirs, keep walking.
                    return Ok(readdir);
                };

                // Any bundle file will always be first.
                let Some(first_pi) = first.meta().path_info.clone() else {
                    panic!(
                        "collectDirDir: no path info for {:?}",
                        first.meta().filename
                    );
                };

                if first_pi.is_leaf_bundle() {
                    handle_bundle_leaf(
                        &mut enq,
                        &fs,
                        &logger,
                        &ignore_file,
                        &path_parser,
                        dir,
                        &first,
                        path,
                        readdir,
                    )?;
                    return Err(skip_dir());
                }

                let mut seen: HashMap<(String, String), FileMetaInfo> = HashMap::new();
                for fi in &readdir {
                    if fi.is_dir() {
                        continue;
                    }

                    let meta = fi.meta();
                    let Some(pi) = &meta.path_info else {
                        panic!("no path info for {:?}", meta.filename);
                    };

                    // Filter out duplicate page or resource.
                    // These would eventually have been filtered out as duplicates when
                    // inserting them into the document store,
                    // but doing it here will preserve a consistent ordering.
                    let base_lang = (pi.base(), meta.lang.clone());
                    if let Some(fi2) = seen.get(&base_lang) {
                        if print_path_warnings && !is_rebuild {
                            logger.warnf(format!(
                                "Duplicate content path: {} file: {} file: {}",
                                go_strconv::quote(&base_lang.0),
                                go_strconv::quote(&fi2.meta().filename),
                                go_strconv::quote(&meta.filename)
                            ));
                        }
                        continue;
                    }
                    seen.insert(base_lang, fi.clone());

                    if meta.lang.is_empty() {
                        panic!("lang not set");
                    }

                    enq.enqueue(fi)?;
                }

                // Keep walking.
                Ok(readdir)
            };

            let mut wfn = |_path: &str, _fi: &FileMetaInfo| -> Result<()> { Ok(()) };

            let mut cfg = WalkwayConfig::new(self.fs.clone(), &mut wfn);
            cfg.logger = Some(self.logger.clone());
            cfg.root = path.to_string();
            cfg.info = Some(root);
            cfg.ignore_file = Some(&ignore_file);
            cfg.path_parser = Some(path_parser.clone());
            cfg.hook_pre = Some(&mut pre_hook);
            let mut w = Walkway::new(cfg);
            w.walk()
        };
        self.num_files_processed += counts.0;
        self.num_pages_processed += counts.1;
        self.num_resources_processed += counts.2;
        result
    }
}

// Go: hugolib/pages_capture.go:handleBundleLeaf
#[allow(clippy::too_many_arguments)]
fn handle_bundle_leaf(
    enq: &mut Enqueuer<'_, '_>,
    fs: &Arc<dyn Fs>,
    logger: &Logger,
    ignore_file: &dyn Fn(&str) -> bool,
    path_parser: &Arc<nh_common::paths::pathparser::PathParser>,
    dir: &FileMetaInfo,
    bundle: &FileMetaInfo,
    in_path: &str,
    readdir: Vec<FileMetaInfo>,
) -> Result<()> {
    let bundle_pi = bundle.meta().path_info.clone().expect("bundle path info");
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();

    let mut walk = |_path: &str, info: &FileMetaInfo| -> Result<()> {
        if info.is_dir() {
            return Ok(());
        }

        let mut info = info.clone();
        let mut pi: Path = info
            .meta()
            .path_info
            .as_deref()
            .cloned()
            .expect("path info");

        if info.meta().filename != bundle.meta().filename {
            // Everything inside a leaf bundle is a Resource,
            // even the content pages.
            // Note that we do allow index.md as page resources, but not in the bundle root.
            if !pi.is_leaf_bundle() || pi.dir() != bundle_pi.dir() {
                modify_path_bundle_type_resource(&mut pi);
                info.meta_mut().path_info = Some(Arc::new(pi.clone()));
            }
        }

        // Filter out duplicate page or resource.
        // These would eventually have been filtered out as duplicates when
        // inserting them into the document store,
        // but doing it here will preserve a consistent ordering.
        let base_lang = (pi.base(), info.meta().lang.clone());
        if seen.contains(&base_lang) {
            return Ok(());
        }
        seen.insert(base_lang);

        enq.enqueue(&info)
    };

    // Start a new walker from the given path.
    let mut cfg = WalkwayConfig::new(fs.clone(), &mut walk);
    cfg.root = in_path.to_string();
    cfg.logger = Some(logger.clone());
    cfg.info = Some(dir.clone());
    cfg.dir_entries = Some(readdir);
    cfg.ignore_file = Some(ignore_file);
    cfg.path_parser = Some(path_parser.clone());
    let mut w = Walkway::new(cfg);

    w.walk()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/pages_capture.go (425 lines; 5/5 funcs executed)
//   types: pagesCollector
// OK L38-60: newPagesCollector( ctx context.Context, h *HugoSites, sp *source.SourceSpec, logger loggers.Logger, infoLogger logg.LevelLogger, m *pageMap, buildC...
// OK L85-219: (c *pagesCollector) Collect() (collectErr error)  [full builds; the partial-build branch (ids) is not ported]
// OK L221-251: (c *pagesCollector) collectDir(dirPath *paths.Path, isDir bool, inFilter func(fim hugofs.FileMetaInfo) bool) error
// OK L253-376: (c *pagesCollector) collectDirDir(path string, root hugofs.FileMetaInfo, inFilter func(fim hugofs.FileMetaInfo) bool) error
// OK L378-425: (c *pagesCollector) handleBundleLeaf(dir, bundle hugofs.FileMetaInfo, inPath string, readdir []hugofs.FileMetaInfo) error
// ---------------------------------------------------------------------------
