//! Port of `hugolib/segments/segments.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

//! Go `hugolib/segments` (`--renderSegments`; unused -> no filtering).

use std::collections::BTreeMap;

use go_value::{Map, MapType, Value};
use nh_common::predicate::{self, P};
use nh_common::{Error, Result};
use nh_config::decode::FieldRef;
use nh_config::decode_struct;
use nh_config::namespace::ConfigNamespace;

/// Go: `segments.SegmentMatcherFields` — a matcher for a segment include or exclude. All of
/// these are Glob patterns.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SegmentMatcherFields {
    pub kind: String,
    pub lang: String,
    pub path: String,
    pub output: String,
}

decode_struct!(
    SegmentMatcherFields,
    "segments.SegmentMatcherFields",
    |s| vec![
        FieldRef::new("Kind", &mut s.kind),
        FieldRef::new("Path", &mut s.path),
        FieldRef::new("Lang", &mut s.lang),
        FieldRef::new("Output", &mut s.output),
    ]
);

/// Go: `segments.SegmentConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SegmentConfig {
    pub excludes: Vec<SegmentMatcherFields>,
    pub includes: Vec<SegmentMatcherFields>,
}

decode_struct!(SegmentConfig, "segments.SegmentConfig", |s| vec![
    FieldRef::new("Excludes", &mut s.excludes),
    FieldRef::new("Includes", &mut s.includes),
]);

type Pred = P<SegmentMatcherFields>;

/// Go: `segments.excludeInclude`.
#[derive(Clone, Default)]
struct ExcludeInclude {
    exclude: Option<Pred>,
    include: Option<Pred>,
}

impl ExcludeInclude {
    /// ShouldExcludeCoarse returns whether the given fields should be excluded. This is used
    /// for the coarser grained checks, e.g. language and output format.
    // Go: hugolib/segments/segments.go:(excludeInclude).ShouldExcludeCoarse
    fn should_exclude_coarse(&self, fields: &SegmentMatcherFields) -> bool {
        self.exclude.as_ref().is_some_and(|e| e(fields))
    }

    /// ShouldExcludeFine returns whether the given fields should be excluded. This is used for
    /// the finer grained checks, e.g. on individual pages.
    // Go: hugolib/segments/segments.go:(excludeInclude).ShouldExcludeFine
    fn should_exclude_fine(&self, fields: &SegmentMatcherFields) -> bool {
        if self.exclude.as_ref().is_some_and(|e| e(fields)) {
            return true;
        }
        self.include.as_ref().is_some_and(|i| !i(fields))
    }
}

/// Go: `segments.SegmentFilter` (the `segmentFilter` implementation). The default filter
/// excludes nothing.
#[derive(Clone)]
pub struct SegmentFilter {
    coarse: Pred,
    fine: Pred,
}

impl Default for SegmentFilter {
    fn default() -> Self {
        SegmentFilter {
            coarse: match_nothing(),
            fine: match_nothing(),
        }
    }
}

impl SegmentFilter {
    // Go: hugolib/segments/segments.go:ShouldExcludeCoarse
    pub fn should_exclude_coarse(&self, f: &SegmentMatcherFields) -> bool {
        (self.coarse)(f)
    }
    // Go: hugolib/segments/segments.go:ShouldExcludeFine
    pub fn should_exclude_fine(&self, f: &SegmentMatcherFields) -> bool {
        (self.fine)(f)
    }
}

fn match_all() -> Pred {
    predicate::new(|_: &SegmentMatcherFields| true)
}

fn match_nothing() -> Pred {
    predicate::new(|_: &SegmentMatcherFields| false)
}

/// Go: `segments.Segments` — a collection of named segments.
#[derive(Clone, Default)]
pub struct Segments {
    s: BTreeMap<String, ExcludeInclude>,
}

impl Segments {
    /// Get returns a SegmentFilter for the given segments (`ss` = `None` is Go's nil slice).
    // Go: hugolib/segments/segments.go:Get
    pub fn get(&self, on_not_found: Option<&dyn Fn(&str)>, ss: Option<&[String]>) -> SegmentFilter {
        let Some(ss) = ss else {
            return SegmentFilter {
                coarse: match_nothing(),
                fine: match_nothing(),
            };
        };
        let mut coarse: Option<Pred> = None;
        let mut fine: Option<Pred> = None;
        for s in ss {
            if let Some(seg) = self.s.get(s) {
                let c1 = seg.clone();
                let c: Pred =
                    predicate::new(move |f: &SegmentMatcherFields| c1.should_exclude_coarse(f));
                coarse = Some(match coarse {
                    None => c,
                    Some(p) => predicate::or(Some(p), vec![c]),
                });
                let f1 = seg.clone();
                let f: Pred =
                    predicate::new(move |f: &SegmentMatcherFields| f1.should_exclude_fine(f));
                fine = Some(match fine {
                    None => f,
                    Some(p) => predicate::or(Some(p), vec![f]),
                });
            } else if let Some(on_not_found) = on_not_found {
                on_not_found(s);
            }
        }

        SegmentFilter {
            coarse: coarse.unwrap_or_else(match_all),
            fine: fine.unwrap_or_else(match_all),
        }
    }
}

/// Go: `getGlob(s)` (nil for "").
// Go: hugolib/segments/segments.go:getGlob
fn get_glob(s: &str) -> Result<Option<nh_common::glob::glob::Glob>> {
    if s.is_empty() {
        return Ok(None);
    }
    match nh_common::glob::glob::get_glob(s) {
        Ok(g) => Ok(Some(g)),
        Err(err) => Err(Error::new(format!(
            "failed to compile Glob {}: {}",
            go_strconv::quote(s.as_bytes()),
            err
        ))),
    }
}

/// A Go nil predicate called as a function: Go crashes with a nil pointer dereference when the
/// combined predicate is evaluated (a matcher entry without any field set, after the first).
fn nil_predicate() -> Pred {
    predicate::new(|_: &SegmentMatcherFields| -> bool {
        panic!("runtime error: invalid memory address or nil pointer dereference")
    })
}

// Go: hugolib/segments/segments.go:compileSegments
fn compile_segments(f: &[SegmentMatcherFields]) -> Result<Option<Pred>> {
    let mut result: Option<Pred> = None;
    let mut section: Option<Pred> = None;

    fn add_to_section(
        section: &mut Option<Pred>,
        matcher_fields: &SegmentMatcherFields,
        f: fn(&SegmentMatcherFields) -> &str,
    ) -> Result<()> {
        let s1 = f(matcher_fields);
        let g = get_glob(s1)?.expect("non-empty pattern");
        let matcher: Pred = predicate::new(move |fields: &SegmentMatcherFields| {
            let s2 = f(fields);
            if s2.is_empty() {
                return false;
            }
            g.matches(s2)
        });
        *section = Some(match section.take() {
            None => matcher,
            Some(p) => predicate::and(Some(p), vec![matcher]),
        });
        Ok(())
    }

    for fields in f {
        if !fields.kind.is_empty() {
            add_to_section(&mut section, fields, |f| &f.kind)?;
        }
        if !fields.path.is_empty() {
            add_to_section(&mut section, fields, |f| &f.path)?;
        }
        if !fields.lang.is_empty() {
            add_to_section(&mut section, fields, |f| &f.lang)?;
        }
        if !fields.output.is_empty() {
            add_to_section(&mut section, fields, |f| &f.output)?;
        }

        result = match result {
            None => section.take(),
            Some(r) => Some(predicate::or(
                Some(r),
                vec![section.take().unwrap_or_else(nil_predicate)],
            )),
        };
        section = None;
    }

    Ok(result)
}

// Go: hugolib/segments/segments.go:DecodeSegments
/// `input` is the `segments` map (`Value::TypedNil` for Go's nil map).
pub fn decode_segments(input: &Value) -> Result<ConfigNamespace<Map, Segments>> {
    let build_config = |input: &Value| -> Result<(Segments, Option<Value>)> {
        let mut sms = Segments::default();
        let m = if input.is_nil() {
            Map::new(MapType::StringAny)
        } else {
            nh_common::maps::maps::to_string_map_e(input)?
        };
        let m = nh_common::maps::params::clean_config_string_map(&m);

        let mut scfgm: BTreeMap<String, SegmentConfig> = BTreeMap::new();
        nh_config::decode::decode_into(&Value::map(m), &mut scfgm)?;

        for (k, v) in &scfgm {
            let mut exclude = None;
            let mut include = None;
            if !v.excludes.is_empty() {
                exclude = compile_segments(&v.excludes)?;
            }
            if !v.includes.is_empty() {
                include = compile_segments(&v.includes)?;
            }

            sms.s.insert(k.clone(), ExcludeInclude { exclude, include });
        }

        Ok((sms, None))
    };

    nh_config::namespace::decode_namespace(input, build_config)
        .map_err(|err| Error::new(format!("failed to decode segments: {err}")))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/segments/segments.go (257 lines; 4/8 funcs executed)
//   types: Segments, excludeInclude, SegmentFilter, segmentFilter, SegmentConfig, SegmentMatcherFields
// OK L42-44: (e excludeInclude) ShouldExcludeCoarse(fields SegmentMatcherFields) bool
// OK L48-53: (e excludeInclude) ShouldExcludeFine(fields SegmentMatcherFields) bool
// OK L68-70: (f segmentFilter) ShouldExcludeCoarse(field SegmentMatcherFields) bool
// OK L72-74: (f segmentFilter) ShouldExcludeFine(fields SegmentMatcherFields) bool
// OK L82-112: (sms Segments) Get(onNotFound func(s string), ss ...string) SegmentFilter
// OK L128-137: getGlob(s string) (glob.Glob, error)
// OK L139-201: compileSegments(f []SegmentMatcherFields) (predicate.P[SegmentMatcherFields], error)
// OK L203-257: DecodeSegments(in map[string]any) (*config.ConfigNamespace[map[string]SegmentConfig, Segments], error)
// ---------------------------------------------------------------------------

// Ports of hugolib/segments/segments_test.go.
#[cfg(test)]
mod tests {
    use super::*;

    fn f(kind: &str, lang: &str, path: &str, output: &str) -> SegmentMatcherFields {
        SegmentMatcherFields {
            kind: kind.to_string(),
            lang: lang.to_string(),
            path: path.to_string(),
            output: output.to_string(),
        }
    }

    // Go: hugolib/segments/segments_test.go:TestCompileSegments (excludes)
    #[test]
    fn compile_segments_excludes() {
        let check = |m: &Pred| {
            assert!(!m(&f("", "no", "", "")));
            assert!(!m(&f("page", "no", "", "")));
            assert!(m(&f("", "no", "", "rss")));
            assert!(!m(&f("", "no", "", "html")));
            assert!(!m(&f("page", "", "", "")));
            assert!(m(&f("page", "no", "", "rss")));
        };

        let m = compile_segments(&[f("", "n*", "", "rss")])
            .unwrap()
            .unwrap();
        check(&m);

        let m = compile_segments(&[f("", "", "/blog/**", ""), f("", "n*", "", "rss")])
            .unwrap()
            .unwrap();
        check(&m);
        assert!(m(&f("", "", "/blog/foo", "")));
    }

    // Go: hugolib/segments/segments_test.go:TestCompileSegments (includes)
    #[test]
    fn compile_segments_includes() {
        let m = compile_segments(&[f("", "", "/docs/**", ""), f("", "no", "", "rss")])
            .unwrap()
            .unwrap();
        assert!(!m(&f("", "no", "", "")));
        assert!(!m(&f("page", "", "", "")));
        assert!(!m(&f("page", "", "/blog/foo", "")));
        assert!(!m(&f("", "en", "", "")));
        assert!(m(&f("", "no", "", "rss")));
        assert!(!m(&f("", "no", "", "html")));
        assert!(m(&f("page", "", "/docs/foo", "")));
    }

    #[test]
    fn compile_segments_invalid_glob() {
        let err = compile_segments(&[f("", "", "[", "")]).err().unwrap();
        assert!(
            err.to_string()
                .starts_with("failed to compile Glob \"[\": "),
            "{err}"
        );
    }

    // Segments.Get: a nil slice excludes nothing; an empty (non-nil) slice or only unknown
    // segments exclude everything (Go's matchAll).
    #[test]
    fn get_nil_and_empty() {
        let mut m = Map::new(MapType::StringAny);
        let mut seg = Map::new(MapType::StringAny);
        let mut ex = Map::new(MapType::StringAny);
        ex.insert("lang", Value::string("n*"));
        seg.insert("excludes", Value::any_list(vec![Value::map(ex)]));
        m.insert("s1", Value::map(seg));
        let ns = decode_segments(&Value::map(m)).unwrap();
        let segs = &ns.config;

        let page_no = f("page", "no", "", "");
        let page_en = f("page", "en", "", "");

        let none = segs.get(None, None);
        assert!(!none.should_exclude_coarse(&page_no));
        assert!(!none.should_exclude_fine(&page_no));

        let empty = segs.get(None, Some(&[]));
        assert!(empty.should_exclude_coarse(&page_en));

        let seen = std::cell::RefCell::new(Vec::new());
        let on_not_found = |s: &str| seen.borrow_mut().push(s.to_string());
        let unknown = segs.get(Some(&on_not_found), Some(&["nope".to_string()]));
        assert!(unknown.should_exclude_fine(&page_en));
        assert_eq!(*seen.borrow(), ["nope"]);

        let s1 = segs.get(None, Some(&["s1".to_string()]));
        assert!(s1.should_exclude_coarse(&page_no));
        assert!(!s1.should_exclude_coarse(&page_en));
        assert!(s1.should_exclude_fine(&page_no));
        assert!(!s1.should_exclude_fine(&page_en));
    }
}
