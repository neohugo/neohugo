//! Markdown through comrak behind an engine-neutral API, plus Hugo's passes: heading anchors, TOC and fragments, summaries, word count, the `Hooks` and `Highlighter` traits and source-context spans.
//!
//! Stub written by T00 (docs/rust-port/REWRITE_PLAN.md §2.1); no API yet. The T04 comrak spike
//! (engine decision, per-feature measurements, the custom passes T22 must write) is in the
//! crate `README.md`; its harness is `tests/it/comrak_spike`.

#![forbid(unsafe_code)]
