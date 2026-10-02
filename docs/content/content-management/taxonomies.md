---
title: Taxonomies
description: Classify content with tags, categories or your own taxonomies, and list terms and their pages.
weight: 60
---

A taxonomy groups pages by *terms*: the taxonomy `tags` with the terms `rust` and `web`.
fugo creates a page for each taxonomy (`/tags/`) and each term (`/tags/rust/`).

## Configure taxonomies

The default taxonomies are `tags` and `categories`. Define your own, singular name first:

{{< code-toggle file=config >}}
[taxonomies]
  tag = "tags"
  category = "categories"
  series = "series"
{{< /code-toggle >}}

Defining `[taxonomies]` replaces the defaults; an empty table, or `disableKinds = ["taxonomy",
"term"]`, turns taxonomies off.

## Assign terms

{{< code-toggle file=content/posts/hello fm=true >}}
title = "Hello"
tags = ["rust", "web"]
series = ["Getting started"]
{{< /code-toggle >}}

A term's key is its name made URL-safe: `Getting started` is `/series/getting-started/`. A
page's position in a term's list can be set with a weight: `tags_weight = 10`.

## Templates

A taxonomy page uses `taxonomy.html`, a term page `term.html` (both fall back to `list.html`).
On a taxonomy page, `page.taxonomy` has the terms:

```html {title="layouts/taxonomy.html"}
<h1>{{ page.title }}</h1>
<ul>
  {% for t in page.taxonomy.terms | by_count %}
    <li><a href="{{ t.page.rel_permalink }}">{{ t.name }}</a> ({{ t.count }})</li>
  {% endfor %}
</ul>
```

A term page lists its pages as any list page does: `page.pages`, `page.term.name`.

A page's terms, e.g. in `single.html`:

```html
{% for t in page.terms.tags %}
  <a href="{{ t.rel_permalink }}">#{{ t.title }}</a>
{% endfor %}
```

Every term of a taxonomy, anywhere: `site.taxonomies.tags` maps each term's key to its name,
page, count and pages.

## Term pages with content

Add `content/tags/rust/_index.md` to give a term page a title, a description and content.
