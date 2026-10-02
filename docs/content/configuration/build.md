---
title: Build and minify
description: Record the classes and tags pages use, and configure HTML, CSS and JavaScript minification.
weight: 100
---

## Build

{{< code-toggle file=config >}}
[build]
  [build.buildStats]
    enable = true
    disableTags = false
    disableClasses = false
    disableIDs = false
{{< /code-toggle >}}

`buildStats`
: Write `build_stats.json` in the project directory: the HTML tags, classes and ids of every
  page, for [Tailwind CSS](/asset-pipelines/tailwind-css/) or PurgeCSS.

`useResourceCacheWhen` and `noJSConfigInAssets` are accepted for compatibility and have no
effect: fugo always runs the tools a pipeline names, and writes no `jsconfig.json`.

## Minify

`fugo build --minify`, or `minifyOutput`, minifies every HTML, CSS, JavaScript, JSON, SVG and
XML file fugo writes. `disableHTML`, `disableCSS`, `disableJS`, `disableJSON`, `disableSVG` and
`disableXML` turn off one type:

{{< code-toggle file=config >}}
[minify]
  minifyOutput = true
  disableXML = true
{{< /code-toggle >}}

fugo's minifiers are lightningcss (CSS), oxc (JavaScript) and minify-html (HTML). Hugo's
`[minify.tdewolff]` options are accepted, and only some apply.
