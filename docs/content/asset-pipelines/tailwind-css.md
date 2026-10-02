---
title: Tailwind CSS
description: Build Tailwind CSS 4 with the tailwind filter, using the classes your pages actually use.
weight: 70
---

`tailwind` runs the [Tailwind CSS](https://tailwindcss.com/) 4 CLI on a CSS resource:

```sh
npm install -D tailwindcss @tailwindcss/cli
```

{{< code-toggle file=config >}}
[build.buildStats]
  enable = true
{{< /code-toggle >}}

With `buildStats` on, fugo writes `build_stats.json` — the tags, classes and ids of every page —
in the project directory. Tailwind scans it for classes:

```css {title="assets/css/main.css"}
@import "tailwindcss";
@source "build_stats.json";
```

Build the stylesheet after every page is rendered, with [`defer`](/asset-pipelines/post-processing/#defer),
so it holds exactly the classes the site uses:

```html {title="layouts/baseof.html"}
<head>
  {{ defer(template="_partials/css.html", key="css") }}
</head>
```

```html {title="layouts/_partials/css.html"}
{%- set css = get_asset(path="css/main.css") | tailwind(options={"minify": build.is_production}) -%}
{%- if build.is_production %}{% set css = css | fingerprint %}{% endif -%}
<link rel="stylesheet" href="{{ css.rel_permalink }}">
```

## Options

`minify`
: Minify the CSS.

`optimize`
: Optimize without minifying.

`disableInlineImports`
: Leave `@import`s of assets to Tailwind. By default fugo inlines them first (imports of
  `tailwindcss` itself stay).

`skipInlineImportsNotFound`
: Keep imports fugo cannot find instead of failing.

## Finding the tool

As for [PostCSS](/asset-pipelines/postcss/#finding-the-tool): `node_modules/.bin/tailwindcss`,
`FUGO_NODE_MODULES`, `PATH`, or `FUGO_TAILWINDCSS_BIN`. The standalone Tailwind binary works
too.
