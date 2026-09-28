//! Port of `hugofs/decorators.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

use std::any::Any;
use std::sync::{Arc, Weak};

use nh_common::Result;

use crate::afero::{File, Fs, forward_file_io};
use crate::fileinfo::{FileMeta, FileMetaInfo, OpenFunc, decorate_file_info};

/// A `NewBaseFileDecorator` callback.
pub type DecoratorCallback = Arc<dyn Fn(&FileMetaInfo) + Send + Sync>;

enum Decorator {
    /// Go: the decorator of `NewBaseFileDecorator`.
    Base { callbacks: Vec<DecoratorCallback> },
    /// Go: the decorator of `decorateDirs`.
    Dirs { meta: Arc<FileMeta> },
}

/// Go: `hugofs.baseFileDecoratorFs`.
pub struct BaseFileDecoratorFs {
    fs: Arc<dyn Fs>,
    decorator: Decorator,
    self_ref: Weak<BaseFileDecoratorFs>,
}

/// Go: `hugofs.NewBaseFileDecorator(fs, callbacks...)` — attaches `FileMeta` (filename, open func,
/// join-stat func) to every file info from `fs`.
// Go: hugofs/decorators.go:NewBaseFileDecorator
pub fn new_base_file_decorator(fs: Arc<dyn Fs>, callbacks: Vec<DecoratorCallback>) -> Arc<dyn Fs> {
    Arc::new_cyclic(|w| BaseFileDecoratorFs {
        fs,
        decorator: Decorator::Base { callbacks },
        self_ref: w.clone(),
    })
}

// Go: hugofs/decorators.go:decorateDirs
pub(crate) fn decorate_dirs(fs: Arc<dyn Fs>, meta: Arc<FileMeta>) -> Arc<dyn Fs> {
    Arc::new_cyclic(|w| BaseFileDecoratorFs {
        fs,
        decorator: Decorator::Dirs { meta },
        self_ref: w.clone(),
    })
}

impl BaseFileDecoratorFs {
    fn arc(&self) -> Arc<BaseFileDecoratorFs> {
        self.self_ref
            .upgrade()
            .expect("baseFileDecoratorFs used after drop")
    }

    // Go: the `decorate` closures of decorateDirs and NewBaseFileDecorator
    fn decorate(&self, fi: FileMetaInfo, filename: &str) -> Result<FileMetaInfo> {
        match &self.decorator {
            Decorator::Dirs { meta } => {
                if !fi.is_dir() {
                    // Leave regular files as they are.
                    return Ok(fi);
                }
                Ok(decorate_file_info(fi, None, "", Some(meta)))
            }
            Decorator::Base { callbacks } => {
                // Store away the original in case it's a symlink.
                let mut meta = FileMeta::new();
                meta.name = fi.name().to_string();

                if fi.is_dir() {
                    let fs = self.fs.clone();
                    let ffs = self.arc();
                    let filename = filename.to_string();
                    meta.join_stat_func = Some(Arc::new(move |name: &str| {
                        let joined_filename = go_path::filepath::join(&[filename.as_str(), name]);
                        let fi = fs.stat(&joined_filename)?;
                        ffs.decorate(fi, &joined_filename)
                    }));
                }

                let ffs = self.arc();
                let fname = filename.to_string();
                let opener: OpenFunc = Arc::new(move || ffs.open_impl(&fname));

                let fim = decorate_file_info(fi, Some(opener), filename, Some(&meta));

                for cb in callbacks {
                    cb(&fim);
                }

                Ok(fim)
            }
        }
    }

    // Go: hugofs/decorators.go:open
    fn open_impl(&self, name: &str) -> Result<Box<dyn File>> {
        let f = self.fs.open(name)?;
        Ok(Box::new(BaseFileDecoratorFile {
            file: f,
            fs: self.arc(),
        }))
    }
}

impl Fs for BaseFileDecoratorFs {
    fn name(&self) -> &str {
        self.fs.name()
    }
    fn embedded(&self) -> Option<&dyn Fs> {
        Some(self.fs.as_ref())
    }
    // Go: hugofs/decorators.go:UnwrapFilesystem
    fn unwrap_filesystem(&self) -> Option<Arc<dyn Fs>> {
        Some(self.fs.clone())
    }
    // Go: hugofs/decorators.go:Stat
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        let fi = self.fs.stat(name)?;
        self.decorate(fi, name)
    }
    // Go: hugofs/decorators.go:Open
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        self.open_impl(name)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `hugofs.baseFileDecoratorFile`.
struct BaseFileDecoratorFile {
    file: Box<dyn File>,
    fs: Arc<BaseFileDecoratorFs>,
}

forward_file_io!(BaseFileDecoratorFile, file);

impl File for BaseFileDecoratorFile {
    fn name(&self) -> String {
        self.file.name()
    }
    fn stat(&self) -> Result<FileMetaInfo> {
        self.file.stat()
    }
    // Go: hugofs/decorators.go:(l *baseFileDecoratorFile) ReadDir
    fn read_dir(&mut self, _n: i32) -> Result<Vec<FileMetaInfo>> {
        let fis = self.file.read_dir(-1)?;

        let lname = self.file.name();
        let mut fisp = Vec::with_capacity(fis.len());
        for fi in fis {
            let mut filename = fi.name().to_string();
            if !lname.is_empty() {
                filename = go_path::filepath::join(&[lname.as_str(), fi.name()]);
            }

            let fid = self
                .fs
                .decorate(fi, &filename)
                .map_err(|e| e.wrap("decorate"))?;

            fisp.push(fid);
        }

        Ok(fisp)
    }
    fn close(&mut self) -> Result<()> {
        self.file.close()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/decorators.go (154 lines; 7/8 funcs executed)
//   types: baseFileDecoratorFs, baseFileDecoratorFile
// OK L27-42: decorateDirs(fs afero.Fs, meta *FileMeta) afero.Fs
// OK L46-85: NewBaseFileDecorator(fs afero.Fs, callbacks ...func(fi FileMetaInfo)) afero.Fs
// OK L92-94: (fs *baseFileDecoratorFs) UnwrapFilesystem() afero.Fs
// OK L96-107: (fs *baseFileDecoratorFs) Stat(name string) (os.FileInfo, error)
// OK L109-111: (fs *baseFileDecoratorFs) Open(name string) (afero.File, error)
// OK L113-119: (fs *baseFileDecoratorFs) open(name string) (afero.File, error)
// OK L126-150: (l *baseFileDecoratorFile) ReadDir(n int) ([]fs.DirEntry, error)
// OK L152-154: (l *baseFileDecoratorFile) Readdir(c int) (ofi []os.FileInfo, err error) (Go panics; use ReadDir)
// ---------------------------------------------------------------------------
