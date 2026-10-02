---
title: Post-processing
description: Delay a resource or a piece of a template until every page is rendered, for steps that need to know the whole site.
weight: 90
---

Some steps need the whole site: purging CSS against every page's classes, or listing what
every page used. Two tools delay work until all pages are rendered.

## post_process

`post_process` makes a resource's URL and content placeholders that fugo fills in after the
last page:

```html
{% set css = get_asset(path="css/main.css") | postcss | minify | fingerprint | post_process %}
<link rel="stylesheet" href="{{ css.rel_permalink }}" integrity="{{ css.data.integrity }}">
```

A chain with `postcss` or `tailwind` and `build_stats.json` (see
[Tailwind CSS](/asset-pipelines/tailwind-css/)) gets every page's classes this way.

## defer

`defer` renders a template after every page and puts its output where it was called:

```html {title="layouts/baseof.html"}
<head>
  {{ defer(template="_partials/deferred-css.html", key="css", data={"lang": lang}) }}
</head>
```

```html {title="layouts/_partials/deferred-css.html"}
{%- set css = get_asset(path="css/main.css") | tailwind | fingerprint -%}
<link rel="stylesheet" href="{{ css.rel_permalink }}">
```

The template is rendered once per `key`, and sees `data`, `site` and `build`, not `page`.
