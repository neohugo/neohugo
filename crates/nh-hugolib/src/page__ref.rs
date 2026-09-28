//! Port of `hugolib/page__ref.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `pageRef` (the page's `RefProvider`): `.Ref`/`.RelRef` (source = the page) and
//! `RefFrom`/`RelRefFrom` (source = e.g. the `ShortcodeWithPage` of the `ref` shortcode, whose
//! position ends up in the REF_NOT_FOUND log). The arguments are weakly decoded into
//! `refArgs{Path, Lang, OutputFormat}` (mapstructure; a decode error is ignored and gives the
//! not-found URL); a `lang` of another site resolves the link on that site.

use go_value::{Map, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;

use crate::page::PageHandle;

/// Go: `refArgs`.
#[derive(Clone, Debug, Default)]
pub struct RefArgs {
    pub path: String,
    pub lang: String,
    pub output_format: String,
}

nh_config::decode_struct!(RefArgs, "hugolib.refArgs", |s| vec![
    FieldRef::new("Path", &mut s.path),
    FieldRef::new("Lang", &mut s.lang),
    FieldRef::new("OutputFormat", &mut s.output_format),
]);

/// Go: `pageRef{p}`.
#[derive(Clone)]
pub struct PageRefProvider<'a> {
    pub p: &'a PageHandle,
}

/// Go: `newPageRef(p)`.
// Go: hugolib/page__ref.go:newPageRef
pub fn new_page_ref(p: &PageHandle) -> PageRefProvider<'_> {
    PageRefProvider { p }
}

impl PageRefProvider<'_> {
    /// The page itself as the `source` of `Ref`/`RelRef` (Go passes `p.p`, the `*pageState`).
    fn self_value(&self) -> Value {
        PageHandle {
            h: self.p.h.clone(),
            id: self.p.id,
            wrapper: crate::page::PageWrapper::None,
        }
        .page_ref()
        .to_value()
    }

    /// Go: `Ref(argsm)`.
    // Go: hugolib/page__ref.go:Ref
    pub fn ref_(&self, argsm: &Map) -> Result<String> {
        self.ref_internal(argsm, &self.self_value())
    }

    /// Go: `RefFrom(argsm, source)`.
    // Go: hugolib/page__ref.go:RefFrom
    pub fn ref_from(&self, argsm: &Map, source: &Value) -> Result<String> {
        self.ref_internal(argsm, source)
    }

    /// Go: `RelRef(argsm)`.
    // Go: hugolib/page__ref.go:RelRef
    pub fn rel_ref(&self, argsm: &Map) -> Result<String> {
        self.rel_ref_internal(argsm, &self.self_value())
    }

    /// Go: `RelRefFrom(argsm, source)`.
    // Go: hugolib/page__ref.go:RelRefFrom
    pub fn rel_ref_from(&self, argsm: &Map, source: &Value) -> Result<String> {
        self.rel_ref_internal(argsm, source)
    }

    /// Go: `decodeRefArgs(args)` — the args and the site to resolve them on (`None`: no site
    /// for the language, logged as not found).
    // Go: hugolib/page__ref.go:decodeRefArgs
    pub fn decode_ref_args(&self, args: &Map) -> Result<(RefArgs, Option<usize>)> {
        let mut ra = RefArgs::default();
        if nh_config::decode::weak_decode_into(&Value::map(args.clone()), &mut ra).is_err() {
            // Go: `return ra, nil, nil` (the error is dropped).
            return Ok((ra, None));
        }

        let ps = self.p.state();
        let mut s = ps.site_idx;

        if !ra.lang.is_empty() && ra.lang != self.p.h.sites[ps.site_idx].language.lang {
            // Find correct site
            let mut found = false;
            for (i, ss) in self.p.h.sites.iter().enumerate() {
                if ss.language.lang == ra.lang {
                    found = true;
                    s = i;
                }
            }

            if !found {
                crate::site::log_not_found(
                    &self.p.h,
                    ps.site_idx,
                    &ra.path,
                    &format!("no site found with lang {}", go_strconv::quote(&ra.lang)),
                    None,
                    &nh_common::text::Position::default(),
                );
                return Ok((ra, None));
            }
        }

        Ok((ra, Some(s)))
    }

    /// Go: `ref(argsm, source)`.
    // Go: hugolib/page__ref.go:ref
    fn ref_internal(&self, argsm: &Map, source: &Value) -> Result<String> {
        let (args, s) = self
            .decode_ref_args(argsm)
            .map_err(|err| Error::new(format!("invalid arguments to Ref: {err}")))?;

        let Some(s) = s else {
            return Ok(crate::site::not_found_url(
                &self.p.h,
                self.p.state().site_idx,
            ));
        };

        if args.path.is_empty() {
            return Ok(String::new());
        }

        crate::site::ref_link(&self.p.h, s, &args.path, source, false, &args.output_format)
    }

    /// Go: `relRef(argsm, source)`.
    // Go: hugolib/page__ref.go:relRef
    fn rel_ref_internal(&self, argsm: &Map, source: &Value) -> Result<String> {
        let (args, s) = self
            .decode_ref_args(argsm)
            .map_err(|err| Error::new(format!("invalid arguments to Ref: {err}")))?;

        let Some(s) = s else {
            return Ok(crate::site::not_found_url(
                &self.p.h,
                self.p.state().site_idx,
            ));
        };

        if args.path.is_empty() {
            return Ok(String::new());
        }

        crate::site::ref_link(&self.p.h, s, &args.path, source, true, &args.output_format)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__ref.go (114 lines; 4/8 funcs executed)
//   types: pageRef, refArgs
// OK L24-26: newPageRef(p *pageState) pageRef
// OK L32-34: (p pageRef) Ref(argsm map[string]any) (string, error)
// OK L36-38: (p pageRef) RefFrom(argsm map[string]any, source any) (string, error)
// OK L40-42: (p pageRef) RelRef(argsm map[string]any) (string, error)
// OK L44-46: (p pageRef) RelRefFrom(argsm map[string]any, source any) (string, error)
// OK L48-74: (p pageRef) decodeRefArgs(args map[string]any) (refArgs, *Site, error)
// OK L76-91: (p pageRef) ref(argsm map[string]any, source any) (string, error)
// OK L93-108: (p pageRef) relRef(argsm map[string]any, source any) (string, error)
// ---------------------------------------------------------------------------
