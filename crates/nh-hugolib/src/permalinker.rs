//! Port of `hugolib/permalinker.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

/// Go: `Permalinker` — permalinks of both the relative and absolute kind (a page, or one of
/// its output formats in `refLink`).
pub trait Permalinker {
    fn permalink(&self) -> String;
    fn rel_permalink(&self) -> String;
}

impl Permalinker for crate::page::PageHandle {
    fn permalink(&self) -> String {
        nh_resource::resourcetypes::Resource::permalink(self)
    }
    fn rel_permalink(&self) -> String {
        nh_resource::resourcetypes::Resource::rel_permalink(self)
    }
}

impl Permalinker for nh_page::page_outputformat::OutputFormat {
    fn permalink(&self) -> String {
        nh_page::page_outputformat::OutputFormat::permalink(self).to_string()
    }
    fn rel_permalink(&self) -> String {
        nh_page::page_outputformat::OutputFormat::rel_permalink(self).to_string()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/permalinker.go (22 lines; 0/0 funcs executed)
//   types: Permalinker
// ---------------------------------------------------------------------------
