//! Port of `hugolib/segments/segments.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).


//! Go `hugolib/segments` (`--renderSegments`; unused -> no filtering).

use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::Map;
use nh_common::Result;
use nh_config::namespace::ConfigNamespace;

/// Go: `segments.SegmentMatcherFields`.
#[derive(Clone, Debug, Default)]
pub struct SegmentMatcherFields {
    pub kind: String,
    pub lang: String,
    pub path: String,
    pub output: String,
}

/// Go: `segments.SegmentFilter` — nil filter excludes nothing.
#[derive(Clone, Default)]
pub struct SegmentFilter {
    pub(crate) exclude: Option<Arc<dyn Fn(&SegmentMatcherFields) -> bool + Send + Sync>>,
}

impl SegmentFilter {
    // Go: hugolib/segments/segments.go:ShouldExcludeCoarse
    pub fn should_exclude_coarse(&self, f: &SegmentMatcherFields) -> bool {
        false
    }
    // Go: hugolib/segments/segments.go:ShouldExcludeFine
    pub fn should_exclude_fine(&self, f: &SegmentMatcherFields) -> bool {
        false
    }
}

/// Go: `segments.Segments`.
pub type Segments = BTreeMap<String, Map>;

// Go: hugolib/segments/segments.go:DecodeSegments
pub fn decode_segments(input: &Map) -> Result<ConfigNamespace<Map, Segments>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/segments/segments.go (257 lines; 4/8 funcs executed)
//   types: Segments, excludeInclude, SegmentFilter, segmentFilter, SegmentConfig, SegmentMatcherFields
//    L42-44: (e excludeInclude) ShouldExcludeCoarse(fields SegmentMatcherFields) bool
//    L48-53: (e excludeInclude) ShouldExcludeFine(fields SegmentMatcherFields) bool
// EX L68-70: (f segmentFilter) ShouldExcludeCoarse(field SegmentMatcherFields) bool
// EX L72-74: (f segmentFilter) ShouldExcludeFine(fields SegmentMatcherFields) bool
// EX L82-112: (sms Segments) Get(onNotFound func(s string), ss ...string) SegmentFilter
//    L128-137: getGlob(s string) (glob.Glob, error)
//    L139-201: compileSegments(f []SegmentMatcherFields) (predicate.P[SegmentMatcherFields], error)
// EX L203-257: DecodeSegments(in map[string]any) (*config.ConfigNamespace[map[string]SegmentConfig, Segments], error)
// ---------------------------------------------------------------------------
