//! Port of `hugolib/site_output.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).


use std::collections::BTreeMap;

use nh_media::output::output_format::Formats;

/// Go: `createSiteOutputFormats(allFormats, outputs, rssDisabled)` — kind -> formats.
// Go: hugolib/site_output.go:createSiteOutputFormats
pub fn create_site_output_formats(all_formats: &Formats, outputs: &BTreeMap<String, Vec<String>>, rss_disabled: bool) -> nh_common::Result<BTreeMap<String, Formats>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/site_output.go (109 lines; 0/2 funcs executed)
//    L25-55: createDefaultOutputFormats(allFormats output.Formats) map[string]output.Formats
//    L57-109: createSiteOutputFormats(allFormats output.Formats, outputs map[string]any, rssDisabled bool) (map[string]output.Formats, error)
// ---------------------------------------------------------------------------
