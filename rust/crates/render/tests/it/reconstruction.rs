//! R's shortcode suite: the reconstruction's shortcodes (`tools/rust-port/i01/seeksnack.txtar`,
//! `layouts/shortcodes/*`) converted to Tera as T61 will write them, run over R's edge page
//! (`about.md` of the `hugolib/content/seeksnack` oracle site) and R's snack pages. The
//! expectations follow Hugo's semantics (the content oracle checks the same page with the
//! oracle's simpler shortcodes).

use neohugo_view::{ContentRenderer, HookVariant};

use crate::support::{content_scope, site};

/// The reconstruction's shortcodes in Tera.
pub const R_SHORTCODES: &[(&str, &str)] = &[
    (
        "layouts/_shortcodes/note.html",
        r#"<div class="note">{{ inner }}</div>"#,
    ),
    (
        "layouts/_shortcodes/box.html",
        r#"<div class="box {{ shortcode | arg(name="class", default="") }}">{{ inner | markdownify }}</div>"#,
    ),
    (
        "layouts/_shortcodes/badge.html",
        r#"<span class="badge">{{ shortcode | arg(index=0, default="") }}{% set second = shortcode | arg(index=1, default="") %}{% if second %}-{{ second }}{% endif %}</span>"#,
    ),
    (
        "layouts/_shortcodes/img.html",
        r#"<img src="{{ shortcode | arg(name="src", default="") }}" alt="{{ shortcode | arg(name="alt", default="") }}" width="{{ shortcode | arg(name="width", default="") }}">"#,
    ),
    ("layouts/_shortcodes/v1.html", "<em>{{ inner }}</em>"),
    (
        "layouts/_shortcodes/quote.html",
        "<blockquote>{{ inner }} — {{ page.title }} ({{ shortcode.ordinal }})</blockquote>",
    ),
    (
        "layouts/_shortcodes/mixed.html",
        "mixed {{ shortcode.name }}",
    ),
    (
        "layouts/_shortcodes/thai.html",
        r#"{{ shortcode | arg(name="คำ", default="") }}"#,
    ),
    ("layouts/_shortcodes/empty.html", "{{- inner -}}"),
    (
        "layouts/_shortcodes/nested.html",
        "<section>{{ inner }}{% if shortcode.parent %} in {{ shortcode.parent.name }}{% endif %}</section>",
    ),
    (
        "layouts/_shortcodes/modalimage.html",
        r#"{% set src = shortcode | arg(name="src", default="") %}{% if src %}<img class="modal" src="{{ src }}">{% else %}<span>no image</span>{% endif %}"#,
    ),
];

/// R's edge page (the oracle's `about.md`), a snack page of the reconstruction, and a page for
/// `modalImage`.
const ABOUT: &str = "---\ntitle: \"About\"\ndate: 2020-01-02T03:04:05.678Z\ntype: seo\nlayout: simple\n---\n\nFirst paragraph with {{< badge \"new\" \"hot\" >}} and {{< img src=\"/a.jpg\" alt=\"An image\" width=300 >}}.\n\n<!--more-->\n\n{{% box class=\"wide\" %}}\nSome **markdown** in a box with {{< badge 42 >}} nested and {{< badge 3.14 true >}}.\n{{% /box %}}\n\n  {{< note >}}\n  Indented note with {{< nested >}}deep {{< badge \"x\" >}}{{< /nested >}}\n  {{< /note >}}\n\nSee [ref]({{< ref \"/disclaimer\" >}}) and {{< relref \"disclaimer.md\" >}}.\n\n{{% v1 %}}version one{{% /v1 %}} {{< v1 >}}*v1 no markup*{{< /v1 >}}\n{{</* escaped \"shortcode\" */>}}\n{{% quote %}}quoted{{% /quote %}} {{< Mixed >}} {{< thai คำ=\"ขนม\" >}} {{< empty />}}\n";

#[test]
fn r_shortcodes_on_r_pages() {
    let mut files: Vec<(&str, &str)> = vec![
        (
            "hugo.toml",
            "baseURL = \"https://seeksnack.com/\"\ntitle = \"SeekSnack\"\ndisableKinds = [\"taxonomy\", \"term\", \"rss\", \"sitemap\", \"404\"]\n[markup.goldmark.renderer]\nunsafe = true\n",
        ),
        ("content/about.md", ABOUT),
        (
            "content/disclaimer.md",
            "---\ntitle: Disclaimer\n---\nText.\n",
        ),
        (
            "content/snacks/koala/index.md",
            "---\ntitle: Koala\n---\n{{% note %}}Lots of *koalas*{{% /note %}}\n\nPrawn-like. {{< badge \"hot\" >}}\n",
        ),
        (
            "content/modal.md",
            "---\ntitle: Modal\n---\n{{< modalImage src=\"a.jpg\" >}} {{< modalImage >}}\n",
        ),
    ];
    files.extend_from_slice(R_SHORTCODES);
    let s = site(&files);
    s.session.render_content().expect("content");
    let get = |path: &str| {
        let id = s.page(path, 0);
        s.session
            .content(
                id,
                HookVariant::Html,
                &content_scope(&s.session, id, HookVariant::Html),
            )
            .expect("content")
    };

    let about = get("/about");
    assert_eq!(
        about.summary,
        r#"<p>First paragraph with <span class="badge">new-hot</span> and <img src="/a.jpg" alt="An image" width="300">.</p>"#
    );
    assert!(about.truncated);
    let expect = [
        // `{{% %}}` with a markdownified inner holding `{{< >}}` output.
        r#"<div class="box wide">Some <strong>markdown</strong> in a box with <span class="badge">42</span> nested and <span class="badge">3.14-true</span>.</div>"#,
        // Indented `{{< >}}` with inner content; `parent` of a nested call.
        "<div class=\"note\">\n  Indented note with <section>deep <span class=\"badge\">x</span> in note</section>\n  </div>",
        // `ref`/`relref` (embedded shortcodes; the test doubles of T35's functions).
        r#"<p>See <a href="https://seeksnack.com/disclaimer/">ref</a> and /disclaimer/.</p>"#,
        // `$_hugo_config` v1 is not reproduced: `{{% v1 %}}` output is Markdown.
        "<p><em>version one</em> <em>*v1 no markup*</em>\n{{&lt; escaped &ldquo;shortcode&rdquo; &gt;}}</p>",
        // Ordinal of a top-level call, `page` in a shortcode, `name` as written, a named
        // Thai parameter, an empty self-closed call.
        "<blockquote>quoted — About (8)</blockquote> mixed Mixed ขนม",
    ];
    for e in expect {
        assert!(about.html.contains(e), "missing {e}\nin {}", about.html);
    }

    let koala = get("/snacks/koala");
    assert_eq!(
        koala.html,
        "<div class=\"note\">Lots of *koalas*</div>\n<p>Prawn-like. <span class=\"badge\">hot</span></p>\n"
    );
    let modal = get("/modal");
    assert_eq!(
        modal.html,
        "<p><img class=\"modal\" src=\"a.jpg\"> <span>no image</span></p>\n"
    );
}
