//! Port of `resources/page/permalinks.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use go_time::GoTimeExt;
use go_value::{Map, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;

use crate::page::Page;

/// A compiled permalink pattern (Go `func(Page) (string, error)`).
pub type ExpandFn = Arc<dyn Fn(&dyn Page) -> Result<String> + Send + Sync>;

/// Go: `pageToPermaAttribute` — given a page and the attribute, returns its replacement.
type PageToPermaAttribute =
    Arc<dyn Fn(&PermalinkExpander, &dyn Page, &str) -> Result<String> + Send + Sync>;

/// Go: `page.PermalinkExpander` (`[permalinks]`: `:year`, `:month`, `:title`, `:slug`, `:sections`...).
#[derive(Clone)]
pub struct PermalinkExpander {
    /// kind -> section -> compiled pattern.
    pub(crate) expanders: BTreeMap<String, BTreeMap<String, ExpandFn>>,
    pub(crate) urlize: Arc<dyn Fn(&str) -> String + Send + Sync>,
    pattern_cache: Arc<Mutex<HashMap<String, ExpandFn>>>,
}

/// Go: `referenceTime` — every field differs from Go's reference time, so formatting it with a
/// Go time layout always gives something other than the layout.
fn reference_time() -> go_value::Time {
    go_time::date(
        2019,
        go_time::Month::NOVEMBER,
        9,
        23,
        1,
        42,
        1,
        &go_time::utc(),
    )
}

/// Go: `knownPermalinkAttributes` (the fixed attribute names).
fn known_permalink_attribute(attr: &str) -> Option<PageToPermaAttribute> {
    let f: PageToPermaAttribute = match attr {
        "year" | "month" | "monthname" | "day" | "weekday" | "weekdayname" | "yearday" => {
            Arc::new(|l, p, a| l.page_to_permalink_date(p, a))
        }
        "section" => Arc::new(|l, p, a| l.page_to_permalink_section(p, a)),
        "sections" => Arc::new(|l, p, a| l.page_to_permalink_sections(p, a)),
        "title" => Arc::new(|l, p, a| l.page_to_permalink_title(p, a)),
        "slug" => Arc::new(|l, p, a| l.page_to_permalink_slug_else_title(p, a)),
        "slugorfilename" => Arc::new(|l, p, a| l.page_to_permalink_slug_else_filename(p, a)),
        "filename" => Arc::new(|l, p, a| l.page_to_permalink_filename(p, a)),
        "contentbasename" => Arc::new(|l, p, a| l.page_to_permalink_content_base_name(p, a)),
        "slugorcontentbasename" => {
            Arc::new(|l, p, a| l.page_to_permalink_slug_or_content_base_name(p, a))
        }
        _ => return None,
    };
    Some(f)
}

/// Go: `permalinkExpandError` — `error expanding %q: %s`.
fn permalink_expand_error(pattern: &str, err: &str) -> Error {
    Error::new(format!(
        "error expanding {}: {err}",
        go_strconv::quote(pattern.as_bytes())
    ))
}

const ERR_PERMALINK_ATTRIBUTE_UNKNOWN: &str = "permalink attribute not recognised";

/// Escape sequence for colons in permalink patterns.
const ESCAPE_PLACEHOLDER_COLON: &str = "\x00";

/// Allow " " and / to represent the root section.
const SECTION_CUT_SET: &str = " /";

impl PermalinkExpander {
    /// Return the callback for the given permalink attribute, or `None` if the attribute is not
    /// valid.
    // Go: resources/page/permalinks.go:callback
    fn callback(&self, attr: &str) -> Option<PageToPermaAttribute> {
        if let Some(callback) = known_permalink_attribute(attr) {
            return Some(callback);
        }

        if attr.starts_with("sections[")
            && let Some(cut) = attr.strip_prefix("sections")
        {
            let fn_ = to_slice_func(cut);
            return Some(Arc::new(move |_l, p, _s| {
                let entries = current_section_entries(p);
                let sliced = fn_(&entries)?;
                let refs: Vec<&str> = sliced.iter().map(|s| s.as_str()).collect();
                Ok(go_path::path::join(&refs))
            }));
        }

        // Make sure this comes after all the other checks.
        if reference_time().format(attr) != attr {
            return Some(Arc::new(|l, p, a| l.page_to_permalink_date(p, a)));
        }

        None
    }

    /// NewPermalinkExpander creates a new PermalinkExpander configured by the given urlize func.
    // Go: resources/page/permalinks.go:NewPermalinkExpander
    pub fn new(
        urlize: Arc<dyn Fn(&str) -> String + Send + Sync>,
        patterns: &BTreeMap<String, BTreeMap<String, String>>,
    ) -> Result<PermalinkExpander> {
        let mut p = PermalinkExpander {
            expanders: BTreeMap::new(),
            urlize,
            pattern_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        // Go iterates the map in random order; the first error wins (byte order here).
        let mut expanders = BTreeMap::new();
        for (kind, patterns) in patterns {
            let e = p.parse(patterns)?;
            expanders.insert(kind.clone(), e);
        }
        p.expanders = expanders;

        Ok(p)
    }

    // Go: resources/page/permalinks.go:normalizeEscapeSequencesIn
    fn normalize_escape_sequences_in(&self, s: &str) -> (String, bool) {
        let s2 = s.replace("\\:", ESCAPE_PLACEHOLDER_COLON);
        let changed = s2 != s;
        (s2, changed)
    }

    // Go: resources/page/permalinks.go:normalizeEscapeSequencesOut
    fn normalize_escape_sequences_out(result: &str) -> String {
        result.replace(ESCAPE_PLACEHOLDER_COLON, ":")
    }

    /// ExpandPattern expands the path in p with the specified expand pattern.
    // Go: resources/page/permalinks.go:ExpandPattern
    pub fn expand_pattern(&self, pattern: &str, p: &dyn Page) -> Result<String> {
        let expand = self.get_or_parse_pattern(pattern)?;
        expand(p)
    }

    /// Go: `Expand(key, p)` — `key` is the section or taxonomy; "" if no pattern.
    // Go: resources/page/permalinks.go:Expand
    pub fn expand(&self, key: &str, p: &dyn Page) -> Result<String> {
        let Some(expanders) = self.expanders.get(&p.kind()) else {
            return Ok(String::new());
        };

        let Some(expand) = expanders.get(key) else {
            return Ok(String::new());
        };

        expand(p)
    }

    // Go: resources/page/permalinks.go:getOrParsePattern
    fn get_or_parse_pattern(&self, pattern: &str) -> Result<ExpandFn> {
        if let Some(f) = self
            .pattern_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(pattern)
        {
            return Ok(f.clone());
        }
        let f = self.parse_pattern(pattern)?;
        let mut cache = self.pattern_cache.lock().unwrap_or_else(|e| e.into_inner());
        Ok(cache.entry(pattern.to_string()).or_insert(f).clone())
    }

    /// The create func of Go's `getOrParsePattern`.
    fn parse_pattern(&self, pattern: &str) -> Result<ExpandFn> {
        let (pattern, normalized) = self.normalize_escape_sequences_in(pattern);

        let matches = find_attribute_matches(&pattern);
        if matches.is_empty() {
            let mut result = pattern;
            if normalized {
                result = Self::normalize_escape_sequences_out(&result);
            }
            return Ok(Arc::new(move |_p| Ok(result.clone())));
        }

        let mut callbacks: Vec<PageToPermaAttribute> = Vec::with_capacity(matches.len());
        let mut replacements: Vec<String> = Vec::with_capacity(matches.len());
        for m in &matches {
            let replacement = m.clone();
            let attr = &replacement[1..];
            let Some(callback) = self.callback(attr) else {
                return Err(permalink_expand_error(
                    &pattern,
                    ERR_PERMALINK_ATTRIBUTE_UNKNOWN,
                ));
            };
            replacements.push(replacement.clone());
            callbacks.push(callback);
        }

        let this = self.clone_without_cache();
        Ok(Arc::new(move |p: &dyn Page| {
            let mut new_field = pattern.clone();

            for (i, replacement) in replacements.iter().enumerate() {
                let attr = &replacement[1..];
                let callback = &callbacks[i];
                let new_attr = match callback(&this, p, attr) {
                    Ok(s) => s,
                    Err(e) => return Err(permalink_expand_error(&pattern, e.message())),
                };

                new_field = new_field.replacen(replacement.as_str(), &new_attr, 1);
            }

            if normalized {
                new_field = Self::normalize_escape_sequences_out(&new_field);
            }

            Ok(new_field)
        }))
    }

    /// The expander as the callbacks see it (the pattern cache is not needed there, and
    /// sharing it would make the cached closures own the cache).
    fn clone_without_cache(&self) -> PermalinkExpander {
        PermalinkExpander {
            expanders: BTreeMap::new(),
            urlize: self.urlize.clone(),
            pattern_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Go: resources/page/permalinks.go:parse
    fn parse(&self, patterns: &BTreeMap<String, String>) -> Result<BTreeMap<String, ExpandFn>> {
        let mut expanders = BTreeMap::new();

        for (k, pattern) in patterns {
            let k = k.trim_matches(|c| SECTION_CUT_SET.contains(c)).to_string();

            let expander = self.get_or_parse_pattern(pattern)?;

            expanders.insert(k, expander);
        }

        Ok(expanders)
    }

    // Go: resources/page/permalinks.go:pageToPermalinkDate
    fn page_to_permalink_date(&self, p: &dyn Page, date_field: &str) -> Result<String> {
        // a Page contains a Node which provides a field Date, time.Time
        let d = p.date();
        Ok(match date_field {
            "year" => d.year().to_string(),
            "month" => format!("{:02}", d.month().0),
            "monthname" => d.month().string(),
            "day" => format!("{:02}", d.day()),
            "weekday" => d.weekday().0.to_string(),
            "weekdayname" => d.weekday().string(),
            "yearday" => d.year_day().to_string(),
            _ => d.format(date_field),
        })
    }

    /// pageToPermalinkTitle returns the URL-safe form of the title
    // Go: resources/page/permalinks.go:pageToPermalinkTitle
    fn page_to_permalink_title(&self, p: &dyn Page, _a: &str) -> Result<String> {
        Ok((self.urlize)(&Page::title(p)))
    }

    /// pageToPermalinkFilename returns the URL-safe form of the filename
    // Go: resources/page/permalinks.go:pageToPermalinkFilename
    fn page_to_permalink_filename(&self, p: &dyn Page, _a: &str) -> Result<String> {
        let mut name = self.translation_base_name(p);
        match name.as_str() {
            "index" => {
                // Page bundles; the directory name will hopefully have a better name.
                // Go dereferences p.File() here (it is non-nil: the name came from it).
                let file = p.file().expect("pageToPermalinkFilename: page has a File");
                let dir = file.dir();
                let dir = dir.strip_suffix('/').unwrap_or(&dir);
                let (_, n) = go_path::filepath::split(dir);
                name = n.to_string();
            }
            "_index" => return Ok(String::new()),
            _ => {}
        }

        Ok((self.urlize)(&name))
    }

    /// if the page has a slug, return the slug, else return the title
    // Go: resources/page/permalinks.go:pageToPermalinkSlugElseTitle
    fn page_to_permalink_slug_else_title(&self, p: &dyn Page, a: &str) -> Result<String> {
        let slug = p.slug();
        if !slug.is_empty() {
            return Ok((self.urlize)(&slug));
        }
        self.page_to_permalink_title(p, a)
    }

    /// if the page has a slug, return the slug, else return the filename
    // Go: resources/page/permalinks.go:pageToPermalinkSlugElseFilename
    fn page_to_permalink_slug_else_filename(&self, p: &dyn Page, a: &str) -> Result<String> {
        let slug = p.slug();
        if !slug.is_empty() {
            return Ok((self.urlize)(&slug));
        }
        self.page_to_permalink_filename(p, a)
    }

    // Go: resources/page/permalinks.go:pageToPermalinkSection
    fn page_to_permalink_section(&self, p: &dyn Page, _a: &str) -> Result<String> {
        Ok(p.section())
    }

    // Go: resources/page/permalinks.go:pageToPermalinkSections
    fn page_to_permalink_sections(&self, p: &dyn Page, _a: &str) -> Result<String> {
        Ok(match p.current_section() {
            Some(cs) => cs.0.sections_path(),
            None => p.sections_path(),
        })
    }

    /// pageToPermalinkContentBaseName returns the URL-safe form of the content base name.
    // Go: resources/page/permalinks.go:pageToPermalinkContentBaseName
    fn page_to_permalink_content_base_name(&self, p: &dyn Page, _a: &str) -> Result<String> {
        let pi = p.path_info();
        Ok((self.urlize)(pi.unnormalized().base_name_no_identifier()))
    }

    /// pageToPermalinkSlugOrContentBaseName returns the URL-safe form of the slug, content base
    /// name.
    // Go: resources/page/permalinks.go:pageToPermalinkSlugOrContentBaseName
    fn page_to_permalink_slug_or_content_base_name(&self, p: &dyn Page, a: &str) -> Result<String> {
        let slug = p.slug();
        if !slug.is_empty() {
            return Ok((self.urlize)(&slug));
        }
        match self.page_to_permalink_content_base_name(p, a) {
            Ok(name) => Ok(name),
            Err(_) => Ok(String::new()),
        }
    }

    // Go: resources/page/permalinks.go:translationBaseName
    fn translation_base_name(&self, p: &dyn Page) -> String {
        match p.file() {
            None => String::new(),
            Some(f) => f.translation_base_name(),
        }
    }
}

/// `p.CurrentSection().SectionsEntries()` (a page without a current section: its own).
fn current_section_entries(p: &dyn Page) -> Vec<String> {
    match p.current_section() {
        Some(cs) => cs.0.sections_entries(),
        None => p.sections_entries(),
    }
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Go: `attributeRegexp.FindAllStringSubmatch(pattern, -1)` with
/// `attributeRegexp = regexp.MustCompile(`:\w+(\[.+?\])?`)`: the full matches, left to right.
/// `\w` is ASCII `[0-9A-Za-z_]`; `.` is any rune but `\n`; the group is lazy (the first `]`
/// after at least one rune).
fn find_attribute_matches(s: &str) -> Vec<String> {
    let b = s.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if b[i] == b':' {
            let mut j = i + 1;
            while j < n && is_word_byte(b[j]) {
                j += 1;
            }
            if j > i + 1 {
                let mut end = j;
                if j < n && b[j] == b'[' {
                    let mut m = j + 1;
                    while m < n {
                        if b[m] == b'\n' {
                            break;
                        }
                        if m >= j + 2 && b[m] == b']' {
                            end = m + 1;
                            break;
                        }
                        m += 1;
                    }
                }
                out.push(s[i..end].to_string());
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

type SliceFn = Arc<dyn Fn(&[String]) -> Result<Vec<String>> + Send + Sync>;

/// toSliceFunc returns a slice func that slices s according to the cut spec.
/// The cut spec must be on form [low:high] (one or both can be omitted),
/// also allowing single slice indices (e.g. [2]) and the special [last] keyword
/// giving the last element of the slice.
/// The returned function will be lenient and not panic in out of bound situations, except
/// for a low bound above the high bound, where Go panics (`slice bounds out of range`); the port
/// returns Go's panic message as an error.
// Go: resources/page/permalinks.go:toSliceFunc
fn to_slice_func(cut: &str) -> SliceFn {
    let cut =
        go_unicode::strings::to_lower_str(go_unicode::strings::trim_space_str(cut)).into_owned();
    if cut.is_empty() {
        return Arc::new(|s| Ok(s.to_vec()));
    }

    let cb = cut.as_bytes();
    if cb.len() < 3 || (cb[0] != b'[' || cb[cb.len() - 1] != b']') {
        return Arc::new(|_s| Ok(Vec::new()));
    }

    type ToN = Arc<dyn Fn(&[String]) -> i64 + Send + Sync>;
    let to_n_func = |s: &str, low: bool| -> ToN {
        if s.is_empty() {
            if low {
                return Arc::new(|_ss| 0);
            } else {
                return Arc::new(|ss| ss.len() as i64);
            }
        }

        if s == "last" {
            return Arc::new(|ss| ss.len() as i64 - 1);
        }

        let (mut n, _) = go_strconv::internal::atoi(s.as_bytes());
        if n < 0 {
            n = 0;
        }
        Arc::new(move |ss| {
            // Prevent out of bound situations. It would not make
            // much sense to panic here.
            if n >= ss.len() as i64 {
                if low {
                    return -1;
                }
                return ss.len() as i64;
            }
            n
        })
    };

    let ops_str = cut[1..cut.len() - 1].to_string();
    let opts: Vec<&str> = ops_str.split(':').collect();

    if !ops_str.contains(':') {
        let to_n = to_n_func(opts[0], true);
        return Arc::new(move |s| {
            if s.is_empty() {
                return Ok(Vec::new());
            }
            let n = to_n(s);
            if n < 0 {
                return Ok(Vec::new());
            }
            let v = &s[n as usize];
            if v.is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![v.clone()])
        });
    }

    let (to_n1, to_n2) = (to_n_func(opts[0], true), to_n_func(opts[1], false));

    Arc::new(move |s| {
        if s.is_empty() {
            return Ok(Vec::new());
        }
        let (n1, n2) = (to_n1(s), to_n2(s));
        if n1 < 0 || n2 < 0 {
            return Ok(Vec::new());
        }
        if n1 > n2 {
            return Err(Error::new(format!(
                "runtime error: slice bounds out of range [{n1}:{n2}]"
            )));
        }
        Ok(s[n1 as usize..n2 as usize].to_vec())
    })
}

/// Go: `permalinksKindsSupport`.
const PERMALINKS_KINDS_SUPPORT: [&str; 4] = [
    kinds::KIND_PAGE,
    kinds::KIND_SECTION,
    kinds::KIND_TAXONOMY,
    kinds::KIND_TERM,
];

/// Go's `%q` of a value in the permalinks errors (`fmt.Errorf("... %q ...", v)`).
fn fmt_q(v: &Value) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf("%q", std::slice::from_ref(v))).into_owned()
}

/// Go: `page.DecodePermalinksConfig(m)` — legacy flat form `posts = "..."` registers for `page` and `term`.
// Go: resources/page/permalinks.go:DecodePermalinksConfig
pub fn decode_permalinks_config(m: &Map) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    let mut permalinks_config: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    permalinks_config.insert(kinds::KIND_PAGE.to_string(), BTreeMap::new());
    permalinks_config.insert(kinds::KIND_SECTION.to_string(), BTreeMap::new());
    permalinks_config.insert(kinds::KIND_TAXONOMY.to_string(), BTreeMap::new());
    permalinks_config.insert(kinds::KIND_TERM.to_string(), BTreeMap::new());

    // Go iterates the map in random order: with several invalid entries the error reported is
    // random, and later plain-string entries overwrite earlier `[permalinks.<kind>]` ones in
    // random order. The port iterates in byte order.
    let config = nh_common::maps::params::clean_config_string_map(m);
    for (k, v) in config.entries.iter() {
        let k = String::from_utf8_lossy(k.as_bytes()).into_owned();
        match v {
            Value::String(s) => {
                // [permalinks]
                //   key = '...'

                // To successfully be backward compatible, "default" patterns need to be set for both page and term
                let s = String::from_utf8_lossy(s.as_bytes()).into_owned();
                permalinks_config
                    .get_mut(kinds::KIND_PAGE)
                    .expect("kind page")
                    .insert(k.clone(), s.clone());
                permalinks_config
                    .get_mut(kinds::KIND_TERM)
                    .expect("kind term")
                    .insert(k.clone(), s);
            }
            Value::Map(pm) if pm.ty == go_value::MapType::Params => {
                // [permalinks.key]
                //   xyz = ???

                if PERMALINKS_KINDS_SUPPORT.contains(&k.as_str()) {
                    // TODO: warn if we overwrite an already set value
                    for (k2, v2) in pm.entries.iter() {
                        match v2 {
                            Value::String(s2) => {
                                permalinks_config
                                    .get_mut(&k)
                                    .expect("supported kind")
                                    .insert(
                                        String::from_utf8_lossy(k2.as_bytes()).into_owned(),
                                        String::from_utf8_lossy(s2.as_bytes()).into_owned(),
                                    );
                            }
                            _ => {
                                return Err(Error::new(format!(
                                    "permalinks configuration invalid: unknown value {} for key {} for kind {}",
                                    fmt_q(v2),
                                    go_strconv::quote(k2.as_bytes()),
                                    go_strconv::quote(k.as_bytes())
                                )));
                            }
                        }
                    }
                } else {
                    return Err(Error::new(format!(
                        "permalinks configuration not supported for kind {}, supported kinds are [{}]",
                        go_strconv::quote(k.as_bytes()),
                        PERMALINKS_KINDS_SUPPORT.join(" ")
                    )));
                }
            }
            _ => {
                return Err(Error::new(format!(
                    "permalinks configuration invalid: unknown value {} for key {}",
                    fmt_q(v),
                    go_strconv::quote(k.as_bytes())
                )));
            }
        }
    }
    Ok(permalinks_config)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/permalinks.go (480 lines; 8/22 funcs executed)
//   types: PermalinkExpander, pageToPermaAttribute, permalinkExpandError
// OK L53-71: (p PermalinkExpander) callback(attr string) (pageToPermaAttribute, bool)
// OK L75-110: NewPermalinkExpander(urlize func(uri string) string, patterns map[string]map[string]string) (PermalinkExpander, error)
// OK L115-118: (l PermalinkExpander) normalizeEscapeSequencesIn(s string) (string, bool)
// OK L120-122: (l PermalinkExpander) normalizeEscapeSequencesOut(result string) string
// OK L125-132: (l PermalinkExpander) ExpandPattern(pattern string, p Page) (string, error)
// OK L136-148: (l PermalinkExpander) Expand(key string, p Page) (string, error)
// OK L153-157: init()
// OK L159-211: (l PermalinkExpander) getOrParsePattern(pattern string) (func(Page) (string, error), error)
// OK L213-228: (l PermalinkExpander) parse(patterns map[string]string) (map[string]func(Page) (string, error), error)
// OK L241-243: (pee *permalinkExpandError) Error() string
// OK L247-267: (l PermalinkExpander) pageToPermalinkDate(p Page, dateField string) (string, error)
// OK L270-272: (l PermalinkExpander) pageToPermalinkTitle(p Page, _ string) (string, error)
// OK L275-287: (l PermalinkExpander) pageToPermalinkFilename(p Page, _ string) (string, error)
// OK L290-295: (l PermalinkExpander) pageToPermalinkSlugElseTitle(p Page, a string) (string, error)
// OK L298-303: (l PermalinkExpander) pageToPermalinkSlugElseFilename(p Page, a string) (string, error)
// OK L305-307: (l PermalinkExpander) pageToPermalinkSection(p Page, _ string) (string, error)
// OK L309-311: (l PermalinkExpander) pageToPermalinkSections(p Page, _ string) (string, error)
// OK L314-316: (l PermalinkExpander) pageToPermalinkContentBaseName(p Page, _ string) (string, error)
// OK L319-328: (l PermalinkExpander) pageToPermalinkSlugOrContentBaseName(p Page, a string) (string, error)
// OK L330-335: (l PermalinkExpander) translationBaseName(p Page) string
// OK L353-432: (l PermalinkExpander) toSliceFunc(cut string) func(s []string) []string
// OK L437-480: DecodePermalinksConfig(m map[string]any) (map[string]map[string]string, error)
// ---------------------------------------------------------------------------
