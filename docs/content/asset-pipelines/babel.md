---
title: Babel
description: Transpile JavaScript with Babel and the babel filter.
weight: 80
---

`babel` runs [Babel](https://babeljs.io/) on a JavaScript resource. Most sites do not need it:
[`js_build`](/asset-pipelines/js-build/) already compiles TypeScript and JSX and lowers modern
syntax (`target`).

```json {title="package.json"}
{ "devDependencies": { "@babel/core": "^7.29.0", "@babel/cli": "^7.29.0", "@babel/preset-env": "^7.29.0" } }
```

fugo installs these packages when it builds and runs Babel without Node.js (see
[npm packages](/asset-pipelines/npm-packages/)).

```json {title="babel.config.json"}
{ "presets": ["@babel/preset-env"] }
```

Babel is not in the default list of commands fugo may run, so allow it:

{{< code-toggle file=config >}}
[security.exec]
  allow = ["^(dart-)?sass(-embedded)?$", "^go$", "^git$", "^npx$", "^tailwindcss$", "^babel$"]
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

fugo finds Babel the same way as [Tailwind CSS](/asset-pipelines/tailwind-css/#finding-the-tool):
the `@babel/cli` package in `node_modules`.
