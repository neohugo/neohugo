//! The docs site's cross-page shortcodes (REWRITE_PLAN.md §3.2, T36 row of §8.2), converted
//! to Tera and run through a full build on a small site shaped like `docs/content/en`:
//!
//! - `include` (`{{% include "_common/x" %}}` → `render_shortcodes`): a headless `_common`
//!   snippet with its own `{{< >}}` shortcode and a link, included between two shortcode
//!   calls of the including page; placeholders are renumbered, and the link hook sees the
//!   snippet as `page_inner`;
//! - `glossary-term` (`site.GetPage "/quick-reference/glossary/<term>"` → `render_shortcodes`)
//!   of a term page that is never rendered (`render: never`, `list: local`);
//! - `quick-reference` (another section's children with their `.Content` and descriptions as a
//!   definition list): `page_content` of other pages from inside a `{{% %}}` shortcode.

use neohugo_build::{BuildRequest, SinkKind, build};

use crate::support::write_files;

const INCLUDE: &str = r#"{%- set path = shortcode | arg(index=0, default="") -%}
{%- if not path %}{{ throw(message="include: the path of the file to include is missing") }}{% endif -%}
{%- set p = get_page(path=path) -%}
{%- if not p %}{{ throw(message="include: no page " ~ path) }}{% endif -%}
{{ render_shortcodes(page=p) }}"#;

const GLOSSARY_TERM: &str = r#"{%- set term = shortcode | arg(index=0, default="") -%}
{%- set p = get_page(path="/quick-reference/glossary/" ~ (term | urlize)) -%}
{%- if not p %}{{ throw(message="glossary-term: no term " ~ term) }}{% endif %}
{{ render_shortcodes(page=p) }}"#;

const QUICK_REFERENCE: &str = r#"{%- set s = get_page(path=shortcode | arg(name="section", default="")) -%}
{%- if not s %}{{ throw(message="quick-reference: no section") }}{% endif -%}
{% for c in s.sections %}{% set c = c | deref %}
## {{ c.link_title }}
{{ c.description }}

{{ page_content(page=c) }}
{% for p in c.pages %}
[{{ p.link_title }}]({{ p.rel_permalink }})
: {{ p.description }}
{% endfor %}{% endfor %}"#;

const BADGE: &str = r#"<span class="badge">{{ shortcode | arg(index=0, default="") }}</span>"#;

const LINK_HOOK: &str = r#"<a href="{{ destination }}" data-page="{{ page.path }}" data-inner="{{ page_inner.path }}">{{ text }}</a>"#;

const LAYOUT: &str = "<main>{{ page.content }}</main>";

fn site() -> Vec<(String, String)> {
    [
        (
            "hugo.toml",
            "baseURL = \"https://example.org/\"\ntitle = \"Docs\"\ndisableKinds = [\"taxonomy\", \"term\", \"rss\", \"sitemap\"]\n[markup.goldmark.renderer]\nunsafe = true\n",
        ),
        ("layouts/single.html", LAYOUT),
        ("layouts/list.html", LAYOUT),
        ("layouts/home.html", LAYOUT),
        ("layouts/_shortcodes/include.html", INCLUDE),
        ("layouts/_shortcodes/glossary-term.html", GLOSSARY_TERM),
        ("layouts/_shortcodes/quick-reference.html", QUICK_REFERENCE),
        ("layouts/_shortcodes/badge.html", BADGE),
        ("layouts/_markup/render-link.html", LINK_HOOK),
        ("content/_index.md", "---\ntitle: Home\n---\n"),
        (
            "content/_common/_index.md",
            "---\ncascade:\n  build:\n    list: never\n    publishResources: false\n    render: never\n---\n",
        ),
        (
            "content/_common/snippet.md",
            "---\n_comment: Do not remove front matter.\n---\n\nSnippet {{< badge \"inner\" >}} with a [link](/functions/).\n",
        ),
        (
            "content/guide.md",
            "---\ntitle: Guide\n---\n\nBefore {{< badge \"a\" >}} and [own](/).\n\n{{% include \"/_common/snippet\" %}}\n\nAfter {{< badge \"b\" >}}.\n\n{{% glossary-term \"floating point\" %}}\n",
        ),
        (
            "content/quick-reference/_index.md",
            "---\ntitle: Quick reference guides\nlinkTitle: Quick reference\n---\n",
        ),
        (
            "content/quick-reference/functions.md",
            "---\ntitle: Functions\n---\n\n{{% quick-reference section=\"/functions\" %}}\n",
        ),
        (
            "content/quick-reference/glossary/_index.md",
            "---\ntitle: Glossary\nbuild:\n  render: always\n  list: always\ncascade:\n  build:\n    render: never\n    list: local\n---\n",
        ),
        (
            "content/quick-reference/glossary/floating-point.md",
            "---\ntitle: floating point\n---\n\nThe term _floating point_ refers to a {{< badge \"number\" >}} type.\n",
        ),
        (
            "content/functions/_index.md",
            "---\ntitle: Functions\n---\n",
        ),
        (
            "content/functions/strings/_index.md",
            "---\ntitle: strings\ndescription: Use these functions to work with strings.\n---\n\nThe *strings* namespace.\n",
        ),
        (
            "content/functions/strings/lower.md",
            "---\ntitle: strings.ToLower\ndescription: Returns the given string, converting all characters to lowercase.\n---\n",
        ),
        (
            "content/functions/math/_index.md",
            "---\ntitle: math\ndescription: Use these functions to perform mathematical operations.\n---\n",
        ),
        (
            "content/functions/math/add.md",
            "---\ntitle: math.Add\ndescription: Adds two or more numbers.\n---\n",
        ),
    ]
    .into_iter()
    .map(|(p, c)| (p.to_owned(), c.to_owned()))
    .collect()
}

#[test]
fn docs_cross_page_shortcodes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("docs");
    write_files(&dir, &site());
    let report = build(BuildRequest {
        source: dir,
        sink: SinkKind::Memory,
        clock: Some("2026-09-27T12:00:00Z".parse().expect("clock")),
        ..BuildRequest::default()
    })
    .unwrap_or_else(|e| panic!("docs-like build: {e}"));
    let mem = report.memory.as_ref().expect("memory");
    let text = |p: &str| mem.text(p).unwrap_or_else(|| panic!("no {p}"));

    // include: the snippet's own shortcode output keeps its place between the including
    // page's two calls (renumbered placeholders), and its link hook sees the snippet as
    // `page_inner`, the including page as `page`.
    let guide = text("guide/index.html");
    println!("{guide}");
    assert!(
        guide.contains(concat!(
            r#"<p>Before <span class="badge">a</span> and <a href="/" data-page="/guide" data-inner="/guide">own</a>.</p>"#,
            "\n",
            r#"<p>Snippet <span class="badge">inner</span> with a <a href="/functions/" data-page="/guide" data-inner="/_common/snippet">link</a>.</p>"#,
            "\n",
            r#"<p>After <span class="badge">b</span>.</p>"#,
        )),
        "{guide}"
    );
    // glossary-term: a term page that is never rendered, its shortcode resolved.
    assert!(
        guide.contains(
            r#"<p>The term <em>floating point</em> refers to a <span class="badge">number</span> type.</p>"#
        ),
        "{guide}"
    );
    assert!(
        mem.get("quick-reference/glossary/floating-point/index.html")
            .is_none()
    );
    assert!(mem.get("_common/snippet/index.html").is_none());

    // quick-reference: the sections of /functions (default order: title), each with its
    // description, its `.Content` and its pages as a definition list.
    let qr = text("quick-reference/functions/index.html");
    println!("{qr}");
    let math = qr.find(r#"<h2 id="math">math</h2>"#).expect("math heading");
    let strings = qr
        .find(r#"<h2 id="strings">strings</h2>"#)
        .expect("strings heading");
    assert!(math < strings, "{qr}");
    assert!(
        qr.contains("<p>Use these functions to work with strings.</p>\n<p>The <em>strings</em> namespace.</p>"),
        "{qr}"
    );
    assert!(
        qr.contains(concat!(
            "<dl>\n<dt><a href=\"/functions/strings/lower/\" data-page=\"/quick-reference/functions\" ",
            "data-inner=\"/quick-reference/functions\">strings.ToLower</a></dt>\n",
            "<dd>Returns the given string, converting all characters to lowercase.</dd>\n</dl>"
        )),
        "{qr}"
    );
    assert!(report.collisions.is_empty());
}
