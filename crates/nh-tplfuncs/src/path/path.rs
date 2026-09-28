//! Port of `tpl/path/path.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! `filepath.ToSlash` is the identity on unix (the golden build ran on darwin).

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Kind, Object, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

// Parity notes: `path.Ext` on a resource uses its String() (= Name()).

/// Go: `path.Namespace` (template value `*path.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

/// Go: `paths.DirFile` — the result of `path.Split` (fields `Dir`, `File`; `String()` is
/// `Dir + "|" + File`).
#[derive(Clone, Debug)]
pub struct DirFile {
    pub dir: GoString,
    pub file: GoString,
}

impl DirFile {
    // Go: common/paths/path.go:(DirFile).String
    fn string(&self) -> GoString {
        let mut b = self.dir.to_vec();
        b.push(b'|');
        b.extend_from_slice(self.file.as_bytes());
        GoString::from(b)
    }
}

nh_common::go_methods!(DirFile {
    "String" => |d, _c, a| { args::exactly(a, 0, "String")?; Ok(Value::String(d.string())) },
});

impl Object for DirFile {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("paths.DirFile")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        Self::go_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, a: &[Value]) -> Option<GoResult<Value>> {
        self.go_call_method(ctx, name, a)
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Dir" => Some(Value::String(self.dir.clone())),
            "File" => Some(Value::String(self.file.clone())),
            _ => None,
        }
    }
    fn go_string(&self) -> Option<GoString> {
        Some(self.string())
    }
    fn is_zero(&self) -> Option<bool> {
        Some(self.dir.is_empty() && self.file.is_empty())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// `cast.ToStringE(path)` then `filepath.ToSlash`.
fn spath(v: &Value) -> GoResult<GoString> {
    Ok(caste::to_string_e(v)?)
}

fn sv(b: impl Into<GoString>) -> Value {
    Value::String(b.into())
}

impl Namespace {
    /// New returns a new instance of the path-namespaced template functions.
    // Go: tpl/path/path.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    /// Base returns the last element of path. Trailing slashes are removed before extracting
    /// the last element. If the path is empty, Base returns ".". If the path consists entirely
    /// of slashes, Base returns "/".
    // Go: tpl/path/path.go:Base
    pub fn base(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Base")?;
        let spath = spath(&a[0])?;
        Ok(sv(go_path::path::base_bytes(spath.as_bytes()).to_vec()))
    }

    /// BaseName returns the last element of path, removing the extension if present.
    // Go: tpl/path/path.go:BaseName
    pub fn base_name(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "BaseName")?;
        let spath = spath(&a[0])?;
        let base = go_path::path::base_bytes(spath.as_bytes());
        let ext = go_path::path::ext_bytes(spath.as_bytes());
        Ok(sv(go_unicode::strings::trim_suffix(base, ext).to_vec()))
    }

    /// Clean replaces the separators used with standard slashes and then extraneous slashes
    /// are removed.
    // Go: tpl/path/path.go:Clean
    pub fn clean(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Clean")?;
        let spath = spath(&a[0])?;
        Ok(sv(go_path::path::clean_bytes(spath.as_bytes())))
    }

    /// Dir returns all but the last element of path, typically the path's directory.
    // Go: tpl/path/path.go:Dir
    pub fn dir(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Dir")?;
        let spath = spath(&a[0])?;
        Ok(sv(go_path::path::dir_bytes(spath.as_bytes())))
    }

    /// Ext returns the file name extension used by path.
    // Go: tpl/path/path.go:Ext
    pub fn ext(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Ext")?;
        let spath = spath(&a[0])?;
        Ok(sv(go_path::path::ext_bytes(spath.as_bytes()).to_vec()))
    }

    /// Join joins any number of path elements into a single path, adding a separating slash if
    /// necessary. The result is Cleaned; in particular, all empty strings are ignored.
    // Go: tpl/path/path.go:Join
    pub fn join(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        let mut path_elements: Vec<GoString> = Vec::new();
        for elem in a {
            match elem {
                Value::List(l) if l.ty == go_value::SliceType::String => {
                    for e in &l.items {
                        if let Value::String(s) = e {
                            path_elements.push(s.clone());
                        }
                    }
                }
                Value::List(l) if l.ty == go_value::SliceType::Any => {
                    for e in &l.items {
                        path_elements.push(caste::to_string_e(e)?);
                    }
                }
                // A nil []string or []interface {}: no elements.
                Value::TypedNil(t) if &**t == "[]string" || &**t == "[]interface {}" => {}
                _ => path_elements.push(caste::to_string_e(elem)?),
            }
        }
        let parts: Vec<&[u8]> = path_elements.iter().map(|s| s.as_bytes()).collect();
        Ok(sv(go_path::path::join_bytes(&parts)))
    }

    /// Split splits path immediately following the final slash, separating it into a directory
    /// and file name component.
    // Go: tpl/path/path.go:Split
    pub fn split(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Split")?;
        let spath = spath(&a[0])?;
        let (dir, file) = go_path::path::split_bytes(spath.as_bytes());
        Ok(Value::object(DirFile {
            dir: GoString::from(dir.to_vec()),
            file: GoString::from(file.to_vec()),
        }))
    }
}

nh_common::go_methods!(Namespace {
    "Base" => |n, ctx, a| n.base(ctx, a),
    "BaseName" => |n, ctx, a| n.base_name(ctx, a),
    "Clean" => |n, ctx, a| n.clean(ctx, a),
    "Dir" => |n, ctx, a| n.dir(ctx, a),
    "Ext" => |n, ctx, a| n.ext(ctx, a),
    "Join" => |n, ctx, a| n.join(ctx, a),
    "Split" => |n, ctx, a| n.split(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*path.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/path/path.go (162 lines; 2/8 funcs executed)
//   types: Namespace
// OK L28-32: New(deps *deps.Deps) *Namespace
// OK L45-52: (ns *Namespace) Ext(path any) (string, error)
// OK L63-70: (ns *Namespace) Dir(path any) (string, error)
// OK L78-85: (ns *Namespace) Base(path any) (string, error)
// OK L93-100: (ns *Namespace) BaseName(path any) (string, error)
// OK L109-118: (ns *Namespace) Split(path any) (paths.DirFile, error)
// OK L126-151: (ns *Namespace) Join(elements ...any) (string, error)
// OK L155-162: (ns *Namespace) Clean(path any) (string, error)
// ---------------------------------------------------------------------------
