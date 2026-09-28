//! Port of `hugofs/component_fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

//! Go `hugofs.componentFs`: the per-component view used by the content/layout/data/i18n walkers.
//! ReadDir order (component_fs.go:64-146) and `applyMeta` (NFC names on darwin, PathInfo parsing,
//! filename language, disabled languages dropped) define the order files are processed in.

use std::any::Any;
use std::sync::{Arc, Weak};

use nh_common::Result;
use nh_common::files::{COMPONENT_FOLDER_CONTENT, COMPONENT_FOLDER_I18N};
use nh_common::paths::pathparser::PathParser;

use crate::afero::{File, Fs, no_regular_file_ops};
use crate::fileinfo::{FileMetaInfo, mode};
use crate::fs::new_base_path_fs;
use crate::rootmapping_fs::is_err_is_dir;

/// Go: `hugofs.ComponentFsOptions`.
#[derive(Clone)]
pub struct ComponentFsOptions {
    /// The filesystem where one or more components are mounted.
    pub fs: Arc<dyn Fs>,
    /// The component name, e.g. "content", "layouts" etc.
    pub component: String,
    pub default_content_language: String,
    /// The parser used to parse paths provided by this filesystem.
    pub path_parser: Arc<PathParser>,
}

/// Go: `hugofs.componentFs` — a filesystem that holds one of the Hugo components, e.g. content,
/// layouts etc.
pub struct ComponentFs {
    pub opts: ComponentFsOptions,
    /// Go: the embedded `afero.Fs` (`NewBasePathFs(opts.Fs, opts.Component)`).
    fs: Arc<dyn Fs>,
    self_ref: Weak<ComponentFs>,
}

impl ComponentFs {
    /// Go: `hugofs.NewComponentFs(opts)` (Go panics on a missing component or fs).
    // Go: hugofs/component_fs.go:NewComponentFs
    pub fn new(opts: ComponentFsOptions) -> Arc<ComponentFs> {
        if opts.component.is_empty() {
            panic!("ComponentFsOptions.PathParser.Component must be set");
        }
        let bfs = new_base_path_fs(opts.fs.clone(), &opts.component);
        Arc::new_cyclic(|w| ComponentFs {
            opts,
            fs: bfs,
            self_ref: w.clone(),
        })
    }

    fn arc(&self) -> Arc<ComponentFs> {
        self.self_ref
            .upgrade()
            .expect("componentFs used after drop")
    }

    /// `Open(name)` + `ReadDir(-1)`: the merged, filtered, decorated and sorted entries of a
    /// directory.
    pub fn read_dir(&self, name: &str) -> Result<Vec<FileMetaInfo>> {
        let mut f = self.open_impl(name)?;
        let r = f.read_dir(-1);
        let _ = f.close();
        r
    }

    // Go: hugofs/component_fs.go:Stat
    pub fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        let fi = self.fs.stat(name)?;
        let (fim, _) = self.apply_meta(fi, name);
        Ok(fim)
    }

    // Go: hugofs/component_fs.go:applyMeta
    pub(crate) fn apply_meta(&self, fi: FileMetaInfo, name: &str) -> (FileMetaInfo, bool) {
        #[cfg(target_os = "macos")]
        let name = crate::nfc::nfc_string(name);
        #[cfg(target_os = "macos")]
        let name = name.as_str();

        let mut fim = fi;
        let pi = self.opts.path_parser.parse(&self.opts.component, name);
        if pi.disabled() {
            return (fim, false);
        }
        if !fim.meta().lang.is_empty()
            && let Some(is_lang_disabled) = &self.opts.path_parser.is_lang_disabled
            && is_lang_disabled(&fim.meta().lang)
        {
            return (fim, false);
        }
        let is_dir = fim.is_dir();
        let meta = fim.meta_mut();
        let file_lang = pi.lang().to_string();
        meta.path_info = Some(Arc::new(pi));
        if !is_dir && !file_lang.is_empty() {
            // A valid lang set in filename.
            // Give priority to myfile.sv.txt inside the sv filesystem.
            meta.weight += 1;
            meta.lang = file_lang;
        }

        if meta.lang.is_empty() {
            meta.lang = self.opts.default_content_language.clone();
        }

        let lang_idx = self
            .opts
            .path_parser
            .language_index
            .as_ref()
            .and_then(|m| m.get(&meta.lang).copied());
        let Some(lang_idx) = lang_idx else {
            panic!("no language found for {}", meta.lang);
        };
        meta.lang_index = lang_idx as i64;

        if is_dir {
            let fs = self.arc();
            let name = name.to_string();
            meta.open_func = Some(Arc::new(move || fs.open_impl(&name)));
        }

        (fim, true)
    }

    // Go: hugofs/component_fs.go:Open
    fn open_impl(&self, name: &str) -> Result<Box<dyn File>> {
        let mut f = self.fs.open(name)?;

        match f.stat() {
            Err(e) => {
                if !is_err_is_dir(&e) {
                    let _ = f.close();
                    return Err(e);
                }
            }
            Ok(fi) => {
                if !fi.is_dir() {
                    return Ok(f);
                }
            }
        }

        Ok(Box::new(ComponentFsDir {
            dir_only_ops: f,
            name: name.to_string(),
            fs: self.arc(),
        }))
    }

    // Go: componentFsDir.ReadDir (the sort)
    fn less(&self, fimi: &FileMetaInfo, fimj: &FileMetaInfo) -> bool {
        if fimi.is_dir() != fimj.is_dir() {
            return fimi.is_dir();
        }
        let (fimim, fimjm) = (fimi.meta(), fimj.meta());

        if fimim.module_ordinal != fimjm.module_ordinal {
            return match self.opts.component.as_str() {
                COMPONENT_FOLDER_I18N => {
                    // The way the language files gets loaded means that
                    // we need to provide the least important files first (e.g. the theme files).
                    fimim.module_ordinal > fimjm.module_ordinal
                }
                _ => fimim.module_ordinal < fimjm.module_ordinal,
            };
        }

        if let Some(pii) = &fimim.path_info {
            let pij = fimjm
                .path_info
                .as_ref()
                .expect("componentFs: entry without PathInfo");
            let (basei, basej) = (pii.base(), pij.base());
            let (exti, extj) = (pii.ext(), pij.ext());
            if self.opts.component == COMPONENT_FOLDER_CONTENT {
                // Pull bundles to the top.
                if pii.is_bundle() != pij.is_bundle() {
                    return pii.is_bundle();
                }
            }

            if exti != extj {
                // This pulls .md above .html.
                return exti > extj;
            }

            if basei != basej {
                return basei < basej;
            }
        }

        if fimim.weight != fimjm.weight {
            return fimim.weight > fimjm.weight;
        }

        fimi.name() < fimj.name()
    }
}

impl Fs for ComponentFs {
    fn name(&self) -> &str {
        self.fs.name()
    }
    fn embedded(&self) -> Option<&dyn Fs> {
        Some(self.fs.as_ref())
    }
    // Go: hugofs/component_fs.go:UnwrapFilesystem
    fn unwrap_filesystem(&self) -> Option<Arc<dyn Fs>> {
        Some(self.fs.clone())
    }
    // Go: hugofs/component_fs.go:Stat
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        ComponentFs::stat(self, name)
    }
    // Go: hugofs/component_fs.go:Open
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        self.open_impl(name)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `hugofs.componentFsDir`.
struct ComponentFsDir {
    dir_only_ops: Box<dyn File>,
    /// the name passed to Open
    name: String,
    fs: Arc<ComponentFs>,
}

no_regular_file_ops!(ComponentFsDir);

impl File for ComponentFsDir {
    fn name(&self) -> String {
        self.dir_only_ops.name()
    }

    // Go: hugofs/component_fs.go:(f *componentFsDir) Stat
    fn stat(&self) -> Result<FileMetaInfo> {
        let fi = self.dir_only_ops.stat()?;
        let (fim, _) = self.fs.apply_meta(fi, &self.name);
        Ok(fim)
    }

    // Go: hugofs/component_fs.go:(f *componentFsDir) ReadDir
    /// Reads count entries from this virtual directory and sorts the entries according to the
    /// component filesystem rules.
    fn read_dir(&mut self, _count: i32) -> Result<Vec<FileMetaInfo>> {
        let fis = self.dir_only_ops.read_dir(-1)?;

        // Filter out any symlinks.
        let mut kept = Vec::with_capacity(fis.len());
        for fi in fis {
            // IsDir will always be false for symlinks.
            let mut keep = fi.is_dir();
            if !keep {
                // This is unfortunate, but is the only way to determine if it is a symlink.
                let info = match fi.info() {
                    Ok(info) => info,
                    Err(e) => {
                        if e.is_not_exist() {
                            continue;
                        }
                        return Err(e);
                    }
                };
                if info.mode() & mode::MODE_SYMLINK == 0 {
                    keep = true;
                }
            }
            if keep {
                kept.push(fi);
            }
        }

        let mut fis = Vec::with_capacity(kept.len());
        for fi in kept {
            let s = go_path::path::join(&[self.name.as_str(), fi.name()]);
            let (fi, ok) = self.fs.apply_meta(fi, &s);
            if ok {
                fis.push(fi);
            }
        }

        let fs = self.fs.clone();
        go_sort::sort_by(&mut fis, |a, b| fs.less(a, b));

        Ok(fis)
    }

    // Go: hugofs/component_fs.go:(f *componentFsDir) Readdirnames
    fn readdirnames(&mut self, count: i32) -> Result<Vec<String>> {
        let dirsi = self.dir_only_ops.read_dir(count)?;
        Ok(dirsi.iter().map(|d| d.name().to_string()).collect())
    }

    fn close(&mut self) -> Result<()> {
        self.dir_only_ops.close()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/component_fs.go (272 lines; 6/10 funcs executed)
//   types: componentFs, componentFsDir, ComponentFsOptions
// OK L31-40: NewComponentFs(opts ComponentFsOptions) *componentFs
// OK L51-53: (fs *componentFs) UnwrapFilesystem() afero.Fs
// OK L64-153: (f *componentFsDir) ReadDir(count int) ([]iofs.DirEntry, error)
// OK L155-162: (f *componentFsDir) Stat() (iofs.FileInfo, error)
// OK L164-171: (fs *componentFs) Stat(name string) (os.FileInfo, error)
// OK L173-215: (fs *componentFs) applyMeta(fi FileNameIsDir, name string) (FileMetaInfo, bool)
// OK L217-219: (f *componentFsDir) Readdir(count int) ([]os.FileInfo, error) (Go panics; use ReadDir)
// OK L221-232: (f *componentFsDir) Readdirnames(count int) ([]string, error)
// OK L247-268: (fs *componentFs) Open(name string) (afero.File, error)
// OK L270-272: (fs *componentFs) ReadDir(name string) ([]os.FileInfo, error) (Go panics; `read_dir` here is Open + ReadDir)
// ---------------------------------------------------------------------------
