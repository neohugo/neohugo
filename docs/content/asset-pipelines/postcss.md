---
title: PostCSS
description: Run PostCSS and its plugins on CSS resources with the postcss filter.
weight: 60
---

`postcss` pipes a CSS resource through [PostCSS](https://postcss.org/). It runs the
`postcss` command of `postcss-cli`, so install it in the project:

```sh
npm install -D postcss postcss-cli autoprefixer
```

```js {title="postcss.config.js"}
module.exports = {
  plugins: [require("autoprefixer")],
};
```

```html
{% set css = get_asset(path="css/main.css") | postcss | minify | fingerprint %}
```

## Options

`config`
: The directory of the PostCSS configuration (default: the project).

`noMap`
: Do not write a source map.

`inlineImports`
: Inline `@import`s of assets before PostCSS runs; `skipInlineImportsNotFound` leaves the
  imports it cannot find.

`use`, `parser`, `stringifier`, `syntax`
: Passed to `postcss-cli`.

## Finding the tool

fugo looks for `postcss` in the project's `node_modules/.bin`, then in each directory of
`FUGO_NODE_MODULES`, then on `PATH`; `FUGO_POSTCSS_BIN` names the binary directly. It runs in
the project directory with the environment `[security.exec] osEnv` allows, and its name must
match `[security.exec] allow` (it does by default).

To run PostCSS with what every page uses (PurgeCSS), wrap the chain in
[post-processing](/asset-pipelines/post-processing/) — or use fugo's built-in
[`purge_css`](/asset-pipelines/purge-css/).
