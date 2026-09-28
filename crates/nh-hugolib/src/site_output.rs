//! Port of `hugolib/site_output.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `createDefaultOutputFormats` / `createSiteOutputFormats` (kind -> output formats). Not on
//! the build path (the per-kind formats come from `allconfig`), ported for completeness.

use std::collections::BTreeMap;

use go_value::Value;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_media::output::output_format::{Formats, OutputFormat, builtin_formats};

/// Go: `createDefaultOutputFormats(allFormats)`.
// Go: hugolib/site_output.go:createDefaultOutputFormats
pub fn create_default_output_formats(all_formats: &Formats) -> BTreeMap<String, Formats> {
    let b = builtin_formats();
    let get = |f: &OutputFormat| all_formats.get_by_name(&f.name).unwrap_or_default();
    let rss_out = all_formats.get_by_name(&b.rss.name);
    let html_out = get(&b.html);
    let robots_out = get(&b.robots_txt);
    let sitemap_out = get(&b.sitemap);
    let http_status_404_out = get(&b.http_status_404_html);

    let mut default_list_types = Formats(vec![html_out.clone()]);
    if let Some(rss) = &rss_out {
        default_list_types.0.push(rss.clone());
    }

    let mut m: BTreeMap<String, Formats> = BTreeMap::new();
    m.insert(kinds::KIND_PAGE.into(), Formats(vec![html_out]));
    m.insert(kinds::KIND_HOME.into(), default_list_types.clone());
    m.insert(kinds::KIND_SECTION.into(), default_list_types.clone());
    m.insert(kinds::KIND_TERM.into(), default_list_types.clone());
    m.insert(kinds::KIND_TAXONOMY.into(), default_list_types);
    // Below are for consistency. They are currently not used during rendering.
    m.insert(kinds::KIND_SITEMAP.into(), Formats(vec![sitemap_out]));
    m.insert(kinds::KIND_ROBOTS_TXT.into(), Formats(vec![robots_out]));
    m.insert(
        kinds::KIND_STATUS_404.into(),
        Formats(vec![http_status_404_out]),
    );

    // May be disabled
    if let Some(rss) = rss_out {
        m.insert(kinds::KIND_RSS.into(), Formats(vec![rss]));
    }

    m
}

/// Go: `createSiteOutputFormats(allFormats, outputs, rssDisabled)` — kind -> formats.
// Go: hugolib/site_output.go:createSiteOutputFormats
pub fn create_site_output_formats(
    all_formats: &Formats,
    outputs: &BTreeMap<String, Vec<String>>,
    rss_disabled: bool,
) -> nh_common::Result<BTreeMap<String, Formats>> {
    let outputs: BTreeMap<String, Value> = outputs
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                Value::string_list(v.iter().map(|s| go_value::GoString::from(s.as_str()))),
            )
        })
        .collect();
    create_site_output_formats_values(all_formats, Some(&outputs), rss_disabled)
}

/// `createSiteOutputFormats` with Go's `map[string]any` (`None` is the nil map: the defaults).
/// The entries are handled in key order (Go ranges over the map; the result does not depend on
/// the order, only which error is reported first can).
// Go: hugolib/site_output.go:createSiteOutputFormats
pub fn create_site_output_formats_values(
    all_formats: &Formats,
    outputs: Option<&BTreeMap<String, Value>>,
    rss_disabled: bool,
) -> nh_common::Result<BTreeMap<String, Formats>> {
    let default_output_formats = create_default_output_formats(all_formats);

    let Some(outputs) = outputs else {
        return Ok(default_output_formats);
    };

    let mut out_formats: BTreeMap<String, Formats> = BTreeMap::new();

    if outputs.is_empty() {
        return Ok(out_formats);
    }

    let mut seen: BTreeMap<String, bool> = BTreeMap::new();

    for (k, v) in outputs {
        let k = kinds::get_kind_any(k);
        if k.is_empty() {
            // Invalid kind
            continue;
        }
        let mut formats = Formats(Vec::new());
        let vals = nh_common::cast::caste::to_string_slice(v);
        for format in vals {
            let format = format.to_str_lossy().into_owned();
            match all_formats.get_by_name(&format) {
                Some(f) => formats.0.push(f),
                None => {
                    if rss_disabled && go_unicode::strings::equal_fold_str(&format, "RSS") {
                        // This is legacy behavior. We used to have both
                        // a RSS page kind and output format.
                        continue;
                    }
                    return Err(Error::new(format!(
                        "failed to resolve output format {} from site config",
                        go_strconv::quote(&format)
                    )));
                }
            }
        }

        // This effectively prevents empty outputs entries for a given Kind.
        // We need at least one.
        if !formats.0.is_empty() {
            seen.insert(k.to_string(), true);
            out_formats.insert(k.to_string(), formats);
        }
    }

    // Add defaults for the entries not provided by the user.
    for (k, v) in default_output_formats {
        if !seen.contains_key(&k) {
            out_formats.insert(k, v);
        }
    }

    Ok(out_formats)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/site_output.go (109 lines; 0/2 funcs executed)
// OK L25-55: createDefaultOutputFormats(allFormats output.Formats) map[string]output.Formats
// OK L57-109: createSiteOutputFormats(allFormats output.Formats, outputs map[string]any, rssDisabled bool) (map[string]output.Formats, error)
// ---------------------------------------------------------------------------
