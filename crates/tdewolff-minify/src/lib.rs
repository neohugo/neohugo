//! Byte-exact Rust port of `github.com/tdewolff/minify/v2@v2.23.8` (all
//! minifiers except `js`, which lives in the `tdewolff-minify-js` crate and
//! plugs into [`M`] through the [`Minifier`] trait like any other minifier).
//!
//! Go `[]byte` values are [`GoBytes`] (shared backing arrays with Go
//! slice/append/aliasing semantics, from `tdewolff-parse`), because the
//! minifiers rewrite their input in place and later reads must observe those
//! writes exactly as in Go. See `PORTING.md` for the module map and
//! deviations.

// Faithful-port allowances: Go names (`css::Font_Face`, `html::Http_Equiv`),
// Go-style index loops, long boolean chains and `break` out of `switch`
// modelled with labelled blocks.
#![allow(non_upper_case_globals)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::len_zero)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::needless_return)]
#![allow(clippy::precedence)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::type_complexity)]
#![allow(clippy::new_without_default)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::single_match)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::absurd_extreme_comparisons)] // Go's `MinInt <= x && x <= MaxInt` range checks
#![allow(clippy::needless_bool)]
#![allow(clippy::needless_bool_assign)]
#![allow(clippy::assign_op_pattern)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::collapsible_match)]

pub mod common;
pub mod css;
pub mod html;
pub mod json;
pub mod svg;
pub mod xml;

mod tokbuf;

use std::collections::HashMap;
use std::sync::Arc;

pub use common::{
    EPSILON, MAX_INT, MIN_INT, base64_std_encode, data_uri, decimal, mediatype, number,
    update_error_position,
};
pub use tdewolff_parse::{GoBytes, GoError, GoReader, Params};

/// Go: minify.go:ErrNotExist's message.
pub const ERR_NOT_EXIST_MSG: &[u8] = b"minifier does not exist for mimetype";

/// Go: minify.go:ErrNotExist — returned when no minifier exists for a given
/// mimetype. Compare with [`is_err_not_exist`].
pub fn err_not_exist() -> GoError {
    GoError::Other(ERR_NOT_EXIST_MSG.to_vec())
}

/// `err == minify.ErrNotExist`
pub fn is_err_not_exist(err: &GoError) -> bool {
    matches!(err, GoError::Other(m) if m.as_slice() == ERR_NOT_EXIST_MSG)
}

/// A Go `io.Writer`. Write errors are Go `error` values.
pub trait Writer {
    /// `Write(p []byte) (n int, err error)`
    fn write(&mut self, b: &[u8]) -> Result<usize, GoError>;

    /// `Write` of a Go slice (read completely before writing).
    fn write_go(&mut self, b: &GoBytes) -> Result<usize, GoError> {
        let v = b.to_vec();
        self.write(&v)
    }
}

impl Writer for Vec<u8> {
    fn write(&mut self, b: &[u8]) -> Result<usize, GoError> {
        self.extend_from_slice(b);
        Ok(b.len())
    }

    fn write_go(&mut self, b: &GoBytes) -> Result<usize, GoError> {
        b.write_to(self);
        Ok(b.len())
    }
}

impl Writer for tdewolff_parse::buffer::Writer {
    fn write(&mut self, b: &[u8]) -> Result<usize, GoError> {
        match tdewolff_parse::buffer::Writer::write(self, b) {
            (n, None) => Ok(n),
            (_, Some(e)) => Err(e),
        }
    }
}

/// Go: minify.go:Minifier — the interface for minifiers. The `&M` parameter
/// is used for minifying embedded resources, such as JS within HTML.
pub trait Minifier: Send + Sync {
    /// `Minify(*M, io.Writer, io.Reader, map[string]string) error`; `params`
    /// is `None` for a nil map.
    fn minify(
        &self,
        m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        params: Option<&Params>,
    ) -> Result<(), GoError>;
}

/// Go: minify.go:MinifierFunc — a function that implements [`Minifier`].
pub struct MinifierFunc<F>(pub F);

impl<F> Minifier for MinifierFunc<F>
where
    F: Fn(&M, &mut dyn Writer, &mut dyn GoReader, Option<&Params>) -> Result<(), GoError>
        + Send
        + Sync,
{
    fn minify(
        &self,
        m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        params: Option<&Params>,
    ) -> Result<(), GoError> {
        (self.0)(m, w, r, params)
    }
}

/// Go `*regexp.Regexp` as used by `M.AddRegexp`: an unanchored search over
/// the mimetype bytes (see `PORTING.md` for the engine choice).
#[derive(Clone, Debug)]
pub struct Regexp {
    re: regex::bytes::Regex,
    src: String,
}

impl Regexp {
    /// Go `regexp.Compile`.
    pub fn compile(expr: &str) -> Result<Regexp, regex::Error> {
        Ok(Regexp {
            re: regex::bytes::Regex::new(expr)?,
            src: expr.to_string(),
        })
    }

    /// Go `regexp.MustCompile`.
    pub fn must_compile(expr: &str) -> Regexp {
        match Regexp::compile(expr) {
            Ok(r) => r,
            Err(e) => panic!("regexp: Compile({:?}): {}", expr, e),
        }
    }

    /// `(*Regexp).Match(b)`
    pub fn is_match(&self, b: &[u8]) -> bool {
        self.re.is_match(b)
    }

    /// `(*Regexp).String()`
    pub fn as_str(&self) -> &str {
        &self.src
    }
}

/// The parts of Go's `*url.URL` that the minifiers read (`M.URL`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Url {
    pub scheme: String,
}

/// Go: minify.go:M — holds a map of mimetype => function to allow recursive
/// minifier calls of the minifier functions.
pub struct M {
    literal: HashMap<Vec<u8>, Arc<dyn Minifier>>,
    pattern: Vec<(Regexp, Arc<dyn Minifier>)>,

    /// Go `M.URL` (nil = `None`); only `Scheme` is used (HTML URL attributes).
    pub url: Option<Url>,
}

impl M {
    // Go: minify.go:New
    /// Returns a new M.
    pub fn new() -> M {
        M {
            literal: HashMap::new(),
            pattern: Vec::new(),
            url: None,
        }
    }

    // Go: minify.go:M.Add
    /// Adds a minifier to the mimetype => function map.
    pub fn add(&mut self, mimetype: impl AsRef<[u8]>, minifier: Arc<dyn Minifier>) {
        self.literal.insert(mimetype.as_ref().to_vec(), minifier);
    }

    // Go: minify.go:M.AddFunc
    /// Adds a minify function to the mimetype => function map.
    pub fn add_func<F>(&mut self, mimetype: impl AsRef<[u8]>, minifier: F)
    where
        F: Fn(&M, &mut dyn Writer, &mut dyn GoReader, Option<&Params>) -> Result<(), GoError>
            + Send
            + Sync
            + 'static,
    {
        self.add(mimetype, Arc::new(MinifierFunc(minifier)));
    }

    // Go: minify.go:M.AddRegexp
    /// Adds a minifier to the mimetype => function map (patterns are tried in
    /// insertion order after the literal map).
    pub fn add_regexp(&mut self, pattern: Regexp, minifier: Arc<dyn Minifier>) {
        self.pattern.push((pattern, minifier));
    }

    // Go: minify.go:M.AddFuncRegexp
    /// Adds a minify function to the mimetype => function map.
    pub fn add_func_regexp<F>(&mut self, pattern: Regexp, minifier: F)
    where
        F: Fn(&M, &mut dyn Writer, &mut dyn GoReader, Option<&Params>) -> Result<(), GoError>
            + Send
            + Sync
            + 'static,
    {
        self.add_regexp(pattern, Arc::new(MinifierFunc(minifier)));
    }

    // Go: minify.go:M.Match
    /// Returns the pattern (or mimetype) and minifier that gets matched with
    /// the mediatype, plus the mediatype's parameters. The minifier is `None`
    /// when no matching minifier exists. It has the same matching algorithm
    /// as [`M::minify`].
    pub fn match_(
        &self,
        mediatype: impl AsRef<[u8]>,
    ) -> (Vec<u8>, Option<Params>, Option<Arc<dyn Minifier>>) {
        let b = GoBytes::from_slice(mediatype.as_ref());
        let (mimetype, params) = tdewolff_parse::mediatype(&b);
        let mimetype = mimetype.to_vec();
        if let Some(minifier) = self.literal.get(&mimetype) {
            return (mimetype, params, Some(minifier.clone()));
        }
        for (pattern, minifier) in &self.pattern {
            if pattern.is_match(&mimetype) {
                return (
                    pattern.as_str().as_bytes().to_vec(),
                    params,
                    Some(minifier.clone()),
                );
            }
        }
        (mimetype, params, None)
    }

    // Go: minify.go:M.Minify
    /// Minifies the content of a Reader and writes it to a Writer. An error
    /// is returned when no such mimetype exists ([`err_not_exist`]) or when an
    /// error occurred in the minifier function. Mediatype may take the form of
    /// 'text/plain' or 'text/plain; charset=UTF-8; version=2.0'.
    pub fn minify(
        &self,
        mediatype: impl AsRef<[u8]>,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
    ) -> Result<(), GoError> {
        let b = GoBytes::from_slice(mediatype.as_ref());
        let (mimetype, params) = tdewolff_parse::mediatype(&b);
        self.minify_mimetype(&mimetype.to_vec(), w, r, params.as_ref())
    }

    // Go: minify.go:M.MinifyMimetype
    /// Lower level version of [`M::minify`] that requires the mediatype to be
    /// split up into mimetype and parameters.
    pub fn minify_mimetype(
        &self,
        mimetype: &[u8],
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        params: Option<&Params>,
    ) -> Result<(), GoError> {
        if let Some(minifier) = self.literal.get(mimetype) {
            return minifier.minify(self, w, r, params);
        }
        for (pattern, minifier) in &self.pattern {
            if pattern.is_match(mimetype) {
                return minifier.minify(self, w, r, params);
            }
        }
        Err(err_not_exist())
    }

    // Go: minify.go:M.Bytes
    /// Minifies an array of bytes. When an error occurs it returns the
    /// original array (which the minifier may have rewritten in place, as in
    /// Go) and the error.
    pub fn bytes(&self, mediatype: impl AsRef<[u8]>, v: GoBytes) -> (GoBytes, Option<GoError>) {
        let mut out = tdewolff_parse::buffer::Writer::new(GoBytes::make(0, v.len()));
        let mut r = tdewolff_parse::buffer::Reader::new(v.clone());
        if let Err(err) = self.minify(mediatype, &mut out, &mut r) {
            return (v, Some(err));
        }
        (out.bytes(), None)
    }

    // Go: minify.go:M.String
    /// Minifies a string. When an error occurs it returns the original string
    /// and the error.
    pub fn string(&self, mediatype: impl AsRef<[u8]>, v: &[u8]) -> (Vec<u8>, Option<GoError>) {
        let mut out = tdewolff_parse::buffer::Writer::new(GoBytes::make(0, v.len()));
        let mut r = tdewolff_parse::buffer::Reader::new(GoBytes::from_slice(v));
        if let Err(err) = self.minify(mediatype, &mut out, &mut r) {
            return (v.to_vec(), Some(err));
        }
        (out.bytes().to_vec(), None)
    }

    /// Convenience: minifies `v` (a private copy) with the given mediatype
    /// into a `Vec<u8>`.
    pub fn minify_bytes(&self, mediatype: impl AsRef<[u8]>, v: &[u8]) -> Result<Vec<u8>, GoError> {
        let mut out = Vec::with_capacity(v.len());
        let mut r = tdewolff_parse::buffer::Reader::new(GoBytes::from_slice(v));
        self.minify(mediatype, &mut out, &mut r)?;
        Ok(out)
    }
}

/// Restores the byte overwritten by the NUL sentinel of a `parse.Input`
/// when dropped (Go: `defer z.Restore()`). Minifiers ported in other crates
/// (js) use it the same way.
pub struct RestoreGuard(pub tdewolff_parse::Input);

impl Drop for RestoreGuard {
    fn drop(&mut self) {
        self.0.restore();
    }
}

/// `params["key"] == "value"` on a possibly-nil Go map.
pub fn param_is(params: Option<&Params>, key: &[u8], value: &[u8]) -> bool {
    match params {
        Some(p) => p.get(key).map(|v| v.as_slice() == value).unwrap_or(false),
        None => false,
    }
}

/// A fresh `map[string]string{k: v}`.
pub(crate) fn params1(key: &[u8], value: &[u8]) -> Params {
    let mut p = Params::new();
    p.insert(key.to_vec(), value.to_vec());
    p
}
