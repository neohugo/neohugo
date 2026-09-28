//! Port of `tpl/os/os.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! The files are read from the site's working filesystems (Go: an overlay of `d.Work` and
//! `d.Content.Fs` for `ReadFile`/`FileExists`/`Stat`, `d.WorkDir` for `ReadDir`).

use std::sync::Arc;

use go_value::{GoString, HostCtx, Kind, Object, SliceType, UintKind, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;
use nh_hugofs::afero::Fs;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_hugofs::overlayfs::{Options, OverlayFs};

/// Go: `os.Namespace` (template value `*os.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    read_file_fs: Option<Arc<dyn Fs>>,
    work_fs: Option<Arc<dyn Fs>>,
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn nil_deref() -> go_value::Error {
    gerr("runtime error: invalid memory address or nil pointer dereference")
}

/// Go: `fs.FileMode` (a named `uint32`; `String()` is the `ls -l` form).
#[derive(Clone, Copy, Debug)]
pub struct FileModeValue(pub u32);

impl FileModeValue {
    // Go: io/fs/fs.go:(FileMode).String
    fn string(&self) -> String {
        const STR: &str = "dalTLDpSugct?";
        let m = self.0;
        let mut buf = String::new();
        for (i, c) in STR.chars().enumerate() {
            if m & (1 << (32 - 1 - i)) != 0 {
                buf.push(c);
            }
        }
        if buf.is_empty() {
            buf.push('-');
        }
        const RWX: &str = "rwxrwxrwx";
        for (i, c) in RWX.chars().enumerate() {
            if m & (1 << (9 - 1 - i)) != 0 {
                buf.push(c);
            } else {
                buf.push('-');
            }
        }
        buf
    }
}

nh_common::go_methods!(FileModeValue {
    "IsDir" => |m, _c, a| { args::exactly(a, 0, "IsDir")?; Ok(Value::Bool(m.0 & nh_hugofs::fileinfo::mode::MODE_DIR != 0)) },
    "IsRegular" => |m, _c, a| { args::exactly(a, 0, "IsRegular")?; Ok(Value::Bool(m.0 & nh_hugofs::fileinfo::mode::MODE_TYPE == 0)) },
    "Perm" => |m, _c, a| { args::exactly(a, 0, "Perm")?; Ok(Value::object(FileModeValue(m.0 & 0o777))) },
    "String" => |m, _c, a| { args::exactly(a, 0, "String")?; Ok(Value::string(m.string())) },
    "Type" => |m, _c, a| { args::exactly(a, 0, "Type")?; Ok(Value::object(FileModeValue(m.0 & nh_hugofs::fileinfo::mode::MODE_TYPE))) },
});

impl Object for FileModeValue {
    nh_common::object_basics!("fs.FileMode");
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::Uint(self.0 as u64, UintKind::Uint32))
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.string()))
    }
}

/// Go: `os.FileInfo` (the `*os.fileStat` of an OS file) as a template value.
#[derive(Clone)]
pub struct FileInfoObject(pub FileMetaInfo);

nh_common::go_methods!(FileInfoObject {
    "IsDir" => |f, _c, a| { args::exactly(a, 0, "IsDir")?; Ok(Value::Bool(f.0.is_dir())) },
    "ModTime" => |f, _c, a| { args::exactly(a, 0, "ModTime")?; Ok(Value::Time(f.0.mod_time.clone())) },
    "Mode" => |f, _c, a| { args::exactly(a, 0, "Mode")?; Ok(Value::object(FileModeValue(f.0.mode()))) },
    "Name" => |f, _c, a| { args::exactly(a, 0, "Name")?; Ok(Value::string(f.0.name())) },
    "Size" => |f, _c, a| { args::exactly(a, 0, "Size")?; Ok(Value::int64(f.0.size)) },
    "Sys" => |_f, _c, a| { args::exactly(a, 0, "Sys")?; Ok(Value::Invalid) },
});

impl Object for FileInfoObject {
    nh_common::object_basics!("*os.fileStat");
    fn kind(&self) -> Kind {
        Kind::Ptr
    }
}

impl Namespace {
    /// New returns a new instance of the os-namespaced template functions.
    // Go: tpl/os/os.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let mut read_file_fs: Option<Arc<dyn Fs>> = None;
        let mut work_fs: Option<Arc<dyn Fs>> = None;

        // The docshelper script does not have or need all the dependencies set up.
        if let Some(ps) = &d.path_spec {
            read_file_fs = Some(Arc::new(OverlayFs::new(Options {
                fss: vec![ps.base_fs.work.clone(), ps.base_fs.content.fs.clone()],
                ..Default::default()
            })));
            // See #9599
            work_fs = Some(ps.base_fs.work_dir.clone());
        }

        Namespace {
            d,
            read_file_fs,
            work_fs,
        }
    }

    /// FileExists checks whether a file exists under the given path.
    // Go: tpl/os/os.go:FileExists
    pub fn file_exists(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "FileExists")?;
        let path = caste::to_string_e(&a[0])?;

        if path.is_empty() {
            return Err(gerr("fileExists needs a path to a file"));
        }

        let fs = self.read_file_fs.as_ref().ok_or_else(nil_deref)?;
        let status = nh_hugofs::afero::exists(fs.as_ref(), &path.to_str_lossy())?;
        Ok(Value::Bool(status))
    }

    /// Getenv retrieves the value of the environment variable named by the key. It returns the
    /// value, which will be empty if the variable is not present.
    // Go: tpl/os/os.go:Getenv
    pub fn getenv(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Getenv")?;
        let Ok(skey) = caste::to_string_e(&a[0]) else {
            return Ok(Value::string(""));
        };
        let skey = skey.to_str_lossy().into_owned();

        self.d
            .exec_helper
            .as_ref()
            .ok_or_else(nil_deref)?
            .sec()
            .check_allowed_get_env(&skey)?;

        Ok(Value::String(getenv(&skey)))
    }

    /// ReadDir lists the directory contents relative to the configured WorkingDir.
    // Go: tpl/os/os.go:ReadDir
    pub fn read_dir(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ReadDir")?;
        let path = caste::to_string_e(&a[0])?;

        let fs = self.work_fs.as_ref().ok_or_else(nil_deref)?;
        let list =
            nh_hugofs::afero::read_dir(fs.as_ref(), &path.to_str_lossy()).map_err(|err| {
                gerr(format!(
                    "failed to read directory {}: {}",
                    go_strconv::quote(path.as_bytes()),
                    err.message()
                ))
            })?;

        Ok(Value::list(
            SliceType::Named(Arc::from("[]fs.FileInfo")),
            list.into_iter()
                .map(|fi| Value::object(FileInfoObject(fi)))
                .collect(),
        ))
    }

    /// ReadFile reads the file named by filename relative to the configured WorkingDir. It
    /// returns the contents as a string. There is an upper size limit set at 1 megabytes.
    // Go: tpl/os/os.go:ReadFile
    pub fn read_file(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ReadFile")?;
        let s = caste::to_string_e(&a[0])?;
        let mut s = s.to_str_lossy().into_owned();

        if let Some(ps) = &self.d.path_spec {
            s = ps.rel_pathify(&s);
        }

        let fs = self.read_file_fs.as_ref().ok_or_else(nil_deref)?;
        match read_file(fs.as_ref(), &s) {
            Ok(b) => Ok(Value::String(GoString::from(b))),
            Err(e) if e.is_not_exist() => Ok(Value::string("")),
            Err(e) => Err(e.into()),
        }
    }

    /// Stat returns the os.FileInfo structure describing file.
    // Go: tpl/os/os.go:Stat
    pub fn stat(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Stat")?;
        let path = caste::to_string_e(&a[0])?;

        if path.is_empty() {
            return Err(gerr("fileStat needs a path to a file"));
        }

        let fs = self.read_file_fs.as_ref().ok_or_else(nil_deref)?;
        let r = fs.stat(&path.to_str_lossy())?;
        Ok(Value::object(FileInfoObject(r)))
    }
}

/// `os.Getenv(key)` (the environment's bytes).
fn getenv(key: &str) -> GoString {
    use std::os::unix::ffi::OsStrExt;
    match std::env::var_os(key) {
        Some(v) => GoString::from(v.as_bytes().to_vec()),
        None => GoString::empty(),
    }
}

/// readFile reads the file named by filename in the given filesystem and returns the contents
/// as a string.
// Go: tpl/os/os.go:readFile
fn read_file(fs: &dyn Fs, filename: &str) -> nh_common::Result<Vec<u8>> {
    let filename = go_path::filepath::clean(filename);
    if filename.is_empty() || filename == "." || filename == "/" {
        return Err(nh_common::herrors::Error::new("invalid filename"));
    }

    nh_hugofs::afero::read_file(fs, &filename)
}

nh_common::go_methods!(Namespace {
    "FileExists" => |n, ctx, a| n.file_exists(ctx, a),
    "Getenv" => |n, ctx, a| n.getenv(ctx, a),
    "ReadDir" => |n, ctx, a| n.read_dir(ctx, a),
    "ReadFile" => |n, ctx, a| n.read_file(ctx, a),
    "Stat" => |n, ctx, a| n.stat(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*os.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/os/os.go (163 lines; 1/7 funcs executed)
//   types: Namespace
// OK L32-52: New(d *deps.Deps) *Namespace
// OK L63-74: (ns *Namespace) Getenv(key any) (string, error)
// OK L78-90: readFile(fs afero.Fs, filename string) (string, error)
// OK L95-110: (ns *Namespace) ReadFile(i any) (string, error)
// OK L113-125: (ns *Namespace) ReadDir(i any) ([]_os.FileInfo, error)
// OK L128-144: (ns *Namespace) FileExists(i any) (bool, error)
// OK L147-163: (ns *Namespace) Stat(i any) (_os.FileInfo, error)
// ---------------------------------------------------------------------------
