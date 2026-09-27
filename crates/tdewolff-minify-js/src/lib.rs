//! Byte-exact Rust port of `github.com/tdewolff/minify/v2@v2.23.8/js`, the
//! JavaScript minifier (ECMAScript 2021+): variable hoisting and renaming,
//! statement-list and expression optimizations, and the printer.
//!
//! It walks the AST of `tdewolff-parse-js` (an arena of nodes addressed by
//! `NodeId`) and mutates it in place exactly where Go does; Go `[]byte`
//! values are `GoBytes`, so in-place rewrites of the input buffer (the
//! renamer's `name[0] = c`, string quote rewriting, number minification,
//! the unrestored NUL sentinel) are observable exactly as in Go.
//!
//! The [`Minifier`] implements `tdewolff_minify::Minifier`, so it plugs into
//! `tdewolff_minify::M` like the other minifiers:
//!
//! ```
//! use std::sync::Arc;
//! use tdewolff_minify::{M, Regexp};
//!
//! let mut m = M::new();
//! let js = Arc::new(tdewolff_minify_js::Minifier { version: 2022, ..Default::default() });
//! m.add("text/javascript", js.clone());
//! m.add_regexp(Regexp::must_compile("^(application|text)/(x-)?(java|ecma)script$"), js);
//! let out = m.minify_bytes("text/javascript", b"var a = 1 + 2 ;").unwrap();
//! assert_eq!(out, b"var a=1+2");
//! ```
//!
//! See `PORTING.md` for the module map and deviations.

// Faithful-port allowances: Go names for constants (`OpExpr`, `NoDecl`, ...),
// Go-style control flow (explicit returns out of `switch` cases, long
// boolean chains mirroring Go's precedence, index loops over lists that are
// mutated through the arena).
#![allow(non_upper_case_globals)]
#![allow(clippy::needless_return)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::precedence)]
#![allow(clippy::len_zero)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::single_match)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::impossible_comparisons)] // Go's `b[r] < '0' && '7' < b[r]` (always false) kept verbatim
#![allow(clippy::manual_strip)]

mod js;
mod stmtlist;
mod util;
mod vars;

#[cfg(test)]
mod unit_tests;

pub use tdewolff_minify::{GoBytes, GoError, GoReader, M, Params, Writer};

/// Go: js.go:Minifier — a JS minifier.
///
/// Field names are Go's in snake_case; the zero value (`Default`) is Go's
/// zero value. neohugo configures `Minifier { version: 2022, ..Default::default() }`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Minifier {
    /// Go `Precision int`: number of significant digits.
    pub precision: i64,
    /// Go `KeepVarNames bool`.
    pub keep_var_names: bool,
    /// Go's unexported `useAlphabetVarNames` (only the upstream tests set
    /// it). Public so the ported tests can construct it; not part of Go's API.
    #[doc(hidden)]
    pub use_alphabet_var_names: bool,
    /// Go `Version int`: the ECMAScript version targeted (0 = latest).
    pub version: i64,
}

impl Minifier {
    // Go: js.go:Minifier.minVersion
    pub(crate) fn min_version(&self, version: i64) -> bool {
        self.version == 0 || version <= self.version
    }

    // Go: js.go:Minifier.Minify
    /// Minifies JS data, it reads from `r` and writes to `w`. `params` is
    /// `None` for a nil map; `params["inline"] == "1"` parses in inline mode
    /// (HTML event attributes: a top-level `return` is allowed).
    ///
    /// The minifier works in place on the reader's bytes (when it has a
    /// `Bytes()` method, like `tdewolff_parse::buffer::Reader`), exactly as Go.
    /// Write errors are only reported by the final `w.Write(nil)`, as in Go.
    pub fn minify(
        &self,
        m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        params: Option<&Params>,
    ) -> Result<(), GoError> {
        js::minify(self, m, w, r, params)
    }

    /// Convenience: minifies a private copy of `input` (Go `parse.Copy`
    /// capacity) into a `Vec<u8>`.
    pub fn minify_bytes(&self, input: &[u8], params: Option<&Params>) -> Result<Vec<u8>, GoError> {
        let mut out = Vec::with_capacity(input.len());
        let mut r = tdewolff_parse::buffer::Reader::new(GoBytes::from_slice(input));
        self.minify(&M::new(), &mut out, &mut r, params)?;
        Ok(out)
    }
}

impl tdewolff_minify::Minifier for Minifier {
    fn minify(
        &self,
        m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        params: Option<&Params>,
    ) -> Result<(), GoError> {
        js::minify(self, m, w, r, params)
    }
}

// Go: js.go:Minify
/// Minifies JS data with the default options, it reads from `r` and writes
/// to `w`.
pub fn minify(
    m: &M,
    w: &mut dyn Writer,
    r: &mut dyn GoReader,
    params: Option<&Params>,
) -> Result<(), GoError> {
    Minifier::default().minify(m, w, r, params)
}
