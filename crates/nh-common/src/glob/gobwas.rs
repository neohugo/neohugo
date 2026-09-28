//! Port of `github.com/gobwas/glob@v0.2.3` (`glob.go`, `syntax`, `compiler`, `match`, `util`):
//! the glob library behind Hugo's `hugofs/glob`, `maps.KeyRenamer`, page matchers and
//! `resources.Match`.
//!
//! Owner: Wave B task T02 (common-paths-text).
//!
//! The compiled matcher tree is Go's (`Matcher::string` prints Go's `String()` form, which the
//! oracle compares), so matching reproduces the upstream behaviour, bugs included.

pub mod ast;
pub mod compiler;
pub mod lexer;
pub mod matcher;

pub use lexer::Rune;
pub use matcher::Matcher;

/// Go: `glob.Compile(pattern, separators...)` — the error is Go's error text.
// Go: github.com/gobwas/glob glob.go:Compile
pub fn compile(pattern: &[u8], separators: &[Rune]) -> Result<Matcher, String> {
    let ast = ast::parse(pattern)?;

    compiler::compile_tree(&ast, separators)
}

/// Go: `glob.QuoteMeta(s)` — quotes all glob pattern meta characters.
// Go: github.com/gobwas/glob glob.go:QuoteMeta
pub fn quote_meta(s: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(2 * s.len());

    // a byte loop is correct because all meta characters are ASCII
    for &c in s {
        if lexer::special(c) {
            b.push(b'\\');
        }
        b.push(c);
    }

    b
}

/// Go: `syntax.Special(b)`.
// Go: github.com/gobwas/glob syntax/syntax.go:Special
pub fn special(b: u8) -> bool {
    lexer::special(b)
}
