//! The template API as data: every filter, function and test a neohugo template may call
//! ([`FUNCS`]), the top-level names of every render context ([`CONTEXTS`], [`HOOK_FIELDS`]), the
//! Hugo constructs that became Tera syntax ([`SYNTAX`]), the conversion rules
//! ([`CONVERSION_RULES`]) and the embedded templates ([`EMBEDDED_TEMPLATES`]).
//!
//! This module is the single source of truth (REWRITE_PLAN.md §4.6): `rust/docs/template-api.md`
//! is generated from it by [`template_api_markdown`] and checked by neohugo-testkit's contract
//! test, and `register_placeholders` registers a kwargs-checking stub for every entry. It depends
//! on nothing, so it is available without the crate's `runtime` feature.

use std::fmt::Write as _;

/// Whether a name is called as a filter (`x | name`), a function (`name()`) or a test (`x is name`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NameKind {
    Filter,
    Function,
    Test,
}

/// Who implements a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    /// A Tera 2.4.0 built-in; neohugo never overrides one.
    Builtin,
    /// tera-contrib 0.3, registered by `register_pure`.
    Contrib,
    /// neohugo: `neohugo-funcs` when pure, `neohugo-sitefuncs` when site-bound.
    Neohugo,
}

/// The render phases in which a name is available (REWRITE_PLAN.md §4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhaseAvail {
    /// Shortcodes, render hooks, `markdownify` / `render_string` only.
    Content,
    /// Layout jobs, `partial()`, `defer` and `execute_as_template` only.
    Layout,
    Both,
}

/// The documented type of a keyword argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArgType {
    Any,
    String,
    Int,
    Number,
    Bool,
    Array,
    Map,
    /// A page value (summary, link or full); read through `neohugo_funcs::PageArg`.
    Page,
    /// A resource view; read through `neohugo_funcs::ResourceArg`.
    Resource,
}

impl ArgType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::String => "string",
            Self::Int => "int",
            Self::Number => "number",
            Self::Bool => "bool",
            Self::Array => "array",
            Self::Map => "map",
            Self::Page => "page",
            Self::Resource => "resource",
        }
    }
}

/// Doc sections of `template-api.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    Logic,
    Collections,
    Pages,
    Strings,
    Encoding,
    Urls,
    Dates,
    Locale,
    Resources,
    Images,
    Templates,
    System,
    Tests,
}

impl Group {
    pub const ALL: [Self; 13] = [
        Self::Logic,
        Self::Collections,
        Self::Pages,
        Self::Strings,
        Self::Encoding,
        Self::Urls,
        Self::Dates,
        Self::Locale,
        Self::Resources,
        Self::Images,
        Self::Templates,
        Self::System,
        Self::Tests,
    ];

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Logic => "Logic, math and errors",
            Self::Collections => "Collections and maps",
            Self::Pages => "Pages, taxonomies, menus and pagination",
            Self::Strings => "Strings",
            Self::Encoding => "Encoding, escaping and hashing",
            Self::Urls => "URLs and paths",
            Self::Dates => "Dates",
            Self::Locale => "Language",
            Self::Resources => "Resources and assets",
            Self::Images => "Images",
            Self::Templates => "Templates",
            Self::System => "Environment, files and debugging",
            Self::Tests => "Tests",
        }
    }
}

/// One keyword argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kwarg {
    pub name: &'static str,
    pub ty: ArgType,
    pub required: bool,
}

/// One filter, function or test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FuncSpec {
    pub name: &'static str,
    pub kind: NameKind,
    pub source: Source,
    pub kwargs: &'static [Kwarg],
    /// Accepts keyword arguments beyond `kwargs` (`partial(name=…, any=…)`).
    pub rest_kwargs: bool,
    pub phase: PhaseAvail,
    /// The output is marked safe (never autoescaped).
    pub safe: bool,
    /// Needs the site model or the render scope `__nh` (neohugo-sitefuncs).
    pub site_bound: bool,
    pub group: Group,
    /// The Hugo functions or methods this replaces ("" when it has no Hugo counterpart).
    pub hugo: &'static str,
    pub doc: &'static str,
}

impl FuncSpec {
    /// The kind code of REWRITE_PLAN.md §4.6: `bi`, `tc`, `F`, `fn` or `T`, plus ` (s)` when site-bound.
    #[must_use]
    pub fn code(&self) -> String {
        let base = match (self.source, self.kind) {
            (Source::Builtin, _) => "bi",
            (Source::Contrib, _) => "tc",
            (Source::Neohugo, NameKind::Filter) => "F",
            (Source::Neohugo, NameKind::Function) => "fn",
            (Source::Neohugo, NameKind::Test) => "T",
        };
        if self.site_bound {
            format!("{base} (s)")
        } else {
            base.to_owned()
        }
    }

    /// The keyword argument called `name`, if declared.
    #[must_use]
    pub fn kwarg(&self, name: &str) -> Option<&'static Kwarg> {
        self.kwargs.iter().find(|k| k.name == name)
    }

    /// The call signature as written in a template, e.g. `x | sort_by(attribute=, reverse=?)`.
    #[must_use]
    pub fn signature(&self) -> String {
        let mut args: Vec<String> = self
            .kwargs
            .iter()
            .map(|k| {
                if k.required {
                    format!("{}=", k.name)
                } else {
                    format!("{}=?", k.name)
                }
            })
            .collect();
        if self.rest_kwargs {
            args.push("…".to_owned());
        }
        let parens = if args.is_empty() && self.kind != NameKind::Function {
            String::new()
        } else {
            format!("({})", args.join(", "))
        };
        match self.kind {
            NameKind::Filter => format!("x | {}{parens}", self.name),
            NameKind::Function => format!("{}{parens}", self.name),
            NameKind::Test => format!("x is {}{parens}", self.name),
        }
    }

    const fn new(
        kind: NameKind,
        group: Group,
        name: &'static str,
        hugo: &'static str,
        doc: &'static str,
    ) -> Self {
        Self {
            name,
            kind,
            source: Source::Neohugo,
            kwargs: &[],
            rest_kwargs: false,
            phase: PhaseAvail::Both,
            safe: false,
            site_bound: false,
            group,
            hugo,
            doc,
        }
    }
    const fn args(mut self, kwargs: &'static [Kwarg]) -> Self {
        self.kwargs = kwargs;
        self
    }
    const fn rest(mut self) -> Self {
        self.rest_kwargs = true;
        self
    }
    const fn builtin(mut self) -> Self {
        self.source = Source::Builtin;
        self
    }
    const fn contrib(mut self) -> Self {
        self.source = Source::Contrib;
        self
    }
    const fn site(mut self) -> Self {
        self.site_bound = true;
        self
    }
    const fn safe(mut self) -> Self {
        self.safe = true;
        self
    }
    const fn only(mut self, phase: PhaseAvail) -> Self {
        self.phase = phase;
        self
    }
}

const fn filter(
    group: Group,
    name: &'static str,
    hugo: &'static str,
    doc: &'static str,
) -> FuncSpec {
    FuncSpec::new(NameKind::Filter, group, name, hugo, doc)
}
const fn func(group: Group, name: &'static str, hugo: &'static str, doc: &'static str) -> FuncSpec {
    FuncSpec::new(NameKind::Function, group, name, hugo, doc)
}
const fn test(name: &'static str, hugo: &'static str, doc: &'static str) -> FuncSpec {
    FuncSpec::new(NameKind::Test, Group::Tests, name, hugo, doc)
}
const fn req(name: &'static str, ty: ArgType) -> Kwarg {
    Kwarg {
        name,
        ty,
        required: true,
    }
}
const fn opt(name: &'static str, ty: ArgType) -> Kwarg {
    Kwarg {
        name,
        ty,
        required: false,
    }
}

use ArgType as A;
use Group as G;
use PhaseAvail as P;

const PAGE_OPT: Kwarg = opt("page", A::Page);
const PAGE_REQ: Kwarg = req("page", A::Page);
const IMAGE_ARGS: &[Kwarg] = &[
    opt("width", A::Int),
    opt("height", A::Int),
    opt("format", A::String),
    opt("quality", A::Int),
    opt("filter", A::String),
    opt("anchor", A::String),
    opt("spec", A::String),
];
const PIPE_OPTIONS: &[Kwarg] = &[opt("options", A::Map)];
const REF_ARGS: &[Kwarg] = &[
    req("path", A::String),
    opt("lang", A::String),
    opt("output_format", A::String),
    PAGE_OPT,
];
const MENU_ARGS: &[Kwarg] = &[req("menu", A::String), req("entry", A::Map), PAGE_OPT];

/// Every filter, function and test (REWRITE_PLAN.md §4.6), including all Tera 2.4.0 built-ins.
/// Order: by [`Group`], then as a reader would look for them.
pub const FUNCS: &[FuncSpec] = &[
    // ── logic, math and errors ──
    func(G::Logic, "throw", "", "Aborts the render at once with `message`.").args(&[req("message", A::String)]).builtin(),
    func(G::Logic, "log_error", "errorf, erroridf", "Records an error with the template position; the build fails when it ends. Prints nothing.")
        .args(&[req("message", A::String)]),
    func(G::Logic, "log_warn", "warnf, warnidf", "Records a warning; `id` lets `ignoreLogs` suppress it. Prints nothing.")
        .args(&[req("message", A::String), opt("id", A::String)]),
    func(G::Logic, "max", "math.Max", "The largest of `values` (numbers).").args(&[req("values", A::Array)]),
    func(G::Logic, "min", "math.Min", "The smallest of `values` (numbers).").args(&[req("values", A::Array)]),
    filter(G::Logic, "abs", "math.Abs", "Absolute value.").builtin(),
    filter(G::Logic, "round", "math.Round, math.Ceil, math.Floor", "`method` is `common` (default), `ceil` or `floor`; `precision` digits after the point.")
        .args(&[opt("method", A::String), opt("precision", A::Int)]).builtin(),
    filter(G::Logic, "int", "int", "Converts to an integer; strings are parsed in `base` (default 10).").args(&[opt("base", A::Int)]).builtin(),
    filter(G::Logic, "float", "float", "Converts to a float.").builtin(),
    filter(G::Logic, "str", "string", "Converts to a string.").builtin(),
    // ── collections and maps ──
    filter(G::Collections, "length", "len", "Length of a string (characters), array or map.").builtin(),
    filter(G::Collections, "default", "", "`value` when the input is undefined (only then; `boolean=true` also replaces falsy values). Not Hugo's `default`.")
        .args(&[req("value", A::Any), opt("boolean", A::Bool)]).builtin(),
    filter(G::Collections, "default_if_empty", "default", "Hugo's `default`: `value` when the input is undefined, none, 0, \"\", or an empty array or map. `false` counts as set.")
        .args(&[req("value", A::Any)]),
    filter(G::Collections, "get", "index", "The map entry `key`, else `default`, else an error.").args(&[req("key", A::String), opt("default", A::Any)]).builtin(),
    filter(G::Collections, "get_path", "index m \"a\" \"b\"", "Walks `path` (keys and integer indices); none when a step is missing.").args(&[req("path", A::Array)]),
    filter(G::Collections, "first", "index l 0", "The first element, or none.").builtin(),
    filter(G::Collections, "last", "", "The last element, or none.").builtin(),
    filter(G::Collections, "nth", "index l n", "The element at index `n`, or none.").args(&[req("n", A::Int)]).builtin(),
    filter(G::Collections, "keys", "", "The keys of a map.").builtin(),
    filter(G::Collections, "values", "", "The values of a map.").builtin(),
    filter(G::Collections, "pairs", "", "`[key, value]` pairs of a map.").builtin(),
    filter(G::Collections, "sort_keys", "(range over a map)", "The map with its keys sorted; Go ranges maps in key order, Tera literals keep insertion order."),
    filter(G::Collections, "append", "append", "The array with `value` appended.").args(&[req("value", A::Any)]),
    filter(G::Collections, "concat", "append l1 l2", "The array followed by the elements of `with`.").args(&[req("with", A::Array)]),
    filter(G::Collections, "merge", "merge", "Deep merge of two maps; `with` wins; keys sorted.").args(&[req("with", A::Map)]),
    filter(G::Collections, "join", "delimit", "Joins with `sep`.").args(&[opt("sep", A::String)]).builtin(),
    filter(G::Collections, "delimit", "delimit l sep last", "Joins with `sep`, and `last` before the final element.").args(&[req("sep", A::String), opt("last", A::String)]),
    filter(G::Collections, "reverse", ".Reverse", "Reversed array or string.").builtin(),
    filter(G::Collections, "unique", "uniq", "Removes duplicates, keeping the first.").builtin(),
    filter(G::Collections, "sort", "", "Tera's sort (by value, or by `attribute` path); not locale-aware. Use `sort_by` for Hugo's `sort`.")
        .args(&[opt("attribute", A::String)]).builtin(),
    filter(G::Collections, "group_by", "", "Tera's grouping by `attribute` path into a map.").args(&[req("attribute", A::String)]).builtin(),
    filter(G::Collections, "sort_by", "sort", "Sorts by `attribute` (a path such as `params.weight`; `\"\"` or `value`: the elements): collation of the render's `lang`, dates as instants, stable.")
        .args(&[req("attribute", A::String), opt("reverse", A::Bool)]),
    filter(G::Collections, "complement", "complement", "Elements not in `without` (pages compared by id).").args(&[req("without", A::Array)]),
    filter(G::Collections, "union", "union", "Elements of either array, first occurrence kept (pages by id).").args(&[req("with", A::Array)]),
    filter(G::Collections, "intersect", "intersect", "Elements present in both (pages by id).").args(&[req("with", A::Array)]),
    filter(G::Collections, "symdiff", "symdiff", "Elements present in exactly one (pages by id).").args(&[req("with", A::Array)]),
    func(G::Collections, "range", "seq", "Integers from `start` (default 0) to `end` (exclusive) by `step_by`; Hugo `seq N` is `range(start=1, end=N+1)`.")
        .args(&[opt("start", A::Int), req("end", A::Int), opt("step_by", A::Int)]).builtin(),
    func(G::Collections, "querify", "querify", "A URL query string from `params`, keys sorted.").args(&[req("params", A::Map)]),
    // ── pages, taxonomies, menus, pagination ──
    func(G::Pages, "get_page", "site.GetPage, .GetPage", "The full value of the page at `path` (relative paths resolve against `page`), or none.")
        .args(&[req("path", A::String), opt("lang", A::String), PAGE_OPT]).site(),
    filter(G::Pages, "deref", "", "The full value (with relations) of a listed summary page.").site(),
    func(G::Pages, "get_terms", ".GetTerms", "The term links of `page` for `taxonomy`; `[]` when it is not a taxonomy.")
        .args(&[req("taxonomy", A::String), PAGE_OPT]).site(),
    func(G::Pages, "related", ".Related", "Pages related to `page` among `pages`, per the `related` config.")
        .args(&[req("pages", A::Array), PAGE_OPT, opt("indices", A::Array), opt("limit", A::Int)]).site(),
    func(G::Pages, "param", ".Param", "The page param `key` (a dotted path), else the site param.").args(&[req("key", A::String), PAGE_OPT]).site(),
    func(G::Pages, "paginator", ".Paginator", "The pager of the current (page, format) over its default list. Recorded: the first call wins.")
        .site().only(P::Layout),
    func(G::Pages, "paginate", ".Paginate", "The pager over `pages`. A re-call with another list or size is an error naming both positions.")
        .args(&[req("pages", A::Array), opt("size", A::Int)]).site().only(P::Layout),
    func(G::Pages, "store_set", ".Store.Set", "Sets `key` in the page store (content-phase writes are buffered per transaction). Prints nothing.")
        .args(&[req("key", A::String), req("value", A::Any), PAGE_OPT]).site(),
    func(G::Pages, "store_get", ".Store.Get", "Reads `key` from the page store, or none.").args(&[req("key", A::String), PAGE_OPT]).site(),
    func(G::Pages, "page_content", ".Content (another page, content phase)", "The rendered content of `page`; memoised and cycle-checked.")
        .args(&[PAGE_REQ]).site().safe(),
    func(G::Pages, "page_summary", ".Summary (content phase)", "The summary of `page`.").args(&[PAGE_REQ]).site().safe(),
    func(G::Pages, "page_plain", ".Plain (content phase)", "The content of `page` as plain text.").args(&[PAGE_REQ]).site(),
    func(G::Pages, "page_word_count", ".WordCount (content phase)", "The word count of `page`.").args(&[PAGE_REQ]).site(),
    func(G::Pages, "page_fragments", ".Fragments (content phase)", "`{headings, identifiers}` of `page`.").args(&[PAGE_REQ]).site(),
    func(G::Pages, "page_toc", ".TableOfContents (content phase)", "The table of contents of `page`.").args(&[PAGE_REQ]).site().safe(),
    func(G::Pages, "render_shortcodes", ".RenderShortcodes", "The source of `page` with its shortcodes expanded (placeholders renumbered).")
        .args(&[PAGE_REQ]).site().safe(),
    func(G::Pages, "is_menu_current", ".IsMenuCurrent", "Whether `entry` of `menu` points at `page`.").args(MENU_ARGS).site(),
    func(G::Pages, "has_menu_current", ".HasMenuCurrent", "Whether a child of `entry` of `menu` points at `page`.").args(MENU_ARGS).site(),
    filter(G::Pages, "by_title", ".ByTitle", "Pages by title (collation of the language).").site(),
    filter(G::Pages, "by_link_title", ".ByLinkTitle", "Pages by link title.").site(),
    filter(G::Pages, "by_date", ".ByDate", "Pages by date, oldest first.").site(),
    filter(G::Pages, "by_publish_date", ".ByPublishDate", "Pages by publish date.").site(),
    filter(G::Pages, "by_lastmod", ".ByLastmod", "Pages by last modification.").site(),
    filter(G::Pages, "by_weight", ".ByWeight", "Pages by Hugo's default order (weight, date, link title, path).").site(),
    filter(G::Pages, "group_by_date", ".GroupByDate", "`[{key, pages}]` grouped by the date formatted with `format` (strftime), newest first.")
        .args(&[req("format", A::String), opt("attribute", A::String)]).site(),
    filter(G::Pages, "group_by_param", ".GroupByParam", "`[{key, pages}]` grouped by the page param `param`.").args(&[req("param", A::String)]).site(),
    filter(G::Pages, "by_count", ".ByCount", "Taxonomy terms by page count, then name.").site(),
    filter(G::Pages, "alphabetical", ".Alphabetical", "Taxonomy terms by name (collation).").site(),
    // ── strings ──
    filter(G::Strings, "lower", "lower", "Lower case.").builtin(),
    filter(G::Strings, "upper", "upper", "Upper case.").builtin(),
    filter(G::Strings, "capitalize", "", "First character upper, the rest lower.").builtin(),
    filter(G::Strings, "title", "", "Tera's naive title case. Hugo's `title` is `title_case`.").builtin(),
    filter(G::Strings, "title_case", "title, strings.Title", "Title case in `style` (`ap`, `chicago`, `go`, `firstupper`, `none`; default: `titleCaseStyle`).")
        .args(&[opt("style", A::String)]),
    filter(G::Strings, "wordcount", "", "Tera's word count (whitespace split).").builtin(),
    filter(G::Strings, "trim", "strings.TrimSpace", "Trims whitespace, or the string `pat` repeatedly.").args(&[opt("pat", A::String)]).builtin(),
    filter(G::Strings, "trim_start", "", "Trims leading whitespace, or `pat`.").args(&[opt("pat", A::String)]).builtin(),
    filter(G::Strings, "trim_end", "", "Trims trailing whitespace, or `pat`.").args(&[opt("pat", A::String)]).builtin(),
    filter(G::Strings, "trim_chars", "trim, strings.Trim", "Trims any of the characters in `chars` from both ends.").args(&[req("chars", A::String)]),
    filter(G::Strings, "trim_start_chars", "strings.TrimLeft", "Trims any of `chars` from the start.").args(&[req("chars", A::String)]),
    filter(G::Strings, "trim_end_chars", "strings.TrimRight", "Trims any of `chars` from the end.").args(&[req("chars", A::String)]),
    filter(G::Strings, "strip_prefix", "strings.TrimPrefix", "Removes `prefix` once.").args(&[req("prefix", A::String)]),
    filter(G::Strings, "strip_suffix", "strings.TrimSuffix", "Removes `suffix` once.").args(&[req("suffix", A::String)]),
    filter(G::Strings, "replace", "replace, strings.Replace", "Replaces every `from` with `to`.").args(&[req("from", A::String), req("to", A::String)]).builtin(),
    filter(G::Strings, "split", "split", "Splits on `pat`.").args(&[req("pat", A::String)]).builtin(),
    filter(G::Strings, "regex_replace", "replaceRE", "Replaces matches of `pattern` with `rep` (`$1` groups).").args(&[req("pattern", A::String), req("rep", A::String)]).contrib(),
    filter(G::Strings, "regex_find", "findRE", "Matches of `pattern`, at most `limit`.").args(&[req("pattern", A::String), opt("limit", A::Int)]),
    filter(G::Strings, "substr", "substr", "`length` characters from `start` (negative counts from the end).").args(&[req("start", A::Int), opt("length", A::Int)]),
    filter(G::Strings, "truncate", "", "Tera's plain-text truncate to `length` characters plus `end`.").args(&[req("length", A::Int), opt("end", A::String)]).builtin(),
    filter(G::Strings, "truncate_html", "truncate", "Hugo's HTML-aware truncate: closes open tags, `ellipsis` default `…`. Keeps the input's safety.")
        .args(&[req("length", A::Int), opt("ellipsis", A::String)]),
    filter(G::Strings, "pad_start", "printf \"%5s\"", "Pads on the left with spaces to `width` characters.").args(&[req("width", A::Int)]),
    filter(G::Strings, "pad_end", "printf \"%-35s\"", "Pads on the right with spaces to `width` characters.").args(&[req("width", A::Int)]),
    filter(G::Strings, "indent", "", "Tera's indent.").args(&[opt("width", A::Int), opt("indentation", A::String), opt("first", A::Bool), opt("blank", A::Bool)]).builtin(),
    filter(G::Strings, "newlines_to_br", "", "Replaces line breaks with `<br>`.").builtin(),
    filter(G::Strings, "pluralize", "", "Tera's suffix pluralizer for a count (`singular`, `plural`). Hugo's `inflect.Pluralize` is `pluralize_word`.")
        .args(&[opt("singular", A::String), opt("plural", A::String)]).builtin(),
    filter(G::Strings, "pluralize_word", "inflect.Pluralize, pluralize", "English plural of a word."),
    filter(G::Strings, "singularize_word", "inflect.Singularize, singularize", "English singular of a word."),
    filter(G::Strings, "humanize", "humanize", "Hugo's humanize (`my-first-post` → `My first post`; numbers → ordinals)."),
    filter(G::Strings, "ordinalize", "humanize (numbers)", "`1` → `1st`."),
    filter(G::Strings, "urlize", "urlize", "Hugo's URL-safe path form of a string."),
    filter(G::Strings, "anchorize", "anchorize", "An anchor id as Hugo generates it; `style` `github` (default), `github-ascii` or `blackfriday`.")
        .args(&[opt("style", A::String)]),
    filter(G::Strings, "plainify", "plainify", "Strips HTML tags."),
    filter(G::Strings, "emojify", "emojify", "Replaces `:shortcode:` emoji.").safe(),
    filter(G::Strings, "markdownify", "markdownify", "Renders Markdown with the current page's hooks; a single paragraph is unwrapped.").site().safe(),
    filter(G::Strings, "render_string", ".RenderString", "Renders Markdown with `page`'s hooks; `display=\"block\"` keeps the paragraph.")
        .args(&[opt("display", A::String), PAGE_OPT]).site().safe(),
    filter(G::Strings, "highlight", "highlight, transform.Highlight", "Syntax highlighting of the input as `lang` (Chroma classes, or inline styles per `noClasses`; needs the site's highlight configuration).")
        .args(&[req("lang", A::String), opt("options", A::Any)]).site().safe(),
    filter(G::Strings, "to_math", "transform.ToMath", "LaTeX to MathML (SHOULD; feature `math`).").args(&[opt("display", A::Bool)]).safe(),
    func(G::Strings, "diagrams_goat", "diagrams.Goat", "`{inner (safe SVG), width, height, wrapped}` for the ASCII diagram `text` (SHOULD; feature `goat`).")
        .args(&[req("text", A::String)]),
    filter(G::Strings, "format_number", "lang.FormatNumber, printf \"%.1f\"", "The number with `precision` decimals in the format of the render's `lang`.")
        .args(&[opt("precision", A::Int)]),
    filter(G::Strings, "filesize_format", "", "Human file size (`binary` units by default).").args(&[opt("binary", A::Bool)]).contrib(),
    // ── encoding, escaping, hashing ──
    filter(G::Encoding, "safe", "safeHTML, safeHTMLAttr, safeURL, safeJS, safeCSS", "Marks the value safe.").builtin(),
    filter(G::Encoding, "escape", "", "Tera's escape (leaves safe input alone). Hugo's `html` is `html_escape`.").builtin(),
    filter(G::Encoding, "escape_html", "", "Tera's HTML escape of a string.").builtin(),
    filter(G::Encoding, "escape_xml", "", "Tera's XML escape (`&quot;`, `&apos;`; leaves safe input alone). Hugo's `transform.XMLEscape` is `xml_escape`.").builtin(),
    filter(G::Encoding, "xml_escape", "transform.XMLEscape", "Drops the characters XML forbids, then escapes `& < > \" '`, tab, newline and CR (`&#34; &#39; &#x9; &#xA; &#xD;`, Go's `xml.EscapeText`) even when the input is safe; the result is safe.").safe(),
    filter(G::Encoding, "html_escape", "html, htmlEscape, transform.HTMLEscape", "Escapes `& < > \" '` even when the input is safe; the result is safe.").safe(),
    filter(G::Encoding, "html_unescape", "htmlUnescape, transform.HTMLUnescape", "Decodes HTML entities."),
    filter(G::Encoding, "jsonify", "jsonify", "JSON with sorted keys, `<>&` escaped as `\\u003c…`; `indent` pretty-prints.").args(&[opt("indent", A::String)]).safe(),
    filter(G::Encoding, "unmarshal", "transform.Unmarshal", "Parses a string or resource as JSON, TOML, YAML, CSV or XML (`format` overrides detection); keys sorted.")
        .args(&[opt("format", A::String)]).site(),
    filter(G::Encoding, "remarshal", "transform.Remarshal", "Re-encodes data as `format` (`toml`, `yaml`, `json`).").args(&[req("format", A::String)]),
    filter(G::Encoding, "urlencode", "urlquery", "Percent-encodes for a URL path (keeps `/`).").contrib(),
    filter(G::Encoding, "urlencode_strict", "urlquery", "Percent-encodes every non-alphanumeric character.").contrib(),
    filter(G::Encoding, "urldecode", "urls.PathUnescape", "Decodes percent-encoding."),
    filter(G::Encoding, "b64_encode", "base64Encode", "Base64 (`url_safe`, `padded`).").args(&[opt("url_safe", A::Bool), opt("padded", A::Bool)]).contrib(),
    filter(G::Encoding, "b64_decode", "base64Decode", "Decodes base64.").args(&[opt("url_safe", A::Bool)]).contrib(),
    filter(G::Encoding, "md5", "md5, crypto.MD5", "Hex MD5."),
    filter(G::Encoding, "sha1", "sha1", "Hex SHA-1."),
    filter(G::Encoding, "sha256", "sha256", "Hex SHA-256."),
    filter(G::Encoding, "fnv32a", "hash.FNV32a", "FNV-1a 32-bit hash as an integer."),
    filter(G::Encoding, "xxhash", "hash.XxHash", "Hex xxHash64."),
    // ── URLs and paths ──
    filter(G::Urls, "abs_url", "absURL", "Absolute URL against `baseURL` (base path kept).").site(),
    filter(G::Urls, "rel_url", "relURL", "Root-relative URL with the base path.").site(),
    filter(G::Urls, "abs_lang_url", "absLangURL", "`abs_url` with the language prefix of `page`'s language.").args(&[PAGE_OPT]).site(),
    filter(G::Urls, "rel_lang_url", "relLangURL", "`rel_url` with the language prefix of `page`'s language.").args(&[PAGE_OPT]).site(),
    func(G::Urls, "ref", "ref", "The permalink of the page at `path`; unresolved per `refLinksErrorLevel`.").args(REF_ARGS).site(),
    func(G::Urls, "rel_ref", "relref", "The relative permalink of the page at `path`.").args(REF_ARGS).site(),
    filter(G::Urls, "parse_url", "urls.Parse", "`{scheme, host, path, fragment, query, is_absolute, string}`."),
    func(G::Urls, "join_url", "urls.JoinPath", "Joins URL `parts` with single slashes.").args(&[req("parts", A::Array)]),
    filter(G::Urls, "path_ext", "path.Ext", "Extension with the dot."),
    filter(G::Urls, "path_base", "path.Base", "Last element."),
    filter(G::Urls, "path_base_name", "path.BaseName", "Last element without extension."),
    filter(G::Urls, "path_dir", "path.Dir", "All but the last element."),
    filter(G::Urls, "path_clean", "path.Clean", "Lexically cleaned path."),
    func(G::Urls, "path_join", "path.Join", "Joins `parts` and cleans the result.").args(&[req("parts", A::Array)]),
    // ── dates ──
    func(G::Dates, "now", "now", "The build time (honours `--clock`) as a date value."),
    filter(G::Dates, "date", ".Format, time.Format, dateFormat", "Formats a date with strftime `format` or `style` (`short`, `medium`, `long`, `full`). A style is localized in `locale` (default: the render's `lang`; Thai uses the Gregorian calendar); a `format`'s month and weekday names are English (Go's `.Format`) unless `locale` is given (`time.Format`, `dateFormat`: `locale=lang`). Accepts a date value, a date string or Unix seconds; none prints nothing.")
        .args(&[opt("format", A::String), opt("style", A::String), opt("locale", A::String)]),
    filter(G::Dates, "to_date", "time.AsTime, time", "Parses a string or number into a date value (`{rfc3339, unix}`)."),
    // ── language ──
    func(G::Locale, "i18n", "i18n, T", "The translation of `key` in `page`'s language; `count` picks the plural form, `data` fills `{{ .Field }}`.")
        .args(&[req("key", A::String), opt("count", A::Number), opt("data", A::Any), PAGE_OPT]).site(),
    // ── resources ──
    func(G::Resources, "get_asset", "resources.Get", "The asset at `path` under `assets/`, or none.").args(&[req("path", A::String)]).site(),
    func(G::Resources, "find_asset", "resources.GetMatch", "The first asset matching the glob `pattern`, or none.").args(&[req("pattern", A::String)]).site(),
    func(G::Resources, "find_assets", "resources.Match", "All assets matching `pattern`.").args(&[req("pattern", A::String)]).site(),
    func(G::Resources, "get_remote", "resources.GetRemote, try", "A remote resource (`options`: headers, method, body, key). Errors propagate unless `optional=true` (then none and a warning).")
        .args(&[req("url", A::String), opt("options", A::Map), opt("optional", A::Bool)]).site(),
    func(G::Resources, "concat_assets", "resources.Concat", "Concatenates `items` into a resource at `target`.").args(&[req("target", A::String), req("items", A::Array)]).site(),
    func(G::Resources, "asset_from_string", "resources.FromString", "A resource at `target` with `content`.").args(&[req("target", A::String), req("content", A::String)]).site(),
    filter(G::Resources, "get_resource", ".Resources.Get", "The resource named `name` in a list (case-insensitive), or none.").args(&[req("name", A::String)]),
    filter(G::Resources, "find_resource", ".Resources.GetMatch", "The first resource matching the glob `pattern`, or none.").args(&[req("pattern", A::String)]),
    filter(G::Resources, "find_resources", ".Resources.Match", "All resources matching `pattern`.").args(&[req("pattern", A::String)]),
    filter(G::Resources, "by_type", ".Resources.ByType", "Resources whose type is `type` (`image`, `page`, …).").args(&[req("type", A::String)]),
    filter(G::Resources, "fingerprint", "fingerprint", "The resource renamed with its hash (`algo`: sha256 default, sha384, sha512, md5); sets `data.integrity`.")
        .args(&[opt("algo", A::String)]).site(),
    filter(G::Resources, "minify", "minify", "The minified resource.").site(),
    filter(G::Resources, "resource_content", ".Content (resource)", "The text of a resource; for a bundled content page, its rendered HTML (marked safe).").site(),
    filter(G::Resources, "publish", ".Publish", "Publishes the resource and returns it.").site(),
    filter(G::Resources, "to_css", "toCSS, css.Sass", "Sass/SCSS to CSS.").args(PIPE_OPTIONS).site(),
    filter(G::Resources, "postcss", "postCSS, css.PostCSS", "Runs PostCSS.").args(PIPE_OPTIONS).site(),
    filter(G::Resources, "tailwind", "css.TailwindCSS", "Runs the Tailwind CLI.").args(PIPE_OPTIONS).site(),
    filter(G::Resources, "babel", "babel, js.Babel", "Runs Babel.").args(PIPE_OPTIONS).site(),
    filter(G::Resources, "js_build", "js.Build", "Bundles with esbuild.").args(PIPE_OPTIONS).site(),
    filter(G::Resources, "execute_as_template", "resources.ExecuteAsTemplate", "Renders the asset as a Tera template with `data`, published at `target`.")
        .args(&[req("target", A::String), opt("data", A::Any)]).site(),
    filter(G::Resources, "post_process", "resources.PostProcess", "Defers the resource's fields until all pages are rendered.").site(),
    // ── images ──
    filter(G::Images, "resize", ".Resize", "Resizes to `width` and/or `height` (or a Hugo `spec`).").args(IMAGE_ARGS).site(),
    filter(G::Images, "fill", ".Fill", "Crops and resizes to fill `width`×`height` at `anchor`.").args(IMAGE_ARGS).site(),
    filter(G::Images, "fit", ".Fit", "Downscales to fit `width`×`height`.").args(IMAGE_ARGS).site(),
    filter(G::Images, "crop", ".Crop", "Crops to `width`×`height` at `anchor`.").args(IMAGE_ARGS).site(),
    filter(G::Images, "process", ".Process", "Any of the above per `spec` (or the typed kwargs).").args(IMAGE_ARGS).site(),
    filter(G::Images, "image_filter", "images.Filter, .Filter", "Applies `filters`, a list of `{\"op\": …}` maps (overlay, grayscale, …).").args(&[req("filters", A::Array)]).site(),
    filter(G::Images, "exif", ".Exif", "EXIF data of an image, or none.").site(),
    filter(G::Images, "image_colors", ".Colors", "Dominant colours as hex strings.").site(),
    func(G::Images, "qr_code", "images.QR", "A QR code image of `text` (COULD).")
        .args(&[req("text", A::String), opt("level", A::String), opt("scale", A::Int), opt("target_dir", A::String)]).site(),
    // ── templates ──
    func(G::Templates, "super", "", "The parent block's content (inside `{% block %}` only).").builtin(),
    func(G::Templates, "partial", "partial (dynamic name or returned value)", "Renders `_partials/<name>` with the kwargs as top-level names; returns its `return_value` or the rendered string.")
        .args(&[req("name", A::String)]).rest().site().safe(),
    func(G::Templates, "partial_cached", "partialCached", "`partial` memoised on (`name`, `key`).").args(&[req("name", A::String), req("key", A::Any)]).rest().site().safe(),
    func(G::Templates, "return_value", "return", "Sets the value the enclosing `partial()` returns. Prints nothing.").args(&[req("value", A::Any)]).site(),
    func(G::Templates, "template_exists", "templates.Exists", "Whether a template called `name` exists.").args(&[req("name", A::String)]).site(),
    func(G::Templates, "defer", "templates.Defer", "A placeholder; `template` is rendered once per `key` with `data` after all pages.")
        .args(&[req("template", A::String), req("key", A::String), opt("data", A::Any)]).site().safe().only(P::Layout),
    filter(G::Templates, "arg", ".Get (shortcodes)", "A shortcode argument by position `index` or by `name`, else `default`: `shortcode | arg(index=0, default=\"\")`.")
        .args(&[opt("index", A::Int), opt("name", A::String), opt("default", A::Any)]).only(P::Content),
    // ── environment, files, debugging ──
    func(G::System, "get_env", "os.Getenv", "An environment variable (\"\" when unset); `name` must match `security.funcs.getenv`, else an error.").args(&[req("name", A::String)]),
    func(G::System, "read_file", "os.ReadFile", "A file of the project (`security` rules apply).").args(&[req("path", A::String)]),
    func(G::System, "file_exists", "os.FileExists", "Whether a project file exists.").args(&[req("path", A::String)]),
    filter(G::System, "dump", "debug.Dump", "Pretty-printed JSON of any value."),
    // ── tests ──
    test("defined", "isset", "The value is defined.").builtin(),
    test("undefined", "", "The value is undefined.").builtin(),
    test("none", ".IsZero (dates)", "The value is none (zero dates serialise as none).").builtin(),
    test("string", "", "A string.").builtin(),
    test("number", "", "A number.").builtin(),
    test("integer", "", "An integer.").builtin(),
    test("float", "", "A float.").builtin(),
    test("bool", "", "A bool.").builtin(),
    test("map", "reflect.IsMap", "A map.").builtin(),
    test("array", "reflect.IsSlice", "An array.").builtin(),
    test("iterable", "", "An array, map or string.").builtin(),
    test("odd", "", "An odd number.").builtin(),
    test("even", "", "An even number.").builtin(),
    test("divisible_by", "", "Divisible by `divisor`.").args(&[req("divisor", A::Int)]).builtin(),
    test("starting_with", "strings.HasPrefix", "Starts with `pat`.").args(&[req("pat", A::String)]).builtin(),
    test("ending_with", "strings.HasSuffix", "Ends with `pat`.").args(&[req("pat", A::String)]).builtin(),
    test("containing", "in, strings.Contains", "Contains `pat` (substring, element or key).").args(&[req("pat", A::Any)]).builtin(),
    test("matching", "findRE (as a condition), where … \"like\"", "Matches the regex `pat`.").args(&[req("pat", A::String)]).contrib(),
    test("version_at_least", "hugo.Version comparisons", "A semver at least `version`; a `-DEV` build ranks below its release.").args(&[req("version", A::String)]),
];

/// The kinds of render, each with its own set of top-level names (REWRITE_PLAN.md §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RenderRole {
    LayoutJob,
    Shortcode,
    RenderHook,
    Partial,
    Component,
    Deferred,
    ExecuteAsTemplate,
    Alias,
    Standalone,
    SitemapIndex,
}

/// One top-level name of a render context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextName {
    pub name: &'static str,
    pub doc: &'static str,
}

/// The top-level names of one kind of render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextSpec {
    pub role: RenderRole,
    pub title: &'static str,
    pub names: &'static [ContextName],
    /// Further names that depend on the call (kwargs, flattened hook fields, component arguments).
    pub note: &'static str,
}

const fn cn(name: &'static str, doc: &'static str) -> ContextName {
    ContextName { name, doc }
}

const SITE: ContextName = cn("site", "`SiteView` of the current language");
const HUGO: ContextName = cn("hugo", "`HugoView`: version, environment, generator");
const LANG: ContextName = cn("lang", "the language code of the page");
const OUTPUT_FORMAT: ContextName = cn("output_format", "`OutputFormatView` being rendered");
const NH: ContextName = cn(
    "__nh",
    "the render scope (`RenderScope`); read by site-bound functions",
);
const DATA: ContextName = cn("data", "the `data=` value of the call");

/// The top-level names of every render (REWRITE_PLAN.md §4.2).
pub const CONTEXTS: &[ContextSpec] = &[
    ContextSpec {
        role: RenderRole::LayoutJob,
        title: "Layout job",
        names: &[
            cn("page", "the full page value of the Full generation"),
            SITE,
            HUGO,
            LANG,
            OUTPUT_FORMAT,
            NH,
        ],
        note: "",
    },
    ContextSpec {
        role: RenderRole::Shortcode,
        title: "Shortcode",
        names: &[
            cn(
                "page",
                "the full page value of the Meta generation: relations yes, content fields no",
            ),
            SITE,
            HUGO,
            LANG,
            cn(
                "shortcode",
                "`ShortcodeView`: name, args, params, is_named_params, ordinal, parent, position",
            ),
            cn("inner", "the inner content (safe)"),
            cn(
                "inner_deindent",
                "the inner content without common indentation",
            ),
            NH,
        ],
        note: "",
    },
    ContextSpec {
        role: RenderRole::RenderHook,
        title: "Render hook",
        names: &[
            cn("page", "the page being rendered (Meta generation)"),
            cn(
                "page_inner",
                "the page whose source holds the hooked node (differs inside `render_shortcodes`)",
            ),
            SITE,
            HUGO,
            LANG,
            NH,
        ],
        note: "plus the hook's fields, flattened (below)",
    },
    ContextSpec {
        role: RenderRole::Partial,
        title: "`partial(name=…, …)`",
        names: &[
            cn("page", "the caller's page"),
            SITE,
            HUGO,
            LANG,
            OUTPUT_FORMAT,
            cn(
                "__nh",
                "a child scope: same page, format and pager; new frame; depth + 1",
            ),
        ],
        note: "plus the call's kwargs as top-level names",
    },
    ContextSpec {
        role: RenderRole::Component,
        title: "Component",
        names: &[],
        note: "only its declared arguments; `@page`, `@site`, `@lang` and `@__nh` may be declared as implicit arguments",
    },
    ContextSpec {
        role: RenderRole::Deferred,
        title: "`defer` template",
        names: &[
            DATA,
            SITE,
            HUGO,
            cn("__nh", "the render scope, phase `Deferred`"),
        ],
        note: "",
    },
    ContextSpec {
        role: RenderRole::ExecuteAsTemplate,
        title: "`execute_as_template`",
        names: &[DATA, SITE, HUGO, NH],
        note: "",
    },
    ContextSpec {
        role: RenderRole::Alias,
        title: "Alias",
        names: &[
            cn("permalink", "the target URL"),
            cn("page", "the target page (link value)"),
            SITE,
            HUGO,
        ],
        note: "",
    },
    ContextSpec {
        role: RenderRole::Standalone,
        title: "Sitemap, robots, 404",
        names: &[
            cn("page", "the standalone page; its `pages` is `site.pages`"),
            SITE,
            HUGO,
            LANG,
            NH,
        ],
        note: "",
    },
    ContextSpec {
        role: RenderRole::SitemapIndex,
        title: "Sitemapindex",
        names: &[
            cn("page", "the standalone page"),
            SITE,
            HUGO,
            LANG,
            NH,
            cn("sites", "`[{language, sitemap_abs_url, last_mod}]`"),
        ],
        note: "",
    },
];

/// The fields a render hook sees flattened into its context, per hook kind (REWRITE_PLAN.md §4.2).
pub const HOOK_FIELDS: &[(&str, &[&str])] = &[
    (
        "link, image",
        &[
            "destination",
            "title",
            "text",
            "plain_text",
            "is_block",
            "attributes",
            "ordinal",
            "position",
        ],
    ),
    (
        "heading",
        &["level", "anchor", "text", "plain_text", "attributes"],
    ),
    (
        "codeblock",
        &[
            "type",
            "inner",
            "options",
            "attributes",
            "ordinal",
            "position",
        ],
    ),
    (
        "blockquote",
        &[
            "type",
            "alert_type",
            "alert_title",
            "alert_sign",
            "text",
            "attributes",
            "ordinal",
        ],
    ),
    ("table", &["thead", "tbody", "attributes", "ordinal"]),
    (
        "passthrough",
        &["type", "inner", "attributes", "ordinal", "position"],
    ),
];

/// A Hugo construct that became Tera syntax (an operator, literal, statement or view field).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxRule {
    pub hugo: &'static str,
    pub tera: &'static str,
}

const fn syn(hugo: &'static str, tera: &'static str) -> SyntaxRule {
    SyntaxRule { hugo, tera }
}

/// The `op` rows of REWRITE_PLAN.md §4.6.
pub const SYNTAX: &[SyntaxRule] = &[
    syn(
        "`and` `or` `not` `eq` `ne` `lt` `le` `gt` `ge`",
        "`and` `or` `not` `==` `!=` `<` `<=` `>` `>=`; pages compare by `.id`, pagers by `.page_number`, dates by `.unix`",
    ),
    syn("`cond c a b`", "`a if c else b`"),
    syn(
        "`print`, `printf`",
        "`~`, plus the filters `pad_start`, `pad_end`, `round`, `format_number`, `jsonify`; `\"\\u{a0}\"` for `%c`; `'\"' ~ x ~ '\"'` for `%q` in attributes",
    ),
    syn(
        "`dict`, `slice`",
        "map and array literals `{\"k\": v}`, `[a, b]`",
    ),
    syn("`index m k`", "`m[k]`, `m.k`"),
    syn(
        "`in`, `strings.Contains`",
        "`x in l`, `\"x\" in s`; pages `p.id in [q.id for q in l]`",
    ),
    syn("`isset m \"k\"`", "`\"k\" in m`, `is defined`"),
    syn(
        "`first N`, `last N`, `after N`",
        "`l[:N]`, `l[-N:]`, `l[N:]`",
    ),
    syn(
        "`where`",
        "`[p for p in pages if p.params.x == v]`; `in`/`intersect` via ids; `like` via `is matching(pat=)`",
    ),
    syn("`apply l \"float\" \".\"`", "`[x | float for x in l]`"),
    syn(
        "`newScratch`, `.Scratch.*`",
        "`{% set %}`, `{% set_global %}`, `merge`",
    ),
    syn(
        "`add` `sub` `mul` `div` `mod`",
        "`+ - * / %`; `//` for Go's integer `div`",
    ),
    syn("`.GetTerms \"tags\"`", "`page.terms.tags`"),
    syn(
        "`.Data.Singular/Plural/Term/Terms`",
        "`page.taxonomy.singular/plural/terms`, `page.term.term`",
    ),
    syn(
        "`.OutputFormats.Get \"rss\"`, `.AlternativeOutputFormats`, `.MediaType`",
        "`page.output_formats.rss`, `page.alternative_output_formats`, `f.media_type.type`",
    ),
    syn(
        "`hugo.Version` / `Environment` / `IsProduction` / `IsDevelopment` / `Generator`",
        "`hugo.version` (`\"0.149.0-DEV\"`), `hugo.environment`, `hugo.is_production`, `hugo.is_development`, `hugo.generator`",
    ),
    syn("`.Site.Config.Privacy.*`", "`site.config.privacy.*`"),
    syn(
        "`.Data.Integrity`, `.Width`, `.Height`",
        "`r.data.integrity`, `r.width`, `r.height`",
    ),
    syn(
        "`partial \"x\" .` (shares the context)",
        "`{% include \"_partials/x.html\" %}`",
    ),
    syn(
        "`partial \"x\" (dict …)` with a literal name",
        "a component defined in `_partials/` (`{% component x(page, sep=\"/\", @lang) %}`), called as `{{ <x page={page} /> }}`",
    ),
    syn("`debug.Timer`", "removed"),
];

/// REWRITE_PLAN.md §4.7, applied by hand when converting layouts (and by `neohugo-migrate`).
pub const CONVERSION_RULES: &[(&str, &[&str])] = &[
    (
        "Names and paths",
        &[
            "Files use v0.146 names (`_partials/`, `_shortcodes/`, `_markup/`, `home|section|taxonomy|term|single|list|all|<layout>[.<lang>][.<fmt>].<ext>`, `baseof.html`). Include and extends literals are lower-case, new-style names.",
            "`.Title` → `page.title`; `.Site.X` and `site.X` → `site.x`.",
            "Params are lower-cased: `.Site.Params.HomeTitle` → `site.params.hometitle`. Data keys keep their case: `item.Name`.",
            "Reserved front-matter keys stay available in params: `where … \"Params.type\"` → `[p for p in l if p.params.type == \"snacks\"]`.",
        ],
    ),
    (
        "Missing values",
        &[
            "Printed params that may be missing → `page.params.x or \"\"` (printing an undefined value is an error).",
            "Nested optional lookups use `?.`: `page.params.a?.b`, `page.parent?.title or \"\"`.",
            "Hugo `default` → `default_if_empty(value=)`. Do not use `or` for bools.",
            "`x == none` is false when `x` is undefined; use `is undefined`, `is none` or truthiness instead.",
        ],
    ),
    (
        "Comparisons",
        &[
            "`eq $p $currentSection` → `p.id == current_section.id`. Pagers compare by `page_number`, dates by `.unix`.",
            "Mixed int/string comparisons get an explicit `int` or `str`.",
        ],
    ),
    (
        "Control flow",
        &[
            "`{{ with X }}…{{ else with Y }}` → `{% if X %}{% set x = X %}…{% elif Y %}…`.",
            "`range $k, $v := m` → `{% for k, v in m %}`; template map literals need `| sort_keys` first.",
            "`range $i, $e := l` → `{% for e in l %}` with `loop.index0` / `loop.first`.",
            "`where` → a list comprehension with `if`; `apply` → a comprehension; `seq N` → `range(start=1, end=N+1)`.",
            "Variable reassignment inside a block → `set_global` (discarded inside includes).",
        ],
    ),
    (
        "Templates and partials",
        &[
            "`define`/`block` in children → `{% extends \"baseof.html\" %}` plus `{% block %}`; delete blocks the parent does not define.",
            "Partials → include, component or `partial()`; component calls pass arguments as `name={expr}`, `name=\"literal\"` or the shorthand `name`.",
            "`try` → `optional=true` on `get_remote`, or a `none` check.",
        ],
    ),
    (
        "Formatting",
        &[
            "Go `printf` → `~`, `pad_start`/`pad_end`, `round`/`format_number`, `jsonify`.",
            "Go date layouts → strftime: `\"2006-01-02\"` → `\"%Y-%m-%d\"`, `\"Jan 2, 2006\"` → `\"%b %-d, %Y\"`. `.Format` stays English; `time.Format` and `dateFormat` localize names, so add `locale=lang`.",
        ],
    ),
    (
        "Removed Hugo idioms",
        &[
            "`range .Paginator.Pages` → `{% set pager = paginator() %}{% for p in pager.pages %}`; delete a second `.Paginate` that follows `.Paginator`.",
            "`{{ $noop := .WordCount }}` → delete.",
            "Another page's `.Content` inside a shortcode → `page_content(page=p)`.",
            "`.Scratch` / `newScratch` → `set`, `set_global`, `merge`; the page store only for cross-template flags.",
        ],
    ),
    (
        "Components",
        &["A component that calls site-bound functions passes `page=` or declares `@__nh`."],
    ),
    (
        "Escaping",
        &[
            "`html`/`htmlEscape` → `html_escape`. In `<script>`, use `jsonify | safe`. In query strings, use `urlencode`. `safeHTML` and the other `safe*` → `safe`.",
        ],
    ),
    (
        "Assets and i18n",
        &[
            "Assets used with `execute_as_template` are Tera templates: `{{ .api }}` → `{{ data.api }}`.",
            "i18n files stay Hugo syntax, limited to `{{ . }}` and `{{ .Field }}`.",
        ],
    ),
];

/// The embedded templates neohugo provides (T32), by v0.146 name. They are loaded under
/// [`EMBEDDED_PREFIX`], a Tera fallback prefix, so user and theme templates of the same name win.
pub const EMBEDDED_TEMPLATES: &[&str] = &[
    "_markup/render-codeblock-goat.html",
    "_markup/render-image.html",
    "_markup/render-link.html",
    "_markup/render-table.html",
    "_partials/_funcs/get-page-images.html",
    "_partials/google_analytics.html",
    "_partials/opengraph.html",
    "_partials/pagination.html",
    "_partials/schema.html",
    "_partials/twitter_cards.html",
    "_shortcodes/details.html",
    "_shortcodes/figure.html",
    "_shortcodes/highlight.html",
    "_shortcodes/instagram.html",
    "_shortcodes/param.html",
    "_shortcodes/qr.html",
    "_shortcodes/ref.html",
    "_shortcodes/relref.html",
    "_shortcodes/vimeo.html",
    "_shortcodes/x.html",
    "_shortcodes/youtube.html",
    "alias.html",
    "robots.txt",
    "rss.xml",
    "sitemap.xml",
    "sitemapindex.xml",
];

/// The Tera fallback prefix of the embedded templates.
pub const EMBEDDED_PREFIX: &str = "_embedded/";

/// Tera 2.4.0 behaviours the template model relies on, verified by T02 against the source and by
/// the `tera_facts` tests of neohugo-testkit.
pub const TERA_FACTS: &[&str] = &[
    "`__nh` is an ordinary identifier (a leading `_` is allowed) and `@__nh` a valid implicit component argument, resolved through the callers' scopes; a component that does not declare it cannot see it.",
    "Component call arguments are `name={expr}`, `name=\"literal\"` or the shorthand `name`; `name=expr` is a syntax error.",
    "`==` and `!=` never fail on an undefined final path segment: the value is undefined, and undefined equals only undefined (`x == none` is false). A missing non-final segment is an error, even inside `if`.",
    "Printing an undefined value is an error; `x or \"\"` and `default(value=)` are the fallbacks.",
    "`?.` (and `?[`) yield undefined when the receiver is undefined or none; the result must still not be printed bare.",
    "Built-in kwargs: `split(pat=)`, `nth(n=)`, `replace(from=, to=)`, `trim(pat=)`, `join(sep=)`, `round(method=, precision=)`, `truncate(length=, end=)`, `default(value=, boolean=)`, `get(key=, default=)`, `range(start=, end=, step_by=)`.",
    "Unknown filters, tests, functions, components and include targets are errors when templates are added; kwargs are checked only when called, so the contract test checks them statically.",
    "tera-contrib 0.3 names: `b64_encode`/`b64_decode`, `filesize_format`, `regex_replace(pattern=, rep=)`, `matching(pat=)`, `urlencode`, `urlencode_strict`, `date(format=, locale=, timezone=)`.",
];

fn md_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

fn phase_str(p: PhaseAvail) -> &'static str {
    match p {
        PhaseAvail::Both => "both",
        PhaseAvail::Content => "content",
        PhaseAvail::Layout => "layout",
    }
}

/// `rust/docs/template-api.md`, generated from the tables of this module.
#[must_use]
pub fn template_api_markdown() -> String {
    let mut out = String::new();
    write_markdown(&mut out).expect("writing to a String cannot fail");
    out
}

fn write_markdown(w: &mut String) -> std::fmt::Result {
    writeln!(w, "# neohugo template API\n")?;
    writeln!(
        w,
        "<!-- GENERATED from neohugo_funcs::spec (rust/crates/funcs/src/spec.rs); do not edit.\n     \
         Regenerate: INSTA_UPDATE=always cargo test -p neohugo-testkit contract -->\n"
    )?;
    writeln!(
        w,
        "Templates are Tera 2.4.0 (REWRITE_PLAN.md §4). Kind codes: `bi` Tera built-in, `tc` \
         tera-contrib, `F` neohugo filter, `fn` neohugo function, `T` neohugo test; `(s)` \
         site-bound (needs the site model or the render scope). Phase: `both`, or the only phase \
         the name works in. `=?` marks an optional kwarg, `…` any further kwargs.\n"
    )?;

    writeln!(
        w,
        "## Render contexts\n\n| Render | Top-level names |\n|---|---|"
    )?;
    for c in CONTEXTS {
        let mut names: Vec<String> = c.names.iter().map(|n| format!("`{}`", n.name)).collect();
        if !c.note.is_empty() {
            names.push(c.note.to_owned());
        }
        writeln!(w, "| {} | {} |", c.title, md_cell(&names.join(", ")))?;
    }
    writeln!(w, "\n| Name | Meaning |\n|---|---|")?;
    let mut seen: Vec<&str> = Vec::new();
    for n in CONTEXTS.iter().flat_map(|c| c.names) {
        if !seen.contains(&n.name) {
            seen.push(n.name);
            writeln!(w, "| `{}` | {} |", n.name, md_cell(n.doc))?;
        }
    }
    writeln!(
        w,
        "\nFlattened render-hook fields:\n\n| Hook | Fields |\n|---|---|"
    )?;
    for (hook, fields) in HOOK_FIELDS {
        let list: Vec<String> = fields.iter().map(|f| format!("`{f}`")).collect();
        writeln!(w, "| {hook} | {} |", list.join(" "))?;
    }

    writeln!(
        w,
        "\n## Syntax that replaces Hugo functions\n\n| Hugo | Tera |\n|---|---|"
    )?;
    for s in SYNTAX {
        writeln!(w, "| {} | {} |", md_cell(s.hugo), md_cell(s.tera))?;
    }

    for group in Group::ALL {
        writeln!(
            w,
            "\n## {}\n\n| Call | Kind | Phase | Safe | Hugo | Description |\n|---|---|---|---|---|---|",
            group.title()
        )?;
        for f in FUNCS.iter().filter(|f| f.group == group) {
            let hugo = if f.hugo.is_empty() {
                String::new()
            } else {
                format!("`{}`", f.hugo.replace(", ", "`, `"))
            };
            let types: Vec<String> = f
                .kwargs
                .iter()
                .map(|k| format!("{}: {}", k.name, k.ty.as_str()))
                .collect();
            let doc = if types.is_empty() {
                f.doc.to_owned()
            } else {
                format!("{} ({})", f.doc, types.join(", "))
            };
            writeln!(
                w,
                "| `{}` | {} | {} | {} | {} | {} |",
                md_cell(&f.signature()),
                f.code(),
                phase_str(f.phase),
                if f.safe { "yes" } else { "" },
                md_cell(&hugo),
                md_cell(&doc)
            )?;
        }
    }

    writeln!(w, "\n## Conversion rules\n")?;
    for (section, rules) in CONVERSION_RULES {
        writeln!(w, "**{section}**\n")?;
        for r in *rules {
            writeln!(w, "- {r}")?;
        }
        writeln!(w)?;
    }

    writeln!(
        w,
        "## Embedded templates\n\nLoaded under the fallback prefix `{EMBEDDED_PREFIX}`; a user or \
         theme template of the same name wins.\n"
    )?;
    for t in EMBEDDED_TEMPLATES {
        writeln!(w, "- `{t}`")?;
    }

    writeln!(
        w,
        "\n## Tera facts\n\nVerified against the tera 2.4.0 source and by the `tera_facts` tests of \
         neohugo-testkit.\n"
    )?;
    for f in TERA_FACTS {
        writeln!(w, "- {f}")?;
    }
    Ok(())
}
