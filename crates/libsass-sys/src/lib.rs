//! Vendored LibSass 3.6.6, compiled exactly like
//! `github.com/bep/golibsass@v1.2.0` compiles it, plus a port of the
//! golibsass Go wrapper (`libsass/transpiler.go`,
//! `libsass/libsasserrors/libsasserrors.go`, `internal/libsass/a__*.go`).
//!
//! Hugo (`resources/resource_transformers/tocss/scss`) uses
//! `libsass.New(options)` + `transpiler.Execute(src)`; the Rust equivalent is
//! [`new`] + [`Transpiler::execute`]:
//!
//! ```
//! use libsass_sys::{new, Options, Transpiler, COMPRESSED_STYLE};
//! let t = new(Options { output_style: COMPRESSED_STYLE, ..Default::default() }).unwrap();
//! let r = t.execute(b"div { color: #ccc; }").unwrap();
//! assert_eq!(r.css, b"div{color:#ccc}\n");
//! ```

pub mod ffi;
pub mod internal;
pub mod libsasserrors;
pub mod transpiler;

pub use internal::{ImportResolver, sass_to_scss};
pub use libsasserrors::{Error, json_to_error};
pub use transpiler::{
    COMPACT_STYLE, COMPRESSED_STYLE, EXPANDED_STYLE, LibsassTranspiler, NESTED_STYLE, Options,
    OutputStyle, SassResult, SourceMapOptions, Transpiler, new, parse_output_style,
};

/// `libsass_version()` (`"[NA]"`: golibsass does not define
/// `LIBSASS_VERSION`).
pub fn libsass_version() -> Vec<u8> {
    unsafe { internal::go_string(ffi::libsass_version()) }
}
