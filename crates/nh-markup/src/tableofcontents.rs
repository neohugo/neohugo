//! Port of `markup/tableofcontents/tableofcontents.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/tableofcontents`: headings collected during rendering (`.TableOfContents`,
//! `.Fragments`, `HeadingsFiltered`). Built but unused by seeksnack; port for completeness.
//!
//! Go builds the tree with `*Heading` pointers and appends children to headings that are
//! already in the tree. The port holds `Arc<Heading>` and mutates through `Arc::make_mut`
//! while building (the builder is the only owner then, so nothing is copied).

use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{GoString, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;
use nh_config::decode_struct;

/// Go: `tableofcontents.Heading`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heading {
    pub id: GoString,
    pub level: i64,
    pub title: GoString,
    pub headings: Headings,
}

/// Go: `tableofcontents.Headings`.
pub type Headings = Vec<Arc<Heading>>;

impl Heading {
    /// Go: `Heading.IsZero()` — true when no ID or Text is set.
    // Go: markup/tableofcontents/tableofcontents.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.id.is_empty() && self.title.is_empty()
    }

    // Go: markup/tableofcontents/tableofcontents.go:walk
    fn walk(self: &Arc<Self>, f: &mut dyn FnMut(&Arc<Heading>)) {
        f(self);
        for h in &self.headings {
            h.walk(f);
        }
    }
}

/// Go: `Headings.FilterBy(fn)` — all headings (depth first) matching the predicate.
// Go: markup/tableofcontents/tableofcontents.go:FilterBy
pub fn filter_by(h: &Headings, f: &dyn Fn(&Heading) -> bool) -> Headings {
    let mut out = Headings::new();
    for h in h {
        h.walk(&mut |h| {
            if f(h) {
                out.push(h.clone());
            }
        });
    }
    out
}

/// Go: `tableofcontents.Fragments`.
#[derive(Clone, Debug, Default)]
pub struct Fragments {
    pub headings: Headings,
    /// Sorted.
    pub identifiers: Vec<GoString>,
    /// With duplicate IDs, the last one (in walk order) wins.
    pub headings_map: BTreeMap<GoString, Arc<Heading>>,
}

/// Go: `tableofcontents.Empty`.
pub fn empty() -> Arc<Fragments> {
    Arc::new(Fragments::default())
}

impl Fragments {
    // Go: markup/tableofcontents/tableofcontents.go:addAt
    fn add_at(&mut self, h: Heading, row: usize, level: usize) {
        while self.headings.len() <= row {
            self.headings.push(Arc::new(Heading::default()));
        }

        if level == 0 {
            self.headings[row] = Arc::new(h);
            return;
        }

        let mut heading = Arc::make_mut(&mut self.headings[row]);

        for _ in 1..level {
            if heading.headings.is_empty() {
                heading.headings.push(Arc::new(Heading::default()));
            }
            let last = heading.headings.len() - 1;
            heading = Arc::make_mut(&mut heading.headings[last]);
        }
        heading.headings.push(Arc::new(h));
    }

    /// Go: `Fragments.ToHTML(startLevel, stopLevel, ordered)` with the levels already cast to
    /// `int`.
    // Go: markup/tableofcontents/tableofcontents.go:ToHTML
    pub fn to_html(&self, start_level: i64, stop_level: i64, ordered: bool) -> Vec<u8> {
        let mut b = TocBuilder {
            s: Vec::new(),
            h: &self.headings,
            start_level,
            stop_level,
            ordered,
        };
        b.build();
        b.s
    }

    /// Go: `Fragments.ToHTML(startLevel, stopLevel any, ordered bool)` as called from templates:
    /// the levels go through `cast.ToIntE` (`startLevel: <err>` / `stopLevel: <err>` errors).
    // Go: markup/tableofcontents/tableofcontents.go:ToHTML
    pub fn to_html_values(
        &self,
        start_level: &Value,
        stop_level: &Value,
        ordered: bool,
    ) -> Result<Vec<u8>> {
        let i_start = nh_common::cast::caste::to_int_e(start_level)
            .map_err(|e| Error::new(format!("startLevel: {e}")))?;
        let i_stop = nh_common::cast::caste::to_int_e(stop_level)
            .map_err(|e| Error::new(format!("stopLevel: {e}")))?;
        Ok(self.to_html(i_start, i_stop, ordered))
    }

    // Go: markup/tableofcontents/tableofcontents.go:(Fragments).walk
    fn walk(&self, f: &mut dyn FnMut(&Arc<Heading>)) {
        for h in &self.headings {
            h.walk(f);
        }
    }
}

/// Go: `tableofcontents.Builder`.
#[derive(Default)]
pub struct Builder {
    pub(crate) identifiers_set: bool,
    /// Go's `toc *Fragments` (nil until the first `AddAt`/`SetIdentifiers`).
    pub(crate) toc: Option<Fragments>,
}

impl Builder {
    // Go: markup/tableofcontents/tableofcontents.go:AddAt
    pub fn add_at(&mut self, h: Heading, row: usize, level: usize) {
        self.toc
            .get_or_insert_with(Fragments::default)
            .add_at(h, row, level);
    }

    // Go: markup/tableofcontents/tableofcontents.go:SetIdentifiers
    pub fn set_identifiers(&mut self, mut ids: Vec<GoString>) {
        let toc = self.toc.get_or_insert_with(Fragments::default);
        self.identifiers_set = true;
        ids.sort();
        toc.identifiers = ids;
    }

    /// Go returns the shared `Empty` when nothing was added.
    // Go: markup/tableofcontents/tableofcontents.go:Build
    pub fn build(self) -> Arc<Fragments> {
        let Some(mut toc) = self.toc else {
            return empty();
        };
        let mut map = BTreeMap::new();
        let mut ids = Vec::new();
        toc.walk(&mut |h| {
            if !h.id.is_empty() {
                map.insert(h.id.clone(), h.clone());
                if !self.identifiers_set {
                    ids.push(h.id.clone());
                }
            }
        });
        toc.headings_map = map;
        toc.identifiers.extend(ids);
        toc.identifiers.sort();
        Arc::new(toc)
    }
}

// Go: markup/tableofcontents/tableofcontents.go:tocBuilder
struct TocBuilder<'a> {
    s: Vec<u8>,
    h: &'a Headings,
    start_level: i64,
    stop_level: i64,
    ordered: bool,
}

impl TocBuilder<'_> {
    // Go: markup/tableofcontents/tableofcontents.go:(*tocBuilder).Build
    fn build(&mut self) {
        let h = self.h;
        self.write_nav(h);
    }

    // Go: markup/tableofcontents/tableofcontents.go:writeNav
    fn write_nav(&mut self, _h: &Headings) {
        self.s.extend_from_slice(b"<nav id=\"TableOfContents\">");
        let h = self.h;
        self.write_headings(1, 0, h);
        self.s.extend_from_slice(b"</nav>");
    }

    // Go: markup/tableofcontents/tableofcontents.go:writeHeadings
    fn write_headings(&mut self, level: i64, indent: i64, h: &Headings) {
        if level < self.start_level {
            for h in h {
                self.write_headings(level + 1, indent, &h.headings);
            }
            return;
        }

        if self.stop_level != -1 && level > self.stop_level {
            return;
        }

        let has_children = !h.is_empty();

        if has_children {
            self.s.push(b'\n');
            self.indent(indent + 1);
            if self.ordered {
                self.s.extend_from_slice(b"<ol>\n");
            } else {
                self.s.extend_from_slice(b"<ul>\n");
            }
        }

        for h in h {
            self.write_heading(level + 1, indent + 2, h);
        }

        if has_children {
            self.indent(indent + 1);
            if self.ordered {
                self.s.extend_from_slice(b"</ol>");
            } else {
                self.s.extend_from_slice(b"</ul>");
            }
            self.s.push(b'\n');
            self.indent(indent);
        }
    }

    // Go: markup/tableofcontents/tableofcontents.go:writeHeading
    fn write_heading(&mut self, level: i64, indent: i64, h: &Heading) {
        self.indent(indent);
        self.s.extend_from_slice(b"<li>");
        if !h.is_zero() {
            self.s.extend_from_slice(b"<a href=\"#");
            self.s.extend_from_slice(h.id.as_bytes());
            self.s.extend_from_slice(b"\">");
            self.s.extend_from_slice(h.title.as_bytes());
            self.s.extend_from_slice(b"</a>");
        }
        self.write_headings(level, indent, &h.headings);
        self.s.extend_from_slice(b"</li>\n");
    }

    // Go: markup/tableofcontents/tableofcontents.go:indent
    fn indent(&mut self, n: i64) {
        for _ in 0..n.max(0) {
            self.s.extend_from_slice(b"  ");
        }
    }
}

/// Go: `tableofcontents.Config` (`startLevel=2`, `endLevel=3`, `ordered=false` by default).
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub start_level: i64,
    pub end_level: i64,
    pub ordered: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            start_level: 2,
            end_level: 3,
            ordered: false,
        }
    }
}

decode_struct!(Config, "tableofcontents.Config", |s| vec![
    FieldRef::new("StartLevel", &mut s.start_level),
    FieldRef::new("EndLevel", &mut s.end_level),
    FieldRef::new("Ordered", &mut s.ordered),
]);

#[cfg(test)]
mod tests {
    use super::*;

    fn h(title: &str, id: &str) -> Heading {
        Heading {
            title: GoString::new(title.as_bytes().to_vec()),
            id: GoString::new(id.as_bytes().to_vec()),
            ..Default::default()
        }
    }

    fn new_test_toc_builder() -> Builder {
        let mut b = Builder::default();
        b.add_at(h("Heading 1", "h1-1"), 0, 0);
        b.add_at(h("1-H2-1", "1-h2-1"), 0, 1);
        b.add_at(h("1-H2-2", "1-h2-2"), 0, 1);
        b.add_at(h("1-H3-1", "1-h2-2"), 0, 2);
        b.add_at(h("Heading 2", "h1-2"), 1, 0);
        b
    }

    fn s(b: Vec<u8>) -> String {
        String::from_utf8(b).unwrap()
    }

    // Go: markup/tableofcontents/tableofcontents_test.go:TestToc
    #[test]
    fn toc() {
        let mut toc = Fragments::default();
        toc.add_at(h("Heading 1", "h1-1"), 0, 0);
        toc.add_at(h("1-H2-1", "1-h2-1"), 0, 1);
        toc.add_at(h("1-H2-2", "1-h2-2"), 0, 1);
        toc.add_at(h("1-H3-1", "1-h2-2"), 0, 2);
        toc.add_at(h("Heading 2", "h1-2"), 1, 0);

        assert_eq!(
            s(toc.to_html(1, -1, false)),
            r##"<nav id="TableOfContents">
  <ul>
    <li><a href="#h1-1">Heading 1</a>
      <ul>
        <li><a href="#1-h2-1">1-H2-1</a></li>
        <li><a href="#1-h2-2">1-H2-2</a>
          <ul>
            <li><a href="#1-h2-2">1-H3-1</a></li>
          </ul>
        </li>
      </ul>
    </li>
    <li><a href="#h1-2">Heading 2</a></li>
  </ul>
</nav>"##
        );
        assert_eq!(
            s(toc.to_html(1, 1, false)),
            r##"<nav id="TableOfContents">
  <ul>
    <li><a href="#h1-1">Heading 1</a></li>
    <li><a href="#h1-2">Heading 2</a></li>
  </ul>
</nav>"##
        );
        assert_eq!(
            s(toc.to_html(1, 2, false)),
            r##"<nav id="TableOfContents">
  <ul>
    <li><a href="#h1-1">Heading 1</a>
      <ul>
        <li><a href="#1-h2-1">1-H2-1</a></li>
        <li><a href="#1-h2-2">1-H2-2</a></li>
      </ul>
    </li>
    <li><a href="#h1-2">Heading 2</a></li>
  </ul>
</nav>"##
        );
        assert_eq!(
            s(toc.to_html(2, 2, false)),
            r##"<nav id="TableOfContents">
  <ul>
    <li><a href="#1-h2-1">1-H2-1</a></li>
    <li><a href="#1-h2-2">1-H2-2</a></li>
  </ul>
</nav>"##
        );
        assert_eq!(
            s(toc.to_html(1, -1, true)),
            r##"<nav id="TableOfContents">
  <ol>
    <li><a href="#h1-1">Heading 1</a>
      <ol>
        <li><a href="#1-h2-1">1-H2-1</a></li>
        <li><a href="#1-h2-2">1-H2-2</a>
          <ol>
            <li><a href="#1-h2-2">1-H3-1</a></li>
          </ol>
        </li>
      </ol>
    </li>
    <li><a href="#h1-2">Heading 2</a></li>
  </ol>
</nav>"##
        );
    }

    // Go: markup/tableofcontents/tableofcontents_test.go:TestTocMissingParent
    #[test]
    fn toc_missing_parent() {
        let mut toc = Fragments::default();
        toc.add_at(h("H2", "h2"), 0, 1);
        toc.add_at(h("H3", "h3"), 1, 2);
        toc.add_at(h("H3", "h3"), 1, 2);

        assert_eq!(
            s(toc.to_html(1, -1, false)),
            r##"<nav id="TableOfContents">
  <ul>
    <li>
      <ul>
        <li><a href="#h2">H2</a></li>
      </ul>
    </li>
    <li>
      <ul>
        <li>
          <ul>
            <li><a href="#h3">H3</a></li>
            <li><a href="#h3">H3</a></li>
          </ul>
        </li>
      </ul>
    </li>
  </ul>
</nav>"##
        );
        assert_eq!(
            s(toc.to_html(3, 3, false)),
            r##"<nav id="TableOfContents">
  <ul>
    <li><a href="#h3">H3</a></li>
    <li><a href="#h3">H3</a></li>
  </ul>
</nav>"##
        );
        assert_eq!(
            s(toc.to_html(1, -1, true)),
            r##"<nav id="TableOfContents">
  <ol>
    <li>
      <ol>
        <li><a href="#h2">H2</a></li>
      </ol>
    </li>
    <li>
      <ol>
        <li>
          <ol>
            <li><a href="#h3">H3</a></li>
            <li><a href="#h3">H3</a></li>
          </ol>
        </li>
      </ol>
    </li>
  </ol>
</nav>"##
        );
    }

    // Go: markup/tableofcontents/tableofcontents_test.go:TestTocMisc
    #[test]
    fn toc_misc() {
        let toc = new_test_toc_builder().build();
        let ids: Vec<&[u8]> = toc.identifiers.iter().map(|x| x.as_bytes()).collect();
        assert_eq!(
            ids,
            vec![&b"1-h2-1"[..], b"1-h2-2", b"1-h2-2", b"h1-1", b"h1-2"]
        );
        let m = &toc.headings_map;
        assert_eq!(
            m[&GoString::new(b"h1-1".to_vec())].title.as_bytes(),
            b"Heading 1"
        );
        assert!(!m.contains_key(&GoString::new(b"doesnot exist".to_vec())));
        // The last heading wins for a duplicate ID.
        assert_eq!(
            m[&GoString::new(b"1-h2-2".to_vec())].title.as_bytes(),
            b"1-H3-1"
        );

        assert!(Builder::default().build().headings.is_empty());
        let e = toc
            .to_html_values(&Value::string("x"), &Value::int(3), false)
            .unwrap_err();
        assert!(e.to_string().starts_with("startLevel: "), "{e}");
        assert_eq!(
            toc.to_html_values(&Value::string("2"), &Value::int(2), false)
                .unwrap(),
            toc.to_html(2, 2, false)
        );
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/tableofcontents/tableofcontents.go (277 lines; 13/14 funcs executed)
//   types: Builder, Headings, Heading, Fragments, tocBuilder, Config
// OK L39-44: (b *Builder) AddAt(h *Heading, row, level int)
// OK L47-54: (b *Builder) SetIdentifiers(ids []string)
// OK L57-72: (b Builder) Build() *Fragments
// OK L79-90: (h Headings) FilterBy(fn func(*Heading) bool) Headings
// OK L102-104: (h Heading) IsZero() bool
// OK L106-111: (h *Heading) walk(fn func(*Heading))
// OK L129-148: (toc *Fragments) addAt(h *Heading, row, level int)
// OK L151-175: (toc *Fragments) ToHTML(startLevel, stopLevel any, ordered bool) (template.HTML, error)
// OK L177-181: (toc Fragments) walk(fn func(*Heading))
// OK L192-194: (b *tocBuilder) Build()
// OK L196-200: (b *tocBuilder) writeNav(h Headings)
// OK L202-240: (b *tocBuilder) writeHeadings(level, indent int, h Headings)
// OK L242-250: (b *tocBuilder) writeHeading(level, indent int, h *Heading)
// OK L252-256: (b *tocBuilder) indent(n int)
// ---------------------------------------------------------------------------
