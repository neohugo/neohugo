//! Port of golang.org/x/text/internal/colltab/weighter.go.

use super::collelem::Elem;

/// A Weighter can be used as a source for a Collator (Go `colltab.Weighter`).
///
/// Go's interface also has `Start`, `StartString` and `Domain`, which
/// `colltab.Table` does not implement (they panic); they are omitted. The
/// `[]byte`/`string` method pairs collapse into one `&[u8]` method. The
/// `Send + Sync` bound lets a `Collator` move between threads (Go callers
/// guard it with a mutex; Rust callers use `&mut`).
pub trait Weighter: Send + Sync {
    /// AppendNext appends Elems to `buf` corresponding to the longest match of
    /// a single character or contraction from the start of `s`, and returns
    /// the number of bytes consumed.
    ///
    /// Go returns a new slice; here `buf[..len_on_entry]` is the Go `buf`
    /// argument and `buf` on return is the Go result `ce` (it may also have
    /// been shortened, mirroring Go code that re-slices `buf`).
    fn append_next(&self, buf: &mut Vec<Elem>, s: &[u8]) -> usize;

    /// Top returns the highest variable primary value.
    fn top(&self) -> u32;
}
