//! Port of `hugofs/rootmapping_fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

//! Go `hugofs.RootMappingFs`: a virtual fs built from module mounts (`From` = component target
//! path, `To` = real source dir). Earlier mounts get higher `Weight` and win.
//!
//! Go keys two `armon/go-radix` trees. The radix operations used here (`Get`, `Insert`,
//! `LongestPrefix`, `WalkPrefix`, `WalkPath`, `Walk`) are byte-prefix operations visiting keys in
//! byte order, which a `BTreeMap<String, _>` reproduces exactly (see [`longest_prefix`],
//! [`walk_prefix`], [`walk_path`]).

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Weak};

use nh_common::Result;
use nh_common::glob::filename_filter::{self, FilenameFilter};
use nh_common::herrors::Error;

use crate::afero::{File, Fs, no_regular_file_ops};
use crate::decorators::decorate_dirs;
use crate::fileinfo::{
    FileMeta, FileMetaInfo, OpenFunc, decorate_file_info, new_dir_name_only_file_info,
    new_file_meta_info_from,
};
use crate::filename_filter_fs::new_filename_filter_fs;
use crate::fs::new_base_path_fs;
use crate::oserror;
use crate::overlayfs;

const FILEPATH_SEPARATOR: &str = "/";

/// Go: `errIsDir` — the sentinel error of `rootMappingDir.Stat` (the file is a directory).
pub(crate) fn err_is_dir() -> Error {
    Error::new("isDir")
}

/// Whether `e` is [`err_is_dir`] (Go: `err == errIsDir`).
pub(crate) fn is_err_is_dir(e: &Error) -> bool {
    e.kind() == nh_common::herrors::ErrorKind::Generic && e.message() == "isDir"
}

/// Go: `hugofs.RootMapping`.
#[derive(Clone, Default)]
pub struct RootMapping {
    /// The virtual mount point, e.g. `content`, `assets/vendor`.
    pub from: String,
    /// The base directory of the virtual mount (the component).
    pub from_base: String,
    /// Real source dir or file, e.g. `<site>/node_modules`.
    pub to: String,
    /// The base of `to`. May be empty if an absolute path was provided.
    pub to_base: String,
    /// The module path/ID.
    pub module: String,
    /// The module ordinal starting with 0 which is the project.
    pub module_ordinal: i64,
    /// Whether this is a mount in the main project.
    pub is_project: bool,
    /// File metadata (lang etc.). Go: `*FileMeta` (nil = a new one).
    pub meta: Arc<FileMeta>,

    pub(crate) fi: Option<FileMetaInfo>,
    /// Also set when this mounts represents a single file with a rename func.
    pub(crate) fi_single_file: Option<FileMetaInfo>,
    /// The virtual mount point, e.g. "blog".
    pub(crate) path: String,
}

impl RootMapping {
    /// Go: `RootMapping{From: from, To: to, Meta: meta}` (the other exported fields are public;
    /// the unexported ones are set by `NewRootMappingFs`).
    pub fn new(from: &str, to: &str, meta: Arc<FileMeta>) -> RootMapping {
        RootMapping {
            from: from.to_string(),
            to: to.to_string(),
            meta,
            ..Default::default()
        }
    }

    // Go: hugofs/rootmapping_fs.go:clean
    fn clean(&mut self) {
        self.from = go_path::filepath::clean(&self.from)
            .trim_matches('/')
            .to_string();
        self.to = go_path::filepath::clean(&self.to);
    }

    // Go: hugofs/rootmapping_fs.go:filename
    fn filename(&self, name: &str) -> String {
        if name.is_empty() {
            return self.to.clone();
        }
        let rest = name.strip_prefix(self.from.as_str()).unwrap_or(name);
        go_path::filepath::join(&[self.to.as_str(), rest])
    }

    // Go: hugofs/rootmapping_fs.go:trimFrom
    fn trim_from(&self, name: &str) -> String {
        if name.is_empty() {
            return String::new();
        }
        name.strip_prefix(self.from.as_str())
            .unwrap_or(name)
            .to_string()
    }

    /// Go's `==` on `RootMapping` values (the `seen` map of `getRoots`): distinct mappings never
    /// share their `*FileMeta`, so pointer identity decides.
    fn same(&self, other: &RootMapping) -> bool {
        Arc::ptr_eq(&self.meta, &other.meta) && self.from == other.from && self.to == other.to
    }
}

/// Go: `hugofs.ComponentPath`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComponentPath {
    pub component: String,
    pub path: String,
    pub lang: String,
    pub watch: bool,
}

impl ComponentPath {
    // Go: hugofs/rootmapping_fs.go:ComponentPathJoined
    pub fn component_path_joined(&self) -> String {
        go_path::path::join(&[self.component.as_str(), self.path.as_str()])
    }
}

/// Go: `hugofs.ReverseLookupProvder`.
pub trait ReverseLookupProvider {
    fn reverse_lookup(&self, filename: &str) -> Result<Vec<ComponentPath>>;
    fn reverse_lookup_component(
        &self,
        component: &str,
        filename: &str,
    ) -> Result<Vec<ComponentPath>>;
}

/// Go: `keyRootMappings`.
struct KeyRootMappings {
    key: String,
    roots: Vec<RootMapping>,
}

type Tree = BTreeMap<String, Vec<RootMapping>>;

/// Go: `radix.Tree.LongestPrefix(key)` — the longest key that is a byte prefix of `key`.
fn longest_prefix<'a>(tree: &'a Tree, key: &str) -> Option<(&'a str, &'a Vec<RootMapping>)> {
    for i in (0..=key.len()).rev() {
        if !key.is_char_boundary(i) {
            continue;
        }
        if let Some((k, v)) = tree.get_key_value(&key[..i]) {
            return Some((k.as_str(), v));
        }
    }
    None
}

/// Go: `radix.Tree.WalkPrefix(prefix, fn)` — every key with the byte prefix, in order.
fn walk_prefix<'a>(
    tree: &'a Tree,
    prefix: &'a str,
) -> impl Iterator<Item = (&'a String, &'a Vec<RootMapping>)> + 'a {
    tree.range::<str, _>((
        std::ops::Bound::Included(prefix),
        std::ops::Bound::Unbounded,
    ))
    .take_while(move |(k, _)| k.starts_with(prefix))
}

/// Go: `radix.Tree.WalkPath(path, fn)` — every key that is a byte prefix of `path`, shortest
/// first.
fn walk_path<'a>(tree: &'a Tree, path: &str) -> Vec<(&'a String, &'a Vec<RootMapping>)> {
    let mut out = Vec::new();
    for i in 0..=path.len() {
        if !path.is_char_boundary(i) {
            continue;
        }
        if let Some(kv) = tree.get_key_value(&path[..i]) {
            out.push(kv);
        }
    }
    out
}

static ROOT_MAPPING_FS_COUNTER: AtomicI32 = AtomicI32::new(0);

/// Go: `hugofs.RootMappingFs` — maps several roots into one. Note that the root of this
/// filesystem is directories only, and they will be returned in Readdir and Readdirnames in the
/// order given.
pub struct RootMappingFs {
    pub id: String,
    pub fs: Arc<dyn Fs>,
    pub(crate) root_map_to_real: BTreeMap<String, Vec<RootMapping>>,
    pub(crate) real_map_to_root: BTreeMap<String, Vec<RootMapping>>,
    self_ref: Weak<RootMappingFs>,
}

// Go: hugofs/rootmapping_fs.go:NewRootMappingFs (addMapping)
fn add_mapping(key: String, rm: RootMapping, to: &mut Tree) {
    // There may be more than one language pointing to the same root.
    to.entry(key).or_default().push(rm);
}

impl RootMappingFs {
    /// Go: `hugofs.NewRootMappingFs(fs, rms...)`.
    // Go: hugofs/rootmapping_fs.go:NewRootMappingFs
    pub fn new(fs: Arc<dyn Fs>, rms: Vec<RootMapping>) -> Result<Arc<RootMappingFs>> {
        let mut root_map_to_real = Tree::new();
        let mut real_map_to_root = Tree::new();
        let id = format!(
            "rfs-{}",
            ROOT_MAPPING_FS_COUNTER.fetch_add(1, Ordering::SeqCst) + 1
        );

        for mut rm in rms {
            rm.clean();

            rm.from_base = nh_common::files::resolve_component_folder(&rm.from).to_string();

            if rm.to.len() < 2 {
                // Go panics.
                return Err(Error::new(format!(
                    "invalid root mapping; from/to: {}/{}",
                    rm.from, rm.to
                )));
            }

            let mut fi = match fs.stat(&rm.to) {
                Ok(fi) => fi,
                Err(e) if e.is_not_exist() => continue,
                Err(e) => return Err(e),
            };

            if rm.from_base.is_empty() {
                // Go panics.
                return Err(Error::new(" rm.FromBase is empty"));
            }

            {
                let meta = Arc::make_mut(&mut rm.meta);
                meta.component = rm.from_base.clone();
                meta.module = rm.module.clone();
                meta.module_ordinal = rm.module_ordinal;
                meta.is_project = rm.is_project;
                meta.base_dir = rm.to_base.clone();
            }

            if !fi.is_dir() {
                // We do allow single file mounts.
                // However, the file system logic will be much simpler with just directories.
                // So, convert this mount into a directory mount with a renamer,
                // which will tell the caller if name should be included.
                let (dir_from, name_from) = go_path::filepath::split(&rm.from);
                let (dir_to, name_to) = go_path::filepath::split(&rm.to);
                let dir_from = dir_from
                    .strip_suffix(FILEPATH_SEPARATOR)
                    .unwrap_or(dir_from)
                    .to_string();
                let dir_to = dir_to
                    .strip_suffix(FILEPATH_SEPARATOR)
                    .unwrap_or(dir_to)
                    .to_string();
                let (name_from, name_to) = (name_from.to_string(), name_to.to_string());
                rm.from = dir_from;
                let mut single_file_meta = FileMeta::copy(Some(&rm.meta));
                single_file_meta.name = name_from.clone();
                rm.fi_single_file = Some(new_file_meta_info_from(&fi, single_file_meta));
                rm.to = dir_to;

                let name_to_filename = format!("{FILEPATH_SEPARATOR}{name_to}");
                {
                    let meta = Arc::make_mut(&mut rm.meta);
                    let (nf, nt) = (name_from.clone(), name_to.clone());
                    meta.rename = Some(Arc::new(move |name: &str, to_from: bool| {
                        if to_from {
                            if name == nt {
                                return (nf.clone(), true);
                            }
                            return (String::new(), false);
                        }

                        if name == nf {
                            return (nt.clone(), true);
                        }

                        (String::new(), false)
                    }));

                    meta.inclusion_filter = filename_filter::append(
                        meta.inclusion_filter.as_ref(),
                        Some(FilenameFilter::new_for_inclusion_func(
                            move |filename: &str| name_to_filename == filename,
                        )),
                    );
                }

                // Refresh the FileInfo object.
                fi = match fs.stat(&rm.to) {
                    Ok(fi) => fi,
                    Err(e) if e.is_not_exist() => continue,
                    Err(e) => return Err(e),
                };
            }

            // Extract "blog" from "content/blog"
            rm.path = rm
                .from
                .strip_prefix(rm.from_base.as_str())
                .unwrap_or(&rm.from)
                .trim_start_matches_once(FILEPATH_SEPARATOR)
                .to_string();
            Arc::make_mut(&mut rm.meta).source_root = fi.meta().filename.clone();

            let mut meta = FileMeta::copy(Some(&rm.meta));

            if !fi.is_dir() {
                let (_, name) = go_path::filepath::split(&rm.from);
                meta.name = name.to_string();
            }

            rm.fi = Some(new_file_meta_info_from(&fi, meta));

            add_mapping(
                format!("{FILEPATH_SEPARATOR}{}", rm.from),
                rm.clone(),
                &mut root_map_to_real,
            );
            let mut rev = rm.to.clone();
            if !rev.starts_with(FILEPATH_SEPARATOR) {
                rev = format!("{FILEPATH_SEPARATOR}{rev}");
            }

            add_mapping(rev, rm, &mut real_map_to_root);
        }

        Ok(Arc::new_cyclic(|w| RootMappingFs {
            id,
            fs,
            root_map_to_real,
            real_map_to_root,
            self_ref: w.clone(),
        }))
    }

    /// Go: `newRootMappingFsFromFromTo(baseDir, fs, fromTo...)`.
    // Go: hugofs/rootmapping_fs.go:newRootMappingFsFromFromTo
    pub fn new_from_from_to(
        base_dir: &str,
        fs: Arc<dyn Fs>,
        from_to: &[&str],
    ) -> Result<Arc<RootMappingFs>> {
        let mut rms = Vec::with_capacity(from_to.len() / 2);
        let mut j = 0;
        while j + 1 < from_to.len() {
            rms.push(RootMapping {
                from: from_to[j].to_string(),
                to: from_to[j + 1].to_string(),
                to_base: base_dir.to_string(),
                ..Default::default()
            });
            j += 2;
        }
        RootMappingFs::new(fs, rms)
    }

    fn arc(&self) -> Arc<RootMappingFs> {
        self.self_ref
            .upgrade()
            .expect("RootMappingFs used after drop")
    }

    /// Go: `Mounts(base)` — the mounts at a virtual path.
    // Go: hugofs/rootmapping_fs.go:Mounts
    pub fn mounts(&self, base: &str) -> Result<Vec<FileMetaInfo>> {
        let base = format!("{FILEPATH_SEPARATOR}{}", self.clean_name(base));
        let roots = self.get_roots_with_prefix(&base);

        if roots.is_empty() {
            return Ok(Vec::new());
        }

        let mut fss = Vec::with_capacity(roots.len());
        for r in roots {
            if let Some(fi) = &r.fi_single_file {
                // A single file mount.
                fss.push(fi.clone());
                continue;
            }
            let bfs = new_base_path_fs(self.fs.clone(), &r.to);
            let mut fs = bfs;
            if let Some(filter) = &r.meta.inclusion_filter {
                fs = new_filename_filter_fs(fs, &r.to, filter.clone());
            }
            let fs = decorate_dirs(fs, r.meta.clone());
            let fi = match fs.stat("") {
                Ok(fi) => fi,
                Err(_) => continue,
            };
            fss.push(fi);
        }

        Ok(fss)
    }

    // Go: hugofs/rootmapping_fs.go:Key
    pub fn key(&self) -> &str {
        &self.id
    }

    /// Go: `Filter(f)` — a copy of this filesystem with only the mappings matching a filter.
    // Go: hugofs/rootmapping_fs.go:Filter
    pub fn filter(&self, f: &dyn Fn(&RootMapping) -> bool) -> Arc<RootMappingFs> {
        let mut root_map_to_real = Tree::new();
        for (b, rms) in &self.root_map_to_real {
            let nrms: Vec<RootMapping> = rms.iter().filter(|rm| f(rm)).cloned().collect();
            if !nrms.is_empty() {
                root_map_to_real.insert(b.clone(), nrms);
            }
        }

        Arc::new_cyclic(|w| RootMappingFs {
            id: self.id.clone(),
            fs: self.fs.clone(),
            root_map_to_real,
            real_map_to_root: self.real_map_to_root.clone(),
            self_ref: w.clone(),
        })
    }

    /// Go: `ReverseLookupComponent(component, filename)` — real filename -> component paths
    /// (e.g. `<site>/node_modules/bootstrap/scss` -> `vendor/bootstrap/scss` in assets).
    // Go: hugofs/rootmapping_fs.go:ReverseLookupComponent
    pub fn reverse_lookup_component(
        &self,
        component: &str,
        filename: &str,
    ) -> Result<Vec<ComponentPath>> {
        let filename = self.clean_name(filename);
        let key = format!("{FILEPATH_SEPARATOR}{filename}");

        let (s, roots) = self.get_roots_reverse(&key);

        if roots.is_empty() {
            return Ok(Vec::new());
        }

        let mut cps = Vec::new();

        let base = key.strip_prefix(s.as_str()).unwrap_or(&key).to_string();
        let (dir, name) = go_path::filepath::split(&base);

        for first in roots {
            if !component.is_empty() && first.from_base != component {
                continue;
            }

            let filename = if let Some(rename) = &first.meta.rename {
                // Single file mount.
                let (newname, ok) = rename(name, true);
                if ok {
                    format!(
                        "{FILEPATH_SEPARATOR}{}",
                        go_path::filepath::join(&[first.path.as_str(), dir, newname.as_str()])
                    )
                } else {
                    continue;
                }
            } else {
                // Now we know that this file _could_ be in this fs.
                format!(
                    "{FILEPATH_SEPARATOR}{}",
                    go_path::filepath::join(&[first.path.as_str(), dir, name])
                )
            };

            cps.push(ComponentPath {
                component: first.from_base.clone(),
                path: nh_common::paths::path::to_slash_trim_leading(&filename),
                lang: first.meta.lang.clone(),
                watch: first.meta.watch,
            });
        }

        Ok(cps)
    }

    /// Go: `ReverseLookup(filename)`.
    // Go: hugofs/rootmapping_fs.go:ReverseLookup
    pub fn reverse_lookup(&self, filename: &str) -> Result<Vec<ComponentPath>> {
        self.reverse_lookup_component("", filename)
    }

    // Go: hugofs/rootmapping_fs.go:hasPrefix
    fn has_prefix(&self, prefix: &str) -> bool {
        walk_prefix(&self.root_map_to_real, prefix).next().is_some()
    }

    // Go: hugofs/rootmapping_fs.go:getRoot
    fn get_root(&self, key: &str) -> Option<&Vec<RootMapping>> {
        self.root_map_to_real.get(key)
    }

    // Go: hugofs/rootmapping_fs.go:getRoots
    fn get_roots(&self, key: &str) -> (String, Vec<RootMapping>) {
        let tree = &self.root_map_to_real;
        let levels = key.matches(FILEPATH_SEPARATOR).count();
        let mut seen: Vec<RootMapping> = Vec::new();

        let mut roots: Vec<RootMapping> = Vec::new();
        let mut s = String::new();
        let mut key = key.to_string();

        loop {
            let Some((ss, vv)) = longest_prefix(tree, &key) else {
                break;
            };
            if levels < 2 && ss == key {
                break;
            }

            for rm in vv {
                if !seen.iter().any(|x| x.same(rm)) {
                    seen.push(rm.clone());
                    roots.push(rm.clone());
                }
            }
            s = ss.to_string();

            // We may have more than one root for this key, so walk up.
            let old_key = key.clone();
            key = go_path::filepath::dir(&key);
            if key == old_key {
                break;
            }
        }

        (s, roots)
    }

    // Go: hugofs/rootmapping_fs.go:getRootsReverse
    fn get_roots_reverse(&self, key: &str) -> (String, Vec<RootMapping>) {
        match longest_prefix(&self.real_map_to_root, key) {
            None => (String::new(), Vec::new()),
            Some((s, v)) => (s.to_string(), v.clone()),
        }
    }

    // Go: hugofs/rootmapping_fs.go:getRootsWithPrefix
    fn get_roots_with_prefix(&self, prefix: &str) -> Vec<RootMapping> {
        let mut roots = Vec::new();
        for (_, v) in walk_prefix(&self.root_map_to_real, prefix) {
            roots.extend(v.iter().cloned());
        }
        roots
    }

    // Go: hugofs/rootmapping_fs.go:getAncestors
    fn get_ancestors(&self, prefix: &str) -> Vec<KeyRootMappings> {
        let mut roots = Vec::new();
        for (s, v) in walk_path(&self.root_map_to_real, prefix) {
            if prefix.starts_with(&format!("{s}{FILEPATH_SEPARATOR}")) {
                roots.push(KeyRootMappings {
                    key: s.clone(),
                    roots: v.clone(),
                });
            }
        }
        roots
    }

    // Go: hugofs/rootmapping_fs.go:newUnionFile
    fn new_union_file(&self, fis: Vec<FileMetaInfo>) -> Result<Box<dyn File>> {
        if fis.len() == 1 {
            return fis[0].meta().open();
        }

        if !fis[0].is_dir() {
            // Pick the last file mount.
            return fis[fis.len() - 1].meta().open();
        }

        let mut openers: Vec<OpenFunc> = Vec::with_capacity(fis.len());
        for fi in fis.iter() {
            let fs = self.arc();
            let meta = fi.meta.clone();
            openers.push(Arc::new(move || {
                let f = meta.open()?;
                Ok(Box::new(RootMappingDir {
                    dir_only_ops: Some(f),
                    fs: fs.clone(),
                    name: meta.name.clone(),
                    meta: Some(meta.clone()),
                }) as Box<dyn File>)
            }));
        }

        let merge: crate::dirsmerger::DirsMerger =
            Arc::new(|mut lofi: Vec<FileMetaInfo>, bofi: Vec<FileMetaInfo>| {
                // Ignore duplicate directory entries
                for fi1 in bofi {
                    let mut found = false;
                    for fi2 in &lofi {
                        if !fi2.is_dir() {
                            continue;
                        }
                        if fi1.name() == fi2.name() {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        lofi.push(fi1);
                    }
                }

                lofi
            });

        let first = fis[0].clone();
        let info: overlayfs::DirInfoFunc = Arc::new(move || Ok(first.clone()));

        Ok(Box::new(overlayfs::open_dir(Some(merge), info, openers)?))
    }

    // Go: hugofs/rootmapping_fs.go:cleanName
    fn clean_name(&self, name: &str) -> String {
        let name = go_path::filepath::clean(name).trim_matches('/').to_string();
        if name == "." {
            return String::new();
        }
        name
    }

    // Go: hugofs/rootmapping_fs.go:collectDirEntries
    fn collect_dir_entries(&self, prefix: &str) -> Result<Vec<FileMetaInfo>> {
        let prefix = format!("{FILEPATH_SEPARATOR}{}", self.clean_name(prefix));

        let mut fis: Vec<FileMetaInfo> = Vec::new();

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new(); // Prevent duplicate directories
        let level = prefix.matches(FILEPATH_SEPARATOR).count();

        let collect_dir = |rm: &RootMapping,
                           fi: &FileMetaInfo,
                           fis: &mut Vec<FileMetaInfo>,
                           seen: &mut std::collections::HashSet<String>|
         -> Result<()> {
            let mut f = fi.meta().open()?;
            let direntries = match f.read_dir(-1) {
                Ok(d) => d,
                Err(e) => {
                    let _ = f.close();
                    return Err(e);
                }
            };

            for mut fi in direntries {
                fi.meta_mut().merge(&rm.meta);
                let meta = fi.meta();

                let rel = meta
                    .filename
                    .strip_prefix(meta.source_root.as_str())
                    .unwrap_or(&meta.filename)
                    .to_string();
                if !filename_filter::match_opt(rm.meta.inclusion_filter.as_ref(), &rel, fi.is_dir())
                {
                    continue;
                }

                if fi.is_dir() {
                    let name = fi.name().to_string();
                    if seen.contains(&name) {
                        continue;
                    }
                    seen.insert(name.clone());
                    let rfs = self.arc();
                    let target = go_path::filepath::join(&[rm.from.as_str(), name.as_str()]);
                    let opener: OpenFunc = Arc::new(move || rfs.open_impl(&target));
                    fi = new_dir_name_only_file_info(&name, Some(fi.meta()), opener);
                } else if let Some(rename) = &rm.meta.rename {
                    let (n, ok) = rename(fi.name(), true);
                    if !ok {
                        continue;
                    }
                    fi.meta_mut().name = n;
                }
                fis.push(fi);
            }

            let _ = f.close();

            Ok(())
        };

        // First add any real files/directories.
        if let Some(rms) = self.get_root(&prefix) {
            for rm in rms {
                let rmfi = rm.fi.as_ref().expect("root mapping without file info");
                collect_dir(rm, rmfi, &mut fis, &mut seen)?;
            }
        }

        // Next add any file mounts inside the given directory.
        let prefix_inside = format!("{prefix}{FILEPATH_SEPARATOR}");
        for (s, v) in walk_prefix(&self.root_map_to_real, &prefix_inside) {
            if s.matches(FILEPATH_SEPARATOR).count() as isize - level as isize != 1 {
                // This directory is not part of the current, but we
                // need to include the first name part to make it
                // navigable.
                let path = s
                    .strip_prefix(prefix_inside.as_str())
                    .unwrap_or(s)
                    .to_string();
                let name = path
                    .split(FILEPATH_SEPARATOR)
                    .next()
                    .unwrap_or("")
                    .to_string();

                if seen.contains(&name) {
                    continue;
                }
                seen.insert(name.clone());
                let rfs = self.arc();
                let opener: OpenFunc = Arc::new(move || rfs.open_impl(&path));

                let fi = new_dir_name_only_file_info(&name, None, opener);
                fis.push(fi);

                continue;
            }

            for rm in v {
                let name = go_path::filepath::base(&rm.from).to_string();
                if seen.contains(&name) {
                    continue;
                }
                seen.insert(name.clone());
                let rfs = self.arc();
                let from = rm.from.clone();
                let opener: OpenFunc = Arc::new(move || rfs.open_impl(&from));
                let fi = new_dir_name_only_file_info(&name, Some(&rm.meta), opener);
                fis.push(fi);
            }
        }

        // Finally add any ancestor dirs with files in this directory.
        let ancestors = self.get_ancestors(&prefix);
        for root in ancestors {
            let subdir = prefix
                .strip_prefix(root.key.as_str())
                .unwrap_or(&prefix)
                .to_string();
            for rm in &root.roots {
                let rmfi = rm.fi.as_ref().expect("root mapping without file info");
                if rmfi.is_dir()
                    && let Ok(fi) = rmfi.meta().join_stat(&subdir)
                {
                    collect_dir(rm, &fi, &mut fis, &mut seen)?;
                }
            }
        }

        Ok(fis)
    }

    // Go: hugofs/rootmapping_fs.go:doStat
    fn do_stat(&self, name: &str) -> Result<Vec<FileMetaInfo>> {
        let fis = self.do_do_stat(name)?;
        // Sanity check. Check that all is either file or directories.
        let mut is_dir = false;
        let mut is_file = false;
        for fi in &fis {
            if fi.is_dir() {
                is_dir = true;
            } else {
                is_file = true;
            }
        }
        if is_dir && is_file {
            // For now.
            return Err(oserror::err_not_exist());
        }

        Ok(fis)
    }

    // Go: hugofs/rootmapping_fs.go:doDoStat
    fn do_do_stat(&self, name: &str) -> Result<Vec<FileMetaInfo>> {
        let name = self.clean_name(name);
        let key = format!("{FILEPATH_SEPARATOR}{name}");

        let roots = self.get_root(&key);

        let Some(roots) = roots else {
            if self.has_prefix(&key) {
                // We have directories mounted below this.
                // Make it look like a directory.
                return Ok(vec![new_dir_name_only_file_info(
                    &name,
                    None,
                    self.virtual_dir_opener(&name),
                )]);
            }

            // Find any real directories with this key.
            let (_, roots) = self.get_roots(&key);
            if roots.is_empty() {
                return Err(oserror::path_error(
                    "LStat",
                    &name,
                    &oserror::err_not_exist(),
                ));
            }

            let mut err: Option<Error> = None;
            let mut fis: Vec<FileMetaInfo> = Vec::new();

            for rm in &roots {
                match self.stat_root(rm, &name) {
                    Ok(fi) => {
                        err = None;
                        fis.push(fi);
                    }
                    Err(e) => err = Some(e),
                }
            }

            if !fis.is_empty() {
                return Ok(fis);
            }

            return Err(err.unwrap_or_else(oserror::err_not_exist));
        };

        Ok(vec![new_dir_name_only_file_info(
            &name,
            Some(&roots[0].meta),
            self.virtual_dir_opener(&name),
        )])
    }

    // Go: hugofs/rootmapping_fs.go:statRoot
    fn stat_root(&self, root: &RootMapping, filename: &str) -> Result<FileMetaInfo> {
        let mut filename = filename.to_string();
        let (dir, name) = {
            let (d, n) = go_path::filepath::split(&filename);
            (d.to_string(), n.to_string())
        };
        if let Some(rename) = &root.meta.rename {
            let (n, ok) = rename(&name, false);
            if !ok {
                return Err(oserror::err_not_exist());
            }
            filename = go_path::filepath::join(&[dir.as_str(), n.as_str()]);
        }

        if !filename_filter::match_opt(
            root.meta.inclusion_filter.as_ref(),
            &root.trim_from(&filename),
            true,
        ) {
            return Err(oserror::err_not_exist());
        }

        let filename = root.filename(&filename);
        let mut fi = self.fs.stat(&filename)?;

        let opener: OpenFunc = if !fi.is_dir() {
            // Open the file directly.
            // Opens the real file directly.
            let fs = self.fs.clone();
            Arc::new(move || fs.open(&filename))
        } else if let Some(rename) = &root.meta.rename {
            // A single file mount where we have mounted the containing directory.
            let (n, ok) = rename(fi.name(), true);
            if !ok {
                return Err(oserror::err_not_exist());
            }
            fi.meta_mut().name = n;
            // Opens the real file directly.
            let fs = self.fs.clone();
            Arc::new(move || fs.open(&filename))
        } else {
            // Make sure metadata gets applied in ReadDir.
            self.real_dir_opener(&filename, root.meta.clone())
        };

        let fim = decorate_file_info(fi, Some(opener), "", Some(&root.meta));

        Ok(fim)
    }

    // Go: hugofs/rootmapping_fs.go:virtualDirOpener
    fn virtual_dir_opener(&self, name: &str) -> OpenFunc {
        let fs = self.arc();
        let name = name.to_string();
        Arc::new(move || {
            Ok(Box::new(RootMappingDir {
                dir_only_ops: None,
                fs: fs.clone(),
                name: name.clone(),
                meta: None,
            }) as Box<dyn File>)
        })
    }

    // Go: hugofs/rootmapping_fs.go:realDirOpener
    fn real_dir_opener(&self, name: &str, meta: Arc<FileMeta>) -> OpenFunc {
        let fs = self.arc();
        let name = name.to_string();
        Arc::new(move || {
            let f = fs.fs.open(&name)?;
            Ok(Box::new(RootMappingDir {
                dir_only_ops: Some(f),
                fs: fs.clone(),
                name: name.clone(),
                meta: Some(meta.clone()),
            }) as Box<dyn File>)
        })
    }

    // Go: hugofs/rootmapping_fs.go:Open
    fn open_impl(&self, name: &str) -> Result<Box<dyn File>> {
        let fis = self.do_stat(name)?;

        self.new_union_file(fis)
    }
}

trait TrimOnce {
    fn trim_start_matches_once(&self, p: &str) -> &str;
}

impl TrimOnce for str {
    /// Go: `strings.TrimPrefix` (one occurrence).
    fn trim_start_matches_once(&self, p: &str) -> &str {
        self.strip_prefix(p).unwrap_or(self)
    }
}

impl Fs for RootMappingFs {
    fn name(&self) -> &str {
        self.fs.name()
    }
    fn embedded(&self) -> Option<&dyn Fs> {
        Some(self.fs.as_ref())
    }
    // Go: hugofs/rootmapping_fs.go:Open
    /// Go: `Open(name)` — opens the named file for reading.
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        self.open_impl(name)
    }
    // Go: hugofs/rootmapping_fs.go:Stat
    /// Go: `Stat(name)` — if multiple roots are found, the last one will be used.
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        let mut fis = self.do_stat(name)?;
        Ok(fis.pop().expect("doStat returned no file infos"))
    }
    // Go: hugofs/rootmapping_fs.go:UnwrapFilesystem
    fn unwrap_filesystem(&self) -> Option<Arc<dyn Fs>> {
        Some(self.fs.clone())
    }
    fn reverse_lookup_provider(&self) -> Option<&dyn ReverseLookupProvider> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ReverseLookupProvider for RootMappingFs {
    fn reverse_lookup(&self, filename: &str) -> Result<Vec<ComponentPath>> {
        RootMappingFs::reverse_lookup(self, filename)
    }
    fn reverse_lookup_component(
        &self,
        component: &str,
        filename: &str,
    ) -> Result<Vec<ComponentPath>> {
        RootMappingFs::reverse_lookup_component(self, component, filename)
    }
}

/// Go: `hugofs.rootMappingDir`.
struct RootMappingDir {
    dir_only_ops: Option<Box<dyn File>>,
    fs: Arc<RootMappingFs>,
    name: String,
    meta: Option<Arc<FileMeta>>,
}

no_regular_file_ops!(RootMappingDir);

impl File for RootMappingDir {
    // Go: hugofs/rootmapping_fs.go:(f *rootMappingDir) Name
    fn name(&self) -> String {
        self.name.clone()
    }

    // Go: hugofs/rootmapping_fs.go:(f *rootMappingDir) Stat
    fn stat(&self) -> Result<FileMetaInfo> {
        Err(err_is_dir())
    }

    // Go: hugofs/rootmapping_fs.go:(f *rootMappingDir) ReadDir
    fn read_dir(&mut self, count: i32) -> Result<Vec<FileMetaInfo>> {
        if let Some(d) = &mut self.dir_only_ops {
            let fis = d.read_dir(count)?;

            let filter = self.meta.as_ref().and_then(|m| m.inclusion_filter.clone());
            let mut result = Vec::new();
            for fi in fis {
                let fim = decorate_file_info(fi, None, "", self.meta.as_deref());
                let meta = fim.meta();
                let rel = meta
                    .filename
                    .strip_prefix(meta.source_root.as_str())
                    .unwrap_or(&meta.filename)
                    .to_string();
                if filename_filter::match_opt(filter.as_ref(), &rel, fim.is_dir()) {
                    result.push(fim);
                }
            }
            return Ok(result);
        }

        self.fs.collect_dir_entries(&self.name)
    }

    // Go: hugofs/rootmapping_fs.go:(f *rootMappingDir) Readdirnames
    fn readdirnames(&mut self, count: i32) -> Result<Vec<String>> {
        let dirs = self.read_dir(count)?;
        Ok(dir_entries_to_names(&dirs))
    }

    // Go: hugofs/rootmapping_fs.go:(f *rootMappingDir) Close
    fn close(&mut self) -> Result<()> {
        match &mut self.dir_only_ops {
            None => Ok(()),
            Some(d) => d.close(),
        }
    }
}

// Go: hugofs/rootmapping_fs.go:dirEntriesToNames
fn dir_entries_to_names(fis: &[FileMetaInfo]) -> Vec<String> {
    fis.iter().map(|d| d.name().to_string()).collect()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/rootmapping_fs.go (854 lines; 26/35 funcs executed)
//   types: RootMapping, keyRootMappings, RootMappingFs, ComponentPath, ReverseLookupProvder, rootMappingDir
// OK L44-168: NewRootMappingFs(fs afero.Fs, rms ...RootMapping) (*RootMappingFs, error)
// OK L170-185: newRootMappingFsFromFromTo( baseDir string, fs afero.Fs, fromTo ...string, ) (*RootMappingFs, error)
// OK L208-211: (rm *RootMapping) clean()
// OK L213-218: (r RootMapping) filename(name string) string
// OK L220-225: (r RootMapping) trimFrom(name string) string
// OK L241-270: (fs *RootMappingFs) Mounts(base string) ([]FileMetaInfo, error)
// OK L272-274: (fs *RootMappingFs) Key() string
// OK L276-278: (fs *RootMappingFs) UnwrapFilesystem() afero.Fs
// OK L281-300: (fs RootMappingFs) Filter(f func(m RootMapping) bool) *RootMappingFs
// OK L303-310: (fs *RootMappingFs) Open(name string) (afero.File, error)
// OK L315-321: (fs *RootMappingFs) Stat(name string) (os.FileInfo, error)
// OK L330-332: (c ComponentPath) ComponentPathJoined() string
// OK L340-342: (fs *RootMappingFs) ReverseLookup(filename string) ([]ComponentPath, error)
// OK L344-386: (fs *RootMappingFs) ReverseLookupComponent(component, filename string) ([]ComponentPath, error)
// OK L388-396: (fs *RootMappingFs) hasPrefix(prefix string) bool
// OK L398-405: (fs *RootMappingFs) getRoot(key string) []RootMapping
// OK L407-440: (fs *RootMappingFs) getRoots(key string) (string, []RootMapping)
// OK L442-449: (fs *RootMappingFs) getRootsReverse(key string) (string, []RootMapping)
// OK L451-459: (fs *RootMappingFs) getRootsWithPrefix(prefix string) []RootMapping
// OK L461-474: (fs *RootMappingFs) getAncestors(prefix string) []keyRootMappings
// OK L476-525: (fs *RootMappingFs) newUnionFile(fis ...FileMetaInfo) (afero.File, error)
// OK L527-533: (fs *RootMappingFs) cleanName(name string) string
// OK L535-654: (rfs *RootMappingFs) collectDirEntries(prefix string) ([]iofs.DirEntry, error)
// OK L656-676: (fs *RootMappingFs) doStat(name string) ([]FileMetaInfo, error)
// OK L678-720: (fs *RootMappingFs) doDoStat(name string) ([]FileMetaInfo, error)
// OK L722-769: (fs *RootMappingFs) statRoot(root RootMapping, filename string) (FileMetaInfo, error)
// OK L771-773: (fs *RootMappingFs) virtualDirOpener(name string) func() (afero.File, error)
// OK L775-783: (fs *RootMappingFs) realDirOpener(name string, meta *FileMeta) func() (afero.File, error)
// OK L795-800: (f *rootMappingDir) Close() error
// OK L802-804: (f *rootMappingDir) Name() string
// OK L806-825: (f *rootMappingDir) ReadDir(count int) ([]iofs.DirEntry, error)
// OK L830-832: (f *rootMappingDir) Stat() (iofs.FileInfo, error)
// OK L834-836: (f *rootMappingDir) Readdir(count int) ([]os.FileInfo, error) (Go panics; use ReadDir)
// OK L840-846: (f *rootMappingDir) Readdirnames(count int) ([]string, error)
// OK L848-854: dirEntriesToNames(fis []iofs.DirEntry) []string
// ---------------------------------------------------------------------------
