//! Port of `github.com/bep/golibsass@v1.2.0/internal/libsass`
//! (`a__libsass.go`, `a__importer.go`; `a__types.go` only declares type
//! aliases, which live in [`crate::ffi`]).
//!
//! All functions here are thin, unsafe wrappers around the C API, like their
//! Go counterparts. Memory ownership follows LibSass: strings handed over
//! with `sass_make_data_context`/`sass_make_import_entry(source)` must be
//! heap copies (LibSass frees them); option setters copy their argument.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::{Arc, LazyLock, Mutex};

use crate::ffi::{
    self, Sass_Compiler, Sass_Context, Sass_Data_Context, Sass_Import_List, Sass_Importer_Entry,
    Sass_Options,
};

/// Go: `type ImportResolver func(currPath string, prevPath string) (newPath
/// string, body string, resolved bool)`.
///
/// Return an empty body to load the import body from the path.
pub type ImportResolver = Arc<dyn Fn(&[u8], &[u8]) -> (Vec<u8>, Vec<u8>, bool) + Send + Sync>;

/// Go `C.CString(s)`: a heap (malloc) copy of all bytes of `s` plus a
/// terminating NUL. C sees the string only up to its first NUL byte.
pub(crate) fn c_string(s: &[u8]) -> *mut c_char {
    unsafe {
        let p = ffi::sass_alloc_memory(s.len() + 1) as *mut u8;
        ptr::copy_nonoverlapping(s.as_ptr(), p, s.len());
        *p.add(s.len()) = 0;
        p as *mut c_char
    }
}

/// A `C.CString` that is freed when dropped. Go leaks the strings it passes
/// to the (copying) option setters; we free them after the call.
pub(crate) struct CStringBuf(*mut c_char);

impl CStringBuf {
    pub(crate) fn new(s: &[u8]) -> CStringBuf {
        CStringBuf(c_string(s))
    }

    pub(crate) fn as_ptr(&self) -> *const c_char {
        self.0
    }
}

impl Drop for CStringBuf {
    fn drop(&mut self) {
        unsafe { ffi::sass_free_memory(self.0 as *mut c_void) };
    }
}

/// Go `C.GoString(p)`: the bytes up to the first NUL; `""` for NULL.
///
/// # Safety
/// `p` must be NULL or point to a NUL-terminated string.
pub(crate) unsafe fn go_string(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    unsafe { CStr::from_ptr(p) }.to_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// a__importer.go

const UINTPTR_OFFSET: i64 = 4096; // the smallest legal pointer

/// Go: `idMap` (maps the importer cookie to the resolver).
struct IdMap {
    m: HashMap<i64, ImportResolver>,
    i: i64,
    id_stack: Vec<i64>,
}

static IMPORTS_STORE: LazyLock<Mutex<IdMap>> = LazyLock::new(|| {
    Mutex::new(IdMap {
        m: HashMap::new(),
        i: UINTPTR_OFFSET,
        id_stack: Vec::new(),
    })
});

impl IdMap {
    // Go: a__importer.go:(*idMap).Delete
    fn delete(&mut self, i: i64) {
        self.id_stack.push(i);
        self.m.remove(&i);
    }

    // Go: a__importer.go:(*idMap).Get
    fn get(&self, i: i64) -> Option<ImportResolver> {
        self.m.get(&i).cloned()
    }

    // Go: a__importer.go:(*idMap).nextID
    fn next_id(&mut self) -> i64 {
        if self.id_stack.is_empty() {
            for i in 1..=50 {
                self.id_stack.push(self.i + i);
            }
            self.i += 50;
        }
        self.id_stack.pop().unwrap()
    }

    // Go: a__importer.go:(*idMap).Set
    fn set(&mut self, v: ImportResolver) -> i64 {
        let id = self.next_id();
        self.m.insert(id, v);
        id
    }
}

fn imports_store() -> std::sync::MutexGuard<'static, IdMap> {
    IMPORTS_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

// Go: a__importer.go:AddImportResolver
/// Adds a function to resolve imports in LibSASS. Make sure to call
/// [`delete_import_resolver`] when done.
///
/// # Safety
/// `opts` must be a live `Sass_Options*`.
pub unsafe fn add_import_resolver(opts: *mut Sass_Options, resolver: ImportResolver) -> i64 {
    let i = imports_store().set(resolver);
    // This looks unsafe, but LibSass is using void* to store an int.
    let id = i as usize as *mut c_void;

    unsafe {
        let importers = ffi::sass_make_importer_list(1);
        ffi::sass_importer_set_list_entry(
            importers,
            0,
            ffi::sass_make_importer(Some(sass_import), 0.0, id),
        );

        ffi::sass_option_set_c_importers(opts, importers);
    }

    i
}

// Go: a__importer.go:DeleteImportResolver
pub fn delete_import_resolver(i: i64) {
    imports_store().delete(i);
}

thread_local! {
    /// A panic raised by an import resolver, re-raised by the transpiler once
    /// control is back in Rust (unwinding through C++ frames is not allowed).
    static RESOLVER_PANIC: RefCell<Option<Box<dyn Any + Send>>> = const { RefCell::new(None) };
}

/// Returns (and clears) a panic payload captured in an import resolver
/// running on this thread.
pub(crate) fn take_resolver_panic() -> Option<Box<dyn Any + Send>> {
    RESOLVER_PANIC.with(|p| p.borrow_mut().take())
}

// Go: a__importer.go cgo preamble: SassImport (C)
/// The C importer function registered with LibSass:
///
/// ```c
/// void* c = sass_importer_get_cookie(imp);
/// uintptr_t ci = (uintptr_t)c;
/// struct Sass_Import* prevPath = sass_compiler_get_last_import(comp);
/// const char* prev_path = sass_import_get_imp_path(prevPath);
/// return BridgeImport(currPath, prev_path, ci);
/// ```
unsafe extern "C" fn sass_import(
    curr_path: *const c_char,
    imp: Sass_Importer_Entry,
    comp: *mut Sass_Compiler,
) -> Sass_Import_List {
    unsafe {
        let c = ffi::sass_importer_get_cookie(imp);
        let ci = c as usize;
        let prev_path_entry = ffi::sass_compiler_get_last_import(comp);
        let prev_path = if prev_path_entry.is_null() {
            ptr::null()
        } else {
            ffi::sass_import_get_imp_path(prev_path_entry)
        };
        // BridgeImport's parameter is a C `int`.
        bridge_import(curr_path, prev_path, ci as c_int)
    }
}

// Go: a__libsass.go:BridgeImport
/// A bridge function to C to resolve imports.
unsafe fn bridge_import(
    curr_path: *const c_char,
    prev_path: *const c_char,
    ci: c_int,
) -> Sass_Import_List {
    unsafe {
        let parent = go_string(prev_path);
        let rel = go_string(curr_path);
        let clist = ffi::sass_make_import_list(1);

        let resolver = imports_store().get(ci as i64);
        if let Some(resolver) = resolver {
            let r = catch_unwind(AssertUnwindSafe(|| resolver(&rel, &parent)));
            match r {
                Ok((npath, body, true)) => {
                    let mut bodyv: *mut c_char = ptr::null_mut(); // nil signals loading from the path.
                    if !body.is_empty() {
                        bodyv = c_string(&body);
                    }
                    // sass_make_import_entry copies the path (Go leaks its CString).
                    let npath = CStringBuf::new(&npath);
                    let entry = ffi::sass_make_import_entry(npath.as_ptr(), bodyv, ptr::null_mut());
                    *clist = entry;
                    return clist;
                }
                Ok(_) => {}
                Err(payload) => {
                    RESOLVER_PANIC.with(|p| {
                        let mut p = p.borrow_mut();
                        if p.is_none() {
                            *p = Some(payload);
                        }
                    });
                }
            }
        }

        let ent = ffi::sass_make_import_entry(curr_path, ptr::null_mut(), ptr::null_mut());
        *clist = ent;
        clist
    }
}

// ---------------------------------------------------------------------------
// a__libsass.go

// Go: a__libsass.go:SassCompilerExecute
pub(crate) unsafe fn sass_compiler_execute(compiler: *mut Sass_Compiler) {
    unsafe { ffi::sass_compiler_execute(compiler) };
}

// Go: a__libsass.go:SassCompilerParse
pub(crate) unsafe fn sass_compiler_parse(compiler: *mut Sass_Compiler) {
    unsafe { ffi::sass_compiler_parse(compiler) };
}

// Go: a__libsass.go:SassContextGetErrorJSON
/// Go frees the returned string (and never deletes the context); here the
/// context owns it and frees it in `sass_delete_data_context`.
pub(crate) unsafe fn sass_context_get_error_json(ctx: *mut Sass_Context) -> Vec<u8> {
    unsafe { go_string(ffi::sass_context_get_error_json(ctx)) }
}

// Go: a__libsass.go:SassContextGetErrorStatus
pub(crate) unsafe fn sass_context_get_error_status(ctx: *mut Sass_Context) -> i64 {
    unsafe { ffi::sass_context_get_error_status(ctx) as i64 }
}

// Go: a__libsass.go:SassContextGetOutputString
/// See [`sass_context_get_error_json`] for ownership.
pub(crate) unsafe fn sass_context_get_output_string(ctx: *mut Sass_Context) -> Vec<u8> {
    unsafe { go_string(ffi::sass_context_get_output_string(ctx)) }
}

// Go: a__libsass.go:SassContextGetSourceMapString
pub(crate) unsafe fn sass_context_get_source_map_string(ctx: *mut Sass_Context) -> Vec<u8> {
    unsafe { go_string(ffi::sass_context_get_source_map_string(ctx)) }
}

// Go: a__libsass.go:SassDataContextGetContext
pub(crate) unsafe fn sass_data_context_get_context(
    ctx: *mut Sass_Data_Context,
) -> *mut Sass_Context {
    unsafe { ffi::sass_data_context_get_context(ctx) }
}

// Go: a__libsass.go:SassDataContextGetOptions
pub(crate) unsafe fn sass_data_context_get_options(
    ctx: *mut Sass_Data_Context,
) -> *mut Sass_Options {
    unsafe { ffi::sass_data_context_get_options(ctx) }
}

// Go: a__libsass.go:SassDataContextSetOptions
pub(crate) unsafe fn sass_data_context_set_options(
    ctx: *mut Sass_Data_Context,
    opt: *mut Sass_Options,
) {
    unsafe { ffi::sass_data_context_set_options(ctx, opt) };
}

// Go: a__libsass.go:SassDeleteCompiler
pub(crate) unsafe fn sass_delete_compiler(compiler: *mut Sass_Compiler) {
    unsafe { ffi::sass_delete_compiler(compiler) };
}

// Go: a__libsass.go:SassDeleteDataContext
pub(crate) unsafe fn sass_delete_data_context(ctx: *mut Sass_Data_Context) {
    unsafe { ffi::sass_delete_data_context(ctx) };
}

// Go: a__libsass.go:SassMakeDataCompiler
pub(crate) unsafe fn sass_make_data_compiler(ctx: *mut Sass_Data_Context) -> *mut Sass_Compiler {
    unsafe { ffi::sass_make_data_compiler(ctx) }
}

// Go: a__libsass.go:SassMakeDataContext
pub(crate) fn sass_make_data_context(s: &[u8]) -> *mut Sass_Data_Context {
    unsafe { ffi::sass_make_data_context(c_string(s)) }
}

// Go: a__libsass.go:SassOptionGetSourceMapFile
pub(crate) unsafe fn sass_option_get_source_map_file(opts: *mut Sass_Options) -> Vec<u8> {
    unsafe { go_string(ffi::sass_option_get_source_map_file(opts)) }
}

// Go: a__libsass.go:SassOptionSetIncludePath
pub(crate) unsafe fn sass_option_set_include_path(o: *mut Sass_Options, s: &[u8]) {
    let s = CStringBuf::new(s);
    unsafe { ffi::sass_option_set_include_path(o, s.as_ptr()) };
}

// Go: a__libsass.go:SassOptionSetInputPath
pub(crate) unsafe fn sass_option_set_input_path(o: *mut Sass_Options, s: &[u8]) {
    let s = CStringBuf::new(s);
    unsafe { ffi::sass_option_set_input_path(o, s.as_ptr()) };
}

// Go: a__libsass.go:SassOptionSetOmitSourceMapURL
pub(crate) unsafe fn sass_option_set_omit_source_map_url(o: *mut Sass_Options, b: bool) {
    unsafe { ffi::sass_option_set_omit_source_map_url(o, b) };
}

// Go: a__libsass.go:SassOptionSetOutputPath
pub(crate) unsafe fn sass_option_set_output_path(o: *mut Sass_Options, s: &[u8]) {
    let s = CStringBuf::new(s);
    unsafe { ffi::sass_option_set_output_path(o, s.as_ptr()) };
}

// Go: a__libsass.go:SassOptionSetOutputStyle
/// Go: `C.sass_option_set_output_style(o, uint32(i))`.
pub(crate) unsafe fn sass_option_set_output_style(o: *mut Sass_Options, i: i64) {
    unsafe { ffi::sass_option_set_output_style(o, i as u32) };
}

// Go: a__libsass.go:SassOptionSetPrecision
/// Go: `C.sass_option_set_precision(o, C.int(i))`.
pub(crate) unsafe fn sass_option_set_precision(o: *mut Sass_Options, i: i64) {
    unsafe { ffi::sass_option_set_precision(o, i as c_int) };
}

// Go: a__libsass.go:SassOptionSetSourceComments
pub(crate) unsafe fn sass_option_set_source_comments(o: *mut Sass_Options, b: bool) {
    unsafe { ffi::sass_option_set_source_comments(o, b) };
}

// Go: a__libsass.go:SassOptionSetSourceMapContents
pub(crate) unsafe fn sass_option_set_source_map_contents(o: *mut Sass_Options, b: bool) {
    unsafe { ffi::sass_option_set_source_map_contents(o, b) };
}

// Go: a__libsass.go:SassOptionSetSourceMapEmbed
pub(crate) unsafe fn sass_option_set_source_map_embed(o: *mut Sass_Options, b: bool) {
    unsafe { ffi::sass_option_set_source_map_embed(o, b) };
}

// Go: a__libsass.go:SassOptionSetSourceMapFile
pub(crate) unsafe fn sass_option_set_source_map_file(o: *mut Sass_Options, s: &[u8]) {
    let s = CStringBuf::new(s);
    unsafe { ffi::sass_option_set_source_map_file(o, s.as_ptr()) };
}

// Go: a__libsass.go:SassOptionSetSourceMapRoot
pub(crate) unsafe fn sass_option_set_source_map_root(o: *mut Sass_Options, s: &[u8]) {
    let s = CStringBuf::new(s);
    unsafe { ffi::sass_option_set_source_map_root(o, s.as_ptr()) };
}

// Go: a__libsass.go:SassToScss
/// SassToScss converts Sass to Scss using sass2scss.
pub fn sass_to_scss(src: &[u8]) -> Vec<u8> {
    let input = CStringBuf::new(src);

    unsafe {
        let chars = ffi::sass2scss(input.as_ptr(), 1);
        let out = go_string(chars);
        // Go leaks `chars`; it is malloc'd by sass2scss.
        ffi::sass_free_memory(chars as *mut c_void);
        out
    }
}
