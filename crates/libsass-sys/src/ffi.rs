//! Raw FFI declarations for the parts of the LibSass 3.6.6 C API
//! (`include/sass/{base,context,functions}.h`, `include/sass2scss.h`) that
//! golibsass uses.

#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_double, c_int, c_uint, c_void};

#[repr(C)]
pub struct Sass_Options {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Sass_Context {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Sass_Data_Context {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Sass_Compiler {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Sass_Import {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Sass_Importer {
    _private: [u8; 0],
}

pub type Sass_Import_Entry = *mut Sass_Import;
pub type Sass_Import_List = *mut Sass_Import_Entry;
pub type Sass_Importer_Entry = *mut Sass_Importer;
pub type Sass_Importer_List = *mut Sass_Importer_Entry;

/// `typedef Sass_Import_List (*Sass_Importer_Fn)(const char* url,
/// Sass_Importer_Entry cb, struct Sass_Compiler* compiler);`
pub type Sass_Importer_Fn = Option<
    unsafe extern "C" fn(
        url: *const c_char,
        cb: Sass_Importer_Entry,
        compiler: *mut Sass_Compiler,
    ) -> Sass_Import_List,
>;

/// `enum Sass_Output_Style` (cgo: `uint32`).
pub type Sass_Output_Style = c_uint;
pub const SASS_STYLE_NESTED: Sass_Output_Style = 0;
pub const SASS_STYLE_EXPANDED: Sass_Output_Style = 1;
pub const SASS_STYLE_COMPACT: Sass_Output_Style = 2;
pub const SASS_STYLE_COMPRESSED: Sass_Output_Style = 3;

unsafe extern "C" {
    // base.h
    pub fn sass_alloc_memory(size: usize) -> *mut c_void;
    pub fn sass_copy_c_string(str: *const c_char) -> *mut c_char;
    pub fn sass_free_memory(ptr: *mut c_void);
    pub fn libsass_version() -> *const c_char;
    pub fn libsass_language_version() -> *const c_char;

    // context.h
    pub fn sass_make_data_context(source_string: *mut c_char) -> *mut Sass_Data_Context;
    pub fn sass_make_data_compiler(data_ctx: *mut Sass_Data_Context) -> *mut Sass_Compiler;
    pub fn sass_compiler_parse(compiler: *mut Sass_Compiler) -> c_int;
    pub fn sass_compiler_execute(compiler: *mut Sass_Compiler) -> c_int;
    pub fn sass_delete_compiler(compiler: *mut Sass_Compiler);
    pub fn sass_delete_data_context(ctx: *mut Sass_Data_Context);
    pub fn sass_data_context_get_context(data_ctx: *mut Sass_Data_Context) -> *mut Sass_Context;
    pub fn sass_data_context_get_options(data_ctx: *mut Sass_Data_Context) -> *mut Sass_Options;
    pub fn sass_data_context_set_options(data_ctx: *mut Sass_Data_Context, opt: *mut Sass_Options);

    pub fn sass_option_set_precision(options: *mut Sass_Options, precision: c_int);
    pub fn sass_option_set_output_style(
        options: *mut Sass_Options,
        output_style: Sass_Output_Style,
    );
    pub fn sass_option_set_source_comments(options: *mut Sass_Options, source_comments: bool);
    pub fn sass_option_set_source_map_embed(options: *mut Sass_Options, source_map_embed: bool);
    pub fn sass_option_set_source_map_contents(
        options: *mut Sass_Options,
        source_map_contents: bool,
    );
    pub fn sass_option_set_omit_source_map_url(
        options: *mut Sass_Options,
        omit_source_map_url: bool,
    );
    pub fn sass_option_set_input_path(options: *mut Sass_Options, input_path: *const c_char);
    pub fn sass_option_set_output_path(options: *mut Sass_Options, output_path: *const c_char);
    pub fn sass_option_set_include_path(options: *mut Sass_Options, include_path: *const c_char);
    pub fn sass_option_set_source_map_file(
        options: *mut Sass_Options,
        source_map_file: *const c_char,
    );
    pub fn sass_option_set_source_map_root(
        options: *mut Sass_Options,
        source_map_root: *const c_char,
    );
    pub fn sass_option_set_c_importers(options: *mut Sass_Options, c_importers: Sass_Importer_List);
    pub fn sass_option_get_source_map_file(options: *mut Sass_Options) -> *const c_char;

    pub fn sass_context_get_output_string(ctx: *mut Sass_Context) -> *const c_char;
    pub fn sass_context_get_error_status(ctx: *mut Sass_Context) -> c_int;
    pub fn sass_context_get_error_json(ctx: *mut Sass_Context) -> *const c_char;
    pub fn sass_context_get_source_map_string(ctx: *mut Sass_Context) -> *const c_char;

    pub fn sass_compiler_get_last_import(compiler: *mut Sass_Compiler) -> Sass_Import_Entry;

    // functions.h
    pub fn sass_make_importer_list(length: usize) -> Sass_Importer_List;
    pub fn sass_importer_set_list_entry(
        list: Sass_Importer_List,
        idx: usize,
        entry: Sass_Importer_Entry,
    );
    pub fn sass_make_importer(
        importer: Sass_Importer_Fn,
        priority: c_double,
        cookie: *mut c_void,
    ) -> Sass_Importer_Entry;
    pub fn sass_importer_get_cookie(cb: Sass_Importer_Entry) -> *mut c_void;
    pub fn sass_make_import_list(length: usize) -> Sass_Import_List;
    pub fn sass_make_import_entry(
        path: *const c_char,
        source: *mut c_char,
        srcmap: *mut c_char,
    ) -> Sass_Import_Entry;
    pub fn sass_import_get_imp_path(entry: Sass_Import_Entry) -> *const c_char;
    /// Not used by golibsass; aborts a compile after a resolver panic (see
    /// `internal::bridge_import`).
    pub fn sass_import_set_error(
        import: Sass_Import_Entry,
        message: *const c_char,
        line: usize,
        col: usize,
    ) -> Sass_Import_Entry;

    // sass2scss.h
    pub fn sass2scss(sass: *const c_char, options: c_int) -> *mut c_char;
}
