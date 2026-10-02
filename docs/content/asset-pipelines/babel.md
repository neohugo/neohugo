---
title: Babel
description: Transpile JavaScript with Babel and the babel filter.
weight: 80
---

`babel` runs [Babel](https://babeljs.io/) on a JavaScript resource. Most sites do not need it:
[`js_build`](/asset-pipelines/js-build/) already compiles TypeScript and JSX and lowers modern
syntax (`target`).

```sh
npm install -D @babel/core @babel/cli @babel/preset-env
```

```json {title="babel.config.json"}
{ "presets": ["@babel/preset-env"] }
```

Babel is not in the default list of commands fugo may run, so allow it:

{{< code-toggle file=config >}}
[security.exec]
  allow = ["^(dart-)?sass(-embedded)?$", "^go$", "^git$", "^npx$", "^postcss$", "^tailwindcss$", "^babel$"]
{{< /code-toggle >}}

```html
{% set js = get_asset(path="js/legacy.js") | babel(options={"minified": true}) %}
```

## Options

`config`
: The Babel configuration file (default: the project's).

`minified`, `compact`, `noComments`
: Output size.

`sourceMap`
: `inline` or `external` (published as `<target>.map`).

`noBabelrc`
: Ignore `.babelrc` files.

`verbose`
: Log what Babel does.

Find the tool as for [PostCSS](/asset-pipelines/postcss/#finding-the-tool), or set
`FUGO_BABEL_BIN`.
