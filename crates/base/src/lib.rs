//! Shared vocabulary of the neohugo rewrite (docs/rust-port/REWRITE_PLAN.md §2.4): typed ids and
//! [`IdVec`], [`PageKind`]/[`KindSet`], [`Value`]/[`Map`]/[`Params`]/[`Date`], the path
//! newtypes, URLs, anchors, inflection and title case, globs, date parsing, diagnostics,
//! [`Collate`] and [`Sink`].

#![forbid(unsafe_code)]

pub mod anchor;
pub mod diag;
pub mod glob;
pub mod id;
pub mod inflect;
pub mod kind;
pub mod params;
pub mod paths;
pub mod text;
pub mod time;
pub mod title;
pub mod url;
pub mod value;

pub use id::{
    FormatId, FrameId, IdVec, Idx, ImageOpId, LangIdx, MediaTypeId, PageId, ResourceId,
    TaxonomyIdx, TermIdx, TxnId,
};
pub use kind::{KindSet, PageKind};
pub use params::Params;
pub use paths::{ContentKey, OutputPath, Permalink, TermKey, UrlPath};
pub use time::{Clock, DateError, parse_date};
pub use value::{Date, Map, Value};

/// Compares strings in a language's collation order (implemented with ICU in
/// `neohugo-locale`).
pub trait Collate: Send + Sync {
    /// The collation order of `a` and `b`.
    fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering;
}

/// Byte order: the collation used when a language has none.
#[derive(Clone, Copy, Debug, Default)]
pub struct ByteOrder;

impl Collate for ByteOrder {
    fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering {
        a.cmp(b)
    }
}

/// Where rendered files go: the publish directory, or memory for `serve` and tests.
pub trait Sink: Send + Sync {
    /// Writes (or replaces) the file at `path`.
    ///
    /// # Errors
    /// I/O errors of the destination.
    fn write(&self, path: &OutputPath, bytes: &[u8]) -> std::io::Result<()>;
    /// Whether a file was written at `path`.
    fn exists(&self, path: &OutputPath) -> bool;
}
